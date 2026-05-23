use std::sync::Arc;

use arc_swap::ArcSwap;
use garudust_agent::Agent;
use garudust_core::{config::AgentConfig, platform::PlatformAdapter, tool::CommandApprover};
use garudust_memory::SessionDb;

use crate::metrics::Metrics;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AgentConfig>,
    pub session_db: Arc<SessionDb>,
    pub agent: Arc<ArcSwap<Agent>>,
    pub metrics: Arc<Metrics>,
    pub approver: Arc<dyn CommandApprover>,
    /// Platform adapters whose health is surfaced via `/health`.
    pub platform_adapters: Vec<Arc<dyn PlatformAdapter>>,
}
