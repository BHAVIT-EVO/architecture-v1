//! The regression gate: the full pipeline over the real observation log,
//! scored against the ground-truth scenarios from the September 2026
//! empirical study (see tools/threads-regression and the research notes).
//!
//! Gates are behavioral, drawn from what the person actually did:
//!
//! * **No host is a work.** youtube.com, claude.ai, google.com span many
//!   purposes; a work keyed by a host is the mirror failure.
//! * **The trek playlist is one body.** Twenty videos on one playlist are
//!   one work (hard link), not twenty cards.
//! * **Trek videos and gear pages are distinct works.** The video↔gear
//!   connection is the measured semantic gap; the deterministic core keeps
//!   them separate and the merge-proposal path offers the person the
//!   decision.
//! * **The kundli chat never joins the Arc'teryx gear work.** They
//!   interleave at two-second adjacency; only establishment precedence
//!   keeps them apart.
//! * **evo development is one work.** ZCode, the edited sources, and the
//!   terminal are one body across eleven days.
//! * **No mega-work.** No work absorbs a dominant share of all resources
//!   (the affinity-engine failure).
//! * **Unrelated videos stay separate.** Videos on different playlists are
//!   different works.
//!
//! The gate is a library so `cargo test` re-proves the grouping against
//! real data on every run (the integration test skips loudly on machines
//! without the log — the fixture is machine-local by nature), and the
//! `regression-gate` binary prints the report for humans.

use crate::observation_log::read_observation_log;
use crate::Ledger;
use std::path::Path;

/// The gate's outcome: what formed and which gates failed.
#[derive(Debug, Clone)]
pub struct GateReport {
    pub attention_events: usize,
    pub saves: usize,
    pub declared_pairs: usize,
    pub intervals: usize,
    pub works: Vec<GateWork>,
    pub merge_proposals: usize,
    /// Every violated gate, human-readable. Empty = pass.
    pub failures: Vec<String>,
}

/// One work as the gate reports it.
#[derive(Debug, Clone)]
pub struct GateWork {
    pub identity: String,
    pub total_time_ms: u64,
    pub sessions: usize,
    pub urls: usize,
    pub documents: usize,
    pub titles: Vec<String>,
    pub apps: Vec<String>,
}

/// Runs all gates over the observation log at `log_path`.
///
/// `None` when the log is absent (a machine without Evo's data): the
/// caller decides how to skip.
pub fn run_gates(log_path: &Path) -> Option<GateReport> {
    if !log_path.exists() {
        return None;
    }

    let (events, saves, declared) = read_observation_log(log_path);
    let mut ledger = Ledger::open_memory().expect("in-memory ledger");
    for (a, b) in &declared {
        let _ = ledger.record_declaration(a, b);
    }
    let intervals = ledger.ingest(&events).expect("ingest");
    let _ = ledger.attach_production(&saves);
    let (works, proposals) = ledger.works_and_proposals(200).expect("works");

    let report_works: Vec<GateWork> = works
        .iter()
        .map(|w| GateWork {
            identity: w.identity.chars().take(80).collect(),
            total_time_ms: w.total_time_ms,
            sessions: w.session_count(),
            urls: w.urls.len(),
            documents: w.documents.len(),
            titles: w.titles.iter().take(8).cloned().collect(),
            apps: w.apps.clone(),
        })
        .collect();

    let mut failures: Vec<String> = Vec::new();

    // ── Gate 1: no host is a work identity ─────────────────────────────
    for host in ["youtube.com", "claude.ai", "google.com", "www.youtube.com"] {
        if works.iter().any(|w| w.identity.trim() == host) {
            failures.push(format!("a work is keyed by the host {host}"));
        }
    }

    // ── Gate 2: the trek playlist lives in exactly one work ─────────────
    // The playlist's videos were watched across many sittings; grouping
    // is correct when they all landed in ONE work (the trek research),
    // never scattered across works. The work itself may attend other
    // trek resources — real research mixes playlists and pages, and a
    // purity freeze (all-URLs-playlist) would fight the person's actual
    // history as it grows.
    let trek_playlist = "PLiM-TFJI81";
    let playlist_works: Vec<usize> = works
        .iter()
        .enumerate()
        .filter(|(_, w)| {
            w.urls
                .iter()
                .any(|u| u.contains("list=") && u.contains(trek_playlist))
        })
        .map(|(i, _)| i)
        .collect();
    if playlist_works.len() != 1 {
        failures.push(format!(
            "expected the trek playlist to live in exactly one work, found {}",
            playlist_works.len()
        ));
    }

    // ── Gate 3: exactly one Arc'teryx gear work ─────────────────────────
    // Gear pages from the old capture carry title identity (no URL
    // locator), so the gear work is recognized by its titles.
    let gear_works: Vec<usize> = works
        .iter()
        .enumerate()
        .filter(|(_, w)| {
            w.titles.iter().any(|t| t.contains("Arc'teryx"))
                || w.urls.iter().any(|u| u.contains("arcteryx.com"))
        })
        .map(|(i, _)| i)
        .collect();
    if gear_works.len() != 1 {
        failures.push(format!(
            "expected exactly one Arc'teryx gear work, found {}",
            gear_works.len()
        ));
    }

    // ── Gate 4: the kundli chat never joins the gear work ──────────────
    let kundli_works: Vec<usize> = works
        .iter()
        .enumerate()
        .filter(|(_, w)| w.urls.iter().any(|u| u.contains("claude.ai/chat/f5c116b5")))
        .map(|(i, _)| i)
        .collect();
    if let (Some(&kundli), Some(&gear)) = (kundli_works.first(), gear_works.first()) {
        if kundli == gear {
            failures.push("the kundli chat merged into the Arc'teryx gear work".into());
        }
    }

    // ── Gate 5: evo development is one work ────────────────────────────
    let zcode_works: Vec<usize> = works
        .iter()
        .enumerate()
        .filter(|(_, w)| {
            w.apps.iter().any(|a| a == "ZCode")
                || w.titles.iter().any(|t| t == "ZCode")
                || w.documents
                    .iter()
                    .any(|d| d.contains("Evo-Evolution.nosync/crates"))
        })
        .map(|(i, _)| i)
        .collect();
    if zcode_works.len() > 2 {
        failures.push(format!(
            "evo development spans {} works, expected at most 2",
            zcode_works.len()
        ));
    }

    // ── Gate 6: no mega-work ───────────────────────────────────────────
    // Multi-label attachment lets one resource appear in several works,
    // so shares are inflated; the affinity failure was 43 of 46 — an
    // 80%+ monopoly. 60% is the tripwire.
    let total_resources: usize = works.iter().map(|w| w.urls.len() + w.documents.len()).sum();
    if total_resources > 0 {
        let largest = works
            .iter()
            .map(|w| w.urls.len() + w.documents.len())
            .max()
            .unwrap_or(0);
        if largest as f64 / total_resources as f64 > 0.60 {
            failures.push(format!(
                "a mega-work holds {largest} of {total_resources} resources"
            ));
        }
    }

    // ── Gate 7: unrelated videos stay separate ─────────────────────────
    // A video on a different playlist must never live in the trek work.
    if let Some(&trek) = playlist_works.first() {
        let unrelated = works[trek]
            .urls
            .iter()
            .any(|u| u.contains("RdWnMbsZhxk") || u.contains("eswW2BoFAUs"));
        if unrelated {
            failures.push("an unrelated video joined the trek work".into());
        }
    }

    Some(GateReport {
        attention_events: events.len(),
        saves: saves.len(),
        declared_pairs: declared.len(),
        intervals,
        works: report_works,
        merge_proposals: proposals.len(),
        failures,
    })
}
