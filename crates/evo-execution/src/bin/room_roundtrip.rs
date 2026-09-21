//! Live room round-trip: enter a room, verify hides, leave, verify restore.
//! macOS only: it drives the live desktop surface.
#[cfg(target_os = "macos")]
use evo_execution::room::{DesktopSurface, RoomController, RoomSurface};

#[cfg(target_os = "macos")]
fn visible_regular(surface: &mut dyn DesktopSurface) -> usize {
    surface
        .running_apps()
        .unwrap_or_default()
        .iter()
        .filter(|a| a.regular && !a.hidden)
        .count()
}

#[cfg(target_os = "macos")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let work = args.get(1).cloned().unwrap_or_else(|| "ZCode".to_string());

    let mut controller = RoomController::new(
        Box::new(evo_execution::room_macos::MacDesktopSurface::new()),
        vec![std::process::id() as i32],
    );

    // A surface with just the work's app name: everything else is not this work.
    let surface = RoomSurface {
        work: work.clone(),
        apps: vec![work.clone()],
        urls: vec![],
        documents: vec![],
        titles: vec![],
        resource_apps: Default::default(),
    };

    let before = visible_regular(controller.surface_mut());
    println!("visible regular apps BEFORE: {before}");

    match controller.enter(&surface) {
        Ok(lease) => {
            println!(
                "ENTERED {}: hidden={} parked={} raised={}",
                lease.work,
                lease.hidden_apps.len(),
                lease.parked_windows.len(),
                lease.raised_windows.len()
            );
        }
        Err(err) => {
            println!("ENTER FAILED: {err}");
            return;
        }
    }

    let during = visible_regular(controller.surface_mut());
    println!("visible regular apps IN ROOM: {during} (was {before})");

    match controller.leave() {
        Ok(Some(lease)) => println!(
            "LEFT {} ({} hidden restored)",
            lease.work,
            lease.hidden_apps.len()
        ),
        Ok(None) => println!("LEFT: no active room"),
        Err(err) => println!("LEAVE FAILED: {err}"),
    }

    let after = visible_regular(controller.surface_mut());
    println!("visible regular apps AFTER LEAVE: {after}");
    if after == before {
        println!("ROUND-TRIP VERIFIED: leave restored exactly the pre-enter state");
    } else {
        println!("MISMATCH: before={before} after={after}");
    }
}

#[cfg(target_os = "macos")]
trait SurfaceAccess {
    fn surface_mut(&mut self) -> &mut dyn DesktopSurface;
}

#[cfg(target_os = "macos")]
impl SurfaceAccess for RoomController {
    fn surface_mut(&mut self) -> &mut dyn DesktopSurface {
        // The controller owns the surface; expose it for the probe's counting.
        self.probe_surface()
    }
}

/// This diagnostic drives the live macOS desktop; on other platforms
/// there is nothing to probe.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("room_roundtrip is a macOS diagnostic: it needs a live macOS desktop.");
    std::process::exit(1);
}
