use rusqlite::Connection;

pub const SCHEMA_VERSION: u32 = 3;

pub fn run(conn: &Connection) -> rusqlite::Result<()> {
    // Bootstrap: base tables and schema_meta (all idempotent).
    // WAL mode is set by SessionDb::open() with a NFS-safe fallback.
    conn.execute_batch(
        "
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

        -- Caches a chat platform bot's own immutable user id, keyed by a hash
        -- of its channel token. Lets adapters skip a network round-trip (e.g.
        -- LINE /v2/bot/info) on every restart; a token change yields a new
        -- hash and triggers a one-time re-fetch.
        CREATE TABLE IF NOT EXISTS line_bot_identity (
            token_hash  TEXT PRIMARY KEY,
            bot_user_id TEXT NOT NULL
        );
    ",
    )?;

    let version: u32 = conn
        .query_row("SELECT version FROM schema_meta LIMIT 1", [], |r| r.get(0))
        .unwrap_or(0);

    if version < 2 {
        migrate_to_v2(conn, version)?;
    }

    if version < 3 {
        migrate_to_v3(conn)?;
    }

    Ok(())
}

/// Migration to v2: rebuild messages_fts with trigram tokenizer for substring search.
///
/// Trigram tokenizer (SQLite 3.34+, always available in bundled builds) lets the
/// session_search tool find partial matches — e.g. querying "pythag" now finds
/// "Pythagorean" without requiring an exact token boundary.
fn migrate_to_v2(conn: &Connection, prev_version: u32) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        -- Drop existing FTS artefacts before rebuilding with trigram.
        DROP TABLE   IF EXISTS messages_fts;
        DROP TRIGGER IF EXISTS messages_ai;
        DROP TRIGGER IF EXISTS messages_ad;

        CREATE VIRTUAL TABLE messages_fts
        USING fts5(
            content,
            content     = 'messages',
            content_rowid = 'rowid',
            tokenize    = 'trigram'
        );

        -- Re-populate from existing messages (no-op for fresh databases).
        INSERT INTO messages_fts(rowid, content) SELECT rowid, content FROM messages;

        CREATE TRIGGER messages_ai AFTER INSERT ON messages BEGIN
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        CREATE TRIGGER messages_ad AFTER DELETE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
        END;
    ",
    )?;

    if prev_version == 0 {
        conn.execute(
            "INSERT INTO schema_meta (version) VALUES (?1)",
            [SCHEMA_VERSION],
        )?;
    } else {
        conn.execute("UPDATE schema_meta SET version = ?1", [SCHEMA_VERSION])?;
    }

    Ok(())
}

/// Migration to v3: add pending_tasks table for crash-recovery of in-flight agent runs.
fn migrate_to_v3(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS pending_tasks (
            id          TEXT PRIMARY KEY,
            session_key TEXT NOT NULL,
            platform    TEXT NOT NULL,
            chat_id     TEXT NOT NULL,
            task        TEXT NOT NULL,
            hint        TEXT,
            created_at  REAL NOT NULL
        );
        UPDATE schema_meta SET version = 3;
    ",
    )
}
