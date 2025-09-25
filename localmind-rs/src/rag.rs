use crate::{
    Result, 
    db::Database,
    vector::VectorStore,
    ollama::OllamaClient,
    document::DocumentProcessor,
};

pub struct RagPipeline {
    db: Database,
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
            return Err("Document produced no chunks".into());
        }

        // For now, we'll store the full document and generate embedding for the full content
        // In a more advanced implementation, we might store chunks separately
        let full_content = content.to_string();
        
        // Generate embedding for the document
        let embedding = self.ollama_client.generate_embedding(&full_content).await?;
        let embedding_bytes = bincode::serialize(&embedding)?;
        
        // Insert document into database
        let doc_id = self.db.insert_document(
            title,
            &full_content,
            url,
            source,
            Some(&embedding_bytes),
        ).await?;
        
        // Add to vector store
        self.vector_store.add_vector(doc_id, embedding)?;
        
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
        
        // Search for similar documents
        let search_results = self.vector_store.search(&query_embedding, 5)?;
        
        if search_results.is_empty() {
            // Fallback to text search if no vector results
            return self.fallback_text_search(input).await;
        }

        // Get document details
        let mut sources = Vec::new();
        let mut context_parts = Vec::new();
        
        for result in search_results {
            if let Some(doc) = self.db.get_document(result.doc_id).await? {
                let snippet = self.create_snippet(&doc.content, input);
                
                sources.push(DocumentSource {
                    doc_id: doc.id,
                    title: doc.title.clone(),
                    content_snippet: snippet.clone(),
                    similarity: result.similarity,
                });
                
                context_parts.push(format!("Title: {}\nContent: {}", doc.title, snippet));
            }
        }
        
        // Build context and prompt
        let context = context_parts.join("\n\n");
        let prompt = self.build_rag_prompt(input, &context);
        
        // Generate response
        let answer = self.ollama_client.generate_completion(&prompt).await?;
        
        Ok(RagResponse {
            answer: answer.trim().to_string(),
            sources,
        })
    }

    async fn fallback_text_search(&self, query: &str) -> Result<RagResponse> {
        // Use SQLite FTS5 for text search
        let documents = self.db.search_documents(query, 3).await?;
        
        if documents.is_empty() {
            return Ok(RagResponse {
                answer: format!("I couldn't find any documents related to '{}'. Try rephrasing your question or adding more documents to search.", query),
                sources: vec![],
            });
        }

        let mut sources = Vec::new();
        let mut context_parts = Vec::new();
        
        for doc in documents {
            let snippet = self.create_snippet(&doc.content, query);
            
            sources.push(DocumentSource {
                doc_id: doc.id,
                title: doc.title.clone(),
                content_snippet: snippet.clone(),
                similarity: 0.0, // No similarity score from text search
            });
            
            context_parts.push(format!("Title: {}\nContent: {}", doc.title, snippet));
        }
        
        let context = context_parts.join("\n\n");
        let prompt = self.build_rag_prompt(query, &context);
        let answer = self.ollama_client.generate_completion(&prompt).await?;
        
        Ok(RagResponse {
            answer: answer.trim().to_string(),
            sources,
        })
    }

    fn create_snippet(&self, content: &str, query: &str) -> String {
        let max_snippet_length = 200;
        
        if content.len() <= max_snippet_length {
            return content.to_string();
        }
        
        // Try to find the query term in the content
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        if let Some(pos) = content_lower.find(&query_lower) {
            let start = pos.saturating_sub(50);
            let end = std::cmp::min(pos + query.len() + 150, content.len());
            let snippet = &content[start..end];
            
            let prefix = if start > 0 { "..." } else { "" };
            let suffix = if end < content.len() { "..." } else { "" };
            
            format!("{}{}{}", prefix, snippet, suffix)
        } else {
            // Just take the beginning of the content
            let end = std::cmp::min(max_snippet_length, content.len());
            let snippet = &content[..end];
            if end < content.len() {
                format!("{}...", snippet)
            } else {
                snippet.to_string()
            }
        }
    }

    fn build_rag_prompt(&self, query: &str, context: &str) -> String {
        format!(
            "Based on the following context, please answer the question. If the context doesn't contain enough information to answer the question, say so.

Context:
{}

Question: {}

Answer:",
            context,
            query
        )
    }

    pub fn vector_store_stats(&self) -> (usize, bool) {
        (self.vector_store.len(), self.vector_store.is_empty())
    }

    pub fn ollama(&self) -> &OllamaClient {
        &self.ollama_client
    }

    // Get search hits immediately without waiting for LLM generation
    pub async fn get_search_hits(&self, input: &str) -> Result<Vec<DocumentSource>> {
        let query_embedding = self.ollama_client.generate_embedding(input).await?;
        let search_results = self.vector_store.search(&query_embedding, 5)?;

        if search_results.is_empty() {
            // Fallback to text search if no vector results
            let documents = self.db.search_documents(input, 3).await?;
            let sources = documents.into_iter().map(|doc| DocumentSource {
                doc_id: doc.id,
                title: doc.title,
                content_snippet: doc.content.chars().take(200).collect::<String>() + "...",
                similarity: 0.0,
            }).collect();
            return Ok(sources);
        }

        let mut sources = Vec::new();
        for result in search_results {
            if let Ok(Some(doc)) = self.db.get_document(result.doc_id).await {
                sources.push(DocumentSource {
                    doc_id: result.doc_id,
                    title: doc.title,
                    content_snippet: doc.content.chars().take(200).collect::<String>() + "...",
                    similarity: result.similarity,
                });
            }
        }

        Ok(sources)
    }

    // Generate answer using specific document IDs for context
    pub async fn generate_answer(&self, input: &str, context_doc_ids: &[i64]) -> Result<String> {
        let mut context_parts = Vec::new();

        // Get content from specified documents
        for &doc_id in context_doc_ids {
            if let Ok(Some(doc)) = self.db.get_document(doc_id).await {
                context_parts.push(format!("From \"{}\":\n{}", doc.title, doc.content));
            }
        }

        if context_parts.is_empty() {
            return Ok("I couldn't find any relevant information to answer your question.".to_string());
        }

        let context = context_parts.join("\n\n");
        let prompt = self.build_rag_prompt(input, &context);
        let answer = self.ollama_client.generate_completion(&prompt).await?;

        Ok(answer.trim().to_string())
    }
}

// Remove the Default implementation as it's not safe and not needed