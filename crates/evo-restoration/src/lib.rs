//! evo-restoration — Restoration Layer Domain Model.
//!
//! This crate implements the canonical computational objects produced by the
//! Restoration layer as defined by IS-0014 (Restoration Model) and governed
//! by RFC-0006 (Restoration Contract).
//!
//! # Purpose
//!
//! The Restoration layer transforms canonical Workspace understanding into a
//! canonical `RestorationPlan` that enables a user to continue previously
//! interrupted work with minimal cognitive reload (IS-0014 §1).
//!
//! The Restoration layer SHALL compute restoration understanding.
//!
//! It SHALL NOT execute restoration (IS-0014 RM-5).
//!
//! # Domain Objects
//!
//! - [`RestorationPlan`] — the canonical output of the Restoration layer.
//! - [`ResumePoint`] — the cognitive entry point into a Workspace.
//! - [`ContextChain`] — the minimum ordered supporting context for the Resume Point.
//! - [`Blocker`] — a descriptive unresolved condition preventing immediate continuation.
//! - [`NextStep`] — the immediate continuation action following the Resume Point.
//!
//! # Invariants
//!
//! All invariants defined by IS-0014 are enforced at construction time.
//! Domain objects are immutable after construction.
//!
//! # Dependencies
//!
//! - `evo-artifact`: for [`evo_artifact::ArtifactId`].
//! - `evo-workspace`: for [`evo_workspace::WorkspaceId`].

mod blocker;
mod context_chain;
mod errors;
mod next_step;
mod restoration_plan;
mod resume_point;

pub use blocker::Blocker;
pub use context_chain::ContextChain;
pub use errors::RestorationError;
pub use next_step::NextStep;
pub use restoration_plan::RestorationPlan;
pub use resume_point::ResumePoint;
