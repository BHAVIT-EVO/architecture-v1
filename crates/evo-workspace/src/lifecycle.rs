//! Workspace Lifecycle State.
//!
//! `WorkspaceLifecycle` describes the current computational state of a Workspace.
//!
//! # IS-0011 — Canonical Lifecycle States
//!
//! IS-0011 §4 defines exactly two canonical lifecycle states:
//!
//! - `Active`: the Workspace represents the current canonical understanding
//!   of a body of work.
//!
//! - `Superseded`: the Workspace has been replaced by a newer canonical
//!   understanding through replay or interpretation evolution, but remains
//!   preserved for historical accountability.
//!
//! No additional lifecycle states exist in the canonical model (IS-0011 §4).
//!
//! Lifecycle state belongs to the Workspace, not to Attachments or Snapshots.
//!
//! # Non-Responsibilities
//!
//! This module does NOT define:
//!
//! - lifecycle transition logic (belongs to a future formation IS);
//! - transition guards or rules;
//! - persistence behavior.

// ── WorkspaceLifecycle ────────────────────────────────────────────────────────

/// The computational lifecycle state of a Workspace.
///
/// Defined by IS-0011 §4. Exactly two states exist in the canonical model.
///
/// # Invariants
///
/// - Every Workspace SHALL possess exactly one lifecycle state (IS-0011 §4).
/// - Lifecycle belongs to the Workspace, not to Attachments or Snapshots (IS-0011 §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceLifecycle {
    /// The Workspace represents the current canonical understanding of a body of work.
    ///
    /// An `Active` Workspace is the authoritative present-tense interpretation.
    Active,

    /// The Workspace has been replaced by a newer canonical understanding through
    /// replay or interpretation evolution.
    ///
    /// A `Superseded` Workspace is preserved for historical accountability only.
    /// It SHALL NOT be treated as the current canonical understanding.
    Superseded,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_variant_exists() {
        let state = WorkspaceLifecycle::Active;
        assert_eq!(state, WorkspaceLifecycle::Active);
    }

    #[test]
    fn superseded_variant_exists() {
        let state = WorkspaceLifecycle::Superseded;
        assert_eq!(state, WorkspaceLifecycle::Superseded);
    }

    #[test]
    fn active_and_superseded_are_distinct() {
        assert_ne!(WorkspaceLifecycle::Active, WorkspaceLifecycle::Superseded);
    }

    #[test]
    fn clone_preserves_variant() {
        let a = WorkspaceLifecycle::Active;
        let b = a.clone();
        assert_eq!(a, b);

        let c = WorkspaceLifecycle::Superseded;
        let d = c.clone();
        assert_eq!(c, d);
    }

    #[test]
    fn debug_format_is_non_empty() {
        let a = format!("{:?}", WorkspaceLifecycle::Active);
        let s = format!("{:?}", WorkspaceLifecycle::Superseded);
        assert!(!a.is_empty());
        assert!(!s.is_empty());
    }
}
