//! Domain error types for the evo-workspace crate.
//!
//! Every error type in this module represents a domain-level violation.
//!
//! No `String` errors.
//! No `anyhow` in domain models.
//!
//! Errors communicate domain meaning, not implementation detail.

// ── ConfidenceError ──────────────────────────────────────────────────────────

/// A violation of the `ConfidenceScore` invariant.
///
/// `ConfidenceScore` is bounded to `[0.0, 1.0]`.
///
/// This error is produced only when a value outside that range is supplied.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfidenceError {
    /// The supplied value is outside the valid range `[0.0, 1.0]`.
    OutOfRange { value: f32 },
}

impl std::fmt::Display for ConfidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidenceError::OutOfRange { value } => {
                write!(
                    f,
                    "confidence score {value} is out of range: must be in [0.0, 1.0]"
                )
            }
        }
    }
}

impl std::error::Error for ConfidenceError {}

// ── WorkspaceError ───────────────────────────────────────────────────────────

/// Errors produced by Workspace operations.
///
/// The canonical Workspace domain object currently defines no intrinsic
/// failure modes. Errors related to Workspace Formation, Replay,
/// Restoration, or Persistence belong to their respective architectural
/// layers and are intentionally excluded from this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {}
impl fmt::Display for WorkspaceError {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl std::error::Error for WorkspaceError {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_error_out_of_range_display() {
        let err = ConfidenceError::OutOfRange { value: 1.5 };
        let msg = err.to_string();
        assert!(msg.contains("1.5"));
        assert!(msg.contains("[0.0, 1.0]"));
    }

    #[test]
    fn confidence_error_equality() {
        let a = ConfidenceError::OutOfRange { value: 2.0 };
        let b = ConfidenceError::OutOfRange { value: 2.0 };
        assert_eq!(a, b);
    }
}
