//! Designation dev tool.
//!
//! Submits one explicit user designation (RFC-0011) to the running daemon
//! over its canonical socket channel and prints the honest response. This is
//! the same client path the desktop shell uses — it exists so scripts and
//! developers can drive the designation flow without the UI.
//!
//! Usage:
//!   cargo run -p evo-daemon --example designate -- "<witnessed subject>"
//!
//! `EVO_STORAGE_ROOT` selects the storage root (default: the canonical shared
//! temp root, the same default the daemon and desktop use).

use evo_daemon::designation::{submit_designation, DesignationResponse};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: designate <witnessed subject>");
        std::process::exit(2);
    }
    let subject = &args[1];
    let root = evo_storage::canonical_storage_root();
    match submit_designation(&root, subject) {
        Ok(DesignationResponse::Accepted) => {
            println!("accepted");
        }
        Ok(DesignationResponse::Rejected(reason)) => {
            println!("rejected: {reason}");
            std::process::exit(3);
        }
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}
