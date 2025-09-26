use crate::{
    Result,
    db::{Database, Document, OperationPriority},
    vector::VectorStore,
    ollama::OllamaClient,
    document::DocumentProcessor,
};

pub struct RagPipeline {
    pub db: Database,
    vector_store: VectorStore,
    ollama_client: OllamaClient,
    document_processor: DocumentProcessor,
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

        // Load existing embeddings from database
        let embeddings = db.get_all_embeddings().await?;
        vector_store.load_vectors(embeddings)?;

        Ok(Self {
            db,
            vector_store,
            ollama_client,
            document_processor,
        })
    }

    pub async fn ingest_document(
        &mut self,
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

        // For now, we'll store the full document and generate embedding for the full content
        // In a more advanced implementation, we might store chunks separately
        let full_content = content.to_string();

        // Generate embedding for the document
        let embedding = self.ollama_client.generate_embedding(&full_content).await?;

        let embedding_bytes = bincode::serialize(&embedding)?;

        // Insert document into database with background priority
        let doc_id = self.db.insert_document(
            title,
            &full_content,
            url,
            source,
            Some(&embedding_bytes),
            None, // is_dead defaults to false
            OperationPriority::BackgroundIngest,
        ).await?;

        // Add to vector store
        self.vector_store.add_vector(doc_id, embedding)?;

        println!("🎉 ingest_document completed successfully for: {}", title);
        Ok(doc_id)
    }

    pub async fn query(&self, input: &str) -> Result<RagResponse> {
        if input.trim().is_empty() {
            return Ok(RagResponse {
                answer: "Please provide a question to search for.".to_string(),
                sources: vec![],
            });
        }

        // Generate embedding for the query
        let query_embedding = self.ollama_client.generate_embedding(input).await?;

        // Find similar documents using vector similarity with 60% cutoff
        let search_results = self.vector_store.search_with_cutoff(&query_embedding, 5, 0.6)?;

        let mut sources = Vec::new();

        // Retrieve full documents for the most similar results
        for search_result in search_results {
            if let Some(doc) = self.db.get_document(search_result.doc_id).await? {
                // Extract a relevant snippet around the query
                let snippet = self.extract_snippet(&doc.content, input);

                sources.push(DocumentSource {
                    doc_id: search_result.doc_id,
                    title: doc.title,
                    content_snippet: snippet,
                    similarity: search_result.similarity,
                });
            }
        }

        if sources.is_empty() {
            return Ok(RagResponse {
                answer: "I couldn't find any relevant information for your query.".to_string(),
                sources: vec![],
            });
        }

        // Build context from sources
        let context = sources.iter()
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
            sources,
        })
    }

    // Add the search method for compatibility
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<(Document, f32)>> {
        // Generate embedding for the query
        let query_embedding = self.ollama_client.generate_embedding(query).await?;

        // Find similar documents using vector similarity with 60% cutoff
        let search_results = self.vector_store.search_with_cutoff(&query_embedding, limit, 0.6)?;

        let mut results = Vec::new();

        // Retrieve full documents for the most similar results
        for search_result in search_results {
            if let Some(doc) = self.db.get_document(search_result.doc_id).await? {
                results.push((doc, search_result.similarity));
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
        // Generate embedding for the query
        let query_embedding = self.ollama_client.generate_embedding(query).await?;

        // Find similar documents using vector similarity with 60% cutoff
        let search_results = self.vector_store.search_with_cutoff(&query_embedding, 10, 0.6)?;

        let mut sources = Vec::new();

        // Retrieve full documents for the most similar results
        for search_result in search_results {
            if let Some(doc) = self.db.get_document(search_result.doc_id).await? {
                // Extract a relevant snippet around the query
                let snippet = self.extract_snippet(&doc.content, query);

                sources.push(DocumentSource {
                    doc_id: search_result.doc_id,
                    title: doc.title,
                    content_snippet: snippet,
                    similarity: search_result.similarity,
                });
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
        let count = self.vector_store.len();
        (count, count == 0)
    }

    pub fn ollama(&self) -> &OllamaClient {
        &self.ollama_client
    }
}