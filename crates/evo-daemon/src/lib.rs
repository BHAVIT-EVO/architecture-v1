//! evo-daemon — Evo's composition root.
//!
//! This crate constructs and hosts the daemon runtime boundary for capture.

mod builder;
pub mod cache;
pub mod continuation;
mod daemon;
pub mod daemon_lock;
pub mod daemon_status;
pub mod designation;
mod errors;
pub mod grouping;
pub mod persistence;
pub mod threads;
pub mod runtime;
pub mod ui;
pub mod understanding;
pub mod work_understanding;
pub mod workspace_replay;

pub use builder::DaemonBuilder;
pub use daemon::Daemon;
pub use errors::DaemonError;
pub use runtime::{
    RestorationInputBoundary, Runtime, VerticalPipeline, VerticalRuntimeHandle,
    WindowFocusRuntime, WindowFocusRuntimeHandle,
};
