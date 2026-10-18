//! The pod runtime host: desktop-level state of Pods, shared between the
//! window UI and the menu-bar presence.
//!
//! Mirrors IS-0022's room host shape (commands execute synchronously from
//! wherever they come — the desktop never waits on a window frame), but
//! the body is now [`evo_pods`]: stage recipes, Dim-by-default
//! containment, honest leases, gesture declarations, isolated instances.
//!
//! URL and application restoration stay at the OS boundary: entering a pod
//! dispatches witnessed resources through the normal macOS opener. Evo never
//! embeds a browser or creates a second per-work web session.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use evo_execution::restore::{
    DispatchResult, RestoreReceipt, RestoreTarget, TargetReceipt, VerificationResult,
};
use evo_pods::pod::PodResource;

use crate::pod_surface::{PodRestorer, SystemPodRestorer};

pub use evo_pods::config::ContainMode;
pub use evo_pods::pod::{Pod, PodId, PodState};
pub use evo_pods::runtime::{Note, PodError, PodRuntime};

/// Commands the runtime executes (menu bar, hotkeys, the bar itself).
/// `Open` is about the window, not the pod: the host ignores it and the
/// presence layer shows the window. `ToggleBar` the app handles
/// (bar state paints with frames).
/// `Eq` is deliberately not derived: `ClaimWindow` carries a live window
/// snapshot (f64 frames), and identity was never the point of a command.
#[derive(Debug, Clone, PartialEq)]
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
    /// Add-to-Pod: claim a live, still-open window's evidence into the pod.
    ClaimWindow(usize, evo_pods::surface::PodWindow),
    /// Add-to-Pod: save an app/file/folder/URL so the pod can re-open it.
    AddResource(usize, evo_pods::pod::PodResource),
    /// Add-to-Pod: open one saved resource now.
    OpenResource(usize, usize),
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
    /// The active pod's display name (menu bar + strip), kept parallel to
    /// the core's lease.
    pub active: Option<String>,
    /// Where Add-to-Pod claims/resources persist (the addon ledger file
    /// under the storage root). None in tests = in-memory only.
    addons_path: Option<std::path::PathBuf>,
    restorer: Box<dyn PodRestorer>,
}

impl Default for PodHost {
    fn default() -> Self {
        Self::new()
    }
}

impl PodHost {
    pub fn new() -> Self {
        let core = PodRuntime::new(
            Box::new(crate::pod_surface::MacPodSurface::new()),
            // Evo itself must never be veiled/parked/hidden/hosted by own
            // containment.
            vec![std::process::id() as i32],
            // Native workspaces arrange only the restored work. They never
            // veil, hide, or minimize unrelated apps on the desktop.
            evo_pods::config::PodConfig {
                contain: ContainMode::Open,
                ..evo_pods::config::PodConfig::default()
            },
        );
        Self {
            core,
            pods: Vec::new(),
            pending_note: None,
            active: None,
            addons_path: None,
            restorer: Box::new(SystemPodRestorer),
        }
    }

    /// Point at the addon ledger and ingest whatever it remembers, so
    /// Add-to-Pod claims survive restarts and engine refreshes alike.
    pub fn set_addons_path(&mut self, path: std::path::PathBuf) {
        if let Ok(text) = std::fs::read_to_string(&path) {
            self.core.import_addons(&text);
            self.core.set_pods(self.pods.clone());
            self.pods = self.core.pods().to_vec();
        }
        self.addons_path = Some(path);
    }

    /// Write the addon ledger. Best-effort by doctrine: the in-memory
    /// truth already governs the session; the file must not be able to
    /// make a claim destructive.
    fn save_addons(&self) {
        if let Some(path) = &self.addons_path {
            let _ = std::fs::write(path, self.core.export_addons());
        }
    }

    /// Re-fold core truth (claims now show in the bar's evidence) into the
    /// host-facing projection the menu bar and cards read.
    fn resync_from_core(&mut self) {
        self.pods = self.core.pods().to_vec();
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
        let pod = self.pods.get(index).cloned()?;
        if self
            .core
            .active()
            .map(|active| active.id == pod.id)
            .unwrap_or(false)
        {
            return Some(format!("Already in \"{}\".", pod.name));
        }
        let mut report = self.restore_work(&pod);
        // Saved resources are independent attempts; a refused document never
        // cancels an Add-to-Pod URL (or the other way around).
        report.append(self.open_resources(index));
        let restore_note = report.note();
        if report.all_failed() {
            // Keep the previous stage, lease and web presence intact when
            // nothing could be brought forward. Failed execution is not a
            // gesture declaration and must not learn/mutate a stage recipe.
            return Some(format!(
                "Pod not entered: restoration failed.{restore_note}"
            ));
        }
        self.core.set_pods(self.pods.clone());
        match self.core.switch(index, now_ms) {
            Ok(core_note) => {
                // Entering never opens an Evo-owned web pane; URL resources
                // were already dispatched through the normal OS opener.
                self.active = Some(pod.name.clone());
                self.resync_from_core();
                Some(format!("{core_note}{restore_note}"))
            }
            // Restoration already ran on the failed path: say so honestly
            // and let the lease (if any) decide what comes back.
            Err(err) => {
                self.active = self.core.active().map(|pod| pod.name.clone());
                self.resync_from_core();
                Some(format!("Pod not entered: {err}{restore_note}"))
            }
        }
    }

    /// Leaves the active pod, veils lifted and lease reversed exactly.
    pub fn leave(&mut self) -> Option<String> {
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
            PodCommand::ClaimWindow(index, window) => {
                let note = self
                    .core
                    .claim_window(index, &window)
                    .unwrap_or_else(|err| format!("Could not add the window: {err}"));
                self.resync_from_core();
                self.save_addons();
                Some(note)
            }
            PodCommand::AddResource(index, resource) => {
                let note = self
                    .core
                    .add_resource(index, resource)
                    .unwrap_or_else(|err| format!("Could not save the resource: {err}"));
                self.resync_from_core();
                self.save_addons();
                Some(note)
            }
            PodCommand::OpenResource(index, resource_index) => {
                match self.open_resource(index, resource_index) {
                    Ok(note) => Some(note),
                    Err(err) => Some(format!("Could not open it: {err}")),
                }
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
        if let Some(active) = self.core.active().cloned()
            && !pods.iter().any(|p| p.id == active.id)
        {
            let _ = self.leave();
            self.active = None;
        }
        self.core.set_pods(pods.clone());
        self.resync_from_core();
    }

    /// Live window inventory for the Add-to-Pod picker. Read-only.
    pub fn open_windows(&mut self) -> Vec<evo_pods::surface::PodWindow> {
        self.core.open_windows()
    }

    /// Re-open everything the pod was taught to own ("Add to Pod"
    /// resources), one add at a time, failures spoken never staged.
    /// Runs before choreography: staged windows land on what's there.
    fn open_resources(&self, index: usize) -> PodRestoreReport {
        let mut report = PodRestoreReport::default();
        if let Some(pod) = self.pods.get(index) {
            for resource in &pod.resources {
                report.extend(self.restorer.open_resource(resource));
            }
        }
        report
    }

    fn open_resource(&self, index: usize, resource_index: usize) -> Result<String, PodError> {
        let pod = self.pods.get(index).ok_or(PodError::NoSuchPod)?;
        let resource = pod
            .resources
            .get(resource_index)
            .ok_or(PodError::NoSuchPod)?;
        let mut report = PodRestoreReport::default();
        report.extend(self.restorer.open_resource(resource));
        Ok(format!(
            "{} for \"{}\":{}",
            resource.value(),
            pod.name,
            report.note()
        ))
    }

    /// Restores the work's place, generically: witnessed apps are brought
    /// forward (plain `open -a` by name — never a second instance), then
    /// documents and URLs through their witnessed app or the OS handler. No
    /// application is named in code and URLs never hydrate an Evo-owned
    /// browser pane.
    fn restore_work(&self, pod: &Pod) -> PodRestoreReport {
        // Only real apps launch: an "app" that appears among the work's
        // window titles is a window, not an application (capture with
        // Accessibility off records titles in the app slot upstream;
        // re-fusing the two here would recreate the very mis-launch that
        // polluted the desktop).
        let title_set: BTreeSet<&str> = pod.surfaces.titles.iter().map(|t| t.trim()).collect();
        let mut seen = BTreeSet::new();
        let mut report = PodRestoreReport::default();
        for app in &pod.surfaces.apps {
            let app = app.trim();
            if app.is_empty() {
                continue;
            }
            if title_set.contains(app) {
                report.skipped_windows += 1;
                continue;
            }
            if !seen.insert(app.to_lowercase()) {
                continue;
            }
            report.extend(
                self.restorer
                    .open_resource(&PodResource::App(app.to_string())),
            );
        }
        // Documents that are really window titles are not openable; they
        // are skipped with the same honesty count, never "opened".
        let mut doc_targets: Vec<RestoreTarget> = Vec::new();
        let mut seen_resources = BTreeSet::new();
        let mut push_target = |resource: &str| {
            let resource = resource.trim();
            if resource.is_empty() || !seen_resources.insert(resource.to_string()) {
                return;
            }
            if title_set.contains(resource) {
                report.skipped_windows += 1;
                return;
            }
            doc_targets.push(RestoreTarget {
                resource: resource.to_string(),
                surface: None,
                app: pod
                    .surfaces
                    .resource_apps
                    .get(resource)
                    .map(|app| app.trim().to_string())
                    .filter(|app| !app.is_empty() && !title_set.contains(app.as_str())),
            });
        };

        for doc in &pod.surfaces.documents {
            push_target(doc);
        }
        // The bundle is the restore contract. Feed every distinct resource
        // through the same safe restore seam; classification still decides
        // whether it opens or raises a live window, and no browser/profile
        // identity is invented here.
        for resource in &pod.bundle.restore_set {
            push_target(resource);
        }
        drop(push_target);
        // Neither PodSurfaces nor PodWindow/PodResource carries witnessed
        // SurfaceIdentity. App names, resource strings and CG window IDs must
        // never be converted into invented profile/tab/native identity.
        report.extend(self.restorer.restore_targets(&doc_targets));
        report
    }
}

#[derive(Default)]
struct PodRestoreReport {
    targets: Vec<TargetReceipt>,
    skipped_windows: usize,
}

impl PodRestoreReport {
    fn extend(&mut self, receipt: RestoreReceipt) {
        // Legacy opened/raised buckets describe accepted dispatch, not the
        // postcondition. All UI claims here come from the per-target receipts.
        self.targets.extend(receipt.targets);
    }

    fn append(&mut self, other: Self) {
        self.targets.extend(other.targets);
        self.skipped_windows += other.skipped_windows;
    }

    fn all_failed(&self) -> bool {
        !self.targets.is_empty()
            && !self.targets.iter().any(|target| {
                matches!(
                    target.dispatch,
                    DispatchResult::OpenAccepted | DispatchResult::RaiseAccepted
                ) && !matches!(target.verification, VerificationResult::Failed { .. })
            })
    }

    fn note(&self) -> String {
        let mut opens = 0;
        let mut raises = 0;
        let mut verified = 0;
        let mut unverified = 0;
        let mut verification_failed = 0;
        let mut refused = 0;
        let mut dispatch_failed = 0;
        let mut reasons = Vec::new();
        for target in &self.targets {
            match &target.dispatch {
                DispatchResult::OpenAccepted => opens += 1,
                DispatchResult::RaiseAccepted => raises += 1,
                DispatchResult::NotDispatched { reason } => {
                    refused += 1;
                    reasons.push(format!("{}: {reason}", target.resource));
                    continue;
                }
                DispatchResult::Failed { reason } => {
                    dispatch_failed += 1;
                    reasons.push(format!("{}: {reason}", target.resource));
                    continue;
                }
            }
            match &target.verification {
                VerificationResult::Verified => verified += 1,
                VerificationResult::Unverified { reason } => {
                    unverified += 1;
                    reasons.push(format!("{}: unverified: {reason}", target.resource));
                }
                VerificationResult::Failed { reason } => {
                    verification_failed += 1;
                    reasons.push(format!(
                        "{}: verification failed: {reason}",
                        target.resource
                    ));
                }
            }
        }
        let mut note = String::new();
        if !self.targets.is_empty() {
            note.push_str(&format!(
                " Requests accepted: {opens} open, {raises} raise; verified: {verified}."
            ));
            if unverified > 0 {
                note.push_str(&format!(" Verification unavailable for {unverified} accepted requests; exact window/tab/profile restoration is not confirmed."));
            }
            if verification_failed > 0 {
                note.push_str(&format!(" Verification failed: {verification_failed}."));
            }
            if refused > 0 || dispatch_failed > 0 {
                note.push_str(&format!(
                    " Not dispatched: {refused}; dispatch failed: {dispatch_failed}."
                ));
            }
        }
        if self.skipped_windows > 0 {
            note.push_str(&format!(
                " {} window-shaped identities skipped (windows are not apps).",
                self.skipped_windows
            ));
        }
        for reason in reasons.iter().take(2) {
            note.push_str(&format!(" [{reason}]"));
        }
        if reasons.len() > 2 {
            note.push_str(&format!(" [+{} more results]", reasons.len() - 2));
        }
        note
    }
}

/// The shared handle: the app and the menu-bar presence both hold this.
pub type SharedPods = Rc<RefCell<PodHost>>;

/// Epoch milliseconds now (injected at call sites that already compute it).
pub fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_execution::restore::RestoreOutcome;
    use evo_pods::config::PodConfig;
    use evo_pods::pod::{PodClaim, PodSurfaces};
    use evo_pods::surface::{CommandSpec, Frame, PodApp, PodSurface, PodWindow};
    use evo_threads::{ResumeBundle, Status, ThreadView};
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct FakeRestorer {
        targets: RefCell<Vec<RestoreTarget>>,
        resources: RefCell<Vec<PodResource>>,
        results: BTreeMap<String, (DispatchResult, VerificationResult)>,
    }

    impl FakeRestorer {
        fn receipt(&self, resource: &str) -> RestoreReceipt {
            let (dispatch, verification) = self.results.get(resource).cloned().unwrap_or((
                DispatchResult::OpenAccepted,
                VerificationResult::Unverified {
                    reason: "test postcondition unavailable".into(),
                },
            ));
            // Deliberately empty legacy buckets: host notes and failure gating
            // must use the per-target dispatch/verification, never these counts.
            RestoreReceipt {
                outcome: RestoreOutcome::default(),
                targets: vec![TargetReceipt {
                    resource: resource.into(),
                    dispatch,
                    verification,
                }],
            }
        }
    }

    impl PodRestorer for Rc<FakeRestorer> {
        fn restore_targets(&self, targets: &[RestoreTarget]) -> RestoreReceipt {
            self.targets.borrow_mut().extend_from_slice(targets);
            let mut receipt = RestoreReceipt::default();
            for target in targets {
                receipt
                    .targets
                    .extend(self.receipt(&target.resource).targets);
            }
            receipt
        }

        fn open_resource(&self, resource: &PodResource) -> RestoreReceipt {
            self.resources.borrow_mut().push(resource.clone());
            self.receipt(resource.value())
        }
    }

    struct FakeSurface {
        windows: Vec<PodWindow>,
        log: Vec<String>,
    }

    impl PodSurface for FakeSurface {
        fn running_apps(&self) -> Result<Vec<PodApp>, String> {
            Ok(vec![
                PodApp {
                    pid: 77,
                    name: "Editor".into(),
                    hidden: false,
                    regular: true,
                },
                PodApp {
                    pid: 88,
                    name: "Other".into(),
                    hidden: false,
                    regular: true,
                },
            ])
        }
        fn windows_of(&self, pid: i32) -> Result<Vec<PodWindow>, String> {
            Ok(self
                .windows
                .iter()
                .filter(|w| w.pid == pid)
                .cloned()
                .collect())
        }
        fn screens(&self) -> Vec<Frame> {
            vec![Frame::new(0.0, 0.0, 1000.0, 800.0)]
        }
        fn hide_app(&mut self, pid: i32) -> Result<(), String> {
            self.log.push(format!("hide:{pid}"));
            Ok(())
        }
        fn unhide_app(&mut self, pid: i32) -> Result<(), String> {
            self.log.push(format!("unhide:{pid}"));
            Ok(())
        }
        fn quit_app(&mut self, pid: i32) -> Result<(), String> {
            self.log.push(format!("quit:{pid}"));
            Ok(())
        }
        fn minimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
            self.log.push(format!("park:{pid}:{window_id}"));
            Ok(())
        }
        fn unminimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
            self.log.push(format!("unpark:{pid}:{window_id}"));
            Ok(())
        }
        fn raise_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
            self.log.push(format!("raise:{pid}:{window_id}"));
            Ok(())
        }
        fn move_window(&mut self, pid: i32, window_id: u32, _frame: Frame) -> Result<(), String> {
            self.log.push(format!("move:{pid}:{window_id}"));
            Ok(())
        }
        fn activate_app(&mut self, pid: i32) -> Result<(), String> {
            self.log.push(format!("activate:{pid}"));
            Ok(())
        }
        fn veil_open(&mut self, _frame: Frame, _alpha: f32) -> Result<u64, String> {
            self.log.push("veil:open".into());
            Ok(1)
        }
        fn veil_update(&mut self, handle: u64, _frame: Frame) -> Result<(), String> {
            self.log.push(format!("veil:update:{handle}"));
            Ok(())
        }
        fn veil_close(&mut self, handle: u64) -> Result<(), String> {
            self.log.push(format!("veil:close:{handle}"));
            Ok(())
        }
        fn run(&mut self, _cmd: CommandSpec) -> Result<(), String> {
            panic!("resource opens must use restoration receipts, not detached surface runs")
        }
        fn debug_log(&self) -> Vec<String> {
            self.log.clone()
        }
    }

    fn fake_host(restorer: Rc<FakeRestorer>) -> PodHost {
        let mut foreign = window("Other window", None);
        foreign.pid = 88;
        foreign.owner = "Other".into();
        foreign.window_id = 10;
        PodHost {
            core: PodRuntime::new(
                Box::new(FakeSurface {
                    windows: vec![window("Notes", Some("/tmp/notes.md")), foreign],
                    log: vec![],
                }),
                vec![],
                PodConfig {
                    contain: ContainMode::Dim,
                    ..Default::default()
                },
            ),
            pods: vec![],
            pending_note: None,
            active: None,
            addons_path: None,
            restorer: Box::new(restorer),
        }
    }

    fn pod_fixture() -> Pod {
        let view = ThreadView {
            id: 41,
            status: Status::Active,
            episodes: vec![],
            anchors: vec![],
            companions: vec![],
            contested: vec![],
            fork: None,
        };
        Pod::from_views(
            &view,
            &ResumeBundle {
                resume_point: String::new(),
                resume_reason: String::new(),
                restore_set: vec![],
                open_deltas: vec![],
            },
            "Receipt pod",
            "test",
            0.0,
            vec![],
            PodSurfaces {
                documents: vec!["/tmp/notes.md".into()],
                titles: vec!["Notes".into()],
                apps: vec!["Editor".into()],
                ..Default::default()
            },
        )
    }

    fn window(title: &str, document: Option<&str>) -> PodWindow {
        PodWindow {
            pid: 77,
            owner: "Editor".into(),
            title: title.into(),
            ax_document: document.map(str::to_owned),
            window_id: 9,
            minimized: false,
            frame: evo_pods::surface::Frame::new(0.0, 0.0, 400.0, 300.0),
        }
    }

    #[test]
    fn add_to_pod_keeps_witnessed_claim_and_saved_resource_after_projection() {
        let restorer = Rc::new(FakeRestorer::default());
        let mut host = fake_host(restorer.clone());
        host.set_pods(vec![pod_fixture()]);

        host.execute(
            PodCommand::ClaimWindow(0, window("Notes", Some("/tmp/claimed.md"))),
            0,
        );
        host.execute(
            PodCommand::AddResource(0, PodResource::Url("https://example.test".into())),
            0,
        );

        // Refresh with the engine's original projection, not the enriched one.
        host.set_pods(vec![pod_fixture()]);

        let pod = &host.pods[0];
        assert!(
            pod.claims
                .contains(&PodClaim::Document("/tmp/claimed.md".into()))
        );
        assert!(pod.surfaces.documents.contains(&"/tmp/claimed.md".into()));
        assert!(
            pod.resources
                .contains(&PodResource::Url("https://example.test".into()))
        );
        // The projection remains owned by the core, so a later refresh cannot
        // drop the user-owned Add-to-Pod evidence.
        assert_eq!(host.core.pods(), host.pods.as_slice());
        assert!(restorer.targets.borrow().is_empty());
        assert!(restorer.resources.borrow().is_empty());
        assert!(host.core.surface_log().is_empty());
    }

    #[test]
    fn bundle_restore_resources_use_normal_targets_not_embedded_pages() {
        let restorer = Rc::new(FakeRestorer::default());
        let mut host = fake_host(restorer.clone());
        let mut pod = pod_fixture();
        pod.bundle.restore_set = vec![
            "https://example.test/work".into(),
            "https://example.test/work".into(),
            "/tmp/notes.md".into(),
        ];
        host.set_pods(vec![pod]);

        host.enter_at(0, 123).expect("pod exists");

        let targets = restorer.targets.borrow();
        let urls: Vec<&RestoreTarget> = targets
            .iter()
            .filter(|target| target.resource == "https://example.test/work")
            .collect();
        assert_eq!(urls.len(), 1, "duplicate bundle URLs dispatch once");
        assert!(urls[0].app.is_none());
        assert!(urls[0].surface.is_none());
        assert!(
            targets
                .iter()
                .any(|target| target.resource == "/tmp/notes.md")
        );
    }

    #[test]
    fn enter_uses_receipts_before_containment_and_preserves_add_to_pod() {
        let restorer = Rc::new(FakeRestorer::default());
        let mut host = fake_host(restorer.clone());
        host.set_pods(vec![pod_fixture()]);
        host.execute(
            PodCommand::AddResource(0, PodResource::Url("https://example.test".into())),
            0,
        );

        let note = host.enter_at(0, 123).expect("pod exists");

        assert!(note.contains("Requests accepted: 3 open"));
        assert!(note.contains("Verification unavailable"));
        assert_eq!(restorer.resources.borrow().len(), 2);
        assert_eq!(restorer.targets.borrow().len(), 1);
        assert_eq!(restorer.targets.borrow()[0].resource, "/tmp/notes.md");
        assert!(host.core.is_active());
        assert!(
            host.core
                .surface_log()
                .iter()
                .any(|line| line == "veil:open")
        );
        assert!(
            host.pods[0]
                .resources
                .contains(&PodResource::Url("https://example.test".into()))
        );
    }

    #[test]
    fn unsupported_restore_receipts_leave_containment_and_lease_untouched() {
        let mut results = BTreeMap::new();
        results.insert(
            "Editor".into(),
            (
                DispatchResult::Failed {
                    reason: "unsupported test opener".into(),
                },
                VerificationResult::Unverified {
                    reason: "no action was accepted".into(),
                },
            ),
        );
        results.insert(
            "/tmp/notes.md".into(),
            (
                DispatchResult::Failed {
                    reason: "unsupported test opener".into(),
                },
                VerificationResult::Unverified {
                    reason: "no action was accepted".into(),
                },
            ),
        );
        let restorer = Rc::new(FakeRestorer {
            results,
            ..Default::default()
        });
        let mut host = fake_host(restorer.clone());
        host.set_pods(vec![pod_fixture()]);

        let note = host.enter_at(0, 123).expect("pod exists");

        assert!(note.contains("restoration failed"));
        assert!(note.contains("dispatch failed: 2"));
        assert!(!host.core.is_active());
        assert!(host.core.surface_log().is_empty());
    }

    #[test]
    fn report_distinguishes_accepted_dispatch_from_unverified_postcondition() {
        let report = PodRestoreReport {
            targets: vec![TargetReceipt {
                resource: "https://example.test".into(),
                dispatch: DispatchResult::OpenAccepted,
                verification: VerificationResult::Unverified {
                    reason: "browser adapter unavailable".into(),
                },
            }],
            skipped_windows: 0,
        };

        assert!(!report.all_failed());
        let note = report.note();
        assert!(note.contains("Requests accepted: 1 open"));
        assert!(note.contains("Verification unavailable for 1"));
        assert!(note.contains("exact window/tab/profile restoration is not confirmed"));
    }

    #[test]
    fn failed_dispatch_does_not_count_as_pod_entry_success() {
        let report = PodRestoreReport {
            targets: vec![TargetReceipt {
                resource: "Editor".into(),
                dispatch: DispatchResult::Failed {
                    reason: "pods require macOS".into(),
                },
                verification: VerificationResult::Unverified {
                    reason: "no action was accepted".into(),
                },
            }],
            skipped_windows: 0,
        };

        assert!(report.all_failed());
        assert!(report.note().contains("dispatch failed: 1"));
    }
}
