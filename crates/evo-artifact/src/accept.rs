//! Orchestration of the Artifact Acceptance Pipeline.
//!
//! This module coordinates the transformation of a transient
//! [`CandidateArtifact`] into a permanent, immutable [`Artifact`] (IS-0005).
//! It acts strictly as an orchestrator, containing no business rules,
//! validation logic, canonicalization logic, or integrity logic of its own.

use crate::artifact::Artifact;
use crate::artifact_id::ArtifactId;
use crate::candidate::CandidateArtifact;
use crate::canonicalization::canonicalize;
use crate::errors::ArtifactError;
use crate::integrity::verify;
use crate::validation::validate;

/// Accepts a Candidate Artifact through the Artifact Acceptance Pipeline
/// (IS-0005).
///
/// Coordinates the sequential execution of Validation, Canonicalization,
/// Identity Assignment, and Integrity Verification, then assembles the
/// accepted [`Artifact`].
///
/// # Arguments
///
/// * `candidate` — The transient Candidate Artifact awaiting acceptance.
/// * `id` — The [`ArtifactId`] supplied to the Acceptance Pipeline.
///
///   The current implementation receives canonical Artifact Identity as an
///   input. The mechanism by which canonical Artifact Identity is established
///   is outside the scope of this crate and is defined by IS-0005 Stage 3.
///
/// # Returns
///
/// A fully materialised, immutable [`Artifact`] upon success, or an
/// [`ArtifactError`] identifying the stage at which the pipeline rejected
/// the candidate (IS-0005 R-2).
pub fn accept(candidate: CandidateArtifact, id: ArtifactId) -> Result<Artifact, ArtifactError> {
    // Stage 1 — Validation
    validate(&candidate)?;

    // Stage 2 — Canonicalization
    let candidate = canonicalize(candidate)?;

    // Stage 3 — Identity Assignment.
    //
    // The current implementation receives canonical Artifact Identity as an
    // input. The mechanism that establishes this identity exists outside this
    // crate.
    let _ = candidate;

    // Stage 4 — Integrity Verification
    let artifact = Artifact::new(id);
    verify(&artifact)?;

    // Stage 5 — Acceptance
    Ok(artifact)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::artifact_id::ArtifactId;
    use crate::candidate::CandidateArtifact;

    fn test_id() -> ArtifactId {
        ArtifactId::new("test-artifact").unwrap()
    }

    #[test]
    fn accept_pipeline_succeeds_for_valid_candidate() {
        let candidate = CandidateArtifact::new();
        let result = accept(candidate, test_id());
        assert!(result.is_ok());
    }

    #[test]
    fn accepted_artifact_carries_assigned_id() {
        let id = test_id();
        let candidate = CandidateArtifact::new();
        let artifact = accept(candidate, id.clone()).unwrap();
        assert_eq!(artifact.id(), &id);
    }

    #[test]
    fn pipeline_consumes_candidate() {
        // CandidateArtifact does not implement Copy.
        // accept takes it by value; this confirms move semantics compile.
        let candidate = CandidateArtifact::new();
        let _ = accept(candidate, test_id());
    }
}
