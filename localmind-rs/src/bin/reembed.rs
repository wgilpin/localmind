use localmind_rs::{
    db::{Database, OperationPriority},
    ollama::OllamaClient,
    lmstudio::LMStudioClient,
    Result
};
use std::env;

enum EmbeddingMode {
    Ollama(OllamaClient),
    LMStudio(LMStudioClient),
}

impl EmbeddingMode {
    async fn generate_embedding(&self, text: &str, document_title: &str) -> Result<Vec<f32>> {
        match self {
            EmbeddingMode::Ollama(client) => client.generate_embedding(text).await,
            EmbeddingMode::LMStudio(client) => {
                client.generate_embedding(text, false, Some(document_title)).await
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 LocalMind Database Re-Embedding Tool");
    println!("========================================");
    println!();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let embedding_mode = if args.len() < 2 {
        println!("Usage: reembed <mode> [options]");
        println!();
        println!("Modes:");
        println!("  ollama <url> <model>          - Use Ollama for embeddings");
        println!("  lmstudio <url> <model>        - Use LM Studio for embeddings");
        println!();
        println!("Examples:");
        println!("  reembed ollama http://localhost:11434 nomic-embed-text");
        println!("  reembed lmstudio http://localhost:1234 text-embedding-nomic-embed-text-v1.5");
        println!();
        return Err("Missing arguments".into());
    } else {
        let mode = &args[1];
        match mode.as_str() {
            "ollama" => {
                if args.len() < 4 {
                    return Err("Usage: reembed ollama <url> <model>".into());
                }
                let url = args[2].clone();
                let model = args[3].clone();

                println!("🦙 Using Ollama embeddings");
                println!("   URL: {}", url);
                println!("   Model: {}", model);

                let client = OllamaClient::with_models(url, model, "".to_string());
                EmbeddingMode::Ollama(client)
            }
            "lmstudio" => {
                if args.len() < 4 {
                    return Err("Usage: reembed lmstudio <url> <model>".into());
                }
                let url = args[2].clone();
                let model = args[3].clone();

                println!("🎨 Using LM Studio embeddings");
                println!("   URL: {}", url);
                println!("   Model: {}", model);

                let client = LMStudioClient::new(url.clone(), model.clone());

                // Test connection
                match client.test_connection().await {
                    Ok(_) => println!("✅ Connection test successful"),
                    Err(e) => {
                        println!("❌ Connection test failed: {}", e);
                        return Err(e);
                    }
                }

                EmbeddingMode::LMStudio(client)
            }
            _ => {
                return Err(format!("Unknown mode: {}. Use 'ollama' or 'lmstudio'", mode).into());
            }
        }
    };

    println!();
    println!("🔌 Connecting to database...");

    // Initialize database
    let db = Database::new().await?;

    // Get all documents with their chunks
    println!("📊 Analyzing database...");
    let documents = db.get_all_documents().await?;

    if documents.is_empty() {
        println!("ℹ️ No documents found in database");
        return Ok(());
    }

    // Count total chunks
    let mut total_chunks = 0;
    let mut chunks_by_doc = Vec::new();

    for doc in &documents {
        let chunks = db.get_chunk_embeddings_for_document(doc.id).await?;
        let chunk_count = chunks.len();
        total_chunks += chunk_count;
        chunks_by_doc.push((doc.id, doc.title.clone(), chunk_count));
    }

    println!("📚 Found {} documents with {} total chunks", documents.len(), total_chunks);
    println!();

    // Confirm before proceeding
    println!("⚠️  WARNING: This will regenerate ALL embeddings in the database!");
    println!("   This operation cannot be undone and may take significant time.");
    println!();
    print!("Continue? (yes/no): ");

    use std::io::{self, Write};
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "yes" {
        println!("❌ Aborted by user");
        return Ok(());
    }

    println!();
    println!("🚀 Starting re-embedding process...");
    println!();

    let mut processed_chunks = 0;
    let mut processed_docs = 0;
    let start_time = std::time::Instant::now();

    // Process each document
    for (doc_idx, doc) in documents.iter().enumerate() {
        let doc_start = std::time::Instant::now();

        // Get chunks for this document
        let chunks = db.get_chunk_embeddings_for_document(doc.id).await?;

        if chunks.is_empty() {
            println!("⚠️  Doc {}/{}: '{}' has no chunks, skipping",
                    doc_idx + 1, documents.len(),
                    doc.title.chars().take(60).collect::<String>());
            continue;
        }

        println!("📝 Doc {}/{}: '{}' ({} chunks)",
                doc_idx + 1, documents.len(),
                doc.title.chars().take(60).collect::<String>(),
                chunks.len());

        // Process each chunk
        for (chunk_idx, chunk_embedding) in chunks.iter().enumerate() {
            // Extract chunk text from document content
            let content_chars: Vec<char> = doc.content.chars().collect();

            let chunk_text = if chunk_embedding.3 <= content_chars.len() {
                content_chars[chunk_embedding.2..chunk_embedding.3]
                    .iter()
                    .collect::<String>()
            } else {
                println!("   ⚠️  Chunk {}: Invalid boundaries, skipping", chunk_idx);
                continue;
            };

            // Generate new embedding with proper formatting
            match embedding_mode.generate_embedding(&chunk_text, &doc.title).await {
                Ok(embedding) => {
                    // Serialize and update in database
                    let embedding_bytes = bincode::serialize(&embedding)?;

                    db.update_chunk_embedding(
                        chunk_embedding.0,
                        &embedding_bytes,
                        OperationPriority::BackgroundIngest,
                    ).await?;

                    processed_chunks += 1;

                    // Progress indicator every 10 chunks
                    if chunk_idx % 10 == 0 && chunk_idx > 0 {
                        print!("   .");
                        io::stdout().flush()?;
                    }
                }
                Err(e) => {
                    println!("   ❌ Chunk {}: Failed to generate embedding: {}", chunk_idx, e);
                }
            }
        }

        println!();

        let doc_elapsed = doc_start.elapsed();
        processed_docs += 1;

        let elapsed = start_time.elapsed();
        let chunks_per_sec = processed_chunks as f64 / elapsed.as_secs_f64();
        let remaining_chunks = total_chunks - processed_chunks;
        let eta_secs = (remaining_chunks as f64 / chunks_per_sec) as u64;

        println!("   ✅ Processed {} chunks in {:.1}s ({:.1} chunks/sec)",
                chunks.len(), doc_elapsed.as_secs_f64(), chunks_per_sec);
        println!("   📊 Overall: {}/{} chunks, ETA: {}m {}s",
                processed_chunks, total_chunks,
                eta_secs / 60, eta_secs % 60);
        println!();
    }

    let total_elapsed = start_time.elapsed();
    let avg_chunks_per_sec = processed_chunks as f64 / total_elapsed.as_secs_f64();

    println!("========================================");
    println!("🎉 Re-embedding complete!");
    println!();
    println!("📊 Statistics:");
    println!("   Documents processed: {}/{}", processed_docs, documents.len());
    println!("   Chunks re-embedded: {}/{}", processed_chunks, total_chunks);
    println!("   Total time: {:.1}s", total_elapsed.as_secs_f64());
    println!("   Average speed: {:.1} chunks/sec", avg_chunks_per_sec);
    println!();
    println!("✅ All embeddings have been regenerated!");
    println!("💡 You may need to restart the application to reload the vector store");

    Ok(())
}