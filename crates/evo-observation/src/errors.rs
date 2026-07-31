//! Strongly typed error hierarchy for the Observation pipeline.
//!
//! Provides distinct, composable error types for each stage of the Observation
//! acceptance pipeline, strictly adhering to IS-0001 §10 failure modes and RFC-0001.

use std::fmt;

pub use crate::evidence::FactError;
use crate::observation_schema::{ObservationSchema, SchemaError};
pub use crate::provenance::SourceError;
// ── ValidationError ───────────────────────────────────────────────────────────

/// Errors that occur during stage-1 Validation (IS-0001 §3 R-2, §10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The referenced schema is unknown to Evo (IS-0001 §10).
    UnknownSchema(ObservationSchema),
    /// Structural validation failed against the schema specification (IS-0001 §10).
    InvalidStructure(String),
    /// Required provenance information is missing or incomplete (IS-0001 §10).
    MissingProvenance(String),
    /// Schema identity error wrapped from model construction.
    Schema(SchemaError),
    /// Observed fact error wrapped from model construction.
    Fact(FactError),
    /// Provenance source error wrapped from model construction.
    Source(SourceError),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Bug 3 fix: was struct-destructured `{ name, version }` — variant is a tuple
            ValidationError::UnknownSchema(schema) => {
                write!(f, "unknown observation schema: {}", schema)
            }
            ValidationError::InvalidStructure(msg) => {
                write!(f, "structural validation failed: {}", msg)
            }
            ValidationError::MissingProvenance(msg) => {
                write!(f, "missing required provenance: {}", msg)
            }
            ValidationError::Schema(err) => write!(f, "schema error: {}", err),
            ValidationError::Fact(err) => write!(f, "fact error: {}", err),
            ValidationError::Source(err) => write!(f, "source error: {}", err),
        }
    }
}

impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValidationError::Schema(err) => Some(err),
            ValidationError::Fact(err) => Some(err),
            ValidationError::Source(err) => Some(err),
            _ => None,
        }
    }
}

impl From<SchemaError> for ValidationError {
    fn from(err: SchemaError) -> Self {
        ValidationError::Schema(err)
    }
}

impl From<FactError> for ValidationError {
    fn from(err: FactError) -> Self {
        ValidationError::Fact(err)
    }
}

impl From<SourceError> for ValidationError {
    fn from(err: SourceError) -> Self {
        ValidationError::Source(err)
    }
}

// ── CanonicalizationError ─────────────────────────────────────────────────────

/// Errors that occur during stage-2 Canonicalization (IS-0001 §3 R-3, §10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalizationError {
    /// Failed to transform evidence into canonical representation.
    NormalizationFailed(String),
    /// Unrecognized or invalid structural format during canonicalization.
    NonCanonical(String),
}

impl fmt::Display for CanonicalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CanonicalizationError::NormalizationFailed(msg) => {
                write!(f, "canonicalization normalization failed: {}", msg)
            }
            CanonicalizationError::NonCanonical(msg) => {
                write!(f, "canonicalization format invalid: {}", msg)
            }
        }
    }
}

impl std::error::Error for CanonicalizationError {}

// ── IntegrityError ────────────────────────────────────────────────────────────

/// Errors that occur during stage-4 Integrity Verification (IS-0001 §3 R-6, §10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    // Bug 4 fix: two variants were both named `VerificationFailed` — compile error.
    // Renamed the struct form to `ChecksumMismatch` to distinguish it.
    /// Integrity verification failed due to a checksum or hash mismatch.
    ChecksumMismatch { expected: String, actual: String },
    /// General integrity verification failure.
    VerificationFailed(String),
}

impl fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrityError::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "integrity verification failed: expected {}, got {}",
                    expected, actual
                )
            }
            IntegrityError::VerificationFailed(msg) => {
                write!(f, "integrity verification failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for IntegrityError {}

// ── PersistenceError ──────────────────────────────────────────────────────────

/// Errors that occur during stage-5 Persistence (IS-0001 §3 R-7, §10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    /// Failed to durably write the observation to persistence storage.
    WriteFailed(String),
    /// Persistence storage target is unavailable or unreachable.
    CommitFailed(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceError::WriteFailed(msg) => {
                write!(f, "persistence write failed: {}", msg)
            }
            PersistenceError::CommitFailed(msg) => {
                write!(f, "persistence commit failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for PersistenceError {}

// ── ObservationError ──────────────────────────────────────────────────────────

/// Aggregate error representing any failure during the Observation acceptance pipeline.
///
/// Maps directly to the rejection decision specified in IS-0001 §3 R-8 and §10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationError {
    /// Stage 1: Candidate validation failure.
    Validation(ValidationError),
    /// Stage 2: Canonicalization failure.
    Canonicalization(CanonicalizationError),
    /// Stage 4: Integrity verification failure.
    Integrity(IntegrityError),
    /// Stage 5: Durable persistence failure.
    Persistence(PersistenceError),
}

impl fmt::Display for ObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObservationError::Validation(err) => {
                write!(f, "observation rejected at validation stage: {}", err)
            }
            ObservationError::Canonicalization(err) => {
                write!(f, "observation rejected at canonicalization stage: {}", err)
            }
            ObservationError::Integrity(err) => {
                write!(f, "observation rejected at integrity stage: {}", err)
            }
            ObservationError::Persistence(err) => {
                write!(f, "observation rejected at persistence stage: {}", err)
            }
        }
    }
}

impl std::error::Error for ObservationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ObservationError::Validation(err) => Some(err),
            ObservationError::Canonicalization(err) => Some(err),
            ObservationError::Integrity(err) => Some(err),
            ObservationError::Persistence(err) => Some(err),
        }
    }
}

impl From<ValidationError> for ObservationError {
    fn from(err: ValidationError) -> Self {
        ObservationError::Validation(err)
    }
}

impl From<CanonicalizationError> for ObservationError {
    fn from(err: CanonicalizationError) -> Self {
        ObservationError::Canonicalization(err)
    }
}

impl From<IntegrityError> for ObservationError {
    fn from(err: IntegrityError) -> Self {
        ObservationError::Integrity(err)
    }
}

impl From<PersistenceError> for ObservationError {
    fn from(err: PersistenceError) -> Self {
        ObservationError::Persistence(err)
    }
}
