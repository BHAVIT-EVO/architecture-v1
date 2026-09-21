//! Reports the occurrence-local contexts used by work inference.
//!
//! Read-only: derives from the canonical Observation log and writes nothing.

use evo_daemon::persistence::load_persisted_observations;
use evo_engagement::{ActivityContext, AffinityGraph, EngagementParams, Reconstruction};
use evo_storage::canonical_storage_root;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn main() {
    let root = canonical_storage_root();
    let observations = match load_persisted_observations(&root) {
        Ok(observations) => observations,
        Err(error) => {
            eprintln!("cannot read the canonical log: {error}");
            std::process::exit(1);
        }
    };
    let reconstruction =
        Reconstruction::from_observations(&observations, EngagementParams::default());
    let contexts = reconstruction.contexts();

    let mut size_distribution: BTreeMap<usize, usize> = BTreeMap::new();
    let mut frequency: BTreeMap<&str, usize> = BTreeMap::new();
    let mut signatures: BTreeMap<Vec<&str>, usize> = BTreeMap::new();
    for context in contexts {
        *size_distribution
            .entry(context.human_subjects().len())
            .or_insert(0) += 1;
        for subject in context.human_subjects() {
            *frequency.entry(subject).or_insert(0) += 1;
        }
        let signature: Vec<&str> = context
            .human_subjects()
            .iter()
            .map(String::as_str)
            .collect();
        *signatures.entry(signature).or_insert(0) += 1;
    }

    println!("observations: {}", observations.len());
    println!("sittings:     {}", reconstruction.episodes().len());
    println!("contexts:     {}", contexts.len());
    println!("human subjects per context:");
    for (size, count) in size_distribution {
        println!("  {count:>4} context(s) with {size:>2} subject(s)");
    }

    let mut ranked_frequency: Vec<(&str, usize)> = frequency.into_iter().collect();
    ranked_frequency.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(right.0)));
    println!("most widespread human subjects:");
    for (subject, count) in ranked_frequency.iter().take(25) {
        println!(
            "  {count:>4}/{:<4} ({:>5.1}%)  {}",
            contexts.len(),
            100.0 * *count as f64 / contexts.len().max(1) as f64,
            trim(subject, 100)
        );
    }

    let mut repeated: Vec<(usize, Vec<&str>)> = signatures
        .into_iter()
        .map(|(signature, count)| (count, signature))
        .collect();
    repeated.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    println!("most repeated exact context signatures:");
    for (count, signature) in repeated.iter().take(20) {
        let names: Vec<String> = signature.iter().map(|name| trim(name, 42)).collect();
        println!("  {count:>4}x  {}", names.join(" | "));
    }

    println!("largest contexts:");
    let mut indexed: Vec<_> = contexts.iter().enumerate().collect();
    indexed.sort_by(|left, right| {
        right
            .1
            .human_subjects()
            .len()
            .cmp(&left.1.human_subjects().len())
            .then(left.0.cmp(&right.0))
    });
    for (index, context) in indexed.iter().take(20) {
        let names: Vec<String> = context
            .human_subjects()
            .iter()
            .map(|name| trim(name, 42))
            .collect();
        println!(
            "  #{index:<4} sitting {:<3} {:>2} subject(s): {}",
            context.episode(),
            context.human_subjects().len(),
            names.join(" | ")
        );
    }

    report_merge_components(contexts, reconstruction.graph(), reconstruction.params());
}

fn report_merge_components(
    contexts: &[ActivityContext],
    graph: &AffinityGraph,
    params: &EngagementParams,
) {
    let mut adjacency: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); contexts.len()];
    let mut bridges: BTreeMap<(usize, usize), Vec<String>> = BTreeMap::new();
    for left in 0..contexts.len() {
        for right in left + 1..contexts.len() {
            let reasons = merge_reasons(&contexts[left], &contexts[right], graph, params);
            if reasons.is_empty() {
                continue;
            }
            adjacency[left].insert(right);
            adjacency[right].insert(left);
            bridges.insert((left, right), reasons);
        }
    }

    let mut components = Vec::new();
    let mut visited = BTreeSet::new();
    for start in 0..contexts.len() {
        if !visited.insert(start) {
            continue;
        }
        let mut component = BTreeSet::new();
        let mut queue = VecDeque::from([start]);
        while let Some(index) = queue.pop_front() {
            component.insert(index);
            for next in &adjacency[index] {
                if visited.insert(*next) {
                    queue.push_back(*next);
                }
            }
        }
        components.push(component);
    }
    components.sort_by_key(|component| std::cmp::Reverse(component.len()));

    println!("current transitive merge components:");
    for component in components.iter().take(15) {
        let mut subjects: BTreeMap<&str, usize> = BTreeMap::new();
        let mut episodes = BTreeSet::new();
        for index in component {
            episodes.insert(contexts[*index].episode());
            for subject in contexts[*index].human_subjects() {
                *subjects.entry(subject).or_insert(0) += 1;
            }
        }
        let mut ranked: Vec<(&str, usize)> = subjects.into_iter().collect();
        ranked.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(right.0)));
        let names: Vec<String> = ranked
            .iter()
            .take(8)
            .map(|(name, count)| format!("{count}x {}", trim(name, 34)))
            .collect();
        println!(
            "  {:>3} contexts / {:>2} sittings / {:>3} subjects: {}",
            component.len(),
            episodes.len(),
            ranked.len(),
            names.join(" | ")
        );

        if component.len() >= 10 {
            let mut edge_reasons: BTreeMap<String, usize> = BTreeMap::new();
            for ((left, right), reasons) in &bridges {
                if component.contains(left) && component.contains(right) {
                    for reason in reasons {
                        *edge_reasons.entry(reason.clone()).or_insert(0) += 1;
                    }
                }
            }
            let mut ranked_reasons: Vec<(String, usize)> = edge_reasons.into_iter().collect();
            ranked_reasons.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
            for (reason, count) in ranked_reasons.iter().take(10) {
                println!("      {count:>4} edge(s): {reason}");
            }
        }
    }
}

fn merge_reasons(
    left: &ActivityContext,
    right: &ActivityContext,
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> Vec<String> {
    let shared: BTreeSet<&String> = left
        .human_subjects()
        .intersection(right.human_subjects())
        .collect();
    if shared.len() >= 2 {
        return vec![format!(
            "shared {}",
            shared
                .iter()
                .map(|name| trim(name, 24))
                .collect::<Vec<_>>()
                .join(" + ")
        )];
    }

    if shared.is_empty() {
        for first in left.human_subjects() {
            for second in right.human_subjects() {
                if pair_supports(first, second, graph, params) {
                    return vec![format!("pair {} <> {}", trim(first, 24), trim(second, 24))];
                }
            }
        }
    }
    Vec::new()
}

fn pair_supports(
    first: &str,
    second: &str,
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> bool {
    graph.evidence(first, second).is_some_and(|evidence| {
        evidence.co_episodes >= params.min_co_episodes
            && (evidence.is_corroborated()
                || evidence.interleave >= 0.5
                || evidence.co_episode >= 0.5)
            && evidence.score(params) >= params.affinity_floor
    })
}

fn trim(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let prefix: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        format!("{prefix}...")
    } else {
        prefix
    }
}
