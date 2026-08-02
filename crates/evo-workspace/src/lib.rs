//! Evo Workspace
//!
//! The canonical Workspace computational primitive.
//!
//! This crate defines the Workspace domain model.
//! A Workspace represents Evo's current best explanation that a collection
//! of Artifact histories collectively describe one coherent body of work.
//!
//! This crate defines only the Workspace domain objects.
//! Formation, replay, restoration, retrieval, and persistence are
//! intentionally implemented by higher architectural layers.

pub mod attachment;
pub mod confidence;
pub mod errors;
pub mod lifecycle;
pub mod snapshot;
pub mod workspace;
pub mod workspace_id;

pub use confidence::ConfidenceScore;

pub use errors::ConfidenceError;

pub use workspace::Workspace;

pub use workspace_id::WorkspaceId;

pub use lifecycle::WorkspaceLifecycle;

pub use attachment::Attachment;

pub use snapshot::Snapshot;

pub use errors::WorkspaceError;