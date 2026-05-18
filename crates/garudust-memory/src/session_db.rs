use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

use crate::migrations;

/// Attempt WAL journal mode; fall back to DELETE (the SQLite default) on
/// filesystems that do not support shared-memory files (NFS, SMB, some tmpfs).
fn enable_wal_or_fallback(conn: &Connection) {
    match conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get::<_, String>(0)) {
        Ok(mode) if mode == "wal" => {}
        Ok(mode) => {
            tracing::warn!(
                actual_mode = %mode,
                "WAL journal mode unavailable (possibly NFS/SMB); using fallback mode"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "PRAGMA journal_mode=WAL failed; continuing without WAL");
        }
    }
}

/// A task that was in-flight when the server last shut down unexpectedly.
/// Returned by [`SessionDb::drain_tasks`] on startup so the gateway can replay it.
pub struct PendingTask {
    pub id: String,
    pub session_key: String,
    pub platform: String,
    pub chat_id: String,
    pub task: String,
    pub hint: Option<String>,
}

pub struct SessionDb {
    conn: Arc<Mutex<Connection>>,
}

impl SessionDb {
    pub fn open(home_dir: &Path) -> anyhow::Result<Self> {
        let db_path = home_dir.join("state.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)?;
        enable_wal_or_fallback(&conn);
        migrations::run(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_session(
        &self,
        id: &str,
        source: &str,
        model: &str,
        started_at: f64,
        ended_at: f64,
        input_tokens: u32,
        output_tokens: u32,
        message_count: u32,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sessions
             (id, source, model, started_at, ended_at, input_tokens, output_tokens, message_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                source,
                model,
                started_at,
                ended_at,
                input_tokens,
                output_tokens,
                message_count
            ],
        )?;
        Ok(())
    }

    pub fn append_messages(
        &self,
        session_id: &str,
        messages: &[(String, String, String, f64)], // (id, role, content_json, created_at)
    ) -> anyhow::Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        for (id, role, content, created_at) in messages {
            tx.execute(
                "INSERT OR IGNORE INTO messages (id, session_id, role, content, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, session_id, role, content, created_at],
            )?;
            // FTS5 index is kept in sync by the messages_ai / messages_ad triggers.
        }
        tx.commit()?;
        Ok(())
    }

    /// Record a task as in-flight before spawning the agent run.
    /// Call [`finish_task`] when the run completes (success or error).
    pub fn begin_task(
        &self,
        id: &str,
        session_key: &str,
        platform: &str,
        chat_id: &str,
        task: &str,
        hint: Option<&str>,
    ) -> anyhow::Result<()> {
        #[allow(clippy::cast_precision_loss)]
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as f64
            / 1000.0;
        self.conn.lock().unwrap().execute(
            "INSERT OR IGNORE INTO pending_tasks \
             (id, session_key, platform, chat_id, task, hint, created_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, session_key, platform, chat_id, task, hint, now],
        )?;
        Ok(())
    }

    /// Remove a pending task — call after the agent run finishes (whether success or error).
    pub fn finish_task(&self, id: &str) -> anyhow::Result<()> {
        self.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM pending_tasks WHERE id=?1", [id])?;
        Ok(())
    }

    /// Return and delete all pending tasks for `platform`.
    /// Called on startup to replay tasks that were interrupted by a crash/restart.
    pub fn drain_tasks(&self, platform: &str) -> anyhow::Result<Vec<PendingTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_key, platform, chat_id, task, hint \
             FROM pending_tasks WHERE platform=?1 ORDER BY created_at",
        )?;
        let tasks: Vec<PendingTask> = stmt
            .query_map([platform], |row| {
                Ok(PendingTask {
                    id: row.get(0)?,
                    session_key: row.get(1)?,
                    platform: row.get(2)?,
                    chat_id: row.get(3)?,
                    task: row.get(4)?,
                    hint: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        if !tasks.is_empty() {
            conn.execute("DELETE FROM pending_tasks WHERE platform=?1", [platform])?;
        }
        Ok(tasks)
    }

    pub fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT content FROM messages_fts WHERE messages_fts MATCH ?1 LIMIT ?2")?;
        let rows = stmt.query_map([query, &limit.to_string()], |row| row.get(0))?;
        rows.collect::<Result<Vec<String>, _>>().map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_in_memory() -> SessionDb {
        let tmp = std::env::temp_dir().join(format!("garudust-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        SessionDb::open(&tmp).unwrap()
    }

    #[test]
    fn save_and_retrieve_session() {
        let db = open_in_memory();
        db.save_session("s1", "test", "model-x", 1.0, 2.0, 10, 20, 3)
            .unwrap();

        // Second save with same id should replace (no unique constraint error)
        db.save_session("s1", "test", "model-x", 1.0, 3.0, 10, 25, 4)
            .unwrap();
    }

    #[test]
    fn append_and_search_messages() {
        let db = open_in_memory();
        db.save_session("s1", "test", "gpt", 0.0, 1.0, 0, 0, 1)
            .unwrap();

        let msg_id = uuid::Uuid::new_v4().to_string();
        db.append_messages(
            "s1",
            &[(msg_id, "user".into(), "hello garudust world".into(), 0.0)],
        )
        .unwrap();

        let results = db.search("garudust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("garudust"));
    }

    #[test]
    fn search_returns_empty_for_no_match() {
        let db = open_in_memory();
        db.save_session("s1", "test", "gpt", 0.0, 1.0, 0, 0, 1)
            .unwrap();
        db.append_messages(
            "s1",
            &[(
                uuid::Uuid::new_v4().to_string(),
                "user".into(),
                "hello world".into(),
                0.0,
            )],
        )
        .unwrap();

        let results = db.search("zzznomatch", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn duplicate_message_id_is_ignored() {
        let db = open_in_memory();
        db.save_session("s1", "test", "gpt", 0.0, 1.0, 0, 0, 1)
            .unwrap();
        let msg = (
            "fixed-id".to_string(),
            "user".to_string(),
            "unique content here".to_string(),
            0.0f64,
        );
        db.append_messages("s1", std::slice::from_ref(&msg))
            .unwrap();
        db.append_messages("s1", &[msg]).unwrap(); // should not error or duplicate

        let results = db.search("unique", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_respects_limit() {
        let db = open_in_memory();
        db.save_session("s1", "test", "gpt", 0.0, 1.0, 0, 0, 5)
            .unwrap();
        let messages: Vec<_> = (0..5)
            .map(|i| {
                (
                    uuid::Uuid::new_v4().to_string(),
                    "user".to_string(),
                    format!("searchterm entry number {i}"),
                    0.0f64,
                )
            })
            .collect();
        db.append_messages("s1", &messages).unwrap();

        let results = db.search("searchterm", 3).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn trigram_substring_search() {
        let db = open_in_memory();
        db.save_session("s1", "test", "gpt", 0.0, 1.0, 0, 0, 1)
            .unwrap();
        db.append_messages(
            "s1",
            &[(
                uuid::Uuid::new_v4().to_string(),
                "user".into(),
                "The Pythagorean theorem is fundamental to geometry".into(),
                0.0,
            )],
        )
        .unwrap();

        // Trigram tokenizer enables substring matches without full-word boundaries.
        let results = db.search("pythag", 10).unwrap();
        assert_eq!(
            results.len(),
            1,
            "trigram should match 'pythag' inside 'Pythagorean'"
        );
    }
}
