//! Resume Point.
//!
//! A `ResumePoint` is the canonical cognitive entry point into a Workspace.
//!
//! It answers: **Where should the user continue thinking?**
//!
//! # IS-0014 Invariants
//!
//! - RSP-1: Exactly one Resume Point SHALL exist per Restoration Plan.
//! - RSP-2: Resume Point SHALL belong to exactly one Workspace.
//! - RSP-3: Resume Point SHALL reference canonical Artifacts only.
//! - RSP-4: Resume Point SHALL NOT reference raw Observations.
//!
//! A `ResumePoint` is immutable after construction.
//!
//! # Non-Responsibilities
//!
//! A `ResumePoint` is NOT:
//! - an application;
//! - a window;
//! - a browser tab;
//! - a monitor layout;
//! - an operating-system action.
//!
//! It represents understanding, not interface state (IS-0014 §5).

use evo_artifact::artifact_id::ArtifactId;
use evo_observation::observed_state::ObservedState;
use evo_workspace::WorkspaceId;

// ── ResumePoint ───────────────────────────────────────────────────────────────

/// The canonical cognitive entry point into a Workspace.
///
/// Identifies the canonical Artifact that represents where the user should
/// continue thinking (IS-0014 §5, RSP-1 through RSP-4).
///
/// # Invariants
///
/// - Belongs to exactly one Workspace (RSP-2).
/// - References exactly one canonical Artifact (RSP-3).
/// - Does not reference raw Observations (RSP-4).
/// - Immutable after construction.
///
/// # Non-Responsibilities
///
/// - Does **not** prescribe operating-system actions.
/// - Carries only optional interface state directly witnessed on the exact
///   occurrence selected as the stopping point.
/// - Does **not** reference raw Observations.
#[derive(Debug, Clone, PartialEq)]
pub struct ResumePoint {
    /// The Workspace this Resume Point belongs to (RSP-2).
    workspace_id: WorkspaceId,

    /// The canonical Artifact representing the cognitive entry point (RSP-3).
    artifact_id: ArtifactId,

    /// Generic local interface state witnessed on the selected stopping
    /// occurrence. Absence means the state was not observable; it is never
    /// filled from an older occurrence or inferred from the Artifact identity.
    observed_state: Option<ObservedState>,
}

impl ResumePoint {
    /// Constructs an immutable `ResumePoint`.
    ///
    /// # Parameters
    ///
    /// - `workspace_id`: the Workspace this Resume Point belongs to (RSP-2).
    /// - `artifact_id`: the canonical Artifact representing where thinking resumes (RSP-3).
    ///
    /// # Guarantees
    ///
    /// - The Resume Point belongs to exactly one Workspace (RSP-2).
    /// - The Resume Point references exactly one canonical Artifact (RSP-3).
    /// - Immutable after construction.
    pub fn new(workspace_id: WorkspaceId, artifact_id: ArtifactId) -> Self {
        Self {
            workspace_id,
            artifact_id,
            observed_state: None,
        }
    }

    /// Constructs a Resume Point with optional state directly witnessed on the
    /// exact occurrence that established this stopping point.
    pub fn new_with_observed_state(
        workspace_id: WorkspaceId,
        artifact_id: ArtifactId,
        observed_state: Option<ObservedState>,
    ) -> Self {
        Self {
            workspace_id,
            artifact_id,
            observed_state,
        }
    }

    /// Returns the Workspace this Resume Point belongs to (RSP-2).
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Returns the canonical Artifact representing the cognitive entry point (RSP-3).
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// Returns generic state directly witnessed where the work stopped.
    pub fn observed_state(&self) -> Option<&ObservedState> {
        self.observed_state.as_ref()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_id() -> WorkspaceId {
        WorkspaceId::new()
    }

    fn artifact_id() -> ArtifactId {
        ArtifactId::new("resume-point-test-artifact").unwrap()
    }

    #[test]
    fn construction_and_accessors() {
        let wid = workspace_id();
        let aid = artifact_id();
        let rp = ResumePoint::new(wid.clone(), aid.clone());

        assert_eq!(rp.workspace_id(), &wid);
        assert_eq!(rp.artifact_id(), &aid);
        assert_eq!(rp.observed_state(), None);
    }

    #[test]
    fn directly_witnessed_state_is_preserved() {
        let state = ObservedState::new().with_insertion_line(37);
        let rp = ResumePoint::new_with_observed_state(
            workspace_id(),
            artifact_id(),
            Some(state.clone()),
        );
        assert_eq!(rp.observed_state(), Some(&state));
    }

    #[test]
    fn clone_is_equal_to_original() {
        let rp = ResumePoint::new(workspace_id(), artifact_id());
        let cloned = rp.clone();
        assert_eq!(rp, cloned);
    }

    #[test]
    fn different_workspaces_are_not_equal() {
        let aid = artifact_id();
        let a = ResumePoint::new(WorkspaceId::new(), aid.clone());
        let b = ResumePoint::new(WorkspaceId::new(), aid);
        assert_ne!(a, b);
    }

    #[test]
    fn different_artifacts_are_not_equal() {
        let wid = workspace_id();
        let a = ResumePoint::new(wid.clone(), ArtifactId::new("artifact-rp-a").unwrap());
        let b = ResumePoint::new(wid, ArtifactId::new("artifact-rp-b").unwrap());
        assert_ne!(a, b);
    }

    #[test]
    fn resume_point_is_immutable_after_construction() {
        // The type system enforces this: no &mut self methods exist.
        let rp = ResumePoint::new(workspace_id(), artifact_id());
        let _ = rp.workspace_id();
        let _ = rp.artifact_id();
    }
}
