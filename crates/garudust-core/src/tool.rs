use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    budget::IterationBudget,
    config::AgentConfig,
    error::{AgentError, ToolError},
    memory::MemoryStore,
    types::{ToolResult, ToolSchema},
};

#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;
    fn toolset(&self) -> &str;

    /// Returns true for tools that write, delete, or execute — i.e. operations
    /// that are hard to reverse. The registry uses this to gate approval and
    /// emit an audit-log entry before dispatch, regardless of how the tool
    /// encodes its arguments internally.
    fn is_destructive(&self) -> bool {
        false
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError>;

    fn to_schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.schema(),
        }
    }
}

#[async_trait]
pub trait SubAgentRunner: Send + Sync + 'static {
    async fn run_task(&self, task: &str, session_id: &str) -> Result<String, AgentError>;
}

pub struct ToolContext {
    pub session_id: String,
    pub agent_id: String,
    pub iteration: u32,
    pub budget: Arc<IterationBudget>,
    pub memory: Arc<dyn MemoryStore>,
    pub config: Arc<AgentConfig>,
    pub approver: Arc<dyn CommandApprover>,
    pub sub_agent: Option<Arc<dyn SubAgentRunner>>,
}

#[async_trait]
pub trait CommandApprover: Send + Sync + 'static {
    /// Called by ToolRegistry::dispatch() for every destructive tool before
    /// execute(). `tool_name` is the registered tool name; `params` is the
    /// JSON-serialised parameter object passed by the model.
    async fn approve(&self, tool_name: &str, params: &str) -> ApprovalDecision;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalDecision {
    Approved,
    ApprovedAlways,
    Denied,
    Yolo,
}
