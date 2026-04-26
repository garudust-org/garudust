use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    budget::IterationBudget,
    config::AgentConfig,
    error::ToolError,
    memory::MemoryStore,
    types::{ToolResult, ToolSchema},
};

#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;
    fn toolset(&self) -> &str;

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

pub struct ToolContext {
    pub session_id: String,
    pub agent_id: String,
    pub iteration: u32,
    pub budget: Arc<IterationBudget>,
    pub memory: Arc<dyn MemoryStore>,
    pub config: Arc<AgentConfig>,
    pub approver: Arc<dyn CommandApprover>,
}

#[async_trait]
pub trait CommandApprover: Send + Sync + 'static {
    async fn approve(&self, command: &str, description: &str) -> ApprovalDecision;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalDecision {
    Approved,
    ApprovedAlways,
    Denied,
    Yolo,
}
