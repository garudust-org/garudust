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
                match agent.run(&task, approver, "cron", None, None).await {
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

    /// Add a non-agent cron job — runs an arbitrary async closure on schedule.
    /// Useful for maintenance tasks (e.g. memory expiry) that don't need an LLM.
    pub async fn add_fn_job<F, Fut>(&self, cron_expr: &str, f: F) -> anyhow::Result<()>
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let job = Job::new_async(cron_expr, move |_, _| {
            let fut = f();
            Box::pin(fut)
        })?;
        self.inner.add(job).await?;
        Ok(())
    }

    pub fn inner_ref(&self) -> &JobScheduler {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use tokio_cron_scheduler::JobScheduler;

    #[tokio::test]
    async fn new_creates_scheduler_without_error() {
        let sched = JobScheduler::new().await;
        assert!(sched.is_ok());
    }

    #[tokio::test]
    async fn fn_job_fires_on_schedule() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut sched = JobScheduler::new().await.unwrap();
        let counter_clone = counter.clone();
        let job = tokio_cron_scheduler::Job::new_async("1/1 * * * * *", move |_, _| {
            let c = counter_clone.clone();
            Box::pin(async move {
                c.fetch_add(1, Ordering::SeqCst);
            })
        })
        .unwrap();
        sched.add(job).await.unwrap();
        sched.start().await.unwrap();
        // tokio_cron_scheduler drives its own system clock, so pause/advance
        // won't work here. We sleep real wall-clock time and accept the ~2 s cost.
        // Under heavy CI load this can flake if the OS delays scheduling.
        tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
        sched.shutdown().await.unwrap();
        assert!(
            counter.load(Ordering::SeqCst) >= 2,
            "expected ≥2 firings, got {}",
            counter.load(Ordering::SeqCst)
        );
    }

    #[test]
    fn invalid_cron_expression_returns_error() {
        let result = tokio_cron_scheduler::Job::new_async("not-a-cron", |_, _| Box::pin(async {}));
        assert!(result.is_err(), "invalid cron expression must fail");
    }

    #[test]
    fn parse_empty_returns_empty() {
        assert!(crate::parse_job_pairs("").is_empty());
    }

    #[test]
    fn parse_task_with_equals_sign() {
        let jobs = crate::parse_job_pairs("0 * * * *=key=value");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].1, "key=value");
    }
}
