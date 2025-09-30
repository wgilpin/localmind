use localmind_rs::{db::{Database, OperationPriority}, document::DocumentProcessor, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 Starting database re-chunking process...");

    // Initialize database (uses default location)
    let db = Database::new().await?;
    let document_processor = DocumentProcessor::default();

    // Get all live documents with URLs
    let documents = db.get_live_documents_with_urls().await?;
    println!("📚 Found {} documents to re-chunk", documents.len());

    if documents.is_empty() {
        println!("ℹ️ No documents found in database");
        return Ok(());
    }

    println!("⚠️ Re-chunking {} documents and updating database", documents.len());
    println!("💡 This will DELETE ALL existing embeddings and create new chunks");
    println!("🚀 You will need to re-embed after rechunking");
    println!();

    // Delete ALL embeddings first
    println!("🗑️  Deleting all existing embeddings...");
    db.delete_all_embeddings().await?;
    println!("✅ All embeddings deleted");
    println!();

    let mut total_chunks = 0;
    let mut processed_docs = 0;

    // Re-process each document and store chunks in database
    for (i, doc) in documents.iter().enumerate() {
        if i % 100 == 0 {
            println!("🔄 Progress: {}/{} documents processed", i, documents.len());
        }

        // Re-chunk the document with improved logic
        let doc_len = doc.content.len();
        match document_processor.chunk_text(&doc.content) {
            Ok(chunks) => {
                // Debug: Check if any chunks exceed document length
                for chunk in &chunks {
                    if chunk.end_pos > doc_len {
                        println!("❌ CRITICAL BUG: Doc {} (len={}) has chunk ending at {} ({}chars over)!",
                                doc.id, doc_len, chunk.end_pos, chunk.end_pos - doc_len);
                    }
                }
                // Store each chunk in the database (without embeddings for now)
                for (chunk_index, chunk) in chunks.iter().enumerate() {
                    // Create a placeholder embedding (empty bytes) - embeddings will be generated later
                    let empty_embedding = bincode::serialize(&Vec::<f32>::new())?;

                    match db.insert_chunk_embedding(
                        doc.id,
                        chunk_index,
                        chunk.start_pos,
                        chunk.end_pos,
                        &empty_embedding,
                        OperationPriority::BackgroundIngest,
                    ).await {
                        Ok(_) => total_chunks += 1,
                        Err(e) => println!("❌ Error storing chunk for doc {}: {}", doc.id, e),
                    }
                }
                processed_docs += 1;
            },
            Err(e) => println!("❌ Error chunking document {}: {}", doc.id, e),
        }
    }

    println!("🎉 Database re-chunking complete!");
    println!("📊 Processed {} documents, created {} chunks", processed_docs, total_chunks);
    println!("💡 Chunks now use improved word-boundary logic");
    println!();
    println!("⚠️  Next step: Run reembed_batched to generate embeddings for all chunks");

    println!("✅ Database re-chunking completed successfully!");

    Ok(())
}