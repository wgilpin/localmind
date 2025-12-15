use crate::{
    db::{Database, Document, OperationPriority},
    document::DocumentProcessor,
    local_embedding::LocalEmbeddingClient,
    vector::VectorStore,
    Result,
};
use std::collections::{HashMap, HashSet};
use tokio::sync::Mutex;

pub struct RagPipeline {
    pub db: Database,
    vector_store: Mutex<VectorStore>,
    embedding_client: LocalEmbeddingClient,
    document_processor: DocumentProcessor,
    query_embedding_cache: Mutex<HashMap<String, Vec<f32>>>,
}

#[derive(Debug)]
pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<DocumentSource>,
}

#[derive(Debug)]
pub struct DocumentSource {
    pub doc_id: i64,
    pub title: String,
    pub content_snippet: String,
    pub similarity: f32,
}

impl RagPipeline {
    /// Initialize RAG pipeline with local Python embedding server.
    ///
    /// The embedding server should be running on localhost (default port 8000,
    /// configurable via EMBEDDING_SERVER_PORT environment variable).
    pub async fn new(db: Database) -> Result<Self> {
        let embedding_client = LocalEmbeddingClient::new();
        
        // Verify embedding server is ready before proceeding
        println!("Checking embedding server health...");
        match embedding_client.health_check().await {
            Ok(true) => {
                println!("Embedding server is ready and model is loaded");
            }
            Ok(false) => {
                eprintln!("WARNING: Embedding server is running but model is not loaded yet");
                eprintln!("Waiting for model to load...");
                // Wait up to 30 seconds for model to load
                for _ in 0..30 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    if let Ok(true) = embedding_client.health_check().await {
                        println!("Embedding server model is now loaded");
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("ERROR: Failed to connect to embedding server: {}", e);
                eprintln!("Make sure the embedding server is running on localhost:8000");
                return Err(format!("Embedding server not available: {}", e).into());
            }
        }
        
        let document_processor = DocumentProcessor::default();
        let mut vector_store = VectorStore::new();

        // Load existing chunk embeddings from database
        let chunk_embeddings = db.get_all_chunk_embeddings().await?;
        let chunk_count = chunk_embeddings.len();
        vector_store.load_chunk_vectors(chunk_embeddings)?;
        println!("Loaded {} chunk embeddings from database", chunk_count);
        
        // Check total document count
        let total_docs = db.count_documents(OperationPriority::UserSearch).await.unwrap_or(0);
        println!("Total documents in database: {}", total_docs);
        
        if chunk_count == 0 && total_docs > 0 {
            println!("WARNING: Documents exist in database but have no embeddings!");
            println!("You may need to re-index your documents using the reembed_batched tool.");
        } else if total_docs == 0 {
            println!("INFO: No documents in database. Add documents to enable search.");
        }

        println!("RAG pipeline initialized with local Python embedding server");

        Ok(Self {
            db,
            vector_store: Mutex::new(vector_store),
            embedding_client,
            document_processor,
            query_embedding_cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn get_embedding_service_name(&self) -> &str {
        "Local Python Embedding Server"
    }

    async fn get_cached_query_embedding(&self, query: &str) -> Result<Vec<f32>> {
        // Check cache first
        {
            let cache = self.query_embedding_cache.lock().await;
            if let Some(cached_embedding) = cache.get(query) {
                println!(
                    "Using cached embedding for query: {}",
                    query.chars().take(50).collect::<String>()
                );
                return Ok(cached_embedding.clone());
            }
        }

        // Generate new embedding with query formatting
        println!(
            "Generating new embedding for query: {}",
            query.chars().take(50).collect::<String>()
        );
        let embedding = self
            .embedding_client
            .generate_embedding(query)
            .await
            .map_err(|e| format!("Failed to generate embedding: {}", e))?;

        // Cache the embedding
        {
            let mut cache = self.query_embedding_cache.lock().await;
            cache.insert(query.to_string(), embedding.clone());

            // Keep cache size reasonable (last 20 queries)
            if cache.len() > 20 {
                if let Some(oldest_key) = cache.keys().next().cloned() {
                    cache.remove(&oldest_key);
                }
            }
        }

        Ok(embedding)
    }

    pub async fn ingest_document(
        &self,
        title: &str,
        content: &str,
        url: Option<&str>,
        source: &str,
    ) -> Result<i64> {
        // Chunk the document
        let chunks = self.document_processor.chunk_text(content)?;

        if chunks.is_empty() {
            println!("Document produced no chunks, returning error");
            return Err("Document produced no chunks".into());
        }

        println!(
            "Processing document: '{}' → {} chunks (content: {} chars)",
            title.chars().take(60).collect::<String>(),
            chunks.len(),
            content.len()
        );

        // Store the full document (without embedding in document table)
        let doc_id = self
            .db
            .insert_document(
                title,
                content,
                url,
                source,
                None, // No embedding at document level
                None, // is_dead defaults to false
                OperationPriority::BackgroundIngest,
            )
            .await?;

        // Generate and store embeddings for each chunk
        for chunk in chunks.iter() {
            // Generate embedding for this chunk with document formatting
            let chunk_embedding = self
                .embedding_client
                .generate_embedding(&chunk.content)
                .await
                .map_err(|e| format!("Failed to generate embedding for chunk: {}", e))?;
            let embedding_bytes = bincode::serialize(&chunk_embedding)?;

            // Use actual chunk boundaries from DocumentChunk
            let chunk_start = chunk.start_pos;
            let chunk_end = chunk.end_pos;

            // Insert chunk embedding
            let embedding_id = self
                .db
                .insert_chunk_embedding(
                    doc_id,
                    chunk_start,
                    chunk_end,
                    &embedding_bytes,
                    OperationPriority::BackgroundIngest,
                )
                .await?;

            // Add to vector store
            {
                let mut vector_store = self.vector_store.lock().await;
                vector_store.add_chunk_vector(
                    embedding_id,
                    doc_id,
                    chunk_start,
                    chunk_end,
                    chunk_embedding,
                )?;
            }
        }

        {
            let vector_store = self.vector_store.lock().await;
            let total_vectors = vector_store.chunk_vector_count();
            println!(
                "ingest_document completed successfully for: {} ({} chunks indexed, {} total vectors in memory)",
                title,
                chunks.len(),
                total_vectors
            );
        }
        Ok(doc_id)
    }

    pub async fn query(&self, input: &str) -> Result<RagResponse> {
        self.query_with_cutoff(input, 0.2).await // Use more permissive default
    }

    pub async fn query_with_cutoff(&self, input: &str, cutoff: f32) -> Result<RagResponse> {
        if input.trim().is_empty() {
            return Ok(RagResponse {
                answer: "Please provide a question to search for.".to_string(),
                sources: vec![],
            });
        }

        // Use the updated search method which now uses chunks
        let sources = self.get_search_hits_with_cutoff(input, cutoff).await?;

        if sources.is_empty() {
            return Ok(RagResponse {
                answer: "I couldn't find any relevant information for your query.".to_string(),
                sources: vec![],
            });
        }

        // Take only top 5 for response generation
        let top_sources = sources.into_iter().take(5).collect::<Vec<_>>();

        // Return sources without generating a completion (completions removed per spec)
        let answer = format!(
            "Found {} relevant document{} for your query.",
            top_sources.len(),
            if top_sources.len() == 1 { "" } else { "s" }
        );

        Ok(RagResponse {
            answer,
            sources: top_sources,
        })
    }

    // Add the search method for compatibility
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<(Document, f32)>> {
        self.search_with_cutoff(query, limit, 0.2).await // Use more permissive default
    }

    pub async fn search_with_cutoff(
        &self,
        query: &str,
        limit: usize,
        cutoff: f32,
    ) -> Result<Vec<(Document, f32)>> {
        println!("Searching for: '{}' (limit: {}, cutoff: {})", query, limit, cutoff);
        
        // Use cached embedding for the query
        let query_embedding = self.get_cached_query_embedding(query).await?;
        println!("Generated query embedding (dimension: {})", query_embedding.len());

        // Search chunk embeddings instead of document embeddings
        let chunk_results = {
            let vector_store = self.vector_store.lock().await;
            let chunk_count = vector_store.chunk_vector_count();
            println!("Searching in vector store: {} chunk vectors available", chunk_count);
            vector_store.search_chunks_with_cutoff(&query_embedding, limit * 2, cutoff)?
        };
        
        println!("Found {} chunk results", chunk_results.len());

        let mut results = Vec::new();
        let mut seen_docs = HashSet::new();

        // Process chunk results and group by document (take highest scoring chunk per doc)
        for chunk_result in chunk_results {
            if seen_docs.contains(&chunk_result.doc_id) {
                continue;
            }
            seen_docs.insert(chunk_result.doc_id);

            if let Some(doc) = self.db.get_document(chunk_result.doc_id).await? {
                results.push((doc, chunk_result.similarity));

                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    // Add the chat method for compatibility
    pub async fn chat(&self, message: &str) -> Result<String> {
        let response = self.query(message).await?;
        Ok(response.answer)
    }

    fn extract_snippet(&self, content: &str, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = content.to_lowercase();

        // Find the position of the first query word
        let mut best_position = 0;
        for word in &query_words {
            if let Some(pos) = content_lower.find(word) {
                best_position = pos;
                break;
            }
        }

        // Extract snippet around that position
        let start = best_position.saturating_sub(100);
        let end = std::cmp::min(best_position + 300, content.len());

        // Ensure start and end are on UTF-8 character boundaries
        let mut safe_start = start;
        while safe_start > 0 && !content.is_char_boundary(safe_start) {
            safe_start -= 1;
        }

        let mut safe_end = end;
        while safe_end > safe_start && !content.is_char_boundary(safe_end) {
            safe_end -= 1;
        }
        // Make sure we don't cut in the middle of a word
        let snippet = &content[safe_start..safe_end];
        format!("...{}\n...", snippet.trim())
    }

    pub async fn document_exists(&self, url: &str) -> Result<bool> {
        // Use background priority since this is typically called during ingestion
        self.db
            .url_exists(url, OperationPriority::BackgroundIngest)
            .await
    }

    pub async fn get_document_count(&self) -> Result<i64> {
        // Use background priority for stats queries
        self.db
            .count_documents(OperationPriority::BackgroundIngest)
            .await
    }

    // Additional methods needed by main.rs
    pub async fn get_search_hits(&self, query: &str) -> Result<Vec<DocumentSource>> {
        self.get_search_hits_with_cutoff(query, 0.2).await // Use more permissive default
    }

    pub async fn get_search_hits_with_cutoff(
        &self,
        query: &str,
        cutoff: f32,
    ) -> Result<Vec<DocumentSource>> {
        // Use cached embedding for the query
        let query_embedding = self.get_cached_query_embedding(query).await?;

        // Search chunk embeddings instead of document embeddings
        let chunk_results = {
            let vector_store = self.vector_store.lock().await;
            vector_store.search_chunks_with_cutoff(&query_embedding, 20, cutoff)?
        };

        let mut sources = Vec::new();
        let mut seen_docs = HashSet::new();

        // Process chunk results and group by document
        for chunk_result in chunk_results {
            // Skip if we already have this document (take highest scoring chunk per doc)
            if seen_docs.contains(&chunk_result.doc_id) {
                continue;
            }
            seen_docs.insert(chunk_result.doc_id);

            if let Some(doc) = self.db.get_document(chunk_result.doc_id).await? {
                // Extract the actual chunk content from the document
                let content_chars: Vec<char> = doc.content.chars().collect();
                let chunk_content = if chunk_result.chunk_end <= content_chars.len() {
                    content_chars[chunk_result.chunk_start..chunk_result.chunk_end]
                        .iter()
                        .collect::<String>()
                } else {
                    // Fallback to snippet extraction if chunk boundaries are off
                    self.extract_snippet(&doc.content, query)
                };

                sources.push(DocumentSource {
                    doc_id: chunk_result.doc_id,
                    title: doc.title,
                    content_snippet: chunk_content,
                    similarity: chunk_result.similarity,
                });

                // Limit to 10 documents
                if sources.len() >= 10 {
                    break;
                }
            }
        }

        Ok(sources)
    }

    // Completion methods removed - this is an embedding-only service

    pub fn vector_store_stats(&self) -> (usize, bool) {
        // Use try_lock to avoid blocking, return 0 if locked
        if let Ok(vector_store) = self.vector_store.try_lock() {
            let chunk_count = vector_store.chunk_len();
            (chunk_count, chunk_count == 0)
        } else {
            (0, true) // Return empty stats if locked
        }
    }

    // Streaming completion methods removed - this is an embedding-only service
}
