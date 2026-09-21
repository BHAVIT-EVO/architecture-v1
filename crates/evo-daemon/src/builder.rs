//! Builder for the Evo daemon composition root.
//!
//! The builder is intentionally minimal and infallible in this frozen
//! implementation.

use crate::daemon::Daemon;

/// Minimal daemon builder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DaemonBuilder;

impl DaemonBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self
    }

    /// Builds the daemon composition root.
    pub fn build(&self) -> Daemon {
        Daemon::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_is_constructible_copy_and_default() {
        let builder = DaemonBuilder::new();
        let copied = builder;
        let cloned = builder.clone();
        let defaulted: DaemonBuilder = Default::default();

        assert_eq!(builder, copied);
        assert_eq!(builder, cloned);
        assert_eq!(builder, defaulted);
    }

    #[test]
    fn builder_builds_default_daemon() {
        let builder = DaemonBuilder::new();
        let daemon = builder.build();
        assert_eq!(daemon, Daemon::new());
    }
}
