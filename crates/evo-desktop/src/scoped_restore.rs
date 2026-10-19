//! One explicitly chosen locator from one current engine work, through RESOURCE.
//!
//! This surface never enters a pod or builds a restore plan. Its only execution
//! boundary is `VoiceSocketClient::scope_exact`, on a worker thread. A missing response
//! is unknown, not a success or a cancellation: the pending latch survives both
//! timeout and dismissal, preventing another dispatch until the worker returns.

use crate::theme::{self, Role, Severity};
use crate::ui::{self, Control};
use evo_daemon::threads::DisplayThread;
use evo_voice::VoiceOutcome;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

pub const RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);

/// Every distinct, protocol-eligible locator in this work's current restore set.
/// No trail, member-subject, resume-point, or unrelated record is added here.
pub fn eligible_resources(work: &DisplayThread) -> Vec<&str> {
    if !work.is_work {
        return Vec::new();
    }
    let mut seen = std::collections::BTreeSet::new();
    work.bundle
        .restore_set
        .iter()
        .map(String::as_str)
        .filter(|locator| eligible_locator(locator) && seen.insert(*locator))
        .collect()
}

fn eligible_locator(locator: &str) -> bool {
    !locator.trim().is_empty() && locator.len() <= 4096 && !locator.starts_with("Downloaded:")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedRequest {
    pub id: u64,
    pub thread_id: u64,
    pub work_name: String,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopedOutcome {
    /// The daemon's actual report; no independent native verification implied.
    Restored {
        title: String,
        summary: String,
    },
    Failed(String),
    RequestFailed(String),
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedResult {
    pub request: ScopedRequest,
    pub outcome: ScopedOutcome,
}

/// Pure interaction state, keyed by engine identity and exact locator strings.
#[derive(Debug, Default)]
pub struct ScopedRestoreState {
    pub visible: bool,
    thread_id: Option<u64>,
    work_name: String,
    selected: Option<String>,
    selection_note: Option<&'static str>,
    pending: Option<ScopedRequest>,
    pub result: Option<ScopedResult>,
    next_request_id: u64,
}

impl ScopedRestoreState {
    pub fn open_for(&mut self, thread_id: u64, works: &[DisplayThread]) {
        // A dispatched request cannot be cancelled by closing or navigating.
        // Reopening while it is pending always shows the actual request owner.
        let thread_id = self.pending.as_ref().map_or(thread_id, |r| r.thread_id);
        if self.thread_id != Some(thread_id) {
            self.thread_id = Some(thread_id);
            self.work_name = String::new();
            self.selected = None;
            self.selection_note = None;
            self.result = None;
        }
        self.visible = true;
        self.sync(works);
    }

    pub fn dismiss(&mut self) {
        self.visible = false;
        self.selected = None;
        self.selection_note = None;
    }

    pub fn thread_id(&self) -> Option<u64> {
        self.thread_id
    }

    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub fn pending(&self) -> Option<&ScopedRequest> {
        self.pending.as_ref()
    }

    fn current_work<'a>(&self, works: &'a [DisplayThread]) -> Option<&'a DisplayThread> {
        let id = self.thread_id?;
        // Duplicate identities do not give the UI authority to choose a work.
        let mut matches = works.iter().filter(|work| work.thread_id == id);
        let work = matches.next()?;
        matches.next().is_none().then_some(work)
    }

    pub fn sync(&mut self, works: &[DisplayThread]) {
        let work = self.current_work(works);
        if let Some(work) = work {
            self.work_name = work.name.clone();
        }
        if self.selected.as_ref().is_some_and(|locator| {
            !work.is_some_and(|work| eligible_resources(work).contains(&locator.as_str()))
        }) {
            self.selected = None;
            self.selection_note = Some(
                "The chosen resource is no longer available in this work. Choose again from the current list.",
            );
        }
    }

    pub fn choose(&mut self, locator: &str, works: &[DisplayThread]) -> bool {
        if self.pending.is_some()
            || !self.visible
            || !self
                .current_work(works)
                .is_some_and(|work| eligible_resources(work).contains(&locator))
        {
            return false;
        }
        self.selected = Some(locator.to_string());
        self.selection_note = None;
        true
    }

    pub fn can_request(&self, works: &[DisplayThread]) -> bool {
        self.visible
            && self.pending.is_none()
            && self.selected.as_deref().is_some_and(|locator| {
                self.current_work(works)
                    .is_some_and(|work| eligible_resources(work).contains(&locator))
            })
    }

    /// Re-checks the current snapshot at dispatch, not merely at selection time.
    pub fn begin_request(&mut self, works: &[DisplayThread]) -> Option<ScopedRequest> {
        self.sync(works);
        if !self.can_request(works) {
            return None;
        }
        let work = self.current_work(works)?;
        let request = ScopedRequest {
            id: self.next_request_id.checked_add(1)?,
            thread_id: work.thread_id,
            work_name: work.name.clone(),
            locator: self.selected.clone()?,
        };
        self.next_request_id = request.id;
        self.pending = Some(request.clone());
        self.result = None;
        Some(request)
    }

    /// Applies a response only to the matching pending request. Also usable by
    /// synthetic UI harnesses that do not connect to any daemon.
    pub fn complete(&mut self, request_id: u64, response: Result<VoiceOutcome, String>) {
        let Some(request) = self
            .pending
            .as_ref()
            .filter(|r| r.id == request_id)
            .cloned()
        else {
            return;
        };
        let outcome = match response {
            Ok(VoiceOutcome::Restored { title, summary, .. }) => {
                ScopedOutcome::Restored { title, summary }
            }
            Ok(VoiceOutcome::Failed { reason }) => ScopedOutcome::Failed(reason),
            Ok(_) => ScopedOutcome::Failed(
                "The daemon did not report a scoped restoration. No broader request was sent."
                    .into(),
            ),
            Err(reason) => ScopedOutcome::RequestFailed(reason),
        };
        self.pending = None;
        // A completed request never leaves an armed operation behind it.
        self.selected = None;
        self.result = Some(ScopedResult { request, outcome });
    }

    fn mark_timeout(&mut self) {
        if let Some(request) = self.pending.clone() {
            self.selected = None;
            self.result = Some(ScopedResult {
                request,
                outcome: ScopedOutcome::TimedOut,
            });
        }
    }
}

struct Completion {
    request_id: u64,
    response: Result<VoiceOutcome, String>,
}

#[derive(Default)]
pub struct ScopedRestore {
    pub state: ScopedRestoreState,
    receiver: Option<Receiver<Completion>>,
    started_at: Option<Instant>,
}

impl ScopedRestore {
    /// Returns immediately; the socket and response parsing live on the worker.
    pub fn dispatch(&mut self, root: &Path, works: &[DisplayThread], ctx: &egui::Context) -> bool {
        let Some(request) = self.state.begin_request(works) else {
            return false;
        };
        let request_id = request.id;
        let root = root.to_path_buf();
        let repaint = ctx.clone();
        let (sender, receiver) = mpsc::channel();
        match std::thread::Builder::new()
            .name("evo-scoped-restore".into())
            .spawn(move || {
                let response = crate::voice::VoiceSocketClient::new(root)
                    .scope_exact(&request.thread_id.to_string(), &request.locator);
                let _ = sender.send(Completion {
                    request_id,
                    response,
                });
                repaint.request_repaint();
            }) {
            Ok(_) => {
                self.receiver = Some(receiver);
                self.started_at = Some(Instant::now());
            }
            Err(error) => self.state.complete(
                request_id,
                Err(format!("Could not start request worker: {error}")),
            ),
        }
        true
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        self.poll_at(Instant::now());
        if self.state.pending.is_some() {
            let timed_out = matches!(
                self.state.result.as_ref().map(|r| &r.outcome),
                Some(ScopedOutcome::TimedOut)
            );
            ctx.request_repaint_after(if timed_out {
                Duration::from_secs(2)
            } else {
                Duration::from_millis(100)
            });
        }
    }

    fn poll_at(&mut self, now: Instant) {
        let completion = self.receiver.as_ref().map(Receiver::try_recv);
        match completion {
            Some(Ok(completion)) => {
                self.state
                    .complete(completion.request_id, completion.response);
                self.receiver = None;
                self.started_at = None;
            }
            Some(Err(TryRecvError::Disconnected)) => {
                if let Some(request) = self.state.pending.as_ref() {
                    self.state.complete(request.id, Err("Request worker ended without a response; dispatch effects are unknown.".into()));
                }
                self.receiver = None;
                self.started_at = None;
            }
            _ => {
                if self
                    .started_at
                    .is_some_and(|start| now.saturating_duration_since(start) >= RESPONSE_TIMEOUT)
                    && !matches!(
                        self.state.result.as_ref().map(|r| &r.outcome),
                        Some(ScopedOutcome::TimedOut)
                    )
                {
                    self.state.mark_timeout();
                }
            }
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, works: &[DisplayThread], root: &Path) {
        if sheet(ctx, &mut self.state, works) {
            self.dispatch(root, works, ctx);
        }
    }
}

/// Effect-free native egui sheet; only the explicit operation press is returned.
pub fn sheet(ctx: &egui::Context, state: &mut ScopedRestoreState, works: &[DisplayThread]) -> bool {
    if !state.visible {
        return false;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.dismiss();
        return false;
    }
    let available = ctx.content_rect();
    let size = egui::vec2(
        (available.width() - theme::S6).clamp(1.0, 600.0),
        (available.height() - theme::S6).clamp(1.0, 560.0),
    );
    let mut requested = false;
    let window = egui::Window::new("Restore selected resource")
        .id(egui::Id::new("evo-scoped-restore-sheet"))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fixed_size(size)
        .frame(theme::sheet_frame())
        .show(ctx, |ui| requested = sheet_contents(ui, state, works));
    if cfg!(test)
        && let Some(window) = window
    {
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new("scope-sheet-rect"), window.response.rect)
        });
    }
    requested
}

/// Native sheet body. Returns only an explicit operation press; rendering and
/// selecting are effect-free. The caller re-checks the snapshot in `dispatch`.
pub fn sheet_contents(
    ui: &mut egui::Ui,
    state: &mut ScopedRestoreState,
    works: &[DisplayThread],
) -> bool {
    state.sync(works);
    ui::headline(ui, "Restore selected…");
    let height = (ui.available_height() - theme::CONTROL_HEIGHT * 2.0 - theme::S5).max(theme::S8);
    egui::ScrollArea::vertical()
        // A response starts at the top of the sheet, rather than landing below
        // a long list at the user's previous resource-list scroll position.
        .id_salt((
            "evo-scope-resources",
            state.thread_id,
            state.pending.as_ref().map(|r| r.id),
            state.result.as_ref().map(|r| r.request.id),
        ))
        .max_height(height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(theme::display(&state.work_name, Role::Title.size(), theme::TEXT));
            ui.label(theme::data(format!("Work id: {}", state.thread_id.map_or_else(|| "unavailable".into(), |id| id.to_string())), Role::Data.size(), theme::MUTED));
            ui::gap(ui, theme::S3);
            if let Some(result) = &state.result {
                let (word, severity, message) = match &result.outcome {
                    ScopedOutcome::Restored { title, summary } => ("Daemon reported", Severity::Neutral, format!("{title} — {summary}")),
                    ScopedOutcome::Failed(reason) => ("Failed", Severity::Bad, reason.clone()),
                    ScopedOutcome::RequestFailed(reason) => ("Request failed", Severity::Bad, format!("{reason} Dispatch effects may be unknown; no retry was sent.")),
                    ScopedOutcome::TimedOut => ("Response timed out", Severity::Caution, "No daemon response within 15 seconds. The result is unknown. This request remains pending; no retry was sent.".into()),
                };
                ui::notice(ui, severity, word, &message);
                ui::gap(ui, theme::S2);
                ui::caption(ui, &format!("Request #{} · work {} · {}", result.request.id, result.request.thread_id, result.request.work_name));
                ui.add(egui::Label::new(theme::data(&result.request.locator, Role::Data.size(), theme::TEXT)).wrap());
                ui::gap(ui, theme::S3);
            } else if let Some(request) = state.pending() {
                ui::caption(ui, &format!("Request #{} pending for work {} — {}", request.id, request.thread_id, request.work_name));
                ui::caption(ui, "Already dispatched. Dismiss closes this sheet; it does not cancel the daemon request.");
                ui.add(egui::Label::new(theme::data(&request.locator, Role::Data.size(), theme::TEXT)).wrap());
                ui::gap(ui, theme::S3);
            }
            ui::caption(ui, "Choose exactly one resource from this work’s current restore set. The daemon will attempt only this locator.");
            let resources = state.current_work(works).map(eligible_resources).unwrap_or_default();
            if resources.is_empty() {
                let message = match state.current_work(works) {
                    None => "This work is no longer present in the current rack. Nothing can be requested.",
                    Some(work) if !work.is_work => "This record is not currently eligible as work. Nothing can be requested.",
                    _ => "This work has no eligible resource locators in its current restore set.",
                };
                ui::notice(ui, Severity::Neutral, "Unavailable", message);
            }
            for locator in resources {
                let chosen = state.selected() == Some(locator);
                let id = egui::Id::new(("evo-scope-resource", state.thread_id, locator));
                let response = ui.add_enabled_ui(state.pending.is_none(), |ui| {
                    ui::row(ui, id, |ui| {
                        ui.horizontal_top(|ui| {
                            let mark = ui::origin_mark(ui, id.with("mark"), chosen);
                            ui.add_space(theme::S2);
                            ui.add(egui::Label::new(theme::data(locator, Role::Data.size(), if chosen { theme::ACCENT } else { theme::TEXT })).wrap());
                            mark.clicked()
                        }).inner
                    })
                }).inner;
                if response.0.clicked() || response.1 {
                    state.choose(locator, works);
                }
                ui::gap(ui, theme::S2);
            }
            if let Some(note) = state.selection_note {
                ui::caption(ui, note);
            }
            if let Some(locator) = state.selected() {
                ui::gap(ui, theme::S3);
                ui::eyebrow(ui, "Exact locator selected");
                ui.add(egui::Label::new(theme::data(locator, Role::Data.size(), theme::TEXT)).wrap());
            }
        });
    ui::gap(ui, theme::S3);
    let dismiss_label = if state.pending.is_some() || state.result.is_some() {
        "Dismiss"
    } else {
        "Cancel"
    };
    let responses = ui::control_row(
        ui,
        vec![
            Control::primary("Restore this resource").enabled(state.can_request(works)),
            Control::quiet(dismiss_label),
        ],
    );
    if cfg!(test) {
        ui.data_mut(|data| {
            data.insert_temp(egui::Id::new("scope-operation-rect"), responses[0].rect);
            data.insert_temp(egui::Id::new("scope-dismiss-rect"), responses[1].rect);
        });
    }
    if responses[1].clicked() {
        state.dismiss();
    }
    responses[0].clicked() && state.can_request(works)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rack::{self, RackAction};
    use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
    use evo_daemon::persistence::load_persisted_observations;
    use evo_daemon::runtime::VerticalPipeline;
    use evo_daemon::threads::display_threads_from_storage;
    use evo_observation::provenance::ObservationSource;
    use evo_storage::Storage;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::SystemTime;

    fn work(id: u64, resources: &[&str]) -> DisplayThread {
        DisplayThread {
            thread_id: id,
            name: format!("Work {id}"),
            bundle: evo_threads::ResumeBundle {
                resume_point: "resume point is not a resource choice".into(),
                resume_reason: String::new(),
                restore_set: resources.iter().map(|r| r.to_string()).collect(),
                open_deltas: vec![],
            },
            is_work: true,
            engagement: 1.0,
            has_open_loop: false,
            urgency_reason: None,
            understanding: crate::understanding_fixture("Synthetic work"),
            last_active_ms: None,
            member_subjects: vec!["unrelated member must not be offered".into()],
            skipped_members: vec![],
        }
    }

    #[test]
    fn offers_all_distinct_eligible_restore_locators_and_no_other_material() {
        let long = "é".repeat(2049);
        let work = work(
            42,
            &[
                "https://a.test/1",
                "/notes/2",
                "Window 3",
                "app:4",
                "file:///5",
                "https://a.test/1",
                "",
                " \n\t",
                "Downloaded:/output",
                &long,
            ],
        );
        assert_eq!(
            eligible_resources(&work),
            [
                "https://a.test/1",
                "/notes/2",
                "Window 3",
                "app:4",
                "file:///5"
            ]
        );
    }

    #[test]
    fn exact_choice_has_no_implicit_default_and_targets_only_its_work() {
        let works = [
            work(
                42,
                &["file:///notes/café\npart two.md", "https://a.test/other"],
            ),
            work(7, &["https://other.test/"]),
        ];
        let mut state = ScopedRestoreState::default();
        state.open_for(42, &works);
        assert!(!state.can_request(&works));
        assert!(!state.choose("café", &works));
        assert!(!state.choose("https://other.test/", &works));
        assert!(state.choose("file:///notes/café\npart two.md", &works));
        let request = state.begin_request(&works).unwrap();
        assert_eq!(request.thread_id, 42);
        assert_eq!(request.locator, "file:///notes/café\npart two.md");
        assert!(state.begin_request(&works).is_none(), "no double dispatch");
        assert!(!state.choose("https://a.test/other", &works));
    }

    #[test]
    fn work_and_resource_reordering_preserve_exact_identity_and_fresh_name() {
        let works = [work(42, &["one", "two"]), work(7, &["elsewhere"])];
        let mut state = ScopedRestoreState::default();
        state.open_for(42, &works);
        state.choose("two", &works);
        let mut reordered = [work(7, &["elsewhere"]), work(42, &["two", "one"])];
        reordered[1].name = "Renamed work".into();
        let request = state.begin_request(&reordered).unwrap();
        assert_eq!(request.thread_id, 42);
        assert_eq!(request.work_name, "Renamed work");
        assert_eq!(request.locator, "two");
    }

    #[test]
    fn stale_choice_disappearance_empty_ineligible_and_duplicate_work_refuse_dispatch() {
        let initial = [work(42, &["one", "two"])];
        for changed in [
            vec![],
            vec![work(7, &["one"])],
            vec![work(42, &["two"])],
            vec![work(42, &[])],
            vec![DisplayThread {
                is_work: false,
                ..work(42, &["one"])
            }],
            vec![work(42, &["one"]), work(42, &["one"])],
        ] {
            let mut state = ScopedRestoreState::default();
            state.open_for(42, &initial);
            assert!(state.choose("one", &initial));
            assert!(!state.can_request(&changed));
            assert!(state.begin_request(&changed).is_none());
            assert!(state.selected().is_none());
            assert!(state.pending().is_none());
            // It must be selected anew even if a later refresh brings it back.
            assert!(!state.can_request(&initial));
        }
    }

    #[test]
    fn navigation_cancel_and_completion_do_not_leave_an_armed_choice() {
        let works = [work(42, &["one"]), work(7, &["two"])];
        let mut state = ScopedRestoreState::default();
        state.open_for(42, &works);
        state.choose("one", &works);
        state.dismiss();
        assert!(state.begin_request(&works).is_none());
        state.open_for(42, &works);
        assert!(!state.can_request(&works));
        state.choose("one", &works);
        state.open_for(7, &works);
        assert!(state.selected().is_none());
        state.choose("two", &works);
        let request = state.begin_request(&works).unwrap();
        state.complete(request.id + 1, Err("wrong response id".into()));
        assert!(state.pending().is_some());
        state.complete(
            request.id,
            Ok(VoiceOutcome::Failed {
                reason: "Refused exact resource".into(),
            }),
        );
        assert!(matches!(
            state.result.as_ref().unwrap().outcome,
            ScopedOutcome::Failed(_)
        ));
        assert!(state.begin_request(&works).is_none());
    }

    #[test]
    fn timeout_and_dismiss_keep_pending_identity_until_the_actual_response() {
        let works = [work(42, &["one"]), work(7, &["two"])];
        let mut restore = ScopedRestore::default();
        restore.state.open_for(42, &works);
        restore.state.choose("one", &works);
        let request = restore.state.begin_request(&works).unwrap();
        let (sender, receiver) = mpsc::channel();
        restore.receiver = Some(receiver);
        let now = Instant::now();
        restore.started_at = Some(now);
        restore.poll_at(now + RESPONSE_TIMEOUT);
        assert_eq!(
            restore.state.result.as_ref().unwrap().outcome,
            ScopedOutcome::TimedOut
        );
        restore.state.dismiss();
        restore.state.open_for(7, &works);
        assert_eq!(restore.state.thread_id(), Some(42));
        assert!(restore.state.begin_request(&works).is_none());
        sender
            .send(Completion {
                request_id: request.id,
                response: Ok(VoiceOutcome::Restored {
                    title: "Late result".into(),
                    summary: "Dispatched 1; verification unavailable.".into(),
                    offered: vec![],
                }),
            })
            .unwrap();
        restore.poll_at(now + RESPONSE_TIMEOUT + Duration::from_secs(1));
        assert!(restore.state.pending().is_none());
        assert_eq!(restore.state.result.as_ref().unwrap().request, request);
        assert!(matches!(
            restore.state.result.as_ref().unwrap().outcome,
            ScopedOutcome::Restored { .. }
        ));
        assert!(!restore.state.can_request(&works));
    }

    fn socket_fixture() -> (std::path::PathBuf, std::path::PathBuf, UnixListener) {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "evo-scoped-ui-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let path = evo_daemon::voice::voice_socket_path(&root);
        let listener = UnixListener::bind(&path).unwrap();
        (root, path, listener)
    }

    #[test]
    fn worker_sends_one_exact_resource_over_socket_and_reports_success_or_failure() {
        for success in [true, false] {
            let (root, path, listener) = socket_fixture();
            let (accepted_tx, accepted_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let locator = "file:///work/café\npart two.md";
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(socket.try_clone().unwrap());
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                assert_eq!(header, format!("RESOURCE 42 {}\n", locator.len()));
                let mut bytes = vec![0; locator.len()];
                reader.read_exact(&mut bytes).unwrap();
                assert_eq!(bytes, locator.as_bytes());
                accepted_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                let response = if success {
                    "OUTCOME RESTORED\nTITLE Exact work\nSUMMARY Dispatched 1; verification unavailable.\nEND\n"
                } else {
                    "OUTCOME FAILED\nREASON Exact surface no longer available.\nEND\n"
                };
                socket.write_all(response.as_bytes()).unwrap();
                listener.set_nonblocking(true).unwrap();
                assert!(
                    matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
                    "no CHOOSE or retry connection"
                );
            });
            let works = [
                work(7, &["elsewhere"]),
                work(42, &[locator, "https://other.test/"]),
            ];
            let ctx = egui::Context::default();
            let mut restore = ScopedRestore::default();
            restore.state.open_for(42, &works);
            restore.state.choose(locator, &works);
            assert!(restore.dispatch(&root, &works, &ctx));
            accepted_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            // The server is withholding its response, yet render/poll return.
            restore.poll(&ctx);
            assert!(restore.state.pending().is_some());
            assert!(!restore.dispatch(&root, &works, &ctx));
            restore.state.dismiss();
            restore.state.open_for(7, &works);
            assert!(!restore.dispatch(&root, &works, &ctx));
            release_tx.send(()).unwrap();
            server.join().unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while restore.state.pending().is_some() && Instant::now() < deadline {
                restore.poll(&ctx);
                std::thread::sleep(Duration::from_millis(1));
            }
            let result = restore
                .state
                .result
                .as_ref()
                .expect("worker result did not arrive");
            assert_eq!(result.request.thread_id, 42);
            assert_eq!(result.request.locator, locator);
            if success {
                assert!(
                    matches!(&result.outcome, ScopedOutcome::Restored { summary, .. } if summary == "Dispatched 1; verification unavailable.")
                );
            } else {
                assert_eq!(
                    result.outcome,
                    ScopedOutcome::Failed("Exact surface no longer available.".into())
                );
            }
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn stale_selection_never_connects_and_unreachable_socket_reports_request_failure() {
        let (root, path, listener) = socket_fixture();
        let ctx = egui::Context::default();
        let mut restore = ScopedRestore::default();
        let works = [work(42, &["one"])];
        restore.state.open_for(42, &works);
        restore.state.choose("one", &works);
        assert!(!restore.dispatch(&root, &[work(42, &["two"])], &ctx));
        listener.set_nonblocking(true).unwrap();
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        drop(listener);
        std::fs::remove_file(path).unwrap();
        restore.state.choose("one", &works);
        assert!(restore.dispatch(&root, &works, &ctx));
        let deadline = Instant::now() + Duration::from_secs(5);
        while restore.state.pending().is_some() && Instant::now() < deadline {
            restore.poll(&ctx);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            matches!(&restore.state.result.as_ref().unwrap().outcome, ScopedOutcome::RequestFailed(reason) if reason.contains("unreachable"))
        );
    }

    fn frame(
        ctx: &egui::Context,
        state: &mut ScopedRestoreState,
        works: &[DisplayThread],
        events: Vec<egui::Event>,
    ) -> bool {
        let mut requested = false;
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(380.0, 640.0),
                )),
                events,
                ..Default::default()
            },
            |ui| requested = sheet_contents(ui, state, works),
        );
        requested
    }

    fn click(
        ctx: &egui::Context,
        state: &mut ScopedRestoreState,
        works: &[DisplayThread],
        at: egui::Pos2,
    ) -> bool {
        frame(ctx, state, works, vec![egui::Event::PointerMoved(at)]);
        frame(
            ctx,
            state,
            works,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
        );
        frame(
            ctx,
            state,
            works,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        )
    }

    #[test]
    fn synthetic_pointer_choice_then_operation_is_exact_and_disabled_while_pending() {
        let works = [work(
            42,
            &["https://example.test/full/locator", "file:///notes/café.md"],
        )];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut state = ScopedRestoreState::default();
        state.open_for(42, &works);
        frame(&ctx, &mut state, &works, vec![]);
        let operation = ctx
            .data_mut(|d| d.get_temp::<egui::Rect>(egui::Id::new("scope-operation-rect")))
            .unwrap();
        assert!(!click(&ctx, &mut state, &works, operation.center()));
        let id = egui::Id::new(("evo-scope-resource", Some(42_u64), "file:///notes/café.md"));
        let choice = ctx.read_response(id).unwrap().rect;
        assert!(!click(&ctx, &mut state, &works, choice.center()));
        assert_eq!(state.selected(), Some("file:///notes/café.md"));
        let operation = ctx
            .data_mut(|d| d.get_temp::<egui::Rect>(egui::Id::new("scope-operation-rect")))
            .unwrap();
        assert!(click(&ctx, &mut state, &works, operation.center()));
        let request = state.begin_request(&works).unwrap();
        assert_eq!(request.thread_id, 42);
        assert_eq!(request.locator, "file:///notes/café.md");
        frame(&ctx, &mut state, &works, vec![]);
        let operation = ctx
            .data_mut(|d| d.get_temp::<egui::Rect>(egui::Id::new("scope-operation-rect")))
            .unwrap();
        assert!(!click(&ctx, &mut state, &works, operation.center()));
        let dismiss = ctx
            .data_mut(|d| d.get_temp::<egui::Rect>(egui::Id::new("scope-dismiss-rect")))
            .unwrap();
        assert!(!click(&ctx, &mut state, &works, dismiss.center()));
        assert!(!state.visible);
        assert!(state.pending().is_some());
    }

    #[test]
    fn narrow_sheet_wraps_long_locator_keeps_operation_and_cancel_in_view() {
        let locator = format!(
            "https://example.test/{}",
            "long-resource-without-spaces".repeat(70)
        );
        let works = [work(42, &[&locator, "second", "third", "fourth", "fifth"])];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut state = ScopedRestoreState::default();
        state.open_for(42, &works);
        frame(&ctx, &mut state, &works, vec![]);
        let resource = ctx
            .read_response(egui::Id::new((
                "evo-scope-resource",
                Some(42_u64),
                locator.as_str(),
            )))
            .unwrap()
            .rect;
        assert!(
            resource.width() <= 380.0,
            "full locator wraps instead of forcing wider sheet"
        );
        assert!(resource.height() > theme::CONTROL_HEIGHT);
        let operation = ctx
            .data_mut(|d| d.get_temp::<egui::Rect>(egui::Id::new("scope-operation-rect")))
            .unwrap();
        let cancel = ctx
            .data_mut(|d| d.get_temp::<egui::Rect>(egui::Id::new("scope-dismiss-rect")))
            .unwrap();
        assert!(
            operation.bottom() <= 640.0 && cancel.bottom() <= 640.0,
            "scrolling resources leave actions visible"
        );
        assert!(!click(&ctx, &mut state, &works, cancel.center()));
        assert!(!state.visible);
        assert!(state.pending().is_none());
    }

    #[test]
    fn native_sheet_fits_narrow_viewport_and_escape_dismisses_without_a_request() {
        let works = [work(42, &["https://example.test/one"])];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut state = ScopedRestoreState::default();
        state.open_for(42, &works);
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(380.0, 640.0));
        for _ in 0..3 {
            let _ = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(viewport),
                    ..Default::default()
                },
                |ui| {
                    assert!(!sheet(ui.ctx(), &mut state, &works));
                },
            );
        }
        let rect = ctx
            .data_mut(|d| d.get_temp::<egui::Rect>(egui::Id::new("scope-sheet-rect")))
            .unwrap();
        assert!(
            viewport.contains_rect(rect),
            "sheet, including its frame, fits the narrow viewport: {rect:?}"
        );
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(viewport),
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: Some(egui::Key::Escape),
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }],
                ..Default::default()
            },
            |ui| assert!(!sheet(ui.ctx(), &mut state, &works)),
        );
        assert!(!state.visible);
        assert!(state.pending().is_none());
    }

    /// One deterministic receipt through the production seams:
    ///
    ///   platform signal → capture normalization/acceptance → daemon
    ///   persistence/index → daemon replay → desktop state → rack selection
    ///   → scoped picker → VoiceSocketClient → daemon-shaped receipt
    ///
    /// The socket peer is deliberately only the final protocol receipt. It
    /// asserts the exact replayed work id and locator sent by the desktop;
    /// calling the real OS executor is outside this test's deterministic
    /// boundary and remains unverified.
    #[test]
    fn replayed_work_reaches_the_rack_picker_and_returns_its_exact_receipt() {
        fn accepted(
            engine: &mut CaptureEngine,
            signal: MacOSSignal,
        ) -> evo_observation::observation::Observation {
            let source = match &signal {
                MacOSSignal::FileSaved { .. } => "macos_fsevents",
                MacOSSignal::InputActivity { .. } => "macos_input",
                _ => "macos_accessibility",
            };
            let adapter = MacOSAdapter::new(ObservationSource::new(source).unwrap());
            let raw = adapter
                .normalize(signal)
                .expect("synthetic signal normalizes")
                .expect("synthetic signal maps to a canonical concept");
            let schema = raw.schema();
            engine
                .ingest(raw, &schema)
                .expect("normalized signal is accepted")
        }

        fn frame_rack(
            ctx: &egui::Context,
            works: &[DisplayThread],
            stack_open: &mut bool,
            merge_armed: &mut Option<usize>,
            events: Vec<egui::Event>,
        ) -> Option<RackAction> {
            let mut action = None;
            let _ = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let mut query = String::new();
                    action = rack::rack(ui, works, &[], stack_open, merge_armed, &mut query);
                },
            );
            action
        }

        fn pointer_events(at: egui::Pos2) -> [Vec<egui::Event>; 3] {
            [
                vec![egui::Event::PointerMoved(at)],
                vec![egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                }],
                vec![egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                }],
            ]
        }

        fn frame_sheet(
            ctx: &egui::Context,
            state: &mut ScopedRestoreState,
            works: &[DisplayThread],
            events: Vec<egui::Event>,
        ) -> bool {
            let mut requested = false;
            let _ = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| requested = sheet_contents(ui, state, works),
            );
            requested
        }

        let root = std::env::temp_dir().join(format!(
            "evo-desktop-receipt-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let _guard = Storage::with_thread_root(root.clone());
        let (boundary_sender, boundary_receiver) = mpsc::channel();
        let mut daemon = VerticalPipeline::new(boundary_sender, root.clone())
            .expect("daemon pipeline replays an empty store");
        let mut capture = CaptureEngine::new();
        // A fixed future epoch keeps the replayed work inside the display
        // path's freshness window without making the test depend on the wall
        // clock. `saturating_sub` treats it as fresh on today's platform.
        let base_epoch = 4_000_000_000_u64;
        let mut observed_at = 0u64;
        for _sitting in 0..2 {
            for _pass in 0..2 {
                for signal in [
                    MacOSSignal::WindowFocusGained {
                        subject: "Quarterly Forecast — Spreadsheet".into(),
                        observed_at: SystemTime::UNIX_EPOCH
                            + Duration::from_secs(base_epoch + observed_at),
                        process_identifier: None,
                        owning_process_name: None,
                        observed_state: None,
                        window_id: None,
                    },
                    MacOSSignal::FileSaved {
                        subject: "/Users/alice/finance/quarterly-forecast.numbers".into(),
                        observed_at: SystemTime::UNIX_EPOCH
                            + Duration::from_secs(base_epoch + observed_at + 200),
                    },
                    MacOSSignal::WindowFocusGained {
                        subject: "quarterly forecast reconcile — Terminal".into(),
                        observed_at: SystemTime::UNIX_EPOCH
                            + Duration::from_secs(base_epoch + observed_at + 220),
                        process_identifier: None,
                        owning_process_name: None,
                        observed_state: None,
                        window_id: None,
                    },
                    MacOSSignal::URLNavigated {
                        subject: "https://example.test/quarterly-forecast".into(),
                        observed_at: SystemTime::UNIX_EPOCH
                            + Duration::from_secs(base_epoch + observed_at + 300),
                    },
                    MacOSSignal::InputActivity {
                        subject: Some("https://example.test/quarterly-forecast".into()),
                        keys: 100,
                        clicks: 0,
                        scrolls: 0,
                        observed_at: SystemTime::UNIX_EPOCH
                            + Duration::from_secs(base_epoch + observed_at + 500),
                    },
                ] {
                    let observation = accepted(&mut capture, signal);
                    assert!(!observation.provenance().is_execution_origin());
                    daemon.handle_observation(observation);
                    observed_at += 600;
                }
            }
            observed_at += 4 * 60 * 60;
        }
        assert!(
            load_persisted_observations(&root)
                .expect("canonical observations load")
                .len()
                >= 20,
            "capture observations must be durably present before replay"
        );
        daemon.settle();
        assert_eq!(
            boundary_receiver.try_iter().count(),
            1,
            "the daemon announces one settled body of work: workspaces={:?}, standings={:?}, outcomes={:?}",
            daemon.index().workspaces(),
            daemon.index().standings(),
            daemon.index().outcomes(),
        );

        // These are the two independent desktop readers: the canonical index
        // projection and the engine replay used by the rack.
        let display = crate::state::load_display_state(&root).expect("desktop state replays");
        assert!(
            display.display.is_some(),
            "replayed evidence reaches desktop state"
        );
        assert!(
            display.selection.is_some(),
            "desktop derives a restore selection"
        );
        let works = display_threads_from_storage(&root).expect("rack replay succeeds");
        assert!(
            works.len() == 1,
            "synthetic evidence produces one rack work: works={works:?}, engine={:?}",
            evo_daemon::threads::engine_from_storage(&root)
                .expect("engine replay")
                .threads()
        );
        let work = &works[0];
        let locator = eligible_resources(work)
            .into_iter()
            .next()
            .expect("the replayed work exposes an eligible exact locator")
            .to_string();

        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut stack_open = false;
        let mut merge_armed = None;
        frame_rack(&ctx, &works, &mut stack_open, &mut merge_armed, vec![]);
        let rack_rect = ctx
            .data_mut(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new((
                    "rack-scoped-restore-rect",
                    work.thread_id,
                )))
            })
            .expect("the replayed work exposes the scoped restore control");
        let mut rack_action = None;
        for events in pointer_events(rack_rect.center()) {
            rack_action = frame_rack(&ctx, &works, &mut stack_open, &mut merge_armed, events);
        }
        assert_eq!(
            rack_action,
            Some(RackAction::RestoreSelected(work.thread_id))
        );

        let mut restore = ScopedRestore::default();
        restore.state.open_for(work.thread_id, &works);
        frame_sheet(&ctx, &mut restore.state, &works, vec![]);
        let resource_rect = ctx
            .read_response(egui::Id::new((
                "evo-scope-resource",
                Some(work.thread_id),
                locator.as_str(),
            )))
            .expect("the picker renders the replayed locator")
            .rect;
        for events in pointer_events(resource_rect.center()) {
            frame_sheet(&ctx, &mut restore.state, &works, events);
        }
        assert_eq!(restore.state.selected(), Some(locator.as_str()));

        let socket = evo_daemon::voice::voice_socket_path(&root);
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("receipt socket binds");
        let (request_sender, request_receiver) = mpsc::channel();
        let expected_work = work.thread_id;
        let expected_locator = locator.clone();
        let server_locator = expected_locator.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("desktop connects to daemon");
            let mut reader = BufReader::new(stream.try_clone().expect("clone receipt socket"));
            let mut header = String::new();
            reader.read_line(&mut header).expect("resource header");
            assert_eq!(
                header,
                format!("RESOURCE {expected_work} {}\n", server_locator.len())
            );
            let mut body = vec![0; server_locator.len()];
            reader.read_exact(&mut body).expect("resource body");
            assert_eq!(body, server_locator.as_bytes());
            request_sender
                .send(())
                .expect("request assertion completes");
            stream
                .write_all(b"OUTCOME RESTORED\nTITLE Quarterly Forecast\nSUMMARY Dispatched 1; verification unavailable.\nEND\n")
                .expect("receipt response");
        });

        assert!(restore.dispatch(&root, &works, &ctx));
        request_receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("daemon receipt request arrives");
        server.join().expect("receipt server exits");
        let deadline = Instant::now() + Duration::from_secs(5);
        while restore.state.pending().is_some() && Instant::now() < deadline {
            restore.poll(&ctx);
            std::thread::sleep(Duration::from_millis(1));
        }
        let result = restore
            .state
            .result
            .as_ref()
            .expect("receipt reaches picker");
        assert_eq!(result.request.thread_id, expected_work);
        assert_eq!(result.request.locator, expected_locator);
        assert_eq!(
            result.outcome,
            ScopedOutcome::Restored {
                title: "Quarterly Forecast".into(),
                summary: "Dispatched 1; verification unavailable.".into(),
            }
        );

        let _ = std::fs::remove_file(socket);
        let _ = std::fs::remove_dir_all(root);
    }
}
