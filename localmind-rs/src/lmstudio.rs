use crate::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct Model {
    id: String,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<Model>,
}

pub struct LMStudioClient {
    base_url: String,
    client: Client,
    embedding_model: String,
}

impl LMStudioClient {
    pub fn new(base_url: String, embedding_model: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
            embedding_model,
        }
    }

    pub async fn test_connection(&self) -> Result<()> {
        let url = format!("{}/v1/models", self.base_url);

        let response = self.client.get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Cannot connect to LM Studio at {}. Make sure LM Studio server is running. Error: {}", self.base_url, e))?;

        if !response.status().is_success() {
            return Err(format!("LM Studio connection failed: {}", response.status()).into());
        }

        let models_response: ModelsResponse = response.json().await?;
        let model_ids: Vec<String> = models_response.data.into_iter().map(|m| m.id).collect();

        println!("LM Studio connection successful. Available models: {:?}", model_ids);

        // Check if we have an embedding model
        let embedding_models: Vec<String> = model_ids.iter()
            .filter(|m| m.to_lowercase().contains("embed") || m.to_lowercase().contains("nomic"))
            .cloned()
            .collect();

        if !embedding_models.is_empty() {
            println!("Found embedding models: {:?}", embedding_models);
        } else {
            println!("Warning: No embedding models found in LM Studio. Make sure to load an embedding model.");
        }

        Ok(())
    }

    /// Get model-specific formatting prefixes
    fn get_instruction_prefixes(&self) -> (String, String) {
        let model_name = self.embedding_model.to_lowercase();
        let base_model_name = model_name.replace("-gpu", "").replace("text-embedding-", "");

        if base_model_name.contains("embeddinggemma") {
            (
                "task: search result | query: ".to_string(),
                "title: {title} | text: ".to_string(),
            )
        } else if base_model_name.contains("nomic-embed-text") {
            (
                "search_query: ".to_string(),
                "search_document: ".to_string(),
            )
        } else if base_model_name.contains("qwen3-embedding") {
            (
                "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: ".to_string(),
                "".to_string(), // Qwen3 doesn't use document prefix
            )
        } else {
            // Default: no prefixes
            ("".to_string(), "".to_string())
        }
    }

    /// Format text with appropriate prefix for the embedding model
    pub fn format_text_for_embedding(&self, text: &str, is_query: bool, document_title: Option<&str>) -> String {
        let (query_prefix, document_prefix) = self.get_instruction_prefixes();

        if is_query {
            format!("{}{}", query_prefix, text)
        } else {
            if self.embedding_model.to_lowercase().contains("embeddinggemma") {
                // EmbeddingGemma uses title in the prefix
                let title = document_title.unwrap_or("content");
                let prefix = document_prefix.replace("{title}", title);
                format!("{}{}", prefix, text)
            } else {
                format!("{}{}", document_prefix, text)
            }
        }
    }

    /// Generate embeddings for a batch of texts using LM Studio's OpenAI-compatible API
    pub async fn generate_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let batch_size = texts.len();
        let max_retries = 3;

        if batch_size > 10 {
            let total_chars: usize = texts.iter().map(|t| t.len()).sum();
            println!("[EMBEDDING] Sending batch of {} texts to LM Studio (total {} chars)", batch_size, total_chars);
        }

        for attempt in 0..max_retries {
            match self.try_generate_embeddings(&texts).await {
                Ok(embeddings) => {
                    if batch_size > 10 {
                        println!("[EMBEDDING] Success: got {} embeddings, each {}-dim",
                                batch_size, embeddings[0].len());
                    }
                    return Ok(embeddings);
                }
                Err(e) => {
                    if attempt == max_retries - 1 {
                        println!("[EMBEDDING] FAILED after {} attempts: {}", max_retries, e);
                        return Err(format!("Failed to get embeddings after {} attempts: {}", max_retries, e).into());
                    }
                    println!("[EMBEDDING] Attempt {} failed, retrying in 1 second...{}", attempt + 1, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        Err("Failed to get embeddings after retries".into())
    }

    async fn try_generate_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let url = format!("{}/v1/embeddings", self.base_url);

        let request = EmbeddingRequest {
            input: texts.to_vec(),
            model: self.embedding_model.clone(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("LM Studio embedding request failed: {}", response.status()).into());
        }

        let embedding_response: EmbeddingResponse = response.json().await?;

        if embedding_response.data.is_empty() {
            return Err("No embeddings in response".into());
        }

        // Sort by index and extract embeddings
        let mut data = embedding_response.data;
        data.sort_by_key(|d| d.index);
        let embeddings: Vec<Vec<f32>> = data.into_iter().map(|d| d.embedding).collect();

        if embeddings.len() != texts.len() {
            return Err(format!("Expected {} embeddings, got {}", texts.len(), embeddings.len()).into());
        }

        Ok(embeddings)
    }

    /// Generate a single embedding with appropriate formatting
    pub async fn generate_embedding(&self, text: &str, is_query: bool, document_title: Option<&str>) -> Result<Vec<f32>> {
        let formatted_text = self.format_text_for_embedding(text, is_query, document_title);
        let embeddings = self.generate_embeddings(vec![formatted_text]).await?;
        Ok(embeddings.into_iter().next().unwrap())
    }

    pub fn get_model_name(&self) -> &str {
        &self.embedding_model
    }
}