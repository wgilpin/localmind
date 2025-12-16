// This script re-chunks all documents in the database using the improved chunking algorithm.
// It backs up the database first, then deletes all existing embeddings and creates new chunks.
// After running this, you must run reembed_batched to generate embeddings for the new chunks.

use localmind_rs::{
    db::{Database, OperationPriority},
    document::DocumentProcessor,
    Result,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn get_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("localmind")
        .join("localmind.db")
}

fn backup_database() -> Result<PathBuf> {
    let db_path = get_db_path();
    if !db_path.exists() {
        return Err("Database file not found".into());
    }

    // Use Unix timestamp for backup filename
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let backup_path = db_path.with_file_name(format!("localmind_backup_{}.db", timestamp));
    
    std::fs::copy(&db_path, &backup_path)?;
    Ok(backup_path)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting database re-chunking process...");
    println!();

    // Backup database first
    println!("📦 Creating database backup...");
    match backup_database() {
        Ok(backup_path) => {
            println!("   ✅ Backup created: {}", backup_path.display());
        }
        Err(e) => {
            println!("   ❌ Backup failed: {}", e);
            println!("   Aborting to prevent data loss.");
            return Err(e);
        }
    }
    println!();

    // Initialize database (uses default location)
    let db = Database::new().await?;
    let document_processor = DocumentProcessor::default();

    // Get all live documents with URLs
    let documents = db.get_live_documents_with_urls().await?;
    println!("Found {} documents to re-chunk", documents.len());

    if documents.is_empty() {
        println!("ℹ️ No documents found in database");
        return Ok(());
    }

    println!(
        "⚠️ Re-chunking {} documents and updating database",
        documents.len()
    );
    println!("This will DELETE ALL existing embeddings and create new chunks");
    println!("You will need to re-embed after rechunking");
    println!();

    // Delete ALL embeddings first
    println!("🗑️  Deleting all existing embeddings...");
    db.delete_all_embeddings().await?;
    println!("All embeddings deleted");
    println!();

    let mut total_chunks = 0;
    let mut processed_docs = 0;

    // Re-process each document and store chunks in database
    for (i, doc) in documents.iter().enumerate() {
        if i % 100 == 0 {
            println!("Progress: {}/{} documents processed", i, documents.len());
        }

        // Re-chunk the document with improved logic
        let doc_len = doc.content.len();
        match document_processor.chunk_text(&doc.content) {
            Ok(chunks) => {
                // Debug: Check if any chunks exceed document length
                for chunk in &chunks {
                    if chunk.end_pos > doc_len {
                        println!(
                            "CRITICAL BUG: Doc {} (len={}) has chunk ending at {} ({}chars over)!",
                            doc.id,
                            doc_len,
                            chunk.end_pos,
                            chunk.end_pos - doc_len
                        );
                    }
                }
                // Store each chunk in the database (without embeddings for now)
                for chunk in chunks.iter() {
                    // Create a placeholder embedding (empty bytes) - embeddings will be generated later
                    let empty_embedding = bincode::serialize(&Vec::<f32>::new())?;

                    match db
                        .insert_chunk_embedding(
                            doc.id,
                            chunk.start_pos,
                            chunk.end_pos,
                            &empty_embedding,
                            OperationPriority::BackgroundIngest,
                        )
                        .await
                    {
                        Ok(_) => total_chunks += 1,
                        Err(e) => println!("Error storing chunk for doc {}: {}", doc.id, e),
                    }
                }
                processed_docs += 1;
            }
            Err(e) => println!("Error chunking document {}: {}", doc.id, e),
        }
    }

    println!("Database re-chunking complete!");
    println!(
        "Processed {} documents, created {} chunks",
        processed_docs, total_chunks
    );
    println!("Chunks now use improved word-boundary logic");
    println!();
    println!("⚠️  Next step: Run reembed_batched to generate embeddings for all chunks");

    println!("Database re-chunking completed successfully!");

    Ok(())
}
