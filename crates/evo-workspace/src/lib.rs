//! Evo Workspace
//!
//! The canonical Workspace computational primitive.
//!
//! A **Workspace** is a body of work worth returning to: Evo's standing
//! explanation that a set of Artifact histories describe one coherent piece of
//! work, together with the internal structure that makes it resumable.
//!
//! A Workspace is emphatically **not** a resource. One file, one window, one
//! address, one application is a resource; a Workspace is what several of them
//! amount to when the evidence says they were being used together for something.
//! Collapsing the two is what turned Home into an activity log, and
//! [`projection`] is the module where the distinction is now enforced.
//!
//! # What lives here and what does not
//!
//! This crate owns the Workspace domain model — [`Workspace`], [`Attachment`],
//! [`ConfidenceScore`], [`Snapshot`], [`WorkspaceLifecycle`], [`WorkspaceId`] —
//! and the single rule that decides which bodies of work become Workspaces
//! ([`projection`]).
//!
//! Reconstructing bodies of work from raw evidence belongs to
//! [`evo_engagement`], one rung below. Persistence, restoration, retrieval, and
//! presentation belong to the layers above. This crate reads no clock, touches no
//! disk, and makes no network call.

pub mod attachment;
pub mod confidence;
pub mod errors;
pub mod lifecycle;
pub mod projection;
pub mod snapshot;
pub mod workspace;
pub mod workspace_id;

mod deterministic;

pub use attachment::{Attachment, ResourceRole};

pub use confidence::ConfidenceScore;

pub use errors::ConfidenceError;

pub use errors::WorkspaceError;

pub use lifecycle::WorkspaceLifecycle;

pub use projection::{
    SubjectArtifacts, project_engagements, project_workspace, project_workspaces,
};

pub use snapshot::Snapshot;

pub use workspace::Workspace;

pub use workspace_id::WorkspaceId;
