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
use crate::identity_assignment::canonical_signature;

use std::collections::HashSet;

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
    let observations = candidate.observations();

    if observations.is_empty() {
        return Err(ValidationError::StructuralValidationFailed(
            "candidate artifact must reference one or more canonical observations".into(),
        ));
    }

    let mut seen = HashSet::with_capacity(observations.len());

    for observation in observations {
        let inserted = seen.insert(observation.id().clone());
        if !inserted {
            return Err(ValidationError::StructuralValidationFailed(
                "candidate artifact must not reference the same observation more than once".into(),
            ));
        }
    }

    let mut signatures = HashSet::with_capacity(observations.len());
    for observation in observations {
        let signature = canonical_signature(observation).map_err(|_| {
            ValidationError::StructuralValidationFailed(
                "candidate artifact must reference canonical observations with supported evidence"
                    .into(),
            )
        })?;
        signatures.insert(signature);
    }

    if signatures.len() > 1 {
        return Err(ValidationError::StructuralValidationFailed(
            "candidate artifact must reference canonical observations that describe one external entity".into(),
        ));
    }

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::candidate::{CandidateArtifact, test_candidate_with_observations, test_observation};

    #[test]
    fn valid_candidate_passes_validation() {
        let candidate = test_candidate_with_observations(vec![test_observation()]);
        assert!(validate(&candidate).is_ok());
    }

    #[test]
    fn validation_borrows_candidate_without_consuming_it() {
        let candidate = test_candidate_with_observations(vec![test_observation()]);
        let _ = validate(&candidate);
        let _ = validate(&candidate);
    }

    #[test]
    fn identical_inputs_produce_identical_outputs() {
        let candidate = test_candidate_with_observations(vec![test_observation()]);
        let result_a = validate(&candidate);
        let result_b = validate(&candidate);
        assert_eq!(result_a.is_ok(), result_b.is_ok());
    }

    #[test]
    fn empty_observation_set_is_rejected() {
        let candidate = CandidateArtifact::new_with_observations(vec![]);
        assert_eq!(
            validate(&candidate),
            Err(ValidationError::StructuralValidationFailed(
                "candidate artifact must reference one or more canonical observations".into()
            ))
        );
    }

    #[test]
    fn duplicate_observation_is_rejected() {
        let observation = test_observation();
        let candidate =
            CandidateArtifact::new_with_observations(vec![observation.clone(), observation]);
        assert_eq!(
            validate(&candidate),
            Err(ValidationError::StructuralValidationFailed(
                "candidate artifact must not reference the same observation more than once".into()
            ))
        );
    }
}
