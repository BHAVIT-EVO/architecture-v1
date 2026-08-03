//! Restoration Plan.
//!
//! A `RestorationPlan` is the canonical computational object produced by the
//! Restoration layer.
//!
//! It represents one coherent strategy for resuming one Workspace (IS-0014 §4).
//!
//! # IS-0014 Invariants
//!
//! - RP-1: Derived exclusively from canonical lower-layer computational objects.
//! - RP-2: Preserves Workspace identity. WorkspaceId SHALL NOT change during Restoration.
//! - RP-3: SHALL NOT modify Workspace understanding.
//! - RP-4: Construction SHALL be deterministic.
//! - RM-5: Restoration SHALL compute. It SHALL NOT execute.
//! - RM-8: One Workspace SHALL produce one Restoration Plan.
//!
//! # Structure
//!
//! A `RestorationPlan` contains exactly (IS-0014 §4):
//!
//! - one `WorkspaceId` (the Workspace being restored);
//! - one `ResumePoint`;
//! - one `ContextChain`;
//! - zero or more `Blocker` values;
//! - one `NextStep`.
//!
//! Construction enforces workspace consistency across all components:
//!
//! - `ResumePoint::workspace_id` MUST match the plan's `WorkspaceId` (RSP-2, RP-2).
//! - `NextStep::workspace_id` MUST match the plan's `WorkspaceId` (NS-1).
//!
//! A `RestorationPlan` is immutable after construction.
//!
//! # Non-Responsibilities
//!
//! - Does **not** execute restoration (RM-5).
//! - Does **not** launch applications.
//! - Does **not** interact with the operating system.
//! - Does **not** modify Workspace understanding (RP-3).
//! - Does **not** modify Observations or Artifact Identity (RM-2, RM-3).

use evo_workspace::WorkspaceId;

use crate::blocker::Blocker;
use crate::context_chain::ContextChain;
use crate::errors::RestorationError;
use crate::next_step::NextStep;
use crate::resume_point::ResumePoint;

// ── RestorationPlan ───────────────────────────────────────────────────────────

/// The canonical computational object produced by the Restoration layer.
///
/// Represents one coherent strategy for resuming one Workspace (IS-0014 §4).
///
/// # Invariants
///
/// - Corresponds to exactly one Workspace (RP-2, RM-8).
/// - Contains exactly one `ResumePoint` (RSP-1).
/// - Contains exactly one `ContextChain`.
/// - Contains zero or more `Blocker` values (BL-1).
/// - Contains exactly one `NextStep`.
/// - `ResumePoint` workspace matches plan workspace (RSP-2, RP-2).
/// - `NextStep` workspace matches plan workspace (NS-1).
/// - Immutable after construction.
///
/// # Non-Responsibilities
///
/// - Does **not** execute restoration (RM-5).
/// - Does **not** modify Workspace understanding (RP-3).
/// - Does **not** modify Observations (RM-2).
/// - Does **not** modify Artifact Identity (RM-3).
/// - Does **not** invoke Replay (IS-0014 §10).
#[derive(Debug, Clone, PartialEq)]
pub struct RestorationPlan {
    /// The Workspace this plan restores (RP-2, RM-8).
    workspace_id: WorkspaceId,

    /// The canonical cognitive entry point (RSP-1).
    resume_point: ResumePoint,

    /// The minimum ordered supporting context for the Resume Point.
    context_chain: ContextChain,

    /// Unresolved conditions preventing immediate continuation (BL-1).
    blockers: Vec<Blocker>,

    /// The immediate continuation action following the Resume Point.
    next_step: NextStep,
}

impl RestorationPlan {
    /// Constructs an immutable `RestorationPlan`.
    ///
    /// # Errors
    ///
    /// - `RestorationError::ResumePointWorkspaceMismatch` if `resume_point.workspace_id()`
    ///   does not match `workspace_id` (RSP-2, RP-2).
    /// - `RestorationError::NextStepWorkspaceMismatch` if `next_step.workspace_id()`
    ///   does not match `workspace_id` (NS-1).
    ///
    /// # Guarantees
    ///
    /// - The plan corresponds to exactly one Workspace (RM-8).
    /// - All components belong to the same Workspace.
    /// - Immutable after construction.
    pub fn new(
        workspace_id: WorkspaceId,
        resume_point: ResumePoint,
        context_chain: ContextChain,
        blockers: Vec<Blocker>,
        next_step: NextStep,
    ) -> Result<Self, RestorationError> {
        if resume_point.workspace_id() != &workspace_id {
            return Err(RestorationError::ResumePointWorkspaceMismatch {
                plan_workspace: workspace_id,
                resume_point_workspace: resume_point.workspace_id().clone(),
            });
        }

        if next_step.workspace_id() != &workspace_id {
            return Err(RestorationError::NextStepWorkspaceMismatch {
                plan_workspace: workspace_id,
                next_step_workspace: next_step.workspace_id().clone(),
            });
        }

        Ok(Self {
            workspace_id,
            resume_point,
            context_chain,
            blockers,
            next_step,
        })
    }

    /// Returns the `WorkspaceId` this plan restores (RP-2).
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Returns the canonical cognitive entry point (RSP-1).
    pub fn resume_point(&self) -> &ResumePoint {
        &self.resume_point
    }

    /// Returns the ordered supporting context for the Resume Point.
    pub fn context_chain(&self) -> &ContextChain {
        &self.context_chain
    }

    /// Returns the unresolved conditions preventing immediate continuation (BL-1).
    ///
    /// An empty slice indicates no blockers exist.
    pub fn blockers(&self) -> &[Blocker] {
        &self.blockers
    }

    /// Returns the immediate continuation action following the Resume Point.
    pub fn next_step(&self) -> &NextStep {
        &self.next_step
    }

    /// Returns `true` if this plan has no blockers.
    ///
    /// A plan with no blockers indicates that continuation may proceed
    /// immediately from the `ResumePoint`.
    pub fn has_blockers(&self) -> bool {
        !self.blockers.is_empty()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use evo_artifact::artifact_id::ArtifactId;

    fn workspace_id() -> WorkspaceId {
        WorkspaceId::new()
    }

    fn artifact_id(label: &str) -> ArtifactId {
        ArtifactId::new(label).unwrap()
    }

    fn valid_plan(wid: WorkspaceId) -> RestorationPlan {
        let resume_point = ResumePoint::new(wid.clone(), artifact_id("rp-test-artifact"));
        let context_chain = ContextChain::new(vec![
            artifact_id("cc-test-1"),
            artifact_id("cc-test-2"),
        ])
        .unwrap();
        let blockers = vec![Blocker::new("failing test in evo-workspace").unwrap()];
        let next_step = NextStep::new(wid.clone(), "continue implementing snapshot construction").unwrap();

        RestorationPlan::new(wid, resume_point, context_chain, blockers, next_step).unwrap()
    }

    #[test]
    fn construction_succeeds_with_consistent_workspaces() {
        let wid = workspace_id();
        let plan = valid_plan(wid.clone());
        assert_eq!(plan.workspace_id(), &wid);
    }

    #[test]
    fn accessors_return_correct_components() {
        let wid = workspace_id();
        let aid = artifact_id("rp-accessor-artifact");
        let resume_point = ResumePoint::new(wid.clone(), aid.clone());
        let context_chain = ContextChain::new(vec![artifact_id("cc-accessor")]).unwrap();
        let blocker = Blocker::new("merge conflict on main").unwrap();
        let next_step =
            NextStep::new(wid.clone(), "resolve the merge conflict then continue").unwrap();

        let plan = RestorationPlan::new(
            wid.clone(),
            resume_point.clone(),
            context_chain.clone(),
            vec![blocker.clone()],
            next_step.clone(),
        )
        .unwrap();

        assert_eq!(plan.workspace_id(), &wid);
        assert_eq!(plan.resume_point(), &resume_point);
        assert_eq!(plan.context_chain(), &context_chain);
        assert_eq!(plan.blockers(), &[blocker]);
        assert_eq!(plan.next_step(), &next_step);
    }

    #[test]
    fn plan_with_no_blockers_is_valid() {
        let wid = workspace_id();
        let plan = RestorationPlan::new(
            wid.clone(),
            ResumePoint::new(wid.clone(), artifact_id("rp-no-blocker")),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(wid, "continue immediately").unwrap(),
        )
        .unwrap();

        assert!(!plan.has_blockers());
        assert!(plan.blockers().is_empty());
    }

    #[test]
    fn plan_with_blockers_reports_has_blockers() {
        let wid = workspace_id();
        let plan = valid_plan(wid);
        assert!(plan.has_blockers());
    }

    #[test]
    fn resume_point_workspace_mismatch_is_rejected() {
        let plan_wid = workspace_id();
        let wrong_wid = workspace_id();

        let resume_point = ResumePoint::new(wrong_wid.clone(), artifact_id("rp-wrong-ws"));
        let context_chain = ContextChain::new(vec![]).unwrap();
        let next_step = NextStep::new(plan_wid.clone(), "next").unwrap();

        let result = RestorationPlan::new(
            plan_wid.clone(),
            resume_point,
            context_chain,
            vec![],
            next_step,
        );

        assert!(matches!(
            result,
            Err(RestorationError::ResumePointWorkspaceMismatch { .. })
        ));
    }

    #[test]
    fn next_step_workspace_mismatch_is_rejected() {
        let plan_wid = workspace_id();
        let wrong_wid = workspace_id();

        let resume_point = ResumePoint::new(plan_wid.clone(), artifact_id("rp-ns-mismatch"));
        let context_chain = ContextChain::new(vec![]).unwrap();
        let next_step = NextStep::new(wrong_wid.clone(), "next from wrong workspace").unwrap();

        let result = RestorationPlan::new(
            plan_wid.clone(),
            resume_point,
            context_chain,
            vec![],
            next_step,
        );

        assert!(matches!(
            result,
            Err(RestorationError::NextStepWorkspaceMismatch { .. })
        ));
    }

    #[test]
    fn plan_workspace_id_is_stable() {
        let wid = workspace_id();
        let plan = valid_plan(wid.clone());
        assert_eq!(plan.workspace_id(), &wid);
        assert_eq!(plan.resume_point().workspace_id(), plan.workspace_id());
        assert_eq!(plan.next_step().workspace_id(), plan.workspace_id());
    }

    #[test]
    fn clone_is_equal_to_original() {
        let wid = workspace_id();
        let plan = valid_plan(wid);
        let cloned = plan.clone();
        assert_eq!(plan, cloned);
    }

    #[test]
    fn multiple_blockers_are_preserved() {
        let wid = workspace_id();
        let blockers = vec![
            Blocker::new("compilation failure").unwrap(),
            Blocker::new("unresolved dependency").unwrap(),
            Blocker::new("failing test").unwrap(),
        ];

        let plan = RestorationPlan::new(
            wid.clone(),
            ResumePoint::new(wid.clone(), artifact_id("rp-multi-blocker")),
            ContextChain::new(vec![]).unwrap(),
            blockers.clone(),
            NextStep::new(wid, "address blockers before continuing").unwrap(),
        )
        .unwrap();

        assert_eq!(plan.blockers().len(), 3);
        assert_eq!(plan.blockers(), blockers.as_slice());
    }

    #[test]
    fn plan_does_not_modify_workspace_understanding() {
        // Structural test: RestorationPlan exposes only read-only accessors.
        // No &mut self methods exist — immutability is enforced by the type system.
        let wid = workspace_id();
        let plan = valid_plan(wid);
        let _ = plan.workspace_id();
        let _ = plan.resume_point();
        let _ = plan.context_chain();
        let _ = plan.blockers();
        let _ = plan.next_step();
        let _ = plan.has_blockers();
    }
}
