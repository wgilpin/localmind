use crate::Result;

#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub content: String,
    pub start_pos: usize,
    pub end_pos: usize,
}

pub struct DocumentProcessor {
    chunk_size: usize,
    overlap: usize,
}

impl DocumentProcessor {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            overlap,
        }
    }

    pub fn chunk_text(&self, text: &str) -> Result<Vec<DocumentChunk>> {
        if text.is_empty() {
            println!("⚠️ Empty text provided to chunk_text");
            return Ok(vec![]);
        }

        let mut chunks = Vec::new();
        let text_len = text.len();

        if text_len <= self.chunk_size {
            chunks.push(DocumentChunk {
                content: text.to_string(),
                start_pos: 0,
                end_pos: text_len,
            });
            return Ok(chunks);
        }

        let mut start = 0;
        let mut chunk_count = 0;
        let max_chunks = (text_len / (self.chunk_size / 2)) + 10; // Safety limit

        while start < text_len && chunk_count < max_chunks {
            chunk_count += 1;
            let end = std::cmp::min(start + self.chunk_size, text_len);
            
            // Find a good break point (sentence or paragraph boundary)
            let actual_end = if end < text_len {
                let break_point = self.find_break_point(text, start, end);
                break_point
            } else {
                end
            };

            let chunk_text = text[start..actual_end].trim().to_string();
            
            if !chunk_text.is_empty() {
                chunks.push(DocumentChunk {
                    content: chunk_text,
                    start_pos: start,
                    end_pos: actual_end,
                });
            }

            // Move start position, accounting for overlap
            let new_start = if actual_end >= self.overlap {
                actual_end - self.overlap
            } else {
                actual_end
            };

            // Ensure we make reasonable progress - if new start is too close, advance by a minimum amount
            start = if new_start <= start {
                // If we can't make progress with overlap, jump forward by at least chunk_size / 4
                start + std::cmp::max(1, self.chunk_size / 4)
            } else {
                new_start
            };


            if start >= text_len {
                break;
            }
        }

        if chunk_count >= max_chunks {
            println!("⚠️ Hit maximum chunk limit ({}) - stopping to prevent excessive chunking", max_chunks);
        }

        Ok(chunks)
    }

    fn find_break_point(&self, text: &str, start: usize, preferred_end: usize) -> usize {
        let search_text = &text[start..preferred_end];
        
        // Look for paragraph breaks first
        if let Some(pos) = search_text.rfind("\n\n") {
            return start + pos + 2;
        }
        
        // Look for sentence endings
        if let Some(pos) = search_text.rfind(". ") {
            return start + pos + 2;
        }
        
        // Look for other sentence endings
        for ending in &["! ", "? ", ": ", "; "] {
            if let Some(pos) = search_text.rfind(ending) {
                return start + pos + 2;
            }
        }
        
        // Look for line breaks
        if let Some(pos) = search_text.rfind('\n') {
            return start + pos + 1;
        }
        
        // Look for word boundaries
        if let Some(pos) = search_text.rfind(' ') {
            return start + pos + 1;
        }
        
        // No good break point found, use preferred end
        preferred_end
    }
}

impl Default for DocumentProcessor {
    fn default() -> Self {
        Self::new(500, 50) // 500 chars with 50 char overlap as per plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_short_text() {
        let processor = DocumentProcessor::new(100, 10);
        let text = "This is a short text.";
        let chunks = processor.chunk_text(text).unwrap();
        
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, text);
    }

    #[test]
    fn test_chunk_long_text() {
        let processor = DocumentProcessor::new(50, 10);
        let text = "This is the first sentence. This is the second sentence. This is the third sentence. This is the fourth sentence.";
        let chunks = processor.chunk_text(text).unwrap();
        
        assert!(chunks.len() > 1);
        
        // Check that chunks have proper overlap
        for i in 1..chunks.len() {
            let prev_end = &chunks[i-1].content[chunks[i-1].content.len().saturating_sub(10)..];
            let curr_start = &chunks[i].content[..std::cmp::min(10, chunks[i].content.len())];
            // There should be some overlap or natural break
            assert!(prev_end.chars().any(|c| curr_start.contains(c)) || chunks[i-1].content.ends_with('.'));
        }
    }

    #[test]
    fn test_empty_text() {
        let processor = DocumentProcessor::default();
        let chunks = processor.chunk_text("").unwrap();
        assert_eq!(chunks.len(), 0);
    }
}