
    pub async fn document_exists(&self, url: Option<&str>, source: &str) -> Result<Option<i64>> {
        if url.is_none() {
            return Ok(None);
        }
        
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id FROM documents WHERE url = ?1 AND source = ?2")?;
        
        let mut rows = stmt.query_map([url.unwrap(), source], |row| {
            Ok(row.get::<_, i64>(0)?)
        })?;
        
        if let Some(row_result) = rows.next() {
            Ok(Some(row_result?))
        } else {
            Ok(None)
        }
    }
