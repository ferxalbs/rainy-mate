// src-tauri/src/services/browser_controller.rs
//
// Native browser automation using chromiumoxide (CDP protocol).
// Replaces the npx-based MCP client for production-ready browser control.

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Result of a browser navigation operation
#[derive(Debug, Clone)]
pub struct NavigationResult {
    pub url: String,
    pub title: String,
    pub content_preview: String,
}

/// Result of a screenshot operation  
#[derive(Debug, Clone)]
pub struct ScreenshotResult {
    pub data_uri: String,
    pub width: u32,
    pub height: u32,
}

/// Native browser controller using Chrome DevTools Protocol
pub struct BrowserController {
    browser: Arc<Mutex<Option<Arc<Browser>>>>,
    handler_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl BrowserController {
    pub fn new() -> Self {
        Self {
            browser: Arc::new(Mutex::new(None)),
            handler_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Launch the browser if not already running
    async fn ensure_browser(&self) -> Result<Arc<Browser>, String> {
        let mut browser_lock = self.browser.lock().await;

        if let Some(browser) = browser_lock.as_ref() {
            // Browser already running, return Arc clone
            return Ok(browser.clone());
        }

        // Launch new browser instance
        println!("[BrowserController] Launching Chrome...");

        let config = BrowserConfig::builder()
            .with_head() // Visible browser for debugging (change to headless for prod)
            .build()
            .map_err(|e| format!("Failed to build browser config: {}", e))?;

        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| format!("Failed to launch browser: {}", e))?;

        // Spawn handler task to process CDP events
        let handle = tokio::spawn(async move {
            loop {
                if handler.next().await.is_none() {
                    break;
                }
            }
        });

        // Store references (wrap Browser in Arc)
        let browser_arc = Arc::new(browser);
        *browser_lock = Some(browser_arc.clone());
        *self.handler_handle.lock().await = Some(handle);

        println!("[BrowserController] Chrome launched successfully");
        Ok(browser_arc)
    }

    /// Navigate to a URL and return page information
    pub async fn navigate(&self, url: &str) -> Result<NavigationResult, String> {
        let browser = self.ensure_browser().await?;

        println!("[BrowserController] Navigating to: {}", url);

        let page = browser
            .new_page(url)
            .await
            .map_err(|e| format!("Failed to create page: {}", e))?;

        // Wait for page to load
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Get page info
        let current_url = page
            .url()
            .await
            .map_err(|e| format!("Failed to get URL: {}", e))?
            .unwrap_or_else(|| url.to_string());

        let title = page
            .evaluate("document.title")
            .await
            .map_err(|e| format!("Failed to get title: {}", e))?
            .into_value::<String>()
            .unwrap_or_else(|_| "Untitled".to_string());

        // Get visible text content (first 2000 chars)
        let content = page
            .evaluate("document.body.innerText.substring(0, 2000)")
            .await
            .map_err(|e| format!("Failed to get content: {}", e))?
            .into_value::<String>()
            .unwrap_or_default();

        println!("[BrowserController] Navigation complete: {}", title);

        Ok(NavigationResult {
            url: current_url,
            title,
            content_preview: content,
        })
    }

    /// Take a screenshot and return as base64 data URI
    pub async fn screenshot(&self) -> Result<ScreenshotResult, String> {
        let browser_lock = self.browser.lock().await;
        let browser = browser_lock
            .as_ref()
            .ok_or("No browser instance. Navigate to a page first.")?;

        // Get the active page (most recently created)
        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Failed to get pages: {}", e))?;

        let page = pages.last().ok_or("No pages open")?;

        println!("[BrowserController] Taking screenshot...");

        let screenshot_bytes = page
            .screenshot(
                ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .full_page(false)
                    .build(),
            )
            .await
            .map_err(|e| format!("Failed to take screenshot: {}", e))?;

        // Convert to base64 data URI
        use base64::prelude::*;
        let base64_data = BASE64_STANDARD.encode(&screenshot_bytes);
        let data_uri = format!("data:image/png;base64,{}", base64_data);

        println!(
            "[BrowserController] Screenshot captured ({} bytes)",
            screenshot_bytes.len()
        );

        Ok(ScreenshotResult {
            data_uri,
            width: 1280, // Default viewport
            height: 720,
        })
    }

    /// Get the full text content of the current page
    pub async fn get_content(&self) -> Result<String, String> {
        let browser_lock = self.browser.lock().await;
        let browser = browser_lock
            .as_ref()
            .ok_or("No browser instance. Navigate to a page first.")?;

        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Failed to get pages: {}", e))?;

        let page = pages.last().ok_or("No pages open")?;

        let content = page
            .content()
            .await
            .map_err(|e| format!("Failed to get content: {}", e))?;

        Ok(content)
    }

    /// Click on an element by CSS selector
    pub async fn click(&self, selector: &str) -> Result<(), String> {
        let browser_lock = self.browser.lock().await;
        let browser = browser_lock
            .as_ref()
            .ok_or("No browser instance. Navigate to a page first.")?;

        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Failed to get pages: {}", e))?;

        let page = pages.last().ok_or("No pages open")?;

        println!("[BrowserController] Clicking: {}", selector);

        let element = page
            .find_element(selector)
            .await
            .map_err(|e| format!("Element not found '{}': {}", selector, e))?;

        element
            .click()
            .await
            .map_err(|e| format!("Click failed: {}", e))?;

        // Wait for any navigation
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Ok(())
    }

    /// Execute JavaScript and return result
    pub async fn evaluate(&self, script: &str) -> Result<String, String> {
        let browser_lock = self.browser.lock().await;
        let browser = browser_lock
            .as_ref()
            .ok_or("No browser instance. Navigate to a page first.")?;

        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Failed to get pages: {}", e))?;

        let page = pages.last().ok_or("No pages open")?;

        let result = page
            .evaluate(script)
            .await
            .map_err(|e| format!("Script evaluation failed: {}", e))?;

        result
            .into_value::<String>()
            .map_err(|e| format!("Failed to parse result: {}", e))
    }

    /// Close the browser gracefully
    #[allow(dead_code)] // @RESERVED - will be used for cleanup
    pub async fn close(&self) {
        let mut browser_lock = self.browser.lock().await;
        // Just drop the Arc reference - browser will close when all refs are dropped
        *browser_lock = None;

        let mut handle_lock = self.handler_handle.lock().await;
        if let Some(handle) = handle_lock.take() {
            handle.abort();
        }

        println!("[BrowserController] Browser closed");
    }
}

impl Default for BrowserController {
    fn default() -> Self {
        Self::new()
    }
}
