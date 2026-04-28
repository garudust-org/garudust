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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_registry_is_empty() {
        let r = SessionRegistry::new();
        assert_eq!(r.count().await, 0);
    }

    #[tokio::test]
    async fn touch_creates_new_session() {
        let r = SessionRegistry::new();
        r.touch("key1", "telegram", "user1").await;
        assert_eq!(r.count().await, 1);
    }

    #[tokio::test]
    async fn touch_same_key_does_not_duplicate() {
        let r = SessionRegistry::new();
        r.touch("key1", "telegram", "user1").await;
        r.touch("key1", "telegram", "user1").await;
        assert_eq!(r.count().await, 1);
    }

    #[tokio::test]
    async fn touch_different_keys_each_counted() {
        let r = SessionRegistry::new();
        r.touch("key1", "telegram", "user1").await;
        r.touch("key2", "discord", "user2").await;
        assert_eq!(r.count().await, 2);
    }

    #[tokio::test]
    async fn touch_updates_last_seen() {
        let r = SessionRegistry::new();
        r.touch("key1", "telegram", "user1").await;
        let first = r.sessions.read().await["key1"].last_seen;
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        r.touch("key1", "telegram", "user1").await;
        let second = r.sessions.read().await["key1"].last_seen;
        assert!(second >= first);
    }
}
