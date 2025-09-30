use crate::Result;
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, SemaphorePermit};

#[derive(Debug, Clone, Copy)]
pub enum OperationPriority {
    UserSearch,      // Highest priority - immediate access
    BackgroundIngest, // Lower priority - can be interrupted
}

pub struct Database {
    conn: Arc<Mutex<Connection>>,
    // Semaphore to control concurrent access with priority
    search_semaphore: Arc<Semaphore>,
    ingest_semaphore: Arc<Semaphore>,
}

pub struct Document {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub url: Option<String>,
    pub source: String,
    pub created_at: String,
    pub embedding: Option<Vec<u8>>,
    pub is_dead: Option<bool>,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("localmind");

        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("localmind.db");

        let conn = Connection::open(&db_path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            // Allow 10 concurrent searches, but only 1 background ingest
            search_semaphore: Arc::new(Semaphore::new(10)),
            ingest_semaphore: Arc::new(Semaphore::new(1)),
        };

        db.init_schema().await?;
        Ok(db)
    }

    async fn init_schema(&self) -> Result<()> {
        let _permit = self.get_priority_access(OperationPriority::UserSearch).await?;
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                url TEXT,
                source TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                embedding BLOB,
                is_dead BOOLEAN DEFAULT 0
            )",
            [],
        )?;

        // Add is_dead column if it doesn't exist (migration)
        let _ = conn.execute(
            "ALTER TABLE documents ADD COLUMN is_dead BOOLEAN DEFAULT 0",
            [],
        );

        // Create FTS table for text search (without content_tokenize for compatibility)
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                title, content
            )",
            [],
        )?;

        // Create trigger to keep FTS in sync
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
                INSERT INTO documents_fts(rowid, title, content) VALUES (new.id, new.title, new.content);
            END",
            [],
        )?;

        // Create embeddings table for chunk embeddings
        conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id INTEGER NOT NULL,
                chunk_index INTEGER NOT NULL,
                chunk_start INTEGER NOT NULL,
                chunk_end INTEGER NOT NULL,
                embedding BLOB NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (document_id) REFERENCES documents (id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create index for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_embeddings_document_id ON embeddings(document_id)",
            [],
        )?;

        Ok(())
    }

    async fn get_priority_access(&self, priority: OperationPriority) -> Result<SemaphorePermit> {
        match priority {
            OperationPriority::UserSearch => {
                // User searches get immediate access
                Ok(self.search_semaphore.acquire().await.unwrap())
            }
            OperationPriority::BackgroundIngest => {
                // Background ingests wait and can be interrupted
                // Try to acquire with timeout to avoid blocking searches
                match tokio::time::timeout(Duration::from_millis(100),
                                         self.ingest_semaphore.acquire()).await {
                    Ok(permit) => Ok(permit.unwrap()),
                    Err(_) => {
                        // If we can't get access quickly, yield to searches
                        tokio::task::yield_now().await;
                        Ok(self.ingest_semaphore.acquire().await.unwrap())
                    }
                }
            }
        }
    }

    async fn execute_with_priority<T, F>(&self, priority: OperationPriority, operation: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let _permit = self.get_priority_access(priority).await?;

        // For background operations, check if we should yield frequently
        if matches!(priority, OperationPriority::BackgroundIngest) {
            // Yield to any waiting search operations
            if self.search_semaphore.available_permits() < 10 {
                tokio::task::yield_now().await;
            }
        }

        let start_time = Instant::now();
        let result = {
            let conn = self.conn.lock().unwrap();
            operation(&conn)
        };

        // Log slow operations for debugging
        let elapsed = start_time.elapsed();
        if elapsed > Duration::from_millis(100) {
            println!("⚠️ Slow database operation took {:?} (priority: {:?})", elapsed, priority);
        }

        result
    }

    pub async fn insert_document(
        &self,
        title: &str,
        content: &str,
        url: Option<&str>,
        source: &str,
        embedding: Option<&[u8]>,
        is_dead: Option<bool>,
        priority: OperationPriority,
    ) -> Result<i64> {
        self.execute_with_priority(priority, |conn| {
            conn.execute(
                "INSERT INTO documents (title, content, url, source, embedding, is_dead) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![title, content, url, source, embedding, is_dead],
            )?;
            Ok(conn.last_insert_rowid())
        }).await
    }

    pub async fn get_document(&self, id: i64) -> Result<Option<Document>> {
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, content, url, source, created_at, embedding, is_dead
                 FROM documents WHERE id = ?1"
            )?;

            let doc = stmt.query_row(params![id], |row| {
                Ok(Document {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    url: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                    embedding: row.get(6)?,
                    is_dead: row.get(7)?,
                })
            });

            match doc {
                Ok(document) => Ok(Some(document)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(Box::new(e)),
            }
        }).await
    }

    pub async fn get_documents_batch(&self, ids: &[i64]) -> Result<Vec<Document>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            // Build the IN clause with placeholders
            let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT id, title, content, url, source, created_at, embedding, is_dead
                 FROM documents WHERE id IN ({})",
                placeholders
            );

            let mut stmt = conn.prepare(&query)?;

            // Convert ids to params
            let params: Vec<_> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

            let docs = stmt.query_map(&params[..], |row| {
                Ok(Document {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    url: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                    embedding: row.get(6)?,
                    is_dead: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

            Ok(docs)
        }).await
    }

    pub async fn search_documents(&self, query: &str, limit: i64) -> Result<Vec<Document>> {
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            let mut stmt = conn.prepare(
                "SELECT d.id, d.title, d.content, d.url, d.source, d.created_at, d.embedding, d.is_dead
                 FROM documents d
                 JOIN documents_fts fts ON d.id = fts.rowid
                 WHERE documents_fts MATCH ?1 AND (d.is_dead IS NULL OR d.is_dead = 0)
                 ORDER BY rank
                 LIMIT ?2"
            )?;

            let docs = stmt.query_map(params![query, limit], |row| {
                Ok(Document {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    url: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                    embedding: row.get(6)?,
                    is_dead: row.get(7)?,
                })
            })?;

            let mut results = Vec::new();
            for doc in docs {
                results.push(doc?);
            }
            Ok(results)
        }).await
    }

    // Legacy method - will be replaced by get_all_chunk_embeddings
    pub async fn get_all_embeddings(&self) -> Result<Vec<(i64, Vec<f32>)>> {
        // For now, return chunk embeddings but use document_id as the key
        // This maintains compatibility while we transition
        let chunk_embeddings = self.get_all_chunk_embeddings().await?;
        Ok(chunk_embeddings.into_iter()
            .map(|(_, doc_id, _, _, _, embedding)| (doc_id, embedding))
            .collect())
    }

    pub async fn insert_chunk_embedding(
        &self,
        document_id: i64,
        chunk_index: usize,
        chunk_start: usize,
        chunk_end: usize,
        embedding: &[u8],
        priority: OperationPriority,
    ) -> Result<i64> {
        self.execute_with_priority(priority, |conn| {
            conn.execute(
                "INSERT INTO embeddings (document_id, chunk_index, chunk_start, chunk_end, embedding)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![document_id, chunk_index as i64, chunk_start as i64, chunk_end as i64, embedding],
            )?;
            Ok(conn.last_insert_rowid())
        }).await
    }

    pub async fn get_all_chunk_embeddings(&self) -> Result<Vec<(i64, i64, usize, usize, usize, Vec<f32>)>> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, document_id, chunk_index, chunk_start, chunk_end, embedding FROM embeddings"
            )?;

            let rows = stmt.query_map([], |row| {
                let id: i64 = row.get(0)?;
                let document_id: i64 = row.get(1)?;
                let chunk_index: i64 = row.get(2)?;
                let chunk_start: i64 = row.get(3)?;
                let chunk_end: i64 = row.get(4)?;
                let embedding_bytes: Vec<u8> = row.get(5)?;
                let embedding: Vec<f32> = bincode::deserialize(&embedding_bytes)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok((id, document_id, chunk_index as usize, chunk_start as usize, chunk_end as usize, embedding))
            })?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        }).await
    }

    pub async fn get_chunk_embeddings_for_document(&self, document_id: i64) -> Result<Vec<(i64, usize, usize, usize, Vec<f32>)>> {
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, chunk_index, chunk_start, chunk_end, embedding
                 FROM embeddings WHERE document_id = ?1 ORDER BY chunk_index"
            )?;

            let rows = stmt.query_map(params![document_id], |row| {
                let id: i64 = row.get(0)?;
                let chunk_index: i64 = row.get(1)?;
                let chunk_start: i64 = row.get(2)?;
                let chunk_end: i64 = row.get(3)?;
                let embedding_bytes: Vec<u8> = row.get(4)?;
                let embedding: Vec<f32> = bincode::deserialize(&embedding_bytes)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok((id, chunk_index as usize, chunk_start as usize, chunk_end as usize, embedding))
            })?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        }).await
    }

    pub async fn delete_chunk_embeddings_for_document(&self, document_id: i64) -> Result<()> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            conn.execute(
                "DELETE FROM embeddings WHERE document_id = ?1",
                params![document_id],
            )?;
            Ok(())
        }).await
    }

    pub async fn url_exists(&self, url: &str, priority: OperationPriority) -> Result<bool> {
        self.execute_with_priority(priority, |conn| {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM documents WHERE url = ?1"
            )?;

            let count: i64 = stmt.query_row(params![url], |row| row.get(0))?;
            Ok(count > 0)
        }).await
    }

    pub async fn count_documents(&self, priority: OperationPriority) -> Result<i64> {
        self.execute_with_priority(priority, |conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))?;
            Ok(count)
        }).await
    }

    // Batch insert method for efficient bookmark ingestion
    pub async fn batch_insert_documents<'a>(
        &self,
        documents: &[(&'a str, &'a str, Option<&'a str>, &'a str, Option<&'a [u8]>, Option<bool>)],
    ) -> Result<Vec<i64>> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            let transaction = conn.unchecked_transaction()?;

            let mut ids = Vec::new();
            {
                let mut stmt = transaction.prepare(
                    "INSERT INTO documents (title, content, url, source, embedding, is_dead) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
                )?;

                for (title, content, url, source, embedding, is_dead) in documents {
                    stmt.execute(params![title, content, url, source, embedding, is_dead])?;
                    ids.push(transaction.last_insert_rowid());

                    // Yield periodically during batch operations
                    if ids.len() % 10 == 0 {
                        std::thread::yield_now();
                    }
                }
            } // stmt is dropped here

            transaction.commit()?;
            Ok(ids)
        }).await
    }

    pub async fn mark_url_as_dead(&self, url: &str) -> Result<()> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            conn.execute(
                "UPDATE documents SET is_dead = 1 WHERE url = ?1",
                params![url],
            )?;
            Ok(())
        }).await
    }

    pub async fn get_live_documents_with_urls(&self) -> Result<Vec<Document>> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, content, url, source, created_at, embedding, is_dead
                 FROM documents
                 WHERE url IS NOT NULL AND (is_dead IS NULL OR is_dead = 0)"
            )?;

            let docs = stmt.query_map([], |row| {
                Ok(Document {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    url: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                    embedding: row.get(6)?,
                    is_dead: row.get(7)?,
                })
            })?;

            let mut results = Vec::new();
            for doc in docs {
                results.push(doc?);
            }
            Ok(results)
        }).await
    }

    pub async fn check_and_mark_dead_urls(&self) -> Result<u32> {
        let documents = self.get_live_documents_with_urls().await?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("LocalMind/1.0")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let mut marked_dead_count = 0;

        for doc in documents {
            if let Some(url) = &doc.url {
                match client.head(url).send().await {
                    Ok(response) => {
                        if response.status() == reqwest::StatusCode::NOT_FOUND {
                            println!("🚫 Marking {} as dead (404)", url);
                            self.mark_url_as_dead(url).await?;
                            marked_dead_count += 1;
                        }
                    }
                    Err(e) => {
                        println!("⚠️ Error checking {}: {}", url, e);
                        // Don't mark as dead for network errors, only for explicit 404s
                    }
                }

                // Small delay to avoid overwhelming servers
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        Ok(marked_dead_count)
    }

    pub async fn get_all_documents(&self) -> Result<Vec<Document>> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, content, url, source, created_at, embedding, is_dead
                 FROM documents
                 WHERE is_dead IS NULL OR is_dead = 0
                 ORDER BY id"
            )?;

            let docs = stmt.query_map([], |row| {
                Ok(Document {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    url: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                    embedding: row.get(6)?,
                    is_dead: row.get(7)?,
                })
            })?;

            let mut results = Vec::new();
            for doc in docs {
                results.push(doc?);
            }
            Ok(results)
        }).await
    }

    pub async fn update_chunk_embedding(
        &self,
        embedding_id: i64,
        embedding_bytes: &[u8],
        priority: OperationPriority,
    ) -> Result<()> {
        self.execute_with_priority(priority, move |conn| {
            conn.execute(
                "UPDATE embeddings SET embedding = ?1 WHERE id = ?2",
                params![embedding_bytes, embedding_id],
            )?;
            Ok(())
        }).await
    }
}
