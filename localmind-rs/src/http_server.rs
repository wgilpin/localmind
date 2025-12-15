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
use regex::Regex;
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

/// Cleans Google Docs mobile basic view content by removing JavaScript, CSS, and HTML artifacts
///
/// The mobile basic view includes inline JavaScript error handling, CSS imports, and CSS styles
/// that pollute the text extraction. This function strips those artifacts to extract clean text.
///
/// # Arguments
/// * `content` - Raw content from Google Docs mobile basic view
///
/// # Returns
/// * Cleaned text content with JS/CSS artifacts removed
fn clean_google_docs_content(content: &str) -> String {
    let mut cleaned = content.to_string();

    // Strategy: Conservatively remove only clearly identifiable CSS/JS artifacts
    // Google Docs mobile basic view structure:
    // 1. JavaScript error handling (if ((!this['DOCS_initDocsMobileWeb'])...)
    // 2. DOCS_initDocsMobileWeb(...args...) call
    // 3. CSS imports and styles
    // 4. Actual document content

    // Remove everything up to and including the DOCS_initDocsMobileWeb call
    // This is safe because it's always JavaScript initialization code
    if let Some(init_pos) = cleaned.find("DOCS_initDocsMobileWeb(") {
        if let Some(close_paren) = cleaned[init_pos..].find(");") {
            cleaned.replace_range(0..init_pos + close_paren + 2, "");
        }
    }

    // Remove CSS imports (@import url(...);) - these are always at the top
    let css_import_re = Regex::new(r"@import\s+url\([^)]+\);?").unwrap();
    cleaned = css_import_re.replace_all(&cleaned, "").to_string();

    // Remove ALL CSS blocks - be aggressive since we know Google Docs mobile view has lots of CSS
    // Match any CSS selector followed by braces with CSS properties
    // This catches: ul.lst-kix_list_b-8{list-style-type:none}, .class{property:value}, etc.
    let css_block_re = Regex::new(r"[\.\#\w\-]+\s*\{[^}]*\}").unwrap();
    cleaned = css_block_re.replace_all(&cleaned, "").to_string();

    // Remove CSS selectors with child combinators (e.g., ".lst-kix_list_13-0 > li{")
    let css_child_re = Regex::new(r"\.[\w\-]+\s*>\s*[\w\-]+\s*\{[^}]*\}").unwrap();
    cleaned = css_child_re.replace_all(&cleaned, "").to_string();

    // Remove list style counter rules with :before pseudo-elements
    let css_before_re = Regex::new(r"\.[\w\-]+\s*>\s*[^{]*:before\s*\{[^}]*\}").unwrap();
    cleaned = css_before_re.replace_all(&cleaned, "").to_string();

    // Remove standalone CSS selectors that appear before text (e.g., ".lst-kix_list_c-0 >")
    // Match the pattern and the capital letter, then replace with just the capital
    let css_selector_re = Regex::new(r"\.lst-kix_[\w\-]+\s*>\s+([A-Z])").unwrap();
    cleaned = css_selector_re.replace_all(&cleaned, "$1").to_string();

    // Remove any remaining CSS-like patterns that start with ul., ol., .lst-kix, etc.
    let css_list_re = Regex::new(r"(?:ul|ol)\.lst-kix_[\w\-]+").unwrap();
    cleaned = css_list_re.replace_all(&cleaned, "").to_string();

    // Remove any remaining .lst-kix patterns
    let css_lst_kix_re = Regex::new(r"\.lst-kix_[\w\-]+").unwrap();
    cleaned = css_lst_kix_re.replace_all(&cleaned, "").to_string();

    // Remove setTimeout and other window. JavaScript calls
    let js_call_re = Regex::new(r"window\.[a-zA-Z]+\([^)]*\);?").unwrap();
    cleaned = js_call_re.replace_all(&cleaned, "").to_string();

    // Remove counter-reset and counter-increment rules (these are CSS-only)
    let counter_re = Regex::new(r"counter-(?:reset|increment):\s*[^;}]+[;}]").unwrap();
    cleaned = counter_re.replace_all(&cleaned, "").to_string();

    // Remove CSS properties that appear standalone (not in blocks)
    // Only match if they look like CSS (property: value; format)
    let css_prop_re = Regex::new(r"^\s*[a-z\-]+:\s*[^;]+;\s*$").unwrap();
    cleaned = css_prop_re.replace_all(&cleaned, "").to_string();

    // Find where actual document content starts
    // Look for common document patterns that indicate real content
    let content_markers = [
        "Architecting Intelligence",
        "The State of",
        "Executive Summary",
        "Abstract",
        "Introduction",
        "Section 1",
        "Part I",
        "Chapter 1",
    ];

    // Find the earliest occurrence of a content marker
    let mut content_start = None;
    for marker in &content_markers {
        if let Some(pos) = cleaned.find(marker) {
            // Only use if it's reasonably early (first 5000 chars) to avoid false positives
            if pos < 5000 {
                content_start = Some(content_start.map_or(pos, |current: usize| current.min(pos)));
            }
        }
    }

    // If we found a content marker, remove everything before it
    // But first check that what we're removing is actually junk
    if let Some(start_pos) = content_start {
        if start_pos > 0 {
            let leading_text = &cleaned[..start_pos];
            // Check if leading text is mostly CSS/JS junk
            let has_css_js = leading_text.contains("lst-kix")
                || leading_text.contains("list-style-type")
                || leading_text.contains("DOCS_")
                || leading_text.contains("@import")
                || leading_text.contains("window.")
                || (leading_text.matches('{').count() > 5 && leading_text.matches('}').count() > 5);

            // Also check if it's mostly non-alphabetic (CSS/JS is mostly punctuation)
            let alpha_count = leading_text.chars().filter(|c| c.is_alphabetic()).count();
            let is_mostly_junk = leading_text.len() > 50 && alpha_count < leading_text.len() / 3;

            if has_css_js || is_mostly_junk {
                cleaned.replace_range(0..start_pos, "");
            }
        }
    }

    // Clean up excessive whitespace (3 or more spaces/newlines → 2 newlines)
    let whitespace_re = Regex::new(r"\s{3,}").unwrap();
    cleaned = whitespace_re.replace_all(&cleaned, "\n\n").to_string();

    // Remove empty lines at the start and end, but preserve content
    let empty_lines_start_re = Regex::new(r"^\s*\n+").unwrap();
    cleaned = empty_lines_start_re.replace(&cleaned, "").to_string();
    let empty_lines_end_re = Regex::new(r"\n+\s*$").unwrap();
    cleaned = empty_lines_end_re.replace(&cleaned, "").to_string();

    // Trim and return - but don't truncate!
    cleaned.trim().to_string()
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

    // Clean Google Docs content if it's from mobile basic view
    if extraction_method.contains("google-docs") {
        println!(
            "Cleaning Google Docs content ({} chars before cleaning)",
            content.len()
        );
        println!(
            "First 500 chars before cleaning: {}",
            content.chars().take(500).collect::<String>()
        );
        println!(
            "Last 500 chars before cleaning: {}",
            content
                .chars()
                .rev()
                .take(500)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        );
        let original_len = content.len();
        content = clean_google_docs_content(&content);
        println!(
            "Google Docs content cleaned ({} chars after cleaning, lost {} chars)",
            content.len(),
            original_len.saturating_sub(content.len())
        );
        println!(
            "First 500 chars after cleaning: {}",
            content.chars().take(500).collect::<String>()
        );
        println!(
            "Last 500 chars after cleaning: {}",
            content
                .chars()
                .rev()
                .take(500)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        );
    }

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
