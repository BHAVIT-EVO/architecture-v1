//! Evo Capture
//!
//! The platform-independent capture boundary for Evo.
//!
//! Capture witnesses structured operating-system events, normalizes them into
//! `RawEvent` values, and forwards them to the Observation acceptance pipeline.
//! It performs no semantic interpretation and owns no downstream reasoning.

pub mod adapters;
pub mod collector;
pub mod engine;
pub mod macos_event_source;
pub mod macos_fsevents;
pub mod macos_input_counter;
pub mod macos_url_poller;
pub mod raw_event;
pub mod repo_identity;
pub mod window_snapshot;

pub use adapters::macos::{
    MacOSAdapter, MacOSSignal, OWNING_PROCESS_NAME_CONTEXT_KEY, OWNING_PROCESS_PID_CONTEXT_KEY,
};
pub use collector::Collector;
pub use engine::CaptureEngine;
pub use macos_event_source::{
    BrowserIdentityProvider, BrowserIdentityRegistry, BrowserIdentityWitness, MacOSEventSource,
    MacOSEventSourceError, UnsupportedBrowserIdentityProvider, accessibility_permission_granted,
    desktop_shell_pid, is_self_window, prompt_for_accessibility_permission,
};
pub use macos_fsevents::{
    FSEventsWatcher, FsSignal, classify_event_path, commit_hash_from_reflog_line,
};
pub use macos_input_counter::{
    FLUSH_INTERVAL, InputCounterError, InputCounters, MacOSInputCounter, SubjectResolver,
};
pub use macos_url_poller::MacOSURLPoller;
pub use raw_event::RawEvent;
