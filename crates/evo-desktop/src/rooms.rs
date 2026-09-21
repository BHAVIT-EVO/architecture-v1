//! The room runtime: the desktop-level state of Rooms, shared between
//! the window UI and the menu-bar presence.
//!
//! A room is a fact about the *desktop* (which apps are hidden, which
//! windows are parked), not about Evo's window — so its state must live where both surfaces reach it and its
//! commands must execute even while the window is hidden. The runtime is
//! one `Arc<Mutex<…>>` the app and the presence target share; menu-bar
//! and hotkey commands execute synchronously on the main thread from the
//! action itself, never queued behind a frame the hidden window cannot
//! paint.
//!
//! The browser panel (Stage 3 panes) is window state and stays with the
//! app — but its *hibernation* (releasing panes) is a desktop-level
//! concern, so it lives here too: leaving or switching a room releases
//! the work's web presence no matter who asked.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use evo_execution::restore::{self, RestoreTarget};

/// Commands the runtime executes (from the menu bar, the hotkey, or the
/// app's own surfaces). `Open` is about the window, not the room: the
/// runtime ignores it and the presence layer shows the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomCommand {
    Enter(usize),
    Leave,
    Cycle,
    Open,
}

/// The shared room state.
pub struct RoomRuntime {
    /// The containment controller (hide/park/raise; the lease is truth).
    pub room: evo_execution::room::RoomController,
    /// Containment surfaces, parallel to the engine's thread order.
    pub surfaces: Vec<evo_execution::room::RoomSurface>,
    /// The work names for those surfaces (what the menu bar shows).
    pub names: Vec<String>,
    /// The active room's work name.
    pub active: Option<String>,
    /// Set when entering a room with web pages: the app opens the
    /// in-app browser panel for them (drained by the next frame).
    pub wants_browser: bool,
    /// The active work's web panes (Stage 3). Hydration is the app's
    /// (it needs the window); hibernation is a room transition.
    pub web: crate::websurface::WebSurfaces,
    /// A note from the last room transition, drained by the app's next
    /// frame (the menu-bar path has no UI of its own to show it).
    pub pending_note: Option<String>,
}

impl RoomRuntime {
    pub fn new() -> Self {
        Self {
            room: evo_execution::room::RoomController::new(
                Box::new(evo_execution::room_macos::MacDesktopSurface::new()),
                // Evo itself must never be hidden by its own containment.
                vec![std::process::id() as i32],
            ),
            surfaces: Vec::new(),
            names: Vec::new(),
            active: None,
            wants_browser: false,
            web: crate::websurface::WebSurfaces::new(),
            pending_note: None,
        }
    }

    /// Enters the room at `index` (switching from any active room):
    /// hibernates the previous room's web panes and establishes
    /// containment, then restores the work's place. Returns the honest
    /// note.
    pub fn enter_at(&mut self, index: usize) -> Option<String> {
        let Some(surface) = self.surfaces.get(index).cloned() else {
            return None;
        };
        // Switching rooms hibernates the room being left: its web panes
        // release, so the machine pays for one room's web presence at a
        // time.
        if let Some(previous) = self.active.clone() {
            if previous != surface.work {
                self.web.hibernate();
            }
        }
        match self.room.switch(&surface) {
            Ok(lease) => {
                self.active = Some(lease.work.clone());
                // Opening the room opens the work — but only once
                // containment holds. Restoring before the switch would
                // leave opened-but-unowned windows behind when the
                // switch fails.
                let restore_note = self.restore_work(&surface);
                if !surface.urls.is_empty() {
                    self.wants_browser = true;
                }
                Some(format!(
                    "Entered \"{}\": {} apps hidden, {} windows parked, {} raised.{restore_note}",
                    lease.work,
                    lease.hidden_apps.len(),
                    lease.parked_windows.len(),
                    lease.raised_windows.len()
                ))
            }
            Err(err) => Some(format!("Room not entered: {err}")),
        }
    }

    /// Leaves the active room, restoring the desktop exactly as it was
    /// and releasing the work's web panes.
    pub fn leave(&mut self) -> Option<String> {
        self.web.hibernate();
        match self.room.leave() {
            Ok(Some(lease)) => {
                self.active = None;
                Some(format!(
                    "Left \"{}\" — your desktop is back exactly as it was.",
                    lease.work
                ))
            }
            Ok(None) => None,
            Err(err) => Some(format!("Could not leave the room: {err}")),
        }
    }

    /// The next room for the cycle hotkey: the work after the active one
    /// (wrapping), or the most recent work.
    pub fn next_index(&self) -> usize {
        let count = self.surfaces.len();
        if count == 0 {
            return 0;
        }
        let current = self
            .names
            .iter()
            .position(|name| Some(name) == self.active.as_ref())
            .or_else(|| {
                self.surfaces
                    .iter()
                    .position(|surface| Some(&surface.work) == self.active.as_ref())
            });
        match current {
            Some(i) => (i + 1) % count,
            None => 0,
        }
    }

    /// Restores the work's place, generically: every witnessed app is
    /// launched (plain `open -a`, so a running app is simply brought
    /// forward — never a second instance), then every document opens
    /// through the application it was witnessed in, or the OS default
    /// handler where no app was witnessed. No application is named in
    /// code or data; the OS does the opening, and every artifact opens
    /// regardless of whether its app is running. URLs are not opened
    /// here — they hydrate the work's own browser panel (per-work
    /// stores, so logins stay isolated per work with no browser named
    /// anywhere).
    ///
    /// Returns the note fragment for the enter report (empty when there
    /// was nothing to restore).
    fn restore_work(&self, surface: &evo_execution::room::RoomSurface) -> String {
        let mut seen = BTreeSet::new();
        let mut launched = 0usize;
        let mut failed: Vec<String> = Vec::new();
        for app in &surface.apps {
            let app = app.trim();
            if app.is_empty() || !seen.insert(app.to_lowercase()) {
                continue;
            }
            match restore::launch_app(app) {
                Ok(()) => launched += 1,
                Err(err) => failed.push(format!("{app}: {err}")),
            }
        }
        let targets: Vec<RestoreTarget> = surface
            .documents
            .iter()
            .map(|doc| RestoreTarget {
                resource: doc.clone(),
                app: surface.resource_apps.get(doc).cloned(),
            })
            .collect();
        let outcome = restore::restore(&targets);
        let mut note = String::new();
        if launched > 0 {
            note.push_str(&format!(" {launched} apps opened."));
        }
        if !targets.is_empty() {
            note.push_str(&format!(" {}", outcome.summary()));
        }
        for failure in failed {
            note.push_str(&format!(" [{failure}]"));
        }
        note
    }

    /// Executes a room command from any surface (menu bar, hotkey).
    pub fn execute(&mut self, command: RoomCommand) {
        eprintln!("EVO-ROOMS: execute {command:?}");
        let note = match command {
            RoomCommand::Enter(index) => self.enter_at(index),
            RoomCommand::Leave => self.leave(),
            RoomCommand::Cycle => {
                let next = self.next_index();
                self.enter_at(next)
            }
            // Showing the window is the presence layer's job.
            RoomCommand::Open => None,
        };
        if let Some(note) = note {
            self.pending_note = Some(note);
        }
    }

    /// Syncs the surfaces and names after a reload (parallel arrays; the
    /// active room survives by name).
    pub fn set_surfaces(
        &mut self,
        surfaces: Vec<evo_execution::room::RoomSurface>,
        names: Vec<String>,
    ) {
        // A room that no longer exists is left honestly: containment
        // without its work is a trap, not a room. The desktop is restored
        // if a lease exists, and the room is gone either way.
        if let Some(active) = self.active.clone() {
            if !surfaces.iter().any(|surface| surface.work == active) {
                let _ = self.leave();
                self.active = None;
                self.web.hibernate();
            }
        }
        self.surfaces = surfaces;
        self.names = names;
    }
}

/// The shared handle: the app and the menu-bar presence both hold this.
pub type SharedRooms = Arc<Mutex<RoomRuntime>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaving_a_vanished_work_is_honest_when_surfaces_change() {
        let mut runtime = RoomRuntime::new();
        runtime.set_surfaces(
            vec![evo_execution::room::RoomSurface {
                work: "Gokyo".to_string(),
                urls: vec![],
                documents: vec![],
                titles: vec![],
                apps: vec![],
                resource_apps: Default::default(),
            }],
            vec!["Gokyo".to_string()],
        );
        // The active room is only ever set through enter (which needs a
        // surface); simulate it directly for the transition test.
        runtime.active = Some("Gokyo".to_string());
        runtime.set_surfaces(vec![], vec![]);
        assert_eq!(runtime.active, None, "a vanished work leaves its room");
    }
}
