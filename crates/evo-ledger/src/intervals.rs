//! Work interval computation: groups events into intervals of continuous work.
//!
//! An interval is a contiguous stretch of time during which the person's
//! attention was on the same task (as determined by the identity chain).
//! A gap longer than `SEGMENT_GAP_MS` (default 5 minutes) starts a new
//! interval — the gap itself is not work time.
//!
//! Intervals carry evidence: the count of events, total dwell time, and
//! whether the person typed (production evidence).

use crate::identity::{IdentityChain, TaskKey};
use crate::ObservedEvent;
use rusqlite::{params, Connection};

/// The gap that starts a new interval (5 minutes, from screenpipe).
pub const SEGMENT_GAP_MS: u64 = 5 * 60 * 1000;

/// The status of a work interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalStatus {
    /// Recent enough that new events might extend it.
    Provisional,
    /// The finalization window has passed; this interval is stable.
    Final,
}

impl IntervalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntervalStatus::Provisional => "provisional",
            IntervalStatus::Final => "final",
        }
    }
}

/// One work interval: a contiguous stretch of the same task.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkInterval {
    pub id: i64,
    pub task_key: TaskKey,
    /// The interval's start time (epoch ms).
    pub start_ms: u64,
    /// The interval's end time (epoch ms).
    pub end_ms: u64,
    /// How many observed events are in this interval.
    pub event_count: u32,
    /// Total dwell time (ms) — how long the person was actually there.
    pub total_dwell_ms: u64,
    /// How many events had typing (production evidence).
    pub typed_count: u32,
    /// Content-free input counters aggregated over the interval.
    pub keys: u64,
    pub clicks: u64,
    pub scrolls: u64,
    /// How many file saves were attributed to this interval after the
    /// fact (production evidence, distinct from typing).
    pub attached_saves: usize,
    /// Whether this interval is provisional or final.
    pub status: IntervalStatus,
    /// All URLs touched in this interval (for search and restore).
    pub urls: Vec<String>,
    /// All window titles seen in this interval (presentation evidence).
    pub titles: Vec<String>,
    /// All document paths touched in this interval.
    pub document_paths: Vec<String>,
    /// All app names used in this interval.
    pub app_names: Vec<String>,
}

impl WorkInterval {
    /// A human-readable title for this interval.
    pub fn title(&self) -> String {
        if let Some(doc) = &self.task_key.document_path {
            let file = doc.rsplit('/').next().unwrap_or(doc);
            return file.to_string();
        }
        if let Some(title) = &self.task_key.window_title {
            return title.clone();
        }
        if let Some(host) = &self.task_key.url_host {
            return host.clone();
        }
        self.task_key.app_name.clone()
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> u64 {
        (self.end_ms.saturating_sub(self.start_ms)) / 1000
    }

    /// Whether this interval has production evidence (the person typed).
    pub fn has_production(&self) -> bool {
        self.typed_count > 0
    }
}

/// Ingests events and computes intervals.
///
/// This is the core algorithm:
/// 1. Sort events by timestamp
/// 2. Walk through them, resolving each event's task key via the identity chain
/// 3. Consecutive events with the same task key AND within the segment gap
///    are grouped into one interval
/// 4. A different task key OR a gap > SEGMENT_GAP_MS starts a new interval
/// 5. Each interval accumulates: event count, dwell time, typed count,
///    and all URLs/docs/apps it touched (for search and restore)
pub fn compute_intervals(events: &[ObservedEvent]) -> Vec<WorkInterval> {
    if events.is_empty() {
        return Vec::new();
    }

    // Sort by timestamp (stable — preserves insertion order for same-timestamp)
    let mut sorted: Vec<&ObservedEvent> = events.iter().collect();
    sorted.sort_by_key(|e| e.timestamp_ms);

    let mut intervals: Vec<WorkInterval> = Vec::new();
    let mut current: Option<WorkInterval> = None;

    for event in sorted {
        let key = IdentityChain::resolve(event);

        match &mut current {
            None => {
                // Start the first interval
                current = Some(new_interval(event, &key));
            }
            Some(interval) => {
                let gap = event.timestamp_ms.saturating_sub(interval.end_ms);
                let same_work = IdentityChain::same_work(&interval.task_key, &key);

                if same_work && gap <= SEGMENT_GAP_MS {
                    // Extend the current interval
                    extend_interval(interval, event);
                } else {
                    // Finalize the current interval and start a new one
                    finalize_interval(interval);
                    intervals.push(current.take().unwrap());
                    current = Some(new_interval(event, &key));
                }
            }
        }
    }

    // Don't forget the last interval
    if let Some(mut interval) = current {
        finalize_interval(&mut interval);
        intervals.push(interval);
    }

    intervals
}

fn new_interval(event: &ObservedEvent, key: &TaskKey) -> WorkInterval {
    WorkInterval {
        id: 0, // assigned by the database
        task_key: key.clone(),
        start_ms: event.timestamp_ms,
        end_ms: event.timestamp_ms,
        event_count: 1,
        total_dwell_ms: event.dwell_ms,
        typed_count: if event.typed { 1 } else { 0 },
        keys: event.keys,
        clicks: event.clicks,
        scrolls: event.scrolls,
        attached_saves: 0,
        status: IntervalStatus::Provisional,
        urls: event
            .url
            .as_ref()
            .map(|u| vec![u.clone()])
            .unwrap_or_default(),
        titles: event
            .window_title
            .trim()
            .is_empty()
            .then(Vec::new)
            .unwrap_or_else(|| vec![event.window_title.clone()]),
        document_paths: event
            .document_path
            .as_ref()
            .map(|d| vec![d.clone()])
            .unwrap_or_default(),
        app_names: vec![event.app_name.clone()],
    }
}

fn extend_interval(interval: &mut WorkInterval, event: &ObservedEvent) {
    interval.end_ms = event.timestamp_ms;
    interval.event_count += 1;
    interval.total_dwell_ms += event.dwell_ms;
    interval.keys += event.keys;
    interval.clicks += event.clicks;
    interval.scrolls += event.scrolls;
    if event.typed {
        interval.typed_count += 1;
    }
    if let Some(url) = &event.url {
        if !interval.urls.contains(url) {
            interval.urls.push(url.clone());
        }
    }
    if !event.window_title.trim().is_empty() && !interval.titles.contains(&event.window_title) {
        interval.titles.push(event.window_title.clone());
    }
    if let Some(doc) = &event.document_path {
        if !interval.document_paths.contains(doc) {
            interval.document_paths.push(doc.clone());
        }
    }
    if !interval.app_names.contains(&event.app_name) {
        interval.app_names.push(event.app_name.clone());
    }
}

fn finalize_interval(interval: &mut WorkInterval) {
    // Check if this interval is old enough to be final
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let age = now_ms.saturating_sub(interval.end_ms);
    if age > 5 * 60 * 1000 {
        // Older than 5 minutes: final
        interval.status = IntervalStatus::Final;
    }
}

/// Queries intervals from the database, most recent first.
/// Only intervals with at least `min_events` events are returned —
/// a single-event interval is a glance, not a work session.
pub fn query(db: &Connection, limit: usize) -> Result<Vec<WorkInterval>, crate::LedgerError> {
    query_with_filter(db, limit, 2) // default: at least 2 events
}

/// Queries intervals with a custom minimum event count filter.
pub fn query_with_filter(
    db: &Connection,
    limit: usize,
    min_events: u32,
) -> Result<Vec<WorkInterval>, crate::LedgerError> {
    let mut stmt = db
        .prepare(
            "SELECT id, task_key_doc, task_key_page, task_key_title, task_key_host, task_key_app,
                confidence, start_ms, end_ms, event_count, total_dwell_ms,
                typed_count, status, urls_json, titles_json,
                keys, clicks, scrolls, production_docs
         FROM intervals
         WHERE event_count >= ?
         ORDER BY start_ms DESC
         LIMIT ?",
        )
        .map_err(crate::LedgerError::Sqlite)?;

    let rows = stmt
        .query_map(params![min_events as i64, limit as i64], |row| {
            let id: i64 = row.get(0)?;
            let doc: Option<String> = row.get(1)?;
            let page: Option<String> = row.get(2)?;
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
            let urls_json: String = row.get(13)?;
            let titles_json: String = row.get(14)?;
            let keys: i64 = row.get(15)?;
            let clicks: i64 = row.get(16)?;
            let scrolls: i64 = row.get(17)?;
            let production_docs: String = row.get(18)?;

            // Hydrate resource evidence: URLs from the interval, documents from
            // the identity path plus every save attributed to this interval.
            let urls: Vec<String> = urls_json
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.to_string())
                .collect();
            let titles: Vec<String> = titles_json
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.to_string())
                .collect();
            let attached_saves = production_docs
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count();
            let mut document_paths: Vec<String> = production_docs
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.to_string())
                .collect();
            if let Some(key_doc) = doc.as_ref().filter(|d| !d.trim().is_empty()) {
                if !document_paths.contains(key_doc) {
                    document_paths.insert(0, key_doc.clone());
                }
            }

            Ok(WorkInterval {
                id,
                task_key: TaskKey {
                    document_path: doc,
                    page,
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
                keys: keys.max(0) as u64,
                clicks: clicks.max(0) as u64,
                scrolls: scrolls.max(0) as u64,
                attached_saves,
                status: if status == "final" {
                    IntervalStatus::Final
                } else {
                    IntervalStatus::Provisional
                },
                urls,
                titles,
                document_paths,
                app_names: Vec::new(),
            })
        })
        .map_err(crate::LedgerError::Sqlite)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(crate::LedgerError::Sqlite)
}
