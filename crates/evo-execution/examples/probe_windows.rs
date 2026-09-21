//! Probe: print the live window list as the macOS executor sees it.
//!
//! Used for end-to-end verification that window-title targets can be resolved
//! honestly on a real machine. This is a verification tool, not product code.

fn main() {
    use evo_execution::WindowSource;
    let source = evo_execution::macos::SystemWindowSource;
    match source.windows() {
        Ok(windows) => {
            println!("window count: {}", windows.len());
            for window in windows {
                println!(
                    "title={:?} owner={} pid={}",
                    window.title, window.owner_name, window.owner_pid
                );
            }
        }
        Err(reason) => {
            eprintln!("cannot inspect windows: {reason}");
            std::process::exit(2);
        }
    }
}
