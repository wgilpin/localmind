use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use tokio::sync::mpsc;
use reqwest;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkItem {
    pub date_added: String,
    pub date_modified: Option<String>,
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub children: Option<Vec<BookmarkItem>>,
}

#[derive(Debug, Deserialize)]
pub struct ChromeBookmarks {
    pub roots: ChromeBookmarkRoots,
}

#[derive(Debug, Deserialize)]
pub struct ChromeBookmarkRoots {
    pub bookmark_bar: BookmarkItem,
    pub other: BookmarkItem,
    pub synced: Option<BookmarkItem>,
}

pub struct BookmarkMonitor {
    bookmarks_path: PathBuf,
    tx: mpsc::UnboundedSender<Vec<BookmarkItem>>,
}

impl BookmarkMonitor {
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<Vec<BookmarkItem>>)> {
        let bookmarks_path = Self::get_chrome_bookmarks_path()?;
        let (tx, rx) = mpsc::unbounded_channel();

        Ok((
            Self {
                bookmarks_path,
                tx,
            },
            rx,
        ))
    }

    pub fn get_chrome_bookmarks_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;

        #[cfg(target_os = "windows")]
        let bookmarks_path = home_dir
            .join("AppData")
            .join("Local")
            .join("Google")
            .join("Chrome")
            .join("User Data")
            .join("Default")
            .join("Bookmarks");

        #[cfg(target_os = "macos")]
        let bookmarks_path = home_dir
            .join("Library")
            .join("Application Support")
            .join("Google")
            .join("Chrome")
            .join("Default")
            .join("Bookmarks");

        #[cfg(target_os = "linux")]
        let bookmarks_path = home_dir
            .join(".config")
            .join("google-chrome")
            .join("Default")
            .join("Bookmarks");

        if !bookmarks_path.exists() {
            return Err(format!("Chrome bookmarks file not found at: {:?}", bookmarks_path).into());
        }

        Ok(bookmarks_path)
    }

    pub fn parse_bookmarks(&self) -> Result<Vec<BookmarkItem>> {
        let content = fs::read_to_string(&self.bookmarks_path)?;
        let chrome_bookmarks: ChromeBookmarks = serde_json::from_str(&content)?;

        let mut all_bookmarks = Vec::new();

        // Extract bookmarks from bookmark bar
        self.extract_bookmarks(&chrome_bookmarks.roots.bookmark_bar, &mut all_bookmarks);

        // Extract bookmarks from other bookmarks
        self.extract_bookmarks(&chrome_bookmarks.roots.other, &mut all_bookmarks);

        // Extract synced bookmarks if present
        if let Some(synced) = &chrome_bookmarks.roots.synced {
            self.extract_bookmarks(synced, &mut all_bookmarks);
        }

        Ok(all_bookmarks)
    }

    fn extract_bookmarks(&self, item: &BookmarkItem, bookmarks: &mut Vec<BookmarkItem>) {
        if let Some(url) = &item.url {
            // This is a bookmark (leaf node)
            if !url.is_empty() {
                bookmarks.push(item.clone());
            }
        }

        // Recursively process children (folders)
        if let Some(children) = &item.children {
            for child in children {
                self.extract_bookmarks(child, bookmarks);
            }
        }
    }

    pub async fn start_monitoring(&self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);
        let bookmarks_path_watcher = self.bookmarks_path.clone();
        let bookmarks_path_monitor = self.bookmarks_path.clone();
        let notification_tx = self.tx.clone();

        // Create watcher in a blocking thread
        let _handle = tokio::task::spawn_blocking(move || {
            let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
                match res {
                    Ok(event) => {
                        if matches!(event.kind, EventKind::Modify(_)) {
                            if let Err(e) = tx.blocking_send(()) {
                                eprintln!("Failed to send file change notification: {}", e);
                            }
                        }
                    }
                    Err(e) => eprintln!("Watch error: {:?}", e),
                }
            }).unwrap();

            watcher.watch(&bookmarks_path_watcher, RecursiveMode::NonRecursive).unwrap();

            // Keep the watcher alive
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        // Process file change notifications
        tokio::spawn(async move {
            while let Some(_) = rx.recv().await {
                // Debounce: wait a bit for file to stabilize
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Parse bookmarks and send update
                let monitor = BookmarkMonitor {
                    bookmarks_path: bookmarks_path_monitor.clone(),
                    tx: notification_tx.clone(),
                };

                match monitor.parse_bookmarks() {
                    Ok(bookmarks) => {
                        if let Err(e) = notification_tx.send(bookmarks) {
                            eprintln!("Failed to send bookmark update: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse bookmarks: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn get_bookmarks_for_ingestion(&self) -> Result<Vec<(String, String, String, bool)>> {
        let bookmarks = self.parse_bookmarks()?;
        let mut result = Vec::new();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("LocalMind/1.0")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        for bookmark in bookmarks {
            if let Some(url) = &bookmark.url {
                let title = if bookmark.name.is_empty() {
                    url.clone()
                } else {
                    bookmark.name.clone()
                };

                // Check if URL is accessible and fetch content
                let (content, is_dead) = match self.fetch_url_content(&client, url).await {
                    Ok(content) => (content, false),
                    Err(e) => {
                        println!("⚠️ Failed to fetch content from {}: {}", url, e);
                        if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                            println!("🚫 Marking {} as dead (404)", url);
                            (format!("Bookmark: {}\nURL: {} (DEAD - 404)", title, url), true)
                        } else {
                            // For other errors, don't mark as dead but use placeholder content
                            (format!("Bookmark: {}\nURL: {} (Error: {})", title, url, e), false)
                        }
                    }
                };

                result.push((title, content, url.clone(), is_dead));
            }
        }

        Ok(result)
    }

    async fn fetch_url_content(&self, client: &reqwest::Client, url: &str) -> Result<String> {
        let response = client.get(url).send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err("404 Not Found".into());
        }

        if !response.status().is_success() {
            return Err(format!("HTTP {}: {}", response.status().as_u16(), response.status().canonical_reason().unwrap_or("Unknown")).into());
        }

        let html = response.text().await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        // Basic HTML content extraction - just get text between <title> tags for now
        let title = self.extract_title_from_html(&html);
        let content = self.extract_content_from_html(&html);

        Ok(format!("Title: {}\nContent: {}", title, content))
    }

    fn extract_title_from_html(&self, html: &str) -> String {
        if let Some(start) = html.find("<title>") {
            if let Some(end) = html[start + 7..].find("</title>") {
                return html[start + 7..start + 7 + end].trim().to_string();
            }
        }
        "No title found".to_string()
    }

    fn extract_content_from_html(&self, html: &str) -> String {
        // Very basic content extraction - remove HTML tags and get first 500 chars
        let mut content = html.clone();

        // Remove script and style tags with their content
        while let Some(start) = content.find("<script") {
            if let Some(end) = content[start..].find("</script>") {
                content.replace_range(start..start + end + 9, "");
            } else {
                break;
            }
        }

        while let Some(start) = content.find("<style") {
            if let Some(end) = content[start..].find("</style>") {
                content.replace_range(start..start + end + 8, "");
            } else {
                break;
            }
        }

        // Remove all HTML tags
        let mut result = String::new();
        let mut in_tag = false;

        for ch in content.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(ch),
                _ => {}
            }
        }

        // Clean up whitespace and limit length
        result
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect()
    }
}

impl Default for BookmarkMonitor {
    fn default() -> Self {
        Self::new().unwrap().0
    }
}