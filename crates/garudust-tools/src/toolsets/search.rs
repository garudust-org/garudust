use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use garudust_memory::SessionDb;
use serde_json::json;

pub struct SessionSearch;

#[async_trait]
impl Tool for SessionSearch {
    fn name(&self) -> &'static str {
        "session_search"
    }
    fn description(&self) -> &'static str {
        "Full-text search across past conversation sessions stored in the local database"
    }
    fn toolset(&self) -> &'static str {
        "memory"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query — supports FTS5 syntax (AND, OR, NOT, phrase)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results to return (default 10, max 50)",
                    "default": 10
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("'query' required".into()))?;

        let limit = params["limit"].as_u64().unwrap_or(10).min(50) as usize;

        let db = SessionDb::open(&ctx.config.home_dir)
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let results = db
            .search(query, limit)
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        if results.is_empty() {
            return Ok(ToolResult::ok("", "No results found."));
        }

        let output = results
            .iter()
            .enumerate()
            .map(|(i, content)| format!("[{}] {}", i + 1, content))
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(ToolResult::ok("", output))
    }
}
