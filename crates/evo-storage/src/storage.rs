//! Storage boundary.
//!
//! `Storage` defines Evo's backend-agnostic persistence boundary.
//! The boundary is intentionally minimal and infrastructure-only.

use crate::errors::StorageError;

/// Canonical persisted object categories defined by the Architecture.
///
/// These variants model only persistence categories. They do not model
/// business behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageObjectKind {
    /// Immutable append-only Observation evidence log.
    Observation,

    /// Persistent Artifact identity state.
    Artifact,

    /// Persistent Workspace state.
    Workspace,

    /// Persistent Workspace Attachment state.
    Attachment,

    /// Persistent immutable Workspace Snapshot state.
    Snapshot,

    /// Persistent Knowledge state.
    Knowledge,

    /// Persistent Decision log state.
    Decision,
}

/// Backend-agnostic storage service boundary.
///
/// `Storage` owns no backend state in this crate. Integrations are expected
/// to provide backend wiring outside this boundary crate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Storage;

impl Storage {
    /// Constructs a storage boundary value.
    pub fn new() -> Self {
        Self
    }

    /// Appends an immutable record for a canonical object category.
    ///
    /// # Parameters
    ///
    /// - `kind`: canonical persistence category.
    /// - `record`: opaque serialized bytes for one persisted record.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::EmptyRecord`] when `record` is empty.
    ///
    /// Returns [`StorageError::BackendNotConfigured`] because this crate is a
    /// boundary and does not bind a concrete backend.
    pub fn append(
        &self,
        kind: StorageObjectKind,
        record: &[u8],
    ) -> Result<(), StorageError> {
        let _ = kind;
        if record.is_empty() {
            return Err(StorageError::EmptyRecord);
        }
        Err(StorageError::BackendNotConfigured)
    }

    /// Reads all persisted records for a canonical object category.
    ///
    /// # Parameters
    ///
    /// - `kind`: canonical persistence category.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::BackendNotConfigured`] because this crate is a
    /// boundary and does not bind a concrete backend.
    pub fn read_all(
        &self,
        kind: StorageObjectKind,
    ) -> Result<Vec<Vec<u8>>, StorageError> {
        let _ = kind;
        Err(StorageError::BackendNotConfigured)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_new_and_default_are_equivalent() {
        let from_new = Storage::new();
        let from_default: Storage = Default::default();
        assert_eq!(from_new, from_default);
    }

    #[test]
    fn storage_is_copy_and_clone() {
        let a = Storage::new();
        let b = a;
        let c = a.clone();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn append_rejects_empty_records() {
        let storage = Storage::new();
        let error = storage
            .append(StorageObjectKind::Observation, &[])
            .unwrap_err();
        assert_eq!(error, StorageError::EmptyRecord);
    }

    #[test]
    fn append_reports_backend_not_configured_for_non_empty_records() {
        let storage = Storage::new();
        let error = storage
            .append(StorageObjectKind::Observation, b"record")
            .unwrap_err();
        assert_eq!(error, StorageError::BackendNotConfigured);
    }

    #[test]
    fn read_all_reports_backend_not_configured() {
        let storage = Storage::new();
        let error = storage.read_all(StorageObjectKind::Workspace).unwrap_err();
        assert_eq!(error, StorageError::BackendNotConfigured);
    }

    #[test]
    fn all_storage_object_kinds_are_constructible_and_distinct() {
        let kinds = [
            StorageObjectKind::Observation,
            StorageObjectKind::Artifact,
            StorageObjectKind::Workspace,
            StorageObjectKind::Attachment,
            StorageObjectKind::Snapshot,
            StorageObjectKind::Knowledge,
            StorageObjectKind::Decision,
        ];

        assert_eq!(kinds.len(), 7);
        for (index, kind) in kinds.iter().enumerate() {
            assert_eq!(kind, &kinds[index]);
        }
        assert_ne!(StorageObjectKind::Observation, StorageObjectKind::Artifact);
        assert_ne!(StorageObjectKind::Workspace, StorageObjectKind::Snapshot);
        assert_ne!(StorageObjectKind::Knowledge, StorageObjectKind::Decision);
    }
}
