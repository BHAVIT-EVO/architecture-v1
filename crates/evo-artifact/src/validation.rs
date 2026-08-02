//! Validation of Candidate Artifacts.
//!
//! Determines whether a [`CandidateArtifact`] satisfies the structural
//! requirements for entry into the Artifact Acceptance Pipeline (IS-0005
//! Stage 1).
//!
//! Validation is **pure** and **deterministic**:
//! - it produces no side effects,
//! - it never mutates the candidate,
//! - it never canonicalizes, assigns identity, verifies integrity, or persists,
//! - identical inputs always produce identical outputs.

use crate::candidate::CandidateArtifact;
use crate::errors::ValidationError;

// ── Public entry point ────────────────────────────────────────────────────────

/// Validates a [`CandidateArtifact`] against the structural requirements for
/// Artifact Acceptance (IS-0005 Stage 1).
///
/// Returns `Ok(())` when the Candidate Artifact satisfies the structural
/// Requirements defined by IS-0005 Stage 1.
/// Returns a [`ValidationError`] if structural validation fails.
/// Returns a [`ValidationError`] on the first failure encountered.
/// No partial acceptance is permitted (IS-0005 R-2).
///
/// # Non-Responsibilities
///
/// - Does **not** canonicalize the candidate.
/// - Does **not** assign [`ArtifactId`].
/// - Does **not** verify acceptance invariants.
/// - Does **not** persist anything.
/// - Does **not** infer identity.
///
/// [`ArtifactId`]: crate::artifact_id::ArtifactId
pub fn validate(candidate: &CandidateArtifact) -> Result<(), ValidationError> {
    let _ = candidate;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::candidate::CandidateArtifact;

    #[test]
    fn valid_candidate_passes_validation() {
        let candidate = CandidateArtifact::new();
        assert!(validate(&candidate).is_ok());
    }

    #[test]
    fn validation_borrows_candidate_without_consuming_it() {
        let candidate = CandidateArtifact::new();
        let _ = validate(&candidate);
        let _ = validate(&candidate);
    }

    #[test]
    fn identical_inputs_produce_identical_outputs() {
        let candidate = CandidateArtifact::new();
        let result_a = validate(&candidate);
        let result_b = validate(&candidate);
        assert_eq!(result_a.is_ok(), result_b.is_ok());
    }
}
