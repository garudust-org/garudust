use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobInfo {
    pub label: String,
    pub schedule: String,
    pub task: String,
    pub created_at: i64,
}

#[async_trait]
pub trait CronManager: Send + Sync + 'static {
    async fn create_job(&self, label: &str, schedule: &str, task: &str) -> anyhow::Result<()>;
    async fn list_jobs(&self) -> Vec<CronJobInfo>;
    async fn delete_job(&self, label: &str) -> anyhow::Result<bool>;
}
