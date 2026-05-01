use std::path::{Path, PathBuf};

use async_trait::async_trait;
use garudust_core::{
    config::AgentConfig,
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde_json::json;

use crate::security::is_sensitive_write_path;

/// Returns the canonical form of `path` for existence checks.
/// For a path that does not yet exist, canonicalizes the parent and re-joins the filename.
fn try_canonicalize(path: &Path) -> Option<PathBuf> {
    if let Ok(c) = std::fs::canonicalize(path) {
        return Some(c);
    }
    // File doesn't exist yet (write case) — canonicalize parent
    let parent = path.parent()?;
    let file_name = path.file_name()?;
    let canonical_parent = std::fs::canonicalize(parent).ok()?;
    Some(canonical_parent.join(file_name))
}

/// Check whether `path` is within one of the allowed root directories.
/// Always blocks paths inside `~/.garudust/` regardless of allowed roots.
fn is_path_allowed(path: &Path, allowed_roots: &[PathBuf]) -> bool {
    let Some(canonical) = try_canonicalize(path) else {
        return false;
    };

    // Never allow access to the garudust secrets directory
    let garudust_dir = AgentConfig::garudust_dir();
    if canonical.starts_with(&garudust_dir) {
        return false;
    }

    allowed_roots
        .iter()
        .any(|root| std::fs::canonicalize(root).is_ok_and(|r| canonical.starts_with(&r)))
}

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
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path required".into()))?;

        if !is_path_allowed(Path::new(path), &ctx.config.security.allowed_read_paths) {
            return Err(ToolError::Execution(format!(
                "path '{path}' is outside allowed read directories"
            )));
        }

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

    fn is_destructive(&self) -> bool {
        true
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
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path required".into()))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("content required".into()))?;

        if !is_path_allowed(Path::new(path), &ctx.config.security.allowed_write_paths) {
            return Err(ToolError::Execution(format!(
                "path '{path}' is outside allowed write directories"
            )));
        }

        if is_sensitive_write_path(Path::new(path)) {
            return Err(ToolError::Execution(format!(
                "path '{path}' is a protected credential or system file"
            )));
        }

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
