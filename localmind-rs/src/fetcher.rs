use reqwest;
use scraper::{Html, Selector};
use std::time::Duration;

pub struct WebFetcher {
    client: reqwest::Client,
}

impl WebFetcher {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    pub async fn fetch_page_content(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        println!("🌐 Fetching content from: {}", url);

        // Skip non-HTTP(S) URLs
        if !url.starts_with("http://") && !url.starts_with("https://") {
            println!("⏭️ Skipping non-HTTP URL: {}", url);
            return Ok(String::new());
        }

        // Fetch the page
        let response = match self.client.get(url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                println!("❌ Failed to fetch {}: {}", url, e);
                return Ok(String::new());
            }
        };

        // Check status
        if !response.status().is_success() {
            println!("❌ HTTP {} for {}", response.status(), url);
            return Ok(String::new());
        }

        // Get HTML text
        let html = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                println!("❌ Failed to get text from {}: {}", url, e);
                return Ok(String::new());
            }
        };

        // Parse HTML and extract text
        let document = Html::parse_document(&html);

        // Remove script and style elements
        let script_selector = Selector::parse("script").unwrap();
        let style_selector = Selector::parse("style").unwrap();

        let mut text_content = String::new();

        // Try to get title
        if let Ok(title_selector) = Selector::parse("title") {
            if let Some(title) = document.select(&title_selector).next() {
                text_content.push_str(&title.inner_html());
                text_content.push_str("\n\n");
            }
        }

        // Try to get meta description
        if let Ok(meta_selector) = Selector::parse("meta[name=\"description\"]") {
            if let Some(meta) = document.select(&meta_selector).next() {
                if let Some(content) = meta.value().attr("content") {
                    text_content.push_str(content);
                    text_content.push_str("\n\n");
                }
            }
        }

        // Try to get article or main content
        let content_selectors = vec![
            "article", "main", "[role=\"main\"]", ".content", "#content",
            "div.post", "div.entry", "div.article-body"
        ];

        let mut found_content = false;
        for selector_str in content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let text = extract_text_from_element(&element, &script_selector, &style_selector);
                    if text.len() > 100 {  // Only use if substantial content
                        text_content.push_str(&text);
                        found_content = true;
                        break;
                    }
                }
            }
        }

        // Fallback to body if no specific content found
        if !found_content {
            if let Ok(body_selector) = Selector::parse("body") {
                if let Some(body) = document.select(&body_selector).next() {
                    text_content.push_str(&extract_text_from_element(&body, &script_selector, &style_selector));
                }
            }
        }

        // Clean up whitespace
        let cleaned = text_content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        // Limit content size to avoid huge documents
        let max_chars = 50000;
        let result = if cleaned.len() > max_chars {
            format!("{}...\n[Content truncated at {} chars]", &cleaned[..max_chars], max_chars)
        } else {
            cleaned
        };

        println!("✅ Fetched {} chars from {}", result.len(), url);
        Ok(result)
    }
}

fn extract_text_from_element(
    element: &scraper::ElementRef,
    script_selector: &Selector,
    style_selector: &Selector,
) -> String {
    let mut text = String::new();

    for node in element.descendants() {
        if let Some(element) = node.value().as_element() {
            // Skip script and style tags
            if element.name() == "script" || element.name() == "style" {
                continue;
            }
        }

        if let Some(text_node) = node.value().as_text() {
            let trimmed = text_node.trim();
            if !trimmed.is_empty() {
                text.push_str(trimmed);
                text.push(' ');
            }
        }
    }

    text
}