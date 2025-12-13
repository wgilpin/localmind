use localmind_rs::{
    db::{Database, OperationPriority},
    lmstudio::LMStudioClient,
    ollama::OllamaClient,
    Result,
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
                client
                    .generate_embedding(text, false, Some(document_title))
                    .await
            }
        }
    }

    async fn generate_embeddings_batch(
        &self,
        texts: Vec<String>,
        document_title: &str,
    ) -> Result<Vec<Vec<f32>>> {
        match self {
            EmbeddingMode::Ollama(_client) => {
                // Ollama doesn't support batch - fall back to sequential
                let mut embeddings = Vec::new();
                for text in texts {
                    embeddings.push(self.generate_embedding(&text, document_title).await?);
                }
                Ok(embeddings)
            }
            EmbeddingMode::LMStudio(client) => {
                // Format all texts with document prefix
                let formatted_texts: Vec<String> = texts
                    .iter()
                    .map(|text| client.format_text_for_embedding(text, false, Some(document_title)))
                    .collect();
                client.generate_embeddings(formatted_texts).await
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("LocalMind Database Re-Embedding Tool (Batched)");
    println!("==================================================");
    println!();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let (embedding_mode, batch_size) = if args.len() < 2 {
        println!("Usage: reembed_batched <mode> <url> <model> [batch_size]");
        println!();
        println!("Modes:");
        println!("  ollama <url> <model> [batch_size]    - Use Ollama for embeddings");
        println!("  lmstudio <url> <model> [batch_size]  - Use LM Studio for embeddings");
        println!();
        println!("Examples:");
        println!("  reembed_batched ollama http://localhost:11434 qwen3-embedding:0.6b 32");
        println!("  reembed_batched lmstudio http://localhost:1234 text-embedding-embeddinggemma-300m-qat 50");
        println!();
        println!("Default batch size: 32");
        println!();
        return Err("Missing arguments".into());
    } else {
        let mode = &args[1];
        let batch_size = if args.len() >= 5 {
            args[4].parse::<usize>().unwrap_or(32)
        } else {
            32
        };

        let embedding_mode = match mode.as_str() {
            "ollama" => {
                if args.len() < 4 {
                    return Err("Usage: reembed_batched ollama <url> <model> [batch_size]".into());
                }
                let url = args[2].clone();
                let model = args[3].clone();

                println!("Using Ollama embeddings");
                println!("   URL: {}", url);
                println!("   Model: {}", model);
                println!(
                    "   Batch size: {} (sequential, Ollama doesn't support batching)",
                    batch_size
                );

                let client = OllamaClient::with_models(url, model, "".to_string());
                EmbeddingMode::Ollama(client)
            }
            "lmstudio" => {
                if args.len() < 4 {
                    return Err("Usage: reembed_batched lmstudio <url> <model> [batch_size]".into());
                }
                let url = args[2].clone();
                let model = args[3].clone();

                println!("Using LM Studio embeddings (BATCHED)");
                println!("   URL: {}", url);
                println!("   Model: {}", model);
                println!("   Batch size: {}", batch_size);

                let client = LMStudioClient::new(url.clone(), model.clone());

                // Test connection
                match client.test_connection().await {
                    Ok(_) => println!("Connection test successful"),
                    Err(e) => {
                        println!("Connection test failed: {}", e);
                        return Err(e);
                    }
                }

                EmbeddingMode::LMStudio(client)
            }
            _ => {
                return Err(format!("Unknown mode: {}. Use 'ollama' or 'lmstudio'", mode).into());
            }
        };

        (embedding_mode, batch_size)
    };

    println!();
    println!("Connecting to database...");

    // Initialize database
    let db = Database::new().await?;

    // Get all documents with their chunks
    println!("Analyzing database...");
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

    println!(
        "Found {} documents with {} total chunks",
        documents.len(),
        total_chunks
    );
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
        println!("Aborted by user");
        return Ok(());
    }

    println!();
    println!(
        "Starting re-embedding process with batch size {}...",
        batch_size
    );
    println!();

    let mut processed_chunks = 0;
    let mut processed_docs = 0;
    let start_time = std::time::Instant::now();

    // Allow up to 15 characters beyond the end for word boundary leeway
    const BOUNDARY_LEEWAY: usize = 15;

    // Process each document
    for (doc_idx, doc) in documents.iter().enumerate() {
        let doc_start = std::time::Instant::now();

        // Get chunks for this document
        let chunks = db.get_chunk_embeddings_for_document(doc.id).await?;

        if chunks.is_empty() {
            println!(
                "⚠️  Doc {}/{}: '{}' has no chunks, skipping",
                doc_idx + 1,
                documents.len(),
                doc.title.chars().take(60).collect::<String>()
            );
            continue;
        }

        println!(
            "Doc {}/{}: '{}' ({} chunks)",
            doc_idx + 1,
            documents.len(),
            doc.title.chars().take(60).collect::<String>(),
            chunks.len()
        );

        let content_len = doc.content.len(); // byte length

        // Process chunks in batches
        for batch_start in (0..chunks.len()).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(chunks.len());
            let batch = &chunks[batch_start..batch_end];

            // Extract chunk texts and collect metadata
            let mut chunk_texts = Vec::new();
            let mut chunk_ids = Vec::new();
            let mut valid_indices = Vec::new();

            for (local_idx, chunk_embedding) in batch.iter().enumerate() {
                let (chunk_id, _chunk_index, chunk_start, chunk_end, _old_embedding) =
                    chunk_embedding;

                if *chunk_end > content_len + BOUNDARY_LEEWAY {
                    // Extract what we can for debugging
                    let actual_chunk_end = (*chunk_end).min(content_len);
                    let partial_chunk = if *chunk_start < content_len {
                        std::str::from_utf8(&doc.content.as_bytes()[*chunk_start..actual_chunk_end])
                            .unwrap_or("[invalid UTF-8]")
                    } else {
                        "[chunk_start beyond document]"
                    };

                    println!("   ⚠️  Chunk {}: Invalid boundaries ({}..{} > {} + {}), skipping. Content: '{}'",
                            batch_start + local_idx, chunk_start, chunk_end, content_len, BOUNDARY_LEEWAY,
                            partial_chunk.chars().take(200).collect::<String>());
                    continue;
                }

                // Clamp chunk_end to actual content length for extraction
                let actual_chunk_end = (*chunk_end).min(content_len);
                let chunk_text =
                    std::str::from_utf8(&doc.content.as_bytes()[*chunk_start..actual_chunk_end])
                        .unwrap_or("")
                        .to_string();
                chunk_texts.push(chunk_text);
                chunk_ids.push(*chunk_id);
                valid_indices.push(local_idx);
            }

            if chunk_texts.is_empty() {
                continue;
            }

            // Generate embeddings for the entire batch
            io::stdout().flush()?;

            match embedding_mode
                .generate_embeddings_batch(chunk_texts.clone(), &doc.title)
                .await
            {
                Ok(embeddings) => {
                    // Update database with batch results
                    for (i, embedding) in embeddings.iter().enumerate() {
                        let embedding_bytes = bincode::serialize(embedding)?;

                        db.update_chunk_embedding(
                            chunk_ids[i],
                            &embedding_bytes,
                            OperationPriority::BackgroundIngest,
                        )
                        .await?;

                        processed_chunks += 1;
                    }
                }
                Err(e) => {
                    println!("Batch failed: {}", e);
                    println!("   Falling back to sequential processing for this batch...");

                    // Fall back to sequential for this batch
                    for (i, text) in chunk_texts.iter().enumerate() {
                        match embedding_mode.generate_embedding(text, &doc.title).await {
                            Ok(embedding) => {
                                let embedding_bytes = bincode::serialize(&embedding)?;
                                db.update_chunk_embedding(
                                    chunk_ids[i],
                                    &embedding_bytes,
                                    OperationPriority::BackgroundIngest,
                                )
                                .await?;
                                processed_chunks += 1;
                            }
                            Err(e) => {
                                println!("   ❌ Chunk {}: {}", batch_start + valid_indices[i], e);
                            }
                        }
                    }
                }
            }
        }

        let doc_elapsed = doc_start.elapsed();
        processed_docs += 1;

        let elapsed = start_time.elapsed();
        let chunks_per_sec = processed_chunks as f64 / elapsed.as_secs_f64();
        let remaining_chunks = total_chunks - processed_chunks;
        let eta_secs = if chunks_per_sec > 0.0 {
            (remaining_chunks as f64 / chunks_per_sec) as u64
        } else {
            0
        };
    }

    let total_elapsed = start_time.elapsed();
    let avg_chunks_per_sec = processed_chunks as f64 / total_elapsed.as_secs_f64();

    println!("========================================");
    println!("Re-embedding complete!");
    println!();
    println!("Statistics:");
    println!(
        "   Documents processed: {}/{}",
        processed_docs,
        documents.len()
    );
    println!(
        "   Chunks re-embedded: {}/{}",
        processed_chunks, total_chunks
    );
    println!("   Total time: {:.1}s", total_elapsed.as_secs_f64());
    println!("   Average speed: {:.1} chunks/sec", avg_chunks_per_sec);
    println!("   Speedup vs sequential: ~{}x", batch_size);
    println!();

    // Save embedding configuration to database
    println!("Saving embedding configuration to database...");
    match &embedding_mode {
        EmbeddingMode::Ollama(_) => {
            db.set_embedding_model(&args[3]).await?;
            db.set_embedding_url(&args[2]).await?;
            println!("   ✅ Saved: Ollama model '{}' at '{}'", args[3], args[2]);
        }
        EmbeddingMode::LMStudio(_) => {
            db.set_embedding_model(&args[3]).await?;
            db.set_embedding_url(&args[2]).await?;
            println!(
                "   ✅ Saved: LM Studio model '{}' at '{}'",
                args[3], args[2]
            );
        }
    }
    println!();

    println!("All embeddings have been regenerated!");
    println!("You may need to restart the application to reload the vector store");

    Ok(())
}
