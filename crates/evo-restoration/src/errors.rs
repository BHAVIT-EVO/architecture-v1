//! Domain error types for the evo-restoration crate.
//!
//! Every error represents a violation of an invariant defined by IS-0014.

use evo_artifact::artifact_id::ArtifactId;
use evo_workspace::WorkspaceId;

/// A violation of the Restoration domain invariants (IS-0014).
#[derive(Debug, Clone, PartialEq)]
pub enum RestorationError {
    /// A `ContextChain` was constructed with a duplicate `ArtifactId`.
    ///
    /// Violates IS-0014 CC-2: Artifacts SHALL appear at most once.
    DuplicateArtifactInContextChain { artifact_id: ArtifactId },

    /// The `ResumePoint` workspace does not match the `RestorationPlan` workspace.
    ///
    /// Violates IS-0014 RSP-2: Resume Point SHALL belong to exactly one Workspace.
    /// Violates IS-0014 RP-2: A Restoration Plan SHALL preserve Workspace identity.
    ResumePointWorkspaceMismatch {
        plan_workspace: WorkspaceId,
        resume_point_workspace: WorkspaceId,
    },

    /// The `NextStep` workspace does not match the `RestorationPlan` workspace.
    ///
    /// Violates IS-0014 NS-1: Next Step SHALL belong to the same Workspace.
    NextStepWorkspaceMismatch {
        plan_workspace: WorkspaceId,
        next_step_workspace: WorkspaceId,
    },
}

impl std::fmt::Display for RestorationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestorationError::DuplicateArtifactInContextChain { artifact_id } => {
                write!(
                    f,
                    "ContextChain contains a duplicate ArtifactId: {artifact_id} (IS-0014 CC-2)"
                )
            }
            RestorationError::ResumePointWorkspaceMismatch {
                plan_workspace,
                resume_point_workspace,
            } => {
                write!(
                    f,
                    "ResumePoint workspace {resume_point_workspace} does not match \
                     RestorationPlan workspace {plan_workspace} (IS-0014 RSP-2, RP-2)"
                )
            }
            RestorationError::NextStepWorkspaceMismatch {
                plan_workspace,
                next_step_workspace,
            } => {
                write!(
                    f,
                    "NextStep workspace {next_step_workspace} does not match \
                     RestorationPlan workspace {plan_workspace} (IS-0014 NS-1)"
                )
            }
        }
    }
}

impl std::error::Error for RestorationError {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact_id() -> ArtifactId {
        ArtifactId::new("error-test-artifact").unwrap()
    }

    fn workspace_id() -> WorkspaceId {
        WorkspaceId::new()
    }

    #[test]
    fn duplicate_artifact_display_contains_key_terms() {
        let err = RestorationError::DuplicateArtifactInContextChain {
            artifact_id: artifact_id(),
        };
        let msg = err.to_string();
        assert!(msg.contains("CC-2"));
        assert!(msg.contains("duplicate"));
    }

    #[test]
    fn resume_point_mismatch_display_contains_key_terms() {
        let err = RestorationError::ResumePointWorkspaceMismatch {
            plan_workspace: workspace_id(),
            resume_point_workspace: workspace_id(),
        };
        let msg = err.to_string();
        assert!(msg.contains("RSP-2"));
        assert!(msg.contains("ResumePoint"));
    }

    #[test]
    fn next_step_mismatch_display_contains_key_terms() {
        let err = RestorationError::NextStepWorkspaceMismatch {
            plan_workspace: workspace_id(),
            next_step_workspace: workspace_id(),
        };
        let msg = err.to_string();
        assert!(msg.contains("NS-1"));
        assert!(msg.contains("NextStep"));
    }

    #[test]
    fn errors_are_clone_and_eq() {
        let a = RestorationError::DuplicateArtifactInContextChain {
            artifact_id: artifact_id(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
