use async_trait::async_trait;
use futures::future;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde::Deserialize;
use serde_json::{json, Value};

pub struct DelegateTask;
pub struct DelegateTasks;

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

    fn bypass_dispatch_timeout(&self) -> bool {
        true
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

// ── Parallel delegation ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SubTask {
    task: String,
    #[serde(default)]
    context: String,
}

#[async_trait]
impl Tool for DelegateTasks {
    fn name(&self) -> &'static str {
        "delegate_tasks"
    }

    fn description(&self) -> &'static str {
        "Spawn multiple sub-agents and run all tasks in parallel, returning each \
         result when all have finished. Use this when the tasks are independent \
         of each other — it is significantly faster than calling delegate_task \
         one at a time. Each sub-agent gets the full tool set and its own \
         iteration budget."
    }

    fn toolset(&self) -> &'static str {
        "agent"
    }

    fn bypass_dispatch_timeout(&self) -> bool {
        true
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "description": "List of independent tasks to run in parallel.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "task": {
                                "type": "string",
                                "description": "Complete, self-contained task description."
                            },
                            "context": {
                                "type": "string",
                                "description": "Optional background context for this sub-task."
                            }
                        },
                        "required": ["task"]
                    },
                    "minItems": 1
                }
            },
            "required": ["tasks"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let sub_tasks: Vec<SubTask> = serde_json::from_value(params["tasks"].clone())
            .map_err(|e| ToolError::InvalidArgs(format!("'tasks' must be an array: {e}")))?;

        if sub_tasks.is_empty() {
            return Err(ToolError::InvalidArgs(
                "'tasks' must contain at least one item".into(),
            ));
        }

        let runner = ctx
            .sub_agent
            .as_ref()
            .ok_or_else(|| ToolError::Execution("sub-agent runner not available".into()))?;

        let futures: Vec<_> = sub_tasks
            .iter()
            .enumerate()
            .map(|(i, st)| {
                let full_task = if st.context.is_empty() {
                    st.task.clone()
                } else {
                    format!("Context:\n{}\n\nTask:\n{}", st.context, st.task)
                };
                let runner = runner.clone();
                let session_id = ctx.session_id.clone();
                async move {
                    let result = runner
                        .run_task(&full_task, &session_id)
                        .await
                        .map_err(|e| ToolError::Execution(e.to_string()))?;
                    Ok::<(usize, String), ToolError>((i, result))
                }
            })
            .collect();

        let results = future::join_all(futures).await;

        let mut outputs: Vec<(usize, String)> =
            results.into_iter().collect::<Result<Vec<_>, _>>()?;
        outputs.sort_by_key(|(i, _)| *i);

        let combined = outputs
            .into_iter()
            .enumerate()
            .map(|(i, (_, out))| format!("## Task {} result\n\n{out}", i + 1))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        Ok(ToolResult::ok("delegate_tasks", combined))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_requires_tasks_array() {
        let schema = DelegateTasks.schema();
        assert_eq!(schema["required"][0], "tasks");
        assert_eq!(schema["properties"]["tasks"]["type"], "array");
    }

    #[test]
    fn subtask_deserializes_without_context() {
        let v: SubTask = serde_json::from_str(r#"{"task":"do something"}"#).unwrap();
        assert_eq!(v.task, "do something");
        assert!(v.context.is_empty());
    }

    #[test]
    fn subtask_deserializes_with_context() {
        let v: SubTask =
            serde_json::from_str(r#"{"task":"do something","context":"bg info"}"#).unwrap();
        assert_eq!(v.context, "bg info");
    }
}
