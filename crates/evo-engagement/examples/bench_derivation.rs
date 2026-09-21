//! Developer probe: how derivation cost scales with the number of distinct
//! resources.
//!
//! Architecture Amendment 1 replaced the persisted Workspace row with
//! derivation on read, which is only defensible if derivation is cheap enough
//! at realistic sizes. This measures that directly, at increasing numbers of
//! distinct resources, so the claim rests on a number rather than an
//! expectation.
//!
//! Usage:
//!   cargo run --release -p evo-engagement --example bench_derivation
//!   cargo run --release -p evo-engagement --example bench_derivation 100 200 400

use evo_engagement::{
    Act, ActCharacter, AffinityGraph, AttentionLedger, Declarations, EngagementParams,
    EngagementSet, Resource, segment_episodes,
};

use std::time::{Duration, Instant, SystemTime};

fn main() {
    let sizes: Vec<usize> = {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.is_empty() {
            vec![50, 100, 200, 400, 800]
        } else {
            args.iter().filter_map(|a| a.parse().ok()).collect()
        }
    };

    println!("== derivation cost vs. distinct resources ==");
    println!("(each resource is attended in 3 sittings, 4 passes per sitting)\n");
    println!("{:>10}  {:>8}  {:>12}  {:>10}", "resources", "acts", "derive", "per-pair");

    for size in sizes {
        let acts = build_acts(size);
        let acts_len = acts.len();
        let params = EngagementParams::default();
        let start = Instant::now();
        let episodes = segment_episodes(acts, &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        let declarations = Declarations::new();
        let graph = AffinityGraph::build(&episodes, &ledger, &declarations, &params);
        let set = EngagementSet::build(&episodes, &ledger, &graph, &declarations, &params);
        let elapsed = start.elapsed();
        let pairs = (size * size.saturating_sub(1)) / 2;
        let per_pair = if pairs == 0 {
            Duration::ZERO
        } else {
            elapsed / pairs as u32
        };
        println!(
            "{size:>10}  {acts_len:>8}  {:>12}  {:>10}  → {} body/bodies of work",
            format!("{elapsed:.2?}"),
            format!("{per_pair:.0?}"),
            set.len()
        );
    }
}

/// Builds acts for `size` distinct resources, each genuinely attended across
/// three sittings so significance is reachable and the full pipeline runs.
fn build_acts(size: usize) -> Vec<Act> {
    let mut acts = Vec::new();
    let mut moment = 0u64;
    for _sitting in 0..3 {
        for _pass in 0..4 {
            for index in 0..size {
                let subject = format!("Resource {index:05} — group {}", index % 8);
                acts.push(Act::new(
                    Resource::new(&subject, "OBS-WINDOW-FOCUS-GAINED"),
                    ActCharacter::Attentional,
                    SystemTime::UNIX_EPOCH + Duration::from_secs(moment),
                ));
                moment += 130;
            }
        }
        moment += 60 * 60;
    }
    acts
}
