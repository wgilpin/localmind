//! Local embedding client for communicating with the Python embedding server.
//!
//! This module provides a Rust HTTP client that communicates with the LocalMind
//! embedding server to generate vector embeddings for text. It includes retry logic
//! for handling server startup delays and validation of embedding dimensions.
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

/// Default embedding server port
const DEFAULT_PORT: u16 = 8000;

/// Expected embedding dimension for embeddinggemma-300M
const EXPECTED_DIMENSION: usize = 768;

/// Maximum number of retry attempts for loading state
const MAX_RETRIES: u32 = 10;

/// Base delay for exponential backoff (milliseconds)
const BASE_DELAY_MS: u64 = 500;

/// Request payload for embedding generation
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    pub text: String,
}

/// Response payload containing generated embedding
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: usize,
}

/// Error response from the embedding server
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub detail: Option<String>,
}

/// HTTP client for the local embedding server
#[derive(Debug, Clone)]
pub struct LocalEmbeddingClient {
    client: Client,
    base_url: String,
}

impl LocalEmbeddingClient {
    /// Create a new LocalEmbeddingClient instance.
    ///
    /// The server port can be configured via the `EMBEDDING_SERVER_PORT` environment variable.
    /// If not set, defaults to port 8000.
    ///
    /// # Returns
    ///
    /// A new `LocalEmbeddingClient` configured to connect to the embedding server.
    pub fn new() -> Self {
        let port = env::var("EMBEDDING_SERVER_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT);

        let base_url = format!("http://localhost:{}", port);

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// Generate an embedding for the given text.
    ///
    /// This method sends the text to the embedding server and receives a vector embedding.
    /// It includes automatic retry logic with exponential backoff for handling server
    /// loading states (503 responses).
    ///
    /// # Arguments
    ///
    /// * `text` - The text to generate an embedding for
    ///
    /// # Returns
    ///
    /// A `Vec<f32>` containing the 768-dimensional embedding vector.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The server is unreachable
    /// - The server returns an error response
    /// - The embedding dimension is incorrect
    /// - Maximum retry attempts are exceeded
    ///
    /// # Example
    ///
    /// ```no_run
    /// use localmind_rs::local_embedding::LocalEmbeddingClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LocalEmbeddingClient::new();
    /// let embedding = client.generate_embedding("Hello, world!").await?;
    /// assert_eq!(embedding.len(), 768);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_embedding(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let url = format!("{}/embed", self.base_url);
        let request_body = EmbeddingRequest {
            text: text.to_string(),
        };

        let mut attempts = 0;

        loop {
            attempts += 1;

            let response = self
                .client
                .post(&url)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to connect to embedding server at {}: {}. \
                         Make sure the Python embedding server is running.",
                        self.base_url,
                        e
                    )
                })?;

            let status = response.status();

            // Handle 503 Service Unavailable (model still loading)
            if status == reqwest::StatusCode::SERVICE_UNAVAILABLE {
                if attempts >= MAX_RETRIES {
                    return Err(anyhow::anyhow!(
                        "Embedding server still loading after {} attempts. \
                         Please wait for the model to finish loading and try again.",
                        MAX_RETRIES
                    ));
                }

                // Exponential backoff
                let delay = Duration::from_millis(BASE_DELAY_MS * 2_u64.pow(attempts - 1));
                log::info!(
                    "Embedding server is loading (attempt {}/{}), retrying in {:?}...",
                    attempts,
                    MAX_RETRIES,
                    delay
                );
                sleep(delay).await;
                continue;
            }

            // Handle other non-success status codes
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());

                // Try to parse as ErrorResponse
                if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&error_text) {
                    return Err(anyhow::anyhow!(
                        "Embedding server error: {} ({})",
                        error_response.error,
                        error_response
                            .detail
                            .unwrap_or_else(|| "No details provided".to_string())
                    ));
                }

                return Err(anyhow::anyhow!(
                    "Embedding server returned status {}: {}",
                    status,
                    error_text
                ));
            }

            // Parse successful response
            let embedding_response: EmbeddingResponse = response
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to parse embedding response: {}", e))?;

            // Validate dimension
            if embedding_response.dimension != EXPECTED_DIMENSION {
                return Err(anyhow::anyhow!(
                    "Embedding dimension mismatch: expected {}, got {}",
                    EXPECTED_DIMENSION,
                    embedding_response.dimension
                ));
            }

            if embedding_response.embedding.len() != EXPECTED_DIMENSION {
                return Err(anyhow::anyhow!(
                    "Embedding vector length mismatch: expected {}, got {}",
                    EXPECTED_DIMENSION,
                    embedding_response.embedding.len()
                ));
            }

            log::debug!(
                "Successfully generated {}-dimensional embedding from model '{}'",
                embedding_response.dimension,
                embedding_response.model
            );

            return Ok(embedding_response.embedding);
        }
    }

    /// Check if the embedding server is healthy and ready to accept requests.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the server is ready, `Ok(false)` if the server is still loading,
    /// or an error if the server is unreachable or in an error state.
    pub async fn health_check(&self) -> anyhow::Result<bool> {
        let url = format!("{}/health", self.base_url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            anyhow::anyhow!(
                "Failed to connect to embedding server health endpoint: {}",
                e
            )
        })?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Health check failed with status: {}",
                response.status()
            ));
        }

        #[derive(Deserialize)]
        struct HealthResponse {
            model_loaded: bool,
        }

        let health: HealthResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse health response: {}", e))?;

        Ok(health.model_loaded)
    }
}

impl Default for LocalEmbeddingClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client_default_port() {
        // Remove any existing port env var for this test
        env::remove_var("EMBEDDING_SERVER_PORT");

        let client = LocalEmbeddingClient::new();
        assert_eq!(client.base_url, "http://localhost:8000");
    }

    #[test]
    fn test_new_client_custom_port() {
        env::set_var("EMBEDDING_SERVER_PORT", "9999");

        let client = LocalEmbeddingClient::new();
        assert_eq!(client.base_url, "http://localhost:9999");

        // Clean up
        env::remove_var("EMBEDDING_SERVER_PORT");
    }

    #[test]
    fn test_embedding_request_serialization() {
        let request = EmbeddingRequest {
            text: "test".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"text\""));
        assert!(json.contains("\"test\""));
    }

    #[test]
    fn test_embedding_response_deserialization() {
        let json = r#"{
            "embedding": [0.1, 0.2, 0.3],
            "model": "google/embeddinggemma-300M",
            "dimension": 3
        }"#;

        let response: EmbeddingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.embedding.len(), 3);
        assert_eq!(response.model, "google/embeddinggemma-300M");
        assert_eq!(response.dimension, 3);
    }

    #[test]
    fn test_error_response_deserialization() {
        let json = r#"{
            "error": "Invalid request",
            "detail": "Text is empty"
        }"#;

        let response: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.error, "Invalid request");
        assert_eq!(response.detail, Some("Text is empty".to_string()));
    }
}
