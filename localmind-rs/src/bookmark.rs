use crate::{
    bookmark_exclusion::ExclusionRules, fetcher::WebFetcher, youtube::YouTubeProcessor, Result,
};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkItem {
    pub date_added: String,
    pub date_modified: Option<String>,
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub children: Option<Vec<BookmarkItem>>,
}

#[derive(Debug, Clone)]
pub struct BookmarkItemWithPath {
    pub item: BookmarkItem,
    pub folder_path: Vec<String>,
    pub folder_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookmarkFolder {
    pub id: String,
    pub name: String,
    pub path: Vec<String>,
    pub bookmark_count: usize,
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

        Ok((Self { bookmarks_path, tx }, rx))
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

    pub fn get_bookmark_roots(&self) -> Result<Vec<BookmarkItem>> {
        let content = fs::read_to_string(&self.bookmarks_path)?;
        let chrome_bookmarks: ChromeBookmarks = serde_json::from_str(&content)?;

        let mut roots = Vec::new();
        roots.push(chrome_bookmarks.roots.bookmark_bar);
        roots.push(chrome_bookmarks.roots.other);
        if let Some(synced) = chrome_bookmarks.roots.synced {
            roots.push(synced);
        }

        Ok(roots)
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

    pub fn extract_bookmarks_with_exclusion(
        &self,
        item: &BookmarkItem,
        bookmarks: &mut Vec<BookmarkItemWithPath>,
        exclusion_rules: &ExclusionRules,
        current_path: &[String],
        current_folder_id: &str,
    ) {
        // Check if current folder is excluded
        if exclusion_rules.is_folder_excluded(current_folder_id) {
            return; // Skip entire folder and all children
        }

        if let Some(url) = &item.url {
            // This is a bookmark (leaf node)
            if !url.is_empty() {
                // Check if URL matches exclusion pattern
                if !exclusion_rules.is_url_excluded(url) {
                    bookmarks.push(BookmarkItemWithPath {
                        item: item.clone(),
                        folder_path: current_path.to_vec(),
                        folder_id: current_folder_id.to_string(),
                    });
                }
            }
        }

        // Recursively process children (folders)
        if let Some(children) = &item.children {
            let mut new_path = current_path.to_vec();
            new_path.push(item.name.clone());

            for child in children {
                self.extract_bookmarks_with_exclusion(
                    child,
                    bookmarks,
                    exclusion_rules,
                    &new_path,
                    &item.id,
                );
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
            let mut watcher =
                notify::recommended_watcher(move |res: notify::Result<Event>| match res {
                    Ok(event) => {
                        if matches!(event.kind, EventKind::Modify(_)) {
                            if let Err(e) = tx.blocking_send(()) {
                                eprintln!("Failed to send file change notification: {}", e);
                            }
                        }
                    }
                    Err(e) => eprintln!("Watch error: {:?}", e),
                })
                .unwrap();

            watcher
                .watch(&bookmarks_path_watcher, RecursiveMode::NonRecursive)
                .unwrap();

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
        let fetcher = WebFetcher::new();

        println!(
            "Starting bookmark ingestion for {} bookmarks",
            bookmarks.len()
        );

        for bookmark in bookmarks {
            if let Some(url) = &bookmark.url {
                let title = if bookmark.name.is_empty() {
                    url.clone()
                } else {
                    bookmark.name.clone()
                };

                println!("Processing bookmark: {} ({})", title, url);

                // Check if this is a YouTube URL and handle specially
                let (processed_title, content) = if YouTubeProcessor::is_youtube_url(url) {
                    println!("Processing YouTube bookmark: {}", url);

                    // Clean up YouTube title
                    let cleaned_title = YouTubeProcessor::cleanup_title(&title);

                    // Try to get transcript first
                    match YouTubeProcessor::fetch_transcript(url).await {
                        Ok(Some(transcript)) => {
                            println!("Using YouTube transcript for bookmark: {}", cleaned_title);
                            (
                                cleaned_title,
                                format!("Bookmark: {}\nURL: {}\n\n{}", title, url, transcript),
                            )
                        }
                        Ok(None) => {
                            println!("⚠️ No YouTube transcript available, using fallback content");
                            // Use WebFetcher as fallback
                            let fallback_content = match fetcher.fetch_page_content(url).await {
                                Ok(content) => {
                                    if content.is_empty() {
                                        format!(
                                            "Bookmark: {}\nURL: {}\n\n[No content extracted]",
                                            title, url
                                        )
                                    } else {
                                        format!("Bookmark: {}\nURL: {}\n\n{}", title, url, content)
                                    }
                                }
                                Err(e) => {
                                    println!(
                                        "⚠️ Failed to fetch fallback content from {}: {}",
                                        url, e
                                    );
                                    format!(
                                        "Bookmark: {}\nURL: {}\n\n[Error fetching content: {}]",
                                        title, url, e
                                    )
                                }
                            };
                            (cleaned_title, fallback_content)
                        }
                        Err(e) => {
                            println!(
                                "⚠️ Failed to fetch YouTube transcript: {}, using fallback content",
                                e
                            );
                            // Use WebFetcher as fallback
                            let fallback_content = match fetcher.fetch_page_content(url).await {
                                Ok(content) => {
                                    if content.is_empty() {
                                        format!(
                                            "Bookmark: {}\nURL: {}\n\n[No content extracted]",
                                            title, url
                                        )
                                    } else {
                                        format!("Bookmark: {}\nURL: {}\n\n{}", title, url, content)
                                    }
                                }
                                Err(e) => {
                                    println!(
                                        "⚠️ Failed to fetch fallback content from {}: {}",
                                        url, e
                                    );
                                    format!(
                                        "Bookmark: {}\nURL: {}\n\n[Error fetching content: {}]",
                                        title, url, e
                                    )
                                }
                            };
                            (cleaned_title, fallback_content)
                        }
                    }
                } else {
                    // Regular webpage processing
                    let content = match fetcher.fetch_page_content(url).await {
                        Ok(content) => {
                            if content.is_empty() {
                                format!(
                                    "Bookmark: {}\nURL: {}\n\n[No content extracted]",
                                    title, url
                                )
                            } else {
                                format!("Bookmark: {}\nURL: {}\n\n{}", title, url, content)
                            }
                        }
                        Err(e) => {
                            println!("⚠️ Failed to fetch content from {}: {}", url, e);
                            format!(
                                "Bookmark: {}\nURL: {}\n\n[Error fetching content: {}]",
                                title, url, e
                            )
                        }
                    };
                    (title, content)
                };

                let content_len = content.len();
                result.push((processed_title.clone(), content, url.clone(), false));
                println!(
                    "Processed bookmark: {} ({} chars)",
                    processed_title, content_len
                );
            }
        }

        println!("Processed {} bookmarks total", result.len());

        Ok(result)
    }

    pub async fn get_bookmarks_metadata(&self) -> Result<Vec<(String, String)>> {
        self.get_bookmarks_metadata_with_exclusion(&ExclusionRules::empty())
            .await
    }

    pub async fn get_bookmarks_metadata_with_exclusion(
        &self,
        exclusion_rules: &ExclusionRules,
    ) -> Result<Vec<(String, String)>> {
        let roots = self.get_bookmark_roots()?;
        let mut bookmarks_with_paths = Vec::new();

        for root in &roots {
            self.extract_bookmarks_with_exclusion(
                root,
                &mut bookmarks_with_paths,
                exclusion_rules,
                &[],
                &root.id,
            );
        }

        let mut result = Vec::new();
        println!(
            "Found {} bookmarks for processing (after exclusions)",
            bookmarks_with_paths.len()
        );

        for bookmark_item in bookmarks_with_paths {
            if let Some(url) = &bookmark_item.item.url {
                let title = if bookmark_item.item.name.is_empty() {
                    url.clone()
                } else {
                    bookmark_item.item.name.clone()
                };

                // Clean up YouTube titles
                let processed_title = if YouTubeProcessor::is_youtube_url(url) {
                    YouTubeProcessor::cleanup_title(&title)
                } else {
                    title
                };

                result.push((processed_title, url.clone()));
            }
        }

        Ok(result)
    }

    pub fn get_bookmark_folders(&self) -> Vec<BookmarkFolder> {
        match self.parse_bookmarks_with_folders() {
            Ok(folders) => {
                println!("Successfully parsed {} bookmark folders", folders.len());
                folders
            }
            Err(e) => {
                eprintln!("ERROR: Failed to parse bookmark folders: {}", e);
                eprintln!("Bookmarks path: {:?}", self.bookmarks_path);
                Vec::new()
            }
        }
    }

    fn parse_bookmarks_with_folders(&self) -> Result<Vec<BookmarkFolder>> {
        let content = fs::read_to_string(&self.bookmarks_path)?;
        let chrome_bookmarks: ChromeBookmarks = serde_json::from_str(&content)?;

        let mut folders = Vec::new();

        // Extract folders from bookmark bar
        self.extract_folders(&chrome_bookmarks.roots.bookmark_bar, &mut folders, &[]);

        // Extract folders from other bookmarks
        self.extract_folders(&chrome_bookmarks.roots.other, &mut folders, &[]);

        // Extract synced bookmarks if present
        if let Some(synced) = &chrome_bookmarks.roots.synced {
            self.extract_folders(synced, &mut folders, &[]);
        }

        Ok(folders)
    }

    fn extract_folders(
        &self,
        item: &BookmarkItem,
        folders: &mut Vec<BookmarkFolder>,
        current_path: &[String],
    ) {
        // Skip if this is a bookmark (has a URL)
        if item.url.is_some() {
            return;
        }

        // If this item has children, it's a folder
        if let Some(children) = &item.children {
            // Count bookmarks in this folder (recursively)
            let bookmark_count = self.count_bookmarks_in_folder(item);

            // Build the path for this folder
            let mut folder_path = current_path.to_vec();
            if !item.name.is_empty() {
                folder_path.push(item.name.clone());
            }

            // Add this folder to the list (only if it has a name and isn't a root)
            if !item.name.is_empty() {
                folders.push(BookmarkFolder {
                    id: item.id.clone(),
                    name: item.name.clone(),
                    path: folder_path.clone(),
                    bookmark_count,
                });
            }

            // Recursively process child folders
            for child in children {
                self.extract_folders(child, folders, &folder_path);
            }
        }
    }

    fn count_bookmarks_in_folder(&self, item: &BookmarkItem) -> usize {
        let mut count = 0;

        if item.url.is_some() {
            count += 1;
        }

        if let Some(children) = &item.children {
            for child in children {
                count += self.count_bookmarks_in_folder(child);
            }
        }

        count
    }

    pub async fn fetch_bookmark_content(&self, url: &str) -> Result<String> {
        let fetcher = WebFetcher::new();

        // Check if this is a YouTube URL and try to get transcript
        if YouTubeProcessor::is_youtube_url(url) {
            println!("Processing YouTube bookmark: {}", url);
            match YouTubeProcessor::fetch_transcript(url).await {
                Ok(Some(transcript)) => {
                    println!("Using YouTube transcript for bookmark: {}", url);
                    return Ok(format!("Bookmark: {}\nURL: {}\n\n{}", url, url, transcript));
                }
                Ok(None) => {
                    println!("⚠️ No YouTube transcript available, using original content");
                }
                Err(e) => {
                    println!(
                        "⚠️ Failed to fetch YouTube transcript: {}, using original content",
                        e
                    );
                }
            }
        }

        // Fallback to regular content fetching
        let content = match fetcher.fetch_page_content(url).await {
            Ok(content) => {
                if content.is_empty() {
                    format!("Bookmark: {}\nURL: {}\n\n[No content extracted]", url, url)
                } else {
                    format!("Bookmark: {}\nURL: {}\n\n{}", url, url, content)
                }
            }
            Err(e) => {
                println!("⚠️ Failed to fetch content from {}: {}", url, e);
                format!(
                    "Bookmark: {}\nURL: {}\n\n[Error fetching content: {}]",
                    url, url, e
                )
            }
        };

        Ok(content)
    }
}

impl Default for BookmarkMonitor {
    fn default() -> Self {
        Self::new().unwrap().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bookmarks_with_exclusion_rules() {
        // This test will verify that exclusion rules filter out bookmarks
        let monitor = BookmarkMonitor::new().unwrap().0;
        let exclusion_rules = ExclusionRules::new(
            vec!["excluded_folder_id".to_string()],
            vec!["*.internal.com".to_string()],
        );

        // Create a test bookmark structure
        let mut bookmarks = Vec::new();
        let test_item = BookmarkItem {
            date_added: "1234567890".to_string(),
            date_modified: None,
            id: "test_id".to_string(),
            name: "Test Bookmark".to_string(),
            url: Some("https://example.com".to_string()),
            children: None,
        };

        monitor.extract_bookmarks_with_exclusion(
            &test_item,
            &mut bookmarks,
            &exclusion_rules,
            &[],
            "root",
        );

        // Should not be excluded
        assert_eq!(bookmarks.len(), 1);
    }

    #[test]
    fn test_extract_bookmarks_excludes_by_folder() {
        let monitor = BookmarkMonitor::new().unwrap().0;
        let exclusion_rules = ExclusionRules::new(vec!["excluded_folder".to_string()], vec![]);

        let mut bookmarks = Vec::new();
        let test_item = BookmarkItem {
            date_added: "1234567890".to_string(),
            date_modified: None,
            id: "test_id".to_string(),
            name: "Test Bookmark".to_string(),
            url: Some("https://example.com".to_string()),
            children: None,
        };

        monitor.extract_bookmarks_with_exclusion(
            &test_item,
            &mut bookmarks,
            &exclusion_rules,
            &[],
            "excluded_folder",
        );

        // Should be excluded
        assert_eq!(bookmarks.len(), 0);
    }

    #[test]
    fn test_extract_bookmarks_excludes_by_domain() {
        let monitor = BookmarkMonitor::new().unwrap().0;
        let exclusion_rules = ExclusionRules::new(vec![], vec!["*.internal.com".to_string()]);

        let mut bookmarks = Vec::new();
        let test_item = BookmarkItem {
            date_added: "1234567890".to_string(),
            date_modified: None,
            id: "test_id".to_string(),
            name: "Internal Site".to_string(),
            url: Some("https://foo.internal.com/page".to_string()),
            children: None,
        };

        monitor.extract_bookmarks_with_exclusion(
            &test_item,
            &mut bookmarks,
            &exclusion_rules,
            &[],
            "root",
        );

        // Should be excluded
        assert_eq!(bookmarks.len(), 0);
    }

    #[test]
    fn test_extract_bookmarks_tracks_folder_path() {
        let monitor = BookmarkMonitor::new().unwrap().0;
        let exclusion_rules = ExclusionRules::empty();

        let mut bookmarks = Vec::new();
        let test_item = BookmarkItem {
            date_added: "1234567890".to_string(),
            date_modified: None,
            id: "test_id".to_string(),
            name: "Test Bookmark".to_string(),
            url: Some("https://example.com".to_string()),
            children: None,
        };

        let folder_path = vec!["Bookmark Bar".to_string(), "Work".to_string()];
        monitor.extract_bookmarks_with_exclusion(
            &test_item,
            &mut bookmarks,
            &exclusion_rules,
            &folder_path,
            "folder_123",
        );

        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].folder_path, folder_path);
        assert_eq!(bookmarks[0].folder_id, "folder_123");
    }

    #[test]
    fn test_get_bookmark_folders_structure() {
        // Test that we can extract folder structure from Chrome bookmarks
        let monitor = BookmarkMonitor::new().unwrap().0;
        let folders = monitor.get_bookmark_folders();

        // Should return folders from actual Chrome bookmarks file (if it exists)
        // This will be > 0 if Chrome bookmarks exist, 0 otherwise
        println!("Found {} bookmark folders", folders.len());

        // Verify structure if folders exist
        if !folders.is_empty() {
            for folder in folders.iter().take(3) {
                assert!(!folder.id.is_empty(), "Folder ID should not be empty");
                assert!(!folder.name.is_empty(), "Folder name should not be empty");
                assert!(
                    folder.bookmark_count >= 0,
                    "Bookmark count should be non-negative"
                );
                println!(
                    "  Folder: {} (ID: {}, Count: {})",
                    folder.name, folder.id, folder.bookmark_count
                );
            }
        }
    }
}
