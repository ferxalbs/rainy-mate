// Web Research Service
// Handles URL content extraction and HTML-to-Markdown conversion
// Part of Rainy Cowork Phase 3: Content Extraction

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during web research operations
#[derive(Error, Debug)]
pub enum WebResearchError {
    #[error("Failed to fetch URL: {0}")]
    FetchError(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Content extraction failed: {0}")]
    ExtractionError(String),
    /// Rate limited response - for future use when implementing rate limiting
    #[allow(dead_code)]
    #[error("Rate limited: try again in {0} seconds")]
    RateLimited(u64),
}

/// Extracted web content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebContent {
    /// Original URL
    pub url: String,
    /// Page title
    pub title: String,
    /// Content converted to Markdown
    pub content_markdown: String,
    /// Meta description if available
    pub description: Option<String>,
    /// Extraction timestamp
    pub extracted_at: DateTime<Utc>,
    /// Content byte size
    pub size_bytes: usize,
}

/// Cached content with expiration
#[derive(Debug, Clone)]
struct CachedContent {
    content: WebContent,
    expires_at: DateTime<Utc>,
}

/// Web research service with caching and rate limiting
#[derive(Clone)]
pub struct WebResearchService {
    client: Client,
    cache: Arc<DashMap<String, CachedContent>>,
    cache_ttl_seconds: i64,
}

impl Default for WebResearchService {
    fn default() -> Self {
        Self::new()
    }
}

impl WebResearchService {
    /// Create a new web research service
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("RainyCowork/0.3.0 (https://github.com/enosislabs/rainy-cowork)")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            cache: Arc::new(DashMap::new()),
            cache_ttl_seconds: 300, // 5 minutes default
        }
    }

    /// Fetch and extract content from a URL
    pub async fn fetch_url(&self, url: &str) -> Result<WebContent, WebResearchError> {
        // Check cache first
        if let Some(cached) = self.get_cached(url) {
            return Ok(cached);
        }

        // Validate URL
        let parsed_url =
            url::Url::parse(url).map_err(|e| WebResearchError::InvalidUrl(e.to_string()))?;

        // Only allow http/https
        if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
            return Err(WebResearchError::InvalidUrl(
                "Only HTTP/HTTPS URLs are supported".to_string(),
            ));
        }

        // Fetch HTML
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| WebResearchError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(WebResearchError::FetchError(format!(
                "HTTP {} - {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let html = response
            .text()
            .await
            .map_err(|e| WebResearchError::ExtractionError(e.to_string()))?;

        // Extract content
        let content = self.extract_content(url, &html)?;

        // Cache result
        self.cache_content(url, &content);

        Ok(content)
    }

    /// Extract structured content from HTML
    fn extract_content(&self, url: &str, html: &str) -> Result<WebContent, WebResearchError> {
        use scraper::{Html, Selector};

        let document = Html::parse_document(html);

        // Extract title
        let title_selector = Selector::parse("title").unwrap();
        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_else(|| "Untitled".to_string())
            .trim()
            .to_string();

        // Extract meta description
        let meta_selector = Selector::parse("meta[name=\"description\"]").unwrap();
        let description = document
            .select(&meta_selector)
            .next()
            .and_then(|el| el.value().attr("content"))
            .map(|s| s.to_string());

        // Convert main content to markdown
        let content_markdown = self.html_to_markdown(&document);
        let size_bytes = content_markdown.len();

        Ok(WebContent {
            url: url.to_string(),
            title,
            content_markdown,
            description,
            extracted_at: Utc::now(),
            size_bytes,
        })
    }

    /// Convert HTML document to Markdown
    fn html_to_markdown(&self, document: &scraper::Html) -> String {
        use scraper::Selector;

        // Try to find main content areas
        let content_selectors = [
            "article",
            "main",
            "[role=\"main\"]",
            ".content",
            ".post-content",
            ".article-content",
            "#content",
            "body",
        ];

        let mut markdown = String::new();

        for selector_str in content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    markdown = self.element_to_markdown(&element);
                    if !markdown.trim().is_empty() && markdown.len() > 100 {
                        break;
                    }
                }
            }
        }

        // Clean up excessive whitespace
        self.clean_markdown(&markdown)
    }

    /// Convert HTML element to markdown recursively
    fn element_to_markdown(&self, element: &scraper::ElementRef) -> String {
        let mut result = String::new();

        for node in element.children() {
            match node.value() {
                scraper::Node::Text(text) => {
                    let text_content = text.text.trim();
                    if !text_content.is_empty() {
                        result.push_str(text_content);
                        result.push(' ');
                    }
                }
                scraper::Node::Element(el) => {
                    let child_ref = scraper::ElementRef::wrap(node);
                    if let Some(child) = child_ref {
                        let tag = el.name();
                        match tag {
                            // Skip unwanted elements
                            "script" | "style" | "nav" | "footer" | "header" | "aside" | "form" => {
                            }

                            // Headers
                            "h1" => {
                                result.push_str("\n\n# ");
                                result.push_str(&self.element_to_markdown(&child));
                                result.push_str("\n\n");
                            }
                            "h2" => {
                                result.push_str("\n\n## ");
                                result.push_str(&self.element_to_markdown(&child));
                                result.push_str("\n\n");
                            }
                            "h3" => {
                                result.push_str("\n\n### ");
                                result.push_str(&self.element_to_markdown(&child));
                                result.push_str("\n\n");
                            }
                            "h4" | "h5" | "h6" => {
                                result.push_str("\n\n#### ");
                                result.push_str(&self.element_to_markdown(&child));
                                result.push_str("\n\n");
                            }

                            // Paragraphs and blocks
                            "p" | "div" => {
                                let content = self.element_to_markdown(&child);
                                if !content.trim().is_empty() {
                                    result.push_str("\n\n");
                                    result.push_str(&content);
                                    result.push_str("\n\n");
                                }
                            }

                            // Lists
                            "ul" | "ol" => {
                                result.push_str("\n\n");
                                result.push_str(&self.element_to_markdown(&child));
                                result.push_str("\n\n");
                            }
                            "li" => {
                                result.push_str("- ");
                                result.push_str(&self.element_to_markdown(&child));
                                result.push('\n');
                            }

                            // Inline formatting
                            "strong" | "b" => {
                                result.push_str("**");
                                result.push_str(&self.element_to_markdown(&child));
                                result.push_str("**");
                            }
                            "em" | "i" => {
                                result.push('_');
                                result.push_str(&self.element_to_markdown(&child));
                                result.push('_');
                            }
                            "code" => {
                                result.push('`');
                                result.push_str(&self.element_to_markdown(&child));
                                result.push('`');
                            }

                            // Links
                            "a" => {
                                if let Some(href) = el.attr("href") {
                                    let text = self.element_to_markdown(&child);
                                    if !text.trim().is_empty() {
                                        result.push('[');
                                        result.push_str(&text);
                                        result.push_str("](");
                                        result.push_str(href);
                                        result.push(')');
                                    }
                                } else {
                                    result.push_str(&self.element_to_markdown(&child));
                                }
                            }

                            // Images
                            "img" => {
                                if let Some(src) = el.attr("src") {
                                    let alt = el.attr("alt").unwrap_or("image");
                                    result.push_str("![");
                                    result.push_str(alt);
                                    result.push_str("](");
                                    result.push_str(src);
                                    result.push(')');
                                }
                            }

                            // Code blocks
                            "pre" => {
                                result.push_str("\n\n```\n");
                                result.push_str(&self.element_to_markdown(&child));
                                result.push_str("\n```\n\n");
                            }

                            // Blockquotes
                            "blockquote" => {
                                result.push_str("\n\n> ");
                                let content = self.element_to_markdown(&child);
                                result.push_str(&content.replace('\n', "\n> "));
                                result.push_str("\n\n");
                            }

                            // Line breaks
                            "br" => {
                                result.push_str("\n");
                            }

                            // Horizontal rules
                            "hr" => {
                                result.push_str("\n\n---\n\n");
                            }

                            // Default: process children
                            _ => {
                                result.push_str(&self.element_to_markdown(&child));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        result
    }

    /// Clean up markdown output
    fn clean_markdown(&self, markdown: &str) -> String {
        // Remove excessive newlines (more than 2 consecutive)
        let re = regex::Regex::new(r"\n{3,}").unwrap();
        let cleaned = re.replace_all(markdown, "\n\n");

        // Trim and return
        cleaned.trim().to_string()
    }

    /// Check cache for content
    fn get_cached(&self, url: &str) -> Option<WebContent> {
        if let Some(cached) = self.cache.get(url) {
            if cached.expires_at > Utc::now() {
                return Some(cached.content.clone());
            }
            // Remove expired entry
            drop(cached);
            self.cache.remove(url);
        }
        None
    }

    /// Cache content
    fn cache_content(&self, url: &str, content: &WebContent) {
        let cached = CachedContent {
            content: content.clone(),
            expires_at: Utc::now() + chrono::Duration::seconds(self.cache_ttl_seconds),
        };
        self.cache.insert(url.to_string(), cached);
    }

    /// Clear all cached content
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let total = self.cache.len();
        let valid = self
            .cache
            .iter()
            .filter(|e| e.expires_at > Utc::now())
            .count();
        (total, valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_url() {
        let service = WebResearchService::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(service.fetch_url("not-a-valid-url"));
        assert!(matches!(result, Err(WebResearchError::InvalidUrl(_))));
    }

    #[test]
    fn test_non_http_url() {
        let service = WebResearchService::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(service.fetch_url("ftp://example.com"));
        assert!(matches!(result, Err(WebResearchError::InvalidUrl(_))));
    }
}
