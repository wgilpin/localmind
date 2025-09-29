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
    println!("💡 This will create chunk entries without embeddings");
    println!("🚀 Embeddings will be generated when documents are accessed");

    let mut total_chunks = 0;
    let mut processed_docs = 0;

    // Re-process each document and store chunks in database
    for (i, doc) in documents.iter().enumerate() {
        if i % 100 == 0 {
            println!("🔄 Progress: {}/{} documents processed", i, documents.len());
        }

        // Re-chunk the document with improved logic
        match document_processor.chunk_text(&doc.content) {
            Ok(chunks) => {
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
    println!("🔧 Embeddings will be generated automatically when documents are accessed");

    println!("✅ Database re-chunking completed successfully!");

    Ok(())
}