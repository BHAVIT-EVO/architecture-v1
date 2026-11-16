//! Evo desktop shell entry point.
//!
//! Launches a real macOS-native Evo window. No browser is involved.

use evo_desktop::app::EvoApp;
use evo_desktop::observability::{self, Level, Logger};
use evo_desktop::theme;

fn main() -> eframe::Result {
    let _ = evo_storage::apply_test_home();
    evo_storage::tighten_all();
    // GUI apps launched via LaunchServices often have no usable stderr; Rust's
    // eprintln! then panics on the failed write and aborts the process. Make
    // fd 2 point at /dev/null so logging can never crash the app.
    #[cfg(target_os = "macos")]
    unsafe {
        unsafe extern "C" {
            fn dup2(oldfd: i32, newfd: i32) -> i32;
        }
        let log_path = std::env::var("HOME")
            .ok()
            .map(|h| std::path::PathBuf::from(h).join("Library/Application Support/Evo/logs/stderr.log"));
        let f = log_path
            .and_then(|p| { let _ = std::fs::create_dir_all(p.parent()?); std::fs::OpenOptions::new().create(true).append(true).open(p).ok() })
            .or_else(|| std::fs::File::open("/dev/null").ok());
        if let Some(f) = f {
            use std::os::unix::io::AsRawFd;
            dup2(f.as_raw_fd(), 2);
        }
    }
    // Crashes always print; they are only written to disk when the user opted
    // into crash reports (EVO_CRASH_REPORTS=1, the preference the app writes).
    let log_dir = observability::default_log_dir();
    let report_crashes = std::env::var("EVO_CRASH_REPORTS").as_deref() == Ok("1");
    observability::install_panic_hook(log_dir.clone(), report_crashes);
    let storage_root = evo_storage::canonical_storage_root();
    Logger::new(log_dir).event(
        Level::Info,
        "app_launch",
        &[
            ("version", env!("CARGO_PKG_VERSION")),
            ("crash_reports", if report_crashes { "on" } else { "off" }),
            ("storage_root", &storage_root.display().to_string()),
        ],
    );

    // "Hey Evo" lives in a per-user LaunchAgent, not in this process. Check it
    // on every launch and install it when it is missing, off the main thread
    // so a cold bootstrap cannot delay the window.
    std::thread::spawn(|| match evo_desktop::voice_agent::ensure_agent_running() {
        Ok(outcome) => eprintln!("EVO-VOICE: {}", outcome.describe()),
        Err(err) => eprintln!("EVO-VOICE: {err}"),
    });

    // EVO_UI selects the shell. `native` (the default) is the egui interface;
    // `web` hosts the new HTML UI in a WKWebView over the same window. The
    // egui interface stays compiled either way (ADR 0002).
    // S9-4: the web shell is the default now. `EVO_UI=native` still reaches
    // the egui interface, so nothing is deleted.
    let web_shell = std::env::var("EVO_UI").unwrap_or_else(|_| "web".into()) == "web";

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Evo")
            // Home's column is 960 wide with 32pt margins; the window opens
            // wide enough for that column and tall enough to show the whole
            // first screen without scrolling (§6).
            .with_inner_size([1200.0, 880.0])
            // The dark chrome: the environment runs edge to edge, under the
            // traffic lights, so the window is one material (DESIGN.md:
            // Environment). `with_titlebar_shown(false)` makes the bar
            // transparent in egui 0.35; combined with the full-size content
            // view the header shares its top band with the traffic lights.
            .with_titlebar_shown(false)
            .with_fullsize_content_view(true)
            // Evo's header carries the wordmark; the native title text would
            // duplicate it under the traffic lights.
            .with_title_shown(false)
            // The declared floor is the one the layout is actually designed to
            // degrade to: below `theme::STACK_BREAKPOINT` the detail view stacks
            // its contextual column under the main column instead of splitting.
            // Stating a wider minimum than that would deny the user a narrowness
            // the design already handles.
            .with_min_inner_size([theme::MIN_WINDOW_WIDTH, 520.0]),
        ..Default::default()
    };
    if web_shell {
        let outcome = eframe::run_native(
            "Evo",
            options,
            Box::new(|cc| Ok(Box::new(evo_desktop::webshell::WebUiApp::new(cc)))),
        );
        eprintln!("EVO-WEB run_native returned {outcome:?}");
        return outcome;
    }
    eframe::run_native("Evo", options, Box::new(|cc| Ok(Box::new(EvoApp::new(cc)))))
}
