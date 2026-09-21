//! Probe: does the real FSEvents watcher deliver events for a write under the
//! checkout's `target/` directory in this environment?
use evo_capture::macos_fsevents::FSEventsWatcher;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    let count = Arc::new(AtomicUsize::new(0));
    let seen = count.clone();
    let watcher = FSEventsWatcher::start(move |signal| {
        seen.fetch_add(1, Ordering::SeqCst);
        println!("  delivered: {signal:?}");
    });
    let _watcher = match watcher {
        Ok(w) => w,
        Err(err) => {
            println!("FSEVENTS-START-FAILED: {err}");
            std::process::exit(2);
        }
    };
    println!("watcher started; waiting 2.5s for the stream to go live");
    std::thread::sleep(Duration::from_millis(2500));

    let dir = std::path::PathBuf::from("target/evo-fsevents-probe");
    std::fs::create_dir_all(&dir).expect("create probe dir");
    let file = dir.join("probe.txt");
    std::fs::write(&file, "hello\n").expect("write probe file");
    println!("wrote {}", std::path::absolute(&file).unwrap_or(file.clone()).display());

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        if count.load(Ordering::SeqCst) > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let n = count.load(Ordering::SeqCst);
    let _ = std::fs::remove_dir_all(&dir);
    println!("SIGNALS DELIVERED: {n}");
    std::process::exit(if n > 0 { 0 } else { 1 });
}
