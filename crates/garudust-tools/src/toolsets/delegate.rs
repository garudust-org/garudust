use async_trait::async_trait;
use futures::stream::{self, StreamExt};
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
         iteration budget. A task that fails is reported as '[FAILED: ...]' in \
         its result block; the other tasks' results are still returned, so you \
         can retry only the failed ones or proceed with what succeeded."
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

        // Build futures eagerly into a Vec so each owns its cloned inputs — a lazy
        // iterator borrowing `sub_tasks` trips a higher-ranked lifetime error under
        // buffer_unordered.
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
                // Carry the index in both arms so a failure is reported against
                // its task instead of being dropped. Never propagate the error
                // here — one failing sub-agent must not discard the others.
                async move {
                    let result = runner
                        .run_task(&full_task, &session_id)
                        .await
                        .map_err(|e| e.to_string());
                    (i, result)
                }
            })
            .collect();

        // Bound the fan-out width: at most `max_concurrent_sub_agents` sub-agents
        // run at once, the rest queue. `0` (or fewer tasks than the cap) means
        // run them all concurrently — identical to the old join_all behaviour.
        let cap = ctx.config.max_concurrent_sub_agents;
        let concurrency = if cap == 0 {
            sub_tasks.len()
        } else {
            cap.min(sub_tasks.len())
        };
        let mut outputs: Vec<(usize, Result<String, String>)> = stream::iter(futures)
            .buffer_unordered(concurrency)
            .collect()
            .await;
        outputs.sort_by_key(|(i, _)| *i);

        // Return partial results: successes are rendered as-is, failures are
        // annotated in place so the parent agent can retry or carry on.
        let combined = outputs
            .into_iter()
            .enumerate()
            .map(|(n, (_, res))| match res {
                Ok(out) => format!("## Task {} result\n\n{out}", n + 1),
                Err(e) => format!("## Task {} result\n\n[FAILED: {e}]", n + 1),
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        Ok(ToolResult::ok("delegate_tasks", combined))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use garudust_core::{
        budget::IterationBudget,
        config::AgentConfig,
        error::AgentError,
        memory::MemoryStore,
        tool::{ApprovalDecision, CommandApprover, SubAgentRunner, ToolContext},
    };

    use super::*;

    /// Records how many sub-agents run concurrently so a test can assert the cap.
    struct CountingRunner {
        current: AtomicUsize,
        max_seen: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl SubAgentRunner for CountingRunner {
        async fn run_task(&self, _task: &str, _session_id: &str) -> Result<String, AgentError> {
            let now = self.current.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_seen.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.current.fetch_sub(1, Ordering::SeqCst);
            Ok("ok".into())
        }
    }

    /// Fails any task whose text contains "BOOM"; succeeds otherwise.
    struct FlakyRunner;
    #[async_trait]
    impl SubAgentRunner for FlakyRunner {
        async fn run_task(&self, task: &str, _session_id: &str) -> Result<String, AgentError> {
            if task.contains("BOOM") {
                Err(AgentError::BudgetExhausted(7))
            } else {
                Ok(format!("done:{task}"))
            }
        }
    }

    struct DenyAll;
    #[async_trait]
    impl CommandApprover for DenyAll {
        async fn approve(&self, _: &str, _: &str, _: &str) -> ApprovalDecision {
            ApprovalDecision::Denied
        }
    }

    struct NopMemory;
    #[async_trait]
    impl MemoryStore for NopMemory {
        async fn read_memory(&self) -> Result<garudust_core::memory::MemoryContent, AgentError> {
            Ok(garudust_core::memory::MemoryContent::default())
        }
        async fn write_memory(
            &self,
            _: &garudust_core::memory::MemoryContent,
        ) -> Result<(), AgentError> {
            Ok(())
        }
        async fn read_user_profile(&self) -> Result<String, AgentError> {
            Ok(String::new())
        }
        async fn write_user_profile(&self, _: &str) -> Result<(), AgentError> {
            Ok(())
        }
    }

    fn ctx_with(cap: usize, runner: Arc<dyn SubAgentRunner>) -> ToolContext {
        let mut config = AgentConfig::default();
        config.max_concurrent_sub_agents = cap;
        ToolContext {
            session_id: "s".into(),
            conv_key: String::new(),
            user_id: String::new(),
            agent_id: "a".into(),
            iteration: 0,
            budget: Arc::new(IterationBudget::new(10)),
            memory: Arc::new(NopMemory),
            config: Arc::new(config),
            approver: Arc::new(DenyAll),
            sub_agent: Some(runner),
            skill_permissions: Arc::new(tokio::sync::RwLock::new(
                garudust_core::tool::SkillPermissions::default(),
            )),
            required_tools: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    fn tasks(n: usize) -> Value {
        let items: Vec<Value> = (0..n).map(|i| json!({ "task": format!("t{i}") })).collect();
        json!({ "tasks": items })
    }

    #[tokio::test]
    async fn cap_bounds_concurrent_sub_agents() {
        let max_seen = Arc::new(AtomicUsize::new(0));
        let runner = Arc::new(CountingRunner {
            current: AtomicUsize::new(0),
            max_seen: max_seen.clone(),
        });
        let ctx = ctx_with(2, runner);

        let out = DelegateTasks.execute(tasks(6), &ctx).await.unwrap();

        assert!(
            max_seen.load(Ordering::SeqCst) <= 2,
            "fan-out exceeded the cap"
        );
        assert!(
            out.content.contains("Task 6 result"),
            "all tasks must complete"
        );
    }

    #[tokio::test]
    async fn cap_zero_runs_all_at_once() {
        let max_seen = Arc::new(AtomicUsize::new(0));
        let runner = Arc::new(CountingRunner {
            current: AtomicUsize::new(0),
            max_seen: max_seen.clone(),
        });
        let ctx = ctx_with(0, runner);

        DelegateTasks.execute(tasks(5), &ctx).await.unwrap();

        assert_eq!(
            max_seen.load(Ordering::SeqCst),
            5,
            "0 means unlimited fan-out"
        );
    }

    #[tokio::test]
    async fn one_failure_keeps_other_results() {
        let ctx = ctx_with(4, Arc::new(FlakyRunner));
        let params = json!({
            "tasks": [
                { "task": "alpha" },
                { "task": "BOOM" },
                { "task": "gamma" },
            ]
        });

        // The whole call still succeeds despite the failing sub-agent.
        let out = DelegateTasks.execute(params, &ctx).await.unwrap();

        // Successful tasks keep their output, in order.
        assert!(out.content.contains("done:alpha"));
        assert!(out.content.contains("done:gamma"));
        // The failure is annotated in place, not dropped.
        assert!(out.content.contains("Task 2 result"));
        assert!(out.content.contains("[FAILED:"));
    }

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
