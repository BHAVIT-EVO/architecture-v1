//! Error types for the evo-retrieval crate.
//!
//! RFC-0007 allows errors only when they prevent Retrieval from executing
//! according to the public computational contract. The absence of matching
//! Workspaces is not an error.

use evo_workspace::WorkspaceId;
use std::fmt;

/// Errors that can occur while executing Retrieval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetrievalError {
    /// A candidate WorkspaceId was returned more than once.
    DuplicateCandidateWorkspaceId { workspace_id: WorkspaceId },

    /// A candidate WorkspaceId was returned that is not present in the input
    /// set of current committed Workspaces.
    UnknownCandidateWorkspaceId { workspace_id: WorkspaceId },

    /// The configured retrieval implementation failed before producing a valid
    /// candidate set.
    ExecutionFailed,
}

impl fmt::Display for RetrievalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetrievalError::DuplicateCandidateWorkspaceId { workspace_id } => {
                write!(
                    f,
                    "retrieval returned duplicate candidate WorkspaceId: {workspace_id}"
                )
            }
            RetrievalError::UnknownCandidateWorkspaceId { workspace_id } => {
                write!(
                    f,
                    "retrieval returned WorkspaceId not present in the current committed Workspaces: {workspace_id}"
                )
            }
            RetrievalError::ExecutionFailed => {
                write!(f, "retrieval failed before producing a valid candidate set")
            }
        }
    }
}

impl std::error::Error for RetrievalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_candidate_error_displays_workspace_id() {
        let workspace_id = WorkspaceId::new();
        let error = RetrievalError::DuplicateCandidateWorkspaceId {
            workspace_id: workspace_id.clone(),
        };

        let message = error.to_string();
        assert!(message.contains(&workspace_id.to_string()));
        assert!(message.contains("duplicate"));
    }

    #[test]
    fn unknown_candidate_error_displays_workspace_id() {
        let workspace_id = WorkspaceId::new();
        let error = RetrievalError::UnknownCandidateWorkspaceId {
            workspace_id: workspace_id.clone(),
        };

        let message = error.to_string();
        assert!(message.contains(&workspace_id.to_string()));
        assert!(message.contains("not present"));
    }

    #[test]
    fn execution_failed_error_is_human_readable() {
        let message = RetrievalError::ExecutionFailed.to_string();
        assert!(!message.is_empty());
        assert!(message.contains("retrieval failed"));
    }

    #[test]
    fn retrieval_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}

        takes_error(&RetrievalError::ExecutionFailed);
    }
}
