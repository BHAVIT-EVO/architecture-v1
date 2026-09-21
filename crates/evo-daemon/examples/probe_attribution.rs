//! Probe: what evidence, if any, ties an *unattended change* to the part of a
//! body of work a person was actually active in?
//!
//! This exists to settle a question by measurement rather than by reasoning. Role
//! assignment must decide whether a resource that Evo only ever saw *change* —
//! zero measurable attention, no witnessed human act — took part in the work
//! ([`ResourceRole::Supporting`]) or was merely present ([`ResourceRole::Context`]).
//! Two cases look identical in the summary statistics (zero attention, locality
//! 1.00, witnessed in two sittings):
//!
//!   * a document the person kept saving while working — genuinely part of it;
//!   * a tool's telemetry log, written because the tool was running — not.
//!
//! The candidate discriminator is that the second is corroborated only by *other
//! unattended changes*, never by anything the person attended. This probe prints,
//! for every unattended change in every body of work in the real store, the
//! evidence tying it to each attended member, so the discriminator can be chosen
//! from what is actually there.
//!
//! Usage:
//!   cargo run --release -p evo-daemon --example probe_attribution
//!
//! Read-only: it derives from the canonical log and writes nothing.

use evo_engagement::{EngagementParams, Reconstruction, ResourceRole};
use std::time::Duration;

fn main() {
    let root = evo_storage::canonical_storage_root();
    let observations = match evo_daemon::persistence::load_persisted_observations(&root) {
        Ok(observations) => observations,
        Err(error) => {
            eprintln!("cannot read the canonical log: {error}");
            std::process::exit(1);
        }
    };
    println!("storage root: {}", root.display());
    println!("observations: {}\n", observations.len());

    let params = EngagementParams::default();
    let reconstruction = Reconstruction::from_observations(&observations, params.clone());
    let graph = reconstruction.graph();

    let mut unattended_total = 0usize;
    let mut with_attended_corroboration = 0usize;

    for (index, engagement) in reconstruction.engagements().iter().enumerate() {
        let members = engagement.subjects();

        // Split the members the way role assignment needs to: the ones a person
        // was demonstrably active in, and the ones Evo only saw change.
        let attended: Vec<&str> = engagement
            .participants()
            .iter()
            .filter(|p| p.attention() > Duration::ZERO || p.person_acted())
            .map(|p| p.subject())
            .collect();
        let unattended: Vec<_> = engagement
            .participants()
            .iter()
            .filter(|p| p.attention() == Duration::ZERO && !p.person_acted())
            .collect();

        if unattended.is_empty() {
            continue;
        }

        println!("── body of work #{} — {}", index + 1, engagement.title());
        println!("   attended members ({}):", attended.len());
        for subject in &attended {
            println!("     · {}", short(subject));
        }
        println!("   unattended changes ({}):", unattended.len());

        for participant in unattended {
            unattended_total += 1;
            println!(
                "     · {}  [{:?}] locality={:.2} sittings={} recurrence={}",
                short(participant.subject()),
                participant.role(),
                participant.locality(),
                participant.witnessed_sittings(),
                participant.recurrence(),
            );

            // Against every attended member: is there anything but co-presence?
            let mut any_attended_corroboration = false;
            for other in &attended {
                if let Some(evidence) = graph.evidence(participant.subject(), other) {
                    let corroborated = evidence.is_corroborated();
                    any_attended_corroboration |= corroborated;
                    println!(
                        "         → attended {:<44} interleave={:.3} co_ep={:.3} lexical={:.3} structural={:.3} corroborated={}",
                        short(other),
                        evidence.interleave,
                        evidence.co_episode,
                        evidence.lexical,
                        evidence.structural,
                        corroborated,
                    );
                }
            }

            // And against the other unattended changes, which is the loophole
            // being tested: churn vouching for churn.
            let mut peer_corroboration = 0usize;
            for other in members.iter() {
                if other.as_str() == participant.subject() || attended.contains(&other.as_str()) {
                    continue;
                }
                if graph
                    .evidence(participant.subject(), other)
                    .is_some_and(|evidence| evidence.is_corroborated())
                {
                    peer_corroboration += 1;
                }
            }
            println!(
                "         ⇒ corroborated by attention: {any_attended_corroboration} · by other unattended changes: {peer_corroboration}"
            );
            if any_attended_corroboration {
                with_attended_corroboration += 1;
            }
        }
        println!();
    }

    println!("== summary ==");
    println!("unattended changes across all bodies of work: {unattended_total}");
    println!("…corroborated by something attended:          {with_attended_corroboration}");
    println!(
        "…corroborated only by other unattended changes: {}",
        unattended_total - with_attended_corroboration
    );
}

/// Long paths and URLs make the table unreadable; the tail is the informative part.
fn short(subject: &str) -> String {
    const LIMIT: usize = 44;
    let characters: Vec<char> = subject.chars().collect();
    if characters.len() <= LIMIT {
        return subject.to_string();
    }
    let tail: String = characters[characters.len() - (LIMIT - 1)..].iter().collect();
    format!("…{tail}")
}
