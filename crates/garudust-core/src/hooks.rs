use async_trait::async_trait;

/// Lifecycle callbacks for the agent run loop.
///
/// Implement this trait to observe or react to key moments without modifying
/// the agent core. All methods have default no-op implementations so you only
/// need to override the events you care about.
#[async_trait]
pub trait AgentHooks: Send + Sync + 'static {
    /// Called at the start of each LLM turn, after the iteration counter is incremented.
    async fn on_turn_start(&self, _iteration: u32, _session_id: &str) {}

    /// Called just before the agent returns its final output for a session.
    /// `output` is the raw response text, before the usage footer is appended.
    async fn on_session_end(&self, _output: &str, _session_id: &str) {}

    /// Called just before the context compression pass runs.
    /// `message_count` is the number of messages in the current history.
    async fn on_pre_compress(&self, _message_count: usize, _session_id: &str) {}

    /// Called when the agent spawns a sub-agent via `delegate_task` / `delegate_tasks`.
    async fn on_delegation(&self, _task: &str, _session_id: &str) {}
}

/// No-op hooks implementation used as the default when no hooks are configured.
pub struct NoopHooks;

#[async_trait]
impl AgentHooks for NoopHooks {}
