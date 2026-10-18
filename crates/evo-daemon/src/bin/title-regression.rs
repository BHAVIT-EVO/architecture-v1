//! Reports title-quality metrics over the live canonical observation log.
//!
//! This deliberately scores the final Home titles rather than an intermediate
//! naming helper. It is a report, not an identity gate: an empty or stale log
//! is reported honestly and never treated as evidence of quality.

use evo_daemon::persistence::load_persisted_observations;
use evo_daemon::threads::display_threads_from_storage;
use evo_daemon::work_understanding::title_metrics;
use std::path::PathBuf;

fn main() {
    let root = std::env::var_os("EVO_STORAGE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(evo_storage::canonical_storage_root);
    let observations = match load_persisted_observations(&root) {
        Ok(observations) => observations,
        Err(error) => {
            eprintln!("TITLE-REGRESSION: could not read {root:?}: {error}");
            std::process::exit(2);
        }
    };
    let works = match display_threads_from_storage(&root) {
        Ok(works) => works,
        Err(error) => {
            eprintln!("TITLE-REGRESSION: could not derive Home titles: {error}");
            std::process::exit(2);
        }
    };
    let metrics = title_metrics(works.iter().map(|work| work.name.as_str()));
    println!(
        "TITLE-REGRESSION: {} canonical observations, {} presented works",
        observations.len(),
        metrics.presented_work_count
    );
    println!(
        "TITLE-REGRESSION: {} Untitled ({:.2}%), {} identifiers ({:.2}%), readability {:.2}%",
        metrics.untitled_count,
        metrics.untitled_rate() * 100.0,
        metrics.identifier_count,
        metrics.identifier_rate() * 100.0,
        metrics.readability_rate() * 100.0,
    );
    println!(
        "TITLE-REGRESSION: identifier floor {}",
        if metrics.identifier_floor_met() {
            "PASS"
        } else {
            "FAIL"
        }
    );
    if !metrics.identifier_floor_met() {
        std::process::exit(1);
    }
}
