use tokio_cron_scheduler::{Job, JobScheduler};

pub struct CronScheduler {
    inner: JobScheduler,
}

impl CronScheduler {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            inner: JobScheduler::new().await?,
        })
    }

    pub async fn add_job(&self, cron_expr: &str, task: String) -> anyhow::Result<()> {
        let job = Job::new_async(cron_expr, move |_uuid, _lock| {
            let task = task.clone();
            Box::pin(async move {
                tracing::info!("Running cron job: {}", task);
                // TODO: spawn agent with task
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
