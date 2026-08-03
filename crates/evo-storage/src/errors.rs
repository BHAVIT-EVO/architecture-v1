//! Error types for the evo-storage crate.
//!
//! `evo-storage` is an infrastructure boundary. Errors in this crate describe
//! boundary-level failures only.

use std::fmt;

/// Errors produced by the storage boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// The supplied record payload was empty.
    EmptyRecord,

    /// No concrete backend has been configured for this boundary.
    BackendNotConfigured,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::EmptyRecord => {
                write!(f, "storage record payload must not be empty")
            }
            StorageError::BackendNotConfigured => {
                write!(f, "storage backend is not configured")
            }
        }
    }
}

impl std::error::Error for StorageError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_record_error_has_human_readable_message() {
        let message = StorageError::EmptyRecord.to_string();
        assert!(!message.is_empty());
        assert!(message.contains("must not be empty"));
    }

    #[test]
    fn backend_not_configured_error_has_human_readable_message() {
        let message = StorageError::BackendNotConfigured.to_string();
        assert!(!message.is_empty());
        assert!(message.contains("not configured"));
    }

    #[test]
    fn storage_error_is_comparable() {
        assert_eq!(StorageError::EmptyRecord, StorageError::EmptyRecord);
        assert_eq!(
            StorageError::BackendNotConfigured,
            StorageError::BackendNotConfigured
        );
        assert_ne!(StorageError::EmptyRecord, StorageError::BackendNotConfigured);
    }

    #[test]
    fn storage_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}

        takes_error(&StorageError::EmptyRecord);
        takes_error(&StorageError::BackendNotConfigured);
    }
}
