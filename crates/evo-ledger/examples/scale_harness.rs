//! Engine scaling harness: times one full engine cycle (parse → ingest →
//! attribution → grouping) over any observation log.
//!
//! Usage: `cargo run --release -p evo-ledger --example scale_harness -- <log-path>`
//!
//! The measured baseline (September 2026, median of 3): 1× (14.6 MB) ≈
//! 0.15 s, 10× ≈ 1.3 s, 20× ≈ 3.6 s before the attribution windowing —
//! after, the target is < 2 s at 20× (a year of capture).

use evo_ledger::hypotheses::Declarations;
use evo_ledger::observation_log::read_observation_log;
use evo_ledger::Ledger;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let path: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").expect("HOME");
            format!("{home}/Library/Application Support/evo/storage/observation.log")
        })
        .into();
    if !path.exists() {
        eprintln!("no log at {path:?}");
        std::process::exit(2);
    }
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    let t0 = Instant::now();
    let (events, saves, declared) = read_observation_log(&path);
    let parse = t0.elapsed();

    let t1 = Instant::now();
    let mut ledger = Ledger::open_memory().expect("in-memory ledger");
    for (a, b) in &declared {
        let _ = ledger.record_declaration(a, b);
    }
    let intervals = ledger.ingest(&events).expect("ingest");
    let ingest = t1.elapsed();

    let t2 = Instant::now();
    let (attached, dropped) = ledger.attach_production(&saves).unwrap_or((0, saves.len()));
    let attach = t2.elapsed();

    let t3 = Instant::now();
    let (works, proposals) = ledger
        .works_and_proposals(200)
        .unwrap_or_else(|_| (Vec::new(), Vec::new()));
    let works_time = t3.elapsed();
    let _ = Declarations::default();

    println!(
        "scale: {:>8.1} MB | parse {:.2}s ingest {:.2}s attach {:.2}s works {:.2}s | TOTAL {:.2}s \
         | {} events, {} saves -> {} intervals -> {} works, {} proposals",
        bytes as f64 / 1e6,
        parse.as_secs_f64(),
        ingest.as_secs_f64(),
        attach.as_secs_f64(),
        works_time.as_secs_f64(),
        (parse + ingest + attach + works_time).as_secs_f64(),
        events.len(),
        saves.len(),
        intervals,
        works.len(),
        proposals.len(),
    );
}
