use std::sync::Arc;

use async_trait::async_trait;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use garudust_core::{
    error::ToolError,
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
            let config = BrowserConfig::builder()
                .arg("--no-sandbox")
                .arg("--disable-dev-shm-usage")
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
