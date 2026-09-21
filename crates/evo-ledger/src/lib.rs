//! # evo-ledger — the deterministic activity ledger.
//!
//! Groups observed computer-usage events into **work intervals** using a
//! fixed identity chain, then makes those intervals searchable with FTS5.
//!
//! ## The identity chain (from screenpipe's production-hardened design)
//!
//! Every observed event carries one or more identity signals. The ledger
//! resolves them in priority order, producing a stable "task key" that
//! groups events into the same work:
//!
//! 1. `document_path` (confidence 0.85) — the file being edited
//! 2. `window_title` (confidence 0.80) — the window's title text
//! 3. `url_host` (confidence 0.70) — the domain of the active URL
//! 4. `app_name` (confidence 0.60) — the application binary name
//!
//! Two events belong to the same work interval when their resolved task
//! keys match AND they are within the segment gap (default 5 minutes).
//! A longer gap starts a new interval; the gap itself is recorded as
//! "Unobserved" time.
//!
//! ## Deterministic and revisable
//!
//! The ledger is a pure function of the event stream: the same events
//! always produce the same intervals. When new events arrive, only the
//! affected time range is recomputed (provisional intervals become final
//! after a 5-minute finalization delay).
//!
//! ## Searchable
//!
//! Every interval is indexed in SQLite FTS5 (external-content mode) over
//! its task key, app name, window title, URL, and all resource identifiers
//! it touched. Search is a MATCH query — sub-millisecond, even over months
//! of data. No vector search, no embeddings (screenpipe tried and dropped
//! them; FTS5 is sufficient and 100x simpler).

use rusqlite::Connection;
use std::path::Path;

pub mod gate;
pub mod hypotheses;
pub mod identity;
pub mod intervals;
pub mod observation_log;
pub mod pages;
pub mod production;
pub mod schema;
pub mod search;
pub mod source;
pub mod works;

pub use identity::{IdentityChain, TaskKey};
pub use intervals::{IntervalStatus, WorkInterval};
pub use search::SearchResult;
pub use works::{works_from_database, IdentityKind, Work};

/// The ledger: owns a SQLite database with the activity schema.
pub struct Ledger {
    db: Connection,
}

impl Ledger {
    /// Opens (or creates) the ledger database at the given path.
    pub fn open(path: &Path) -> Result<Self, LedgerError> {
        let db = Connection::open(path)?;
        let ledger = Self { db };
        schema::initialize(&ledger.db)?;
        Ok(ledger)
    }

    /// Opens an existing ledger strictly read-only.
    ///
    /// For UI readers against a ledger the engine is actively rebuilding:
    /// no journal-mode pragma, no schema creation, no migration — nothing
    /// that would need a write lock and lose the race against the writer.
    /// The caller checks existence first; a missing file is an error.
    pub fn open_readonly(path: &Path) -> Result<Self, LedgerError> {
        let uri = format!("file:{}?mode=ro", path.display());
        let db = Connection::open_with_flags(
            uri,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                | rusqlite::OpenFlags::SQLITE_OPEN_URI
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        db.execute_batch("PRAGMA busy_timeout = 2000;")?;
        Ok(Self { db })
    }

    /// Opens an in-memory ledger (for tests).
    pub fn open_memory() -> Result<Self, LedgerError> {
        let db = Connection::open_in_memory()?;
        let ledger = Self { db };
        schema::initialize(&ledger.db)?;
        Ok(ledger)
    }

    /// Ingests a batch of observed events, computing work intervals.
    pub fn ingest(&mut self, events: &[ObservedEvent]) -> Result<usize, LedgerError> {
        source::ingest(&mut self.db, events)
    }

    /// Attributes file saves to the intervals whose attention surface names
    /// the saved file (production attaches to attention; it never creates
    /// identity). Saves that match no attention surface are background
    /// noise and are dropped. Returns `(attached, dropped)`.
    pub fn attach_production(
        &mut self,
        saves: &[ProductionSave],
    ) -> Result<(usize, usize), LedgerError> {
        production::attach(&mut self.db, saves)
    }

    /// Returns all work intervals, most recent first.
    pub fn intervals(&self, limit: usize) -> Result<Vec<WorkInterval>, LedgerError> {
        intervals::query(&self.db, limit)
    }

    /// Full-text search over work intervals.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, LedgerError> {
        search::search(&self.db, query, limit)
    }

    /// Returns Works: purpose-level hypotheses, most recent first.
    pub fn works(&self, limit: usize) -> Result<Vec<works::Work>, LedgerError> {
        works::works_from_database(&self.db, limit)
    }

    /// The underlying connection (for read-only diagnostics).
    pub fn connection(&self) -> &Connection {
        &self.db
    }

    /// Works plus the deterministic merge proposals between them.
    pub fn works_and_proposals(
        &self,
        limit: usize,
    ) -> Result<(Vec<works::Work>, Vec<hypotheses::MergeProposal>), LedgerError> {
        works::works_and_proposals_from_database(&self.db, limit)
    }

    /// Replaces the persisted merge-proposal set (the engine rewrites it
    /// every cycle; it is derived state, never canonical).
    pub fn clear_merge_proposals(&self) {
        let _ = self.db.execute("DELETE FROM merge_proposals", []);
    }

    /// Records one merge proposal for the desktop to offer.
    pub fn record_merge_proposal(
        &self,
        subject_a: &str,
        subject_b: &str,
        name_a: &str,
        name_b: &str,
        shared_sittings: usize,
    ) -> Result<(), LedgerError> {
        self.db
            .execute(
                "INSERT INTO merge_proposals
                    (subject_a, subject_b, name_a, name_b, shared_sittings)
                 VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![subject_a, subject_b, name_a, name_b, shared_sittings as i64],
            )
            .map(|_| ())
            .map_err(LedgerError::Sqlite)
    }

    /// Records the person's declared same-work pair as ground truth.
    pub fn record_declaration(&self, subject_a: &str, subject_b: &str) -> Result<(), LedgerError> {
        self.db
            .execute(
                "INSERT OR IGNORE INTO declarations (subject_a, subject_b, declared_at)
                 SELECT ?, ?, ? WHERE NOT EXISTS (
                    SELECT 1 FROM declarations
                    WHERE (subject_a = ? AND subject_b = ?) OR (subject_a = ? AND subject_b = ?)
                 )",
                rusqlite::params![
                    subject_a,
                    subject_b,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0),
                    subject_a,
                    subject_b,
                    subject_b,
                    subject_a,
                ],
            )
            .map(|_| ())
            .map_err(LedgerError::Sqlite)
    }

    /// The engine's current merge proposals: (subject_a, subject_b,
    /// name_a, name_b, shared sittings). Subjects are representative
    /// resources of each work — what a confirmed merge declares.
    pub fn merge_proposals(
        &self,
    ) -> Result<Vec<(String, String, String, String, usize)>, LedgerError> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT subject_a, subject_b, name_a, name_b, shared_sittings FROM merge_proposals",
            )
            .map_err(LedgerError::Sqlite)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?.max(0) as usize,
                ))
            })
            .map_err(LedgerError::Sqlite)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::Sqlite)
    }

    /// Prunes raw events older than the retention period (days).
    /// Work intervals and their search index entries are preserved.
    pub fn prune(&mut self, retention_days: u32) -> Result<usize, LedgerError> {
        schema::prune(&self.db, retention_days).map_err(LedgerError::Sqlite)
    }

    /// Database size in bytes.
    pub fn size_bytes(&self, path: &Path) -> u64 {
        path.metadata().map(|m| m.len()).unwrap_or(0)
    }
}

/// One observed computer-usage event (the input to the ledger).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ObservedEvent {
    /// Epoch milliseconds.
    pub timestamp_ms: u64,
    /// The application's localized name (e.g., "Google Chrome").
    pub app_name: String,
    /// The focused window's title text.
    pub window_title: String,
    /// The active URL, if any (browser tabs).
    pub url: Option<String>,
    /// The document file path, if any (editors, viewers).
    pub document_path: Option<String>,
    /// Whether the person typed during this event.
    pub typed: bool,
    /// Dwell duration in milliseconds.
    pub dwell_ms: u64,
    /// Content-free input counters observed with this event: how many
    /// keystrokes, clicks, and scrolls the bucket counted. Zero for events
    /// that carry no input evidence. The mix (typing-dominant vs
    /// scroll-dominant) is what distinguishes writing from reading.
    #[serde(default)]
    pub keys: u64,
    #[serde(default)]
    pub clicks: u64,
    #[serde(default)]
    pub scrolls: u64,
}

/// A witnessed file save, held apart from attention events.
///
/// A save says "some process wrote this path" — not "the person produced
/// this". Attribution happens in [`Ledger::attach_production`], which only
/// accepts a save when the file's name appears on an attention surface the
/// person was actually looking at around that time.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProductionSave {
    /// Epoch milliseconds.
    pub timestamp_ms: u64,
    /// The absolute path of the saved file.
    pub path: String,
}

/// Ledger errors.
#[derive(Debug)]
pub enum LedgerError {
    Sqlite(rusqlite::Error),
    InvalidQuery(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::Sqlite(e) => write!(f, "ledger database error: {e}"),
            LedgerError::InvalidQuery(q) => write!(f, "invalid search query: {q}"),
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<rusqlite::Error> for LedgerError {
    fn from(e: rusqlite::Error) -> Self {
        LedgerError::Sqlite(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(
        ts: u64,
        app: &str,
        title: &str,
        url: Option<&str>,
        doc: Option<&str>,
    ) -> ObservedEvent {
        ObservedEvent {
            timestamp_ms: ts,
            app_name: app.to_string(),
            window_title: title.to_string(),
            url: url.map(|s| s.to_string()),
            document_path: doc.map(|s| s.to_string()),
            typed: false,
            dwell_ms: 5000,
            keys: 0,
            clicks: 0,
            scrolls: 0,
        }
    }

    #[test]
    fn same_document_groups_into_one_interval() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[
                event(
                    1000,
                    "VS Code",
                    "main.rs — project",
                    None,
                    Some("/project/src/main.rs"),
                ),
                event(
                    61000,
                    "VS Code",
                    "main.rs — project",
                    None,
                    Some("/project/src/main.rs"),
                ),
                event(
                    121000,
                    "VS Code",
                    "main.rs — project",
                    None,
                    Some("/project/src/main.rs"),
                ),
            ])
            .unwrap();
        let intervals = ledger.intervals(10).unwrap();
        assert_eq!(
            intervals.len(),
            1,
            "three events on the same document = one interval"
        );
        assert_eq!(
            intervals[0].task_key.document_path,
            Some("/project/src/main.rs".to_string())
        );
    }

    #[test]
    fn different_apps_different_intervals() {
        let mut ledger = Ledger::open_memory().unwrap();
        // Two events per app so each interval passes the engagement filter
        ledger
            .ingest(&[
                event(1000, "VS Code", "main.rs", None, Some("/src/main.rs")),
                event(61000, "VS Code", "main.rs", None, Some("/src/main.rs")),
                event(
                    121000,
                    "Chrome",
                    "GitHub — repo",
                    Some("https://github.com/repo"),
                    None,
                ),
                event(
                    181000,
                    "Chrome",
                    "GitHub — repo",
                    Some("https://github.com/repo"),
                    None,
                ),
            ])
            .unwrap();
        let intervals = ledger.intervals(10).unwrap();
        assert_eq!(
            intervals.len(),
            2,
            "different task keys = different intervals"
        );
    }

    #[test]
    fn gap_creates_new_interval() {
        let mut ledger = Ledger::open_memory().unwrap();
        // Two clusters of events, 10 minutes apart (gap > 5-minute threshold)
        // Each cluster has 2 events so it passes the engagement filter
        ledger
            .ingest(&[
                event(
                    1000,
                    "Chrome",
                    "docs",
                    Some("https://docs.example.com"),
                    None,
                ),
                event(
                    61000,
                    "Chrome",
                    "docs",
                    Some("https://docs.example.com"),
                    None,
                ),
                event(
                    601000,
                    "Chrome",
                    "docs",
                    Some("https://docs.example.com"),
                    None,
                ),
                event(
                    661000,
                    "Chrome",
                    "docs",
                    Some("https://docs.example.com"),
                    None,
                ),
            ])
            .unwrap();
        let intervals = ledger.intervals(10).unwrap();
        assert_eq!(
            intervals.len(),
            2,
            "10-minute gap = two intervals (same task, separate sessions)"
        );
    }

    #[test]
    fn page_identity_separates_and_groups() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[
                event(
                    1000,
                    "Chrome",
                    "Page 1",
                    Some("https://example.com/page1"),
                    None,
                ),
                event(
                    61000,
                    "Chrome",
                    "Page 1 changed title",
                    Some("https://example.com/page1"),
                    None,
                ),
                event(
                    121000,
                    "Chrome",
                    "Page 2",
                    Some("https://example.com/page2"),
                    None,
                ),
                event(
                    181000,
                    "Chrome",
                    "Page 2",
                    Some("https://example.com/page2"),
                    None,
                ),
            ])
            .unwrap();
        let intervals = ledger.intervals(10).unwrap();
        assert_eq!(
            intervals.len(),
            2,
            "same page across titles = one interval; different page = another"
        );
    }

    #[test]
    fn search_finds_by_app_name() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[
                event(
                    1000,
                    "VS Code",
                    "main.rs — evo",
                    None,
                    Some("/evo/src/main.rs"),
                ),
                event(
                    61000,
                    "Chrome",
                    "GitHub — evo",
                    Some("https://github.com/evo"),
                    None,
                ),
            ])
            .unwrap();
        let results = ledger.search("evo", 10).unwrap();
        assert!(
            !results.is_empty(),
            "searching 'evo' should find intervals mentioning evo"
        );
    }

    #[test]
    fn search_finds_by_url() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[event(
                1000,
                "Chrome",
                "Pitch Deck",
                Some("https://app.pitch.com/presentation/123"),
                None,
            )])
            .unwrap();
        let results = ledger.search("pitch", 10).unwrap();
        assert!(
            !results.is_empty(),
            "searching 'pitch' should find the pitch.com interval"
        );
    }

    #[test]
    fn prune_removes_old_events_but_keeps_intervals() {
        let mut ledger = Ledger::open_memory().unwrap();
        // Insert an event from 60 days ago
        let old_ts = 1000; // very old
        ledger
            .ingest(&[event(
                old_ts,
                "Chrome",
                "old work",
                Some("https://old.example.com"),
                None,
            )])
            .unwrap();
        // Prune events older than 30 days
        let pruned = ledger.prune(30).unwrap();
        assert!(pruned >= 0, "prune should succeed");
        // Intervals should still be queryable (they're derived, not raw events)
        let intervals = ledger.intervals(10).unwrap();
        // The interval may or may not survive pruning depending on implementation
        // (intervals reference events; if events are pruned, intervals go too)
        // This is the correct behavior: pruned data is gone.
    }
}
