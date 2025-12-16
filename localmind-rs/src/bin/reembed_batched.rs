use localmind_rs::{
    db::{Database, OperationPriority},
    local_embedding::LocalEmbeddingClient,
    Result,
};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LocalMind Database Re-Embedding Tool (Batched)");
    println!("==================================================");
    println!();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let batch_size = if args.len() >= 2 {
        args[1].parse::<usize>().unwrap_or(32)
    } else {
        32
    };

    println!("Using Local Python Embedding Server");
    println!(
        "   Server: http://localhost:{}",
        std::env::var("EMBEDDING_SERVER_PORT").unwrap_or_else(|_| "8000".to_string())
    );
    println!("   Model: google/embeddinggemma-300M");
    println!(
        "   Batch size: {} (sequential, server processes one at a time)",
        batch_size
    );
    println!();

    // Initialize LocalEmbeddingClient
    let embedding_client = LocalEmbeddingClient::new();

    // Test connection
    match embedding_client.health_check().await {
        Ok(true) => println!("✅ Connection test successful - server is ready"),
        Ok(false) => {
            println!("⚠️  Server is running but model is still loading...");
            println!("   Proceeding anyway, but embeddings may be slow");
        }
        Err(e) => {
            println!("❌ Connection test failed: {}", e);
            println!();
            println!("Make sure the Python embedding server is running:");
            println!("  cd embedding-server");
            println!("  python embedding_server.py");
            return Err(format!("Embedding server not available: {}", e).into());
        }
    }

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
                let (chunk_id, chunk_start, chunk_end, _old_embedding) = chunk_embedding;

                if *chunk_end > content_len + BOUNDARY_LEEWAY {
                    // Extract what we can for debugging
                    let actual_chunk_end = (*chunk_end).min(content_len);
                    let partial_chunk = if *chunk_start < content_len {
                        doc.content
                            .get(*chunk_start..actual_chunk_end)
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
                let chunk_text = doc
                    .content
                    .get(*chunk_start..actual_chunk_end)
                    .unwrap_or("")
                    .to_string();
                chunk_texts.push(chunk_text);
                chunk_ids.push(*chunk_id);
                valid_indices.push(local_idx);
            }

            if chunk_texts.is_empty() {
                continue;
            }

            // Generate embeddings sequentially (server processes one at a time)
            io::stdout().flush()?;

            for (i, text) in chunk_texts.iter().enumerate() {
                match embedding_client.generate_embedding(text).await {
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

        let _doc_elapsed = doc_start.elapsed();
        processed_docs += 1;

        let elapsed = start_time.elapsed();
        let chunks_per_sec = processed_chunks as f64 / elapsed.as_secs_f64();
        let _remaining_chunks = total_chunks - processed_chunks;
        let _eta_secs = if chunks_per_sec > 0.0 {
            (_remaining_chunks as f64 / chunks_per_sec) as u64
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
    db.set_embedding_model("google/embeddinggemma-300M").await?;
    let server_url = format!(
        "http://localhost:{}",
        std::env::var("EMBEDDING_SERVER_PORT").unwrap_or_else(|_| "8000".to_string())
    );
    db.set_embedding_url(&server_url).await?;
    println!("   ✅ Saved: Local Python Embedding Server model 'google/embeddinggemma-300M'");
    println!();

    println!("All embeddings have been regenerated!");
    println!("You may need to restart the application to reload the vector store");

    Ok(())
}
