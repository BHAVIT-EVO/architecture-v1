//! evo-daemon — Evo's composition root.
//!
//! This crate constructs and hosts the daemon runtime boundary for capture.

mod builder;
pub mod cache;
pub mod classification;
pub mod classification_rules;
pub mod continuation;
mod daemon;
pub mod daemon_lock;
pub mod daemon_status;
pub mod designation;
mod errors;
pub mod feedback;
pub mod grouping;
pub mod name_index;
pub mod naming;
pub mod persistence;
pub mod runtime;
pub mod threads;
pub mod ui;
pub mod understanding;
pub mod voice;
pub mod work_understanding;
pub mod workspace_replay;

pub use builder::DaemonBuilder;
pub use daemon::Daemon;
pub use errors::DaemonError;
pub use classification::ClassifiedSurfaces;
pub use classification_rules::{
    ClassificationAssignment, ClassificationError, ClassificationInterval, ClassificationMatcher,
    ClassificationRule, ClassificationRules, ClassificationSource, DismissalRuleProposal,
    IntervalClassification, MatchOp, MatchOperator, RuleApplication, StructuralField,
    WitnessedFields, propose_repeated_dismissal_rule,
};
pub use name_index::NameIndex;
pub use runtime::{
    RestorationInputBoundary, Runtime, VerticalPipeline, VerticalRuntimeHandle, WindowFocusRuntime,
    WindowFocusRuntimeHandle,
};

/// Test-only readiness helpers shared by the socket-channel tests.
#[cfg(test)]
pub(crate) mod socket_test_support {
    //! Waiting for the socket FILE to exist races with the listener's
    //! bind→listen under parallel test load: the file is created by bind,
    //! and a connect arriving before the OS registers the listen backlog
    //! gets ECONNREFUSED. These tests therefore wait until the listener
    //! actually ACCEPTS a connection — bounded, never a sleep-and-hope.

    use std::io::ErrorKind;
    use std::os::unix::net::UnixStream;
    use std::path::Path;
    use std::time::{Duration, Instant};

    /// Blocks until a listener accepts a connection at `socket`, or panics
    /// after a bounded budget with the last connect error. The probe
    /// connection is dropped before any bytes are written, so the server
    /// thread reads EOF, rejects harmlessly, and the signal channel
    /// receives nothing from the probe.
    pub(crate) fn wait_for_listener(socket: &Path) {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut last_err = None;
        while Instant::now() < deadline {
            match UnixStream::connect(socket) {
                Ok(_probe) => return,
                Err(err) => {
                    debug_assert!(matches!(
                        err.kind(),
                        ErrorKind::ConnectionRefused
                            | ErrorKind::NotFound
                            | ErrorKind::PermissionDenied
                    ));
                    last_err = Some(err);
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "listener at {} never accepted within the budget; last error: {last_err:?}",
            socket.display()
        );
    }
}
