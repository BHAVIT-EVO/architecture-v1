//! evo-replay
//!
//! Deterministic re-execution of Workspace Formation over historical
//! canonical computational primitives.
//!
//! This crate implements IS-0013 (Workspace Replay).
//!
//! Workspace Replay reproduces Workspace understanding by re-executing
//! the Workspace Formation contract defined by IS-0012 over the complete
//! canonical Observation history in canonical Observation order.
//!
//! # Architectural Contract
//!
//! - Replay consumes only canonical Observations, canonical Artifacts, and
//!   the Workspace Formation rules supplied by the caller (IS-0013 §4).
//! - The canonical unit of Replay is the canonical Observation (IS-0013 §5).
//! - Replay executes Formation in canonical Observation order (IS-0013 §5).
//! - Replay does not implement independent Workspace Recognition, Attachment
//!   Evaluation, Workspace Decision, or Snapshot Construction (IS-0013 §8).
//! - Replay does not perform persistence, restoration, learning, or retrieval
//!   (IS-0013 §11).

pub mod corpus;
pub mod replay;

pub use corpus::ReplayCorpus;
pub use replay::replay;

