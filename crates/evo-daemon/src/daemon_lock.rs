//! Single-instance daemon lock.
//!
//! The desktop shell spawns the daemon as its capture worker. If a previous
//! daemon is still alive on the same canonical storage root — for example
//! after the desktop shell restarts quickly — a second daemon must not start:
//! two writers to the same append-only logs would race. The lock is a pid
//! file inside the storage root; the daemon refuses to start when the pid is
//! still alive and removes the file on exit.

use crate::errors::DaemonError;

use std::fs;
use std::path::{Path, PathBuf};

/// Name of the single-instance lock file inside the storage root.
pub const DAEMON_PID_FILENAME: &str = "daemon.pid";

/// The held single-instance lock. Removing the pid file happens on drop.
#[derive(Debug)]
pub struct DaemonLock {
    path: PathBuf,
}

impl DaemonLock {
    /// Attempts to acquire the single-instance lock for `storage_root`.
    ///
    /// # Errors
    ///
    /// Returns [`DaemonError::DaemonAlreadyRunning`] when the storage root is
    /// already locked by a live daemon process, or a storage error when the
    /// pid file cannot be written.
    pub fn acquire(storage_root: &Path) -> Result<Self, DaemonError> {
        let path = storage_root.join(DAEMON_PID_FILENAME);

        if let Some(pid) = read_pid(&path)? {
            if process_is_alive(pid) {
                return Err(DaemonError::DaemonAlreadyRunning {
                    pid,
                    root: storage_root.display().to_string(),
                });
            }
            // The previous process is gone; its stale lock can be replaced.
        }

        fs::create_dir_all(storage_root).map_err(|_| storage_error())?;
        fs::write(&path, format!("{}\n", std::process::id()))
            .map_err(|_| storage_error())?;
        Ok(Self { path })
    }

    /// Returns the lock file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for DaemonLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Returns whether a daemon is currently holding the single-instance lock on
/// `storage_root`. The desktop shell uses this before spawning a worker.
pub fn daemon_is_running(storage_root: &Path) -> bool {
    read_pid(&storage_root.join(DAEMON_PID_FILENAME))
        .map(|pid| pid.is_some_and(process_is_alive))
        .unwrap_or(false)
}

/// Returns the pid of the daemon currently holding the single-instance lock,
/// when it is alive. Used by the desktop shell to replace an orphaned daemon
/// (one spawned by a previous desktop instance that died without cleanup).
pub fn running_daemon_pid(storage_root: &Path) -> Option<u32> {
    let pid = read_pid(&storage_root.join(DAEMON_PID_FILENAME)).ok()??;
    if process_is_alive(pid) {
        Some(pid)
    } else {
        None
    }
}

fn read_pid(path: &Path) -> Result<Option<u32>, DaemonError> {
    match fs::read_to_string(path) {
        Ok(contents) => contents
            .trim()
            .parse::<u32>()
            .ok()
            .map(Some)
            .ok_or_else(storage_error),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(storage_error()),
    }
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    // kill(pid, 0) reports only whether the process exists; it sends no
    // signal and has no side effect on the target. pid 0 targets the caller's
    // process group and -1 targets every signalable process, so both are
    // rejected before the syscall.
    let pid = pid as i32;
    if pid <= 0 {
        return false;
    }
    unsafe { kill(pid, 0) == 0 }
}

#[cfg(not(unix))]
fn process_is_alive(_pid: u32) -> bool {
    // Without a portable liveness check the lock is treated as held; the
    // daemon only runs on macOS, where the unix path applies.
    true
}

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

fn storage_error() -> DaemonError {
    DaemonError::from(evo_storage::StorageError::BackendNotConfigured)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("evo-daemon-lock-{label}-{nanos}"))
    }

    #[test]
    fn lock_acquires_and_releases() {
        let root = unique_root("acquire-release");
        let lock = DaemonLock::acquire(&root).expect("first acquire succeeds");
        assert!(root.join(DAEMON_PID_FILENAME).exists());
        drop(lock);
        assert!(!root.join(DAEMON_PID_FILENAME).exists());
    }

    #[test]
    fn lock_refuses_second_alive_daemon() {
        let root = unique_root("second-daemon");
        let _first = DaemonLock::acquire(&root).expect("first acquire succeeds");
        let error = DaemonLock::acquire(&root).expect_err("second acquire must refuse");
        assert!(matches!(error, DaemonError::DaemonAlreadyRunning { .. }));
    }

    #[test]
    fn stale_lock_is_replaced() {
        let root = unique_root("stale");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(DAEMON_PID_FILENAME), "4294967295\n").unwrap();
        // 4294967295 casts to pid -1, which is explicitly rejected, so the
        // stale lock is replaced.
        let lock = DaemonLock::acquire(&root).expect("stale lock is replaceable");
        assert_eq!(
            fs::read_to_string(lock.path()).unwrap().trim(),
            std::process::id().to_string()
        );
    }

    #[test]
    fn daemon_is_running_reflects_live_lock() {
        let root = unique_root("is-running");
        assert!(!daemon_is_running(&root));
        let _lock = DaemonLock::acquire(&root).unwrap();
        assert!(daemon_is_running(&root));
    }
}
