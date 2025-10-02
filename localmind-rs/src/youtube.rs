use crate::Result;
use yt_transcript_rs::YouTubeTranscriptApi;
use url::Url;

pub struct YouTubeProcessor;

impl YouTubeProcessor {
    /// Check if a URL is a YouTube video URL
    pub fn is_youtube_url(url: &str) -> bool {
        if let Ok(parsed_url) = Url::parse(url) {
            match parsed_url.host_str() {
                Some(host) => {
                    host == "youtube.com"
                        || host == "www.youtube.com"
                        || host == "youtu.be"
                        || host == "www.youtu.be"
                        || host == "m.youtube.com"
                },
                None => false,
            }
        } else {
            false
        }
    }

    /// Extract video ID from YouTube URL
    pub fn extract_video_id(url: &str) -> Option<String> {
        if let Ok(parsed_url) = Url::parse(url) {
            match parsed_url.host_str() {
                Some("youtu.be") | Some("www.youtu.be") => {
                    // Format: https://youtu.be/VIDEO_ID
                    parsed_url.path()
                        .strip_prefix('/')
                        .map(|id| id.to_string())
                }
                Some("youtube.com") | Some("www.youtube.com") | Some("m.youtube.com") => {
                    // Format: https://www.youtube.com/watch?v=VIDEO_ID
                    if let Some(query) = parsed_url.query() {
                        for pair in query.split('&') {
                            if let Some((key, value)) = pair.split_once('=') {
                                if key == "v" {
                                    return Some(value.to_string());
                                }
                            }
                        }
                    }
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Clean up YouTube video title by removing bracketed numbers
    pub fn cleanup_title(title: &str) -> String {
        // Remove bracketed numbers at the beginning: "(1) Video Title" -> "Video Title"
        let cleaned = if let Some(captures) = regex::Regex::new(r"^\([^)]*\)\s*")
            .ok()
            .and_then(|re| re.find(title))
        {
            title[captures.end()..].to_string()
        } else {
            title.to_string()
        };

        cleaned.trim().to_string()
    }

    /// Fetch transcript for a YouTube video
    pub async fn fetch_transcript(url: &str) -> Result<Option<String>> {
        let video_id = match Self::extract_video_id(url) {
            Some(id) => id,
            None => return Ok(None),
        };

        println!("Fetching YouTube transcript for video ID: {}", video_id);

        // Initialize YouTube transcript API
        let api = YouTubeTranscriptApi::new(None, None, None)
            .map_err(|e| format!("Failed to initialize YouTube transcript API: {}", e))?;

        // Fetch transcript
        match api.fetch_transcript(&video_id, &["en"], false).await {
            Ok(transcript) => {
                let text = transcript.text();
                if text.trim().is_empty() {
                    println!("⚠️ Empty transcript received for video: {}", video_id);
                    Ok(None)
                } else {
                    println!("Successfully fetched transcript ({} chars) for video: {}", text.len(), video_id);
                    Ok(Some(text))
                }
            }
            Err(e) => {
                println!("⚠️ Failed to fetch YouTube transcript for {}: {}", video_id, e);
                Ok(None)
            }
        }
    }

    /// Process YouTube URL and return enhanced content if transcript is available
    pub async fn process_youtube_content(url: &str, original_title: &str, original_content: &str) -> Result<(String, String)> {
        if !Self::is_youtube_url(url) {
            return Ok((original_title.to_string(), original_content.to_string()));
        }

        println!("Processing YouTube URL: {}", url);

        // Clean up title
        let cleaned_title = Self::cleanup_title(original_title);

        // Try to fetch transcript
        match Self::fetch_transcript(url).await? {
            Some(transcript) => {
                println!("Using transcript as content for YouTube video: {}", cleaned_title);
                Ok((cleaned_title, transcript))
            }
            None => {
                println!("⚠️ No transcript available, using original content for: {}", cleaned_title);
                Ok((cleaned_title, original_content.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_youtube_url_detection() {
        assert!(YouTubeProcessor::is_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(YouTubeProcessor::is_youtube_url("https://youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(YouTubeProcessor::is_youtube_url("https://youtu.be/dQw4w9WgXcQ"));
        assert!(YouTubeProcessor::is_youtube_url("https://m.youtube.com/watch?v=dQw4w9WgXcQ"));

        assert!(!YouTubeProcessor::is_youtube_url("https://example.com"));
        assert!(!YouTubeProcessor::is_youtube_url("https://google.com"));
        assert!(!YouTubeProcessor::is_youtube_url("not a url"));
    }

    #[test]
    fn test_video_id_extraction() {
        assert_eq!(
            YouTubeProcessor::extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            YouTubeProcessor::extract_video_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            YouTubeProcessor::extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=30s"),
            Some("dQw4w9WgXcQ".to_string())
        );

        assert_eq!(YouTubeProcessor::extract_video_id("https://example.com"), None);
        assert_eq!(YouTubeProcessor::extract_video_id("not a url"), None);
    }

    #[test]
    fn test_title_cleanup() {
        assert_eq!(
            YouTubeProcessor::cleanup_title("(1) Amazing Video Title"),
            "Amazing Video Title"
        );
        assert_eq!(
            YouTubeProcessor::cleanup_title("(42) Another Video"),
            "Another Video"
        );
        assert_eq!(
            YouTubeProcessor::cleanup_title("Regular Title Without Brackets"),
            "Regular Title Without Brackets"
        );
        assert_eq!(
            YouTubeProcessor::cleanup_title("(New) YouTube Video"),
            "YouTube Video"
        );
    }
}