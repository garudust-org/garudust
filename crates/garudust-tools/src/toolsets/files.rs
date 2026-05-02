use std::path::{Path, PathBuf};

use async_trait::async_trait;
use garudust_core::{
    config::AgentConfig,
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde::Deserialize;
use serde_json::json;

use crate::security::is_sensitive_write_path;

/// Maximum bytes returned from a single file read.
/// Prevents large files from flooding the context window.
const MAX_FILE_READ_BYTES: usize = 512 * 1_024; // 512 KB

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

        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let content = if bytes.len() > MAX_FILE_READ_BYTES {
            format!(
                "{}\n[truncated — file is {} bytes, showing first {} KB]",
                String::from_utf8_lossy(&bytes[..MAX_FILE_READ_BYTES]),
                bytes.len(),
                MAX_FILE_READ_BYTES / 1_024,
            )
        } else {
            String::from_utf8_lossy(&bytes).into_owned()
        };

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

// ── ListDirectory ─────────────────────────────────────────────────────────────

const MAX_ENTRIES: usize = 200;

fn format_size(bytes: u64) -> String {
    if bytes < 1_024 {
        format!("{bytes} B")
    } else if bytes < 1_024 * 1_024 {
        format!("{}.{} KB", bytes / 1_024, (bytes % 1_024) * 10 / 1_024)
    } else {
        format!(
            "{}.{} MB",
            bytes / (1_024 * 1_024),
            (bytes % (1_024 * 1_024)) * 10 / (1_024 * 1_024)
        )
    }
}

#[derive(Deserialize)]
struct ListDirInput {
    path: String,
    pattern: Option<String>,
    max_depth: Option<usize>,
}

struct DirEntry {
    rel_path: String,
    is_dir: bool,
    size: Option<u64>,
}

pub struct ListDirectory;

#[async_trait]
impl Tool for ListDirectory {
    fn name(&self) -> &'static str {
        "list_directory"
    }
    fn description(&self) -> &'static str {
        "List files and directories at a given path. Supports glob patterns and depth limits."
    }
    fn toolset(&self) -> &'static str {
        "files"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list"
                },
                "pattern": {
                    "type": "string",
                    "description": "Glob filter applied to relative paths (e.g. '**/*.rs', '*.toml')"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Recursion depth limit (default 3, max 10)",
                    "default": 3
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let input: ListDirInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let dir = Path::new(&input.path);
        let max_depth = input.max_depth.unwrap_or(3).min(10);

        if !is_path_allowed(dir, &ctx.config.security.allowed_read_paths) {
            return Err(ToolError::Execution(format!(
                "path '{}' is outside allowed read directories",
                input.path
            )));
        }
        if !dir.exists() {
            return Err(ToolError::Execution(format!(
                "path '{}' does not exist",
                input.path
            )));
        }
        if !dir.is_dir() {
            return Err(ToolError::Execution(format!(
                "path '{}' is not a directory",
                input.path
            )));
        }

        // Build glob matcher if a pattern was supplied.
        let glob_matcher = input
            .pattern
            .as_deref()
            .map(|p| {
                let glob = globset::Glob::new(p)
                    .map_err(|e| ToolError::InvalidArgs(format!("invalid glob pattern: {e}")))?;
                globset::GlobSet::builder()
                    .add(glob)
                    .build()
                    .map_err(|e| ToolError::InvalidArgs(format!("invalid glob pattern: {e}")))
            })
            .transpose()?;

        // Walk the directory tree.
        let mut entries: Vec<DirEntry> = Vec::new();
        let mut truncated = false;

        for result in walkdir::WalkDir::new(dir)
            .max_depth(max_depth)
            .sort_by_file_name()
        {
            let entry = result.map_err(|e| ToolError::Execution(e.to_string()))?;

            // Skip the root itself.
            if entry.path() == dir {
                continue;
            }

            let rel = entry
                .path()
                .strip_prefix(dir)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();

            // Apply glob filter when provided.
            if let Some(ref m) = glob_matcher {
                if !m.is_match(&rel) {
                    continue;
                }
            }

            if entries.len() >= MAX_ENTRIES {
                truncated = true;
                break;
            }

            let is_dir = entry.file_type().is_dir();
            let size = if is_dir {
                None
            } else {
                entry.metadata().ok().map(|m| m.len())
            };

            entries.push(DirEntry {
                rel_path: rel,
                is_dir,
                size,
            });
        }

        if entries.is_empty() {
            return Ok(ToolResult::ok(
                "",
                format!("{} — no items found", input.path),
            ));
        }

        let header = format!(
            "{} ({} items{})\n",
            input.path,
            entries.len(),
            if truncated {
                format!(", showing first {MAX_ENTRIES}")
            } else {
                String::new()
            }
        );

        let lines: Vec<String> = entries
            .iter()
            .map(|e| {
                if e.is_dir {
                    format!("[dir]  {}/", e.rel_path)
                } else {
                    let sz = e.size.map(format_size).unwrap_or_default();
                    format!("[file] {}  {}", e.rel_path, sz)
                }
            })
            .collect();

        Ok(ToolResult::ok(
            "",
            format!("{header}\n{}", lines.join("\n")),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use garudust_core::{
        budget::IterationBudget,
        config::AgentConfig,
        tool::{ApprovalDecision, CommandApprover, ToolContext},
    };

    use super::*;

    struct AutoApprove;
    #[async_trait]
    impl CommandApprover for AutoApprove {
        async fn approve(&self, _: &str, _: &str) -> ApprovalDecision {
            ApprovalDecision::Approved
        }
    }

    struct NopMemory;
    #[async_trait]
    impl garudust_core::memory::MemoryStore for NopMemory {
        async fn read_memory(
            &self,
        ) -> Result<garudust_core::memory::MemoryContent, garudust_core::AgentError> {
            Ok(garudust_core::memory::MemoryContent::default())
        }
        async fn write_memory(
            &self,
            _: &garudust_core::memory::MemoryContent,
        ) -> Result<(), garudust_core::AgentError> {
            Ok(())
        }
        async fn read_user_profile(&self) -> Result<String, garudust_core::AgentError> {
            Ok(String::new())
        }
        async fn write_user_profile(&self, _: &str) -> Result<(), garudust_core::AgentError> {
            Ok(())
        }
    }

    fn make_ctx() -> ToolContext {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut config = AgentConfig::default();
        // Allow the entire workspace root for test reads.
        let workspace_root = cwd
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists() && p.join("crates").exists())
            .unwrap_or(&cwd)
            .to_path_buf();
        config.security.allowed_read_paths = vec![workspace_root];
        ToolContext {
            session_id: "s".into(),
            agent_id: "a".into(),
            iteration: 0,
            budget: Arc::new(IterationBudget::new(10)),
            memory: Arc::new(NopMemory),
            config: Arc::new(config),
            approver: Arc::new(AutoApprove),
            sub_agent: None,
        }
    }

    #[tokio::test]
    async fn lists_current_directory() {
        let ctx = make_ctx();
        let cwd = std::env::current_dir().unwrap();
        let result = ListDirectory
            .execute(json!({ "path": cwd.to_str().unwrap() }), &ctx)
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains('['));
    }

    #[tokio::test]
    async fn glob_pattern_filters_results() {
        let ctx = make_ctx();
        let cwd = std::env::current_dir().unwrap();
        let result = ListDirectory
            .execute(
                json!({ "path": cwd.to_str().unwrap(), "pattern": "**/*.toml" }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        // Every file line should end with .toml (dirs may still appear if matched)
        for line in result.content.lines() {
            if line.starts_with("[file]") {
                assert!(line.contains(".toml"), "unexpected file: {line}");
            }
        }
    }

    #[tokio::test]
    async fn rejects_nonexistent_path() {
        let ctx = make_ctx();
        let err = ListDirectory
            .execute(json!({ "path": "/nonexistent/path/xyz" }), &ctx)
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::Execution(_)));
    }

    #[tokio::test]
    async fn rejects_file_path() {
        let ctx = make_ctx();
        let cwd = std::env::current_dir().unwrap();
        // Cargo.toml exists in workspace root
        let file_path = cwd.join("Cargo.toml");
        if file_path.exists() {
            let err = ListDirectory
                .execute(json!({ "path": file_path.to_str().unwrap() }), &ctx)
                .await
                .unwrap_err();
            assert!(matches!(err, ToolError::Execution(_)));
        }
    }

    #[tokio::test]
    async fn read_file_truncates_large_content() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        // Write slightly more than the cap so truncation fires
        let big = "x".repeat(MAX_FILE_READ_BYTES + 1_024);
        tmp.write_all(big.as_bytes()).unwrap();

        let mut config = AgentConfig::default();
        config.security.allowed_read_paths = vec![tmp.path().parent().unwrap().to_path_buf()];
        let ctx = ToolContext {
            session_id: "s".into(),
            agent_id: "a".into(),
            iteration: 0,
            budget: Arc::new(IterationBudget::new(10)),
            memory: Arc::new(NopMemory),
            config: Arc::new(config),
            approver: Arc::new(AutoApprove),
            sub_agent: None,
        };

        let result = ReadFile
            .execute(json!({ "path": tmp.path().to_str().unwrap() }), &ctx)
            .await
            .unwrap();

        assert!(
            result.content.contains("[truncated"),
            "expected truncation notice"
        );
        assert!(
            result.content.len() < big.len(),
            "output should be shorter than input"
        );
    }

    #[tokio::test]
    async fn read_file_small_file_untruncated() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();

        let mut config = AgentConfig::default();
        config.security.allowed_read_paths = vec![tmp.path().parent().unwrap().to_path_buf()];
        let ctx = ToolContext {
            session_id: "s".into(),
            agent_id: "a".into(),
            iteration: 0,
            budget: Arc::new(IterationBudget::new(10)),
            memory: Arc::new(NopMemory),
            config: Arc::new(config),
            approver: Arc::new(AutoApprove),
            sub_agent: None,
        };

        let result = ReadFile
            .execute(json!({ "path": tmp.path().to_str().unwrap() }), &ctx)
            .await
            .unwrap();

        assert_eq!(result.content, "hello world");
    }
}
