//! evo-retrieval — Retrieval Service Boundary.
//!
//! This crate exposes the retrieval service boundary defined by RFC-0007.
//! RFC-0007 specifies what Retrieval consumes and what it produces, but it
//! does not prescribe a retrieval algorithm. The crate therefore defines only
//! the public service shape required by the architecture.
//!
//! # Public Computational Surface
//!
//! - [`Retrieval`] — the retrieval service boundary.
//! - [`RetrievalError`] — failures that prevent Retrieval from executing
//!   according to RFC-0007.
//!
//! # Contract
//!
//! - Consume one trigger and the current committed Workspaces.
//! - Produce zero or more candidate `WorkspaceId` values.
//! - Preserve the implementation-determined ordering of candidate
//!   `WorkspaceId` values.
//! - Reject duplicate candidate `WorkspaceId` values.
//! - Reject candidate `WorkspaceId` values that are not present in the input
//!   Workspace set.
//!
//! # Non-Responsibilities
//!
//! This crate does not define:
//!
//! - Trigger models.
//! - Query objects.
//! - Ranking objects.
//! - Embedding types.
//! - Search request or result types.
//! - Index structures.
//! - Retrieval algorithms.
//!
//! Those remain intentionally unspecified by RFC-0007.

mod errors;
mod retrieval;

pub use errors::RetrievalError;
pub use retrieval::Retrieval;
