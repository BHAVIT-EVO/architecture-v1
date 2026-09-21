//! Benchmark: full-log reparse vs. incremental derived-index refresh.
//!
//! Evo's desktop shell reloads canonical state every ~2 seconds. Before the
//! derived index, every reload re-parsed the entire Observation log (and
//! re-derived subjects/locators/designation, then re-derived the bodies of work
//! from scratch) — O(N) per reload. This example measures that per-reload cost
//! against the incremental path: a long-lived index that tails only the records
//! appended since the last read, and re-derives the understanding only when
//! evidence actually arrived.
//!
//! Usage:
//!   cargo run -p evo-daemon --example bench_reload          # 50_000 observations
//!   cargo run -p evo-daemon --example bench_reload 200000   # 200_000 observations
//!
//! The numbers are printed, not asserted: this is developer tooling that
//! demonstrates the repeated-work reduction on the current machine.

use evo_daemon::cache::CanonicalIndex;
use evo_daemon::persistence::{
    current_designated_artifact, load_artifact_locators, load_artifact_subjects,
    load_current_designation, load_persisted_observations, persist_observation,
};
use evo_daemon::workspace_replay::replay_workspaces_from_root;
use evo_observation::evidence::{Evidence, FactValue, ObservedFact};
use evo_observation::observation::Observation;
use evo_observation::observation_id::ObservationId;
use evo_observation::observation_schema::ObservationSchema;
use evo_observation::provenance::{ObservationSource, Provenance};
use evo_storage::Storage;

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(50_000);
    let root = std::env::temp_dir().join(format!(
        "evo-bench-reload-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));

    let _guard = Storage::with_thread_root(root.clone());
    generate_observations(count);

    println!("== full-log reparse vs. incremental index refresh ==");
    println!(
        "storage root: {}",
        root.display()
    );
    println!("observations in log: {count}\n");

    // The OLD per-reload path: every reload re-parsed the full log through
    // every derivation helper, then re-derived the bodies of work from the
    // whole history (the desktop shell called all of these).
    let old_start = Instant::now();
    for _ in 0..3 {
        let _ = load_persisted_observations(&root).expect("observations");
        let _ = load_artifact_subjects(&root).expect("subjects");
        let _ = load_artifact_locators(&root).expect("locators");
        let _ = load_current_designation(&root).expect("designation");
        let _ = current_designated_artifact(&root).expect("designated");
        let _ = replay_workspaces_from_root(&root).expect("bodies of work");
    }
    let old_elapsed = old_start.elapsed() / 3;

    // The NEW path: one full replay at construction…
    let build_start = Instant::now();
    let mut index = CanonicalIndex::new(&root).expect("index builds");
    let build_elapsed = build_start.elapsed();

    // …then incremental refreshes that read only new records and re-derive the
    // understanding only when evidence arrived. With no new records, an idle
    // refresh is the steady-state per-2s reload cost, which is what the shell
    // spends almost all of its reloads doing.
    let refresh_start = Instant::now();
    for _ in 0..20 {
        index.refresh().expect("refresh succeeds");
    }
    let refresh_elapsed = refresh_start.elapsed() / 20;

    println!("old path, per reload (full reparse):   {old_elapsed:?}");
    println!("new path, full build (one-time replay): {build_elapsed:?}");
    println!("new path, per reload (incremental tail): {refresh_elapsed:?}");
    println!(
        "steady-state reduction: {:.1}x per reload",
        old_elapsed.as_nanos() as f64 / refresh_elapsed.as_nanos().max(1) as f64
    );

    // Clean up the benchmark root.
    drop(_guard);
    let _ = std::fs::remove_dir_all(&root);
}

/// Persists `count` genuine canonical Observations: a cycling set of distinct
/// window subjects plus a couple of designations, so the derived maps are
/// non-trivial. Every record is produced by the real persistence encoder.
fn generate_observations(count: usize) {
    let source = ObservationSource::new("bench_reload").expect("non-empty");
    let schema = ObservationSchema::window_focus_gained_v1();
    let fact_name = schema.canonical_fact_name().expect("window focus fact");

    for index in 0..count {
        // Cycle through 1_000 distinct subjects so identity derivation, the
        // subjects/locators maps, and engagement inference all have real work.
        //
        // The cycling index must be the WHOLE subject, not a suffix on a unique
        // one. An earlier version read `format!("Benchmark Window {index:05} —
        // subject {}", index % 1_000)`, whose leading unique index made every
        // one of the `count` subjects distinct; at 50_000 observations that is
        // 50_000 distinct resources and ~1.25 billion affinity pairs, so the
        // benchmark did not finish and measured nothing. Distinct-resource
        // scaling is measured on its own by `evo-engagement`'s
        // `bench_derivation`; what this benchmark isolates is per-reload cost
        // as the LOG grows, which needs the resource count held fixed.
        let subject = format!("Benchmark Window — subject {:05}", index % 1_000);
        let observed_at = UNIX_EPOCH.checked_add(Duration::from_secs(index as u64)).expect("time");
        let observation = Observation::new(
            ObservationId::new(),
            schema.clone(),
            Provenance::new(source.clone(), observed_at, HashMap::new()),
            Evidence::new(vec![ObservedFact::new(
                fact_name.clone(),
                FactValue::Text(subject),
            )
            .expect("fact")]),
        );
        persist_observation(&observation).expect("observation persists");
    }
}
