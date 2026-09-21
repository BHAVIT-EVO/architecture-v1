//! Selectivity probe — for every locus the model would let *found* a body of
//! work, the shape of its relationships in the witnessed graph, so the line
//! between a work locus and a resource that is merely beside everything can be
//! read off the data rather than asserted.
//!
//! The failure this investigates: a resource present beside all of a person's
//! activity (a login screen, a music player, an inbox, a system file that churns
//! all day) passes `seeds()` — a person was there, and returned — and then, as a
//! locus, `Thread::around` admits everything co-present with it. It both founds a
//! body of work and drags the rest in. The question is whether such resources are
//! distinguishable, *before* clustering, from genuine loci by the shape of their
//! affinities alone.
//!
//! For each seed it prints: degree (above-floor neighbours), strongest and
//! second-strongest affinity, total relatedness, and concentration (strongest /
//! total). A selective locus has few strong ties and high concentration; a
//! ubiquitous one has many moderate ties and low concentration. Then it simulates
//! connected components (plain above-floor edges) over the seeds that survive a
//! few candidate selectivity gates, so the effect of each gate on the real blob
//! is visible. Read-only.
//!
//! ```text
//! cargo run -p evo-daemon --example probe_selectivity
//! ```

use evo_engagement::{AffinityGraph, EngagementParams, Reconstruction};
use evo_storage::canonical_storage_root;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

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

    // Reconstruct under the OLD connectivity rule so the seed universe and graph
    // are the pre-fix ones — the blob we are trying to separate.
    let params = EngagementParams {
        reciprocal_rank: usize::MAX,
        ..EngagementParams::default()
    };
    let reconstruction = Reconstruction::from_observations(&observations, params);
    let set = reconstruction.set();
    let graph = reconstruction.graph();

    let mut seeds: BTreeSet<String> = BTreeSet::new();
    for engagement in set.engagements() {
        for participant in engagement.participants() {
            if participant.is_seed() {
                seeds.insert(participant.subject().to_string());
            }
        }
    }

    println!("================================================================");
    println!("EVO — SELECTIVITY PROBE  (OLD reconstruction, {} seeds)", seeds.len());
    println!("root: {}", root.display());
    println!("floor {:.2}   (degree = above-floor neighbours in full graph)", params.affinity_floor);
    println!("================================================================");

    // Per-seed relationship shape.
    struct Shape {
        subject: String,
        degree: usize,
        strongest: f64,
        second: f64,
        total: f64,
        concentration: f64,
    }
    let subjects: Vec<&String> = graph.subjects().collect();
    let mut shapes: Vec<Shape> = Vec::new();
    for seed in &seeds {
        let mut affs: Vec<f64> = subjects
            .iter()
            .filter(|other| **other != seed)
            .map(|other| graph.affinity(seed, other, &params))
            .filter(|a| *a >= params.affinity_floor)
            .collect();
        affs.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let degree = affs.len();
        let strongest = affs.first().copied().unwrap_or(0.0);
        let second = affs.get(1).copied().unwrap_or(0.0);
        let total: f64 = affs.iter().sum();
        let concentration = if total > 0.0 { strongest / total } else { 0.0 };
        shapes.push(Shape {
            subject: seed.clone(),
            degree,
            strongest,
            second,
            total,
            concentration,
        });
    }

    // Sorted by degree, high first: ubiquity should sort to the top.
    shapes.sort_by(|a, b| {
        b.degree
            .cmp(&a.degree)
            .then_with(|| a.subject.cmp(&b.subject))
    });
    println!();
    println!("SEED SHAPE (sorted by degree, high = beside more things)");
    println!("  deg  strong  2nd   total  conc   subject");
    for shape in &shapes {
        println!(
            "  {:>3}  {:>5.2}  {:>4.2}  {:>5.2}  {:>4.2}   {}",
            shape.degree,
            shape.strongest,
            shape.second,
            shape.total,
            shape.concentration,
            trim(&shape.subject, 54),
        );
    }

    // Candidate gates. Each keeps a seed as an eligible *founding locus* only if
    // its relationships are selective by some witnessed measure. We then show the
    // connected components (plain above-floor edges) over the survivors — the
    // clustering the model would run if only these founded work.
    let degree_cutoffs = [6usize, 8, 10, 12];
    for cutoff in degree_cutoffs {
        let survivors: BTreeSet<String> = shapes
            .iter()
            .filter(|s| s.degree <= cutoff)
            .map(|s| s.subject.clone())
            .collect();
        report_gate(
            &format!("degree <= {cutoff}"),
            &survivors,
            &seeds,
            graph,
            &params,
        );
    }
    for conc_bp in [30u32, 40, 50] {
        let conc = conc_bp as f64 / 100.0;
        let survivors: BTreeSet<String> = shapes
            .iter()
            .filter(|s| s.concentration >= conc)
            .map(|s| s.subject.clone())
            .collect();
        report_gate(
            &format!("concentration >= {conc:.2}"),
            &survivors,
            &seeds,
            graph,
            &params,
        );
    }
}

/// Print the connected components (plain above-floor edges) over the survivors
/// of a gate, plus which seeds the gate dropped.
fn report_gate(
    label: &str,
    survivors: &BTreeSet<String>,
    all_seeds: &BTreeSet<String>,
    graph: &AffinityGraph,
    params: &EngagementParams,
) {
    let components = connected_components(survivors, graph, params);
    let multi: Vec<&BTreeSet<String>> = components.iter().filter(|c| c.len() > 1).collect();
    let dropped: Vec<&String> = all_seeds.difference(survivors).collect();
    println!();
    println!("----------------------------------------------------------------");
    println!(
        "GATE {label}: {} of {} seeds survive -> {} components ({} multi, {} singletons)",
        survivors.len(),
        all_seeds.len(),
        components.len(),
        multi.len(),
        components.len() - multi.len(),
    );
    for component in components.iter().filter(|c| c.len() > 1) {
        println!("  component ({} seeds):", component.len());
        for subject in component.iter() {
            println!("      {}", trim(subject, 56));
        }
    }
    let dropped_show: Vec<String> = dropped.iter().take(24).map(|s| trim(s, 40)).collect();
    println!("  dropped ({}): {}", dropped.len(), dropped_show.join(" · "));
}

fn connected_components(
    subjects: &BTreeSet<String>,
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> Vec<BTreeSet<String>> {
    let mut adjacency: BTreeMap<&String, BTreeSet<&String>> = BTreeMap::new();
    for subject in subjects {
        adjacency.entry(subject).or_default();
    }
    for left in subjects {
        for right in subjects {
            if left >= right {
                continue;
            }
            if graph.affinity(left, right, params) >= params.affinity_floor {
                adjacency.entry(left).or_default().insert(right);
                adjacency.entry(right).or_default().insert(left);
            }
        }
    }
    let mut components = Vec::new();
    let mut visited: BTreeSet<&String> = BTreeSet::new();
    for subject in subjects {
        if visited.contains(subject) {
            continue;
        }
        let mut component: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<&String> = VecDeque::new();
        queue.push_back(subject);
        visited.insert(subject);
        while let Some(current) = queue.pop_front() {
            component.insert(current.clone());
            if let Some(neighbours) = adjacency.get(current) {
                for neighbour in neighbours {
                    if visited.insert(neighbour) {
                        queue.push_back(neighbour);
                    }
                }
            }
        }
        components.push(component);
    }
    components
}

fn trim(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    let mut out: String = value.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}
