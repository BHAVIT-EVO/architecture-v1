//! Operational capture-status file for the daemon worker.
//!
//! The daemon may legitimately run with partial capture: when macOS
//! Accessibility is not granted, the focused-window source cannot attach, but
//! file-system capture (FSEvents) and user designations still operate safely
//! and independently. The daemon survives in that state rather than exiting.
//!
//! This file records which capture sources are actually live so the desktop
//! shell never claims capture is fully operational when it is not. It is
//! operational state, not canonical evidence: nothing in the Observation
//! logs depends on it, it carries no Observations, and it is removed when the
//! daemon stops. The desktop reads it only to render an honest status line.

use crate::errors::DaemonError;

use std::fs;
use std::path::Path;

/// Name of the capture-status file inside the storage root.
pub const DAEMON_STATUS_FILENAME: &str = "daemon.status";

/// The daemon's reported capture capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureStatus {
    /// Every configured capture source is live.
    Full,
    /// Some sources are live and some are not. The detail names what is
    /// unavailable and why, in plain language the desktop can surface.
    Partial { detail: String },
}

impl CaptureStatus {
    /// Parses a status file written by [`write_capture_status`].
    ///
    /// Returns `None` for an unreadable, malformed, or missing file: the
    /// desktop then reports only what it can prove from the process itself
    /// (a live daemon process with no status file predates this mechanism
    /// and is treated as full capture).
    pub fn parse(contents: &str) -> Option<Self> {
        let mut capture: Option<&str> = None;
        let mut detail: Option<&str> = None;
        for line in contents.lines() {
            let (key, value) = line.split_once('=')?;
            match key.trim() {
                "capture" => capture = Some(value.trim()),
                "detail" => detail = Some(value.trim()),
                _ => {}
            }
        }
        match capture? {
            "full" => Some(CaptureStatus::Full),
            "partial" => Some(CaptureStatus::Partial {
                detail: detail?.to_string(),
            }),
            _ => None,
        }
    }

    /// Serializes to the status file format.
    pub fn serialize(&self) -> String {
        match self {
            CaptureStatus::Full => "capture=full\n".to_string(),
            CaptureStatus::Partial { detail } => {
                format!("capture=partial\ndetail={detail}\n")
            }
        }
    }
}

/// Writes the daemon's capture status to `root/daemon.status`.
///
/// Called by the daemon itself after its runtime sources are constructed. The
/// owner of this capture process (the desktop shell that spawned it, when
/// declared through `EVO_DESKTOP_PID`) is recorded as operational state so a
/// later desktop instance can tell an orphaned daemon from its own.
pub fn write_capture_status(root: &Path, status: &CaptureStatus) -> Result<(), DaemonError> {
    write_capture_report(root, status, &[])
}

/// Writes the capture status together with the per-source health report.
///
/// The source lines are operational diagnostics — never canonical evidence:
/// each names a capture source and whether it is live. The desktop's status
/// banner reads only the aggregate `capture=` line; `evo-doctor` and other
/// diagnostics read the source lines to answer "is Evo actually watching?"
pub fn write_capture_report(
    root: &Path,
    status: &CaptureStatus,
    sources: &[(&'static str, &'static str)],
) -> Result<(), DaemonError> {
    let path = root.join(DAEMON_STATUS_FILENAME);
    fs::create_dir_all(root).map_err(|_| {
        DaemonError::from(evo_storage::StorageError::BackendNotConfigured)
    })?;
    let mut contents = status.serialize();
    for (source, state) in sources {
        contents.push_str(&format!("{source}={state}\n"));
    }
    if let Some(owner) = owner_pid_from_env() {
        contents.push_str(&format!("owner_pid={owner}\n"));
    }
    fs::write(&path, contents).map_err(|_| {
        DaemonError::from(evo_storage::StorageError::BackendNotConfigured)
    })
}

/// The per-source health report from a daemon's status file: `(source, state)`
/// pairs such as `window-focus=active` or `url=active`. Empty when the file
/// carries no source lines (an older daemon binary).
pub fn read_capture_sources(root: &Path) -> Vec<(String, String)> {
    let Ok(contents) = fs::read_to_string(root.join(DAEMON_STATUS_FILENAME)) else {
        return Vec::new();
    };
    let mut sources = Vec::new();
    for line in contents.lines() {
        if let Some((source, state)) = line.split_once('=') {
            let source = source.trim();
            // Only known operational source keys are reported; unknown keys
            // are ignored so a future file version never misparses.
            if matches!(
                source,
                "window-focus" | "file-save" | "commit" | "url" | "designation"
            ) {
                sources.push((source.to_string(), state.trim().to_string()));
            }
        }
    }
    sources
}

/// Reads the daemon's capture status from `root/daemon.status`.
///
/// Returns `None` when the file is missing or not parseable (see
/// [`CaptureStatus::parse`]).
pub fn read_capture_status(root: &Path) -> Option<CaptureStatus> {
    let contents = fs::read_to_string(root.join(DAEMON_STATUS_FILENAME)).ok()?;
    CaptureStatus::parse(&contents)
}

/// The desktop shell pid the running daemon was spawned by, from its status
/// file. `None` when the file is missing, malformed, or the daemon declared
/// no owner (e.g. a development launch).
pub fn read_desktop_owner_pid(root: &Path) -> Option<u32> {
    let contents = fs::read_to_string(root.join(DAEMON_STATUS_FILENAME)).ok()?;
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("owner_pid=") {
            return value.trim().parse::<u32>().ok();
        }
    }
    None
}

/// Whether a running daemon should be replaced by the caller: only when it
/// explicitly declared a desktop-shell owner that is not the caller. A
/// daemon that declared no owner (development launch) is never replaced.
///
/// Pure and deterministic, so the desktop's lifecycle decision is testable.
pub fn should_replace_daemon(owner: Option<u32>, my_pid: u32) -> bool {
    owner.is_some_and(|owner| owner != my_pid)
}

fn owner_pid_from_env() -> Option<u32> {
    std::env::var("EVO_DESKTOP_PID")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|pid| *pid > 0)
}

/// Removes the capture-status file. Called when the daemon stops.
pub fn clear_capture_status(root: &Path) {
    let _ = fs::remove_file(root.join(DAEMON_STATUS_FILENAME));
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-daemon-status-{label}-{nanos}"))
    }

    #[test]
    fn parse_full_round_trips() {
        let status = CaptureStatus::Full;
        assert_eq!(CaptureStatus::parse(&status.serialize()), Some(status));
    }

    #[test]
    fn parse_partial_round_trips() {
        let status = CaptureStatus::Partial {
            detail: "Accessibility permission is required to witness focused windows".to_string(),
        };
        assert_eq!(CaptureStatus::parse(&status.serialize()), Some(status));
    }

    #[test]
    fn parse_rejects_malformed_files() {
        assert_eq!(CaptureStatus::parse(""), None);
        assert_eq!(CaptureStatus::parse("capture=unknown\n"), None);
        assert_eq!(CaptureStatus::parse("capture=partial\n"), None);
        assert_eq!(CaptureStatus::parse("not a status file\n"), None);
    }

    #[test]
    fn write_read_clear_round_trip() {
        let root = unique_root("round-trip");
        let partial = CaptureStatus::Partial {
            detail: "reason".to_string(),
        };
        write_capture_status(&root, &partial).expect("write succeeds");
        assert_eq!(read_capture_status(&root), Some(partial));

        write_capture_status(&root, &CaptureStatus::Full).expect("write succeeds");
        assert_eq!(read_capture_status(&root), Some(CaptureStatus::Full));

        clear_capture_status(&root);
        assert_eq!(read_capture_status(&root), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_file_reads_as_none() {
        let root = unique_root("missing");
        assert_eq!(read_capture_status(&root), None);
        assert_eq!(read_desktop_owner_pid(&root), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn owner_pid_round_trips_through_the_status_file() {
        let root = unique_root("owner");
        unsafe { std::env::set_var("EVO_DESKTOP_PID", "4321") };
        write_capture_status(&root, &CaptureStatus::Full).expect("write");
        assert_eq!(read_desktop_owner_pid(&root), Some(4321));
        // The capture semantics are unaffected by the owner record.
        assert_eq!(read_capture_status(&root), Some(CaptureStatus::Full));
        unsafe { std::env::remove_var("EVO_DESKTOP_PID") };
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn orphaned_daemon_is_detected_by_owner_mismatch() {
        // A daemon that declared no owner (development) is never replaced.
        assert!(!should_replace_daemon(None, 111));
        // A daemon owned by me is mine.
        assert!(!should_replace_daemon(Some(111), 111));
        // A daemon owned by a different (dead) shell is an orphan.
        assert!(should_replace_daemon(Some(999), 111));
    }

    #[test]
    fn per_source_health_round_trips_through_the_status_file() {
        let root = unique_root("sources");
        let sources = [
            ("window-focus", "unavailable"),
            ("file-save", "active"),
            ("commit", "active"),
            ("url", "active"),
            ("designation", "active"),
        ];
        write_capture_report(&root, &CaptureStatus::Partial {
            detail: "Accessibility permission is required to witness focused windows".to_string(),
        }, &sources)
        .expect("write");
        let reported = read_capture_sources(&root);
        let expected: Vec<(String, String)> = sources
            .iter()
            .map(|(s, v)| (s.to_string(), v.to_string()))
            .collect();
        assert_eq!(reported, expected);
        // The aggregate status is unchanged by the source lines.
        assert_eq!(
            read_capture_status(&root),
            Some(CaptureStatus::Partial {
                detail: "Accessibility permission is required to witness focused windows"
                    .to_string()
            })
        );
        // Unknown keys never surface as sources.
        let _ = std::fs::write(root.join(DAEMON_STATUS_FILENAME), "capture=full\nmystery=xyz\n");
        assert!(read_capture_sources(&root).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
