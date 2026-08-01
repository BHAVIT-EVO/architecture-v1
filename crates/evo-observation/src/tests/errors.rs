use crate::errors::{
    CanonicalizationError, IntegrityError, ObservationError, PersistenceError, ValidationError,
};

use crate::observation_schema::SchemaError;
#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_validation_error_display_formatting() {
        let err = ValidationError::UnknownSchema {
            name: "app_focus".to_string(),
            version: 1,
        };
        assert_eq!(err.to_string(), "unknown observation schema: app_focus/1");

        let err = ValidationError::InvalidStructure("missing field x".to_string());
        assert_eq!(
            err.to_string(),
            "structural validation failed: missing field x"
        );

        let err = ValidationError::MissingProvenance("no source provided".to_string());
        assert_eq!(
            err.to_string(),
            "missing required provenance: no source provided"
        );
    }

    #[test]
    fn test_canonicalization_error_display_formatting() {
        let err = CanonicalizationError::EncodingFailed("utf8 error".to_string());
        assert_eq!(
            err.to_string(),
            "canonicalization encoding failed: utf8 error"
        );

        let err = CanonicalizationError::InvalidFormat("bad payload".to_string());
        assert_eq!(
            err.to_string(),
            "canonicalization format invalid: bad payload"
        );
    }

    #[test]
    fn test_integrity_error_display_formatting() {
        let err = IntegrityError::ChecksumMismatch {
            expected: "abc".to_string(),
            actual: "xyz".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "integrity checksum mismatch: expected abc, got xyz"
        );

        let err = IntegrityError::VerificationFailed("corrupted bytes".to_string());
        assert_eq!(
            err.to_string(),
            "integrity verification failed: corrupted bytes"
        );
    }

    #[test]
    fn test_persistence_error_display_formatting() {
        let err = PersistenceError::WriteFailed("disk full".to_string());
        assert_eq!(err.to_string(), "persistence write failed: disk full");

        let err = PersistenceError::StorageUnavailable("connection timeout".to_string());
        assert_eq!(
            err.to_string(),
            "persistence storage unavailable: connection timeout"
        );
    }

    #[test]
    fn test_observation_error_stage_wrapping_display() {
        let val_err = ValidationError::InvalidStructure("bad fact".to_string());
        let obs_err: ObservationError = val_err.into();

        assert_eq!(
            obs_err.to_string(),
            "observation rejected at validation stage: structural validation failed: bad fact"
        );
    }

    #[test]
    fn test_error_source_chaining() {
        let schema_err = SchemaError::EmptyName;
        let val_err = ValidationError::from(schema_err.clone());
        let obs_err = ObservationError::from(val_err.clone());

        assert!(obs_err.source().is_some());
        let source_err = obs_err.source().unwrap();
        assert_eq!(source_err.to_string(), val_err.to_string());

        let leaf_err = source_err.source().unwrap();
        assert_eq!(leaf_err.to_string(), schema_err.to_string());
    }

    #[test]
    fn test_error_trait_bounds_and_conversions() {
        fn takes_std_error(_: &dyn Error) {}

        let err =
            ObservationError::Validation(ValidationError::InvalidStructure("test".to_string()));
        takes_std_error(&err);

        let err = CanonicalizationError::InvalidFormat("test".to_string());
        takes_std_error(&err);

        let err = IntegrityError::VerificationFailed("test".to_string());
        takes_std_error(&err);

        let err = PersistenceError::WriteFailed("test".to_string());
        takes_std_error(&err);
    }

    #[test]
    fn test_equality_and_cloning() {
        let err1 = ObservationError::Integrity(IntegrityError::ChecksumMismatch {
            expected: "a".to_string(),
            actual: "b".to_string(),
        });
        let err2 = err1.clone();

        assert_eq!(err1, err2);
    }
}
