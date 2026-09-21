//! Cluster-structure decomposition — *why* the reconstructed threads have the
//! membership they do, on this machine's real history.
//!
//! [`explain_standing`](./explain_standing.rs) decomposes each thread's
//! **standing** (significance and continuation). This one decomposes each
//! thread's **shape**: how many resources it fused, how many of those are seeds
//! (loci that may centre a body of work) versus admitted members, and — the
//! decisive question when a thread has seventy members — whether its seeds form
//! one connected component because they are genuinely one activity, or because a
//! resource beside everything bridged distinct activities into one blob.
//!
//! It reproduces, from public accessors only, the two structural steps the model
//! runs:
//!
//!   * **stage-1 clustering** — connected components of the *seed* graph over
//!     edges reaching `affinity_floor` (mirrors `cluster_within`); and
//!   * **membership admission** — every co-present resource with max-affinity to
//!     a seed reaching `affinity_floor` (mirrors `Thread::around`), summarised as
//!     a strength/specificity/attention distribution so over-admission is
//!     visible as a count rather than a wall of names.
//!
//! For every seed connected component it lists the inter-seed edges and the
//! single strongest kind of evidence behind each, so a bridge — an edge from one
//! activity's seed to another's, through a resource present beside both — is
//! named rather than inferred. Read-only: opens the canonical root, decodes,
//! derives, prints. Nothing written, no network.
//!
//! ```text
//! cargo run -p evo-daemon --example explain_clusters
//! ```

use evo_engagement::{AffinityGraph, EngagementParams, EvidenceKind, Reconstruction, Standing};
use evo_storage::canonical_storage_root;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Duration;

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

    let params = EngagementParams::default();
    let reconstruction = Reconstruction::from_observations(&observations, params);
    let set = reconstruction.set();
    let graph = reconstruction.graph();

    // ---- the seed universe: every locus, across every thread ---------------
    // A Participant is a seed when it centred the thread it is in (post
    // refinement). The union over all threads is the founding-loci set the
    // clustering ran on.
    let mut seeds: BTreeSet<String> = BTreeSet::new();
    for engagement in set.engagements() {
        for participant in engagement.participants() {
            if participant.is_seed() {
                seeds.insert(participant.subject().to_string());
            }
        }
    }

    println!("================================================================");
    println!("EVO — CLUSTER-STRUCTURE DECOMPOSITION");
    println!("root: {}", root.display());
    println!(
        "observations: {}  threads: {} (work {}, remembered {})  seeds: {}",
        observations.len(),
        set.engagements().len(),
        set.work().len(),
        set.remembered().len(),
        seeds.len(),
    );
    println!(
        "floors: affinity {:.2}  cohesion {:.2}  speaks-for-work {:.2}",
        params.affinity_floor, params.cohesion_floor, params.speaks_for_work,
    );
    println!("================================================================");

    // ---- stage 1 reproduced: connected components of the seed graph --------
    // Exactly the edges cluster_within builds its components from: a seed pair
    // whose combined affinity reaches the floor. A component larger than one
    // real activity is the clustering fusing distinct work; the edge list under
    // it shows what did the fusing.
    let components = connected_components(&seeds, graph, &params);
    println!();
    println!(
        "SEED CONNECTED COMPONENTS (stage-1 clustering input): {} components over {} seeds",
        components.len(),
        seeds.len(),
    );
    for (index, component) in components.iter().enumerate() {
        if component.len() == 1 {
            continue; // singletons are their own thread; nothing fused.
        }
        println!();
        println!(
            "  component #{index}: {} seeds ------------------------------------",
            component.len()
        );
        for seed in component {
            // A seed's strongest tie *to another seed in this component*: the
            // edge that keeps it in the blob. Naming its evidence kind is what
            // tells "one activity" from "bridged by a companion".
            let (partner, aff, kind) = strongest_internal_tie(seed, component, graph, &params);
            match partner {
                Some(partner) => println!(
                    "    {:<44} via {:<9} aff={:.2}  ->  {}",
                    trim(seed, 44),
                    kind_word(kind),
                    aff,
                    trim(partner, 40),
                ),
                None => println!(
                    "    {:<44} ISOLATED (no above-floor edge to any seed here)",
                    trim(seed, 44)
                ),
            }
        }
    }

    // ---- per-thread shape: seeds vs admitted members -----------------------
    println!();
    println!("PER-THREAD SHAPE (members = seeds + admitted; admission = Thread::around)");

    // Accumulate the refined seed universe as we go: the seeds that survive
    // being measured against the person's other work (specificity reaches
    // speaks-for-work), which is exactly what `refined_seeds` keeps. If a
    // thread's every seed is ubiquitous, its originals are kept — the same
    // fallback `refined_seeds` uses, so this mirrors the model rather than a
    // stricter rule of my own.
    let mut refined_universe: BTreeSet<String> = BTreeSet::new();

    for engagement in set.engagements() {
        let members = engagement.participants();
        let seed_members: Vec<&str> = members
            .iter()
            .filter(|p| p.is_seed())
            .map(|p| p.subject())
            .collect();

        // Admission bands over the non-seed members: how strongly each admitted
        // resource attaches, and whether the person was ever witnessed in it.
        let mut band_floor_cohesion = 0usize; // [affinity_floor, cohesion_floor)
        let mut band_cohesion_speaks = 0usize; // [cohesion_floor, speaks_for_work)
        let mut band_speaks_up = 0usize; // [speaks_for_work, 1]
        let mut attention_zero = 0usize; // admitted, never attended
        let mut speaks_here = 0usize; // specificity says it is of this work
        for participant in members {
            if participant.is_seed() {
                continue;
            }
            let strength = participant.strength();
            if strength < params.cohesion_floor {
                band_floor_cohesion += 1;
            } else if strength < params.speaks_for_work {
                band_cohesion_speaks += 1;
            } else {
                band_speaks_up += 1;
            }
            if participant.attention() == Duration::ZERO {
                attention_zero += 1;
            }
            if participant.specificity() >= params.speaks_for_work {
                speaks_here += 1;
            }
        }

        println!();
        println!(
            "[{}] {}",
            standing_label(engagement.standing()),
            trim(engagement.title(), 64),
        );
        println!(
            "      members {:<3} = seeds {:<2} + admitted {:<3}   admitted-strength: [flr,coh)={} [coh,spk)={} [spk,1]={}   admitted-att0={}   speaks-here(members)={}",
            members.len(),
            seed_members.len(),
            members.len() - seed_members.len(),
            band_floor_cohesion,
            band_cohesion_speaks,
            band_speaks_up,
            attention_zero,
            speaks_here,
        );
        // The seeds themselves, named with their specificity — few enough to
        // always print in full. A ✓ marks the ones that survive refinement
        // (specificity reaches speaks-for-work); those are what a re-cluster
        // would run on.
        let surviving: Vec<&str> = seed_members
            .iter()
            .copied()
            .filter(|subject| {
                members
                    .iter()
                    .find(|p| p.subject() == *subject)
                    .is_some_and(|p| p.specificity() >= params.speaks_for_work)
            })
            .collect();
        // Mirror refined_seeds' fallback: if none survive, keep the originals.
        if surviving.is_empty() {
            refined_universe.extend(seed_members.iter().map(|s| s.to_string()));
        } else {
            refined_universe.extend(surviving.iter().map(|s| s.to_string()));
        }
        if !seed_members.is_empty() {
            for seed in &seed_members {
                let specificity = members
                    .iter()
                    .find(|p| p.subject() == *seed)
                    .map(|p| p.specificity())
                    .unwrap_or(1.0);
                let survives = specificity >= params.speaks_for_work;
                println!(
                    "        seed {} spec={:.2}  {}",
                    if survives { "✓" } else { "✗" },
                    specificity,
                    trim(seed, 54),
                );
            }
        }
        // The distinctive, attended, this-work members that a *correct* thread
        // would be built from: seed or speaks-for-work, and actually attended.
        let distinctive_attended: Vec<&str> = members
            .iter()
            .filter(|p| {
                (p.is_seed() || p.specificity() >= params.speaks_for_work)
                    && p.attention() > Duration::ZERO
            })
            .map(|p| p.subject())
            .collect();
        println!(
            "        distinctive+attended core: {} member(s)",
            distinctive_attended.len()
        );
        for subject in &distinctive_attended {
            println!("           * {}", trim(subject, 55));
        }
    }

    // ---- the fix, simulated: re-cluster over the refined seed universe -----
    // The model forms threads, measures specificity, and refines the seeds —
    // but re-forms `Thread::around` the survivors without re-running the
    // clustering, so a blob already fused by a bridge cannot come apart. This
    // is what a re-cluster over the survivors would produce: the same stage-1
    // connected components, now over seeds that each earned their place against
    // the person's other work. If a genuine activity separates from the blob
    // here, a bridge — not shared activity — was holding it in.
    let refined_components = connected_components(&refined_universe, graph, &params);
    println!();
    println!("================================================================");
    println!(
        "RE-CLUSTER SIMULATION over the refined seed universe: {} seeds -> {} components",
        refined_universe.len(),
        refined_components.len(),
    );
    println!("(components of size 1 are omitted)");
    for (index, component) in refined_components.iter().enumerate() {
        if component.len() == 1 {
            continue;
        }
        println!();
        println!("  refined component #{index}: {} seeds", component.len());
        for seed in component {
            println!("    {}", trim(seed, 60));
        }
    }
    let singletons = refined_components.iter().filter(|c| c.len() == 1).count();
    println!();
    println!("  + {singletons} singleton component(s) (each its own body of work)");

    // ---- the other candidate fix: corroboration-gated clustering ----------
    // affinity.rs states the design intent that corroboration — evidence about
    // *what things are* (shared distinctive vocabulary, a shared container, or a
    // statement) — "decides which relationships survive clustering". But
    // cluster_within merges on the raw score, which interleave alone (weight
    // 0.45) can carry over the floor. So a chat window, a desktop, or a music
    // player alt-tabbed through a work afternoon earns a presence-only edge to
    // the work and fuses with it. This reproduces clustering as the docstring
    // says it should behave: an edge counts only if it is corroborated.
    let corroborated_components =
        connected_components_where(&seeds, &params, |a, b| corroborated_edge(graph, a, b, &params));
    println!();
    println!("================================================================");
    println!(
        "CORROBORATION-GATED SIMULATION over all {} seeds -> {} components",
        seeds.len(),
        corroborated_components.len(),
    );
    println!("(an edge counts only if lexical/structural/declared, not presence-only)");
    for (index, component) in corroborated_components.iter().enumerate() {
        if component.len() == 1 {
            continue;
        }
        println!();
        println!("  corroborated component #{index}: {} seeds", component.len());
        for seed in component {
            println!("    {}", trim(seed, 60));
        }
    }
    let corr_singletons = corroborated_components
        .iter()
        .filter(|c| c.len() == 1)
        .count();
    println!();
    println!("  + {corr_singletons} singleton component(s)");

    // The presence-only bridges: the seed pairs holding the largest raw
    // component together that corroboration would cut. This is what the gate
    // removes — and what must be inspected to be sure it removes only ambient
    // company, never a work locus from its own activity.
    if let Some(blob) = components.iter().find(|c| c.len() > 2) {
        println!();
        println!(
            "PRESENCE-ONLY EDGES inside the largest raw component ({} seeds) — the bridges a gate would cut:",
            blob.len()
        );
        let mut cut = 0usize;
        for (i, left) in blob.iter().enumerate() {
            for right in blob.iter().skip(i + 1) {
                let aff = graph.affinity(left, right, &params);
                if aff < params.affinity_floor {
                    continue;
                }
                let corroborated = graph
                    .evidence(left, right)
                    .is_some_and(|e| e.is_corroborated());
                if !corroborated {
                    cut += 1;
                    println!(
                        "    {:<40} --P-- {:<40} aff={:.2}",
                        trim(left, 40),
                        trim(right, 40),
                        aff
                    );
                }
            }
        }
        println!("    ({cut} presence-only edges total)");
    }

    // ---- rule search: reciprocal-nearest clustering -----------------------
    // Ambient company (a desktop, a music player, a chat window) attaches
    // moderately to many activities but is the *strongest* tie of none of them;
    // a work locus's strongest ties are its own activity. So an edge that is
    // among BOTH endpoints' strongest few is evidence of one activity, while an
    // edge strong for only one side is one thing reaching past another. This
    // keeps a source file with the search for its own code (mutual) and drops a
    // desktop that everything happens to pass through (one-sided). Tested at a
    // few k so the choice is read off the data, not asserted.
    for k in [1usize, 2, 3] {
        let ranked = ranked_neighbours(&seeds, graph, &params);
        let comps = connected_components_where(&seeds, &params, |a, b| {
            reciprocal(&ranked, a, b, k)
        });
        let multi: Vec<&Vec<String>> = comps.iter().filter(|c| c.len() > 1).collect();
        println!();
        println!("================================================================");
        println!(
            "RECIPROCAL-NEAREST (k={k}) over all {} seeds -> {} components ({} multi-seed, {} singletons)",
            seeds.len(),
            comps.len(),
            multi.len(),
            comps.len() - multi.len(),
        );
        for (index, component) in comps.iter().enumerate() {
            if component.len() == 1 {
                continue;
            }
            println!("  component #{index}: {} seeds", component.len());
            for seed in component {
                println!("      {}", trim(seed, 58));
            }
        }
    }
}

/// Each seed's neighbours ranked by affinity, strongest first (canonical
/// tie-break), restricted to above-floor ties.
fn ranked_neighbours(
    seeds: &BTreeSet<String>,
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> BTreeMap<String, Vec<String>> {
    let mut ranked: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for seed in seeds {
        let mut neighbours: Vec<(String, f64)> = seeds
            .iter()
            .filter(|other| *other != seed)
            .map(|other| (other.clone(), graph.affinity(seed, other, params)))
            .filter(|(_, aff)| *aff >= params.affinity_floor)
            .collect();
        neighbours.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        ranked.insert(seed.clone(), neighbours.into_iter().map(|(s, _)| s).collect());
    }
    ranked
}

/// Whether each of `a`, `b` is within the other's top-`k` strongest ties.
fn reciprocal(ranked: &BTreeMap<String, Vec<String>>, a: &str, b: &str, k: usize) -> bool {
    let in_top = |x: &str, y: &str| {
        ranked
            .get(x)
            .is_some_and(|list| list.iter().take(k).any(|n| n == y))
    };
    in_top(a, b) && in_top(b, a)
}

/// Connected components of the seed graph over edges reaching `affinity_floor`.
/// Deterministic: BFS in canonical (sorted) order, exactly as `cluster_within`.
fn connected_components(
    seeds: &BTreeSet<String>,
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> Vec<Vec<String>> {
    connected_components_where(seeds, params, |left, right| {
        graph.affinity(left, right, params) >= params.affinity_floor
    })
}

/// Connected components under an arbitrary edge predicate, so the same
/// deterministic traversal can reproduce clustering under different rules for
/// what counts as a relationship.
fn connected_components_where(
    seeds: &BTreeSet<String>,
    _params: &EngagementParams,
    edge: impl Fn(&str, &str) -> bool,
) -> Vec<Vec<String>> {
    // Adjacency restricted to seed↔seed edges the predicate admits.
    let mut adjacency: BTreeMap<&String, BTreeSet<&String>> = BTreeMap::new();
    for seed in seeds {
        adjacency.entry(seed).or_default();
    }
    for left in seeds {
        for right in seeds {
            if left >= right {
                continue;
            }
            if edge(left, right) {
                adjacency.get_mut(left).unwrap().insert(right);
                adjacency.get_mut(right).unwrap().insert(left);
            }
        }
    }

    let mut components: Vec<Vec<String>> = Vec::new();
    let mut visited: BTreeSet<&String> = BTreeSet::new();
    for seed in seeds {
        if visited.contains(seed) {
            continue;
        }
        let mut component: Vec<String> = Vec::new();
        let mut queue: VecDeque<&String> = VecDeque::new();
        queue.push_back(seed);
        visited.insert(seed);
        while let Some(current) = queue.pop_front() {
            component.push(current.clone());
            for neighbour in &adjacency[current] {
                if visited.insert(neighbour) {
                    queue.push_back(neighbour);
                }
            }
        }
        component.sort();
        components.push(component);
    }
    // Largest first, so the blobs lead.
    components.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    components
}

/// Whether a seed pair is related *and corroborated* — the design intent
/// affinity.rs states for what should survive clustering.
fn corroborated_edge(
    graph: &AffinityGraph,
    left: &str,
    right: &str,
    params: &EngagementParams,
) -> bool {
    graph.affinity(left, right, params) >= params.affinity_floor
        && graph
            .evidence(left, right)
            .is_some_and(|evidence| evidence.is_corroborated())
}

/// A seed's strongest above-floor tie to another seed in the same component,
/// and the dominant kind of evidence behind that tie.
fn strongest_internal_tie<'a>(
    seed: &str,
    component: &'a [String],
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> (Option<&'a String>, f64, Option<EvidenceKind>) {
    let mut best: Option<&String> = None;
    let mut best_aff = 0.0_f64;
    let mut best_kind: Option<EvidenceKind> = None;
    for other in component {
        if other == seed {
            continue;
        }
        let aff = graph.affinity(seed, other, params);
        if aff >= params.affinity_floor && aff > best_aff {
            best_aff = aff;
            best = Some(other);
            best_kind = graph.evidence(seed, other).map(|e| e.strongest(params));
        }
    }
    (best, best_aff, best_kind)
}

fn kind_word(kind: Option<EvidenceKind>) -> &'static str {
    match kind {
        Some(EvidenceKind::Declared) => "declared",
        Some(EvidenceKind::Interleaved) => "interleave",
        Some(EvidenceKind::Recurring) => "recur",
        Some(EvidenceKind::Lexical) => "lexical",
        Some(EvidenceKind::Structural) => "structural",
        None => "-",
    }
}

fn standing_label(standing: Standing) -> &'static str {
    match standing {
        Standing::Remembered => "REMEMBERED ",
        Standing::Continuable => "CONTINUABLE",
        Standing::Restorable => "RESTORABLE ",
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
