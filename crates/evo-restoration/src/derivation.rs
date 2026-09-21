//! Canonical Restoration Derivation (IS-0021 §25).
//!
//! This module implements the frozen Restoration Derivation algorithms from
//! IS-0021 §25. It transforms canonical Workspace/Snapshot understanding into
//! a deterministic derivation outcome.
//!
//! # Input Boundary
//!
//! Derivation consumes only (IS-0021 §25 as amended by RFC-0013):
//!
//! - the selected canonical Snapshot;
//! - the canonical Workspace that owns that Snapshot;
//! - the canonical designated Artifact (RFC-0011), when it exists;
//! - the Current Continuation Surface resolved to canonical Artifacts by the
//!   Artifact layer (RFC-0013), when it exists.
//!
//! The Snapshot's Attachment Set is the canonical representation of the
//! Artifacts it references (IS-0021 §25.1). No other input participates.
//!
//! # Output Boundary
//!
//! Derivation produces exactly one [`DerivationOutcome`] per input:
//!
//! - [`DerivationOutcome::Complete`] — a canonical [`RestorationPlan`] when
//!   every required component can be established from canonical evidence
//!   (IS-0021 §25.6);
//! - [`DerivationOutcome::Insufficient`] — a structured
//!   [`InsufficientRestoration`] record naming exactly which required
//!   components could not be established and why (IS-0021 §25.8).
//!
//! # Behavior under the current canonical model
//!
//! - **Resume Point** (IS-0021 §25.2): derivable when the canonical input
//!   identifies a designated canonical Artifact (RFC-0011) among the eligible
//!   candidates, when exactly one Attachment of the selected Snapshot carries
//!   [`ResourceRole::Continuation`], or when the Snapshot represents exactly one
//!   distinct eligible Artifact. Declared intent is consulted first and
//!   outranks the Attachment's role. Attachment order, Attachment Confidence,
//!   ArtifactId ordering, recency, and frequency are never Resume Point
//!   evidence.
//! - **Context Chain** (IS-0021 §25.3): the Attachments of the selected
//!   Snapshot that played a part in the work, other than the Resume Point
//!   itself, ordered by that part and then by canonical Snapshot order.
//!   Attachments carrying [`ResourceRole::Context`] are excluded: a resource
//!   that was merely present while the work happened is not context *for* the
//!   Resume Point, and CC-4 asks for the minimum, not the maximum. The complete
//!   membership set stays on the Workspace for surfaces that want all of it.
//! - **Blockers** (IS-0021 §25.4): zero — the current model contains no
//!   canonical unresolved-work representation, and no Blocker is invented from
//!   runtime or machine state.
//! - **Next Step** (IS-0021 §25.5): established by the same evidence that
//!   establishes the Resume Point when that evidence says something about
//!   continuation — a designation (RFC-0011) or the Continuation role. A single
//!   eligible Artifact establishes *where* to resume without saying anything
//!   about continuation, so the Next Step is then recorded as missing rather
//!   than manufactured.
//!
//! ## Authority change: the Context Chain and the Next Step (IS-0021 §25.3, §25.5)
//!
//! **The old rule.** §25.3 required an empty Context Chain and §25.2 required
//! insufficiency whenever a Snapshot referenced more than one Artifact, both
//! justified by the same claim: *"the current Snapshot model contains no
//! canonical supporting/reference/continuation relationship, so Workspace
//! membership alone never establishes Context relevance."* §25.5 likewise
//! admitted only a designation as continuation evidence.
//!
//! **Why it prevented the product from working.** The claim was true of the
//! Snapshot model as it stood and the conclusion followed correctly from it. It
//! is no longer true: an [`Attachment`](evo_workspace::attachment::Attachment)
//! now carries a [`ResourceRole`] — the part the Artifact played in the body of
//! work — precisely because a Workspace that could not distinguish where work
//! happens from what was merely open forced Restoration to treat every member
//! identically. Leaving the old rule in place with the new field present meant
//! a real body of work, which has many members by definition, could never
//! establish a Resume Point, so Restoration could never say where to continue
//! and the plan carried no context to show alongside it. Restoration was
//! structurally unable to restore.
//!
//! **The new rule.** Membership still never establishes relevance — the *role*
//! does, and the role is canonical Attachment state derived from witnessed
//! evidence, not an interpretation made here. The Resume Point is the member the
//! evidence names as where the work stopped; the Context Chain is the members
//! that played a part in it, in the order of that part. Declared intent
//! continues to outrank every inference (RFC-0011). Where the evidence names
//! nothing, derivation still records missing evidence rather than ranking
//! candidates by confidence, identifier, or attachment order.
//!
//! The designation evidence enters the input as the canonical designated
//! Artifact, resolved by the Artifact layer's deterministic identity
//! derivation over the canonical Observation log (RFC-0011 §4). Derivation
//! itself never consults raw Observations and never performs Artifact
//! Resolution (IS-0021 §4, §25.1).
//!
//! # Invariants
//!
//! - Deterministic (IS-0021 §25.9): identical inputs produce identical outcomes.
//! - Replayable (IS-0021 §25.10): derivation never mutates the Workspace,
//!   Snapshot, Artifacts, Attachments, or any historical understanding.
//! - Consumes canonical input only (IS-0021 §25.1).
//! - Never executes anything (IS-0021 §6, §20).
//! - Never performs Workspace Formation or Artifact Resolution (IS-0021 §4).

use evo_artifact::artifact_id::ArtifactId;
use evo_observation::observed_state::ObservedState;
use evo_workspace::attachment::{Attachment, ResourceRole};
use evo_workspace::snapshot::Snapshot;
use evo_workspace::workspace::Workspace;
use evo_workspace::WorkspaceId;

use crate::blocker::Blocker;
use crate::context_chain::ContextChain;
use crate::errors::RestorationError;
use crate::next_step::NextStep;
use crate::resume_point::ResumePoint;
use crate::restoration_plan::RestorationPlan;

// ── Canonical reason constants (IS-0021 §25) ─────────────────────────────────

/// Resume Point missing because the selected Snapshot references no Artifacts.
const NO_CANDIDATE_REASON: &str = "No eligible Artifact is represented \
    by the selected Snapshot, so the Resume Point cannot be established from \
    canonical evidence (IS-0021 §25.2).";

/// Resume Point missing because several eligible Artifacts exist and no
/// canonical evidence distinguishes one of them.
const MULTIPLE_CANDIDATES_REASON: &str = "Multiple eligible Artifacts are \
    represented by the selected Snapshot, no single Attachment carries the \
    Continuation role, and no designation names one of them, so no canonical \
    evidence distinguishes one as the Resume Point (IS-0021 §25.2).";

/// Next Step missing because no canonical evidence says anything about where
/// the work continues.
const NEXT_STEP_MISSING_REASON: &str = "No Attachment of the selected \
    Snapshot carries the Continuation role and no designation exists, so no \
    canonical continuation evidence establishes the Next Step \
    (IS-0021 §25.5).";

/// The canonical description of a Next Step established by WorkDesignated
/// evidence (RFC-0011, IS-0021 §25.5). Platform-independent and explainable
/// from the canonical evidence: the user declared the continuation.
const NEXT_STEP_DESIGNATED_DESCRIPTION: &str =
    "The user designated this resource as the work to continue.";

/// The canonical description of a Next Step established by the Continuation
/// role (IS-0021 §25.5 as amended).
///
/// It states the witnessed evidence and nothing beyond it: of the places this
/// body of work happens, this is the one attended most recently. It does not
/// claim to know what the person intended to do next, because no canonical
/// evidence records that.
const NEXT_STEP_CONTINUATION_DESCRIPTION: &str =
    "This is the resource you were last working in as part of this work, so \
     this is where the work continues.";

// ── RestorationInput ─────────────────────────────────────────────────────────

/// The canonical input boundary for Restoration Derivation (IS-0021 §25 as
/// amended by RFC-0013).
///
/// Carries the canonical Workspace and the selected canonical Snapshot owned
/// by that Workspace, the canonical designated Artifact (RFC-0011) when one
/// exists, and the Current Continuation Surface resolved to canonical
/// Artifacts by the Artifact layer (RFC-0013) when a valid declaration
/// exists. The Snapshot's Attachment Set is the canonical representation of
/// the Artifacts it references (IS-0021 §25.1).
#[derive(Debug, Clone, PartialEq)]
pub struct RestorationInput<'a> {
    workspace: &'a Workspace,
    snapshot: &'a Snapshot,
    /// The canonical designated Artifact established by the current
    /// WorkDesignated evidence (RFC-0011, IS-0021 §25.1), when it exists.
    /// It is resolved by the Artifact layer before derivation; this input
    /// carries canonical Artifact-level understanding, never raw Observations.
    designated: Option<ArtifactId>,
    /// The Current Continuation Surface (RFC-0013) resolved to canonical
    /// Artifacts by the Artifact layer, when the latest valid declaration
    /// exists. Derivation intersects it with the Workspace's Artifacts; it
    /// never consults raw Observations and never performs Artifact
    /// Resolution (IS-0021 §4, §25.1).
    continuation_surface: Option<Vec<ArtifactId>>,
    /// Optional state witnessed on the exact work-local occurrence selected
    /// for an Artifact. This is occurrence evidence, not Artifact identity or
    /// membership, and is never synthesized by Restoration.
    observed_states: Vec<(ArtifactId, ObservedState)>,
}

impl<'a> RestorationInput<'a> {
    /// Constructs a canonical derivation input with no designation evidence
    /// and no continuation surface.
    ///
    /// # Errors
    ///
    /// Returns [`RestorationError::SnapshotNotPartOfWorkspace`] when `snapshot`
    /// is not a member of `workspace`'s Snapshot History (IS-0021 §25.7).
    pub fn new(workspace: &'a Workspace, snapshot: &'a Snapshot) -> Result<Self, RestorationError> {
        Self::new_with_designated(workspace, snapshot, None)
    }

    /// Constructs a canonical derivation input with the canonical designated
    /// Artifact (RFC-0011) when current WorkDesignated evidence resolves one.
    ///
    /// # Errors
    ///
    /// Returns [`RestorationError::SnapshotNotPartOfWorkspace`] when `snapshot`
    /// is not a member of `workspace`'s Snapshot History (IS-0021 §25.7).
    pub fn new_with_designated(
        workspace: &'a Workspace,
        snapshot: &'a Snapshot,
        designated: Option<ArtifactId>,
    ) -> Result<Self, RestorationError> {
        Self::new_with_continuation_surface(workspace, snapshot, designated, None)
    }

    /// Constructs a canonical derivation input with the canonical designated
    /// Artifact (RFC-0011) and the Current Continuation Surface resolved to
    /// canonical Artifacts (RFC-0013).
    ///
    /// `continuation_surface` is the Artifact-level resolution of the latest
    /// valid continuation-surface declaration, provided by the Artifact layer
    /// (RFC-0013 Restoration Derivation Interaction). Derivation intersects it
    /// per Workspace; members outside this Workspace are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`RestorationError::SnapshotNotPartOfWorkspace`] when `snapshot`
    /// is not a member of `workspace`'s Snapshot History (IS-0021 §25.7).
    pub fn new_with_continuation_surface(
        workspace: &'a Workspace,
        snapshot: &'a Snapshot,
        designated: Option<ArtifactId>,
        continuation_surface: Option<Vec<ArtifactId>>,
    ) -> Result<Self, RestorationError> {
        Self::new_with_observed_states(
            workspace,
            snapshot,
            designated,
            continuation_surface,
            Vec::new(),
        )
    }

    /// Constructs a derivation input carrying state directly witnessed on the
    /// exact work-local stopping occurrence for each supplied Artifact.
    pub fn new_with_observed_states(
        workspace: &'a Workspace,
        snapshot: &'a Snapshot,
        designated: Option<ArtifactId>,
        continuation_surface: Option<Vec<ArtifactId>>,
        observed_states: Vec<(ArtifactId, ObservedState)>,
    ) -> Result<Self, RestorationError> {
        if !workspace.snapshots().iter().any(|candidate| candidate == snapshot) {
            return Err(RestorationError::SnapshotNotPartOfWorkspace {
                workspace_id: workspace.id().clone(),
            });
        }
        Ok(Self {
            workspace,
            snapshot,
            designated,
            continuation_surface,
            observed_states,
        })
    }

    /// Returns the canonical Workspace referenced by this input.
    pub fn workspace(&self) -> &'a Workspace {
        self.workspace
    }

    /// Returns the selected canonical Snapshot referenced by this input.
    pub fn snapshot(&self) -> &'a Snapshot {
        self.snapshot
    }

    /// Returns the canonical designated Artifact (RFC-0011), when current
    /// WorkDesignated evidence resolves one.
    pub fn designated(&self) -> Option<&ArtifactId> {
        self.designated.as_ref()
    }

    /// Returns the Current Continuation Surface resolved to canonical
    /// Artifacts (RFC-0013), when the latest valid declaration exists.
    pub fn continuation_surface(&self) -> Option<&[ArtifactId]> {
        self.continuation_surface.as_deref()
    }

    /// Returns state witnessed for `artifact` on its exact selected occurrence.
    pub fn observed_state(&self, artifact: &ArtifactId) -> Option<&ObservedState> {
        self.observed_states
            .iter()
            .find(|(candidate, _)| candidate == artifact)
            .map(|(_, state)| state)
    }
}

// ── RestorationComponent ─────────────────────────────────────────────────────

/// A required RestorationPlan component that derivation must establish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RestorationComponent {
    /// The canonical cognitive entry point (IS-0021 §25.2).
    ResumePoint,
    /// The immediate continuation following the Resume Point (IS-0021 §25.5).
    NextStep,
}

impl std::fmt::Display for RestorationComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestorationComponent::ResumePoint => write!(f, "Resume Point"),
            RestorationComponent::NextStep => write!(f, "Next Step"),
        }
    }
}

// ── MissingEvidence ──────────────────────────────────────────────────────────

/// A structured record of why a required component could not be established
/// from canonical evidence (IS-0021 §25.8).
///
/// The reason is canonical derivation output — it identifies the unmet
/// invariant or missing canonical input. It is never fabricated UI copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingEvidence {
    component: RestorationComponent,
    reason: String,
}

impl MissingEvidence {
    /// Constructs a missing-evidence record for `component`.
    pub fn new(component: RestorationComponent, reason: impl Into<String>) -> Self {
        Self {
            component,
            reason: reason.into(),
        }
    }

    /// Returns the component that could not be established.
    pub fn component(&self) -> RestorationComponent {
        self.component
    }

    /// Returns the canonical reason the component is missing.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

// ── InsufficientRestoration ──────────────────────────────────────────────────

/// The structured outcome produced when canonical evidence cannot satisfy a
/// required RestorationPlan invariant (IS-0021 §25.8).
///
/// Carries every component that *could* be derived plus a missing-evidence
/// record for every component that could not. Insufficiency is a normal and
/// explicit derivation outcome, never a guessed plan.
#[derive(Debug, Clone, PartialEq)]
pub struct InsufficientRestoration {
    workspace_id: WorkspaceId,
    resume_point: Option<ResumePoint>,
    resume_point_missing: Option<MissingEvidence>,
    context_chain: ContextChain,
    blockers: Vec<Blocker>,
    next_step_missing: MissingEvidence,
    /// The derived Continuation Surface of this Workspace (RFC-0013): the
    /// Artifacts of the Workspace referenced by the Current Continuation
    /// Surface, in canonical order. Carried even on an Insufficient outcome
    /// (RFC-0013 Gating Invariants: the surface does not gate completeness).
    continuation_surface: Vec<ArtifactId>,
}

impl InsufficientRestoration {
    /// Constructs an insufficient-outcome record.
    ///
    /// Exactly one of `resume_point` and `resume_point_missing` must be
    /// present (IS-0021 §25.2, §25.8). When a Resume Point is present it must
    /// belong to `workspace_id` (IS-0021 §25.7).
    pub fn new(
        workspace_id: WorkspaceId,
        resume_point: Option<ResumePoint>,
        resume_point_missing: Option<MissingEvidence>,
        context_chain: ContextChain,
        blockers: Vec<Blocker>,
        next_step_missing: MissingEvidence,
    ) -> Result<Self, RestorationError> {
        Self::new_with_continuation_surface(
            workspace_id,
            resume_point,
            resume_point_missing,
            context_chain,
            blockers,
            next_step_missing,
            Vec::new(),
        )
    }

    /// Constructs an insufficient-outcome record carrying the derived
    /// Continuation Surface of this Workspace (RFC-0013). The surface is
    /// derived state and is carried even when other components are
    /// insufficient; it never gates completeness.
    pub fn new_with_continuation_surface(
        workspace_id: WorkspaceId,
        resume_point: Option<ResumePoint>,
        resume_point_missing: Option<MissingEvidence>,
        context_chain: ContextChain,
        blockers: Vec<Blocker>,
        next_step_missing: MissingEvidence,
        continuation_surface: Vec<ArtifactId>,
    ) -> Result<Self, RestorationError> {
        match (&resume_point, &resume_point_missing) {
            (Some(point), None) => {
                if point.workspace_id() != &workspace_id {
                    return Err(RestorationError::ResumePointWorkspaceMismatch {
                        plan_workspace: workspace_id,
                        resume_point_workspace: point.workspace_id().clone(),
                    });
                }
            }
            (None, Some(_)) => {}
            (Some(_), Some(_)) => {
                return Err(RestorationError::InvalidInsufficientOutcome {
                    detail: "both a Resume Point and missing-evidence for the Resume Point \
                             were supplied; exactly one is required"
                        .to_string(),
                });
            }
            (None, None) => {
                return Err(RestorationError::InvalidInsufficientOutcome {
                    detail: "neither a Resume Point nor missing-evidence for the Resume Point \
                             was supplied; exactly one is required"
                        .to_string(),
                });
            }
        }

        Ok(Self {
            workspace_id,
            resume_point,
            resume_point_missing,
            context_chain,
            blockers,
            next_step_missing,
            continuation_surface,
        })
    }

    /// Returns the Workspace this outcome describes (IS-0021 §25.7).
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Returns the derived Resume Point, when canonical evidence established it.
    pub fn resume_point(&self) -> Option<&ResumePoint> {
        self.resume_point.as_ref()
    }

    /// Returns why the Resume Point could not be established, when it could not.
    pub fn resume_point_missing(&self) -> Option<&MissingEvidence> {
        self.resume_point_missing.as_ref()
    }

    /// Returns the derived Context Chain (IS-0021 §25.3).
    pub fn context_chain(&self) -> &ContextChain {
        &self.context_chain
    }

    /// Returns the derived Blockers (IS-0021 §25.4). Zero under the current model.
    pub fn blockers(&self) -> &[Blocker] {
        &self.blockers
    }

    /// Returns why the Next Step could not be established (IS-0021 §25.5).
    pub fn next_step_missing(&self) -> &MissingEvidence {
        &self.next_step_missing
    }

    /// Returns the derived Continuation Surface of this Workspace (RFC-0013),
    /// in canonical order. Empty when no valid declaration exists or when no
    /// declared member belongs to this Workspace.
    pub fn continuation_surface(&self) -> &[ArtifactId] {
        &self.continuation_surface
    }
}

// ── DerivationOutcome ────────────────────────────────────────────────────────

/// The canonical output of Restoration Derivation (IS-0021 §8, §25.8).
///
/// Exactly one outcome is produced per input:
///
/// - [`DerivationOutcome::Complete`] when every required component was
///   established and a valid [`RestorationPlan`] was constructed (§25.6);
/// - [`DerivationOutcome::Insufficient`] naming the components that could not
///   be established (§25.8).
#[derive(Debug, Clone, PartialEq)]
pub enum DerivationOutcome {
    /// A valid canonical RestorationPlan (IS-0021 §25.6).
    Complete(RestorationPlan),
    /// A structured record of what could and could not be derived (IS-0021 §25.8).
    Insufficient(InsufficientRestoration),
}

impl DerivationOutcome {
    /// Returns the Workspace this outcome describes.
    pub fn workspace_id(&self) -> &WorkspaceId {
        match self {
            DerivationOutcome::Complete(plan) => plan.workspace_id(),
            DerivationOutcome::Insufficient(insufficient) => insufficient.workspace_id(),
        }
    }

    /// Returns `true` when a full RestorationPlan was constructed.
    pub fn is_complete(&self) -> bool {
        matches!(self, DerivationOutcome::Complete(_))
    }

    /// Returns the derived Resume Point, when established.
    pub fn resume_point(&self) -> Option<&ResumePoint> {
        match self {
            DerivationOutcome::Complete(plan) => Some(plan.resume_point()),
            DerivationOutcome::Insufficient(insufficient) => insufficient.resume_point(),
        }
    }

    /// Returns why the Resume Point could not be established, when it could not.
    pub fn resume_point_missing(&self) -> Option<&MissingEvidence> {
        match self {
            DerivationOutcome::Complete(_) => None,
            DerivationOutcome::Insufficient(insufficient) => insufficient.resume_point_missing(),
        }
    }

    /// Returns the derived Context Chain.
    pub fn context_chain(&self) -> &ContextChain {
        match self {
            DerivationOutcome::Complete(plan) => plan.context_chain(),
            DerivationOutcome::Insufficient(insufficient) => insufficient.context_chain(),
        }
    }

    /// Returns the derived Blockers.
    pub fn blockers(&self) -> &[Blocker] {
        match self {
            DerivationOutcome::Complete(plan) => plan.blockers(),
            DerivationOutcome::Insufficient(insufficient) => insufficient.blockers(),
        }
    }

    /// Returns the derived Next Step, when canonical continuation evidence
    /// established it.
    pub fn next_step(&self) -> Option<&NextStep> {
        match self {
            DerivationOutcome::Complete(plan) => Some(plan.next_step()),
            DerivationOutcome::Insufficient(_) => None,
        }
    }

    /// Returns the derived Continuation Surface of this Workspace (RFC-0013),
    /// in canonical order.
    pub fn continuation_surface(&self) -> &[ArtifactId] {
        match self {
            DerivationOutcome::Complete(plan) => plan.continuation_surface(),
            DerivationOutcome::Insufficient(insufficient) => insufficient.continuation_surface(),
        }
    }

    /// Returns why the Next Step could not be established, when it could not.
    pub fn next_step_missing(&self) -> Option<&MissingEvidence> {
        match self {
            DerivationOutcome::Complete(_) => None,
            DerivationOutcome::Insufficient(insufficient) => Some(insufficient.next_step_missing()),
        }
    }
}

// ── derive_restoration_plan ──────────────────────────────────────────────────

/// Derives the canonical Restoration understanding for one Workspace/Snapshot
/// input (IS-0021 §21, §25).
///
/// # Behavior
///
/// - Deterministic and replayable (IS-0021 §25.9, §25.10).
/// - Consumes canonical input only (IS-0021 §25.1).
/// - Never mutates the Workspace, Snapshot, or any lower-layer object.
/// - Never executes anything.
/// - Produces [`DerivationOutcome::Insufficient`] rather than fabricating a
///   component the canonical evidence cannot establish (§25.8).
pub fn derive_restoration_plan(input: &RestorationInput<'_>) -> DerivationOutcome {
    let workspace = input.workspace();
    let snapshot = input.snapshot();
    let workspace_id = workspace.id().clone();

    // §25.1 / §25.2 — Candidate set: the distinct Artifacts represented
    // by the selected Snapshot's Attachment Set, each with the part it played.
    // Duplicate attachments of the same Artifact collapse to one candidate; the
    // candidate set is a set of Artifacts, and canonical Snapshot order is
    // preserved.
    let candidates = distinct_candidates(snapshot);

    // §25.4 — No canonical unresolved-work representation exists in the
    // current model, so zero Blockers are derived. No Blocker is invented.
    let blockers: Vec<Blocker> = Vec::new();

    // RFC-0013 Per-Workspace Consumption: the derived surface for this
    // Workspace is the intersection of the Current Continuation Surface with
    // the Workspace's canonical Artifacts, in canonical deterministic order
    // (ascending ArtifactId). Surface members outside this Workspace are
    // ignored. This is derivation output, never canonical state, and it never
    // gates completeness.
    let continuation_surface = derive_workspace_surface(input, &candidates);

    // §25.2 / §25.5 — Continuation evidence, strongest first. A designation is
    // the user's own statement of what they are continuing (RFC-0011) and
    // therefore outranks anything inferred from witnessed behaviour; the
    // Continuation role is the strongest thing the evidence itself says.
    // Neither is a ranking of candidates: each names one Artifact outright.
    let continuation = designated_candidate(input, &candidates)
        .map(|id| (id, NEXT_STEP_DESIGNATED_DESCRIPTION))
        .or_else(|| {
            sole_continuation(&candidates).map(|id| (id, NEXT_STEP_CONTINUATION_DESCRIPTION))
        });

    if let Some((target, next_step_description)) = continuation {
        // §25.3 — The context of the Resume Point is the other members that
        // played a part in this work, in the order of that part.
        let context_chain = derive_context_chain(&candidates, &target);
        let resume_point = resume_point(input, workspace_id.clone(), target);
        let next_step = NextStep::new(workspace_id.clone(), next_step_description)
            .expect("the canonical continuation descriptions are non-empty");
        let plan = RestorationPlan::new_with_continuation_surface(
            workspace_id,
            resume_point,
            context_chain,
            blockers,
            next_step,
            continuation_surface,
        )
        .expect(
            "a plan whose Resume Point and Next Step were derived for this Workspace is \
             valid (IS-0021 §25.6)",
        );
        return DerivationOutcome::Complete(plan);
    }

    let next_step_missing = MissingEvidence::new(
        RestorationComponent::NextStep,
        NEXT_STEP_MISSING_REASON,
    );

    // §25.3 — With no Resume Point derived, or a Resume Point that is the only
    // member there is, there is nothing for a Context Chain to contextualise.
    let context_chain = ContextChain::new(Vec::new())
        .expect("an empty Context Chain is always valid (IS-0021 CC-1)");

    match candidates.len() {
        0 => DerivationOutcome::Insufficient(
            InsufficientRestoration::new_with_continuation_surface(
                workspace_id,
                None,
                Some(MissingEvidence::new(
                    RestorationComponent::ResumePoint,
                    NO_CANDIDATE_REASON,
                )),
                context_chain,
                blockers,
                next_step_missing,
                continuation_surface,
            )
            .expect("an outcome with missing Resume Point evidence is valid"),
        ),
        1 => {
            // §25.2 — Exactly one eligible Artifact: it is the Resume
            // Point. No ranking, recency, or confidence rule is applied.
            let resume_point = resume_point(input, workspace_id, candidates[0].artifact_id.clone());
            DerivationOutcome::Insufficient(
                InsufficientRestoration::new_with_continuation_surface(
                    resume_point.workspace_id().clone(),
                    Some(resume_point),
                    None,
                    context_chain,
                    blockers,
                    next_step_missing,
                    continuation_surface,
                )
                .expect("an outcome with a derived Resume Point is valid"),
            )
        }
        _ => DerivationOutcome::Insufficient(
            InsufficientRestoration::new_with_continuation_surface(
                workspace_id,
                None,
                Some(MissingEvidence::new(
                    RestorationComponent::ResumePoint,
                    MULTIPLE_CANDIDATES_REASON,
                )),
                context_chain,
                blockers,
                next_step_missing,
                continuation_surface,
            )
            .expect("an outcome with missing Resume Point evidence is valid"),
        ),
    }
}

fn resume_point(
    input: &RestorationInput<'_>,
    workspace_id: WorkspaceId,
    artifact_id: ArtifactId,
) -> ResumePoint {
    ResumePoint::new_with_observed_state(
        workspace_id,
        artifact_id.clone(),
        input.observed_state(&artifact_id).cloned(),
    )
}

/// One distinct Artifact represented by the selected Snapshot, together with
/// the part it played in the body of work (IS-0021 §25.1).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    artifact_id: ArtifactId,
    role: ResourceRole,
}

/// Returns the designated canonical Artifact (RFC-0011) when the current
/// designation names one of this Snapshot's eligible candidates.
///
/// A designation naming an Artifact outside this Workspace establishes nothing
/// here (IS-0021 §25.7); it is another Workspace's evidence.
fn designated_candidate(
    input: &RestorationInput<'_>,
    candidates: &[Candidate],
) -> Option<ArtifactId> {
    let designated = input.designated()?;
    candidates
        .iter()
        .find(|candidate| &candidate.artifact_id == designated)
        .map(|candidate| candidate.artifact_id.clone())
}

/// Returns the Artifact the evidence names as where the work stopped: the sole
/// candidate carrying [`ResourceRole::Continuation`].
///
/// The Workspace layer's projection assigns that role to exactly one member of
/// a body of work, and it does so from the one piece of evidence that bears on
/// the question — which member the person attended most recently. Derivation
/// takes that at face value rather than re-deriving it, because derivation sees
/// Artifacts and Attachments and no longer has the witnessed times.
///
/// Two candidates carrying the role would mean the claim "this is where the work
/// stopped" was made twice, which distinguishes nothing; derivation then reports
/// no continuation evidence rather than choosing between them.
fn sole_continuation(candidates: &[Candidate]) -> Option<ArtifactId> {
    let mut found: Option<&Candidate> = None;
    for candidate in candidates {
        if candidate.role != ResourceRole::Continuation {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(candidate);
    }
    found.map(|candidate| candidate.artifact_id.clone())
}

/// Derives the Context Chain for a derived Resume Point (IS-0021 §25.3).
///
/// The chain is the candidates that played a part in this body of work, other
/// than the Resume Point itself, ordered by that part (most important first) and
/// then by canonical Snapshot order within a part. [`ResourceRole`] declares its
/// variants in importance order, so the ordering is the enum's own and not a
/// second judgement made here.
///
/// Candidates carrying [`ResourceRole::Context`] are excluded. Such a resource
/// was witnessed while the work was happening but nothing connects it to the
/// work beyond having been there, and CC-4 asks for the minimum context needed
/// to understand the Resume Point rather than the maximum that could be listed.
/// The Workspace keeps the complete membership set, so nothing is lost to
/// surfaces that want all of it.
fn derive_context_chain(candidates: &[Candidate], resume_point: &ArtifactId) -> ContextChain {
    let mut ordered: Vec<(ResourceRole, usize, ArtifactId)> = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| &candidate.artifact_id != resume_point)
        .filter(|(_, candidate)| candidate.role != ResourceRole::Context)
        .map(|(position, candidate)| (candidate.role, position, candidate.artifact_id.clone()))
        .collect();
    // Deterministic: (role, canonical Snapshot position) is a total order, since
    // duplicate Artifacts have already collapsed to one candidate each (CC-1).
    ordered.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    ContextChain::new(ordered.into_iter().map(|(_, _, id)| id).collect())
        .expect("candidates are distinct Artifacts, so the chain cannot repeat one (CC-2)")
}

/// Derives the per-Workspace Continuation Surface (RFC-0013): the set of
/// Artifacts restoring this body of work should actually open, in canonical
/// deterministic order (ascending ArtifactId).
///
/// - A valid declaration naming members of this Workspace is ground truth and is
///   used alone, intersected with the whole body of work's Attachment Set.
///   Declared members outside this Workspace are another Workspace's evidence and
///   are ignored here.
/// - A declaration naming nothing in this Workspace says nothing about it, and
///   is treated here exactly as no declaration at all.
/// - Absent a declaration that speaks about this Workspace, the surface is the
///   selected Snapshot's candidates whose [`ResourceRole`] opens on restore — the
///   places the work was happening in the sitting being returned to.
/// - A body of work with no such member yields an empty surface, explicit and
///   honest.
///
/// The surface never gates completeness; it is derivation output.
///
/// # Authority change: an undeclared surface (RFC-0013)
///
/// **The old rule.** The Continuation Surface came from an explicit
/// OBS-CONTINUATION-SURFACE declaration and from nothing else: "No valid
/// declaration → empty surface, explicit and honest."
///
/// **Why it prevented the product from working.** The surface is what
/// Restoration Execution opens. With no declaration the surface was empty, so
/// pressing Continue on a body of work Evo had understood perfectly well opened
/// nothing at all — the user had to first tell Evo where they were in order to
/// be taken back there, which is the one thing Evo exists to spare them.
/// Honesty was never the problem: the emptiness was truthful and useless.
/// Nothing had to be guessed, either, because by the time this runs the
/// Workspace layer has already recorded which members are the places the work
/// happens, as canonical Attachment roles derived from witnessed evidence.
///
/// **The new rule.** Absent a declaration the surface is the members whose role
/// says the work happens there ([`ResourceRole::opens_on_restore`]). A
/// declaration still supersedes it completely, because the user's judgement
/// outranks inference — including the inference that they wanted more opened
/// than they asked for. Roles are canonical state read as given, not a second
/// judgement made here, so this adds no new evidence and no new threshold.
///
/// # Authority change: a declaration that does not speak here (RFC-0013 §4)
///
/// **The old rule.** Per-Workspace Consumption intersected the one Current
/// Continuation Surface with each Workspace's Artifacts and used the result as
/// that Workspace's surface, whatever it came to — including nothing.
///
/// **Why it prevented the product from working.** A declared surface is also a
/// declared grouping of its members, so everything a person names as "where I
/// continue" belongs to one body of work by construction. Every *other* body of
/// work therefore intersects the declaration emptily. Under the old rule that
/// emptiness silenced them: saying where you continue in one piece of work
/// switched Continue off for all the rest of your work, permanently, and the
/// only repair would be to declare a surface in each of them — the explicit
/// declaration Evo exists to spare the user.
///
/// **The new rule.** A declaration is authoritative about the body of work it
/// names members of, and says nothing about any other. An empty intersection is
/// therefore silence, not an instruction, and this Workspace's surface is
/// derived from roles as it would be with no declaration in the log at all.
/// Nothing is guessed and no threshold is introduced: the test is whether the
/// user's own statement mentions this work.
///
/// # Conformance correction: a declaration is about the work, not the last sitting (RFC-0013 §4)
///
/// **What the code did.** Per-Workspace Consumption intersected the declared
/// surface with the *selected Snapshot's* candidates, because the Snapshot is what
/// the rest of the derivation is scoped to.
///
/// **What the contract says.** RFC-0013 specifies `surface(W) = { artifact ∈
/// artifacts(W) : … }`, and `artifacts(W)` is the Workspace's canonical Attachment
/// Set. This document was right and the code was wrong, so nothing was amended
/// here — unlike the two authority changes above, this was a conformance defect.
///
/// **Why it mattered.** A Snapshot is one sitting; a body of work is all of them.
/// A person who declares "continue from the brief, the notes and that commit" is
/// speaking about their work, not about whichever sitting happens to be most
/// recent — and a commit made on Tuesday is not in Thursday's sitting. Such a
/// member was silently dropped from the surface it was explicitly named into: Evo
/// took an unambiguous instruction and discarded part of it without saying so,
/// which §10 forbids outright ("the user's judgement outranks inference") and which
/// no user could diagnose, since the declaration was accepted and the resource
/// remained a visible member of the work.
///
/// **Corrected.** The declaration is intersected with the Workspace's canonical
/// Attachment Set — every Artifact in the body of work, across every sitting. The
/// intersection still confines a declaration to the work it speaks about, so
/// nothing leaks between bodies of work; it simply no longer confines it to a
/// single sitting of that work.
///
/// The undeclared fallback stays Snapshot-scoped, and for the opposite reason:
/// with no instruction to honour, the sitting being returned to is exactly what
/// "where I left off" means, and opening the union of every sitting would open
/// more than the minimum (§12, §16). Declared breadth is the user's choice;
/// inferred breadth is Evo's, and Evo's is the narrow one.
fn derive_workspace_surface(
    input: &RestorationInput<'_>,
    candidates: &[Candidate],
) -> Vec<ArtifactId> {
    // Declared intent, intersected with the whole body of work (RFC-0013
    // Per-Workspace Consumption, as amended). Used alone: what the person named
    // is what they asked to have opened, no more.
    let declared: Vec<ArtifactId> = match input.continuation_surface() {
        Some(surface) => distinct_attachments(input.workspace().attachments())
            .into_iter()
            .map(|member| member.artifact_id)
            .filter(|member| surface.contains(member))
            .collect(),
        None => Vec::new(),
    };

    let mut members: Vec<ArtifactId> = if declared.is_empty() {
        // No declaration speaks about this body of work: open the places the
        // work was happening in the sitting being returned to, and only those.
        candidates
            .iter()
            .filter(|candidate| candidate.role.opens_on_restore())
            .map(|candidate| candidate.artifact_id.clone())
            .collect()
    } else {
        declared
    };
    // Canonical deterministic order: ascending ArtifactId (RFC-0013
    // Per-Workspace Consumption).
    members.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    members
}

/// Returns the distinct Artifacts represented by the Snapshot's Attachment Set,
/// each with the part it played, preserving canonical Snapshot order
/// (IS-0021 §25.1, §25.2).
///
/// A Snapshot may attach the same Artifact more than once. Those attachments are
/// one Artifact making one claim about its part in the work, so they collapse to
/// the most important part claimed — the same rule the Workspace projection
/// applies when it builds the Attachment Set, and the only one that cannot lose
/// a Continuation.
fn distinct_candidates(snapshot: &Snapshot) -> Vec<Candidate> {
    distinct_attachments(snapshot.attachments())
}

/// The distinct Artifacts an Attachment Set represents, each with the most
/// important part it claims, preserving canonical order.
///
/// Used for a Snapshot's Attachment Set (one sitting) and for a Workspace's (the
/// whole body of work) alike: both are Attachment Sets, and the collapsing rule
/// is the same for both.
fn distinct_attachments(attachments: &[Attachment]) -> Vec<Candidate> {
    let mut distinct: Vec<Candidate> = Vec::new();
    for attachment in attachments {
        match distinct
            .iter_mut()
            .find(|candidate| &candidate.artifact_id == attachment.artifact_id())
        {
            Some(existing) => existing.role = existing.role.min(attachment.role()),
            None => distinct.push(Candidate {
                artifact_id: attachment.artifact_id().clone(),
                role: attachment.role(),
            }),
        }
    }
    distinct
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use evo_workspace::confidence::ConfidenceScore;
    use evo_workspace::lifecycle::WorkspaceLifecycle;

    use std::time::UNIX_EPOCH;

    fn attachment(id: &str) -> Attachment {
        attachment_as(id, ResourceRole::Supporting)
    }

    fn attachment_as(id: &str, role: ResourceRole) -> Attachment {
        Attachment::new(
            ArtifactId::new(id).unwrap(),
            ConfidenceScore::new(1.0).unwrap(),
            role,
        )
    }

    fn snapshot(attachments: Vec<Attachment>) -> Snapshot {
        Snapshot::new(UNIX_EPOCH, WorkspaceLifecycle::Active, attachments)
    }

    fn workspace(snapshots: Vec<Snapshot>) -> Workspace {
        let attachments = snapshots
            .last()
            .map(|snapshot| snapshot.attachments().to_vec())
            .unwrap_or_default();
        Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            attachments,
            snapshots,
        )
    }

    fn derive(workspace: &Workspace, snapshot: &Snapshot) -> DerivationOutcome {
        let input = RestorationInput::new(workspace, snapshot)
            .expect("fixture snapshot must belong to the fixture workspace");
        derive_restoration_plan(&input)
    }

    #[test]
    fn input_rejects_snapshot_outside_workspace_history() {
        let workspace = workspace(vec![snapshot(vec![attachment("artifact-a")])]);
        let foreign = snapshot(vec![attachment("artifact-x")]);

        let result = RestorationInput::new(&workspace, &foreign);
        assert!(matches!(
            result,
            Err(RestorationError::SnapshotNotPartOfWorkspace { .. })
        ));
    }

    #[test]
    fn resume_point_carries_only_directly_supplied_observed_state() {
        let snapshot = snapshot(vec![attachment("artifact-single")]);
        let workspace = workspace(vec![snapshot.clone()]);
        let state = ObservedState::new()
            .with_document_locator("/work/automation-x.ts")
            .with_insertion_line(37);
        let input = RestorationInput::new_with_observed_states(
            &workspace,
            &snapshot,
            None,
            None,
            vec![(ArtifactId::new("artifact-single").unwrap(), state.clone())],
        )
        .unwrap();

        let outcome = derive_restoration_plan(&input);
        assert_eq!(outcome.resume_point().unwrap().observed_state(), Some(&state));
    }

    #[test]
    fn missing_or_foreign_state_is_never_fabricated_for_resume_point() {
        let snapshot = snapshot(vec![attachment("artifact-single")]);
        let workspace = workspace(vec![snapshot.clone()]);
        let input = RestorationInput::new_with_observed_states(
            &workspace,
            &snapshot,
            None,
            None,
            vec![(
                ArtifactId::new("artifact-from-another-work").unwrap(),
                ObservedState::new().with_insertion_line(99),
            )],
        )
        .unwrap();

        let first = derive_restoration_plan(&input);
        let second = derive_restoration_plan(&input);
        assert_eq!(first, second, "state-aware derivation remains deterministic");
        assert_eq!(first.resume_point().unwrap().observed_state(), None);
    }

    // Fixture A — Sufficient evidence for the strongest plan the current
    // canonical model supports: a single eligible Artifact establishes
    // the Resume Point; Context is empty; Blockers are zero; the Next Step is
    // recorded as missing with the exact §25.5 reason.
    #[test]
    fn fixture_a_sufficient_evidence_derives_resume_point_and_names_missing_continuation() {
        let snapshot = snapshot(vec![attachment("artifact-single")]);
        let workspace = workspace(vec![snapshot.clone()]);

        let outcome = derive(&workspace, &snapshot);

        match &outcome {
            DerivationOutcome::Insufficient(insufficient) => {
                let resume_point = insufficient
                    .resume_point()
                    .expect("single eligible artifact must establish the Resume Point");
                assert_eq!(resume_point.workspace_id(), workspace.id());
                assert_eq!(resume_point.artifact_id().as_str(), "artifact-single");
                assert!(insufficient.resume_point_missing().is_none());

                // §25.3 — membership alone is never Context relevance.
                assert!(insufficient.context_chain().is_empty());
                // §25.4 — no canonical unresolved-work representation.
                assert!(insufficient.blockers().is_empty());

                // §25.5 — the missing canonical input is named exactly.
                let missing = insufficient.next_step_missing();
                assert_eq!(missing.component(), RestorationComponent::NextStep);
                assert!(missing.reason().contains("25.5"));
                assert!(missing.reason().contains("continuation"));
            }
            other => panic!("single-artifact evidence must yield the strongest derivable outcome, got {other:?}"),
        }

        assert_eq!(outcome.workspace_id(), workspace.id());
        assert!(!outcome.is_complete());
    }

    // Fixture B — Resume Point unresolved: Artifacts exist but no canonical
    // evidence distinguishes one of them as the Resume Point.
    #[test]
    fn fixture_b_multiple_artifacts_without_distinguishing_evidence_is_insufficient() {
        let snapshot = snapshot(vec![
            attachment("artifact-alpha"),
            attachment("artifact-beta"),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);

        let outcome = derive(&workspace, &snapshot);

        let DerivationOutcome::Insufficient(insufficient) = &outcome else {
            panic!("multiple eligible artifacts must yield an insufficient outcome");
        };

        assert!(insufficient.resume_point().is_none());
        let missing = insufficient
            .resume_point_missing()
            .expect("missing Resume Point must be recorded with evidence");
        assert_eq!(missing.component(), RestorationComponent::ResumePoint);
        assert!(missing.reason().contains("25.2"));
        assert!(missing.reason().contains("Multiple eligible Artifacts"));

        // The missing evidence is never papered over by picking an Artifact.
        assert_eq!(outcome.resume_point(), None);
    }

    // Fixture C — Context Chain: multiple Artifacts exist in the Workspace but
    // only canonical supporting relationships can include context. The chain
    // stays empty (§25.3). Also proves duplicate attachments of one Artifact
    // collapse to a single eligible candidate (§25.2 candidate set semantics).
    #[test]
    fn fixture_c_context_chain_does_not_include_workspace_members_and_duplicates_collapse() {
        // Two distinct Artifacts, no canonical relationship: chain stays empty.
        let multi = snapshot(vec![
            attachment("artifact-alpha"),
            attachment("artifact-beta"),
        ]);
        let workspace_multi = workspace(vec![multi.clone()]);
        let outcome_multi = derive(&workspace_multi, &multi);
        assert!(outcome_multi.context_chain().is_empty());

        // Two attachments of the SAME Artifact: one eligible candidate, so the
        // Resume Point is derivable from the single Artifact.
        let duplicated = snapshot(vec![
            attachment("artifact-shared"),
            attachment("artifact-shared"),
        ]);
        let workspace_dup = workspace(vec![duplicated.clone()]);
        let outcome_dup = derive(&workspace_dup, &duplicated);
        let resume_point = outcome_dup
            .resume_point()
            .expect("one distinct eligible Artifact establishes the Resume Point");
        assert_eq!(resume_point.artifact_id().as_str(), "artifact-shared");
    }

    // Fixture D — Blockers: zero under the current model, and never invented
    // from runtime or machine state.
    #[test]
    fn fixture_d_blockers_are_zero_without_canonical_unresolved_work() {
        for attachments in [
            vec![attachment("artifact-only")],
            vec![attachment("artifact-a"), attachment("artifact-b")],
            vec![],
        ] {
            let snapshot = snapshot(attachments);
            let workspace = workspace(vec![snapshot.clone()]);
            let outcome = derive(&workspace, &snapshot);
            assert!(outcome.blockers().is_empty());
        }
    }

    // Fixture E — Determinism: identical canonical inputs produce identical
    // derivation outcomes (IS-0021 §25.9).
    #[test]
    fn fixture_e_derivation_is_deterministic() {
        let snapshot = snapshot(vec![
            attachment("artifact-alpha"),
            attachment("artifact-beta"),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);

        let first = derive(&workspace, &snapshot);
        let second = derive(&workspace, &snapshot);

        assert_eq!(first, second);
        assert_eq!(first.resume_point(), second.resume_point());
        assert_eq!(first.resume_point_missing(), second.resume_point_missing());
        assert_eq!(first.next_step_missing(), second.next_step_missing());
    }

    // Fixture F — Immutability: derivation never changes the Workspace or
    // Snapshot (IS-0021 §25.10, §18).
    #[test]
    fn fixture_f_derivation_never_mutates_workspace_or_snapshot() {
        let snapshot = snapshot(vec![attachment("artifact-immutable")]);
        let workspace = workspace(vec![snapshot.clone()]);
        let original_workspace = workspace.clone();
        let original_snapshot = snapshot.clone();

        let _ = derive(&workspace, &snapshot);

        assert_eq!(workspace, original_workspace);
        assert_eq!(snapshot, original_snapshot);
        assert_eq!(workspace.snapshots(), original_workspace.snapshots());
        assert_eq!(snapshot.attachments(), original_snapshot.attachments());
    }

    // Fixture G — Designation (RFC-0011): the current designation's resolved
    // Artifact resolves the multi-candidate insufficiency and establishes the
    // Next Step (§25.2, §25.5).
    #[test]
    fn fixture_g_designation_resolves_multi_candidate_ambiguity() {
        let snapshot = snapshot(vec![
            attachment("artifact-alpha"),
            attachment("artifact-beta"),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        let designated = ArtifactId::new("artifact-beta").unwrap();
        let input = RestorationInput::new_with_designated(&workspace, &snapshot, Some(designated))
            .expect("valid input");

        let outcome = derive_restoration_plan(&input);

        let DerivationOutcome::Complete(plan) = &outcome else {
            panic!("designation must establish a Complete plan, got {outcome:?}");
        };
        assert_eq!(
            plan.resume_point().artifact_id().as_str(),
            "artifact-beta"
        );
        assert_eq!(plan.next_step().workspace_id(), workspace.id());
        assert!(plan.next_step().description().contains("designated"));
        // §25.3 as amended — the other member played a part in the work, so it
        // is context for the Resume Point. Under the old rule this chain was
        // required to be empty, which left Restoration with a destination and
        // nothing to orient the person once they arrived.
        let labels: Vec<&str> = plan
            .context_chain()
            .artifacts()
            .iter()
            .map(ArtifactId::as_str)
            .collect();
        assert_eq!(labels, vec!["artifact-alpha"]);
        assert!(plan.blockers().is_empty());
        assert!(outcome.is_complete());
    }

    // Fixture H — Designation referencing an Artifact outside the Workspace
    // establishes nothing for this Workspace (IS-0021 §25.7): the multi
    // candidate case stays insufficient.
    #[test]
    fn fixture_h_designation_outside_workspace_is_insufficient() {
        let snapshot = snapshot(vec![
            attachment("artifact-alpha"),
            attachment("artifact-beta"),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        let foreign = ArtifactId::new("artifact-foreign").unwrap();
        let input =
            RestorationInput::new_with_designated(&workspace, &snapshot, Some(foreign))
                .expect("valid input");

        let outcome = derive_restoration_plan(&input);
        assert!(!outcome.is_complete());
        assert!(outcome.next_step().is_none());
        assert!(outcome.resume_point().is_none());
        let missing = outcome
            .resume_point_missing()
            .expect("missing Resume Point evidence must be recorded");
        assert!(missing.reason().contains("25.2"));
    }

    // Fixture I — Single-Artifact Workspace with a designation pointing at
    // that Artifact: the designation establishes the Resume Point and the
    // Next Step (a Complete plan), strengthening the single-artifact case.
    #[test]
    fn fixture_i_designation_completes_single_artifact_workspace() {
        let snapshot = snapshot(vec![attachment("artifact-single")]);
        let workspace = workspace(vec![snapshot.clone()]);
        let designated = ArtifactId::new("artifact-single").unwrap();
        let input = RestorationInput::new_with_designated(&workspace, &snapshot, Some(designated))
            .expect("valid input");

        let outcome = derive_restoration_plan(&input);
        assert!(outcome.is_complete());
        assert_eq!(
            outcome.resume_point().unwrap().artifact_id().as_str(),
            "artifact-single"
        );
        assert_eq!(
            outcome.next_step().unwrap().workspace_id(),
            workspace.id()
        );
    }

    // Fixture J — Supersession semantics: with a designation pointing at one
    // candidate, a different designation pointing elsewhere leaves this
    // Workspace without continuation evidence.
    #[test]
    fn fixture_j_designation_targets_exactly_the_designated_artifact() {
        let snapshot = snapshot(vec![
            attachment("artifact-alpha"),
            attachment("artifact-beta"),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);

        let designated = ArtifactId::new("artifact-alpha").unwrap();
        let input =
            RestorationInput::new_with_designated(&workspace, &snapshot, Some(designated))
                .expect("valid input");
        let outcome = derive_restoration_plan(&input);
        assert!(outcome.is_complete());
        assert_eq!(
            outcome.resume_point().unwrap().artifact_id().as_str(),
            "artifact-alpha"
        );
    }

    // ── The Continuation role (IS-0021 §25.2, §25.3, §25.5 as amended) ───────

    /// The behaviour the old rule made impossible: a body of work with several
    /// members establishes where it resumes, without a designation.
    #[test]
    fn the_continuation_role_establishes_the_resume_point_and_the_next_step() {
        let snapshot = snapshot(vec![
            attachment_as("artifact-notes", ResourceRole::Reference),
            attachment_as("artifact-draft", ResourceRole::Continuation),
            attachment_as("artifact-terminal", ResourceRole::Primary),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);

        let outcome = derive(&workspace, &snapshot);

        let DerivationOutcome::Complete(plan) = &outcome else {
            panic!("the Continuation role establishes a Complete plan, got {outcome:?}");
        };
        assert_eq!(plan.resume_point().artifact_id().as_str(), "artifact-draft");
        assert_eq!(plan.resume_point().workspace_id(), workspace.id());
        assert_eq!(plan.next_step().workspace_id(), workspace.id());
        assert!(plan.next_step().description().contains("last working in"));
        assert!(plan.blockers().is_empty());
    }

    /// §25.3 — the chain is ordered by the part each member played, and the
    /// Resume Point is not context for itself.
    #[test]
    fn the_context_chain_is_the_other_members_in_order_of_the_part_they_played() {
        let snapshot = snapshot(vec![
            // Deliberately out of importance order in the Snapshot.
            attachment_as("artifact-page", ResourceRole::Reference),
            attachment_as("artifact-draft", ResourceRole::Continuation),
            attachment_as("artifact-notes", ResourceRole::Supporting),
            attachment_as("artifact-terminal", ResourceRole::Primary),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);

        let chain = derive(&workspace, &snapshot).context_chain().artifacts().to_vec();
        let labels: Vec<&str> = chain.iter().map(ArtifactId::as_str).collect();
        assert_eq!(
            labels,
            vec!["artifact-terminal", "artifact-notes", "artifact-page"],
            "context is ordered by the part each member played, and the Resume Point \
             is not its own context"
        );
    }

    /// §25.3 / CC-4 — a resource that was merely present while the work
    /// happened is not context *for* the Resume Point.
    #[test]
    fn merely_present_members_are_not_context_for_the_resume_point() {
        let snapshot = snapshot(vec![
            attachment_as("artifact-draft", ResourceRole::Continuation),
            attachment_as("artifact-notes", ResourceRole::Supporting),
            attachment_as("artifact-passing", ResourceRole::Context),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);

        let outcome = derive(&workspace, &snapshot);
        let labels: Vec<&str> = outcome
            .context_chain()
            .artifacts()
            .iter()
            .map(ArtifactId::as_str)
            .collect();
        assert_eq!(labels, vec!["artifact-notes"]);
        // Nothing is lost: the Workspace still attaches everything.
        assert_eq!(workspace.attachments().len(), 3);
    }

    /// §10 — the user's judgement outranks inference. A designation moves the
    /// Resume Point even when the evidence named a different member.
    #[test]
    fn a_designation_outranks_the_continuation_role() {
        let snapshot = snapshot(vec![
            attachment_as("artifact-draft", ResourceRole::Continuation),
            attachment_as("artifact-notes", ResourceRole::Supporting),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        let designated = ArtifactId::new("artifact-notes").unwrap();
        let input = RestorationInput::new_with_designated(&workspace, &snapshot, Some(designated))
            .expect("valid input");

        let outcome = derive_restoration_plan(&input);
        let DerivationOutcome::Complete(plan) = &outcome else {
            panic!("a designation establishes a Complete plan, got {outcome:?}");
        };
        assert_eq!(plan.resume_point().artifact_id().as_str(), "artifact-notes");
        assert!(plan.next_step().description().contains("designated"));
        // The member the evidence named becomes context rather than vanishing.
        let labels: Vec<&str> = plan
            .context_chain()
            .artifacts()
            .iter()
            .map(ArtifactId::as_str)
            .collect();
        assert_eq!(labels, vec!["artifact-draft"]);
    }

    /// A designation naming an Artifact outside this Workspace establishes
    /// nothing here (§25.7), and must not suppress the evidence that does.
    #[test]
    fn a_foreign_designation_leaves_the_continuation_role_standing() {
        let snapshot = snapshot(vec![
            attachment_as("artifact-draft", ResourceRole::Continuation),
            attachment_as("artifact-notes", ResourceRole::Supporting),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        let foreign = ArtifactId::new("artifact-elsewhere").unwrap();
        let input = RestorationInput::new_with_designated(&workspace, &snapshot, Some(foreign))
            .expect("valid input");

        let outcome = derive_restoration_plan(&input);
        assert!(outcome.is_complete());
        assert_eq!(
            outcome.resume_point().unwrap().artifact_id().as_str(),
            "artifact-draft"
        );
    }

    /// The claim "this is where the work stopped" made twice distinguishes
    /// nothing, so derivation reports missing evidence instead of choosing.
    #[test]
    fn two_continuation_roles_are_no_continuation_evidence() {
        let snapshot = snapshot(vec![
            attachment_as("artifact-alpha", ResourceRole::Continuation),
            attachment_as("artifact-beta", ResourceRole::Continuation),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);

        let outcome = derive(&workspace, &snapshot);
        assert!(!outcome.is_complete());
        assert!(outcome.resume_point().is_none());
        assert!(
            outcome
                .resume_point_missing()
                .expect("missing evidence is recorded")
                .reason()
                .contains("Continuation role")
        );
    }

    /// Duplicate attachments of one Artifact are one claim about one Artifact,
    /// and collapsing them must not discard the strongest part claimed.
    #[test]
    fn duplicate_attachments_collapse_to_the_most_important_part() {
        for roles in [
            [ResourceRole::Continuation, ResourceRole::Reference],
            [ResourceRole::Reference, ResourceRole::Continuation],
        ] {
            let snapshot = snapshot(vec![
                attachment_as("artifact-draft", roles[0]),
                attachment_as("artifact-draft", roles[1]),
                attachment_as("artifact-notes", ResourceRole::Supporting),
            ]);
            let workspace = workspace(vec![snapshot.clone()]);

            let outcome = derive(&workspace, &snapshot);
            assert!(outcome.is_complete(), "roles {roles:?} lost the Continuation");
            assert_eq!(
                outcome.resume_point().unwrap().artifact_id().as_str(),
                "artifact-draft"
            );
        }
    }

    /// Ordering may not depend on the order attachments happen to arrive in
    /// (IS-0021 §25.9).
    #[test]
    fn the_context_chain_does_not_depend_on_attachment_order() {
        let forward = snapshot(vec![
            attachment_as("artifact-draft", ResourceRole::Continuation),
            attachment_as("artifact-terminal", ResourceRole::Primary),
            attachment_as("artifact-notes", ResourceRole::Supporting),
        ]);
        let reversed = snapshot(vec![
            attachment_as("artifact-notes", ResourceRole::Supporting),
            attachment_as("artifact-terminal", ResourceRole::Primary),
            attachment_as("artifact-draft", ResourceRole::Continuation),
        ]);
        let forward_workspace = workspace(vec![forward.clone()]);
        let reversed_workspace = workspace(vec![reversed.clone()]);

        let first = derive(&forward_workspace, &forward);
        let second = derive(&reversed_workspace, &reversed);
        assert_eq!(
            first.context_chain().artifacts(),
            second.context_chain().artifacts()
        );
        assert_eq!(
            first.resume_point().unwrap().artifact_id(),
            second.resume_point().unwrap().artifact_id()
        );
    }

    // ── RFC-0013 Continuation Surface ────────────────────────────────────────

    fn derive_with_surface(
        workspace: &Workspace,
        snapshot: &Snapshot,
        designated: Option<ArtifactId>,
        surface: Option<Vec<ArtifactId>>,
    ) -> DerivationOutcome {
        let input = RestorationInput::new_with_continuation_surface(
            workspace,
            snapshot,
            designated,
            surface,
        )
        .expect("fixture snapshot must belong to the fixture workspace");
        derive_restoration_plan(&input)
    }

    // RFC-0013 Per-Workspace Consumption: a designated Artifact that is also
    // declared in the surface yields a Complete plan carrying the surface
    // (the intersection of the declared surface with the Workspace's
    // Artifacts, in ascending ArtifactId order).
    #[test]
    fn surface_members_are_intersected_with_the_workspace() {
        let snapshot = snapshot(vec![
            attachment("artifact-zeta"),
            attachment("artifact-alpha"),
            attachment("artifact-beta"),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        let designated = ArtifactId::new("artifact-beta").unwrap();
        let surface = vec![
            ArtifactId::new("artifact-beta").unwrap(),
            ArtifactId::new("artifact-alpha").unwrap(),
            ArtifactId::new("artifact-foreign").unwrap(), // outside the Workspace
        ];

        let outcome = derive_with_surface(&workspace, &snapshot, Some(designated), Some(surface));

        let DerivationOutcome::Complete(plan) = &outcome else {
            panic!("designation must establish a Complete plan, got {outcome:?}");
        };
        // The declared foreign Artifact never leaks into this Workspace's
        // surface; members are in canonical order (ascending ArtifactId).
        assert_eq!(plan.continuation_surface().len(), 2);
        assert_eq!(
            plan.continuation_surface()[0].as_str(),
            "artifact-alpha"
        );
        assert_eq!(
            plan.continuation_surface()[1].as_str(),
            "artifact-beta"
        );
        // The surface never changes Resume Point / Next Step semantics.
        assert_eq!(
            plan.resume_point().artifact_id().as_str(),
            "artifact-beta"
        );
        assert!(plan.next_step().description().contains("designated"));
    }

    // RFC-0013 §4, as amended: the intersection is with the body of work, not
    // with the sitting being returned to. A person who names a resource they
    // last touched two sittings ago has still named it, and Evo may not quietly
    // drop part of an instruction it accepted.
    #[test]
    fn a_declared_member_from_an_earlier_sitting_stays_on_the_surface() {
        let earlier = snapshot(vec![
            attachment_as("artifact-brief", ResourceRole::Primary),
            attachment_as("artifact-commit", ResourceRole::Reference),
        ]);
        let latest = snapshot(vec![attachment_as("artifact-brief", ResourceRole::Primary)]);
        // The body of work is every sitting; the commit is a member of it even
        // though the most recent sitting never touched it.
        let workspace = Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![
                attachment_as("artifact-brief", ResourceRole::Primary),
                attachment_as("artifact-commit", ResourceRole::Reference),
            ],
            vec![earlier, latest.clone()],
        );
        let surface = vec![
            ArtifactId::new("artifact-brief").unwrap(),
            ArtifactId::new("artifact-commit").unwrap(),
        ];

        let outcome = derive_with_surface(&workspace, &latest, None, Some(surface));

        assert_eq!(
            outcome.continuation_surface(),
            [
                ArtifactId::new("artifact-brief").unwrap(),
                ArtifactId::new("artifact-commit").unwrap()
            ],
            "both declared members survive; the commit is not dropped for being \
             absent from the latest sitting"
        );
    }

    // The undeclared fallback is deliberately the other scope: with no
    // instruction to honour, "where I left off" is the sitting being returned
    // to, and opening every sitting's places of work would exceed the minimum
    // (§12, §16).
    #[test]
    fn an_undeclared_surface_stays_within_the_sitting_being_returned_to() {
        let earlier = snapshot(vec![attachment_as("artifact-old-primary", ResourceRole::Primary)]);
        let latest = snapshot(vec![attachment_as("artifact-now-primary", ResourceRole::Primary)]);
        let workspace = Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![
                attachment_as("artifact-old-primary", ResourceRole::Primary),
                attachment_as("artifact-now-primary", ResourceRole::Primary),
            ],
            vec![earlier, latest.clone()],
        );

        let outcome = derive_with_surface(&workspace, &latest, None, None);

        assert_eq!(
            outcome.continuation_surface(),
            [ArtifactId::new("artifact-now-primary").unwrap()],
            "the sitting being returned to is what opens, not every sitting's primaries"
        );
    }

    // RFC-0013 Insufficiency: the derived surface is carried even on an
    // Insufficient outcome (it does not gate completeness).
    #[test]
    fn insufficient_outcome_carries_the_derived_surface() {
        let snapshot = snapshot(vec![
            attachment("artifact-alpha"),
            attachment("artifact-beta"),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        let surface = vec![ArtifactId::new("artifact-beta").unwrap()];

        // No designation: the plan is insufficient (multi-candidate Resume
        // Point), but the surface is still derived.
        let outcome = derive_with_surface(&workspace, &snapshot, None, Some(surface));
        assert!(!outcome.is_complete());
        assert_eq!(outcome.continuation_surface().len(), 1);
        assert_eq!(
            outcome.continuation_surface()[0].as_str(),
            "artifact-beta"
        );
    }

    // AMENDED RULE (RFC-0013). This test previously read
    // `no_declaration_means_empty_surface` and asserted that without a
    // declaration nothing is ever opened. Under the amended rule the surface
    // falls back to the members whose role says the work happens there, so
    // pressing Continue works without the user first having to say where they
    // were (§9). Emptiness is still honest when no member is such a place —
    // that guarantee is kept in the test below.
    #[test]
    fn no_declaration_opens_the_places_the_work_happens() {
        let snapshot = snapshot(vec![
            attachment_as("artifact-draft", ResourceRole::Continuation),
            attachment_as("artifact-terminal", ResourceRole::Primary),
            attachment_as("artifact-notes", ResourceRole::Supporting),
            attachment_as("artifact-spec", ResourceRole::Reference),
            attachment_as("artifact-music", ResourceRole::Context),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);

        let outcome = derive_with_surface(&workspace, &snapshot, None, None);

        // The Continuation and the Primary open; the Supporting, Reference and
        // Context members stay remembered and closed (§11, §12, §16).
        assert_eq!(
            outcome
                .continuation_surface()
                .iter()
                .map(ArtifactId::as_str)
                .collect::<Vec<_>>(),
            vec!["artifact-draft", "artifact-terminal"]
        );
        // Everything the Workspace holds is still known — the surface narrows
        // what opens, not what is remembered.
        assert_eq!(outcome.context_chain().artifacts().len(), 3);
    }

    // RFC-0013 Insufficiency: a body of work with no member that is a place the
    // work happens opens nothing. The surface is empty because the evidence
    // says so, never guessed from membership, recency, or anything else.
    #[test]
    fn a_body_of_work_with_no_place_to_continue_opens_nothing() {
        let snapshot = snapshot(vec![
            attachment_as("artifact-alpha", ResourceRole::Supporting),
            attachment_as("artifact-beta", ResourceRole::Reference),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        let designated = ArtifactId::new("artifact-beta").unwrap();

        let outcome = derive_with_surface(&workspace, &snapshot, Some(designated), None);
        assert!(outcome.is_complete());
        assert!(outcome.continuation_surface().is_empty());
    }

    // RFC-0013 / §10: an explicit declaration is ground truth and supersedes
    // the roles completely, including by opening less than the roles would.
    #[test]
    fn a_declaration_supersedes_the_derived_surface() {
        let snapshot = snapshot(vec![
            attachment_as("artifact-draft", ResourceRole::Continuation),
            attachment_as("artifact-terminal", ResourceRole::Primary),
            attachment_as("artifact-notes", ResourceRole::Supporting),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        // The person says: take me back to the notes, and only the notes —
        // even though the derived roles would have opened two other members.
        let surface = vec![ArtifactId::new("artifact-notes").unwrap()];

        let outcome = derive_with_surface(&workspace, &snapshot, None, Some(surface));

        assert_eq!(
            outcome
                .continuation_surface()
                .iter()
                .map(ArtifactId::as_str)
                .collect::<Vec<_>>(),
            vec!["artifact-notes"]
        );
        // The declaration changes what opens, not where the work stopped.
        assert_eq!(
            outcome.resume_point().unwrap().artifact_id().as_str(),
            "artifact-draft"
        );
    }

    // RFC-0013 Boundary: a surface never changes the Resume Point, Next Step,
    // Context Chain, or Blockers; it is additive derived state.
    #[test]
    fn surface_does_not_change_continuation_semantics() {
        let snapshot = snapshot(vec![attachment("artifact-single")]);
        let workspace = workspace(vec![snapshot.clone()]);
        let designated = ArtifactId::new("artifact-single").unwrap();
        let surface = vec![ArtifactId::new("artifact-single").unwrap()];

        let with_surface =
            derive_with_surface(&workspace, &snapshot, Some(designated.clone()), Some(surface));
        let without_surface =
            derive_with_surface(&workspace, &snapshot, Some(designated), None);

        let DerivationOutcome::Complete(plan_with) = &with_surface else {
            panic!("designation must establish a Complete plan");
        };
        let DerivationOutcome::Complete(plan_without) = &without_surface else {
            panic!("designation must establish a Complete plan");
        };
        assert_eq!(
            plan_with.resume_point(),
            plan_without.resume_point()
        );
        assert_eq!(plan_with.context_chain(), plan_without.context_chain());
        assert_eq!(plan_with.blockers(), plan_without.blockers());
        assert_eq!(plan_with.next_step(), plan_without.next_step());
        assert_eq!(plan_with.continuation_surface().len(), 1);
        // The sole member of this fixture is Supporting — not a place the work
        // happens — so with no declaration nothing opens. Either way the rest
        // of the plan is identical, which is what this test exists to prove.
        assert!(plan_without.continuation_surface().is_empty());
    }

    // RFC-0013 §4, as amended: a declaration is authoritative about the body of
    // work it names members of and silent about every other one. Since a
    // declared surface also declares its members into one body of work, this is
    // the ordinary case for all the person's *other* work — it must keep the
    // continuation its own evidence establishes.
    #[test]
    fn a_declaration_naming_nothing_here_leaves_this_work_its_own_continuation() {
        let snapshot = snapshot(vec![
            attachment_as("artifact-here-primary", ResourceRole::Primary),
            attachment_as("artifact-here-reference", ResourceRole::Reference),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        // The person declared where they continue — in some other body of work.
        let elsewhere = vec![ArtifactId::new("artifact-other-work").unwrap()];

        let spoken_elsewhere =
            derive_with_surface(&workspace, &snapshot, None, Some(elsewhere));
        let never_spoken = derive_with_surface(&workspace, &snapshot, None, None);

        assert_eq!(
            spoken_elsewhere.continuation_surface(),
            [ArtifactId::new("artifact-here-primary").unwrap()],
            "the place this work happens still opens"
        );
        assert_eq!(
            spoken_elsewhere.continuation_surface(),
            never_spoken.continuation_surface(),
            "a statement about other work is silence here, not an empty instruction"
        );
    }

    // RFC-0013 Determinism: identical inputs produce identical surfaces and
    // identical outcomes.
    #[test]
    fn surface_derivation_is_deterministic() {
        let snapshot = snapshot(vec![
            attachment("artifact-zeta"),
            attachment("artifact-alpha"),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        let designated = ArtifactId::new("artifact-zeta").unwrap();
        let surface = vec![ArtifactId::new("artifact-alpha").unwrap()];

        let first = derive_with_surface(&workspace, &snapshot, Some(designated.clone()), Some(surface.clone()));
        let second = derive_with_surface(&workspace, &snapshot, Some(designated), Some(surface));
        assert_eq!(first, second);
        assert_eq!(first.continuation_surface(), second.continuation_surface());
    }

    #[test]
    fn invalid_insufficient_outcome_combinations_are_rejected() {
        let wid = WorkspaceId::new();
        let chain = ContextChain::new(vec![]).unwrap();
        let missing = MissingEvidence::new(RestorationComponent::NextStep, "reason");

        // Neither Resume Point nor missing evidence.
        assert!(matches!(
            InsufficientRestoration::new(wid.clone(), None, None, chain.clone(), vec![], missing.clone()),
            Err(RestorationError::InvalidInsufficientOutcome { .. })
        ));

        // Both Resume Point and missing evidence.
        let point = ResumePoint::new(wid.clone(), ArtifactId::new("artifact-rp").unwrap());
        assert!(matches!(
            InsufficientRestoration::new(
                wid.clone(),
                Some(point),
                Some(missing.clone()),
                chain.clone(),
                vec![],
                missing,
            ),
            Err(RestorationError::InvalidInsufficientOutcome { .. })
        ));

        // Resume Point from a different Workspace.
        let point = ResumePoint::new(WorkspaceId::new(), ArtifactId::new("artifact-rp").unwrap());
        assert!(matches!(
            InsufficientRestoration::new(wid, Some(point), None, chain, vec![], MissingEvidence::new(RestorationComponent::NextStep, "reason")),
            Err(RestorationError::ResumePointWorkspaceMismatch { .. })
        ));
    }
}
