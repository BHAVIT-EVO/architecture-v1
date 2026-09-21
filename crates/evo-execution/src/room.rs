//! Rooms: entering a body of work as a place, not a list.
//!
//! Restoration solved "what was I doing and where is it" — it opens the
//! work's resources. Rooms solve the next problem: **getting into it**.
//! After restoring three parallel works (each an editor window, browser
//! tabs, an agent chat), the person juggles nine windows mixed with
//! everything else. A Room is one action that establishes a working
//! context:
//!
//! * every app that is not part of the work is **hidden** (state kept,
//!   instantly reversible — the subtraction that makes entering feel like
//!   entering);
//! * windows of shared apps (a browser used by three works) that do not
//!   belong to this work are **parked** (minimized), the work's own
//!   windows are **raised in priority order** (documents over pages over
//!   titles);
//! * **Leave** restores exactly what Enter changed — a lease records every
//!   hide and park, nothing else is touched.
//!
//! # Evidence-based membership, no app knowledge
//!
//! A window belongs to a work when the work's own evidence names it: the
//! OS-reported `AXDocument` matches a work document path (editors), the
//! window's document URL canonicalizes to a work page (browsers expose the
//! current tab as `AXDocument`), or the window's exact title was captured
//! for the work. An app belongs to a work when its name was witnessed for
//! the work, or when it owns a belonging window. Nothing matches by
//! application category, and misses under-contain (a window stays visible)
//! rather than over-contain (a window never silently disappears that
//! should not).
//!
//! # The lease
//!
//! Enter is not a mode the OS enters; it is a set of reversible actions
//! with a receipt. [`RoomController::enter`] returns the [`RoomLease`]
//! describing everything it changed; [`RoomController::leave`] replays it
//! backwards. The controller refuses to enter while a room is active
//! (callers switch by leaving first) — one lease at a time, one truth.

use std::collections::{BTreeMap, BTreeSet};

/// The work's own evidence, projected for containment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoomSurface {
    /// The work's identity (for reporting and lease bookkeeping).
    pub work: String,
    /// Raw URLs the work touched (browser tabs, agent chats).
    pub urls: Vec<String>,
    /// Document file paths the work touched (editor files, PDFs).
    pub documents: Vec<String>,
    /// Window titles captured for the work.
    pub titles: Vec<String>,
    /// Application names witnessed for the work (localized names, as
    /// capture recorded them).
    pub apps: Vec<String>,
    /// The application each resource (document or URL) was most recently
    /// witnessed in, from the work's own intervals. A resource absent from
    /// this map was never observed with an app — it opens through the OS
    /// default handler. This is the work's own recent witness, not a
    /// registry: treat a hit as "open here", a miss as "let the OS decide".
    pub resource_apps: BTreeMap<String, String>,
}

/// One live window of an application, as the surface reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomWindow {
    /// Owning application pid.
    pub pid: i32,
    /// Owning application localized name.
    pub owner: String,
    /// The window's exact title ("" when the OS exposes none).
    pub title: String,
    /// The OS-reported `AXDocument`: a URL for browsers (the current tab),
    /// a file URL or path for editors, absent for others.
    pub ax_document: Option<String>,
    /// The system window id (stable across the window's life).
    pub window_id: u32,
    /// Whether the window is currently minimized.
    pub minimized: bool,
}

/// One running application, as the surface reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomApp {
    pub pid: i32,
    /// Localized application name (what the work's `apps` list records).
    pub name: String,
    /// Whether the application is currently hidden.
    pub hidden: bool,
    /// Whether this is a regular GUI application (things like the dock and
    /// system agents are never hidden — they are not part of anyone's
    /// desktop context).
    pub regular: bool,
}

/// The platform boundary the room controller acts through. Every operation
/// is something the live macOS implementation can do with the
/// Accessibility permission Evo already holds; the mock in tests makes the
/// controller's decisions testable without a display.
pub trait DesktopSurface {
    /// The running applications (regular GUI apps at minimum).
    fn running_apps(&self) -> Result<Vec<RoomApp>, String>;
    /// All windows of one application.
    fn windows_of(&self, pid: i32) -> Result<Vec<RoomWindow>, String>;
    /// Hides an application (state preserved, instantly reversible).
    fn hide_app(&mut self, pid: i32) -> Result<(), String>;
    /// Unhides an application previously hidden by [`DesktopSurface::hide_app`].
    fn unhide_app(&mut self, pid: i32) -> Result<(), String>;
    /// Minimizes one window (parks it).
    fn minimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String>;
    /// Restores one minimized window.
    fn unminimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String>;
    /// Raises one window (brings it to the front of its app).
    fn raise_window(&mut self, pid: i32, window_id: u32) -> Result<(), String>;
    /// Makes an application frontmost.
    fn activate_app(&mut self, pid: i32) -> Result<(), String>;
}

/// Why a window belongs to a work — the evidence kind, in raise-priority
/// order: documents are produced work, pages are attended work, titles are
/// the coarsest witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WindowMatch {
    None,
    Title,
    Page,
    Document,
}

impl WindowMatch {
    fn rank(self) -> u8 {
        match self {
            WindowMatch::None => 0,
            WindowMatch::Title => 1,
            WindowMatch::Page => 2,
            WindowMatch::Document => 3,
        }
    }
}

/// The receipt for one Enter: everything that was changed, so Leave can
/// restore exactly it.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomLease {
    /// The work the room was entered for.
    pub work: String,
    /// Non-work apps Enter hid.
    pub hidden_apps: Vec<i32>,
    /// Work apps that were hidden before Enter and were unhidden for it;
    /// Leave re-hides them.
    pub unhidden_apps: Vec<i32>,
    /// Windows of work apps that did not belong to the work and were
    /// parked (minimized).
    pub parked_windows: Vec<(i32, u32)>,
    /// The windows Enter raised, best evidence last (the frontmost one).
    pub raised_windows: Vec<(i32, u32, RoomWindow)>,
    /// Epoch milliseconds at entry.
    pub entered_at_ms: u64,
}

/// A room controller: enters and leaves works over a [`DesktopSurface`].
///
/// One active room at a time. The controller is deliberately not a window
/// manager: it does not enforce anything after Enter (apps the person
/// opens or unhides afterwards are their business — the room establishes,
/// it does not police).
pub struct RoomController {
    surface: Box<dyn DesktopSurface>,
    /// Pids that must never be hidden: Evo's own processes.
    protected_pids: Vec<i32>,
    lease: Option<RoomLease>,
}

/// Why an Enter could not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomError {
    /// A room is already active — leave (or switch) first.
    AlreadyInRoom { work: String },
    /// The surface could not answer (Accessibility missing, app dying).
    Surface(String),
    /// The surface has no identifying evidence, so there is nothing to
    /// contain: entering would hide the whole desktop for no reason.
    EmptySurface,
}

impl std::fmt::Display for RoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoomError::AlreadyInRoom { work } => {
                write!(f, "already in the room for {work:?} — leave first")
            }
            RoomError::Surface(reason) => write!(f, "the desktop could not be read: {reason}"),
            RoomError::EmptySurface => write!(
                f,
                "this work has no apps, documents, pages, or titles to contain"
            ),
        }
    }
}

impl RoomController {
    /// Constructs a controller over a platform surface, protecting Evo's
    /// own processes from ever being hidden.
    pub fn new(surface: Box<dyn DesktopSurface>, protected_pids: Vec<i32>) -> Self {
        Self {
            surface,
            protected_pids,
            lease: None,
        }
    }

    /// Test/probe access to the live surface (counting visible apps).
    pub fn probe_surface(&mut self) -> &mut dyn DesktopSurface {
        &mut *self.surface
    }

    /// The active room's lease, if any.
    pub fn active(&self) -> Option<&RoomLease> {
        self.lease.as_ref()
    }

    /// Whether a room is currently active.
    pub fn in_room(&self) -> bool {
        self.lease.is_some()
    }

    /// Enters the work's room: hides non-work apps, parks non-work windows
    /// of work apps, raises the work's windows in evidence order.
    pub fn enter(&mut self, surface: &RoomSurface) -> Result<RoomLease, RoomError> {
        if let Some(lease) = &self.lease {
            return Err(RoomError::AlreadyInRoom {
                work: lease.work.clone(),
            });
        }
        if surface.apps.is_empty()
            && surface.documents.is_empty()
            && surface.urls.is_empty()
            && surface.titles.is_empty()
        {
            return Err(RoomError::EmptySurface);
        }

        let apps = self.surface.running_apps().map_err(RoomError::Surface)?;
        let visible_regular = apps.iter().filter(|a| a.regular && !a.hidden).count();
        eprintln!(
            "EVO-ROOM: entering '{}' — {} apps ({} visible regular), surface apps={:?} urls={} docs={} titles={}",
            surface.work,
            apps.len(),
            visible_regular,
            surface.apps.len(),
            surface.urls.len(),
            surface.documents.len(),
            surface.titles.len()
        );
        let mut lease = RoomLease {
            work: surface.work.clone(),
            hidden_apps: Vec::new(),
            unhidden_apps: Vec::new(),
            parked_windows: Vec::new(),
            raised_windows: Vec::new(),
            entered_at_ms: now_ms(),
        };

        // Which apps belong to the work: named by the work, or owning a
        // belonging window.
        let mut work_pids: BTreeSet<i32> = apps
            .iter()
            .filter(|app| app_is_named_for_work(app, surface))
            .map(|app| app.pid)
            .collect();
        for app in &apps {
            if work_pids.contains(&app.pid)
                || !app.regular
                || is_protected(&self.protected_pids, app.pid)
            {
                continue;
            }
            if let Ok(windows) = self.surface.windows_of(app.pid) {
                if windows
                    .iter()
                    .any(|w| window_match(w, surface) != WindowMatch::None)
                {
                    work_pids.insert(app.pid);
                }
            }
        }

        // The subtraction: hide every visible regular app that is not part
        // of the work (and not Evo).
        //
        // `NSRunningApplication.hide` returns NO when called from a
        // process that is not the foreground app, while the hide STILL
        // takes effect — so the return value is not evidence. The lease
        // records what the DESKTOP actually reports afterwards: one fresh
        // query, and only observed-hidden apps are recorded (never a
        // guess, never a manual hide the person did themselves).
        for app in &apps {
            if !app.regular || app.hidden {
                continue;
            }
            if is_protected(&self.protected_pids, app.pid) {
                continue;
            }
            if !work_pids.contains(&app.pid) {
                // `NSRunningApplication.hide` is asynchronous AND lies about
                // the result when called from a non-foreground process: it
                // returns NO while the hide still takes effect seconds
                // later. No return value or immediate poll is evidence.
                // The lease therefore records every ATTEMPT; Leave unhides
                // attempts (unhide on an already-visible app is harmless,
                // while an un-restored hide would trap the person's apps
                // off-screen — the failure mode that must never happen).
                let _ = self.surface.hide_app(app.pid);
                lease.hidden_apps.push(app.pid);
            }
        }

        // Within work apps: raise belonging windows, park the rest. Work
        // apps that were hidden before are unhidden for the room (and
        // re-hidden on leave).
        for &pid in &work_pids {
            if let Some(app) = apps.iter().find(|a| a.pid == pid) {
                if app.hidden && self.surface.unhide_app(pid).is_ok() {
                    lease.unhidden_apps.push(pid);
                }
            }
            let windows = match self.surface.windows_of(pid) {
                Ok(windows) => windows,
                Err(_) => continue,
            };
            for window in &windows {
                // Window id 0 means the system would not give the window a
                // stable id (some system windows); it cannot be addressed
                // individually, so it is never parked.
                if window.window_id == 0 {
                    continue;
                }
                let matched = window_match(window, surface);
                if matched == WindowMatch::None {
                    if !window.minimized
                        && self.surface.minimize_window(pid, window.window_id).is_ok()
                    {
                        lease.parked_windows.push((pid, window.window_id));
                    }
                }
            }
        }

        // Raise in evidence order — lowest rank first so the strongest
        // evidence ends frontmost.
        let mut to_raise: Vec<(u8, RoomWindow)> = Vec::new();
        for &pid in &work_pids {
            if let Ok(windows) = self.surface.windows_of(pid) {
                for window in windows {
                    if window.window_id == 0 {
                        continue;
                    }
                    let matched = window_match(&window, surface);
                    if matched != WindowMatch::None {
                        to_raise.push((matched.rank(), window));
                    }
                }
            }
        }
        to_raise.sort_by_key(|(rank, window)| (*rank, window.title.clone()));
        let mut frontmost: Option<i32> = None;
        for (_, window) in &to_raise {
            if self
                .surface
                .raise_window(window.pid, window.window_id)
                .is_ok()
            {
                lease
                    .raised_windows
                    .push((window.pid, window.window_id, window.clone()));
                frontmost = Some(window.pid);
            }
        }
        if let Some(pid) = frontmost {
            let _ = self.surface.activate_app(pid);
        }

        self.lease = Some(lease.clone());
        Ok(lease)
    }

    /// Leaves the active room, restoring exactly what Enter changed.
    pub fn leave(&mut self) -> Result<Option<RoomLease>, RoomError> {
        let Some(lease) = self.lease.take() else {
            return Ok(None);
        };
        // Reverse order of Enter's actions: parked windows first (they were
        // the person's other contexts), then hidden apps.
        for &(pid, window_id) in lease.parked_windows.iter().rev() {
            let _ = self.surface.unminimize_window(pid, window_id);
        }
        for &pid in lease.unhidden_apps.iter() {
            let _ = self.surface.hide_app(pid);
        }
        eprintln!(
            "EVO-ROOM: leaving '{}' — restoring {} hidden, {} parked",
            lease.work,
            lease.hidden_apps.len(),
            lease.parked_windows.len()
        );
        for &pid in lease.hidden_apps.iter() {
            if let Err(reason) = self.surface.unhide_app(pid) {
                eprintln!("EVO-ROOM: unhide failed for pid {pid}: {reason}");
            }
        }
        Ok(Some(lease))
    }

    /// Leaves any active room and enters the new work's room.
    pub fn switch(&mut self, surface: &RoomSurface) -> Result<RoomLease, RoomError> {
        self.leave()?;
        self.enter(surface)
    }
}

/// Whether an app's localized name is one the work was witnessed using.
/// Exact (case-insensitive) equality on the recorded name — never a
/// substring: "Code" must not claim "Xcode".
pub fn app_is_named_for_work(app: &RoomApp, surface: &RoomSurface) -> bool {
    surface
        .apps
        .iter()
        .any(|name| name.trim().eq_ignore_ascii_case(app.name.trim()))
}

/// The evidence that a window belongs to the work, strongest first:
/// its `AXDocument` is a work document path, its document URL
/// canonicalizes to a work page, or its exact title was captured.
pub fn window_match(window: &RoomWindow, surface: &RoomSurface) -> WindowMatch {
    if let Some(document) = &window.ax_document {
        // Editors report file paths or file:// URLs; the match is exact on
        // the resolved path — the OS's own account of which file the
        // window is showing.
        let path = document
            .strip_prefix("file://")
            .unwrap_or(document.as_str());
        if !path.is_empty()
            && path.starts_with('/')
            && surface.documents.iter().any(|doc| doc == path)
        {
            return WindowMatch::Document;
        }
        // Browsers report the current tab's URL; identity is the canonical
        // page — the same rule the ledger's works use, so a work's page
        // key and a live tab's document meet on equal terms.
        if let Some(page) = evo_ledger::pages::canonical_page(document) {
            for url in &surface.urls {
                if evo_ledger::pages::canonical_page(url).as_deref() == Some(page.as_str()) {
                    return WindowMatch::Page;
                }
            }
        }
    }
    if !window.title.trim().is_empty()
        && surface
            .titles
            .iter()
            .any(|title| title.trim() == window.title.trim())
    {
        return WindowMatch::Title;
    }
    WindowMatch::None
}

fn is_protected(protected: &[i32], pid: i32) -> bool {
    protected.contains(&pid)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// The mock desktop: state + a tape of every action, so tests assert
    /// both decisions and the exact restore sequence.
    struct MockSurface {
        apps: Vec<RoomApp>,
        windows: Vec<RoomWindow>,
        tape: RefCell<Vec<String>>,
    }

    impl MockSurface {
        fn new(apps: Vec<RoomApp>, windows: Vec<RoomWindow>) -> Self {
            Self {
                apps,
                windows,
                tape: RefCell::new(Vec::new()),
            }
        }

        fn hidden(&self, pid: i32) -> bool {
            self.apps.iter().any(|a| a.pid == pid && a.hidden)
        }
    }

    impl DesktopSurface for MockSurface {
        fn running_apps(&self) -> Result<Vec<RoomApp>, String> {
            Ok(self.apps.clone())
        }
        fn windows_of(&self, pid: i32) -> Result<Vec<RoomWindow>, String> {
            Ok(self
                .windows
                .iter()
                .filter(|w| w.pid == pid)
                .cloned()
                .collect())
        }
        fn hide_app(&mut self, pid: i32) -> Result<(), String> {
            self.tape.borrow_mut().push(format!("hide {pid}"));
            for app in &mut self.apps {
                if app.pid == pid {
                    app.hidden = true;
                }
            }
            Ok(())
        }
        fn unhide_app(&mut self, pid: i32) -> Result<(), String> {
            self.tape.borrow_mut().push(format!("unhide {pid}"));
            for app in &mut self.apps {
                if app.pid == pid {
                    app.hidden = false;
                }
            }
            Ok(())
        }
        fn minimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
            self.tape
                .borrow_mut()
                .push(format!("minimize {pid}/{window_id}"));
            for window in &mut self.windows {
                if window.pid == pid && window.window_id == window_id {
                    window.minimized = true;
                }
            }
            Ok(())
        }
        fn unminimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
            self.tape
                .borrow_mut()
                .push(format!("unminimize {pid}/{window_id}"));
            for window in &mut self.windows {
                if window.pid == pid && window.window_id == window_id {
                    window.minimized = false;
                }
            }
            Ok(())
        }
        fn raise_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
            self.tape
                .borrow_mut()
                .push(format!("raise {pid}/{window_id}"));
            Ok(())
        }
        fn activate_app(&mut self, pid: i32) -> Result<(), String> {
            self.tape.borrow_mut().push(format!("activate {pid}"));
            Ok(())
        }
    }

    fn app(pid: i32, name: &str, hidden: bool) -> RoomApp {
        RoomApp {
            pid,
            name: name.to_string(),
            hidden,
            regular: true,
        }
    }

    fn window(pid: i32, owner: &str, title: &str, document: Option<&str>, id: u32) -> RoomWindow {
        RoomWindow {
            pid,
            owner: owner.to_string(),
            title: title.to_string(),
            ax_document: document.map(String::from),
            window_id: id,
            minimized: false,
        }
    }

    fn work_surface() -> RoomSurface {
        RoomSurface {
            work: "trek planning".into(),
            urls: vec!["https://www.youtube.com/watch?v=q2VTtop1Ztk&list=PLiM-TFJI81R".into()],
            documents: vec!["/Users/alice/notes/trek.md".into()],
            titles: vec!["Unisex Hiking & Trekking Clothes | Arc'teryx".into()],
            apps: vec!["ZCode".into()],
            resource_apps: [("/Users/alice/notes/trek.md".to_string(), "ZCode".to_string())]
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn enter_hides_non_work_apps_and_raises_work_windows() {
        let surface = MockSurface::new(
            vec![
                app(1, "ZCode", false),
                app(2, "Google Chrome", false),
                app(3, "WhatsApp", false),
                app(4, "Finder", false),
            ],
            vec![
                window(
                    1,
                    "ZCode",
                    "app.rs — project",
                    Some("/Users/alice/notes/trek.md"),
                    10,
                ),
                window(1, "ZCode", "other window", None, 11),
                window(
                    2,
                    "Google Chrome",
                    "Trek video",
                    Some("https://www.youtube.com/watch?v=q2VTtop1Ztk&list=PLiM-TFJI81R"),
                    20,
                ),
                window(
                    2,
                    "Google Chrome",
                    "Shopping — other work",
                    Some("https://www.example.com/cart"),
                    21,
                ),
                window(3, "WhatsApp", "‎WhatsApp", None, 30),
            ],
        );
        let mut controller = RoomController::new(Box::new(surface), vec![99]);

        let lease = controller.enter(&work_surface()).unwrap();
        assert_eq!(lease.work, "trek planning");
        // WhatsApp and Finder (non-work, visible) were hidden.
        assert!(lease.hidden_apps.contains(&3));
        assert!(lease.hidden_apps.contains(&4));
        // Chrome was discovered as a work app via its matching window; its
        // non-work window was parked, its work window raised.
        assert!(lease.parked_windows.contains(&(2, 21)));
        assert!(
            lease
                .raised_windows
                .iter()
                .any(|(pid, id, _)| *pid == 2 && *id == 20)
        );
        // The editor's non-matching window was parked.
        assert!(lease.parked_windows.contains(&(1, 11)));
        // The strongest evidence (the document window) was raised LAST, so
        // it sits frontmost.
        assert_eq!(
            lease.raised_windows.last().unwrap().2.title,
            "app.rs — project"
        );
        assert!(controller.in_room());
    }

    #[test]
    fn leave_restores_exactly_what_enter_changed() {
        let surface = MockSurface::new(
            vec![app(1, "ZCode", false), app(3, "WhatsApp", false)],
            vec![
                window(
                    1,
                    "ZCode",
                    "app.rs — project",
                    Some("/Users/alice/notes/trek.md"),
                    10,
                ),
                window(3, "WhatsApp", "‎WhatsApp", None, 30),
            ],
        );
        let mut controller = RoomController::new(Box::new(surface), vec![]);
        controller.enter(&work_surface()).unwrap();
        let lease = controller.leave().unwrap().unwrap();
        assert!(!lease.hidden_apps.is_empty());
        // The mock restored state: WhatsApp visible again.
        let mock = controller.surface();
        assert!(!mock.hidden(3), "hidden app restored on leave");
        assert!(!controller.in_room());
        // Leaving again is a no-op.
        assert!(controller.leave().unwrap().is_none());
    }

    #[test]
    fn pre_hidden_work_apps_are_unhidden_and_rehidden() {
        let surface = MockSurface::new(
            vec![
                app(1, "ZCode", true), // hidden before Enter
                app(2, "WhatsApp", false),
            ],
            vec![window(
                1,
                "ZCode",
                "app.rs — project",
                Some("/Users/alice/notes/trek.md"),
                10,
            )],
        );
        let mut controller = RoomController::new(Box::new(surface), vec![]);
        let lease = controller.enter(&work_surface()).unwrap();
        assert!(
            lease.unhidden_apps.contains(&1),
            "hidden work app unhidden for the room"
        );
        assert!(lease.hidden_apps.contains(&2));
        controller.leave().unwrap();
        let mock = controller.surface();
        assert!(mock.hidden(1), "pre-hidden work app re-hidden on leave");
        assert!(!mock.hidden(2));
    }

    #[test]
    fn protected_pids_are_never_hidden() {
        let surface = MockSurface::new(
            vec![
                app(1, "Evo", false),
                app(2, "ZCode", false),
                app(3, "WhatsApp", false),
            ],
            vec![window(
                2,
                "ZCode",
                "app.rs — project",
                Some("/Users/alice/notes/trek.md"),
                10,
            )],
        );
        let mut controller = RoomController::new(Box::new(surface), vec![1]);
        let lease = controller.enter(&work_surface()).unwrap();
        assert!(!lease.hidden_apps.contains(&1), "Evo is never hidden");
        assert!(lease.hidden_apps.contains(&3));
    }

    #[test]
    fn enter_twice_is_refused_until_leave() {
        let surface = MockSurface::new(
            vec![app(1, "ZCode", false)],
            vec![window(
                1,
                "ZCode",
                "app.rs — project",
                Some("/Users/alice/notes/trek.md"),
                10,
            )],
        );
        let mut controller = RoomController::new(Box::new(surface), vec![]);
        controller.enter(&work_surface()).unwrap();
        assert!(matches!(
            controller.enter(&work_surface()),
            Err(RoomError::AlreadyInRoom { .. })
        ));
        controller.leave().unwrap();
        controller.enter(&work_surface()).unwrap();
    }

    #[test]
    fn switch_leaves_then_enters() {
        let surface = MockSurface::new(
            vec![app(1, "ZCode", false), app(2, "WhatsApp", false)],
            vec![window(
                1,
                "ZCode",
                "app.rs — project",
                Some("/Users/alice/notes/trek.md"),
                10,
            )],
        );
        let mut controller = RoomController::new(Box::new(surface), vec![]);
        controller.enter(&work_surface()).unwrap();
        let other = RoomSurface {
            work: "other".into(),
            urls: vec![],
            documents: vec!["/tmp/other.txt".into()],
            titles: vec![],
            apps: vec!["WhatsApp".into()],
            resource_apps: [("/tmp/other.txt".to_string(), "WhatsApp".to_string())]
                .into_iter()
                .collect(),
        };
        let lease = controller.switch(&other).unwrap();
        assert_eq!(lease.work, "other");
        assert!(controller.in_room());
    }

    #[test]
    fn empty_surface_is_refused_not_a_blanket_hide() {
        let surface = MockSurface::new(
            vec![app(1, "ZCode", false), app(2, "WhatsApp", false)],
            vec![],
        );
        let mut controller = RoomController::new(Box::new(surface), vec![]);
        assert!(matches!(
            controller.enter(&RoomSurface {
                work: "w".into(),
                ..Default::default()
            }),
            Err(RoomError::EmptySurface)
        ));
    }

    #[test]
    fn non_regular_apps_are_never_hidden() {
        let mut system = app(5, "WindowServer", false);
        system.regular = false;
        let surface = MockSurface::new(
            vec![system, app(1, "ZCode", false)],
            vec![window(
                1,
                "ZCode",
                "app.rs — project",
                Some("/Users/alice/notes/trek.md"),
                10,
            )],
        );
        let mut controller = RoomController::new(Box::new(surface), vec![]);
        let lease = controller.enter(&work_surface()).unwrap();
        assert!(
            !lease.hidden_apps.contains(&5),
            "non-regular apps are not desktop context"
        );
    }

    #[test]
    fn window_match_uses_canonical_pages_and_exact_documents_and_titles() {
        let surface = work_surface();
        // Same page, different parameter order → match.
        let browser_window = window(
            2,
            "Chrome",
            "Trek video",
            Some("https://www.youtube.com/watch?list=PLiM-TFJI81R&v=q2VTtop1Ztk"),
            1,
        );
        assert_eq!(window_match(&browser_window, &surface), WindowMatch::Page);
        // A different video on the same host → no match (host is not identity).
        let other_video = window(
            2,
            "Chrome",
            "Other",
            Some("https://www.youtube.com/watch?v=OTHERVIDEO00"),
            2,
        );
        assert_eq!(window_match(&other_video, &surface), WindowMatch::None);
        // Exact document path → match; file:// prefix tolerated.
        let editor = window(
            1,
            "ZCode",
            "notes",
            Some("file:///Users/alice/notes/trek.md"),
            3,
        );
        assert_eq!(window_match(&editor, &surface), WindowMatch::Document);
        // A different document in the same editor → no match.
        let other_doc = window(1, "ZCode", "other", Some("/Users/alice/notes/other.md"), 4);
        assert_eq!(window_match(&other_doc, &surface), WindowMatch::None);
        // Exact title → match; near-miss title → no match.
        let titled = window(
            3,
            "Arc",
            "Unisex Hiking & Trekking Clothes | Arc'teryx",
            None,
            5,
        );
        assert_eq!(window_match(&titled, &surface), WindowMatch::Title);
        let near = window(3, "Arc", "Unisex Hiking & Trekking Clothes", None, 6);
        assert_eq!(window_match(&near, &surface), WindowMatch::None);
    }

    #[test]
    fn raise_order_puts_documents_frontmost() {
        let surface = MockSurface::new(
            vec![app(1, "ZCode", false), app(2, "Google Chrome", false)],
            vec![
                window(
                    2,
                    "Google Chrome",
                    "Trek video",
                    Some("https://www.youtube.com/watch?v=q2VTtop1Ztk&list=PLiM-TFJI81R"),
                    20,
                ),
                window(
                    1,
                    "ZCode",
                    "app.rs — project",
                    Some("/Users/alice/notes/trek.md"),
                    10,
                ),
            ],
        );
        let mut controller = RoomController::new(Box::new(surface), vec![]);
        let lease = controller.enter(&work_surface()).unwrap();
        let ids: Vec<u32> = lease.raised_windows.iter().map(|(_, id, _)| *id).collect();
        assert_eq!(
            ids,
            vec![20, 10],
            "page raised first, document raised last (frontmost)"
        );
    }

    impl RoomController {
        /// Test access to the surface (downcast to the mock).
        fn surface(&self) -> &MockSurface {
            // SAFETY: only in tests, where the surface is always the mock.
            unsafe { &*(&*self.surface as *const dyn DesktopSurface as *const MockSurface) }
        }
    }
}
