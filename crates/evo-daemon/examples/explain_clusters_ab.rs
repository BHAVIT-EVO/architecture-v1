//! Clustering A/B — the *same* real history reconstructed under the old
//! connectivity rule and the new one, side by side, so the difference is the
//! rule and not the data.
//!
//! `reciprocal_rank` controls stage-1 clustering connectivity in
//! `cluster_within`. Setting it larger than any resource's degree makes every
//! above-floor edge "mutually strongest", which reproduces the old rule
//! (connected components over the whole above-floor graph) exactly. Setting it
//! to the default (2) keeps only edges that are, for both loci, among their
//! strongest few. This example runs several values against one decode of the
//! canonical store and prints, for each, the work surface (what Home would show)
//! and the largest threads (so an over-admitting mega-thread is visible as a
//! count). Read-only: opens the canonical root, decodes, derives, prints.
//!
//! ```text
//! cargo run -p evo-daemon --example explain_clusters_ab
//! ```

use evo_engagement::{Engagement, EngagementParams, Reconstruction, Standing};
use evo_storage::canonical_storage_root;

fn main() {
    let root = canonical_storage_root();
    let observations = match evo_daemon::persistence::load_persisted_observations(&root) {
        Ok(observations) => observations,
        Err(error) => {
            println!("could not read the Observation log: {error:?}");
            return;
        }
    };
    if observations.is_empty() {
        println!("no observations recorded on this machine");
        return;
    }

    println!("================================================================");
    println!("EVO — CLUSTERING A/B (same {} observations)", observations.len());
    println!("root: {}", root.display());
    println!("================================================================");

    // usize::MAX as reciprocal_rank ≡ the old rule: every above-floor edge is
    // mutually "strongest", so components form over the whole above-floor graph.
    for (label, rank) in [
        ("OLD  (rank=∞, whole above-floor graph)", usize::MAX),
        ("k=1  (mutual best only)", 1),
        ("NEW  (rank=2, default)", 2),
        ("k=3", 3),
    ] {
        let params = EngagementParams {
            reciprocal_rank: rank,
            ..EngagementParams::default()
        };
        let reconstruction = Reconstruction::from_observations(&observations, params);
        let set = reconstruction.set();

        println!();
        println!("----------------------------------------------------------------");
        println!(
            "{label}   ->  threads {} (work {}, remembered {})",
            set.engagements().len(),
            set.work().len(),
            set.remembered().len(),
        );

        // The work surface: what Home would present, in Home's own order.
        println!("  WORK SURFACE (Home):");
        if set.work().is_empty() {
            println!("      (nothing presentable)");
        }
        for engagement in set.work() {
            println!(
                "      [{}] {:>3} members  {}s  \"{}\"",
                standing_label(engagement.standing()),
                engagement.participants().len(),
                engagement.attention().as_secs(),
                trim(engagement.title(), 58),
            );
        }

        // The largest threads regardless of standing — an over-admitting blob
        // shows here as a big member count whether or not it is called work.
        let mut by_size: Vec<&Engagement> = set.engagements().iter().collect();
        by_size.sort_by(|a, b| b.participants().len().cmp(&a.participants().len()));
        println!("  LARGEST THREADS (any standing):");
        for engagement in by_size.into_iter().take(6) {
            println!(
                "      [{}] {:>3} members  \"{}\"",
                standing_label(engagement.standing()),
                engagement.participants().len(),
                trim(engagement.title(), 58),
            );
        }
    }
}

fn standing_label(standing: Standing) -> &'static str {
    match standing {
        Standing::Remembered => "REM",
        Standing::Continuable => "CON",
        Standing::Restorable => "RES",
    }
}

fn trim(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    let mut out: String = value.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}
