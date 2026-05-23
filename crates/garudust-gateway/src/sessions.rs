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

    /// Remove sessions whose `last_seen` is older than `max_age`.
    /// Returns the session keys that were evicted so callers can clean up agent state.
    pub async fn cleanup_idle(&self, max_age: std::time::Duration) -> Vec<String> {
        let cutoff = Utc::now()
            - chrono::Duration::from_std(max_age).unwrap_or(chrono::Duration::seconds(3600));
        let mut map = self.sessions.write().await;
        let expired: Vec<String> = map
            .iter()
            .filter(|(_, s)| s.last_seen < cutoff)
            .map(|(k, _)| k.clone())
            .collect();
        for key in &expired {
            map.remove(key);
        }
        expired
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

    // ── cleanup_idle ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn cleanup_idle_removes_old_sessions() {
        let r = SessionRegistry::new();
        r.touch("old", "telegram", "u1").await;
        r.touch("fresh", "telegram", "u2").await;

        // Back-date "old" by writing directly into the map
        {
            let mut map = r.sessions.write().await;
            map.get_mut("old").unwrap().last_seen = Utc::now() - chrono::Duration::hours(2);
        }

        let evicted = r.cleanup_idle(std::time::Duration::from_secs(3600)).await;
        assert_eq!(evicted, vec!["old".to_string()]);
        assert_eq!(r.count().await, 1);
    }

    #[tokio::test]
    async fn cleanup_idle_keeps_recent_sessions() {
        let r = SessionRegistry::new();
        r.touch("key1", "discord", "u1").await;
        r.touch("key2", "discord", "u2").await;

        let evicted = r.cleanup_idle(std::time::Duration::from_secs(3600)).await;
        assert!(evicted.is_empty());
        assert_eq!(r.count().await, 2);
    }

    #[tokio::test]
    async fn cleanup_idle_zero_duration_evicts_all() {
        let r = SessionRegistry::new();
        r.touch("a", "slack", "u1").await;
        r.touch("b", "slack", "u2").await;

        // Back-date both sessions so last_seen is strictly before cutoff even
        // when max_age = 0 (cutoff = Utc::now(), strict `<` comparison).
        {
            let mut map = r.sessions.write().await;
            let past = Utc::now() - chrono::Duration::milliseconds(100);
            map.get_mut("a").unwrap().last_seen = past;
            map.get_mut("b").unwrap().last_seen = past;
        }

        let evicted = r.cleanup_idle(std::time::Duration::ZERO).await;
        assert_eq!(evicted.len(), 2);
        assert_eq!(r.count().await, 0);
    }
}
