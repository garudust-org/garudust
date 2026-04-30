use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde_json::json;
use tokio::process::Command;

/// Only these variables are forwarded to subprocesses.
/// Secrets (API keys, tokens, passwords) are deliberately excluded.
const ENV_ALLOWLIST: &[&str] = &[
    "PATH", "HOME", "USER", "LOGNAME", "SHELL", "LANG", "LC_ALL", "TMPDIR", "TEMP", "TMP", "TERM",
];

pub struct Terminal;

/// Returns true if the command string contains a literal path to the garudust secrets file,
/// which should never be directly accessible from the terminal tool.
fn references_secrets_file(command: &str) -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    let protected = format!("{home}/.garudust/.env");
    command.contains(&protected)
        || command.contains("~/.garudust/.env")
        || command.contains("$HOME/.garudust/.env")
        || command.contains("${HOME}/.garudust/.env")
}

#[async_trait]
impl Tool for Terminal {
    fn name(&self) -> &'static str {
        "terminal"
    }
    fn description(&self) -> &'static str {
        "Run a shell command and return the output"
    }
    fn toolset(&self) -> &'static str {
        "terminal"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command":     { "type": "string", "description": "Shell command to execute" },
                "description": { "type": "string", "description": "What this command does" },
                "timeout_secs": { "type": "integer", "default": 30 }
            },
            "required": ["command", "description"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let command = params["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("command required".into()))?;
        let timeout_secs = params["timeout_secs"].as_u64().unwrap_or(30);

        if references_secrets_file(command) {
            return Err(ToolError::Execution(
                "access to ~/.garudust/.env is not permitted".into(),
            ));
        }

        // Approval and audit logging are handled by ToolRegistry::dispatch()
        // via the is_destructive() property — no per-tool check needed here.

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);

        // Clear all environment variables and only pass through the safe allowlist.
        // This prevents secrets (API keys, tokens) from leaking into subprocesses.
        cmd.env_clear();
        for key in ENV_ALLOWLIST {
            if let Ok(val) = std::env::var(key) {
                cmd.env(key, val);
            }
        }

        let output =
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output())
                .await
                .map_err(|_| ToolError::Timeout(timeout_secs))?
                .map_err(|e| ToolError::Execution(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        let combined = if stderr.is_empty() {
            stdout
        } else if stdout.is_empty() {
            stderr
        } else {
            format!("{stdout}\n[stderr]\n{stderr}")
        };

        let is_error = !output.status.success();
        if is_error {
            Ok(ToolResult::err("", combined))
        } else {
            Ok(ToolResult::ok("", combined))
        }
    }
}
