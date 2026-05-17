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
    fn name(&self) -> &str {
        "cron_create"
    }
    fn toolset(&self) -> &str {
        "cron"
    }
    fn description(&self) -> &str {
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
    fn name(&self) -> &str {
        "cron_list"
    }
    fn toolset(&self) -> &str {
        "cron"
    }
    fn description(&self) -> &str {
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
    fn name(&self) -> &str {
        "cron_delete"
    }
    fn toolset(&self) -> &str {
        "cron"
    }
    fn description(&self) -> &str {
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
