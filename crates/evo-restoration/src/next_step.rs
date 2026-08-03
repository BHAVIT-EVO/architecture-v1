//! Next Step.
//!
//! A `NextStep` represents the immediate continuation action that naturally
//! follows the `ResumePoint`.
//!
//! # IS-0014 Invariants
//!
//! - NS-1: Next Step SHALL belong to the same Workspace.
//! - NS-2: Next Step SHALL immediately follow the Resume Point.
//! - NS-3: Next Step SHALL NOT depend on implementation-specific execution mechanisms.
//!
//! `NextStep` is platform-independent. It describes what should happen next
//! in terms of the work itself, not in terms of operating-system or interface
//! operations (NS-3).
//!
//! Workspace consistency (NS-1) is enforced by `RestorationPlan` at construction.
//!
//! A `NextStep` is immutable after construction.
//!
//! # Non-Responsibilities
//!
//! - Does **not** launch applications.
//! - Does **not** interact with the operating system.
//! - Does **not** prescribe execution mechanisms (NS-3).
//! - Does **not** define ordering algorithms.

use evo_workspace::WorkspaceId;

use crate::errors::RestorationError;

// ── NextStep ──────────────────────────────────────────────────────────────────

/// The immediate continuation action that naturally follows the `ResumePoint`.
///
/// Represents understanding of what should happen next, independent of any
/// platform-specific execution mechanism (IS-0014 §8, NS-3).
///
/// # Invariants
///
/// - Belongs to exactly one Workspace (NS-1).
/// - Platform-independent (NS-3).
/// - Immutable after construction.
///
/// # Non-Responsibilities
///
/// - Does **not** prescribe operating-system actions.
/// - Does **not** depend on interface state.
/// - Does **not** execute anything.
#[derive(Debug, Clone, PartialEq)]
pub struct NextStep {
    /// The Workspace this Next Step belongs to (NS-1).
    workspace_id: WorkspaceId,

    /// A platform-independent description of the immediate continuation action (NS-3).
    description: String,
}

impl NextStep {
    /// Constructs an immutable `NextStep`.
    ///
    /// # Parameters
    ///
    /// - `workspace_id`: the Workspace this Next Step belongs to (NS-1).
    /// - `description`: a platform-independent description of the immediate
    ///   continuation action (NS-3). Must not prescribe execution mechanisms.
    ///
    /// # Guarantees
    ///
    /// - Belongs to exactly one Workspace (NS-1).
    /// - Platform-independent; no execution mechanism implied (NS-3).
    /// - Immutable after construction.
    pub fn new(workspace_id: WorkspaceId, description: impl Into<String>) -> Result<Self, RestorationError> {
        Ok(Self {
            workspace_id,
            description: description.into(),
        })
    }

    /// Returns the Workspace this Next Step belongs to (NS-1).
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Returns the platform-independent description of the immediate continuation action.
    pub fn description(&self) -> &str {
        &self.description
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_id() -> WorkspaceId {
        WorkspaceId::new()
    }

    #[test]
    fn construction_and_accessors() {
        let wid = workspace_id();
        let desc = "implement integrity verification for the context chain";
        let ns = NextStep::new(wid.clone(), desc).unwrap();

        assert_eq!(ns.workspace_id(), &wid);
        assert_eq!(ns.description(), desc);
    }

    #[test]
    fn clone_is_equal_to_original() {
        let ns = NextStep::new(workspace_id(), "resume snapshot construction").unwrap();
        let cloned = ns.clone();
        assert_eq!(ns, cloned);
    }

    #[test]
    fn different_workspaces_are_not_equal() {
        let desc = "continue implementation";
        let a = NextStep::new(WorkspaceId::new(), desc).unwrap();
        let b = NextStep::new(WorkspaceId::new(), desc).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_descriptions_are_not_equal() {
        let wid = workspace_id();
        let a = NextStep::new(wid.clone(), "write tests for blocker").unwrap();
        let b = NextStep::new(wid, "write tests for context chain").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn next_step_is_immutable_after_construction() {
        let ns = NextStep::new(workspace_id(), "complete the restoration plan").unwrap();
        let _ = ns.workspace_id();
        let _ = ns.description();
    }

    #[test]
    fn string_and_str_both_construct() {
        let a = NextStep::new(workspace_id(), "from &str").unwrap();
        let b = NextStep::new(workspace_id(), String::from("from String")).unwrap();
        assert_eq!(a.description(), "from &str");
        assert_eq!(b.description(), "from String");
    }
}
