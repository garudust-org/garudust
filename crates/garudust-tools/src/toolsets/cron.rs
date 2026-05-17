use std::sync::Arc;

use async_trait::async_trait;
use garudust_core::{
    cron::CronManager,
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde_json::{json, Value};
use tokio::sync::Mutex;

type CronSlot = Arc<Mutex<Option<Arc<dyn CronManager>>>>;

pub struct CronCreate {
    pub slot: CronSlot,
}

pub struct CronList {
    pub slot: CronSlot,
}

pub struct CronDelete {
    pub slot: CronSlot,
}

async fn get_manager(slot: &CronSlot) -> Result<Arc<dyn CronManager>, ToolError> {
    slot.lock()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| ToolError::Execution("cron scheduler not available".into()))
}

#[async_trait]
impl Tool for CronCreate {
    fn name(&self) -> &'static str {
        "cron_create"
    }
    fn toolset(&self) -> &'static str {
        "cron"
    }
    fn description(&self) -> &'static str {
        "Schedule a recurring autonomous task. The agent will execute the task text on the given \
         cron schedule. Uses 6-field cron syntax: sec min hour day_of_month month day_of_week \
         (e.g. '0 30 8 * * *' = every day at 08:30). Runtime jobs are not persisted across \
         server restarts; add them to config.yaml for permanent schedules."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "label": {
                    "type": "string",
                    "description": "Unique short name for this job (e.g. 'morning_news')"
                },
                "schedule": {
                    "type": "string",
                    "description": "6-field cron expression: sec min hour dom month dow"
                },
                "task": {
                    "type": "string",
                    "description": "Task instruction the agent will execute on each firing"
                }
            },
            "required": ["label", "schedule", "task"]
        })
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let label = params["label"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("'label' required".into()))?;
        let schedule = params["schedule"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("'schedule' required".into()))?;
        let task = params["task"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("'task' required".into()))?;

        let mgr = get_manager(&self.slot).await?;
        mgr.create_job(label, schedule, task)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolResult::ok(
            "",
            format!("Cron job '{label}' created with schedule: {schedule}"),
        ))
    }
}

#[async_trait]
impl Tool for CronList {
    fn name(&self) -> &'static str {
        "cron_list"
    }
    fn toolset(&self) -> &'static str {
        "cron"
    }
    fn description(&self) -> &'static str {
        "List all active runtime cron jobs (created via cron_create). \
         Config-file jobs defined in config.yaml are not shown here."
    }
    fn schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, _params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let mgr = get_manager(&self.slot).await?;
        let jobs = mgr.list_jobs().await;

        if jobs.is_empty() {
            return Ok(ToolResult::ok("", "No active runtime cron jobs."));
        }

        let lines = jobs
            .iter()
            .map(|j| {
                let ts = chrono::DateTime::from_timestamp(j.created_at, 0).map_or_else(
                    || j.created_at.to_string(),
                    |dt| dt.format("%Y-%m-%d %H:%M UTC").to_string(),
                );
                format!(
                    "- [{}]  schedule: {}  task: {}  created: {}",
                    j.label, j.schedule, j.task, ts
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolResult::ok("", lines))
    }
}

#[async_trait]
impl Tool for CronDelete {
    fn name(&self) -> &'static str {
        "cron_delete"
    }
    fn toolset(&self) -> &'static str {
        "cron"
    }
    fn description(&self) -> &'static str {
        "Remove a runtime cron job by its label. Has no effect on config-file jobs."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "label": {
                    "type": "string",
                    "description": "Label of the job to remove"
                }
            },
            "required": ["label"]
        })
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let label = params["label"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("'label' required".into()))?;

        let mgr = get_manager(&self.slot).await?;
        let removed = mgr
            .delete_job(label)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        if removed {
            Ok(ToolResult::ok("", format!("Cron job '{label}' removed.")))
        } else {
            Ok(ToolResult::ok(
                "",
                format!("No cron job found with label '{label}'."),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use garudust_core::{
        budget::IterationBudget,
        config::AgentConfig,
        cron::{CronJobInfo, CronManager},
        tool::{ApprovalDecision, CommandApprover, SkillPermissions, ToolContext},
    };
    use serde_json::json;
    use tokio::sync::{Mutex, RwLock};

    use super::*;

    // ── stubs ─────────────────────────────────────────────────────────────────

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
        ToolContext {
            session_id: "test".into(),
            conv_key: String::new(),
            agent_id: "test".into(),
            iteration: 0,
            budget: Arc::new(IterationBudget::new(10)),
            memory: Arc::new(NopMemory),
            config: Arc::new(AgentConfig::default()),
            approver: Arc::new(AutoApprove),
            sub_agent: None,
            skill_permissions: Arc::new(RwLock::new(SkillPermissions::default())),
            required_tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // ── mock CronManager ─────────────────────────────────────────────────────

    struct MockCron {
        jobs: std::sync::Mutex<Vec<CronJobInfo>>,
    }

    impl MockCron {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                jobs: std::sync::Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl CronManager for MockCron {
        async fn create_job(&self, label: &str, schedule: &str, task: &str) -> anyhow::Result<()> {
            let mut jobs = self.jobs.lock().unwrap();
            if jobs.iter().any(|j| j.label == label) {
                anyhow::bail!("label '{label}' already exists");
            }
            jobs.push(CronJobInfo {
                label: label.to_string(),
                schedule: schedule.to_string(),
                task: task.to_string(),
                created_at: 0,
            });
            Ok(())
        }

        async fn list_jobs(&self) -> Vec<CronJobInfo> {
            self.jobs.lock().unwrap().clone()
        }

        async fn delete_job(&self, label: &str) -> anyhow::Result<bool> {
            let mut jobs = self.jobs.lock().unwrap();
            if let Some(idx) = jobs.iter().position(|j| j.label == label) {
                jobs.remove(idx);
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    fn make_slot(mgr: Option<Arc<dyn CronManager>>) -> CronSlot {
        Arc::new(Mutex::new(mgr))
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn empty_slot_returns_error() {
        let tool = CronCreate {
            slot: make_slot(None),
        };
        let result = tool
            .execute(
                json!({"label":"x","schedule":"0 * * * * *","task":"t"}),
                &make_ctx(),
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not available"));
    }

    #[tokio::test]
    async fn create_then_list_shows_job() {
        let slot = make_slot(Some(MockCron::new()));
        let ctx = make_ctx();

        CronCreate { slot: slot.clone() }
            .execute(
                json!({"label":"morning","schedule":"0 30 8 * * *","task":"do something"}),
                &ctx,
            )
            .await
            .expect("create should succeed");

        let out = CronList { slot }
            .execute(json!({}), &ctx)
            .await
            .expect("list should succeed")
            .content;

        assert!(out.contains("morning"), "label missing from list: {out}");
        assert!(out.contains("0 30 8 * * *"), "schedule missing: {out}");
    }

    #[tokio::test]
    async fn duplicate_label_returns_error() {
        let slot = make_slot(Some(MockCron::new()));
        let ctx = make_ctx();

        CronCreate { slot: slot.clone() }
            .execute(
                json!({"label":"dup","schedule":"0 * * * * *","task":"t"}),
                &ctx,
            )
            .await
            .expect("first create should succeed");

        let result = CronCreate { slot }
            .execute(
                json!({"label":"dup","schedule":"0 * * * * *","task":"t2"}),
                &ctx,
            )
            .await;
        assert!(result.is_err(), "duplicate label must fail");
    }

    #[tokio::test]
    async fn delete_existing_job_removes_it() {
        let slot = make_slot(Some(MockCron::new()));
        let ctx = make_ctx();

        CronCreate { slot: slot.clone() }
            .execute(
                json!({"label":"bye","schedule":"0 * * * * *","task":"t"}),
                &ctx,
            )
            .await
            .unwrap();

        let del = CronDelete { slot: slot.clone() }
            .execute(json!({"label":"bye"}), &ctx)
            .await
            .expect("delete should succeed")
            .content;
        assert!(del.contains("removed"), "unexpected message: {del}");

        let list = CronList { slot }
            .execute(json!({}), &ctx)
            .await
            .unwrap()
            .content;
        assert!(!list.contains("bye"), "deleted job still visible: {list}");
    }

    #[tokio::test]
    async fn delete_nonexistent_label_not_found() {
        let slot = make_slot(Some(MockCron::new()));
        let out = CronDelete { slot }
            .execute(json!({"label":"ghost"}), &make_ctx())
            .await
            .expect("delete of unknown label should not error")
            .content;
        assert!(
            out.contains("No cron job found"),
            "expected not-found message, got: {out}"
        );
    }

    #[tokio::test]
    async fn list_empty_manager_reports_no_jobs() {
        let slot = make_slot(Some(MockCron::new()));
        let out = CronList { slot }
            .execute(json!({}), &make_ctx())
            .await
            .expect("list should succeed")
            .content;
        assert!(out.contains("No active"), "unexpected: {out}");
    }
}
