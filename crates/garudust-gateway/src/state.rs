use std::sync::Arc;

use garudust_core::config::AgentConfig;
use garudust_memory::SessionDb;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AgentConfig>,
    pub session_db: Arc<SessionDb>,
}
