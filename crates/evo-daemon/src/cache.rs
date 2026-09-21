//! Derived canonical index — a rebuildable read cache over the Observation log.
//!
//! `CanonicalIndex` holds every derived view the desktop reads: Artifact
//! subjects and locators, the current WorkDesignated designation and its
//! resolved Artifact (RFC-0011), the current Continuation Surface (RFC-0013),
//! the bodies of work worth returning to, and the Restoration outcome derived
//! for each of them.
//!
//! # Why this exists
//!
//! Evo's Observation log is append-only and long-lived (TRACE-0001 describes
//! millions of Observations). A consumer that re-parses the whole log on every
//! 2-second desktop reload does O(N) work per reload — quadratic over a day of
//! use. This index reduces each *read* to the records appended since the last
//! one, so the repeated parsing cost is proportional to new evidence.
//!
//! # What changed, and why
//!
//! **The old rule.** The index tailed three logs — `observation.log`,
//! `workspace.log` and `restoration.log` — and treated the latter two as the
//! record of what Evo understood. Workspaces were reconstructed by replaying
//! stored delta records; Restoration outcomes were read back from disk.
//!
//! **Why it could not work.** Those stored records encode the model this
//! rewrite exists to remove: one Artifact witnessed twice became one Workspace.
//! Reading them back resurrects that model on every start, no matter what the
//! live path does. The formats cannot be salvaged either — an attachment record
//! carries no role, and the delta format can only *add* attachments and
//! snapshots, while the membership and roles of a projected body of work both
//! change as evidence accumulates.
//!
//! **The new rule.** Only `observation.log` is read. Understanding is *derived*
//! from it ([`crate::understanding`]): the index accumulates the Acts and
//! Declarations the log witnesses, and recomputes the Workspaces and their
//! Restoration outcomes from the whole accumulated history.
//!
//! # Recompute cadence
//!
//! Relatedness is a property of the corpus, so the derivation reads all of it —
//! measured at 473ms for 1245 Observations over 427 resources on the real
//! machine. That rules out recomputing per Observation. Instead
//! [`CanonicalIndex::ingest_observation`] only marks the understanding stale,
//! and [`CanonicalIndex::refresh`] recomputes at the end if anything new
//! arrived. Refresh is the desktop's existing ~2s reload, which is exactly the
//! right cadence: it is already the rate at which Home can visibly change.
//!
//! # Non-canonical, rebuildable, never authoritative
//!
//! The index is a cache, never canonical state:
//!
//! - it is derived exclusively from the append-only Observation log;
//! - it is fully rebuildable from scratch ([`CanonicalIndex::new`] performs a
//!   full replay);
//! - losing it loses nothing canonical (ARCHITECTURE §2, §6);
//! - a log that shrank below the tracked offset (the writer recovered a torn
//!   tail at startup) triggers a full rebuild from the beginning, so no record
//!   is ever missed.
//!
//! # Recovery semantics
//!
//! Tail reads use [`Storage::read_from`]: a torn trailing record stops the read
//! at the last complete record (and is retried on the next refresh), and
//! genuine mid-log corruption is reported as an error rather than silently
//! discarded.

use crate::errors::DaemonError;
use crate::persistence::{
    continuation_surface_subjects_of, decode_observation_record, designation_subject_of,
};
use crate::understanding::{self, Evidence, ObservedStateOccurrence, Understanding};
use evo_artifact::artifact_id::ArtifactId;
use evo_artifact::derive_artifact_id_from_observations;
use evo_engagement::{Act, Declarations, EngagementParams, Standing};
use evo_observation::evidence::FactValue;
use evo_observation::observation::Observation;
use evo_observation::observation_schema::ObservationSchema;
use evo_observation::observed_state::ObservedState;
use evo_restoration::DerivationOutcome;
use evo_storage::{Storage, StorageObjectKind};
use evo_workspace::workspace::Workspace;
use evo_engagement::WorkId;
use evo_retrieval::WorkResolution;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The derived read cache over the canonical Observation log.
#[derive(Debug)]
pub struct CanonicalIndex {
    root: PathBuf,
    /// Byte offset of the first not-yet-read Observation record.
    observation_offset: u64,
    /// The number of Observations ingested, in append order. Used only as the
    /// deterministic append-order tie-break for declaration currency
    /// (RFC-0011 §5: where times are equal, the later append-order record is
    /// current).
    observation_count: usize,
    /// Artifact identity → the subject its content Observations witnessed.
    subjects: HashMap<String, String>,
    /// Artifact identity → (frozen schema name, witnessed subject).
    locators: HashMap<String, (String, String)>,
    /// Witnessed subject → the distinct canonical Artifacts established by
    /// the content Observations witnessing it. Nothing here invents identity:
    /// resolution looks this up (RFC-0011 §4).
    subject_to_artifacts: HashMap<String, Vec<ArtifactId>>,
    /// State (or explicit absence) carried by each content occurrence, in
    /// canonical append order. Rebuildable from the Observation log.
    observed_states: Vec<ObservedStateOccurrence>,
    /// The current WorkDesignated designation: (observed time, append-order
    /// index, subject).
    designation: Option<(SystemTime, usize, String)>,
    /// The current Continuation Surface (RFC-0013): (observed time,
    /// append-order index, canonically ordered subjects) of the latest valid
    /// OBS-CONTINUATION-SURFACE declaration. Supersession mirrors the
    /// designation rule: the latest valid declaration by canonical
    /// Observation Time is current.
    continuation_surface: Option<(SystemTime, usize, Vec<String>)>,
    /// Witnessed acts, in canonical append order — one of the two inputs to
    /// the reconstruction. Accumulated while tailing so that recomputing the
    /// understanding never re-parses the log.
    acts: Vec<Act>,
    /// Everything the person stated, plus witnessed structural containment —
    /// the other reconstruction input.
    declarations: Declarations,
    /// The limits the reconstruction is computed under.
    params: EngagementParams,
    /// The derived understanding: the bodies of work and their Restoration
    /// outcomes. Recomputed from `acts`/`declarations`, never read from disk.
    understanding: Understanding,
    /// Whether evidence has arrived since the understanding was last derived.
    stale: bool,
    /// When set, `reconstruct()` is suppressed until this time. This prevents
    /// execution-originated observations (opening a URL, raising a window)
    /// from immediately re-deriving workspace formation, which could shift
    /// artifact assignments between workspaces.
    suppress_until: Option<SystemTime>,
}

impl CanonicalIndex {
    /// Builds the index from canonical state by full replay.
    ///
    /// This is the rebuild path: it reads the Observation log from the
    /// beginning and derives the complete state. Any index state is
    /// reproducible from this.
    ///
    /// # Errors
    ///
    /// Returns a [`DaemonError`] when the log is genuinely corrupted.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, DaemonError> {
        let mut index = Self {
            root: root.into(),
            observation_offset: 0,
            observation_count: 0,
            subjects: HashMap::new(),
            locators: HashMap::new(),
            subject_to_artifacts: HashMap::new(),
            observed_states: Vec::new(),
            designation: None,
            continuation_surface: None,
            acts: Vec::new(),
            declarations: Declarations::new(),
            params: EngagementParams::default(),
            understanding: Understanding::default(),
            stale: false,
            suppress_until: None,
        };
        index.refresh()?;
        Ok(index)
    }

    /// Constructs the index WITHOUT deriving the legacy engagement
    /// understanding.
    ///
    /// For the live daemon: with a growing log the derivation takes
    /// minutes at startup and would delay capture the whole while.
    /// Understanding is the ledger engine's job (its own process); the
    /// daemon witnesses, persists, and serves declarations. The index is
    /// left stale — a declaration settle or an explicit reconstruct
    /// derives when needed.
    pub fn new_capture_only(root: impl Into<PathBuf>) -> Result<Self, DaemonError> {
        let mut index = Self {
            root: root.into(),
            observation_offset: 0,
            observation_count: 0,
            subjects: HashMap::new(),
            locators: HashMap::new(),
            subject_to_artifacts: HashMap::new(),
            observed_states: Vec::new(),
            designation: None,
            continuation_surface: None,
            acts: Vec::new(),
            declarations: Declarations::new(),
            params: EngagementParams::default(),
            understanding: Understanding::default(),
            stale: false,
            suppress_until: None,
        };
        index.refresh_capture_only()?;
        Ok(index)
    }

    /// Tails the Observation log WITHOUT recomputing understanding.
    /// Used at startup: the daemon must begin capturing immediately, not
    /// minutes from now. The index remains stale until something settles
    /// it explicitly.
    pub fn refresh_capture_only(&mut self) -> Result<(), DaemonError> {
        if self.any_log_shrank() {
            self.reset();
        }
        self.tail_observations()?;
        Ok(())
    }

    /// Tails the Observation log and, if any evidence arrived, recomputes the
    /// derived understanding.
    ///
    /// # Errors
    ///
    /// Returns a [`DaemonError`] when the log is genuinely corrupted.
    pub fn refresh(&mut self) -> Result<(), DaemonError> {
        if self.any_log_shrank() {
            // The writer recovered a torn tail, so the tracked offset no longer
            // points at a record boundary. Re-derive from the beginning rather
            // than risk a skipped or duplicated record.
            self.reset();
        }
        self.tail_observations()?;
        self.reconstruct();
        Ok(())
    }

    /// Suppresses reconstruction for the given duration. After execution opens
    /// native resources, the resulting observations should not immediately
    /// re-derive workspace formation — doing so can shift artifact assignments
    /// between workspaces.
    pub fn suppress_reconstruction(&mut self, duration: std::time::Duration) {
        self.suppress_until = Some(std::time::SystemTime::now() + duration);
    }

    /// Recomputes the derived understanding when evidence has arrived since it
    /// was last derived, and reports whether it recomputed.
    ///
    /// Exposed so the daemon can bring the understanding up to date once a burst
    /// of capture has settled, rather than after every single record. A no-op
    /// returning `false` when nothing changed, so calling it is always safe.
    pub fn reconstruct(&mut self) -> bool {
        if !self.stale {
            return false;
        }
        // Suppress reconstruction during the post-execution window to prevent
        // execution-originated observations from shifting artifact assignments
        // between workspaces.
        if let Some(until) = self.suppress_until {
            if std::time::SystemTime::now() < until {
                return false;
            }
            self.suppress_until = None;
        }
        let evidence = Evidence {
            acts: self.acts.clone(),
            declarations: self.declarations.clone(),
            witnessed: self.subject_to_artifacts.clone(),
            designated: self.current_designated_artifact(),
            surface: Some(self.current_continuation_surface_artifacts())
                .filter(|artifacts| !artifacts.is_empty()),
            observed_states: self.observed_states.clone(),
        };
        self.understanding = understanding::derive(evidence, self.params);
        self.stale = false;
        true
    }

    /// The bodies of work worth returning to, most recently active first.
    pub fn workspaces(&self) -> &[Workspace] {
        self.understanding.workspaces()
    }

    /// Artifact identity → the subject its content Observations witnessed.
    pub fn subjects(&self) -> &HashMap<String, String> {
        &self.subjects
    }

    /// Artifact identity → (frozen schema name, witnessed subject).
    pub fn locators(&self) -> &HashMap<String, (String, String)> {
        &self.locators
    }

    /// The derived Restoration outcomes, keyed by Workspace identity.
    pub fn outcomes(&self) -> &HashMap<String, DerivationOutcome> {
        self.understanding.outcomes()
    }

    /// What each body of work is about, keyed by Workspace identity.
    ///
    /// Derived alongside the outcomes and never persisted — see
    /// [`crate::understanding::Understanding::titles`] for why Home takes its
    /// name from here rather than from a resource inside the work.
    pub fn titles(&self) -> &HashMap<String, String> {
        self.understanding.titles()
    }

    /// How much of each body of work Evo is prepared to claim, keyed by
    /// Workspace identity.
    ///
    /// Derived alongside the outcomes and titles and never persisted. Home reads
    /// it to present only actual bodies of work
    /// ([`evo_engagement::Standing::is_work`]) as work, while a grouping held
    /// back as [`evo_engagement::Standing::Remembered`] — a login screen, a
    /// music player, an inbox — stays findable rather than shown as work. See
    /// [`crate::understanding::Understanding::standings`] for why the decision
    /// comes from the Engagement layer rather than being remade here.
    pub fn standings(&self) -> &HashMap<String, Standing> {
        self.understanding.standings()
    }

    /// Resolves a natural-language reference against all remembered work,
    /// including bodies held as `Remembered` and therefore hidden at rest on
    /// Home. Ambiguity is returned explicitly; callers must not guess.
    pub fn resolve_work(&self, reference: &str) -> WorkResolution {
        self.understanding.resolve_work(reference)
    }

    /// Finds the projected Workspace carrying one stable work identity.
    pub fn workspace_for_work_id(&self, work_id: WorkId) -> Option<&Workspace> {
        self.workspaces().iter().find(|workspace| {
            workspace.id().to_string() == evo_workspace::WorkspaceId::from_work_id(work_id.as_u128()).to_string()
        })
    }

    /// The current WorkDesignated designation — the subject the user marked
    /// as the work to continue and the moment it was witnessed (RFC-0011).
    pub fn designation(&self) -> Option<(String, SystemTime)> {
        self.designation
            .as_ref()
            .map(|(time, _, subject)| (subject.clone(), *time))
    }

    /// Resolves the canonical Artifact a witnessed subject designates
    /// (RFC-0011 §4). The rule itself lives in
    /// [`understanding::resolve_subject`], so the live index and a replay
    /// cannot resolve a subject differently.
    pub fn resolve_designated_artifact(&self, subject: &str) -> Option<ArtifactId> {
        understanding::resolve_subject(&self.subject_to_artifacts, subject)
    }

    /// The canonical designated Artifact of the current WorkDesignated
    /// designation, when both exist.
    pub fn current_designated_artifact(&self) -> Option<ArtifactId> {
        let subject = self
            .designation
            .as_ref()
            .map(|(_, _, subject)| subject.as_str())?;
        self.resolve_designated_artifact(subject)
    }

    /// The current Continuation Surface (RFC-0013): the canonically ordered
    /// subjects of the latest valid OBS-CONTINUATION-SURFACE declaration.
    pub fn current_continuation_surface(&self) -> Option<Vec<String>> {
        self.continuation_surface
            .as_ref()
            .map(|(_, _, subjects)| subjects.clone())
    }

    /// Resolves the Current Continuation Surface to canonical Artifacts
    /// (RFC-0013). Shares the resolution rule with a replay through
    /// [`understanding::resolve_surface`].
    pub fn current_continuation_surface_artifacts(&self) -> Vec<ArtifactId> {
        match &self.continuation_surface {
            Some((_, _, subjects)) => {
                understanding::resolve_surface(&self.subject_to_artifacts, subjects)
            }
            None => Vec::new(),
        }
    }

    /// Ingests one freshly persisted Observation into the derived views
    /// without re-reading the log.
    ///
    /// This is the daemon-side update path: the daemon is the only writer, so
    /// the Observation it just persisted is exactly the next record in append
    /// order. The state produced here is identical to what a full replay
    /// derives (proven by the equivalence tests).
    ///
    /// The understanding is *not* recomputed here — see the module note on
    /// recompute cadence. Call [`CanonicalIndex::reconstruct`] or
    /// [`CanonicalIndex::refresh`] to bring it up to date.
    pub fn ingest_observation(&mut self, observation: Observation) {
        self.observation_count += 1;
        self.absorb(&observation);

        // Reference-only schemas (RFC-0011 WorkDesignated; RFC-0012
        // RepositoryMembership and WorkGrouped; RFC-0013 ContinuationSurface)
        // reference existing canonical Artifacts; they never establish one, so
        // they never enter the subject/locator maps.
        if observation.schema().is_reference_only() {
            if observation.schema() == &ObservationSchema::work_designated_v1() {
                let Some(subject) = designation_subject_of(&observation) else {
                    return;
                };
                if self.supersedes(&self.designation, observation.provenance().observed_at()) {
                    self.designation = Some((
                        observation.provenance().observed_at(),
                        self.observation_count,
                        subject,
                    ));
                }
                return;
            }
            if observation.schema() == &ObservationSchema::continuation_surface_v1() {
                let Some(subjects) = continuation_surface_subjects_of(&observation) else {
                    return;
                };
                if self.supersedes(
                    &self.continuation_surface,
                    observation.provenance().observed_at(),
                ) {
                    self.continuation_surface = Some((
                        observation.provenance().observed_at(),
                        self.observation_count,
                        subjects,
                    ));
                }
            }
            return;
        }

        let Some(fact_name) = observation.schema().canonical_fact_name() else {
            return;
        };
        let subject = match observation.evidence().fact(fact_name) {
            Some(fact) => match fact.value() {
                FactValue::Text(text) if !text.trim().is_empty() => text.clone(),
                _ => return,
            },
            None => return,
        };
        self.observed_states.push(ObservedStateOccurrence {
            subject: subject.clone(),
            observed_at: observation.provenance().observed_at(),
            state: ObservedState::from_context(observation.provenance().context()),
        });
        let Ok(artifact_id) =
            derive_artifact_id_from_observations(std::slice::from_ref(&observation))
        else {
            return;
        };
        let artifact_key = artifact_id.to_string();
        self.subjects
            .entry(artifact_key.clone())
            .or_insert_with(|| subject.clone());
        self.locators
            .entry(artifact_key)
            .or_insert_with(|| (observation.schema().name().to_string(), subject.clone()));
        let artifacts = self.subject_to_artifacts.entry(subject).or_default();
        if !artifacts.contains(&artifact_id) {
            artifacts.push(artifact_id);
        }
    }

    /// Adds one Observation to the reconstruction inputs.
    ///
    /// Routing is delegated to [`evo_engagement::interpret`] on the single
    /// record rather than reimplemented, so the incremental path cannot drift
    /// from the batch path. Designations, groupings and containments accumulate
    /// and are applied in append order; the continuation surface supersedes on
    /// canonical Observation Time, which the record itself carries. Absorbing
    /// records one at a time is therefore equivalent to interpreting the whole
    /// log at once.
    fn absorb(&mut self, observation: &Observation) {
        let (acts, declarations) = evo_engagement::interpret(std::slice::from_ref(observation));
        self.acts.extend(acts);
        for subject in declarations.designations() {
            self.declarations.designate(subject.clone());
        }
        for (first, second) in declarations.groupings() {
            self.declarations.group(first.clone(), second.clone());
        }
        let continuations: Vec<String> = declarations.continuations().cloned().collect();
        if !continuations.is_empty() {
            self.declarations
                .continue_from(continuations, observation.provenance().observed_at());
        }
        for (member, container) in declarations.containments() {
            self.declarations.contain(member.clone(), container.clone());
        }
        self.stale = true;
    }

    /// Whether a declaration witnessed at `time` supersedes the currently held
    /// one: later canonical Observation Time wins, and on equal time the later
    /// append-order record wins (RFC-0011 §5).
    fn supersedes<T>(&self, current: &Option<(SystemTime, usize, T)>, time: SystemTime) -> bool {
        match current {
            None => true,
            Some((current_time, current_index, _)) => {
                time > *current_time
                    || (time == *current_time && self.observation_count > *current_index)
            }
        }
    }

    /// Whether the Observation log is shorter than the last-read offset (a torn
    /// tail was recovered by the writer), requiring a full rebuild.
    fn any_log_shrank(&self) -> bool {
        log_len(&self.root, "observation.log").is_some_and(|len| self.observation_offset > len)
    }

    /// Clears all derived state so a full rebuild starts from a clean slate.
    fn reset(&mut self) {
        self.observation_offset = 0;
        self.observation_count = 0;
        self.subjects.clear();
        self.locators.clear();
        self.subject_to_artifacts.clear();
        self.observed_states.clear();
        self.designation = None;
        self.continuation_surface = None;
        self.acts.clear();
        self.declarations = Declarations::new();
        self.understanding = Understanding::default();
        self.stale = true;
        self.suppress_until = None;
    }

    fn tail_observations(&mut self) -> Result<(), DaemonError> {
        let _guard = Storage::with_thread_root(self.root.clone());
        let storage = Storage::new();
        let (records, offset) =
            storage.read_from(StorageObjectKind::Observation, self.observation_offset)?;
        for record in records {
            let observation = decode_observation_record(&record)?;
            self.ingest_observation(observation);
        }
        self.observation_offset = offset;
        Ok(())
    }
}

/// The current byte length of one storage log, when it exists.
fn log_len(root: &Path, filename: &str) -> Option<u64> {
    std::fs::metadata(root.join(filename))
        .ok()
        .map(|metadata| metadata.len())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::persistence::{
        current_designated_artifact as full_current_designated_artifact,
        load_artifact_locators as full_load_artifact_locators,
        load_artifact_subjects as full_load_artifact_subjects,
        load_current_designation as full_load_current_designation, persist_observation,
    };
    use crate::workspace_replay::replay_workspaces_from_root;
    use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
    use evo_observation::provenance::ObservationSource;
    use evo_storage::Storage;

    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-cache-{label}-{nanos}"))
    }

    fn secs(n: u64) -> SystemTime {
        UNIX_EPOCH
            .checked_add(Duration::from_secs(n))
            .expect("a representable moment")
    }

    /// Drives one canonical signal through the real capture → acceptance →
    /// persistence path and returns the accepted Observation, exactly as the
    /// daemon does. Nothing about understanding is persisted, because
    /// understanding is derived.
    fn drive(signal: MacOSSignal) -> Observation {
        let source = ObservationSource::new("cache-test").expect("non-empty");
        let adapter = MacOSAdapter::new(source);
        let mut engine = CaptureEngine::new();
        let raw = adapter
            .normalize(signal)
            .expect("normalization is infallible")
            .expect("canonical signal");
        let schema = raw.schema();
        let observation = engine.ingest(raw, &schema).expect("observation acceptance");
        // Mirror the daemon pipeline: the accepted Observation is durably
        // persisted before any downstream stage reports success (IS-0001 R-7).
        persist_observation(&observation).expect("observation persists");
        observation
    }

    fn focus(subject: &str, at: u64) -> MacOSSignal {
        MacOSSignal::WindowFocusGained {
            subject: subject.to_string(),
            observed_at: secs(at),
            process_identifier: None,
            owning_process_name: None,
            observed_state: None,
        }
    }

    fn saved(subject: &str, at: u64) -> MacOSSignal {
        MacOSSignal::FileSaved {
            subject: subject.to_string(),
            observed_at: secs(at),
        }
    }

    /// One coherent body of work: heterogeneous resources used together,
    /// returned to across sittings, worked in deeply. Written into `root`
    /// through the real capture path; returns the last moment used.
    fn drive_one_body_of_work(sittings: usize) -> u64 {
        let mut moment = 0u64;
        for _sitting in 0..sittings {
            for _pass in 0..4 {
                drive(focus("Internship Assignment — Editor", moment));
                moment += 240;
                drive(saved("/Users/alice/internship/assignment.odt", moment));
                moment += 30;
                drive(focus("internship assignment research — Browser", moment));
                moment += 240;
            }
            moment += 3 * 60 * 60;
        }
        moment
    }

    /// The full-replay ground truth the index must match.
    fn assert_matches(root: &Path, index: &CanonicalIndex) {
        assert_eq!(
            index.subjects(),
            &full_load_artifact_subjects(root).expect("subjects")
        );
        assert_eq!(
            index.locators(),
            &full_load_artifact_locators(root).expect("locators")
        );
        assert_eq!(
            index.designation(),
            full_load_current_designation(root).expect("designation")
        );
        assert_eq!(
            index.current_designated_artifact(),
            full_current_designated_artifact(root).expect("designated")
        );
        // Replay-equivalence (§19): the understanding recomputed from the log
        // is the understanding the live index holds, because they are the same
        // function of the same input.
        assert_eq!(
            index.workspaces(),
            replay_workspaces_from_root(root)
                .expect("replay")
                .as_slice()
        );
    }

    #[test]
    fn index_matches_full_replay_after_build_and_incremental_refresh() {
        let root = unique_root("equivalence");
        let _guard = Storage::with_thread_root(root.clone());

        let mut moment = drive_one_body_of_work(3);

        // A full-build index matches the full-replay ground truth.
        let mut index = CanonicalIndex::new(&root).expect("index builds");
        assert_matches(&root, &index);
        assert_eq!(
            index.workspaces().len(),
            1,
            "one coherent task is one Workspace"
        );

        // Incremental refreshes with no new evidence change nothing.
        let before = index.workspaces().to_vec();
        index.refresh().expect("idle refresh succeeds");
        assert_eq!(index.workspaces(), before.as_slice());
        assert_matches(&root, &index);

        // New evidence appended after the index was built: a refresh tails only
        // the new records and still matches a full replay.
        moment += 3 * 60 * 60;
        for _pass in 0..4 {
            drive(focus("Internship Assignment — Editor", moment));
            moment += 240;
            drive(saved("/Users/alice/internship/assignment.odt", moment));
            moment += 30;
            drive(focus("internship assignment research — Browser", moment));
            moment += 240;
        }
        index.refresh().expect("incremental refresh succeeds");
        assert_matches(&root, &index);
    }

    /// The §0–1 property, asserted of the index rather than a unit under test:
    /// a resource focused over and over, used with nothing else, is **kept but
    /// held off Home** — it becomes a Remembered body, never work. Dropping it
    /// would be silent loss; presenting it would be an activity log.
    #[test]
    fn a_resource_used_with_nothing_else_is_remembered_not_work() {
        let root = unique_root("no-lone-workspace");
        let _guard = Storage::with_thread_root(root.clone());

        let mut moment = 0u64;
        for _ in 0..20 {
            drive(focus("Some Player — Now Playing", moment));
            moment += 90;
        }

        let index = CanonicalIndex::new(&root).expect("index builds");
        // The body is kept — the projection no longer discards what it merely
        // witnessed (NO SILENT LOSS), so it is findable by name.
        assert!(
            !index.workspaces().is_empty(),
            "a lone resource is remembered, not thrown away"
        );
        // But it is not work: every standing the index derived is below the
        // work threshold, so Home (which filters by standing) shows none of it.
        assert!(
            !index.standings().is_empty()
                && index.standings().values().all(|standing| !standing.is_work()),
            "repeated focus alone is attention, not work"
        );
        // The evidence is still fully recorded.
        assert!(
            !index.subjects().is_empty(),
            "the Observation evidence is preserved"
        );
    }

    #[test]
    fn index_resolves_designation_exactly_like_full_replay() {
        let root = unique_root("designation-equivalence");
        let _guard = Storage::with_thread_root(root.clone());

        // A content Observation establishes the Artifact; a designation
        // (RFC-0011) witnessed later references it. Resolution is
        // subject-based (RFC-0011 §4) and works whether or not the subject
        // turned out to be part of a body of work.
        drive(saved("/tmp/plan.md", 1));
        drive(MacOSSignal::WorkDesignated {
            subject: "/tmp/plan.md".into(),
            observed_at: secs(2),
        });

        let index = CanonicalIndex::new(&root).expect("index builds");
        assert_matches(&root, &index);

        let designated = index
            .current_designated_artifact()
            .expect("designated artifact");
        let (subject, _) = index.designation().expect("designation");
        assert_eq!(subject, "/tmp/plan.md");
        assert_eq!(
            index.resolve_designated_artifact("/tmp/plan.md"),
            Some(designated)
        );
        // A never-witnessed subject never resolves (no guessing).
        assert_eq!(index.resolve_designated_artifact("/tmp/other.md"), None);
    }

    /// The declared surface is ground truth (§10) and reaches the derivation:
    /// what the person said is where they continue must survive into the
    /// Restoration outcome.
    #[test]
    fn a_declared_continuation_surface_reaches_the_derived_outcome() {
        let root = unique_root("declared-surface");
        let _guard = Storage::with_thread_root(root.clone());

        let moment = drive_one_body_of_work(3);
        // A Continuation Surface names the resources the person continues from,
        // so it names at least two (RFC-0013 §4).
        drive(MacOSSignal::ContinuationSurface {
            subjects: vec![
                "/Users/alice/internship/assignment.odt".into(),
                "internship assignment research — Browser".into(),
            ],
            observed_at: secs(moment + 10),
        });

        let index = CanonicalIndex::new(&root).expect("index builds");
        let surface = index.current_continuation_surface_artifacts();
        assert_eq!(surface.len(), 2, "both declared subjects resolve");

        let workspace = index
            .workspaces()
            .first()
            .expect("the body of work is still there");
        let outcome = index
            .outcomes()
            .get(&workspace.id().to_string())
            .expect("an outcome was derived");
        let resume = outcome
            .resume_point()
            .expect("a declared surface establishes a Resume Point");
        assert!(
            surface.contains(resume.artifact_id()),
            "the person's statement outranks the inference: resuming from {} \
             which is not one of the declared {surface:?}",
            resume.artifact_id().as_str()
        );
    }

    /// Test K: restart. Identity and relationships survive because they are
    /// re-derived from the log, not read back from a store of conclusions.
    #[test]
    fn identity_and_relationships_survive_a_restart() {
        let root = unique_root("restart");
        let _guard = Storage::with_thread_root(root.clone());

        drive_one_body_of_work(3);

        let (before_ids, before_workspaces) = {
            let index = CanonicalIndex::new(&root).expect("index builds");
            assert_eq!(index.workspaces().len(), 1);
            (
                index
                    .workspaces()
                    .iter()
                    .map(|workspace| workspace.id().to_string())
                    .collect::<Vec<_>>(),
                index.workspaces().to_vec(),
            )
        }; // The index is dropped; the Observation log remains.

        let rebuilt = CanonicalIndex::new(&root).expect("index rebuilds");
        assert_eq!(
            rebuilt
                .workspaces()
                .iter()
                .map(|workspace| workspace.id().to_string())
                .collect::<Vec<_>>(),
            before_ids,
            "Workspace identity is stable across restart"
        );
        assert_eq!(rebuilt.workspaces(), before_workspaces.as_slice());
        assert_eq!(
            rebuilt.outcomes().len(),
            before_workspaces.len(),
            "every Workspace still has a derived outcome after restart"
        );
    }

    #[test]
    fn daemon_side_ingest_matches_full_replay() {
        let root = unique_root("daemon-ingest");
        let _guard = Storage::with_thread_root(root.clone());

        let mut index = CanonicalIndex::new(&root).expect("index builds");

        // Feed Observations the way the daemon pipeline does: persist, then
        // ingest in memory without re-reading the log.
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                index.ingest_observation(drive(focus("Report — Editor", moment)));
                moment += 240;
                index.ingest_observation(drive(saved("/Users/alice/report.md", moment)));
                moment += 30;
                index.ingest_observation(drive(focus("report sources — Browser", moment)));
                moment += 240;
            }
            moment += 3 * 60 * 60;
        }
        index.ingest_observation(drive(MacOSSignal::WorkDesignated {
            subject: "/Users/alice/report.md".into(),
            observed_at: secs(moment),
        }));
        index.reconstruct();

        // The daemon-side in-memory updates produce exactly what a full replay
        // derives from the persisted log.
        assert_matches(&root, &index);
        assert_eq!(index.workspaces().len(), 1);

        // And a freshly built index over the same log agrees, field for field.
        let rebuilt = CanonicalIndex::new(&root).expect("rebuild");
        assert_eq!(rebuilt.workspaces(), index.workspaces());
        assert_eq!(rebuilt.outcomes(), index.outcomes());
        assert_eq!(rebuilt.subjects(), index.subjects());
        assert_eq!(rebuilt.locators(), index.locators());
    }

    /// A torn tail recovered by the writer shortens the log below the tracked
    /// offset. The index must rebuild from the beginning rather than resume
    /// mid-record — and the rebuilt understanding must equal a fresh one.
    #[test]
    fn index_rebuilds_after_the_log_shrinks() {
        let root = unique_root("rebuild");
        let _guard = Storage::with_thread_root(root.clone());

        drive_one_body_of_work(3);

        let mut index = CanonicalIndex::new(&root).expect("index builds");
        assert_eq!(index.workspaces().len(), 1);

        // The index had read past what the log now contains.
        index.observation_offset = u64::MAX;
        index.refresh().expect("refresh rebuilds");

        assert_matches(&root, &index);
        let fresh = CanonicalIndex::new(&root).expect("fresh index");
        assert_eq!(index.workspaces(), fresh.workspaces());
        assert_eq!(index.outcomes(), fresh.outcomes());
        assert_eq!(
            index.observation_count,
            fresh.observation_count,
            "the rebuild replayed every record exactly once"
        );
    }

    /// Absorbing records one at a time must equal interpreting the whole log at
    /// once — otherwise the live path and the replay path are two models.
    #[test]
    fn incremental_absorption_equals_batch_interpretation() {
        let root = unique_root("absorb");
        let _guard = Storage::with_thread_root(root.clone());

        let moment = drive_one_body_of_work(2);
        drive(MacOSSignal::RepositoryMembership {
            member: "/Users/alice/internship/assignment.odt".into(),
            repository: "/Users/alice/internship".into(),
            observed_at: secs(moment + 5),
        });
        drive(MacOSSignal::WorkGrouped {
            first: "Internship Assignment — Editor".into(),
            second: "internship assignment research — Browser".into(),
            observed_at: secs(moment + 10),
        });
        drive(MacOSSignal::ContinuationSurface {
            subjects: vec![
                "Internship Assignment — Editor".into(),
                "internship assignment research — Browser".into(),
            ],
            observed_at: secs(moment + 15),
        });

        let index = CanonicalIndex::new(&root).expect("index builds");
        let observations =
            crate::persistence::load_persisted_observations(&root).expect("observations");
        let (acts, declarations) = evo_engagement::interpret(&observations);

        assert_eq!(index.acts.len(), acts.len());
        assert_eq!(
            index.declarations, declarations,
            "record-at-a-time absorption is the same as interpreting the log"
        );
    }
}
