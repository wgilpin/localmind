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
};
use serde::{Deserialize, Serialize};

// Global state for the RAG pipeline
type RagState = Arc<Mutex<Option<RagPipeline>>>;

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
    
    tauri::Builder::default()
        .manage(rag_state)
        .invoke_handler(tauri::generate_handler![
            search_query,
            search_hits,
            generate_response,
            ingest_document,
            health_check,
            get_stats
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
