#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;
use tokio::sync::Mutex;
use localmind_rs::{
    db::Database,
    ollama::OllamaClient,
    rag::RagPipeline,
    bookmark::BookmarkMonitor,
    fetcher::WebFetcher
};
use serde::{Deserialize, Serialize};
use tauri::Manager;

// Global state for the RAG pipeline and bookmarks
type RagState = Arc<Mutex<Option<RagPipeline>>>;
type BookmarkState = Arc<Mutex<Option<BookmarkMonitor>>>;

#[derive(Serialize)]
struct BookmarkProgress {
    current: usize,
    total: usize,
    current_title: String,
    completed: bool,
}

#[derive(Serialize, Deserialize)]
struct SearchResponse {
    answer: String,
    sources: Vec<DocumentSourceResponse>,
}

#[derive(Serialize, Deserialize)]
struct DocumentSourceResponse {
    doc_id: i64,
    title: String,
    content_snippet: String,
    similarity: f32,
}

#[derive(Serialize, Deserialize)]
struct IngestRequest {
    title: String,
    content: String,
    url: Option<String>,
    source: String,
}

// Helper function to limit bookmark content to 2k characters (UTF-8 safe)
fn truncate_bookmark_content(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        content.to_string()
    } else {
        // Find a safe UTF-8 character boundary
        let mut boundary = max_chars;
        while boundary > 0 && !content.is_char_boundary(boundary) {
            boundary -= 1;
        }

        if boundary == 0 {
            // If we can't find a boundary, just return the truncation message
            format!("[Content truncated - unable to find safe UTF-8 boundary at {} chars]", max_chars)
        } else {
            format!("{}...\n[Content truncated at {} chars]", &content[..boundary], boundary)
        }
    }
}

#[derive(Serialize)]
struct SearchHitsResponse {
    query: String,
    sources: Vec<DocumentSourceResponse>,
    has_results: bool,
}

#[tauri::command]
async fn search_query(
    query: String,
    state: tauri::State<'_, RagState>
) -> Result<SearchResponse, String> {
    let rag_lock = state.lock().await;
    let rag = rag_lock
        .as_ref()
        .ok_or("RAG pipeline not initialized")?;

    let response = rag.query(&query).await
        .map_err(|e| format!("Search failed: {}", e))?;

    let sources = response.sources.into_iter().map(|s| DocumentSourceResponse {
        doc_id: s.doc_id,
        title: s.title,
        content_snippet: s.content_snippet,
        similarity: s.similarity,
    }).collect();

    Ok(SearchResponse {
        answer: response.answer,
        sources,
    })
}

#[tauri::command]
async fn search_hits(
    query: String,
    state: tauri::State<'_, RagState>
) -> Result<SearchHitsResponse, String> {
    let rag_lock = state.lock().await;
    let rag = rag_lock
        .as_ref()
        .ok_or("RAG pipeline not initialized")?;

    let sources = rag.get_search_hits(&query).await
        .map_err(|e| format!("Search failed: {}", e))?;

    let has_results = !sources.is_empty();
    let source_responses = sources.into_iter().map(|s| DocumentSourceResponse {
        doc_id: s.doc_id,
        title: s.title,
        content_snippet: s.content_snippet,
        similarity: s.similarity,
    }).collect();

    Ok(SearchHitsResponse {
        query,
        sources: source_responses,
        has_results,
    })
}

#[tauri::command]
async fn generate_response(
    query: String,
    context_sources: Vec<i64>, // Document IDs to use for context
    state: tauri::State<'_, RagState>
) -> Result<String, String> {
    let rag_lock = state.lock().await;
    let rag = rag_lock
        .as_ref()
        .ok_or("RAG pipeline not initialized")?;

    let answer = rag.generate_answer(&query, &context_sources).await
        .map_err(|e| format!("Generation failed: {}", e))?;

    Ok(answer)
}

#[tauri::command]
async fn ingest_document(
    request: IngestRequest,
    state: tauri::State<'_, RagState>
) -> Result<i64, String> {
    let mut rag_lock = state.lock().await;
    let rag = rag_lock
        .as_mut()
        .ok_or("RAG pipeline not initialized")?;

    let doc_id = rag.ingest_document(
        &request.title,
        &request.content,
        request.url.as_deref(),
        &request.source,
    ).await
    .map_err(|e| format!("Document ingestion failed: {}", e))?;

    Ok(doc_id)
}

#[tauri::command]
async fn start_bookmark_ingestion(
    window: tauri::Window,
    rag_state: tauri::State<'_, RagState>,
    bookmark_state: tauri::State<'_, BookmarkState>
) -> Result<String, String> {
    let bookmark_lock = bookmark_state.lock().await;
    let bookmark_monitor = bookmark_lock
        .as_ref()
        .ok_or("Bookmark monitor not available")?;

    // Get existing bookmarks
    let existing_bookmarks = bookmark_monitor.get_bookmarks_for_ingestion().await
        .map_err(|e| format!("Failed to get bookmarks: {}", e))?;

    if existing_bookmarks.is_empty() {
        return Ok("No bookmarks found to ingest".to_string());
    }

    let total = existing_bookmarks.len();
    println!("Starting bookmark ingestion: {} bookmarks", total);

    // Clone states for the background task
    let rag_state_clone = rag_state.inner().clone();
    let window_clone = window.clone();

    // Start bookmark ingestion in background
    tokio::spawn(async move {
        let fetcher = WebFetcher::new();
        let mut ingested_count = 0;

        for (index, (title, content, url, is_dead)) in existing_bookmarks.into_iter().enumerate() {
            // Send progress update to UI
            let progress = BookmarkProgress {
                current: index + 1,
                total,
                current_title: title.clone(),
                completed: false,
            };

            if let Err(e) = window_clone.emit("bookmark-progress", &progress) {
                eprintln!("Failed to emit progress: {}", e);
            }

            // Skip ingesting dead bookmarks
            if is_dead {
                println!("🚫 Skipping dead bookmark: {}", title);
                continue;
            }

            // Use the content already fetched by the bookmark monitor
            let final_content = truncate_bookmark_content(&content, 2000);

            // Ingest the bookmark with fetched content - acquire lock only for ingestion
            {
                let mut rag_lock = rag_state_clone.lock().await;
                if let Some(ref mut rag) = *rag_lock {
                    match rag.ingest_document(&title, &final_content, Some(&url), "chrome_bookmark").await {
                        Ok(_) => {
                            ingested_count += 1;
                        }
                        Err(e) => {
                            eprintln!("Failed to ingest bookmark '{}': {}", title, e);
                        }
                    }
                }
            } // Lock released here

            // Small delay to prevent overwhelming the system and allow searches
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Send completion notification
        let final_progress = BookmarkProgress {
            current: total,
            total,
            current_title: format!("Completed! {} bookmarks ingested", ingested_count),
            completed: true,
        };

        if let Err(e) = window_clone.emit("bookmark-progress", &final_progress) {
            eprintln!("Failed to emit completion: {}", e);
        }

        println!("Bookmark ingestion completed: {} bookmarks ingested", ingested_count);
    });

    Ok(format!("Started ingesting {} bookmarks", total))
}

#[tauri::command]
async fn get_bookmark_status() -> Result<serde_json::Value, String> {
    match BookmarkMonitor::get_chrome_bookmarks_path() {
        Ok(path) => {
            let monitor = BookmarkMonitor::default();
            match monitor.parse_bookmarks() {
                Ok(bookmarks) => Ok(serde_json::json!({
                    "available": true,
                    "path": path.to_string_lossy(),
                    "bookmark_count": bookmarks.len()
                })),
                Err(e) => Ok(serde_json::json!({
                    "available": false,
                    "path": path.to_string_lossy(),
                    "error": format!("Failed to parse bookmarks: {}", e)
                }))
            }
        }
        Err(e) => Ok(serde_json::json!({
            "available": false,
            "error": format!("Chrome bookmarks not found: {}", e)
        }))
    }
}

#[tauri::command]
async fn health_check(
    state: tauri::State<'_, RagState>
) -> Result<String, String> {
    let rag_lock = state.lock().await;
    if let Some(rag) = rag_lock.as_ref() {
        let (count, _) = rag.vector_store_stats();

        // Check model availability
        match rag.ollama().check_models_available().await {
            Ok((embedding_ok, completion_ok, available_models)) => {
                let (embedding_model, completion_model) = rag.ollama().get_model_names();

                if !embedding_ok || !completion_ok {
                    let mut missing = Vec::new();
                    if !embedding_ok {
                        missing.push(format!("❌ Embedding: {} (run: ollama pull {})",
                            embedding_model, embedding_model));
                    }
                    if !completion_ok {
                        missing.push(format!("❌ Completion: {} (run: ollama pull {})",
                            completion_model, completion_model));
                    }
                    return Ok(format!(
                        "⚠️  Missing models:\n{}\n\nAvailable: {:?}\n{} documents loaded",
                        missing.join("\n"),
                        available_models,
                        count
                    ));
                }

                Ok(format!(
                    "✅ RAG Pipeline OK\n📚 {} documents loaded\n🤖 Models: {} (embed), {} (chat)",
                    count, embedding_model, completion_model
                ))
            },
            Err(e) => {
                Ok(format!(
                    "⚠️  Ollama not connected: {}\nPlease run: ollama serve\n{} documents loaded",
                    e, count
                ))
            }
        }
    } else {
        Err("RAG pipeline not initialized".to_string())
    }
}

#[tauri::command]
async fn get_stats(
    state: tauri::State<'_, RagState>
) -> Result<serde_json::Value, String> {
    let rag_lock = state.lock().await;
    if let Some(rag) = rag_lock.as_ref() {
        let (count, is_empty) = rag.vector_store_stats();
        Ok(serde_json::json!({
            "document_count": count,
            "vector_store_empty": is_empty,
            "status": "ready"
        }))
    } else {
        Ok(serde_json::json!({
            "document_count": 0,
            "vector_store_empty": true,
            "status": "initializing"
        }))
    }
}

#[tokio::main]
async fn main() {
    // Initialize database
    let db = match Database::new().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            return;
        }
    };

    // Initialize Ollama client
    let ollama = OllamaClient::new("http://localhost:11434".to_string());

    // Check if required models are available
    match ollama.check_models_available().await {
        Ok((embedding_ok, completion_ok, available_models)) => {
            let (embedding_model, completion_model) = ollama.get_model_names();

            if !embedding_ok || !completion_ok {
                eprintln!("⚠️  WARNING: Required Ollama models are missing!");
                eprintln!("");
                if !embedding_ok {
                    eprintln!("❌ Embedding model '{}' is not installed", embedding_model);
                    eprintln!("   Please run: ollama pull {}", embedding_model);
                }
                if !completion_ok {
                    eprintln!("❌ Completion model '{}' is not installed", completion_model);
                    eprintln!("   Please run: ollama pull {}", completion_model);
                }
                eprintln!("");
                eprintln!("Available models: {:?}", available_models);
                eprintln!("");
                eprintln!("The application will start but search functionality will be limited until models are installed.");
            } else {
                println!("✅ Ollama models: OK (embedding: {}, completion: {})",
                    embedding_model, completion_model);
            }
        },
        Err(e) => {
            eprintln!("⚠️  WARNING: Cannot connect to Ollama service");
            eprintln!("   Error: {}", e);
            eprintln!("   Make sure Ollama is running: ollama serve");
            eprintln!("");
            eprintln!("The application will start but search functionality will not work.");
        }
    }

    println!("LocalMind Rust implementation initialized!");
    println!("Database: OK");

    // Initialize RAG pipeline
    let rag_pipeline = match RagPipeline::new(db, ollama).await {
        Ok(rag) => {
            println!("RAG Pipeline: OK");
            Some(rag)
        },
        Err(e) => {
            eprintln!("Failed to initialize RAG pipeline: {}", e);
            println!("RAG Pipeline: Failed (will retry)");
            None
        }
    };

    // Create shared state
    let rag_state: RagState = Arc::new(Mutex::new(rag_pipeline));

    // Initialize bookmark monitoring (but don't process existing bookmarks yet)
    let bookmark_state = if let Ok((bookmark_monitor, mut bookmark_rx)) = BookmarkMonitor::new() {
        println!("Chrome Bookmarks: Found (monitoring will start after UI)");

        // Clone the RAG state for the bookmark processing task
        let rag_state_clone = rag_state.clone();

        // Start the bookmark file watcher
        if let Err(e) = bookmark_monitor.start_monitoring().await {
            eprintln!("Failed to start bookmark monitoring: {}", e);
        }

        // Process bookmark updates in the background WITH PROPER LOCKING
        tokio::spawn(async move {
            let fetcher = WebFetcher::new();

            while let Some(bookmarks) = bookmark_rx.recv().await {
                println!("Received {} bookmark updates", bookmarks.len());
                let mut ingested_count = 0;

                for bookmark in bookmarks {
                    if let Some(url) = &bookmark.url {
                        let title = if bookmark.name.is_empty() {
                            url.clone()
                        } else {
                            bookmark.name.clone()
                        };

                        // Fetch actual page content
                        let content = match fetcher.fetch_page_content(url).await {
                            Ok(page_content) if !page_content.is_empty() => {
                                println!("✅ Using fetched content for: {}", title);
                                truncate_bookmark_content(&page_content, 2000)
                            }
                            _ => {
                                println!("⏭️ Using URL as content for: {}", title);
                                let fallback_content = format!("Bookmark: {}\nURL: {}", title, url);
                                truncate_bookmark_content(&fallback_content, 2000)
                            }
                        };

                        // Acquire lock ONLY for ingestion
                        {
                            let mut rag_lock = rag_state_clone.lock().await;
                            if let Some(ref mut rag) = *rag_lock {
                                match rag.ingest_document(&title, &content, Some(url), "chrome_bookmark").await {
                                    Ok(_) => {
                                        ingested_count += 1;
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to ingest bookmark '{}': {}", title, e);
                                    }
                                }
                            }
                        } // Lock released here

                        // Small delay to allow other operations
                        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    }
                }

                if ingested_count > 0 {
                    println!("Successfully ingested {} new bookmarks", ingested_count);
                }
            }
        });

        Some(bookmark_monitor)
    } else {
        println!("Chrome Bookmarks: Not found (monitoring disabled)");
        None
    };

    let bookmark_state: BookmarkState = Arc::new(Mutex::new(bookmark_state));

    tauri::Builder::default()
        .manage(rag_state.clone())
        .manage(bookmark_state.clone())
        .invoke_handler(tauri::generate_handler![
            search_query,
            search_hits,
            generate_response,
            ingest_document,
            start_bookmark_ingestion,
            get_bookmark_status,
            health_check,
            get_stats
        ])
        .setup(move |app| {
            // Start automatic bookmark processing after UI is ready
            let app_handle = app.handle();
            let rag_state_setup = rag_state.clone();
            let bookmark_state_setup = bookmark_state.clone();

            tokio::spawn(async move {
                // Small delay to ensure UI is fully loaded
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                let bookmark_lock = bookmark_state_setup.lock().await;
                if let Some(bookmark_monitor) = bookmark_lock.as_ref() {
                    if let Ok(existing_bookmarks) = bookmark_monitor.get_bookmarks_for_ingestion().await {
                        if !existing_bookmarks.is_empty() {
                            let total = existing_bookmarks.len();
                            println!("Auto-processing {} existing bookmarks...", total);

                            drop(bookmark_lock); // Release the bookmark lock before processing

                            // First check if models are available
                            let models_available = {
                                let rag_lock = rag_state_setup.lock().await;
                                if let Some(ref rag) = *rag_lock {
                                    match rag.ollama().check_models_available().await {
                                        Ok((embedding_ok, completion_ok, _)) => embedding_ok && completion_ok,
                                        Err(_) => false
                                    }
                                } else {
                                    false
                                }
                            }; // Release RAG lock

                            if !models_available {
                                eprintln!("❌ Cannot process bookmarks: Ollama models not available");
                                return;
                            }

                            let fetcher = WebFetcher::new();
                            let mut ingested_count = 0;

                            println!("🚀 Starting bookmark processing loop for {} bookmarks", total);

                            for (index, (title, _url_as_content, url)) in existing_bookmarks.into_iter().enumerate() {
                                println!("\n📌 Processing bookmark {}/{}: {} - {}", index + 1, total, title, url);

                                // Check if URL already exists in database
                                let should_process = {
                                    let mut rag_lock = rag_state_setup.lock().await;
                                    if let Some(ref mut rag) = *rag_lock {
                                        match rag.document_exists(&url).await {
                                            Ok(true) => {
                                                println!("⏭️  Skipping existing bookmark: {} - {}", title, url);
                                                false
                                            }
                                            Ok(false) => {
                                                println!("✅ New bookmark, proceeding with processing");
                                                true
                                            }
                                            Err(e) => {
                                                eprintln!("⚠️  Error checking URL existence for {}: {}. Processing anyway.", url, e);
                                                true
                                            }
                                        }
                                    } else {
                                        false
                                    }
                                }; // Release lock after checking

                                if !should_process {
                                    continue;
                                }

                                // Send progress update to UI
                                let progress = BookmarkProgress {
                                    current: index + 1,
                                    total,
                                    current_title: title.clone(),
                                    completed: false,
                                };

                                if let Err(e) = app_handle.emit_all("bookmark-progress", &progress) {
                                    eprintln!("Failed to emit progress: {}", e);
                                }

                                // Fetch actual page content
                                let content = match fetcher.fetch_page_content(&url).await {
                                    Ok(page_content) if !page_content.is_empty() => {
                                        truncate_bookmark_content(&page_content, 2000)
                                    }
                                    Err(_e) => {
                                        truncate_bookmark_content(&url, 2000)
                                    }
                                    Ok(_) => {
                                        truncate_bookmark_content(&url, 2000)
                                    }
                                };

                                // Ingest the bookmark with fetched content - acquire lock only for ingestion
                                {
                                    let mut rag_lock = rag_state_setup.lock().await;
                                    if let Some(ref mut rag) = *rag_lock {
                                        match rag.ingest_document(&title, &content, Some(&url), "chrome_bookmark").await {
                                            Ok(_) => {
                                                ingested_count += 1;
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to ingest bookmark '{}': {}", title, e);
                                            }
                                        }
                                    }
                                } // Release lock after ingestion

                                // Small delay to allow other operations like search
                                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                            }

                            // Send completion notification
                            let final_progress = BookmarkProgress {
                                current: total,
                                total,
                                current_title: format!("Completed! {} bookmarks ingested", ingested_count),
                                completed: true,
                            };

                            if let Err(e) = app_handle.emit_all("bookmark-progress", &final_progress) {
                                eprintln!("Failed to emit completion: {}", e);
                            }

                            println!("Auto-ingestion completed: {} bookmarks processed", ingested_count);
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}