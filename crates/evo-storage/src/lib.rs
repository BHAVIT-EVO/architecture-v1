//! evo-storage — Backend-Agnostic Persistence Boundary.
//!
//! This crate defines Evo's storage boundary for canonical architectural
//! objects.
//!
//! # Architectural Scope
//!
//! The Architecture (Section 6: Storage Philosophy) requires:
//!
//! - immutable, append-only Observation evidence;
//! - persisted canonical derived state used for restoration speed and
//!   explainability;
//! - backend independence.
//!
//! This crate exposes the storage service boundary used by the architecture's
//! canonical persistence layer.
//!
//! # Public Computational Surface
//!
//! - [`Storage`] — persistence service boundary.
//! - [`StorageObjectKind`] — canonical persisted object categories.
//! - [`StorageError`] — storage-boundary errors.
//!
//! # Non-Responsibilities
//!
//! This crate does not define:
//!
//! - Observation, Artifact, Workspace, Knowledge, History, Retrieval,
//!   or Restoration business logic;
//! - serialization formats;
//! - replay policy.
//!
//! This crate does own canonical storage-root resolution and the single
//! legacy-root migration performed at startup ([`root`]); it defines no
//! record-level migration or versioning framework beyond that.

mod errors;
mod root;
mod storage;

pub use errors::StorageError;
pub use root::{
    canonical_storage_root, default_storage_root, fabrication_root, legacy_storage_root,
    migrate_legacy_root, prepare_storage_root, MigrationOutcome,
};
pub use storage::Storage;
pub use storage::StorageObjectKind;
