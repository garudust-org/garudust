use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde_json::json;

// ─── WebFetch ─────────────────────────────────────────────────────────────────

pub struct WebFetch;

#[async_trait]
impl Tool for WebFetch {
    fn name(&self) -> &'static str {
        "web_fetch"
    }
    fn description(&self) -> &'static str {
        "Fetch content from a URL"
    }
    fn toolset(&self) -> &'static str {
        "web"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to fetch" }
            },
            "required": ["url"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let url = params["url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("url required".into()))?;

        let body = reqwest::get(url)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .text()
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolResult::ok("", body))
    }
}

// ─── WebSearch ────────────────────────────────────────────────────────────────

pub struct WebSearch;

#[async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &'static str {
        "web_search"
    }
    fn description(&self) -> &'static str {
        "Search the web via Brave Search API. Requires BRAVE_SEARCH_API_KEY."
    }
    fn toolset(&self) -> &'static str {
        "web"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of results to return (1-10, default 5)",
                    "default": 5
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("query required".into()))?;
        let count = params["count"].as_u64().unwrap_or(5).clamp(1, 10);

        let api_key = std::env::var("BRAVE_SEARCH_API_KEY").map_err(|_| {
            ToolError::Execution(
                "BRAVE_SEARCH_API_KEY not set. Get a free key at https://brave.com/search/api/"
                    .into(),
            )
        })?;

        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .query(&[("q", query), ("count", &count.to_string())])
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &api_key)
            .send()
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ToolError::Execution(format!(
                "Brave Search API error {status}: {body}"
            )));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let results = data["web"]["results"]
            .as_array()
            .ok_or_else(|| ToolError::Execution("unexpected response format".into()))?;

        if results.is_empty() {
            return Ok(ToolResult::ok("", "No results found."));
        }

        let formatted: Vec<String> = results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let title = r["title"].as_str().unwrap_or("(no title)");
                let url = r["url"].as_str().unwrap_or("");
                let desc = r["description"].as_str().unwrap_or("").trim();
                format!("{}. **{}**\n   {}\n   {}", i + 1, title, url, desc)
            })
            .collect();

        Ok(ToolResult::ok("", formatted.join("\n\n")))
    }
}
