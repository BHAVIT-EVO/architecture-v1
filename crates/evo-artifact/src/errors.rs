//! Strongly typed error hierarchy for the Artifact Acceptance Pipeline.
//!
//! Provides distinct, composable error types for each stage of the Artifact
//! Acceptance Pipeline, strictly derived from IS-0005 §6 failure modes and
//! RFC-0002.
//!
//! # Pipeline Mapping
//!
//! | Stage | IS-0005 | Error Type |
//! |-------|---------|------------|
//! | 1 | Validation | [`ValidationError`] |
//! | 2 | Canonicalization | [`CanonicalizationError`] |
//! | 3 | Identity Assignment | [`IdentityAssignmentError`] |
//! | 4 | Integrity Verification | [`IntegrityError`] |
//! | 5 | Persistence | [`PersistenceError`] |
//!
//! All stage errors are wrapped by [`ArtifactError`], the aggregate type
//! returned by the Acceptance Pipeline (IS-0005 R-2).

use std::fmt;

use crate::artifact_id::ArtifactIdError;

// ── ValidationError ───────────────────────────────────────────────────────────

/// Errors that occur during Stage 1 — Validation (IS-0005 Stage 1, §6).
///
/// Validation verifies that a Candidate Artifact satisfies all structural
/// requirements required for acceptance. Rejection here is total: no
/// subsequent stage executes (IS-0005 §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The Candidate Artifact does not satisfy the structural requirements
    /// for acceptance (IS-0005 Stage 1).
    StructuralValidationFailed(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::StructuralValidationFailed(msg) => {
                write!(f, "artifact candidate validation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

// ── CanonicalizationError ─────────────────────────────────────────────────────

/// Errors that occur during Stage 2 — Canonicalization (IS-0005 Stage 2, §6).
///
/// Canonicalization produces one canonical representation of the Artifact
/// without altering the meaning of the identity hypothesis (IS-0005 Stage 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalizationError {
    /// Failed to transform the Candidate Artifact into its canonical
    /// representation.
    CanonicalizationFailed(String),
}

impl fmt::Display for CanonicalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CanonicalizationError::CanonicalizationFailed(msg) => {
                write!(f, "artifact canonicalization failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for CanonicalizationError {}

// ── IdentityAssignmentError ───────────────────────────────────────────────────

/// Errors that occur during Stage 3 — Identity Assignment (IS-0005 Stage 3,
/// §6).
///
/// Identity Assignment establishes the stable computational identity of the
/// accepted Artifact (IS-0005 Stage 3).
///
/// The only domain error currently defined for identity construction is
/// [`ArtifactIdError`], which is wrapped here to preserve error source
/// chaining.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityAssignmentError {
    /// The Artifact Identity could not be assigned because the provided
    /// identifier was invalid (IS-0005 Stage 3).
    InvalidId(ArtifactIdError),
}

impl fmt::Display for IdentityAssignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityAssignmentError::InvalidId(err) => {
                write!(f, "artifact identity assignment failed: {}", err)
            }
        }
    }
}

impl std::error::Error for IdentityAssignmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IdentityAssignmentError::InvalidId(err) => Some(err),
        }
    }
}

impl From<ArtifactIdError> for IdentityAssignmentError {
    fn from(err: ArtifactIdError) -> Self {
        IdentityAssignmentError::InvalidId(err)
    }
}

// ── IntegrityError ────────────────────────────────────────────────────────────

/// Errors that occur during Stage 4 — Integrity Verification (IS-0005 Stage 4,
/// §6).
///
/// Integrity verification confirms that the assembled Artifact satisfies every
/// acceptance invariant. It does not perform identity inference (IS-0005 Stage
/// 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    /// One or more acceptance invariants were not satisfied (IS-0005 Stage 4).
    VerificationFailed(String),
}

impl fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrityError::VerificationFailed(msg) => {
                write!(f, "artifact integrity verification failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for IntegrityError {}

// ── PersistenceError ──────────────────────────────────────────────────────────

/// Errors that occur during Stage 5 — Persistence (IS-0005 Stage 5, §6).
///
/// Persistence occurs only after all preceding stages have succeeded. An
/// Artifact that fails to persist is rejected; no partial acceptance is
/// permitted (IS-0005 R-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    /// Failed to durably write the Artifact to the persistence layer.
    WriteFailed(String),
    /// The persistence layer could not complete the commit.
    CommitFailed(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceError::WriteFailed(msg) => {
                write!(f, "artifact persistence write failed: {}", msg)
            }
            PersistenceError::CommitFailed(msg) => {
                write!(f, "artifact persistence commit failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for PersistenceError {}

// ── ArtifactError ─────────────────────────────────────────────────────────────

/// Aggregate error representing any failure during the Artifact Acceptance
/// Pipeline.
///
/// Maps directly to the atomic rejection decision specified in IS-0005 R-2:
/// acceptance either produces one canonical Artifact or rejects the Candidate.
/// Every variant identifies the stage at which rejection occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactError {
    /// Stage 1: Candidate validation failure.
    Validation(ValidationError),
    /// Stage 2: Canonicalization failure.
    Canonicalization(CanonicalizationError),
    /// Stage 3: Identity assignment failure.
    IdentityAssignment(IdentityAssignmentError),
    /// Stage 4: Integrity verification failure.
    Integrity(IntegrityError),
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactError::Validation(err) => {
                write!(f, "artifact rejected at validation stage: {}", err)
            }
            ArtifactError::Canonicalization(err) => {
                write!(f, "artifact rejected at canonicalization stage: {}", err)
            }
            ArtifactError::IdentityAssignment(err) => {
                write!(f, "artifact rejected at identity assignment stage: {}", err)
            }
            ArtifactError::Integrity(err) => {
                write!(f, "artifact rejected at integrity stage: {}", err)
            }
        }
    }
}

impl std::error::Error for ArtifactError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ArtifactError::Validation(err) => Some(err),
            ArtifactError::Canonicalization(err) => Some(err),
            ArtifactError::IdentityAssignment(err) => Some(err),
            ArtifactError::Integrity(err) => Some(err),
        }
    }
}

impl From<ValidationError> for ArtifactError {
    fn from(err: ValidationError) -> Self {
        ArtifactError::Validation(err)
    }
}

impl From<CanonicalizationError> for ArtifactError {
    fn from(err: CanonicalizationError) -> Self {
        ArtifactError::Canonicalization(err)
    }
}

impl From<IdentityAssignmentError> for ArtifactError {
    fn from(err: IdentityAssignmentError) -> Self {
        ArtifactError::IdentityAssignment(err)
    }
}

impl From<IntegrityError> for ArtifactError {
    fn from(err: IntegrityError) -> Self {
        ArtifactError::Integrity(err)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ValidationError ───────────────────────────────────────────────────────

    #[test]
    fn validation_error_structural_validation_failed_displays_correctly() {
        let err = ValidationError::StructuralValidationFailed("missing required field".into());
        assert_eq!(
            err.to_string(),
            "artifact candidate validation failed: missing required field"
        );
    }

    #[test]
    fn validation_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&ValidationError::StructuralValidationFailed("x".into()));
    }

    #[test]
    fn validation_error_has_no_source() {
        use std::error::Error;
        let err = ValidationError::StructuralValidationFailed("x".into());
        assert!(err.source().is_none());
    }

    #[test]
    fn validation_error_clones_correctly() {
        let err = ValidationError::StructuralValidationFailed("field".into());
        assert_eq!(err.clone(), err);
    }

    #[test]
    fn validation_error_equality() {
        let a = ValidationError::StructuralValidationFailed("a".into());
        let b = ValidationError::StructuralValidationFailed("a".into());
        let c = ValidationError::StructuralValidationFailed("b".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ── CanonicalizationError ─────────────────────────────────────────────────

    #[test]
    fn canonicalization_error_canonicalization_failed_displays_correctly() {
        let err = CanonicalizationError::CanonicalizationFailed("invalid encoding".into());
        assert_eq!(
            err.to_string(),
            "artifact canonicalization failed: invalid encoding"
        );
    }

    #[test]
    fn canonicalization_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&CanonicalizationError::CanonicalizationFailed("x".into()));
    }

    #[test]
    fn canonicalization_error_has_no_source() {
        use std::error::Error;
        let err = CanonicalizationError::CanonicalizationFailed("x".into());
        assert!(err.source().is_none());
    }

    #[test]
    fn canonicalization_error_clones_correctly() {
        let err = CanonicalizationError::CanonicalizationFailed("enc".into());
        assert_eq!(err.clone(), err);
    }

    #[test]
    fn canonicalization_error_equality() {
        let a = CanonicalizationError::CanonicalizationFailed("a".into());
        let b = CanonicalizationError::CanonicalizationFailed("a".into());
        let c = CanonicalizationError::CanonicalizationFailed("b".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ── IdentityAssignmentError ───────────────────────────────────────────────

    #[test]
    fn identity_assignment_error_invalid_id_displays_correctly() {
        let id_err = ArtifactIdError::Empty;
        let err = IdentityAssignmentError::InvalidId(id_err);
        assert_eq!(
            err.to_string(),
            "artifact identity assignment failed: artifact identifier must not be empty"
        );
    }

    #[test]
    fn identity_assignment_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&IdentityAssignmentError::InvalidId(ArtifactIdError::Empty));
    }

    #[test]
    fn identity_assignment_error_source_chains_to_artifact_id_error() {
        use std::error::Error;
        let err = IdentityAssignmentError::InvalidId(ArtifactIdError::Empty);
        let source = err.source().expect("source must be present");
        assert_eq!(source.to_string(), ArtifactIdError::Empty.to_string());
    }

    #[test]
    fn identity_assignment_error_converts_from_artifact_id_error() {
        let id_err = ArtifactIdError::Empty;
        let err: IdentityAssignmentError = id_err.into();
        assert_eq!(err, IdentityAssignmentError::InvalidId(ArtifactIdError::Empty));
    }

    #[test]
    fn identity_assignment_error_clones_correctly() {
        let err = IdentityAssignmentError::InvalidId(ArtifactIdError::Empty);
        assert_eq!(err.clone(), err);
    }

    #[test]
    fn identity_assignment_error_equality() {
        let a = IdentityAssignmentError::InvalidId(ArtifactIdError::Empty);
        let b = IdentityAssignmentError::InvalidId(ArtifactIdError::Empty);
        assert_eq!(a, b);
    }

    // ── IntegrityError ────────────────────────────────────────────────────────

    #[test]
    fn integrity_error_verification_failed_displays_correctly() {
        let err = IntegrityError::VerificationFailed("invariant violated".into());
        assert_eq!(
            err.to_string(),
            "artifact integrity verification failed: invariant violated"
        );
    }

    #[test]
    fn integrity_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&IntegrityError::VerificationFailed("x".into()));
    }

    #[test]
    fn integrity_error_has_no_source() {
        use std::error::Error;
        let err = IntegrityError::VerificationFailed("x".into());
        assert!(err.source().is_none());
    }

    #[test]
    fn integrity_error_clones_correctly() {
        let err = IntegrityError::VerificationFailed("check".into());
        assert_eq!(err.clone(), err);
    }

    #[test]
    fn integrity_error_equality() {
        let a = IntegrityError::VerificationFailed("a".into());
        let b = IntegrityError::VerificationFailed("a".into());
        let c = IntegrityError::VerificationFailed("b".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ── ArtifactError — stage wrapping ────────────────────────────────────────

    #[test]
    fn artifact_error_validation_stage_displays_correctly() {
        let err: ArtifactError =
            ValidationError::StructuralValidationFailed("bad structure".into()).into();
        assert_eq!(
            err.to_string(),
            "artifact rejected at validation stage: artifact candidate validation failed: bad structure"
        );
    }

    #[test]
    fn artifact_error_canonicalization_stage_displays_correctly() {
        let err: ArtifactError =
            CanonicalizationError::CanonicalizationFailed("encoding error".into()).into();
        assert_eq!(
            err.to_string(),
            "artifact rejected at canonicalization stage: artifact canonicalization failed: encoding error"
        );
    }

    #[test]
    fn artifact_error_identity_assignment_stage_displays_correctly() {
        let err: ArtifactError =
            IdentityAssignmentError::InvalidId(ArtifactIdError::Empty).into();
        assert_eq!(
            err.to_string(),
            "artifact rejected at identity assignment stage: artifact identity assignment failed: artifact identifier must not be empty"
        );
    }

    #[test]
    fn artifact_error_integrity_stage_displays_correctly() {
        let err: ArtifactError =
            IntegrityError::VerificationFailed("invariant broken".into()).into();
        assert_eq!(
            err.to_string(),
            "artifact rejected at integrity stage: artifact integrity verification failed: invariant broken"
        );
    }

    // ── ArtifactError — From conversions ──────────────────────────────────────

    #[test]
    fn artifact_error_converts_from_validation_error() {
        let stage_err = ValidationError::StructuralValidationFailed("x".into());
        let err: ArtifactError = stage_err.clone().into();
        assert_eq!(err, ArtifactError::Validation(stage_err));
    }

    #[test]
    fn artifact_error_converts_from_canonicalization_error() {
        let stage_err = CanonicalizationError::CanonicalizationFailed("x".into());
        let err: ArtifactError = stage_err.clone().into();
        assert_eq!(err, ArtifactError::Canonicalization(stage_err));
    }

    #[test]
    fn artifact_error_converts_from_identity_assignment_error() {
        let stage_err = IdentityAssignmentError::InvalidId(ArtifactIdError::Empty);
        let err: ArtifactError = stage_err.clone().into();
        assert_eq!(err, ArtifactError::IdentityAssignment(stage_err));
    }

    #[test]
    fn artifact_error_converts_from_integrity_error() {
        let stage_err = IntegrityError::VerificationFailed("x".into());
        let err: ArtifactError = stage_err.clone().into();
        assert_eq!(err, ArtifactError::Integrity(stage_err));
    }

    // ── ArtifactError — std::error::Error ────────────────────────────────────

    #[test]
    fn artifact_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&ArtifactError::Validation(
            ValidationError::StructuralValidationFailed("x".into()),
        ));
        takes_error(&ArtifactError::Canonicalization(
            CanonicalizationError::CanonicalizationFailed("x".into()),
        ));
        takes_error(&ArtifactError::IdentityAssignment(
            IdentityAssignmentError::InvalidId(ArtifactIdError::Empty),
        ));
        takes_error(&ArtifactError::Integrity(
            IntegrityError::VerificationFailed("x".into()),
        ));

    }

    #[test]
    fn artifact_error_source_chains_to_stage_error() {
        use std::error::Error;

        let stage_err = ValidationError::StructuralValidationFailed("bad".into());
        let agg_err = ArtifactError::Validation(stage_err.clone());
        let source = agg_err.source().expect("source must be present");
        assert_eq!(source.to_string(), stage_err.to_string());
    }

    #[test]
    fn artifact_error_identity_assignment_source_chains_three_levels() {
        use std::error::Error;

        // ArtifactError → IdentityAssignmentError → ArtifactIdError
        let agg_err = ArtifactError::IdentityAssignment(
            IdentityAssignmentError::InvalidId(ArtifactIdError::Empty),
        );

        let level_1 = agg_err.source().expect("level 1 source must be present");
        assert_eq!(
            level_1.to_string(),
            IdentityAssignmentError::InvalidId(ArtifactIdError::Empty).to_string()
        );

        let level_2 = level_1.source().expect("level 2 source must be present");
        assert_eq!(level_2.to_string(), ArtifactIdError::Empty.to_string());
    }

    // ── ArtifactError — Clone and Eq ──────────────────────────────────────────

    #[test]
    fn artifact_error_clones_correctly() {
        let err = ArtifactError::Integrity(IntegrityError::VerificationFailed(
            "clone-test".into(),
        ));
        assert_eq!(err.clone(), err);
    }
}
