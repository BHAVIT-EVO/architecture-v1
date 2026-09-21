//! Evo Capture
//!
//! The platform-independent capture boundary for Evo.
//!
//! Capture witnesses structured operating-system events, normalizes them into
//! `RawEvent` values, and forwards them to the Observation acceptance pipeline.
//! It performs no semantic interpretation and owns no downstream reasoning.

pub mod adapters;
pub mod collector;
pub mod macos_event_source;
pub mod macos_fsevents;
pub mod macos_input_counter;
pub mod macos_url_poller;
pub mod repo_identity;
pub mod engine;
pub mod raw_event;

pub use collector::Collector;
pub use adapters::macos::{
    MacOSAdapter, MacOSSignal, OWNING_PROCESS_PID_CONTEXT_KEY,
};
pub use engine::CaptureEngine;
pub use macos_event_source::{
    accessibility_permission_granted, desktop_shell_pid, is_self_window,
    prompt_for_accessibility_permission, MacOSEventSource, MacOSEventSourceError,
};
pub use macos_fsevents::{classify_event_path, commit_hash_from_reflog_line, FSEventsWatcher, FsSignal};
pub use macos_input_counter::{
    InputCounterError, InputCounters, MacOSInputCounter, SubjectResolver, FLUSH_INTERVAL,
};
pub use macos_url_poller::MacOSURLPoller;
pub use raw_event::RawEvent;
