use super::args::*;
use super::{truncate_output, SkillExecutor};
use crate::models::neural::CommandResult;
use serde_json::Value;

impl SkillExecutor {
    pub(super) async fn execute_browser(
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
            "navigate" | "browse_url" | "open_new_tab" => {
                let args: BrowserNavigateArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                if let Err(e) =
                    Self::enforce_domain_scope(&args.url, allowed_domains, blocked_domains)
                {
                    return self.error(&e);
                }
                match self.browser.navigate(&args.url).await {
                    Ok(result) => {
                        let output = serde_json::json!({
                            "url": result.url,
                            "title": result.title,
                            "content_preview": result.content_preview,
                        });
                        CommandResult {
                            success: true,
                            output: Some(output.to_string()),
                            error: None,
                            exit_code: Some(0),
                        }
                    }
                    Err(e) => {
                        if e.to_lowercase().contains("timed out")
                            || e.to_lowercase().contains("timeout")
                        {
                            self.browser.close().await;
                        }
                        self.error(&e)
                    }
                }
            }
            "screenshot" => match self.browser.screenshot().await {
                Ok(result) => {
                    let output = serde_json::json!({
                        "summary": "Screenshot captured successfully",
                        "width": result.width,
                        "height": result.height,
                        "data_uri": result.data_uri,
                        "has_image": true,
                    });
                    CommandResult {
                        success: true,
                        output: Some(output.to_string()),
                        error: None,
                        exit_code: Some(0),
                    }
                }
                Err(e) => self.error(&e),
            },
            "click" | "click_element" => {
                let args: BrowserClickArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                match self.browser.click(&args.selector).await {
                    Ok(()) => CommandResult {
                        success: true,
                        output: Some(format!("Clicked element: {}", args.selector)),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }
            "wait_for_selector" => {
                let args: WaitForSelectorArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                let timeout_ms = args.timeout_ms.unwrap_or(10_000).clamp(500, 60_000);
                match self
                    .browser
                    .wait_for_selector(&args.selector, timeout_ms)
                    .await
                {
                    Ok(()) => CommandResult {
                        success: true,
                        output: Some(format!(
                            "Selector '{}' found within {}ms",
                            args.selector, timeout_ms
                        )),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }
            "type_text" => {
                let args: TypeTextArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                let clear_first = args.clear_first.unwrap_or(false);
                match self
                    .browser
                    .type_text(&args.selector, &args.text, clear_first)
                    .await
                {
                    Ok(()) => CommandResult {
                        success: true,
                        output: Some(format!("Typed text into '{}'", args.selector)),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }
            "submit_form" => {
                let args: SubmitFormArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                let wait_ms = args.wait_ms.unwrap_or(1200).clamp(100, 10_000);
                let form_selector_json = match serde_json::to_string(&args.form_selector) {
                    Ok(v) => v,
                    Err(e) => return self.error(&format!("Invalid form selector: {}", e)),
                };
                let submit_selector_json = match serde_json::to_string(&args.submit_selector) {
                    Ok(v) => v,
                    Err(e) => return self.error(&format!("Invalid submit selector: {}", e)),
                };

                let script = format!(
                    "(function() {{
                        const formSel = {form_sel};
                        const submitSel = {submit_sel};
                        const form = formSel ? document.querySelector(formSel) : document.querySelector('form');
                        if (!form) return JSON.stringify({{ ok: false, error: 'form_not_found' }});
                        if (submitSel) {{
                            const submitEl = document.querySelector(submitSel);
                            if (!submitEl) return JSON.stringify({{ ok: false, error: 'submit_not_found' }});
                            submitEl.click();
                            return JSON.stringify({{ ok: true, method: 'click_submit' }});
                        }}
                        const ev = new Event('submit', {{ bubbles: true, cancelable: true }});
                        form.dispatchEvent(ev);
                        if (typeof form.submit === 'function') form.submit();
                        return JSON.stringify({{ ok: true, method: 'form_submit' }});
                    }})()",
                    form_sel = form_selector_json,
                    submit_sel = submit_selector_json
                );

                match self.browser.evaluate(script.as_str()).await {
                    Ok(result) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
                        CommandResult {
                            success: true,
                            output: Some(result),
                            error: None,
                            exit_code: Some(0),
                        }
                    }
                    Err(e) => self.error(&format!("Failed to submit form: {}", e)),
                }
            }
            "go_back" => {
                let args: GoBackArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                let wait_ms = args.wait_ms.unwrap_or(1000).clamp(100, 10_000);
                let go_back_script =
                    "(function() { history.back(); return JSON.stringify({ ok: true }); })()";
                match self.browser.evaluate(go_back_script).await {
                    Ok(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
                        let snapshot_script = "(function() { return JSON.stringify({ url: location.href, title: document.title || '', content_preview: (document.body ? document.body.innerText.substring(0, 2000) : '') }); })()";
                        match self.browser.evaluate(snapshot_script).await {
                            Ok(snapshot) => CommandResult {
                                success: true,
                                output: Some(snapshot),
                                error: None,
                                exit_code: Some(0),
                            },
                            Err(e) => {
                                self.error(&format!("Back navigation completed but snapshot failed: {}", e))
                            }
                        }
                    }
                    Err(e) => self.error(&format!("Failed to navigate back: {}", e)),
                }
            }
            "get_content" | "get_page_content" => match self.browser.get_content().await {
                Ok(content) => CommandResult {
                    success: true,
                    output: Some(truncate_output(&content)),
                    error: None,
                    exit_code: Some(0),
                },
                Err(e) => self.error(&e),
            },
            "get_page_snapshot" => {
                let script = "(function() { return JSON.stringify({ url: location.href, title: document.title || '', content_preview: (document.body ? document.body.innerText.substring(0, 2000) : '') }); })()";
                match self.browser.evaluate(script).await {
                    Ok(snapshot) => CommandResult {
                        success: true,
                        output: Some(snapshot),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&format!("Failed to snapshot page: {}", e)),
                }
            }
            "extract_links" => {
                let args: ExtractLinksArgs = serde_json::from_value(params.clone()).unwrap_or_default();
                let limit = args.limit.unwrap_or(100).clamp(1, 500);
                let script = format!(
                    "(function() {{ const out = []; const els = document.querySelectorAll('a[href]'); for (let i = 0; i < els.length && out.length < {}; i++) {{ const el = els[i]; out.push({{ href: el.href || '', text: (el.innerText || el.textContent || '').trim(), title: (el.getAttribute('title') || '').trim() }}); }} return JSON.stringify(out); }})()",
                    limit
                );
                match self.browser.evaluate(&script).await {
                    Ok(result) => CommandResult {
                        success: true,
                        output: Some(result),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&format!("Failed to extract links: {}", e)),
                }
            }
            _ => self.error(&format!("Unknown browser method: {}", method)),
        }
    }
}
