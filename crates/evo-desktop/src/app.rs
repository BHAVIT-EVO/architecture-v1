//! The Evo desktop application shell.
//!
//! State and dispatch only. Every pixel is drawn by `home`, `detail` or
//! `shell`, over the primitives in `ui` and `theme`; this file's whole job is
//! to hold Evo's derived understanding, keep it fresh, and hand the right
//! borrows to the right screen.
//!
//! The shell reads canonical Workspace/Snapshot understanding and its derived
//! Restoration outcome from the daemon's persistence boundary and re-reads them
//! periodically, so the screen reflects newly produced boundaries without
//! owning any canonical semantics. The shell never performs Restoration
//! Derivation itself, and it has exactly two write paths — a Work Designation
//! and a Continuation Surface declaration — both of which go to the daemon.
//!
//! # Derived state and user state are cleared by different events
//!
//! A canonical reload replaces *derived* state: the Workspace list, the
//! derivation outcomes, the restoration selection, the preflight dispositions.
//! It must never touch *user-authored transient* state: an execution report, an
//! in-progress continuation edit, or a status message the user has not yet
//! read. Conflating the two is what previously made a successful restoration
//! erase its own result — the restoration produced new Observations, the new
//! Observations changed the canonical signature, and the changed signature
//! cleared the report describing the restoration that caused it.
//!
//! Transient state is therefore scoped to *navigation*, not to reloads: opening
//! a different body of work resets it, because it belonged to the previous one.

use crate::daemon::{DaemonManager, DaemonStatus};
use crate::pod_bar_drag;
use crate::state::{self, DisplayState};
use crate::ui::{self, Control};
use crate::{detail, dock, home, rack, shell, theme};

use evo_daemon::cache::CanonicalIndex;
use evo_daemon::persistence::load_persisted_observations;
use evo_daemon::threads::{app_names_for_threads, resource_apps_for_threads};
use evo_engagement::Standing;
use evo_execution::{
    ExecutionReport, Locator, PreflightOutcome, RestorationSelection, preflight_selection,
};
use evo_restoration::DerivationOutcome;
use evo_workspace::workspace::Workspace;
use evo_workspace::workspace_id::WorkspaceId;
use std::sync::mpsc;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// How often the desktop shell re-reads canonical storage.
pub const RELOAD_PERIOD: Duration = Duration::from_secs(15);

/// How long the "Preparing your workspace" transition is shown before the
/// shell runs the (synchronous) restoration.
///
/// Long enough that the transition paints and reads as intentional; short
/// enough to feel immediate. The restoration is performed here in `logic`
/// after this dwell, never inside the click handler — so the click can return
/// and the transition can present a frame before `run_execution` blocks the
/// thread, and so a future asynchronous binding layer replaces only this step.
pub const CONTINUE_DWELL: Duration = Duration::from_millis(650);

/// The Evo desktop shell state.
pub struct EvoApp {
    storage_root: PathBuf,
    /// Click/swipe actions from the notch surface thread (carousel
    /// Restore/Close chips, drags, open-rack). `Some` only under the
    /// evo-notch feature when the surface spawned; the receiver going
    /// away frees the surface's channel.
    /// The derived canonical index over the storage logs, refreshed
    /// incrementally on every reload so repeated reloads do not re-parse the
    /// full history. A fresh index built from the logs produces identical
    /// state (the logs remain the authoritative canonical evidence).
    index: Option<CanonicalIndex>,
    /// Every remembered Workspace loaded from canonical storage.
    workspaces: Vec<Workspace>,
    /// The Workspace presented on the detail screen.
    display: Option<Workspace>,
    outcome: Option<DerivationOutcome>,
    /// Every persisted derivation outcome, keyed by Workspace identity, so
    /// the remembered-work list reflects each body of work's truthful status.
    outcomes: HashMap<String, DerivationOutcome>,
    /// What each remembered body of work is about, keyed by Workspace
    /// identity — the name Home calls it by, derived from the vocabulary the
    /// work's own members share.
    titles: HashMap<String, String>,
    /// The Engagement-layer standing of each remembered body of work, keyed by
    /// Workspace identity — whether the evidence has earned it a place as work
    /// (Continuable/Restorable) or Evo has only witnessed it (Remembered).
    /// Home presents only work as work; Remembered bodies stay findable by
    /// name and are never discarded (WORK-MODEL: no silent loss).
    standings: HashMap<String, Standing>,
    /// The human-readable subject each canonical Artifact's Observations
    /// witnessed, derived from the persisted Observation log.
    subjects: HashMap<String, String>,
    /// The plain-language kind each canonical Artifact's frozen Observation
    /// schema establishes, derived from the persisted Observation log.
    kinds: HashMap<String, String>,
    /// The executable-target locator for each canonical Artifact, derived from
    /// the persisted Observation log for the Execution layer.
    locators: HashMap<String, Locator>,
    /// The current designation (RFC-0011): the subject the user marked as the
    /// work to continue and when it was witnessed, derived from the log.
    designation: Option<(String, SystemTime)>,
    /// The derived selective-restoration understanding for the displayed
    /// Workspace: what Evo would restore from the current continuation, what
    /// is honestly unavailable, and what remains historical. Purely derived
    /// presentation material; never canonical.
    selection: Option<RestorationSelection>,
    /// The Executor Preflight disposition of every current-continuation member
    /// of the displayed Workspace (READY / UNAVAILABLE / AMBIGUOUS /
    /// UNSUPPORTED), computed at reload from the derived selection plus
    /// execution-time OS state. Presentation material only — transient, never
    /// persisted, never continuation evidence (IS-0021 §7).
    preflight: Vec<PreflightOutcome>,
    /// The per-Workspace home-row material, derived at reload from canonical
    /// state. Presentation-only and transient — never canonical, never
    /// persisted, never a model object (Law XVI).
    home_cards: Vec<state::WorkspaceCard>,
    /// The desktop-owned daemon worker process.
    daemon: DaemonManager,
    // The ledger engine supervisor: keeps the compute process alive so
    // Works stay current across reboots and crashes.
    engine: crate::engine::EngineManager,

    // ── User-authored transient state ───────────────────────────────────
    /// Which body of work the user opened, keyed by canonical Workspace
    /// identity — never by title (IS-0011 W-3: identity carries no meaning,
    /// and a title is presentation).
    selected: Option<WorkspaceId>,
    /// The identity the transient state below belongs to, so navigating to a
    /// different body of work resets it and a canonical reload does not.
    opened: Option<WorkspaceId>,
    /// The structured result of the user's last Continue request, as reported
    /// by the domain Restoration Execution boundary. Cleared by the user, or
    /// by opening different work — never by a reload.
    execution_result: Option<ExecutionReport>,
    /// Desktop windows present before the current work was opened. The
    /// unified workspace arranger uses this boundary to tile only windows
    /// belonging to the newly restored work.
    workspace_before: Option<crate::unified_workspace::WorkspaceSnapshot>,
    /// Deadline for polling native windows after a restore. macOS accepts an
    /// open request before the target app has created its AX windows.
    workspace_arrangement_deadline: Option<Instant>,
    /// User-selected tiling preset. Automatic mode remains the default and
    /// picks a Setpiece-style preset from the number of restored windows.
    workspace_layout_preference: Option<evo_pods::layout::LayoutId>,
    /// Where the Continue action is in its lifecycle for the open body of
    /// work. User-authored transient: entering `Preparing` is the user
    /// committing to restore, so it is cleared by navigation like every other
    /// transient and never by a reload. The shell reads it in `logic` to run
    /// the restoration after `CONTINUE_DWELL`, which keeps the blocking call
    /// out of the click handler and the render pass.
    continue_phase: detail::ContinuePhase,
    /// The honest result of the user's last designation action, together with
    /// whether the daemon accepted it — so a rejection is never presented as
    /// a success.
    designation_status: Option<(String, bool)>,
    /// The user's in-progress edit of the current continuation surface: the
    /// witnessed subjects they have marked. `None` means they have not edited
    /// the declared surface, so the marks reflect the derived current surface.
    /// The edit becomes canonical only through an explicit declaration
    /// (RFC-0013) — it is never canonical while it is being made.
    continuation_draft: Option<BTreeSet<String>>,
    /// The honest result of the user's last continuation-surface declaration,
    /// as reported by the daemon, together with whether it was accepted.
    continuation_status: Option<(String, bool)>,
    /// The honest result of the user's last related-work declaration
    /// (RFC-0012 WorkGrouped), as reported by the daemon, together with
    /// whether it was accepted.
    grouping_status: Option<(String, bool)>,
    /// Whether the rack's stack row is expanded. Transient presentation
    /// only (DESIGN.md: the stack is one click away, visibly).
    stack_open: bool,
    /// The merge proposal currently armed for confirmation, if any. The
    /// desktop's own arming step: a first press only arms; Confirm alone
    /// declares. Disarmed by Decline, a click elsewhere, or a proposal list
    /// that shrank (rack.rs owns those rules; this only carries the state).
    merge_armed: Option<usize>,
    /// Explicit, user-selected work IDs. Co-sitting ledger proposals are not
    /// evidence that two intentions are the same.
    group_source: Option<u64>,
    group_target: Option<u64>,
    group_confirm: bool,
    /// The work currently being renamed (RFC-0014): the engine thread id of
    /// the card whose Rename… control was pressed, plus the draft text.
    /// Transient presentation; submitted through the daemon's canonical
    /// naming channel and cleared on submit or cancel.
    rename_thread: Option<u64>,
    rename_draft: String,
    /// The rack's search text, owned by the shell like the stack's open
    /// state. A non-empty query narrows the deck to matching works (the
    /// person's own names first, RFC-0014 read-back).
    search_query: String,
    /// The new-engine view: work threads derived by `evo-threads` from the
    /// same observation log the legacy Engagement pipeline reads, with each
    /// thread's resume bundle. Presentation-only; rebuilt on reload.
    engine_threads: Vec<evo_daemon::threads::DisplayThread>,
    /// The pod host, shared with the menu-bar presence: containment
    /// (hide/park/raise) and the active pod. Pod commands execute on the main
    /// thread from wherever they come — the desktop never waits for this
    /// window to paint.
    pods: crate::pods::SharedPods,
    /// The global hotkey + menu bar controller (keeps its allocations
    /// alive).
    pod_presence: Option<crate::presence::PodPresence>,
    /// The engine's current merge proposals: (subject_a, subject_b,
    /// name_a, name_b, shared sittings). Deterministic co-sitting between
    /// established works, offered to the person — confirming one declares
    /// the pair same-work through the daemon.
    /// The honest account of the last restore, shown with the threads it
    /// came from: what opened, what was raised, what could not be opened.
    restore_note: Option<String>,
    /// Effect-free scoped picker plus one daemon RESOURCE worker. Never enters pods.
    scoped_restore: crate::scoped_restore::ScopedRestore,
    /// Receiver for background engine computation results.
    /// The engine rebuilds on a dedicated thread; results arrive here.
    engine_receiver:
        Option<std::sync::mpsc::Receiver<Result<Vec<evo_daemon::threads::DisplayThread>, String>>>,
    engine_error: Option<String>,
    /// Whether an engine-threads rebuild is currently running on the worker
    /// thread. One rebuild at a time; cleared when its result is drained.
    engine_rebuild_in_flight: bool,
    /// A pending capture-daemon restart, requested by the capture-partial
    /// banner's "Check again" action. Consumed on the next `logic` pass —
    /// the same deferred-execution pattern as `continue_phase`, because the
    /// banner closure cannot also borrow `self.daemon`.
    daemon_restart_requested: bool,
    /// The first-run gate: `Some(step)` (1–3) while it is in progress, `None`
    /// once the user has entered the product. Persisted as a marker file
    /// outside canonical storage, so Evo never writes a presentation fact into
    /// its evidence log.
    first_run: Option<u8>,
    /// Whether the Settings sheet is open.
    settings_open: bool,

    /// The honest reason canonical storage could not be read, when it could
    /// not be read.
    status: Option<String>,
    last_reload: Instant,
    last_signature: Option<(String, usize)>,
    /// Voice events from the mic/wake/transcription thread ("Hey Evo").
    /// `None` when the voice-audio feature is off or the thread failed to
    /// start. The thread exits when this receiver is dropped.
    voice_events: Option<std::sync::mpsc::Receiver<evo_voice::VoiceEvent>>,
    /// The floating ambiguity panel: shown when a voice trigger resolves
    /// to multiple similar candidates. Numbered rows; the user picks by
    /// voice ordinal, click, or number key.
    voice_panel: crate::voice::AmbiguityPanel,
    /// A transient voice outcome message ("Resumed …", "Couldn't find …")
    /// with the time it was set, so it fades after a few seconds.
    voice_toast: Option<(String, Instant)>,
    /// The notch surface's action channel: drags and chip clicks.
    notch_actions: Option<mpsc::Receiver<crate::notch_window::SurfaceAction>>,
}

/// Locks the shared pod host from a field reference — a free
/// function so the render closure (which destructures `self` and may only
/// capture fields, never the whole) can reach it too.
/// Stable pod identity: the engine-lineage ThreadId. The same work keeps
/// the same pod id, accent color, and Add-to-Pod claims across restarts,
/// because the engine mints ids by replay order over the append-only log.
/// (The former name-hash identity died with the read-path unification: a
/// renamed work is still the same work.)
fn pod_id_of(thread_id: u64) -> u64 {
    thread_id
}

fn lock_pods(pods: &crate::pods::SharedPods) -> std::cell::RefMut<'_, crate::pods::PodHost> {
    pods.borrow_mut()
}

/// The rename session (RFC-0014 WorkNamed): the person's own word for a
/// body of work, collected against one armed card and sent through the
/// daemon's canonical naming channel. Nothing canonical is written here —
/// the daemon validates, persists, and re-derives; the shell only reports
/// the honest outcome.
fn rename_control_panel(
    ui: &mut egui::Ui,
    works: &[evo_daemon::threads::DisplayThread],
    root: &Path,
    armed: &mut Option<u64>,
    draft: &mut String,
    note: &mut Option<String>,
    ctx: &egui::Context,
) {
    let Some(thread_id) = *armed else {
        return;
    };
    let Some(work) = works.iter().find(|w| w.thread_id == thread_id) else {
        // The armed work faded from Home (rebuild, decay): close honestly.
        *armed = None;
        draft.clear();
        return;
    };

    ui::gap(ui, theme::S4);
    ui.collapsing(format!("Rename “{}”", work.name), |ui| {
        ui::caption(
            ui,
            "Give this work the name you'd use to ask for it again. Evo uses your word, not a guess.",
        );
        ui::gap(ui, theme::S2);

        let field = ui.add(
            egui::TextEdit::singleline(draft)
                .id_salt(("evo-rename-field", thread_id))
                .hint_text(theme::text(
                    theme::Role::Body,
                    "What do you call this work?",
                    theme::MUTED,
                ))
                .desired_width(ui.available_width().min(360.0)),
        );
        field.request_focus();

        let confirmed = ui
            .horizontal(|ui| {
                let save = ui::control(ui, Control::primary("Save name"));
                let cancel = ui::control(ui, Control::quiet("Cancel"));
                save.clicked() || cancel.clicked()
            })
            .inner;
        let save = field.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter));

        let submit_now = (confirmed && !save) || save;
        if submit_now && !draft.trim().is_empty() {
            // The subject the person is naming: the work's strongest witnessed
            // member — never a download or an application identity, which are
            // products and containers, not the work itself (the same rule the
            // grouping correction applies). The daemon validates the
            // declaration's shape at the boundary; the person's word needs no
            // witnessing to be spoken (Law IX).
            let subject = work
                .member_subjects
                .iter()
                .find(|s| {
                    !s.trim().is_empty()
                        && !s.starts_with("Downloaded:")
                        && !s.starts_with("app:")
                })
                .or_else(|| work.member_subjects.first())
                .cloned()
                .unwrap_or_else(|| work.bundle.resume_point.clone());
            match state::submit_naming(root, &subject, draft.trim()) {
                Ok(evo_daemon::naming::NamingResponse::Accepted) => {
                    *note = Some("Name sent to capture; it appears on the next refresh.".into());
                    *armed = None;
                    draft.clear();
                    // Ask for a fresh rebuild so the new name shows up.
                    ctx.request_repaint();
                }
                Ok(evo_daemon::naming::NamingResponse::Rejected(reason)) => {
                    *note = Some(format!("Name declined: {reason}"));
                }
                Err(err) => {
                    *note = Some(format!("Naming failed: {err}"));
                }
            }
        } else if confirmed && draft.trim().is_empty() {
            *note = Some("A blank name says nothing; nothing was changed.".into());
            *armed = None;
            draft.clear();
        }
    });
    // Pressing Escape anywhere closes the session without sending.
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        *armed = None;
        draft.clear();
    }
}

/// An explicit correction inside Home's scrollable surface, not an inferred
/// ledger proposal. The final press replays canonical evidence before sending
/// anything to the daemon; failed or stale reads never create a declaration.
fn manual_grouping_control(
    ui: &mut egui::Ui,
    works: &[evo_daemon::threads::DisplayThread],
    root: &Path,
    source: &mut Option<u64>,
    target: &mut Option<u64>,
    armed: &mut bool,
    note: &mut Option<String>,
) {
    ui::gap(ui, theme::S4);
    ui.collapsing("Group two works manually", |ui| {
        ui::caption(ui, "If two cards describe one intention, choose both. Being open together is not enough evidence to group them.");
        let old_pair = (*source, *target);
        for (label, selected) in [("First work", &mut *source), ("Second work", &mut *target)] {
            egui::ComboBox::from_label(label)
                .selected_text(selected.and_then(|id| works.iter().find(|w| w.thread_id == id).map(|w| w.name.as_str())).unwrap_or("Choose work"))
                .show_ui(ui, |ui| {
                    for work in works {
                        ui.selectable_value(selected, Some(work.thread_id), &work.name);
                    }
                });
        }
        if old_pair != (*source, *target) {
            *armed = false;
        }
        let pair = source.zip(*target).and_then(|(a, b)| evo_daemon::threads::grouping_subjects(works, a, b));
        if pair.is_none() && source.is_some() && target.is_some() {
            ui::caption(ui, "These works need distinct witnessed resources before Evo can safely group them.");
        }
        if pair.is_some() && ui::control(ui, Control::secondary("Review grouping")).clicked() {
            *armed = true;
        }
        if *armed && pair.is_some() {
            ui::caption(ui, "This joins both works on the next refresh. It does not restore anything.");
            if ui::control(ui, Control::primary("Confirm grouping")).clicked() {
                *armed = false;
                let (a, b) = pair.expect("checked");
                let current = evo_daemon::threads::grouping_subjects_from_storage(root, source.expect("chosen"), target.expect("chosen"));
                *note = Some(match current {
                    Ok(Some((current_a, current_b))) if a == current_a && b == current_b => {
                        match evo_daemon::grouping::submit_grouping(root, &a, &b) {
                            Ok(evo_daemon::grouping::GroupingResponse::Accepted) => "Grouping forwarded to capture; check the next refresh.".into(),
                            Ok(evo_daemon::grouping::GroupingResponse::Rejected(reason)) => format!("Grouping declined: {reason}"),
                            Err(err) => format!("Grouping failed: {err}"),
                        }
                    }
                    Ok(_) => "Work changed or has a shared resource. Refresh and choose again; nothing was grouped.".into(),
                    Err(err) => format!("Cannot check current work evidence; nothing was grouped: {err}"),
                });
            }
            if ui::control(ui, Control::quiet("Cancel grouping")).clicked() {
                *armed = false;
            }
        }
    });
}

impl EvoApp {
    /// Constructs the shell, starts the daemon worker, and performs the first
    /// canonical load.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        // Canonical state lives in the persistent Application Support
        // location; a populated legacy temp root is migrated forward safely
        // at startup (BE-TRACE-0001 §3.4). A migration failure preserves the
        // legacy data untouched and surfaces honestly through the shell's
        // storage-error state below — it is never silently ignored.
        let storage_root = match evo_storage::prepare_storage_root() {
            Ok(root) => root,
            Err(err) => {
                eprintln!("EVO-DESKTOP storage-root preparation failed: {err}");
                evo_storage::canonical_storage_root()
            }
        };
        // The first index build is the full replay of every storage log, and
        // the legacy engagement reconstruction that follows it is heavy. It
        // is deliberately NOT built eagerly here: doing so held the first
        // paint hostage to a full-history reconstruction. `reload` reads the
        // ledger (the engine process's Works) first and only falls back to
        // the legacy index when the ledger has nothing to show.
        let index = None;
        // The shell declares itself for the capture self-exclusion
        // boundary: a launchd-owned daemon reads this pid file (the env
        // route only exists when the shell itself spawned the daemon).
        let _ = std::fs::write(
            storage_root.join("desktop.pid"),
            std::process::id().to_string(),
        );
        // The menu-bar presence and global hotkey push pod commands into
        // the app; the receiver is stored in the struct (the initial
        // placeholder receiver is replaced here). The egui context rides
        // along so every command also wakes the frame loop — a command
        // The pod host is shared with the menu-bar presence so pod
        // commands execute on the main thread from the action itself —
        // the desktop never waits for this window to paint (a hidden
        // window paints nothing).
        let pods: crate::pods::SharedPods =
            std::rc::Rc::new(std::cell::RefCell::new(crate::pods::PodHost::new()));
        // Add-to-Pod persistence: the addon ledger lives with the rest of
        // the storage root; what a person teaches a pod survives restarts
        // and engine refreshes alike.
        pods.borrow_mut()
            .set_addons_path(storage_root.join("pod-addons.tsv"));
        // The old Pod Bar/presence surface is retired. Work launches stay
        // native to macOS, without registering a second workspace switcher
        // or global pod hotkey layer.
        let pod_presence = None;
        let workspace_layout_preference =
            std::fs::read_to_string(storage_root.join("workspace-layout.txt"))
                .ok()
                .and_then(|key| evo_pods::layout::LayoutId::from_key(key.trim()));
        let mut app = EvoApp {
            storage_root: storage_root.clone(),
            index,
            workspaces: Vec::new(),
            display: None,
            outcome: None,
            outcomes: HashMap::new(),
            titles: HashMap::new(),
            standings: HashMap::new(),
            subjects: HashMap::new(),
            kinds: HashMap::new(),
            locators: HashMap::new(),
            designation: None,
            selection: None,
            preflight: Vec::new(),
            home_cards: Vec::new(),
            daemon: DaemonManager::new(storage_root.clone()),
            engine: crate::engine::EngineManager::new(storage_root.clone()),
            selected: None,
            opened: None,
            execution_result: None,
            workspace_before: None,
            workspace_arrangement_deadline: None,
            workspace_layout_preference,
            continue_phase: detail::ContinuePhase::Idle,
            designation_status: None,
            continuation_draft: None,
            continuation_status: None,
            grouping_status: None,
            stack_open: false,
            merge_armed: None,
            group_source: None,
            group_target: None,
            group_confirm: false,
            rename_thread: None,
            rename_draft: String::new(),
            search_query: String::new(),
            engine_threads: Vec::new(),
            pods,
            pod_presence,
            restore_note: None,
            scoped_restore: crate::scoped_restore::ScopedRestore::default(),
            engine_receiver: None,
            engine_error: None,
            engine_rebuild_in_flight: false,
            first_run: shell::first_run_pending(),
            settings_open: false,
            status: None,
            last_reload: Instant::now(),
            last_signature: None,
            // Set by the capture-partial banner's "Check again" and consumed
            // on the next logic pass: the closure that renders the banner
            // cannot also reach `self.daemon` (borrow conflict), so the
            // restart is deferred exactly like the Continue lifecycle.
            daemon_restart_requested: false,
            // The voice thread owns mic + wake + transcription; wired
            // right after the struct literal (it needs `&mut app` for
            // the notch action channel).
            voice_events: None,
            voice_panel: crate::voice::AmbiguityPanel::default(),
            voice_toast: None,
            notch_actions: None,
        };
        // The login agent is the sole microphone/notch owner. Opening the
        // desktop must not create a second listener or competing overlay, and
        // quitting it must not terminate the independent voice interaction.
        app.reload();
        app
    }

    /// Re-reads canonical state by refreshing the derived index (only the
    /// records appended since the last reload are parsed).
    fn reload(&mut self) {
        // ─── The engine path: Works from evo-threads (W2 unification) ──
        // Home's authority on what a work is: the work engine replayed over
        // the canonical observation log, through one call. Thread ids are
        // engine lineage — stable across restarts — and every surface (rack
        // cards, pods, menu bar, merge vocabulary) now speaks them.
        //
        // A full log replay is heavier than the ledger's indexed SELECT, so
        // it runs on a worker thread and lands through `engine_receiver`;
        // the previous list keeps rendering until the fresh one arrives.
        // The ledger stays as ingestion + FTS5 search + merge-proposal
        // shadow (its hypothesis layer is scored by the thread gate until
        // parity retires it); it no longer names Home's work.
        let ledger_path = self.storage_root.join("ledger.db");
        let mut ledger_has_works = false;
        if ledger_path.exists() && evo_ledger::Ledger::open_readonly(&ledger_path).is_ok() {
            ledger_has_works = true;
            // The ledger's co-sitting candidates describe temporal overlap,
            // not a relationship between the engine works Home displays.
            // Never surface them as a declaration to this second authority.
        }

        // Refresh the engine threads off the UI thread.
        if !self.engine_rebuild_in_flight {
            self.engine_rebuild_in_flight = true;
            let (sender, receiver) = std::sync::mpsc::channel();
            let rebuild_root = self.storage_root.clone();
            self.engine_receiver = Some(receiver);
            if let Err(err) = std::thread::Builder::new()
                .name("evo-engine-threads".into())
                .spawn(move || {
                    let threads = evo_daemon::threads::display_threads_from_storage(&rebuild_root)
                        .map_err(|err| err.to_string());
                    let _ = sender.send(threads);
                })
            {
                self.engine_rebuild_in_flight = false;
                self.engine_error = Some(format!("Cannot start work-list refresh: {err}"));
            }
        }

        // ─── The legacy path: engagement reconstruction ────────────────
        // The legacy pipeline re-derives its affinity understanding from the
        // full observation history on this thread. With weeks of evidence a
        // single reconstruction outlasts the reload period — the UI thread
        // would never paint. The engine path above is Home's authority, so
        // the legacy reconstruction only runs when the engine returned
        // nothing (first run, empty log) and the ledger confirms nothing
        // either.
        if ledger_has_works || !self.engine_threads.is_empty() {
            self.last_reload = Instant::now();
            return;
        }

        let refreshed: Result<DisplayState, String> = match &mut self.index {
            Some(index) => index
                .refresh()
                .map_err(|err| err.to_string())
                .map(|()| state::display_state_from_index(index)),
            None => {
                // The index could not be built; retry the full replay so a
                // transient read failure does not pin the shell to an error
                // forever.
                match CanonicalIndex::new(&self.storage_root) {
                    Ok(index) => {
                        let built = state::display_state_from_index(&index);
                        self.index = Some(index);
                        Ok(built)
                    }
                    Err(err) => Err(err.to_string()),
                }
            }
        };
        match refreshed {
            Ok(DisplayState {
                workspaces,
                outcomes,
                titles,
                standings,
                subjects,
                kinds,
                locators,
                designation,
                ..
            }) => {
                self.status = None;
                self.workspaces = workspaces;
                self.outcomes = outcomes;
                self.titles = titles;
                self.standings = standings;
                self.subjects = subjects;
                self.kinds = kinds;
                self.locators = locators;
                self.designation = designation;
                // A selection whose Workspace is no longer remembered falls
                // back to the list honestly rather than showing an empty
                // detail screen for work Evo cannot find.
                if let Some(id) = &self.selected
                    && !self.workspaces.iter().any(|workspace| workspace.id() == id)
                {
                    self.selected = None;
                }
                self.derive_display_state();

                // The per-Artifact (schema, subject) resource evidence the
                // selective-restoration derivation reads. Empty when the index
                // could not be built, which honestly yields no reopenable
                // resources rather than a guess.
                let no_evidence: HashMap<String, (String, String)> = HashMap::new();
                let evidence = self
                    .index
                    .as_ref()
                    .map(CanonicalIndex::locators)
                    .unwrap_or(&no_evidence);
                self.home_cards = self
                    .workspaces
                    .iter()
                    .map(|workspace| {
                        let outcome = self.outcomes.get(&workspace.id().to_string());
                        let selection = outcome
                            .map(|outcome| state::selection_for(workspace, outcome, evidence));
                        state::workspace_card(
                            workspace,
                            outcome,
                            selection.as_ref(),
                            &self.subjects,
                            &self.kinds,
                            &self.titles,
                            &self.standings,
                        )
                    })
                    .collect();
            }
            Err(err) => {
                if self.status.as_deref() != Some(err.as_str()) {
                    self.status = Some(err);
                    if let Some((_, signature)) = &self.last_signature {
                        eprintln!("EVO-DESKTOP storage read failed after workspace {signature}");
                    }
                }
            }
        }
        self.last_reload = Instant::now();
    }

    /// Builds the containment pod for one engine display thread — the work
    /// wearing its pod face, keyed by its engine-lineage ThreadId. Evidence
    /// comes from the work's own witnessed material: the anchors and
    /// companions the engine measured, split structurally into URLs,
    /// documents, and window subjects. The application list is a separate
    /// provenance join from the same observations: a subject that is also a
    /// member title is a window, never an app to launch (the B1 lesson, held
    /// at this boundary too).
    fn pod_for_thread(
        work: &evo_daemon::threads::DisplayThread,
        witnessed_apps: &[String],
        resource_apps: &std::collections::BTreeMap<String, String>,
    ) -> evo_pods::pod::Pod {
        let mut urls: Vec<String> = Vec::new();
        let mut documents: Vec<String> = Vec::new();
        let mut titles: Vec<String> = Vec::new();
        let mut member_subjects: Vec<String> = Vec::new();
        let mut push_subject = |subject: &str| {
            if subject.is_empty()
                || subject.starts_with("Downloaded:")
                || member_subjects.contains(&subject.to_string())
            {
                return;
            }
            member_subjects.push(subject.to_string());
            if subject.starts_with("http://") || subject.starts_with("https://") {
                urls.push(subject.to_string());
            } else if subject.starts_with('/') || subject.starts_with("file://") {
                documents.push(subject.to_string());
            } else {
                titles.push(subject.trim().to_string());
            }
        };
        for entry in &work.understanding.trail {
            push_subject(&entry.raw);
        }
        for subject in &work.member_subjects {
            push_subject(subject);
        }
        push_subject(&work.bundle.resume_point);
        // Restore order lives in the bundle the pod carries (resume point,
        // then the engine's restore set); the surfaces above are the
        // containment evidence the runtime matches live windows against.

        let pod_id = pod_id_of(work.thread_id);
        evo_pods::pod::Pod {
            id: evo_pods::pod::PodId(pod_id),
            resources: vec![],
            claims: vec![],
            name: work.name.clone(),
            color: evo_pods::pod::PodColor::from_thread(pod_id),
            reason: work.bundle.resume_reason.clone(),
            engagement: work.engagement,
            bundle: work.bundle.clone(),
            surfaces: evo_pods::pod::PodSurfaces {
                urls,
                documents,
                titles,
                apps: witnessed_apps.to_vec(),
                resource_apps: resource_apps.clone(),
            },
            badges: evo_pods::pod::badges_from_bundle(&work.bundle),
            member_subjects,
            companions: vec![],
            strongest_anchor: Some(work.bundle.resume_point.clone()).filter(|r| !r.is_empty()),
            contested: vec![],
            state: evo_pods::pod::PodState::Inactive,
        }
    }

    /// Applies one batch of engine display threads: Home's list, and the
    /// pods rebuilt from the same threads so rack index i and pod index i
    /// are the same work by construction (they are built from one Vec in
    /// one order — the old parallel-derivation drift is structurally gone).
    fn apply_engine_threads(&mut self, threads: Vec<evo_daemon::threads::DisplayThread>) {
        // App names are provenance attached to focused-window observations.
        // Join them to each thread's exact witnessed resources before the pod
        // opens anything; this is what lets a work containing Terminal,
        // Safari, and an editor reopen all three through macOS while keeping
        // the pod's spatial arrangement separate from canonical identity.
        let (apps_by_work, resource_apps_by_work) = load_persisted_observations(&self.storage_root)
            .map(|observations| {
                (
                    app_names_for_threads(&observations, &threads),
                    resource_apps_for_threads(&observations, &threads),
                )
            })
            .unwrap_or_default();
        let empty_resource_apps = std::collections::BTreeMap::new();
        let pods: Vec<evo_pods::pod::Pod> = threads
            .iter()
            .map(|thread| {
                Self::pod_for_thread(
                    thread,
                    apps_by_work
                        .get(&thread.thread_id)
                        .map(|apps| apps.as_slice())
                        .unwrap_or(&[]),
                    resource_apps_by_work
                        .get(&thread.thread_id)
                        .unwrap_or(&empty_resource_apps),
                )
            })
            .collect();
        self.engine_threads = threads;
        self.scoped_restore.state.sync(&self.engine_threads);
        lock_pods(&self.pods).set_pods(pods);
    }

    /// Derives the presented Workspace's detail state — the displayed
    /// Workspace, its persisted derived outcome, its derived selective
    /// restoration selection, and its Executor Preflight dispositions — from
    /// the user's selection and the derived index.
    ///
    /// Runs at every reload and also whenever the user opens a body of work,
    /// so navigation responds on the frame it happens rather than waiting for
    /// the next canonical reload. Everything produced here is *derived*; the
    /// user's own transient state is deliberately untouched.
    fn derive_display_state(&mut self) {
        let display = self
            .selected
            .as_ref()
            .and_then(|id| {
                self.workspaces
                    .iter()
                    .find(|workspace| workspace.id() == id)
            })
            .cloned();

        // The canonical boundary the shell consumes, logged when it moves. It
        // is diagnostic only: nothing the user authored is cleared by it.
        let signature = match display.as_ref() {
            Some(workspace) => state_signature(Some(workspace)),
            None if self.workspaces.is_empty() => None,
            None => Some(("home".to_string(), self.workspaces.len())),
        };
        if signature != self.last_signature {
            self.last_signature = signature;
            log_state_change(&self.storage_root, display.as_ref());
        }

        self.outcome = display
            .as_ref()
            .and_then(|workspace| self.outcomes.get(&workspace.id().to_string()).cloned());
        self.display = display;

        // The derived selective-restoration selection for the displayed
        // Workspace: what Evo would restore from the current continuation,
        // what is honestly unavailable, and what remains historical.
        self.selection = match (self.display.as_ref(), self.outcome.as_ref()) {
            (Some(workspace), Some(outcome)) => self
                .index
                .as_ref()
                .map(|index| state::selection_for(workspace, outcome, index.locators())),
            _ => None,
        };
        // Clear preflight when no workspace is selected to avoid stale data.
        if self.selection.is_none() {
            self.preflight = Vec::new();
        }
        // NOTE: Preflight is NOT computed here. Computing it on every reload
        // (every 2 seconds) triggers a full Accessibility API sweep that
        // blocks the main thread. Preflight is computed separately on
        // workspace navigation and before execution.
    }

    /// Computes Executor Preflight dispositions for the current selection.
    ///
    /// This triggers Accessibility API sweeps and must NOT run on every
    /// reload. It runs only on workspace navigation and before execution.
    fn compute_preflight(&mut self) {
        self.preflight = match &self.selection {
            Some(selection) => preflight_selection(selection, &evo_execution::MacOSExecutor::new()),
            None => Vec::new(),
        };
    }

    /// Resets the transient state that belonged to the body of work the user
    /// just navigated away from.
    ///
    /// This is the *only* place any of it is cleared without the user asking.
    /// It is keyed by canonical Workspace identity, so it fires on navigation
    /// and never on a reload — a continuation draft or an execution report is
    /// about one body of work, and stops being meaningful when you leave it.
    fn follow_navigation(&mut self) {
        if self.opened.as_ref() == self.selected.as_ref() {
            return;
        }
        self.opened = self.selected.clone();
        self.execution_result = None;
        self.continue_phase = detail::ContinuePhase::Idle;
        self.continuation_draft = None;
        self.continuation_status = None;
        self.grouping_status = None;
        self.designation_status = None;
    }
}

fn state_signature(display: Option<&Workspace>) -> Option<(String, usize)> {
    display.map(|workspace| (workspace.id().to_string(), workspace.snapshots().len()))
}

fn log_state_change(storage_root: &Path, display: Option<&Workspace>) {
    match display {
        Some(workspace) => eprintln!(
            "EVO-DESKTOP storage={} workspace={} snapshots={}",
            storage_root.display(),
            workspace.id(),
            workspace.snapshots().len()
        ),
        None => eprintln!(
            "EVO-DESKTOP storage={} workspace=none",
            storage_root.display()
        ),
    }
}

impl EvoApp {
    /// Non-UI logic: re-check the daemon worker, re-read canonical storage,
    /// and keep the surface alive.
    fn logic_impl(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The window never dies: closing it hides Evo instead. A menu-bar
        // app whose loop can lose its window is a menu-bar app whose
        // commands stop being processed; hiding keeps the pod presence
        // (menu bar, hotkey, containment) alive; "Open Evo" restores the
        // window.
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        // While the Pod Bar is open, egui's own input-driven repaints
        // keep it alive (pointer motion and presses each request a frame);
        // a continuous repaint loop here once cost 60 fps of churn.
        if std::mem::take(&mut self.daemon_restart_requested) {
            self.daemon.restart();
        }
        self.daemon.poll();
        self.engine.poll();

        // Pod commands from the menu bar and hotkey execute directly on
        // the main thread inside the action itself (see presence.rs) —
        // this window may be hidden and painting nothing. What arrives
        // here is the note they left, shown at the next opportunity.
        {
            let mut pods_guard = lock_pods(&self.pods);
            if let Some(note) = pods_guard.pending_note.take() {
                self.restore_note = Some(note);
            }
            let active = pods_guard.active.clone();
            let names = pods_guard.names();
            drop(pods_guard);
            // Keep the menu bar in sync with the pod list.
            if let Some(presence) = &mut self.pod_presence {
                presence.update_pods(&names, active);
            }
        }
        if self.last_reload.elapsed() >= RELOAD_PERIOD {
            self.reload();
        }

        // The Continue action's execution steps. They live here, after the
        // click that set `Preparing` has already returned and the transition
        // has had a frame to paint, precisely because each step is synchronous
        // and blocks this thread: running the whole plan in the click handler
        // would freeze the very frame meant to show "Preparing".
        //
        // Exactly one step runs per frame. That is what makes the progress the
        // user sees honest rather than decorative: a row is marked because
        // `execute_step` returned a result for it and the frame after it
        // painted, never because a plan predicted it. A future asynchronous or
        // remote executor replaces only this block — the click handler, the
        // transition, and the report all stay exactly as they are.
        match std::mem::take(&mut self.continue_phase) {
            detail::ContinuePhase::Idle => {}
            detail::ContinuePhase::Preparing { since } => {
                if self.selected != self.opened {
                    // The user navigated away between committing and the dwell
                    // elapsing. Cancel: never reopen work someone just left.
                    // The `take` above already left this Idle. The transient
                    // reset in `follow_navigation` also clears it, but that
                    // runs in the render pass, so the race is guarded here too.
                } else if since.elapsed() >= CONTINUE_DWELL {
                    // Decide the plan — no OS action yet. Deciding and acting
                    // are separate so the surface can name every step before
                    // any of them has a result.
                    // Refresh preflight before execution so OS state is current.
                    self.compute_preflight();
                    let executor = evo_execution::MacOSExecutor::new();
                    let steps = state::plan_execution(
                        self.outcome.as_ref(),
                        self.selection.as_ref(),
                        &self.locators,
                        &executor,
                    );
                    self.continue_phase = detail::ContinuePhase::Executing {
                        steps,
                        done: Vec::new(),
                    };
                    ctx.request_repaint();
                } else {
                    // Keep frames coming so the transition animates and the
                    // dwell actually elapses, rather than waiting on the 2s
                    // reload tick.
                    self.continue_phase = detail::ContinuePhase::Preparing { since };
                    ctx.request_repaint();
                }
            }
            in_flight @ detail::ContinuePhase::Executing { .. } => {
                if self.selected != self.opened {
                    // The user navigated away while execution was in flight.
                    // Cancel: never continue restoring work someone just left.
                    // The `take` above already left this Idle. The transient
                    // reset in `follow_navigation` also clears it, but that
                    // runs in the render pass, so the race is guarded here too.
                } else {
                    self.continue_phase = in_flight;
                    let executor = evo_execution::MacOSExecutor::new();
                    match self.continue_phase.advance(&executor) {
                        // Finished: the report replaces the in-flight list, and Evo
                        // gets out of the way of the native applications.
                        Some(report) => {
                            self.execution_result = Some(report);
                            if self.workspace_before.is_some() {
                                self.workspace_arrangement_deadline =
                                    Some(Instant::now() + Duration::from_secs(2));
                            }
                            // Suppress workspace re-derivation for a short period after
                            // execution. The OS actions (opening URLs, raising windows)
                            // create new observations that could shift artifact
                            // assignments between workspaces if processed immediately.
                            if let Some(_index) = self.index.as_mut() {
                                // Execution provenance: observations preserved;
                                // reconstruction uses origin tag, not temporal suppression.
                            }
                        }
                        // A step just reported. Paint it before the next one blocks.
                        None => ctx.request_repaint(),
                    }
                }
            }
        }

        // Native apps create their windows asynchronously after NSWorkspace
        // accepts an open request. Poll briefly instead of treating the first
        // empty AX inventory as a successful (but invisible) arrangement.
        if let Some(deadline) = self.workspace_arrangement_deadline {
            if let Some(before) = self.workspace_before.as_ref() {
                let mut surface = crate::pod_surface::MacPodSurface::new();
                let arranged = crate::unified_workspace::arrange_new_windows_with_layout(
                    &mut surface,
                    before,
                    std::process::id() as i32,
                    self.workspace_layout_preference,
                );
                if arranged.is_some() || Instant::now() >= deadline {
                    if arranged.is_none() {
                        eprintln!("EVO workspace: no new native windows observed before timeout");
                    }
                    self.workspace_before = None;
                    self.workspace_arrangement_deadline = None;
                } else {
                    ctx.request_repaint_after(Duration::from_millis(50));
                }
            } else {
                self.workspace_arrangement_deadline = None;
            }
        }

        ctx.request_repaint_after(Duration::from_secs(2));

        // Voice events: "Hey Evo" triggers resolved by the daemon.
        self.poll_voice(ctx);
        self.scoped_restore.poll(ctx);
    }
}

impl EvoApp {
    /// Drains voice events from the mic thread and routes them.
    ///
    /// - `HeardCommand` → transient "heard" toast.
    /// - `Outcome(Restored)` → confirmation toast (the daemon already
    ///   restored; the desktop just reports it honestly).
    /// - `Outcome(Ambiguous)` → the numbered panel (shown by
    ///   `drain_voice_events`); the user picks by voice/click/key.
    /// - `Outcome(NotFound)` → honest "couldn't find it" with recents.
    /// - `Outcome(Failed)` → the honest error.
    fn poll_voice(&mut self, ctx: &egui::Context) {
        // The notch surface's actions first: chip clicks and drags are
        // user intent that must not wait on the voice stream.
        loop {
            let action = self
                .notch_actions
                .as_ref()
                .and_then(|actions| actions.try_recv().ok());
            match action {
                Some(crate::notch_window::SurfaceAction::Choose(work_id)) => {
                    self.choose_work(ctx, &work_id);
                }
                Some(crate::notch_window::SurfaceAction::ChooseCandidate(candidate)) => {
                    self.choose_candidate(ctx, &candidate);
                }
                Some(crate::notch_window::SurfaceAction::Dismiss) => {
                    self.voice_toast = None;
                }
                Some(crate::notch_window::SurfaceAction::OpenMain) => {
                    // Rack deep-link: nothing extra here yet; the
                    // window is already frontmost when clicked.
                }
                Some(crate::notch_window::SurfaceAction::Browse) => {
                    // The works carousel is the agent's surface: the
                    // headless evo-agent owns the WORKS round-trip and
                    // feeds the listing back to the notch. The desktop
                    // shell has no notch-owned browse to open.
                }
                Some(crate::notch_window::SurfaceAction::ContinueRecent) => {
                    // Same ownership: the agent resumes the daemon's
                    // most recent work; the desktop reports it through
                    // its own voice path when it owns the mic.
                }
                Some(crate::notch_window::SurfaceAction::ChoiceStep(_)) => {
                    // The surface thread steps its own engine and
                    // re-renders; nothing for the app to do.
                }
                None => break,
            }
        }
        let Some(receiver) = self.voice_events.as_ref() else {
            return;
        };
        let events = crate::voice::drain_voice_events(receiver, &mut self.voice_panel);
        for event in events {
            match event {
                evo_voice::VoiceEvent::WakeGesture { score } => {
                    // The notch surface consumes the same event stream;
                    // in the main window show an honest listening toast.
                    let detail = score
                        .map(|s| format!(" (score {s:.2})"))
                        .unwrap_or_default();
                    self.voice_toast = Some((format!("Listening…{detail}"), Instant::now()));
                    ctx.request_repaint();
                }
                evo_voice::VoiceEvent::Listening { partial } => {
                    let shown = partial.unwrap_or_default();
                    let listening_text = if shown.is_empty() {
                        "Listening… say a command".to_string()
                    } else {
                        format!("Listening: “{shown}”")
                    };
                    self.voice_toast = Some((listening_text, Instant::now()));
                    // Repaint now so partial transcripts animate live.
                    ctx.request_repaint_after(Duration::from_millis(250));
                }
                evo_voice::VoiceEvent::HeardCommand { command } => {
                    // While the ambiguity panel is visible, an ordinal
                    // ("the second one", "number 2") is a pick, not a
                    // new command.
                    if self.voice_panel.is_visible()
                        && let Some(index) = crate::voice::parse_ordinal_pick(&command)
                    {
                        self.voice_panel.pick(index);
                        ctx.request_repaint();
                        continue;
                    }
                    self.voice_toast = Some((format!("Heard: \"{command}\""), Instant::now()));
                    ctx.request_repaint();
                }
                evo_voice::VoiceEvent::Outcome(outcome) => match outcome {
                    evo_voice::VoiceOutcome::Restored {
                        title,
                        summary,
                        offered,
                    } => {
                        let extra = if offered.is_empty() {
                            String::new()
                        } else {
                            format!(" {} more offered", offered.len())
                        };
                        self.voice_toast = Some((
                            format!("Resumed \"{title}\" — {summary}{extra}"),
                            Instant::now(),
                        ));
                        ctx.request_repaint();
                    }
                    evo_voice::VoiceOutcome::Ambiguous { .. } => {
                        // The panel is already shown by drain_voice_events.
                        ctx.request_repaint();
                    }
                    evo_voice::VoiceOutcome::NotFound { recent } => {
                        let names: Vec<&str> =
                            recent.iter().map(|c| c.title.as_str()).take(3).collect();
                        let hint = if names.is_empty() {
                            String::new()
                        } else {
                            format!(" Recent: {}", names.join(", "))
                        };
                        self.voice_toast =
                            Some((format!("Couldn't find that work.{hint}"), Instant::now()));
                        ctx.request_repaint();
                    }
                    evo_voice::VoiceOutcome::Failed { reason } => {
                        self.voice_toast =
                            Some((format!("Voice restore failed: {reason}"), Instant::now()));
                        ctx.request_repaint();
                    }
                    evo_voice::VoiceOutcome::NoCommand => {}
                    // The engine-backed read path: the rack, spoken.
                    evo_voice::VoiceOutcome::Brief { count, narration } => {
                        let head = match count {
                            0 => "No works".to_string(),
                            1 => "One work".to_string(),
                            n => format!("{n} works"),
                        };
                        self.voice_toast = Some((format!("{head}: {narration}"), Instant::now()));
                        ctx.request_repaint();
                    }
                },
            }
        }
        // A pick from the panel (click or number key) sends CHOOSE.
        if let Some(candidate) = self.voice_panel.take_pick() {
            self.choose_candidate(ctx, &candidate);
        }
        // Fade the toast after a few seconds.
        if let Some((_, since)) = &self.voice_toast
            && since.elapsed() >= Duration::from_secs(5)
        {
            self.voice_toast = None;
        }
    }

    /// Restores one candidate work by id over the daemon's CHOOSE verb
    /// and toasts the honest outcome. Shared by the ambiguity panel's
    /// click/ordinal pick and the notch carousel's Restore chip.
    fn choose_work(&mut self, ctx: &egui::Context, work_id: &str) {
        let client = crate::voice::VoiceSocketClient::new(self.storage_root.clone());
        match client.choose(work_id) {
            Ok(evo_voice::VoiceOutcome::Restored {
                title,
                summary,
                offered,
            }) => {
                let extra = if offered.is_empty() {
                    String::new()
                } else {
                    format!(" {} more offered", offered.len())
                };
                self.voice_toast = Some((
                    format!("Resumed \"{title}\" — {summary}{extra}"),
                    Instant::now(),
                ));
            }
            Ok(other) => {
                self.voice_toast = Some((format!("Voice outcome: {other:?}"), Instant::now()));
            }
            Err(err) => {
                self.voice_toast = Some((format!("Voice choose failed: {err}"), Instant::now()));
            }
        }
        ctx.request_repaint();
    }

    fn choose_candidate(&mut self, ctx: &egui::Context, candidate: &evo_voice::VoiceCandidate) {
        let client = crate::voice::VoiceSocketClient::new(self.storage_root.clone());
        match client.choose_candidate(candidate) {
            Ok(evo_voice::VoiceOutcome::Restored {
                title,
                summary,
                offered,
            }) => {
                let extra = if offered.is_empty() {
                    String::new()
                } else {
                    format!(" {} more offered", offered.len())
                };
                self.voice_toast = Some((
                    format!("Resumed \"{title}\" — {summary}{extra}"),
                    Instant::now(),
                ));
            }
            Ok(other) => {
                self.voice_toast = Some((format!("Voice outcome: {other:?}"), Instant::now()));
            }
            Err(err) => {
                self.voice_toast = Some((format!("Voice choose failed: {err}"), Instant::now()));
            }
        }
        ctx.request_repaint();
    }

    /// Renders the shell.
    ///
    /// The primary surface is Home — "what bodies of work can I continue?".
    /// A detail screen for one body of work is entered only by the user's
    /// explicit selection, keyed by canonical Workspace identity. The
    /// contextual continuation panel lives inside that detail screen and is
    /// never the application's navigation.
    fn ui_impl(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let max = ui.max_rect();
        // The environment. A soft vertical band, not a flat page fill: it is
        // what the surfaces above it are lit against, and it is the only thing
        // in the window that is allowed to be a gradient.
        ui::paint_environment(ui.painter(), max);

        // The first-run gate owns the whole window. No chrome, because there
        // is nothing to navigate yet.
        if let Some(step) = self.first_run {
            shell::first_run(ui, step, &mut self.first_run, &mut self.daemon);
            return;
        }

        let daemon_status = self.daemon.poll();

        // The engine-threads worker's result, drained when it lands: Home's
        // list and the pod set are rebuilt together from one Vec, so rack
        // index i and pod index i stay the same work by construction. The
        // previous list keeps rendering until then.
        if self.engine_rebuild_in_flight
            && let Some(receiver) = &self.engine_receiver
        {
            match receiver.try_recv() {
                Ok(Ok(threads)) => {
                    self.engine_rebuild_in_flight = false;
                    self.engine_error = None;
                    self.apply_engine_threads(threads);
                }
                Ok(Err(err)) => {
                    self.engine_rebuild_in_flight = false;
                    self.engine_error = Some(format!("Cannot read recorded work: {err}"));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.engine_rebuild_in_flight = false;
                    self.engine_error = Some(
                        "Work-list refresh stopped unexpectedly; retry on the next refresh.".into(),
                    );
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }

        // Navigation is resolved before anything is drawn, so the frame the
        // user's click lands on already shows the work they opened rather than
        // waiting for the next canonical reload.
        if self.display.as_ref().map(Workspace::id) != self.selected.as_ref() {
            self.derive_display_state();
            // Compute preflight once on navigation, not on every reload.
            self.compute_preflight();
        }
        self.follow_navigation();

        let header_rect = egui::Rect::from_min_max(
            max.min,
            egui::pos2(max.max.x, max.min.y + theme::HEADER_HEIGHT),
        );
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(max.min.x, max.min.y + theme::HEADER_HEIGHT),
            max.max,
        );
        if shell::header(ui, header_rect, &daemon_status) {
            self.settings_open = true;
        }

        let view = state::ShellView::from_selection(self.selected.as_ref(), &self.workspaces);
        // Individual field bindings, so the canonical record can be read while
        // the user's own transient state is written. The split is the point:
        // what Evo knows is handed over immutably, what the user is in the
        // middle of is handed over mutably, and nothing in the first can be
        // written through.
        let Self {
            storage_root,
            display,
            outcome,
            subjects,
            kinds,
            titles,
            standings,
            locators,
            designation,
            selection,
            preflight,
            home_cards,
            daemon,
            selected,
            execution_result,
            continue_phase,
            designation_status,
            continuation_draft,
            continuation_status,
            grouping_status,
            status,
            settings_open,
            workspace_before,
            workspace_arrangement_deadline,
            ..
        } = self;

        // The transport dock's Continue gate, written from inside the screen
        // scope below (the same shared plan the stage announces — one
        // definition, two readers). False on Home, where there is no plan.
        let mut continuable = false;

        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(body_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                // What Evo can and cannot currently witness sits above the
                // scrolling work and does not scroll: a statement about Evo's
                // own ability must not be able to scroll away from the list
                // whose completeness it qualifies.
                if !matches!(daemon_status, DaemonStatus::Capturing) {
                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(theme::M6, theme::M2))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            shell::capture_notice(ui, &daemon_status, daemon);
                        });
                }

                match view {
                    state::ShellView::Home => {
                        if let Some(note) = self.restore_note.as_deref() {
                            ui::caption(ui, note);
                            ui::gap(ui, theme::S3);
                        }
                        if let Some(err) = self.engine_error.as_deref() {
                            // Do not offer stale cards as though a failed
                            // canonical refresh had confirmed them.
                            shell::storage_error(ui, err);
                            return;
                        }
                        // The rack is the primary surface: up to three
                        // track cards, the stack behind them, and the way
                        // into a work's pod.
                        let rack_request = ui::scroll(ui, |ui| {
                            let request = rack::rack(
                                ui,
                                &self.engine_threads,
                                &[],
                                &mut self.stack_open,
                                &mut self.merge_armed,
                                &mut self.search_query,
                            );
                            if self.engine_threads.len() >= 2 {
                                manual_grouping_control(
                                    ui,
                                    &self.engine_threads,
                                    storage_root,
                                    &mut self.group_source,
                                    &mut self.group_target,
                                    &mut self.group_confirm,
                                    &mut self.restore_note,
                                );
                            }
                            let panel_ctx = ui.ctx().clone();
                            rename_control_panel(
                                ui,
                                &self.engine_threads,
                                storage_root,
                                &mut self.rename_thread,
                                &mut self.rename_draft,
                                &mut self.restore_note,
                                &panel_ctx,
                            );
                            request
                        });
                        match rack_request {
                            None => {}
                            Some(rack::RackAction::RestoreSelected(thread_id)) => {
                                self.scoped_restore.state.open_for(thread_id, &self.engine_threads);
                            }
                            Some(rack::RackAction::Rename(thread_id)) => {
                                // Arm the rename session against the chosen
                                // work. The field renders beside the rack;
                                // nothing is sent until the person confirms.
                                self.rename_thread = Some(thread_id);
                                self.rename_draft = self
                                    .engine_threads
                                    .iter()
                                    .find(|w| w.thread_id == thread_id)
                                    .map(|w| w.name.clone())
                                    .unwrap_or_default();
                            }
                            Some(rack::RackAction::NotWork(thread_id)) => {
                                // The person's own class for the work's strongest
                                // witnessed member: this surface is not work.
                                // The daemon validates and persists it; the
                                // engine then stops founding or naming a work
                                // by it — retroactively, over the whole log —
                                // and deletes nothing. The subject rule is the
                                // one the rename session uses: the strongest
                                // witnessed member, never a download or an
                                // application identity.
                                let subject = self
                                    .engine_threads
                                    .iter()
                                    .find(|w| w.thread_id == thread_id)
                                    .and_then(|work| {
                                        work.member_subjects
                                            .iter()
                                            .find(|s| {
                                                !s.trim().is_empty()
                                                    && !s.starts_with("Downloaded:")
                                                    && !s.starts_with("app:")
                                            })
                                            .or_else(|| work.member_subjects.first())
                                            .cloned()
                                            .or_else(|| {
                                                let point = work.bundle.resume_point.trim();
                                                (!point.is_empty()).then(|| point.to_string())
                                            })
                                    });
                                self.restore_note = Some(match subject {
                                    None => "This work has no witnessed surface to classify yet; nothing was sent.".into(),
                                    Some(subject) => {
                                        match state::submit_classification(storage_root, &subject, "NotWork") {
                                            Ok(evo_daemon::classification::ClassificationResponse::Accepted) => "Classification sent to capture; it applies on the next refresh.".into(),
                                            Ok(evo_daemon::classification::ClassificationResponse::Rejected(reason)) => format!("Classification declined: {reason}"),
                                            Err(err) => format!("Classification failed: {err}"),
                                        }
                                    }
                                });
                            }
                            Some(rack::RackAction::Dismiss(thread_id)) => {
                                // Dismissal is a presentation correction for a
                                // witnessed work subject. It does not classify
                                // the subject, delete history, or write a
                                // derived thread id into the observation log.
                                let subject = self
                                    .engine_threads
                                    .iter()
                                    .find(|w| w.thread_id == thread_id)
                                    .and_then(|work| {
                                        work.member_subjects
                                            .iter()
                                            .find(|s| {
                                                !s.trim().is_empty()
                                                    && !s.starts_with("Downloaded:")
                                                    && !s.starts_with("app:")
                                            })
                                            .or_else(|| work.member_subjects.first())
                                            .cloned()
                                            .or_else(|| {
                                                let point = work.bundle.resume_point.trim();
                                                (!point.is_empty()).then(|| point.to_string())
                                            })
                                    });
                                self.restore_note = Some(match subject {
                                    None => "This work has no witnessed surface to dismiss; nothing was sent.".into(),
                                    Some(subject) => match state::submit_dismissal(storage_root, &subject) {
                                        Ok(evo_daemon::feedback::FeedbackResponse::Accepted) => "Work dismissed from Home; its evidence remains retrievable.".into(),
                                        Ok(evo_daemon::feedback::FeedbackResponse::Rejected(reason)) => format!("Dismissal declined: {reason}"),
                                        Err(err) => format!("Dismissal failed: {err}"),
                                    },
                                });
                            }
                            Some(rack::RackAction::Merge(_)) => {} // No inferred proposals on Home.
                            Some(rack::RackAction::Continue(index)) => {
                                // Continuing a work IS entering its pod:
                                // containment hides everything not of this
                                // work, restoration opens every artifact
                                // (files through their witnessed apps, URLs
                                // through the normal macOS handler), and the
                                // pod
                                // report says what happened. One path, one
                                // note; the menu-bar Enter does the same.
                                let mut surface = crate::pod_surface::MacPodSurface::new();
                                *workspace_before = Some(crate::unified_workspace::snapshot(
                                    &mut surface,
                                    std::process::id() as i32,
                                ));
                                if let Some(note) =
                                    lock_pods(&self.pods).enter_at(index, crate::pods::epoch_ms())
                                {
                                    self.restore_note = Some(note);
                                }
                                *workspace_arrangement_deadline =
                                    Some(Instant::now() + Duration::from_secs(2));
                            }
                            Some(rack::RackAction::Open(index)) => {
                                // Opening a card enters its pod: containment,
                                // restoration, and the pod report — the same
                                // path the menu bar takes.
                                let mut surface = crate::pod_surface::MacPodSurface::new();
                                *workspace_before = Some(crate::unified_workspace::snapshot(
                                    &mut surface,
                                    std::process::id() as i32,
                                ));
                                if let Some(note) =
                                    lock_pods(&self.pods).enter_at(index, crate::pods::epoch_ms())
                                {
                                    self.restore_note = Some(note);
                                }
                                *workspace_arrangement_deadline =
                                    Some(Instant::now() + Duration::from_secs(2));
                                // The centre stage: the same work opened as
                                // the canonical Workspace, so the detail
                                // screen's derivation, marks, and Continue
                                // lifecycle are all in play. The bridge is
                                // witnessed membership; when the record has
                                // not yet linked them, the work still enters
                                // (engine truth) and Evo says so.
                                if let Some(work) = self.engine_threads.get(index) {
                                    let bridged = state::workspace_for_thread(
                                        &work.member_subjects,
                                        &self.workspaces,
                                        subjects,
                                    );
                                    match bridged {
                                        Some(id) => {
                                            *selected = Some(id);
                                        }
                                        None => {
                                            self.restore_note =
                                                Some("Evo has not yet linked this work to its record; it is entered as a pod. The stage follows once the record connects them.".to_string());
                                        }
                                    }
                                }
                            }
                        }
                        // The rack is the primary surface; with no engine
                        // work the honest states below explain why it is
                        // empty.
                        if self.engine_threads.is_empty() {
                            if let Some(status) = self.engine_error.as_deref().or(status.as_deref()) {
                                shell::storage_error(ui, status);
                                return;
                            }
                            // With no evidence at all, the reason there is no
                            // evidence is the whole screen. A blocked permission
                            // gets the one action that can change it; otherwise
                            // Evo simply says it has nothing yet.
                            match &daemon_status {
                                DaemonStatus::PermissionRequired => {
                                    home::permission_required(ui, None, || {
                                        crate::daemon::open_accessibility_settings()
                                    });
                                }
                                DaemonStatus::PartialCapture { detail } => {
                                    // Degraded capture: file saves, URLs and commits
                                    // work, but window-focus needs Accessibility.
                                    // When no evidence exists yet, say so honestly
                                    // rather than repeating the permission prompt
                                    // (D-05: empty state for degraded must explain
                                    // the cause, not hide it).
                                    ui::raised(ui, |ui_inner| {
                                        ui_inner.set_width(ui_inner.available_width());
                                        ui::eyebrow(ui_inner, "Capture partial");
                                        ui::display(ui_inner, "File saves, URLs and commits are recorded. Window focus needs Accessibility.");
                                        ui::gap(ui_inner, theme::S2);
                                        ui::caption(ui_inner,
                                            &format!("{detail}\nGrant Accessibility in System Settings, then click Check again.",),
                                        );
                                        ui::gap(ui_inner, theme::S3);
                                        // The same one place as the header notice:
                                        // the restart is deferred through the flag
                                        // because this closure cannot also borrow
                                        // the daemon.
                                        shell::permission_actions(ui_inner, || {
                                            self.daemon_restart_requested = true;
                                        });
                                    });
                                }
                                other => {
                                    home::empty(ui, matches!(other, DaemonStatus::Capturing))
                                }
                            }
                        }
                    }
                    state::ShellView::Workspace(_) => {
                        let Some(workspace) = display.as_ref() else {
                            // Only reachable if the derived display state and
                            // the selection disagree, which would be a bug
                            // rather than a state to invent a screen for.
                            home::empty(ui, matches!(daemon_status, DaemonStatus::Capturing));
                            return;
                        };
                        // Written out as explicit shared reborrows rather than
                        // relying on `&mut T` coercing to `&T` at each field:
                        // the canonical half of this hand-off is read-only by
                        // design, and saying so here is what makes that
                        // guarantee visible at the boundary.
                        let canon = detail::Canonical {
                            workspace,
                            outcome: outcome.as_ref(),
                            selection: selection.as_ref(),
                            preflight: preflight.as_slice(),
                            subjects: &*subjects,
                            kinds: &*kinds,
                            titles: &*titles,
                            standings: &*standings,
                            locators: &*locators,
                            designation: designation.as_ref(),
                        };
                        let mut transient = detail::Transient {
                            storage_root: storage_root.as_path(),
                            selected,
                            designation_status,
                            continuation_draft,
                            continuation_status,
                            grouping_status,
                            execution_result,
                            continue_phase,
                        };
                        // The same work cards Home builds, handed through so
                        // the Place can offer a sideways step into another body
                        // of work without a return trip Home. `place_nav`
                        // filters them to work other than this one.
                        detail::screen(ui, &canon, home_cards.as_slice(), &mut transient);
                        // The dock's Continue gate is this screen's gate: one
                        // definition (the shared plan), two readers — the
                        // promise on the dock and the act in execution can
                        // never disagree (UI-VALIDATION-0001 B.9).
                        continuable = detail::continue_ready(&canon);
                    }
                }
            },
        );

        // The dock: the persistent control panel under every screen. It
        // rides above the content, below the settings sheet, and owns the
        // primary action with its live progress — the one place to look
        // for what Evo will do next (DESIGN.md: The dock).
        let dock_rect = egui::Rect::from_min_max(
            egui::pos2(max.min.x, max.max.y - theme::ACTION_BAR_HEIGHT),
            egui::pos2(max.max.x, max.max.y),
        );
        let dock_ctx = dock::DockContext {
            // Pod chrome was removed from the product surface. The dock now
            // exposes only native work continuation and library navigation.
            active_pod: None,
            in_focus: matches!(view, state::ShellView::Workspace(_)),
            // Set inside the screen scope from the same shared plan the
            // stage announces; false on Home, where there is no plan.
            continuable,
        };
        match dock::show(
            ui,
            dock_rect,
            &dock_ctx,
            &self.continue_phase,
            self.execution_result.as_ref(),
        ) {
            Some(dock::DockAction::Library) => {
                *selected = None;
            }
            Some(dock::DockAction::Continue) => {
                if matches!(self.continue_phase, detail::ContinuePhase::Idle) {
                    let mut surface = crate::pod_surface::MacPodSurface::new();
                    *workspace_before = Some(crate::unified_workspace::snapshot(
                        &mut surface,
                        std::process::id() as i32,
                    ));
                    // Commit now; `logic` runs the plan after the dwell so
                    // the transition paints first. One step per frame keeps
                    // the progress the dock shows honest.
                    self.continue_phase = detail::ContinuePhase::Preparing {
                        since: Instant::now(),
                    };
                }
            }
            Some(dock::DockAction::Eject) => {
                // Leaving restores the desktop through the same path the
                // menu bar takes.
                if let Some(note) = lock_pods(&self.pods).leave() {
                    self.restore_note = Some(note);
                }
            }
            Some(dock::DockAction::DismissReport) => {
                self.execution_result = None;
            }
            None => {}
        }

        // Settings, above everything. Called every frame so it leaves along
        // the path it arrived by.
        self.scoped_restore
            .show(ui.ctx(), &self.engine_threads, storage_root.as_path());
        shell::settings(
            ui,
            settings_open,
            &daemon_status,
            daemon,
            storage_root.as_path(),
        );
        self.show_workspace_layout_settings(ui.ctx());

        // The Pod Bar (⌥Space) floats above every other surface. The bar
        // paints above the settings window too: summoning it must always
        // work, whatever else is open.
        self.paint_pod_bar(ui.ctx());
        self.paint_voice_panel(ui.ctx());
        // The former Pod Bar is intentionally not painted. Native apps own
        // their windows; Evo only coordinates the restored work.
    }

    /// The window-layout picker and drag-to-learn gesture hosted on the
    /// desktop. Mirrors the Settings layout shelf but lives where the
    /// bars/cards sit so a user can choose a layout and drag a window to
    /// teach the arrangement from here.
    fn paint_pod_bar(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        egui::Window::new("pod-bar-desktop")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -34.0))
            .show(ctx, |ui| {
                // Drag-to-learn target: the shelf is the card. We use a small
        // explicit drag source on the shelf itself (a 320x28 strip over the
        // work-tile row) whose drop teaches the active work's recipe. That is
        // the smallest gesture that can honestly be called learn_drag and it
        // keeps the teaching path inside this card instead of a pointer-release
        // coincidence anywhere on the shelf.
        let teaching_pos = pod_bar_drag::update_state(ctx, egui::Id::new("evo-pod-bar-shelf"));
        if let Some(hover) = teaching_pos {
            let frame = evo_pods::surface::Frame {
                x: hover.x as f64 - 160.0,
                y: hover.y as f64 - 100.0,
                w: 320.0,
                h: 200.0,
            };
            if let Some(workspace) = self.display.as_ref() {
                let pod_id = evo_pods::pod::PodId(
                    workspace.id().as_str().to_string().strip_prefix("pod-")
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0),
                );
                let resource = workspace.id().as_str().to_string();
                self.pods.borrow_mut().core.learn_frame(pod_id, &resource, frame);
            }
        }
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Work tiles").strong());
                    if ui
                        .selectable_label(self.workspace_layout_preference.is_none(), "Auto")
                        .clicked()
                    {
                        self.workspace_layout_preference = None;
                        let _ =
                            std::fs::remove_file(self.storage_root.join("workspace-layout.txt"));
                    }
                    for (id, label) in [
                        (evo_pods::layout::LayoutId::Full, "Full"),
                        (evo_pods::layout::LayoutId::Halves, "Halves"),
                        (evo_pods::layout::LayoutId::Thirds, "Thirds"),
                        (evo_pods::layout::LayoutId::Quads, "Quads"),
                        (evo_pods::layout::LayoutId::MainSide, "Main + Side"),
                        (evo_pods::layout::LayoutId::LeftSplitRight, "Left split + right"),
                        (evo_pods::layout::LayoutId::LeftRightSplit, "Left + right split"),
                        (evo_pods::layout::LayoutId::MainSideVertical, "Main + side (vertical)"),
                        (evo_pods::layout::LayoutId::HalvesVertical, "Halves (vertical)"),
                        (evo_pods::layout::LayoutId::ThirdsVertical, "Thirds (vertical)"),
                        (evo_pods::layout::LayoutId::HeroRails, "5–8"),
                        (evo_pods::layout::LayoutId::Stack, "Stack"),
                    ] {
                        if ui
                            .selectable_label(self.workspace_layout_preference == Some(id), label)
                            .clicked()
                        {
                            self.workspace_layout_preference = Some(id);
                            let _ = std::fs::write(
                                self.storage_root.join("workspace-layout.txt"),
                                id.key(),
                            );
                        }
                    }
                });
                ui.label("Choose a preset or let Evo fit the number of windows automatically.");
            });
    }

    /// A drag source state for the pod bar shelf. Holds the dragged frame so
    /// the drop can teach the arrangement. One shelf per bar; the drag is
    /// the explicit gesture the user performs.


    /// A small workspace-layout control inside Settings. Presets are the same
    /// Setpiece-derived seeds used by the native arranger; automatic mode
    /// chooses from the number of windows in the opened work.
    fn show_workspace_layout_settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        egui::Window::new("workspace-layout-settings")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -34.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Work tiles").strong());
                    if ui
                        .selectable_label(self.workspace_layout_preference.is_none(), "Auto")
                        .clicked()
                    {
                        self.workspace_layout_preference = None;
                        let _ =
                            std::fs::remove_file(self.storage_root.join("workspace-layout.txt"));
                    }
                    for (id, label) in [
                        (evo_pods::layout::LayoutId::Full, "Full"),
                        (evo_pods::layout::LayoutId::Halves, "Halves"),
                        (evo_pods::layout::LayoutId::Thirds, "Thirds"),
                        (evo_pods::layout::LayoutId::Quads, "Quads"),
                        (evo_pods::layout::LayoutId::MainSide, "Main + Side"),
                        (evo_pods::layout::LayoutId::LeftSplitRight, "Left split + right"),
                        (evo_pods::layout::LayoutId::LeftRightSplit, "Left + right split"),
                        (evo_pods::layout::LayoutId::MainSideVertical, "Main + side (vertical)"),
                        (evo_pods::layout::LayoutId::HalvesVertical, "Halves (vertical)"),
                        (evo_pods::layout::LayoutId::ThirdsVertical, "Thirds (vertical)"),
                        (evo_pods::layout::LayoutId::HeroRails, "5–8"),
                        (evo_pods::layout::LayoutId::Stack, "Stack"),
                    ] {
                        if ui
                            .selectable_label(self.workspace_layout_preference == Some(id), label)
                            .clicked()
                        {
                            self.workspace_layout_preference = Some(id);
                            let _ = std::fs::write(
                                self.storage_root.join("workspace-layout.txt"),
                                id.key(),
                            );
                        }
                    }
                });
                ui.label("Choose a preset or let Evo fit the number of windows automatically.");
            });
    }

    /// Renders the voice ambiguity panel and toast.
    ///
    /// The panel is a floating numbered list, bottom-center, shown when a
    /// voice trigger resolved to multiple similar candidates. Each row shows
    /// its ordinal number, the work title, and a distinguishing hint. The
    /// user picks by clicking a row, pressing 1/2/3, or saying "the second
    /// one" (ordinals are also parsed from the next voice command while
    /// the panel is visible). Esc dismisses with no restore — the panel
    /// never guesses.
    ///
    /// Painted just below the Pod Bar in z-order: both are floating
    /// overlays, and the bar must always win.
    fn paint_voice_panel(&mut self, ctx: &egui::Context) {
        // Toast: transient outcome message, fades after a few seconds.
        if let Some((message, _)) = &self.voice_toast {
            egui::Window::new("voice-toast")
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -80.0))
                .show(ctx, |ui| {
                    ui.label(message);
                });
        }

        if !self.voice_panel.is_visible() {
            return;
        }

        // Number-key shortcuts: 1/2/3 pick the candidate.
        for (index, key) in [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3]
            .iter()
            .enumerate()
        {
            if ctx.input(|i| i.key_pressed(*key)) {
                self.voice_panel.pick(index);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.voice_panel.dismiss();
            return;
        }

        let mut picked_via_ui: Option<usize> = None;
        egui::Window::new("voice-ambiguity")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -120.0))
            .show(ctx, |ui| {
                ui.label("Which one did you mean?");
                ui.add_space(8.0);
                for (index, candidate) in self.voice_panel.candidates().iter().enumerate() {
                    let row = ui.horizontal(|ui| {
                        ui.label(format!("{}. ", index + 1));
                        ui.label(&candidate.title);
                    });
                    if row.response.clicked() {
                        picked_via_ui = Some(index);
                    }
                }
                ui.add_space(8.0);
                ui.label("Say \"the second one\", press 1–3, or click. Esc to cancel.");
            });
        if let Some(index) = picked_via_ui {
            self.voice_panel.pick(index);
        }
    }
}

impl eframe::App for EvoApp {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.logic_impl(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.ui_impl(ui, frame);
    }
}

// Reads events from the legacy SQLite store into the ledger's event format.
// ─── Work title presentation ────────────────────────────────────────────

/// Trailing title segments that recur across many works.
///
/// macOS window titles append app chrome — "… - Google Chrome - Profile" —
/// with either dash flavor as the separator. Rather than hardcoding any app
/// or name, a trailing segment that appears across at least this many
/// distinct works' titles is chrome: the person did not write it, their
/// window manager did.
#[allow(dead_code)]
fn recurring_title_suffixes(
    works: &[evo_ledger::works::Work],
) -> std::collections::HashSet<String> {
    use std::collections::HashSet;

    let min_recurrence = 3;
    let mut suffix_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for work in works {
        // Collect the work's own trailing segments without double counting:
        // "A - B - B" contributes {B} once.
        let segments = title_segments(&work.title());
        let mut seen = HashSet::new();
        for segment in segments.iter().rev().take(3) {
            if segment.is_empty() {
                continue;
            }
            if seen.insert(segment.clone()) {
                *suffix_counts.entry(segment.clone()).or_insert(0) += 1;
            }
        }
    }

    suffix_counts
        .into_iter()
        .filter(|(_, count)| *count >= min_recurrence)
        .map(|(segment, _)| segment)
        .collect()
}

/// Strips recurring chrome suffixes from a work title.
///
/// Only trailing segments are removed (chrome is appended, never prepended),
/// and never all of them — a title is always shown.
#[allow(dead_code)]
fn clean_work_title(title: &str, chrome: &std::collections::HashSet<String>) -> String {
    let mut segments = title_segments(title);
    // Drop trailing chrome segments, but keep at least the leading segment.
    while segments.len() > 1 {
        let last = segments[segments.len() - 1].clone();
        if chrome.contains(&last) {
            segments.pop();
        } else {
            break;
        }
    }
    let cleaned = segments.join(" — ");
    if cleaned.is_empty() {
        title.to_string()
    } else {
        cleaned
    }
}

/// The person-readable name of a work, from the richest evidence it
/// carries, in precedence order:
///
/// 1. The window titles the person actually read (page names, document
///    names) — the most descriptive text a work has. The longest
///    non-URL-shaped title wins; chrome is stripped, nothing hardcoded.
/// 2. The document the work is identified by: its filename.
/// 3. A meaningful name extracted from a URL (path segments that read like
///    words) or a document stem.
/// 4. The bare identity: host, title, or app.
#[allow(dead_code)]
fn work_display_name(
    work: &evo_ledger::works::Work,
    chrome: &std::collections::HashSet<String>,
) -> String {
    // 1. Window titles: the page names the person read.
    let best_title = work
        .titles
        .iter()
        .filter(|t| {
            !t.trim().is_empty()
                && !t.starts_with("http://")
                && !t.starts_with("https://")
                && !t.starts_with("file://")
        })
        .max_by_key(|t| t.chars().count())
        .map(|t| clean_work_title(t, chrome))
        .filter(|t| !t.trim().is_empty());
    if let Some(name) = best_title {
        return name;
    }

    // 2. The identity document's filename.
    if work.identity_kind == evo_ledger::works::IdentityKind::Document
        && let Some(file) = work.identity.rsplit('/').next()
        && !file.is_empty()
    {
        return file.to_string();
    }

    // 3. A meaningful name from the work's resources — but a bare host
    //    ("youtube.com") is not a name for a work that is a collection:
    //    when the work's URLs share one opaque collection token (a
    //    playlist), the honest name is the collection.
    for resource in work.documents.iter().chain(work.urls.iter()) {
        if let Some(name) = evo_daemon::work_understanding::meaningful_name(resource) {
            let host_only =
                !name.contains(' ') && (name.contains('.') || name == name.to_lowercase());
            if host_only
                && !work.urls.is_empty()
                && let Some(collection) = shared_collection_token(&work.urls)
            {
                return format!("Playlist {collection}");
            }
            if !host_only {
                return name;
            }
        }
    }

    // 4. The bare identity, chrome-stripped.
    clean_work_title(&work.title(), chrome)
}

/// The opaque token shared by every URL of a collection (a playlist id),
/// when one exists. Display naming only; identity is the ledger's.
fn shared_collection_token(urls: &[String]) -> Option<String> {
    let mut shared: Option<std::collections::BTreeSet<String>> = None;
    for url in urls {
        let tokens: std::collections::BTreeSet<String> =
            evo_ledger::pages::opaque_tokens(url).into_iter().collect();
        shared = Some(match shared {
            None => tokens,
            Some(previous) => previous.intersection(&tokens).cloned().collect(),
        });
    }
    shared?
        .into_iter()
        .max_by_key(|t| t.chars().count())
        .map(|t| t.chars().take(12).collect::<String>())
}

/// A human recency phrase: "just now", "12 minutes ago", "3 hours ago",
/// "2 days ago". Pure arithmetic on witnessed time, never a guess.
#[allow(dead_code)]
fn time_ago(then_ms: u64, now_ms: u64) -> String {
    let secs = now_ms.saturating_sub(then_ms) / 1000;
    if secs < 60 {
        return "just now".to_string();
    }
    let minutes = secs / 60;
    if minutes < 60 {
        return format!(
            "{minutes} minute{} ago",
            if minutes == 1 { "" } else { "s" }
        );
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours} hour{} ago", if hours == 1 { "" } else { "s" });
    }
    let days = hours / 24;
    format!("{days} day{} ago", if days == 1 { "" } else { "s" })
}

/// Splits a window title into its dash-separated segments. macOS apps use
/// both " — " (em dash) and " - " (hyphen) as separators.
fn title_segments(title: &str) -> Vec<String> {
    title
        .replace(" — ", " - ")
        .split(" - ")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// A compact fallback name for a resource the understanding layer could not
/// name: domain for URLs, filename for paths.
#[allow(dead_code)]
fn short_resource_name(resource: &str) -> String {
    if let Some(rest) = resource.strip_prefix("file://") {
        return rest.rsplit('/').next().unwrap_or(rest).to_string();
    }
    if resource.starts_with("http://") || resource.starts_with("https://") {
        let rest = resource
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        return rest.split('/').next().unwrap_or(rest).to_string();
    }
    resource.chars().take(40).collect()
}

#[allow(dead_code)]
fn read_events_from_sqlite(db_path: &std::path::Path) -> Vec<evo_ledger::ObservedEvent> {
    let output = std::process::Command::new("sqlite3")
        .arg("-json")
        .arg(db_path)
        .arg(
            r#"
            SELECT ts_ms, app_id, window_title,
                   COALESCE(normalized_url, '') as url,
                   COALESCE(download_path, '') as doc,
                   input_chars, dwell_ms
            FROM canonical_events
            WHERE ts_ms > (SELECT MAX(ts_ms) - 604800000 FROM canonical_events)
            ORDER BY ts_ms
        "#,
        )
        .output()
        .ok();

    let Some(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let json = String::from_utf8_lossy(&output.stdout);
    let mut events = Vec::new();
    for line in json.lines() {
        let trimmed = line.trim().trim_start_matches('[').trim_end_matches(']');
        if !trimmed.starts_with('{') {
            continue;
        }
        let ts = json_u64(trimmed, "ts_ms").unwrap_or(0);
        if ts == 0 {
            continue;
        }
        let app = json_str(trimmed, "app_id").unwrap_or_default();
        if app.is_empty() {
            continue;
        }
        events.push(evo_ledger::ObservedEvent {
            timestamp_ms: ts,
            app_name: app,
            window_title: json_str(trimmed, "window_title").unwrap_or_default(),
            url: json_str(trimmed, "url").filter(|u| !u.is_empty()),
            document_path: json_str(trimmed, "doc").filter(|d| !d.is_empty()),
            typed: json_u64(trimmed, "input_chars").unwrap_or(0) > 0,
            dwell_ms: json_u64(trimmed, "dwell_ms").unwrap_or(0).min(600000),
            keys: 0,
            clicks: 0,
            scrolls: 0,
        });
    }
    events
}

#[allow(dead_code)]
fn json_u64(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\":");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

#[allow(dead_code)]
fn json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

impl Drop for EvoApp {
    fn drop(&mut self) {
        // The daemon is our capture worker; leaving Evo stops capture. All
        // canonical state is already durably persisted by the daemon itself.
        self.daemon.kill();
    }
}
