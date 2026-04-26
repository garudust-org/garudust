use rusqlite::Connection;

pub const SCHEMA_VERSION: u32 = 1;

pub fn run(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS schema_meta (
            version INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id           TEXT PRIMARY KEY,
            source       TEXT NOT NULL,
            user_id      TEXT,
            model        TEXT,
            system_prompt TEXT,
            started_at   REAL NOT NULL,
            ended_at     REAL,
            input_tokens  INTEGER DEFAULT 0,
            output_tokens INTEGER DEFAULT 0,
            message_count INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS messages (
            id         TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            role       TEXT NOT NULL,
            content    TEXT NOT NULL,
            created_at REAL NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts
        USING fts5(content, content='messages', content_rowid='rowid');

        CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
        END;
    ")
}
