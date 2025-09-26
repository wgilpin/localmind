// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use localmind_rs::{
    db::{Database, OperationPriority},
    rag::RagPipeline as RAG,
    bookmark::BookmarkMonitor,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, State, Window};
use tokio::sync::Mutex;

type RagState = Arc<Mutex<Option<RAG>>>;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SearchResult {
    id: i64,
    title: String,
    content: String,
    url: Option<String>,
    source: String,
    similarity_score: f32,
}

#[derive(Debug, Serialize)]
struct BookmarkProgress {
    current: usize,
    total: usize,
    current_title: String,
    completed: bool,
}

#[tauri::command]
async fn search_documents(
    query: String,
    state: State<'_, RagState>,
) -> Result<Vec<SearchResult>, String> {
    println!("📝 search_documents called with query: {}", query);
    let rag_lock = state.lock().await;
    let rag = rag_lock
        .as_ref()
        .ok_or("RAG system not initialized")?;

    let results = rag
        .search(&query, 10)
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    let search_results = results
        .into_iter()
        .map(|(doc, score)| SearchResult {
            id: doc.id,
            title: doc.title,
            content: doc.content,
            url: doc.url,
            source: doc.source,
            similarity_score: score,
        })
        .collect();

    Ok(search_results)
}

#[tauri::command]
async fn get_document_count(state: State<'_, RagState>) -> Result<i64, String> {
    println!("📝 get_document_count called");
    let rag_lock = state.lock().await;
    let rag = rag_lock
        .as_ref()
        .ok_or("RAG system not initialized")?;

    rag.db
        .count_documents(OperationPriority::UserSearch)
        .await
        .map_err(|e| format!("Failed to count documents: {}", e))
}

#[tauri::command]
async fn chat_with_rag(
    message: String,
    state: State<'_, RagState>,
) -> Result<String, String> {
    println!("📝 chat_with_rag called with message: {}", message);
    let rag_lock = state.lock().await;
    let rag = rag_lock
        .as_ref()
        .ok_or("RAG system not initialized")?;

    rag.chat(&message)
        .await
        .map_err(|e| format!("Chat failed: {}", e))
}

#[tauri::command]
async fn add_document(
    title: String,
    content: String,
    url: Option<String>,
    source: String,
    state: State<'_, RagState>,
) -> Result<String, String> {
    println!("📝 add_document called for: {}", title);
    let mut rag_lock = state.lock().await;
    let rag = rag_lock
        .as_mut()
        .ok_or("RAG system not initialized")?;

    rag.ingest_document(&title, &content, url.as_deref(), &source)
        .await
        .map_err(|e| format!("Failed to add document: {}", e))?;

    Ok(format!("Document '{}' added successfully", title))
}

#[tauri::command]
async fn ingest_bookmarks(
    window: Window,
    state: State<'_, RagState>,
) -> Result<String, String> {
    println!("📝 ingest_bookmarks called");
    // Initialize bookmark monitor
    let monitor = BookmarkMonitor::new()
        .map_err(|e| format!("Failed to initialize bookmark monitor: {}", e))?
        .0;

    // Get bookmarks for ingestion (with readability-processed content)
    let existing_bookmarks = monitor
        .get_bookmarks_for_ingestion()
        .await
        .map_err(|e| format!("Failed to get bookmarks: {}", e))?;

    let total = existing_bookmarks.len();
    println!("Starting bookmark ingestion: {} bookmarks", total);

    // Clone states for the background task
    let rag_state_clone = state.inner().clone();
    let window_clone = window.clone();

    // Start bookmark ingestion in background
    tokio::spawn(async move {
        let mut ingested_count = 0;

        for (index, (title, content, url, _is_dead)) in existing_bookmarks.into_iter().enumerate() {
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

            {
                let mut rag_lock = rag_state_clone.lock().await;
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
async fn get_ollama_models() -> Result<Vec<String>, String> {
    println!("📝 get_ollama_models called");
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

    #[derive(Deserialize)]
    struct Model {
        name: String,
    }

    #[derive(Deserialize)]
    struct ModelsResponse {
        models: Vec<Model>,
    }

    let models_response: ModelsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse models: {}", e))?;

    let model_names: Vec<String> = models_response
        .models
        .into_iter()
        .map(|m| m.name)
        .collect();

    Ok(model_names)
}

#[derive(Debug, Serialize)]
struct SystemStats {
    document_count: i64,
    status: String,
}

#[tauri::command]
async fn get_stats(state: State<'_, RagState>) -> Result<SystemStats, String> {
    println!("📝 get_stats called");
    let rag_lock = state.lock().await;

    match rag_lock.as_ref() {
        Some(rag) => {
            println!("  ✅ RAG is initialized, getting doc count");
            let doc_count = rag.db
                .count_documents(OperationPriority::UserSearch)
                .await
                .unwrap_or(0);

            println!("  📊 Document count: {}", doc_count);
            Ok(SystemStats {
                document_count: doc_count,
                status: if doc_count > 0 { "ready".to_string() } else { "empty".to_string() },
            })
        }
        None => {
            println!("  ⏳ RAG not initialized yet");
            Ok(SystemStats {
                document_count: 0,
                status: "initializing".to_string(),
            })
        }
    }
}

#[derive(Debug, Serialize)]
struct SearchHitResult {
    has_results: bool,
    query: String,
    sources: Vec<SearchSource>,
}

#[derive(Debug, Serialize)]
struct SearchSource {
    doc_id: i64,
    title: String,
    content_snippet: String,
    similarity: f32,
}

#[tauri::command]
async fn search_hits(
    query: String,
    state: State<'_, RagState>,
) -> Result<SearchHitResult, String> {
    println!("📝 search_hits called with query: {}", query);
    let rag_lock = state.lock().await;
    let rag = rag_lock
        .as_ref()
        .ok_or("RAG system not initialized")?;

    let hits = rag
        .get_search_hits(&query)
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    let sources: Vec<SearchSource> = hits
        .into_iter()
        .map(|hit| SearchSource {
            doc_id: hit.doc_id,
            title: hit.title,
            content_snippet: hit.content_snippet,
            similarity: hit.similarity,
        })
        .collect();

    Ok(SearchHitResult {
        has_results: !sources.is_empty(),
        query,
        sources,
    })
}

#[tauri::command]
async fn generate_response(
    query: String,
    context_sources: Vec<i64>,
    state: State<'_, RagState>,
) -> Result<String, String> {
    println!("📝 generate_response called with query: {}", query);
    let rag_lock = state.lock().await;
    let rag = rag_lock
        .as_ref()
        .ok_or("RAG system not initialized")?;

    rag.generate_answer(&query, &context_sources)
        .await
        .map_err(|e| format!("Failed to generate response: {}", e))
}

fn main() {
    println!("🚀 Starting LocalMind application");

    // Create the runtime for the entire application
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build runtime");

    println!("✅ Tokio runtime created");

    // Enter the runtime context
    let _guard = runtime.enter();

    // Build and run the Tauri app
    tauri::Builder::default()
        .manage(RagState::default())
        .setup(move |app| {
            println!("🔧 Tauri setup starting");
            let rag_state = app.state::<RagState>();
            let rag_state_clone = rag_state.inner().clone();

            // Try to get the main window
            let _window = match app.get_window("main") {
                Some(w) => {
                    println!("✅ Got main window");
                    w
                },
                None => {
                    eprintln!("❌ Could not get main window!");
                    return Err("Could not get main window".into());
                }
            };

            // Initialize RAG system in the background using tokio::spawn directly
            tokio::spawn(async move {
                println!("🚀 Starting RAG initialization task");

                match init_rag_system().await {
                    Ok(rag) => {
                        println!("✅ RAG system initialized successfully");
                        {
                            let mut rag_lock = rag_state_clone.lock().await;
                            *rag_lock = Some(rag);
                            println!("✅ RAG stored in state");
                        }

                        // Start automatic bookmark monitoring
                        println!("📚 Starting automatic bookmark monitoring...");
                        if let Err(e) = start_bookmark_monitoring(rag_state_clone.clone(), _window).await {
                            eprintln!("❌ Failed to start bookmark monitoring: {}", e);
                        } else {
                            println!("✅ Bookmark monitoring started successfully");
                        }
                        println!("🎉 Initialization complete - system ready");
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to initialize RAG system: {}", e);
                        eprintln!("Debug info:");
                        eprintln!("  - Current directory: {:?}", std::env::current_dir());
                        eprintln!("  - Data directory: {:?}", dirs::data_dir());
                    }
                }
            });

            println!("✅ Tauri setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_documents,
            get_document_count,
            chat_with_rag,
            add_document,
            get_ollama_models,
            get_stats,
            search_hits,
            generate_response,
            ingest_bookmarks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn init_rag_system() -> Result<RAG, Box<dyn std::error::Error + Send + Sync>> {
    use localmind_rs::{db::Database, ollama::OllamaClient};

    println!("📁 Initializing database...");

    // Initialize database with error handling
    let db = match Database::new().await {
        Ok(database) => {
            println!("✅ Database initialized successfully");
            database
        },
        Err(e) => {
            eprintln!("❌ Database initialization failed: {}", e);
            return Err(e.into());
        }
    };

    println!("🤖 Initializing Ollama client...");
    // Initialize Ollama client
    let ollama_client = OllamaClient::new("http://localhost:11434".to_string());
    println!("✅ Ollama client initialized");

    println!("🔧 Initializing RAG pipeline...");
    // Initialize RAG pipeline
    let rag = match RAG::new(db, ollama_client).await {
        Ok(rag_pipeline) => {
            println!("✅ RAG pipeline initialized successfully");
            rag_pipeline
        },
        Err(e) => {
            eprintln!("❌ RAG pipeline initialization failed: {}", e);
            return Err(e.into());
        }
    };

    Ok(rag)
}

async fn start_bookmark_monitoring(
    rag_state: RagState,
    window: Window,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔗 Initializing bookmark monitor...");
    // Initialize bookmark monitor
    let (monitor, mut rx) = BookmarkMonitor::new()?;

    println!("👀 Starting file system monitoring...");
    // Start monitoring for changes
    monitor.start_monitoring().await?;

    println!("📖 Getting existing bookmarks...");
    // Get bookmark metadata only (no content fetching yet)
    let bookmark_metadata = monitor.get_bookmarks_metadata().await?;
    if !bookmark_metadata.is_empty() {
        println!("🚀 DEBUG: About to start event emission loop with {} bookmarks", bookmark_metadata.len());
        println!("📚 Processing {} existing bookmarks WITH EVENTS...", bookmark_metadata.len());

        let total = bookmark_metadata.len();
        let mut ingested_count = 0;

        for (index, (title, url)) in bookmark_metadata.into_iter().enumerate() {
            // Send progress update to UI
            let progress = BookmarkProgress {
                current: index + 1,
                total,
                current_title: title.clone(),
                completed: false,
            };

            if let Err(e) = window.emit("bookmark-progress", &progress) {
                eprintln!("❌ Failed to emit progress: {}", e);
            } else {
                println!("✅ Successfully emitted progress event: {}/{} - {}", index + 1, total, title);
            }

            {
                let mut rag_lock = rag_state.lock().await;
                if let Some(ref mut rag) = *rag_lock {
                    // Check if bookmark already exists
                    if !rag.document_exists(&url).await.unwrap_or(false) {
                        // Fetch content here where we have access to window for progress
                        println!("🌐 Fetching content for: {}", title);
                        let content = match monitor.fetch_bookmark_content(&url).await {
                            Ok(content) => content,
                            Err(e) => {
                                eprintln!("❌ Failed to fetch content for '{}': {}", title, e);
                                format!("Bookmark: {}\nURL: {}\n\n[Error fetching content: {}]", title, url, e)
                            }
                        };

                        match rag.ingest_document(&title, &content, Some(&url), "chrome_bookmark").await {
                            Ok(_) => {
                                ingested_count += 1;
                                println!("✅ Ingested bookmark: {}", title);
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to ingest bookmark '{}': {}", title, e);
                            }
                        }
                    } else {
                        println!("⏭️ Skipping existing bookmark: {}", title);
                    }
                }
            }

            // Small delay to prevent overwhelming the system
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Send completion notification
        let final_progress = BookmarkProgress {
            current: total,
            total,
            current_title: format!("Completed! {} new bookmarks ingested", ingested_count),
            completed: true,
        };

        if let Err(e) = window.emit("bookmark-progress", &final_progress) {
            eprintln!("Failed to emit completion: {}", e);
        }

        println!("✅ Initial bookmark ingestion completed: {} bookmarks ingested", ingested_count);
    } else {
        println!("📚 No existing bookmarks found");
    }

    // Listen for bookmark changes
    println!("👂 Starting bookmark change listener...");
    tokio::spawn(async move {
        while let Some(updated_bookmarks) = rx.recv().await {
            println!("📚 Detected bookmark changes, processing {} bookmarks...", updated_bookmarks.len());
            // Process updated bookmarks similar to above
            // ... (for now just log the change)
        }
    });

    Ok(())
}