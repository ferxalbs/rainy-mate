use super::args::*;
use super::{truncate_output, SkillExecutor};
use crate::models::neural::CommandResult;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

impl SkillExecutor {
    pub(super) async fn execute_web(
        &self,
        method: &str,
        params: &Option<Value>,
        allowed_domains: &[String],
        blocked_domains: &[String],
    ) -> CommandResult {
        let params = match params {
            Some(p) => p,
            None => return self.error("Missing parameters"),
        };

        match method {
            "web_search" => {
                let args: WebSearchArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_web_search(&args.query).await
            }
            "read_web_page" => {
                let args: ReadWebPageArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_read_web_page(&args.url, allowed_domains, blocked_domains)
                    .await
            }
            "http_get_json" => {
                let args: HttpGetJsonArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_http_get_json(args, allowed_domains, blocked_domains)
                    .await
            }
            "http_get_text" => {
                let args: HttpGetTextArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_http_get_text(args, allowed_domains, blocked_domains)
                    .await
            }
            "http_post_json" => {
                let args: HttpPostJsonArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_http_post_json(args, allowed_domains, blocked_domains)
                    .await
            }
            _ => CommandResult {
                success: false,
                output: None,
                error: Some(format!("Unknown web method: {}", method)),
                exit_code: Some(1),
            },
        }
    }

    async fn handle_web_search(&self, query: &str) -> CommandResult {
        match self
            .managed_research
            .perform_research(query.to_string(), None)
            .await
        {
            Ok(result) => CommandResult {
                success: true,
                output: Some(format!("Research Result for '{}':\n{}", query, result.content)),
                error: None,
                exit_code: Some(0),
            },
            Err(e) => self.error(&format!("Web search failed: {}", e)),
        }
    }

    async fn handle_read_web_page(
        &self,
        url: &str,
        allowed_domains: &[String],
        blocked_domains: &[String],
    ) -> CommandResult {
        if let Err(e) = Self::enforce_domain_scope(url, allowed_domains, blocked_domains) {
            return self.error(&e);
        }
        match self.browser.navigate(url).await {
            Ok(nav_result) => match self.browser.get_content().await {
                Ok(content) => CommandResult {
                    success: true,
                    output: Some(truncate_output(&content)),
                    error: None,
                    exit_code: Some(0),
                },
                Err(_) => CommandResult {
                    success: true,
                    output: Some(truncate_output(&nav_result.content_preview)),
                    error: None,
                    exit_code: Some(0),
                },
            },
            Err(e) => self.error(&format!("Failed to read web page: {}", e)),
        }
    }

    async fn handle_http_get_json(
        &self,
        args: HttpGetJsonArgs,
        allowed_domains: &[String],
        blocked_domains: &[String],
    ) -> CommandResult {
        self.handle_http_json_request(
            "GET",
            args.url,
            None,
            args.timeout_ms,
            args.max_bytes,
            allowed_domains,
            blocked_domains,
        )
        .await
    }

    async fn handle_http_post_json(
        &self,
        args: HttpPostJsonArgs,
        allowed_domains: &[String],
        blocked_domains: &[String],
    ) -> CommandResult {
        self.handle_http_json_request(
            "POST",
            args.url,
            Some(args.body),
            args.timeout_ms,
            args.max_bytes,
            allowed_domains,
            blocked_domains,
        )
        .await
    }

    async fn handle_http_get_text(
        &self,
        args: HttpGetTextArgs,
        allowed_domains: &[String],
        blocked_domains: &[String],
    ) -> CommandResult {
        if let Err(e) = Self::enforce_domain_scope(&args.url, allowed_domains, blocked_domains) {
            return self.error(&e);
        }
        let parsed_url = match Self::validate_http_url(&args.url) {
            Ok(u) => u,
            Err(e) => return self.error(&e),
        };

        let timeout_ms = args.timeout_ms.unwrap_or(15_000).clamp(1_000, 60_000);
        let max_bytes = args
            .max_bytes
            .unwrap_or(512 * 1024)
            .clamp(1_024, 2 * 1024 * 1024);

        let client = match reqwest::Client::builder()
            .timeout(tokio::time::Duration::from_millis(timeout_ms))
            .user_agent("rainy-cowork-agent/1.0")
            .build()
        {
            Ok(c) => c,
            Err(e) => return self.error(&format!("Failed to create HTTP client: {}", e)),
        };

        let mut last_error: Option<String> = None;
        for attempt in 0..=2 {
            match client
                .get(parsed_url.clone())
                .header("accept", "text/*,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if (status.is_server_error() || status.as_u16() == 429) && attempt < 2 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            250 * (attempt as u64 + 1),
                        ))
                        .await;
                        continue;
                    }
                    if !status.is_success() {
                        return self.error(&format!(
                            "HTTP request failed with status {} for {}",
                            status, parsed_url
                        ));
                    }

                    let content_type = resp
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();

                    let bytes = match resp.bytes().await {
                        Ok(b) => b,
                        Err(e) => return self.error(&format!("Failed to read response body: {}", e)),
                    };
                    if bytes.len() > max_bytes {
                        return self.error(&format!(
                            "Response size {} exceeds max_bytes {}",
                            bytes.len(),
                            max_bytes
                        ));
                    }

                    let text = String::from_utf8_lossy(&bytes).to_string();
                    let output = serde_json::json!({
                        "method": "GET",
                        "url": parsed_url.as_str(),
                        "status": status.as_u16(),
                        "content_type": content_type,
                        "bytes": bytes.len(),
                        "text": truncate_output(&text),
                    });

                    return CommandResult {
                        success: true,
                        output: Some(output.to_string()),
                        error: None,
                        exit_code: Some(0),
                    };
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    if attempt < 2 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            250 * (attempt as u64 + 1),
                        ))
                        .await;
                    }
                }
            }
        }

        self.error(&format!(
            "HTTP text request failed after retries: {}",
            last_error.unwrap_or_else(|| "unknown error".to_string())
        ))
    }

    async fn handle_http_json_request(
        &self,
        method: &str,
        url: String,
        body: Option<Value>,
        timeout_ms_opt: Option<u64>,
        max_bytes_opt: Option<usize>,
        allowed_domains: &[String],
        blocked_domains: &[String],
    ) -> CommandResult {
        if let Err(e) = Self::enforce_domain_scope(&url, allowed_domains, blocked_domains) {
            return self.error(&e);
        }
        let parsed_url = match Self::validate_http_url(&url) {
            Ok(u) => u,
            Err(e) => return self.error(&e),
        };

        let timeout_ms = timeout_ms_opt.unwrap_or(15_000).clamp(1_000, 60_000);
        let max_bytes = max_bytes_opt
            .unwrap_or(512 * 1024)
            .clamp(1_024, 2 * 1024 * 1024);

        let client = match reqwest::Client::builder()
            .timeout(tokio::time::Duration::from_millis(timeout_ms))
            .user_agent("rainy-cowork-agent/1.0")
            .build()
        {
            Ok(c) => c,
            Err(e) => return self.error(&format!("Failed to create HTTP client: {}", e)),
        };

        let mut last_error: Option<String> = None;
        for attempt in 0..=2 {
            let request_builder = if method == "POST" {
                let req = client
                    .post(parsed_url.clone())
                    .header("accept", "application/json")
                    .header("content-type", "application/json");
                match &body {
                    Some(v) => req.json(v),
                    None => req,
                }
            } else {
                client
                    .get(parsed_url.clone())
                    .header("accept", "application/json")
            };

            let response = request_builder.send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if (status.is_server_error() || status.as_u16() == 429) && attempt < 2 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            250 * (attempt as u64 + 1),
                        ))
                        .await;
                        continue;
                    }

                    if !status.is_success() {
                        return self.error(&format!(
                            "HTTP request failed with status {} for {}",
                            status, parsed_url
                        ));
                    }

                    let content_type = resp
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();
                    let bytes = match resp.bytes().await {
                        Ok(b) => b,
                        Err(e) => return self.error(&format!("Failed to read response body: {}", e)),
                    };

                    if bytes.len() > max_bytes {
                        return self.error(&format!(
                            "Response size {} exceeds max_bytes {}",
                            bytes.len(),
                            max_bytes
                        ));
                    }

                    let parsed_json: Value = match serde_json::from_slice(&bytes) {
                        Ok(v) => v,
                        Err(e) => {
                            return self.error(&format!(
                                "Response is not valid JSON (content-type '{}'): {}",
                                content_type, e
                            ));
                        }
                    };

                    let output = serde_json::json!({
                        "method": method,
                        "url": parsed_url.as_str(),
                        "status": status.as_u16(),
                        "content_type": content_type,
                        "bytes": bytes.len(),
                        "data": parsed_json,
                    });

                    return CommandResult {
                        success: true,
                        output: Some(truncate_output(&output.to_string())),
                        error: None,
                        exit_code: Some(0),
                    };
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    if attempt < 2 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            250 * (attempt as u64 + 1),
                        ))
                        .await;
                    }
                }
            }
        }

        self.error(&format!(
            "HTTP request failed after retries: {}",
            last_error.unwrap_or_else(|| "unknown error".to_string())
        ))
    }
}
