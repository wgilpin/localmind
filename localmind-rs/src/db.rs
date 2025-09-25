use crate::Result;
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

pub struct Document {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub url: Option<String>,
    pub source: String,
    pub created_at: String,
    pub embedding: Option<Vec<u8>>,
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
            conn: Arc::new(Mutex::new(conn))
        };
        
        db.init_schema().await?;
        Ok(db)
    }

    async fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                url TEXT,
                source TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                embedding BLOB
            )",
            [],
        )?;

        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                title, content, content='documents', content_rowid='id'
            )",
            [],
        )?;

        // Create triggers to keep FTS in sync
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
                INSERT INTO documents_fts(rowid, title, content) VALUES (new.id, new.title, new.content);
            END",
            [],
        )?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
                INSERT INTO documents_fts(documents_fts, rowid, title, content) VALUES('delete', old.id, old.title, old.content);
            END",
            [],
        )?;

        Ok(())
    }

    pub async fn insert_document(
        &self,
        title: &str,
        content: &str,
        url: Option<&str>,
        source: &str,
        embedding: Option<&[u8]>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        
        let _id = conn.execute(
            "INSERT INTO documents (title, content, url, source, embedding) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![title, content, url, source, embedding],
        )?;
        
        Ok(conn.last_insert_rowid())
    }

    pub async fn get_document(&self, id: i64) -> Result<Option<Document>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, content, url, source, created_at, embedding 
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
            })
        });

        match doc {
            Ok(document) => Ok(Some(document)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub async fn search_documents(&self, query: &str, limit: i64) -> Result<Vec<Document>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT d.id, d.title, d.content, d.url, d.source, d.created_at, d.embedding
             FROM documents d
             JOIN documents_fts fts ON d.id = fts.rowid
             WHERE documents_fts MATCH ?1
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
            })
        })?;

        let mut results = Vec::new();
        for doc in docs {
            results.push(doc?);
        }
        Ok(results)
    }

    pub async fn get_all_embeddings(&self) -> Result<Vec<(i64, Vec<f32>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, embedding FROM documents WHERE embedding IS NOT NULL"
        )?;

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let embedding_bytes: Vec<u8> = row.get(1)?;
            let embedding: Vec<f32> = bincode::deserialize(&embedding_bytes)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok((id, embedding))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}