use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

use crate::migrations;

/// Persistent cache of LINE message texts, keyed by message id.
///
/// LINE webhooks carry only a `quotedMessageId` for replies — never the quoted
/// content — and there is no API to re-fetch a *text* message. The adapter
/// therefore records every text it sees (inbound webhooks and the bot's own
/// outbound sends) so quotes still resolve after a process restart, which an
/// in-memory map alone cannot survive.
///
/// Rows carry a `created_at` timestamp; the caller prunes by age via
/// [`MessageCacheStore::prune`].
pub struct MessageCacheStore {
    conn: Arc<Mutex<Connection>>,
}

impl MessageCacheStore {
    pub fn open(home_dir: &Path) -> anyhow::Result<Self> {
        let db_path = home_dir.join("state.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)?;
        // Several components hold their own connection to state.db; wait out
        // short write locks instead of surfacing SQLITE_BUSY to the webhook path.
        conn.busy_timeout(Duration::from_secs(5))?;
        // Idempotent; ensures the table exists even if this store opens the
        // database before any other component runs the migrations.
        migrations::run(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Returns the cached text for this message id, if present.
    pub fn get(&self, message_id: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT text FROM line_message_cache WHERE message_id = ?1",
            params![message_id],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    /// Stores (or replaces) the text for this message id, stamped now.
    pub fn put(&self, message_id: &str, text: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO line_message_cache (message_id, text, created_at)
             VALUES (?1, ?2, ?3)",
            params![message_id, text, now_epoch()],
        )?;
        Ok(())
    }

    /// Deletes rows older than `max_age_secs`. Returns the number removed.
    pub fn prune(&self, max_age_secs: u64) -> anyhow::Result<usize> {
        let cutoff = now_epoch() - max_age_secs as f64;
        let conn = self.conn.lock().unwrap();
        let removed = conn.execute(
            "DELETE FROM line_message_cache WHERE created_at < ?1",
            params![cutoff],
        )?;
        Ok(removed)
    }
}

fn now_epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_tmp() -> (MessageCacheStore, std::path::PathBuf) {
        let tmp = std::env::temp_dir().join(format!("garudust-msgcache-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        (MessageCacheStore::open(&tmp).unwrap(), tmp)
    }

    #[test]
    fn get_missing_returns_none() {
        let (store, _t) = open_tmp();
        assert_eq!(store.get("nope"), None);
    }

    #[test]
    fn put_then_get_roundtrips() {
        let (store, _t) = open_tmp();
        store.put("m1", "สวัสดีครับ").unwrap();
        assert_eq!(store.get("m1").as_deref(), Some("สวัสดีครับ"));
    }

    #[test]
    fn put_replaces_on_same_id() {
        let (store, _t) = open_tmp();
        store.put("m1", "old").unwrap();
        store.put("m1", "new").unwrap();
        assert_eq!(store.get("m1").as_deref(), Some("new"));
    }

    #[test]
    fn survives_reopen() {
        let (store, dir) = open_tmp();
        store.put("m1", "persist").unwrap();
        drop(store);
        let reopened = MessageCacheStore::open(&dir).unwrap();
        assert_eq!(reopened.get("m1").as_deref(), Some("persist"));
    }

    #[test]
    fn prune_removes_only_expired_rows() {
        let (store, _t) = open_tmp();
        store.put("fresh", "keep me").unwrap();
        // Backdate one row beyond the TTL cutoff.
        {
            let conn = store.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO line_message_cache (message_id, text, created_at)
                 VALUES ('stale', 'drop me', ?1)",
                params![now_epoch() - 100.0],
            )
            .unwrap();
        }
        let removed = store.prune(50).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.get("stale"), None);
        assert_eq!(store.get("fresh").as_deref(), Some("keep me"));
    }

    #[test]
    fn prune_on_empty_is_noop() {
        let (store, _t) = open_tmp();
        assert_eq!(store.prune(0).unwrap(), 0);
    }
}
