use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

use crate::migrations;

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
            let affected = tx.execute(
                "INSERT OR IGNORE INTO messages (id, session_id, role, content, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, session_id, role, content, created_at],
            )?;
            if affected > 0 {
                let rowid = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO messages_fts(rowid, content) VALUES (?1, ?2)",
                    params![rowid, content],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT content FROM messages_fts WHERE messages_fts MATCH ?1 LIMIT ?2")?;
        let rows = stmt.query_map([query, &limit.to_string()], |row| row.get(0))?;
        rows.collect::<Result<Vec<String>, _>>().map_err(Into::into)
    }
}
