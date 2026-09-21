//! Selective Restoration — the derived selection of what Evo should actually
//! restore for one Workspace.
//!
//! # The three distinct questions
//!
//! The pipeline must never collapse these (RFC-0010, RFC-0013):
//!
//! - **Workspace membership** — "what belongs to this body of work?"
//!   Durable, historical, answered by Workspace Formation.
//! - **Continuation Surface** — "what did the user declare their work
//!   currently continues across?" RFC-0013, the per-Workspace intersection
//!   of the current declaration.
//! - **Selective restoration** — "of the current continuation, what can
//!   actually be reopened right now?" This module.
//!
//! A Workspace may contain many related artifacts. The Continuation Surface
//! narrows that to what the user declared current. Selective restoration
//! further partitions the surface into:
//!
//! - **restore-worthy** — surface members that resolve to an executable
//!   target (a reopenable file, URL, or window);
//! - **unavailable** — surface members with no executable target (a commit
//!   is a repository object, not an OS-launchable resource) or whose
//!   resource identity cannot be resolved;
//! - **withheld** — Workspace members outside the current surface: durable
//!   membership, not opened. Split by [`Disposition`] into the ones Evo keeps
//!   to hand and the ones it keeps only as history — see below.
//!
//! # Three dispositions, not two
//!
//! Every member of a body of work gets exactly one of three answers to "what
//! happens to this when the person comes back", and each answer carries the
//! reason it was given:
//!
//! - [`Disposition::OpenNow`] — opened without being asked for. Its reason is
//!   tied to continuation: either this is the Resume Point, or it is another
//!   place the continuation happens.
//! - [`Disposition::AvailableIfNeeded`] — part of this work, deliberately not
//!   opened, and one gesture away. The person consulted it while working, or it
//!   is a place the work happens outside the sitting being returned to.
//! - [`Disposition::HistoryOnly`] — witnessed while the work was happening,
//!   with nothing beyond being present tying it to the work. Kept so the record
//!   stays complete and so the person can still find it, never offered.
//!
//! The second and third used to be one list, and collapsing them is what makes
//! restoration feel like a browser history: a reference document the person read
//! three times while writing was presented exactly like a window that happened
//! to be on screen. Nothing is *lost* by the distinction — every withheld member
//! is still listed with its reason, so nothing becomes unrecoverable — but the
//! two are different promises and Evo now makes them separately.
//!
//! # Purely derived, never canonical
//!
//! [`RestorationSelection`] is computed deterministically from canonical
//! inputs only — the derived plan outcome, the Workspace's canonical
//! membership, and canonical (schema, subject) resource evidence per
//! Artifact. It is rebuildable from the canonical Observation log, never
//! persisted as canonical state, and never feeds Workspace Formation,
//! Artifact identity, or continuation derivation.
//!
//! # Boundary: selection is not execution
//!
//! This module answers "what should Evo bring back", not "open everything".
//! RFC-0013 CS-4: the Continuation Surface never authorizes opening,
//! focusing, or launching any resource; execution remains a separate layer
//! (IS-0019 §11, IS-0021 §20) consuming the derived plan. No recency, no
//! frequency, no ranking, no application names, no guessing.

use evo_artifact::artifact_id::ArtifactId;
use evo_restoration::DerivationOutcome;
use evo_workspace::WorkspaceId;
use evo_workspace::attachment::ResourceRole;

use std::collections::HashMap;

use crate::resource::{ResourceIdentity, classify_resource_identity};

// ── Canonical disposition reasons ────────────────────────────────────────────
//
// Fixed sentences, not generated prose. Each states the witnessed ground for the
// disposition and nothing beyond it, and each is chosen from facts the selection
// can actually see: whether the member is the Resume Point, whether it is in the
// current continuation, and the part it played in the work.

/// Opened because it is where the work continues.
const OPEN_RESUME_POINT_REASON: &str = "This is where the work continues, so \
    this is what Evo puts you back into.";

/// Opened because the continuation happens here too.
const OPEN_SURFACE_MEMBER_REASON: &str = "The continuation of this work also \
    happens here, so it opens alongside the Resume Point.";

/// Withheld: a place the work happens, but not part of the current continuation.
const WITHHELD_PLACE_OF_WORK_REASON: &str = "This is one of the places this work \
    happens, but not part of the continuation you are returning to, so Evo keeps \
    it to hand instead of opening it.";

/// Withheld: consulted during the work, but not a place the work continues.
const WITHHELD_CONSULTED_REASON: &str = "You used this as part of this work but \
    the work does not continue here, so Evo keeps it to hand instead of opening \
    it.";

/// Withheld: present while the work happened, and nothing more.
const HISTORY_ONLY_REASON: &str = "Evo witnessed this while the work was \
    happening but nothing beyond being there ties it to the work, so it stays in \
    the record without being offered.";

// ── Disposition ──────────────────────────────────────────────────────────────

/// What Evo does with one member of a body of work when the person comes back.
///
/// Declared in order of how much of the person's attention the disposition
/// spends: opening something costs a window and a glance whether they wanted it
/// or not, offering something costs nothing until they reach for it, and keeping
/// something in the record costs nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Disposition {
    /// Opened without being asked for. Only for members of the current
    /// continuation, and every one carries a reason tied to continuation.
    OpenNow,
    /// Part of this work, not opened, one gesture away.
    AvailableIfNeeded,
    /// Kept in the record so it remains findable; never offered.
    HistoryOnly,
}

impl Disposition {
    /// Whether Evo opens this member without being asked.
    pub fn opens_unasked(self) -> bool {
        matches!(self, Disposition::OpenNow)
    }

    /// The canonical word for this disposition.
    ///
    /// Exists so a presentation layer cannot drift into a vocabulary of its own
    /// — the same reason [`crate::preflight::PreflightStatus::label`] exists.
    /// Three words, because there are three answers; a UI that shows two has
    /// collapsed a distinction this layer derived.
    pub fn label(self) -> &'static str {
        match self {
            Disposition::OpenNow => "OPEN NOW",
            Disposition::AvailableIfNeeded => "AVAILABLE IF NEEDED",
            Disposition::HistoryOnly => "HISTORY ONLY",
        }
    }
}

/// One Artifact selected for restoration, with its derived resource identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedResource {
    artifact_id: ArtifactId,
    identity: ResourceIdentity,
    /// Why this resource is opened without being asked for, tied to
    /// continuation. See [`SelectedResource::reason`].
    reason: String,
}

impl SelectedResource {
    /// Constructs a selected resource for one Artifact, with the reason it is
    /// opened without being asked for.
    pub fn new(
        artifact_id: ArtifactId,
        identity: ResourceIdentity,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            artifact_id,
            identity,
            reason: reason.into(),
        }
    }

    /// The canonical Artifact this selection refers to.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// The derived resource identity of the Artifact.
    pub fn identity(&self) -> &ResourceIdentity {
        &self.identity
    }

    /// Why this resource opens without being asked for.
    ///
    /// Always tied to continuation, because that is the only ground on which
    /// anything may be opened unasked: this is either the Resume Point or another
    /// place the continuation happens. Nothing is opened because it was recent,
    /// frequent, large, or of a familiar kind.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The executable target of this resource, when one exists (a commit
    /// has identity but no target — never guessed).
    pub fn executable_target(&self) -> Option<crate::locator::LocatorKind> {
        self.identity.executable_target()
    }
}

/// One member of the body of work that Evo does not open, and why.
///
/// Withholding is not discarding. Every member here is named, so a person who
/// wants it can still reach it, and the reason is stated so they can tell
/// *why* Evo did not put it in front of them. That is what keeps selective
/// restoration honest: the withheld set is the set Evo could be wrong about, so
/// it stays visible and recoverable rather than being silently dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithheldMember {
    artifact_id: ArtifactId,
    disposition: Disposition,
    reason: String,
}

impl WithheldMember {
    /// Constructs a withheld-member record.
    ///
    /// `disposition` is [`Disposition::AvailableIfNeeded`] or
    /// [`Disposition::HistoryOnly`]; a withheld member is by definition not
    /// opened.
    pub fn new(
        artifact_id: ArtifactId,
        disposition: Disposition,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            artifact_id,
            disposition,
            reason: reason.into(),
        }
    }

    /// The canonical Artifact that is not opened.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// Whether Evo keeps this to hand or keeps it only as history.
    pub fn disposition(&self) -> Disposition {
        self.disposition
    }

    /// Why this member is not opened.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// Why a surface member is not restore-worthy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableReason {
    /// The canonical Artifact that is not restore-worthy.
    artifact_id: ArtifactId,
    /// The derived resource identity, when canonical evidence classifies
    /// one (a commit is classified but has no executable target).
    identity: Option<ResourceIdentity>,
    /// The honest reason, naming the missing capability exactly.
    reason: String,
}

impl UnavailableReason {
    /// Constructs an unavailable-reason record.
    pub fn new(
        artifact_id: ArtifactId,
        identity: Option<ResourceIdentity>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            artifact_id,
            identity,
            reason: reason.into(),
        }
    }

    /// The canonical Artifact that is not restore-worthy.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// The derived resource identity, when one is classifiable.
    pub fn identity(&self) -> Option<&ResourceIdentity> {
        self.identity.as_ref()
    }

    /// The honest reason this resource cannot be restored.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// The derived selective-restoration understanding for one Workspace.
///
/// Partitions the Workspace's current continuation into restore-worthy and
/// unavailable members, and keeps the durable historical membership
/// separate. Purely derived: rebuildable from canonical state, never
/// canonical itself, never used to mutate Workspace membership, Artifact
/// identity, or continuation semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorationSelection {
    workspace_id: WorkspaceId,
    /// The current continuation members that are restore-worthy, in
    /// canonical order (ascending ArtifactId).
    restore_worthy: Vec<SelectedResource>,
    /// The current continuation members that cannot be restored, in
    /// canonical order, each with the honest reason.
    unavailable: Vec<UnavailableReason>,
    /// Workspace members outside the current continuation surface: durable
    /// membership, never opened, each with its [`Disposition`] and reason.
    withheld: Vec<WithheldMember>,
    /// Which restore-worthy member is where the work continues, when the
    /// derivation established one. Derived, like everything else here: it is
    /// the plan's own Resume Point, recorded so restoration order does not
    /// have to infer it from a reason string.
    resume_point: Option<ArtifactId>,
}

impl RestorationSelection {
    /// Constructs a selection for one Workspace.
    pub fn new(
        workspace_id: WorkspaceId,
        restore_worthy: Vec<SelectedResource>,
        unavailable: Vec<UnavailableReason>,
        withheld: Vec<WithheldMember>,
    ) -> Self {
        Self {
            workspace_id,
            restore_worthy,
            unavailable,
            withheld,
            resume_point: None,
        }
    }

    /// Records which member is where the work continues.
    pub fn with_resume_point(mut self, resume_point: Option<ArtifactId>) -> Self {
        self.resume_point = resume_point;
        self
    }

    /// Where the work continues, if the derivation established it and it is
    /// part of this selection's current continuation.
    pub fn resume_point(&self) -> Option<&ArtifactId> {
        self.resume_point.as_ref()
    }

    /// The order Evo restores this selection in, as a sort key.
    ///
    /// Where the work continues sorts before everything else; everything else
    /// sorts by canonical ArtifactId. Restoration is not a list to get
    /// through — it is putting someone back where they were — so the Resume
    /// Point goes first even when it is not first alphabetically, and the rest
    /// follow canonically so the sequence is identical on every run.
    ///
    /// This is the one place the restoration order is decided, so the plan Evo
    /// announces and the plan Evo acts on cannot diverge.
    pub fn restoration_key<'a>(&self, artifact_id: &'a ArtifactId) -> (u8, &'a str) {
        if self.resume_point.as_ref() == Some(artifact_id) {
            (0, "")
        } else {
            (1, artifact_id.as_str())
        }
    }

    /// The Workspace this selection describes.
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// The current continuation members that are restore-worthy, in
    /// canonical order. Every one carries the reason it opens unasked.
    pub fn restore_worthy(&self) -> &[SelectedResource] {
        &self.restore_worthy
    }

    /// The current continuation members that cannot be restored, in
    /// canonical order, each with the honest reason.
    pub fn unavailable(&self) -> &[UnavailableReason] {
        &self.unavailable
    }

    /// Every Workspace member outside the current continuation, in canonical
    /// order, each with its [`Disposition`] and the reason it is not opened.
    ///
    /// Nothing here is opened — this is the set execution never receives (see
    /// [`crate::engine::execute_selection`]). Everything here is named, so
    /// nothing a person might want becomes unreachable because Evo judged it
    /// unnecessary. There is deliberately no accessor for the union of both
    /// dispositions: *kept to hand* and *kept as record* are different answers
    /// to a person returning to work, and collapsing them is what makes
    /// restoration read like a list of everything that happened.
    pub fn withheld(&self) -> &[WithheldMember] {
        &self.withheld
    }

    /// The withheld members Evo keeps to hand: part of this work, one gesture
    /// away, not opened unasked.
    pub fn available_if_needed(&self) -> impl Iterator<Item = &WithheldMember> {
        self.withheld
            .iter()
            .filter(|member| member.disposition() == Disposition::AvailableIfNeeded)
    }

    /// The withheld members Evo keeps only as record: present while the work
    /// happened, with nothing beyond that tying them to it.
    pub fn history_only(&self) -> impl Iterator<Item = &WithheldMember> {
        self.withheld
            .iter()
            .filter(|member| member.disposition() == Disposition::HistoryOnly)
    }

    /// Whether the selection has no restore-worthy member.
    pub fn is_empty(&self) -> bool {
        self.restore_worthy.is_empty()
    }
}

/// Computes the selective-restoration understanding for one Workspace.
///
/// # Inputs (canonical only)
///
/// - `outcome` — the derived plan outcome, whose per-Workspace Continuation
///   Surface (RFC-0013) is the current continuation this selection narrows;
/// - `workspace_members` — the canonical Artifacts of the Workspace with the
///   part each played in the work (its Attachment Set membership and roles),
///   for the withheld partition;
/// - `resource_evidence` — ArtifactId → (frozen schema name, witnessed
///   subject), derived from the canonical Observation log by the Artifact
///   layer. The same evidence the Execution layer's locator classification
///   consumes.
///
/// # Rules
///
/// - A surface member whose canonical evidence classifies to an executable
///   target is **restore-worthy** — [`Disposition::OpenNow`] — and carries the
///   reason it opens: it is the Resume Point, or another place the continuation
///   happens. No other ground opens anything unasked.
/// - A surface member whose identity is a commit (no executable target) is
///   **unavailable**, honestly named.
/// - A surface member whose identity cannot be resolved from canonical
///   evidence is **unavailable** — no guessing (Law V, Law VI).
/// - A Workspace member outside the surface is **withheld**, and the part it
///   played decides which promise Evo makes about it:
///   [`ResourceRole::Context`] means Evo witnessed it beside the work and
///   nothing more, so it is [`Disposition::HistoryOnly`]; any other role means
///   the person used it as part of this work, so it is
///   [`Disposition::AvailableIfNeeded`] and one gesture away.
///
/// # Why the role and not a measurement
///
/// The role is canonical Attachment state the Workspace layer already derived
/// from witnessed evidence, read here as given. This function introduces no
/// threshold, no ranking, and no second judgement about importance: if a
/// resource's part in the work is wrong, the fix belongs where roles are
/// assigned, not here. Recency, frequency, and application identity play no
/// part at any point.
///
/// Deterministic: identical canonical inputs produce identical selections.
#[allow(clippy::module_name_repetitions)]
pub fn select_restoration(
    outcome: &DerivationOutcome,
    workspace_members: &[(ArtifactId, ResourceRole)],
    resource_evidence: &HashMap<String, (String, String)>,
) -> RestorationSelection {
    let workspace_id = outcome.workspace_id().clone();
    // The current continuation of this Workspace (RFC-0013 per-Workspace
    // intersection), already in canonical order.
    let surface = outcome.continuation_surface();
    // The one member that may be opened because the work continues *there*
    // rather than merely alongside. Absent when the derivation established no
    // Resume Point, in which case no surface member can claim to be it.
    let resume_point = match outcome {
        DerivationOutcome::Complete(plan) => Some(plan.resume_point().artifact_id().clone()),
        DerivationOutcome::Insufficient(insufficient) => insufficient
            .resume_point()
            .map(|point| point.artifact_id().clone()),
    };

    // The continuation surface must be a subset of workspace membership.
    // Artifacts that appear in the surface but are NOT members of this
    // Workspace are excluded — they may have arrived through derivation
    // side-effects (for example, execution-originated observations that
    // feed back into re-derivation) and must not become restore-worthy.
    let member_ids: std::collections::HashSet<String> = workspace_members
        .iter()
        .map(|(id, _)| id.as_str().to_string())
        .collect();

    let mut restore_worthy: Vec<SelectedResource> = Vec::new();
    let mut unavailable: Vec<UnavailableReason> = Vec::new();

    for artifact_id in surface {
        // Guard: only Workspace members may be restored. A surface member
        // that is not in the Workspace's membership is silently excluded
        // rather than guessed into a restore target.
        if !member_ids.contains(artifact_id.as_str()) {
            continue;
        }
        let evidence = resource_evidence.get(artifact_id.as_str());
        // Tied to continuation, and to nothing else: this is either where the
        // work continues, or another place that same continuation happens.
        let reason = if resume_point.as_ref() == Some(artifact_id) {
            OPEN_RESUME_POINT_REASON
        } else {
            OPEN_SURFACE_MEMBER_REASON
        };
        match evidence {
            Some((schema_name, subject)) => {
                match classify_resource_identity(schema_name, subject) {
                    Some(identity) if identity.is_restorable() => {
                        restore_worthy.push(SelectedResource::new(
                            artifact_id.clone(),
                            identity,
                            reason,
                        ));
                    }
                    Some(identity) => {
                        // A commit (or another identity without an executable
                        // target): a real resource, honestly not reopenable.
                        unavailable.push(UnavailableReason::new(
                            artifact_id.clone(),
                            Some(identity),
                            "this resource is a repository object (a commit) and has no \
                             executable target; Evo does not guess how to reopen it",
                        ));
                    }
                    None => {
                        unavailable.push(UnavailableReason::new(
                            artifact_id.clone(),
                            None,
                            "canonical evidence does not classify this Artifact to a \
                             reopenable resource; Evo does not guess",
                        ));
                    }
                }
            }
            None => {
                unavailable.push(UnavailableReason::new(
                    artifact_id.clone(),
                    None,
                    "no canonical resource evidence is recorded for this Artifact; \
                     Evo does not guess",
                ));
            }
        }
    }

    // Withheld membership: Workspace members outside the current continuation.
    // Durable, never rewritten by this selection, and never silently dropped —
    // each one is listed with the promise Evo makes about it.
    let mut withheld: Vec<WithheldMember> = workspace_members
        .iter()
        .filter(|(artifact_id, _)| !surface.contains(artifact_id))
        .map(|(artifact_id, role)| {
            let (disposition, reason) = match role {
                ResourceRole::Context => (Disposition::HistoryOnly, HISTORY_ONLY_REASON),
                ResourceRole::Continuation | ResourceRole::Primary => (
                    Disposition::AvailableIfNeeded,
                    WITHHELD_PLACE_OF_WORK_REASON,
                ),
                ResourceRole::Supporting | ResourceRole::Reference => {
                    (Disposition::AvailableIfNeeded, WITHHELD_CONSULTED_REASON)
                }
            };
            WithheldMember::new(artifact_id.clone(), disposition, reason)
        })
        .collect();
    withheld.sort_by(|a, b| a.artifact_id().as_str().cmp(b.artifact_id().as_str()));

    // The Resume Point is carried through only when it is actually part of
    // this selection's restore-worthy set: a derivation may name a Resume Point
    // that resolves to nothing openable, and that must not become a phantom
    // first place in the restoration order.
    let resume_point = resume_point.filter(|point| {
        restore_worthy
            .iter()
            .any(|selected| selected.artifact_id() == point)
    });

    RestorationSelection::new(workspace_id, restore_worthy, unavailable, withheld)
        .with_resume_point(resume_point)
}

#[cfg(test)]
mod tests {
    use super::*;

    use evo_restoration::{
        ContextChain, DerivationOutcome, NextStep, RestorationPlan, ResumePoint,
    };
    use evo_workspace::WorkspaceId;

    fn artifact(id: &str) -> ArtifactId {
        ArtifactId::new(id).unwrap()
    }

    /// Workspace membership as the Workspace layer hands it over: each member
    /// paired with the role that layer already derived for it from witnessed
    /// evidence. This function never re-judges the role.
    fn member(id: &str, role: ResourceRole) -> (ArtifactId, ResourceRole) {
        (artifact(id), role)
    }

    /// The withheld set as bare ids, in canonical order — for the tests that
    /// only care that a member was not opened, not which way it was kept.
    fn withheld_ids(selection: &RestorationSelection) -> Vec<&str> {
        selection
            .withheld()
            .iter()
            .map(|member| member.artifact_id().as_str())
            .collect()
    }

    fn record_evidence(
        map: &mut HashMap<String, (String, String)>,
        artifact_id: &str,
        schema: &str,
        subject: &str,
    ) {
        map.insert(
            artifact_id.to_string(),
            (schema.to_string(), subject.to_string()),
        );
    }

    /// Builds a Complete plan whose surface is exactly the given artifact
    /// ids (the derivation already intersected the global surface with the
    /// Workspace). `resume` must be one of the artifacts.
    fn plan_with_surface(resume: &str, surface: &[&str]) -> RestorationPlan {
        let workspace_id = WorkspaceId::new();
        let surface_ids: Vec<ArtifactId> = surface.iter().map(|id| artifact(id)).collect();
        RestorationPlan::new_with_continuation_surface(
            workspace_id.clone(),
            ResumePoint::new(workspace_id.clone(), artifact(resume)),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
            surface_ids,
        )
        .expect("valid plan")
    }

    fn outcome_for(resume: &str, surface: &[&str]) -> DerivationOutcome {
        DerivationOutcome::Complete(plan_with_surface(resume, surface))
    }

    const SCHEMA_FILE: &str = "OBS-FILE-SAVED";
    const SCHEMA_URL: &str = "OBS-URL-NAVIGATED";
    const SCHEMA_WINDOW: &str = "OBS-WINDOW-FOCUS-GAINED";
    const SCHEMA_COMMIT: &str = "OBS-COMMIT-MADE";

    // ── Mission matrix 1: many artifacts, two in the surface ────────────────

    #[test]
    fn only_surface_members_are_restore_candidates() {
        // Workspace has five artifacts; the user declared a two-member
        // surface. Only those two may be restore-worthy; the other three
        // remain durable historical membership.
        let workspace_artifacts = [
            member("artifact-a", ResourceRole::Continuation),
            member("artifact-b", ResourceRole::Primary),
            member("artifact-c", ResourceRole::Primary),
            member("artifact-d", ResourceRole::Supporting),
            member("artifact-e", ResourceRole::Context),
        ];
        let outcome = outcome_for("artifact-a", &["artifact-a", "artifact-b"]);
        let mut evidence_map = HashMap::new();
        record_evidence(&mut evidence_map, "artifact-a", SCHEMA_FILE, "/repo/a.md");
        record_evidence(
            &mut evidence_map,
            "artifact-b",
            SCHEMA_URL,
            "https://docs.dev/a",
        );
        record_evidence(&mut evidence_map, "artifact-c", SCHEMA_FILE, "/repo/c.md");
        record_evidence(&mut evidence_map, "artifact-d", SCHEMA_WINDOW, "Window D");
        record_evidence(&mut evidence_map, "artifact-e", SCHEMA_FILE, "/repo/e.md");

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert_eq!(selection.restore_worthy().len(), 2);
        let ids: Vec<&str> = selection
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().as_str())
            .collect();
        assert_eq!(ids, vec!["artifact-a", "artifact-b"]);
        assert!(selection.unavailable().is_empty());
        // The other three are withheld: durable membership, not opened merely
        // because they belong to the body of work.
        assert_eq!(selection.withheld().len(), 3);
        // And every one of them says which way it is kept, and why. C and D
        // are places this work happens and things it consulted, so they stay
        // to hand; E was only ever present, so it stays in the record.
        assert_eq!(
            withheld_ids(&selection),
            vec!["artifact-c", "artifact-d", "artifact-e"]
        );
        assert_eq!(selection.available_if_needed().count(), 2);
        assert_eq!(selection.history_only().count(), 1);
        for withheld in selection.withheld() {
            assert!(!withheld.reason().is_empty());
            assert!(!withheld.disposition().opens_unasked());
        }
    }

    // ── Mission matrix 2: historical artifacts outside the surface ──────────

    #[test]
    fn historical_artifacts_are_never_restore_candidates() {
        let workspace_artifacts = [
            member("artifact-current", ResourceRole::Continuation),
            member("artifact-historical-1", ResourceRole::Reference),
            member("artifact-historical-2", ResourceRole::Context),
        ];
        let outcome = outcome_for("artifact-current", &["artifact-current"]);
        let mut evidence_map = HashMap::new();
        record_evidence(
            &mut evidence_map,
            "artifact-current",
            SCHEMA_FILE,
            "/repo/now.md",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-historical-1",
            SCHEMA_FILE,
            "/repo/old1.md",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-historical-2",
            SCHEMA_FILE,
            "/repo/old2.md",
        );

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert_eq!(selection.restore_worthy().len(), 1);
        assert_eq!(
            selection.restore_worthy()[0].artifact_id().as_str(),
            "artifact-current"
        );
        let historical = withheld_ids(&selection);
        assert_eq!(
            historical,
            vec!["artifact-historical-1", "artifact-historical-2"]
        );
        // Neither is opened, and the two are not confused with each other: one
        // was consulted during the work, the other was merely present.
        assert_eq!(
            selection.withheld()[0].disposition(),
            Disposition::AvailableIfNeeded
        );
        assert_eq!(
            selection.withheld()[1].disposition(),
            Disposition::HistoryOnly
        );
    }

    // ── Mission matrix 3 + 4: tool switching keeps resource identity stable ──

    #[test]
    fn tool_switch_does_not_change_resource_identity() {
        // The same file witnessed again (as a different editor would) is the
        // same resource identity; the selection is driven by canonical
        // evidence, never by application identity.
        let workspace_artifacts = [member("artifact-file", ResourceRole::Continuation)];
        let outcome = outcome_for("artifact-file", &["artifact-file"]);
        let mut evidence_map = HashMap::new();
        record_evidence(
            &mut evidence_map,
            "artifact-file",
            SCHEMA_FILE,
            "/repo/src/lib.rs",
        );

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert_eq!(selection.restore_worthy().len(), 1);
        let identity = selection.restore_worthy()[0].identity();
        assert_eq!(
            identity,
            &ResourceIdentity::FilePath("/repo/src/lib.rs".into())
        );
        // The identity carries no application name whatsoever.
        assert_eq!(identity.subject(), "/repo/src/lib.rs");
    }

    // ── Mission matrix 5 + 6: old tools / recent unrelated resources ────────

    #[test]
    fn old_tool_and_recent_unrelated_resource_are_historical_not_restored() {
        let workspace_artifacts = [
            member("artifact-current", ResourceRole::Continuation),
            member("artifact-old-tool", ResourceRole::Primary),
            member("artifact-unrelated", ResourceRole::Context),
        ];
        let outcome = outcome_for("artifact-current", &["artifact-current"]);
        let mut evidence_map = HashMap::new();
        record_evidence(
            &mut evidence_map,
            "artifact-current",
            SCHEMA_FILE,
            "/repo/now.md",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-old-tool",
            SCHEMA_FILE,
            "/repo/old.md",
        );
        // A URL opened recently in the same period, but never declared part
        // of the continuation: it is not restored.
        record_evidence(
            &mut evidence_map,
            "artifact-unrelated",
            SCHEMA_URL,
            "https://example.com/random",
        );

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert_eq!(selection.restore_worthy().len(), 1);
        assert_eq!(selection.withheld().len(), 2);
        // The old tool is still a place this work happens, so it is kept to
        // hand rather than discarded; the unrelated URL is record only.
        assert_eq!(selection.available_if_needed().count(), 1);
        assert_eq!(selection.history_only().count(), 1);
    }

    // ── Mission matrix 7: no surface → no guessed restoration set ───────────

    #[test]
    fn no_surface_means_no_guessed_restore_set() {
        let workspace_artifacts = [
            member("artifact-a", ResourceRole::Continuation),
            member("artifact-b", ResourceRole::Primary),
        ];
        // An empty surface: the plan carries no declared continuation.
        let outcome = outcome_for("artifact-a", &[]);
        let mut evidence_map = HashMap::new();
        record_evidence(&mut evidence_map, "artifact-a", SCHEMA_FILE, "/repo/a.md");
        record_evidence(&mut evidence_map, "artifact-b", SCHEMA_FILE, "/repo/b.md");

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        // Nothing is guessed into a restore set from membership or recency.
        assert!(selection.restore_worthy().is_empty());
        assert!(selection.unavailable().is_empty());
        // But nothing is lost either: both members are still named, still
        // reachable, and both are the highest roles this work has — so with no
        // continuation to open, Evo offers them rather than opening them.
        assert_eq!(selection.withheld().len(), 2);
        assert_eq!(selection.available_if_needed().count(), 2);
        assert!(selection.is_empty());
    }

    // ── Mission matrix 8: unresolvable resource → unavailable, not guessed ──

    #[test]
    fn unresolvable_resource_is_unavailable_not_guessed() {
        let workspace_artifacts = [
            member("artifact-commit", ResourceRole::Primary),
            member("artifact-no-evidence", ResourceRole::Supporting),
            member("artifact-ok", ResourceRole::Continuation),
        ];
        let outcome = outcome_for(
            "artifact-ok",
            &["artifact-commit", "artifact-no-evidence", "artifact-ok"],
        );
        let mut evidence_map = HashMap::new();
        record_evidence(
            &mut evidence_map,
            "artifact-commit",
            SCHEMA_COMMIT,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        );
        record_evidence(&mut evidence_map, "artifact-ok", SCHEMA_FILE, "/repo/ok.md");
        // artifact-no-evidence has no canonical resource evidence.

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert_eq!(selection.restore_worthy().len(), 1);
        assert_eq!(
            selection.restore_worthy()[0].artifact_id().as_str(),
            "artifact-ok"
        );
        // Both unresolvable members are honestly unavailable, with reasons.
        assert_eq!(selection.unavailable().len(), 2);
        for unavailable in selection.unavailable() {
            assert!(!unavailable.reason().is_empty());
            if unavailable.artifact_id().as_str() == "artifact-commit" {
                assert!(unavailable.identity().is_some());
                assert_eq!(
                    unavailable.identity().unwrap(),
                    &ResourceIdentity::Commit(
                        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".into()
                    )
                );
            } else {
                assert!(unavailable.identity().is_none());
            }
        }
    }

    // ── Mission matrix 9: selection never changes Workspace membership ──────

    #[test]
    fn selection_never_mutates_workspace_membership_or_identity() {
        let workspace_artifacts = [
            member("artifact-a", ResourceRole::Continuation),
            member("artifact-b", ResourceRole::Reference),
        ];
        let before = workspace_artifacts.to_vec();
        let outcome = outcome_for("artifact-a", &["artifact-a"]);
        let mut evidence_map = HashMap::new();
        record_evidence(&mut evidence_map, "artifact-a", SCHEMA_FILE, "/repo/a.md");
        record_evidence(&mut evidence_map, "artifact-b", SCHEMA_FILE, "/repo/b.md");

        let _ = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        // The Workspace's canonical artifact set is untouched — every member
        // and every role it arrived with.
        assert_eq!(workspace_artifacts, before.as_slice());
        // Artifact identities are untouched — the selection references the
        // same canonical identities and never derives new ones.
        for (id, _) in workspace_artifacts {
            assert_eq!(id, ArtifactId::new(id.as_str()).unwrap());
        }
    }

    // ── Mission matrix 11 + 12: deterministic + restart-equivalent ──────────

    #[test]
    fn selection_is_deterministic_and_replay_equivalent() {
        let workspace_artifacts = [
            member("artifact-z", ResourceRole::Primary),
            member("artifact-a", ResourceRole::Continuation),
            member("artifact-m", ResourceRole::Context),
        ];
        let outcome = outcome_for("artifact-a", &["artifact-a", "artifact-z"]);
        let mut evidence_map = HashMap::new();
        record_evidence(&mut evidence_map, "artifact-z", SCHEMA_FILE, "/repo/z.md");
        record_evidence(&mut evidence_map, "artifact-a", SCHEMA_URL, "https://a.dev");
        record_evidence(&mut evidence_map, "artifact-m", SCHEMA_WINDOW, "Window M");

        // Identical canonical inputs → identical selection (a full replay
        // re-derives the same derived state).
        let first = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        let second = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert_eq!(first, second);
        // Canonical order: ascending ArtifactId.
        assert_eq!(
            first.restore_worthy()[0].artifact_id().as_str(),
            "artifact-a"
        );
        assert_eq!(
            first.restore_worthy()[1].artifact_id().as_str(),
            "artifact-z"
        );
    }

    // ── Mission matrix 13: no application/domain/profession hardcoding ──────

    #[test]
    fn no_application_or_domain_hardcoding_in_the_selection() {
        // The selection classifies canonical schemas only; a hypothetical
        // future schema that is not frozen here is unavailable, never
        // guessed, and never matched by name.
        let workspace_artifacts = [member("artifact-future", ResourceRole::Continuation)];
        let outcome = outcome_for("artifact-future", &["artifact-future"]);
        let mut evidence_map = HashMap::new();
        record_evidence(
            &mut evidence_map,
            "artifact-future",
            "OBS-FUTURE-COLLECTOR",
            "anything",
        );
        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert!(selection.restore_worthy().is_empty());
        assert_eq!(selection.unavailable().len(), 1);
        assert!(
            selection.unavailable()[0]
                .reason()
                .contains("does not guess")
        );
    }

    // ── Mission matrix 14 + 15: one abstraction for every collector ─────────

    #[test]
    fn every_frozen_collector_feeds_the_same_selection() {
        let workspace_artifacts = [
            member("artifact-window", ResourceRole::Primary),
            member("artifact-file", ResourceRole::Continuation),
            member("artifact-url", ResourceRole::Supporting),
            member("artifact-commit", ResourceRole::Reference),
        ];
        let outcome = outcome_for(
            "artifact-file",
            &[
                "artifact-window",
                "artifact-file",
                "artifact-url",
                "artifact-commit",
            ],
        );
        let mut evidence_map = HashMap::new();
        record_evidence(
            &mut evidence_map,
            "artifact-window",
            SCHEMA_WINDOW,
            "Editor — main.rs",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-file",
            SCHEMA_FILE,
            "/repo/main.rs",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-url",
            SCHEMA_URL,
            "https://docs.rs/evolve",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-commit",
            SCHEMA_COMMIT,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        );

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        // Window, file, and URL are restore-worthy through one abstraction;
        // the commit is honestly unavailable.
        assert_eq!(selection.restore_worthy().len(), 3);
        assert_eq!(selection.unavailable().len(), 1);
        assert!(matches!(
            selection.unavailable()[0].identity(),
            Some(ResourceIdentity::Commit(_))
        ));
    }

    // ── Surface member outside the Workspace is never selected ──────────────

    #[test]
    fn cross_workspace_members_do_not_leak_into_the_selection() {
        // A surface member declared in another Workspace cannot appear in
        // this Workspace's selection because the plan's surface is already
        // the per-Workspace intersection; guard the invariant anyway: an
        // artifact absent from the Workspace membership is never historical.
        let workspace_artifacts = [member("artifact-own", ResourceRole::Continuation)];
        let outcome = outcome_for("artifact-own", &["artifact-own"]);
        let mut evidence_map = HashMap::new();
        record_evidence(
            &mut evidence_map,
            "artifact-own",
            SCHEMA_FILE,
            "/repo/own.md",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-foreign",
            SCHEMA_FILE,
            "/other/foreign.md",
        );

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert_eq!(selection.restore_worthy().len(), 1);
        // The Workspace has exactly one member and it is in the surface, so
        // nothing is withheld; the foreign artifact never leaks in.
        assert!(selection.withheld().is_empty());
    }

    // ── Mission §9: the three-month problem ─────────────────────────────────

    #[test]
    fn three_month_surface_switch_keeps_old_work_historical() {
        // Month 1: the Workspace contains A B C D E; the surface was A B.
        // Month 3: the user returns and declares C D. The Workspace
        // membership A B C D E must survive unchanged; the current
        // continuation is exactly C D; A B E remain historical. Time never
        // rewrites membership.
        let workspace_artifacts = [
            member("artifact-a", ResourceRole::Primary),
            member("artifact-b", ResourceRole::Supporting),
            member("artifact-c", ResourceRole::Continuation),
            member("artifact-d", ResourceRole::Primary),
            member("artifact-e", ResourceRole::Context),
        ];
        let outcome = outcome_for("artifact-c", &["artifact-c", "artifact-d"]);
        let mut evidence_map = HashMap::new();
        for (id, path) in [
            ("artifact-a", "/repo/a.md"),
            ("artifact-b", "/repo/b.md"),
            ("artifact-c", "/repo/c.md"),
            ("artifact-d", "/repo/d.md"),
            ("artifact-e", "/repo/e.md"),
        ] {
            record_evidence(&mut evidence_map, id, SCHEMA_FILE, path);
        }

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        // Current continuation: exactly C and D.
        let worthy: Vec<&str> = selection
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().as_str())
            .collect();
        assert_eq!(worthy, vec!["artifact-c", "artifact-d"]);
        // Old members remain durable withheld membership — never deleted,
        // never rebuilt into a new Workspace.
        let historical = withheld_ids(&selection);
        assert_eq!(historical, vec!["artifact-a", "artifact-b", "artifact-e"]);
        assert!(selection.unavailable().is_empty());
    }

    // ── Mission §8 reverse: the surface moves to newer work ────────────────

    #[test]
    fn surface_moves_to_newer_work_old_resources_stay_historical() {
        // A and B were the current continuation; the user later declared C.
        // C is restored; A and B remain historical members. There is no
        // recency-based promotion and no deletion of old membership.
        let workspace_artifacts = [
            member("artifact-a", ResourceRole::Primary),
            member("artifact-b", ResourceRole::Supporting),
            member("artifact-c", ResourceRole::Continuation),
        ];
        let outcome = outcome_for("artifact-c", &["artifact-c"]);
        let mut evidence_map = HashMap::new();
        for (id, path) in [
            ("artifact-a", "/repo/a.md"),
            ("artifact-b", "/repo/b.md"),
            ("artifact-c", "/repo/c.md"),
        ] {
            record_evidence(&mut evidence_map, id, SCHEMA_FILE, path);
        }

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        let worthy: Vec<&str> = selection
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().as_str())
            .collect();
        assert_eq!(worthy, vec!["artifact-c"]);
        assert_eq!(selection.withheld().len(), 2);
        for withheld in selection.withheld() {
            assert_ne!(withheld.artifact_id().as_str(), "artifact-c");
        }
    }

    // ── Mission §8: recent but irrelevant ───────────────────────────────────

    #[test]
    fn recently_witnessed_unrelated_resources_are_not_restored() {
        // The Workspace's continuation is A and B. Around the same period,
        // other resources were witnessed — a browser tab, a chat window —
        // but never declared into the continuation. They must not enter the
        // restore set: recent observation is not continuation evidence.
        let workspace_artifacts = [
            member("artifact-a", ResourceRole::Continuation),
            member("artifact-b", ResourceRole::Primary),
            member("artifact-recent-browser", ResourceRole::Context),
            member("artifact-recent-chat", ResourceRole::Context),
        ];
        let outcome = outcome_for("artifact-a", &["artifact-a", "artifact-b"]);
        let mut evidence_map = HashMap::new();
        record_evidence(&mut evidence_map, "artifact-a", SCHEMA_FILE, "/repo/a.md");
        record_evidence(&mut evidence_map, "artifact-b", SCHEMA_FILE, "/repo/b.md");
        record_evidence(
            &mut evidence_map,
            "artifact-recent-browser",
            SCHEMA_URL,
            "https://unrelated.example/tab",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-recent-chat",
            SCHEMA_WINDOW,
            "Chat — unrelated",
        );

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        let worthy: Vec<&str> = selection
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().as_str())
            .collect();
        assert_eq!(worthy, vec!["artifact-a", "artifact-b"]);
        assert_eq!(selection.withheld().len(), 2);
        for withheld in selection.withheld() {
            let id = withheld.artifact_id().as_str();
            assert!(
                id == "artifact-recent-browser" || id == "artifact-recent-chat",
                "only the unrelated recent resources are withheld: {id}"
            );
            // Nothing beyond having been there ties them to the work, so they
            // are kept as record — not offered, not opened, not deleted.
            assert_eq!(withheld.disposition(), Disposition::HistoryOnly);
        }
    }

    // ── Mission §9: the three dispositions are distinct, and stated ─────────

    #[test]
    fn withheld_members_state_which_way_they_are_kept_and_why() {
        // One body of work with all five roles. The surface is the two
        // continuation members. Every remaining member must be withheld with
        // a disposition and a reason, and the reasons must distinguish "part
        // of this work, kept to hand" from "witnessed, nothing more".
        let workspace_artifacts = [
            member("artifact-continuation", ResourceRole::Continuation),
            member("artifact-primary-open", ResourceRole::Primary),
            member("artifact-primary-held", ResourceRole::Primary),
            member("artifact-supporting", ResourceRole::Supporting),
            member("artifact-reference", ResourceRole::Reference),
            member("artifact-context", ResourceRole::Context),
        ];
        let outcome = outcome_for(
            "artifact-continuation",
            &["artifact-continuation", "artifact-primary-open"],
        );
        let mut evidence_map = HashMap::new();
        for (id, path) in [
            ("artifact-continuation", "/repo/continuation.md"),
            ("artifact-primary-open", "/repo/open.md"),
            ("artifact-primary-held", "/repo/held.md"),
            ("artifact-supporting", "/repo/supporting.md"),
            ("artifact-reference", "/repo/reference.md"),
            ("artifact-context", "/repo/context.md"),
        ] {
            record_evidence(&mut evidence_map, id, SCHEMA_FILE, path);
        }

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);

        // Two open, and each says why it opens — the Resume Point says so as
        // the place the work continues, the other as another place the same
        // continuation happens.
        assert_eq!(selection.restore_worthy().len(), 2);
        let resume = selection
            .restore_worthy()
            .iter()
            .find(|selected| selected.artifact_id().as_str() == "artifact-continuation")
            .expect("the resume point is restore-worthy");
        let alongside = selection
            .restore_worthy()
            .iter()
            .find(|selected| selected.artifact_id().as_str() == "artifact-primary-open")
            .expect("the other surface member is restore-worthy");
        assert_ne!(resume.reason(), alongside.reason());
        for selected in selection.restore_worthy() {
            assert!(!selected.reason().is_empty());
        }

        // Four withheld, in canonical order, each with a stated disposition.
        let held: Vec<(&str, Disposition)> = selection
            .withheld()
            .iter()
            .map(|member| (member.artifact_id().as_str(), member.disposition()))
            .collect();
        assert_eq!(
            held,
            vec![
                ("artifact-context", Disposition::HistoryOnly),
                ("artifact-primary-held", Disposition::AvailableIfNeeded),
                ("artifact-reference", Disposition::AvailableIfNeeded),
                ("artifact-supporting", Disposition::AvailableIfNeeded),
            ]
        );

        // A place of work reads differently from something consulted, and both
        // read differently from something merely witnessed.
        let reason_for = |id: &str| -> String {
            selection
                .withheld()
                .iter()
                .find(|member| member.artifact_id().as_str() == id)
                .expect("member is withheld")
                .reason()
                .to_string()
        };
        let place = reason_for("artifact-primary-held");
        let consulted = reason_for("artifact-supporting");
        let record = reason_for("artifact-context");
        assert_ne!(place, consulted);
        assert_ne!(consulted, record);
        assert_ne!(place, record);
        // Supporting and Reference were both consulted; they say the same
        // thing because the same thing is true of both.
        assert_eq!(consulted, reason_for("artifact-reference"));

        // Nothing withheld opens unasked, and nothing withheld is lost: every
        // member of the Workspace is accounted for exactly once.
        assert!(
            selection
                .withheld()
                .iter()
                .all(|member| !member.disposition().opens_unasked())
        );
        assert_eq!(
            selection.restore_worthy().len()
                + selection.unavailable().len()
                + selection.withheld().len(),
            workspace_artifacts.len()
        );
    }

    #[test]
    fn a_role_can_be_withheld_without_being_demoted_to_record() {
        // The distinction this test protects: a member that the person worked
        // *in* is not part of the continuation being restored, but it is also
        // not incidental. Collapsing the two is what makes restoration read
        // like a list of everything that happened.
        let workspace_artifacts = [
            member("artifact-resume", ResourceRole::Continuation),
            member("artifact-elsewhere", ResourceRole::Continuation),
        ];
        let outcome = outcome_for("artifact-resume", &["artifact-resume"]);
        let mut evidence_map = HashMap::new();
        record_evidence(
            &mut evidence_map,
            "artifact-resume",
            SCHEMA_FILE,
            "/repo/here.md",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-elsewhere",
            SCHEMA_FILE,
            "/repo/elsewhere.md",
        );

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert_eq!(selection.restore_worthy().len(), 1);
        assert_eq!(selection.withheld().len(), 1);
        assert_eq!(
            selection.withheld()[0].disposition(),
            Disposition::AvailableIfNeeded
        );
        assert_eq!(selection.history_only().count(), 0);
    }

    #[test]
    fn silence_withholds_everything_and_discards_nothing() {
        // Evo cannot say where this work resumes. The honest answer is to open
        // nothing — and to still be able to name every place the work happened.
        let workspace_artifacts = [
            member("artifact-a", ResourceRole::Primary),
            member("artifact-b", ResourceRole::Supporting),
            member("artifact-c", ResourceRole::Context),
        ];
        let outcome = outcome_for("artifact-a", &[]);
        let mut evidence_map = HashMap::new();
        for (id, path) in [
            ("artifact-a", "/repo/a.md"),
            ("artifact-b", "/repo/b.md"),
            ("artifact-c", "/repo/c.md"),
        ] {
            record_evidence(&mut evidence_map, id, SCHEMA_FILE, path);
        }

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert!(selection.is_empty());
        assert!(selection.restore_worthy().is_empty());
        // Silence about where to resume is not silence about what the work is.
        assert_eq!(selection.withheld().len(), 3);
        assert_eq!(selection.available_if_needed().count(), 2);
        assert_eq!(selection.history_only().count(), 1);
    }

    // ── Restoration order: where the work continues comes first ─────────────

    #[test]
    fn the_resume_point_sorts_before_every_other_restore_worthy_member() {
        // The Resume Point is deliberately last alphabetically, so an order
        // that merely sorted by identity would put the person's own place at
        // the end of their own restoration.
        let workspace_artifacts = [
            member("artifact-a-docs", ResourceRole::Primary),
            member("artifact-b-notes", ResourceRole::Primary),
            member("artifact-z-resume", ResourceRole::Continuation),
        ];
        let outcome = outcome_for(
            "artifact-z-resume",
            &["artifact-a-docs", "artifact-b-notes", "artifact-z-resume"],
        );
        let mut evidence_map = HashMap::new();
        for (id, path) in [
            ("artifact-a-docs", "/repo/a.md"),
            ("artifact-b-notes", "/repo/b.md"),
            ("artifact-z-resume", "/repo/z.md"),
        ] {
            record_evidence(&mut evidence_map, id, SCHEMA_FILE, path);
        }
        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);

        assert_eq!(
            selection.resume_point().map(|id| id.as_str()),
            Some("artifact-z-resume")
        );
        let mut order: Vec<&str> = selection
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().as_str())
            .collect();
        // The stored set stays in canonical order — the selection is a set with
        // a known first, not a pre-sorted queue.
        assert_eq!(
            order,
            vec!["artifact-a-docs", "artifact-b-notes", "artifact-z-resume"]
        );

        // Restoration order puts the person back where they were, then the rest
        // canonically.
        let ids: Vec<ArtifactId> = selection
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().clone())
            .collect();
        let mut ordered = ids.clone();
        ordered.sort_by(|a, b| {
            selection
                .restoration_key(a)
                .cmp(&selection.restoration_key(b))
        });
        order = ordered.iter().map(|id| id.as_str()).collect();
        assert_eq!(
            order,
            vec!["artifact-z-resume", "artifact-a-docs", "artifact-b-notes"]
        );
    }

    #[test]
    fn a_resume_point_that_resolves_to_nothing_openable_claims_no_first_place() {
        // The derivation named a Resume Point, but it is a commit: nothing to
        // open. It must not become a phantom first step in the restoration.
        let workspace_artifacts = [
            member("artifact-commit", ResourceRole::Continuation),
            member("artifact-file", ResourceRole::Primary),
        ];
        let outcome = outcome_for("artifact-commit", &["artifact-commit", "artifact-file"]);
        let mut evidence_map = HashMap::new();
        record_evidence(
            &mut evidence_map,
            "artifact-commit",
            SCHEMA_COMMIT,
            "9f2c1ab",
        );
        record_evidence(
            &mut evidence_map,
            "artifact-file",
            SCHEMA_FILE,
            "/repo/f.md",
        );

        let selection = select_restoration(&outcome, &workspace_artifacts, &evidence_map);
        assert!(
            selection.resume_point().is_none(),
            "a Resume Point outside the restore-worthy set is not a first place"
        );
        // And it is still reported, with its own reason — not dropped.
        assert_eq!(selection.unavailable().len(), 1);
    }
}
