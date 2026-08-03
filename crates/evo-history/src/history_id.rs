//! History Identity.
//!
//! `HistoryId` is the stable, immutable identity assigned to a
//! `HistoricalUnderstanding`.
//!
//! # IS-0016 Requirements (§14)
//!
//! - Immutable.
//! - Globally unique; never reused.
//! - Cloneable.
//! - Hashable.
//! - Comparable.
//!
//! # IS-0016 Invariants
//!
//! - H-1: Every HistoricalUnderstanding has exactly one HistoryId.
//! - H-2: HistoryId never changes.
//!
//! Replay SHALL create a new `HistoricalUnderstanding` with a new `HistoryId`.
//! Replay SHALL NEVER reuse an existing `HistoryId` (IS-0016 §5).
//!
//! `HistoryId` encodes no meaning. It uniquely identifies one committed
//! understanding and nothing else.

use uuid::Uuid;

// ── HistoryId ─────────────────────────────────────────────────────────────────

/// The stable, immutable identity of a `HistoricalUnderstanding`.
///
/// Globally unique. Never reused. Never changed (IS-0016 §5, H-1, H-2).
///
/// # Non-Responsibilities
///
/// - Does **not** encode the committed understanding.
/// - Does **not** encode the Workspace reference.
/// - Does **not** change under replay (H-2).
/// - Does **not** encode semantic meaning.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HistoryId(Uuid);

impl HistoryId {
    /// Creates a new globally unique `HistoryId`.
    ///
    /// Every call produces a distinct identity that is never reused (IS-0016 §5).
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for HistoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for HistoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn new_produces_unique_identities() {
        let a = HistoryId::new();
        let b = HistoryId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn clone_preserves_identity() {
        let id = HistoryId::new();
        assert_eq!(id.clone(), id);
    }

    #[test]
    fn display_is_stable() {
        let id = HistoryId::new();
        assert_eq!(id.to_string(), id.to_string());
    }

    #[test]
    fn display_is_non_empty() {
        assert!(!HistoryId::new().to_string().is_empty());
    }

    #[test]
    fn equality_is_identity_based() {
        let a = HistoryId::new();
        let b = a.clone();
        let c = HistoryId::new();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn default_produces_valid_identity() {
        let id: HistoryId = Default::default();
        assert!(!id.to_string().is_empty());
    }

    #[test]
    fn is_hashable() {
        let id = HistoryId::new();
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn two_new_ids_are_never_equal() {
        // Replay must never reuse a HistoryId (IS-0016 §5).
        for _ in 0..20 {
            assert_ne!(HistoryId::new(), HistoryId::new());
        }
    }
}
