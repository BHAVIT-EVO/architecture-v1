//! Context Chain.
//!
//! A `ContextChain` contains the minimum ordered supporting context necessary
//! to understand the `ResumePoint`.
//!
//! # IS-0014 Invariants
//!
//! - CC-1: Ordering SHALL be deterministic.
//! - CC-2: Artifacts SHALL appear at most once.
//! - CC-3: Every Artifact SHALL belong to the same Workspace.
//! - CC-4: Context Chain SHALL minimize cognitive reload rather than maximize
//!         historical completeness.
//!
//! `ContextChain` enforces CC-2 at construction time by rejecting duplicate
//! `ArtifactId` values.
//!
//! CC-3 is enforced at the `RestorationPlan` level, where the Workspace is known.
//!
//! CC-1 and CC-4 are contracts on the formation algorithm that produces the
//! chain; this type preserves the order supplied at construction without
//! reordering.
//!
//! A `ContextChain` is immutable after construction.
//!
//! # Non-Responsibilities
//!
//! - Does **not** determine which Artifacts belong in the chain.
//! - Does **not** define ordering algorithms.
//! - Does **not** reference interface state.

use evo_artifact::artifact_id::ArtifactId;

use crate::errors::RestorationError;

// ── ContextChain ──────────────────────────────────────────────────────────────

/// The minimum ordered supporting context required to understand the `ResumePoint`.
///
/// References only canonical `ArtifactId` values (IS-0014 §6).
///
/// # Invariants
///
/// - No duplicate `ArtifactId` values (CC-2).
/// - Ordered as supplied at construction (CC-1).
/// - Immutable after construction.
///
/// # Non-Responsibilities
///
/// - Does **not** determine which Artifacts appear in the chain.
/// - Does **not** reorder Artifacts.
/// - Does **not** verify Workspace membership (CC-3 is enforced by `RestorationPlan`).
#[derive(Debug, Clone, PartialEq)]
pub struct ContextChain {
    /// Ordered canonical Artifact references (CC-1, CC-2).
    artifacts: Vec<ArtifactId>,
}

impl ContextChain {
    /// Constructs a `ContextChain` from an ordered sequence of canonical `ArtifactId` values.
    ///
    /// # Errors
    ///
    /// Returns `RestorationError::DuplicateArtifactInContextChain` if any `ArtifactId`
    /// appears more than once (CC-2).
    ///
    /// # Guarantees
    ///
    /// - No duplicate `ArtifactId` values (CC-2).
    /// - The supplied order is preserved exactly (CC-1).
    /// - Immutable after construction.
    pub fn new(artifacts: Vec<ArtifactId>) -> Result<Self, RestorationError> {
        for (i, candidate) in artifacts.iter().enumerate() {
            for existing in artifacts[..i].iter() {
                if existing == candidate {
                    return Err(RestorationError::DuplicateArtifactInContextChain {
                        artifact_id: (*candidate).clone(),
                    });
                }
            }
        }
        Ok(Self { artifacts })
    }

    /// Returns the ordered sequence of canonical `ArtifactId` values.
    ///
    /// The order is the deterministic ordering supplied at construction (CC-1).
    pub fn artifacts(&self) -> &[ArtifactId] {
        &self.artifacts
    }

    /// Returns `true` if the chain contains no Artifacts.
    ///
    /// An empty chain is valid. It indicates that no supporting context is
    /// required beyond the `ResumePoint` itself.
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    /// Returns the number of Artifacts in the chain.
    pub fn len(&self) -> usize {
        self.artifacts.len()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn aid(label: &str) -> ArtifactId {
        ArtifactId::new(label).unwrap()
    }

    #[test]
    fn empty_chain_is_valid() {
        let chain = ContextChain::new(vec![]).unwrap();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn single_artifact_is_valid() {
        let chain = ContextChain::new(vec![aid("cc-artifact-1")]).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.artifacts()[0], aid("cc-artifact-1"));
    }

    #[test]
    fn multiple_distinct_artifacts_are_valid() {
        let chain = ContextChain::new(vec![
            aid("cc-a"),
            aid("cc-b"),
            aid("cc-c"),
        ])
        .unwrap();
        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn duplicate_artifact_is_rejected() {
        let result = ContextChain::new(vec![aid("cc-dup"), aid("cc-other"), aid("cc-dup")]);
        assert!(matches!(
            result,
            Err(RestorationError::DuplicateArtifactInContextChain { .. })
        ));
    }

    #[test]
    fn adjacent_duplicates_are_rejected() {
        let result = ContextChain::new(vec![aid("cc-x"), aid("cc-x")]);
        assert!(matches!(
            result,
            Err(RestorationError::DuplicateArtifactInContextChain { .. })
        ));
    }

    #[test]
    fn order_is_preserved() {
        let ids = vec![aid("cc-z"), aid("cc-a"), aid("cc-m")];
        let chain = ContextChain::new(ids.clone()).unwrap();
        assert_eq!(chain.artifacts(), ids.as_slice());
    }

    #[test]
    fn clone_is_equal_to_original() {
        let chain = ContextChain::new(vec![aid("cc-clone-1"), aid("cc-clone-2")]).unwrap();
        let cloned = chain.clone();
        assert_eq!(chain, cloned);
    }

    #[test]
    fn error_contains_duplicate_artifact_id() {
        let dup = aid("cc-duplicate-id");
        let result = ContextChain::new(vec![dup.clone(), aid("cc-other"), dup.clone()]);
        match result {
            Err(RestorationError::DuplicateArtifactInContextChain { artifact_id }) => {
                assert_eq!(artifact_id, dup);
            }
            _ => panic!("expected DuplicateArtifactInContextChain"),
        }
    }
}
