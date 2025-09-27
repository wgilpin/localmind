use crate::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct CompletionRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct CompletionResponse {
    response: String,
    done: bool,
}

pub struct OllamaClient {
    base_url: String,
    client: Client,
    embedding_model: String,
    completion_model: String,
}

impl OllamaClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
            embedding_model: "qwen3-embedding:0.6b".to_string(),
            completion_model: "llama3.2:3b".to_string(),
        }
    }

    pub fn with_models(base_url: String, embedding_model: String, completion_model: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
            embedding_model,
            completion_model,
        }
    }

    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.base_url);
        
        let request = EmbeddingRequest {
            model: self.embedding_model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Ollama embedding request failed: {}", response.status()).into());
        }

        let embedding_response: EmbeddingResponse = response.json().await?;
        Ok(embedding_response.embedding)
    }

    pub async fn generate_completion(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        let request = CompletionRequest {
            model: self.completion_model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Ollama completion request failed: {}", response.status()).into());
        }

        let completion_response: CompletionResponse = response.json().await?;
        Ok(completion_response.response)
    }

    pub async fn generate_completion_with_cancellation(&self, prompt: &str, cancel_token: CancellationToken) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        let request = CompletionRequest {
            model: self.completion_model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        // Use tokio::select! to race between the request and cancellation
        tokio::select! {
            response_result = self.client.post(&url).json(&request).send() => {
                let response = response_result?;

                if !response.status().is_success() {
                    return Err(format!("Ollama completion request failed: {}", response.status()).into());
                }

                // Race between response parsing and cancellation
                tokio::select! {
                    json_result = response.json::<CompletionResponse>() => {
                        let completion_response = json_result?;
                        Ok(completion_response.response)
                    }
                    _ = cancel_token.cancelled() => {
                        Err("Request was cancelled".into())
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                Err("Request was cancelled".into())
            }
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("Ollama models request failed: {}", response.status()).into());
        }

        #[derive(Deserialize)]
        struct Model {
            name: String,
        }

        #[derive(Deserialize)]
        struct ModelsResponse {
            models: Vec<Model>,
        }

        let models_response: ModelsResponse = response.json().await?;
        Ok(models_response.models.into_iter().map(|m| m.name).collect())
    }

    pub async fn check_models_available(&self) -> Result<(bool, bool, Vec<String>)> {
        let available_models = self.list_models().await?;

        let embedding_available = available_models.iter()
            .any(|m| m == &self.embedding_model);

        let completion_available = available_models.iter()
            .any(|m| m == &self.completion_model);

        Ok((embedding_available, completion_available, available_models))
    }

    pub fn get_model_names(&self) -> (String, String) {
        (self.embedding_model.clone(), self.completion_model.clone())
    }
}