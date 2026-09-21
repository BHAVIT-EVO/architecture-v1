//! `evo-doctor` — engineering health check for an Evo installation.
//!
//! This is developer/support tooling, not product code. It reports the
//! operational state of a storage root without changing anything:
//!
//! - which storage root is in use;
//! - whether a daemon holds the single-instance lock (and its pid);
//! - what the daemon's own capture-status file reports;
//! - record counts per canonical log, and per retired log still on disk;
//! - whether the live derived index agrees, record for record, with a full
//!   replay of the Observation log (the replayability check).
//!
//! Usage:
//!   cargo run -p evo-daemon --example evo_doctor
//!   EVO_STORAGE_ROOT=/path/to/root cargo run -p evo-daemon --example evo_doctor
//!
//! Exits 0 when the root is consistent, 1 otherwise.

use evo_daemon::cache::CanonicalIndex;
use evo_daemon::daemon_lock::daemon_is_running;
use evo_daemon::daemon_status::{read_capture_sources, read_capture_status, CaptureStatus};
use evo_daemon::persistence::{
    load_artifact_locators, load_artifact_subjects, load_current_continuation_surface,
    load_current_designation, load_persisted_observations,
};
use evo_daemon::workspace_replay::{
    replay_continuation_surface_from_observations, replay_workspaces_from_observations,
};
use evo_storage::{Storage, StorageObjectKind};

fn main() {
    let root = evo_storage::canonical_storage_root();
    let mut problems: Vec<String> = Vec::new();

    println!("== Evo doctor ==");
    println!("storage root: {}", root.display());

    // 1. Daemon lock.
    if daemon_is_running(&root) {
        let pid = std::fs::read_to_string(root.join("daemon.pid"))
            .ok()
            .and_then(|text| text.trim().parse::<u32>().ok());
        match pid {
            Some(pid) => println!("daemon: running (pid {pid})"),
            None => println!("daemon: lock held but pid unreadable"),
        }
    } else {
        let stale = std::fs::read_to_string(root.join("daemon.pid"))
            .ok()
            .and_then(|text| text.trim().parse::<u32>().ok());
        match stale {
            Some(pid) => {
                println!("daemon: not running; stale lock left by dead pid {pid} (replaced on next start)");
                problems.push(format!("stale daemon lock present (pid {pid} is not alive)"));
            }
            None => println!("daemon: not running"),
        }
    }

    // 2. Capture status file (aggregate + per-source health).
    match read_capture_status(&root) {
        Some(CaptureStatus::Full) => println!("capture status: full"),
        Some(CaptureStatus::Partial { detail }) => {
            println!("capture status: partial — {detail}");
        }
        None => println!("capture status: none reported"),
    }
    for (source, state) in read_capture_sources(&root) {
        println!("  collector {source}: {state}");
    }

    // 3. Record counts per canonical log. A missing log is empty, not an error.
    for kind in [StorageObjectKind::Observation, StorageObjectKind::Artifact] {
        let count = count_records(&root, kind);
        println!("{kind:?}: {count} record(s)");
    }

    // Logs the retired persisted-formation model wrote. Nothing reads or
    // appends to them now — a body of work is derived from the Observation log,
    // never stored. Records already on disk are left exactly where they are
    // (the store is append-only), so report them as inert rather than pretend
    // the root is empty.
    for kind in [
        StorageObjectKind::Workspace,
        StorageObjectKind::Attachment,
        StorageObjectKind::Snapshot,
        StorageObjectKind::Restoration,
    ] {
        let count = count_records(&root, kind);
        if count > 0 {
            println!("{kind:?}: {count} record(s) — retired log, no longer written or read");
        }
    }

    // 4. Canonical replay loads.
    let observations = match load_persisted_observations(&root) {
        Ok(observations) => {
            println!("observations load: ok ({})", observations.len());
            observations
        }
        Err(err) => {
            problems.push(format!("observation log failed to load: {err}"));
            Vec::new()
        }
    };
    if let Err(err) = load_current_designation(&root) {
        problems.push(format!("designation failed to load: {err}"));
    } else {
        println!("designation load: ok");
    }
    if let Err(err) = load_current_continuation_surface(&root) {
        problems.push(format!("continuation surface failed to load: {err}"));
    } else {
        println!("continuation surface load: ok");
    }
    if let Err(err) = load_artifact_subjects(&root) {
        problems.push(format!("artifact subjects failed to load: {err}"));
    } else {
        println!("artifact subjects load: ok");
    }
    if let Err(err) = load_artifact_locators(&root) {
        problems.push(format!("artifact locators failed to load: {err}"));
    } else {
        println!("artifact locators load: ok");
    }

    // 5. Integrity: the long-lived derived index must agree with a full replay
    // of the same Observation log. The index is never canonical; this is the
    // check that the understanding it is serving is rebuildable from evidence
    // alone, and that tailing incrementally has not drifted from replaying.
    let index = match CanonicalIndex::new(&root) {
        Ok(index) => index,
        Err(err) => {
            problems.push(format!("derived index failed to build: {err}"));
            return finish(problems);
        }
    };
    let index_subject_ids: std::collections::BTreeSet<String> =
        index.subjects().keys().cloned().collect();
    let replay_subjects = load_artifact_subjects(&root);
    let replay_subject_ids: std::collections::BTreeSet<String> = replay_subjects
        .map(|subjects| subjects.into_keys().collect())
        .unwrap_or_default();
    if index_subject_ids != replay_subject_ids {
        problems.push("derived index and full replay disagree on artifact subjects".to_string());
    } else {
        println!(
            "index check: {} artifact subject(s) — consistent",
            index_subject_ids.len()
        );
    }

    // 6. Replayability (RFC-0003 Req 7, §19): re-derive the bodies of work from
    // the Observation log alone and require the result to equal the index's
    // understanding *value for value* — same identities, same members, same
    // roles, same sittings. Whole-value equality is the point: agreeing on
    // identities while disagreeing on what the work consists of would still be
    // a divergence between what a live session shows and what a restart
    // reconstructs.
    let replayed = replay_workspaces_from_observations(&observations);
    let mut replay_divergences: Vec<String> = Vec::new();
    for replayed_workspace in &replayed {
        match index
            .workspaces()
            .iter()
            .find(|workspace| workspace.id() == replayed_workspace.id())
        {
            Some(indexed) if indexed == replayed_workspace => {}
            Some(_) => replay_divergences.push(format!(
                "workspace {} differs between the derived index and a full replay",
                replayed_workspace.id()
            )),
            None => replay_divergences.push(format!(
                "full replay derived workspace {} that the index does not hold",
                replayed_workspace.id()
            )),
        }
    }
    for indexed in index.workspaces() {
        if !replayed
            .iter()
            .any(|workspace| workspace.id() == indexed.id())
        {
            replay_divergences.push(format!(
                "index holds workspace {} that a full replay of the Observation log does not derive",
                indexed.id()
            ));
        }
    }
    if replay_divergences.is_empty() {
        println!(
            "canonical replay check: {} body/bodies of work re-derived from the Observation log — consistent",
            replayed.len()
        );
    } else {
        println!("canonical replay check: {} divergence(s) found", replay_divergences.len());
        for divergence in &replay_divergences {
            problems.push(divergence.clone());
        }
    }

    // 7. Continuation-surface replay (RFC-0013): the derived current surface
    // must agree with the full replay of the same canonical Observation log.
    let replay_surface = replay_continuation_surface_from_observations(&observations);
    let index_surface = index.current_continuation_surface();
    if replay_surface != index_surface {
        problems.push(
            "derived index and full replay disagree on the current continuation surface".to_string(),
        );
    } else {
        match &index_surface {
            Some(subjects) => println!(
                "continuation surface replay check: consistent ({} declared subject(s))",
                subjects.len()
            ),
            None => println!("continuation surface replay check: none declared — consistent"),
        }
    }

    if !observations.is_empty() {
        println!("\nNOTE: every result above was derived from the canonical Observation log.");
    }

    finish(problems);
}

fn count_records(root: &std::path::Path, kind: StorageObjectKind) -> usize {
    let _guard = Storage::with_thread_root(root.to_path_buf());
    Storage::new()
        .read_all(kind)
        .map(|records| records.len())
        .unwrap_or(0)
}

fn finish(problems: Vec<String>) {
    if problems.is_empty() {
        println!("\nresult: OK — canonical state loads and the derived index matches the full replay");
        std::process::exit(0);
    }
    println!("\nresult: {} problem(s) found", problems.len());
    for problem in &problems {
        println!("  - {problem}");
    }
    std::process::exit(1);
}
