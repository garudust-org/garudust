use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde_json::{json, Value};

pub struct DelegateTask;

#[async_trait]
impl Tool for DelegateTask {
    fn name(&self) -> &'static str {
        "delegate_task"
    }

    fn description(&self) -> &'static str {
        "Spawn a sub-agent to run an independent task in parallel. \
         Use this to decompose complex work: break the overall goal into \
         self-contained sub-tasks and delegate each one. Each sub-agent \
         gets the full tool set and runs to completion before returning \
         its output."
    }

    fn toolset(&self) -> &'static str {
        "agent"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "The complete, self-contained task description for the sub-agent."
                },
                "context": {
                    "type": "string",
                    "description": "Optional background context the sub-agent should know about."
                }
            },
            "required": ["task"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let task = params["task"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("'task' required".into()))?;
        let context = params["context"].as_str().unwrap_or("");

        let full_task = if context.is_empty() {
            task.to_string()
        } else {
            format!("Context:\n{context}\n\nTask:\n{task}")
        };

        let runner = ctx
            .sub_agent
            .as_ref()
            .ok_or_else(|| ToolError::Execution("sub-agent runner not available".into()))?;

        let output = runner
            .run_task(&full_task, &ctx.session_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolResult::ok("delegate_task", output))
    }
}
