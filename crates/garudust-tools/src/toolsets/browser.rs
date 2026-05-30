use std::sync::Arc;

use async_trait::async_trait;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use garudust_core::{
    error::ToolError,
    net_guard,
    tool::{Tool, ToolContext},
    types::ToolResult,
};

struct BrowserSession {
    _browser: Browser,
    page: Page,
}

pub struct BrowserTool {
    session: Arc<Mutex<Option<BrowserSession>>>,
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
        }
    }
}

impl BrowserTool {
    pub fn new() -> Self {
        Self::default()
    }

    async fn ensure_session(&self) -> Result<(), ToolError> {
        let mut guard = self.session.lock().await;
        if guard.is_none() {
            // --no-sandbox is required when running as root (e.g. Docker).
            // Avoid it otherwise to keep the renderer sandbox intact.
            let running_as_root = std::env::var("USER").is_ok_and(|u| u == "root")
                || std::env::var("UID").is_ok_and(|u| u == "0");

            let mut builder = BrowserConfig::builder().arg("--disable-dev-shm-usage");
            if running_as_root {
                builder = builder.arg("--no-sandbox");
            }
            let config = builder
                .build()
                .map_err(|e| ToolError::Execution(format!("browser config: {e}")))?;

            let (browser, mut handler) = Browser::launch(config).await.map_err(|e| {
                ToolError::Execution(format!(
                    "failed to launch browser (is Chrome/Chromium installed?): {e}"
                ))
            })?;

            tokio::spawn(async move { while handler.next().await.is_some() {} });

            let page = browser
                .new_page("about:blank")
                .await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            *guard = Some(BrowserSession {
                _browser: browser,
                page,
            });
        }
        Ok(())
    }

    async fn get_page(&self) -> Result<Page, ToolError> {
        let guard = self.session.lock().await;
        guard
            .as_ref()
            .map(|s| s.page.clone())
            .ok_or_else(|| ToolError::Execution("no browser session — call navigate first".into()))
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &'static str {
        "browser"
    }

    fn description(&self) -> &'static str {
        "Control a real Chrome/Chromium browser via CDP. Handles JavaScript-heavy pages, \
         login forms, screenshots, and JS evaluation. Maintains a single session across calls."
    }

    fn toolset(&self) -> &'static str {
        "browser"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["navigate", "get_text", "screenshot", "click", "type", "evaluate", "close"],
                    "description": "navigate: open URL | get_text: visible text of page | screenshot: save PNG | click: click element | type: type text | evaluate: run JS | close: close browser"
                },
                "url": {
                    "type": "string",
                    "description": "URL to open (action=navigate)"
                },
                "selector": {
                    "type": "string",
                    "description": "CSS selector for target element (action=click or type)"
                },
                "text": {
                    "type": "string",
                    "description": "Text to type into element (action=type)"
                },
                "script": {
                    "type": "string",
                    "description": "JavaScript expression to evaluate (action=evaluate)"
                },
                "path": {
                    "type": "string",
                    "description": "File path to save screenshot PNG (action=screenshot, default: /tmp/garudust-screenshot.png)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = params["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("action required".into()))?;

        match action {
            "navigate" => {
                let url = params["url"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("url required for navigate".into()))?;
                // SSRF guard: reject non-http(s) schemes (file:, chrome:, data:),
                // private/reserved IPs, and cloud metadata endpoints *before* we
                // spawn Chrome — otherwise a prompt-injected URL could drive the
                // headless browser into the internal network or local filesystem.
                net_guard::is_safe_url(url)?;
                self.ensure_session().await?;
                let page = self.get_page().await?;
                page.goto(url)
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                let title: String = page
                    .evaluate("document.title")
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?
                    .into_value()
                    .unwrap_or_default();
                Ok(ToolResult::ok(
                    "browser",
                    format!("Navigated to {url}\nTitle: {title}"),
                ))
            }

            "get_text" => {
                let page = self.get_page().await?;
                let text: String = page
                    .evaluate("document.body.innerText")
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?
                    .into_value()
                    .unwrap_or_default();
                let out = if text.len() > 8000 {
                    text[..8000].to_string()
                } else {
                    text
                };
                Ok(ToolResult::ok("browser", out))
            }

            "screenshot" => {
                let path = params["path"]
                    .as_str()
                    .unwrap_or("/tmp/garudust-screenshot.png");

                // Restrict screenshot writes to /tmp to prevent arbitrary file writes
                let path_buf = std::path::PathBuf::from(path);
                let safe_root = std::path::Path::new("/tmp");
                let canonical_parent = path_buf
                    .parent()
                    .and_then(|p| std::fs::canonicalize(p).ok())
                    .unwrap_or_else(|| safe_root.to_path_buf());
                if !canonical_parent.starts_with(safe_root) {
                    return Err(ToolError::InvalidArgs(
                        "screenshot path must be under /tmp".into(),
                    ));
                }

                let page = self.get_page().await?;
                let data = page
                    .screenshot(chromiumoxide::page::ScreenshotParams::builder().build())
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                std::fs::write(path, &data).map_err(|e| ToolError::Execution(e.to_string()))?;
                Ok(ToolResult::ok(
                    "browser",
                    format!("Screenshot saved to {path} ({} bytes)", data.len()),
                ))
            }

            "click" => {
                let selector = params["selector"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("selector required for click".into()))?;
                let page = self.get_page().await?;
                page.find_element(selector)
                    .await
                    .map_err(|e| {
                        ToolError::Execution(format!("element '{selector}' not found: {e}"))
                    })?
                    .click()
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                Ok(ToolResult::ok("browser", format!("Clicked '{selector}'")))
            }

            "type" => {
                let selector = params["selector"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("selector required for type".into()))?;
                let text = params["text"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("text required for type".into()))?;
                let page = self.get_page().await?;
                page.find_element(selector)
                    .await
                    .map_err(|e| {
                        ToolError::Execution(format!("element '{selector}' not found: {e}"))
                    })?
                    .type_str(text)
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                Ok(ToolResult::ok(
                    "browser",
                    format!("Typed into '{selector}'"),
                ))
            }

            "evaluate" => {
                let script = params["script"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("script required for evaluate".into()))?;
                let page = self.get_page().await?;
                let result = page
                    .evaluate(script)
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                let value: Value = result.into_value().unwrap_or(Value::Null);
                Ok(ToolResult::ok("browser", value.to_string()))
            }

            "close" => {
                let mut guard = self.session.lock().await;
                *guard = None;
                Ok(ToolResult::ok("browser", "Browser closed".to_string()))
            }

            other => Err(ToolError::InvalidArgs(format!("unknown action: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use garudust_core::{
        budget::IterationBudget,
        config::AgentConfig,
        memory::{MemoryContent, MemoryStore},
        tool::{ApprovalDecision, CommandApprover, SkillPermissions, ToolContext},
        AgentError,
    };
    use serde_json::json;
    use tokio::sync::RwLock;

    use super::*;

    struct AutoApprove;
    #[async_trait]
    impl CommandApprover for AutoApprove {
        async fn approve(&self, _: &str, _: &str, _: &str) -> ApprovalDecision {
            ApprovalDecision::Approved
        }
    }

    struct NopMemory;
    #[async_trait]
    impl MemoryStore for NopMemory {
        async fn read_memory(&self) -> Result<MemoryContent, AgentError> {
            Ok(MemoryContent::default())
        }
        async fn write_memory(&self, _: &MemoryContent) -> Result<(), AgentError> {
            Ok(())
        }
        async fn read_user_profile(&self) -> Result<String, AgentError> {
            Ok(String::new())
        }
        async fn write_user_profile(&self, _: &str) -> Result<(), AgentError> {
            Ok(())
        }
    }

    fn make_ctx() -> ToolContext {
        ToolContext {
            session_id: "test".into(),
            conv_key: String::new(),
            user_id: String::new(),
            agent_id: "test".into(),
            iteration: 0,
            budget: Arc::new(IterationBudget::new(10)),
            memory: Arc::new(NopMemory),
            config: Arc::new(AgentConfig::default()),
            approver: Arc::new(AutoApprove),
            sub_agent: None,
            skill_permissions: Arc::new(RwLock::new(SkillPermissions::default())),
            required_tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// navigate must reject SSRF-able URLs *before* launching Chrome, so these
    /// assertions hold even in CI where no browser is installed.
    async fn assert_navigate_blocked(url: &str) {
        let tool = BrowserTool::default();
        let err = tool
            .execute(json!({ "action": "navigate", "url": url }), &make_ctx())
            .await
            .expect_err("navigate should reject unsafe URL before any browser launch");
        let msg = err.to_string();
        assert!(
            msg.contains("blocked") || msg.contains("only http/https") || msg.contains("no host"),
            "unexpected error for {url}: {msg}"
        );
    }

    #[tokio::test]
    async fn navigate_rejects_loopback() {
        assert_navigate_blocked("http://127.0.0.1/admin").await;
        assert_navigate_blocked("http://localhost:8080/").await;
    }

    #[tokio::test]
    async fn navigate_rejects_private_ranges() {
        assert_navigate_blocked("http://10.0.0.1/").await;
        assert_navigate_blocked("http://192.168.1.1/").await;
        assert_navigate_blocked("http://172.16.0.1/").await;
    }

    #[tokio::test]
    async fn navigate_rejects_cloud_metadata() {
        assert_navigate_blocked("http://169.254.169.254/latest/meta-data/").await;
        assert_navigate_blocked("http://metadata.google.internal/computeMetadata/v1/").await;
    }

    #[tokio::test]
    async fn navigate_rejects_file_and_other_schemes() {
        assert_navigate_blocked("file:///etc/passwd").await;
        assert_navigate_blocked("chrome://settings").await;
        assert_navigate_blocked("data:text/html,<h1>x</h1>").await;
    }

    #[tokio::test]
    async fn navigate_requires_url() {
        let tool = BrowserTool::default();
        let err = tool
            .execute(json!({ "action": "navigate" }), &make_ctx())
            .await
            .expect_err("navigate without url should error");
        assert!(err.to_string().contains("url required"));
    }
}
