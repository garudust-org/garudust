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
        // Base schema — idempotent.
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS doc_sources (
                id          TEXT PRIMARY KEY,
                path        TEXT NOT NULL,
                file_name   TEXT NOT NULL,
                ingested_at REAL NOT NULL,
                chunk_count INTEGER NOT NULL DEFAULT 0,
                session_key TEXT NOT NULL DEFAULT ''
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS doc_chunks USING fts5(
                source_id UNINDEXED,
                chunk_idx UNINDEXED,
                content,
                tokenize = 'trigram'
            );
            ",
        )?;

        // Add session_key column to existing databases that pre-date this
        // migration.  SQLite returns an error when the column already exists;
        // we swallow that specific error so this stays idempotent.
        let _ = conn.execute_batch(
            "ALTER TABLE doc_sources ADD COLUMN session_key TEXT NOT NULL DEFAULT '';",
        );

        // Drop the old UNIQUE constraint on path alone (if the table was
        // created without session_key).  SQLite cannot drop constraints
        // directly, so we leave old rows in place; duplicate-path handling
        // is now scoped to (session_key, path) pairs via the SELECT before INSERT.
        Ok(())
    }

    /// Store `chunks` for `source_path` scoped to `session_key`.
    /// Re-ingesting the same (session, path) pair replaces all previous chunks atomically.
    pub fn ingest(
        &self,
        session_key: &str,
        source_path: &str,
        chunks: &[String],
    ) -> anyhow::Result<()> {
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

        // Replace existing entry for this (session, path) pair.
        let old_id: Option<String> = tx
            .query_row(
                "SELECT id FROM doc_sources WHERE session_key = ?1 AND path = ?2",
                params![session_key, source_path],
                |r| r.get(0),
            )
            .ok();
        if let Some(old) = old_id {
            tx.execute("DELETE FROM doc_chunks WHERE source_id = ?1", params![old])?;
            tx.execute("DELETE FROM doc_sources WHERE id = ?1", params![old])?;
        }

        tx.execute(
            "INSERT INTO doc_sources (id, path, file_name, ingested_at, chunk_count, session_key) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                source_id,
                source_path,
                file_name,
                now,
                i64::try_from(chunks.len()).unwrap_or(i64::MAX),
                session_key,
            ],
        )?;

        for (i, chunk) in chunks.iter().enumerate() {
            tx.execute(
                "INSERT INTO doc_chunks (source_id, chunk_idx, content) VALUES (?1, ?2, ?3)",
                params![source_id, i64::try_from(i).unwrap_or(0), chunk],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// FTS5 full-text search scoped to `session_key`.
    /// `query` supports FTS5 syntax (AND, OR, NOT, "phrase").
    pub fn search(
        &self,
        session_key: &str,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.file_name, s.path, c.chunk_idx, c.content
             FROM doc_chunks c
             JOIN doc_sources s ON s.id = c.source_id AND s.session_key = ?2
             WHERE doc_chunks MATCH ?1
             ORDER BY rank
             LIMIT ?3",
        )?;
        let results = stmt
            .query_map(
                params![query, session_key, i64::try_from(limit).unwrap_or(20)],
                |row| {
                    Ok(SearchResult {
                        file_name: row.get(0)?,
                        path: row.get(1)?,
                        chunk_idx: row.get(2)?,
                        content: row.get(3)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    /// List all documents ingested for `session_key`, newest first.
    pub fn list(&self, session_key: &str) -> anyhow::Result<Vec<DocInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT file_name, path, ingested_at, chunk_count \
             FROM doc_sources WHERE session_key = ?1 ORDER BY ingested_at DESC",
        )?;
        let docs = stmt
            .query_map(params![session_key], |row| {
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

    /// Remove all chunks for `path` within `session_key`.
    /// Returns `true` if the document existed.
    pub fn forget(&self, session_key: &str, path: &str) -> anyhow::Result<bool> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let id: Option<String> = tx
            .query_row(
                "SELECT id FROM doc_sources WHERE session_key = ?1 AND path = ?2",
                params![session_key, path],
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_temp() -> (TempDir, DocStore) {
        let dir = TempDir::new().unwrap();
        let store = DocStore::open(dir.path()).unwrap();
        (dir, store)
    }

    const SESSION_A: &str = "line:group_a";
    const SESSION_B: &str = "line:group_b";

    #[test]
    fn ingest_and_search() {
        let (_dir, store) = open_temp();
        let chunks = vec![
            "ราคาสินค้า A คือ 100 บาท".to_string(),
            "ราคาสินค้า B คือ 250 บาท".to_string(),
            "โปรโมชัน ซื้อ 2 แถม 1 ทุกรายการ".to_string(),
        ];
        store.ingest(SESSION_A, "/tmp/test.txt", &chunks).unwrap();

        let hits = store.search(SESSION_A, "สินค้า A", 5).unwrap();
        assert!(!hits.is_empty(), "should find chunk about สินค้า A");
        assert!(hits[0].content.contains("สินค้า A"));
    }

    #[test]
    fn sessions_are_isolated() {
        let (_dir, store) = open_temp();
        // Use completely disjoint content so trigram overlap cannot cause
        // cross-session hits.
        store
            .ingest(SESSION_A, "/tmp/alpha.txt", &["XYZALPHA unique token".to_string()])
            .unwrap();
        store
            .ingest(SESSION_B, "/tmp/beta.txt", &["QRSTBETA unique token".to_string()])
            .unwrap();

        // Session B must not find Session A's exclusive term
        let hits_b = store.search(SESSION_B, "XYZALPHA", 5).unwrap();
        assert!(hits_b.is_empty(), "session B must not see session A docs");

        // Session A must not find Session B's exclusive term
        let hits_a = store.search(SESSION_A, "QRSTBETA", 5).unwrap();
        assert!(hits_a.is_empty(), "session A must not see session B docs");

        // Each session sees only its own document in the list
        let list_a = store.list(SESSION_A).unwrap();
        assert_eq!(list_a.len(), 1);
        assert_eq!(list_a[0].file_name, "alpha.txt");
        let list_b = store.list(SESSION_B).unwrap();
        assert_eq!(list_b.len(), 1);
        assert_eq!(list_b[0].file_name, "beta.txt");
    }

    #[test]
    fn same_path_different_sessions_coexist() {
        let (_dir, store) = open_temp();
        store
            .ingest(SESSION_A, "/tmp/shared_name.txt", &["เนื้อหาของ A".to_string()])
            .unwrap();
        store
            .ingest(SESSION_B, "/tmp/shared_name.txt", &["เนื้อหาของ B".to_string()])
            .unwrap();

        let hits_a = store.search(SESSION_A, "เนื้อหา", 5).unwrap();
        assert_eq!(hits_a.len(), 1);
        assert!(hits_a[0].content.contains('A'));

        let hits_b = store.search(SESSION_B, "เนื้อหา", 5).unwrap();
        assert_eq!(hits_b.len(), 1);
        assert!(hits_b[0].content.contains('B'));
    }

    #[test]
    fn reingest_replaces_old_chunks() {
        let (_dir, store) = open_temp();
        store
            .ingest(SESSION_A, "/tmp/doc.txt", &["เวอร์ชัน 1".to_string()])
            .unwrap();
        store
            .ingest(SESSION_A, "/tmp/doc.txt", &["เวอร์ชัน 2".to_string()])
            .unwrap();

        let docs = store.list(SESSION_A).unwrap();
        assert_eq!(docs.len(), 1, "re-ingest must not duplicate source entry");
        assert_eq!(docs[0].chunk_count, 1);

        let hits = store.search(SESSION_A, "เวอร์ชัน", 5).unwrap();
        assert_eq!(hits.len(), 1, "only the new chunk should exist");
        assert!(hits[0].content.contains('2'), "old chunk must be gone");
    }

    #[test]
    fn search_no_match_returns_empty() {
        let (_dir, store) = open_temp();
        store
            .ingest(SESSION_A, "/tmp/a.txt", &["hello world".to_string()])
            .unwrap();
        let hits = store.search(SESSION_A, "ไม่มีคำนี้ในเอกสาร", 5).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn forget_removes_document() {
        let (_dir, store) = open_temp();
        store
            .ingest(SESSION_A, "/tmp/b.txt", &["จะลบออก".to_string()])
            .unwrap();
        let removed = store.forget(SESSION_A, "/tmp/b.txt").unwrap();
        assert!(removed);
        assert!(store.list(SESSION_A).unwrap().is_empty());
        assert!(store.search(SESSION_A, "จะลบออก", 5).unwrap().is_empty());
    }

    #[test]
    fn forget_only_affects_own_session() {
        let (_dir, store) = open_temp();
        store
            .ingest(SESSION_A, "/tmp/c.txt", &["เนื้อหา".to_string()])
            .unwrap();
        store
            .ingest(SESSION_B, "/tmp/c.txt", &["เนื้อหา".to_string()])
            .unwrap();

        // B forgets its copy — A's copy must survive
        let removed = store.forget(SESSION_B, "/tmp/c.txt").unwrap();
        assert!(removed);
        assert!(!store.search(SESSION_A, "เนื้อหา", 5).unwrap().is_empty());
    }

    #[test]
    fn forget_missing_returns_false() {
        let (_dir, store) = open_temp();
        let removed = store.forget(SESSION_A, "/tmp/notexist.txt").unwrap();
        assert!(!removed);
    }
}
