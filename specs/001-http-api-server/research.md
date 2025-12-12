# Research: HTTP REST API Server for Chrome Extension Integration

**Date**: 2025-01-27  
**Feature**: HTTP REST API Server  
**Phase**: 0 - Research & Technical Decisions

## Research Questions

### 1. HTTP Server Library Selection

**Question**: Which Rust HTTP server library should be used for the embedded HTTP server?

**Alternatives Considered**:

1. **axum** (v0.7+)
   - Pros: Modern, async-first, built on Tokio and Tower, excellent ergonomics, strong type safety
   - Cons: Relatively newer ecosystem, less mature than actix-web
   - CORS: Built-in via `tower-http` crate
   - Performance: Excellent, minimal overhead
   - Integration: Native Tokio integration, works seamlessly with existing async code

2. **warp** (v0.3+)
   - Pros: Functional style, composable filters, lightweight
   - Cons: Steeper learning curve, less intuitive for REST APIs
   - CORS: Built-in via `warp::cors()`
   - Performance: Good, but filter composition can be complex

3. **actix-web** (v4+)
   - Pros: Mature, battle-tested, high performance, comprehensive features
   - Cons: Heavier dependency, more complex API, actor model overhead
   - CORS: Built-in via `actix-cors` crate
   - Performance: Excellent, but may be overkill for single endpoint

4. **hyper** (raw)
   - Pros: Minimal, foundational HTTP library
   - Cons: Requires manual routing, CORS, JSON parsing - violates simplicity mandate
   - Not considered: Too low-level, would require implementing HTTP protocol manually

**Decision**: **axum** v0.7+

**Rationale**: 
- Native Tokio integration aligns with existing async runtime
- Clean, intuitive API that matches Rust idioms
- Built-in CORS support via `tower-http` (minimal additional dependency)
- Excellent performance with minimal overhead
- Strong type safety reduces bugs
- Active development and good documentation
- Simpler than actix-web for single-endpoint use case
- More ergonomic than warp for REST API patterns

**Dependencies to Add**:
- `axum = "0.7"` - HTTP server framework
- `tower = "0.4"` - Middleware utilities (required by axum)
- `tower-http = { version = "0.5", features = ["cors"] }` - CORS middleware

### 2. CORS Configuration

**Question**: How should CORS be configured for browser extension requests?

**Decision**: Use `tower-http::cors::CorsLayer` with `allow_origin(Any)` and `allow_methods([POST, OPTIONS])`

**Rationale**:
- Spec requires `Access-Control-Allow-Origin: *` (allow all origins)
- Browser extensions can have various origin formats (chrome-extension://, moz-extension://)
- Allowing all origins simplifies implementation for localhost-only server
- Security risk is minimal since server only listens on localhost
- Standard CORS headers needed: `Access-Control-Allow-Methods`, `Access-Control-Allow-Headers`

**Implementation Pattern**:
```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods([Method::POST, Method::OPTIONS])
    .allow_headers([header::CONTENT_TYPE]);
```

### 3. Port Conflict Handling

**Question**: How should the HTTP server handle port conflicts and communicate the selected port?

**Decision**: 
- Try ports sequentially from 3000 to 3010 (11 attempts)
- Log the successfully bound port to stdout/stderr
- Chrome extension will implement port discovery (out of scope for this spec)

**Rationale**:
- Simple sequential port scanning is sufficient for localhost-only server
- Logging to stdout allows Chrome extension to parse port number if needed
- Upper limit prevents infinite retries
- Graceful degradation: if no ports available, log error but continue running Tauri GUI

**Implementation Pattern**:
```rust
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

### 4. Request Size Limiting

**Question**: How should request size limits be enforced?

**Decision**: Use axum's built-in body size limit via `RequestBodyLimit` middleware

**Rationale**:
- Axum provides `RequestBodyLimit` middleware for this purpose
- 10MB limit specified in requirements (FR-004a)
- Middleware approach is clean and reusable
- Returns appropriate HTTP 413 status automatically

**Implementation Pattern**:
```rust
use axum::extract::DefaultBodyLimit;
use tower::ServiceBuilder;

let app = Router::new()
    .layer(ServiceBuilder::new()
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB
        .layer(cors));
```

### 5. Shared State Access Pattern

**Question**: How should the HTTP server access the shared RAG state?

**Decision**: Use `Arc<RwLock<RagState>>` passed to HTTP handlers via axum's `State` extractor

**Rationale**:
- Existing codebase already uses `RagState` wrapped in `Arc<RwLock<Option<RAG>>>`
- Axum's `State` extractor provides type-safe access to shared state
- `RwLock` allows concurrent reads (searches) while writes (document ingestion) are exclusive
- Matches existing Tauri IPC command pattern

**Implementation Pattern**:
```rust
use axum::{extract::State, Router};

#[derive(Clone)]
struct AppState {
    rag_state: RagState, // Arc<RwLock<Option<RAG>>>
}

let app = Router::new()
    .route("/documents", post(handle_post_documents))
    .with_state(AppState { rag_state });
```

### 6. Error Response Format

**Question**: How should errors be serialized to match the spec's JSON format?

**Decision**: Use custom error types implementing `IntoResponse` trait, returning `{ message: "..." }` format

**Rationale**:
- Spec requires consistent `{ message: "..." }` format for all errors
- Axum's `IntoResponse` trait allows custom error handling
- Can create error types for each HTTP status code (400, 413, 503, 500)
- Type-safe error handling prevents inconsistent formats

**Implementation Pattern**:
```rust
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
```

### 7. YouTube Transcript Integration

**Question**: How should YouTube transcript fetching integrate with the HTTP request handler?

**Decision**: Reuse existing `YouTubeProcessor` from `youtube.rs` module, call within request handler

**Rationale**:
- Existing `YouTubeProcessor::fetch_transcript()` already implements 30-second timeout logic
- No need to duplicate functionality
- Can call synchronously within async handler (transcript fetch is already async)
- Matches existing bookmark ingestion pattern

**Implementation Pattern**:
```rust
use crate::youtube::YouTubeProcessor;

if let Some(url) = &request.url {
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
```

### 8. Server Lifecycle Management

**Question**: How should the HTTP server be started and stopped alongside Tauri?

**Decision**: Start HTTP server in Tauri's `setup` function after RAG initialization, run on separate Tokio task

**Rationale**:
- Tauri's `setup` function runs before window creation
- Can spawn HTTP server task after RAG system initializes
- Server runs independently on Tokio runtime (already available)
- Graceful shutdown handled by Tokio when application exits
- Matches existing bookmark monitoring pattern (also spawned in setup)

**Implementation Pattern**:
```rust
tokio::spawn(async move {
    match init_rag_system().await {
        Ok(rag) => {
            // Store RAG in state...
            
            // Start HTTP server
            if let Err(e) = start_http_server(rag_state_clone.clone()).await {
                eprintln!("Failed to start HTTP server: {}", e);
            }
        }
        Err(e) => { /* handle error */ }
    }
});
```

## Technical Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| HTTP Server Library | axum 0.7+ | Best Tokio integration, clean API, built-in CORS |
| CORS Configuration | tower-http CorsLayer | Standard middleware, supports `*` origin |
| Port Conflict Handling | Sequential scan 3000-3010 | Simple, sufficient for localhost |
| Request Size Limit | axum DefaultBodyLimit | Built-in middleware, 10MB limit |
| State Access | Arc<RwLock<RagState>> via State | Matches existing pattern |
| Error Format | Custom IntoResponse impl | Consistent JSON format |
| YouTube Integration | Reuse YouTubeProcessor | Avoid duplication |
| Lifecycle | Tokio spawn in Tauri setup | Matches existing patterns |

## Dependencies to Add

```toml
[dependencies]
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }
```

## Open Questions Resolved

✅ HTTP server library choice → axum  
✅ CORS implementation → tower-http CorsLayer  
✅ Port conflict handling → Sequential scan with logging  
✅ Request size limiting → axum DefaultBodyLimit middleware  
✅ State sharing pattern → Arc<RwLock> via axum State  
✅ Error response format → Custom IntoResponse implementation  
✅ YouTube transcript integration → Reuse existing YouTubeProcessor  
✅ Server lifecycle → Tokio spawn in Tauri setup  

All technical decisions resolved. Ready for Phase 1 design.
