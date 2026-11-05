//! The committed ground-truth regression, as a `cargo test` gate.
//!
//! `crates/evo-threads/src/bin/regression.rs` is the same gate with a
//! human-readable report and an exit code; this test is its face inside the
//! normal test suite, so `cargo test --workspace` re-proves the engine
//! against the founder's hand labels on every run. The corpus is committed
//! (`tools/threads-regression/`), so this runs on every machine — that is
//! the point of it being the shadow evaluator's floor.
//!
//! What is asserted where, and why:
//! * **Hard:** the corpus is the real input-capture era (Typed events
//!   present), the engine reconstructs threads from it, the reconstruction is
//!   reproducible, and the two safety properties hold: no thread fuses two
//!   distinct labeled works, and no labeled leisure resource ever anchors a
//!   thread. Phase 1's input/naming capture removed the old corpus's
//!   data-coverage artifact (attention-only surfaces whose placement could
//!   not be tested); with placement claims testable, violating them is a
//!   regression.
//! * The corpus window (Sep 13–15, 2026 — the log's input-capture era) was
//!   chosen because that is where the engine's placement claims are
//!   disambiguated by Typed evidence. Regenerate with
//!   `tools/threads-regression/export_observation_log.py` using the flags in
//!   docs/BUILD-LOG.md.

use evo_threads::{
    ground_truth::{canonical_label, is_work_label},
    Declaration, Engine, Event, Interaction, Origin, ThreadId,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

/// The event record shapes the corpus uses:
///   [timestamp_ms, origin, interaction, resource, optional_weight]
/// and declaration rows, which carry the SameWork pair in slots 5–6:
///   [timestamp_ms, "Person", "Declared", "", null, resourceA, resourceB]
fn parse_events(raw: &str) -> Vec<Event> {
    let parsed: Vec<serde_json::Value> =
        serde_json::from_str(raw).expect("the committed corpus parses");
    parsed
        .iter()
        .map(|v| Event {
            timestamp: v[0].as_u64().unwrap_or(0),
            origin: match v[1].as_str().unwrap_or("Person") {
                "Person" => Origin::Person,
                "System" => Origin::System,
                _ => Origin::Execution,
            },
            interaction: match v[2].as_str().unwrap_or("Focused") {
                "Downloaded" => Interaction::Downloaded,
                "Typed" => Interaction::Typed,
                "Saved" => Interaction::Saved,
                "Mutated" => Interaction::Mutated,
                "Visited" => Interaction::Visited,
                "CommandRan" => Interaction::CommandRan,
                "Declared" => Interaction::Declared,
                _ => Interaction::Focused,
            },
            resource: v[3].as_str().unwrap_or("").to_string(),
            weight: v.get(4).and_then(|w| w.as_u64()),
            detail: match v[2].as_str() {
                Some("Declared") => Some(Declaration::SameWork {
                    a: v[5].as_str().unwrap_or("").to_string(),
                    b: v[6].as_str().unwrap_or("").to_string(),
                }),
                _ => None,
            },
        })
        .collect()
}

#[test]
fn ground_truth_leisure_never_anchors_and_reported_works_stay_coherent() {
    let repo = env!("CARGO_MANIFEST_DIR").trim_end_matches("/crates/evo-threads");
    let events_raw = fs::read_to_string(format!(
        "{repo}/tools/threads-regression/sample_events.json"
    ))
    .expect("the committed corpus ships with the repo");
    let labels_raw = fs::read_to_string(format!(
        "{repo}/tools/threads-regression/ground_truth_labels.json"
    ))
    .expect("the committed labels ship with the repo");

    let events = parse_events(&events_raw);
    assert!(
        events.len() > 100,
        "the corpus must be the real recorded day, not a stub"
    );
    assert!(
        events.iter().any(|e| e.interaction == Interaction::Typed),
        "the corpus must be the input-capture era (Typed events present); \
         without input evidence, placement claims are untestable"
    );

    let mut engine = Engine::new();
    for event in &events {
        engine.apply(event.clone());
    }
    // Settle one day past the last event so the final sitting is visible —
    // the same boundary the binary and the daemon read path use.
    let last_ts = events.last().map(|e| e.timestamp).unwrap_or(0);
    engine.settle(last_ts + 86_400_000);

    // The founder's hand labels, canonicalized.
    let labels: Vec<serde_json::Value> = serde_json::from_str(&labels_raw).unwrap_or_default();
    let truth: BTreeMap<String, String> = labels
        .iter()
        .filter_map(|g| {
            let label = g["label"].as_str()?;
            (!label.is_empty()).then(|| {
                (
                    g["resource"].as_str().unwrap_or("").trim().to_string(),
                    canonical_label(label),
                )
            })
        })
        .collect();
    assert!(
        truth.len() > 20,
        "the labels must be the real hand labels, not a stub"
    );

    // Resource → thread, from the engine's own views (the same read the
    // binary scores with).
    let threads = engine.threads();
    let mut res2thread: BTreeMap<String, ThreadId> = BTreeMap::new();
    for t in &threads {
        for ep in &t.episodes {
            for (r, _) in &ep.participants {
                res2thread.entry(r.clone()).or_insert(t.id);
            }
        }
        for a in &t.anchors {
            res2thread.entry(a.resource.clone()).or_insert(t.id);
        }
        for (r, _) in &t.companions {
            res2thread.entry(r.clone()).or_insert(t.id);
        }
    }

    // Safety property 1 (hard since Phase 1): leisure resources must never
    // anchor a thread. The committed window is the input-capture era — Typed
    // evidence exists — so placement claims are testable and violating this
    // is a regression, not a coverage artifact.
    let leisure_anchored: Vec<&String> = threads
        .iter()
        .flat_map(|t| t.anchors.iter())
        .filter(|a| {
            truth
                .get(&a.resource)
                .map(|w| w == "leisure")
                .unwrap_or(false)
        })
        .map(|a| &a.resource)
        .collect();
    assert!(
        leisure_anchored.is_empty(),
        "labeled leisure resources anchored threads: {leisure_anchored:?}"
    );

    // Safety property 2 (hard since Phase 1): no thread fuses two distinct
    // labeled works. With Typed evidence in the corpus, identity comes from
    // production; an engine that glues two works into one thread is wrong
    // here, not data-limited. Fusion counts work labels only — noise/comms
    // decorating a thread is context, never a second work (the same rule the
    // binary's exit gate applies; the two can never drift).
    let mut thread_works: BTreeMap<ThreadId, BTreeSet<String>> = BTreeMap::new();
    for (r, w) in &truth {
        if !is_work_label(w) {
            continue;
        }
        if let Some(t) = res2thread.get(r) {
            thread_works.entry(*t).or_default().insert(w.clone());
        }
    }
    let fused_threads: Vec<(&ThreadId, &BTreeSet<String>)> =
        thread_works.iter().filter(|(_, ws)| ws.len() > 1).collect();
    assert!(
        fused_threads.is_empty(),
        "threads fusing distinct labeled works: {fused_threads:?}"
    );
    assert!(
        !threads.is_empty(),
        "a real recorded day must reconstruct at least one thread"
    );
    // The reproducibility gate: the same log replays to the same threads.
    let mut engine_again = Engine::new();
    for event in &events {
        engine_again.apply(event.clone());
    }
    engine_again.settle(last_ts + 86_400_000);
    let ids_first: Vec<u64> = threads.iter().map(|t| t.id).collect();
    let ids_again: Vec<u64> = engine_again.threads().iter().map(|t| t.id).collect();
    assert_eq!(
        ids_first, ids_again,
        "the same corpus must reconstruct identical thread lineage"
    );
    println!(
        "ground_truth: threads={} fused_threads={} leisure_anchored={}",
        threads.len(),
        fused_threads.len(),
        leisure_anchored.len()
    );
    println!(
        "  (fusion/leisure hard-asserted since Phase 1: the committed window \
         is the input-capture era, so placement claims are testable)"
    );
}
