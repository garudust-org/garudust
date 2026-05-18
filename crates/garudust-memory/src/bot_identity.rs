use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection, OptionalExtension};

use crate::migrations;

/// Persistent cache for a chat-platform bot's own user id.
///
/// The id (e.g. a LINE bot's `userId`) is immutable for the lifetime of a
/// channel token, so it is fetched from the platform API at most once ever and
/// then read straight from `state.db` on every subsequent start — a transient
/// network failure at restart can no longer disable mention detection.
///
/// Rows are keyed by a hash of the channel token: rotating the token yields a
/// new key and a one-time re-fetch, while the stale row is harmlessly ignored.
pub struct BotIdentityStore {
    conn: Arc<Mutex<Connection>>,
}

impl BotIdentityStore {
    pub fn open(home_dir: &Path) -> anyhow::Result<Self> {
        let db_path = home_dir.join("state.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)?;
        // Idempotent; ensures the table exists even if this store opens the
        // database before any other component runs the migrations.
        migrations::run(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Returns the cached bot user id for this token hash, if present.
    pub fn get(&self, token_hash: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT bot_user_id FROM line_bot_identity WHERE token_hash = ?1",
            params![token_hash],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    /// Stores (or replaces) the bot user id for this token hash.
    pub fn put(&self, token_hash: &str, bot_user_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO line_bot_identity (token_hash, bot_user_id)
             VALUES (?1, ?2)",
            params![token_hash, bot_user_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_tmp() -> (BotIdentityStore, std::path::PathBuf) {
        let tmp = std::env::temp_dir().join(format!("garudust-botid-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        (BotIdentityStore::open(&tmp).unwrap(), tmp)
    }

    #[test]
    fn get_missing_returns_none() {
        let (store, _t) = open_tmp();
        assert_eq!(store.get("nope"), None);
    }

    #[test]
    fn put_then_get_roundtrips() {
        let (store, _t) = open_tmp();
        store.put("hash-a", "Udeadbeef").unwrap();
        assert_eq!(store.get("hash-a").as_deref(), Some("Udeadbeef"));
    }

    #[test]
    fn put_replaces_on_same_hash() {
        let (store, _t) = open_tmp();
        store.put("hash-a", "Uold").unwrap();
        store.put("hash-a", "Unew").unwrap();
        assert_eq!(store.get("hash-a").as_deref(), Some("Unew"));
    }

    #[test]
    fn distinct_hashes_are_independent() {
        let (store, _t) = open_tmp();
        store.put("hash-a", "Ua").unwrap();
        store.put("hash-b", "Ub").unwrap();
        assert_eq!(store.get("hash-a").as_deref(), Some("Ua"));
        assert_eq!(store.get("hash-b").as_deref(), Some("Ub"));
    }

    #[test]
    fn survives_reopen() {
        let (store, dir) = open_tmp();
        store.put("hash-a", "Upersist").unwrap();
        drop(store);
        let reopened = BotIdentityStore::open(&dir).unwrap();
        assert_eq!(reopened.get("hash-a").as_deref(), Some("Upersist"));
    }
}
