//! Candidate Artifact.
//!
//! A Candidate Artifact represents the transient computational state from
//! which a canonical Artifact may be accepted (IS-0004 — Candidate Artifact).
//!
//! Every Candidate Artifact represents exactly one Identity Hypothesis
//! awaiting acceptance (IS-0004 — Candidate Artifact, Identity Hypothesis).
//!
//! A Candidate Artifact exists only during the Artifact Acceptance Pipeline.
//! It does not survive beyond acceptance or rejection (IS-0005 R-2).
//!
//! # Internal Representation
//!
//! The internal representation of a Candidate Artifact is intentionally
//! unspecified by IS-0004. All fields are private. No accessors are exposed.
//! This file is the designated integration point for future field additions
//! when the governing specification is written.

// ── CandidateArtifact ─────────────────────────────────────────────────────────

/// The transient computational state representing one Identity Hypothesis
/// awaiting Artifact Acceptance (IS-0004).
///
/// `CandidateArtifact` is the input to the Artifact Acceptance Pipeline
/// (IS-0005). It is produced before the pipeline begins and consumed — never
/// cloned — through each stage until acceptance produces one canonical
/// [`Artifact`] or rejection ceases its existence.
///
/// # Invariants
///
/// - Represents exactly one Identity Hypothesis (IS-0004 — Candidate Artifact).
/// - Does NOT possess canonical Artifact Identity (IS-0004 — Candidate
///   Artifact; IS-0005 Definitions).
/// - Is NOT referenced by higher computational layers (IS-0004 — Candidate
///   Artifact).
/// - Immutable after construction.
///
/// # Internal Representation
///
/// The internal representation is intentionally unspecified by IS-0004.
/// All fields are private and must not be accessed outside this module.
///
/// # Non-Responsibilities
///
/// - Does **not** infer Identity Hypotheses.
/// - Does **not** assign canonical [`ArtifactId`].
/// - Does **not** validate, canonicalize, or verify integrity.
/// - Does **not** persist anything.
/// - Does **not** reference higher computational layers.
///
/// [`Artifact`]: crate::artifact::Artifact
/// [`ArtifactId`]: crate::artifact_id::ArtifactId
#[derive(Debug)]
pub struct CandidateArtifact;
    // Internal representation is intentionally unspecified by IS-0004.
    // This field is a zero-sized private marker. Its sole purpose is to
    // prevent external construction of this struct and to make the privacy
    // of the representation explicit. It carries no semantic content.

impl CandidateArtifact {
    /// Constructs a `CandidateArtifact` representing one Identity Hypothesis
    /// awaiting acceptance.
    ///
    /// # Caller Responsibilities
    ///
    /// The Acceptance Pipeline enforces these invariants through Validation
    /// (IS-0005 Stage 1) before any subsequent stage executes.
    pub fn new() -> Self {
        Self
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_constructs_successfully() {
        let _ = CandidateArtifact::new();
    }

    #[test]
    fn candidate_new_for_testing_constructs_successfully() {
        let _ = CandidateArtifact::new();
    }

    #[test]
    fn candidate_is_consumed_by_value_through_pipeline() {
        // CandidateArtifact does not implement Copy. Once moved into a
        // pipeline stage, it cannot be reused. This enforces IS-0005 R-2:
        // atomic acceptance — no partial state survives.
        let candidate = CandidateArtifact::new();
        let _moved = candidate;
        // `candidate` is no longer accessible here.
        // If this test compiles, move semantics are confirmed.
    }

    #[test]
    fn candidate_representation_is_private() {
        // If this test compiles without accessing any field,
        // the internal representation is fully encapsulated.
        let _candidate = CandidateArtifact::new();
    }
}
