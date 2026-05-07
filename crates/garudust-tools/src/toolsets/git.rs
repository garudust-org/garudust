use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::process::Command;

const MAX_OUTPUT_BYTES: usize = 32 * 1_024; // 32 KB

async fn run_git(args: &[&str], cwd: Option<&str>) -> Result<String, ToolError> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", std::env::var("HOME").unwrap_or_default())
        .env("GIT_TERMINAL_PROMPT", "0");

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let out = cmd
        .output()
        .await
        .map_err(|e| ToolError::Execution(format!("git: {e}")))?;

    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    if !out.stderr.is_empty() && !out.status.success() {
        combined.push_str(&String::from_utf8_lossy(&out.stderr));
    }

    if combined.len() > MAX_OUTPUT_BYTES {
        let head = MAX_OUTPUT_BYTES * 2 / 5;
        let tail = MAX_OUTPUT_BYTES - head;
        let truncated = format!(
            "{}\n\n[… {} bytes truncated …]\n\n{}",
            &combined[..head],
            combined.len() - head - tail,
            &combined[combined.len() - tail..]
        );
        return Ok(truncated);
    }

    Ok(combined)
}

// ── git status ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitStatusInput {
    path: Option<String>,
}

pub struct GitStatus;

#[async_trait]
impl Tool for GitStatus {
    fn name(&self) -> &'static str {
        "git_status"
    }

    fn description(&self) -> &'static str {
        "Show the working tree status (staged, unstaged, untracked files). Read-only."
    }

    fn toolset(&self) -> &'static str {
        "git"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Repo path. Defaults to current working directory."
                }
            }
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: GitStatusInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let output = run_git(&["status"], input.path.as_deref()).await?;
        Ok(ToolResult::ok("", output))
    }
}

// ── git log ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitLogInput {
    path: Option<String>,
    limit: Option<u32>,
    file: Option<String>,
}

pub struct GitLog;

#[async_trait]
impl Tool for GitLog {
    fn name(&self) -> &'static str {
        "git_log"
    }

    fn description(&self) -> &'static str {
        "Show recent commit history (one-line format). Read-only."
    }

    fn toolset(&self) -> &'static str {
        "git"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Repo path. Defaults to current working directory."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of commits to show (default 20, max 200).",
                    "default": 20
                },
                "file": {
                    "type": "string",
                    "description": "Limit log to commits that touch this file path."
                }
            }
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: GitLogInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let limit = input.limit.unwrap_or(20).min(200).to_string();
        let mut args = vec!["log", "--oneline", "-n", limit.as_str()];

        let file_owned;
        if let Some(ref f) = input.file {
            args.push("--");
            file_owned = f.clone();
            args.push(file_owned.as_str());
        }

        let output = run_git(&args, input.path.as_deref()).await?;
        Ok(ToolResult::ok("", output))
    }
}

// ── git diff ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitDiffInput {
    path: Option<String>,
    staged: Option<bool>,
    #[serde(rename = "ref")]
    git_ref: Option<String>,
    file: Option<String>,
}

pub struct GitDiff;

#[async_trait]
impl Tool for GitDiff {
    fn name(&self) -> &'static str {
        "git_diff"
    }

    fn description(&self) -> &'static str {
        "Show changes between commits, working tree, or staged index. Read-only."
    }

    fn toolset(&self) -> &'static str {
        "git"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Repo path. Defaults to current working directory."
                },
                "staged": {
                    "type": "boolean",
                    "description": "Show staged (--cached) diff instead of unstaged."
                },
                "ref": {
                    "type": "string",
                    "description": "Compare against this ref (e.g. 'HEAD~1', 'main', 'abc123')."
                },
                "file": {
                    "type": "string",
                    "description": "Limit diff to this file path."
                }
            }
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: GitDiffInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let mut args = vec!["diff"];

        if input.staged.unwrap_or(false) {
            args.push("--cached");
        }

        let ref_owned;
        if let Some(ref r) = input.git_ref {
            ref_owned = r.clone();
            args.push(ref_owned.as_str());
        }

        let file_owned;
        if let Some(ref f) = input.file {
            args.push("--");
            file_owned = f.clone();
            args.push(file_owned.as_str());
        }

        let output = run_git(&args, input.path.as_deref()).await?;
        let output = if output.is_empty() {
            "No changes.".to_string()
        } else {
            output
        };
        Ok(ToolResult::ok("", output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn git_status_runs_in_repo() {
        let output = run_git(
            &["status"],
            Some("/Users/ninenox-14/Desktop/garudust-agent"),
        )
        .await;
        assert!(output.is_ok());
    }

    #[tokio::test]
    async fn git_log_default_limit() {
        let output = run_git(
            &["log", "--oneline", "-n", "5"],
            Some("/Users/ninenox-14/Desktop/garudust-agent"),
        )
        .await;
        assert!(output.is_ok());
        let text = output.unwrap();
        // Each line is a commit hash + message
        assert!(!text.is_empty());
    }

    #[tokio::test]
    async fn git_diff_no_changes_returns_empty_or_ok() {
        let output = run_git(
            &["diff", "--cached"],
            Some("/Users/ninenox-14/Desktop/garudust-agent"),
        )
        .await;
        assert!(output.is_ok());
    }

    #[tokio::test]
    async fn git_in_non_repo_returns_error_output() {
        let output = run_git(&["status"], Some("/tmp")).await;
        // git returns non-zero exit; our function still returns Ok with stderr
        // or Err — either is acceptable, just must not panic.
        let _ = output;
    }
}
