use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub key: String,
    pub platform: String,
    pub user_id: String,
    pub started_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

pub struct SessionRegistry {
    sessions: RwLock<HashMap<String, SessionMeta>>,
}

impl SessionRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            sessions: RwLock::new(HashMap::new()),
        })
    }

    pub async fn touch(&self, key: &str, platform: &str, user_id: &str) {
        let now = Utc::now();
        let mut map = self.sessions.write().await;
        map.entry(key.to_string())
            .and_modify(|s| s.last_seen = now)
            .or_insert(SessionMeta {
                key: key.to_string(),
                platform: platform.to_string(),
                user_id: user_id.to_string(),
                started_at: now,
                last_seen: now,
            });
    }

    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }
}
