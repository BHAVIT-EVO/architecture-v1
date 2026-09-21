//! Daemon composition root for the capture runtime.
//!
//! The daemon owns the runtime composition boundary and exposes the capture
//! spine without taking over capture, translation, or observation acceptance.

use crate::errors::DaemonError;
use crate::runtime::{RestorationInputBoundary, VerticalRuntimeHandle, WindowFocusRuntimeHandle};
use std::path::PathBuf;

/// The Evo daemon composition root.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Daemon {
    runtime: crate::runtime::Runtime,
}

impl Daemon {
    /// Constructs a daemon from the public runtime boundary.
    pub fn new() -> Self {
        Self {
            runtime: crate::runtime::Runtime::new(),
        }
    }

    /// Returns the runtime boundary.
    pub fn runtime(&self) -> &crate::runtime::Runtime {
        &self.runtime
    }

    /// Starts the macOS WindowFocusGained runtime spine.
    pub fn start_window_focus_runtime(
        &self,
    ) -> Result<(WindowFocusRuntimeHandle, std::sync::mpsc::Receiver<evo_observation::observation::Observation>), DaemonError>
    {
        self.runtime.start_window_focus_runtime()
    }

    /// Starts the daemon-owned vertical runtime spine through the Workspace/Snapshot boundary.
    pub fn start_vertical_runtime(
        &self,
    ) -> Result<
        (
            VerticalRuntimeHandle,
            std::sync::mpsc::Receiver<Result<RestorationInputBoundary, DaemonError>>,
        ),
        DaemonError,
    > {
        self.runtime.start_vertical_runtime()
    }

    /// Starts the daemon-owned vertical runtime spine through the Workspace/Snapshot boundary
    /// using the supplied storage root.
    pub fn start_vertical_runtime_with_storage_root(
        &self,
        storage_root: impl Into<PathBuf>,
    ) -> Result<
        (
            VerticalRuntimeHandle,
            std::sync::mpsc::Receiver<Result<RestorationInputBoundary, DaemonError>>,
        ),
        DaemonError,
    > {
        self.runtime
            .start_vertical_runtime_with_storage_root(storage_root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_constructs_and_exposes_runtime_boundary() {
        let daemon = Daemon::new();
        assert_eq!(daemon.runtime(), &crate::runtime::Runtime::new());
    }

    #[test]
    fn daemon_is_cloneable_and_defaultable() {
        let daemon = Daemon::new();
        let cloned = daemon.clone();
        let defaulted: Daemon = Default::default();

        assert_eq!(daemon, cloned);
        assert_eq!(daemon, defaulted);
    }
}
