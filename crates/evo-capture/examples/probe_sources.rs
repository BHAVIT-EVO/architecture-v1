//! Developer probe: replicates the daemon runtime composition (AX source +
//! URL poller + FSEvents watcher + a concurrent worker consuming signals)
//! to reproduce the teardown crash outside the test harness.
//!
//! Usage:
//!   cargo run -p evo-capture --example probe_sources


#[cfg(target_os = "macos")]
mod platform_ref {
    #![allow(unused_imports, dead_code)]
    use evo_capture::{FSEventsWatcher, MacOSEventSource, MacOSSignal, MacOSURLPoller};
    use std::sync::mpsc::channel;
    use std::time::Duration;
}

#[cfg(target_os = "macos")]
use platform_ref::*;


#[cfg(target_os = "macos")]
fn main() {
    let (signal_sender, signal_receiver) = channel::<MacOSSignal>();

    let fs = FSEventsWatcher::start({
        let sender = signal_sender.clone();
        move |signal| {
            let _ = sender.send(signal);
        }
    });
    println!(
        "fs watcher start: {:?}",
        fs.as_ref().map(|_| "ok").map_err(|e| e.to_string())
    );

    let url = MacOSURLPoller::start({
        let sender = signal_sender.clone();
        move |signal| {
            let _ = sender.send(signal);
        }
    });
    println!(
        "url poller start: {:?}",
        url.as_ref().map(|_| "ok").map_err(|e| e.to_string())
    );

    let source = MacOSEventSource::new(
        evo_capture::desktop_shell_pid(),
        {
        let sender = signal_sender.clone();
        move |signal| {
            let _ = sender.send(signal);
        }
    });
    println!(
        "event source start: {:?}",
        source.as_ref().map(|_| "ok").map_err(|e| e.to_string())
    );

    // A concurrent worker that drains signals (mirrors the daemon worker).
    let worker = std::thread::spawn(move || {
        let mut count = 0usize;
        while let Ok(signal) = signal_receiver.recv() {
            count += 1;
            let _ = signal;
            if count % 10 == 0 {
                eprintln!("EVO-PROBE worker consumed {count} signals");
            }
        }
        eprintln!("EVO-PROBE worker exiting after {count} signals");
    });

    std::thread::sleep(Duration::from_secs(3));
    println!("dropping watchers");
    drop(fs);
    drop(url);
    drop(source);
    drop(signal_sender);
    println!("waiting for worker");
    let _ = worker.join();
    println!("probe done");
}

/// Off-macOS this target does not exist: it exercises macOS APIs directly.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("probe_sources is a macOS diagnostic target; nothing to do on this platform.");
}
