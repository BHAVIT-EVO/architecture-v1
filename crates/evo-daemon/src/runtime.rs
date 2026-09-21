//! Runtime spine for evo-daemon.
//!
//! The runtime owns the live capture composition for the supported macOS
//! WindowFocusGained path and exposes two runtime boundaries:
//!
//! - accepted Observations; and
//! - the daemon-owned Workspace/Snapshot handoff that feeds restoration
//!   consumers.

use crate::cache::CanonicalIndex;
use crate::daemon_status::{
    clear_capture_status, write_capture_report, CaptureStatus,
};
use crate::continuation::{continuation_surface_socket_path, spawn_continuation_listener};
use crate::designation::{designation_socket_path, spawn_designation_listener};
use crate::grouping::{grouping_socket_path, spawn_grouping_listener};
use crate::errors::DaemonError;
use crate::persistence::persist_observation;
use evo_artifact::accept::accept as accept_artifact;
use evo_capture::{
    CaptureEngine, FSEventsWatcher, MacOSAdapter, MacOSEventSource, MacOSSignal, MacOSInputCounter,
    MacOSURLPoller,
};
use evo_observation::observation::Observation;
use evo_observation::provenance::ObservationSource;
use evo_restoration::DerivationOutcome;
use evo_workspace::snapshot::Snapshot;
use evo_workspace::workspace::Workspace;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Public runtime boundary for the daemon.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Runtime;

/// Keepalive handle for the active macOS focus runtime.
pub struct WindowFocusRuntime {
    _source: MacOSEventSource,
}

/// Keepalive handle for the daemon-owned vertical runtime spine.
pub struct VerticalRuntimeHandle {
    capture: Option<WindowFocusRuntime>,
    url_poller: Option<MacOSURLPoller>,
    fs_watcher: Option<FSEventsWatcher>,
    input_counter: Option<MacOSInputCounter>,
    worker: Option<JoinHandle<()>>,
    designation_listener: Option<JoinHandle<()>>,
    grouping_listener: Option<JoinHandle<()>>,
    continuation_listener: Option<JoinHandle<()>>,
    storage_root: PathBuf,
}

impl Drop for VerticalRuntimeHandle {
    fn drop(&mut self) {
        // 1. Stop every capture source first. Their signal senders close,
        //    which lets the worker's signal loop terminate on its own.
        self.capture.take();
        self.url_poller.take();
        self.fs_watcher.take();
        self.input_counter.take();
        // 2. Join the worker so no thread touches storage or platform FFI
        //    after the runtime is torn down.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // 3. The declaration listeners block on socket accepts; drop their
        //    handles without joining, and remove the sockets so the next
        //    daemon start binds fresh.
        self.designation_listener.take();
        self.grouping_listener.take();
        self.continuation_listener.take();
        let _ = std::fs::remove_file(designation_socket_path(&self.storage_root));
        let _ = std::fs::remove_file(grouping_socket_path(&self.storage_root));
        let _ = std::fs::remove_file(continuation_surface_socket_path(&self.storage_root));
        // 4. Remove the operational capture-status file: a stopped daemon
        //    makes no claims about capture.
        clear_capture_status(&self.storage_root);
    }
}

/// The daemon-owned handoff boundary between the vertical Workspace/Snapshot
/// spine and Restoration consumers.
///
/// Carries the canonical Restoration Derivation input (Workspace + selected
/// Snapshot) together with the canonical derivation outcome produced from it.
#[derive(Debug, Clone, PartialEq)]
pub struct RestorationInputBoundary {
    workspace: Workspace,
    snapshot: Snapshot,
    outcome: DerivationOutcome,
}

/// Alias kept small and explicit for callers that only need the running handle.
pub type WindowFocusRuntimeHandle = WindowFocusRuntime;

impl Runtime {
    /// Creates a new runtime boundary value.
    pub fn new() -> Self {
        Self
    }

    /// Starts the live macOS WindowFocusGained spine and returns the runtime
    /// keepalive plus a receiver of accepted Observations.
    pub fn start_window_focus_runtime(
        &self,
    ) -> Result<(WindowFocusRuntimeHandle, Receiver<Observation>), DaemonError> {
        let (sender, receiver) = channel();
        let pipeline = Rc::new(RefCell::new(LiveCapturePipeline::new(
            sender,
            std::sync::Arc::new(std::sync::Mutex::new(None)),
        )));

        let source = MacOSEventSource::new(
            evo_capture::desktop_shell_pid(),
            {
                let pipeline = Rc::clone(&pipeline);
                move |signal| {
                    let _ = pipeline.borrow_mut().handle_signal(signal);
                }
            },
        )?;

        Ok((WindowFocusRuntime { _source: source }, receiver))
    }

    /// Starts the live macOS WindowFocusGained spine and the daemon-owned
    /// Workspace/Snapshot boundary.
    pub fn start_vertical_runtime(
        &self,
    ) -> Result<
        (
            VerticalRuntimeHandle,
            Receiver<Result<RestorationInputBoundary, DaemonError>>,
        ),
        DaemonError,
    > {
        self.start_vertical_runtime_with_storage_root(evo_storage::canonical_storage_root())
    }

    /// Starts the live macOS WindowFocusGained spine and the daemon-owned
    /// Workspace/Snapshot boundary using the supplied storage root.
    pub fn start_vertical_runtime_with_storage_root(
        &self,
        storage_root: impl Into<PathBuf>,
    ) -> Result<
        (
            VerticalRuntimeHandle,
            Receiver<Result<RestorationInputBoundary, DaemonError>>,
        ),
        DaemonError,
    > {
        // One signal channel feeds the live pipeline from every capture
        // source: the Accessibility event source (focus), the URL poller
        // (navigation), the FSEvents watcher (file saves and commits), and
        // the user designation channel (RFC-0011).
        let (signal_sender, signal_receiver) = channel::<MacOSSignal>();
        let (designation_sender, designation_receiver) = channel::<MacOSSignal>();
        let (observation_sender, _observation_receiver) = channel::<Observation>();
        let storage_root = storage_root.into();
        // Build the derived canonical index up front: this validates the
        // storage root (a corrupt log fails loudly at startup) and gives the
        // vertical pipeline a rebuildable read cache that it keeps current
        // without re-parsing the full logs on every observation.
        let canonical_index = CanonicalIndex::new_capture_only(storage_root.as_path())?;

        // Window-focus capture requires Accessibility permission. When it is
        // not granted, the daemon does not exit: the other sources (file
        // saves, designations) operate independently and safely, and the
        // daemon records that window-focus capture is unavailable so the
        // desktop never claims full capture. The source is skipped, not
        // faked: without Accessibility no window-focus signal is ever
        // produced.
        let focus_source = match MacOSEventSource::new(
            evo_capture::desktop_shell_pid(),
            {
                let sender = signal_sender.clone();
                move |signal| {
                    let _ = sender.send(signal);
                }
            },
        ) {
            Ok(source) => Some(source),
            Err(evo_capture::MacOSEventSourceError::AccessibilityPermissionRequired) => None,
            Err(err) => return Err(DaemonError::Capture(err)),
        };
        let capture = focus_source.map(|_source| WindowFocusRuntime { _source });

        // The content-free input counter: a fifth capture source. Its flush
        // buckets attribute to the currently focused surface through the
        // shared subject slot. A missing Input Monitoring grant degrades
        // honestly — the daemon stays up, the status line says the source
        // is unavailable, and nothing is ever guessed about input.
        let counter_subject_slot: std::sync::Arc<std::sync::Mutex<Option<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let counter_sink_sender = signal_sender.clone();
        let counter_subject = counter_subject_slot.clone();
        let input_counter = match MacOSInputCounter::start(
            Box::new(move || {
                counter_subject.lock().ok().and_then(|slot| slot.clone())
            }),
            Box::new(move |signal| {
                let _ = counter_sink_sender.send(signal);
            }),
        ) {
            Ok(counter) => Some(counter),
            Err(evo_capture::InputCounterError::InputMonitoringRequired) => None,
            Err(evo_capture::InputCounterError::UnsupportedPlatform) => None,
            Err(evo_capture::InputCounterError::RuntimeSetupFailed(step)) => {
                return Err(DaemonError::Designation(format!(
                    "input counter setup failed: {step}"
                )))
            }
        };

        let url_poller = MacOSURLPoller::start({
            let sender = signal_sender.clone();
            move |signal| {
                let _ = sender.send(signal);
            }
        })?;

        let fs_watcher = FSEventsWatcher::start({
            let sender = signal_sender.clone();
            let storage_root = storage_root.clone();
            move |signal| {
                // Evo must never witness its own canonical writes. When the
                // storage root lives under the watched home directory (a
                // developer/testing configuration), the daemon's own append
                // writes would otherwise surface as FileSaved observations
                // and feed back into capture. This is capture scope at the
                // composition boundary — it never alters the meaning of any
                // emitted fact. The membership witness for Evo's own paths
                // is suppressed alongside the save itself.
                match &signal {
                    MacOSSignal::FileSaved { subject, .. }
                    | MacOSSignal::RepositoryMembership { member: subject, .. } => {
                        if storage_path_is_inside(&storage_root, subject) {
                            return;
                        }
                    }
                    _ => {}
                }
                let _ = sender.send(signal);
            }
        })?;

        // The user designation channel: the desktop sends explicit
        // designations over the socket; the daemon (the canonical runtime
        // owner) validates, accepts, persists, and derives them.
        let designation_listener = spawn_designation_listener(
            storage_root.clone(),
            designation_sender.clone(),
        );
        // The user grouping channel (RFC-0012): the desktop sends explicit
        // related-work declarations over its own socket; the daemon validates
        // both subjects, canonicalizes the pair, and forwards it for
        // persistence and derivation.
        let grouping_listener = spawn_grouping_listener(storage_root.clone(), designation_sender.clone());
        // The user continuation-surface channel (RFC-0013): the desktop sends
        // explicit continuation declarations over its own socket; the daemon
        // validates every subject, canonicalizes the set, and forwards it for
        // persistence and derivation.
        let continuation_listener =
            spawn_continuation_listener(storage_root.clone(), designation_sender);

        let (boundary_sender, boundary_receiver) = channel();
        let worker_root = storage_root.clone();
        let worker = thread::spawn(move || {
            let _storage_guard = evo_storage::Storage::with_thread_root(worker_root.clone());
            let mut capture_pipeline =
                LiveCapturePipeline::new(observation_sender, counter_subject_slot.clone());
            let mut vertical_pipeline = VerticalPipeline::with_index(boundary_sender, canonical_index);

            // Designations are drained before capture signals so an explicit
            // user action is never starved by the capture stream.
            loop {
                while let Ok(signal) = designation_receiver.try_recv() {
                    if let Some(observation) = capture_pipeline.handle_signal(signal) {
                        vertical_pipeline.handle_observation(observation);
                    }
                }
                match signal_receiver.recv_timeout(Duration::from_millis(250)) {
                    Ok(signal) => {
                        if let Some(observation) = capture_pipeline.handle_signal(signal) {
                            vertical_pipeline.handle_observation(observation);
                        }
                    }
                    // Capture has gone quiet. The daemon does NOT derive its
                    // legacy engagement understanding here: that derivation is
                    // a whole-corpus affinity rebuild whose cost grows with
                    // the entire observation history, so periodic settling
                    // would pin a core forever on a well-used machine.
                    // Understanding from capture is the ledger engine's job
                    // (a separate process over the same log); the daemon's
                    // own derivation runs only when the person speaks — a
                    // declaration settles immediately in `handle_observation`
                    // — and when the worker drains for the final publish.
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => {
                        // Publish what the final records established before the
                        // worker exits.
                        vertical_pipeline.settle();
                        break;
                    }
                }
            }
        });

        // Report what capture is actually live. This is operational state
        // (never canonical evidence): the desktop reads it to avoid claiming
        // full capture when window-focus is unavailable, and diagnostics read
        // the per-source health lines to answer "is Evo actually watching?".
        // Every source below has already started (a source that fails to
        // start aborts the runtime, so a running daemon's report is honest).
        let status = match &capture {
            Some(_) => CaptureStatus::Full,
            None => CaptureStatus::Partial {
                detail: "Accessibility permission is required to witness focused windows"
                    .to_string(),
            },
        };
        let sources = [
            (
                "window-focus",
                if capture.is_some() { "active" } else { "unavailable" },
            ),
            ("file-save", "active"),
            ("commit", "active"),
            ("url", "active"),
            (
                "input-count",
                if input_counter.is_some() {
                    "active"
                } else {
                    "unavailable: grant Input Monitoring in System Settings > Privacy & Security"
                },
            ),
            ("designation", "active"),
            ("grouping", "active"),
            ("continuation-surface", "active"),
        ];
        write_capture_report(&storage_root, &status, &sources)?;

        Ok((
            VerticalRuntimeHandle {
                capture,
                url_poller: Some(url_poller),
                fs_watcher: Some(fs_watcher),
                input_counter,
                worker: Some(worker),
                designation_listener: Some(designation_listener),
                grouping_listener: Some(grouping_listener),
                continuation_listener: Some(continuation_listener),
                storage_root,
            },
            boundary_receiver,
        ))
    }
}

impl WindowFocusRuntime {
    /// Runs the live macOS run loop until the process stops it.
    pub fn run(&self) -> Result<(), DaemonError> {
        #[cfg(target_os = "macos")]
        unsafe {
            CFRunLoopRun();
            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            Err(DaemonError::UnsupportedPlatform)
        }
    }
}

impl VerticalRuntimeHandle {
    /// Runs the underlying macOS capture runtime.
    ///
    /// With window-focus capture available, this drives the macOS run loop.
    /// Without it (partial capture), the daemon stays alive as long as the
    /// remaining sources — file saves and designations — keep their threads
    /// running; this parks on the same loop so the process serves those
    /// sources until it is stopped.
    pub fn run(&self) -> Result<(), DaemonError> {
        match self.capture.as_ref() {
            Some(capture) => capture.run(),
            None => {
                #[cfg(target_os = "macos")]
                unsafe {
                    CFRunLoopRun();
                }
                #[cfg(not(target_os = "macos"))]
                {
                    return Err(DaemonError::UnsupportedPlatform);
                }
                Ok(())
            }
        }
    }
}

impl RestorationInputBoundary {
    /// Constructs a new restoration input boundary carrying the canonical
    /// derivation input and its derived outcome.
    pub fn new(workspace: Workspace, snapshot: Snapshot, outcome: DerivationOutcome) -> Self {
        Self {
            workspace,
            snapshot,
            outcome,
        }
    }

    /// Returns the canonical Workspace handed to Restoration Derivation.
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Returns the canonical Snapshot handed to Restoration Derivation.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// Returns the canonical derivation outcome produced from the input.
    pub fn outcome(&self) -> &DerivationOutcome {
        &self.outcome
    }
}

/// The live capture pipeline shared by every macOS producer.
///
/// Each platform signal is normalized by the adapter appropriate to its
/// witness mechanism and ingested against the frozen Observation Schema its
/// canonical concept carries (IS-0003). The pipeline never invents a schema:
/// a signal that does not map to a canonical concept produces no Observation.
struct LiveCapturePipeline {
    accessibility_adapter: MacOSAdapter,
    filesystem_adapter: MacOSAdapter,
    designation_adapter: MacOSAdapter,
    grouping_adapter: MacOSAdapter,
    continuation_adapter: MacOSAdapter,
    input_adapter: MacOSAdapter,
    engine: CaptureEngine,
    sender: Sender<Observation>,
    /// The document-grain subject of the surface the person is currently
    /// on, shared with the input counter so its flush buckets attribute to
    /// what is actually focused — frontmost attribution, honestly coarse.
    current_subject: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    /// Capture-side write-storm suppression (see [`StormTracker`]).
    storms: StormTracker,
}

/// Capture-side write-storm suppression.
///
/// A build or sync storm writes tens of thousands of files in seconds
/// through one directory tree; persisting every write floods the canonical
/// log with machine output the ledger would drop at attribution anyway —
/// one compile can otherwise cost 80,000 records. The rule is behavioral,
/// the same shape the ledger's attribution applies: when a directory (or
/// any ancestor below the root-ish path prefix) receives more than
/// [`STORM_BURST_MAX`] saves within [`STORM_BURST_WINDOW_MS`], the tree is
/// mid-storm and further saves in it are machine output, not recorded.
/// A person saving files by hand never trips it — and even a hand-export
/// that did would lose nothing the ledger kept: attention still captures
/// the work, and the save rule at attribution is the same.
struct StormTracker {
    /// ancestor directory -> recent save timestamps (epoch ms).
    windows: std::collections::HashMap<String, std::collections::VecDeque<u64>>,
}

/// Saves within this window in one directory tree before it counts as a storm.
const STORM_BURST_MAX: usize = 6;
const STORM_BURST_WINDOW_MS: u64 = 2 * 60 * 1000;
/// Ancestor directories shallower than this many components are never
/// counted — they contain everything, so every save would share a window.
const STORM_MIN_COMPONENTS: usize = 4;

impl StormTracker {
    fn new() -> Self {
        Self { windows: std::collections::HashMap::new() }
    }

    /// Whether this save is machine output inside an active storm — and
    /// therefore not recorded. The save is registered either way.
    fn is_storm(&mut self, path: &str, now_ms: u64) -> bool {
        let mut is_storm = false;
        let mut dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        loop {
            if dir.split('/').filter(|c| !c.is_empty()).count() >= STORM_MIN_COMPONENTS {
                let window = self.windows.entry(dir.to_string()).or_default();
                window.push_back(now_ms);
                while window
                    .front()
                    .is_some_and(|t| now_ms.saturating_sub(*t) > STORM_BURST_WINDOW_MS)
                {
                    window.pop_front();
                }
                if window.len() > STORM_BURST_MAX {
                    is_storm = true;
                }
            }
            match dir.rsplit_once('/') {
                Some((parent, _)) => dir = parent,
                None => break,
            }
        }
        is_storm
    }
}

impl LiveCapturePipeline {
    fn new(
        sender: Sender<Observation>,
        current_subject: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    ) -> Self {
        let accessibility_source =
            ObservationSource::new("macos_accessibility").expect("hardcoded source name is non-empty");
        let filesystem_source =
            ObservationSource::new("macos_fsevents").expect("hardcoded source name is non-empty");
        let designation_source =
            ObservationSource::new("user_designation").expect("hardcoded source name is non-empty");
        let grouping_source =
            ObservationSource::new("user_grouping").expect("hardcoded source name is non-empty");
        let input_source =
            ObservationSource::new("macos_input").expect("hardcoded source name is non-empty");
        let continuation_source =
            ObservationSource::new("user_continuation").expect("hardcoded source name is non-empty");
        Self {
            accessibility_adapter: MacOSAdapter::new(accessibility_source),
            filesystem_adapter: MacOSAdapter::new(filesystem_source),
            designation_adapter: MacOSAdapter::new(designation_source),
            grouping_adapter: MacOSAdapter::new(grouping_source),
            continuation_adapter: MacOSAdapter::new(continuation_source),
            input_adapter: MacOSAdapter::new(input_source),
            engine: CaptureEngine::new(),
            sender,
            current_subject,
            storms: StormTracker::new(),
        }
    }

    /// Routes one platform signal through its frozen schema into acceptance.
    ///
    /// Returns the accepted Observation when the signal maps to a canonical
    /// concept, or `None` when it does not. Provenance sources are honest
    /// about which mechanism witnessed the fact (`macos_accessibility` for
    /// AX-witnessed events, `macos_fsevents` for filesystem-witnessed ones,
    /// `user_designation` for the user's explicit designation act, and
    /// `user_continuation` for the user's explicit continuation-surface
    /// declaration act).
    fn handle_signal(&mut self, signal: MacOSSignal) -> Option<Observation> {
        // Write-storm suppression: a save inside an active directory-tree
        // storm is machine output (a build, a sync, an export batch) — the
        // ledger would drop it at attribution, so it is not recorded at
        // all. Membership witnesses ride the same rule: they arrive with
        // the saves they describe.
        match &signal {
            MacOSSignal::FileSaved { subject, observed_at } => {
                let now_ms = observed_at
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                if self.storms.is_storm(subject, now_ms) {
                    return None;
                }
            }
            MacOSSignal::RepositoryMembership { member, observed_at, .. } => {
                let now_ms = observed_at
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                if self.storms.is_storm(member, now_ms) {
                    return None;
                }
            }
            _ => {}
        }

        // Document-grain identity at the composition boundary: when the
        // focused window exposes the document it is showing, that document
        // is the subject the whole system reasons about — one document is
        // one resource regardless of which window showed it. The window
        // title survives in the observation's own observed state. The same
        // resolution feeds the shared current-subject slot the input
        // counter attributes its flush buckets to.
        let signal = match signal {
            MacOSSignal::WindowFocusGained {
                subject,
                observed_at,
                process_identifier,
                owning_process_name,
                observed_state,
            } => {
                let locator = observed_state
                    .as_ref()
                    .and_then(|state| state.document_locator().map(str::to_string));
                let resolved = MacOSAdapter::resolve_focus_subject(&subject, locator);
                if let Ok(mut slot) = self.current_subject.lock() {
                    *slot = Some(resolved.clone());
                }
                MacOSSignal::WindowFocusGained {
                    subject: resolved,
                    observed_at,
                    process_identifier,
                    owning_process_name,
                    observed_state,
                }
            }
            other => other,
        };
        let adapter = match &signal {
            MacOSSignal::FileSaved { .. }
            | MacOSSignal::CommitMade { .. }
            | MacOSSignal::RepositoryMembership { .. } => &self.filesystem_adapter,
            MacOSSignal::WorkDesignated { .. } => &self.designation_adapter,
            MacOSSignal::WorkGrouped { .. } => &self.grouping_adapter,
            MacOSSignal::ContinuationSurface { .. } => &self.continuation_adapter,
            MacOSSignal::InputActivity { .. } => &self.input_adapter,
            _ => &self.accessibility_adapter,
        };
        let raw_event = adapter.normalize(signal).ok()??;
        let schema = raw_event.schema();
        let observation = self.engine.ingest(raw_event, &schema).ok()?;
        let _ = self.sender.send(observation.clone());
        Some(observation)
    }
}

/// The daemon's vertical spine: persist canonical evidence, keep the derived
/// index current, and publish the understanding once capture settles.
///
/// # What changed, and why
///
/// **The old rule.** Every accepted Observation ran Workspace Formation
/// immediately, persisted the resulting Workspace state and Restoration
/// outcome, and announced a boundary. A declaration additionally re-derived and
/// re-persisted the outcome of every remembered Workspace, and could
/// *originate* a Workspace for a declared subject that belonged to none.
///
/// **Why it could not work.** Deciding membership per Observation means deciding
/// it without the corpus that determines it (see [`crate::understanding`]), and
/// persisting the decision means an append-only store full of guesses that
/// cannot be withdrawn. Declaration-triggered origination existed only to patch
/// the same defect from the other side: because formation could not see that a
/// declared resource belonged with anything, the runtime had to manufacture a
/// Workspace for it by hand.
///
/// **The new rule.** The runtime persists Observations and Artifacts — the two
/// canonical layers — and nothing else. Understanding is derived by
/// [`crate::understanding`] from the accumulated evidence, so declarations need
/// no special origination path: a declaration is ground truth inside the
/// reconstruction itself, which is where it belongs (§10).
///
/// # When derivation runs
///
/// Deriving reads the whole corpus, so running it per Observation would be
/// quadratic over a day of use. It therefore runs when capture has *settled*:
/// [`VerticalPipeline::settle`] is called on the worker's idle tick, and an
/// explicit user declaration settles immediately rather than waiting behind the
/// capture stream. This is scheduling, not semantics — what gets derived is a
/// pure function of the evidence, so a burst of ten focus changes and the same
/// ten replayed later derive the same understanding either way.
pub struct VerticalPipeline {
    /// The derived canonical index: the bodies of work, their Restoration
    /// outcomes, and the standing declarations. Derived from the Observation
    /// log only; a fresh index rebuilt from the log produces identical state.
    index: CanonicalIndex,
    sender: Sender<Result<RestorationInputBoundary, DaemonError>>,
    /// The Restoration outcome most recently announced for each Workspace, so
    /// settling republishes only what actually changed.
    announced: HashMap<String, DerivationOutcome>,
}

impl VerticalPipeline {
    /// Constructs the pipeline over a freshly built derived canonical index.
    ///
    /// # Errors
    ///
    /// Returns a [`DaemonError`] when the Observation log is corrupt.
    pub fn new(
        sender: Sender<Result<RestorationInputBoundary, DaemonError>>,
        storage_root: impl Into<PathBuf>,
    ) -> Result<Self, DaemonError> {
        Ok(Self::with_index(sender, CanonicalIndex::new(storage_root)?))
    }

    /// Constructs the pipeline over an existing derived canonical index.
    pub fn with_index(
        sender: Sender<Result<RestorationInputBoundary, DaemonError>>,
        index: CanonicalIndex,
    ) -> Self {
        Self {
            index,
            sender,
            announced: HashMap::new(),
        }
    }

    /// Accepts one witnessed Observation into canonical state.
    ///
    /// Persistence order is the canonical one: the Observation is durably
    /// recorded before anything downstream reports success (IS-0001 R-7), then
    /// the Artifact it establishes is accepted through the Artifact Acceptance
    /// Pipeline (IS-0005), then the derived index absorbs it.
    ///
    /// Reference-only schemas (RFC-0011 WorkDesignated; RFC-0012
    /// RepositoryMembership and WorkGrouped; RFC-0013 ContinuationSurface)
    /// reference existing canonical Artifacts and never establish one, so no
    /// Artifact acceptance runs for them. They do settle immediately: a
    /// declaration is the person speaking, and their word never waits behind
    /// the capture stream (§10).
    pub fn handle_observation(&mut self, observation: Observation) {
        if let Err(err) = persist_observation(&observation) {
            let _ = self.sender.send(Err(err));
            return;
        }
        let reference_only = observation.schema().is_reference_only();
        let user_declaration = observation.schema().is_user_declaration();
        if !reference_only {
            // The Artifact layer stays canonical: acceptance validates,
            // canonicalizes, assigns identity, verifies integrity, and
            // persists (IS-0005). Its identity is derived by the same rule the
            // index applies, so the two never disagree.
            if let Err(err) = accept_artifact(observation.clone()) {
                let _ = self.sender.send(Err(DaemonError::from(err)));
                return;
            }
        }
        self.index.ingest_observation(observation);
        if user_declaration {
            // Only the person's own declarations settle immediately (§10).
            // Machine-witnessed reference records — a file's repository
            // membership arrives with every build — take the capture path:
            // settling on each would rebuild the whole-corpus understanding
            // per file write.
            self.settle();
        }
    }

    /// Derives the understanding if evidence has arrived since it was last
    /// derived, and announces every body of **work** whose Restoration outcome
    /// changed.
    ///
    /// The announcement is a Restoration boundary — a "get back into it"
    /// signal — so it carries only work. Remembered bodies are kept in the
    /// index (findable by name, projected like any other) but are never
    /// pushed here: witnessing a scrap is not an invitation to resume it. This
    /// is a gate on the *channel*, not on existence, so nothing is lost.
    ///
    /// Idempotent and cheap when nothing changed, so the worker can call it on
    /// every idle tick.
    pub fn settle(&mut self) {
        if !self.index.reconstruct() {
            return;
        }
        // Cloned up front because announcing borrows `self` mutably.
        let workspaces = self.index.workspaces().to_vec();
        let outcomes = self.index.outcomes().clone();
        let standings = self.index.standings().clone();
        for workspace in workspaces {
            let key = workspace.id().to_string();
            // Only work is announced. A Remembered body remains in the index,
            // reachable by name, but generates no Restoration boundary.
            if !standings.get(&key).is_some_and(|standing| standing.is_work()) {
                continue;
            }
            let Some(outcome) = outcomes.get(&key) else {
                continue;
            };
            if self.announced.get(&key) == Some(outcome) {
                continue;
            }
            // The last Snapshot is the most recent sitting of this body of
            // work, which is what "resume where you left off" means.
            let Some(snapshot) = workspace.snapshots().last().cloned() else {
                continue;
            };
            self.announced.insert(key, outcome.clone());
            let boundary =
                RestorationInputBoundary::new(workspace, snapshot, outcome.clone());
            let _ = self.sender.send(Ok(boundary));
        }
    }

    /// The derived index, for callers that read understanding directly rather
    /// than through the boundary channel.
    pub fn index(&self) -> &CanonicalIndex {
        &self.index
    }
}

/// Whether a witnessed file path lies inside the canonical storage root.
///
/// Used at the composition boundary to keep Evo from re-witnessing its own
/// append writes when the storage root is under the watched home directory.
fn storage_path_is_inside(root: &Path, subject: &str) -> bool {
    Path::new(subject).starts_with(root)
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRunLoopRun();
}

#[cfg(test)]
mod storm_tests {
    use super::StormTracker;

    /// One compile's worth of writes through one tree: only the first few
    /// record; the storm swallows the rest.
    #[test]
    fn a_build_storm_is_suppressed_after_the_first_few_saves() {
        let mut tracker = StormTracker::new();
        let mut suppressed = 0;
        for i in 0..500 {
            if tracker.is_storm(
                &format!("/Users/alice/project/target/debug/deps/lib_{i}.rlib"),
                1_000_000 + i * 10, // 10ms apart: a real build's cadence
            ) {
                suppressed += 1;
            }
        }
        // The first STORM_BURST_MAX saves per ancestor window record; the
        // remaining ~490 are machine output.
        assert!(suppressed > 480, "storm must swallow the bulk, got {suppressed}");
    }

    /// A person saving a handful of files by hand never trips the rule.
    #[test]
    fn human_paced_saves_are_never_suppressed() {
        let mut tracker = StormTracker::new();
        for i in 0..5 {
            let stormy = tracker.is_storm(
                &format!("/Users/alice/Documents/report-{i}.md"),
                1_000_000 + i * 30_000, // 30s apart: a person's cadence
            );
            assert!(!stormy, "a hand-paced save is never machine output");
        }
    }

    /// The storm window expires: a tree that went quiet accepts saves again.
    #[test]
    fn the_storm_window_expires() {
        let mut tracker = StormTracker::new();
        for i in 0..20 {
            tracker.is_storm(
                &format!("/Users/alice/project/target/debug/out-{i}.txt"),
                1_000_000 + i * 5,
            );
        }
        // Three minutes later, past the window, saves record again.
        let later = 1_000_000 + 3 * 60 * 1000;
        assert!(!tracker.is_storm("/Users/alice/project/target/debug/manual.txt", later));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::understanding;
    use crate::workspace_replay::replay_workspaces_from_root;
    use evo_capture::MacOSSignal;
    use evo_observation::observation_schema::ObservationSchema;
    use evo_observation::provenance::ObservationSource;
    use evo_storage::Storage;
    use evo_workspace::attachment::ResourceRole;
    use std::time::SystemTime;
    use std::path::PathBuf;

    #[test]
    fn runtime_is_constructible_copy_and_default() {
        let runtime = Runtime::new();
        let copied = runtime;
        let cloned = runtime.clone();
        let defaulted: Runtime = Default::default();

        assert_eq!(runtime, copied);
        assert_eq!(runtime, cloned);
        assert_eq!(runtime, defaulted);
    }

    #[test]
    fn live_pipeline_translates_supported_signal_into_accepted_observation() {
        let (sender, receiver) = channel();
        let mut pipeline = LiveCapturePipeline::new(sender, std::sync::Arc::new(std::sync::Mutex::new(None)));

        let accepted = pipeline.handle_signal(MacOSSignal::WindowFocusGained {
            subject: "editor-window".into(),
            observed_at: SystemTime::UNIX_EPOCH,
            process_identifier: Some(4242),
            owning_process_name: None,
            observed_state: None,
        });
        assert!(accepted.is_some(), "focus signal should be accepted");

        let observation = receiver
            .try_recv()
            .expect("accepted observation should be emitted");
        assert_eq!(observation.schema(), &ObservationSchema::window_focus_gained_v3());
        assert_eq!(
            observation.provenance().source(),
            &ObservationSource::new("macos_accessibility").unwrap()
        );
        // The owning process the signal carried is provenance of the
        // witnessing and survives the pipeline into `Provenance::context`
        // (BE-TRACE-0001 §2.1); it never becomes an Evidence fact.
        assert_eq!(
            observation
                .provenance()
                .context()
                .get(evo_capture::OWNING_PROCESS_PID_CONTEXT_KEY)
                .map(String::as_str),
            Some("4242")
        );
        assert_eq!(observation.evidence().facts().len(), 1);
        assert_eq!(
            observation.evidence().fact("WindowFocusGained").unwrap().value(),
            &evo_observation::evidence::FactValue::Text("editor-window".into())
        );
    }

    #[test]
    fn live_pipeline_ignores_application_activation() {
        let (sender, receiver) = channel();
        let mut pipeline = LiveCapturePipeline::new(sender, std::sync::Arc::new(std::sync::Mutex::new(None)));

        let accepted = pipeline.handle_signal(MacOSSignal::ApplicationActivated {
            bundle_identifier: Some("com.apple.TextEdit".into()),
            application_name: Some("TextEdit".into()),
            process_identifier: Some(42),
            observed_at: SystemTime::UNIX_EPOCH,
        });

        assert!(accepted.is_none());
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn document_grain_focus_resolves_the_subject_and_feeds_the_counter_slot() {
        // A focused editor window exposing its document: the subject the
        // system reasons about becomes the file, not the window title, and
        // the shared slot the input counter reads holds the same resolved
        // subject.
        let (sender, _receiver) = channel();
        let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
        let mut pipeline = LiveCapturePipeline::new(sender, slot.clone());

        let observation = pipeline
            .handle_signal(MacOSSignal::WindowFocusGained {
                subject: "main.rs — evo — Visual Studio Code".into(),
                observed_at: SystemTime::UNIX_EPOCH,
                process_identifier: Some(4242),
                owning_process_name: None,
                observed_state: Some(
                    evo_observation::observed_state::ObservedState::new()
                        .with_document_locator("file:///Users/p/evo/src/main.rs"),
                ),
            })
            .expect("focus with a document is accepted");

        assert_eq!(
            observation.evidence().fact("WindowFocusGained").unwrap().value(),
            &evo_observation::evidence::FactValue::Text("file:///Users/p/evo/src/main.rs".into()),
            "the document is the subject"
        );
        assert_eq!(
            *slot.lock().unwrap(),
            Some("file:///Users/p/evo/src/main.rs".to_string()),
            "the counter's attribution slot holds the resolved subject"
        );
    }

    #[test]
    fn input_buckets_are_accepted_with_the_current_subject() {
        // A flush bucket attributed to the focused surface becomes a
        // canonical InputActivity observation, content-free.
        let (sender, receiver) = channel();
        let slot = std::sync::Arc::new(std::sync::Mutex::new(Some(
            "file:///Users/p/evo/src/main.rs".to_string(),
        )));
        let mut pipeline = LiveCapturePipeline::new(sender, slot);

        let observation = pipeline
            .handle_signal(MacOSSignal::InputActivity {
                subject: Some("file:///Users/p/evo/src/main.rs".into()),
                keys: 61,
                clicks: 4,
                scrolls: 12,
                observed_at: SystemTime::UNIX_EPOCH,
            })
            .expect("an attributed input bucket is accepted");

        assert_eq!(
            observation.schema().name(),
            "OBS-INPUT-ACTIVITY",
            "the input source has its own frozen schema"
        );
        assert_eq!(
            observation.evidence().fact("Keys").unwrap().value(),
            &evo_observation::evidence::FactValue::Integer(61)
        );
        assert!(receiver.try_recv().is_ok());
    }

    #[test]
    fn live_pipeline_routes_each_signal_to_its_frozen_schema() {
        let (sender, receiver) = channel();
        let mut pipeline = LiveCapturePipeline::new(sender, std::sync::Arc::new(std::sync::Mutex::new(None)));
        let observed_at = SystemTime::UNIX_EPOCH;

        let focus = pipeline
            .handle_signal(MacOSSignal::WindowFocusGained {
                subject: "editor-window".into(),
                observed_at,
                process_identifier: None,
                owning_process_name: None,
                observed_state: None,
            })
            .expect("focus signal accepted");
        assert_eq!(
            focus.schema(),
            &ObservationSchema::window_focus_gained_v3()
        );

        let file = pipeline
            .handle_signal(MacOSSignal::FileSaved {
                subject: "/Users/alice/Documents/report.md".into(),
                observed_at,
            })
            .expect("file signal accepted");
        assert_eq!(file.schema(), &ObservationSchema::file_saved_v1());
        assert_eq!(
            file.provenance().source().as_str(),
            "macos_fsevents"
        );

        let url = pipeline
            .handle_signal(MacOSSignal::URLNavigated {
                subject: "https://example.com/doc".into(),
                observed_at,
            })
            .expect("url signal accepted");
        assert_eq!(url.schema(), &ObservationSchema::url_navigated_v1());
        assert_eq!(
            url.provenance().source().as_str(),
            "macos_accessibility"
        );

        let commit = pipeline
            .handle_signal(MacOSSignal::CommitMade {
                subject: "abcdef0123456789abcdef0123456789abcdef01".into(),
                observed_at,
            })
            .expect("commit signal accepted");
        assert_eq!(commit.schema(), &ObservationSchema::commit_made_v1());
        assert_eq!(
            commit.provenance().source().as_str(),
            "macos_fsevents"
        );

        assert_eq!(receiver.try_iter().count(), 4);
    }

    #[ignore = "starts the real platform runtime (AX observer, FSEvents watcher, input tap) in-process; CoreFoundation cannot always tear these down under parallel test load — run explicitly with: cargo test -p evo-daemon -- --ignored"]
    #[test]
    fn runtime_start_vertical_runtime_exposes_boundary_receiver() {
        let runtime = Runtime::new();
        // The live event source witnesses the real frontmost application and
        // emits genuine observations, which the pipeline persists. Route that
        // real persistence into an isolated root so the canonical default
        // storage stays clean during tests.
        let result = runtime.start_vertical_runtime_with_storage_root(unique_root("boundary-receiver"));

        // The vertical runtime must start even without Accessibility
        // permission: it degrades to partial capture (file saves and
        // designations) rather than exiting. Only a non-macOS platform is an
        // acceptable failure.
        match result {
            Ok((_handle, receiver)) => {
                assert!(receiver.try_recv().is_err());
            }
            Err(DaemonError::Capture(
                evo_capture::MacOSEventSourceError::UnsupportedPlatform,
            )) => {}
            Err(err) => panic!("unexpected runtime error: {err}"),
        }
    }

    #[test]
    fn runtime_without_focus_source_reports_partial_status() {
        // The decision to keep the daemon alive without window-focus capture
        // is structural; exercise the status reporting that follows it:
        // writing `capture=partial` and reading it back exactly.
        let root = unique_root("partial-status");
        let partial = crate::daemon_status::CaptureStatus::Partial {
            detail: "Accessibility permission is required to witness focused windows".to_string(),
        };
        crate::daemon_status::write_capture_status(&root, &partial).expect("write");
        assert_eq!(crate::daemon_status::read_capture_status(&root), Some(partial));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn storage_path_filter_excludes_only_the_storage_root() {
        let root = std::env::temp_dir().join("evo-test-storage-root");
        // Inside the storage root: Evo's own canonical writes.
        let inside = root.join("observation.log");
        assert!(storage_path_is_inside(&root, inside.to_string_lossy().as_ref()));
        // A file that merely shares a prefix is not inside the root.
        let sibling = std::env::temp_dir().join("evo-test-storage-root-other/notes.md");
        assert!(!storage_path_is_inside(&root, sibling.to_string_lossy().as_ref()));
        // Unrelated paths are not inside.
        assert!(!storage_path_is_inside(&root, "/Users/alice/Documents/report.md"));
    }

    // ── The settle-and-announce path ─────────────────────────────────────────

    fn t(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(secs))
            .expect("representable fixture time")
    }

    fn focus(subject: &str, at: u64) -> MacOSSignal {
        MacOSSignal::WindowFocusGained {
            subject: subject.to_string(),
            observed_at: t(at),
            process_identifier: None,
            owning_process_name: None,
            observed_state: None,
        }
    }

    fn saved(subject: &str, at: u64) -> MacOSSignal {
        MacOSSignal::FileSaved {
            subject: subject.to_string(),
            observed_at: t(at),
        }
    }

    /// A harness over the exact daemon composition: real capture normalization
    /// and acceptance, real persistence, the real derived index. Only the
    /// worker thread's `recv_timeout` is replaced by an explicit
    /// [`Harness::settle`] call, so a test controls when capture goes quiet.
    struct Harness {
        capture: LiveCapturePipeline,
        observations: Receiver<Observation>,
        pipeline: VerticalPipeline,
        boundaries: Receiver<Result<RestorationInputBoundary, DaemonError>>,
    }

    impl Harness {
        fn new(root: &Path) -> Self {
            let (observation_sender, observations) = channel();
            let (boundary_sender, boundaries) = channel();
            let pipeline = VerticalPipeline::new(boundary_sender, root.to_path_buf())
                .expect("pipeline builds over an empty log");
            Self {
                capture: LiveCapturePipeline::new(
                    observation_sender,
                    std::sync::Arc::new(std::sync::Mutex::new(None)),
                ),
                observations,
                pipeline,
                boundaries,
            }
        }

        /// Drives one platform signal along the whole daemon path.
        fn drive(&mut self, signal: MacOSSignal) {
            self.capture.handle_signal(signal);
            let observation = self
                .observations
                .try_recv()
                .expect("a canonical signal produces an accepted observation");
            self.pipeline.handle_observation(observation);
        }

        /// What the worker does on an idle tick.
        fn settle(&mut self) {
            self.pipeline.settle();
        }

        fn announced(&self) -> Vec<RestorationInputBoundary> {
            self.boundaries
                .try_iter()
                .map(|result| result.expect("no persistence error"))
                .collect()
        }
    }

    /// One coherent task: a document, a terminal working on it, and a browser
    /// page researching it — returned to across several sittings.
    fn drive_one_body_of_work(harness: &mut Harness, sittings: usize) -> u64 {
        let mut moment = 0u64;
        for _sitting in 0..sittings {
            for _pass in 0..4 {
                harness.drive(focus("Migration Plan — Editor", moment));
                moment += 200;
                harness.drive(saved("/Users/alice/ops/migration-plan.md", moment));
                moment += 20;
                harness.drive(focus("migration plan rollout — Terminal", moment));
                moment += 200;
            }
            moment += 4 * 60 * 60;
        }
        moment
    }

    /// Understanding is published when capture settles, not per Observation.
    /// Before the model existed this was the other way round, and every focus
    /// change announced a body of work.
    #[test]
    fn nothing_is_announced_until_capture_settles() {
        let root = unique_root("settle-boundary");
        let _guard = Storage::with_thread_root(root.clone());
        let mut harness = Harness::new(&root);

        drive_one_body_of_work(&mut harness, 3);
        assert!(
            harness.announced().is_empty(),
            "content Observations alone announce nothing"
        );

        harness.settle();
        let announced = harness.announced();
        assert_eq!(
            announced.len(),
            1,
            "one coherent task settles into one body of work"
        );

        // And what was announced is exactly what the index derived.
        let workspaces = harness.pipeline.index().workspaces();
        assert_eq!(announced[0].workspace(), &workspaces[0]);
        assert_eq!(
            Some(announced[0].outcome()),
            harness
                .pipeline
                .index()
                .outcomes()
                .get(&workspaces[0].id().to_string())
        );
    }

    /// Test A/B/C at the runtime boundary: a resource used with nothing else is
    /// evidence, never a body of work, however long it is looked at.
    #[test]
    fn a_resource_used_alone_is_never_announced() {
        let root = unique_root("settle-alone");
        let _guard = Storage::with_thread_root(root.clone());
        let mut harness = Harness::new(&root);

        for pass in 0..20u64 {
            harness.drive(focus("Spotify Premium", pass * 300));
            harness.settle();
        }

        assert!(
            harness.announced().is_empty(),
            "focus alone never announces a body of work"
        );
        // The Observations are still canonical: nothing was discarded, the
        // resource simply is not work.
        assert_eq!(
            harness.pipeline.index().subjects().len(),
            1,
            "the resource was witnessed and remains addressable evidence"
        );
    }

    /// Settling is idempotent: the boundary channel carries *changes*, so a
    /// quiet worker republishes nothing.
    #[test]
    fn settling_without_new_evidence_announces_nothing() {
        let root = unique_root("settle-idempotent");
        let _guard = Storage::with_thread_root(root.clone());
        let mut harness = Harness::new(&root);

        drive_one_body_of_work(&mut harness, 3);
        harness.settle();
        assert_eq!(harness.announced().len(), 1);

        for _tick in 0..5 {
            harness.settle();
        }
        assert!(
            harness.announced().is_empty(),
            "an idle worker republishes nothing"
        );
    }

    /// §10. An explicit declaration is the person speaking; it settles at once
    /// rather than waiting behind the capture stream.
    #[test]
    fn a_declaration_settles_immediately() {
        let root = unique_root("settle-declaration");
        let _guard = Storage::with_thread_root(root.clone());
        let mut harness = Harness::new(&root);

        let moment = drive_one_body_of_work(&mut harness, 3);
        assert!(harness.announced().is_empty(), "capture has not settled yet");

        harness.drive(MacOSSignal::WorkDesignated {
            subject: "/Users/alice/ops/migration-plan.md".into(),
            observed_at: t(moment + 5),
        });

        let announced = harness.announced();
        assert_eq!(
            announced.len(),
            1,
            "the declaration published the understanding without an idle tick"
        );
        assert_eq!(
            harness.pipeline.index().designation().map(|(subject, _)| subject),
            Some("/Users/alice/ops/migration-plan.md".to_string())
        );
    }

    /// §12/§16. What is announced opens less than it remembers: only the places
    /// the work happens are opened on restore.
    #[test]
    fn what_is_announced_opens_less_than_it_remembers() {
        let root = unique_root("settle-selective");
        let _guard = Storage::with_thread_root(root.clone());
        let mut harness = Harness::new(&root);

        let mut moment = drive_one_body_of_work(&mut harness, 3);
        // Reference material consulted once during the work.
        for reference in [
            "Migration Plan RFC — Browser",
            "migration plan changelog — Browser",
            "Migration Plan — Preview",
        ] {
            harness.drive(focus(reference, moment));
            moment += 60;
        }
        harness.settle();

        let announced = harness.announced();
        assert_eq!(announced.len(), 1);
        let workspace = announced[0].workspace();
        let opening = workspace
            .attachments()
            .iter()
            .filter(|attachment| attachment.role().opens_on_restore())
            .count();
        assert!(
            opening < workspace.attachments().len(),
            "a body of work remembers more than it opens (remembered {}, opens {opening})",
            workspace.attachments().len()
        );
        assert!(opening >= 1, "something must open, or there is no restore");
        assert!(
            workspace
                .attachments()
                .iter()
                .any(|attachment| attachment.role() == ResourceRole::Continuation),
            "exactly one resource is the continuation point"
        );
    }

    /// §17/§18 and ARCHITECTURE §2: the runtime persists canonical evidence and
    /// nothing else. Derived understanding is never written, so it can never go
    /// stale against the evidence that produced it.
    #[test]
    fn the_runtime_persists_only_canonical_evidence() {
        let root = unique_root("settle-canonical-only");
        let _guard = Storage::with_thread_root(root.clone());
        let mut harness = Harness::new(&root);

        drive_one_body_of_work(&mut harness, 2);
        harness.settle();

        for canonical in ["observation.log", "artifact.log"] {
            assert!(
                root.join(canonical).exists(),
                "{canonical} is canonical and must be written"
            );
        }
        for derived in ["workspace.log", "restoration.log"] {
            assert!(
                !root.join(derived).exists(),
                "{derived} is derived understanding and must never be persisted"
            );
        }
    }

    /// Test K/L at the runtime boundary: restart the daemon over the same
    /// canonical log and the understanding comes back identical, because it is
    /// re-derived rather than reloaded.
    #[test]
    fn understanding_survives_a_restart_because_it_is_rederived() {
        let root = unique_root("settle-restart");
        let _guard = Storage::with_thread_root(root.clone());

        let live = {
            let mut harness = Harness::new(&root);
            drive_one_body_of_work(&mut harness, 3);
            harness.settle();
            harness.pipeline.index().workspaces().to_vec()
        };
        assert_eq!(live.len(), 1);

        // A fresh daemon over the same root.
        let restarted = Harness::new(&root);
        assert_eq!(restarted.pipeline.index().workspaces(), live.as_slice());
        // And a full replay of the log agrees with both.
        assert_eq!(
            replay_workspaces_from_root(&root).expect("replay succeeds"),
            live
        );
    }

    /// A reference-only declaration references canonical Artifacts and never
    /// establishes one (RFC-0012 §4), so no Artifact acceptance runs for it.
    #[test]
    fn a_declaration_establishes_no_artifact() {
        let root = unique_root("settle-no-artifact");
        let _guard = Storage::with_thread_root(root.clone());
        let mut harness = Harness::new(&root);

        harness.drive(saved("/Users/alice/project/parser.rs", 10));
        harness.drive(saved("/Users/alice/project/telemetry.rs", 20));
        let witnessed = harness.pipeline.index().subjects().len();

        harness.drive(MacOSSignal::RepositoryMembership {
            member: "/Users/alice/project/parser.rs".into(),
            repository: "/Users/alice/project".into(),
            observed_at: t(30),
        });

        assert_eq!(
            harness.pipeline.index().subjects().len(),
            witnessed,
            "a declaration adds no Artifact"
        );
        let observations = crate::persistence::load_persisted_observations(&root)
            .expect("observations load");
        assert!(
            observations
                .iter()
                .any(|observation| observation.schema()
                    == &ObservationSchema::repository_membership_v1()),
            "the declaration is canonical evidence and was persisted"
        );
    }

    /// §19. The runtime's incremental absorption and a batch read of the same
    /// log are the same function of the same evidence.
    #[test]
    fn live_absorption_equals_a_batch_read_of_the_log() {
        let root = unique_root("settle-equivalence");
        let _guard = Storage::with_thread_root(root.clone());
        let mut harness = Harness::new(&root);

        let moment = drive_one_body_of_work(&mut harness, 3);
        harness.drive(MacOSSignal::ContinuationSurface {
            subjects: vec![
                "/Users/alice/ops/migration-plan.md".into(),
                "Migration Plan — Editor".into(),
            ],
            observed_at: t(moment + 5),
        });

        let observations = crate::persistence::load_persisted_observations(&root)
            .expect("observations load");
        let batch = understanding::derive(
            understanding::evidence_of(&observations),
            evo_engagement::EngagementParams::default(),
        );
        assert_eq!(harness.pipeline.index().workspaces(), batch.workspaces());
        assert_eq!(harness.pipeline.index().outcomes(), batch.outcomes());
    }

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-daemon-{label}-{nanos}"))
    }
}
