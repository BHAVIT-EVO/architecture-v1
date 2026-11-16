//! Opt-in focused-document check, limited to one explicitly supplied fixture.
//!
//! Inspect only (no OS action):
//! cargo run -p evo-execution --example verify_fixture_document -- /absolute/fixture.txt
//! Open that exact fixture, then inspect its focused AXDocument:
//! cargo run -p evo-execution --example verify_fixture_document -- --open /absolute/fixture.txt
//!
//! No desktop-wide window enumeration, capture, or title matching is performed.

use evo_execution::macos;
use evo_execution::restore::{DispatchResult, TargetReceipt, VerificationResult};
use std::path::Path;
use std::time::{Duration, Instant};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let first = args
        .next()
        .ok_or("supply the exact absolute path of a fixture document")?;
    let open = first == "--open";
    let resource = if open {
        args.next()
            .ok_or("--open requires the exact fixture path")?
    } else {
        first
    };
    if args.next().is_some()
        || !Path::new(&resource).is_absolute()
        || !Path::new(&resource).is_file()
    {
        return Err(
            "supply one existing absolute fixture document path (optionally preceded by --open)"
                .into(),
        );
    }

    let dispatch = if open {
        match macos::open_target("file", &resource) {
            Ok(()) => DispatchResult::OpenAccepted,
            Err(reason) => DispatchResult::Failed { reason },
        }
    } else {
        DispatchResult::NotDispatched {
            reason: "inspect-only fixture probe".into(),
        }
    };
    let deadline = Instant::now() + Duration::from_secs(3);
    let verification = loop {
        let result = macos::verify_resource("file", &resource, None, None);
        if !open
            || result == VerificationResult::Verified
            || Instant::now() >= deadline
            || matches!(&dispatch, DispatchResult::Failed { .. })
        {
            break result;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let receipt = TargetReceipt {
        resource,
        dispatch,
        verification,
    };
    println!("{receipt:#?}");
    if receipt.verification == VerificationResult::Verified
        && !matches!(receipt.dispatch, DispatchResult::Failed { .. })
    {
        Ok(())
    } else {
        Err("fixture document postcondition was not verified; see receipt".into())
    }
}
