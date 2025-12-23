//! Main application state and eframe App implementation

use crate::db::Database;
use crate::rag::RagPipeline;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::state::{
    BookmarkFolderView, DocumentView, InitStatus, SearchResultView, Toast, ToastType, View,
};
use super::views;
use super::widgets;

/// Type alias for shared RAG state
pub type RagState = Arc<RwLock<Option<RagPipeline>>>;

/// Main application state holding all UI and backend references
pub struct LocalMindApp {
    /// Shared reference to backend RAG pipeline
    pub rag: RagState,

    /// Active view (Home, SearchResults, DocumentDetail)
    pub current_view: View,

    /// Current search input text
    pub search_query: String,

    /// Cached search results (filtered by cutoff)
    pub search_results: Vec<SearchResultView>,

    /// All results before filtering
    pub all_results: Vec<SearchResultView>,

    /// Current similarity threshold (0.0-1.0)
    pub similarity_cutoff: f32,

    /// Currently viewed document
    pub selected_document: Option<DocumentView>,

    /// Recent documents for home screen
    pub recent_documents: Vec<DocumentView>,

    /// Settings modal visibility
    pub settings_open: bool,

    /// Folder IDs marked for exclusion
    pub excluded_folders: HashSet<String>,

    /// Domain patterns for exclusion
    pub excluded_domains: Vec<String>,

    /// Domain input field text
    pub pending_domain: String,

    /// Folder tree for settings
    pub bookmark_folders: Vec<BookmarkFolderView>,

    /// Active toast notifications
    pub toasts: Vec<Toast>,

    /// Application initialization state
    pub init_status: InitStatus,

    /// Counter for generating unique toast IDs
    pub next_toast_id: u64,

    /// Tokio runtime handle for async operations (kept alive)
    #[allow(dead_code)]
    runtime: tokio::runtime::Handle,

    /// Receiver for RAG initialization completion (just a success/error signal)
    init_receiver: Option<std::sync::mpsc::Receiver<Result<(), String>>>,

    /// Receiver for recent documents
    recent_docs_receiver: Option<std::sync::mpsc::Receiver<Vec<DocumentView>>>,

    /// Receiver for search results
    search_receiver: Option<std::sync::mpsc::Receiver<Vec<SearchResultView>>>,

    /// Receiver for document loading
    document_receiver: Option<std::sync::mpsc::Receiver<Option<DocumentView>>>,

    /// Previous view for back navigation
    previous_view: View,

    /// Receiver for exclusion rules loading
    exclusion_rules_receiver: Option<std::sync::mpsc::Receiver<(Vec<String>, Vec<String>)>>,

    /// Receiver for saving exclusion rules
    save_exclusion_receiver: Option<std::sync::mpsc::Receiver<Result<usize, String>>>,

    /// Receiver for bookmark progress events
    bookmark_progress_receiver: Option<std::sync::mpsc::Receiver<BookmarkProgress>>,

    /// ID of the current bookmark progress toast (for replacing)
    bookmark_progress_toast_id: Option<u64>,
}

/// Bookmark ingestion progress event
///
/// Sent through a channel to update the UI during bookmark processing.
#[derive(Debug, Clone)]
pub struct BookmarkProgress {
    pub current: usize,
    pub total: usize,
    pub current_title: String,
    pub completed: bool,
}

/// Build a tree structure from a flat list of folders based on their paths
fn build_folder_tree(mut folders: Vec<BookmarkFolderView>) -> Vec<BookmarkFolderView> {
    // Sort by path depth (shallowest first) so parents are processed before children
    folders.sort_by_key(|f| f.path.len());

    let mut tree: Vec<BookmarkFolderView> = Vec::new();

    for folder in folders {
        let folder_path = folder.path.clone();

        if folder_path.len() == 1 {
            // Root level folder
            tree.push(folder);
        } else {
            // Find parent
            let parent_path: Vec<String> = folder_path[..folder_path.len() - 1].to_vec();

            // Find parent in tree (recursively if needed)
            if let Some(parent_folder) = find_folder_by_path(&mut tree, &parent_path) {
                parent_folder.children.push(folder);
            } else {
                // Parent not found, add as root (shouldn't happen with sorted list, but handle gracefully)
                tree.push(folder);
            }
        }
    }

    tree
}

/// Find a folder in the tree by its path
fn find_folder_by_path<'a>(
    tree: &'a mut [BookmarkFolderView],
    path: &[String],
) -> Option<&'a mut BookmarkFolderView> {
    if path.is_empty() {
        return None;
    }

    // Find root level folder
    let root_name = &path[0];
    for folder in tree.iter_mut() {
        if folder.name == *root_name && folder.path.len() == 1 {
            // Found root, now navigate to nested path if needed
            if path.len() == 1 {
                return Some(folder);
            } else {
                return find_folder_in_children(folder, &path[1..]);
            }
        }
    }

    None
}

/// Find a folder in children by path
fn find_folder_in_children<'a>(
    folder: &'a mut BookmarkFolderView,
    path: &[String],
) -> Option<&'a mut BookmarkFolderView> {
    if path.is_empty() {
        return Some(folder);
    }

    let target_name = &path[0];
    for child in folder.children.iter_mut() {
        if child.name == *target_name {
            if path.len() == 1 {
                return Some(child);
            } else {
                return find_folder_in_children(child, &path[1..]);
            }
        }
    }

    None
}

impl LocalMindApp {
    /// Create a new LocalMindApp instance with backend initialization
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        println!("LocalMindApp::new() starting");

        // Create tokio runtime for async operations
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        let runtime_handle = runtime.handle().clone();

        // Create shared RAG state
        let rag_state: RagState = Arc::new(RwLock::new(None));
        let rag_state_clone = rag_state.clone();

        // Create channel for RAG initialization notification
        let (init_tx, init_rx) = std::sync::mpsc::channel();

        // Create channel for bookmark progress
        let (bookmark_progress_tx, bookmark_progress_rx) = std::sync::mpsc::channel();

        // Spawn RAG initialization in background
        let ctx = cc.egui_ctx.clone();
        let bookmark_progress_tx_clone = bookmark_progress_tx.clone();
        let runtime_handle_for_bookmarks = runtime_handle.clone();
        runtime_handle.spawn(async move {
            println!("Starting RAG initialization task");

            match init_rag_system().await {
                Ok(rag) => {
                    println!("RAG system initialized successfully");
                    {
                        let mut rag_lock = rag_state_clone.write().await;
                        *rag_lock = Some(rag);
                        println!("RAG stored in state");
                    }

                    // Signal success
                    let _ = init_tx.send(Ok(()));

                    // Start bookmark monitoring with progress reporting
                    let rag_for_bookmarks = rag_state_clone.clone();
                    let bookmark_progress_tx_for_monitor = bookmark_progress_tx_clone.clone();
                    runtime_handle_for_bookmarks.spawn(async move {
                        if let Err(e) = start_bookmark_monitoring(
                            rag_for_bookmarks,
                            bookmark_progress_tx_for_monitor,
                        )
                        .await
                        {
                            eprintln!("Failed to start bookmark monitoring: {}", e);
                        }
                    });

                    // Request repaint to update UI
                    ctx.request_repaint();
                }
                Err(e) => {
                    eprintln!("Failed to initialize RAG system: {}", e);
                    let _ = init_tx.send(Err(e.to_string()));
                    ctx.request_repaint();
                }
            }
        });

        // Spawn HTTP server in background
        let rag_state_for_http = rag_state.clone();
        runtime_handle.spawn(async move {
            // Wait a moment for RAG to initialize
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if let Err(e) = start_http_server(rag_state_for_http).await {
                eprintln!("Failed to start HTTP server: {}", e);
            }
        });

        // Keep runtime alive by storing it
        // We leak the runtime intentionally - it will live for the app's lifetime
        std::mem::forget(runtime);

        println!("LocalMindApp::new() complete");

        Self {
            rag: rag_state,
            current_view: View::Home,
            search_query: String::new(),
            search_results: Vec::new(),
            all_results: Vec::new(),
            similarity_cutoff: 0.3,
            selected_document: None,
            recent_documents: Vec::new(),
            settings_open: false,
            excluded_folders: HashSet::new(),
            excluded_domains: Vec::new(),
            pending_domain: String::new(),
            bookmark_folders: Vec::new(),
            toasts: Vec::new(),
            init_status: InitStatus::WaitingForEmbedding,
            next_toast_id: 0,
            runtime: runtime_handle,
            init_receiver: Some(init_rx),
            recent_docs_receiver: None,
            search_receiver: None,
            document_receiver: None,
            previous_view: View::Home,
            bookmark_progress_receiver: Some(bookmark_progress_rx),
            bookmark_progress_toast_id: None,
            exclusion_rules_receiver: None,
            save_exclusion_receiver: None,
        }
    }

    /// Add a toast notification
    pub fn add_toast(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    /// Remove expired toasts
    pub fn cleanup_toasts(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Get next unique toast ID
    pub fn next_toast_id(&mut self) -> u64 {
        let id = self.next_toast_id;
        self.next_toast_id += 1;
        id
    }

    /// Check for RAG initialization completion
    fn check_init_status(&mut self) {
        if let Some(ref rx) = self.init_receiver {
            match rx.try_recv() {
                Ok(Ok(_)) => {
                    println!("RAG initialization confirmed");
                    self.init_status = InitStatus::Ready;
                    self.init_receiver = None;

                    // Add success toast
                    let id = self.next_toast_id();
                    self.add_toast(Toast::success(id, "System ready"));

                    // Trigger loading recent documents
                    self.load_recent_documents();
                }
                Ok(Err(e)) => {
                    eprintln!("RAG initialization failed: {}", e);
                    self.init_status = InitStatus::Error(e.clone());
                    self.init_receiver = None;

                    // Add error toast
                    let id = self.next_toast_id();
                    self.add_toast(Toast::error(id, format!("Initialization failed: {}", e)));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still waiting
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Channel closed unexpectedly
                    self.init_status =
                        InitStatus::Error("Initialization channel closed".to_string());
                    self.init_receiver = None;
                }
            }
        }
    }

    /// Load recent documents for home screen
    fn load_recent_documents(&mut self) {
        if self.recent_docs_receiver.is_some() {
            return; // Already loading
        }

        let rag = self.rag.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let runtime_handle = self.runtime.clone();

        runtime_handle.spawn(async move {
            let rag_lock = rag.read().await;
            let docs = if let Some(ref rag) = *rag_lock {
                match rag.db.get_recent_documents(10).await {
                    Ok(docs) => docs
                        .into_iter()
                        .map(|doc| DocumentView {
                            id: doc.id,
                            title: doc.title,
                            content: strip_html(&doc.content),
                            url: doc.url,
                            source: doc.source,
                            created_at: doc.created_at,
                        })
                        .collect(),
                    Err(e) => {
                        eprintln!("Failed to load recent documents: {}", e);
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            };
            let _ = tx.send(docs);
        });

        self.recent_docs_receiver = Some(rx);
    }

    /// Check if recent documents have loaded
    fn check_recent_documents(&mut self) {
        if let Some(ref rx) = self.recent_docs_receiver {
            match rx.try_recv() {
                Ok(docs) => {
                    self.recent_documents = docs;
                    println!("Loaded {} recent documents", self.recent_documents.len());
                    self.recent_docs_receiver = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still loading
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Channel closed, clear receiver
                    self.recent_docs_receiver = None;
                }
            }
        }
    }

    /// Trigger a search with the current query
    pub fn trigger_search(&mut self) {
        let query = self.search_query.trim().to_string();
        if query.is_empty() {
            return;
        }

        if self.search_receiver.is_some() {
            return; // Already searching
        }

        println!("Triggering search for: {}", query);

        let rag = self.rag.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let runtime_handle = self.runtime.clone();

        runtime_handle.spawn(async move {
            let rag_lock = rag.read().await;
            let results = if let Some(ref rag) = *rag_lock {
                match rag.get_search_hits_with_cutoff(&query, 0.0).await {
                    Ok(hits) => {
                        hits.into_iter()
                            .map(|hit| SearchResultView {
                                doc_id: hit.doc_id,
                                title: hit.title,
                                snippet: create_snippet(&hit.content_snippet, 200),
                                similarity: hit.similarity,
                                url: None, // URL not in SearchHit, will need to fetch from doc
                            })
                            .collect()
                    }
                    Err(e) => {
                        eprintln!("Search failed: {}", e);
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            };
            let _ = tx.send(results);
        });

        self.search_receiver = Some(rx);
        self.current_view = View::SearchResults;
    }

    /// Check if search results have arrived
    fn check_search_results(&mut self) {
        if let Some(ref rx) = self.search_receiver {
            match rx.try_recv() {
                Ok(results) => {
                    println!("Search returned {} results", results.len());
                    self.all_results = results;
                    // Filter by current cutoff
                    self.search_results = self
                        .all_results
                        .iter()
                        .filter(|r| r.similarity >= self.similarity_cutoff)
                        .cloned()
                        .collect();
                    self.search_receiver = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still searching
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Channel closed, clear receiver
                    self.search_receiver = None;
                }
            }
        }
    }

    /// Check if a search is in progress
    pub fn is_search_pending(&self) -> bool {
        self.search_receiver.is_some()
    }

    /// Load a document by ID for viewing
    pub fn load_document(&mut self, doc_id: i64) {
        if self.document_receiver.is_some() {
            return; // Already loading
        }

        println!("Loading document: {}", doc_id);

        let rag = self.rag.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let runtime_handle = self.runtime.clone();

        runtime_handle.spawn(async move {
            let rag_lock = rag.read().await;
            let doc = if let Some(ref rag) = *rag_lock {
                match rag.db.get_document(doc_id).await {
                    Ok(Some(doc)) => Some(DocumentView {
                        id: doc.id,
                        title: doc.title,
                        content: strip_html(&doc.content),
                        url: doc.url,
                        source: doc.source,
                        created_at: doc.created_at,
                    }),
                    Ok(None) => {
                        eprintln!("Document not found: {}", doc_id);
                        None
                    }
                    Err(e) => {
                        eprintln!("Failed to load document: {}", e);
                        None
                    }
                }
            } else {
                None
            };
            let _ = tx.send(doc);
        });

        self.document_receiver = Some(rx);
        self.previous_view = self.current_view.clone();
        self.current_view = View::DocumentDetail;
    }

    /// Check if a document has been loaded
    fn check_document_loaded(&mut self) {
        if let Some(ref rx) = self.document_receiver {
            match rx.try_recv() {
                Ok(Some(doc)) => {
                    println!("Document loaded: {}", doc.title);
                    self.selected_document = Some(doc);
                    self.document_receiver = None;
                }
                Ok(None) => {
                    // Document not found, go back
                    eprintln!("Document not found, going back");
                    self.current_view = self.previous_view.clone();
                    self.document_receiver = None;

                    let id = self.next_toast_id();
                    self.add_toast(Toast::error(id, "Document not found"));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still loading
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Channel closed, clear receiver
                    self.document_receiver = None;
                }
            }
        }
    }

    /// Navigate back from document view
    pub fn navigate_back(&mut self) {
        match self.current_view {
            View::DocumentDetail => {
                self.current_view = self.previous_view.clone();
                self.selected_document = None;
            }
            View::SearchResults => {
                self.current_view = View::Home;
                self.search_results.clear();
                self.all_results.clear();
            }
            View::Home => {
                // Already at home, nothing to do
            }
        }
    }

    /// Check if a document is currently loading
    pub fn is_document_loading(&self) -> bool {
        self.document_receiver.is_some()
    }

    /// Check if exclusion rules save is in progress
    pub fn is_saving_exclusion_rules(&self) -> bool {
        self.save_exclusion_receiver.is_some()
    }

    /// Load bookmark folders for settings
    pub fn load_bookmark_folders(&mut self) {
        use crate::bookmark::BookmarkMonitor;

        match BookmarkMonitor::new() {
            Ok((monitor, _)) => {
                let folders = monitor.get_bookmark_folders();
                // Convert BookmarkFolder to BookmarkFolderView and build tree structure
                let flat_views: Vec<BookmarkFolderView> = folders
                    .into_iter()
                    .map(|f| BookmarkFolderView {
                        id: f.id,
                        name: f.name,
                        path: f.path,
                        children: Vec::new(),
                        bookmark_count: f.bookmark_count,
                    })
                    .collect();

                // Build tree structure from flat list based on paths
                self.bookmark_folders = build_folder_tree(flat_views);
                println!("Loaded {} bookmark folders", self.bookmark_folders.len());
            }
            Err(e) => {
                eprintln!("Failed to load bookmark folders: {}", e);
                let id = self.next_toast_id();
                self.add_toast(Toast::error(
                    id,
                    format!("Failed to load bookmark folders: {}", e),
                ));
            }
        }
    }

    /// Load current exclusion rules from database
    pub fn load_exclusion_rules(&mut self) {
        if self.exclusion_rules_receiver.is_some() {
            return; // Already loading
        }

        let rag = self.rag.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let runtime_handle = self.runtime.clone();

        runtime_handle.spawn(async move {
            let rag_lock = rag.read().await;
            let result = if let Some(ref rag) = *rag_lock {
                let folders = rag.db.get_excluded_folders().await.unwrap_or_default();
                let domains = rag.db.get_excluded_domains().await.unwrap_or_default();
                (folders, domains)
            } else {
                (Vec::new(), Vec::new())
            };
            let _ = tx.send(result);
        });

        self.exclusion_rules_receiver = Some(rx);
    }

    /// Check if exclusion rules have loaded
    fn check_exclusion_rules_loaded(&mut self) {
        if let Some(ref rx) = self.exclusion_rules_receiver {
            match rx.try_recv() {
                Ok((folders, domains)) => {
                    println!(
                        "Loaded exclusion rules: {} folders, {} domains",
                        folders.len(),
                        domains.len()
                    );
                    self.excluded_folders = folders.into_iter().collect();
                    self.excluded_domains = domains;
                    self.exclusion_rules_receiver = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still loading
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Channel closed, clear receiver
                    self.exclusion_rules_receiver = None;
                }
            }
        }
    }

    /// Save exclusion rules to database and remove matching bookmarks
    pub fn save_exclusion_rules(&mut self) -> crate::Result<()> {
        if self.save_exclusion_receiver.is_some() {
            return Err("Save already in progress".into());
        }

        let rag = self.rag.clone();
        let folders: Vec<String> = self.excluded_folders.iter().cloned().collect();
        let domains = self.excluded_domains.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let runtime_handle = self.runtime.clone();

        runtime_handle.spawn(async move {
            let rag_lock = rag.read().await;
            let result = if let Some(ref rag) = *rag_lock {
                // Save exclusion rules
                if let Err(e) = rag.db
                    .set_excluded_folders(&folders)
                    .await
                {
                    let _ = tx.send(Err(e.to_string()));
                    return;
                }
                if let Err(e) = rag.db
                    .set_excluded_domains(&domains)
                    .await
                {
                    let _ = tx.send(Err(e.to_string()));
                    return;
                }

                // Remove matching bookmarks
                let mut removed_count = 0;
                for domain in &domains {
                    match rag.db.delete_bookmarks_by_url_pattern(domain).await {
                        Ok(count) => removed_count += count,
                        Err(e) => {
                            let _ = tx.send(Err(e.to_string()));
                            return;
                        }
                    }
                }

                for folder_id in &folders {
                    match rag.db.delete_bookmarks_by_folder(folder_id).await {
                        Ok(count) => removed_count += count,
                        Err(e) => {
                            let _ = tx.send(Err(e.to_string()));
                            return;
                        }
                    }
                }

                Ok(removed_count)
            } else {
                Err("RAG not initialized".to_string())
            };
            let _ = tx.send(result);
        });

        self.save_exclusion_receiver = Some(rx);
        Ok(())
    }

    /// Check if save exclusion rules has completed
    fn check_save_exclusion_rules(&mut self) -> Option<Result<usize, String>> {
        if let Some(ref rx) = self.save_exclusion_receiver {
            match rx.try_recv() {
                Ok(result) => {
                    self.save_exclusion_receiver = None;
                    Some(result)
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still saving
                    None
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Channel closed, clear receiver
                    self.save_exclusion_receiver = None;
                    None
                }
            }
        } else {
            None
        }
    }

    /// Set the bookmark progress receiver
    /// Set the bookmark progress receiver
    ///
    /// Configures the channel receiver for bookmark ingestion progress events.
    /// Progress events are processed in `check_bookmark_progress()`.
    pub fn set_bookmark_progress_receiver(
        &mut self,
        receiver: std::sync::mpsc::Receiver<BookmarkProgress>,
    ) {
        self.bookmark_progress_receiver = Some(receiver);
    }

    /// Check for bookmark progress events and update toasts
    fn check_bookmark_progress(&mut self) {
        // Collect all pending messages first to avoid borrow checker issues
        let mut pending_progress = Vec::new();
        if let Some(ref rx) = self.bookmark_progress_receiver {
            while let Ok(progress) = rx.try_recv() {
                pending_progress.push(progress);
            }
        }

        // Process collected messages
        for progress in pending_progress {
            if progress.completed {
                // Remove progress toast if it exists
                if let Some(progress_id) = self.bookmark_progress_toast_id.take() {
                    self.toasts.retain(|t| t.id != progress_id);
                }

                // Add completion toast
                let id = self.next_toast_id();
                self.add_toast(Toast::success(
                    id,
                    format!("Completed! {} bookmarks ingested", progress.current),
                ));
            } else {
                // Update or create progress toast
                let percentage = if progress.total > 0 {
                    (progress.current as f32 / progress.total as f32 * 100.0) as usize
                } else {
                    0
                };

                let message = format!(
                    "Processing bookmarks... {}/{} ({}%)",
                    progress.current, progress.total, percentage
                );

                // Remove old progress toast if it exists
                if let Some(progress_id) = self.bookmark_progress_toast_id.take() {
                    self.toasts.retain(|t| t.id != progress_id);
                }

                // Create new progress toast (persistent until completion)
                let id = self.next_toast_id();
                self.bookmark_progress_toast_id = Some(id);
                self.add_toast(Toast::new(
                    id,
                    message,
                    ToastType::Info,
                    std::time::Duration::ZERO, // Persistent until replaced
                ));
            }
        }
    }
}

/// Create a content snippet, truncating at word boundaries
fn create_snippet(content: &str, max_len: usize) -> String {
    let content = content.trim();
    if content.len() <= max_len {
        return content.to_string();
    }

    // Find a good break point (whitespace)
    let truncated = &content[..max_len];
    if let Some(last_space) = truncated.rfind(char::is_whitespace) {
        format!("{}...", &content[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

impl eframe::App for LocalMindApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for async updates
        self.check_init_status();
        self.check_recent_documents();
        self.check_search_results();
        self.check_document_loaded();
        self.check_bookmark_progress();
        self.check_exclusion_rules_loaded();
        self.cleanup_toasts();

        // Handle Escape key for back navigation or closing settings
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.settings_open {
                self.settings_open = false;
            } else {
                self.navigate_back();
            }
        }

        // Check for save completion
        if let Some(result) = self.check_save_exclusion_rules() {
            match result {
                Ok(removed_count) => {
                    if removed_count > 0 {
                        println!(
                            "Removed {} bookmarks matching exclusion rules",
                            removed_count
                        );
                    }
                    // Show success toast
                    let id = self.next_toast_id();
                    self.add_toast(Toast::success(
                        id,
                        format!(
                            "Exclusion rules saved{}",
                            if removed_count > 0 {
                                format!(" ({} bookmarks removed)", removed_count)
                            } else {
                                String::new()
                            }
                        ),
                    ));
                    // Close settings modal
                    self.settings_open = false;
                }
                Err(e) => {
                    let id = self.next_toast_id();
                    self.add_toast(Toast::error(id, format!("Failed to save: {}", e)));
                }
            }
        }

        // Top panel with search bar and status
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // App title
                ui.heading("LocalMind");

                ui.add_space(20.0);

                // Search input (disabled until ready)
                let search_enabled = matches!(self.init_status, InitStatus::Ready);
                let mut should_search = false;
                ui.add_enabled_ui(search_enabled, |ui| {
                    let response = ui.add_sized(
                        [400.0, 32.0],
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Search documents...")
                            .margin(egui::Margin {
                                left: 8.0,
                                right: 8.0,
                                top: 8.0,
                                bottom: 5.0,
                            }),
                    );

                    // Handle Enter key for search
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        should_search = true;
                    }
                });

                // Trigger search outside the closure to avoid borrow issues
                if should_search {
                    self.trigger_search();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Settings button
                    if ui.button("⚙").clicked() {
                        self.settings_open = !self.settings_open;
                        if self.settings_open {
                            // Load bookmark folders and exclusion rules when opening
                            self.load_bookmark_folders();
                            self.load_exclusion_rules();
                        }
                    }

                    ui.add_space(10.0);

                    // Status indicator
                    match &self.init_status {
                        InitStatus::Starting => {
                            ui.spinner();
                            ui.label("Starting...");
                        }
                        InitStatus::WaitingForEmbedding => {
                            ui.spinner();
                            ui.label("Initializing...");
                        }
                        InitStatus::Ready => {
                            ui.label("✓ Ready");
                        }
                        InitStatus::Error(msg) => {
                            ui.colored_label(egui::Color32::RED, format!("✗ {}", msg));
                        }
                    }
                });
            });
            ui.add_space(8.0);
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_view {
                View::Home => {
                    views::home::render_home_view(ui, self);
                }
                View::SearchResults => {
                    views::search::render_search_results(ui, self);
                }
                View::DocumentDetail => {
                    if self.is_document_loading() {
                        // Show loading state
                        ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.spinner();
                            ui.add_space(10.0);
                            ui.label("Loading document...");
                        });
                    } else {
                        views::document::render_document_view(ui, self);
                    }
                }
            }
        });

        // Settings modal
        if self.settings_open {
            use widgets::settings;
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(true)
                .default_size([600.0, 500.0])
                .show(ctx, |ui| {
                    if settings::render_settings_modal(ui, self) {
                        self.settings_open = false;
                    }
                });
        }

        // Toast overlay (bottom-right)
        widgets::toast::render_toasts(ctx, &self.toasts);

        // Request repaint while initializing, loading, or searching
        if !matches!(self.init_status, InitStatus::Ready | InitStatus::Error(_))
            || self.recent_docs_receiver.is_some()
            || self.search_receiver.is_some()
            || self.document_receiver.is_some()
            || self.exclusion_rules_receiver.is_some()
            || self.save_exclusion_receiver.is_some()
        {
            ctx.request_repaint();
        }
    }
}

/// Strip HTML tags from content and return plain text
///
/// Uses `html2text` crate to convert HTML to readable plain text.
/// Wraps text at 80 characters per line.
///
/// # Arguments
/// * `content` - HTML content to convert
///
/// # Returns
/// Plain text version of the content with HTML tags removed
pub fn strip_html(content: &str) -> String {
    html2text::from_read(content.as_bytes(), 80)
}

/// Initialize the RAG system
async fn init_rag_system() -> crate::Result<RagPipeline> {
    println!("Initializing database...");

    let db = match Database::new().await {
        Ok(database) => {
            println!("Database initialized successfully");
            database
        }
        Err(e) => {
            eprintln!("Database initialization failed: {}", e);
            return Err(e);
        }
    };

    println!("Initializing RAG pipeline...");
    let rag = match RagPipeline::new(db).await {
        Ok(rag_pipeline) => {
            println!("RAG pipeline initialized successfully");
            rag_pipeline
        }
        Err(e) => {
            eprintln!("RAG pipeline initialization failed: {}", e);
            return Err(e);
        }
    };

    Ok(rag)
}

/// Start bookmark monitoring with progress reporting
async fn start_bookmark_monitoring(
    rag_state: RagState,
    progress_tx: std::sync::mpsc::Sender<BookmarkProgress>,
) -> crate::Result<()> {
    use crate::bookmark::BookmarkMonitor;
    use crate::bookmark_exclusion::ExclusionRules;

    println!("Initializing bookmark monitor...");
    let (monitor, mut rx) = BookmarkMonitor::new()?;

    println!("Starting file system monitoring...");
    monitor.start_monitoring().await?;

    println!("Getting existing bookmarks...");

    // Load exclusion rules from database
    let exclusion_rules = {
        let rag_lock = rag_state.read().await;
        if let Some(ref rag) = *rag_lock {
            let folders = rag.db.get_excluded_folders().await.unwrap_or_default();
            let domains = rag.db.get_excluded_domains().await.unwrap_or_default();
            ExclusionRules::new(folders, domains)
        } else {
            ExclusionRules::empty()
        }
    };

    // Get bookmark metadata only (no content fetching yet), applying exclusion rules
    let bookmark_metadata = monitor
        .get_bookmarks_metadata_with_exclusion(&exclusion_rules)
        .await?;

    if !bookmark_metadata.is_empty() {
        println!(
            "Processing {} existing bookmarks WITH PROGRESS...",
            bookmark_metadata.len()
        );

        let total = bookmark_metadata.len();
        let mut ingested_count = 0;

        for (index, (title, url)) in bookmark_metadata.into_iter().enumerate() {
            {
                let rag_lock = rag_state.read().await;
                if let Some(ref rag) = *rag_lock {
                    // Send progress update for all bookmarks being processed
                    let _ = progress_tx.send(BookmarkProgress {
                        current: index + 1,
                        total,
                        current_title: title.clone(),
                        completed: false,
                    });

                    // Check if bookmark already exists
                    if !rag.document_exists(&url).await.unwrap_or(false) {

                        // Fetch content
                        let content = match monitor.fetch_bookmark_content(&url).await {
                            Ok(content) => content,
                            Err(e) => {
                                eprintln!("Failed to fetch content for '{}': {}", title, e);
                                format!(
                                    "Bookmark: {}\nURL: {}\n\n[Error fetching content: {}]",
                                    title, url, e
                                )
                            }
                        };

                        match rag
                            .ingest_document(&title, &content, Some(&url), "chrome_bookmark")
                            .await
                        {
                            Ok(_) => {
                                ingested_count += 1;
                                println!("Ingested bookmark: {}", title);
                            }
                            Err(e) => {
                                eprintln!("Failed to ingest bookmark '{}': {}", title, e);
                            }
                        }
                    }
                }
            }

            // Small delay to prevent overwhelming the system
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Always send completion notification, even if no new bookmarks were ingested
        // This ensures the progress toast is dismissed
        let _ = progress_tx.send(BookmarkProgress {
            current: total,
            total,
            current_title: if ingested_count > 0 {
                format!("Completed! {} new bookmarks ingested", ingested_count)
            } else {
                format!("Completed! All {} bookmarks already indexed", total)
            },
            completed: true,
        });

        println!(
            "Initial bookmark ingestion completed: {} new bookmarks ingested ({} total processed)",
            ingested_count, total
        );
    } else {
        println!("No existing bookmarks found");
    }

    // Listen for bookmark changes
    println!("Starting bookmark change listener...");
    tokio::spawn(async move {
        while let Some(updated_bookmarks) = rx.recv().await {
            println!(
                "Detected bookmark changes, processing {} bookmarks...",
                updated_bookmarks.len()
            );
            // TODO: Process updated bookmarks with progress reporting
            // For now just log the change
        }
    });

    Ok(())
}

/// Start the HTTP server for Chrome extension compatibility
async fn start_http_server(rag_state: RagState) -> crate::Result<()> {
    use axum::{
        extract::State,
        http::{header, Method, StatusCode},
        response::{IntoResponse, Json, Response},
        routing::post,
        Router,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tokio::net::TcpListener;
    use tower::ServiceBuilder;
    use tower_http::cors::{Any, CorsLayer};

    #[derive(Clone)]
    struct AppState {
        rag_state: RagState,
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

    #[derive(Deserialize)]
    struct DocumentRequest {
        title: String,
        content: String,
        url: Option<String>,
        #[serde(default = "default_extraction_method", rename = "extractionMethod")]
        extraction_method: String,
    }

    fn default_extraction_method() -> String {
        "dom".to_string()
    }

    #[derive(Serialize)]
    struct SuccessResponse {
        message: String,
        #[serde(rename = "extractionMethod")]
        extraction_method: String,
    }

    async fn handle_post_documents(
        State(state): State<AppState>,
        Json(request): Json<DocumentRequest>,
    ) -> Result<Json<SuccessResponse>, ApiError> {
        if request.title.is_empty() || request.content.is_empty() {
            return Err(ApiError {
                status: StatusCode::BAD_REQUEST,
                message: "Title and content are required.".to_string(),
            });
        }

        let rag_lock = state.rag_state.read().await;
        let rag = rag_lock.as_ref().ok_or_else(|| ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: "System initializing. Please wait.".to_string(),
        })?;

        println!(
            "Processing document: title='{}', url={:?}",
            request.title.chars().take(60).collect::<String>(),
            request.url.as_deref()
        );

        rag.ingest_document(
            &request.title,
            &request.content,
            request.url.as_deref(),
            "chrome_extension",
        )
        .await
        .map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to add document: {}", e),
        })?;

        Ok(Json(SuccessResponse {
            message: "Document added successfully.".to_string(),
            extraction_method: request.extraction_method,
        }))
    }

    // Find available port
    let mut port = None;
    for p in 3000..=3010 {
        match TcpListener::bind(format!("127.0.0.1:{}", p)).await {
            Ok(_) => {
                port = Some(p);
                break;
            }
            Err(_) => continue,
        }
    }

    let port = port.ok_or("No available ports in range 3000-3010")?;
    println!("Starting HTTP server on port {}", port);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    let app_state = AppState { rag_state };

    let app = Router::new()
        .route("/documents", post(handle_post_documents))
        .layer(
            ServiceBuilder::new()
                .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))
                .layer(cors),
        )
        .with_state(app_state);

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    println!("HTTP server listening on http://localhost:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_basic() {
        let html = "<p>Hello <b>world</b>!</p>";
        let result = strip_html(html);
        assert!(result.contains("Hello"));
        assert!(result.contains("world"));
        assert!(!result.contains("<p>"));
        assert!(!result.contains("<b>"));
    }

    #[test]
    fn test_strip_html_plain_text() {
        let plain = "Just plain text";
        let result = strip_html(plain);
        assert_eq!(result.trim(), plain);
    }
}
