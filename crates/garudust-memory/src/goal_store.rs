use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Persists an active goal per session to `~/.garudust/goals/{hash}`.
/// The agent injects the goal into every turn so it is never forgotten
/// across a long conversation.
pub struct GoalStore {
    dir: PathBuf,
}

impl GoalStore {
    pub fn new(home_dir: &Path) -> Self {
        Self {
            dir: home_dir.join("goals"),
        }
    }

    fn path(&self, session_key: &str) -> PathBuf {
        let mut h = DefaultHasher::new();
        session_key.hash(&mut h);
        self.dir.join(format!("{:016x}", h.finish()))
    }

    pub async fn get(&self, session_key: &str) -> Option<String> {
        tokio::fs::read_to_string(self.path(session_key))
            .await
            .ok()
            .filter(|s| !s.trim().is_empty())
    }

    pub async fn set(&self, session_key: &str, goal: &str) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.dir).await?;
        tokio::fs::write(self.path(session_key), goal).await?;
        Ok(())
    }

    pub async fn clear(&self, session_key: &str) {
        let _ = tokio::fs::remove_file(self.path(session_key)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store() -> (GoalStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = GoalStore::new(dir.path());
        (store, dir)
    }

    #[tokio::test]
    async fn set_get_clear() {
        let (store, _dir) = tmp_store();
        assert!(store.get("sess1").await.is_none());
        store.set("sess1", "finish the report").await.unwrap();
        assert_eq!(store.get("sess1").await.unwrap(), "finish the report");
        store.clear("sess1").await;
        assert!(store.get("sess1").await.is_none());
    }

    #[tokio::test]
    async fn different_sessions_are_isolated() {
        let (store, _dir) = tmp_store();
        store.set("sessA", "goal A").await.unwrap();
        store.set("sessB", "goal B").await.unwrap();
        assert_eq!(store.get("sessA").await.unwrap(), "goal A");
        assert_eq!(store.get("sessB").await.unwrap(), "goal B");
        store.clear("sessA").await;
        assert!(store.get("sessA").await.is_none());
        assert_eq!(store.get("sessB").await.unwrap(), "goal B");
    }
}
