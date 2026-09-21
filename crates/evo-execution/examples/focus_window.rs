//! Real restoration-execution dev tool.
//!
//! Performs one real macOS restoration action — focusing the open window with
//! the given exact title — through the production executor and prints the
//! honest status. This is the same execution path the desktop's Resume action
//! uses; it exists so the real-machine behavior can be verified directly.
//!
//! Usage:
//!   cargo run -p evo-execution --example focus_window -- "<exact window title>"

use evo_execution::TargetStatus;

fn main() {
    let title = std::env::args()
        .nth(1)
        .expect("usage: focus_window <exact window title>");
    let executor = evo_execution::MacOSExecutor::new();
    // The inherent `focus_window` is a private helper; the public execution
    // surface is the PlatformExecutor trait method.
    match evo_execution::engine::PlatformExecutor::focus_window(&executor, &title) {
        TargetStatus::Opened { detail } => println!("opened: {detail}"),
        TargetStatus::Unavailable { reason } => {
            println!("unavailable: {reason}");
            std::process::exit(3);
        }
        TargetStatus::Ambiguous { reason } => {
            println!("ambiguous: {reason}");
            std::process::exit(4);
        }
        TargetStatus::Failed { reason } => {
            println!("failed: {reason}");
            std::process::exit(5);
        }
        TargetStatus::Unsupported { reason } => {
            println!("unsupported: {reason}");
            std::process::exit(6);
        }
    }
}
