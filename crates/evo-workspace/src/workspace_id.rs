//! Workspace Identity.
//!
//! `WorkspaceId` is the stable computational identity assigned to a Workspace.
//!
//! # IS-0011 Invariants
//!
//! - W-1: Every Workspace SHALL possess exactly one Workspace Identity.
//! - W-2: Workspace Identity SHALL remain stable throughout the Workspace lifetime.
//! - W-3: Workspace Identity SHALL NEVER encode semantic meaning.
//!
//! `WorkspaceId` is opaque. It encodes no meaning, no user intent, and no
//! semantic interpretation.
//!
//! Once assigned, a `WorkspaceId` is immutable.

use uuid::Uuid;
use std::str::FromStr;

// ── WorkspaceId ───────────────────────────────────────────────────────────────

/// The stable computational identity of a Workspace.
///
/// `WorkspaceId` is opaque and globally unique (IS-0011 §4, W-1, W-2, W-3).
///
/// It carries no semantic meaning.
///
/// # Non-Responsibilities
///
/// - Does **not** encode semantic meaning.
/// - Does **not** encode user intent.
/// - Does **not** encode semantic interpretation.
/// - Does **not** reflect the content of the Workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(Uuid);

impl WorkspaceId {
    /// Creates a new globally unique `WorkspaceId`.
    ///
    /// Every call produces a distinct identity.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Constructs a `WorkspaceId` from a stable 128-bit value.
    ///
    /// This constructor is crate-visible so deterministic Workspace
    /// Formation can derive replayable identities without changing the
    /// public API surface.
    pub(crate) fn from_u128(value: u128) -> Self {
        Self(Uuid::from_u128(value))
    }

    /// Constructs the opaque Workspace identity for a stable work hypothesis.
    /// The UUID remains semantically opaque; the source identity is owned by
    /// evo-engagement and is only carried across this projection boundary.
    pub fn from_work_id(value: u128) -> Self {
        Self::from_u128(value)
    }

    /// Returns the canonical string representation of this identity.
    ///
    /// The string form is stable for the lifetime of the `WorkspaceId`.
    pub fn as_str(&self) -> impl std::fmt::Display + '_ {
        self.0
    }
}

impl Default for WorkspaceId {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for WorkspaceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_produces_unique_identities() {
        let a = WorkspaceId::new();
        let b = WorkspaceId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn clone_preserves_identity() {
        let id = WorkspaceId::new();
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn display_is_stable() {
        let id = WorkspaceId::new();
        let first = id.to_string();
        let second = id.to_string();
        assert_eq!(first, second);
    }

    #[test]
    fn display_is_non_empty() {
        let id = WorkspaceId::new();
        assert!(!id.to_string().is_empty());
    }

    #[test]
    fn equality_is_identity_based() {
        let a = WorkspaceId::new();
        let b = a.clone();
        let c = WorkspaceId::new();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn default_produces_valid_identity() {
        let id: WorkspaceId = Default::default();
        assert!(!id.to_string().is_empty());
    }

    #[test]
    fn hash_is_consistent() {
        use std::collections::HashSet;
        let id = WorkspaceId::new();
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }
}
