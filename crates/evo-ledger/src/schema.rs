//! SQLite schema for the activity ledger.
//!
//! Tables:
//! - `events` — raw observed events (prunable)
//! - `intervals` — derived work intervals (the product surface)
//! - `intervals_fts` — FTS5 external-content search index over intervals
//!
//! The FTS index uses external-content mode (`content='intervals'`) which
//! makes DELETE operations ~400x faster than standalone FTS mode (this is
//! screenpipe's hard-won lesson: standalone mode rewrites the entire
//! inverted index per deleted row).

use rusqlite::{params, Connection};

/// Creates the schema if it doesn't exist. Idempotent.
pub fn initialize(db: &Connection) -> Result<(), rusqlite::Error> {
    // Performance pragmas (from screenpipe's battle-tested set)
    db.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA temp_store = MEMORY;
        PRAGMA wal_autocheckpoint = 0;
        PRAGMA busy_timeout = 2000;
    ",
    )?;

    db.execute_batch("
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_ms INTEGER NOT NULL,
            app_name TEXT NOT NULL,
            window_title TEXT NOT NULL DEFAULT '',
            url TEXT,
            document_path TEXT,
            typed INTEGER NOT NULL DEFAULT 0,
            dwell_ms INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp_ms);
        CREATE INDEX IF NOT EXISTS idx_events_app ON events(app_name);

        CREATE TABLE IF NOT EXISTS intervals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_key_doc TEXT,
            task_key_page TEXT,
            task_key_title TEXT,
            task_key_host TEXT,
            task_key_app TEXT NOT NULL,
            confidence REAL NOT NULL DEFAULT 0.6,
            start_ms INTEGER NOT NULL,
            end_ms INTEGER NOT NULL,
            event_count INTEGER NOT NULL DEFAULT 0,
            total_dwell_ms INTEGER NOT NULL DEFAULT 0,
            typed_count INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'final',
            -- Full-text searchable text: all the identifiers this interval touched
            search_text TEXT NOT NULL DEFAULT '',
            -- All URLs touched in this interval, newline-separated (restore targets)
            urls_json TEXT NOT NULL DEFAULT '',
            -- Window titles seen in this interval, newline-separated. Titles
            -- are presentation evidence: the page names the person read,
            -- kept even when identity resolves to a URL host or document.
            titles_json TEXT NOT NULL DEFAULT '',
            -- Content-free input counters aggregated over the interval. The
            -- keys-vs-scrolls mix is what distinguishes writing from reading.
            keys INTEGER NOT NULL DEFAULT 0,
            clicks INTEGER NOT NULL DEFAULT 0,
            scrolls INTEGER NOT NULL DEFAULT 0,
            -- File saves attributed to this interval, newline-separated.
            -- Production attaches to attention; it never creates identity.
            production_docs TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_intervals_start ON intervals(start_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_intervals_task ON intervals(task_key_app, task_key_host, task_key_doc);

        -- The person's declared work groupings: ground truth over every
        -- derived rule. Written by the engine from OBS-WORK-GROUPED records.
        CREATE TABLE IF NOT EXISTS declarations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            subject_a TEXT NOT NULL,
            subject_b TEXT NOT NULL,
            declared_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_declarations_pair
            ON declarations(subject_a, subject_b);

        -- Deterministic merge proposals the engine derived (two
        -- established works co-present in several sittings). Offered to
        -- the person; confirmed merges become declarations.
        CREATE TABLE IF NOT EXISTS merge_proposals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            subject_a TEXT NOT NULL,
            subject_b TEXT NOT NULL,
            name_a TEXT NOT NULL,
            name_b TEXT NOT NULL,
            shared_sittings INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS intervals_fts USING fts5(
            search_text,
            task_key_app,
            task_key_title,
            task_key_host,
            task_key_doc,
            content='intervals',
            content_rowid='id',
            tokenize='unicode61'
        );

        -- Triggers to keep the FTS index in sync with the intervals table
        CREATE TRIGGER IF NOT EXISTS intervals_ai AFTER INSERT ON intervals BEGIN
            INSERT INTO intervals_fts(rowid, search_text, task_key_app, task_key_title, task_key_host, task_key_doc)
            VALUES (new.id, new.search_text, new.task_key_app, new.task_key_title, new.task_key_host, new.task_key_doc);
        END;
        CREATE TRIGGER IF NOT EXISTS intervals_ad AFTER DELETE ON intervals BEGIN
            INSERT INTO intervals_fts(intervals_fts, rowid, search_text, task_key_app, task_key_title, task_key_host, task_key_doc)
            VALUES ('delete', old.id, old.search_text, old.task_key_app, old.task_key_title, old.task_key_host, old.task_key_doc);
        END;
        CREATE TRIGGER IF NOT EXISTS intervals_au AFTER UPDATE ON intervals BEGIN
            INSERT INTO intervals_fts(intervals_fts, rowid, search_text, task_key_app, task_key_title, task_key_host, task_key_doc)
            VALUES ('delete', old.id, old.search_text, old.task_key_app, old.task_key_title, old.task_key_host, old.task_key_doc);
            INSERT INTO intervals_fts(rowid, search_text, task_key_app, task_key_title, task_key_host, task_key_doc)
            VALUES (new.id, new.search_text, new.task_key_app, new.task_key_title, new.task_key_host, new.task_key_doc);
        END;
    ")?;

    // Migration for ledgers created before the evidence columns existed.
    add_column_if_missing(db, "intervals", "task_key_page", "TEXT")?;
    add_column_if_missing(db, "intervals", "urls_json", "TEXT NOT NULL DEFAULT ''")?;
    add_column_if_missing(db, "intervals", "titles_json", "TEXT NOT NULL DEFAULT ''")?;
    add_column_if_missing(db, "intervals", "keys", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(db, "intervals", "clicks", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(db, "intervals", "scrolls", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(
        db,
        "intervals",
        "production_docs",
        "TEXT NOT NULL DEFAULT ''",
    )?;

    Ok(())
}

/// Adds a column to a table if it is not already present.
fn add_column_if_missing(
    db: &Connection,
    table: &str,
    column: &str,
    decl: &str,
) -> Result<(), rusqlite::Error> {
    let present: i64 = db.query_row(
        &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = ?",),
        params![column],
        |row| row.get(0),
    )?;
    if present == 0 {
        db.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl};"))?;
    }
    Ok(())
}

/// Prunes raw events older than the retention period.
/// Intervals (the derived work) are preserved — only the raw event
/// ledger is pruned, which is the bulk of the storage.
pub fn prune(db: &Connection, retention_days: u32) -> Result<usize, rusqlite::Error> {
    let cutoff_ms = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0))
        - (retention_days as i64 * 86_400_000);
    let n = db.execute(
        "DELETE FROM events WHERE timestamp_ms < ?",
        params![cutoff_ms],
    )?;
    // Compact the database (reclaims space from deleted rows)
    let _ = db.execute_batch("VACUUM;");
    Ok(n)
}
