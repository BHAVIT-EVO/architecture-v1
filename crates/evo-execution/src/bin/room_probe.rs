//! Live probe of the room DesktopSurface. macOS only: it drives the
//! live desktop surface.
#[cfg(target_os = "macos")]
use evo_execution::room::DesktopSurface;
#[cfg(target_os = "macos")]
use evo_execution::room_macos::MacDesktopSurface;

#[cfg(target_os = "macos")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut surface = MacDesktopSurface::new();
    let apps = surface.running_apps().expect("apps");
    if args.get(1).map(|a| a == "hide-test").unwrap_or(false) {
        // Find Finder and hide it
        for app in &apps {
            if app.name == "Finder" && !app.hidden {
                println!("hiding Finder (pid {})...", app.pid);
                match surface.hide_app(app.pid) {
                    Ok(()) => println!("hide OK"),
                    Err(e) => println!("hide FAILED: {e}"),
                }
                let re = surface.running_apps().unwrap();
                println!(
                    "Finder hidden now: {}",
                    re.iter()
                        .find(|a| a.pid == app.pid)
                        .map(|a| a.hidden)
                        .unwrap_or(true)
                );
                return;
            }
        }
        println!("Finder not found visible");
        return;
    }
    if args.get(1).map(|a| a == "minimize-test").unwrap_or(false) {
        // Minimize one Finder window via AX (park).
        let finder = apps.iter().find(|a| a.name == "ZCode" && !a.hidden);
        let Some(finder) = finder else {
            println!("Finder not visible");
            return;
        };
        let windows = surface.windows_of(finder.pid).unwrap_or_default();
        println!(
            "ZCode windows: {:?}",
            windows
                .iter()
                .map(|w| (
                    w.window_id,
                    w.minimized,
                    w.title.chars().take(20).collect::<String>()
                ))
                .collect::<Vec<_>>()
        );
        let Some(w) = windows.iter().find(|w| w.window_id != 0 && !w.minimized) else {
            println!("no addressable window");
            return;
        };
        println!("minimizing Finder window {w:?}...");
        match surface.minimize_window(finder.pid, w.window_id) {
            Ok(()) => println!("minimize OK"),
            Err(e) => println!("minimize FAILED: {e}"),
        }
        return;
    }
    let regular_count = apps.iter().filter(|a| a.regular).count();
    println!(
        "apps: {} ({} regular, {} hidden)",
        apps.len(),
        regular_count,
        apps.iter().filter(|a| a.hidden).count()
    );
    for app in apps.iter().filter(|a| a.regular).take(60) {
        println!("  pid={} hidden={} {}", app.pid, app.hidden, app.name);
    }
}
// (minimize test appended)

/// This diagnostic drives the live macOS desktop; on other platforms
/// there is nothing to probe.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("room_probe is a macOS diagnostic: it needs a live macOS desktop.");
    std::process::exit(1);
}
