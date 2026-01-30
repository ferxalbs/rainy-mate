# Rust Expert - Complete Examples

## Example 1: Production-Ready HTTP Client

```rust
// src/lib.rs
pub mod client;
pub mod error;
pub mod config;

pub use client::HttpClient;
pub use error::{Error, Result};
pub use config::Config;

// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("Timeout after {0}s")]
    Timeout(u64),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
}

// src/config.rs
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    
    #[serde(default)]
    pub user_agent: Option<String>,
}

fn default_timeout() -> u64 { 30 }
fn default_max_retries() -> usize { 3 }

impl Config {
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
    
    pub fn validate(&self) -> crate::Result<()> {
        if self.timeout_secs == 0 {
            return Err(crate::Error::Config(
                "Timeout must be greater than 0".to_string()
            ));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            user_agent: None,
        }
    }
}

// src/client.rs
use crate::{Config, Error, Result};
use reqwest::Client;
use std::sync::Arc;

pub struct HttpClient {
    client: Client,
    config: Arc<Config>,
}

impl HttpClient {
    pub fn new(config: Config) -> Result<Self> {
        config.validate()?;
        
        let mut builder = Client::builder()
            .timeout(config.timeout());
        
        if let Some(ref ua) = config.user_agent {
            builder = builder.user_agent(ua);
        }
        
        let client = builder
            .build()
            .map_err(|e| Error::Config(e.to_string()))?;
        
        Ok(Self {
            client,
            config: Arc::new(config),
        })
    }
    
    pub async fn get(&self, url: &str) -> Result<String> {
        self.validate_url(url)?;
        
        let mut attempts = 0;
        let max_attempts = self.config.max_retries + 1;
        
        loop {
            attempts += 1;
            
            match self.client.get(url).send().await {
                Ok(response) => {
                    let text = response.text().await?;
                    return Ok(text);
                }
                Err(e) if attempts < max_attempts && e.is_timeout() => {
                    tracing::warn!(
                        "Request timeout (attempt {}/{}): {}",
                        attempts,
                        max_attempts,
                        url
                    );
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    
    fn validate_url(&self, url: &str) -> Result<()> {
        url.parse::<reqwest::Url>()
            .map_err(|_| Error::InvalidUrl(url.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_validation() {
        let config = Config {
            timeout_secs: 0,
            max_retries: 3,
            user_agent: None,
        };
        
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_url_validation() {
        let config = Config::default();
        let client = HttpClient::new(config).unwrap();
        
        assert!(client.validate_url("invalid url").is_err());
        assert!(client.validate_url("https://example.com").is_ok());
    }
}
```

## Example 2: Async File Processor with Error Recovery

```rust
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct FileProcessor {
    input_dir: PathBuf,
    output_dir: PathBuf,
}

impl FileProcessor {
    pub fn new(input_dir: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> Self {
        Self {
            input_dir: input_dir.as_ref().to_path_buf(),
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }
    
    pub async fn process_all(&self) -> Result<Vec<PathBuf>> {
        fs::create_dir_all(&self.output_dir)
            .await
            .context("Failed to create output directory")?;
        
        let mut entries = fs::read_dir(&self.input_dir)
            .await
            .context("Failed to read input directory")?;
        
        let mut processed = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if !path.is_file() {
                continue;
            }
            
            match self.process_file(&path).await {
                Ok(output_path) => {
                    tracing::info!("Processed: {}", path.display());
                    processed.push(output_path);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to process {}: {:?}",
                        path.display(),
                        e
                    );
                    // Continue processing other files
                }
            }
        }
        
        Ok(processed)
    }
    
    async fn process_file(&self, input_path: &Path) -> Result<PathBuf> {
        let content = fs::read_to_string(input_path)
            .await
            .with_context(|| format!("Failed to read {}", input_path.display()))?;
        
        let processed = self.transform_content(&content)?;
        
        let file_name = input_path
            .file_name()
            .context("Invalid file name")?;
        let output_path = self.output_dir.join(file_name);
        
        let mut file = fs::File::create(&output_path)
            .await
            .with_context(|| format!("Failed to create {}", output_path.display()))?;
        
        file.write_all(processed.as_bytes())
            .await
            .context("Failed to write output")?;
        
        file.flush()
            .await
            .context("Failed to flush output")?;
        
        Ok(output_path)
    }
    
    fn transform_content(&self, content: &str) -> Result<String> {
        // Example transformation: uppercase
        Ok(content.to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_process_all() -> Result<()> {
        let temp_input = TempDir::new()?;
        let temp_output = TempDir::new()?;
        
        // Create test file
        let test_file = temp_input.path().join("test.txt");
        fs::write(&test_file, "hello world").await?;
        
        let processor = FileProcessor::new(
            temp_input.path(),
            temp_output.path(),
        );
        
        let processed = processor.process_all().await?;
        
        assert_eq!(processed.len(), 1);
        
        let output_content = fs::read_to_string(&processed[0]).await?;
        assert_eq!(output_content, "HELLO WORLD");
        
        Ok(())
    }
}
```

## Example 3: Thread-Safe Cache with Generics

```rust
use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Cache<K, V> {
    store: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    ttl: Duration,
}

struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }
    
    pub fn insert(&self, key: K, value: V) {
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };
        
        self.store.write().insert(key, entry);
    }
    
    pub fn get(&self, key: &K) -> Option<V> {
        let store = self.store.read();
        
        store.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }
    
    pub fn remove(&self, key: &K) -> Option<V> {
        self.store.write().remove(key).map(|entry| entry.value)
    }
    
    pub fn clear_expired(&self) {
        let now = Instant::now();
        self.store.write().retain(|_, entry| entry.expires_at > now);
    }
    
    pub fn len(&self) -> usize {
        self.store.read().len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.store.read().is_empty()
    }
}

impl<K, V> Clone for Cache<K, V> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            ttl: self.ttl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_cache_basic() {
        let cache = Cache::new(Duration::from_secs(1));
        
        cache.insert("key1", "value1");
        assert_eq!(cache.get(&"key1"), Some("value1"));
        
        cache.remove(&"key1");
        assert_eq!(cache.get(&"key1"), None);
    }
    
    #[test]
    fn test_cache_expiration() {
        let cache = Cache::new(Duration::from_millis(100));
        
        cache.insert("key", "value");
        assert_eq!(cache.get(&"key"), Some("value"));
        
        thread::sleep(Duration::from_millis(150));
        assert_eq!(cache.get(&"key"), None);
    }
    
    #[test]
    fn test_cache_thread_safe() {
        let cache = Cache::new(Duration::from_secs(10));
        
        let mut handles = vec![];
        
        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = thread::spawn(move || {
                cache_clone.insert(i, i * 2);
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        assert_eq!(cache.len(), 10);
    }
}
```

## Example 4: Builder Pattern with Validation

```rust
use std::net::IpAddr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
    
    #[error("Invalid port: {0}")]
    InvalidPort(u16),
    
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    host: IpAddr,
    port: u16,
    workers: usize,
    max_connections: usize,
    timeout_secs: u64,
}

#[derive(Default)]
pub struct ServerConfigBuilder {
    host: Option<IpAddr>,
    port: Option<u16>,
    workers: Option<usize>,
    max_connections: Option<usize>,
    timeout_secs: Option<u64>,
}

impl ServerConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn host(mut self, host: IpAddr) -> Self {
        self.host = Some(host);
        self
    }
    
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }
    
    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = Some(workers);
        self
    }
    
    pub fn max_connections(mut self, max: usize) -> Self {
        self.max_connections = Some(max);
        self
    }
    
    pub fn timeout_secs(mut self, timeout: u64) -> Self {
        self.timeout_secs = Some(timeout);
        self
    }
    
    pub fn build(self) -> Result<ServerConfig, BuildError> {
        let host = self.host
            .ok_or(BuildError::MissingField("host"))?;
        
        let port = self.port
            .ok_or(BuildError::MissingField("port"))?;
        
        if port < 1024 {
            return Err(BuildError::InvalidPort(port));
        }
        
        let workers = self.workers.unwrap_or_else(num_cpus::get);
        let max_connections = self.max_connections.unwrap_or(1000);
        let timeout_secs = self.timeout_secs.unwrap_or(30);
        
        if workers == 0 {
            return Err(BuildError::Invalid(
                "Workers must be greater than 0".to_string()
            ));
        }
        
        Ok(ServerConfig {
            host,
            port,
            workers,
            max_connections,
            timeout_secs,
        })
    }
}

impl ServerConfig {
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_builder_success() {
        let config = ServerConfig::builder()
            .host("127.0.0.1".parse().unwrap())
            .port(8080)
            .workers(4)
            .build()
            .unwrap();
        
        assert_eq!(config.port, 8080);
        assert_eq!(config.workers, 4);
    }
    
    #[test]
    fn test_builder_missing_field() {
        let result = ServerConfig::builder()
            .port(8080)
            .build();
        
        assert!(matches!(result, Err(BuildError::MissingField("host"))));
    }
    
    #[test]
    fn test_builder_invalid_port() {
        let result = ServerConfig::builder()
            .host("127.0.0.1".parse().unwrap())
            .port(80)
            .build();
        
        assert!(matches!(result, Err(BuildError::InvalidPort(80))));
    }
}
```

## Example 5: Zero-Copy String Processing

```rust
use std::borrow::Cow;

pub struct TextProcessor;

impl TextProcessor {
    /// Process text, only allocating if modification is needed
    pub fn normalize<'a>(text: &'a str) -> Cow<'a, str> {
        if text.chars().all(|c| !c.is_whitespace() || c == ' ') {
            // No modification needed
            Cow::Borrowed(text)
        } else {
            // Need to modify
            let normalized = text
                .chars()
                .map(|c| if c.is_whitespace() { ' ' } else { c })
                .collect();
            Cow::Owned(normalized)
        }
    }
    
    /// Remove prefix efficiently
    pub fn remove_prefix<'a>(text: &'a str, prefix: &str) -> &'a str {
        text.strip_prefix(prefix).unwrap_or(text)
    }
    
    /// Split and process without allocation when possible
    pub fn process_lines(text: &str) -> Vec<&str> {
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalize_borrowed() {
        let text = "hello world";
        let result = TextProcessor::normalize(text);
        
        // Should borrow, not allocate
        assert!(matches!(result, Cow::Borrowed(_)));
    }
    
    #[test]
    fn test_normalize_owned() {
        let text = "hello\tworld\n";
        let result = TextProcessor::normalize(text);
        
        // Should allocate
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "hello world ");
    }
}
```

## Example 6: Custom Iterator

```rust
pub struct FibonacciIterator {
    current: u64,
    next: u64,
}

impl FibonacciIterator {
    pub fn new() -> Self {
        Self {
            current: 0,
            next: 1,
        }
    }
}

impl Default for FibonacciIterator {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for FibonacciIterator {
    type Item = u64;
    
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.current;
        
        // Check for overflow
        let (next, overflow) = self.current.overflowing_add(self.next);
        if overflow {
            return None;
        }
        
        self.current = self.next;
        self.next = next;
        
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fibonacci() {
        let fibs: Vec<_> = FibonacciIterator::new().take(10).collect();
        assert_eq!(fibs, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
    }
    
    #[test]
    fn test_fibonacci_stops_on_overflow() {
        let count = FibonacciIterator::new().count();
        assert!(count < 100); // Should stop before 100 iterations
    }
}
```