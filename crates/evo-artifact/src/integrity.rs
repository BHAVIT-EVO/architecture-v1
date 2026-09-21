//! Integrity Verification of the Artifact Entity.
//!
//! This module implements Stage 4 of the Artifact Acceptance Pipeline
//! (IS-0005 Stage 4). It is responsible for performing read-only structural
//! verification of a fully assembled [`Artifact`] prior to persistence.
//!
//! Integrity Verification confirms that a fully constructed [`Artifact`]
//! satisfies the structural invariants required before persistence.

use crate::artifact::Artifact;
use crate::errors::IntegrityError;

/// Performs read-only structural integrity verification on an Artifact
/// (IS-0005 Stage 4).
///
/// Returns `Ok(())` if the Artifact satisfies all acceptance invariants, or an
/// [`IntegrityError`] if any invariant is violated.
pub fn verify(artifact: &Artifact) -> Result<(), IntegrityError> {
    // // The frozen architecture intentionally exposes only the Artifact itself
    // during Stage 4. Therefore this implementation verifies only invariants
    // that are observable from the current Artifact representation.
    // Additional integrity checks should only be added when new invariants
    // become part of the frozen specifications.

    if artifact.id().as_str().trim().is_empty() {
        return Err(IntegrityError::VerificationFailed(
            "artifact must contain exactly one non-empty artifact identity".into(),
        ));
    }

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::artifact::Artifact;
    use crate::artifact_id::ArtifactId;

    fn test_artifact() -> Artifact {
        let id = ArtifactId::new("test-artifact").unwrap();
        Artifact::new(id)
    }

    #[test]
    fn verify_integrity_succeeds_for_valid_artifact() {
        let artifact = test_artifact();
        let result = verify(&artifact);
        assert!(result.is_ok());
    }
}
