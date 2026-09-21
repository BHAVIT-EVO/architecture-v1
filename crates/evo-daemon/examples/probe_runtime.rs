//! Developer probe: replicates `runtime_start_vertical_runtime_exposes_boundary_receiver`
//! outside the test harness so the teardown crash can be iterated on.
//!
//! Usage:
//!   cargo run -p evo-daemon --example probe_runtime

use evo_daemon::runtime::Runtime;
use evo_storage::Storage;
use std::path::PathBuf;
use std::time::{SystemTime, Duration};

fn main() {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("evo-daemon-probe-runtime-{nanos}"));
    let _guard = Storage::with_thread_root(root.clone());

    let runtime = Runtime::new();
    let result = runtime.start_vertical_runtime_with_storage_root(unique_root("probe"));
    match result {
        Ok((_handle, receiver)) => {
            println!("runtime started; boundary receiver empty: {}", receiver.try_recv().is_err());
            // Let real capture run for a moment, then tear down like the test.
            std::thread::sleep(Duration::from_secs(3));
            println!("dropping handle");
            drop(_handle);
            println!("handle dropped");
        }
        Err(err) => println!("runtime error: {err}"),
    }
    let _ = root;
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("evo-daemon-{label}-{nanos}"))
}
