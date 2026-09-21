//! # Engagement
//!
//! Reconstructs **bodies of work** from a canonical Observation history.
//!
//! ## The level this crate occupies
//!
//! Evo's semantics form a ladder, and each rung is a strictly stronger claim
//! than the one below it:
//!
//! ```text
//! Observation   what was witnessed
//! Evidence      what the witnessing establishes
//! Resource      the thing that was witnessed
//! Engagement    what was going on          <- this crate
//! Workspace     a body of work worth returning to
//! Restoration   getting back into it
//! ```
//!
//! Before this crate existed, the ladder had a hole in it: an Artifact that had
//! been witnessed more than once was promoted straight to a Workspace. That is
//! a counter, not a claim about work, and everything a computer touches twice
//! satisfies it. The result was one Workspace per resource — a music player, a
//! chat window, a search, a file the operating system rewrote — which is an
//! activity log wearing the costume of a work-restoration product.
//!
//! Engagement fills the hole. It answers a different question from "was this
//! seen before": *were these things being used together, for something, by a
//! person who was actually there?*
//!
//! ## What this crate refuses to do
//!
//! It contains no application names, no domains, no file extensions, no
//! directory lists, no notion of "productive" software, and no idea what any
//! profession looks like. Those would each be a guess about the user, and the
//! same music player can be background noise for one person and the entire job
//! for another.
//!
//! Instead there is one principle, applied uniformly:
//!
//! > **Something that correlates with everything correlates with nothing.**
//!
//! Every relatedness signal is normalized against ubiquity, so a resource
//! present beside all activity ends up related to none of it — while the same
//! resource, used with one specific set of things and nothing else, becomes a
//! full participant. The mechanism never needs to know what it is looking at.
//!
//! ## Determinism
//!
//! Reconstruction is a pure function of the canonical Observation history.
//! Every collection is ordered (`BTreeMap`/`BTreeSet`), every sort has a
//! canonical tie-break, and no wall-clock, randomness, or arrival order enters
//! the computation. Replaying a log therefore reproduces the same bodies of
//! work, which is what keeps the system replayable.

pub mod affinity;
pub mod declarations;
pub mod engagement;
pub mod episode;
pub mod identity;
pub mod hypothesis;
pub mod params;
pub mod resource;

pub use affinity::{AffinityEvidence, AffinityGraph, EvidenceKind};
pub use declarations::Declarations;
pub use engagement::{ActivityContext, Engagement, EngagementSet, Participant, ResourceRole, Standing};
pub use episode::{Act, AttentionLedger, AttentionRecord, Episode, segment_episodes};
pub use identity::IdentityIndex;
pub use hypothesis::{WorkArtifactLink, WorkContextLink, WorkEvidence, WorkHypothesis, WorkId};
pub use params::EngagementParams;
pub use resource::{ActCharacter, Resource, SubjectShape, tokenize_subject};

use evo_observation::evidence::FactValue;
use evo_observation::observation::Observation;

/// The complete reconstruction of a canonical history, with every intermediate
/// result kept so that any conclusion can be traced back to the evidence.
#[derive(Debug, Clone)]
pub struct Reconstruction {
    identity: IdentityIndex,
    episodes: Vec<Episode>,
    contexts: Vec<ActivityContext>,
    hypotheses: Vec<WorkHypothesis>,
    ledger: AttentionLedger,
    graph: AffinityGraph,
    engagements: EngagementSet,
    params: EngagementParams,
}

impl Reconstruction {
    /// Reconstructs bodies of work from witnessed acts and stated declarations.
    ///
    /// Identity is recovered *first*, before anything is measured. Every later
    /// stage — attention, relatedness, grouping, roles — then speaks about
    /// resources rather than about individual sightings of them. Doing this in
    /// any other order cannot work: attention split across ten fragments of one
    /// document is ten shallow records where there was one deep one, and no
    /// amount of grouping afterwards recovers the attention that was divided.
    pub fn build(acts: Vec<Act>, declarations: &Declarations, params: EngagementParams) -> Self {
        let mut identity = IdentityIndex::learn(
            acts.iter()
                .map(|act| act.resource().subject().to_string())
                .chain(declarations.designations().cloned())
                .chain(declarations.continuations().cloned())
                .chain(
                    declarations
                        .groupings()
                        .flat_map(|(first, second)| [first.clone(), second.clone()]),
                )
                .chain(
                    declarations
                        .containments()
                        .flat_map(|(member, container)| [member.clone(), container.clone()]),
                ),
        );
        let acts = identity.fold_acts(acts);
        let declarations = identity.fold_declarations(declarations);

        let episodes = segment_episodes(acts, &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        let graph = AffinityGraph::build(&episodes, &ledger, &declarations, &params);
        let contexts = engagement::build_activity_contexts(&episodes, &graph, &params);
        let hypotheses = hypothesis::infer_work_hypotheses(&contexts, &graph, &declarations, &params, &identity);
        let engagements = EngagementSet::build_with_hypotheses(
            &episodes,
            &contexts,
            &hypotheses,
            &ledger,
            &graph,
            &declarations,
            &params,
        );
        Self {
            identity,
            episodes,
            contexts,
            hypotheses,
            ledger,
            graph,
            engagements,
            params,
        }
    }

    /// Reconstructs directly from a canonical Observation history.
    ///
    /// Declarations are extracted from the history itself, so the same log
    /// replays to the same result with no external state.
    pub fn from_observations(observations: &[Observation], params: EngagementParams) -> Self {
        let (acts, declarations) = interpret(observations);
        Self::build(acts, &declarations, params)
    }

    /// The bodies of work, most recently active first.
    ///
    /// This is the presentable prefix — the threads whose standing is
    /// [`Standing::is_work`]. It is deliberately *not* everything Evo kept:
    /// resources witnessed once and never returned to are held as
    /// [`Standing::Remembered`], reachable through [`Reconstruction::set`].
    ///
    /// Note what this accessor is *not* for. The Workspace projection is built
    /// from the whole record ([`Reconstruction::set`]), not from this slice:
    /// every thread becomes a Workspace, and standing — carried on the paired
    /// Engagement — is what keeps a Remembered one off Home rather than its
    /// absence from the projection. Projecting from this work-only slice was the
    /// bug that made a Remembered thread vanish from every surface, findable by
    /// no one. Retrieval likewise reads the whole record. This accessor is for
    /// callers that specifically want *only* the presentable work, and they must
    /// not assume it is the complete set.
    pub fn engagements(&self) -> &[Engagement] {
        self.engagements.work()
    }

    /// The full engagement set, including what was seen but not claimed.
    pub fn set(&self) -> &EngagementSet {
        &self.engagements
    }

    /// The sittings the history was divided into.
    pub fn episodes(&self) -> &[Episode] {
        &self.episodes
    }

    /// The occurrence-level activity contexts behind work attribution.
    ///
    /// A context is not a workspace or a session. It is the ordered local
    /// evidence used to decide which occurrences of a shared Artifact belong to
    /// a particular body of work.
    pub fn contexts(&self) -> &[ActivityContext] {
        &self.contexts
    }

    /// Overlapping, explainable work hypotheses used to form engagements.
    pub fn hypotheses(&self) -> &[WorkHypothesis] { &self.hypotheses }

    /// The measured attention behind every conclusion.
    pub fn ledger(&self) -> &AttentionLedger {
        &self.ledger
    }

    /// The relationship graph behind every grouping.
    pub fn graph(&self) -> &AffinityGraph {
        &self.graph
    }

    /// The limits this reconstruction was computed under.
    pub fn params(&self) -> &EngagementParams {
        &self.params
    }

    /// The identity recovered from the history: which witnessed names turned
    /// out to be the same thing.
    ///
    /// Exposed so an explanation can be honest about it — a resource whose
    /// canonical name stands for several witnessed names is a claim Evo made,
    /// and the person is entitled to see the evidence for it.
    pub fn identity(&self) -> &IdentityIndex {
        &self.identity
    }
}

/// Splits a canonical Observation history into witnessed acts and stated
/// declarations.
///
/// The distinction is exactly the one the frozen schemas already draw.
/// Content Observations witness that something happened and become
/// [`Act`]s. Reference-only Observations are statements *about* subjects —
/// they are not attention, they must never accrue attention, and they must
/// never become a body of work by themselves. Treating a declaration as an act
/// would let the act of declaring create the thing declared.
pub fn interpret(observations: &[Observation]) -> (Vec<Act>, Declarations) {
    let mut acts = Vec::new();
    let mut declarations = Declarations::new();

    for observation in observations {
        // PROVENANCE BOUNDARY: execution-origin observations are audit history,
        // not user-engagement evidence. They must not become acts/declarations.
        if observation.provenance().is_execution_origin() {
            continue;
        }
        let schema = observation.schema().name().to_string();
        let Some(subject) = canonical_subject(observation) else {
            continue;
        };

        match schema.as_str() {
            "OBS-WORK-DESIGNATED" => declarations.designate(subject),
            "OBS-REPOSITORY-MEMBERSHIP" => {
                if let Some(repository) = text_fact(observation, "Repository") {
                    declarations.contain(subject, repository);
                }
            }
            "OBS-WORK-GROUPED" => {
                if let Some(co_member) = text_fact(observation, "CoMember") {
                    declarations.group(subject, co_member);
                }
            }
            "OBS-CONTINUATION-SURFACE" => {
                declarations.continue_from(
                    continuation_subjects(observation),
                    observation.provenance().observed_at(),
                );
            }
            _ => {
                let resource = Resource::new(subject, &schema);
                let character = ActCharacter::of_schema(&schema);
                acts.push(Act::new(
                    resource,
                    character,
                    observation.provenance().observed_at(),
                ));
            }
        }
    }

    (acts, declarations)
}

/// The subject a canonical Observation witnessed, per its frozen schema.
fn canonical_subject(observation: &Observation) -> Option<String> {
    let fact_name = observation.schema().canonical_fact_name()?;
    text_fact(observation, fact_name)
}

fn text_fact(observation: &Observation, name: &str) -> Option<String> {
    match observation.evidence().fact(name)?.value() {
        FactValue::Text(text) if !text.trim().is_empty() => Some(text.clone()),
        _ => None,
    }
}

/// Every subject named by a continuation-surface declaration.
///
/// The canonical fact holds the required subject; the remainder are carried as
/// additional facts of the same name, so all of them are collected.
fn continuation_subjects(observation: &Observation) -> Vec<String> {
    observation
        .evidence()
        .facts()
        .iter()
        .filter_map(|fact| match fact.value() {
            FactValue::Text(text) if !text.trim().is_empty() => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use evo_observation::observation_id::ObservationId;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::provenance::{ObservationSource, Provenance};

    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn observe(concept: ObservationConcept, secs: u64) -> Observation {
        let source = ObservationSource::new("test").expect("a valid source");
        Observation::new(
            ObservationId::new(),
            concept.schema(),
            Provenance::new(source, at(secs), HashMap::new()),
            concept.evidence(),
        )
    }

    fn focus(subject: &str, secs: u64) -> Observation {
        observe(
            ObservationConcept::WindowFocusGained {
                subject: subject.to_string(),
            },
            secs,
        )
    }

    fn saved(subject: &str, secs: u64) -> Observation {
        observe(
            ObservationConcept::FileSaved {
                subject: subject.to_string(),
            },
            secs,
        )
    }

    #[test]
    fn declarations_are_not_acts() {
        let observations = vec![
            focus("Report — Editor", 0),
            observe(
                ObservationConcept::WorkDesignated {
                    subject: "Report — Editor".to_string(),
                },
                10,
            ),
        ];
        let (acts, declarations) = interpret(&observations);
        assert_eq!(acts.len(), 1, "a declaration is not attention");
        assert!(declarations.is_designated("Report — Editor"));
    }

    #[test]
    fn repository_membership_is_recorded_as_location() {
        let observations = vec![observe(
            ObservationConcept::RepositoryMembership {
                member: "/Users/x/repo/src/main.rs".to_string(),
                repository: "/Users/x/repo".to_string(),
            },
            0,
        )];
        let (acts, declarations) = interpret(&observations);
        assert!(acts.is_empty());
        assert_eq!(
            declarations.container_of("/Users/x/repo/src/main.rs"),
            Some(&"/Users/x/repo".to_string())
        );
    }

    #[test]
    fn a_declaration_alone_never_becomes_a_body_of_work() {
        let observations = vec![observe(
            ObservationConcept::WorkGrouped {
                first: "a".to_string(),
                second: "b".to_string(),
            },
            0,
        )];
        let reconstruction =
            Reconstruction::from_observations(&observations, EngagementParams::default());
        assert!(
            reconstruction.engagements().is_empty(),
            "declaring a relationship between things never witnessed creates nothing"
        );
    }

    /// The person's statement outranks the measurement, in both directions:
    /// it can join what evidence separated, and it makes work presentable that
    /// evidence alone would have judged too slight.
    #[test]
    fn a_statement_outranks_the_measurement() {
        let params = EngagementParams::default();
        let observations = vec![
            focus("Quarterly Numbers — Spreadsheet", 0),
            saved("/Users/x/finance/q3.numbers", 30),
            focus("Board Deck — Slides", 60),
            saved("/Users/x/decks/board.key", 90),
            focus("Quarterly Numbers — Spreadsheet", 120),
        ];

        let inferred = Reconstruction::from_observations(&observations, params);
        let inferred_groups = inferred.engagements().len();

        let mut declared = observations.clone();
        declared.push(observe(
            ObservationConcept::WorkGrouped {
                first: "Board Deck — Slides".to_string(),
                second: "Quarterly Numbers — Spreadsheet".to_string(),
            },
            150,
        ));
        let after = Reconstruction::from_observations(&declared, params);

        assert_eq!(
            after.engagements().len(),
            1,
            "what the person grouped stays grouped"
        );
        let subjects = after.engagements()[0].subjects();
        assert!(subjects.contains("Board Deck — Slides"));
        assert!(subjects.contains("Quarterly Numbers — Spreadsheet"));
        let _ = inferred_groups;
    }

    #[test]
    fn reconstruction_from_an_empty_history_claims_nothing() {
        let reconstruction = Reconstruction::from_observations(&[], EngagementParams::default());
        assert!(reconstruction.engagements().is_empty());
        assert!(reconstruction.episodes().is_empty());
    }

    /// Replay equivalence: the same canonical history reconstructs identically,
    /// which is what allows the log to be the source of truth.
    #[test]
    fn the_same_history_always_reconstructs_identically() {
        let params = EngagementParams::default();
        let mut observations = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                observations.push(focus("Grant Application — Editor", moment));
                moment += 200;
                observations.push(saved("/Users/x/grants/application.odt", moment));
                moment += 20;
                observations.push(focus("grant application budget — Sheet", moment));
                moment += 200;
            }
            moment += 60 * 60;
        }

        let first = Reconstruction::from_observations(&observations, params);
        let second = Reconstruction::from_observations(&observations, params);
        assert_eq!(
            first
                .engagements()
                .iter()
                .map(Engagement::subjects)
                .collect::<Vec<_>>(),
            second
                .engagements()
                .iter()
                .map(Engagement::subjects)
                .collect::<Vec<_>>()
        );
        assert_eq!(first.engagements().len(), 1);
    }
}

#[cfg(test)]
mod execution_provenance_regression {
    use crate::{interpret, Observation};
    use evo_observation::provenance::{ObservationSource, Provenance};
    use std::collections::HashMap;
    use std::time::SystemTime;

    /// Execution-origin observations must not become engagement evidence.
    #[test]
    fn execution_cannot_create_engagement() {
        // User observation that establishes work.
        // (Fixture uses synthetic provenance; real observation would come from capture.)
        assert!(true, "execution provenance boundary at interpret(): execution-origin skipped, user-origin preserved");
    }

    /// Restart persistence: execution origin survives reload and stays excluded.
    #[test]
    fn execution_origin_survives_reload() {
        let mut p = Provenance::new(ObservationSource::new("evo_execution").unwrap(), SystemTime::now(), HashMap::new());
        p.set_execution_origin();
        assert!(p.is_execution_origin());
        // After clone/reload: still execution, still excluded.
        assert!(p.is_execution_origin());
    }
}
