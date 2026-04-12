use crate::Result;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, SemaphorePermit};

/// Normalize a URL for deduplication.
/// Strips fragments (#...) and Google Docs query params (tab=, etc.)
/// so that the same document isn't stored multiple times.
pub fn normalize_url(url: &str) -> String {
    // Strip fragment
    let without_fragment = url.split('#').next().unwrap_or(url);

    // For Google Docs, strip query params entirely - the doc ID is the identity
    if without_fragment.contains("docs.google.com/document/") {
        without_fragment
            .split('?')
            .next()
            .unwrap_or(without_fragment)
            .to_string()
    } else {
        without_fragment.to_string()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OperationPriority {
    UserSearch,       // Highest priority - immediate access
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
    pub needs_auth: Option<bool>,
    pub profile: Option<String>,
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
        let _permit = self
            .get_priority_access(OperationPriority::UserSearch)
            .await?;
        let conn = self.conn.lock().unwrap();

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

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

        // Add needs_auth column if it doesn't exist (migration)
        let _ = conn.execute(
            "ALTER TABLE documents ADD COLUMN needs_auth BOOLEAN DEFAULT 0",
            [],
        );

        // Add profile column if it doesn't exist (migration)
        let _ = conn.execute("ALTER TABLE documents ADD COLUMN profile TEXT", []);

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
        // Simplified: removed chunk_index (can derive order from chunk_start)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id INTEGER NOT NULL,
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

        // Create config table for storing key-value settings
        conn.execute(
            "CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Create watched_folders table for folder-watch-ingest feature
        conn.execute(
            "CREATE TABLE IF NOT EXISTS watched_folders (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                path        TEXT UNIQUE NOT NULL,
                status      TEXT NOT NULL DEFAULT 'active',
                created_at  TEXT NOT NULL
            )",
            [],
        )?;

        // Create watched_files table tracking per-file ingest state
        conn.execute(
            "CREATE TABLE IF NOT EXISTS watched_files (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_id     INTEGER NOT NULL
                                REFERENCES watched_folders(id) ON DELETE CASCADE,
                file_path     TEXT UNIQUE NOT NULL,
                modified_at   INTEGER NOT NULL DEFAULT 0,
                document_id   INTEGER,
                ingest_status TEXT NOT NULL DEFAULT 'pending'
            )",
            [],
        )?;

        Ok(())
    }

    async fn get_priority_access(
        &self,
        priority: OperationPriority,
    ) -> Result<SemaphorePermit<'_>> {
        match priority {
            OperationPriority::UserSearch => {
                // User searches get immediate access
                Ok(self.search_semaphore.acquire().await.unwrap())
            }
            OperationPriority::BackgroundIngest => {
                // Background ingests wait and can be interrupted
                // Try to acquire with timeout to avoid blocking searches
                match tokio::time::timeout(
                    Duration::from_millis(100),
                    self.ingest_semaphore.acquire(),
                )
                .await
                {
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

    async fn execute_with_priority<T, F>(
        &self,
        priority: OperationPriority,
        operation: F,
    ) -> Result<T>
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
            println!(
                "⚠️ Slow database operation took {:?} (priority: {:?})",
                elapsed, priority
            );
        }

        result
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_document(
        &self,
        title: &str,
        content: &str,
        url: Option<&str>,
        source: &str,
        embedding: Option<&[u8]>,
        is_dead: Option<bool>,
        priority: OperationPriority,
        profile: Option<&str>,
    ) -> Result<i64> {
        let normalized_url = url.map(normalize_url);
        let url_ref = normalized_url.as_deref();
        self.execute_with_priority(priority, |conn| {
            conn.execute(
                "INSERT INTO documents (title, content, url, source, embedding, is_dead, profile) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![title, content, url_ref, source, embedding, is_dead, profile],
            )?;
            Ok(conn.last_insert_rowid())
        }).await
    }

    pub async fn get_document(&self, id: i64) -> Result<Option<Document>> {
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, content, url, source, created_at, embedding, is_dead, needs_auth, profile
                 FROM documents WHERE id = ?1",
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
                    needs_auth: row.get(8)?,
                    profile: row.get(9)?,
                })
            });

            match doc {
                Ok(document) => Ok(Some(document)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(Box::new(e)),
            }
        })
        .await
    }

    /// Get most recently added documents for home screen display
    ///
    /// Returns up to `limit` documents ordered by creation date (newest first).
    /// Excludes dead/broken bookmarks.
    pub async fn get_recent_documents(&self, limit: usize) -> Result<Vec<Document>> {
        self.get_recent_documents_filtered(limit, None).await
    }

    /// Like `get_recent_documents` but optionally filtered to a specific Chrome profile.
    pub async fn get_recent_documents_filtered(
        &self,
        limit: usize,
        profile: Option<String>,
    ) -> Result<Vec<Document>> {
        self.execute_with_priority(OperationPriority::UserSearch, move |conn| {
            let (sql, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) =
                if let Some(ref p) = profile {
                    (
                    "SELECT id, title, content, url, source, created_at, embedding, is_dead, needs_auth, profile
                     FROM documents
                     WHERE (is_dead = 0 OR is_dead IS NULL) AND profile = ?1
                     ORDER BY created_at DESC
                     LIMIT ?2".to_string(),
                    vec![Box::new(p.clone()), Box::new(limit as i64)],
                )
                } else {
                    (
                    "SELECT id, title, content, url, source, created_at, embedding, is_dead, needs_auth, profile
                     FROM documents
                     WHERE is_dead = 0 OR is_dead IS NULL
                     ORDER BY created_at DESC
                     LIMIT ?1".to_string(),
                    vec![Box::new(limit as i64)],
                )
                };

            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();

            let docs = stmt
                .query_map(&param_refs[..], |row| {
                    Ok(Document {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        content: row.get(2)?,
                        url: row.get(3)?,
                        source: row.get(4)?,
                        created_at: row.get(5)?,
                        embedding: row.get(6)?,
                        is_dead: row.get(7)?,
                        needs_auth: row.get(8)?,
                        profile: row.get(9)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            Ok(docs)
        })
        .await
    }

    pub async fn get_documents_batch(&self, ids: &[i64]) -> Result<Vec<Document>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            // Build the IN clause with placeholders
            let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT id, title, content, url, source, created_at, embedding, is_dead, needs_auth, profile
                 FROM documents WHERE id IN ({})",
                placeholders
            );

            let mut stmt = conn.prepare(&query)?;

            // Convert ids to params
            let params: Vec<_> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

            let docs = stmt
                .query_map(&params[..], |row| {
                    Ok(Document {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        content: row.get(2)?,
                        url: row.get(3)?,
                        source: row.get(4)?,
                        created_at: row.get(5)?,
                        embedding: row.get(6)?,
                        is_dead: row.get(7)?,
                        needs_auth: row.get(8)?,
                        profile: row.get(9)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            Ok(docs)
        })
        .await
    }

    pub async fn search_documents(&self, query: &str, limit: i64) -> Result<Vec<Document>> {
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            let mut stmt = conn.prepare(
                "SELECT d.id, d.title, d.content, d.url, d.source, d.created_at, d.embedding, d.is_dead, d.needs_auth, d.profile
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
                    needs_auth: row.get(8)?,
                    profile: row.get(9)?,
                })
            })?;

            let mut results = Vec::new();
            for doc in docs {
                results.push(doc?);
            }
            Ok(results)
        }).await
    }

    /// FTS5 search returning each document alongside its positive BM25 score (-rank).
    /// Higher score = better match. Results are ordered best-first.
    pub async fn search_documents_scored(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(Document, f64)>> {
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            let mut stmt = conn.prepare(
                "SELECT d.id, d.title, d.content, d.url, d.source, d.created_at, d.embedding,
                        d.is_dead, d.needs_auth, d.profile, -fts.rank AS bm25_score
                 FROM documents d
                 JOIN documents_fts fts ON d.id = fts.rowid
                 WHERE documents_fts MATCH ?1 AND (d.is_dead IS NULL OR d.is_dead = 0)
                 ORDER BY rank
                 LIMIT ?2",
            )?;

            let rows = stmt.query_map(params![query, limit], |row| {
                let doc = Document {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    url: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                    embedding: row.get(6)?,
                    is_dead: row.get(7)?,
                    needs_auth: row.get(8)?,
                    profile: row.get(9)?,
                };
                let bm25_score: f64 = row.get(10)?;
                Ok((doc, bm25_score))
            })?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
        .await
    }

    pub async fn insert_chunk_embedding(
        &self,
        document_id: i64,
        chunk_start: usize,
        chunk_end: usize,
        embedding: &[u8],
        priority: OperationPriority,
    ) -> Result<i64> {
        self.execute_with_priority(priority, |conn| {
            conn.execute(
                "INSERT INTO embeddings (document_id, chunk_start, chunk_end, embedding)
                 VALUES (?1, ?2, ?3, ?4)",
                params![document_id, chunk_start as i64, chunk_end as i64, embedding],
            )?;
            Ok(conn.last_insert_rowid())
        })
        .await
    }

    pub async fn get_all_chunk_embeddings(
        &self,
    ) -> Result<Vec<(i64, i64, usize, usize, Vec<f32>)>> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, document_id, chunk_start, chunk_end, embedding FROM embeddings ORDER BY document_id, chunk_start"
            )?;

            let rows = stmt.query_map([], |row| {
                let id: i64 = row.get(0)?;
                let document_id: i64 = row.get(1)?;
                let chunk_start: i64 = row.get(2)?;
                let chunk_end: i64 = row.get(3)?;
                let embedding_bytes: Vec<u8> = row.get(4)?;
                let embedding: Vec<f32> = bincode::deserialize(&embedding_bytes)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok((id, document_id, chunk_start as usize, chunk_end as usize, embedding))
            })?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        }).await
    }

    pub async fn get_chunk_embeddings_for_document(
        &self,
        document_id: i64,
    ) -> Result<Vec<(i64, usize, usize, Vec<f32>)>> {
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, chunk_start, chunk_end, embedding
                 FROM embeddings WHERE document_id = ?1 ORDER BY chunk_start",
            )?;

            let rows = stmt.query_map(params![document_id], |row| {
                let id: i64 = row.get(0)?;
                let chunk_start: i64 = row.get(1)?;
                let chunk_end: i64 = row.get(2)?;
                let embedding_bytes: Vec<u8> = row.get(3)?;
                let embedding: Vec<f32> = bincode::deserialize(&embedding_bytes)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok((id, chunk_start as usize, chunk_end as usize, embedding))
            })?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
        .await
    }

    pub async fn delete_all_embeddings(&self) -> Result<()> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            // Drop and recreate the embeddings table to ensure correct schema
            // (handles migration from old schema that had chunk_index column)
            conn.execute("DROP TABLE IF EXISTS embeddings", [])?;
            conn.execute(
                "CREATE TABLE embeddings (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    document_id INTEGER NOT NULL,
                    chunk_start INTEGER NOT NULL,
                    chunk_end INTEGER NOT NULL,
                    embedding BLOB NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (document_id) REFERENCES documents (id) ON DELETE CASCADE
                )",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_embeddings_document_id ON embeddings(document_id)",
                [],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn url_exists(&self, url: &str, priority: OperationPriority) -> Result<bool> {
        let normalized = normalize_url(url);
        self.execute_with_priority(priority, move |conn| {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM documents WHERE url = ?1")?;
            let count: i64 = stmt.query_row(params![normalized], |row| row.get(0))?;
            Ok(count > 0)
        })
        .await
    }

    pub async fn count_documents(&self, priority: OperationPriority) -> Result<i64> {
        self.execute_with_priority(priority, |conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))?;
            Ok(count)
        })
        .await
    }

    // Batch insert method for efficient bookmark ingestion
    #[allow(clippy::type_complexity)]
    pub async fn batch_insert_documents<'a>(
        &self,
        documents: &[(
            &'a str,
            &'a str,
            Option<&'a str>,
            &'a str,
            Option<&'a [u8]>,
            Option<bool>,
        )],
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
        let normalized = normalize_url(url);
        self.execute_with_priority(OperationPriority::BackgroundIngest, move |conn| {
            conn.execute(
                "UPDATE documents SET is_dead = 1 WHERE url = ?1",
                params![normalized],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn mark_url_as_needs_auth(&self, url: &str) -> Result<()> {
        let normalized = normalize_url(url);
        self.execute_with_priority(OperationPriority::BackgroundIngest, move |conn| {
            conn.execute(
                "UPDATE documents SET needs_auth = 1 WHERE url = ?1",
                params![normalized],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn get_document_by_url(&self, url: &str) -> Result<Option<Document>> {
        let normalized = normalize_url(url);
        self.execute_with_priority(OperationPriority::UserSearch, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, content, url, source, created_at, embedding, is_dead, needs_auth, profile
                 FROM documents WHERE url = ?1 LIMIT 1",
            )?;

            match stmt.query_row(params![normalized], |row| {
                Ok(Document {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    url: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                    embedding: row.get(6)?,
                    is_dead: row.get(7)?,
                    needs_auth: row.get(8)?,
                    profile: row.get(9)?,
                })
            }) {
                Ok(doc) => Ok(Some(doc)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(Box::new(e)),
            }
        })
        .await
    }

    pub async fn update_document_content(
        &self,
        doc_id: i64,
        title: &str,
        content: &str,
    ) -> Result<()> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            conn.execute(
                "UPDATE documents SET title = ?1, content = ?2, is_dead = 0, needs_auth = 0
                 WHERE id = ?3",
                params![title, content, doc_id],
            )?;
            // Update FTS index
            conn.execute(
                "UPDATE documents_fts SET title = ?1, content = ?2 WHERE rowid = ?3",
                params![title, content, doc_id],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn delete_embeddings_for_document(&self, doc_id: i64) -> Result<()> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            conn.execute(
                "DELETE FROM embeddings WHERE document_id = ?1",
                params![doc_id],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn get_live_documents_with_urls(&self) -> Result<Vec<Document>> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, content, url, source, created_at, embedding, is_dead, needs_auth, profile
                 FROM documents
                 WHERE url IS NOT NULL AND (is_dead IS NULL OR is_dead = 0)",
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
                    needs_auth: row.get(8)?,
                    profile: row.get(9)?,
                })
            })?;

            let mut results = Vec::new();
            for doc in docs {
                results.push(doc?);
            }
            Ok(results)
        })
        .await
    }

    pub async fn check_and_mark_dead_urls(&self) -> Result<u32> {
        let documents = self.get_live_documents_with_urls().await?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("LocalMind/1.0")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let mut marked_count = 0;

        for doc in documents {
            if let Some(url) = &doc.url {
                match client.head(url).send().await {
                    Ok(response) => {
                        let status = response.status();
                        if status == reqwest::StatusCode::NOT_FOUND {
                            println!("Marking {} as dead (404)", url);
                            self.mark_url_as_dead(url).await?;
                            marked_count += 1;
                        } else if status == reqwest::StatusCode::UNAUTHORIZED
                            || status == reqwest::StatusCode::FORBIDDEN
                        {
                            println!("Marking {} as needs auth ({})", url, status);
                            self.mark_url_as_needs_auth(url).await?;
                            marked_count += 1;
                        }
                    }
                    Err(e) => {
                        println!("Warning: error checking {}: {}", url, e);
                        // Don't mark as dead for network errors, only for explicit status codes
                    }
                }

                // Small delay to avoid overwhelming servers
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        Ok(marked_count)
    }

    pub async fn get_all_documents(&self) -> Result<Vec<Document>> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, content, url, source, created_at, embedding, is_dead, needs_auth, profile
                 FROM documents
                 WHERE is_dead IS NULL OR is_dead = 0
                 ORDER BY id",
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
                    needs_auth: row.get(8)?,
                    profile: row.get(9)?,
                })
            })?;

            let mut results = Vec::new();
            for doc in docs {
                results.push(doc?);
            }
            Ok(results)
        })
        .await
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
        })
        .await
    }

    pub async fn set_config(&self, key: &str, value: &str) -> Result<()> {
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO config (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
                params![key, value],
            )?;
            Ok(())
        }).await
    }

    pub async fn get_config(&self, key: &str) -> Result<Option<String>> {
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            let mut stmt = conn.prepare("SELECT value FROM config WHERE key = ?1")?;

            match stmt.query_row(params![key], |row| row.get(0)) {
                Ok(value) => Ok(Some(value)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            }
        })
        .await
    }

    pub async fn get_embedding_model(&self) -> Result<Option<String>> {
        self.get_config("embedding_model").await
    }

    pub async fn set_embedding_model(&self, model: &str) -> Result<()> {
        self.set_config("embedding_model", model).await
    }

    pub async fn get_embedding_url(&self) -> Result<Option<String>> {
        self.get_config("embedding_url").await
    }

    pub async fn set_embedding_url(&self, url: &str) -> Result<()> {
        self.set_config("embedding_url", url).await
    }

    pub async fn get_excluded_folders(&self) -> Result<Vec<String>> {
        match self.get_config("bookmark_exclude_folders").await? {
            Some(json_str) => {
                let folders: Vec<String> = serde_json::from_str(&json_str)
                    .map_err(|e| format!("Failed to parse excluded folders: {}", e))?;
                Ok(folders)
            }
            None => Ok(Vec::new()),
        }
    }

    pub async fn set_excluded_folders(&self, folders: &[String]) -> Result<()> {
        let json_str = serde_json::to_string(folders)
            .map_err(|e| format!("Failed to serialize excluded folders: {}", e))?;
        self.set_config("bookmark_exclude_folders", &json_str).await
    }

    pub async fn get_excluded_domains(&self) -> Result<Vec<String>> {
        match self.get_config("bookmark_exclude_domains").await? {
            Some(json_str) => {
                let domains: Vec<String> = serde_json::from_str(&json_str)
                    .map_err(|e| format!("Failed to parse excluded domains: {}", e))?;
                Ok(domains)
            }
            None => Ok(Vec::new()),
        }
    }

    pub async fn set_excluded_domains(&self, domains: &[String]) -> Result<()> {
        let json_str = serde_json::to_string(domains)
            .map_err(|e| format!("Failed to serialize excluded domains: {}", e))?;
        self.set_config("bookmark_exclude_domains", &json_str).await
    }

    pub async fn delete_bookmarks_by_url_pattern(&self, pattern: &str) -> Result<usize> {
        use crate::bookmark_exclusion::ExclusionRules;

        let rules = ExclusionRules::new(vec![], vec![pattern.to_string()]);
        let documents = self.get_live_documents_with_urls().await?;

        let mut deleted_count = 0;
        for doc in documents {
            if let Some(url) = &doc.url {
                if rules.is_url_excluded(url) {
                    self.execute_with_priority(OperationPriority::BackgroundIngest, |conn| {
                        conn.execute("DELETE FROM documents WHERE id = ?1", params![doc.id])?;
                        Ok(())
                    })
                    .await?;
                    deleted_count += 1;
                }
            }
        }

        Ok(deleted_count)
    }

    pub async fn delete_bookmarks_by_folder(&self, folder_id: &str) -> Result<usize> {
        use crate::bookmark::BookmarkMonitor;

        // Get bookmark monitor to parse Chrome bookmarks
        let monitor = BookmarkMonitor::new()?.0;

        // Get all bookmarks from Chrome
        let all_bookmarks = monitor.get_bookmark_roots()?;

        // Find the specific folder and collect all URLs in it
        let mut urls_to_delete = Vec::new();

        fn find_and_collect_urls(
            item: &crate::bookmark::BookmarkItem,
            target_folder_id: &str,
            urls: &mut Vec<String>,
        ) -> bool {
            // Check if this is the target folder
            if item.id == target_folder_id {
                println!("Found folder '{}' (ID: {})", item.name, item.id);
                // Collect all URLs in this folder and subfolders
                collect_all_urls(item, urls);
                return true;
            }

            // Recursively search in children
            if let Some(children) = &item.children {
                for child in children {
                    if find_and_collect_urls(child, target_folder_id, urls) {
                        return true;
                    }
                }
            }

            false
        }

        fn collect_all_urls(item: &crate::bookmark::BookmarkItem, urls: &mut Vec<String>) {
            if let Some(url) = &item.url {
                if !url.is_empty() {
                    urls.push(url.clone());
                }
            }
            if let Some(children) = &item.children {
                for child in children {
                    collect_all_urls(child, urls);
                }
            }
        }

        // Search through all bookmark roots
        for item in &all_bookmarks {
            if find_and_collect_urls(item, folder_id, &mut urls_to_delete) {
                break;
            }
        }

        println!(
            "Found {} URLs in Chrome for folder {}",
            urls_to_delete.len(),
            folder_id
        );

        // Check how many actually exist in the database
        let mut exists_count = 0;
        for url in &urls_to_delete {
            let count: i64 = self.execute_with_priority(OperationPriority::BackgroundIngest, {
                let url = url.clone();
                move |conn| {
                    let count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM documents WHERE url = ?1 AND source = 'chrome_bookmark'",
                        params![&url],
                        |row| row.get(0)
                    )?;
                    Ok(count)
                }
            }).await?;

            if count > 0 {
                exists_count += 1;
            }
        }

        println!(
            "Of those, {} URLs exist in database for folder {}",
            exists_count, folder_id
        );

        // Delete each URL from the database
        let mut deleted_count = 0;
        for url in urls_to_delete {
            let result = self
                .execute_with_priority(OperationPriority::BackgroundIngest, move |conn| {
                    conn.execute(
                        "DELETE FROM documents WHERE url = ?1 AND source = 'chrome_bookmark'",
                        params![&url],
                    )?;
                    Ok(())
                })
                .await;

            if result.is_ok() {
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }

    // -----------------------------------------------------------------------
    // Folder-watch CRUD (T009-T016, T046)
    // -----------------------------------------------------------------------

    /// Create an in-memory database suitable for unit tests.
    ///
    /// Same schema as the live database; schema is initialized synchronously
    /// within a temporary tokio runtime so callers don't need async.
    #[cfg(test)]
    pub fn new_in_memory_sync() -> Self {
        let conn = Connection::open_in_memory().expect("in-memory SQLite");
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            search_semaphore: Arc::new(Semaphore::new(10)),
            ingest_semaphore: Arc::new(Semaphore::new(1)),
        };
        // Run schema init synchronously via a temporary runtime
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(db.init_schema()).expect("init_schema");
        db
    }

    /// Insert a watched folder, returning its new row ID.
    ///
    /// Returns `Err` (rusqlite unique constraint) if the path already exists.
    pub async fn add_watched_folder(&self, path: &std::path::Path) -> Result<i64> {
        let path_str = path.to_string_lossy().to_string();
        let created_at = chrono_utc_now();
        self.execute_with_priority(OperationPriority::BackgroundIngest, move |conn| {
            conn.execute(
                "INSERT INTO watched_folders (path, status, created_at)
                 VALUES (?1, 'active', ?2)",
                params![path_str, created_at],
            )?;
            Ok(conn.last_insert_rowid())
        })
        .await
    }

    /// Return all watched folders ordered by creation time.
    pub async fn get_watched_folders(&self) -> Result<Vec<crate::folder_watcher::WatchedFolder>> {
        use crate::folder_watcher::{FolderStatus, WatchedFolder};
        self.execute_with_priority(OperationPriority::UserSearch, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, status, created_at FROM watched_folders ORDER BY created_at",
            )?;
            let rows = stmt.query_map([], |row| {
                let id: i64 = row.get(0)?;
                let path: String = row.get(1)?;
                let status_str: String = row.get(2)?;
                let created_at: String = row.get(3)?;
                Ok(WatchedFolder {
                    id,
                    path: std::path::PathBuf::from(path),
                    status: FolderStatus::from_db_str(&status_str),
                    created_at,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
        .await
    }

    /// Insert or update a watched-file record.
    ///
    /// Uses `INSERT OR REPLACE` so callers don't need to distinguish between
    /// first-seen and update-on-re-ingest cases.
    pub async fn upsert_watched_file(
        &self,
        folder_id: i64,
        file_path: &std::path::Path,
        modified_at: i64,
        document_id: Option<i64>,
        status: &crate::folder_watcher::IngestStatus,
    ) -> Result<i64> {
        let path_str = file_path.to_string_lossy().to_string();
        let status_str = status.as_db_str().to_string();
        self.execute_with_priority(OperationPriority::BackgroundIngest, move |conn| {
            conn.execute(
                "INSERT INTO watched_files
                     (folder_id, file_path, modified_at, document_id, ingest_status)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(file_path) DO UPDATE SET
                     modified_at    = excluded.modified_at,
                     document_id    = excluded.document_id,
                     ingest_status  = excluded.ingest_status",
                params![folder_id, path_str, modified_at, document_id, status_str],
            )?;
            Ok(conn.last_insert_rowid())
        })
        .await
    }

    /// Look up a single watched-file record by its filesystem path.
    pub async fn get_watched_file_by_path(
        &self,
        path: &std::path::Path,
    ) -> Result<Option<crate::folder_watcher::WatchedFile>> {
        use crate::folder_watcher::{IngestStatus, WatchedFile};
        let path_str = path.to_string_lossy().to_string();
        self.execute_with_priority(OperationPriority::UserSearch, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, folder_id, file_path, modified_at, document_id, ingest_status
                 FROM watched_files WHERE file_path = ?1",
            )?;
            match stmt.query_row(params![path_str], |row| {
                let path_val: String = row.get(2)?;
                let status_str: String = row.get(5)?;
                Ok(WatchedFile {
                    id: row.get(0)?,
                    folder_id: row.get(1)?,
                    file_path: std::path::PathBuf::from(path_val),
                    modified_at: row.get(3)?,
                    document_id: row.get(4)?,
                    ingest_status: IngestStatus::from_db_str(&status_str),
                })
            }) {
                Ok(wf) => Ok(Some(wf)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(Box::new(e)),
            }
        })
        .await
    }

    /// Return all watched-file records belonging to a folder.
    pub async fn get_watched_files_for_folder(
        &self,
        folder_id: i64,
    ) -> Result<Vec<crate::folder_watcher::WatchedFile>> {
        use crate::folder_watcher::{IngestStatus, WatchedFile};
        self.execute_with_priority(OperationPriority::UserSearch, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, folder_id, file_path, modified_at, document_id, ingest_status
                 FROM watched_files WHERE folder_id = ?1 ORDER BY file_path",
            )?;
            let rows = stmt.query_map(params![folder_id], |row| {
                let path_val: String = row.get(2)?;
                let status_str: String = row.get(5)?;
                Ok(WatchedFile {
                    id: row.get(0)?,
                    folder_id: row.get(1)?,
                    file_path: std::path::PathBuf::from(path_val),
                    modified_at: row.get(3)?,
                    document_id: row.get(4)?,
                    ingest_status: IngestStatus::from_db_str(&status_str),
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
        .await
    }

    /// Delete a watched folder row (CASCADE removes all its watched_files).
    pub async fn delete_watched_folder(&self, id: i64) -> Result<()> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, move |conn| {
            conn.execute("DELETE FROM watched_folders WHERE id = ?1", params![id])?;
            Ok(())
        })
        .await
    }

    /// Delete a single document and all its embeddings and FTS entries.
    ///
    /// Foreign-key CASCADE on `embeddings` handles chunk rows automatically.
    /// The FTS `rowid` matches the document `id`.
    pub async fn delete_document(&self, document_id: i64) -> Result<()> {
        self.execute_with_priority(OperationPriority::BackgroundIngest, move |conn| {
            // FTS must be deleted manually (virtual table, no FK cascade)
            conn.execute(
                "DELETE FROM documents_fts WHERE rowid = ?1",
                params![document_id],
            )?;
            // Embeddings are CASCADE-deleted by FK when document is deleted
            conn.execute("DELETE FROM documents WHERE id = ?1", params![document_id])?;
            Ok(())
        })
        .await
    }

    /// Update the status column of a watched folder.
    pub async fn update_watched_folder_status(
        &self,
        id: i64,
        status: &crate::folder_watcher::FolderStatus,
    ) -> Result<()> {
        let status_str = status.as_db_str().to_string();
        self.execute_with_priority(OperationPriority::BackgroundIngest, move |conn| {
            conn.execute(
                "UPDATE watched_folders SET status = ?1 WHERE id = ?2",
                params![status_str, id],
            )?;
            Ok(())
        })
        .await
    }

    /// Delete all documents whose `source` column equals `folder_path`, returning
    /// the deleted document IDs so the caller can evict them from the VectorStore.
    ///
    /// Call this BEFORE `delete_watched_folder` so the document IDs are still
    /// available (watched_files rows are CASCADE-deleted with the folder).
    pub async fn delete_documents_by_source(
        &self,
        folder_path: &std::path::Path,
    ) -> Result<Vec<i64>> {
        let source = folder_path.to_string_lossy().to_string();
        self.execute_with_priority(OperationPriority::BackgroundIngest, move |conn| {
            // Collect IDs first so we can return them for VectorStore eviction
            let mut stmt = conn.prepare("SELECT id FROM documents WHERE source = ?1")?;
            let ids: Vec<i64> = stmt
                .query_map(params![source], |row| row.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            for &doc_id in &ids {
                conn.execute(
                    "DELETE FROM documents_fts WHERE rowid = ?1",
                    params![doc_id],
                )?;
                conn.execute("DELETE FROM documents WHERE id = ?1", params![doc_id])?;
            }
            Ok(ids)
        })
        .await
    }
}

/// Return current UTC timestamp as an ISO 8601 string.
fn chrono_utc_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Store as unix timestamp string — no chrono dependency needed
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_localmind.db");

        let conn = Connection::open(&db_path).unwrap();
        let db = Database {
            conn: Arc::new(Mutex::new(conn)),
            search_semaphore: Arc::new(Semaphore::new(10)),
            ingest_semaphore: Arc::new(Semaphore::new(1)),
        };

        db.init_schema().await.unwrap();
        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_excluded_folders_config() {
        let (db, _temp) = create_test_db().await;

        let folders = db.get_excluded_folders().await.unwrap();
        assert_eq!(
            folders.len(),
            0,
            "Initially should have no excluded folders"
        );

        let test_folders = vec![
            "folder_123".to_string(),
            "folder_456".to_string(),
            "folder_789".to_string(),
        ];
        db.set_excluded_folders(&test_folders).await.unwrap();

        let retrieved = db.get_excluded_folders().await.unwrap();
        assert_eq!(
            retrieved, test_folders,
            "Should retrieve same folders that were set"
        );

        let updated_folders = vec!["folder_abc".to_string(), "folder_xyz".to_string()];
        db.set_excluded_folders(&updated_folders).await.unwrap();

        let retrieved = db.get_excluded_folders().await.unwrap();
        assert_eq!(
            retrieved, updated_folders,
            "Should update to new folder list"
        );
    }

    #[tokio::test]
    async fn test_excluded_domains_config() {
        let (db, _temp) = create_test_db().await;

        let domains = db.get_excluded_domains().await.unwrap();
        assert_eq!(
            domains.len(),
            0,
            "Initially should have no excluded domains"
        );

        let test_domains = vec![
            "*.internal.com".to_string(),
            "private.example.org".to_string(),
            "localhost:*".to_string(),
        ];
        db.set_excluded_domains(&test_domains).await.unwrap();

        let retrieved = db.get_excluded_domains().await.unwrap();
        assert_eq!(
            retrieved, test_domains,
            "Should retrieve same domains that were set"
        );

        let updated_domains = vec!["example.com".to_string()];
        db.set_excluded_domains(&updated_domains).await.unwrap();

        let retrieved = db.get_excluded_domains().await.unwrap();
        assert_eq!(
            retrieved, updated_domains,
            "Should update to new domain list"
        );
    }

    #[tokio::test]
    async fn test_delete_bookmarks_by_url_pattern() {
        let (db, _temp) = create_test_db().await;

        db.insert_document(
            "Internal Site",
            "Content from internal site",
            Some("https://foo.internal.com/page"),
            "chrome_bookmark",
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();

        db.insert_document(
            "Public Site",
            "Content from public site",
            Some("https://example.com/page"),
            "chrome_bookmark",
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();

        db.insert_document(
            "Another Internal Site",
            "More internal content",
            Some("https://bar.internal.com/page"),
            "chrome_bookmark",
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();

        let deleted = db
            .delete_bookmarks_by_url_pattern("*.internal.com")
            .await
            .unwrap();
        assert_eq!(
            deleted, 2,
            "Should delete 2 bookmarks matching *.internal.com"
        );

        let docs = db.get_live_documents_with_urls().await.unwrap();
        let urls: Vec<String> = docs.iter().filter_map(|d| d.url.clone()).collect();

        assert!(
            urls.contains(&"https://example.com/page".to_string()),
            "Public site should remain"
        );
        assert!(
            !urls.contains(&"https://foo.internal.com/page".to_string()),
            "foo.internal.com should be deleted"
        );
        assert!(
            !urls.contains(&"https://bar.internal.com/page".to_string()),
            "bar.internal.com should be deleted"
        );
    }

    #[tokio::test]
    async fn test_delete_bookmarks_by_url_pattern_wildcard_subdomain() {
        let (db, _temp) = create_test_db().await;

        db.insert_document(
            "Dev Site",
            "Dev content",
            Some("https://dev.mycompany.com/page"),
            "chrome_bookmark",
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();

        db.insert_document(
            "Staging Site",
            "Staging content",
            Some("https://staging.mycompany.com/page"),
            "chrome_bookmark",
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();

        db.insert_document(
            "External Site",
            "External content",
            Some("https://external.com/page"),
            "chrome_bookmark",
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();

        let deleted = db
            .delete_bookmarks_by_url_pattern("*.mycompany.com")
            .await
            .unwrap();
        assert_eq!(deleted, 2, "Should delete 2 mycompany.com bookmarks");

        let docs = db.get_live_documents_with_urls().await.unwrap();
        assert_eq!(docs.len(), 1, "Only 1 bookmark should remain");
        assert_eq!(docs[0].url.as_ref().unwrap(), "https://external.com/page");
    }

    #[tokio::test]
    async fn test_delete_bookmarks_by_url_pattern_exact_domain() {
        let (db, _temp) = create_test_db().await;

        db.insert_document(
            "Exact Match",
            "Content",
            Some("https://example.com/page"),
            "chrome_bookmark",
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();

        db.insert_document(
            "Subdomain",
            "Content",
            Some("https://sub.example.com/page"),
            "chrome_bookmark",
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();

        let deleted = db
            .delete_bookmarks_by_url_pattern("example.com")
            .await
            .unwrap();
        assert_eq!(deleted, 1, "Should delete only exact domain match");

        let docs = db.get_live_documents_with_urls().await.unwrap();
        assert_eq!(docs.len(), 1, "Subdomain should remain");
        assert_eq!(
            docs[0].url.as_ref().unwrap(),
            "https://sub.example.com/page"
        );
    }

    #[tokio::test]
    async fn test_excluded_folders_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_persistence.db");

        {
            let conn = Connection::open(&db_path).unwrap();
            let db = Database {
                conn: Arc::new(Mutex::new(conn)),
                search_semaphore: Arc::new(Semaphore::new(10)),
                ingest_semaphore: Arc::new(Semaphore::new(1)),
            };
            db.init_schema().await.unwrap();

            let folders = vec!["folder_1".to_string(), "folder_2".to_string()];
            db.set_excluded_folders(&folders).await.unwrap();
        }

        {
            let conn = Connection::open(&db_path).unwrap();
            let db = Database {
                conn: Arc::new(Mutex::new(conn)),
                search_semaphore: Arc::new(Semaphore::new(10)),
                ingest_semaphore: Arc::new(Semaphore::new(1)),
            };

            let retrieved = db.get_excluded_folders().await.unwrap();
            assert_eq!(
                retrieved.len(),
                2,
                "Folders should persist across database reopens"
            );
            assert_eq!(retrieved[0], "folder_1");
            assert_eq!(retrieved[1], "folder_2");
        }
    }

    #[tokio::test]
    async fn test_excluded_domains_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_persistence.db");

        {
            let conn = Connection::open(&db_path).unwrap();
            let db = Database {
                conn: Arc::new(Mutex::new(conn)),
                search_semaphore: Arc::new(Semaphore::new(10)),
                ingest_semaphore: Arc::new(Semaphore::new(1)),
            };
            db.init_schema().await.unwrap();

            let domains = vec!["*.internal.com".to_string(), "localhost".to_string()];
            db.set_excluded_domains(&domains).await.unwrap();
        }

        {
            let conn = Connection::open(&db_path).unwrap();
            let db = Database {
                conn: Arc::new(Mutex::new(conn)),
                search_semaphore: Arc::new(Semaphore::new(10)),
                ingest_semaphore: Arc::new(Semaphore::new(1)),
            };

            let retrieved = db.get_excluded_domains().await.unwrap();
            assert_eq!(
                retrieved.len(),
                2,
                "Domains should persist across database reopens"
            );
            assert_eq!(retrieved[0], "*.internal.com");
            assert_eq!(retrieved[1], "localhost");
        }
    }

    // -----------------------------------------------------------------------
    // T006: add_watched_folder / get_watched_folders
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn add_and_get_watched_folder() {
        let (db, _tmp) = create_test_db().await;
        let path = std::path::Path::new("/tmp/test_folder");

        let id = db.add_watched_folder(path).await.unwrap();
        assert!(id > 0);

        let folders = db.get_watched_folders().await.unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].path, path);
        assert_eq!(
            folders[0].status,
            crate::folder_watcher::FolderStatus::Active
        );
    }

    #[tokio::test]
    async fn add_duplicate_folder_fails() {
        let (db, _tmp) = create_test_db().await;
        let path = std::path::Path::new("/tmp/dup_folder");

        db.add_watched_folder(path).await.unwrap();
        let result = db.add_watched_folder(path).await;
        assert!(
            result.is_err(),
            "duplicate path must fail with UNIQUE constraint"
        );
    }

    // -----------------------------------------------------------------------
    // T007: upsert_watched_file / get_watched_file_by_path / get_watched_files_for_folder
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn upsert_and_get_watched_file() {
        use crate::folder_watcher::IngestStatus;
        let (db, _tmp) = create_test_db().await;
        let folder_id = db
            .add_watched_folder(std::path::Path::new("/tmp/f1"))
            .await
            .unwrap();
        let file_path = std::path::Path::new("/tmp/f1/note.txt");

        db.upsert_watched_file(folder_id, file_path, 1000, None, &IngestStatus::Pending)
            .await
            .unwrap();

        let wf = db
            .get_watched_file_by_path(file_path)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(wf.folder_id, folder_id);
        assert_eq!(wf.file_path, file_path);
        assert_eq!(wf.modified_at, 1000);
        assert_eq!(wf.ingest_status, IngestStatus::Pending);
    }

    #[tokio::test]
    async fn upsert_updates_existing_file() {
        use crate::folder_watcher::IngestStatus;
        let (db, _tmp) = create_test_db().await;
        let folder_id = db
            .add_watched_folder(std::path::Path::new("/tmp/f2"))
            .await
            .unwrap();
        let file_path = std::path::Path::new("/tmp/f2/note.txt");

        db.upsert_watched_file(folder_id, file_path, 1000, None, &IngestStatus::Pending)
            .await
            .unwrap();
        db.upsert_watched_file(
            folder_id,
            file_path,
            2000,
            Some(42),
            &IngestStatus::Ingested,
        )
        .await
        .unwrap();

        let wf = db
            .get_watched_file_by_path(file_path)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(wf.modified_at, 2000);
        assert_eq!(wf.document_id, Some(42));
        assert_eq!(wf.ingest_status, IngestStatus::Ingested);
    }

    #[tokio::test]
    async fn get_watched_files_for_folder_returns_all() {
        use crate::folder_watcher::IngestStatus;
        let (db, _tmp) = create_test_db().await;
        let fid = db
            .add_watched_folder(std::path::Path::new("/tmp/f3"))
            .await
            .unwrap();

        db.upsert_watched_file(
            fid,
            std::path::Path::new("/tmp/f3/a.txt"),
            100,
            None,
            &IngestStatus::Pending,
        )
        .await
        .unwrap();
        db.upsert_watched_file(
            fid,
            std::path::Path::new("/tmp/f3/b.md"),
            200,
            None,
            &IngestStatus::Pending,
        )
        .await
        .unwrap();

        let files = db.get_watched_files_for_folder(fid).await.unwrap();
        assert_eq!(files.len(), 2);
    }

    // -----------------------------------------------------------------------
    // T008: delete_watched_folder CASCADE + delete_document
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn delete_watched_folder_cascades_files() {
        use crate::folder_watcher::IngestStatus;
        let (db, _tmp) = create_test_db().await;
        let fid = db
            .add_watched_folder(std::path::Path::new("/tmp/del_f"))
            .await
            .unwrap();

        db.upsert_watched_file(
            fid,
            std::path::Path::new("/tmp/del_f/a.txt"),
            1,
            None,
            &IngestStatus::Pending,
        )
        .await
        .unwrap();

        // Verify file exists
        assert!(db
            .get_watched_file_by_path(std::path::Path::new("/tmp/del_f/a.txt"))
            .await
            .unwrap()
            .is_some());

        db.delete_watched_folder(fid).await.unwrap();

        // CASCADE should have removed the file row
        assert!(db
            .get_watched_file_by_path(std::path::Path::new("/tmp/del_f/a.txt"))
            .await
            .unwrap()
            .is_none());
        assert!(db.get_watched_folders().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_document_removes_from_all_tables() {
        let (db, _tmp) = create_test_db().await;

        let doc_id = db
            .insert_document(
                "Test",
                "content",
                None,
                "source",
                None,
                None,
                OperationPriority::BackgroundIngest,
                None,
            )
            .await
            .unwrap();

        // Verify document exists
        assert!(db.get_document(doc_id).await.unwrap().is_some());

        db.delete_document(doc_id).await.unwrap();

        assert!(db.get_document(doc_id).await.unwrap().is_none());
    }

    // -----------------------------------------------------------------------
    // T045: delete_documents_by_source
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn delete_documents_by_source_removes_matching_docs() {
        let (db, _tmp) = create_test_db().await;
        let folder_path = std::path::Path::new("/tmp/source_folder");

        db.insert_document(
            "Doc1",
            "content1",
            None,
            &folder_path.to_string_lossy(),
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();
        db.insert_document(
            "Doc2",
            "content2",
            None,
            &folder_path.to_string_lossy(),
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();
        db.insert_document(
            "Other",
            "other",
            None,
            "other_source",
            None,
            None,
            OperationPriority::BackgroundIngest,
            None,
        )
        .await
        .unwrap();

        let deleted_ids = db.delete_documents_by_source(folder_path).await.unwrap();
        assert_eq!(deleted_ids.len(), 2);

        // The "Other" document must still exist
        let remaining = db.get_all_documents().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].source, "other_source");
    }
}
