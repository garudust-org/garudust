use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde_json::json;

pub struct ReadFile;

#[async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &'static str {
        "read_file"
    }
    fn description(&self) -> &'static str {
        "Read a file from the filesystem"
    }
    fn toolset(&self) -> &'static str {
        "files"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to read" }
            },
            "required": ["path"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path required".into()))?;

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolResult::ok("", content))
    }
}

pub struct WriteFile;

#[async_trait]
impl Tool for WriteFile {
    fn name(&self) -> &'static str {
        "write_file"
    }
    fn description(&self) -> &'static str {
        "Write content to a file"
    }
    fn toolset(&self) -> &'static str {
        "files"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path":    { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path required".into()))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("content required".into()))?;

        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::Execution(e.to_string()))?;
        }
        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolResult::ok("", format!("Written to {path}")))
    }
}
