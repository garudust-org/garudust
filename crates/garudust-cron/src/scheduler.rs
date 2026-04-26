use std::sync::Arc;

use garudust_agent::Agent;
use garudust_core::tool::CommandApprover;
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct CronScheduler {
    inner: JobScheduler,
    agent: Arc<Agent>,
    approver: Arc<dyn CommandApprover>,
}

impl CronScheduler {
    pub async fn new(
        agent: Arc<Agent>,
        approver: Arc<dyn CommandApprover>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            inner: JobScheduler::new().await?,
            agent,
            approver,
        })
    }

    pub async fn add_job(&self, cron_expr: &str, task: String) -> anyhow::Result<()> {
        let agent = self.agent.clone();
        let approver = self.approver.clone();
        let job = Job::new_async(cron_expr, move |_uuid, _lock| {
            let agent = agent.clone();
            let approver = approver.clone();
            let task = task.clone();
            Box::pin(async move {
                tracing::info!(task = %task, "cron job starting");
                match agent.run(&task, approver, "cron").await {
                    Ok(result) => tracing::info!(
                        task = %task,
                        iterations = result.iterations,
                        "cron job completed"
                    ),
                    Err(e) => tracing::error!(task = %task, error = %e, "cron job failed"),
                }
            })
        })?;
        self.inner.add(job).await?;
        Ok(())
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        self.inner.start().await?;
        Ok(())
    }
}
