//! Cohesion probe — under the *new* connectivity rule, the internal shape of
//! every thread the model forms: whether its members cohere with each other (a
//! body of work is things used together) or merely with the one resource that
//! founded it (a hub with spokes, which the crate's own principle says is
//! nothing).
//!
//! This tests the hypothesis the selectivity probe pointed to: a login screen or
//! a music player is not distinguishable from real work by its own degree, but
//! the *thread it founds* is — its members are co-present with the hub and not
//! with each other, so the thread is a star, while genuine work is a clique.
//!
//! For each engagement it prints: participant count, seed count, and cohesion
//! density = the fraction of member-pairs that are themselves above-floor
//! related. A clique approaches 1.0; a star approaches 0.0. Read-only.
//!
//! ```text
//! cargo run -p evo-daemon --example probe_cohesion
//! ```

use evo_engagement::{EngagementParams, Reconstruction, Standing};
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

    let params = EngagementParams::default(); // reciprocal_rank = 2
    let reconstruction = Reconstruction::from_observations(&observations, params);
    let set = reconstruction.set();
    let graph = reconstruction.graph();

    println!("================================================================");
    println!("EVO — COHESION PROBE  (new rule, {} threads)", set.engagements().len());
    println!("root: {}", root.display());
    println!("floor {:.2}   density = fraction of member-pairs above floor", params.affinity_floor);
    println!("================================================================");
    println!();
    println!("  memb seed  dens  mean  stand  title");

    struct Row {
        members: usize,
        seeds: usize,
        density: f64,
        mean_affinity: f64,
        standing: Standing,
        title: String,
    }
    let mut rows: Vec<Row> = Vec::new();

    for engagement in set.engagements() {
        let subjects: Vec<&str> = engagement
            .participants()
            .iter()
            .map(|p| p.subject())
            .collect();
        let seeds: Vec<&str> = engagement
            .participants()
            .iter()
            .filter(|p| p.is_seed())
            .map(|p| p.subject())
            .collect();
        let non_seeds: Vec<&str> = engagement
            .participants()
            .iter()
            .filter(|p| !p.is_seed())
            .map(|p| p.subject())
            .collect();

        // Cohesion density over ALL member pairs, and mean pairwise affinity.
        let mut pairs = 0usize;
        let mut linked = 0usize;
        let mut affinity_sum = 0.0_f64;
        for (i, a) in subjects.iter().enumerate() {
            for b in subjects.iter().skip(i + 1) {
                pairs += 1;
                let affinity = graph.affinity(a, b, &params);
                affinity_sum += affinity;
                if affinity >= params.affinity_floor {
                    linked += 1;
                }
            }
        }
        let density = if pairs > 0 { linked as f64 / pairs as f64 } else { 0.0 };
        let mean_affinity = if pairs > 0 { affinity_sum / pairs as f64 } else { 0.0 };

        // Hub fraction: of the non-seed members, how many cohere ONLY with a seed
        // and with no other member — i.e., pure spokes of a star.
        let _ = &non_seeds;

        rows.push(Row {
            members: subjects.len(),
            seeds: seeds.len(),
            density,
            mean_affinity,
            standing: engagement.standing(),
            title: engagement.title().to_string(),
        });
    }

    // Largest first — the mega-threads are the ones to scrutinise.
    rows.sort_by(|a, b| b.members.cmp(&a.members));
    for row in &rows {
        println!(
            "  {:>4} {:>4}  {:>4.2}  {:>4.2}   {}  {}",
            row.members,
            row.seeds,
            row.density,
            row.mean_affinity,
            standing_label(row.standing),
            trim(&row.title, 46),
        );
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
