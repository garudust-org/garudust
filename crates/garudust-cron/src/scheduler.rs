use std::sync::Arc;

use async_trait::async_trait;
use garudust_agent::Agent;
use garudust_core::{cron::CronJobInfo, tool::CommandApprover};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

struct CronJobEntry {
    uuid: Uuid,
    label: String,
    schedule: String,
    task: String,
    created_at: i64,
}

pub struct CronScheduler {
    inner: JobScheduler,
    agent: Arc<Agent>,
    approver: Arc<dyn CommandApprover>,
    jobs: Mutex<Vec<CronJobEntry>>,
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
            jobs: Mutex::new(Vec::new()),
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

#[async_trait]
impl garudust_core::cron::CronManager for CronScheduler {
    async fn create_job(&self, label: &str, schedule: &str, task: &str) -> anyhow::Result<()> {
        let mut jobs = self.jobs.lock().await;
        if jobs.iter().any(|j| j.label == label) {
            anyhow::bail!("a cron job with label '{label}' already exists; delete it first");
        }

        let agent = self.agent.clone();
        let approver = self.approver.clone();
        let task_str = task.to_string();
        let job = Job::new_async(schedule, move |_uuid, _lock| {
            let agent = agent.clone();
            let approver = approver.clone();
            let task = task_str.clone();
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

        let uuid = self.inner.add(job).await?;
        tracing::info!(label = %label, schedule = %schedule, "runtime cron job created");

        jobs.push(CronJobEntry {
            uuid,
            label: label.to_string(),
            schedule: schedule.to_string(),
            task: task.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        });

        Ok(())
    }

    async fn list_jobs(&self) -> Vec<CronJobInfo> {
        self.jobs
            .lock()
            .await
            .iter()
            .map(|e| CronJobInfo {
                label: e.label.clone(),
                schedule: e.schedule.clone(),
                task: e.task.clone(),
                created_at: e.created_at,
            })
            .collect()
    }

    async fn delete_job(&self, label: &str) -> anyhow::Result<bool> {
        let mut jobs = self.jobs.lock().await;
        if let Some(idx) = jobs.iter().position(|j| j.label == label) {
            let entry = jobs.remove(idx);
            self.inner
                .remove(&entry.uuid)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            tracing::info!(label = %label, "runtime cron job deleted");
            Ok(true)
        } else {
            Ok(false)
        }
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
