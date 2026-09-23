//! The pod runtime host: desktop-level state of Pods, shared between the
//! window UI and the menu-bar presence.
//!
//! Mirrors IS-0022's room host shape (commands execute synchronously from
//! wherever they come — the desktop never waits on a window frame), but
//! the body is now [`evo_pods`]: stage recipes, Dim-by-default
//! containment, honest leases, gesture declarations, isolated instances.
//!
//! The browser panel (weighed-down web panes) stays window state with the
//! app — but **hibernation is a pod transition**: leaving or switching
//! releases the pod's web presence no matter who asked.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use evo_execution::restore::{self, RestoreTarget};

pub use evo_pods::config::ContainMode;
pub use evo_pods::pod::{Pod, PodId, PodState};
pub use evo_pods::runtime::{Note, PodError, PodRuntime};

/// Commands the runtime executes (menu bar, hotkeys, the bar itself).
/// `Open` is about the window, not the pod: the host ignores it and the
/// presence layer shows the window. `ToggleBar` the app handles
/// (bar state paints with frames).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PodCommand {
    /// Enter/switch to pod at index.
    Enter(usize),
    /// Leave the active pod.
    Leave,
    /// Cycle to the next pod (IS-0022's ⌘⇧E).
    Cycle,
    /// Show the window (presence-owned — AppKit, no frames needed).
    Open,
    /// Toggle the Pod Bar overlay (⌥Space; app handles it at frame time).
    ToggleBar,
}

/// The shared pod state handoff.
pub struct PodHost {
    /// The pods core: choreography + containment + lease + stuff that
    /// must never wait on a painted frame.
    pub core: PodRuntime,
    /// Engine-projected pods, parallel to Home's work order.
    pub pods: Vec<Pod>,
    /// A note from the last transition, drained by the app's next frame
    /// (the menu-bar path has no UI of its own).
    pub pending_note: Option<String>,
    /// Set when entering a pod with web pages: the app hydrates its
    /// browser pane for them (drained by the next frame).
    pub wants_browser: bool,
    /// The active pod's web panes. Hydration is the app's (it needs the
    /// window); hibernation is a pod transition.
    pub web: crate::websurface::WebSurfaces,
    /// The active pod's display name (menu bar + strip), kept parallel to
    /// the core's lease.
    pub active: Option<String>,
}

impl PodHost {
    pub fn new() -> Self {
        let core = PodRuntime::new(
            Box::new(crate::pod_surface::MacPodSurface::new()),
            // Evo itself must never be veiled/parked/hidden/hosted by own
            // containment.
            vec![std::process::id() as i32],
            evo_pods::config::PodConfig::default(),
        );
        Self {
            core,
            pods: Vec::new(),
            pending_note: None,
            wants_browser: false,
            web: crate::websurface::WebSurfaces::new(),
            active: None,
        }
    }

    /// Display names parallel to the pod list (menu bar).
    pub fn names(&self) -> Vec<String> {
        self.pods.iter().map(|p| p.name.clone()).collect()
    }

    /// Enters the pod at `index` (switching if needed): its stage restores
    /// the work's place first (so missing surfaces reopen), then the
    /// choreography lands, the veil drops, the note speaks. Returns the
    /// honest note.
    pub fn enter_at(&mut self, index: usize, now_ms: u64) -> Option<String> {
        let Some(pod) = self.pods.get(index).cloned() else {
            return None;
        };
        if self
            .core
            .active()
            .map(|active| active.name == pod.name)
            .unwrap_or(false)
        {
            return Some(format!("Already in \"{}\".", pod.name));
        }
        // Leaving implicitly: hibernate the previous pod's web pane so the
        // machine pays for one web presence at a time.
        let restore_note = self.restore_work(&pod);
        self.web.hibernate();
        self.core.set_pods(self.pods.clone());
        match self.core.switch(index, now_ms) {
            Ok(core_note) => {
                // Entering never opens the Pages pane: the web presence is
                // on demand (the Pages button), not a hijack on entry.
                self.active = Some(pod.name.clone());
                Some(format!("{core_note}{restore_note}"))
            }
            // Restoration already ran on the failed path: say so honestly
            // and let the lease (if any) decide what comes back.
            Err(err) => Some(format!("Pod not entered: {err}{restore_note}")),
        }
    }

    /// Leaves the active pod, veils lifted and lease reversed exactly.
    pub fn leave(&mut self) -> Option<String> {
        self.wants_browser = false;
        self.web.hibernate();
        self.active = None;
        match self.core.leave() {
            Ok(note) => Some(note),
            Err(err) => Some(format!("Could not leave the pod: {err}")),
        }
    }

    /// The next pod for the cycle habit (wraps).
    pub fn next_index(&self) -> usize {
        if self.pods.is_empty() {
            return 0;
        }
        self.core
            .active()
            .and_then(|active| self.pods.iter().position(|p| p.id == active.id))
            .map(|i| (i + 1) % self.pods.len())
            .unwrap_or(0)
    }

    /// Executes a command from any surface (menu bar, hotkey). The
    /// window/bar commands are drained by the app's frames, not here.
    pub fn execute(&mut self, command: PodCommand, now_ms: u64) {
        eprintln!("EVO-PODS: execute {command:?}");
        let note = match command {
            PodCommand::Enter(index) => self.enter_at(index, now_ms),
            PodCommand::Leave => self.leave(),
            PodCommand::Cycle => {
                let next = self.next_index();
                self.enter_at(next, now_ms)
            }
            PodCommand::Open | PodCommand::ToggleBar => None,
        };
        if let Some(note) = note {
            self.pending_note = Some(note);
        }
    }

    /// Syncs the engine-derived pods after a reload. The pod that
    /// vanished from Home leaves honestly: containment without its work
    /// is a trap.
    pub fn set_pods(&mut self, pods: Vec<Pod>) {
        if let Some(active) = self.core.active().cloned() {
            if !pods.iter().any(|p| p.id == active.id) {
                let _ = self.leave();
                self.active = None;
            }
        }
        self.core.set_pods(pods.clone());
        self.pods = pods;
    }

    /// Restores the work's place, generically: witnessed apps are brought
    /// forward (plain `open -a` by name — never a second instance), then
    /// documents through their witnessed app or the OS handler. No
    /// application is named in code; URLs hydrate the pod's own browser
    /// pane (per-work stores), not a system browser.
    fn restore_work(&self, pod: &Pod) -> String {
        // Only real apps launch: an "app" that appears among the work's
        // window titles is a window, not an application (capture with
        // Accessibility off records titles in the app slot upstream;
        // re-fusing the two here would recreate the very mis-launch that
        // polluted the desktop).
        let title_set: BTreeSet<&str> = pod.surfaces.titles.iter().map(|t| t.trim()).collect();
        let mut seen = BTreeSet::new();
        let mut launched = 0usize;
        let mut failed: Vec<String> = Vec::new();
        let mut skipped_windows = 0usize;
        for app in &pod.surfaces.apps {
            let app = app.trim();
            if app.is_empty() {
                continue;
            }
            if title_set.contains(app) {
                skipped_windows += 1;
                continue;
            }
            if !seen.insert(app.to_lowercase()) {
                continue;
            }
            match restore::launch_app(app) {
                Ok(()) => launched += 1,
                Err(err) => failed.push(format!("{app}: {err}")),
            }
        }
        // Documents that are really window titles are not openable; they
        // are skipped with the same honesty count, never "opened".
        let mut doc_targets: Vec<RestoreTarget> = Vec::new();
        for doc in &pod.surfaces.documents {
            let doc = doc.trim();
            if doc.is_empty() {
                continue;
            }
            if title_set.contains(doc) {
                skipped_windows += 1;
                continue;
            }
            doc_targets.push(RestoreTarget {
                resource: doc.to_string(),
                app: pod
                    .surfaces
                    .resource_apps
                    .get(doc)
                    .map(|app| app.trim().to_string())
                    .filter(|app| !app.is_empty() && !title_set.contains(app.as_str())),
            });
        }
        let outcome = restore::restore(&doc_targets);
        let mut note = String::new();
        if launched > 0 {
            note.push_str(&format!(" {launched} apps opened."));
        }
        if !doc_targets.is_empty() {
            note.push_str(&format!(" {}", outcome.summary()));
        }
        if skipped_windows > 0 {
            note.push_str(&format!(
                " {skipped_windows} window-shaped identities skipped (windows are not apps)."
            ));
        }
        let reported = failed.len().min(2);
        for failure in failed.iter().take(reported) {
            note.push_str(&format!(" [{failure}]"));
        }
        if failed.len() > reported {
            note.push_str(&format!(" [+{} more could not open]", failed.len() - reported));
        }
        note
    }
}

/// The shared handle: the app and the menu-bar presence both hold this.
pub type SharedPods = Arc<Mutex<PodHost>>;

/// Epoch milliseconds now (injected at call sites that already compute it).
pub fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
