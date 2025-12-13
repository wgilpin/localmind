use crate::Result;
use std::collections::HashSet;
use url::Url;

/// Manages exclusion rules for bookmarks based on folder IDs and domain patterns
#[derive(Debug, Clone)]
pub struct ExclusionRules {
    excluded_folders: HashSet<String>,
    excluded_domain_patterns: Vec<String>,
}

impl ExclusionRules {
    /// Creates a new ExclusionRules instance
    pub fn new(folders: Vec<String>, domains: Vec<String>) -> Self {
        Self {
            excluded_folders: folders.into_iter().collect(),
            excluded_domain_patterns: domains,
        }
    }

    /// Creates an empty ExclusionRules instance (nothing excluded)
    pub fn empty() -> Self {
        Self {
            excluded_folders: HashSet::new(),
            excluded_domain_patterns: Vec::new(),
        }
    }

    /// Checks if a folder ID is excluded
    pub fn is_folder_excluded(&self, folder_id: &str) -> bool {
        self.excluded_folders.contains(folder_id)
    }

    /// Checks if a URL matches any exclusion pattern
    pub fn is_url_excluded(&self, url: &str) -> bool {
        for pattern in &self.excluded_domain_patterns {
            if Self::matches_domain_pattern(url, pattern) {
                return true;
            }
        }
        false
    }

    /// Validates a domain pattern format
    pub fn validate_pattern(pattern: &str) -> Result<()> {
        // Empty pattern
        if pattern.is_empty() {
            return Err("Pattern cannot be empty".into());
        }

        // Too long (DNS limit)
        if pattern.len() > 253 {
            return Err("Pattern exceeds maximum length (253 characters)".into());
        }

        // Contains protocol
        if pattern.starts_with("http://") || pattern.starts_with("https://") {
            return Err("Pattern cannot contain protocol (http:// or https://)".into());
        }

        // Contains path
        if pattern.contains('/') {
            return Err("Pattern cannot contain path segments (/)".into());
        }

        // Contains space
        if pattern.contains(' ') {
            return Err("Pattern cannot contain spaces".into());
        }

        // Starts with dot (except for wildcards like *.example.com)
        if pattern.starts_with('.') && !pattern.starts_with("*.") {
            return Err("Pattern cannot start with dot".into());
        }

        // Double wildcard
        if pattern.contains("**") {
            return Err("Pattern cannot contain double wildcard (**)".into());
        }

        // Valid characters: letters, numbers, dots, hyphens, asterisks, colons
        for c in pattern.chars() {
            if !c.is_alphanumeric() && c != '.' && c != '-' && c != '*' && c != ':' {
                return Err(format!("Pattern contains invalid character: '{}'", c).into());
            }
        }

        Ok(())
    }

    /// Matches a URL against a domain pattern
    fn matches_domain_pattern(url: &str, pattern: &str) -> bool {
        // Parse URL to extract host
        let host = match Url::parse(url) {
            Ok(parsed_url) => match parsed_url.host_str() {
                Some(h) => h.to_string(),
                None => return false,
            },
            Err(_) => {
                // If URL parsing fails, try treating the input as just a host
                url.to_string()
            }
        };

        // Get port if present
        let full_host = match Url::parse(url) {
            Ok(parsed_url) => match parsed_url.port() {
                Some(port) => format!("{}:{}", host, port),
                None => host.clone(),
            },
            Err(_) => host.clone(),
        };

        // Pattern matching logic
        if pattern.contains('*') {
            // Wildcard pattern matching
            Self::wildcard_match(&full_host, pattern) || Self::wildcard_match(&host, pattern)
        } else {
            // Exact match (with www. handling)
            host == pattern || host == format!("www.{}", pattern) || full_host == pattern
        }
    }

    /// Performs wildcard pattern matching
    fn wildcard_match(text: &str, pattern: &str) -> bool {
        // Split pattern by wildcards
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 1 {
            // No wildcards - exact match
            return text == pattern;
        }

        let mut text_pos = 0;

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 {
                // First part - must match at start
                if !text.starts_with(part) {
                    return false;
                }
                text_pos = part.len();
            } else if i == parts.len() - 1 {
                // Last part - must match at end
                if !text.ends_with(part) {
                    return false;
                }
            } else {
                // Middle part - must exist somewhere after current position
                match text[text_pos..].find(part) {
                    Some(pos) => {
                        text_pos += pos + part.len();
                    }
                    None => return false,
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pattern_empty() {
        let result = ExclusionRules::validate_pattern("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_pattern_too_long() {
        let long_pattern = "a".repeat(254);
        let result = ExclusionRules::validate_pattern(&long_pattern);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("maximum length"));
    }

    #[test]
    fn test_validate_pattern_with_protocol() {
        assert!(ExclusionRules::validate_pattern("http://example.com").is_err());
        assert!(ExclusionRules::validate_pattern("https://example.com").is_err());
    }

    #[test]
    fn test_validate_pattern_with_path() {
        let result = ExclusionRules::validate_pattern("example.com/path");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path"));
    }

    #[test]
    fn test_validate_pattern_with_space() {
        let result = ExclusionRules::validate_pattern("example com");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("space"));
    }

    #[test]
    fn test_validate_pattern_starts_with_dot() {
        let result = ExclusionRules::validate_pattern(".example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("dot"));
    }

    #[test]
    fn test_validate_pattern_double_wildcard() {
        let result = ExclusionRules::validate_pattern("**example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("double wildcard"));
    }

    #[test]
    fn test_validate_pattern_valid_exact() {
        assert!(ExclusionRules::validate_pattern("example.com").is_ok());
    }

    #[test]
    fn test_validate_pattern_valid_wildcard_subdomain() {
        assert!(ExclusionRules::validate_pattern("*.example.com").is_ok());
    }

    #[test]
    fn test_validate_pattern_valid_wildcard_prefix() {
        assert!(ExclusionRules::validate_pattern("example.*").is_ok());
    }

    #[test]
    fn test_validate_pattern_valid_wildcard_port() {
        assert!(ExclusionRules::validate_pattern("localhost:*").is_ok());
        assert!(ExclusionRules::validate_pattern("*:8080").is_ok());
    }

    #[test]
    fn test_validate_pattern_valid_ip() {
        assert!(ExclusionRules::validate_pattern("192.168.1.1").is_ok());
        assert!(ExclusionRules::validate_pattern("192.168.*.*").is_ok());
    }

    #[test]
    fn test_matches_exact_domain() {
        assert!(ExclusionRules::matches_domain_pattern(
            "https://example.com/path",
            "example.com"
        ));
    }

    #[test]
    fn test_matches_www_domain() {
        assert!(ExclusionRules::matches_domain_pattern(
            "https://www.example.com/path",
            "example.com"
        ));
    }

    #[test]
    fn test_matches_wildcard_subdomain() {
        assert!(ExclusionRules::matches_domain_pattern(
            "https://foo.example.com/path",
            "*.example.com"
        ));
        assert!(ExclusionRules::matches_domain_pattern(
            "https://bar.example.com/path",
            "*.example.com"
        ));
        assert!(!ExclusionRules::matches_domain_pattern(
            "https://example.com/path",
            "*.example.com"
        ));
    }

    #[test]
    fn test_matches_wildcard_suffix() {
        assert!(ExclusionRules::matches_domain_pattern(
            "https://example.com/path",
            "*example.com"
        ));
        assert!(ExclusionRules::matches_domain_pattern(
            "https://myexample.com/path",
            "*example.com"
        ));
    }

    #[test]
    fn test_matches_wildcard_prefix() {
        assert!(ExclusionRules::matches_domain_pattern(
            "https://example.com/path",
            "example.*"
        ));
        assert!(ExclusionRules::matches_domain_pattern(
            "https://example.org/path",
            "example.*"
        ));
    }

    #[test]
    fn test_matches_localhost_wildcard_port() {
        assert!(ExclusionRules::matches_domain_pattern(
            "http://localhost:3000/path",
            "localhost:*"
        ));
        assert!(ExclusionRules::matches_domain_pattern(
            "http://localhost:8080/path",
            "localhost:*"
        ));
        assert!(!ExclusionRules::matches_domain_pattern(
            "http://localhost/path",
            "localhost:*"
        ));
    }

    #[test]
    fn test_matches_wildcard_port() {
        assert!(ExclusionRules::matches_domain_pattern(
            "http://example.com:8080/path",
            "*:8080"
        ));
        assert!(ExclusionRules::matches_domain_pattern(
            "http://localhost:8080/path",
            "*:8080"
        ));
        assert!(!ExclusionRules::matches_domain_pattern(
            "http://example.com:3000/path",
            "*:8080"
        ));
    }

    #[test]
    fn test_matches_ip_patterns() {
        assert!(ExclusionRules::matches_domain_pattern(
            "http://192.168.1.1/path",
            "192.168.1.1"
        ));
        assert!(ExclusionRules::matches_domain_pattern(
            "http://192.168.1.1/path",
            "192.168.*.*"
        ));
        assert!(ExclusionRules::matches_domain_pattern(
            "http://192.168.5.10/path",
            "192.168.*.*"
        ));
    }

    #[test]
    fn test_is_folder_excluded() {
        let rules = ExclusionRules::new(vec!["123".to_string(), "456".to_string()], vec![]);
        assert!(rules.is_folder_excluded("123"));
        assert!(rules.is_folder_excluded("456"));
        assert!(!rules.is_folder_excluded("789"));
    }

    #[test]
    fn test_is_url_excluded() {
        let rules = ExclusionRules::new(
            vec![],
            vec![
                "*.internal.com".to_string(),
                "private.example.org".to_string(),
            ],
        );
        assert!(rules.is_url_excluded("https://foo.internal.com/page"));
        assert!(rules.is_url_excluded("https://private.example.org/page"));
        assert!(!rules.is_url_excluded("https://example.com/page"));
    }

    #[test]
    fn test_empty_rules() {
        let rules = ExclusionRules::empty();
        assert!(!rules.is_folder_excluded("123"));
        assert!(!rules.is_url_excluded("https://example.com"));
    }
}
