//! Workspace Attachment.
//!
//! An `Attachment` represents the evidential membership relationship between
//! an Artifact and a Workspace.
//!
//! # IS-0011 Invariants
//!
//! - W-4: Workspace SHALL own Attachments. Workspace SHALL NOT own Artifacts.
//! - W-5: Every Attachment SHALL reference exactly one Artifact.
//! - W-6: Every Attachment SHALL contain exactly one Confidence Score.
//! - W-7: Confidence SHALL represent evidential strength only.
//!
//! An `Attachment` is immutable after creation (IS-0011 §3, §4).
//!
//! An `Attachment` is NOT ownership.
//!
//! An `Attachment` SHALL NOT modify, merge, or redefine Artifact Identity.

use evo_artifact::artifact_id::ArtifactId;

use crate::confidence::ConfidenceScore;

// ── Attachment ────────────────────────────────────────────────────────────────

/// The evidential membership relationship between an Artifact and a Workspace.
///
/// An `Attachment` is evidence only. It does not imply ownership (IS-0011 §3).
///
/// # Invariants
///
/// - References exactly one `ArtifactId` (W-5).
/// - Carries exactly one `ConfidenceScore` (W-6).
/// - Immutable after construction.
///
/// # Non-Responsibilities
///
/// - Does **not** own the referenced Artifact.
/// - Does **not** modify the referenced Artifact.
/// - Does **not** merge Artifacts.
/// - Does **not** redefine Artifact Identity.
/// - Does **not** encode importance, priority, or value (W-7).
#[derive(Debug, Clone, PartialEq)]
pub struct Attachment {
    /// The stable computational reference to the supporting Artifact (W-5).
    artifact_id: ArtifactId,

    /// The evidential strength of this membership relationship (W-6, W-7).
    confidence: ConfidenceScore,
}

impl Attachment {
    /// Constructs an immutable `Attachment`.
    ///
    /// The caller supplies the `ArtifactId` of the supporting Artifact and the
    /// confidence score representing its evidential strength.
    ///
    /// # Guarantees
    ///
    /// - References exactly one Artifact (W-5).
    /// - Carries exactly one confidence score (W-6).
    /// - Immutable after construction.
    pub fn new(artifact_id: ArtifactId, confidence: ConfidenceScore) -> Self {
        Self {
            artifact_id,
            confidence,
        }
    }

    /// Returns the stable computational reference to the supporting Artifact.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// Returns the evidential strength of this membership relationship.
    pub fn confidence(&self) -> &ConfidenceScore {
        &self.confidence
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact_id() -> ArtifactId {
        ArtifactId::new("test-artifact-for-attachment").unwrap()
    }

    fn confidence() -> ConfidenceScore {
        ConfidenceScore::new(0.8).unwrap()
    }

    #[test]
    fn construction_and_accessors() {
        let id = artifact_id();
        let conf = confidence();
        let attachment = Attachment::new(id.clone(), conf);

        assert_eq!(attachment.artifact_id(), &id);
        assert_eq!(attachment.confidence().value(), 0.8);
    }

    #[test]
    fn clone_preserves_fields() {
        let attachment = Attachment::new(artifact_id(), confidence());
        let cloned = attachment.clone();
        assert_eq!(attachment, cloned);
    }

    #[test]
    fn equality_requires_matching_artifact_and_confidence() {
        let a = Attachment::new(
            ArtifactId::new("artifact-a").unwrap(),
            ConfidenceScore::new(0.5).unwrap(),
        );
        let b = Attachment::new(
            ArtifactId::new("artifact-a").unwrap(),
            ConfidenceScore::new(0.5).unwrap(),
        );
        assert_eq!(a, b);
    }

    #[test]
    fn different_artifact_ids_are_not_equal() {
        let a = Attachment::new(
            ArtifactId::new("artifact-x").unwrap(),
            ConfidenceScore::new(0.5).unwrap(),
        );
        let b = Attachment::new(
            ArtifactId::new("artifact-y").unwrap(),
            ConfidenceScore::new(0.5).unwrap(),
        );
        assert_ne!(a, b);
    }

    #[test]
    fn different_confidence_scores_are_not_equal() {
        let id = ArtifactId::new("artifact-z").unwrap();
        let a = Attachment::new(id.clone(), ConfidenceScore::new(0.3).unwrap());
        let b = Attachment::new(id, ConfidenceScore::new(0.9).unwrap());
        assert_ne!(a, b);
    }

    #[test]
    fn zero_confidence_is_valid() {
        let attachment = Attachment::new(artifact_id(), ConfidenceScore::new(0.0).unwrap());
        assert_eq!(attachment.confidence().value(), 0.0);
    }

    #[test]
    fn max_confidence_is_valid() {
        let attachment = Attachment::new(artifact_id(), ConfidenceScore::new(1.0).unwrap());
        assert_eq!(attachment.confidence().value(), 1.0);
    }
}
