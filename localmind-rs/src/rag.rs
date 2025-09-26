use crate::{
    Result,
    db::{Database, Document, OperationPriority},
    vector::VectorStore,
    ollama::OllamaClient,
    document::DocumentProcessor,
};
use std::collections::{HashSet, HashMap};
use tokio::sync::Mutex;

pub struct RagPipeline {
    pub db: Database,
    vector_store: Mutex<VectorStore>,
    ollama_client: OllamaClient,
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
    pub async fn new(db: Database, ollama_client: OllamaClient) -> Result<Self> {
        let document_processor = DocumentProcessor::default();
        let mut vector_store = VectorStore::new();

        // Load existing chunk embeddings from database
        let chunk_embeddings = db.get_all_chunk_embeddings().await?;
        vector_store.load_chunk_vectors(chunk_embeddings)?;

        // For backward compatibility, also load old document embeddings
        let legacy_embeddings = db.get_all_embeddings().await?;
        vector_store.load_vectors(legacy_embeddings)?;

        Ok(Self {
            db,
            vector_store: Mutex::new(vector_store),
            ollama_client,
            document_processor,
            query_embedding_cache: Mutex::new(HashMap::new()),
        })
    }

    async fn get_cached_query_embedding(&self, query: &str) -> Result<Vec<f32>> {
        // Check cache first
        {
            let cache = self.query_embedding_cache.lock().await;
            if let Some(cached_embedding) = cache.get(query) {
                println!("🔍 Using cached embedding for query: {}", query.chars().take(50).collect::<String>());
                return Ok(cached_embedding.clone());
            }
        }

        // Generate new embedding
        println!("🔍 Generating new embedding for query: {}", query.chars().take(50).collect::<String>());
        let embedding = self.ollama_client.generate_embedding(query).await?;

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
            println!("❌ Document produced no chunks, returning error");
            return Err("Document produced no chunks".into());
        }

        println!("📝 Processing document: '{}' → {} chunks (content: {} chars)",
                 title.chars().take(60).collect::<String>(),
                 chunks.len(),
                 content.len());

        // Store the full document (without embedding in document table)
        let doc_id = self.db.insert_document(
            title,
            content,
            url,
            source,
            None, // No embedding at document level
            None, // is_dead defaults to false
            OperationPriority::BackgroundIngest,
        ).await?;

        // Generate and store embeddings for each chunk
        for (chunk_index, chunk) in chunks.iter().enumerate() {
            // Generate embedding for this chunk
            let chunk_embedding = self.ollama_client.generate_embedding(&chunk.content).await?;
            let embedding_bytes = bincode::serialize(&chunk_embedding)?;

            // Use actual chunk boundaries from DocumentChunk
            let chunk_start = chunk.start_pos;
            let chunk_end = chunk.end_pos;

            // Insert chunk embedding
            let embedding_id = self.db.insert_chunk_embedding(
                doc_id,
                chunk_index,
                chunk_start,
                chunk_end,
                &embedding_bytes,
                OperationPriority::BackgroundIngest,
            ).await?;

            // Add to vector store
            {
                let mut vector_store = self.vector_store.lock().await;
                vector_store.add_chunk_vector(
                    embedding_id,
                    doc_id,
                    chunk_index,
                    chunk_start,
                    chunk_end,
                    chunk_embedding,
                )?;
            }
        }

        println!("🎉 ingest_document completed successfully for: {} ({} chunks indexed)", title, chunks.len());
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

        // Build context from sources (now using chunk content)
        let context = top_sources.iter()
            .map(|s| format!("Source: {}\n{}", s.title, s.content_snippet))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        // Generate response using context
        let prompt = format!(
            "Context information:\n{}\n\nQuestion: {}\n\nBased on the context above, provide a helpful answer:",
            context,
            input
        );

        let answer = self.ollama_client.generate_completion(&prompt).await
            .unwrap_or_else(|_| "I encountered an error generating a response.".to_string());

        Ok(RagResponse {
            answer,
            sources: top_sources,
        })
    }

    // Add the search method for compatibility
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<(Document, f32)>> {
        self.search_with_cutoff(query, limit, 0.2).await // Use more permissive default
    }

    pub async fn search_with_cutoff(&self, query: &str, limit: usize, cutoff: f32) -> Result<Vec<(Document, f32)>> {
        // Use cached embedding for the query
        let query_embedding = self.get_cached_query_embedding(query).await?;

        // Search chunk embeddings instead of document embeddings
        let chunk_results = {
            let vector_store = self.vector_store.lock().await;
            vector_store.search_chunks_with_cutoff(&query_embedding, limit * 2, cutoff)?
        };

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

        // Make sure we don't cut in the middle of a word
        let snippet = &content[start..end];
        format!("...{}\n...", snippet.trim())
    }

    pub async fn document_exists(&self, url: &str) -> Result<bool> {
        // Use background priority since this is typically called during ingestion
        self.db.url_exists(url, OperationPriority::BackgroundIngest).await
    }

    pub async fn get_document_count(&self) -> Result<i64> {
        // Use background priority for stats queries
        self.db.count_documents(OperationPriority::BackgroundIngest).await
    }

    // Additional methods needed by main.rs
    pub async fn get_search_hits(&self, query: &str) -> Result<Vec<DocumentSource>> {
        self.get_search_hits_with_cutoff(query, 0.2).await // Use more permissive default
    }

    pub async fn get_search_hits_with_cutoff(&self, query: &str, cutoff: f32) -> Result<Vec<DocumentSource>> {
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

    pub async fn generate_answer(&self, query: &str, context_doc_ids: &[i64]) -> Result<String> {
        let mut context_parts = Vec::new();

        // Get documents by IDs
        for &doc_id in context_doc_ids {
            if let Some(doc) = self.db.get_document(doc_id).await? {
                let snippet = self.extract_snippet(&doc.content, query);
                context_parts.push(format!("Source: {}\n{}", doc.title, snippet));
            }
        }

        if context_parts.is_empty() {
            return Ok("I couldn't find any relevant information for your query.".to_string());
        }

        let context = context_parts.join("\n\n---\n\n");

        // Generate response using context
        let prompt = format!(
            "Context information:\n{}\n\nQuestion: {}\n\nBased on the context above, provide a helpful answer:",
            context,
            query
        );

        let answer = self.ollama_client.generate_completion(&prompt).await
            .unwrap_or_else(|_| "I encountered an error generating a response.".to_string());

        Ok(answer)
    }

    pub fn vector_store_stats(&self) -> (usize, bool) {
        // Use try_lock to avoid blocking, return 0 if locked
        if let Ok(vector_store) = self.vector_store.try_lock() {
            let chunk_count = vector_store.chunk_len();
            let legacy_count = vector_store.len();
            let total_count = chunk_count + legacy_count;
            (total_count, total_count == 0)
        } else {
            (0, true) // Return empty stats if locked
        }
    }

    pub fn ollama(&self) -> &OllamaClient {
        &self.ollama_client
    }
}