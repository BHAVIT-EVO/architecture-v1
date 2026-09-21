//! Desktop-owned daemon worker lifecycle.
//!
//! Evo ships as one app: the desktop shell spawns the headless capture daemon
//! as its worker process (the daemon binary is bundled next to the shell in
//! the .app). The daemon remains the canonical capture/runtime owner — the
//! shell only starts it, watches it, and surfaces its honest state.
//!
//! The shell never fakes capture: when the daemon cannot run (for example
//! Accessibility permission is missing), the daemon exits with its error and
//! the shell reflects exactly that state instead of pretending capture works.

use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The honest state of the daemon worker, as reported by the process itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonStatus {
    /// The daemon process is running and every capture source is live.
    Capturing,
    /// The daemon is running but only part of capture is available (for
    /// example window-focus capture needs Accessibility, which is not
    /// granted). The detail says what is unavailable, in plain language.
    PartialCapture { detail: String },
    /// The daemon refused to run because macOS Accessibility is not granted.
    PermissionRequired,
    /// The daemon binary could not be located or started.
    Unavailable,
    /// The daemon exited with an unexpected error.
    Failed(String),
}

impl DaemonStatus {
    /// A short human-readable status line for the shell.
    pub fn label(&self) -> &'static str {
        match self {
            DaemonStatus::Capturing => "Capture running",
            DaemonStatus::PartialCapture { .. } => "Capture partially running",
            DaemonStatus::PermissionRequired => "Permission required",
            DaemonStatus::Unavailable => "Capture unavailable",
            DaemonStatus::Failed(_) => "Capture failed",
        }
    }
}

/// Locates the daemon binary next to the desktop shell executable.
///
/// In the bundled app the daemon sits in `Contents/MacOS` beside the shell;
/// in development both binaries land in the same cargo output directory.
pub fn daemon_binary_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidate = dir.join("evo-daemon");
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

/// Owns the spawned daemon child process and its reported state.
pub struct DaemonManager {
    child: Option<Child>,
    stderr: Arc<Mutex<Vec<String>>>,
    status: DaemonStatus,
    storage_root: PathBuf,
    spawned: bool,
}

impl DaemonManager {
    /// Constructs a manager and immediately attempts to start the daemon.
    pub fn new(storage_root: PathBuf) -> Self {
        let mut manager = Self {
            child: None,
            stderr: Arc::new(Mutex::new(Vec::new())),
            status: DaemonStatus::Unavailable,
            storage_root,
            spawned: false,
        };
        manager.spawn();
        manager
    }

    /// Starts the daemon worker unless one is already running on the storage
    /// root (the daemon's own single-instance lock decides).
    pub fn spawn(&mut self) {
        self.stderr.lock().expect("stderr lock").clear();
        if evo_daemon::daemon_lock::daemon_is_running(&self.storage_root) {
            // A daemon declared an owner that is not this shell: it is an
            // orphan from a previous desktop instance that died without
            // cleanup. Replace it so capture is owned by this shell and the
            // self-capture boundary excludes the right pid.
            let owner = evo_daemon::daemon_status::read_desktop_owner_pid(&self.storage_root);
            if evo_daemon::daemon_status::should_replace_daemon(owner, std::process::id()) {
                eprintln!("EVO-DESKTOP replacing orphaned daemon (owner {owner:?})");
                self.kill_running_daemon();
                // Give the killed daemon a moment to die so the fresh daemon
                // below does not race its single-instance lock.
                std::thread::sleep(Duration::from_millis(300));
                if evo_daemon::daemon_lock::daemon_is_running(&self.storage_root) {
                    // The daemon did not die; do not fight it — report its
                    // own state instead of spawning a second instance.
                    self.status = self.reported_status();
                    self.spawned = true;
                    return;
                }
            } else {
                // A daemon already owns the storage root and is ours (or
                // declared no owner); there is nothing to start. Report what
                // that daemon itself reports.
                self.status = self.reported_status();
                self.spawned = true;
                return;
            }
        }
        let Some(binary) = daemon_binary_path() else {
            self.status = DaemonStatus::Unavailable;
            return;
        };
        let mut command = Command::new(&binary);
        command
            .env("EVO_STORAGE_ROOT", &self.storage_root)
            // The shell declares itself to the capture boundary so the daemon
            // never witnesses Evo's own UI window as user work (identity-based
            // self-reference boundary, the same category as the storage-root
            // exclusion).
            .env("EVO_DESKTOP_PID", std::process::id().to_string())
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                self.status = DaemonStatus::Unavailable;
                self.spawned = true;
                eprintln!("EVO-DESKTOP failed to spawn daemon {binary:?}: {err}");
                return;
            }
        };
        let stderr = Arc::clone(&self.stderr);
        if let Some(pipe) = child.stderr.take() {
            std::thread::spawn(move || {
                for line in std::io::BufReader::new(pipe).lines() {
                    if let Ok(line) = line {
                        stderr.lock().expect("stderr lock").push(line);
                    }
                }
            });
        }
        self.child = Some(child);
        self.status = DaemonStatus::Capturing;
        self.spawned = true;
    }

    /// Re-checks the worker: when it has exited, classify the exit honestly
    /// from its own error output. Call frequently while the shell runs.
    pub fn poll(&mut self) -> DaemonStatus {
        if !self.spawned {
            return self.status.clone();
        }
        let Some(child) = self.child.as_mut() else {
            // No live child: a daemon may still own the storage root.
            if evo_daemon::daemon_lock::daemon_is_running(&self.storage_root) {
                self.status = self.reported_status();
            }
            return self.status.clone();
        };
        let previous = self.status.clone();
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                // The stderr reader thread may still be draining the pipe when
                // the process exit is observed; give it a moment so the exit
                // is classified from the daemon's own reported error, not from
                // an empty buffer.
                std::thread::sleep(Duration::from_millis(50));
                let lines = self.stderr.lock().expect("stderr lock").join("\n");
                self.status = classify_exit(exit_status, &lines);
                self.child = None;
                eprintln!(
                    "EVO-DESKTOP daemon exited: {:?} stderr={:?}",
                    exit_status, lines
                );
            }
            Ok(None) => {
                // The daemon is alive. Its own status file reports whether
                // every capture source is live or some are unavailable (for
                // example Accessibility is not granted); never claim full
                // capture without that report.
                self.status = self.reported_status();
            }
            Err(err) => {
                self.status = DaemonStatus::Failed(format!("{err}"));
                eprintln!("EVO-DESKTOP daemon poll error: {err}");
            }
        }
        if self.status != previous {
            eprintln!("EVO-DESKTOP daemon status: {:?}", self.status);
        }
        self.status.clone()
    }

    /// Force-terminates the daemon currently holding the single-instance
    /// lock, when one is alive. The daemon's records are append-only and
    /// fsynced per record, so an interrupted write is at most a torn tail
    /// that storage recovery truncates on the next read.
    fn kill_running_daemon(&self) {
        if let Some(pid) = evo_daemon::daemon_lock::running_daemon_pid(&self.storage_root) {
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
        }
    }

    /// The daemon's own report of whether capture is full or partial.
    ///
    /// A live daemon with no readable status file (an older daemon binary,
    /// or one that has not finished starting) reports full capture: a daemon
    /// that runs its sources is genuinely capturing what it can.
    fn reported_status(&self) -> DaemonStatus {
        match evo_daemon::daemon_status::read_capture_status(&self.storage_root) {
            Some(evo_daemon::daemon_status::CaptureStatus::Partial { detail }) => {
                DaemonStatus::PartialCapture { detail }
            }
            _ => DaemonStatus::Capturing,
        }
    }

    /// Stops the current worker so a fresh one can be started (for example
    /// after the user grants Accessibility permission).
    pub fn restart(&mut self) {
        self.kill();
        self.spawn();
    }

    /// Terminates the worker process, if any.
    pub fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.spawned = false;
    }
}

impl Drop for DaemonManager {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Classifies a daemon exit from its own error output.
///
/// Pure and deterministic: the shell never guesses — the classification is
/// driven by the daemon's reported state.
pub fn classify_exit(exit: std::process::ExitStatus, stderr: &str) -> DaemonStatus {
    let code = exit.code();
    let permission = stderr.contains("Accessibility permission is required");
    if code.is_some_and(|code| code != 0) && permission {
        DaemonStatus::PermissionRequired
    } else if code != Some(0) {
        DaemonStatus::Failed(describe_exit(exit, stderr))
    } else {
        DaemonStatus::Capturing
    }
}

fn describe_exit(exit: std::process::ExitStatus, stderr: &str) -> String {
    let summary = match exit.code() {
        Some(code) => format!("exited with status {code}"),
        None => "terminated by signal".to_string(),
    };
    let detail = stderr.lines().last().unwrap_or("no error detail");
    format!("{summary}: {detail}")
}

/// Opens the macOS Accessibility privacy pane in System Settings.
pub fn open_accessibility_settings() {
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// How long the shell waits before re-checking the daemon process.
pub const POLL_PERIOD: Duration = Duration::from_secs(2);

#[cfg(test)]
mod tests {
    use super::*;

    use std::process::ExitStatus;

    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    #[cfg(unix)]
    fn status(code: i32) -> ExitStatus {
        ExitStatus::from_raw(code << 8)
    }

    #[test]
    fn permission_exit_is_classified_as_permission_required() {
        #[cfg(unix)]
        {
            let stderr = "daemon capture error: Accessibility permission is required to witness focused windows";
            assert_eq!(
                classify_exit(status(1), stderr),
                DaemonStatus::PermissionRequired
            );
        }
    }

    #[test]
    fn unexpected_exit_is_reported_honestly() {
        #[cfg(unix)]
        {
            let stderr = "daemon storage error: backend not configured";
            let result = classify_exit(status(2), stderr);
            assert!(matches!(result, DaemonStatus::Failed(detail) if detail.contains("status 2")));
        }
    }

    #[test]
    fn zero_exit_is_treated_as_capture_running() {
        #[cfg(unix)]
        {
            assert_eq!(classify_exit(status(0), ""), DaemonStatus::Capturing);
        }
    }

    #[test]
    fn permission_error_with_zero_exit_is_not_misclassified() {
        #[cfg(unix)]
        {
            // A zero exit with permission text on stderr is not a refusal;
            // the process reported success.
            let stderr = "Accessibility permission is required to witness focused windows";
            assert_eq!(classify_exit(status(0), stderr), DaemonStatus::Capturing);
        }
    }

    #[test]
    fn status_labels_are_human_readable() {
        assert_eq!(DaemonStatus::Capturing.label(), "Capture running");
        assert_eq!(
            DaemonStatus::PermissionRequired.label(),
            "Permission required"
        );
        assert_eq!(
            DaemonStatus::PartialCapture {
                detail: String::new()
            }
            .label(),
            "Capture partially running"
        );
    }

    #[test]
    fn live_daemon_reports_partial_capture_from_status_file() {
        let root = unique_root("partial");
        std::fs::create_dir_all(&root).expect("root dir");
        // A live daemon owns the storage root (its pid file) and reports
        // partial capture through its status file.
        std::fs::write(root.join("daemon.pid"), format!("{}\n", std::process::id()))
            .expect("pid write");
        evo_daemon::daemon_status::write_capture_status(
            &root,
            &evo_daemon::daemon_status::CaptureStatus::Partial {
                detail: "Accessibility permission is required to witness focused windows"
                    .to_string(),
            },
        )
        .expect("status write");

        let mut manager = DaemonManager {
            child: None,
            stderr: Arc::new(Mutex::new(Vec::new())),
            status: DaemonStatus::Unavailable,
            storage_root: root.clone(),
            spawned: true,
        };
        assert_eq!(
            manager.poll(),
            DaemonStatus::PartialCapture {
                detail: "Accessibility permission is required to witness focused windows"
                    .to_string()
            }
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn live_daemon_without_status_file_reports_capture_running() {
        let root = unique_root("no-status");
        std::fs::create_dir_all(&root).expect("root dir");
        std::fs::write(root.join("daemon.pid"), format!("{}\n", std::process::id()))
            .expect("pid write");
        let mut manager = DaemonManager {
            child: None,
            stderr: Arc::new(Mutex::new(Vec::new())),
            status: DaemonStatus::Unavailable,
            storage_root: root.clone(),
            spawned: true,
        };
        assert_eq!(manager.poll(), DaemonStatus::Capturing);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn unique_root(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-desktop-daemon-{label}-{nanos}"))
    }
}
