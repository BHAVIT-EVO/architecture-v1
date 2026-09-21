//! Deterministic identity derivation for Artifact Acceptance.
//!
//! This module owns the Stage 3 identity derivation mechanism used by the
//! Artifact Acceptance Pipeline.
//!
//! Artifact Identity is derived only from canonical Observation evidence.
//! It does not depend on ObservationId, observed_at, Workspace state, or
//! higher-level interpretation.

use crate::artifact_id::ArtifactId;
use crate::candidate::CandidateArtifact;
use crate::errors::IdentityAssignmentError;
use evo_observation::evidence::FactValue;
use evo_observation::observation::Observation;
use std::collections::BTreeSet;

pub(crate) fn assign_identity(
    candidate: &CandidateArtifact,
) -> Result<ArtifactId, IdentityAssignmentError> {
    derive_artifact_id_from_observations(candidate.observations())
}

/// Derives the canonical Artifact Identity the Acceptance Pipeline would
/// assign to a body of canonical Observations.
///
/// This is the same deterministic fingerprint the pipeline's Stage 3 identity
/// assignment uses (IS-0005): identity is derived only from the set of
/// distinct canonical signatures, never from ObservationId, observed_at,
/// Workspace state, or higher-level interpretation. Exposing it lets
/// downstream consumers (for example the desktop shell's evidence view) link
/// persisted canonical Observations to the Artifact they establish without
/// reimplementing the identity rule.
pub fn derive_artifact_id_from_observations(
    observations: &[Observation],
) -> Result<ArtifactId, IdentityAssignmentError> {
    let mut signatures = BTreeSet::new();

    for observation in observations {
        signatures.insert(canonical_signature(observation)?);
    }

    let mut hash = fnv1a64(b"artifact-candidate");
    for signature in signatures {
        hash = hash_bytes(hash, signature.as_bytes());
    }

    let id = format!("artifact-{:016x}", hash);

    ArtifactId::new(id).map_err(IdentityAssignmentError::from)
}

pub fn canonical_signature(observation: &Observation) -> Result<String, IdentityAssignmentError> {
    let schema = observation.schema();
    let fact_name = schema
        .canonical_fact_name()
        .ok_or_else(invalid_candidate_identity)?;

    let fact = observation
        .evidence()
        .fact(fact_name)
        .ok_or_else(invalid_candidate_identity)?;

    let subject = match fact.value() {
        FactValue::Text(text) if !text.trim().is_empty() => text.clone(),
        _ => return Err(invalid_candidate_identity()),
    };

    Ok(format!(
        "{}|{}|{}|{}",
        schema.name(),
        schema.version(),
        fact_name,
        subject
    ))
}

fn invalid_candidate_identity() -> IdentityAssignmentError {
    IdentityAssignmentError::InvalidId(crate::artifact_id::ArtifactIdError::Empty)
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

fn fnv1a64(seed: &[u8]) -> u64 {
    hash_bytes(FNV_OFFSET_BASIS, seed)
}

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::candidate::{test_candidate_with_observations, test_observation_with_subject};
    use std::time::SystemTime;

    #[test]
    fn public_derivation_matches_pipeline_identity() {
        // The public derivation must agree with the pipeline's Stage 3
        // identity assignment for the same canonical Observations.
        let observation = test_observation_with_subject("shared-subject", SystemTime::UNIX_EPOCH);
        let candidate = test_candidate_with_observations(vec![observation.clone()]);

        let pipeline_id = assign_identity(&candidate).expect("pipeline identity succeeds");
        let public_id =
            derive_artifact_id_from_observations(&[observation]).expect("public derivation succeeds");
        assert_eq!(public_id, pipeline_id);
    }

    #[test]
    fn public_derivation_is_deterministic_and_subject_scoped() {
        let observation = test_observation_with_subject("subject-a", SystemTime::UNIX_EPOCH);
        let first =
            derive_artifact_id_from_observations(&[observation.clone()]).expect("first derivation");
        let second =
            derive_artifact_id_from_observations(&[observation]).expect("second derivation");
        assert_eq!(first, second);

        let other =
            derive_artifact_id_from_observations(&[test_observation_with_subject(
                "subject-b",
                SystemTime::UNIX_EPOCH,
            )])
            .expect("other subject derivation");
        assert_ne!(first, other);
    }

    #[test]
    fn public_derivation_ignores_observation_ids_and_timestamps() {
        // Identity is derived only from canonical signatures; differing
        // ObservationIds or observed_at must not change the Artifact.
        let first =
            derive_artifact_id_from_observations(&[test_observation_with_subject(
                "subject-x",
                SystemTime::UNIX_EPOCH,
            )])
            .expect("first");
        let later =
            derive_artifact_id_from_observations(&[test_observation_with_subject(
                "subject-x",
                SystemTime::UNIX_EPOCH
                    .checked_add(std::time::Duration::from_secs(999))
                    .unwrap(),
            )])
            .expect("later");
        assert_eq!(first, later);
    }
}
