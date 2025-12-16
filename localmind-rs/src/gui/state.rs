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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitStatus {
    /// Application just launched
    Starting,
    /// Waiting for Python embedding server
    WaitingForEmbedding,
    /// RAG pipeline initialized, search available
    Ready,
    /// Initialization failed with message
    Error(String),
}

impl Default for InitStatus {
    fn default() -> Self {
        Self::Starting
    }
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
