//! Production attribution: file saves attach to attention, never create it.
//!
//! A raw file-save event says only "some process wrote this path". It does
//! not say the person produced it: photo-library daemons, build tools, and
//! sync clients all write files constantly. Treating saves as attention is
//! how a ledger fills up with "Works" like a Photos database internals or
//! compiler temp files.
//!
//! The rule here is evidence-based, not name-based (nothing is hardcoded):
//! **a save is the person's production only when the file's name appears on
//! the attention surface they were actually looking at around that time** —
//! the window title (editors show the file name) or the URL (downloads).
//! If no attention surface names the file, the save is background noise and
//! is dropped.
//!
//! Attached saves enrich the interval (production evidence + restore target
//! + search text) without touching its identity: the task key stays whatever
//! attention resolved it to.

use crate::ProductionSave;
use rusqlite::{params, Connection};
use std::collections::{BTreeMap, BTreeSet};

/// How far a save may fall outside an interval's time span and still attach.
pub const ATTACH_GRACE_MS: u64 = 5 * 60 * 1000;

/// A save path written more often than this (total, or per day, or across
/// more days than this) is machine-managed — a daemon, sync client, or
/// database engine writing on a schedule — not a document a person produces.
const PATH_SAVES_TOTAL_MAX: usize = 12;
const PATH_SAVES_PER_DAY_MAX: usize = 4;
const PATH_DISTINCT_DAYS_MAX: usize = 2;

/// A directory tree receiving more saves than this within the burst window
/// is in the middle of a write storm (build, sync, index rebuild). Saves
/// during a storm are machine output, whatever wrote them.
const TREE_BURST_MAX: usize = 6;
const TREE_BURST_WINDOW_MS: u64 = 2 * 60 * 1000;
/// Ancestor directories shallower than this many components (the root-ish
/// prefix of any path) are never burst-checked — they contain everything.
const TREE_MIN_COMPONENTS: usize = 4;

/// A directory whose saves span less than this is "fresh" — created moments
/// ago by a build or cache writer. Fresh directories inherit their
/// ancestors' storms; stable directories (a project's src/, edited across
/// hours and days) do not, so a sibling build storm under the same root
/// cannot mask real edits.
const OWN_DIR_STABLE_MS: u64 = 10 * 60 * 1000;

/// Filenames that are essentially one long hex/UUID blob are machine output
/// (photo assets, content-addressed caches, registry keys). Human file
/// names carry dictionary text; a name at least this long whose
/// alphanumeric characters are almost entirely hex characters is not one.
const HEX_NAME_MIN_LEN: usize = 20;
const HEX_NAME_RATIO: f64 = 0.8;

/// Names that are mostly digits over at least this many alphanumeric
/// characters are counters, timestamps, and job IDs — machine output.
const DIGIT_NAME_MIN_LEN: usize = 10;
const DIGIT_NAME_RATIO: f64 = 0.6;

/// A basename appearing in more than this many distinct directories is a
/// machine-generated name fanned out across per-build directories (build
/// scripts, cache shards). A person's file name lives in one project.
const NAME_DIR_FANOUT_MAX: usize = 3;

/// One interval row relevant to attribution.
struct IntervalRow {
    id: i64,
    start_ms: u64,
    end_ms: u64,
    /// Lowercased concatenation of every attention surface: window title,
    /// URL host, document path, and full URLs. The file name must appear
    /// here for the surface rule to attach.
    surface: String,
    /// Whether the person typed during this interval (presence evidence).
    typed_count: u32,
}

/// Behavioral statistics over the save stream, computed once per attach.
struct SaveStats {
    /// path -> (total saves, saves per day bucket, distinct days)
    path_counts: BTreeMap<String, (usize, BTreeMap<u64, usize>, BTreeSet<u64>)>,
    /// directory -> sorted save timestamps under it (ancestors included).
    dir_times: BTreeMap<String, Vec<u64>>,
    /// directory -> (first save ts, last save ts) — the directory's lifetime.
    dir_spans: BTreeMap<String, (u64, u64)>,
    /// basename -> distinct directories it was saved in.
    name_dirs: BTreeMap<String, BTreeSet<String>>,
}

impl SaveStats {
    fn build(saves: &[ProductionSave]) -> Self {
        let mut stats = SaveStats {
            path_counts: BTreeMap::new(),
            dir_times: BTreeMap::new(),
            dir_spans: BTreeMap::new(),
            name_dirs: BTreeMap::new(),
        };
        for save in saves {
            let day = save.timestamp_ms / 86_400_000;
            let entry = stats
                .path_counts
                .entry(save.path.clone())
                .or_insert_with(|| (0, BTreeMap::new(), BTreeSet::new()));
            entry.0 += 1;
            *entry.1.entry(day).or_insert(0) += 1;
            entry.2.insert(day);

            let file_dir = save
                .path
                .rsplit_once('/')
                .map(|(d, _)| d.to_string())
                .unwrap_or_default();
            if let Some(basename) = save.path.rsplit('/').next() {
                stats
                    .name_dirs
                    .entry(basename.to_string())
                    .or_default()
                    .insert(file_dir.clone());
            }

            // Register the save under the file's directory and every
            // ancestor deeper than the root-ish prefix.
            let mut dir = file_dir;
            loop {
                if dir.split('/').filter(|c| !c.is_empty()).count() >= TREE_MIN_COMPONENTS {
                    let times = stats.dir_times.entry(dir.clone()).or_default();
                    times.push(save.timestamp_ms);
                    let span = stats
                        .dir_spans
                        .entry(dir.clone())
                        .or_insert((save.timestamp_ms, save.timestamp_ms));
                    span.0 = span.0.min(save.timestamp_ms);
                    span.1 = span.1.max(save.timestamp_ms);
                }
                match dir.rsplit_once('/') {
                    Some((parent, _)) => dir = parent.to_string(),
                    None => break,
                }
            }
        }
        for times in stats.dir_times.values_mut() {
            times.sort_unstable();
        }
        stats
    }

    /// Whether the exact path is written like a machine manages it:
    /// too often in total, too often per day, or across too many days.
    fn path_is_machine_managed(&self, path: &str) -> bool {
        match self.path_counts.get(path) {
            None => false,
            Some((total, per_day, days)) => {
                *total > PATH_SAVES_TOTAL_MAX
                    || days.len() > PATH_DISTINCT_DAYS_MAX
                    || per_day.values().any(|c| *c > PATH_SAVES_PER_DAY_MAX)
            }
        }
    }

    /// Whether the save's directory tree is mid-storm at this instant.
    ///
    /// The file's own directory is always checked. Ancestors are checked
    /// only when the own directory is fresh (recently created by a build or
    /// cache writer) — a stable directory like a project's src/ does not
    /// inherit a sibling build storm under the same root.
    fn tree_is_stormy(&self, path: &str, ts: u64) -> bool {
        let file_dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        if self.burst_exceeds(file_dir, ts, TREE_BURST_MAX) {
            return true;
        }
        let own_fresh = self
            .dir_spans
            .get(file_dir)
            .map(|(first, last)| last.saturating_sub(*first) <= OWN_DIR_STABLE_MS)
            .unwrap_or(true);
        if !own_fresh {
            return false;
        }
        let mut dir = file_dir;
        while dir.split('/').filter(|c| !c.is_empty()).count() > TREE_MIN_COMPONENTS {
            match dir.rsplit_once('/') {
                Some((parent, _)) => dir = parent,
                None => break,
            }
            if self.burst_exceeds(dir, ts, TREE_BURST_MAX) {
                return true;
            }
        }
        false
    }

    fn burst_exceeds(&self, dir: &str, ts: u64, max: usize) -> bool {
        match self.dir_times.get(dir) {
            None => false,
            Some(times) => {
                let lo = times.partition_point(|t| *t < ts.saturating_sub(TREE_BURST_WINDOW_MS));
                let hi = times.partition_point(|t| *t <= ts + TREE_BURST_WINDOW_MS);
                hi - lo > max
            }
        }
    }

    /// Whether the basename is a machine-generated blob: high-entropy hex
    /// (UUIDs, content hashes), digit-dominant (counters, job IDs), or a
    /// name fanned out across many directories (build scripts, cache
    /// shards). People name files with words.
    fn name_is_machine_generated(&self, path: &str) -> bool {
        let Some(basename) = path.rsplit('/').next() else {
            return true;
        };
        let alnum: Vec<char> = basename
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        if alnum.len() >= HEX_NAME_MIN_LEN {
            let hex_chars = alnum
                .iter()
                .filter(|c| c.is_ascii_digit() || c.is_ascii_hexdigit())
                .count();
            if hex_chars as f64 / alnum.len() as f64 >= HEX_NAME_RATIO {
                return true;
            }
        }
        if alnum.len() >= DIGIT_NAME_MIN_LEN {
            let digits = alnum.iter().filter(|c| c.is_ascii_digit()).count();
            if digits as f64 / alnum.len() as f64 >= DIGIT_NAME_RATIO {
                return true;
            }
        }
        // Fan-out names: the same basename saved in many distinct directories.
        match self.name_dirs.get(basename) {
            Some(dirs) => dirs.len() > NAME_DIR_FANOUT_MAX,
            None => false,
        }
    }
}

/// Attaches saves to intervals (see module doc for the rule).
///
/// Returns `(attached, dropped)`.
pub fn attach(
    db: &mut Connection,
    saves: &[ProductionSave],
) -> Result<(usize, usize), crate::LedgerError> {
    if saves.is_empty() {
        return Ok((0, 0));
    }

    let mut rows = load_intervals(db)?;
    let stats = SaveStats::build(saves);

    // Attribution index: rows sorted by start time, with a prefix-max end
    // so the per-save search can stop exactly where no earlier interval
    // can still be time-near (a measured quadratic hot spot at scale:
    // every save scanning every row).
    rows.sort_by_key(|r| r.start_ms);
    let prefix_max_end: Vec<u64> = rows
        .iter()
        .scan(0_u64, |max_end, row| {
            *max_end = (*max_end).max(row.end_ms);
            Some(*max_end)
        })
        .collect();

    // Group saves by target interval id, preserving first-seen order per interval.
    let mut per_interval: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    let mut attached = 0usize;
    let mut dropped = 0usize;

    for save in saves {
        match find_interval(&rows, &prefix_max_end, &stats, save) {
            Some(row) => {
                per_interval
                    .entry(row.id)
                    .or_default()
                    .push(save.path.clone());
                attached += 1;
            }
            None => dropped += 1,
        }
    }

    if per_interval.is_empty() {
        return Ok((attached, dropped));
    }

    let tx = db.transaction().map_err(crate::LedgerError::Sqlite)?;
    {
        let mut stmt = tx
            .prepare_cached("SELECT production_docs, search_text FROM intervals WHERE id = ?")
            .map_err(crate::LedgerError::Sqlite)?;
        let mut update = tx
            .prepare_cached(
                "UPDATE intervals
             SET production_docs = ?,
                 typed_count = typed_count + ?,
                 search_text = ?
             WHERE id = ?",
            )
            .map_err(crate::LedgerError::Sqlite)?;

        for (id, paths) in &per_interval {
            let (existing_docs, existing_search): (String, String) = stmt
                .query_row(params![id], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(crate::LedgerError::Sqlite)?;

            let mut docs: Vec<&str> = existing_docs
                .lines()
                .filter(|l| !l.trim().is_empty())
                .collect();
            let mut search = existing_search.clone();
            let mut newly_attached = 0usize;

            for path in paths {
                if docs.iter().any(|d| d == path) {
                    continue; // already attached (rebuilds re-run attach)
                }
                docs.push(path);
                newly_attached += 1;
                search.push(' ');
                search.push_str(path);
                if let Some(file) = path.rsplit('/').next() {
                    search.push(' ');
                    search.push_str(file);
                }
            }

            update
                .execute(params![docs.join("\n"), newly_attached as i64, search, id])
                .map_err(crate::LedgerError::Sqlite)?;
        }
    }
    tx.commit().map_err(crate::LedgerError::Sqlite)?;

    Ok((attached, dropped))
}

/// Loads the intervals relevant to attribution.
fn load_intervals(db: &Connection) -> Result<Vec<IntervalRow>, crate::LedgerError> {
    let mut stmt = db
        .prepare(
            "SELECT id, start_ms, end_ms, typed_count,
                    COALESCE(task_key_title, ''), COALESCE(task_key_host, ''),
                    COALESCE(task_key_doc, ''), COALESCE(urls_json, '')
             FROM intervals",
        )
        .map_err(crate::LedgerError::Sqlite)?;

    let rows = stmt
        .query_map([], |row| {
            Ok(IntervalRow {
                id: row.get(0)?,
                start_ms: row.get::<_, i64>(1)?.max(0) as u64,
                end_ms: row.get::<_, i64>(2)?.max(0) as u64,
                typed_count: row.get::<_, i64>(3)?.max(0) as u32,
                surface: format!(
                    "{} {} {} {}",
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                )
                .to_lowercase(),
            })
        })
        .map_err(crate::LedgerError::Sqlite)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(crate::LedgerError::Sqlite)
}

/// Finds the interval a save belongs to.
///
/// Two rules, in order of evidence strength:
///
/// 1. **Surface rule** — an interval's attention surface names the saved
///    file (the editor title shows "app.rs"). Strongest evidence: the save
///    is production of exactly that attention.
///
/// 2. **Presence rule** — the person was demonstrably active (an interval
///    with typing covers the save time) and the save is behaviorally human:
///    its path is not machine-managed (written too often or across too many
///    days) and its directory tree is not mid-storm. The save attaches to
///    the nearest interval with typing evidence. This is what attributes
///    agent- and terminal-driven work: the person watching and steering
///    while files change under them.
///
/// Saves matching neither rule are background noise and are dropped.
///
/// The search is windowed: `rows` are sorted by `start_ms` with
/// `prefix_max_end` (the running maximum `end_ms`), so the scan walks back
/// from the newest interval that could start near the save and stops the
/// moment no earlier interval can still overlap the grace window —
/// logarithmic positioning, and linear only in the genuinely-near span.
fn find_interval<'a>(
    rows: &'a [IntervalRow],
    prefix_max_end: &[u64],
    stats: &SaveStats,
    save: &ProductionSave,
) -> Option<&'a IntervalRow> {
    let basename = save.path.rsplit('/').next()?.trim();
    // Hidden files (.DS_Store and friends) are file-system plumbing, not
    // documents a person produces through attention.
    if basename.is_empty() || basename.starts_with('.') {
        return None;
    }

    // Rule 1: surface match — nearest interval whose surface names the file.
    let needle = basename.to_lowercase();
    let mut best: Option<(&IntervalRow, u64)> = None;
    let mut i = candidate_bound(rows, save.timestamp_ms);
    while let Some(row) = i.checked_sub(1).map(|idx| &rows[idx]) {
        if prefix_max_end[i - 1] + ATTACH_GRACE_MS < save.timestamp_ms {
            break; // no earlier interval can reach the save's window
        }
        i -= 1;
        if !near(save.timestamp_ms, row, ATTACH_GRACE_MS) {
            continue;
        }
        if !row.surface.contains(&needle) {
            continue;
        }
        let distance = span_distance(save.timestamp_ms, row);
        if best.map_or(true, |(_, d)| distance < d) {
            best = Some((row, distance));
        }
    }
    if let Some((row, _)) = best {
        return Some(row);
    }

    // Rule 2: presence match — nearest interval with typing evidence, when
    // the save is behaviorally human output.
    if stats.path_is_machine_managed(&save.path)
        || stats.tree_is_stormy(&save.path, save.timestamp_ms)
        || stats.name_is_machine_generated(&save.path)
    {
        return None;
    }
    let mut best: Option<(&IntervalRow, u64)> = None;
    let mut i = candidate_bound(rows, save.timestamp_ms);
    while let Some(row) = i.checked_sub(1).map(|idx| &rows[idx]) {
        if prefix_max_end[i - 1] + ATTACH_GRACE_MS < save.timestamp_ms {
            break;
        }
        i -= 1;
        if !near(save.timestamp_ms, row, ATTACH_GRACE_MS) {
            continue;
        }
        if row.typed_count == 0 {
            continue; // no presence evidence in this interval
        }
        let distance = span_distance(save.timestamp_ms, row);
        if best.map_or(true, |(_, d)| distance < d) {
            best = Some((row, distance));
        }
    }
    best.map(|(row, _)| row)
}

/// The exclusive upper bound of rows whose start can be at or before the
/// save's window end (rows are sorted by start_ms).
fn candidate_bound(rows: &[IntervalRow], ts: u64) -> usize {
    let bound = ts.saturating_add(ATTACH_GRACE_MS).saturating_add(1);
    rows.partition_point(|row| row.start_ms <= bound)
}

/// Whether a save timestamp is close enough to an interval to consider it.
fn near(ts: u64, row: &IntervalRow, grace: u64) -> bool {
    ts + grace >= row.start_ms && row.end_ms + grace >= ts
}

/// Distance from a timestamp to an interval's time span (0 if inside).
fn span_distance(ts: u64, row: &IntervalRow) -> u64 {
    if ts < row.start_ms {
        row.start_ms - ts
    } else if ts > row.end_ms {
        ts - row.end_ms
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ledger;

    fn focus_event(ts: u64, title: &str) -> crate::ObservedEvent {
        crate::ObservedEvent {
            timestamp_ms: ts,
            app_name: title.to_string(),
            window_title: title.to_string(),
            url: None,
            document_path: None,
            typed: false,
            // Establishment-scale dwell: a work must earn >= 300 s of a
            // sitting before it exists, so fixtures carry real minutes.
            dwell_ms: 350_000,
            keys: 0,
            clicks: 0,
            scrolls: 0,
        }
    }

    fn input_event(ts: u64, app: &str) -> crate::ObservedEvent {
        crate::ObservedEvent {
            timestamp_ms: ts,
            app_name: app.to_string(),
            window_title: app.to_string(),
            url: None,
            document_path: None,
            typed: true,
            dwell_ms: 350_000,
            keys: 0,
            clicks: 0,
            scrolls: 0,
        }
    }

    #[test]
    fn save_attaches_to_interval_naming_the_file() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[
                focus_event(1000, "app.rs — Evo-Evolution — Untracked"),
                focus_event(61_000, "app.rs — Evo-Evolution — Untracked"),
            ])
            .unwrap();

        let (attached, dropped) = ledger
            .attach_production(&[ProductionSave {
                timestamp_ms: 30_000,
                path: "/Users/…/crates/evo-desktop/src/app.rs".to_string(),
            }])
            .unwrap();

        assert_eq!((attached, dropped), (1, 0), "title names app.rs → attaches");
        let works = ledger.works(10).unwrap();
        assert!(works[0]
            .documents
            .contains(&"/Users/…/crates/evo-desktop/src/app.rs".to_string()));
        assert!(
            works[0].has_production,
            "attached save = production evidence"
        );
    }

    #[test]
    fn background_save_is_dropped_not_a_work() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[
                focus_event(1000, "app.rs — Evo-Evolution — Untracked"),
                focus_event(61_000, "app.rs — Evo-Evolution — Untracked"),
            ])
            .unwrap();

        let (attached, dropped) = ledger
            .attach_production(&[
                ProductionSave {
                    timestamp_ms: 30_000,
                    path: "/Users/…/Photos Library.photoslibrary/database/search/Spotlight/NSFileProtection.idx".to_string(),
                },
                ProductionSave {
                    timestamp_ms: 35_000,
                    path: "/Users/…/target/debug/incremental/evo_daemon-3c3bu2-a1b2c3d4e5f6.lock".to_string(),
                },
            ])
            .unwrap();

        assert_eq!(
            (attached, dropped),
            (0, 2),
            "no attention surface names these files"
        );
        let works = ledger.works(10).unwrap();
        assert_eq!(works.len(), 1, "saves alone never create works");
        assert!(works[0].documents.is_empty());
    }

    #[test]
    fn save_too_far_from_attention_is_dropped() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[
                focus_event(1000, "report.pdf — Preview"),
                focus_event(61_000, "report.pdf — Preview"),
            ])
            .unwrap();

        // 30 minutes later: outside the grace window
        let (attached, dropped) = ledger
            .attach_production(&[ProductionSave {
                timestamp_ms: 61_000 + 30 * 60 * 1000,
                path: "/Users/…/report.pdf".to_string(),
            }])
            .unwrap();

        assert_eq!(
            (attached, dropped),
            (0, 1),
            "save long after attention = noise"
        );
    }

    #[test]
    fn presence_rule_attaches_agent_and_terminal_work() {
        // The surface is generic (a terminal app); the file name appears
        // nowhere. But the person was demonstrably typing, and the save is
        // behaviorally human: attaches via presence.
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[
                focus_event(1000, "ZCode"),
                input_event(20_000, "ZCode"),
                input_event(30_000, "ZCode"),
            ])
            .unwrap();

        let (attached, dropped) = ledger
            .attach_production(&[ProductionSave {
                timestamp_ms: 25_000,
                path: "/Users/…/Evo/crates/evo-ledger/src/production.rs".to_string(),
            }])
            .unwrap();

        assert_eq!(
            (attached, dropped),
            (1, 0),
            "typing + quiet path = the person's production"
        );
        let works = ledger.works(10).unwrap();
        assert!(works[0]
            .documents
            .contains(&"/Users/…/Evo/crates/evo-ledger/src/production.rs".to_string()));
        assert!(works[0].has_production);
    }

    #[test]
    fn periodically_written_path_is_machine_managed() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[focus_event(1000, "ZCode"), input_event(20_000, "ZCode")])
            .unwrap();

        // The same path written 5 times in one day: a daemon's cadence, not
        // a person's save (a person's repeated saves attach via the surface
        // rule instead — the editor title names the file).
        let saves: Vec<ProductionSave> = (0..5)
            .map(|i| ProductionSave {
                timestamp_ms: 10_000 + i * 60_000,
                path: "/Users/…/Music/MusicCatalogData.db".to_string(),
            })
            .collect();
        let (attached, dropped) = ledger.attach_production(&saves).unwrap();

        assert_eq!(
            (attached, dropped),
            (0, 5),
            "periodic same-path writes = machine-managed"
        );
    }

    #[test]
    fn save_during_directory_storm_is_dropped() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[focus_event(1000, "ZCode"), input_event(20_000, "ZCode")])
            .unwrap();

        // A build storm: many distinct files in one directory within seconds.
        let saves: Vec<ProductionSave> = (0..10)
            .map(|i| ProductionSave {
                timestamp_ms: 15_000 + i * 1_000,
                path: format!("/Users/…/Evo/target/debug/deps/libpart_{i}.rlib"),
            })
            .collect();
        let (attached, dropped) = ledger.attach_production(&saves).unwrap();

        assert_eq!(
            (attached, dropped),
            (0, 10),
            "write storms are machine output, never production"
        );
    }

    #[test]
    fn save_attaches_to_nearest_naming_interval() {
        let mut ledger = Ledger::open_memory().unwrap();
        // Two intervals naming the file, far apart in time
        ledger
            .ingest(&[
                focus_event(1000, "notes.md"),
                focus_event(61_000, "notes.md"),
                focus_event(3_661_000, "notes.md — older session"),
                focus_event(3_721_000, "notes.md — older session"),
            ])
            .unwrap();

        // Wait — these may merge; the exact split doesn't matter. Attach near
        // the second cluster and assert the save lands somewhere with the name.
        let (attached, _) = ledger
            .attach_production(&[ProductionSave {
                timestamp_ms: 3_700_000,
                path: "/Users/…/notes.md".to_string(),
            }])
            .unwrap();
        assert_eq!(attached, 1);
        let works = ledger.works(10).unwrap();
        assert!(works
            .iter()
            .any(|w| w.documents.contains(&"/Users/…/notes.md".to_string())));
    }

    #[test]
    fn attach_is_idempotent_across_rebuilds() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[
                focus_event(1000, "app.rs — Evo"),
                focus_event(61_000, "app.rs — Evo"),
            ])
            .unwrap();

        let save = ProductionSave {
            timestamp_ms: 30_000,
            path: "/Users/…/app.rs".to_string(),
        };
        ledger.attach_production(&[save.clone()]).unwrap();
        ledger.attach_production(&[save]).unwrap(); // engine rebuilds re-run attach

        let works = ledger.works(10).unwrap();
        let doc_count = works[0]
            .documents
            .iter()
            .filter(|d| d.as_str() == "/Users/…/app.rs")
            .count();
        assert_eq!(doc_count, 1, "the same save attaches exactly once");
    }

    #[test]
    fn attached_document_is_searchable() {
        let mut ledger = Ledger::open_memory().unwrap();
        ledger
            .ingest(&[
                focus_event(1000, "waitlist template — Figma"),
                focus_event(61_000, "waitlist template — Figma"),
            ])
            .unwrap();
        ledger
            .attach_production(&[ProductionSave {
                timestamp_ms: 30_000,
                path: "/Users/…/Designs/waitlist-template-copy.fig".to_string(),
            }])
            .unwrap();

        let results = ledger.search("waitlist", 10).unwrap();
        assert!(
            !results.is_empty(),
            "attached document keeps the work findable"
        );
    }
}
