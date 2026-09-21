//! Real-machine verification of the macOS executor.
//!
//! Verifies the honest resolution of a window-title target against the live
//! window list:
//!
//! - a title that is genuinely open → the window is raised and the owning
//!   application activated (`Opened`);
//! - a title that is not open → `Unavailable`, never a guess;
//! - a URL target → `Opened`/`Failed` through NSWorkspace.
//!
//! This is a verification tool, not product code.
//!
//! The window under verification is taken from the live window list unless one
//! is named on the command line, so this file names no application (§17): which
//! window it exercises is whatever this machine actually has open.
//!
//! Usage:
//!   cargo run -p evo-execution --example verify_execution
//!   cargo run -p evo-execution --example verify_execution "<open window title>"

use evo_execution::engine::PlatformExecutor;
use evo_execution::macos::SystemWindowSource;
use evo_execution::{MacOSExecutor, WindowSource};

fn main() {
    let executor = MacOSExecutor::new();

    // A window this machine really has open, so the raise action is harmless.
    // Naming one here would hardcode a real application into the repository and
    // would also be a guess about what is running; ask the machine instead.
    match std::env::args().nth(1).or_else(first_open_window) {
        Some(open_title) => {
            let open = executor.focus_window(&open_title);
            println!("focus_window({open_title:?}) -> {open:?}");
        }
        None => {
            // Empty on a real Mac means this process is not Accessibility-
            // trusted, not that nothing is open. The negative cases below still
            // verify; say plainly which one does not.
            println!(
                "focus_window(open) -> not exercised: no window is visible to this \
                 process. Window titles come from the Accessibility API, which \
                 returns nothing to an untrusted process; grant Accessibility, or \
                 pass a title explicitly."
            );
        }
    }

    // A title that is not open: must be Unavailable, never a guess.
    let missing = executor.focus_window("Evo does not own this window title");
    println!("focus_window(missing) -> {missing:?}");

    // URL through NSWorkspace.
    let url = executor.open_url("https://example.com");
    println!("open_url -> {url:?}");

    // A real local file through NSWorkspace.
    let probe = std::env::temp_dir().join("evo-execution-open-probe.txt");
    std::fs::write(&probe, "probe\n").expect("probe file");
    let file = executor.open_file(probe.to_str().expect("utf8 path"));
    println!("open_file -> {file:?}");

    // A missing file must be Unavailable, never a substitute.
    let missing = executor.open_file(
        std::env::temp_dir()
            .join("evo-execution-definitely-missing")
            .to_str()
            .expect("utf8 path"),
    );
    println!("open_file(missing) -> {missing:?}");
}

/// A window this machine actually has open, if any is visible to this process.
///
/// The lowest title in sorted order, so two runs against an unchanged desktop
/// exercise the same window — the live list arrives in application-launch
/// order, which is not something a verification run should depend on.
fn first_open_window() -> Option<String> {
    SystemWindowSource
        .windows()
        .ok()?
        .into_iter()
        .map(|window| window.title)
        .filter(|title| !title.trim().is_empty())
        .min()
}
