use axum::{
    extract::State,
    http::{header, Method, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use localmind_rs::rag::RagPipeline as RAG;
use localmind_rs::youtube::YouTubeProcessor;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

// Type alias matching main.rs
type RagState = Arc<RwLock<Option<RAG>>>;

/// Application state for axum HTTP server
/// Wraps RagState to share RAG system with Tauri GUI
#[derive(Clone)]
pub struct AppState {
    pub rag_state: RagState,
}

/// API error type for HTTP responses
/// Implements IntoResponse to format errors as JSON
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "message": self.message }));
        (self.status, body).into_response()
    }
}

/// Request payload for POST /documents endpoint
#[derive(Deserialize)]
pub struct DocumentRequest {
    pub title: String,
    pub content: String,
    pub url: Option<String>,
    #[serde(default = "default_extraction_method", rename = "extractionMethod")]
    pub extraction_method: String,
}

fn default_extraction_method() -> String {
    "dom".to_string()
}

/// Success response for POST /documents endpoint
#[derive(Serialize)]
pub struct SuccessResponse {
    pub message: String,
    #[serde(rename = "extractionMethod")]
    pub extraction_method: String,
}

/// Finds an available port starting from start_port, trying up to max_attempts ports
/// Returns Some(port) if a port is available, None if all ports are in use
///
/// # Arguments
/// * `start_port` - First port to try (e.g., 3000)
/// * `max_attempts` - Maximum number of ports to try (e.g., 11 for ports 3000-3010)
///
/// # Returns
/// * `Option<u16>` - Available port number, or None if no ports available
pub async fn find_available_port(start_port: u16, max_attempts: u16) -> Option<u16> {
    for offset in 0..max_attempts {
        let port = start_port + offset;
        match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            Ok(_) => {
                println!("HTTP server bound to port {}", port);
                return Some(port);
            }
            Err(_) => continue,
        }
    }
    None
}

/// Handler for POST /documents endpoint
///
/// Accepts document data, validates it, and stores it in the RAG system.
/// Handles YouTube URL detection and transcript fetching if applicable.
///
/// # Arguments
/// * `state` - Application state containing shared RAG state
/// * `request` - Document request payload (title, content, url, extractionMethod)
///
/// # Returns
/// * `Ok(Json<SuccessResponse>)` - Success response with message and extractionMethod
/// * `Err(ApiError)` - Error response with appropriate HTTP status code
///
/// # Errors
/// * `400 Bad Request` - Missing required fields (title or content)
/// * `503 Service Unavailable` - RAG system not yet initialized
/// * `500 Internal Server Error` - Document ingestion failed
async fn handle_post_documents(
    State(state): State<AppState>,
    Json(request): Json<DocumentRequest>,
) -> Result<Json<SuccessResponse>, ApiError> {
    // Validate required fields
    if request.title.is_empty() || request.content.is_empty() {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "Title and content are required.".to_string(),
        });
    }

    // Check if RAG system is initialized
    let rag_lock = state.rag_state.read().await;
    let rag = rag_lock.as_ref().ok_or_else(|| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        message: "System initializing. Please wait a moment and try again.".to_string(),
    })?;

    // Process YouTube URL if present
    let mut title = request.title.clone();
    let mut content = request.content.clone();
    let mut extraction_method = request.extraction_method.clone();

    if let Some(ref url) = request.url {
        if YouTubeProcessor::is_youtube_url(url) {
            println!("Detected YouTube URL: {}", url);

            // Fetch transcript with 30-second timeout
            match timeout(
                Duration::from_secs(30),
                YouTubeProcessor::fetch_transcript(url),
            )
            .await
            {
                Ok(Ok(Some(transcript))) => {
                    println!(
                        "Successfully fetched YouTube transcript ({} chars)",
                        transcript.len()
                    );
                    content = transcript;
                    title = YouTubeProcessor::cleanup_title(&title);
                    extraction_method = "youtube_transcript".to_string();
                }
                Ok(Ok(None)) => {
                    println!("YouTube transcript not available, using provided content");
                    // Fall back to provided content - title still gets cleaned
                    title = YouTubeProcessor::cleanup_title(&title);
                }
                Ok(Err(e)) => {
                    println!(
                        "Error fetching YouTube transcript: {}, using provided content",
                        e
                    );
                    // Fall back to provided content - title still gets cleaned
                    title = YouTubeProcessor::cleanup_title(&title);
                }
                Err(_) => {
                    println!("YouTube transcript fetch timed out after 30 seconds, using provided content");
                    // Fall back to provided content - title still gets cleaned
                    title = YouTubeProcessor::cleanup_title(&title);
                }
            }
        }
    }

    // Log request processing per FR-019
    println!(
        "Processing document request: title='{}', url={:?}, extraction_method={}",
        title.chars().take(60).collect::<String>(),
        request.url.as_deref(),
        extraction_method
    );

    // Ingest document (lock is held during async call, which is fine for read lock)
    rag.ingest_document(&title, &content, request.url.as_deref(), "chrome_extension")
        .await
        .map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to add document: {}", e),
        })?;

    drop(rag_lock); // Release lock after ingestion

    Ok(Json(SuccessResponse {
        message: "Document added successfully.".to_string(),
        extraction_method,
    }))
}

/// Starts the HTTP server on an available port (3000-3010)
/// Server runs in the same process as Tauri GUI, sharing RagState
///
/// # Arguments
/// * `rag_state` - Shared RAG state (Arc<RwLock<Option<RAG>>>)
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error + Send + Sync>>` - Error if server fails to start
pub async fn start_http_server(
    rag_state: RagState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Find available port
    let port = find_available_port(3000, 11)
        .await
        .ok_or("No available ports in range 3000-3010")?;

    println!("Starting HTTP server on port {}", port);

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    // Build application state
    let app_state = AppState { rag_state };

    // Build router with POST /documents route
    let app = Router::new()
        .route("/documents", post(handle_post_documents))
        .layer(
            ServiceBuilder::new()
                .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB limit
                .layer(cors),
        )
        .with_state(app_state);

    // Start server
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    println!("HTTP server listening on http://localhost:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
