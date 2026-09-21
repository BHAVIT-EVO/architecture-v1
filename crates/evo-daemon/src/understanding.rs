//! Deriving Workspace understanding from canonical Observation evidence.
//!
//! This module is the single place where the daemon turns witnessed evidence
//! into the things Home shows: the bodies of work worth returning to, and the
//! Restoration outcome derived for each of them.
//!
//! # What replaced what
//!
//! **The old rule.** Workspace understanding was built one Observation at a
//! time. Each accepted content Observation ran Workspace Formation against the
//! remembered Workspaces, and the result was appended to `workspace.log` as a
//! delta record. Understanding was therefore *stored* state, and the daemon
//! maintained two paths to it: the live per-Observation fold, and a separate
//! replay that re-ran the same fold over the log.
//!
//! **Why it could not work.** Whether two resources belong to one body of work
//! depends on how ubiquitous each of them is across *everything else* — a
//! resource present beside all activity is related to none of it. That is a
//! property of the corpus, and it is unknowable from one Observation. A
//! per-Observation decision has to guess, and an append-only store cannot
//! withdraw a guess that later turns out wrong. The observable consequence was
//! the failure this rewrite exists to fix: an Artifact witnessed twice became a
//! Workspace, so a music player, a chat window and a stray search each became
//! "work", and Home became the Observation log with different typography.
//!
//! **The new rule.** Understanding is a *projection* of the whole canonical
//! history:
//!
//! ```text
//! Observation log  →  Acts + Declarations  →  Reconstruction  →  Workspaces
//!                                                            →  Restoration outcomes
//! ```
//!
//! Nothing about the projection is persisted, because there is nothing a
//! persisted copy could add. Live state and replayed state are the same pure
//! function of the same input, so replay-equivalence holds by construction
//! rather than by keeping two code paths in agreement. Workspace identity
//! survives restart because it is *re-derived* from the founding sighting — an
//! immutable fact of an append-only log — and not because it was written down.
//!
//! This is why `workspace.log` and `restoration.log` are no longer read or
//! written. Their record formats could not express the new model in any case:
//! an attachment record carries no role, and the delta format can only ever
//! *add* attachments and snapshots, while a projection's membership and roles
//! both change as evidence accumulates.

use std::collections::{BTreeMap, HashMap};
use std::time::SystemTime;

use crate::persistence::{continuation_surface_subjects_of, designation_subject_of};
use evo_artifact::artifact_id::ArtifactId;
use evo_artifact::derive_artifact_id_from_observations;
use evo_engagement::{Act, Declarations, EngagementParams, Reconstruction, Standing};
use evo_observation::evidence::FactValue;
use evo_observation::observation::Observation;
use evo_observation::observation_schema::ObservationSchema;
use evo_observation::observed_state::ObservedState;
use evo_restoration::{derive_restoration_plan, DerivationOutcome, RestorationInput};
use evo_retrieval::{RetrievalIndex, WorkResolution};
use evo_workspace::projection::{project_engagements, SubjectArtifacts};
use evo_workspace::workspace::Workspace;

/// The derived Workspace understanding of one canonical history.
#[derive(Debug, Clone, Default)]
pub struct Understanding {
    /// The bodies of work, most recently active first.
    workspaces: Vec<Workspace>,
    /// The Restoration outcome derived for each Workspace, keyed by Workspace
    /// identity.
    outcomes: HashMap<String, DerivationOutcome>,
    /// What each body of work is *about*, keyed by Workspace identity.
    titles: HashMap<String, String>,
    /// How much of each body of work Evo is prepared to claim — whether it is
    /// presented as work at all, and whether it may be auto-opened — keyed by
    /// Workspace identity.
    standings: HashMap<String, Standing>,
    /// Which canonical Artifact each witnessed resource name resolves to.
    artifacts: SubjectArtifacts,
    retrieval: RetrievalIndex,
}

impl Understanding {
    /// Resolves a natural-language work reference to stable WorkId evidence.
    pub fn resolve_work(&self, reference: &str) -> WorkResolution {
        self.retrieval.resolve(reference)
    }
    /// The bodies of work, most recently active first.
    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    /// The Restoration outcomes, keyed by Workspace identity.
    pub fn outcomes(&self) -> &HashMap<String, DerivationOutcome> {
        &self.outcomes
    }

    /// What each body of work is about, keyed by Workspace identity.
    ///
    /// Derived, never persisted, and never canonical — the same status as
    /// [`Understanding::outcomes`], and for the same reason: it is a reading of
    /// the evidence, so it must be recomputed when the evidence grows rather
    /// than frozen at the moment the work was first noticed.
    ///
    /// # Authority change: Home names the work, not a window in it (§15, §26)
    ///
    /// **The old rule.** A Workspace carried no name and Home derived one by
    /// looking up the witnessed subject of the Workspace's Resume Point — the
    /// resource the person's attention was last in.
    ///
    /// **Why it prevented the product from working.** No resource's name is a
    /// name for the work containing it. On the real machine this titled an
    /// afternoon of coursework "Spotify Premium" and a body of work in which an
    /// RFC was being drafted "‎WhatsApp", because those were the windows holding
    /// attention when the sitting ended. §15 asks Home to say *what the person
    /// was working on* ("Continue working on / Internship assignment"); a
    /// window title answers a different question, and answering it in that slot
    /// makes Home read as an activity log — the precise failure this rewrite
    /// exists to end.
    ///
    /// **The new rule.** The name comes from the layer that measured what the
    /// members have in common: the Engagement chooses, among the resources the
    /// work actually continues in, the name carrying most of the work's own
    /// distinctive vocabulary ([`evo_engagement::Engagement::title`]). It is
    /// still a witnessed name — Evo invents no wording (§14) — but it is chosen
    /// *because* it is about the same thing as the rest of the work, rather than
    /// because it happened to be in front. When the members share no measurable
    /// vocabulary the Engagement says so
    /// ([`evo_engagement::Engagement::titled_by_shared_vocabulary`] is false) and
    /// the name comes instead from a resource the evidence never witnessed
    /// outside this work — not from whichever member held the most of this
    /// work's attention, which is the signal the vocabulary rule exists to
    /// overrule. Home therefore never presents an attention measurement as a
    /// meaning, in either branch. That is also deliberately not the Resume
    /// Point: naming the work and saying where it resumes are different claims,
    /// and Home reports them in different places rather than letting one stand
    /// in for the other.
    pub fn titles(&self) -> &HashMap<String, String> {
        &self.titles
    }

    /// How much of each body of work Evo is prepared to claim, keyed by
    /// Workspace identity.
    ///
    /// Derived, never persisted, and never canonical — the same status as
    /// [`Understanding::titles`], and re-read every derivation for the same
    /// reason: standing is a reading of the evidence, which grows.
    ///
    /// # Why this leaves the Engagement layer at all
    ///
    /// The Engagement layer already decides, for every grouping it forms,
    /// whether it is a body of work ([`Standing::Continuable`] or
    /// [`Standing::Restorable`]) or is only [`Standing::Remembered`] — witnessed
    /// and kept, but not a thing to present as work. That decision is the whole
    /// point of the crate's cohesion and significance limits, and it was being
    /// thrown away here: the projection turned *every* engagement into a
    /// Workspace and Home showed them all, so a login screen, a music player and
    /// an inbox appeared as bodies of work beside real ones — the exact failure
    /// [`WORK-MODEL`](../../../../docs/architecture/WORK-MODEL.md) exists to end.
    ///
    /// Carried here beside the title, keyed by Workspace identity so Home joins
    /// it to the workspace list the same way it joins a name. Nothing is
    /// dropped: a `Remembered` body of work is still projected, still a
    /// Workspace, still findable — its standing simply says Home must not
    /// present it as work. `50 remembered ≠ 50 opened` is expressed by standing,
    /// not by discarding.
    pub fn standings(&self) -> &HashMap<String, Standing> {
        &self.standings
    }

    /// Which canonical Artifact each witnessed resource name resolves to.
    ///
    /// The same resolution the projection used to attach members, exposed so a
    /// reader can join a body of work's per-resource evidence to the canonical
    /// selection keyed by Artifact identity — without re-deriving identity a
    /// second way and risking a different answer than the product's.
    pub fn artifacts(&self) -> &SubjectArtifacts {
        &self.artifacts
    }

    /// Consumes this understanding, yielding its derived views.
    pub fn into_parts(
        self,
    ) -> (
        Vec<Workspace>,
        HashMap<String, DerivationOutcome>,
        HashMap<String, String>,
        HashMap<String, Standing>,
    ) {
        (self.workspaces, self.outcomes, self.titles, self.standings)
    }
}

/// The canonical evidence one derivation reads.
///
/// Grouped into a struct rather than passed as six positional arguments so
/// that adding evidence later cannot silently reorder an existing call.
#[derive(Debug, Clone, Default)]
pub struct Evidence {
    /// Witnessed acts, in canonical append order.
    pub acts: Vec<Act>,
    /// Everything the person stated, plus witnessed structural containment.
    pub declarations: Declarations,
    /// Witnessed subject → the distinct canonical Artifacts established by the
    /// content Observations witnessing it.
    pub witnessed: HashMap<String, Vec<ArtifactId>>,
    /// The canonical designated Artifact (RFC-0011), when one resolves.
    pub designated: Option<ArtifactId>,
    /// The Current Continuation Surface resolved to canonical Artifacts
    /// (RFC-0013), when a valid declaration exists.
    pub surface: Option<Vec<ArtifactId>>,
    /// Occurrence-local interface state, including explicit absence. Keeping
    /// state-less occurrences is what prevents an older cursor/selection from
    /// being reused after a newer observation where that state was unobservable.
    pub observed_states: Vec<ObservedStateOccurrence>,
}

/// State directly witnessed with one canonical content Observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedStateOccurrence {
    pub subject: String,
    pub observed_at: SystemTime,
    pub state: Option<ObservedState>,
}

/// Builds the canonical evidence of a whole Observation history.
///
/// This is the batch counterpart of the daemon's record-at-a-time absorption in
/// [`crate::cache`]. Both must produce the same evidence, and a differential
/// test holds them to it — that equality is what makes replay a check on the
/// live path rather than a second implementation of it.
///
/// Reference-only schemas (RFC-0011 WorkDesignated; RFC-0012
/// RepositoryMembership and WorkGrouped; RFC-0013 ContinuationSurface)
/// reference existing canonical Artifacts and never establish one, so they
/// never enter the witnessed map. The current designation and Continuation
/// Surface are resolved by the same supersession rule the live index applies:
/// latest canonical Observation Time wins, and on equal time the later
/// append-order record wins (RFC-0011 §5).
pub fn evidence_of(observations: &[Observation]) -> Evidence {
    let (acts, declarations) = evo_engagement::interpret(observations);

    let mut witnessed: HashMap<String, Vec<ArtifactId>> = HashMap::new();
    let mut observed_states = Vec::new();
    for observation in observations {
        if observation.schema().is_reference_only() {
            continue;
        }
        let Some(subject) = witnessed_subject_of(observation) else {
            continue;
        };
        observed_states.push(ObservedStateOccurrence {
            subject: subject.clone(),
            observed_at: observation.provenance().observed_at(),
            state: ObservedState::from_context(observation.provenance().context()),
        });
        let Ok(artifact_id) =
            derive_artifact_id_from_observations(std::slice::from_ref(observation))
        else {
            continue;
        };
        let established = witnessed.entry(subject).or_default();
        if !established.contains(&artifact_id) {
            established.push(artifact_id);
        }
    }

    let designated = current_designation(observations)
        .and_then(|subject| resolve_subject(&witnessed, &subject));
    let surface = declared_continuation_surface(observations)
        .map(|subjects| resolve_surface(&witnessed, &subjects))
        .filter(|artifacts| !artifacts.is_empty());

    Evidence {
        acts,
        declarations,
        witnessed,
        designated,
        surface,
        observed_states,
    }
}

/// The subject of the current WorkDesignated declaration (RFC-0011): the latest
/// valid declaration by canonical Observation Time, with the later append-order
/// record winning on equal time (RFC-0011 §5).
pub fn current_designation(observations: &[Observation]) -> Option<String> {
    latest_declaration(observations, &ObservationSchema::work_designated_v1(), |o| {
        designation_subject_of(o)
    })
}

/// The declared subjects of the current Continuation Surface (RFC-0013), under
/// the same supersession rule.
pub fn declared_continuation_surface(observations: &[Observation]) -> Option<Vec<String>> {
    latest_declaration(
        observations,
        &ObservationSchema::continuation_surface_v1(),
        continuation_surface_subjects_of,
    )
}

/// The value of the latest valid declaration of one schema: later canonical
/// Observation Time wins, and on equal time the later append-order record wins
/// (RFC-0011 §5). Stated once so designation and surface currency cannot drift
/// apart.
fn latest_declaration<T>(
    observations: &[Observation],
    schema: &ObservationSchema,
    value_of: impl Fn(&Observation) -> Option<T>,
) -> Option<T> {
    let mut current: Option<(SystemTime, usize, T)> = None;
    for (index, observation) in observations.iter().enumerate() {
        if observation.schema() != schema {
            continue;
        }
        let Some(value) = value_of(observation) else {
            continue;
        };
        let time = observation.provenance().observed_at();
        let supersedes = match &current {
            None => true,
            Some((current_time, current_index, _)) => {
                time > *current_time || (time == *current_time && index > *current_index)
            }
        };
        if supersedes {
            current = Some((time, index, value));
        }
    }
    current.map(|(_, _, value)| value)
}

/// The subject a content Observation witnessed, when it witnessed a usable one.
///
/// Every content schema carries exactly one subject-valued canonical fact.
pub fn witnessed_subject_of(observation: &Observation) -> Option<String> {
    let fact_name = observation.schema().canonical_fact_name()?;
    match observation.evidence().fact(fact_name)?.value() {
        FactValue::Text(text) if !text.trim().is_empty() => Some(text.clone()),
        _ => None,
    }
}

/// Resolves a declared subject to the canonical Artifact it names (RFC-0011
/// §4): the Artifact established by the content Observations witnessing that
/// subject, when **exactly one** distinct Artifact was established.
///
/// A subject nothing ever witnessed resolves to nothing, and so does a subject
/// whose witnessing established several Artifacts — ambiguity is preserved,
/// never resolved by guessing. Stated once here because the live index, the
/// batch builder, and the desktop all have to apply the identical rule.
pub fn resolve_subject(
    witnessed: &HashMap<String, Vec<ArtifactId>>,
    subject: &str,
) -> Option<ArtifactId> {
    match witnessed.get(subject) {
        Some(artifacts) if artifacts.len() == 1 => artifacts.first().cloned(),
        _ => None,
    }
}

/// Resolves a declared Continuation Surface to canonical Artifacts (RFC-0013).
///
/// Each declared subject resolves through [`resolve_subject`]; a subject that
/// resolves to nothing is skipped rather than guessed at. The result is
/// deduplicated and returned in canonical ascending ArtifactId order, so the
/// surface does not depend on the order the person happened to name things in.
pub fn resolve_surface(
    witnessed: &HashMap<String, Vec<ArtifactId>>,
    subjects: &[String],
) -> Vec<ArtifactId> {
    let mut resolved: Vec<ArtifactId> = Vec::new();
    for subject in subjects {
        if let Some(artifact) = resolve_subject(witnessed, subject) {
            if !resolved.contains(&artifact) {
                resolved.push(artifact);
            }
        }
    }
    resolved.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    resolved
}

/// Derives the complete Workspace understanding of one canonical history.
///
/// Pure: it reads no clock, writes nothing, and depends on no state beyond the
/// evidence handed to it. The same evidence always derives the same
/// understanding, which is what makes the live path and the replay path the
/// same function.
pub fn derive(evidence: Evidence, params: EngagementParams) -> Understanding {
    let Evidence {
        acts,
        declarations,
        witnessed,
        designated,
        surface,
        observed_states,
    } = evidence;

    let reconstruction = Reconstruction::build(acts, &declarations, params);
    let retrieval = RetrievalIndex::from_reconstruction(&reconstruction);
    let artifacts = subject_artifacts(&reconstruction, &witnessed);
    let projected = project_engagements(&reconstruction, &artifacts);

    // What each body of work is about, taken from the layer that measured what
    // its members have in common. Derived here rather than in the projection
    // because a Workspace carries no name (W-3): naming is a reading of the
    // evidence, and it is re-read every time the evidence grows.
    let titles: HashMap<String, String> = projected
        .iter()
        .map(|(engagement, workspace)| {
            (workspace.id().to_string(), engagement.title().to_string())
        })
        .collect();
    // How much Evo will claim of each body of work, taken from the layer that
    // measured it — and, until now, discarded here. Keyed by Workspace identity
    // beside the title, and for the same reason (W-3): standing is a reading of
    // the evidence, not a fact of the Workspace, so it is re-derived rather than
    // stored. A `Remembered` grouping is still projected and still a Workspace;
    // its standing is what tells Home to keep it findable rather than present it
    // as work.
    let standings: HashMap<String, Standing> = projected
        .iter()
        .map(|(engagement, workspace)| (workspace.id().to_string(), engagement.standing()))
        .collect();
    let workspaces: Vec<Workspace> = projected
        .iter()
        .map(|(_, workspace)| workspace.clone())
        .collect();

    let mut outcomes = HashMap::new();
    for (engagement, workspace) in &projected {
        // The *last* Snapshot is the most recent sitting of this body of work,
        // which is exactly what "resume from where you left off" means. Under
        // the old model the last Snapshot merely recorded the last time
        // formation happened to run.
        let Some(snapshot) = workspace.snapshots().last() else {
            continue;
        };
        let states = work_local_observed_states(
            engagement,
            reconstruction.identity(),
            &artifacts,
            &observed_states,
        );
        let Ok(input) = RestorationInput::new_with_observed_states(
            workspace,
            snapshot,
            designated.clone(),
            surface.clone(),
            states,
        ) else {
            // A Snapshot the projection built from this Workspace is always a
            // member of it; this arm cannot be reached without a projection
            // bug, and inventing an outcome would be worse than omitting one.
            continue;
        };
        let outcome = derive_restoration_plan(&input);
        outcomes.insert(outcome.workspace_id().to_string(), outcome);
    }

    Understanding {
        workspaces,
        outcomes,
        titles,
        standings,
        artifacts,
        retrieval,
    }
}

/// Resolves occurrence-local state for each participant at the exact timestamp
/// that participant was last witnessed inside this body of work.
///
/// The last matching log occurrence wins on an equal timestamp, including when
/// its state is absent. That makes append-order replay deterministic and, more
/// importantly, prevents a previous stateful occurrence from being silently
/// substituted for a newer state-less one.
fn work_local_observed_states(
    engagement: &evo_engagement::Engagement,
    identity: &evo_engagement::IdentityIndex,
    artifacts: &SubjectArtifacts,
    occurrences: &[ObservedStateOccurrence],
) -> Vec<(ArtifactId, ObservedState)> {
    let mut states = Vec::new();
    for participant in engagement.participants() {
        let (Some(observed_at), Some(artifact)) = (
            participant.last_seen(),
            artifacts.get(participant.subject()),
        ) else {
            continue;
        };
        let state = occurrences
            .iter()
            .filter(|occurrence| {
                occurrence.observed_at == observed_at
                    && identity.canonical(&occurrence.subject) == participant.subject()
            })
            .next_back()
            .and_then(|occurrence| occurrence.state.clone());
        if let Some(state) = state {
            if !states.iter().any(|(existing, _)| existing == artifact) {
                states.push((artifact.clone(), state));
            }
        }
    }
    states
}

/// Resolves each canonical resource to the Artifact that carries its history.
///
/// # Why this needs a rule at all
///
/// Resource identity is *learned from the corpus*: several witnessed names can
/// turn out to be the same thing (a document seen with and without an unsaved
/// marker, a window seen with and without a line number). Each of those names
/// established its own Artifact, so a canonical resource can have several
/// Artifacts competing to represent it, and something has to choose.
///
/// Choosing by iteration order over a `HashMap` would make Home depend on hash
/// seeds, which would break replay-equivalence outright. So the rule is
/// deterministic and stated in one place:
///
/// 1. Only a witnessed subject that established **exactly one** distinct
///    Artifact is eligible. A subject whose witnessing established several is
///    ambiguous, and ambiguity is preserved rather than resolved by guessing
///    (RFC-0011 §4).
/// 2. Among the eligible names of one canonical resource, the **founding
///    sighting** wins — the exact string witnessed the first time the resource
///    was ever seen. It is an immutable fact of an append-only log, so it does
///    not move as the corpus grows.
/// 3. Failing that (the founding sighting came from a declaration, which
///    establishes no Artifact), the lowest name in sort order wins.
fn subject_artifacts(
    reconstruction: &Reconstruction,
    witnessed: &HashMap<String, Vec<ArtifactId>>,
) -> SubjectArtifacts {
    // Sorted so the grouping below never depends on HashMap iteration order.
    let mut names: Vec<&String> = witnessed.keys().collect();
    names.sort_unstable();

    let mut candidates: BTreeMap<&str, Vec<(&str, &ArtifactId)>> = BTreeMap::new();
    for name in names {
        // Rule 1: exactly one Artifact, or no claim at all.
        let [artifact] = witnessed[name].as_slice() else {
            continue;
        };
        let canonical = reconstruction.identity().canonical(name);
        candidates
            .entry(canonical)
            .or_default()
            .push((name.as_str(), artifact));
    }

    let mut artifacts = SubjectArtifacts::new();
    for (canonical, eligible) in candidates {
        let founding = reconstruction.identity().founding_name(canonical);
        // Rule 2, then rule 3 — `eligible` is already in sorted name order.
        let chosen = eligible
            .iter()
            .find(|(name, _)| Some(*name) == founding)
            .or_else(|| eligible.first());
        if let Some((_, artifact)) = chosen {
            artifacts.insert(canonical.to_string(), (*artifact).clone());
        }
    }

    artifacts
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use evo_observation::observation_id::ObservationId;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::provenance::{ObservationSource, Provenance};

    use std::time::{Duration, UNIX_EPOCH};

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn observe(concept: ObservationConcept, secs: u64) -> Observation {
        let source = ObservationSource::new("understanding-test").expect("a valid source");
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

    fn focus_with_state(subject: &str, secs: u64, state: ObservedState) -> Observation {
        let mut observation = focus(subject, secs);
        let mut context = HashMap::new();
        state.write_context(&mut context);
        observation = Observation::new(
            observation.id().clone(),
            observation.schema().clone(),
            Provenance::new(
                observation.provenance().source().clone(),
                observation.provenance().observed_at(),
                context,
            ),
            observation.evidence().clone(),
        );
        observation
    }

    fn saved(subject: &str, secs: u64) -> Observation {
        observe(
            ObservationConcept::FileSaved {
                subject: subject.to_string(),
            },
            secs,
        )
    }

    fn derive_from(observations: &[Observation]) -> Understanding {
        derive(evidence_of(observations), EngagementParams::default())
    }

    #[test]
    fn occurrence_state_is_persisted_and_does_not_leak_across_subjects() {
        let mut first = focus("X editor", 0);
        let mut first_context = HashMap::new();
        ObservedState::new()
            .with_document_locator("/work/x.ts")
            .with_insertion_line(37)
            .write_context(&mut first_context);
        first = Observation::new(
            first.id().clone(),
            first.schema().clone(),
            Provenance::new(
                first.provenance().source().clone(),
                first.provenance().observed_at(),
                first_context,
            ),
            first.evidence().clone(),
        );
        let observations = vec![first, focus("Y editor", 60), focus("X editor", 120)];
        let evidence = evidence_of(&observations);
        assert_eq!(evidence.observed_states.len(), 3);
        assert_eq!(evidence.observed_states[0].state.as_ref().and_then(ObservedState::insertion_line), Some(37));
        assert_eq!(evidence.observed_states[1].state, None);
        assert_eq!(evidence.observed_states[2].state, None);
    }

    #[test]
    fn selected_work_local_stopping_occurrence_reaches_the_resume_point() {
        let mut observations = Vec::new();
        let mut moment = 0u64;
        for sitting in 0..3 {
            for _ in 0..4 {
                observations.push(focus("Automation X editor", moment));
                moment += 180;
                observations.push(focus("Automation X documentation", moment));
                moment += 180;
            }
            if sitting < 2 {
                moment += 3 * 60 * 60;
            }
        }
        observations.push(focus_with_state(
            "Automation X editor",
            moment,
            ObservedState::new()
                .with_document_locator("/work/automation-x.ts")
                .with_insertion_line(77),
        ));

        let understanding = derive_from(&observations);
        let states: Vec<_> = understanding
            .outcomes()
            .values()
            .filter_map(DerivationOutcome::resume_point)
            .filter_map(|point| point.observed_state())
            .collect();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].document_locator(), Some("/work/automation-x.ts"));
        assert_eq!(states[0].insertion_line(), Some(77));

        // A newer state-less occurrence becomes the stopping occurrence. Evo
        // must not reuse line 77 simply because it was the last known value.
        observations.push(focus("Automation X editor", moment + 60));
        let later = derive_from(&observations);
        assert!(later
            .outcomes()
            .values()
            .filter_map(DerivationOutcome::resume_point)
            .all(|point| point.observed_state().is_none()));
    }

    /// The failure this module exists to fix, restated for the standing model. A
    /// resource focused repeatedly and used with nothing else is not a body of
    /// work, however many times it was witnessed — which is what the old
    /// twice-witnessed rule counted. It is still *kept*: projected as a Workspace
    /// held as [`Standing::Remembered`], findable by name and off Home. Refusing
    /// to call something work is not the same as throwing it away, and the second
    /// is the silent loss this model ends.
    #[test]
    fn a_resource_used_with_nothing_else_is_remembered_not_work() {
        let mut observations = Vec::new();
        let mut moment = 0;
        for _ in 0..12 {
            observations.push(focus("Some Player — Now Playing", moment));
            moment += 90;
        }
        let understanding = derive_from(&observations);
        // Kept, not dropped: the lone resource is still a Workspace.
        assert!(
            !understanding.workspaces().is_empty(),
            "a lone resource is remembered, not thrown away — silent loss is the failure"
        );
        // But every standing it carries says it is not work: Home never presents
        // it, and nothing auto-opens it.
        assert!(
            !understanding.standings().is_empty()
                && understanding
                    .standings()
                    .values()
                    .all(|standing| !standing.is_work()),
            "repeated focus alone is attention Evo keeps, not work it presents"
        );
    }

    /// The other half of the same claim: heterogeneous resources used together,
    /// returned to, and worked in deeply do converge into one body of work.
    #[test]
    fn resources_used_together_across_sittings_converge_into_one_workspace() {
        let mut observations = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                observations.push(focus("Internship Assignment — Editor", moment));
                moment += 240;
                observations.push(saved("/Users/x/internship/assignment.odt", moment));
                moment += 30;
                observations.push(focus("internship assignment research — Browser", moment));
                moment += 240;
            }
            moment += 3 * 60 * 60;
        }

        let understanding = derive_from(&observations);
        assert_eq!(
            understanding.workspaces().len(),
            1,
            "one coherent task is one Workspace"
        );
        let workspace = &understanding.workspaces()[0];
        assert_eq!(workspace.attachments().len(), 3);
        assert!(
            understanding
                .outcomes()
                .contains_key(&workspace.id().to_string()),
            "every projected Workspace gets a derived Restoration outcome"
        );
    }

    /// §15/§26: every body of work Home can show carries a derived name, that
    /// name is keyed by Workspace identity, and it is something Evo actually
    /// witnessed rather than wording Evo composed.
    ///
    /// This is the wiring claim, not the choosing claim. Which member's name wins
    /// is decided and tested in the Engagement layer
    /// (`the_work_is_titled_for_what_its_members_share`); what is asserted here is
    /// that the choice survives projection — the step that used to discard it,
    /// leaving Home to name a body of work after whichever window attention
    /// happened to end in.
    #[test]
    fn every_projected_body_of_work_carries_a_witnessed_derived_name() {
        let mut observations = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                observations.push(focus("Internship Assignment — Editor", moment));
                moment += 240;
                observations.push(saved("/Users/x/internship/assignment.odt", moment));
                moment += 30;
                observations.push(focus("internship assignment research — Browser", moment));
                moment += 240;
            }
            moment += 3 * 60 * 60;
        }

        let evidence = evidence_of(&observations);
        let witnessed: Vec<String> = evidence.witnessed.keys().cloned().collect();
        let understanding = derive(evidence, EngagementParams::default());

        assert!(!understanding.workspaces().is_empty());
        assert_eq!(
            understanding.titles().len(),
            understanding.workspaces().len(),
            "one name per body of work, no orphans in either direction"
        );

        for workspace in understanding.workspaces() {
            let title = understanding
                .titles()
                .get(&workspace.id().to_string())
                .expect("a body of work Home can show is never nameless");
            assert!(
                !title.trim().is_empty(),
                "an empty name claims nothing and must not be presented as a name"
            );
            // A displayed name is a slice of a witnessed subject — a basename, an
            // address without its machinery, or the subject itself. Nothing is
            // assembled, so the name can always be traced back to evidence (§14).
            assert!(
                witnessed.iter().any(|subject| subject.contains(title.as_str())),
                "the name {title:?} must be traceable to a witnessed subject"
            );
        }

        // Re-derivation is the replay path (§19): the same evidence names the same
        // work the same way, so a name is stable across restarts.
        let again = derive_from(&observations);
        assert_eq!(understanding.titles(), again.titles());
    }

    /// Replay-equivalence by construction (§19): the same evidence derives the
    /// same understanding, including Workspace identity.
    #[test]
    fn the_same_evidence_always_derives_the_same_understanding() {
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

        let first = derive_from(&observations);
        let second = derive_from(&observations);
        assert_eq!(first.workspaces(), second.workspaces());
        assert_eq!(first.outcomes(), second.outcomes());
    }

    /// Restoration selectivity (§12): a projected Workspace does not open
    /// everything it remembers.
    #[test]
    fn a_projected_workspace_opens_less_than_it_remembers() {
        let mut observations = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                observations.push(focus("Thesis Chapter Three — Editor", moment));
                moment += 300;
                observations.push(saved("/Users/x/thesis/chapter-three.tex", moment));
                moment += 20;
                observations.push(focus("thesis chapter three sources — Browser", moment));
                moment += 60;
                observations.push(focus("thesis chapter three figures — Viewer", moment));
                moment += 60;
            }
            moment += 4 * 60 * 60;
        }

        let understanding = derive_from(&observations);
        assert_eq!(understanding.workspaces().len(), 1);
        let workspace = &understanding.workspaces()[0];
        let opens = workspace
            .attachments()
            .iter()
            .filter(|attachment| attachment.opens_on_restore())
            .count();
        assert!(opens >= 1, "something must open, or restoring does nothing");
        assert!(
            opens < workspace.attachments().len(),
            "restoring must not throw every remembered resource at the person"
        );
    }

    /// The determinism rule this module states: the Artifact chosen for a
    /// canonical resource never depends on `HashMap` iteration order. Two
    /// witnessed names that fold to one resource must resolve to one Artifact,
    /// and to the same one every time.
    #[test]
    fn folded_names_resolve_to_one_stable_artifact() {
        // "Report — Editor" and "Report — Editor — Edited" fold to one resource
        // once the corpus shows the marker attached to only one of several
        // states of the same stem.
        let mut observations = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for pass in 0..4 {
                let name = if pass % 2 == 0 {
                    "Report — Editor"
                } else {
                    "Report — Editor — Edited"
                };
                observations.push(focus(name, moment));
                moment += 240;
                observations.push(saved("/Users/x/reports/report.odt", moment));
                moment += 30;
                observations.push(focus("report figures — Viewer", moment));
                moment += 240;
            }
            moment += 3 * 60 * 60;
        }

        let evidence = evidence_of(&observations);
        let reconstruction = Reconstruction::build(
            evidence.acts.clone(),
            &evidence.declarations,
            EngagementParams::default(),
        );
        let artifacts = subject_artifacts(&reconstruction, &evidence.witnessed);

        // Recomputing over the same evidence gives the identical mapping,
        // whatever order the HashMap happens to iterate in.
        for _ in 0..8 {
            let again = subject_artifacts(&reconstruction, &evidence.witnessed);
            assert_eq!(artifacts, again);
        }

        // Every canonical name the projection will look up is present exactly
        // once, and no witnessed alias leaks in beside its canonical form.
        for canonical in artifacts.keys() {
            assert_eq!(
                reconstruction.identity().canonical(canonical),
                canonical.as_str(),
                "the map is keyed by canonical names only"
            );
        }
    }

    /// Ambiguity is preserved, never guessed: a witnessed subject that
    /// established more than one distinct Artifact resolves to none.
    #[test]
    fn an_ambiguous_subject_resolves_to_no_artifact() {
        let observations = vec![focus("Shared Name", 0), saved("/tmp/other", 30)];
        let mut evidence = evidence_of(&observations);
        // Force the ambiguity the real log produces when one subject was
        // witnessed under two different schemas.
        let extra = ArtifactId::new("a-second-artifact-for-the-same-subject").unwrap();
        evidence
            .witnessed
            .get_mut("Shared Name")
            .expect("witnessed")
            .push(extra);

        let reconstruction = Reconstruction::build(
            evidence.acts.clone(),
            &evidence.declarations,
            EngagementParams::default(),
        );
        let artifacts = subject_artifacts(&reconstruction, &evidence.witnessed);
        assert!(
            !artifacts.contains_key("Shared Name"),
            "two candidate Artifacts is ambiguity, and ambiguity is preserved"
        );
    }

    #[test]
    fn an_empty_history_claims_nothing() {
        let understanding = derive(Evidence::default(), EngagementParams::default());
        assert!(understanding.workspaces().is_empty());
        assert!(understanding.outcomes().is_empty());
        assert!(understanding.titles().is_empty());
        assert!(understanding.standings().is_empty());
    }

    /// Standing reaches Home, keyed by Workspace identity, for every body of
    /// work — the step this module used to drop.
    ///
    /// The Engagement layer decides whether a grouping is a body of work or is
    /// only [`Standing::Remembered`]; that decision is worthless if the
    /// projection discards it and Home shows every thread as work regardless
    /// (which is how a login screen, a music player and an inbox were presented
    /// as bodies of work). This pins the wiring: one standing per Workspace, no
    /// orphans in either direction — nothing is dropped — and a genuinely
    /// coherent, returned-to task is carried as work rather than demoted.
    #[test]
    fn every_projected_body_of_work_carries_its_standing() {
        let mut observations = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                observations.push(focus("Internship Assignment — Editor", moment));
                moment += 240;
                observations.push(saved("/Users/x/internship/assignment.odt", moment));
                moment += 30;
                observations.push(focus("internship assignment research — Browser", moment));
                moment += 240;
            }
            moment += 3 * 60 * 60;
        }

        let understanding = derive_from(&observations);
        assert!(!understanding.workspaces().is_empty());
        assert_eq!(
            understanding.standings().len(),
            understanding.workspaces().len(),
            "one standing per body of work, no orphans in either direction"
        );
        for workspace in understanding.workspaces() {
            let standing = understanding
                .standings()
                .get(&workspace.id().to_string())
                .copied()
                .expect("a body of work Home can show always carries a standing");
            // This fixture is a coherent task worked across sittings and
            // returned to: it is work, not a hub-with-spokes held as Remembered.
            assert!(
                standing.is_work(),
                "a coherent returned-to task is presented as work, not merely remembered"
            );
        }

        // Re-derivation is the replay path (§19): the same evidence assigns the
        // same standing, so what Home presents as work is stable across restarts.
        let again = derive_from(&observations);
        assert_eq!(understanding.standings(), again.standings());
    }
}
