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
//! This crate therefore exposes only a backend-agnostic service boundary and
//! does not implement a concrete storage engine.
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
//! - concrete storage backends;
//! - migration or replay policy.

mod errors;
mod storage;

pub use errors::StorageError;
pub use storage::Storage;
pub use storage::StorageObjectKind;
