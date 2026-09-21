//! Persistence orchestration for the Artifact Acceptance Pipeline.
//!
//! Stage 5 requests durable storage of an Accepted Artifact through the
//! storage boundary. This module keeps the pipeline orchestration separate
//! from the storage call itself.

use crate::artifact::Artifact;
use crate::errors::PersistenceError;
use evo_storage::{Storage, StorageObjectKind};

pub(crate) fn persist(artifact: &Artifact) -> Result<(), PersistenceError> {
    let storage = Storage::new();
    let record = artifact.id().as_str().as_bytes();

    storage
        .append(StorageObjectKind::Artifact, record)
        .map_err(|err| PersistenceError::WriteFailed(err.to_string()))
}
