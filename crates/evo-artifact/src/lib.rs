//! evo-artifact
//!
//! Canonical Artifact Identity for Evo.
//!
//! This crate implements RFC-0002,
//! IS-0004 (Artifact Model),
//! and IS-0005 (Artifact Acceptance).

pub mod accept;
pub mod artifact;
pub mod artifact_id;
pub mod candidate;
pub(crate) mod canonicalization;
pub mod errors;
mod identity_assignment;
pub(crate) mod integrity;
mod persistence;
pub(crate) mod validation;

pub use identity_assignment::{canonical_signature, derive_artifact_id_from_observations};
