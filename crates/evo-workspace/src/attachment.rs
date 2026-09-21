//! Workspace Attachment.
//!
//! An `Attachment` represents the evidential membership relationship between
//! an Artifact and a Workspace, and the part that Artifact played in the work.
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
//!
//! # Why membership and part are two fields
//!
//! W-7 is emphatic that confidence is evidential strength and nothing else —
//! not importance, not priority, not value. That was the right rule and it left
//! a gap: with confidence forbidden from carrying importance, and nothing else
//! on the Attachment able to carry it, a Workspace had no way to say that some
//! of its members are where the work happens and others are things that were
//! merely open. Everything attached was equally attached, so Restoration could
//! only open all of it.
//!
//! The two claims are genuinely independent, in both directions. A reference
//! page consulted once can belong beyond any doubt and still be the last thing
//! a person needs reopened. A document that is unmistakably the work can belong
//! on thinner evidence than that, because it shares no wording or location with
//! anything else in it. So the Attachment carries both, separately: how strongly
//! this Artifact is claimed to belong ([`Attachment::confidence`]), and what part
//! it played ([`Attachment::role`]).

use evo_artifact::artifact_id::ArtifactId;

use crate::confidence::ConfidenceScore;

/// The part an Artifact played in a body of work.
///
/// This is the Engagement layer's [`evo_engagement::ResourceRole`], unchanged
/// and not a translation of it. A parallel enum here would be a second place for
/// the same judgement to live, and the two would drift.
pub use evo_engagement::ResourceRole;

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
/// - Does **not** encode importance in its Confidence Score (W-7) — importance
///   is [`Attachment::role`], which is a separate field for exactly that reason.
#[derive(Debug, Clone, PartialEq)]
pub struct Attachment {
    /// The stable computational reference to the supporting Artifact (W-5).
    artifact_id: ArtifactId,

    /// The evidential strength of this membership relationship (W-6, W-7).
    confidence: ConfidenceScore,

    /// The part this Artifact played in the body of work.
    role: ResourceRole,
}

impl Attachment {
    /// Constructs an immutable `Attachment`.
    ///
    /// The caller supplies the `ArtifactId` of the supporting Artifact, the
    /// confidence score representing the evidential strength of its membership,
    /// and the part it played in the work.
    ///
    /// # Guarantees
    ///
    /// - References exactly one Artifact (W-5).
    /// - Carries exactly one confidence score (W-6).
    /// - Immutable after construction.
    pub fn new(artifact_id: ArtifactId, confidence: ConfidenceScore, role: ResourceRole) -> Self {
        Self {
            artifact_id,
            confidence,
            role,
        }
    }

    /// Returns the stable computational reference to the supporting Artifact.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// Returns the evidential strength of this membership relationship.
    ///
    /// How strongly this Artifact is claimed to belong — never how important it
    /// is (W-7). See [`Attachment::role`] for that.
    pub fn confidence(&self) -> &ConfidenceScore {
        &self.confidence
    }

    /// Returns the part this Artifact played in the body of work.
    ///
    /// This is what Restoration reads to decide what to open, so that returning
    /// to work puts the person back where they were instead of reopening
    /// everything that was ever attached.
    pub fn role(&self) -> ResourceRole {
        self.role
    }

    /// Whether Restoration should open this Artifact automatically.
    pub fn opens_on_restore(&self) -> bool {
        self.role.opens_on_restore()
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
        let attachment = Attachment::new(id.clone(), conf, ResourceRole::Primary);

        assert_eq!(attachment.artifact_id(), &id);
        assert_eq!(attachment.confidence().value(), 0.8);
        assert_eq!(attachment.role(), ResourceRole::Primary);
    }

    #[test]
    fn clone_preserves_fields() {
        let attachment = Attachment::new(artifact_id(), confidence(), ResourceRole::Supporting);
        let cloned = attachment.clone();
        assert_eq!(attachment, cloned);
    }

    #[test]
    fn equality_requires_matching_artifact_and_confidence() {
        let a = Attachment::new(
            ArtifactId::new("artifact-a").unwrap(),
            ConfidenceScore::new(0.5).unwrap(),
            ResourceRole::Reference,
        );
        let b = Attachment::new(
            ArtifactId::new("artifact-a").unwrap(),
            ConfidenceScore::new(0.5).unwrap(),
            ResourceRole::Reference,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn different_artifact_ids_are_not_equal() {
        let a = Attachment::new(
            ArtifactId::new("artifact-x").unwrap(),
            ConfidenceScore::new(0.5).unwrap(),
            ResourceRole::Primary,
        );
        let b = Attachment::new(
            ArtifactId::new("artifact-y").unwrap(),
            ConfidenceScore::new(0.5).unwrap(),
            ResourceRole::Primary,
        );
        assert_ne!(a, b);
    }

    #[test]
    fn different_confidence_scores_are_not_equal() {
        let id = ArtifactId::new("artifact-z").unwrap();
        let a = Attachment::new(
            id.clone(),
            ConfidenceScore::new(0.3).unwrap(),
            ResourceRole::Primary,
        );
        let b = Attachment::new(
            id,
            ConfidenceScore::new(0.9).unwrap(),
            ResourceRole::Primary,
        );
        assert_ne!(a, b);
    }

    /// The same Artifact, believed to belong just as strongly, playing a
    /// different part, is a different Attachment. If it were not, the model
    /// would have no way to record that a resource stopped being where the work
    /// continues.
    #[test]
    fn different_roles_are_not_equal() {
        let id = ArtifactId::new("artifact-role").unwrap();
        let strength = ConfidenceScore::new(0.7).unwrap();
        let primary = Attachment::new(id.clone(), strength, ResourceRole::Primary);
        let reference = Attachment::new(id, strength, ResourceRole::Reference);
        assert_ne!(primary, reference);
    }

    /// The claim Restoration acts on. Only where the work happens is opened;
    /// everything else stays available without being thrown at the person.
    #[test]
    fn only_the_places_the_work_happens_open_on_restore() {
        let strength = ConfidenceScore::new(1.0).unwrap();
        for (role, opens) in [
            (ResourceRole::Continuation, true),
            (ResourceRole::Primary, true),
            (ResourceRole::Supporting, false),
            (ResourceRole::Reference, false),
            (ResourceRole::Context, false),
        ] {
            let attachment = Attachment::new(artifact_id(), strength, role);
            assert_eq!(
                attachment.opens_on_restore(),
                opens,
                "{role:?} should {}open on restore",
                if opens { "" } else { "not " }
            );
        }
    }

    #[test]
    fn zero_confidence_is_valid() {
        let attachment = Attachment::new(
            artifact_id(),
            ConfidenceScore::new(0.0).unwrap(),
            ResourceRole::Context,
        );
        assert_eq!(attachment.confidence().value(), 0.0);
    }

    #[test]
    fn max_confidence_is_valid() {
        let attachment = Attachment::new(
            artifact_id(),
            ConfidenceScore::new(1.0).unwrap(),
            ResourceRole::Primary,
        );
        assert_eq!(attachment.confidence().value(), 1.0);
    }
}
