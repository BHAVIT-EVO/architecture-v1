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
pub mod canonicalization;
pub mod errors;
pub mod integrity;
pub mod validation;