//! Sends one bridge message to the real backend, outside the window.
//!
//! Used for evidence and for checking the trust surface without clicking:
//! the exact command, the exact reply, the same storage root the app uses.
//!
//! ```text
//! cargo run -p evo-desktop --example bridge_cli -- '{"id":1,"cmd":"get_privacy"}'
//! ```

fn main() {
    let body = std::env::args().nth(1).expect("one JSON bridge message");
    let root = std::env::var("EVO_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            evo_storage::prepare_storage_root()
                .unwrap_or_else(|_| evo_storage::canonical_storage_root())
        });
    eprintln!("EVO-BRIDGE root {}", root.display());
    match evo_desktop::bridge::parse(body.as_bytes()) {
        Ok(request) => {
            println!(
                "{}",
                evo_desktop::bridge::Backend::new(root).respond(&request)
            );
        }
        Err(error) => {
            eprintln!("refused: {}", error.to_json());
            std::process::exit(2);
        }
    }
}
