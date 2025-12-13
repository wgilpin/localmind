# Quickstart: HTTP REST API Server Implementation

**Date**: 2025-01-27  
**Feature**: HTTP REST API Server for Chrome Extension Integration

## Overview

This guide provides a quick start for implementing the HTTP REST API server that enables Chrome extension integration with the LocalMind Rust application.

**Important**: The HTTP server runs **within the same process** as the Tauri GUI application, not as a separate executable. When you launch the LocalMind application, both the GUI window and HTTP server start automatically in the same process, sharing the same RAG state and database.

## Prerequisites

- Rust 1.75+ installed
- Existing LocalMind codebase with Tauri GUI
- Tokio async runtime (already in dependencies)
- Understanding of axum HTTP framework

## Step 1: Add Dependencies

Add to `localmind-rs/Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }
```

## Step 2: Create HTTP Server Module

Create `localmind-rs/src/http_server.rs`:

```rust
use axum::{
    extract::{Request, State},
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
use tower_http::cors::{CorsLayer, Any};

use crate::rag::RagState;
use crate::youtube::YouTubeProcessor;

#[derive(Clone)]
struct AppState {
    rag_state: RagState,
}

#[derive(Deserialize)]
struct DocumentRequest {
    title: String,
    content: String,
    url: Option<String>,
    #[serde(default = "default_extraction_method")]
    extractionMethod: String,
}

fn default_extraction_method() -> String {
    "dom".to_string()
}

#[derive(Serialize)]
struct SuccessResponse {
    message: String,
    extractionMethod: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "message": self.message }));
        (self.status, body).into_response()
    }
}

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
    let mut title = request.title;
    let mut content = request.content;
    
    if let Some(ref url) = request.url {
        if YouTubeProcessor::is_youtube_url(url) {
            match YouTubeProcessor::fetch_transcript(url).await {
                Ok(Some(transcript)) => {
                    content = transcript;
                    title = YouTubeProcessor::cleanup_title(&title);
                }
                _ => {
                    // Fall back to provided content
                }
            }
        }
    }

    // Ingest document
    drop(rag_lock); // Release read lock
    let rag_lock = state.rag_state.read().await;
    let rag = rag_lock.as_ref().unwrap();
    
    rag.ingest_document(&title, &content, request.url.as_deref(), "chrome_extension")
        .await
        .map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to add document: {}", e),
        })?;

    Ok(Json(SuccessResponse {
        message: "Document added successfully.".to_string(),
        extractionMethod: request.extractionMethod,
    }))
}

pub async fn start_http_server(rag_state: RagState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Find available port
    let port = find_available_port(3000, 11).await
        .ok_or("No available ports in range 3000-3010")?;
    
    println!("Starting HTTP server on port {}", port);

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    // Build application
    let app_state = AppState { rag_state };
    let app = Router::new()
        .route("/documents", post(handle_post_documents))
        .layer(
            ServiceBuilder::new()
                .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB
                .layer(cors),
        )
        .with_state(app_state);

    // Start server
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    println!("HTTP server listening on http://localhost:{}", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn find_available_port(start_port: u16, max_attempts: u16) -> Option<u16> {
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
```

## Step 3: Integrate into Main Application

Update `localmind-rs/src/main.rs`:

1. Add module declaration:
```rust
mod http_server;
```

2. Import HTTP server function:
```rust
use crate::http_server::start_http_server;
```

3. Start HTTP server after RAG initialization in `setup` function:
```rust
tokio::spawn(async move {
    match init_rag_system().await {
        Ok(rag) => {
            // ... existing RAG initialization code ...
            
            // Start HTTP server
            let rag_state_for_http = rag_state_clone.clone();
            tokio::spawn(async move {
                if let Err(e) = start_http_server(rag_state_for_http).await {
                    eprintln!("Failed to start HTTP server: {}", e);
                }
            });
        }
        Err(e) => { /* ... */ }
    }
});
```

## Step 4: Test the Implementation

### Manual Testing

1. Start the LocalMind application:
```bash
cd localmind-rs
cargo run
```

2. Check logs for HTTP server startup message:
```
HTTP server listening on http://localhost:3000
```

3. Send test request:
```bash
curl -X POST http://localhost:3000/documents \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Test Document",
    "content": "This is a test document.",
    "url": "https://example.com",
    "extractionMethod": "dom"
  }'
```

4. Verify document appears in Tauri GUI search results.

### CORS Testing

Test CORS headers:
```bash
curl -X OPTIONS http://localhost:3000/documents \
  -H "Origin: chrome-extension://test" \
  -H "Access-Control-Request-Method: POST" \
  -v
```

Should return:
- `Access-Control-Allow-Origin: *`
- `Access-Control-Allow-Methods: POST, OPTIONS`
- `Access-Control-Allow-Headers: content-type`

## Common Issues

### Port Already in Use

**Symptom**: Server fails to start on port 3000

**Solution**: Server automatically tries ports 3001-3010. Check logs for which port was bound. Update Chrome extension to try multiple ports (out of scope for this implementation).

### RAG System Not Initialized

**Symptom**: Returns HTTP 503 "System initializing"

**Solution**: Wait for RAG system to finish initializing. This is expected behavior during application startup.

### Request Size Exceeded

**Symptom**: Returns HTTP 413 "Request payload exceeds 10MB limit"

**Solution**: Reduce document content size. 10MB limit is intentional to prevent DoS attacks.

## Next Steps

1. Implement Chrome extension port discovery (separate task)
2. Add integration tests for HTTP server
3. Add unit tests for request validation
4. Monitor performance and optimize if needed

## References

- [axum documentation](https://docs.rs/axum/)
- [tower-http CORS](https://docs.rs/tower-http/latest/tower_http/cors/)
- [OpenAPI specification](./contracts/openapi.yaml)
- [Data model](./data-model.md)
