//! Error types for the evo-daemon crate.

use std::fmt;

/// Errors that can occur at the daemon boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonError {
    /// Artifact acceptance error.
    Artifact(evo_artifact::errors::ArtifactError),
    /// Capture source setup or runtime error.
    Capture(evo_capture::MacOSEventSourceError),
    /// Observation acceptance error.
    Observation(evo_observation::errors::ObservationError),
    /// Restoration Derivation error.
    Restoration(evo_restoration::RestorationError),
    /// Storage boundary error.
    Storage(evo_storage::StorageError),
    /// Another daemon instance already holds the storage root.
    DaemonAlreadyRunning { pid: u32, root: String },
    /// The user designation channel (RFC-0011) failed.
    Designation(String),
    /// Runtime unavailable on this platform.
    UnsupportedPlatform,
}

impl fmt::Display for DaemonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaemonError::Artifact(err) => write!(f, "daemon artifact error: {err}"),
            DaemonError::Capture(err) => write!(f, "daemon capture error: {err}"),
            DaemonError::Observation(err) => write!(f, "daemon observation error: {err}"),
            DaemonError::Restoration(err) => write!(f, "daemon restoration error: {err}"),
            DaemonError::Storage(err) => write!(f, "daemon storage error: {err}"),
            DaemonError::DaemonAlreadyRunning { pid, root } => write!(
                f,
                "another daemon instance is already running (pid {pid}) on storage root {root}"
            ),
            DaemonError::Designation(detail) => write!(f, "daemon designation error: {detail}"),
            DaemonError::UnsupportedPlatform => {
                write!(f, "daemon runtime is only available on supported platforms")
            }
        }
    }
}

impl std::error::Error for DaemonError {}

impl From<evo_artifact::errors::ArtifactError> for DaemonError {
    fn from(value: evo_artifact::errors::ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<evo_capture::MacOSEventSourceError> for DaemonError {
    fn from(value: evo_capture::MacOSEventSourceError) -> Self {
        Self::Capture(value)
    }
}

impl From<evo_observation::errors::ObservationError> for DaemonError {
    fn from(value: evo_observation::errors::ObservationError) -> Self {
        Self::Observation(value)
    }
}

impl From<evo_restoration::RestorationError> for DaemonError {
    fn from(value: evo_restoration::RestorationError) -> Self {
        Self::Restoration(value)
    }
}

impl From<evo_storage::StorageError> for DaemonError {
    fn from(value: evo_storage::StorageError) -> Self {
        Self::Storage(value)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn daemon_error_implements_std_error() {
        fn assert_error<T: std::error::Error>() {}

        assert_error::<super::DaemonError>();
    }
}
