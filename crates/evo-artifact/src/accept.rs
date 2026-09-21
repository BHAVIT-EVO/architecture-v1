//! Orchestration of the Artifact Acceptance Pipeline.
//!
//! This module coordinates the transformation of a canonical
//! [`evo_observation::observation::Observation`] into a permanent, immutable
//! [`Artifact`] (IS-0005).
//! It acts strictly as an orchestrator, containing no business rules,
//! validation logic, canonicalization logic, identity assignment logic, or
//! integrity logic of its own.

use crate::artifact::Artifact;
use crate::candidate::CandidateArtifact;
use crate::canonicalization::canonicalize;
use crate::errors::ArtifactError;
use crate::identity_assignment::assign_identity;
use crate::integrity::verify;
use crate::persistence::persist;
use crate::validation::validate;
use evo_observation::observation::Observation;

/// Accepts a canonical Observation through the Artifact Acceptance Pipeline
/// (IS-0005).
///
/// Coordinates the sequential execution of Validation, Canonicalization,
/// Identity Assignment, and Integrity Verification, then assembles the
/// accepted [`Artifact`].
///
/// # Arguments
///
/// * `observation` — The canonical Observation awaiting Artifact acceptance.
///
/// # Returns
///
/// A fully materialised, immutable [`Artifact`] upon success, or an
/// [`ArtifactError`] identifying the stage at which the pipeline rejected
/// the candidate (IS-0005 R-2).
pub fn accept(observation: Observation) -> Result<Artifact, ArtifactError> {
    accept_observations(vec![observation])
}

/// Accepts one or more canonical Observations through the Artifact Acceptance
/// Pipeline (IS-0005).
pub fn accept_observations(observations: Vec<Observation>) -> Result<Artifact, ArtifactError> {
    let candidate = CandidateArtifact::new_with_observations(observations);
    accept_candidate(candidate)
}

pub(crate) fn accept_candidate(candidate: CandidateArtifact) -> Result<Artifact, ArtifactError> {
    // Stage 1 — Validation
    validate(&candidate)?;

    // Stage 2 — Canonicalization
    let candidate = canonicalize(candidate)?;

    // Stage 3 — Identity Assignment.
    let id = assign_identity(&candidate)?;

    // Stage 4 — Integrity Verification
    let artifact = Artifact::new(id);
    verify(&artifact)?;

    // Stage 5 — Persistence
    persist(&artifact)?;

    // Stage 6 — Acceptance
    Ok(artifact)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::candidate::{test_candidate_with_observations, test_observation};
    use evo_storage::Storage;

    /// Isolates the artifact-acceptance persistence stage (IS-0005 Stage 5)
    /// from the canonical default storage root.
    ///
    /// Artifact Acceptance persists every accepted Artifact. Tests exercising
    /// the full pipeline must therefore redirect the thread-local storage root
    /// to a unique scratch location so `cargo test --workspace` never writes
    /// test artifacts into Evo's canonical storage.
    fn isolated_storage() -> impl Drop {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        Storage::with_thread_root(std::env::temp_dir().join(format!(
            "evo-artifact-acceptance-test-{nanos}"
        )))
    }

    #[test]
    fn accept_pipeline_succeeds_for_valid_candidate() {
        let _guard = isolated_storage();
        let result = accept(test_observation());
        assert!(result.is_ok());
    }

    #[test]
    fn accepted_artifact_receives_deterministic_identity() {
        let _guard = isolated_storage();
        let artifact_1 = accept(test_observation()).unwrap();
        let artifact_2 = accept(test_observation()).unwrap();

        assert_eq!(artifact_1.id(), artifact_2.id());
        assert!(!artifact_1.id().as_str().is_empty());
    }

    #[test]
    fn accept_constructs_candidate_internally() {
        let _guard = isolated_storage();
        let candidate = test_candidate_with_observations(vec![test_observation()]);
        let _ = accept_candidate(candidate);
    }

    #[test]
    fn identical_canonical_observations_produce_one_artifact_identity() {
        let _guard = isolated_storage();
        let first = crate::candidate::test_observation_with_identity(
            "123e4567-e89b-12d3-a456-426614174000",
            "same-subject",
            std::time::SystemTime::UNIX_EPOCH,
        );
        let second = crate::candidate::test_observation_with_identity(
            "123e4567-e89b-12d3-a456-426614174001",
            "same-subject",
            std::time::SystemTime::UNIX_EPOCH
                .checked_add(std::time::Duration::from_secs(60))
                .unwrap(),
        );

        let single = accept(first.clone()).unwrap();
        let multi = accept_observations(vec![first, second]).unwrap();

        assert_eq!(single.id(), multi.id());
    }

    #[test]
    fn observation_specific_fields_do_not_drive_identity() {
        let _guard = isolated_storage();
        let first = crate::candidate::test_observation_with_identity(
            "123e4567-e89b-12d3-a456-426614174010",
            "same-subject",
            std::time::SystemTime::UNIX_EPOCH,
        );
        let second = crate::candidate::test_observation_with_identity(
            "123e4567-e89b-12d3-a456-426614174011",
            "same-subject",
            std::time::SystemTime::UNIX_EPOCH
                .checked_add(std::time::Duration::from_secs(9_999))
                .unwrap(),
        );

        let first_artifact = accept(first).unwrap();
        let second_artifact = accept(second).unwrap();

        assert_eq!(first_artifact.id(), second_artifact.id());
    }

    #[test]
    fn different_canonical_subjects_do_not_merge() {
        let _guard = isolated_storage();
        let first = crate::candidate::test_observation_with_identity(
            "123e4567-e89b-12d3-a456-426614174020",
            "subject-a",
            std::time::SystemTime::UNIX_EPOCH,
        );
        let second = crate::candidate::test_observation_with_identity(
            "123e4567-e89b-12d3-a456-426614174021",
            "subject-b",
            std::time::SystemTime::UNIX_EPOCH,
        );

        let first_artifact = accept(first).unwrap();
        let second_artifact = accept(second).unwrap();

        assert_ne!(first_artifact.id(), second_artifact.id());
    }
}
