use std::sync::Arc;

use garudust_agent::Agent;
use garudust_core::config::AgentConfig;
use garudust_memory::SessionDb;

use crate::metrics::Metrics;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AgentConfig>,
    pub session_db: Arc<SessionDb>,
    pub agent: Arc<Agent>,
    pub metrics: Arc<Metrics>,
}
