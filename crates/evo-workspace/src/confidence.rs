//! Confidence Score.
//!
//! `ConfidenceScore` represents the evidential strength of an Attachment.
//!
//! # IS-0011 Invariants
//!
//! - W-6: Every Attachment SHALL contain exactly one Confidence Score.
//! - W-7: Confidence SHALL represent evidential strength only.
//!         Confidence SHALL NEVER represent importance, priority, or value.
//!
//! `ConfidenceScore` is bounded to `[0.0, 1.0]`.
//!
//! Construction fails if the supplied value violates this invariant.
//!
//! `ConfidenceScore` is immutable after construction.

use crate::errors::ConfidenceError;

// ── ConfidenceScore ───────────────────────────────────────────────────────────

/// The evidential strength of an Attachment relationship.
///
/// Bounded to `[0.0, 1.0]` (IS-0011 W-6, W-7).
///
/// `ConfidenceScore` represents only evidential strength.
///
/// # Invariants
///
/// - Value is always in `[0.0, 1.0]`.
/// - Immutable after construction.
///
/// # Non-Responsibilities
///
/// - Does **not** represent importance.
/// - Does **not** represent priority.
/// - Does **not** represent user preference.
/// - Does **not** represent value judgements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfidenceScore(f32);

impl ConfidenceScore {
    /// The minimum valid confidence score: `0.0`.
    pub const MIN: f32 = 0.0;

    /// The maximum valid confidence score: `1.0`.
    pub const MAX: f32 = 1.0;

    /// Constructs a `ConfidenceScore` from the supplied value.
    ///
    /// # Errors
    ///
    /// Returns `ConfidenceError::OutOfRange` if `value` is not in `[0.0, 1.0]`.
    pub fn new(value: f32) -> Result<Self, ConfidenceError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(ConfidenceError::OutOfRange { value });
        }
        Ok(Self(value))
    }

    /// Returns the raw `f32` value of this confidence score.
    pub fn value(&self) -> f32 {
        self.0
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ConfidenceError;

    #[test]
    fn zero_is_valid() {
        let score = ConfidenceScore::new(0.0).unwrap();
        assert_eq!(score.value(), 0.0);
    }

    #[test]
    fn one_is_valid() {
        let score = ConfidenceScore::new(1.0).unwrap();
        assert_eq!(score.value(), 1.0);
    }

    #[test]
    fn midpoint_is_valid() {
        let score = ConfidenceScore::new(0.5).unwrap();
        assert_eq!(score.value(), 0.5);
    }

    #[test]
    fn below_zero_is_rejected() {
        let err = ConfidenceScore::new(-0.1).unwrap_err();
        assert_eq!(err, ConfidenceError::OutOfRange { value: -0.1 });
    }

    #[test]
    fn above_one_is_rejected() {
        let err = ConfidenceScore::new(1.1).unwrap_err();
        assert_eq!(err, ConfidenceError::OutOfRange { value: 1.1 });
    }

    #[test]
    fn clone_preserves_value() {
        let score = ConfidenceScore::new(0.75).unwrap();
        let cloned = score;
        assert_eq!(score.value(), cloned.value());
    }

    #[test]
    fn equality() {
        let a = ConfidenceScore::new(0.3).unwrap();
        let b = ConfidenceScore::new(0.3).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn nan_is_rejected() {
        // f32::NAN is not >= 0.0, so it fails the lower-bound check.
        let err = ConfidenceScore::new(f32::NAN);
        assert!(err.is_err());
    }

    #[test]
    fn infinity_is_rejected() {
        let err = ConfidenceScore::new(f32::INFINITY).unwrap_err();
        assert!(matches!(err, ConfidenceError::OutOfRange { .. }));
    }

    #[test]
    fn negative_infinity_is_rejected() {
        let err = ConfidenceScore::new(f32::NEG_INFINITY).unwrap_err();
        assert!(matches!(err, ConfidenceError::OutOfRange { .. }));
    }
}
