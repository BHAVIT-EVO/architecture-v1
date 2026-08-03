//! Knowledge Identity.
//!
//! `KnowledgeId` is the stable, immutable identity assigned to a `Knowledge`.
//!
//! # IS-0015 Invariants
//!
//! - KI-1: Every Knowledge possesses exactly one immutable KnowledgeId.
//! - KI-8: Revision preserves Knowledge identity.
//!
//! `KnowledgeId` survives revision. It is the one component of a `Knowledge`
//! that is never replaced (IS-0015 §4, §9).
//!
//! If revision would produce a fundamentally different architectural constraint,
//! a new `Knowledge` with a new `KnowledgeId` SHALL be created instead (IS-0015 §4).
//!
//! `KnowledgeId` encodes no meaning. It encodes no architectural constraint.
//! It encodes no evidence.

use uuid::Uuid;

// ── KnowledgeId ───────────────────────────────────────────────────────────────

/// The stable, immutable identity of a `Knowledge`.
///
/// Survives revision throughout the entire lifetime of the `Knowledge` (IS-0015 §4, KI-1, KI-8).
///
/// Encodes no semantic meaning. Encodes no constraint content.
///
/// # Non-Responsibilities
///
/// - Does **not** encode the constraint.
/// - Does **not** encode evidence references.
/// - Does **not** change under revision (KI-8).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KnowledgeId(Uuid);

impl KnowledgeId {
    /// Creates a new globally unique `KnowledgeId`.
    ///
    /// Every call produces a distinct identity.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for KnowledgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for KnowledgeId {
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
        let a = KnowledgeId::new();
        let b = KnowledgeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn clone_preserves_identity() {
        let id = KnowledgeId::new();
        assert_eq!(id.clone(), id);
    }

    #[test]
    fn display_is_stable() {
        let id = KnowledgeId::new();
        assert_eq!(id.to_string(), id.to_string());
    }

    #[test]
    fn display_is_non_empty() {
        assert!(!KnowledgeId::new().to_string().is_empty());
    }

    #[test]
    fn equality_is_identity_based() {
        let a = KnowledgeId::new();
        let b = a.clone();
        let c = KnowledgeId::new();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn default_produces_valid_identity() {
        let id: KnowledgeId = Default::default();
        assert!(!id.to_string().is_empty());
    }

    #[test]
    fn is_hashable() {
        let id = KnowledgeId::new();
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }
}