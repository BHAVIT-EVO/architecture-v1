//! User-visible desktop copy. Keep product promises and capability claims here
//! so screens can share wording without drifting into claims the runtime cannot
//! support.

pub const BRAND: &str = "Evo";
pub const SETTINGS: &str = "Settings";
pub const SETTINGS_DONE: &str = "Done";
pub const SETTINGS_CAPTURE: &str = "Capture";
pub const SETTINGS_CAPTURE_RUNNING: &str = "Evo is recording what it witnesses.";
pub const SETTINGS_CAPTURE_LIMITED: &str =
    "Some capture sources are unavailable; window focus is not permitted.";
pub const SETTINGS_CAPTURE_BLOCKED: &str = "Evo cannot observe your work yet.";
pub const SETTINGS_CAPTURE_UNAVAILABLE: &str = "The capture worker is not running.";
pub const SETTINGS_CAPTURE_FAILED: &str = "The capture worker stopped.";
pub const SETTINGS_STORAGE_PATH: &str = "Local storage directory";
pub const SETTINGS_CAPTURE_WORKER: &str = "Capture worker";
pub const SETTINGS_DIAGNOSTICS_LIMIT: &str =
    "Diagnostics are shown on this screen. This build has no diagnostic upload action.";
pub const HOME_TITLE: &str = "Pick up where you left off.";
pub const HOME_LABEL: &str = "Your work";
pub const HOME_CONTINUE_TITLE: &str = "Pick up where you left off.";
pub const HOME_SEARCH: &str = "Find a work by name";
pub const HOME_CONTINUE: &str = "Continue";
pub const HOME_PERMISSION_OPEN: &str = "Open System Settings";
pub const HOME_PERMISSION_RECHECK: &str = "Check again";
pub const HOME_LOADING: &str = "Loading witnessed work…";
pub const HOME_EMPTY_LEARNING: &str = "Evo is learning your work";
pub const HOME_EMPTY_WAITING: &str = "Evo is not capturing yet";
pub const FIRST_RUN_INTRO_TITLE: &str = "A clearer way back to your work";
pub const FIRST_RUN_INTRO_BODY: &str = "Evo builds a local record from computer activity it can observe, then uses that record to help you return to work. What it can see depends on the capture sources and permissions enabled on this Mac.";
pub const FIRST_RUN_STORAGE_TITLE: &str = "Your record lives on this Mac";
pub const FIRST_RUN_STORAGE_BODY: &str = "The desktop stores observation and work records in Evo's local Application Support folder. There is no account or sync control in this desktop build. Read Privacy & trust in Settings for the current capture and storage boundaries.";
pub const FIRST_RUN_PERMISSION_TITLE: &str = "Choose whether to grant Accessibility";
pub const FIRST_RUN_CONTINUE: &str = "Continue";
pub const FIRST_RUN_ENTER: &str = "Enter Evo";
pub const FIRST_RUN_PERMISSION_LIMIT: &str = "Without this permission, window-focus capture is unavailable; other capture sources may still run.";
pub const SYSTEM_SETTINGS_OPEN: &str = "Open System Settings";
pub const PERMISSION_RECHECK: &str = "Check again";
pub const FIRST_RUN_PERMISSION_BODY: &str = "Accessibility lets Evo witness focused windows. Without it, window-focus capture is unavailable; other capture sources may still run. You can grant it now or later in Settings.";
pub const CAPTURE_BLOCKED_TITLE: &str = "Evo cannot see focused windows";
pub const THEME_LABEL: &str = "Appearance";
pub const TRUST_LABEL: &str = "Privacy & trust";
pub const HELP_LABEL: &str = "Help";
pub const DIAGNOSTICS_LABEL: &str = "Diagnostics";
pub const COMING_SOON: &str = "Coming soon";
pub const STORAGE_ERROR_TITLE: &str = "Evo could not read its record";
pub const STORAGE_ERROR_LEDE: &str = "Nothing is shown because nothing could be read. Evo will not show a partial or remembered list as if it were current.";
pub const HELP_BODY: &str = "Use Home to find witnessed work. Select a work to inspect its record; choose Continue to ask Evo to reopen eligible resources. Search narrows the current work list. If capture is limited, Settings explains the reported state and offers the macOS permission controls.";
pub const PRIVACY_BODY: &str = "The desktop reads and writes Evo's local observation and work records under the displayed Application Support directory. Capture depends on the running Evo worker and granted macOS permissions. The desktop UI contains no account, sync, analytics, or diagnostic-upload controls. This is a description of this build, not a claim that every machine-level process or third-party dependency is absent.";
pub const AUDIO_BODY: &str = "This desktop window does not start a microphone listener. The optional evo-agent can run a local microphone and speech-recognition pipeline when built with voice features and its required model files; it communicates with the daemon over a local Unix socket. Voice agent status is not exposed by this desktop build.";
pub const NOT_BUILT_LABEL: &str = "Not available in this build";
