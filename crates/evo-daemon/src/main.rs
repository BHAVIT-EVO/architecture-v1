use evo_daemon::{daemon_lock, Daemon, DaemonError};

use std::thread;

fn main() {
    if let Err(err) = real_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), DaemonError> {
    // Canonical state lives in the persistent Application Support location; a
    // populated legacy temp root is migrated forward safely at startup
    // (BE-TRACE-0001 §3.4). A migration failure preserves the legacy data and
    // fails honestly — the daemon exits non-zero rather than pretending.
    let storage_root = evo_storage::prepare_storage_root()?;
    // One daemon per storage root: a second writer to the same append-only
    // canonical logs would race. The desktop shell spawns this worker, so
    // the lock prevents a stale daemon from being joined by a new one.
    let _lock = daemon_lock::DaemonLock::acquire(&storage_root)?;

    // When Accessibility is not yet granted, surface the native System
    // Settings grant before the honest refusal so the user knows exactly
    // what permission is required and how to grant it.
    if !evo_capture::accessibility_permission_granted() {
        evo_capture::prompt_for_accessibility_permission();
    }

    let daemon = Daemon::new();
    match daemon.start_vertical_runtime_with_storage_root(storage_root) {
        Ok((runtime, restoration_boundary)) => {
            // The daemon remains the canonical runtime owner. The boundary
            // results are consumed by the desktop shell through the durable
            // storage boundary; here they are drained for error visibility.
            let printer = thread::spawn(move || {
                while let Ok(result) = restoration_boundary.recv() {
                    if let Err(err) = result {
                        eprintln!("{err}");
                    }
                }
            });

            match runtime.run() {
                Ok(()) => Ok(()),
                Err(DaemonError::UnsupportedPlatform) => Ok(()),
                Err(err) => {
                    drop(printer);
                    Err(err)
                }
            }
        }
        // A daemon that cannot start its runtime must exit non-zero so the
        // desktop shell can classify the failure honestly from the process
        // exit (a zero exit would be reported as capture running).
        Err(err) => Err(err),
    }
}

