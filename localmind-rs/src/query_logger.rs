use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize, Clone)]
pub struct ResultEntry {
    pub rank: usize,
    pub doc_id: i64,
    pub title: String,
    pub score: f32,
}

#[derive(Serialize)]
struct LogLine {
    timestamp: u64,
    query: String,
    results: Vec<ResultEntry>,
    outcome: String,
    clicked_doc_id: Option<i64>,
}

pub struct QueryLogger {
    log_path: PathBuf,
    pending_query: Option<String>,
    pending_results: Option<Vec<ResultEntry>>,
    pending_timestamp: Option<u64>,
}

impl QueryLogger {
    pub fn new(log_path: PathBuf) -> Self {
        Self {
            log_path,
            pending_query: None,
            pending_results: None,
            pending_timestamp: None,
        }
    }

    pub fn record_search(&mut self, query: &str, results: &[crate::gui::state::SearchResultView]) {
        self.finalize("new_search", None);

        let entries: Vec<ResultEntry> = results
            .iter()
            .enumerate()
            .map(|(i, r)| ResultEntry {
                rank: i + 1,
                doc_id: r.doc_id,
                title: r.title.clone(),
                score: r.similarity,
            })
            .collect();

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.pending_query = Some(query.to_string());
        self.pending_results = Some(entries);
        self.pending_timestamp = Some(ts);
    }

    pub fn finalize(&mut self, outcome: &str, clicked_doc_id: Option<i64>) {
        if let (Some(query), Some(results), Some(timestamp)) = (
            self.pending_query.take(),
            self.pending_results.take(),
            self.pending_timestamp.take(),
        ) {
            let line = LogLine {
                timestamp,
                query,
                results,
                outcome: outcome.to_string(),
                clicked_doc_id,
            };

            if let Ok(json) = serde_json::to_string(&line) {
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.log_path)
                {
                    let _ = writeln!(f, "{}", json);
                }
            }
        }
    }
}
