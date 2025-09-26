use reqwest;
use scraper::{Html, Selector};
use std::time::Duration;
use pdf_extract;

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

        // Check content type to handle different file types properly
        let content_type = response.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        // Handle PDF files
        if content_type.contains("application/pdf") || url.to_lowercase().ends_with(".pdf") {
            println!("📄 Detected PDF file: {}", url);

            // Get binary content for PDF
            let pdf_bytes = match response.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    println!("❌ Failed to get PDF bytes from {}: {}", url, e);
                    return Ok(String::new());
                }
            };

            // Extract text from PDF
            let filename = url.split('/').last().unwrap_or("document.pdf");

            match pdf_extract::extract_text_from_mem(&pdf_bytes) {
                Ok(text) if !text.trim().is_empty() => {
                    let cleaned_text = text
                        .lines()
                        .map(|line| line.trim())
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");

                    let result = if cleaned_text.len() > 50000 {
                        // Ensure UTF-8 boundary safety when truncating
                        let mut boundary = 50000;
                        while boundary > 0 && !cleaned_text.is_char_boundary(boundary) {
                            boundary -= 1;
                        }
                        if boundary == 0 {
                            format!("PDF Document: {}\nURL: {}\n\n[PDF content too large and unable to find safe UTF-8 boundary]", filename, url)
                        } else {
                            format!("PDF Document: {}\nURL: {}\n\n{}...\n\n[PDF content truncated at {} chars]", filename, url, &cleaned_text[..boundary], boundary)
                        }
                    } else {
                        format!("PDF Document: {}\nURL: {}\n\n{}", filename, url, cleaned_text)
                    };

                    println!("✅ Extracted {} chars of text from PDF: {}", result.len(), url);
                    return Ok(result);
                }
                Ok(_) => {
                    // PDF parsed but no text content
                    let placeholder = format!(
                        "PDF Document: {}\nURL: {}\nSize: {} bytes\n\n[This PDF file contains no extractable text content - it may be image-based or encrypted]",
                        filename, url, pdf_bytes.len()
                    );
                    println!("⚠️ PDF contains no extractable text: {}", url);
                    return Ok(placeholder);
                }
                Err(e) => {
                    // PDF extraction failed, return safe placeholder
                    let placeholder = format!(
                        "PDF Document: {}\nURL: {}\nSize: {} bytes\n\n[PDF text extraction failed: {}. Document indexed for reference.]",
                        filename, url, pdf_bytes.len(), e
                    );
                    println!("⚠️ PDF text extraction failed for {}: {}", url, e);
                    return Ok(placeholder);
                }
            }
        }

        // Handle other binary content types that should not be processed as text
        if content_type.contains("image/")
            || content_type.contains("video/")
            || content_type.contains("audio/")
            || content_type.contains("application/zip")
            || content_type.contains("application/octet-stream") {
            println!("🚫 Skipping binary content type '{}': {}", content_type, url);
            let filename = url.split('/').last().unwrap_or("file");
            return Ok(format!("Binary file: {} ({})\nURL: {}", filename, content_type, url));
        }

        // Get HTML text (only for text-based content)
        let html = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                println!("❌ Failed to get text from {}: {}", url, e);
                return Ok(String::new());
            }
        };

        // Check if the content looks like binary data that was incorrectly served as text
        // PDF files often start with %PDF
        if html.starts_with("%PDF") {
            println!("📄 Detected PDF content served as text: {}", url);
            let filename = url.split('/').last().unwrap_or("document.pdf");

            // Try to extract text from the PDF content
            match pdf_extract::extract_text_from_mem(html.as_bytes()) {
                Ok(text) if !text.trim().is_empty() => {
                    let cleaned_text = text
                        .lines()
                        .map(|line| line.trim())
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");

                    let result = format!("PDF Document: {}\nURL: {}\n\n{}", filename, url, cleaned_text);
                    println!("✅ Extracted text from PDF served as text: {}", url);
                    return Ok(result);
                }
                _ => {
                    let placeholder = format!(
                        "PDF Document: {}\nURL: {}\n\n[This is a PDF file served as text content, but text extraction failed or no text found.]",
                        filename, url
                    );
                    println!("⚠️ Could not extract text from PDF served as text: {}", url);
                    return Ok(placeholder);
                }
            }
        }

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
            // Make sure we don't cut in the middle of a UTF-8 character
            let mut boundary = max_chars;
            while boundary > 0 && !cleaned.is_char_boundary(boundary) {
                boundary -= 1;
            }
            if boundary == 0 {
                format!("[Content too large and unable to find safe UTF-8 boundary]")
            } else {
                format!("{}...\n[Content truncated at {} chars]", &cleaned[..boundary], boundary)
            }
        } else {
            cleaned
        };

        println!("✅ Fetched {} chars from {}", result.len(), url);
        Ok(result)
    }
}

fn extract_text_from_element(
    element: &scraper::ElementRef,
    _script_selector: &Selector,
    _style_selector: &Selector,
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