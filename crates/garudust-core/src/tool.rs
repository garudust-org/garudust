use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    budget::IterationBudget,
    config::AgentConfig,
    error::{AgentError, ToolError},
    memory::MemoryStore,
    types::{ToolResult, ToolSchema},
};

/// Accumulated permissions from all skills loaded in the current session.
/// Each entry maps a tool name to `true` (allowed) or `false` (denied).
/// Union semantics: `true` from any skill wins over `false` from another.
/// Tools absent from the map are not restricted by skill permissions.
#[derive(Debug, Default, Clone)]
pub struct SkillPermissions(pub HashMap<String, bool>);

impl SkillPermissions {
    /// Merge another skill's permissions using union semantics (allow wins).
    pub fn merge(&mut self, other: &HashMap<String, bool>) {
        for (tool, allowed) in other {
            let entry = self.0.entry(tool.clone()).or_insert(false);
            if *allowed {
                *entry = true;
            }
        }
    }

    /// Returns `Some(false)` only if the tool is explicitly denied by every
    /// loaded skill that mentions it. Returns `None` if no skill restricts it.
    pub fn check(&self, tool_name: &str) -> Option<bool> {
        self.0.get(tool_name).copied()
    }
}

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

    /// Parameter-aware variant called by the registry. Override this when
    /// destructiveness depends on the specific arguments (e.g. a terminal tool
    /// that can also run read-only commands). The default delegates to
    /// `is_destructive()` so existing tools need no changes.
    fn is_destructive_for(&self, _params: &serde_json::Value) -> bool {
        self.is_destructive()
    }

    /// Return `true` to opt out of the agent's global `tool_timeout_secs` budget.
    /// Override on tools that manage their own timeout internally (e.g. `Terminal`).
    fn bypass_dispatch_timeout(&self) -> bool {
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
    /// Accumulated permissions from all skills loaded this session via skill_view.
    /// Shared across all tool dispatches within the same agent turn.
    pub skill_permissions: Arc<RwLock<SkillPermissions>>,
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
