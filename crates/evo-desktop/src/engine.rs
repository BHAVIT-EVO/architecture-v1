//! Ledger engine lifecycle: the desktop ensures the compute process runs.
//!
//! The engine (`evo-ledger-engine`) rebuilds `ledger.db` from the
//! observation log in its own process — the screenpipe architecture that
//! keeps all derivation off the UI thread. Without a supervisor it only
//! runs when started by hand, which means a reboot silently freezes the
//! Works list. This manager owns that lifecycle the same way
//! [`crate::daemon::DaemonManager`] owns the capture daemon: locate the
//! binary beside the desktop shell, start it if no live engine holds the
//! pid file, and restart it when it dies.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Whether the engine process is alive, from the pid file it maintains.
fn engine_alive(data_dir: &std::path::Path) -> bool {
    let pid_path = data_dir.join("ledger-engine.pid");
    let Ok(contents) = std::fs::read_to_string(&pid_path) else {
        return false;
    };
    let Ok(pid) = contents.trim().parse::<u32>() else {
        return false;
    };
    // `kill -0` tests liveness without signalling.
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Locates the engine binary next to the desktop shell executable — the
/// same layout contract the daemon uses (bundled app: `Contents/MacOS`;
/// development: the shared cargo output directory).
pub fn engine_binary_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidate = dir.join("evo-ledger-engine");
    candidate.is_file().then_some(candidate)
}

/// Owns the spawned engine child process.
pub struct EngineManager {
    child: Option<Child>,
    storage_root: PathBuf,
    spawned: bool,
    /// Consecutive failed spawn attempts, so a broken binary cannot turn
    /// the UI thread into a respawn loop.
    failures: u32,
}

impl EngineManager {
    /// Constructs the manager and starts the engine unless a live one
    /// already holds the pid file.
    pub fn new(storage_root: PathBuf) -> Self {
        let mut manager = Self {
            child: None,
            storage_root,
            spawned: false,
            failures: 0,
        };
        manager.spawn();
        manager
    }

    /// The directory the engine keeps its pid file in: the storage root,
    /// where the engine writes it (and where capture's self-write
    /// exclusion already covers it).
    fn data_dir(&self) -> PathBuf {
        self.storage_root.clone()
    }

    /// Starts the engine when none is running.
    pub fn spawn(&mut self) {
        self.spawned = true;
        if engine_alive(&self.data_dir()) {
            return; // already supervised elsewhere or by a previous shell
        }
        if self.failures >= 3 {
            return; // stop hammering a broken binary
        }
        let Some(binary) = engine_binary_path() else {
            return;
        };
        let command = Command::new(&binary)
            .env("EVO_STORAGE_ROOT", &self.storage_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match command {
            Ok(child) => {
                self.child = Some(child);
                self.failures = 0;
            }
            Err(err) => {
                self.failures += 1;
                eprintln!("EVO-DESKTOP failed to spawn ledger engine {binary:?}: {err}");
            }
        }
    }

    /// Re-checks the engine: when it has exited, restart it. Call from the
    /// shell's logic tick — the liveness check is one file read and one
    /// `kill -0` per call, never a computation.
    pub fn poll(&mut self) {
        if !self.spawned {
            self.spawn();
            return;
        }
        // Reap our own child if it exited, then ensure one is running.
        if let Some(child) = self.child.as_mut() {
            if child.try_wait().map(|w| w.is_some()).unwrap_or(true) {
                self.child = None;
                self.failures = self.failures.saturating_add(1);
                self.spawn();
            }
            return;
        }
        if !engine_alive(&self.data_dir()) {
            self.spawn();
        }
    }
}
