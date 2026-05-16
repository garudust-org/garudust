use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

pub struct DocStore {
    conn: Arc<Mutex<Connection>>,
}

pub struct SearchResult {
    pub file_name: String,
    pub path: String,
    pub chunk_idx: i64,
    pub content: String,
}

pub struct DocInfo {
    pub file_name: String,
    pub path: String,
    pub ingested_at: f64,
    pub chunk_count: i64,
}

impl DocStore {
    /// Open (or create) the RAG document store at `home_dir/state.db`.
    /// Shares the same SQLite file as `SessionDb`; tables are independent.
    pub fn open(home_dir: &Path) -> anyhow::Result<Self> {
        let db_path = home_dir.join("state.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)?;
        Self::migrate(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn migrate(conn: &Connection) -> rusqlite::Result<()> {
        // Idempotent — safe to run on every open.
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS doc_sources (
                id          TEXT PRIMARY KEY,
                path        TEXT NOT NULL UNIQUE,
                file_name   TEXT NOT NULL,
                ingested_at REAL NOT NULL,
                chunk_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS doc_chunks USING fts5(
                source_id UNINDEXED,
                chunk_idx UNINDEXED,
                content,
                tokenize = 'trigram'
            );
            ",
        )
    }

    /// Store `chunks` for `source_path`. Re-ingesting the same path replaces
    /// all previous chunks atomically.
    pub fn ingest(&self, source_path: &str, chunks: &[String]) -> anyhow::Result<()> {
        let source_id = uuid::Uuid::new_v4().to_string();
        let file_name = Path::new(source_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(source_path)
            .to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64();

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // Replace existing entry for this path
        let old_id: Option<String> = tx
            .query_row(
                "SELECT id FROM doc_sources WHERE path = ?1",
                params![source_path],
                |r| r.get(0),
            )
            .ok();
        if let Some(old) = old_id {
            tx.execute("DELETE FROM doc_chunks WHERE source_id = ?1", params![old])?;
            tx.execute("DELETE FROM doc_sources WHERE id = ?1", params![old])?;
        }

        tx.execute(
            "INSERT INTO doc_sources (id, path, file_name, ingested_at, chunk_count) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![source_id, source_path, file_name, now, chunks.len() as i64],
        )?;

        for (i, chunk) in chunks.iter().enumerate() {
            tx.execute(
                "INSERT INTO doc_chunks (source_id, chunk_idx, content) VALUES (?1, ?2, ?3)",
                params![source_id, i as i64, chunk],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// FTS5 full-text search across all ingested documents.
    /// `query` supports FTS5 syntax (AND, OR, NOT, "phrase").
    pub fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<SearchResult>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.file_name, s.path, c.chunk_idx, c.content
             FROM doc_chunks c
             JOIN doc_sources s ON s.id = c.source_id
             WHERE doc_chunks MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(SearchResult {
                    file_name: row.get(0)?,
                    path: row.get(1)?,
                    chunk_idx: row.get(2)?,
                    content: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    /// List all ingested documents, newest first.
    pub fn list(&self) -> anyhow::Result<Vec<DocInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT file_name, path, ingested_at, chunk_count \
             FROM doc_sources ORDER BY ingested_at DESC",
        )?;
        let docs = stmt
            .query_map([], |row| {
                Ok(DocInfo {
                    file_name: row.get(0)?,
                    path: row.get(1)?,
                    ingested_at: row.get(2)?,
                    chunk_count: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(docs)
    }

    /// Remove all chunks for `path`. Returns `true` if the document existed.
    pub fn forget(&self, path: &str) -> anyhow::Result<bool> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let id: Option<String> = tx
            .query_row(
                "SELECT id FROM doc_sources WHERE path = ?1",
                params![path],
                |r| r.get(0),
            )
            .ok();
        let Some(id) = id else {
            tx.commit()?;
            return Ok(false);
        };
        tx.execute("DELETE FROM doc_chunks WHERE source_id = ?1", params![id])?;
        tx.execute("DELETE FROM doc_sources WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(true)
    }
}
