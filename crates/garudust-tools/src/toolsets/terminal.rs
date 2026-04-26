use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{ApprovalDecision, Tool, ToolContext},
    types::ToolResult,
};
use serde_json::json;
use tokio::process::Command;

pub struct Terminal;

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
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let command = params["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("command required".into()))?;
        let description = params["description"].as_str().unwrap_or("run command");
        let timeout_secs = params["timeout_secs"].as_u64().unwrap_or(30);

        let decision = ctx.approver.approve(command, description).await;
        if decision == ApprovalDecision::Denied {
            return Err(ToolError::ApprovalDenied);
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            Command::new("sh").arg("-c").arg(command).output(),
        )
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
