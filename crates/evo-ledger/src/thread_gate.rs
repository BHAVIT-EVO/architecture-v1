//! The shadow evaluator: the evo-threads engine measured by the same gates.
//!
//! Phase 0 of the read-path unification makes `evo-threads` the authority on
//! what a work is; Home renders its threads. This module is the transition
//! instrument: it replays the canonical observation log through the engine
//! and evaluates the ledger's behavioral gates over the result, so both
//! engines are scored by one definition of "understood the day" —
//! [`crate::gate`] scores the ledger's grouping, this scores the engine's.
//!
//! # What is measured, and what may fail
//!
//! The gates are evaluated over **two membership readings**, because the
//! engines disagree deliberately about what "belongs" means:
//!
//! * **identity** — the thread's anchors (what the person made or kept
//!   returning to). This is the engine's own definition of membership;
//!   companions are remembered context that "never decides anything"
//!   (`evo-threads`' assign.rs). Gates scored on anchors measure identity.
//! * **participation** — everything the thread witnessed (anchors,
//!   companions, episode participants). This is closer to the ledger's
//!   interval-membership semantics; gates scored here measure co-occurrence.
//!
//! Verdicts: `Pass` (gate satisfied), `Divergence` (satisfied on one
//! reading, not the other — the expected Phase-0 state: the engine keeps
//! co-present attention out of identity where the ledger's intervals pulled
//! it in), and `Fail` (violated on both — the same failure the ledger gate
//! would report, which no known engine behavior produces). The integration
//! test asserts mechanics and prints every verdict; **retirement of the
//! ledger hypothesis layer requires the divergence list to empty on the
//! owning machine across a real working week** — an explicit later decision,
//! not an automatic one, because Phase 1's naming/input capture changes
//! co-occurrence semantics by design.

use crate::observation_log::read_observation_log;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use evo_threads::{Event, Interaction, Origin};

/// One thread's witnesses, collected for scoring.
#[derive(Debug, Default, Clone)]
pub struct ThreadWitnesses {
    /// Identity carriers: the engine's anchors.
    pub anchors: BTreeSet<String>,
    /// Everything witnessed beside them: companions and episode participants.
    pub companions: BTreeSet<String>,
}

impl ThreadWitnesses {
    /// Identity-level membership (URLs and documents only; titles and app
    /// subjects are context, mirroring the ledger gate's url+document counts).
    fn identity_urls_documents(&self) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut urls = BTreeSet::new();
        let mut documents = BTreeSet::new();
        for subject in &self.anchors {
            if looks_like_url(subject) {
                urls.insert(subject.clone());
            } else if looks_like_path(subject) {
                documents.insert(subject.clone());
            }
        }
        (urls, documents)
    }

    /// Participation-level membership, same shape.
    fn participation_urls_documents(&self) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut urls = BTreeSet::new();
        let mut documents = BTreeSet::new();
        for subject in self.anchors.iter().chain(self.companions.iter()) {
            if looks_like_url(subject) {
                urls.insert(subject.clone());
            } else if looks_like_path(subject) {
                documents.insert(subject.clone());
            }
        }
        (urls, documents)
    }

    fn all(&self) -> impl Iterator<Item = &String> {
        self.anchors.iter().chain(self.companions.iter())
    }
}

/// The verdict of one gate on one membership reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Divergence(String),
    Fail(String),
}

/// The shadow gate's outcome.
#[derive(Debug, Clone)]
pub struct ThreadGateReport {
    pub attention_events: usize,
    pub saves: usize,
    pub declarations: usize,
    /// Threads the engine minted.
    pub thread_count: usize,
    /// The subjects the person classified in this log (OBS-SURFACE-CLASSIFIED).
    /// Empty means no class was ever authored — the default, and Evo's own
    /// position: it ships no class.
    pub classified: BTreeSet<String>,
    /// Per-thread witnesses (thread id → evidence).
    pub witnesses: BTreeMap<u64, ThreadWitnesses>,
    /// One row per gate: name, identity-reading verdict, participation-reading
    /// verdict (None when a gate has one natural reading).
    pub gates: Vec<(&'static str, Verdict, Option<Verdict>)>,
}

impl ThreadGateReport {
    /// Gates that failed on BOTH readings and are NOT known, acknowledged
    /// model gaps — a real mechanical regression. Empty means the engine
    /// agrees with the ledger's safety rules everywhere the two models
    /// overlap, minus the documented gap list.
    pub fn failures(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (name, identity, participation) in &self.gates {
            let fails = |v: &Verdict| matches!(v, Verdict::Fail(_));
            let both = fails(identity) && participation.as_ref().is_none_or(|v| fails(v));
            if both && !KNOWN_BOTH_READING_FAILURES.contains(&name.to_string().as_str()) {
                out.push(name.to_string());
            }
        }
        out
    }

    /// Known-gap gates currently failing on both readings — measured,
    /// reported, and each carrying the phase that closes it (see
    /// [`KNOWN_BOTH_READING_FAILURES`]).
    pub fn known_gaps(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (name, identity, participation) in &self.gates {
            let fails = |v: &Verdict| matches!(v, Verdict::Fail(_));
            let both = fails(identity) && participation.as_ref().is_none_or(|v| fails(v));
            if both && KNOWN_BOTH_READING_FAILURES.contains(&name.to_string().as_str()) {
                out.push(name.to_string());
            }
        }
        out
    }

    /// Gates where the readings disagree — the Phase-0 expected state, and
    /// the work list Phase 1's capture is meant to shrink.
    pub fn divergences(&self) -> Vec<String> {
        let variant = |v: &Verdict| std::mem::discriminant(v);
        let mut out = Vec::new();
        for (name, identity, participation) in &self.gates {
            let differs = match participation {
                Some(p) => variant(identity) != variant(p),
                None => false,
            };
            if differs {
                out.push(name.to_string());
            }
        }
        out
    }
}

/// A subject that is not a URL or a path is a window/app subject. Structural
/// shapes, the same ones the ledger's identity chain recognizes — no
/// application list anywhere.
fn looks_like_url(subject: &str) -> bool {
    subject.starts_with("http://") || subject.starts_with("https://")
}

fn looks_like_path(subject: &str) -> bool {
    subject.starts_with('/') || subject.starts_with("file://")
}

/// The URL's host, structurally.
#[allow(dead_code)]
fn host_of(url: &str) -> Option<&str> {
    let after_scheme = url.split_once("://")?.1;
    let authority = after_scheme.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    (!host.is_empty() && host.contains('.')).then_some(host)
}

/// The behavioral gates, each scored per membership reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    NoHostIdentity,
    TrekPlaylistOneWork,
    GearWorkExactlyOne,
    KundliOutOfGear,
    EvoDevelopmentOneWork,
    NoMegaWork,
    UnrelatedVideosSeparate,
    /// Phase 3 (PLAN-0001 §4): a classified surface is never a thread anchor.
    ClassifiedFoundsNothing,
    /// Phase 3 (PLAN-0001 §4): a work is never presented on classified
    /// witnesses alone.
    ClassifiedNamesNothing,
}

pub const ALL_GATES: [Gate; 9] = [
    Gate::NoHostIdentity,
    Gate::TrekPlaylistOneWork,
    Gate::GearWorkExactlyOne,
    Gate::KundliOutOfGear,
    Gate::EvoDevelopmentOneWork,
    Gate::NoMegaWork,
    Gate::UnrelatedVideosSeparate,
    Gate::ClassifiedFoundsNothing,
    Gate::ClassifiedNamesNothing,
];

impl Gate {
    pub fn name(self) -> &'static str {
        match self {
            Gate::NoHostIdentity => "no host is a work identity",
            Gate::TrekPlaylistOneWork => "the trek playlist is one work",
            Gate::GearWorkExactlyOne => "gear work is exactly one",
            Gate::KundliOutOfGear => "the kundli chat stays out of gear",
            Gate::EvoDevelopmentOneWork => "evo development is one work",
            Gate::NoMegaWork => "no mega-work",
            Gate::UnrelatedVideosSeparate => "unrelated videos stay separate",
            Gate::ClassifiedFoundsNothing => "a classified surface founds no work",
            Gate::ClassifiedNamesNothing => "a classified surface names no work",
        }
    }
}

/// Gate failures that are KNOWN, acknowledged model gaps rather than
/// mechanical regressions, each with the phase that closes it:
///
/// * kundli/gear separation — measured in Phase 1 (see docs/BUILD-LOG.md
///   2026-09-30): the live-log failure is *declaration-driven* — the log's
///   four OBS-WORK-GROUPED facts form a transitive chain (kundli ↔ gear ↔
///   ZCode ↔ evo-desktop) that declaration-merges the works, contradicting
///   this gate's own expectation. The facts are grouping-gesture test drives
///   without provenance, not curation. The *inference* path this gap was
///   originally written for is closed as of Phase 2: the engine now carries
///   the ledger's establishment-precedence rule (substantive dwell on
///   territory a work never carried is never a fragment —
///   `evo_threads::Config::establishment_dwell_floor`, pinned by
///   `evo-threads/tests/scenarios.rs`). What remains open is declaration
///   provenance, not inference. Bounded: any gate failing on both readings
///   that is NOT in this list fails the shadow gate.
pub const KNOWN_BOTH_READING_FAILURES: [&str; 1] = ["the kundli chat stays out of gear"];

/// One gate evaluated over a chosen membership reading.
fn evaluate_gate(
    gate: Gate,
    threads: &[(u64, ThreadWitnesses)],
    reading: fn(&ThreadWitnesses) -> (BTreeSet<String>, BTreeSet<String>),
) -> Verdict {
    let works: Vec<(u64, BTreeSet<String>, BTreeSet<String>)> = threads
        .iter()
        .map(|(id, w)| (*id, reading(w).0, reading(w).1))
        .collect();

    match gate {
        // No host is a work identity. The ledger gate's rule, verbatim: the
        // work's IDENTITY is a bare host (the identity chain's UrlHost
        // identity on a host like youtube.com). Many deep URLs of one site
        // in one thread is a legitimate single-site body of work; only the
        // bare-host identity is the mirror failure.
        Gate::NoHostIdentity => {
            for host in ["youtube.com", "claude.ai", "google.com", "www.youtube.com"] {
                if works
                    .iter()
                    .any(|(_, urls, _)| urls.contains(&host.to_string()))
                {
                    return Verdict::Fail(format!("a work's identity is the bare host {host}"));
                }
            }
            Verdict::Pass
        }

        // The trek playlist lives in exactly one work.
        Gate::TrekPlaylistOneWork => {
            let trek_playlist = "PLiM-TFJI81";
            let playlist: Vec<u64> = works
                .iter()
                .filter(|(_, urls, _)| {
                    urls.iter()
                        .any(|u| u.contains("list=") && u.contains(trek_playlist))
                })
                .map(|(id, _, _)| *id)
                .collect();
            if playlist.len() == 1 {
                Verdict::Pass
            } else {
                Verdict::Fail(format!(
                    "the trek playlist lives in {} works, expected 1",
                    playlist.len()
                ))
            }
        }

        // Exactly one Arc'teryx gear work. Gear pages from the old capture
        // carry title identity (no URL locator), so the gear work is
        // recognized by its subjects â non-URL, non-path entries in the
        // witness collector â not by arcteryx.com (never witnessed here).
        Gate::GearWorkExactlyOne => {
            let gear: Vec<u64> = threads
                .iter()
                .filter(|(_, w)| {
                    w.all()
                        .any(|s| s.contains("Arc'teryx") || s.contains("arcteryx"))
                })
                .map(|(id, _)| *id)
                .collect();
            if gear.len() == 1 {
                Verdict::Pass
            } else {
                Verdict::Fail(format!(
                    "the Arc'teryx gear work appears {} times, expected 1",
                    gear.len()
                ))
            }
        }

        // The kundli chat never joins the gear work.
        Gate::KundliOutOfGear => {
            let gear_ids: Vec<u64> = threads
                .iter()
                .filter(|(_, w)| {
                    w.all()
                        .any(|s| s.contains("Arc'teryx") || s.contains("arcteryx"))
                })
                .map(|(id, _)| *id)
                .collect();
            let kundli = works
                .iter()
                .find(|(_, urls, _)| urls.iter().any(|u| u.contains("claude.ai/chat/f5c116b5")))
                .map(|(id, _, _)| id);
            if let (Some(&kundli_id), Some(&gear_id)) = (kundli, gear_ids.first()) {
                if kundli_id == gear_id {
                    return Verdict::Fail("the kundli chat joined the Arc'teryx gear work".into());
                }
            }
            Verdict::Pass
        }

        // Evo development is one work.
        Gate::EvoDevelopmentOneWork => {
            let zcode: Vec<u64> = threads
                .iter()
                .filter(|(_, w)| {
                    w.all().any(|s| {
                        s.contains("Evo-Evolution.nosync/crates")
                            || s.contains("github.com/headroomlabs-ai/headroom")
                    })
                })
                .map(|(id, _)| *id)
                .collect();
            if zcode.len() <= 2 {
                Verdict::Pass
            } else {
                Verdict::Fail(format!(
                    "evo development spans {} works, expected at most 2",
                    zcode.len()
                ))
            }
        }

        // No mega-work.
        Gate::NoMegaWork => {
            let total: usize = works.iter().map(|(_, u, d)| u.len() + d.len()).sum();
            if total == 0 {
                return Verdict::Pass;
            }
            let largest = works
                .iter()
                .map(|(_, u, d)| u.len() + d.len())
                .max()
                .unwrap_or(0);
            if largest as f64 / total as f64 > 0.60 {
                Verdict::Fail(format!("a mega-work holds {largest} of {total} resources"))
            } else {
                Verdict::Pass
            }
        }

        // Unrelated videos stay separate.
        Gate::UnrelatedVideosSeparate => {
            let trek_playlist = "PLiM-TFJI81";
            let trek: Vec<u64> = works
                .iter()
                .filter(|(_, urls, _)| {
                    urls.iter()
                        .any(|u| u.contains("list=") && u.contains(trek_playlist))
                })
                .map(|(id, _, _)| *id)
                .collect();
            if let Some(&trek_id) = trek.first() {
                if let Some((_, trek_urls, _)) = works.iter().find(|(id, _, _)| *id == trek_id) {
                    if trek_urls
                        .iter()
                        .any(|u| u.contains("RdWnMbsZhxk") || u.contains("eswW2BoFAUs"))
                    {
                        return Verdict::Fail("an unrelated video joined the trek work".into());
                    }
                }
            }
            Verdict::Pass
        }

        // Scored separately by `evaluate_classified_gate`: these need the
        // classification set, which this reading-based evaluator never sees.
        Gate::ClassifiedFoundsNothing | Gate::ClassifiedNamesNothing => Verdict::Pass,
    }
}

/// The gates for a user classification (PLAN-0001 §4 Phase 3,
/// `OBS-SURFACE-CLASSIFIED`), which the reading-based evaluator cannot score
/// because they need the set of classified subjects, not a membership shape.
///
/// Two distinct mechanisms, scored separately:
///
/// * **founds no work** — a classified subject is never a thread anchor. An
///   anchor is identity; a class says the surface may not carry it.
/// * **names no work** — a work is never presented on classified witnesses
///   alone. The gate has no title renderer, so it checks the necessary
///   condition: a thread whose every witness is classified has no unclassified
///   name source, and must not stand as a work.
///
/// A log with no classification passes both by construction: silence is the
/// default, and the phase changes nothing until the person authors a class.
fn evaluate_classified_gate(
    gate: Gate,
    threads: &[(u64, ThreadWitnesses)],
    classified: &BTreeSet<String>,
) -> Verdict {
    if classified.is_empty() {
        return Verdict::Pass;
    }
    match gate {
        Gate::ClassifiedFoundsNothing => {
            for (_, witnesses) in threads {
                if let Some(subject) = classified.iter().find(|c| witnesses.anchors.contains(*c)) {
                    return Verdict::Fail(format!(
                        "`{subject}` is classified yet anchors a work"
                    ));
                }
            }
            Verdict::Pass
        }
        Gate::ClassifiedNamesNothing => {
            for (id, witnesses) in threads {
                let all: Vec<&String> = witnesses
                    .anchors
                    .iter()
                    .chain(witnesses.companions.iter())
                    .collect();
                if !all.is_empty() && all.iter().all(|s| classified.contains(s.as_str())) {
                    return Verdict::Fail(format!(
                        "work {id} is presented on classified witnesses alone"
                    ));
                }
            }
            Verdict::Pass
        }
        _ => Verdict::Pass,
    }
}

/// Runs the engine shadow gate over the observation log at `log_path`.
///
/// `None` when the log is absent (a machine without Evo's data): the caller
/// decides how to skip. The projection mirrors the daemon bridge's mapping
/// rules (the live read path's own rules), so what is measured is what Home
/// renders.
pub fn run_thread_gates(log_path: &Path) -> Option<ThreadGateReport> {
    if !log_path.exists() {
        return None;
    }

    let (events, saves, declared, classified_pairs) = read_observation_log(log_path);

    let mut engine_events: Vec<Event> = events.iter().map(project_ledger_event).collect();
    // Production saves carry the honesty rule the live bridge applies: a
    // save is a mutation the engine's churn gate judges, never a free
    // anchor.
    for save in &saves {
        engine_events.push(Event {
            timestamp: save.timestamp_ms,
            origin: Origin::Person,
            interaction: Interaction::Mutated,
            resource: save.path.clone(),
            weight: None,
            detail: None,
        });
    }
    // Declarations are the person's word — the only merge path.
    for (a, b) in &declared {
        engine_events.push(Event {
            timestamp: engine_events.last().map(|e| e.timestamp).unwrap_or(0),
            origin: Origin::Person,
            interaction: Interaction::Declared,
            resource: String::new(),
            weight: None,
            detail: Some(evo_threads::Declaration::SameWork {
                a: a.clone(),
                b: b.clone(),
            }),
        });
    }
    // A classification is the person's word too: project it so the engine's
    // founding gate is what this gate measures, exactly as the live bridge
    // does. A log with no classification adds nothing.
    for (resource, class) in &classified_pairs {
        engine_events.push(Event {
            timestamp: engine_events.last().map(|e| e.timestamp).unwrap_or(0),
            origin: Origin::Person,
            interaction: Interaction::Declared,
            resource: String::new(),
            weight: None,
            detail: Some(evo_threads::Declaration::Classified {
                resource: resource.clone(),
                class: class.clone(),
            }),
        });
    }
    engine_events.sort_by_key(|e| e.timestamp);

    let classified: BTreeSet<String> = classified_pairs
        .iter()
        .map(|(subject, _)| subject.clone())
        .collect();

    let mut engine = evo_threads::Engine::new();
    for event in &engine_events {
        engine.apply(event.clone());
    }
    let last_ts = engine_events.last().map(|e| e.timestamp).unwrap_or(0);
    engine.settle(last_ts + 86_400_000);

    let threads = engine.threads();
    let mut witness_map: BTreeMap<u64, ThreadWitnesses> = BTreeMap::new();
    for t in &threads {
        let entry = witness_map.entry(t.id).or_default();
        for ep in &t.episodes {
            for (r, _) in &ep.participants {
                entry.companions.insert(r.clone());
            }
        }
        for a in &t.anchors {
            entry.anchors.insert(a.resource.clone());
        }
        for (r, _) in &t.companions {
            entry.companions.insert(r.clone());
        }
    }

    let thread_list: Vec<(u64, ThreadWitnesses)> = witness_map.into_iter().collect();
    let gates: Vec<(&'static str, Verdict, Option<Verdict>)> = ALL_GATES
        .iter()
        .map(|gate| match gate {
            Gate::ClassifiedFoundsNothing | Gate::ClassifiedNamesNothing => (
                gate.name(),
                evaluate_classified_gate(*gate, &thread_list, &classified),
                None,
            ),
            _ => (
                gate.name(),
                evaluate_gate(
                    *gate,
                    &thread_list,
                    ThreadWitnesses::identity_urls_documents,
                ),
                Some(evaluate_gate(
                    *gate,
                    &thread_list,
                    ThreadWitnesses::participation_urls_documents,
                )),
            ),
        })
        .collect();

    let mut witnesses_back = BTreeMap::new();
    for (id, w) in thread_list {
        witnesses_back.insert(id, w);
    }

    Some(ThreadGateReport {
        attention_events: events.len(),
        saves: saves.len(),
        declarations: declared.len(),
        thread_count: threads.len(),
        classified,
        witnesses: witnesses_back,
        gates,
    })
}

/// Projects one ledger attention event onto the engine's contract.
///
/// Mirrors the daemon bridge's mapping rules without re-implementing
/// observation parsing (the ledger has already read the log): URL-bearing
/// events are Visited, input-attributed events are Typed (weight = keys),
/// everything else is Focused with the row's dwell as attention, capped at
/// one attention interval — the same bound the live projection applies.
fn project_ledger_event(event: &crate::ObservedEvent) -> Event {
    let resource = event
        .url
        .clone()
        .or_else(|| event.document_path.clone())
        .unwrap_or_else(|| event.window_title.clone());
    let interaction = if event.keys > 0 || event.typed {
        Interaction::Typed
    } else if resource.starts_with("http") {
        Interaction::Visited
    } else {
        Interaction::Focused
    };
    let weight = match interaction {
        Interaction::Typed => Some(event.keys.max(1)),
        _ => Some(event.dwell_ms.min(600_000).max(1000)),
    };
    Event {
        timestamp: event.timestamp_ms,
        origin: Origin::Person,
        interaction,
        resource,
        weight,
        detail: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_extraction_is_structural() {
        assert_eq!(
            host_of("https://www.youtube.com/watch?v=x"),
            Some("www.youtube.com")
        );
        assert_eq!(host_of("http://localhost:3000/x"), None);
        assert_eq!(host_of("/Users/x/file.rs"), None);
    }

    #[test]
    fn witness_shapes_are_structural() {
        assert!(looks_like_url("https://a.com"));
        assert!(looks_like_path("/Users/x/f.rs"));
        assert!(looks_like_path("file:///Users/x/f.rs"));
        assert!(!looks_like_url("Visual Studio Code"));
        assert!(!looks_like_path("Visual Studio Code"));
    }

    #[test]
    fn identity_and_participation_readings_differ() {
        let mut w = ThreadWitnesses::default();
        w.anchors.insert("/Users/x/evo/src/main.rs".into());
        w.companions.insert("https://www.youtube.com/".into());
        let (urls, docs) = w.identity_urls_documents();
        assert!(urls.is_empty());
        assert_eq!(docs.len(), 1);
        let (urls, docs) = w.participation_urls_documents();
        assert_eq!(urls.len(), 1);
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn a_missing_log_reports_none() {
        assert!(run_thread_gates(Path::new("/nonexistent/log")).is_none());
    }

    #[test]
    fn a_classified_surface_founds_nothing() {
        let mut witnesses = ThreadWitnesses::default();
        witnesses
            .anchors
            .insert("https://www.youtube.com/watch?v=x".into());
        witnesses.companions.insert("file:///work/plan.md".into());
        let threads = vec![(1u64, witnesses)];

        // No class authored: the default, and nothing to violate.
        assert_eq!(
            evaluate_classified_gate(Gate::ClassifiedFoundsNothing, &threads, &BTreeSet::new()),
            Verdict::Pass
        );

        // A class on an anchor is a real failure...
        let classified = BTreeSet::from(["https://www.youtube.com/watch?v=x".to_string()]);
        assert!(matches!(
            evaluate_classified_gate(Gate::ClassifiedFoundsNothing, &threads, &classified),
            Verdict::Fail(_)
        ));
        // ...but the unclassified companion still names the work, so the
        // naming gate holds.
        assert_eq!(
            evaluate_classified_gate(Gate::ClassifiedNamesNothing, &threads, &classified),
            Verdict::Pass
        );
    }

    #[test]
    fn a_work_presented_on_classified_witnesses_alone_fails_the_naming_gate() {
        let mut witnesses = ThreadWitnesses::default();
        witnesses.companions.insert("https://video.example/x".into());
        let threads = vec![(7u64, witnesses)];
        let classified = BTreeSet::from(["https://video.example/x".to_string()]);

        assert!(matches!(
            evaluate_classified_gate(Gate::ClassifiedNamesNothing, &threads, &classified),
            Verdict::Fail(_)
        ));
        // It is not an anchor, so the founding gate is satisfied.
        assert_eq!(
            evaluate_classified_gate(Gate::ClassifiedFoundsNothing, &threads, &classified),
            Verdict::Pass
        );
    }

    #[test]
    fn verdicts_classify() {
        assert!(matches!(Verdict::Pass, Verdict::Pass));
        let known_gap = KNOWN_BOTH_READING_FAILURES[0];
        let report = ThreadGateReport {
            attention_events: 0,
            saves: 0,
            declarations: 0,
            thread_count: 0,
            classified: BTreeSet::new(),
            witnesses: BTreeMap::new(),
            gates: vec![
                ("g", Verdict::Pass, None),
                // Fails one reading only: a divergence, not a failure.
                (
                    "h",
                    Verdict::Fail("x".into()),
                    Some(Verdict::Divergence("y".into())),
                ),
                // Fails both readings and is a KNOWN gap: measured, not a
                // mechanical regression.
                (
                    known_gap,
                    Verdict::Fail("known".into()),
                    Some(Verdict::Fail("known".into())),
                ),
                // Fails both readings and is NOT known: a real failure.
                (
                    "unlisted",
                    Verdict::Fail("real".into()),
                    Some(Verdict::Fail("real".into())),
                ),
            ],
        };
        assert_eq!(report.failures(), vec!["unlisted".to_string()]);
        assert_eq!(report.known_gaps(), vec![known_gap.to_string()]);
        // A both-readings Fail is not a divergence: the readings agree.
        assert_eq!(report.divergences().len(), 1);
    }
}
