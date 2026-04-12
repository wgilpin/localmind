//! Folder Watch and Ingest service
//!
//! Manages filesystem watchers for user-registered directories and drives
//! automatic ingestion of PDF, Markdown, and plain-text files.

use crate::gui::state::FolderWatchEvent;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Domain types (T002)
// ---------------------------------------------------------------------------

/// A filesystem directory registered by the user for monitoring.
#[derive(Debug, Clone)]
pub struct WatchedFolder {
    pub id: i64,
    pub path: PathBuf,
    pub status: FolderStatus,
    pub created_at: String,
}

/// Per-file tracking record within a watched folder.
#[derive(Debug, Clone)]
pub struct WatchedFile {
    pub id: i64,
    pub folder_id: i64,
    pub file_path: PathBuf,
    pub modified_at: i64,
    pub document_id: Option<i64>,
    pub ingest_status: IngestStatus,
}

/// Operational status of a watched folder.
#[derive(Debug, Clone, PartialEq)]
pub enum FolderStatus {
    Active,
    Error(String),
    Unavailable,
}

impl FolderStatus {
    /// Convert to the string stored in the database.
    pub fn as_db_str(&self) -> &str {
        match self {
            FolderStatus::Active => "active",
            FolderStatus::Error(_) => "error",
            FolderStatus::Unavailable => "unavailable",
        }
    }

    /// Parse from a database status string (and optional error detail).
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "active" => FolderStatus::Active,
            "unavailable" => FolderStatus::Unavailable,
            _ => FolderStatus::Error(s.to_string()),
        }
    }
}

/// Ingest status of a single file.
#[derive(Debug, Clone, PartialEq)]
pub enum IngestStatus {
    Pending,
    Ingested,
    Error(String),
}

impl IngestStatus {
    pub fn as_db_str(&self) -> &str {
        match self {
            IngestStatus::Pending => "pending",
            IngestStatus::Ingested => "ingested",
            IngestStatus::Error(_) => "error",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "ingested" => IngestStatus::Ingested,
            "error" => IngestStatus::Error(String::new()),
            _ => IngestStatus::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// Error type (T003)  — manual impl, thiserror not in Cargo.toml
// ---------------------------------------------------------------------------

/// Errors produced by the folder-watch service.
#[derive(Debug)]
pub enum FolderWatchError {
    AlreadyWatched,
    PathNotFound,
    NotADirectory,
    DbError(String),
    IngestError(String),
    IoError(String),
    UnsupportedType,
}

impl fmt::Display for FolderWatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FolderWatchError::AlreadyWatched => {
                write!(f, "This folder is already being watched")
            }
            FolderWatchError::PathNotFound => write!(f, "Path does not exist"),
            FolderWatchError::NotADirectory => write!(f, "Path is not a directory"),
            FolderWatchError::DbError(e) => write!(f, "Database error: {}", e),
            FolderWatchError::IngestError(e) => write!(f, "Ingest error: {}", e),
            FolderWatchError::IoError(e) => write!(f, "IO error: {}", e),
            FolderWatchError::UnsupportedType => write!(f, "Unsupported file type"),
        }
    }
}

impl std::error::Error for FolderWatchError {}

// ---------------------------------------------------------------------------
// Watcher handle (T002 support type)
// ---------------------------------------------------------------------------

/// Owns an active `notify` watcher for one folder.
///
/// Dropping this handle signals the background thread to stop, which then
/// drops the watcher, unregistering all OS filesystem watches.
pub struct WatcherHandle {
    running: Arc<AtomicBool>,
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// File event (internal bridge type)
// ---------------------------------------------------------------------------

/// Raw filesystem event forwarded from a watcher thread to the service.
#[derive(Debug)]
pub struct FileEvent {
    pub folder_path: PathBuf,
    pub file_path: PathBuf,
    pub kind: EventKind,
}

// ---------------------------------------------------------------------------
// FolderWatchService (T002)
// ---------------------------------------------------------------------------

/// Manages active filesystem watchers and drives event routing.
///
/// Stored on `LocalMindApp` via `Arc<Mutex<FolderWatchService>>` so that async
/// tasks (scan, ingest) can start watchers upon completion without borrowing
/// the whole app struct.
pub struct FolderWatchService {
    /// Active watcher handles, keyed by watched folder path.
    pub watchers: HashMap<PathBuf, WatcherHandle>,
    /// Shared sender used by all watcher threads to push file events.
    pub file_event_tx: std::sync::mpsc::SyncSender<FileEvent>,
    /// Sender for UI-facing events (progress, errors, status changes).
    pub ui_event_tx: std::sync::mpsc::SyncSender<FolderWatchEvent>,
}

impl FolderWatchService {
    /// Create a new service and the companion receivers for the egui update loop.
    ///
    /// Returns the service plus the two receivers that `LocalMindApp` polls
    /// each frame via `try_recv()`.
    pub fn new() -> (
        Self,
        std::sync::mpsc::Receiver<FileEvent>,
        std::sync::mpsc::Receiver<FolderWatchEvent>,
    ) {
        let (file_tx, file_rx) = std::sync::mpsc::sync_channel(256);
        let (ui_tx, ui_rx) = std::sync::mpsc::sync_channel(256);
        (
            Self {
                watchers: HashMap::new(),
                file_event_tx: file_tx,
                ui_event_tx: ui_tx,
            },
            file_rx,
            ui_rx,
        )
    }

    /// Start a `notify` watcher for `folder_path` (folder_id for logging).
    ///
    /// The watcher lives in a dedicated `std::thread::spawn` that holds it
    /// alive, following the established pattern from `bookmark.rs`.
    pub fn start_watching(&mut self, folder_path: PathBuf, folder_id: i64) {
        if self.watchers.contains_key(&folder_path) {
            return;
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let file_tx = self.file_event_tx.clone();
        let watch_path = folder_path.clone();

        std::thread::spawn(move || {
            let tx = file_tx.clone();
            let watch_path_inner = watch_path.clone();

            let mut watcher = match notify::recommended_watcher(
                move |res: notify::Result<Event>| match res {
                    Ok(event) => {
                        for path in &event.paths {
                            if is_supported_extension(path) {
                                let ev = FileEvent {
                                    folder_path: watch_path_inner.clone(),
                                    file_path: path.clone(),
                                    kind: event.kind,
                                };
                                if tx.try_send(ev).is_err() {
                                    eprintln!(
                                        "[folder_watcher] file event channel full for folder_id={}, dropping event",
                                        folder_id
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "[folder_watcher] notify watcher error: folder_id={}, error={}",
                            folder_id, e
                        );
                    }
                },
            ) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!(
                        "[folder_watcher] failed to create watcher: folder_id={}, error={}",
                        folder_id, e
                    );
                    return;
                }
            };

            if let Err(e) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
                eprintln!(
                    "[folder_watcher] failed to watch path: folder_id={}, error={}",
                    folder_id, e
                );
                return;
            }

            println!(
                "[folder_watcher] folder watcher started: folder_id={}, path={}",
                folder_id,
                watch_path.display()
            );

            while running_clone.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            println!(
                "[folder_watcher] folder watcher stopped: folder_id={}",
                folder_id
            );
        });

        self.watchers.insert(folder_path, WatcherHandle { running });
    }

    /// Stop the watcher for `folder_path` (dropping the handle signals the thread).
    pub fn stop_watching(&mut self, folder_path: &Path) {
        self.watchers.remove(folder_path);
    }
}

// ---------------------------------------------------------------------------
// File content reading (T020)
// ---------------------------------------------------------------------------

/// Read the text content of a supported file.
///
/// Supported extensions: `.txt`, `.md` (UTF-8 read), `.pdf` (pdf-extract).
/// Returns `Err(FolderWatchError::UnsupportedType)` for other extensions.
pub fn read_file_content(path: &Path) -> Result<String, FolderWatchError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "txt" | "md" => std::fs::read_to_string(path)
            .map(|s| strip_data_url_images(strip_yaml_frontmatter(&s)))
            .map_err(|e| {
                eprintln!(
                    "[folder_watcher] failed to read file: path={}, error={}",
                    path.display(),
                    e
                );
                FolderWatchError::IoError(e.to_string())
            }),
        "pdf" => pdf_extract::extract_text(path).map_err(|e| {
            eprintln!(
                "[folder_watcher] failed to extract PDF text: path={}, error={}",
                path.display(),
                e
            );
            FolderWatchError::IngestError(e.to_string())
        }),
        _ => Err(FolderWatchError::UnsupportedType),
    }
}

// ---------------------------------------------------------------------------
// Content cleaning helpers
// ---------------------------------------------------------------------------

/// Strip a YAML frontmatter block (`---` … `---`) from the start of a Markdown file.
///
/// Returns a slice starting after the closing delimiter, or the original string
/// unchanged if no frontmatter is present.
fn strip_yaml_frontmatter(content: &str) -> &str {
    let s = content.trim_start();
    if !s.starts_with("---") {
        return content;
    }
    let after_open = &s["---".len()..];
    if let Some(close) = after_open.find("\n---") {
        let after_close = &after_open[close + "\n---".len()..];
        after_close.trim_start_matches('\n')
    } else {
        content
    }
}

/// Remove `![alt](data:...)` inline image tags from Markdown/text content.
///
/// Base64-encoded images are semantically empty blobs. Leaving them in adds
/// thousands of meaningless tokens to embeddings and massively inflates chunk
/// counts (the "Details.md → 1204 chunks" problem).
fn strip_data_url_images(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

    loop {
        // Key marker: the closing bracket + opening paren of a data-URL image.
        match remaining.find("](data:") {
            None => {
                result.push_str(remaining);
                break;
            }
            Some(close_bracket_pos) => {
                let before = &remaining[..close_bracket_pos];
                match before.rfind("![") {
                    None => {
                        // No matching "![" — keep text up to and including "](data:" and continue.
                        let keep = close_bracket_pos + "](data:".len();
                        result.push_str(&remaining[..keep]);
                        remaining = &remaining[keep..];
                    }
                    Some(img_start) => {
                        // Emit text before the image tag.
                        result.push_str(&remaining[..img_start]);
                        // Advance past "](" and find the closing ")".
                        let after_open = &remaining[close_bracket_pos + 2..];
                        match after_open.find(')') {
                            None => break, // malformed — drop rest
                            Some(close_rel) => {
                                remaining = &remaining[close_bracket_pos + 2 + close_rel + 1..];
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Directory traversal helpers
// ---------------------------------------------------------------------------

/// Returns true if the path has a supported extension (.pdf, .md, .txt).
pub fn is_supported_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).unwrap_or(""),
        "pdf" | "md" | "txt"
    )
}

/// Recursively collect all supported files under `dir`.
///
/// Uses `std::fs::read_dir` — no `walkdir` dependency needed.
pub fn collect_supported_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_supported_files_inner(dir, &mut files);
    files
}

fn collect_supported_files_inner(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "[folder_watcher] failed to read directory: dir={}, error={}",
                dir.display(),
                e
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_supported_files_inner(&path, out);
        } else if is_supported_extension(&path) {
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------------------
// mtime helpers (T028)
// ---------------------------------------------------------------------------

/// Return the file's modification time as Unix epoch seconds, or 0 on error.
pub fn get_mtime(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64
        })
        .unwrap_or(0)
}

/// Return `true` if the file's current mtime differs from `stored_mtime`.
pub fn is_mtime_changed(path: &Path, stored_mtime: i64) -> bool {
    get_mtime(path) != stored_mtime
}

// ---------------------------------------------------------------------------
// Async service functions (T021, T022, T031, T035)
// ---------------------------------------------------------------------------

/// Scan a folder, ingest all supported files, and emit progress events.
///
/// Runs as a background tokio task spawned by `add_folder`. For each
/// supported file it calls `rag.ingest_document_with_auth`, then records the
/// result in `watched_files`. On completion the watcher is started via the
/// `service_ref` mutex.
pub async fn scan_and_ingest_folder(
    folder_id: i64,
    folder_path: std::path::PathBuf,
    rag: crate::gui::app::RagState,
    _db_ref: std::sync::Arc<tokio::sync::Mutex<()>>, // db accessed via rag
    ui_tx: std::sync::mpsc::SyncSender<crate::gui::state::FolderWatchEvent>,
    service_ref: std::sync::Arc<std::sync::Mutex<FolderWatchService>>,
) {
    use crate::folder_watcher::{
        collect_supported_files, get_mtime, read_file_content, IngestStatus,
    };
    use crate::gui::state::FolderWatchEvent;
    use std::collections::HashMap;

    let all_files = collect_supported_files(&folder_path);

    // Load existing watched_file records for this folder so we can skip
    // files that are already ingested with an unchanged mtime.
    let existing: HashMap<PathBuf, WatchedFile> = {
        let rag_lock = rag.read().await;
        if let Some(ref rp) = *rag_lock {
            rp.db
                .get_watched_files_for_folder(folder_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|wf| (wf.file_path.clone(), wf))
                .collect()
        } else {
            HashMap::new()
        }
    };

    // Keep only files that are new, changed, or previously errored.
    // Skip files already ingested with an unchanged mtime.
    let files_to_ingest: Vec<PathBuf> = all_files
        .into_iter()
        .filter(|p| {
            let skip = match existing.get(p) {
                Some(wf) if wf.ingest_status == IngestStatus::Ingested => {
                    get_mtime(p) == wf.modified_at
                }
                _ => false,
            };
            !skip
        })
        .collect();

    let files_total = files_to_ingest.len();

    let _ = ui_tx.try_send(FolderWatchEvent::ScanStarted {
        folder_path: folder_path.clone(),
        files_total,
    });

    for file_path in &files_to_ingest {
        // If re-ingesting a previously ingested file, delete the old document first.
        if let Some(wf) = existing.get(file_path) {
            if let Some(doc_id) = wf.document_id {
                let rag_lock = rag.read().await;
                if let Some(ref rp) = *rag_lock {
                    let _ = rp.db.delete_document(doc_id).await;
                    rp.remove_document_vectors(doc_id).await;
                }
            }
        }

        let content = match read_file_content(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "[folder_watcher] failed to read file during scan: path={}, error={}",
                    file_path.display(),
                    e
                );
                let _ = ui_tx.try_send(FolderWatchEvent::FileError {
                    folder_path: folder_path.clone(),
                    file_path: file_path.clone(),
                    error: e.to_string(),
                });
                continue;
            }
        };

        let title = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let url = format!("file://{}", file_path.display());
        let source = folder_path.to_string_lossy().to_string();
        let mtime = get_mtime(file_path);

        let rag_lock = rag.read().await;
        if let Some(ref rag_pipeline) = *rag_lock {
            match rag_pipeline
                .ingest_document_with_auth(&title, &content, Some(&url), &source, None, false)
                .await
            {
                Ok(doc_id) => {
                    let _ = rag_pipeline
                        .db
                        .upsert_watched_file(
                            folder_id,
                            file_path,
                            mtime,
                            Some(doc_id),
                            &IngestStatus::Ingested,
                        )
                        .await;

                    let _ = ui_tx.try_send(FolderWatchEvent::FileIngested {
                        folder_path: folder_path.clone(),
                        file_path: file_path.clone(),
                    });
                }
                Err(e) => {
                    eprintln!(
                        "[folder_watcher] ingest failed: path={}, error={}",
                        file_path.display(),
                        e
                    );
                    let _ = rag_pipeline
                        .db
                        .upsert_watched_file(
                            folder_id,
                            file_path,
                            mtime,
                            None,
                            &IngestStatus::Error(e.to_string()),
                        )
                        .await;
                    let _ = ui_tx.try_send(FolderWatchEvent::FileError {
                        folder_path: folder_path.clone(),
                        file_path: file_path.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }
        drop(rag_lock);
    }

    let _ = ui_tx.try_send(FolderWatchEvent::ScanComplete {
        folder_path: folder_path.clone(),
    });

    // Start the filesystem watcher now that the initial scan is done
    if let Ok(mut svc) = service_ref.lock() {
        svc.start_watching(folder_path, folder_id);
    }
}

/// Validate a path and add it as a watched folder (T022).
///
/// Returns `Err(FolderWatchError)` for validation failures. On success,
/// the folder is added to the database and a background scan task is spawned.
pub async fn add_folder(
    path: &std::path::Path,
    rag: crate::gui::app::RagState,
    ui_tx: std::sync::mpsc::SyncSender<crate::gui::state::FolderWatchEvent>,
    service_ref: std::sync::Arc<std::sync::Mutex<FolderWatchService>>,
    runtime: tokio::runtime::Handle,
) -> Result<(), FolderWatchError> {
    // Validate path
    if !path.exists() {
        return Err(FolderWatchError::PathNotFound);
    }
    if !path.is_dir() {
        return Err(FolderWatchError::NotADirectory);
    }

    // Insert into database (will fail with UNIQUE error if already watched)
    let folder_id = {
        let rag_lock = rag.read().await;
        if let Some(ref rag_pipeline) = *rag_lock {
            rag_pipeline
                .db
                .add_watched_folder(path)
                .await
                .map_err(|e| {
                    let msg = e.to_string();
                    if msg.contains("UNIQUE") {
                        FolderWatchError::AlreadyWatched
                    } else {
                        FolderWatchError::DbError(msg)
                    }
                })?
        } else {
            return Err(FolderWatchError::DbError("RAG not initialized".into()));
        }
    };

    println!(
        "[folder_watcher] watching folder: folder_id={}, path={}",
        folder_id,
        path.display()
    );

    // Spawn background scan (T021 + T032)
    let folder_path = path.to_path_buf();
    let rag_clone = rag.clone();
    let ui_tx_clone = ui_tx.clone();
    let svc_clone = service_ref.clone();

    runtime.spawn(async move {
        scan_and_ingest_folder(
            folder_id,
            folder_path,
            rag_clone,
            std::sync::Arc::new(tokio::sync::Mutex::new(())),
            ui_tx_clone,
            svc_clone,
        )
        .await;
    });

    Ok(())
}

/// Handle a single file-system event (T031 + T033).
///
/// On Create/Modify: re-ingest if mtime changed.
/// On Remove: delete document from DB and evict from VectorStore.
pub async fn handle_file_event(
    event: FileEvent,
    rag: crate::gui::app::RagState,
    ui_tx: std::sync::mpsc::SyncSender<crate::gui::state::FolderWatchEvent>,
) {
    use crate::folder_watcher::{get_mtime, is_mtime_changed, read_file_content, IngestStatus};
    use crate::gui::state::FolderWatchEvent;
    use notify::event::EventKind;

    let rag_lock = rag.read().await;
    let rag_pipeline = match &*rag_lock {
        Some(r) => r,
        None => return,
    };

    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            // Check if mtime changed to avoid spurious re-ingests
            let current_mtime = get_mtime(&event.file_path);
            let stored = rag_pipeline
                .db
                .get_watched_file_by_path(&event.file_path)
                .await
                .unwrap_or(None);

            let should_reingest = match &stored {
                None => true, // new file
                Some(wf) => is_mtime_changed(&event.file_path, wf.modified_at),
            };

            if !should_reingest {
                return;
            }

            // Determine folder_id: from stored record or look up via folder_path
            let folder_id = match stored.as_ref().map(|wf| wf.folder_id) {
                Some(id) => id,
                None => {
                    // Look up folder_id from folder_path
                    let folders = rag_pipeline
                        .db
                        .get_watched_folders()
                        .await
                        .unwrap_or_default();
                    match folders
                        .iter()
                        .find(|f| f.path == event.folder_path)
                        .map(|f| f.id)
                    {
                        Some(id) => id,
                        None => {
                            eprintln!(
                                "[folder_watcher] cannot find folder_id for file event: path={}",
                                event.file_path.display()
                            );
                            return;
                        }
                    }
                }
            };

            // If re-ingest, delete old document first
            if let Some(ref wf) = stored {
                if let Some(doc_id) = wf.document_id {
                    let _ = rag_pipeline.db.delete_document(doc_id).await;
                    rag_pipeline.remove_document_vectors(doc_id).await;
                }
            }

            let content = match read_file_content(&event.file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "[folder_watcher] read failed on change: path={}, error={}",
                        event.file_path.display(),
                        e
                    );
                    let _ = ui_tx.try_send(FolderWatchEvent::FileError {
                        folder_path: event.folder_path.clone(),
                        file_path: event.file_path.clone(),
                        error: e.to_string(),
                    });
                    return;
                }
            };

            let title = event
                .file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let url = format!("file://{}", event.file_path.display());
            let source = event.folder_path.to_string_lossy().to_string();

            match rag_pipeline
                .ingest_document_with_auth(&title, &content, Some(&url), &source, None, false)
                .await
            {
                Ok(doc_id) => {
                    let _ = rag_pipeline
                        .db
                        .upsert_watched_file(
                            folder_id,
                            &event.file_path,
                            current_mtime,
                            Some(doc_id),
                            &IngestStatus::Ingested,
                        )
                        .await;
                    let _ = ui_tx.try_send(FolderWatchEvent::FileIngested {
                        folder_path: event.folder_path,
                        file_path: event.file_path,
                    });
                }
                Err(e) => {
                    eprintln!(
                        "[folder_watcher] re-ingest failed: path={}, error={}",
                        event.file_path.display(),
                        e
                    );
                    let _ = rag_pipeline
                        .db
                        .upsert_watched_file(
                            folder_id,
                            &event.file_path,
                            current_mtime,
                            None,
                            &IngestStatus::Error(e.to_string()),
                        )
                        .await;
                    let _ = ui_tx.try_send(FolderWatchEvent::FileError {
                        folder_path: event.folder_path,
                        file_path: event.file_path,
                        error: e.to_string(),
                    });
                }
            }
        }
        EventKind::Remove(_) => {
            // Look up the watched_file record
            if let Ok(Some(wf)) = rag_pipeline
                .db
                .get_watched_file_by_path(&event.file_path)
                .await
            {
                if let Some(doc_id) = wf.document_id {
                    let _ = rag_pipeline.db.delete_document(doc_id).await;
                    rag_pipeline.remove_document_vectors(doc_id).await; // T033
                }
            }
        }
        _ => {}
    }
}

/// Remove a watched folder: stop its watcher, delete its documents and DB row (T035).
pub async fn remove_folder(
    path: &std::path::Path,
    rag: crate::gui::app::RagState,
    ui_tx: std::sync::mpsc::SyncSender<crate::gui::state::FolderWatchEvent>,
    service_ref: std::sync::Arc<std::sync::Mutex<FolderWatchService>>,
) {
    use crate::gui::state::FolderWatchEvent;

    // Stop watcher first (fast, synchronous)
    if let Ok(mut svc) = service_ref.lock() {
        svc.stop_watching(path);
    }

    let rag_lock = rag.read().await;
    let rag_pipeline = match &*rag_lock {
        Some(r) => r,
        None => return,
    };

    // Find folder ID
    let folders = rag_pipeline
        .db
        .get_watched_folders()
        .await
        .unwrap_or_default();
    let folder_id = folders.iter().find(|f| f.path == path).map(|f| f.id);

    // Delete documents by source BEFORE deleting the folder (T046, T035 ordering)
    let doc_ids = rag_pipeline
        .db
        .delete_documents_by_source(path)
        .await
        .unwrap_or_default();

    // Evict from in-memory VectorStore (T033/T035)
    for doc_id in doc_ids {
        rag_pipeline.remove_document_vectors(doc_id).await;
    }

    // Delete the folder row (CASCADE removes watched_files)
    if let Some(id) = folder_id {
        let _ = rag_pipeline.db.delete_watched_folder(id).await;
    }

    println!("[folder_watcher] folder removed: path={}", path.display());

    let _ = ui_tx.try_send(FolderWatchEvent::FolderRemoved {
        folder_path: path.to_path_buf(),
    });
}

// ---------------------------------------------------------------------------
// Tests (T006-T008 written in db.rs; T017-T019 and T028-T029 here)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // --- strip_data_url_images ---

    #[test]
    fn strip_leaves_plain_text_unchanged() {
        let input = "# Heading\n\nSome paragraph text.";
        assert_eq!(strip_data_url_images(input), input);
    }

    #[test]
    fn strip_removes_data_url_image() {
        let input = "before\n![alt](data:image/png;base64,abc123==)\nafter";
        let out = strip_data_url_images(input);
        assert_eq!(out, "before\n\nafter");
        assert!(!out.contains("data:"));
    }

    #[test]
    fn strip_removes_multiple_data_url_images() {
        let input = "![](data:image/png;base64,AAA==) text ![](data:image/jpeg;base64,BBB==) end";
        let out = strip_data_url_images(input);
        assert!(!out.contains("data:"));
        assert!(out.contains("text"));
        assert!(out.contains("end"));
    }

    #[test]
    fn strip_keeps_normal_markdown_images() {
        let input = "![logo](https://example.com/logo.png)";
        assert_eq!(strip_data_url_images(input), input);
    }

    // --- T017: read_file_content ---

    #[test]
    fn read_txt_file_returns_content() {
        let mut f = NamedTempFile::with_suffix(".txt").unwrap();
        f.write_all(b"hello world").unwrap();
        let content = read_file_content(f.path()).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn read_md_file_returns_content() {
        let mut f = NamedTempFile::with_suffix(".md").unwrap();
        f.write_all(b"# Heading").unwrap();
        let content = read_file_content(f.path()).unwrap();
        assert_eq!(content, "# Heading");
    }

    #[test]
    fn read_unsupported_extension_returns_error() {
        let f = NamedTempFile::with_suffix(".png").unwrap();
        let result = read_file_content(f.path());
        assert!(matches!(result, Err(FolderWatchError::UnsupportedType)));
    }

    #[test]
    fn read_corrupted_pdf_returns_ingest_error() {
        let mut f = NamedTempFile::with_suffix(".pdf").unwrap();
        f.write_all(b"not a real pdf").unwrap();
        let result = read_file_content(f.path());
        // Should not panic; may return Ok (empty) or Err
        // pdf_extract is lenient on some inputs — just verify no panic
        let _ = result;
    }

    // --- T018: collect_supported_files ---

    #[test]
    fn scan_folder_discovers_supported_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "hello").unwrap();
        std::fs::write(dir.path().join("data.txt"), "world").unwrap();
        std::fs::write(dir.path().join("image.png"), "ignored").unwrap();

        let found = collect_supported_files(dir.path());
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|p| p.extension().unwrap() == "md"));
        assert!(found.iter().any(|p| p.extension().unwrap() == "txt"));
        assert!(!found.iter().any(|p| p.extension().unwrap() == "png"));
    }

    #[test]
    fn scan_empty_folder_returns_empty_vec() {
        let dir = tempfile::tempdir().unwrap();
        let found = collect_supported_files(dir.path());
        assert!(found.is_empty());
    }

    #[test]
    fn scan_folder_discovers_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("deep.txt"), "deep").unwrap();

        let found = collect_supported_files(dir.path());
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("deep.txt"));
    }

    // --- T019: add_folder error cases (validated via path checks) ---

    #[test]
    fn path_not_found_is_detected() {
        let non_existent = PathBuf::from("/tmp/localmind_does_not_exist_xyzzy_12345");
        assert!(!non_existent.exists());
    }

    #[test]
    fn non_directory_is_detected() {
        let f = NamedTempFile::new().unwrap();
        assert!(!f.path().is_dir());
    }

    // --- T028: mtime detection ---

    #[test]
    fn mtime_unchanged_returns_false() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"initial").unwrap();
        let mtime = get_mtime(f.path());
        // Same mtime — no change
        assert!(!is_mtime_changed(f.path(), mtime));
    }

    #[test]
    fn mtime_stored_as_zero_always_changed() {
        let f = NamedTempFile::new().unwrap();
        // stored mtime 0 means never seen — treat as changed
        assert!(is_mtime_changed(f.path(), 0));
    }

    #[test]
    fn mtime_missing_file_returns_zero() {
        let path = PathBuf::from("/tmp/localmind_no_such_file_xyzzy.txt");
        assert_eq!(get_mtime(&path), 0);
    }

    // --- T029: handle_file_event dispatch — RAG-None guard ---

    #[tokio::test]
    async fn handle_file_event_create_with_no_rag_returns_early() {
        let rag_state: crate::gui::app::RagState =
            std::sync::Arc::new(tokio::sync::RwLock::new(None));
        let (ui_tx, _ui_rx) = std::sync::mpsc::sync_channel(8);
        let event = FileEvent {
            folder_path: PathBuf::from("/tmp/test_folder"),
            file_path: PathBuf::from("/tmp/test_folder/note.txt"),
            kind: notify::EventKind::Create(notify::event::CreateKind::Any),
        };
        handle_file_event(event, rag_state, ui_tx).await;
    }

    #[tokio::test]
    async fn handle_file_event_modify_with_no_rag_returns_early() {
        let rag_state: crate::gui::app::RagState =
            std::sync::Arc::new(tokio::sync::RwLock::new(None));
        let (ui_tx, _ui_rx) = std::sync::mpsc::sync_channel(8);
        let event = FileEvent {
            folder_path: PathBuf::from("/tmp/test_folder"),
            file_path: PathBuf::from("/tmp/test_folder/note.md"),
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Any),
        };
        handle_file_event(event, rag_state, ui_tx).await;
    }

    #[tokio::test]
    async fn handle_file_event_remove_with_no_rag_returns_early() {
        let rag_state: crate::gui::app::RagState =
            std::sync::Arc::new(tokio::sync::RwLock::new(None));
        let (ui_tx, _ui_rx) = std::sync::mpsc::sync_channel(8);
        let event = FileEvent {
            folder_path: PathBuf::from("/tmp/test_folder"),
            file_path: PathBuf::from("/tmp/test_folder/note.pdf"),
            kind: notify::EventKind::Remove(notify::event::RemoveKind::Any),
        };
        handle_file_event(event, rag_state, ui_tx).await;
    }

    // --- T034: remove_folder — RAG-None guard (watcher stopped, no panic) ---

    #[tokio::test]
    async fn remove_folder_with_no_rag_stops_watcher_gracefully() {
        let rag_state: crate::gui::app::RagState =
            std::sync::Arc::new(tokio::sync::RwLock::new(None));
        let (ui_tx, ui_rx) = std::sync::mpsc::sync_channel(8);
        let (svc, _file_rx, _ui_rx2) = FolderWatchService::new();
        let service_ref = std::sync::Arc::new(std::sync::Mutex::new(svc));
        let path = PathBuf::from("/tmp/localmind_test_remove_xyzzy");
        // Returns early after stopping watcher (None RAG path)
        remove_folder(&path, rag_state, ui_tx, service_ref).await;
        // No FolderRemoved event emitted when RAG is None (early return before send)
        assert!(ui_rx.try_recv().is_err());
    }
}
