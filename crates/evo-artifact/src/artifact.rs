//! Artifact Domain Model.
//!
//! An Artifact is the canonical computational representation of Evo's current
//! identity hypothesis regarding an external entity. (IS-0004)
//!
//! It provides a stable computational reference for higher architectural layers.
//! Observational accountability (IS-0004 R-4) is mandated by the architecture,
//! but the storage representation of that accountability is not embedded within
//! this domain object.

use crate::artifact_id::ArtifactId;

// ── Artifact ─────────────────────────────────────────────────────────────────

/// The canonical computational representation of an Artifact.
///
/// Every `Artifact` represents exactly one current identity hypothesis (IS-0004 R-2).
/// It provides a stable computational reference (R-3) while maintaining independence
/// from higher computational reasoning (I-5).
///
/// # Invariants
///
/// - Represents exactly one identity hypothesis.
/// # Non-Responsibilities
///
/// - Does **not** store or manage direct references to Observations.
/// - Does **not** determine Workspace or Task membership.
/// - Does **not** contain user intent or semantic interpretation.
/// - Does **not** modify or overwrite Observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// The stable computational reference for this Artifact (IS-0004 R-3).
    id: ArtifactId,
}

impl Artifact {
    
    /// Returns the stable computational reference for this Artifact.
    pub fn id(&self) -> &ArtifactId {
        &self.id
    }
}

impl Artifact {
    pub fn new(id: ArtifactId) -> Self {
        Self { id }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to generate a valid ArtifactId for testing
    fn create_test_artifact_id() -> ArtifactId {
        ArtifactId::new("test-artifact-ref").unwrap()
    }

    #[test]
    fn test_artifact_construction_and_accessors() {
        let id = create_test_artifact_id();
        let artifact = Artifact::new(id.clone());

        assert_eq!(artifact.id(), &id);
    }

    #[test]
    fn test_equality() {
        let id1 = create_test_artifact_id();
        let id2 = create_test_artifact_id(); // Uses the same string

        let artifact1 = Artifact::new(id1);
        let artifact2 = Artifact::new(id2);

        assert_eq!(artifact1, artifact2);
    }

    #[test]
    fn test_cloning() {
        let id = create_test_artifact_id();
        let artifact1 = Artifact::new(id);
        let artifact2 = artifact1.clone();

        assert_eq!(artifact1, artifact2);
    }

    #[test]
    fn test_immutability() {
        let id = create_test_artifact_id();
        let artifact = Artifact::new(id);

        // Accessor returns an immutable reference
        let _id_ref: &ArtifactId = artifact.id();

        // If this test compiles without exposing a mutable API or allowing
        // field reassignment, the struct's immutability invariant is preserved.
    }
}