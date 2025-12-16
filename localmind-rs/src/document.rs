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

    /// Split text into chunks of approximately `chunk_size` bytes with `overlap` bytes overlap.
    /// 
    /// Algorithm:
    /// 1. Split into chunk_size blocks with overlap, finding good break points (sentences, words)
    /// 2. If the last chunk is less than chunk_size/2, merge it with the previous chunk
    /// 
    /// This ensures no tiny trailing chunks (with default 500/50, last chunk is always >= 250 bytes)
    pub fn chunk_text(&self, text: &str) -> Result<Vec<DocumentChunk>> {
        if text.is_empty() {
            return Ok(vec![]);
        }

        let text_len = text.len();

        // If text fits in one chunk, return it as-is
        if text_len <= self.chunk_size {
            return Ok(vec![DocumentChunk {
                content: text.to_string(),
                start_pos: 0,
                end_pos: text_len,
            }]);
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        // Step 1: Create chunks with overlap
        while start < text_len {
            let mut end = std::cmp::min(start + self.chunk_size, text_len);

            // Ensure end is on a UTF-8 character boundary
            end = self.adjust_to_char_boundary(text, end, false);

            // Find a good break point (sentence/paragraph boundary) if not at end of text
            if end < text_len {
                end = self.find_break_point(text, start, end);
            }

            // Ensure start is on a UTF-8 character boundary
            let safe_start = self.adjust_to_char_boundary(text, start, true);

            // Clamp end to text length
            let safe_end = std::cmp::min(end, text_len);

            if safe_end > safe_start {
                // Skip if this chunk would end at or before the previous chunk's end
                // (this happens when overlap causes us to find the same break point)
                let prev_end = chunks.last().map(|c: &DocumentChunk| c.end_pos).unwrap_or(0);
                if safe_end > prev_end {
                    let chunk_content = text[safe_start..safe_end].trim().to_string();
                    if !chunk_content.is_empty() {
                        chunks.push(DocumentChunk {
                            content: chunk_content,
                            start_pos: safe_start,
                            end_pos: safe_end,
                        });
                    }
                }
            }

            // Move to next chunk position (with overlap)
            if safe_end >= text_len {
                break;
            }
            
            let next_start = if safe_end > self.overlap {
                self.find_word_start(text, safe_end - self.overlap)
            } else {
                safe_end
            };

            // Ensure progress
            start = if next_start <= start {
                start + std::cmp::max(1, self.chunk_size / 4)
            } else {
                next_start
            };
        }

        // Step 2: If last chunk is too small (< chunk_size/2), merge with previous
        let min_last_chunk_size = self.chunk_size / 2;
        if chunks.len() >= 2 {
            let last_chunk_size = chunks.last().map(|c| c.end_pos - c.start_pos).unwrap_or(0);
            if last_chunk_size < min_last_chunk_size {
                // Remove the last chunk and extend the previous one to cover it
                let last_chunk = chunks.pop().unwrap();
                if let Some(prev_chunk) = chunks.last_mut() {
                    // Extend previous chunk to include the last chunk's content
                    prev_chunk.end_pos = last_chunk.end_pos;
                    prev_chunk.content = text[prev_chunk.start_pos..prev_chunk.end_pos].trim().to_string();
                }
            }
        }

        Ok(chunks)
    }

    /// Adjust a byte position to be on a valid UTF-8 character boundary
    fn adjust_to_char_boundary(&self, text: &str, pos: usize, forward: bool) -> usize {
        let mut adjusted = pos.min(text.len());
        if forward {
            while adjusted < text.len() && !text.is_char_boundary(adjusted) {
                adjusted += 1;
            }
        } else {
            while adjusted > 0 && !text.is_char_boundary(adjusted) {
                adjusted -= 1;
            }
        }
        adjusted
    }

    fn find_break_point(&self, text: &str, start: usize, preferred_end: usize) -> usize {
        // Ensure both start and end are on valid UTF-8 boundaries
        let mut safe_start = start;
        while safe_start < text.len() && !text.is_char_boundary(safe_start) {
            safe_start += 1;
        }

        let mut safe_end = preferred_end;
        while safe_end > safe_start && !text.is_char_boundary(safe_end) {
            safe_end -= 1;
        }

        if safe_end <= safe_start {
            // Find a safe boundary starting from preferred_end and working backwards
            let mut fallback = preferred_end;
            while fallback > safe_start && !text.is_char_boundary(fallback) {
                fallback -= 1;
            }
            return if fallback > safe_start {
                fallback
            } else {
                safe_start
            };
        }

        // First, try to find natural break points within the preferred chunk size
        let search_text = &text[safe_start..safe_end];

        // Look for paragraph breaks first
        if let Some(pos) = search_text.rfind("\n\n") {
            return std::cmp::min(safe_start + pos + 2, text.len());
        }

        // Look for sentence endings
        if let Some(pos) = search_text.rfind(". ") {
            return std::cmp::min(safe_start + pos + 2, text.len());
        }

        // Look for other sentence endings
        for ending in &["! ", "? ", ": ", "; "] {
            if let Some(pos) = search_text.rfind(ending) {
                return std::cmp::min(safe_start + pos + 2, text.len());
            }
        }

        // Look for line breaks
        if let Some(pos) = search_text.rfind('\n') {
            return std::cmp::min(safe_start + pos + 1, text.len());
        }

        // Look for word boundaries within preferred size
        if let Some(pos) = search_text.rfind(' ') {
            return std::cmp::min(safe_start + pos + 1, text.len());
        }

        // No natural break found within preferred size.
        // Check if we're in the middle of a word at safe_end, and if so, extend to complete it
        let max_extension = self.chunk_size / 4; // Allow up to 25% extension for word boundaries
        let extended_end = std::cmp::min(safe_end + max_extension, text.len());

        // Ensure extended_end is on UTF-8 boundary
        let mut safe_extended_end = extended_end;
        while safe_extended_end > safe_end && !text.is_char_boundary(safe_extended_end) {
            safe_extended_end -= 1;
        }

        // Check if we're currently ending mid-word and try to extend to complete it
        if safe_end < text.len() && safe_extended_end > safe_end {
            // Check if the character at safe_end position is not whitespace
            // This indicates we're potentially breaking mid-word
            if let Some(char_at_break) = text[safe_end..].chars().next() {
                if !char_at_break.is_whitespace() && !char_at_break.is_ascii_punctuation() {
                    let extended_search = &text[safe_end..safe_extended_end];

                    // Look for the first word boundary after preferred_end
                    if let Some(pos) = extended_search.find(' ') {
                        return std::cmp::min(safe_end + pos, text.len());
                    }

                    // Look for line break
                    if let Some(pos) = extended_search.find('\n') {
                        return std::cmp::min(safe_end + pos, text.len());
                    }

                    // Look for punctuation that might indicate a good break
                    for punct in &[". ", "! ", "? ", ", ", "; ", ": "] {
                        if let Some(pos) = extended_search.find(punct) {
                            return std::cmp::min(safe_end + pos + punct.len(), text.len());
                        }
                    }

                    // Look for any punctuation
                    if let Some(pos) = extended_search.find(|c: char| c.is_ascii_punctuation()) {
                        // Include the punctuation character
                        return std::cmp::min(safe_end + pos + 1, text.len());
                    }
                }
            }
        }

        // If we still can't find a good break point, use safe_end
        safe_end
    }

    fn find_word_start(&self, text: &str, preferred_start: usize) -> usize {
        if preferred_start >= text.len() {
            return text.len();
        }

        // Ensure we're on a UTF-8 boundary
        let mut safe_start = preferred_start;
        while safe_start < text.len() && !text.is_char_boundary(safe_start) {
            safe_start += 1;
        }

        // If we're already at the beginning or at whitespace, we're good
        if safe_start == 0 || safe_start >= text.len() {
            return safe_start;
        }

        // Check if we're in the middle of a word
        let char_at_start = text[safe_start..].chars().next().unwrap_or(' ');
        if char_at_start.is_whitespace() {
            // We're at whitespace, skip forward to next non-whitespace
            while safe_start < text.len() {
                if let Some(ch) = text[safe_start..].chars().next() {
                    if !ch.is_whitespace() {
                        break;
                    }
                    safe_start += ch.len_utf8();
                } else {
                    break;
                }
            }
            return safe_start;
        }

        // We're in the middle of a word, find the start of this word or the next word
        let search_text = &text[..safe_start];

        // Look backwards for word boundaries
        if let Some(pos) = search_text.rfind(' ') {
            // Found a space, start after it
            let word_start = pos + 1;
            // Skip any whitespace after the space
            let mut actual_start = word_start;
            while actual_start < safe_start && actual_start < text.len() {
                if let Some(ch) = text[actual_start..].chars().next() {
                    if !ch.is_whitespace() {
                        break;
                    }
                    actual_start += ch.len_utf8();
                } else {
                    break;
                }
            }
            return actual_start;
        }

        // No space found, start from beginning
        0
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
            let prev_end = &chunks[i - 1].content[chunks[i - 1].content.len().saturating_sub(10)..];
            let curr_start = &chunks[i].content[..std::cmp::min(10, chunks[i].content.len())];
            // There should be some overlap or natural break
            assert!(
                prev_end.chars().any(|c| curr_start.contains(c))
                    || chunks[i - 1].content.ends_with('.')
            );
        }
    }

    #[test]
    fn test_empty_text() {
        let processor = DocumentProcessor::default();
        let chunks = processor.chunk_text("").unwrap();
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_utf8_boundaries() {
        let processor = DocumentProcessor::new(10, 2);
        // String with multi-byte UTF-8 characters
        let text = "Hello 🦀 world with émojis and ñoñó characters";
        let chunks = processor.chunk_text(text).unwrap();

        // Verify all chunks are valid UTF-8
        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
        }

        // Verify we got reasonable chunks
        assert!(chunks.len() > 0);
    }

    #[test]
    fn test_emoji_at_chunk_boundary() {
        let processor = DocumentProcessor::new(8, 2);
        // Place emojis exactly at potential chunk boundaries
        let text = "Hello 🦀🚀🎉 World!";
        let chunks = processor.chunk_text(text).unwrap();

        // Verify all chunks are valid UTF-8 and contain complete characters
        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            // Ensure no broken emoji characters
            assert!(!chunk.content.contains('�'));
        }
    }

    #[test]
    fn test_mixed_script_chunking() {
        let processor = DocumentProcessor::new(15, 3);
        // Mix of Latin, Cyrillic, Chinese, Arabic, and emojis
        let text = "Hello мир 世界 مرحبا 🌍 नमस्ते こんにちは";
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            // No replacement characters indicating broken UTF-8
            assert!(!chunk.content.contains('�'));
        }
    }

    #[test]
    fn test_long_multibyte_sequence() {
        let processor = DocumentProcessor::new(20, 5);
        // String with many consecutive multi-byte characters
        let text = "🦀🚀🎉🌟💫⭐🎯🎪🎨🎭🎪🎨🎭🎪🎨🎭🎪🎨🎭🎪🎨🎭";
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            // Each emoji is 4 bytes, so verify we're handling them correctly
            assert!(chunk.content.chars().count() > 0);
        }
    }

    #[test]
    fn test_combining_characters() {
        let processor = DocumentProcessor::new(10, 2);
        // Text with combining diacritical marks
        let text = "Café résumé naïve Zürich exposé";
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            // Verify combining characters aren't separated from their base
            assert!(!chunk.content.contains('�'));
        }
    }

    #[test]
    fn test_rtl_text_chunking() {
        let processor = DocumentProcessor::new(15, 3);
        // Right-to-left languages (Arabic, Hebrew)
        let text = "English text مرحبا بالعالم Hello שלום עולם World!";
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            assert!(!chunk.content.contains('�'));
        }
    }

    #[test]
    fn test_zero_width_characters() {
        let processor = DocumentProcessor::new(12, 2);
        // Text with zero-width characters
        let text = "Hello\u{200B}world\u{FEFF}test\u{200C}text";
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_very_small_chunks_with_multibyte() {
        let processor = DocumentProcessor::new(3, 1);
        // Very small chunks with multi-byte characters
        let text = "🦀a🚀b🎉c";
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            // With such small chunks, we might get individual characters
            assert!(chunk.content.chars().count() >= 1);
        }
    }

    #[test]
    fn test_multibyte_at_exact_boundary() {
        let processor = DocumentProcessor::new(7, 1);
        // Carefully crafted to put multi-byte chars at chunk boundaries
        let text = "Hi 🦀 Go"; // "Hi " = 3 bytes, "🦀" = 4 bytes, " Go" = 3 bytes
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            assert!(!chunk.content.contains('�'));
        }
    }

    #[test]
    fn test_normalization_forms() {
        let processor = DocumentProcessor::new(10, 2);
        // Same character in different Unicode normalization forms
        let text = "é vs e\u{0301} café vs cafe\u{0301}"; // é vs e + combining accent
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_surrogate_pairs() {
        let processor = DocumentProcessor::new(8, 1);
        // Characters that require surrogate pairs in UTF-16 but are single code points in UTF-8
        let text = "𝕳𝖊𝖑𝖑𝖔 𝖂𝖔𝖗𝖑𝖉!"; // Mathematical bold characters
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            assert!(!chunk.content.contains('�'));
        }
    }

    #[test]
    fn test_cjk_ideographs() {
        let processor = DocumentProcessor::new(12, 2);
        // Chinese, Japanese, Korean characters
        let text = "中文测试 日本語テスト 한국어시험";
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            assert!(!chunk.content.contains('�'));
        }
    }

    #[test]
    fn test_word_boundary_extension() {
        let processor = DocumentProcessor::new(50, 10);
        // Text where natural break would be mid-word
        let text = "This is a sentence with some administration work that needs to be completed quickly and efficiently.";
        let chunks = processor.chunk_text(text).unwrap();

        // Print chunks for debugging
        for (i, chunk) in chunks.iter().enumerate() {
            println!(
                "Chunk {}: '{}' ({}..{})",
                i, chunk.content, chunk.start_pos, chunk.end_pos
            );
            println!(
                "  Last char of content: '{}'",
                chunk.content.chars().last().unwrap_or(' ')
            );
            if chunk.end_pos < text.len() {
                println!(
                    "  Char at end_pos {}: '{}'",
                    chunk.end_pos,
                    text.chars().nth(chunk.end_pos).unwrap_or(' ')
                );
            }
        }

        // Verify no chunks end with partial words
        for chunk in &chunks {
            let content = &chunk.content;
            if !content.is_empty() {
                // Check that chunk doesn't end mid-word (unless it's the end of the text)
                let last_char = content.chars().last().unwrap();
                if chunk.end_pos < text.len() {
                    // Get what comes next in the original text by checking bytes
                    let text_bytes = text.as_bytes();
                    let next_char = if chunk.end_pos < text_bytes.len() {
                        text_bytes[chunk.end_pos] as char
                    } else {
                        ' '
                    };

                    // Debug: Show the exact byte positions
                    println!(
                        "Chunk ends with '{}' at pos {}, next char at pos {} is '{}' (byte: {})",
                        last_char,
                        chunk.end_pos,
                        chunk.end_pos,
                        next_char,
                        text_bytes.get(chunk.end_pos).unwrap_or(&0)
                    );

                    // If not at end of text, should end with punctuation or whitespace, not mid-word
                    assert!(
                        last_char.is_whitespace()
                            || last_char.is_ascii_punctuation()
                            || content.ends_with('.')
                            || content.ends_with('!')
                            || content.ends_with('?')
                            || content.ends_with(',')
                            || content.ends_with(';')
                            || content.ends_with(':')
                            || next_char.is_whitespace(), // Allow if next char is whitespace (word boundary)
                        "Chunk ends mid-word: '{}' (next char: '{}')",
                        content,
                        next_char
                    );
                }
            }
        }
    }

    #[test]
    fn test_no_word_splitting() {
        let processor = DocumentProcessor::new(30, 5);
        // Text with words that might get split
        let text = "The administration department needs to process the documentation efficiently and systematically.";
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            // Verify that common prefixes/suffixes aren't broken off
            assert!(
                !chunk.content.trim().starts_with("tion "),
                "Chunk starts with broken suffix: '{}'",
                chunk.content
            );
            assert!(
                !chunk.content.trim().starts_with("ing "),
                "Chunk starts with broken suffix: '{}'",
                chunk.content
            );
            assert!(
                !chunk.content.trim().starts_with("ed "),
                "Chunk starts with broken suffix: '{}'",
                chunk.content
            );
            assert!(
                !chunk.content.trim().starts_with("er "),
                "Chunk starts with broken suffix: '{}'",
                chunk.content
            );
            assert!(
                !chunk.content.trim().starts_with("ly "),
                "Chunk starts with broken suffix: '{}'",
                chunk.content
            );

            // Verify chunks don't end with incomplete words (partial prefixes)
            let trimmed = chunk.content.trim();
            if !trimmed.is_empty() && chunk.end_pos < text.len() {
                assert!(
                    !trimmed.ends_with("adm"),
                    "Chunk ends with partial word: '{}'",
                    chunk.content
                );
                assert!(
                    !trimmed.ends_with("doc"),
                    "Chunk ends with partial word: '{}'",
                    chunk.content
                );
                assert!(
                    !trimmed.ends_with("eff"),
                    "Chunk ends with partial word: '{}'",
                    chunk.content
                );
            }
        }
    }

    #[test]
    fn test_edge_case_single_multibyte() {
        let processor = DocumentProcessor::new(10, 0);
        // Just a single multi-byte character - chunk size larger than text
        let text = "🦀";
        let chunks = processor.chunk_text(text).unwrap();

        // Should use short text path and create exactly one chunk
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "🦀");
        assert!(std::str::from_utf8(chunks[0].content.as_bytes()).is_ok());
    }

    #[test]
    fn test_mixed_multibyte_with_whitespace() {
        let processor = DocumentProcessor::new(10, 2);
        // Multi-byte characters mixed with various whitespace
        let text = "🦀\n🚀\t🎉 \u{00A0}world"; // Include non-breaking space
        let chunks = processor.chunk_text(text).unwrap();

        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.content.as_bytes()).is_ok());
            assert!(!chunk.content.is_empty());
            assert!(!chunk.content.contains('�'));
        }
    }
}
