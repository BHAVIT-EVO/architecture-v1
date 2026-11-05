//! Opt-in native identity check for the synthetic AppKit fixture.
//! Usage: verify_fixture_identity --raise <pid> <launch-bits> <AXIdentifier> <absolute-file>
//! Logical capture ids below belong to the controlled fixture only, not Evo's
//! capture registry. This verifies the native adapter, not capture integration.

use evo_execution::restore::{RestoreTarget, VerificationResult, restore_with_receipt};
use evo_observation::SurfaceIdentity;

fn main() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.len() != 5 || arguments[0] != "--raise" {
        return Err(
            "supply --raise <fixture-pid> <launch-bits> <AXIdentifier> <absolute-fixture-file>"
                .into(),
        );
    }
    let process_id = arguments[1].parse().map_err(|_| "invalid fixture PID")?;
    let resource = &arguments[4];
    if !std::path::Path::new(resource).is_absolute() || !std::path::Path::new(resource).is_file() {
        return Err("fixture file must be an existing absolute path".into());
    }
    let surface = SurfaceIdentity {
        capture_session: "explicit-native-fixture".into(),
        window_id: "window-1".into(),
        process_id,
        process_started_at: arguments[2].clone(),
        window_identifier: Some(arguments[3].clone()),
        browser_window: None,
        browser_profile: None,
        browser_session: None,
        browser_tab: None,
    };
    if !surface.is_valid() {
        return Err("invalid fixture identity".into());
    }
    let receipt = restore_with_receipt(&[RestoreTarget {
        resource: resource.clone(),
        app: None,
        surface: Some(surface),
    }]);
    println!("{receipt:#?}");
    if receipt.targets.len() == 1
        && receipt.targets[0].verification == VerificationResult::Verified
        && receipt.outcome.raised == vec![resource.clone()]
    {
        Ok(())
    } else {
        Err("native fixture identity/resource postcondition was not verified".into())
    }
}
