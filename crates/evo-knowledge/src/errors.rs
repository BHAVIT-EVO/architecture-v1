//! Domain error types for the evo-knowledge crate.
//!
//! Every error represents a violation of an invariant defined by IS-0015.

use evo_observation::observation_id::ObservationId;
use evo_artifact::artifact_id::ArtifactId;

/// A violation of the Knowledge domain invariants (IS-0015).
#[derive(Debug, Clone, PartialEq,)]
pub enum KnowledgeError {
    DuplicateObservationEvidence {
        observation_id: ObservationId,
    },

    DuplicateArtifactEvidence {
        artifact_id: ArtifactId,
    },
    /// A `Knowledge` was constructed or revised without any supporting evidence.
    ///
    /// Violates IS-0015 KI-3: Knowledge never exists without supporting evidence.
    NoSupportingEvidence,

    /// A `SupportingEvidence` was constructed with a duplicate `ObservationId`.
    ///
    /// Violates IS-0015 §6: Supporting Evidence SHALL NOT duplicate Observation data.
    DuplicateObservationReference { observation_id: ObservationId },

    /// A `SupportingEvidence` was constructed with a duplicate `ArtifactId`.
    ///
    /// Violates IS-0015 §6: Supporting Evidence SHALL NOT duplicate Artifact identity.
    DuplicateArtifactReference { artifact_id: ArtifactId },
}

impl std::fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { 
            KnowledgeError::DuplicateObservationEvidence { .. } => {
                write!(f, "duplicate observation evidence")
            }

            KnowledgeError::DuplicateArtifactEvidence { .. } => {
                write!(f, "duplicate artifact evidence")
            }
            KnowledgeError::NoSupportingEvidence => {
                write!(
                    f,
                    "Knowledge cannot exist without supporting evidence (IS-0015 KI-3)"
                )
            }
            KnowledgeError::DuplicateObservationReference { observation_id } => {
                write!(
                    f,
                    "SupportingEvidence contains a duplicate ObservationId: {observation_id} \
                     (IS-0015 §6)"
                )
            }
            KnowledgeError::DuplicateArtifactReference { artifact_id } => {
                write!(
                    f,
                    "SupportingEvidence contains a duplicate ArtifactId: {artifact_id} \
                     (IS-0015 §6)"
                )
            }
        }
    }
}

impl std::error::Error for KnowledgeError {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_supporting_evidence_display_contains_key_terms() {
        let msg = KnowledgeError::NoSupportingEvidence.to_string();
        assert!(msg.contains("KI-3"));
        assert!(msg.contains("supporting evidence"));
    }

    #[test]
    fn duplicate_observation_display_contains_key_terms() {
        let oid = ObservationId::new();
        let err = KnowledgeError::DuplicateObservationReference {
            observation_id: oid,
        };
        let msg = err.to_string();
        assert!(msg.contains("IS-0015 §6"));
        assert!(msg.contains("ObservationId"));
    }

    #[test]
    fn duplicate_artifact_display_contains_key_terms() {
        let aid = ArtifactId::new("error-test-artifact").unwrap();
        let err = KnowledgeError::DuplicateArtifactReference { artifact_id: aid };
        let msg = err.to_string();
        assert!(msg.contains("IS-0015 §6"));
        assert!(msg.contains("ArtifactId"));
    }

    #[test]
    fn errors_are_clone_and_eq() {
        let a = KnowledgeError::NoSupportingEvidence;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn duplicate_observation_evidence_display_contains_key_terms() {
        let oid = ObservationId::new();
        let err = KnowledgeError::DuplicateObservationEvidence {
            observation_id: oid,
        };
        let msg = err.to_string();
        assert!(msg.contains("duplicate observation"));
    }

    #[test]
    fn duplicate_artifact_evidence_display_contains_key_terms() {
        let aid = ArtifactId::new("error-test-artifact").unwrap();
        let err = KnowledgeError::DuplicateArtifactEvidence { artifact_id: aid };
        let msg = err.to_string();
        assert!(msg.contains("duplicate artifact"));
    }
}