//! Application state types for the egui frontend

use std::time::{Duration, Instant};

/// Navigation state for the main content area
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum View {
    /// Default view showing recent documents
    #[default]
    Home,
    /// Search results list after query
    SearchResults,
    /// Full document view
    DocumentDetail,
}

/// Application initialization progress
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InitStatus {
    /// Application just launched
    #[default]
    Starting,
    /// Waiting for Python embedding server
    WaitingForEmbedding,
    /// RAG pipeline initialized, search available
    Ready,
    /// Initialization failed with message
    Error(String),
}

/// Toast visual style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    /// General information, progress (blue)
    Info,
    /// Operation completed successfully (green)
    Success,
    /// Error occurred (red)
    Error,
}

/// Notification message with auto-dismiss
#[derive(Debug, Clone)]
pub struct Toast {
    /// Unique identifier
    pub id: u64,
    /// Notification text
    pub message: String,
    /// Info, Success, or Error
    pub toast_type: ToastType,
    /// When toast was created
    pub created_at: Instant,
    /// Auto-dismiss after (Duration::ZERO = persistent)
    pub duration: Duration,
}

impl Toast {
    /// Create a new toast notification
    pub fn new(
        id: u64,
        message: impl Into<String>,
        toast_type: ToastType,
        duration: Duration,
    ) -> Self {
        Self {
            id,
            message: message.into(),
            toast_type,
            created_at: Instant::now(),
            duration,
        }
    }

    /// Create an info toast with default 5 second duration
    pub fn info(id: u64, message: impl Into<String>) -> Self {
        Self::new(id, message, ToastType::Info, Duration::from_secs(5))
    }

    /// Create a success toast with default 3 second duration
    pub fn success(id: u64, message: impl Into<String>) -> Self {
        Self::new(id, message, ToastType::Success, Duration::from_secs(3))
    }

    /// Create an error toast with default 8 second duration
    pub fn error(id: u64, message: impl Into<String>) -> Self {
        Self::new(id, message, ToastType::Error, Duration::from_secs(8))
    }

    /// Check if this toast should be dismissed
    pub fn is_expired(&self) -> bool {
        if self.duration == Duration::ZERO {
            return false; // Persistent toast
        }
        self.created_at.elapsed() >= self.duration
    }
}

/// A Chrome profile available for filtering
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeProfileInfo {
    /// Directory name: "Default", "Profile 1", etc.
    pub dir_name: String,
    /// Human-readable name, e.g. "Will"
    pub display_name: String,
}

/// UI representation of a search result
#[derive(Debug, Clone)]
pub struct SearchResultView {
    /// Document ID for fetching full content
    pub doc_id: i64,
    /// Document title
    pub title: String,
    /// Content preview (first ~200 chars)
    pub snippet: String,
    /// Similarity score (0.0-1.0)
    pub similarity: f32,
    /// Source URL if available
    pub url: Option<String>,
    /// Chrome profile this bookmark came from
    pub profile: Option<String>,
    /// Whether this document requires authentication to access
    pub is_needs_auth: bool,
}

/// UI representation of a full document
#[derive(Debug, Clone)]
pub struct DocumentView {
    /// Document ID
    pub id: i64,
    /// Document title
    pub title: String,
    /// Full content (HTML stripped to plain text)
    pub content: String,
    /// Source URL
    pub url: Option<String>,
    /// Source type (e.g., "chrome_bookmark")
    pub source: String,
    /// Creation timestamp
    pub created_at: String,
    /// Chrome profile this document came from
    pub profile: Option<String>,
    /// Whether this document requires authentication to access
    pub is_needs_auth: bool,
}

// ---------------------------------------------------------------------------
// Folder-watch types (T004)
// ---------------------------------------------------------------------------

/// Progress state for an in-progress folder scan shown in the UI.
#[derive(Debug, Clone)]
pub struct FolderWatchProgress {
    pub folder_path: std::path::PathBuf,
    pub files_total: usize,
    pub files_done: usize,
    pub current_file: Option<std::path::PathBuf>,
    pub error: Option<String>,
}

/// Events sent from the folder-watch backend to the egui update loop.
#[derive(Debug, Clone)]
pub enum FolderWatchEvent {
    /// Initial scan started; `files_total` is the count of supported files found.
    ScanStarted {
        folder_path: std::path::PathBuf,
        files_total: usize,
    },
    /// One file was successfully ingested during a scan or re-ingest.
    FileIngested {
        folder_path: std::path::PathBuf,
        file_path: std::path::PathBuf,
    },
    /// One file failed to ingest; others in the same folder continue.
    FileError {
        folder_path: std::path::PathBuf,
        file_path: std::path::PathBuf,
        error: String,
    },
    /// Initial scan for a folder has completed (all files attempted).
    ScanComplete { folder_path: std::path::PathBuf },
    /// A watched folder and all its content have been removed.
    FolderRemoved { folder_path: std::path::PathBuf },
    /// The operational status of a folder changed (e.g., became unavailable).
    FolderStatusChanged {
        folder_path: std::path::PathBuf,
        status: crate::folder_watcher::FolderStatus,
    },
    /// `add_folder` failed at the service layer (e.g., AlreadyWatched, DbError).
    ///
    /// The folder was NOT added to the watched list.
    AddFolderFailed {
        folder_path: std::path::PathBuf,
        error: String,
    },
}

/// UI representation of a bookmark folder for tree display
#[derive(Debug, Clone)]
pub struct BookmarkFolderView {
    /// Chrome folder ID
    pub id: String,
    /// Folder display name
    pub name: String,
    /// Full path from root
    pub path: Vec<String>,
    /// Nested folders
    pub children: Vec<BookmarkFolderView>,
    /// Number of bookmarks in folder
    pub bookmark_count: usize,
}
