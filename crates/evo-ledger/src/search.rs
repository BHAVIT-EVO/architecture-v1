//! FTS5 full-text search over work intervals.
//!
//! Uses SQLite FTS5 with external-content mode and a two-phase query
//! (from screenpipe's search architecture):
//!
//! Phase 1: FTS MATCH query returns candidate row IDs (fast, indexed)
//! Phase 2: Hydrate full interval data for just those IDs (avoids
//!          dragging large text columns through the scan)
//!
//! The search text includes: app names, window titles, URL hosts,
//! full URLs, document paths, and filenames — so searching "pitch deck"
//! finds the interval with "app.pitch.com" in its URLs, and searching
//! "main.rs" finds the interval editing that file.

use crate::identity::TaskKey;
use crate::intervals::{IntervalStatus, WorkInterval};
use rusqlite::{params, Connection};

/// A search result: the interval plus a relevance score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub interval: WorkInterval,
    /// FTS5 rank (lower is better in SQLite's default; we negate for intuitive sorting).
    pub score: f64,
    /// Which field matched (for highlighting).
    pub matched_field: String,
}

/// Performs a full-text search over work intervals.
///
/// The query is passed directly to FTS5's MATCH syntax, so it supports:
/// - Simple words: `pitch` matches any interval containing "pitch"
/// - Phrases: `"pitch deck"` matches the exact phrase
/// - Prefix: `pitch*` matches "pitch", "pitcher", "pitching"
/// - Boolean: `pitch OR deck` matches either
/// - Field-scoped: `task_key_app:Chrome` matches only the app field
pub fn search(
    db: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, crate::LedgerError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    // Sanitize the query: escape special FTS5 characters if the query
    // doesn't look like it's using FTS5 syntax intentionally.
    // (If it contains OR, AND, quotes, or colons, pass through as-is.)
    let fts_query = if trimmed.contains('"') || trimmed.contains(':') || trimmed.contains(" OR ") {
        trimmed.to_string()
    } else {
        // Treat each word as a prefix match for better recall
        trimmed
            .split_whitespace()
            .map(|w| format!("{w}*"))
            .collect::<Vec<_>>()
            .join(" ")
    };

    // Two-phase query (screenpipe pattern):
    // Phase 1: FTS returns candidate IDs + rank
    // Phase 2: Hydrate full rows for those IDs only
    let sql = "
        WITH candidates AS MATERIALIZED (
            SELECT rowid, rank
            FROM intervals_fts
            WHERE intervals_fts MATCH ?
            ORDER BY rank
            LIMIT ?
        )
        SELECT
            c.rowid as id,
            i.task_key_doc, i.task_key_page, i.task_key_title, i.task_key_host, i.task_key_app,
            i.confidence, i.start_ms, i.end_ms, i.event_count,
            i.total_dwell_ms, i.typed_count, i.status, i.search_text,
            c.rank as score
        FROM candidates c
        JOIN intervals i ON i.id = c.rowid
        ORDER BY c.rank
    ";

    let mut stmt = db.prepare(sql).map_err(crate::LedgerError::Sqlite)?;
    let rows = stmt
        .query_map(params![fts_query, limit as i64], |row| {
            let id: i64 = row.get(0)?;
            let doc: Option<String> = row.get(1)?;
            let _page: Option<String> = row.get(2)?;
            let title: Option<String> = row.get(3)?;
            let host: Option<String> = row.get(4)?;
            let app: String = row.get(5)?;
            let confidence: f64 = row.get(6)?;
            let start_ms: i64 = row.get(7)?;
            let end_ms: i64 = row.get(8)?;
            let event_count: i64 = row.get(9)?;
            let total_dwell_ms: i64 = row.get(10)?;
            let typed_count: i64 = row.get(11)?;
            let status: String = row.get(12)?;
            let search_text: String = row.get(13)?;
            let score: f64 = row.get(14)?;

            Ok((
                WorkInterval {
                    id,
                    task_key: TaskKey {
                        document_path: doc,
                        page: None,
                        window_title: title,
                        url_host: host,
                        app_name: app,
                        confidence,
                    },
                    start_ms: start_ms as u64,
                    end_ms: end_ms as u64,
                    event_count: event_count as u32,
                    total_dwell_ms: total_dwell_ms as u64,
                    typed_count: typed_count as u32,
                    keys: 0,
                    clicks: 0,
                    scrolls: 0,
                    attached_saves: 0,
                    status: if status == "final" {
                        IntervalStatus::Final
                    } else {
                        IntervalStatus::Provisional
                    },
                    urls: extract_urls(&search_text),
                    titles: Vec::new(),
                    document_paths: Vec::new(),
                    app_names: Vec::new(),
                },
                score,
                search_text,
            ))
        })
        .map_err(crate::LedgerError::Sqlite)?;

    let mut results = Vec::new();
    for row in rows {
        let (interval, score, search_text) = row.map_err(crate::LedgerError::Sqlite)?;
        // Determine which field matched (for highlighting)
        let matched_field = if let Some(host) = &interval.task_key.url_host {
            if search_text.to_lowercase().contains(&host.to_lowercase()) {
                "url".to_string()
            } else {
                "text".to_string()
            }
        } else {
            "text".to_string()
        };
        results.push(SearchResult {
            interval,
            score: -score, // negate: higher score = better match
            matched_field,
        });
    }

    Ok(results)
}

/// Extracts URLs from the search text (they're space-separated in the index).
fn extract_urls(search_text: &str) -> Vec<String> {
    search_text
        .split_whitespace()
        .filter(|word| word.starts_with("http://") || word.starts_with("https://"))
        .map(String::from)
        .collect()
}
