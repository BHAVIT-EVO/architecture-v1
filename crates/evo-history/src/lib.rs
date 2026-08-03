//! evo-history — Historical Understanding Domain Model.
//!
//! This crate implements the canonical computational object defined by
//! IS-0016 (Historical Understanding Model) and governed by RFC-0005
//! (Historical Understanding Contract).
//!
//! # Purpose
//!
//! Historical Understanding is the immutable record of the committed
//! understanding that justified an architectural action (IS-0016 §1).
//!
//! It exists solely to preserve historical accountability.
//!
//! # Design Goal (IS-0016 §3)
//!
//! `HistoricalUnderstanding` answers one question only:
//!
//! > *"What understanding actually justified this architectural behaviour?"*
//!
//! # Public Computational Surface (IS-0016 §12)
//!
//! - [`HistoryId`] — stable, unique identity for a committed understanding.
//! - [`HistoricalUnderstanding`] — the immutable canonical object.
//! - [`HistoricalError`] — uninhabited; no construction failures exist.
//!
//! # Canonical Components (IS-0016 §4)
//!
//! A `HistoricalUnderstanding` consists of exactly:
//!
//! - `HistoryId`
//! - `WorkspaceId` (reference only; `Workspace` is never owned)
//! - `RestorationPlan` (owned; frozen permanently after construction)
//!
//! # Dependencies
//!
//! - `evo-workspace`: for [`evo_workspace::WorkspaceId`].
//! - `evo-restoration`: for [`evo_restoration::RestorationPlan`].
//!
//! No dependency on `evo-observation`, `evo-artifact`, `evo-knowledge`, or
//! `evo-replay` is required (IS-0016 §16).

mod errors;
mod historical_understanding;
mod history_id;

pub use errors::HistoricalError;
pub use historical_understanding::HistoricalUnderstanding;
pub use history_id::HistoryId;
