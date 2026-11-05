//! Offline identity-path fixture, not a live macOS/browser pass.
//!
//! Exercises the real capture registry, normalizer, acceptance, daemon durable
//! persistence/read-back, RESOURCE request planner, shared surface conversion,
//! and exact browser execution/verification algorithm. Only witnesses and
//! retained browser handles are synthetic. No listener, native adapter,
//! environment configuration, model, browser, permissions, or network is used.
//!
//! Run: cargo test --offline -p evo-daemon --lib browser_identity_fixture
//! Witness data, time and assertions are fixed; acceptance's opaque UUIDs and
//! exclusive scratch-directory names are intentionally not asserted literals.

use super::handle_voice_request;
use crate::persistence::{load_persisted_observations, persist_observation};
use evo_capture::{
    BrowserIdentityProvider as CaptureBrowserProvider, BrowserIdentityRegistry,
    BrowserIdentityWitness, CaptureEngine, MacOSAdapter, MacOSSignal,
};
use evo_execution::browser_macos::{
    BrowserIdentity, BrowserIdentityProvider, BrowserMacOSCapability, BrowserRestoreReceipt,
    restore_browser_identity, witness_focused_browser,
};
use evo_execution::restore::{
    DispatchResult, RestoreAction, RestoreOutcome, RestoreTarget, VerificationResult, plan,
};
use evo_observation::evidence::FactValue;
use evo_observation::observation::Observation;
use evo_observation::observed_state::ObservedState;
use evo_observation::provenance::ObservationSource;
use evo_observation::surface_identity::{BrowserIdentityObservation, SurfaceIdentity};
use evo_storage::{Storage, StorageObjectKind};
use evo_voice::{RestoredSubjects, VoiceOutcome};
use std::cell::{Cell, RefCell};
use std::io::Cursor;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

const URL: &str = "https://browser-fixture.test/work?exact=1#section";
const PID: i32 = 717;
const NATIVE_WINDOW: &str = "fixture-ax-window-7";
static SCRATCH_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

fn identity() -> BrowserIdentity {
    BrowserIdentity {
        process_id: PID,
        process_started_at: format!("{:016x}", 1_791_000_000.5f64.to_bits()),
        native_window_identifier: "fixture-browser-window:41".into(),
        profile_token: "fixture-profile-opaque".into(),
        session_token: "fixture-context-opaque".into(),
        tab_token: "fixture-tab-opaque".into(),
        url: URL.into(),
    }
}

fn native_surface() -> SurfaceIdentity {
    SurfaceIdentity {
        capture_session: "synthetic-browser-capture".into(),
        window_id: "window-7".into(),
        process_id: PID,
        process_started_at: identity().process_started_at,
        window_identifier: Some(NATIVE_WINDOW.into()),
        browser_window: None,
        browser_profile: None,
        browser_session: None,
        browser_tab: None,
    }
}

/// Slot/generation is the retained handle; URL and identity tokens never
/// construct one. All actions are recorded, and this fake has no opener.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RetainedTab {
    slot: usize,
    generation: u64,
}

struct SyntheticBrowser {
    capability: Cell<BrowserMacOSCapability>,
    live: RefCell<Vec<BrowserIdentity>>,
    process_reads: Cell<usize>,
    evidence_reads: Cell<usize>,
    enumeration_reads: Cell<usize>,
    focus_reads: Cell<usize>,
    restart_on_process_read: Cell<Option<usize>>,
    changed_before_dispatch: RefCell<Option<BrowserIdentity>>,
    focus_error: RefCell<Option<String>>,
    postcondition: RefCell<Option<Result<BrowserIdentity, String>>>,
    dispatch_attempts: RefCell<Vec<RetainedTab>>,
}

impl Default for SyntheticBrowser {
    fn default() -> Self {
        Self {
            capability: Cell::new(BrowserMacOSCapability::ExactIdentity),
            live: RefCell::new(vec![identity()]),
            process_reads: Cell::new(0),
            evidence_reads: Cell::new(0),
            enumeration_reads: Cell::new(0),
            focus_reads: Cell::new(0),
            restart_on_process_read: Cell::new(None),
            changed_before_dispatch: RefCell::new(None),
            focus_error: RefCell::new(None),
            postcondition: RefCell::new(None),
            dispatch_attempts: RefCell::new(Vec::new()),
        }
    }
}

impl SyntheticBrowser {
    fn reset_reads(&self) {
        self.process_reads.set(0);
        self.evidence_reads.set(0);
        self.enumeration_reads.set(0);
        self.focus_reads.set(0);
    }
}

impl BrowserIdentityProvider for SyntheticBrowser {
    type Tab = RetainedTab;

    fn capability(&self) -> BrowserMacOSCapability {
        self.capability.get()
    }

    fn process_started_at(&self, pid: i32) -> Result<String, String> {
        assert_eq!(pid, PID);
        let read = self.process_reads.get() + 1;
        self.process_reads.set(read);
        if self
            .restart_on_process_read
            .get()
            .is_some_and(|restart| read >= restart)
        {
            Ok(format!("{:016x}", 1_791_000_001.5f64.to_bits()))
        } else {
            Ok(identity().process_started_at)
        }
    }

    fn tabs(&self, pid: i32) -> Result<Vec<Self::Tab>, String> {
        assert_eq!(pid, PID);
        self.enumeration_reads.set(self.enumeration_reads.get() + 1);
        Ok((0..self.live.borrow().len())
            .map(|slot| RetainedTab {
                slot,
                generation: 1,
            })
            .collect())
    }

    fn evidence(&self, tab: &Self::Tab) -> Result<BrowserIdentity, String> {
        assert_eq!(tab.generation, 1, "never bind a replacement generation");
        let read = self.evidence_reads.get() + 1;
        self.evidence_reads.set(read);
        if !self.dispatch_attempts.borrow().is_empty() {
            if let Some(result) = &*self.postcondition.borrow() {
                return result.clone();
            }
        }
        if read == self.live.borrow().len() + 1 {
            if let Some(changed) = &*self.changed_before_dispatch.borrow() {
                return Ok(changed.clone());
            }
        }
        self.live
            .borrow()
            .get(tab.slot)
            .cloned()
            .ok_or_else(|| "stale retained tab handle".into())
    }

    fn focus_tab(&self, tab: &Self::Tab) -> Result<(), String> {
        self.dispatch_attempts.borrow_mut().push(tab.clone());
        if let Some(reason) = &*self.focus_error.borrow() {
            return Err(reason.clone());
        }
        Ok(())
    }

    fn globally_focused_tab(&self, pid: i32) -> Result<Self::Tab, String> {
        assert_eq!(pid, PID);
        self.focus_reads.set(self.focus_reads.get() + 1);
        if !self.dispatch_attempts.borrow().is_empty() {
            if let Some(Err(reason)) = &*self.postcondition.borrow() {
                return Err(reason.clone());
            }
        }
        Ok(RetainedTab {
            slot: self
                .dispatch_attempts
                .borrow()
                .last()
                .map_or(0, |tab| tab.slot),
            generation: 1,
        })
    }
}

/// Synthetic equivalent of the configured runtime bridge. Browser fields are
/// read through the real witness algorithm; the AX token comes from the native
/// sample and is never relabelled as a browser-native window token.
struct CaptureBridge(Rc<SyntheticBrowser>);

impl CaptureBrowserProvider for CaptureBridge {
    fn witness_focused_surface(&self, native: &SurfaceIdentity) -> BrowserIdentityWitness {
        let observed = witness_focused_browser(native.process_id, self.0.as_ref()).unwrap();
        BrowserIdentityWitness::Observed(BrowserIdentityObservation {
            process_id: Some(observed.process_id),
            process_started_at: Some(observed.process_started_at),
            window_identifier: native.window_identifier.clone(),
            browser_window: Some(observed.native_window_identifier),
            browser_profile: Some(observed.profile_token),
            browser_session: Some(observed.session_token),
            browser_tab: Some(observed.tab_token),
        })
    }
}

struct Fixture {
    root: PathBuf,
    browser: Rc<SyntheticBrowser>,
    surface: SurfaceIdentity,
    accepted: Vec<Observation>,
    work_id: String,
}

impl Fixture {
    fn new() -> Self {
        // create_dir exclusively claims only a new child. It never clears or
        // writes any existing scratch root, even after an interrupted test.
        let parent = std::env::temp_dir().join("opencode");
        std::fs::create_dir_all(&parent).unwrap();
        let root = loop {
            let sequence = SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let candidate = parent.join(format!(
                "evo-synthetic-browser-{}-{sequence}",
                std::process::id()
            ));
            match std::fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create isolated fixture root: {error}"),
            }
        };
        let browser = Rc::new(SyntheticBrowser::default());
        let mut registry = BrowserIdentityRegistry::new(CaptureBridge(Rc::clone(&browser)));
        let surface = registry.enrich(native_surface());
        assert_eq!(surface.window_identifier.as_deref(), Some(NATIVE_WINDOW));
        assert_eq!(
            surface.browser_window.as_deref(),
            Some("fixture-browser-window:41")
        );
        assert_ne!(surface.window_identifier, surface.browser_window);
        assert_eq!(
            BrowserIdentity::from_surface(&surface, URL).unwrap(),
            identity()
        );
        let at = UNIX_EPOCH + Duration::from_secs(1_791_000_000);
        let state = ObservedState::new()
            .with_document_locator(URL)
            .with_window_title("Synthetic browser work")
            .with_surface_identity(surface.clone());
        let signals = [
            MacOSSignal::WindowFocusGained {
                subject: URL.into(),
                observed_at: at,
                process_identifier: Some(PID),
                owning_process_name: Some("Synthetic Browser".into()),
                observed_state: Some(state),
                window_id: Some("window-7".parse().unwrap()),
            },
            MacOSSignal::InputActivity {
                subject: Some(URL.into()),
                keys: 150,
                clicks: 0,
                scrolls: 0,
                observed_at: at + Duration::from_millis(10),
            },
            MacOSSignal::SurfaceNamed {
                subject: URL.into(),
                name: "Synthetic browser work".into(),
                observed_at: at + Duration::from_millis(20),
            },
        ];
        let adapter =
            MacOSAdapter::new(ObservationSource::new("synthetic_browser_fixture").unwrap());
        let mut engine = CaptureEngine::new();
        let mut accepted = Vec::new();
        {
            // Thread-local scope protects both writes and reads without setting
            // process environment or consulting the user's canonical history.
            let _root = Storage::with_thread_root(root.clone());
            assert!(
                Storage::new()
                    .read_all(StorageObjectKind::Observation)
                    .unwrap()
                    .is_empty()
            );
            for signal in signals {
                let event = adapter.normalize(signal).unwrap().unwrap();
                let schema = event.schema();
                let observation = engine.ingest(event, &schema).unwrap();
                persist_observation(&observation).unwrap();
                accepted.push(observation);
            }
        }
        let loaded = load_persisted_observations(&root).unwrap();
        assert_eq!(
            loaded, accepted,
            "durable read-back preserves every accepted byte-level field"
        );
        assert_eq!(loaded[0].evidence().facts().len(), 1);
        assert_eq!(
            loaded[0]
                .evidence()
                .fact("WindowFocusGained")
                .unwrap()
                .value(),
            &FactValue::Text(URL.into())
        );
        assert_eq!(
            ObservedState::from_context(loaded[0].provenance().context())
                .unwrap()
                .surface_identity(),
            Some(&surface)
        );
        let facts = super::voice_facts(&root).unwrap();
        assert_eq!(facts.briefs.len(), 1);
        assert_eq!(facts.briefs[0].restore_set, [URL]);
        let work_id = facts.briefs[0].thread_id.to_string();
        browser.reset_reads();
        Self {
            root,
            browser,
            surface,
            accepted,
            work_id,
        }
    }

    /// Real request-time daemon planning calls our injected browser executor.
    /// The callback is not SystemPlatform: no concrete DevTools/native calls
    /// are covered here. Its outcome counts only a verified identity receipt.
    fn request(
        &self,
    ) -> (
        VoiceOutcome,
        BrowserRestoreReceipt,
        Arc<Mutex<RestoredSubjects>>,
    ) {
        let before = std::fs::read(self.root.join("observation.log")).unwrap();
        let restored = Arc::new(Mutex::new(RestoredSubjects::new()));
        let mut receipt = None;
        let mut calls = 0;
        let outcome = handle_voice_request(
            &format!("RESOURCE {} {}", self.work_id, URL.len()),
            &mut Cursor::new(URL.as_bytes()),
            &self.root,
            &restored,
            None,
            |targets| {
                calls += 1;
                assert_eq!(
                    targets,
                    &[RestoreTarget {
                        resource: URL.into(),
                        app: None,
                        surface: Some(self.surface.clone()),
                    }]
                );
                assert!(
                    matches!(
                        plan(targets, &[]).as_slice(),
                        [RestoreAction::Unopenable { .. }]
                    ),
                    "the legacy URL-opening plan must not erase exact identity"
                );
                assert!(
                    restored.try_lock().unwrap().is_execution(URL),
                    "daemon reserves execution provenance before dispatch without holding its lock"
                );
                let expected = BrowserIdentity::from_surface(
                    targets[0].surface.as_ref().unwrap(),
                    &targets[0].resource,
                )
                .unwrap();
                assert_eq!(
                    expected,
                    identity(),
                    "production conversion retains all browser constraints"
                );
                let result = restore_browser_identity(&expected, self.browser.as_ref());
                let outcome = if result.dispatch == DispatchResult::RaiseAccepted
                    && result.verification == VerificationResult::Verified
                {
                    RestoreOutcome {
                        raised: vec![URL.into()],
                        ..Default::default()
                    }
                } else {
                    RestoreOutcome {
                        unopened: vec![(
                            URL.into(),
                            format!("{:?}; {:?}", result.dispatch, result.verification),
                        )],
                        ..Default::default()
                    }
                };
                receipt = Some(result);
                outcome
            },
        );
        assert_eq!(calls, 1, "never retry or restore an unbound URL");
        assert_eq!(
            std::fs::read(self.root.join("observation.log")).unwrap(),
            before,
            "live handles, dispatch and postcondition evidence never enter canonical history"
        );
        assert_eq!(
            load_persisted_observations(&self.root).unwrap(),
            self.accepted
        );
        assert_eq!(
            std::fs::read_dir(&self.root).unwrap().count(),
            1,
            "only the isolated canonical observation log is created"
        );
        (outcome, receipt.unwrap(), restored)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Only this exclusively created fixture child is removed.
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn assert_refused(fixture: &Fixture, reason: &str) {
    let (outcome, receipt, restored) = fixture.request();
    assert_eq!(receipt.resource, URL);
    assert!(
        matches!(&receipt.dispatch, DispatchResult::NotDispatched { reason: actual } if actual.contains(reason)),
        "{receipt:?}"
    );
    assert!(matches!(
        receipt.verification,
        VerificationResult::Unverified { .. }
    ));
    assert_eq!(receipt.postcondition, None);
    assert!(
        fixture.browser.dispatch_attempts.borrow().is_empty(),
        "no tab focus or URL fallback"
    );
    assert!(
        matches!(outcome, VoiceOutcome::Failed { .. }),
        "{outcome:?}"
    );
    assert!(
        !restored.lock().unwrap().is_execution(URL),
        "refusal releases the daemon reservation"
    );
}

#[test]
fn synthetic_capture_persist_daemon_target_dispatch_and_postcondition() {
    let fixture = Fixture::new();
    let (outcome, receipt, restored) = fixture.request();
    assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
    assert_eq!(receipt.verification, VerificationResult::Verified);
    assert_eq!(receipt.postcondition, Some(identity()));
    assert_eq!(
        *fixture.browser.dispatch_attempts.borrow(),
        [RetainedTab {
            slot: 0,
            generation: 1
        }]
    );
    assert_eq!(fixture.browser.process_reads.get(), 4);
    assert_eq!(fixture.browser.evidence_reads.get(), 3);
    assert_eq!(fixture.browser.focus_reads.get(), 1);
    assert!(
        matches!(outcome, VoiceOutcome::Restored { .. }),
        "{outcome:?}"
    );
    assert!(restored.lock().unwrap().is_execution(URL));
}

#[test]
fn same_url_in_another_profile_session_or_tab_is_not_a_fallback() {
    for field in 0..3 {
        let fixture = Fixture::new();
        let mut wrong = identity();
        match field {
            0 => wrong.profile_token = "replacement-profile".into(),
            1 => wrong.session_token = "replacement-session".into(),
            _ => wrong.tab_token = "replacement-tab".into(),
        }
        *fixture.browser.live.borrow_mut() = vec![wrong];
        assert_refused(&fixture, "no live tab");
        assert_eq!(fixture.browser.enumeration_reads.get(), 1);
    }
}

#[test]
fn stale_process_refuses_before_focus_even_with_exact_url_and_tokens() {
    for read in [1, 2] {
        let fixture = Fixture::new();
        fixture.browser.restart_on_process_read.set(Some(read));
        assert_refused(&fixture, "lifetime changed");
    }
}

#[test]
fn retained_tab_mutation_refuses_without_searching_for_a_same_url_replacement() {
    let fixture = Fixture::new();
    let mut replacement = identity();
    replacement.tab_token = "replacement-at-same-url".into();
    *fixture.browser.live.borrow_mut() = vec![identity(), replacement.clone()];
    *fixture.browser.changed_before_dispatch.borrow_mut() = Some(replacement);
    assert_refused(&fixture, "changed before dispatch");
    assert_eq!(fixture.browser.enumeration_reads.get(), 1);
    assert_eq!(fixture.browser.evidence_reads.get(), 3);
}

#[test]
fn exact_tokens_with_changed_window_or_url_refuse_without_navigation() {
    for field in 0..2 {
        let fixture = Fixture::new();
        let mut changed = identity();
        if field == 0 {
            changed.native_window_identifier = NATIVE_WINDOW.into();
        } else {
            changed.url = "https://browser-fixture.test/work?exact=1#different".into();
        }
        *fixture.browser.live.borrow_mut() = vec![changed];
        assert_refused(&fixture, "changed before dispatch");
    }
}

#[test]
fn duplicate_identity_refuses_instead_of_using_window_or_url_as_tiebreaker() {
    let fixture = Fixture::new();
    let mut duplicate = identity();
    duplicate.url = "https://browser-fixture.test/other".into();
    duplicate.native_window_identifier = "other-window".into();
    *fixture.browser.live.borrow_mut() = vec![identity(), duplicate];
    assert_refused(&fixture, "ambiguous");
}

#[test]
fn unavailable_browser_provider_refuses_without_inspection_or_dispatch() {
    let fixture = Fixture::new();
    fixture
        .browser
        .capability
        .set(BrowserMacOSCapability::Unsupported);
    assert_refused(&fixture, "provider unavailable");
    assert_eq!(fixture.browser.process_reads.get(), 0);
    assert_eq!(fixture.browser.enumeration_reads.get(), 0);
    assert_eq!(fixture.browser.evidence_reads.get(), 0);
}

#[test]
fn stale_retained_handle_at_dispatch_is_failed_without_retry_or_url_fallback() {
    let fixture = Fixture::new();
    *fixture.browser.focus_error.borrow_mut() = Some("stale retained tab handle".into());
    let (outcome, receipt, restored) = fixture.request();
    assert!(matches!(receipt.dispatch, DispatchResult::Failed { .. }));
    assert!(matches!(
        receipt.verification,
        VerificationResult::Unverified { .. }
    ));
    assert_eq!(receipt.postcondition, None);
    assert_eq!(fixture.browser.dispatch_attempts.borrow().len(), 1);
    assert_eq!(fixture.browser.focus_reads.get(), 0);
    assert!(matches!(outcome, VoiceOutcome::Failed { .. }));
    assert!(!restored.lock().unwrap().is_execution(URL));
}

#[test]
fn accepted_dispatch_with_any_wrong_postcondition_is_not_restoration() {
    for field in 0..7 {
        let fixture = Fixture::new();
        let mut wrong = identity();
        match field {
            0 => wrong.process_id += 1,
            1 => wrong.process_started_at = format!("{:016x}", 1_791_000_001.5f64.to_bits()),
            2 => wrong.native_window_identifier = NATIVE_WINDOW.into(),
            3 => wrong.profile_token = "wrong-profile".into(),
            4 => wrong.session_token = "wrong-session".into(),
            5 => wrong.tab_token = "wrong-tab".into(),
            _ => wrong.url = "https://browser-fixture.test/wrong".into(),
        }
        *fixture.browser.postcondition.borrow_mut() = Some(Ok(wrong));
        let (outcome, receipt, restored) = fixture.request();
        assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
        assert!(matches!(
            receipt.verification,
            VerificationResult::Failed { .. }
        ));
        assert_eq!(receipt.postcondition, None);
        assert_eq!(
            fixture.browser.dispatch_attempts.borrow().len(),
            1,
            "no retry"
        );
        assert!(
            matches!(outcome, VoiceOutcome::Failed { .. }),
            "accepted is not verified"
        );
        assert!(!restored.lock().unwrap().is_execution(URL));
    }
}

#[test]
fn unreadable_global_focus_or_post_dispatch_restart_never_claims_success() {
    for restart in [None, Some(3), Some(4)] {
        let fixture = Fixture::new();
        fixture.browser.restart_on_process_read.set(restart);
        if restart.is_none() {
            *fixture.browser.postcondition.borrow_mut() =
                Some(Err("browser is not the globally focused application".into()));
        }
        let (outcome, receipt, restored) = fixture.request();
        assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
        assert!(matches!(
            receipt.verification,
            VerificationResult::Failed { .. }
        ));
        assert_eq!(receipt.postcondition, None);
        assert_eq!(fixture.browser.dispatch_attempts.borrow().len(), 1);
        assert!(matches!(outcome, VoiceOutcome::Failed { .. }));
        assert!(!restored.lock().unwrap().is_execution(URL));
    }
}

#[test]
fn surface_conversion_never_fills_missing_browser_constraints_from_native_or_url() {
    let fixture = Fixture::new();
    for field in 0..4 {
        let mut incomplete = fixture.surface.clone();
        match field {
            0 => incomplete.browser_window = None,
            1 => incomplete.browser_profile = None,
            2 => incomplete.browser_session = None,
            _ => incomplete.browser_tab = None,
        }
        assert!(BrowserIdentity::from_surface(&incomplete, URL).is_err());
    }
    let mut corrupt = fixture.surface.clone();
    corrupt.window_id = "window-0".into();
    assert!(BrowserIdentity::from_surface(&corrupt, URL).is_err());
    for locator in [
        "A page title",
        "/fixture/work.html",
        "file:///fixture/work.html",
        "https://browser-fixture.test/nul\0",
    ] {
        assert!(BrowserIdentity::from_surface(&fixture.surface, locator).is_err());
    }
    assert!(fixture.browser.dispatch_attempts.borrow().is_empty());
}
