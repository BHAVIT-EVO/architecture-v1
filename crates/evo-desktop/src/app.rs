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
use crate::state::{self, DisplayState};
use crate::ui;
use crate::{detail, home, shell, theme};

use evo_daemon::cache::CanonicalIndex;
use evo_engagement::Standing;
use evo_execution::{
    ExecutionReport, Locator, PreflightOutcome, RestorationSelection, preflight_selection,
};
use evo_restoration::DerivationOutcome;
use evo_workspace::workspace::Workspace;
use evo_workspace::workspace_id::WorkspaceId;

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
    /// The user's presentation-only retrieval query over remembered work.
    /// Filtering by it never changes canonical state or ordering semantics.
    search_query: String,
    /// The user's presentation-only resource-kind filter over remembered work
    /// (presentation state only; never continuation evidence).
    kind_filter: state::WorkKindFilter,
    /// The user's presentation-only time filter over remembered work, judged
    /// by the factual canonical latest witness instant.
    period_filter: state::WorkPeriodFilter,
    /// Memoized retrieval material per remembered Workspace, rebuilt only when
    /// the canonical boundary changed.
    retrieval_memo: home::Memo,
    /// The new-engine view: work threads derived by `evo-threads` from the
    /// same observation log the legacy Engagement pipeline reads, with each
    /// thread's resume bundle. Presentation-only; rebuilt on reload.
    engine_threads: Vec<evo_daemon::threads::DisplayThread>,
    /// The room runtime, shared with the menu-bar presence: containment
    /// (hide/park/raise), the active room, and the web panes'
    /// hibernation. Room commands execute on the main thread from wherever they come — the desktop never waits for this
    /// window to paint.
    rooms: crate::rooms::SharedRooms,
    /// Whether the room browser panel is open. Only meaningful while a
    /// room is active.
    room_browser: bool,
    /// The global hotkey + menu bar controller (keeps its allocations
    /// alive).
    room_presence: Option<crate::presence::RoomPresence>,
    /// The engine's current merge proposals: (subject_a, subject_b,
    /// name_a, name_b, shared sittings). Deterministic co-sitting between
    /// established works, offered to the person — confirming one declares
    /// the pair same-work through the daemon.
    merge_proposals: Vec<(String, String, String, String, usize)>,
    /// The honest account of the last restore, shown with the threads it
    /// came from: what opened, what was raised, what could not be opened.
    restore_note: Option<String>,
    /// Which work card is expanded (showing its detail view).
    expanded_work: Option<usize>,
    /// Receiver for background engine computation results.
    /// The engine rebuilds on a dedicated thread; results arrive here.
    engine_receiver: Option<std::sync::mpsc::Receiver<Vec<evo_daemon::threads::DisplayThread>>>,
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
}

/// Locks the shared room runtime from a field reference — a free
/// function so the render closure (which destructures `self` and may only
/// capture fields, never the whole) can reach it too.
fn lock_rooms(
    rooms: &crate::rooms::SharedRooms,
) -> std::sync::MutexGuard<'_, crate::rooms::RoomRuntime> {
    rooms
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
        // The menu-bar presence and global hotkey push room commands into
        // the app; the receiver is stored in the struct (the initial
        // placeholder receiver is replaced here). The egui context rides
        // along so every command also wakes the frame loop — a command
        // The room runtime is shared with the menu-bar presence so room
        // commands execute on the main thread from the action itself —
        // the desktop never waits for this window to paint (a hidden
        // window paints nothing).
        let rooms: crate::rooms::SharedRooms = std::sync::Arc::new(std::sync::Mutex::new(
            crate::rooms::RoomRuntime::new(),
        ));
        let room_presence = crate::presence::RoomPresence::new(rooms.clone(), cc.egui_ctx.clone());
        let mut app = Self {
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
            continue_phase: detail::ContinuePhase::Idle,
            designation_status: None,
            continuation_draft: None,
            continuation_status: None,
            grouping_status: None,
            search_query: String::new(),
            kind_filter: state::WorkKindFilter::All,
            period_filter: state::WorkPeriodFilter::All,
            retrieval_memo: home::Memo::default(),
            engine_threads: Vec::new(),
            rooms,
            room_browser: false,
            room_presence: Some(room_presence),
            merge_proposals: Vec::new(),
            restore_note: None,
            expanded_work: None,
            engine_receiver: None,
            first_run: shell::first_run_pending(),
            settings_open: false,
            status: None,
            last_reload: Instant::now(),
            last_signature: None,
        };
        app.reload();
        app
    }

    /// Locks the shared room runtime. Every caller is the main thread
    /// (render, logic, and the menu-bar actions), so the lock is
    /// uncontended by construction.
    fn rooms(&self) -> std::sync::MutexGuard<'_, crate::rooms::RoomRuntime> {
        lock_rooms(&self.rooms)
    }

    /// Re-reads canonical state by refreshing the derived index (only the
    /// records appended since the last reload are parsed).
    fn reload(&mut self) {
        // ─── The ledger path: Works from the engine process ────────────
        // Read Works directly from the ledger database (written by
        // evo-ledger-engine, a separate process). This is a fast indexed
        // SELECT — no engine computation, no background thread needed.
        //
        // Includes: recency-weighted lifecycle (2-week half-life, 30-day fade)
        // and FTS5 search integration.
        // The ledger lives inside the storage root: the capture layer's
        // self-write exclusion covers it there, so the engine's rebuilds
        // never feed back through the file watcher.
        let ledger_path = self.storage_root.join("ledger.db");
        let mut ledger_has_works = false;
        if ledger_path.exists() {
            if let Ok(ledger) = evo_ledger::Ledger::open_readonly(&ledger_path) {
                let works = ledger.works(30).unwrap_or_default();
                ledger_has_works = !works.is_empty();
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                // macOS window titles append app chrome ("… — Google Chrome
                // — Profile"). Nothing is hardcoded: a trailing segment that
                // recurs across many works' titles is chrome, and it is
                // dropped; the leading content the person named stays.
                let chrome_suffixes = recurring_title_suffixes(&works);

                // Each pair carries the work's containment surface so the
                // two derived lists stay exactly parallel through the
                // filter and the sort below: the surface at index i is the
                // room of the thread at index i. Indexing them separately
                // (as they once were) makes Enter open the wrong work's
                // room the moment the ledger order differs from the
                // engagement order.
                let mut paired: Vec<(
                    evo_daemon::threads::DisplayThread,
                    evo_execution::room::RoomSurface,
                )> = works
                    .iter()
                    .filter_map(|w| {
                        // ─── Work lifecycle: fade, don't delete ─────────
                        // 30-day fade: untouched for a month → off the surface
                        let days_since =
                            (now_ms.saturating_sub(w.last_active_ms)) as f64 / 86_400_000.0;
                        if days_since > 30.0 {
                            return None; // faded from default view (still in ledger)
                        }

                        // Recency decay: 2-week half-life on engagement
                        let decayed_engagement =
                            w.total_time_ms as f64 * 0.5_f64.powf(days_since / 14.0);

                        // The person-readable name, from the richest
                        // evidence the work carries: the page titles the
                        // person actually read, then the document they
                        // edited, then a meaningful URL name, then the bare
                        // identity. Chrome is stripped, never hardcoded.
                        let name = work_display_name(w, &chrome_suffixes);
                        let phase = match w.engagement() {
                            evo_ledger::works::Engagement::Building => {
                                evo_daemon::work_understanding::WorkPhase::Building
                            }
                            evo_ledger::works::Engagement::Writing => {
                                evo_daemon::work_understanding::WorkPhase::Writing
                            }
                            evo_ledger::works::Engagement::Reading => {
                                evo_daemon::work_understanding::WorkPhase::Researching
                            }
                        };
                        let recency = time_ago(w.last_active_ms, now_ms);

                        // Honest evidence lines, never invented.
                        let mut completed: Vec<String> = Vec::new();
                        if w.attached_saves > 0 {
                            completed.push(format!(
                                "{} file{} saved",
                                w.attached_saves,
                                if w.attached_saves == 1 { "" } else { "s" }
                            ));
                        }
                        if w.has_production {
                            completed.push("Typed here".to_string());
                        }

                        // The restore neighborhood: files are stronger
                        // anchors than pages (they are the produced work),
                        // so documents lead, then sites, deduplicated.
                        let mut restore_set: Vec<String> = Vec::new();
                        for doc in w.documents.iter().chain(w.urls.iter()) {
                            if !restore_set.contains(doc) {
                                restore_set.push(doc.clone());
                            }
                            if restore_set.len() >= 5 {
                                break;
                            }
                        }
                        let sites = w.urls.len();
                        let files = w.documents.len();
                        let next_step = if sites == 0 && files == 0 {
                            format!("Open {} again", name)
                        } else {
                            format!(
                                "Reopen {}{}{}",
                                if files > 0 {
                                    format!("{} file{}", files, if files == 1 { "" } else { "s" })
                                } else {
                                    String::new()
                                },
                                if files > 0 && sites > 0 { " and " } else { "" },
                                if sites > 0 {
                                    format!("{} site{}", sites, if sites == 1 { "" } else { "s" })
                                } else {
                                    String::new()
                                },
                            )
                        };

                        // ─── Build the DisplayThread ─────────────────────
                        let bundle = evo_threads::ResumeBundle {
                            resume_point: w
                                .documents
                                .first()
                                .cloned()
                                .or_else(|| w.urls.first().cloned())
                                .unwrap_or_default(),
                            resume_reason: format!(
                                "{} across {} sessions, last active {}",
                                w.time_summary(),
                                w.session_count(),
                                recency,
                            ),
                            restore_set: restore_set.clone(),
                            open_deltas: vec![],
                        };

                        // The containment surface built from the same work
                        // record the thread was built from — same evidence,
                        // same index. Its label is the work's display name
                        // (not the ledger identity) so every room-keyed
                        // thing — the active-room strip, the menu bar's
                        // dot, the parking ledger — speaks one name.
                        let surface = evo_execution::room::RoomSurface {
                            work: name.clone(),
                            urls: w.urls.clone(),
                            documents: w.documents.clone(),
                            titles: w.titles.clone(),
                            apps: w.apps.clone(),
                            // Per-resource witnessed apps: each document
                            // (and URL) maps to the application it was most
                            // recently seen in. Restore opens every file
                            // through its own witnessed app — never one
                            // work-level app for everything.
                            resource_apps: w
                                .documents
                                .iter()
                                .chain(w.urls.iter())
                                .filter_map(|resource| {
                                    w.app_for_resource(resource)
                                        .map(|app| (resource.clone(), app.to_string()))
                                })
                                .collect(),
                        };
                        let narrative = format!(
                            "You spent {} across {} sessions {} this — {}. {}",
                            w.time_summary(),
                            w.session_count(),
                            phase.verb(),
                            if recency == "just now" {
                                "active moments ago".to_string()
                            } else {
                                format!("last active {}", recency)
                            },
                            if w.attached_saves > 0 {
                                format!("{} files were produced here.", w.attached_saves)
                            } else if w.has_production {
                                "You were typing here.".to_string()
                            } else {
                                "No typing was observed — this was reading.".to_string()
                            },
                        );

                        // The containment surface built from the same work
                        // record the thread was built from — same evidence,
                        // same index. Its label is the work's display name
                        // (not the ledger identity) so every room-keyed
                        // thing — the active-room strip, the menu bar's
                        // dot, the parking ledger — speaks one name.
                        // (The surface itself is built above, next to the
                        // thread, from the same work record.)
                        Some((
                            evo_daemon::threads::DisplayThread {
                                thread_id: 0,
                                name: name.clone(),
                                bundle,
                                is_work: true,
                                engagement: decayed_engagement,
                                understanding: evo_daemon::work_understanding::WorkUnderstanding {
                                    title: name.clone(),
                                    phase,
                                    trail: w
                                        .urls
                                        .iter()
                                        .chain(w.documents.iter())
                                        .take(8)
                                        .map(|u| evo_daemon::work_understanding::TrailEntry {
                                            name: evo_daemon::work_understanding::meaningful_name(
                                                u,
                                            )
                                            .unwrap_or_else(|| short_resource_name(u)),
                                            raw: u.clone(),
                                            action: if u.starts_with("http") {
                                                evo_daemon::work_understanding::TrailAction::Visited
                                            } else {
                                                evo_daemon::work_understanding::TrailAction::Opened
                                            },
                                        })
                                        .collect(),
                                    completed,
                                    pending: vec![],
                                    next_step,
                                    narrative,
                                },
                                last_active_ms: Some(w.last_active_ms),
                                member_subjects: w
                                    .urls
                                    .iter()
                                    .chain(w.documents.iter())
                                    .chain(w.titles.iter())
                                    .cloned()
                                    .collect(),
                            },
                            surface,
                        ))
                    })
                    .collect::<Vec<_>>();

                // Sort by decayed engagement (recent + long work first),
                // carrying each work's containment surface alongside.
                paired.sort_by(|a, b| {
                    b.0.engagement
                        .partial_cmp(&a.0.engagement)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let (threads, surfaces): (Vec<_>, Vec<_>) = paired.into_iter().collect();
                self.engine_threads = threads;
                // Containment surfaces, parallel to the threads (built from
                // the same filtered, sorted pairing): the work's own
                // evidence (apps, pages, documents, titles) is what the
                // room controller matches live windows against. The names
                // ride along for the menu bar; a room whose work vanished
                // from the list is left honestly.
                let names: Vec<String> = self
                    .engine_threads
                    .iter()
                    .map(|thread| thread.name.clone())
                    .collect();
                lock_rooms(&self.rooms).set_surfaces(surfaces, names);
                self.merge_proposals = ledger.merge_proposals().unwrap_or_default();
            }
        }

        // ─── The legacy path: engagement reconstruction ────────────────
        // The legacy pipeline re-derives its affinity understanding from the
        // full observation history on this thread. With weeks of evidence a
        // single reconstruction outlasts the reload period — the UI thread
        // would never paint. The ledger engine (its own process) is the
        // Home surface now, so the legacy reconstruction only runs when the
        // ledger has nothing to show.
        if ledger_has_works {
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
                if let Some(id) = &self.selected {
                    if !self.workspaces.iter().any(|workspace| workspace.id() == id) {
                        self.selected = None;
                    }
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

impl eframe::App for EvoApp {
    /// Non-UI logic: re-check the daemon worker, re-read canonical storage,
    /// and keep the surface alive.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The window never dies: closing it hides Evo instead. A menu-bar
        // app whose loop can lose its window is a menu-bar app whose
        // commands stop being processed; hiding keeps the room presence
        // (menu bar, hotkey, containment) alive. The browser panes
        // hibernate with the hidden window — nothing pays RAM while
        // unseen, and "Open Evo" restores the window.
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            lock_rooms(&self.rooms).web.hibernate();
            self.room_browser = false;
        }

        self.daemon.poll();
        self.engine.poll();

        // Room commands from the menu bar and hotkey execute directly on
        // the main thread inside the action itself (see presence.rs) —
        // this window may be hidden and painting nothing. What arrives
        // here is the note they left, shown at the next opportunity.
        {
            let mut rooms = lock_rooms(&self.rooms);
            if let Some(note) = rooms.pending_note.take() {
                self.restore_note = Some(note);
            }
            let active = rooms.active.clone();
            let names = rooms.names.clone();
            drop(rooms);
            // Keep the menu bar in sync with the room list.
            if let Some(presence) = &mut self.room_presence {
                presence.update_rooms(&names, active);
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
                            // Suppress workspace re-derivation for a short period after
                            // execution. The OS actions (opening URLs, raising windows)
                            // create new observations that could shift artifact
                            // assignments between workspaces if processed immediately.
                            if let Some(index) = self.index.as_mut() {
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

        ctx.request_repaint_after(Duration::from_secs(2));
    }

    /// Renders the shell.
    ///
    /// The primary surface is Home — "what bodies of work can I continue?".
    /// A detail screen for one body of work is entered only by the user's
    /// explicit selection, keyed by canonical Workspace identity. The
    /// contextual continuation panel lives inside that detail screen and is
    /// never the application's navigation.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
            workspaces,
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
            search_query,
            kind_filter,
            period_filter,
            retrieval_memo,
            status,
            settings_open,
            ..
        } = self;

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
                        // Entering a room with web pages opens them here:
                        // the work's web presence lives inside Evo, so the
                        // person never juggles a browser to get back in.
                        // Drained once — hiding the panel stays hidden
                        // until the next room entry.
                        {
                            let mut rooms = lock_rooms(&self.rooms);
                            if rooms.wants_browser {
                                rooms.wants_browser = false;
                                self.room_browser = true;
                            }
                        }
                        // Stage 3: the room browser. While a room is
                        // active and the panel is open, the work's own
                        // pages render inside this window — no app switch,
                        // no second window to juggle. The panel replaces
                        // the list: a room occupies the whole surface or
                        // none of it.
                        if self.room_browser {
                            let mut rooms = lock_rooms(&self.rooms);
                            if let Some(room) = rooms.active.clone() {
                                // Hydration is idempotent and honest: the
                                // first open (or a room switch) creates the
                                // panes; the same work re-opened keeps
                                // them.
                                if !rooms.web.is_hydrated_for(&room) {
                                    let urls: Vec<String> = rooms
                                        .surfaces
                                        .iter()
                                        .find(|surface| surface.work == room)
                                        .map(|surface| surface.urls.clone())
                                        .unwrap_or_default();
                                    rooms.web.hydrate(&room, &urls);
                                }
                                drop(rooms);

                                // The room strip: where you are, and the
                                // way back to the list.
                                egui::Frame::new()
                                    .inner_margin(egui::Margin::symmetric(
                                        theme::M6,
                                        theme::M2,
                                    ))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.label("\u{25CF}");
                                            ui.strong(format!("In: {room}"));
                                            if ui.small_button("Hide browser").clicked() {
                                                // Hiding is hibernation,
                                                // not hiding: the panes
                                                // and their processes
                                                // leave RAM; the data
                                                // store stays on disk.
                                                lock_rooms(&self.rooms).web.hibernate();
                                                self.room_browser = false;
                                            }
                                            ui.weak("  logins isolated per work");
                                        });
                                    });

                                // The tab strip: back/forward and the
                                // work's own pages. Nothing more — Evo
                                // hosts the work's web presence, it does
                                // not become a browser. A work with many
                                // pages wraps rather than scrolls: every
                                // page stays reachable.
                                let mut rooms = lock_rooms(&self.rooms);
                                let (can_back, can_forward) =
                                    (rooms.web.can_go_back(), rooms.web.can_go_forward());
                                ui.horizontal_wrapped(|ui| {
                                    ui.add_enabled_ui(can_back, |ui| {
                                        if ui.button("\u{2039}").clicked() {
                                            rooms.web.go_back();
                                        }
                                    });
                                    ui.add_enabled_ui(can_forward, |ui| {
                                        if ui.button("\u{203A}").clicked() {
                                            rooms.web.go_forward();
                                        }
                                    });
                                    ui.separator();
                                    for index in 0..rooms.web.pane_count() {
                                        let label: String = rooms
                                            .web
                                            .tab_label(index)
                                            .chars()
                                            .take(28)
                                            .collect();
                                        if ui
                                            .selectable_label(
                                                rooms.web.active_index() == index,
                                                label,
                                            )
                                            .clicked()
                                        {
                                            rooms.web.switch_tab(index);
                                        }
                                    }
                                });
                                ui.add_space(theme::S2);

                                // The pane itself: egui reserves the rect
                                // and never interacts with it — the native
                                // web view above it receives every mouse
                                // and keyboard event over that region.
                                if rooms.web.pane_count() == 0 {
                                    ui::caption(
                                        ui,
                                        "This work has no web pages to host.",
                                    );
                                } else {
                                    let pane = ui.available_rect_before_wrap();
                                    ui.painter().rect_filled(pane, 4.0, theme::WELL);
                                    ui.allocate_rect(pane, egui::Sense::hover());
                                    rooms.web.layout_active((
                                        pane.min.x,
                                        pane.min.y,
                                        pane.width(),
                                        pane.height(),
                                    ));
                                }
                                return;
                            }
                            // A browser panel without a room cannot exist:
                            // the room was left while the panel was open.
                            lock_rooms(&self.rooms).web.hibernate();
                            self.room_browser = false;
                        }
                        // The new engine's work section is the primary surface.
                        // When it has work to show, the legacy pipeline's Home
                        // screen (with its separate heading, search, and list)
                        // is not rendered at all — one clean section, not two.
                        let active_room = {
                            let rooms = lock_rooms(&self.rooms);
                            rooms.active.clone()
                        };
                        match home::work_section(
                            ui,
                            &self.engine_threads,
                            &mut self.expanded_work,
                            self.restore_note.as_deref(),
                            search_query,
                            &self.merge_proposals,
                            active_room.as_deref(),
                        ) {
                            None => {}
                            Some(home::WorkAction::Merge {
                                subject_a,
                                subject_b,
                                other_name,
                            }) => {
                                // The person confirmed a merge proposal: the
                                // declaration goes through the daemon's
                                // grouping socket and becomes ground truth
                                // the engine honors from the next cycle.
                                match evo_daemon::grouping::submit_grouping(
                                    &*storage_root,
                                    &subject_a,
                                    &subject_b,
                                ) {
                                    // The socket's answer is reported exactly:
                                    // Accepted means the declaration was
                                    // forwarded for canonical persistence;
                                    // Rejected means it was NOT — conflating
                                    // the two once showed failures as
                                    // success.
                                    Ok(evo_daemon::grouping::GroupingResponse::Accepted) => {
                                        self.restore_note = Some(format!(
                                            "Declared one work with {other_name} — they will join on the next refresh."
                                        ));
                                    }
                                    Ok(evo_daemon::grouping::GroupingResponse::Rejected(reason)) => {
                                        self.restore_note =
                                            Some(format!("Merge declined: {reason}"));
                                    }
                                    Err(err) => {
                                        self.restore_note =
                                            Some(format!("Merge declaration failed: {err}"));
                                    }
                                }
                            }
                            Some(home::WorkAction::Continue(index)) => {
                                // Continuing a work IS entering its room:
                                // containment hides everything not of this
                                // work, restoration opens every artifact
                                // (files through their witnessed apps, the
                                // work's pages in its own browser panel —
                                // logins isolated per work), and the room
                                // report says what happened. One path, one
                                // note; the menu-bar Enter does the same.
                                if let Some(note) =
                                    lock_rooms(&self.rooms).enter_at(index)
                                {
                                    self.restore_note = Some(note);
                                }
                            }
                            Some(home::WorkAction::OpenBrowser) => {
                                // The room browser opens for the active
                                // room only; hydration happens in the
                                // render pass that first shows the panel.
                                if lock_rooms(&self.rooms).active.is_some() {
                                    self.room_browser = true;
                                }
                            }
                            Some(home::WorkAction::Leave) => {
                                // Leaving restores the desktop and releases
                                // the work's web panes through the
                                // runtime — the same path the menu bar
                                // takes.
                                if let Some(note) = lock_rooms(&self.rooms).leave() {
                                    self.room_browser = false;
                                    self.restore_note = Some(note);
                                }
                            }
                        }
                        // The new engine has work: the Home screen is complete.
                        // The legacy screen is not rendered — no duplicate
                        // heading, no stale list, no confusion.
                        if !self.engine_threads.is_empty() {
                            return;
                        }
                        // No new-engine work either: show why.
                        if workspaces.is_empty() {
                            if let Some(status) = status.as_deref() {
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
                                    home::permission_required(ui, Some(detail), || {
                                        crate::daemon::open_accessibility_settings()
                                    });
                                }
                                other => {
                                    home::empty(ui, matches!(other, DaemonStatus::Capturing))
                                }
                            }
                            return;
                        }
                        // The threads section was already rendered above
                        // (before the empty-check), so it shows even when
                        // the legacy pipeline has nothing. The Restore
                        // action is handled there.
                        let mut retrieval = home::Retrieval {
                            query: search_query,
                            kind: kind_filter,
                            period: period_filter,
                            memo: retrieval_memo,
                        };
                        home::screen(
                            ui,
                            workspaces.as_slice(),
                            home_cards.as_slice(),
                            &*subjects,
                            &*kinds,
                            status.is_some(),
                            selected,
                            &mut retrieval,
                        );
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
                    }
                }
            },
        );

        // Settings, above everything. Called every frame so it leaves along
        // the path it arrived by.
        shell::settings(
            ui,
            settings_open,
            &daemon_status,
            daemon,
            storage_root.as_path(),
        );
    }
}

/// Reads events from the legacy SQLite store into the ledger's event format.
// ─── Work title presentation ────────────────────────────────────────────────

/// Trailing title segments that recur across many works.
///
/// macOS window titles append app chrome — "… - Google Chrome - Profile" —
/// with either dash flavor as the separator. Rather than hardcoding any app
/// or name, a trailing segment that appears across at least this many
/// distinct works' titles is chrome: the person did not write it, their
/// window manager did.
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
    if work.identity_kind == evo_ledger::works::IdentityKind::Document {
        if let Some(file) = work.identity.rsplit('/').next() {
            if !file.is_empty() {
                return file.to_string();
            }
        }
    }

    // 3. A meaningful name from the work's resources — but a bare host
    //    ("youtube.com") is not a name for a work that is a collection:
    //    when the work's URLs share one opaque collection token (a
    //    playlist), the honest name is the collection.
    for resource in work.documents.iter().chain(work.urls.iter()) {
        if let Some(name) = evo_daemon::work_understanding::meaningful_name(resource) {
            let host_only =
                !name.contains(' ') && (name.contains('.') || name == name.to_lowercase());
            if host_only && !work.urls.is_empty() {
                if let Some(collection) = shared_collection_token(&work.urls) {
                    return format!("Playlist {collection}");
                }
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

fn json_u64(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\":");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

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
