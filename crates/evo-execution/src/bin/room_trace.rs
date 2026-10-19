//! Traces the enter classification: which apps would be work vs hidden.
//! macOS only: it drives the live desktop surface.
#[cfg(target_os = "macos")]
use evo_execution::room::{DesktopSurface, RoomSurface};
#[cfg(target_os = "macos")]
use evo_execution::room_macos::MacDesktopSurface;

#[cfg(target_os = "macos")]
fn main() {
    let mut surface = MacDesktopSurface::new();
    let apps = surface.running_apps().expect("apps");

    let work_surface = RoomSurface {
        work: "ZCode".into(),
        apps: vec!["".into(), "ZCode".into(), "zcode".into()],
        urls: vec![],
        documents: vec![
            "/Users/bhavitsaini/Desktop/Evo-Evolution.nosync/crates/evo-capture/src/adapters/macos.rs".into(),
        ],
        titles: vec!["ZCode".into()],
        resource_apps: Default::default(),
    };

    println!("REGULAR APPS:");
    for app in apps.iter().filter(|a| a.regular) {
        println!("  pid={} hidden={} {}", app.pid, app.hidden, app.name);
        let windows = surface.windows_of(app.pid).unwrap_or_default();
        for w in &windows {
            let m = evo_execution::room::window_match(w, &work_surface);
            println!(
                "    win id={} match={:?} doc={:?} title={}",
                w.window_id,
                m,
                w.ax_document
                    .as_deref()
                    .map(|d| d.chars().take(40).collect::<String>()),
                w.title.chars().take(30).collect::<String>()
            );
        }
    }
}

/// This diagnostic drives the live macOS desktop; on other platforms
/// there is nothing to probe.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("room_trace is a macOS diagnostic: it needs a live macOS desktop.");
    std::process::exit(1);
}
