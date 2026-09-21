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

    /// Canonical storage-root preparation or migration failed. The legacy
    /// source was left intact; nothing was partially installed.
    Migration { source: String },

    /// Tooling that fabricates Observations asked for a writable root while
    /// pointed at the canonical one, where the person's real history lives.
    ///
    /// Refused rather than filtered. Fabricated Observations travel the same
    /// acceptance path as real ones and are indistinguishable once written, so
    /// the only reliable isolation is to never write them there at all.
    FabricationAgainstCanonicalRoot,
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
            StorageError::Migration { source } => {
                write!(f, "storage root migration failed: {source}")
            }
            StorageError::FabricationAgainstCanonicalRoot => write!(
                f,
                "refusing to fabricate Observations against the canonical storage root; \
                 set EVO_STORAGE_ROOT to a scratch directory first"
            ),
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
        assert_ne!(
            StorageError::EmptyRecord,
            StorageError::BackendNotConfigured
        );
    }

    #[test]
    fn storage_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}

        takes_error(&StorageError::EmptyRecord);
        takes_error(&StorageError::BackendNotConfigured);
        takes_error(&StorageError::Migration {
            source: "copy failed".to_string(),
        });
    }

    #[test]
    fn migration_error_has_human_readable_message() {
        let message = StorageError::Migration {
            source: "copy observation.log: broken pipe".to_string(),
        }
        .to_string();
        assert!(message.contains("migration failed"));
        assert!(message.contains("observation.log"));
    }
}
