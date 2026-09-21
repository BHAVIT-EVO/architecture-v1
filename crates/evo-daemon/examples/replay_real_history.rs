//! Replays this machine's real canonical Observation history through the
//! Engagement reconstruction and reports what Home would show.
//!
//! This exists because unit tests cannot answer the question that matters:
//! given a real person's real day, does Evo produce something they would
//! actually want to return to? Synthetic fixtures are written by whoever wrote
//! the algorithm and inherit its assumptions. A real log does not.
//!
//! Read-only. It opens the canonical storage root, decodes the Observation log,
//! and prints. Nothing is written, so it is safe to run against live state.
//!
//! ```text
//! cargo run -p evo-daemon --example replay_real_history
//! EVO_STORAGE_ROOT=/some/other/root cargo run -p evo-daemon --example replay_real_history
//! ```

use evo_daemon::persistence::load_persisted_observations;
use evo_engagement::{EngagementParams, Reconstruction, ResourceRole};
use evo_observation::observation::Observation;
use evo_storage::canonical_storage_root;

use std::collections::BTreeMap;
use std::time::{Duration, Instant, SystemTime};

fn main() {
    let root = canonical_storage_root();
    println!("storage root: {}", root.display());

    let observations = match load_persisted_observations(&root) {
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

    report_history(&observations);

    let params = EngagementParams::default();
    // Timed because the cost decides an architectural question: whether the
    // Workspace projection can be recomputed from the whole corpus on every
    // Observation, or has to be made incremental. Wall clock is read here, in
    // a reporting harness — never inside the model, which must stay
    // deterministic.
    let started = Instant::now();
    let reconstruction = Reconstruction::from_observations(&observations, params);
    let elapsed = started.elapsed();

    println!();
    println!("=== reconstruction ===");
    println!("recomputed in:         {elapsed:.2?}");
    println!("sittings:              {}", reconstruction.episodes().len());
    println!(
        "resources witnessed:   {}",
        reconstruction.ledger().subjects().count()
    );
    println!(
        "attention measured:    {}",
        human(reconstruction.ledger().total_attention())
    );
    let work = reconstruction.set().work();
    let remembered = reconstruction.set().remembered();
    println!("bodies of work:        {}", work.len());
    println!(
        "remembered, not work:  {}  (witnessed, kept, retrievable by name — never on Home)",
        remembered.len()
    );
    println!(
        "unattended resources:  {}  (no person ever witnessed at them — reported, not dropped)",
        reconstruction.set().unattended().len()
    );

    println!();
    println!("=== what Home would show ===");
    if work.is_empty() {
        println!("(nothing — Evo has no evidence of work it can stand behind)");
    }
    for engagement in work {
        println!();
        println!("Continue working on");
        println!("  {}", engagement.title());
        // Why it is called that. A name Home shows has to be answerable for, and
        // the two cases read very differently: a name chosen *because* it carries
        // the vocabulary the work's members share, versus a name that is simply
        // the resource this work most leads in because the members share nothing
        // measurable. Printing which is which is how the difference stays visible
        // instead of both looking like insight.
        println!(
            "  (named by {})",
            if engagement.titled_by_shared_vocabulary() {
                "vocabulary shared with the rest of the work"
            } else {
                "belonging — no member shares measurable vocabulary, so the name \
                 comes from a resource never witnessed outside this work"
            }
        );
        if let Some(last) = engagement.last_active() {
            println!("  Last active: {}", ago(last));
        }
        println!("  You were working in:");
        for participant in engagement.participants() {
            if participant.role() == ResourceRole::Primary {
                println!("    • {}", participant.resource().display_name());
            }
        }
        let supporting: Vec<_> = engagement
            .participants()
            .iter()
            .filter(|participant| participant.role() != ResourceRole::Primary)
            .collect();
        if !supporting.is_empty() {
            println!("  Also part of this ({} in the sidebar):", supporting.len());
            for participant in supporting.iter().take(8) {
                println!(
                    "    - [{:?}] {} — {}",
                    participant.role(),
                    participant.resource().display_name(),
                    human(participant.attention())
                );
            }
            if supporting.len() > 8 {
                println!("    - … and {} more", supporting.len() - 8);
            }
        }
        // The measurements the title and the roles were decided from, in the order
        // the model ranked them. This is the audit trail for a name: if the wrong
        // resource is heading a body of work, the reason is here.
        println!("  Ranking (most attended first; leadership needs exclusive + returned):");
        let members = engagement.subjects();
        for participant in engagement.participants() {
            println!(
                "    [{:?}] exclusive={} returned={} here={}/{} locality={:.2} vocab={:.3} attention={} corroborated={} — {}",
                participant.role(),
                participant.is_exclusive(),
                participant.returned(&params),
                participant.sittings_here(),
                participant.witnessed_sittings(),
                participant.locality(),
                reconstruction
                    .graph()
                    .shared_vocabulary(participant.subject(), &members),
                human(participant.attention()),
                participant.is_corroborated(),
                participant.subject(),
            );
        }
    }
}

/// What the raw history actually contains, per schema.
fn report_history(observations: &[Observation]) {
    let mut by_schema: BTreeMap<String, usize> = BTreeMap::new();
    let mut subjects: BTreeMap<String, usize> = BTreeMap::new();
    for observation in observations {
        *by_schema
            .entry(observation.schema().name().to_string())
            .or_insert(0) += 1;
    }
    let (acts, declarations) = evo_engagement::interpret(observations);
    for act in &acts {
        *subjects
            .entry(act.resource().subject().to_string())
            .or_insert(0) += 1;
    }

    println!("=== history ===");
    println!("observations:          {}", observations.len());
    for (schema, count) in &by_schema {
        println!("  {count:>6}  {schema}");
    }
    println!("distinct subjects:     {}", subjects.len());
    println!(
        "declarations:          {}",
        if declarations.is_empty() {
            "none".to_string()
        } else {
            format!(
                "{} designated, {} grouped, {} continuation",
                declarations.designations().count(),
                declarations.groupings().count(),
                declarations.continuations().count()
            )
        }
    );
}

fn human(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m");
    }
    format!("{}h {}m", minutes / 60, minutes % 60)
}

fn ago(moment: SystemTime) -> String {
    match SystemTime::now().duration_since(moment) {
        Ok(elapsed) => format!("{} ago", human(elapsed)),
        Err(_) => "just now".to_string(),
    }
}
