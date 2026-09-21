//! Standing decomposition — for every reconstructed thread on this machine's
//! real history, print *exactly* which witnessed condition put it where it is.
//!
//! [`audit_work_model`](./audit_work_model.rs) measures the corpus against the
//! product contract. This one is narrower and sharper: it re-derives, in the
//! open, the two decisions that decide what Home shows —
//!
//!   * significance (Remembered vs a body of work): core size, return, and the
//!     deepest single sitting the core's own attention reached, against the
//!     threshold; and
//!   * continuation (Continuable vs Restorable): for each member, the gate that
//!     admits a Primary (engaged — worked in for more than a glance in some one
//!     sitting — and speaks-for-work, under the primary cap), then whether a sole
//!     strictly-latest Primary exists to become the Resume Point.
//!
//! It mirrors `is_significant` / `assign_roles` / `name_the_continuation` in
//! `evo-engagement` using only public accessors, so the printed "why" is the
//! same arithmetic the model actually ran. Read-only: opens the canonical root,
//! decodes, derives, prints. Nothing written, no network.
//!
//! ```text
//! cargo run -p evo-daemon --example explain_standing
//! ```

use evo_engagement::{
    AttentionLedger, EngagementParams, Participant, Reconstruction, ResourceRole, Standing,
};
use evo_storage::canonical_storage_root;

use std::collections::BTreeMap;
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
    let ledger = reconstruction.ledger();

    println!("================================================================");
    println!("EVO — STANDING DECOMPOSITION");
    println!("root: {}", root.display());
    println!(
        "observations: {}  sittings: {}  threads: {} (work {}, remembered {})",
        observations.len(),
        reconstruction.episodes().len(),
        set.engagements().len(),
        set.work().len(),
        set.remembered().len(),
    );
    println!(
        "thresholds: sustained_sitting_attention {}s  min_revisits {}  speaks_for_work {:.2}  max_primary {}",
        params.sustained_sitting_attention.as_secs(),
        params.min_revisits,
        params.speaks_for_work,
        params.max_primary,
    );
    println!("================================================================");

    for engagement in set.engagements() {
        let standing = engagement.standing();
        println!();
        println!(
            "[{}] {}",
            standing_label(standing),
            trim(engagement.title(), 72)
        );
        println!(
            "      members {}   attention {}s   sittings {}",
            engagement.participants().len(),
            engagement.attention().as_secs(),
            engagement.recurrence(),
        );

        // ---- significance decomposition (mirrors is_significant) ----------
        let core: Vec<&Participant> = engagement
            .participants()
            .iter()
            .filter(|p| p.is_seed() || p.speaks_for_work(&params))
            .collect();
        let core_returned = core.iter().any(|p| {
            ledger
                .get(p.subject())
                .is_some_and(|r| r.human_acts >= params.min_revisits)
        });
        let core_depth = deepest_sitting(&core, ledger);
        let relationship_ok = core.len() >= 2;
        let depth_ok = core_depth >= params.sustained_sitting_attention;
        println!(
            "      SIGNIFICANCE: core(seeds∪speaks-for)={} [{}]   returned={} [{}]   deepest-core-sitting={}s [{}]",
            core.len(),
            pass(relationship_ok),
            core_returned,
            pass(core_returned),
            core_depth.as_secs(),
            pass(depth_ok),
        );
        if standing == Standing::Remembered {
            let reason = if !relationship_ok {
                "fewer than two core resources (nothing distinctively of this work)"
            } else if !core_returned {
                "no core resource was returned to (human_acts < min_revisits)"
            } else {
                "core never reached a sitting's worth of attention (habit-shaped, not a work session)"
            };
            println!("      -> REMEMBERED because: {reason}");
        }

        // ---- continuation decomposition (mirrors assign_roles gate) -------
        // The four conjuncts of the Primary gate, per attended member.
        let mut any_primary = false;
        let mut primary_lastseen: Vec<(String, Option<std::time::SystemTime>)> = Vec::new();
        for p in engagement.participants() {
            let exclusive = p.is_exclusive();
            let returned = p.returned(&params);
            let engaged = p.engaged(&params);
            let speaks = p.speaks_for_work(&params);
            let attended = p.attention() > Duration::ZERO;
            let role = p.role();
            if role == ResourceRole::Primary || role == ResourceRole::Continuation {
                any_primary = true;
                primary_lastseen.push((p.subject().to_string(), p.last_seen()));
            }
            // Only surface the members that carry attention — the ones eligible
            // to lead. A one-line gate readout each.
            if attended || role != ResourceRole::Reference {
                println!(
                    "        {:<12} eng={} excl={} ret={} speaks={:.2}{} att={}s  {}",
                    format!("{:?}", role),
                    yn(engaged),
                    yn(exclusive),
                    yn(returned),
                    p.specificity(),
                    if speaks { "✓" } else { "✗" },
                    p.attention().as_secs(),
                    trim(p.subject(), 52),
                );
            }
        }
        // Why a Resume Point exists or not.
        if standing != Standing::Remembered {
            if !any_primary {
                println!(
                    "      -> NO RESUME POINT: no member cleared the Primary gate (engaged ∧ speaks-for-work)"
                );
            } else {
                let latest = primary_lastseen.iter().filter_map(|(_, t)| *t).max();
                let at_latest = primary_lastseen
                    .iter()
                    .filter(|(_, t)| *t == latest)
                    .count();
                if at_latest == 1 {
                    println!("      -> RESUME POINT: one Primary is strictly most-recent");
                } else {
                    println!(
                        "      -> NO RESUME POINT: {at_latest} Primaries tie for most-recent (silence, not a coin-flip)"
                    );
                }
            }
        }
    }
}

/// Mirror of the private `deepest_sitting` in evo-engagement.
fn deepest_sitting(participants: &[&Participant], ledger: &AttentionLedger) -> Duration {
    let mut sittings: BTreeMap<usize, Duration> = BTreeMap::new();
    for participant in participants {
        let Some(record) = ledger.get(participant.subject()) else {
            continue;
        };
        for (episode, attention) in &record.per_episode {
            *sittings.entry(*episode).or_insert(Duration::ZERO) += *attention;
        }
    }
    sittings.into_values().max().unwrap_or(Duration::ZERO)
}

fn standing_label(standing: Standing) -> &'static str {
    match standing {
        Standing::Remembered => "REMEMBERED ",
        Standing::Continuable => "CONTINUABLE",
        Standing::Restorable => "RESTORABLE ",
    }
}

fn pass(ok: bool) -> &'static str {
    if ok { "PASS" } else { "FAIL" }
}

fn yn(value: bool) -> char {
    if value { 'Y' } else { 'n' }
}

fn trim(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    let mut out: String = value.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}
