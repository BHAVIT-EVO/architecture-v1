//! Evo Ledger Engine — the standalone compute process.
//!
//! Screenpipe architecture: engine process → database → UI reads database.
//!
//! Data sources (in priority order):
//! 1. observation.log — the canonical append-only log (primary source).
//!    Contains window focus events, URL navigations, file saves, and
//!    input activity (keystroke counts) from the input counter.
//! 2. evo.db (legacy) — if the old daemon's canonical_events table exists,
//!    we read from it as a supplemental source. If not (schema changed),
//!    we skip it gracefully.
//!
//! The engine:
//! 1. Reads attention events (focus, URL, input) from observation.log
//! 2. Optionally reads from evo.db if available
//! 3. Merges and sorts by timestamp
//! 4. Computes work intervals via the identity chain — attention only
//! 5. Attributes file saves to the intervals whose attention surface names
//!    the saved file (saves never create Works; background writes drop)
//! 6. Writes Works to ledger.db with FTS5 search index
//! 7. Runs continuously (10-second polling cycle)

use std::path::PathBuf;
use std::time::Duration;

fn main() {
    // The storage root: the desktop passes it (so a bundled app can relocate
    // storage), or it is the canonical location. The ledger database and
    // the engine pid file live INSIDE the storage root on purpose: the
    // capture layer suppresses its own writes there (the self-reference
    // boundary), and the engine rebuilds the database every cycle —
    // outside the root those writes would feed back through the file
    // watcher as a record storm (Evo witnessing itself).
    let storage_root = match std::env::var_os("EVO_STORAGE_ROOT") {
        Some(root) => PathBuf::from(root),
        None => {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join("Library/Application Support/evo/storage")
        }
    };
    let data_dir = storage_root
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| storage_root.clone());
    let db_path = data_dir.join("evo.db");
    let ledger_path = storage_root.join("ledger.db");
    let observation_log = storage_root.join("observation.log");
    let pid_path = storage_root.join("ledger-engine.pid");

    // One engine per data directory: a second instance would race the
    // ledger rebuild. A stale pid file (crashed engine) is detected by
    // liveness, not by file presence, so crashes never wedge the system.
    if let Some(running) = running_engine_pid(&pid_path) {
        println!("EVO-LEDGER-ENGINE already running as pid {running}; exiting");
        return;
    }
    let _ = std::fs::write(&pid_path, std::process::id().to_string());
    println!("EVO-LEDGER-ENGINE starting (pid {})", std::process::id());
    println!("  Observation log: {observation_log:?}");
    println!("  Legacy SQLite: {db_path:?}");
    println!("  Ledger: {ledger_path:?}");

    let mut last_observation_len: u64 = 0;

    loop {
        std::thread::sleep(Duration::from_secs(10));

        // ─── Check for new data ────────────────────────────────────────
        let obs_len = std::fs::metadata(&observation_log)
            .map(|m| m.len())
            .unwrap_or(0);

        if obs_len == last_observation_len {
            continue; // No new data
        }
        last_observation_len = obs_len;

        // ─── Read attention events + file saves from observation.log ───
        let (mut events, saves, declared) =
            evo_ledger::observation_log::read_observation_log(&observation_log);
        let obs_count = events.len();
        let save_count = saves.len();

        // ─── Optionally read from legacy SQLite (supplemental) ─────────
        let legacy_events = read_legacy_sqlite(&db_path);
        let legacy_count = legacy_events.len();
        events.extend(legacy_events);

        if events.is_empty() {
            println!("EVO-LEDGER-ENGINE: no events found (obs: {obs_len} bytes)");
            continue;
        }

        // Sort by timestamp
        events.sort_by_key(|e| e.timestamp_ms);
        let mut sorted_saves = saves;
        sorted_saves.sort_by_key(|s| s.timestamp_ms);

        // ─── Rebuild ledger fresh (prevents duplication) ───────────────
        // Remove sidecar files too: a stale -wal against a fresh -shm/main
        // is a corruption hazard on the next open.
        let _ = std::fs::remove_file(&ledger_path);
        let _ = std::fs::remove_file(storage_root.join("ledger.db-wal"));
        let _ = std::fs::remove_file(storage_root.join("ledger.db-shm"));
        let mut ledger = match evo_ledger::Ledger::open(&ledger_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("EVO-LEDGER-ENGINE: ledger open failed: {e}");
                continue;
            }
        };

        // The person's declarations are ground truth: recorded before
        // grouping so this cycle's works already honor them.
        for (a, b) in &declared {
            let _ = ledger.record_declaration(a, b);
        }

        match ledger.ingest(&events) {
            Ok(intervals) => {
                // Attribute production: saves attach to attention, or drop.
                let (attached, dropped) = ledger
                    .attach_production(&sorted_saves)
                    .unwrap_or((0, save_count));

                // No prune here: the database is rebuilt from scratch every
                // cycle, so retention pruning (and its VACUUM, which rewrites
                // the whole file) would be pure write amplification — and
                // every write the engine makes is filesystem churn the
                // capture layer has to ignore.
                let (works, proposals) = ledger.works_and_proposals(30).unwrap_or_default();
                // Deterministic merge proposals, offered to the person: a
                // confirmed merge returns as a declaration next cycle.
                //
                // The proposal's subjects are each work's most-dwelt
                // member's RAW witness (the URL, document path, or window
                // title the capture layer actually recorded) — never a
                // lexicographic pick: the BTreeSet's first URL can be a
                // channel page rather than the work's own body, and
                // title-keyed works (old records without URL locators)
                // have no URLs at all. The raw witness is what both the
                // daemon socket (lenient) and the engine's
                // canonical_subject map onto the work's pages.
                ledger.clear_merge_proposals();
                for proposal in &proposals {
                    let (Some(a), Some(b)) =
                        (works.get(proposal.work_a), works.get(proposal.work_b))
                    else {
                        continue;
                    };
                    let subject_a = representative_subject(a);
                    let subject_b = representative_subject(b);
                    if subject_a.is_empty() || subject_b.is_empty() {
                        continue;
                    }
                    let _ = ledger.record_merge_proposal(
                        &subject_a,
                        &subject_b,
                        &a.identity,
                        &b.identity,
                        proposal.shared_sittings,
                    );
                }
                let ledger_size = std::fs::metadata(&ledger_path)
                    .map(|m| m.len())
                    .unwrap_or(0);
                println!(
                    "EVO-LEDGER-ENGINE: {} attention events ({}, {} legacy) → {} intervals; {} saves ({} attached, {} background) → {} works [{} KB]",
                    events.len(), obs_count, legacy_count,
                    intervals, save_count, attached, dropped,
                    works.len(), ledger_size / 1024,
                );
                for work in works.iter().take(8) {
                    println!(
                        "  [{}] {} — {} across {} sessions",
                        work.identity_kind.label(),
                        work.title().chars().take(50).collect::<String>(),
                        work.time_summary(),
                        work.session_count()
                    );
                }
            }
            Err(e) => {
                eprintln!("EVO-LEDGER-ENGINE: ingest failed: {e}");
            }
        }
    }
}

// ─── Merge-proposal subjects ───────────────────────────────────────────────

/// The raw witness of a work's most-dwelt member: the URL, document path,
/// or window title the capture layer recorded for the page that carried
/// the most attention. Declarations round-trip through the daemon socket
/// and the engine's `canonical_subject`, both of which reason over these
/// raw forms — so the proposal must speak the same vocabulary.
fn representative_subject(work: &evo_ledger::works::Work) -> String {
    use std::collections::BTreeMap;
    // Page key -> (dwell, raw witness). The interval's own urls/documents
    // are the raw witnesses; for title-keyed pages the title itself.
    let mut pages: BTreeMap<String, (u64, String)> = BTreeMap::new();
    for interval in &work.intervals {
        let key = interval
            .task_key
            .page
            .clone()
            .or_else(|| interval.task_key.document_path.clone())
            .or_else(|| interval.task_key.window_title.clone())
            .unwrap_or_else(|| interval.task_key.app_name.clone());
        let witness = interval
            .urls
            .first()
            .cloned()
            .or_else(|| interval.document_paths.first().cloned())
            .or_else(|| interval.task_key.window_title.clone())
            .unwrap_or_else(|| interval.task_key.app_name.clone());
        let entry = pages.entry(key).or_insert((0, witness));
        entry.0 += interval.total_dwell_ms;
    }
    pages
        .into_iter()
        .max_by_key(|(_, (dwell, _))| *dwell)
        .map(|(_, (_, witness))| witness)
        .unwrap_or_default()
}

// ─── Single-instance guard ─────────────────────────────────────────────────

/// The pid recorded in the pid file, when that process is still alive.
fn running_engine_pid(pid_path: &std::path::Path) -> Option<u32> {
    let contents = std::fs::read_to_string(pid_path).ok()?;
    let pid: u32 = contents.trim().parse().ok()?;
    // `kill -0` tests liveness without signalling: exit 0 = alive.
    let status = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .ok()?;
    if status.success() {
        Some(pid)
    } else {
        None
    }
}

// ─── Legacy SQLite reader (SUPPLEMENTAL — optional) ──────────────────────

fn read_legacy_sqlite(db_path: &std::path::Path) -> Vec<evo_ledger::ObservedEvent> {
    let check = std::process::Command::new("sqlite3")
        .arg(db_path)
        .arg("SELECT COUNT(*) FROM sqlite_master WHERE name='canonical_events'")
        .output()
        .ok();

    let Some(check) = check else {
        return Vec::new();
    };
    if !check.status.success() {
        return Vec::new();
    }
    let table_exists = String::from_utf8_lossy(&check.stdout).trim() == "1";
    if !table_exists {
        return Vec::new();
    }

    let output = std::process::Command::new("sqlite3")
        .arg("-json")
        .arg(db_path)
        .arg(
            r#"
            SELECT ts_ms, app_id, window_title,
                   COALESCE(normalized_url, '') as url,
                   COALESCE(download_path, '') as doc,
                   input_chars, dwell_ms
            FROM canonical_events
            WHERE ts_ms > (SELECT MAX(ts_ms) - 604800000 FROM canonical_events)
            ORDER BY ts_ms
        "#,
        )
        .output()
        .ok();

    let Some(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let json = String::from_utf8_lossy(&output.stdout);
    let mut events = Vec::new();
    for line in json.lines() {
        let trimmed = line.trim().trim_start_matches('[').trim_end_matches(']');
        if !trimmed.starts_with('{') {
            continue;
        }
        let ts = json_u64(trimmed, "ts_ms").unwrap_or(0);
        if ts == 0 {
            continue;
        }
        let app = json_str(trimmed, "app_id").unwrap_or_default();
        if app.is_empty() {
            continue;
        }
        events.push(evo_ledger::ObservedEvent {
            timestamp_ms: ts,
            app_name: app,
            window_title: json_str(trimmed, "window_title").unwrap_or_default(),
            url: json_str(trimmed, "url").filter(|u| !u.is_empty()),
            document_path: json_str(trimmed, "doc").filter(|d| !d.is_empty()),
            typed: json_u64(trimmed, "input_chars").unwrap_or(0) > 0,
            dwell_ms: json_u64(trimmed, "dwell_ms").unwrap_or(0).min(600_000),
            keys: 0,
            clicks: 0,
            scrolls: 0,
        });
    }
    events
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn unhex(hex: &str) -> String {
    evo_ledger::observation_log::unhex(hex)
}

fn json_u64(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\":");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
