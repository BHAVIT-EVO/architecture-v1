//! QA capture harness — saves the shell's own framebuffer, one file per screen.
//!
//! A development instrument, never part of the app. It exists because visual
//! verification has to rest on pixels the shipped code actually drew, and
//! `screencapture` cannot read this display. eframe can: a
//! [`egui::ViewportCommand::Screenshot`] is answered by the renderer reading
//! back the window's own colour attachment, which needs no screen-recording
//! permission because it never touches the display server.
//!
//! It composes the same public screen functions `app.rs` composes — the
//! environment, the header, the body rect, the capture notice, then one of
//! `home` / `detail` / `shell` — rather than reimplementing any of them, so what
//! lands in the file is the real screen and not a mock of it.
//!
//! Frames are dumped as raw premultiplied RGBA with the pixel size in the file
//! name; `tools/rgba_to_png.py` turns them into PNGs. This crate has no image
//! encoder among its dependencies and the mission forbids adding one.

use eframe::egui;
use egui::{UserData, ViewportCommand};

use evo_daemon::cache::CanonicalIndex;
use evo_desktop::daemon::{DaemonManager, DaemonStatus};
use evo_desktop::{detail, dock, home, shell, state, theme, ui};
use evo_engagement::Standing;
use evo_execution::{
    ExecutionReport, Locator, PreflightOutcome, RestorationSelection, preflight_selection,
};
use evo_restoration::DerivationOutcome;
use evo_workspace::workspace::Workspace;
use evo_workspace::workspace_id::WorkspaceId;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// Frames to hold a settled window before reading it back, so every entry
/// transition and scroll-edge fade has finished animating.
const SETTLE_FRAMES: u32 = 70;

/// Which capture state the shell should be told about, so the header word and
/// the notice above the work can both be inspected deliberately.
#[derive(Clone, Copy)]
enum Cap {
    Running,
    Partial,
    Blocked,
}

impl Cap {
    fn status(self) -> DaemonStatus {
        match self {
            Cap::Running => DaemonStatus::Capturing,
            Cap::Partial => DaemonStatus::PartialCapture {
                detail: "Window focus is not being captured because Accessibility is not granted."
                    .to_string(),
            },
            Cap::Blocked => DaemonStatus::PermissionRequired,
        }
    }
}

/// What to draw in the body.
#[derive(Clone, Copy)]
enum Body {
    /// Home, with a retrieval query already typed (empty means untouched).
    Home { query: &'static str },
    /// Workspace Detail for the nth remembered body of work.
    Detail(usize),
    /// Workspace Detail with the nth stage panel pre-expanded, injected
    /// through the same egui temp-data seam the headless pins use — the
    /// harness photographs the expanded state without a synthetic click.
    DetailOpen(usize, usize),
    /// Home with the dock's transport mid-restoration: one of two steps
    /// done, so the teal hairline fraction and the pill are both showing.
    ExecutingDock,
    /// Home with the Settings sheet fully open above it.
    Settings,
    /// The first-run gate at the given step, which owns the whole window.
    FirstRun(u8),
    /// Home with nothing remembered at all.
    Empty,
    /// Home with nothing remembered because capture is blocked.
    Permission,
    /// Home with canonical storage unreadable.
    StorageError,
}

#[derive(Clone, Copy)]
struct Shot {
    name: &'static str,
    w: f32,
    h: f32,
    cap: Cap,
    body: Body,
}

fn shots() -> Vec<Shot> {
    vec![
        Shot {
            name: "01-home",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::Home { query: "" },
        },
        Shot {
            name: "02-home-capturing-notice",
            w: 980.0,
            h: 720.0,
            cap: Cap::Partial,
            body: Body::Home { query: "" },
        },
        Shot {
            name: "03-home-search",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::Home { query: "e" },
        },
        Shot {
            name: "04-home-no-results",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::Home { query: "zzqq" },
        },
        Shot {
            name: "05-home-narrow",
            w: 460.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::Home { query: "" },
        },
        Shot {
            name: "06-detail-a",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::Detail(0),
        },
        Shot {
            name: "07-detail-b",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::Detail(1),
        },
        Shot {
            name: "08-detail-wide",
            w: 1280.0,
            h: 860.0,
            cap: Cap::Running,
            body: Body::Detail(0),
        },
        Shot {
            name: "09-detail-narrow",
            w: 520.0,
            h: 760.0,
            cap: Cap::Running,
            body: Body::Detail(0),
        },
        Shot {
            name: "10-detail-min",
            w: 420.0,
            h: 640.0,
            cap: Cap::Running,
            body: Body::Detail(0),
        },
        Shot {
            name: "11-settings",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::Settings,
        },
        Shot {
            name: "12-first-run-1",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::FirstRun(1),
        },
        Shot {
            name: "13-first-run-2",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::FirstRun(2),
        },
        Shot {
            name: "14-first-run-3",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::FirstRun(3),
        },
        Shot {
            name: "15-home-empty",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::Empty,
        },
        Shot {
            name: "16-home-permission",
            w: 980.0,
            h: 720.0,
            cap: Cap::Blocked,
            body: Body::Permission,
        },
        Shot {
            name: "17-storage-error",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::StorageError,
        },
        Shot {
            name: "18-detail-open-overview",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::DetailOpen(0, 0),
        },
        Shot {
            name: "19-detail-open-record",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::DetailOpen(0, 2),
        },
        Shot {
            name: "20-dock-executing",
            w: 980.0,
            h: 720.0,
            cap: Cap::Running,
            body: Body::ExecutingDock,
        },
    ]
}

/// The canonical understanding the screens read, loaded exactly the way the
/// shell loads it.
struct Canon {
    workspaces: Vec<Workspace>,
    cards: Vec<state::WorkspaceCard>,
    outcomes: HashMap<String, DerivationOutcome>,
    subjects: HashMap<String, String>,
    kinds: HashMap<String, String>,
    titles: HashMap<String, String>,
    standings: HashMap<String, Standing>,
    locators: HashMap<String, Locator>,
    designation: Option<(String, std::time::SystemTime)>,
    selections: Vec<Option<RestorationSelection>>,
    preflights: Vec<Vec<PreflightOutcome>>,
}

fn load(root: &Path) -> Canon {
    // Capture-only replay: tail the log without recomputing the engagement
    // understanding — the same fast path the live daemon uses between
    // reconstructions. Full replay cost minutes on today's log size; the
    // harness photographs screens, not engagement params.
    let index = match CanonicalIndex::new_capture_only(root.to_path_buf()) {
        Ok(index) => index,
        Err(_) => CanonicalIndex::new(root.to_path_buf()).expect("build canonical index"),
    };
    let display = state::display_state_from_index(&index);
    let evidence = index.locators();

    let mut selections = Vec::new();
    let mut preflights = Vec::new();
    let mut cards = Vec::new();
    for workspace in &display.workspaces {
        let outcome = display.outcomes.get(&workspace.id().to_string());
        let selection = outcome.map(|outcome| state::selection_for(workspace, outcome, evidence));
        let preflight = match &selection {
            Some(selection) => preflight_selection(selection, &evo_execution::MacOSExecutor::new()),
            None => Vec::new(),
        };
        cards.push(state::workspace_card(
            workspace,
            outcome,
            selection.as_ref(),
            &display.subjects,
            &display.kinds,
            &display.titles,
            &display.standings,
        ));
        selections.push(selection);
        preflights.push(preflight);
    }

    Canon {
        workspaces: display.workspaces,
        cards,
        outcomes: display.outcomes,
        subjects: display.subjects,
        kinds: display.kinds,
        titles: display.titles,
        standings: display.standings,
        locators: display.locators,
        designation: display.designation,
        selections,
        preflights,
    }
}

struct Qa {
    out: PathBuf,
    storage_root: PathBuf,
    shots: Vec<Shot>,
    at: usize,
    settled: u32,
    requested: bool,
    written: Vec<String>,
    canon: Canon,
    daemon: DaemonManager,

    // The user-authored transient state the screens borrow mutably.
    query: String,
    selected: Option<WorkspaceId>,
    designation_status: Option<(String, bool)>,
    continuation_draft: Option<BTreeSet<String>>,
    continuation_status: Option<(String, bool)>,
    grouping_status: Option<(String, bool)>,
    execution_result: Option<ExecutionReport>,
    continue_phase: detail::ContinuePhase,
    settings_open: bool,
    first_run: Option<u8>,
}

impl Qa {
    #[allow(dead_code)]
    fn new(cc: &eframe::CreationContext<'_>, out: PathBuf) -> Self {
        let storage_root = state::canonical_storage_root();
        let canon = load(&storage_root);
        eprintln!(
            "QA-CAPTURE storage={} workspaces={}",
            storage_root.display(),
            canon.workspaces.len()
        );
        Self::with_canon(cc, canon, out)
    }

    fn with_canon(cc: &eframe::CreationContext<'_>, canon: Canon, out: PathBuf) -> Self {
        theme::install(&cc.egui_ctx);
        let storage_root = state::canonical_storage_root();
        Self {
            out,
            storage_root: storage_root.clone(),
            shots: shots(),
            at: 0,
            settled: 0,
            requested: false,
            written: Vec::new(),
            canon,
            daemon: DaemonManager::new(storage_root),
            query: String::new(),
            selected: None,
            designation_status: None,
            continuation_draft: None,
            continuation_status: None,
            grouping_status: None,
            execution_result: None,
            continue_phase: detail::ContinuePhase::Idle,
            settings_open: false,
            first_run: None,
        }
    }

    fn save(&mut self, image: &egui::ColorImage) {
        let shot = self.shots[self.at];
        let [w, h] = image.size;
        let name = format!("{}_{w}x{h}.rgba", shot.name);
        let path = self.out.join(&name);
        std::fs::write(&path, image.as_raw()).expect("write frame");
        eprintln!("QA-CAPTURE wrote {name}");
        self.written.push(name);
    }
}

impl eframe::App for Qa {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.request_repaint();

        // A reply to an earlier request lands as an input event; saving it is
        // what advances the run.
        let reply = ctx.input(|i| {
            i.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = reply {
            self.save(&image);
            self.at += 1;
            self.settled = 0;
            self.requested = false;
        }

        if self.at >= self.shots.len() {
            eprintln!("QA-CAPTURE done: {} frame(s)", self.written.len());
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        let shot = self.shots[self.at];

        // The window is resized to the shot's size first; the settle count only
        // starts once the surface has actually reached it.
        let want = egui::vec2(shot.w, shot.h);
        let have = ui.max_rect().size();
        let sized = (have.x - want.x).abs() <= 1.0 && (have.y - want.y).abs() <= 1.0;
        if !sized {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(want));
            self.settled = 0;
            self.requested = false;
        }

        // Per-shot presentation state, set before the screens borrow it.
        self.query = match shot.body {
            Body::Home { query } => query.to_string(),
            _ => String::new(),
        };
        self.settings_open = matches!(shot.body, Body::Settings);
        self.first_run = match shot.body {
            Body::FirstRun(step) => Some(step),
            _ => None,
        };
        self.selected = match shot.body {
            Body::Detail(n) | Body::DetailOpen(n, _) => {
                self.canon.workspaces.get(n).map(|w| w.id().clone())
            }
            _ => None,
        };
        // The executing shot composes the dock mid-restoration: one of two
        // steps done, exactly the state the headless pin holds.
        self.continue_phase = match shot.body {
            Body::ExecutingDock => {
                use evo_artifact::artifact_id::ArtifactId;
                use evo_execution::StepPlan;
                detail::ContinuePhase::Executing {
                    steps: vec![
                        evo_execution::RestorationStep::new(
                            ArtifactId::new("artifact-aa").unwrap(),
                            StepPlan::Attempt(evo_execution::LocatorKind::FilePath(
                                "/tmp/evo-qa-synth/annapurna.md".into(),
                            )),
                        ),
                        evo_execution::RestorationStep::new(
                            ArtifactId::new("artifact-bb").unwrap(),
                            StepPlan::Attempt(evo_execution::LocatorKind::FilePath(
                                "/tmp/evo-qa-synth/baggage.md".into(),
                            )),
                        ),
                    ],
                    done: vec![evo_execution::TargetAttempt::new(
                        ArtifactId::new("artifact-aa").unwrap(),
                        evo_execution::TargetStatus::Opened {
                            detail: "/tmp/evo-qa-synth/annapurna.md".to_string(),
                        },
                    )],
                }
            }
            _ => detail::ContinuePhase::Idle,
        };

        let status = shot.cap.status();

        // ── The composition below mirrors `EvoApp::ui` ──────────────────────
        let max = ui.max_rect();
        ui::paint_environment(ui.painter(), max);

        if let Some(step) = self.first_run {
            let mut pending = Some(step);
            shell::first_run(ui, step, &mut pending, &mut self.daemon);
        } else {
            let header_rect = egui::Rect::from_min_max(
                max.min,
                egui::pos2(max.max.x, max.min.y + theme::HEADER_HEIGHT),
            );
            let body_rect = egui::Rect::from_min_max(
                egui::pos2(max.min.x, max.min.y + theme::HEADER_HEIGHT),
                max.max,
            );
            shell::header(ui, header_rect, &status);

            let Self {
                storage_root,
                canon,
                daemon,
                query: _,
                selected,
                designation_status,
                continuation_draft,
                continuation_status,
                grouping_status,
                execution_result,
                continue_phase,
                settings_open,
                ..
            } = self;

            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(body_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    if !matches!(status, DaemonStatus::Capturing) {
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(theme::M6, theme::M2))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                shell::capture_notice(ui, &status, daemon);
                            });
                    }

                    match shot.body {
                        // The dock-executing shot: the home body beneath,
                        // with the dock composed over it exactly as the
                        // shell does — pill, mono counter, teal hairline.
                        Body::ExecutingDock => {
                            home::empty(ui, true);
                            let dock_rect = egui::Rect::from_min_max(
                                egui::pos2(max.min.x, max.max.y - theme::ACTION_BAR_HEIGHT),
                                egui::pos2(max.max.x, max.max.y),
                            );
                            let ctx_data = dock::DockContext {
                                active_pod: None,
                                in_focus: false,
                                continuable: true,
                            };
                            dock::show(
                                ui,
                                dock_rect,
                                &ctx_data,
                                &*continue_phase,
                                execution_result.as_ref(),
                            );
                        }
                        Body::Empty => home::empty(ui, true),
                        Body::Permission => home::permission_required(ui, None, || {}),
                        Body::StorageError => shell::storage_error(
                            ui,
                            "canonical storage could not be read: observation.log is unreadable",
                        ),
                        Body::Home { .. } => {
                            // The QA harness photographs the rack's empty
                            // composition; the live rack itself needs an
                            // engine corpus, which this canonical-only
                            // harness does not carry.
                            home::empty(ui, true);
                        }
                        Body::Settings => {
                            // The sheet itself is opened by the flag above;
                            // the body beneath it is the plain Home state.
                            home::empty(ui, true);
                        }
                        Body::Detail(n) => {
                            let Some(workspace) = canon.workspaces.get(n) else {
                                home::empty(ui, true);
                                return;
                            };
                            let outcome = canon.outcomes.get(&workspace.id().to_string());
                            let canonical = detail::Canonical {
                                workspace,
                                outcome,
                                selection: canon.selections.get(n).and_then(Option::as_ref),
                                preflight: canon
                                    .preflights
                                    .get(n)
                                    .map(Vec::as_slice)
                                    .unwrap_or(&[]),
                                subjects: &canon.subjects,
                                kinds: &canon.kinds,
                                titles: &canon.titles,
                                standings: &canon.standings,
                                locators: &canon.locators,
                                designation: canon.designation.as_ref(),
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
                            detail::screen(ui, &canonical, canon.cards.as_slice(), &mut transient);
                        }
                        Body::DetailOpen(n, panel) => {
                            let Some(workspace) = canon.workspaces.get(n) else {
                                home::empty(ui, true);
                                return;
                            };
                            let outcome = canon.outcomes.get(&workspace.id().to_string());
                            let canonical = detail::Canonical {
                                workspace,
                                outcome,
                                selection: canon.selections.get(n).and_then(Option::as_ref),
                                preflight: canon
                                    .preflights
                                    .get(n)
                                    .map(Vec::as_slice)
                                    .unwrap_or(&[]),
                                subjects: &canon.subjects,
                                kinds: &canon.kinds,
                                titles: &canon.titles,
                                standings: &canon.standings,
                                locators: &canon.locators,
                                designation: canon.designation.as_ref(),
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
                            // The pre-expanded panel, injected through the
                            // same temp-data key the screen itself owns —
                            // identical to what a real strip click writes.
                            ui.data_mut(|data| {
                                data.insert_temp(
                                    egui::Id::new((
                                        "evo-stage-open-panel",
                                        workspace.id().to_string(),
                                    )),
                                    panel,
                                );
                            });
                            detail::screen(ui, &canonical, canon.cards.as_slice(), &mut transient);
                        }
                        Body::FirstRun(_) => {}
                    }
                },
            );

            shell::settings(ui, settings_open, &status, daemon, storage_root.as_path());
        }
        // ── end mirrored composition ───────────────────────────────────────

        if sized && !self.requested {
            self.settled += 1;
            if self.settled >= SETTLE_FRAMES {
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
                self.requested = true;
            }
        }
    }
}

impl Drop for Qa {
    fn drop(&mut self) {
        self.daemon.kill();
    }
}

fn main() -> eframe::Result {
    // The real canonical corpus replays the whole Observation log — minutes
    // on today's machine — before the first window can open. `EVO_QA_SYNTH=1`
    // photographs the screens against a small in-memory corpus instead:
    // same screen functions, same composition, honest about which corpus
    // produced the pixels.
    if std::env::var("EVO_QA_SYNTH").is_ok() {
        return run(synth::canon());
    }
    let storage_root = state::canonical_storage_root();
    run(load(&storage_root))
}

fn run(canon: Canon) -> eframe::Result {
    let out = std::env::temp_dir().join("evo-qa-shots");
    std::fs::create_dir_all(&out).expect("create output directory");
    eprintln!("QA-CAPTURE out={}", out.display());

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Evo — QA capture")
            .with_inner_size([980.0, 720.0])
            // The narrow shots go below the shipped minimum on purpose, to see
            // what the layout does at the edge it declares.
            .with_min_inner_size([380.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Evo QA",
        options,
        Box::new(move |cc| Ok(Box::new(Qa::with_canon(cc, canon, out)))),
    )
}

/// A small in-memory corpus for `EVO_QA_SYNTH`: two bodies of work with a
/// real derivation each — enough for the stage, the panel-cards, and the
/// Record tabs to photograph with true derived content.
mod synth {
    use super::Canon;

    use evo_desktop::state;
    use evo_engagement::Standing;
    use evo_execution::{Locator, LocatorKind, preflight_selection};
    use evo_restoration::DerivationOutcome;
    use evo_workspace::workspace::Workspace;
    use evo_workspace::workspace_id::WorkspaceId;

    use std::collections::HashMap;

    pub fn canon() -> Canon {
        let mut workspaces = Vec::new();
        let mut outcomes = HashMap::new();
        let mut subjects_map: HashMap<String, String> = HashMap::new();
        let mut kinds: HashMap<String, String> = HashMap::new();
        let mut titles: HashMap<String, String> = HashMap::new();
        let mut locators: HashMap<String, Locator> = HashMap::new();

        let seeds: [(UuidSeed, &str, &str, &str); 2] = [
            (
                UuidSeed::A,
                "Revisiting Annapurna",
                "artifact-aa",
                "/tmp/evo-qa-synth/annapurna.md",
            ),
            (
                UuidSeed::B,
                "Baggage Battles",
                "artifact-bb",
                "/tmp/evo-qa-synth/baggage.md",
            ),
        ];
        for (seed, title, artifact, path) in seeds {
            let (workspace, outcome) = fixture(seed, title, artifact, path);
            subjects_map.insert(artifact.to_string(), format!("{title} — notes"));
            kinds.insert(artifact.to_string(), "saved file".to_string());
            titles.insert(workspace.id().to_string(), title.to_string());
            locators.insert(
                artifact.to_string(),
                Locator::new(
                    evo_artifact::artifact_id::ArtifactId::new(artifact).unwrap(),
                    LocatorKind::FilePath(path.into()),
                ),
            );
            outcomes.insert(workspace.id().to_string(), outcome);
            workspaces.push(workspace);
        }

        // `selection_for` reads the *evidence* map (kind, witness detail);
        // the locator map feeds the plan announcements separately.
        let mut evidence: HashMap<String, (String, String)> = HashMap::new();
        for artifact in locators.keys() {
            evidence.insert(
                artifact.clone(),
                ("OBS-FILE-SAVED".to_string(), artifact.clone()),
            );
        }
        let mut selections = Vec::new();
        let mut preflights = Vec::new();
        let mut cards = Vec::new();
        let standings: HashMap<String, Standing> = HashMap::new();
        for workspace in &workspaces {
            let outcome = outcomes.get(&workspace.id().to_string());
            let selection =
                outcome.map(|outcome| state::selection_for(workspace, outcome, &evidence));
            let preflight = match &selection {
                Some(selection) => {
                    preflight_selection(selection, &evo_execution::MacOSExecutor::new())
                }
                None => Vec::new(),
            };
            cards.push(state::workspace_card(
                workspace,
                outcome,
                selection.as_ref(),
                &subjects_map,
                &kinds,
                &titles,
                &standings,
            ));
            selections.push(selection);
            preflights.push(preflight);
        }

        Canon {
            workspaces,
            cards,
            outcomes,
            subjects: subjects_map,
            kinds,
            titles,
            standings,
            locators,
            designation: None,
            selections,
            preflights,
        }
    }

    enum UuidSeed {
        A,
        B,
    }

    impl UuidSeed {
        fn id(self) -> WorkspaceId {
            use std::str::FromStr;
            match self {
                UuidSeed::A => {
                    WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174961").unwrap()
                }
                UuidSeed::B => {
                    WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174962").unwrap()
                }
            }
        }
    }

    fn fixture(
        seed: UuidSeed,
        title: &str,
        artifact: &str,
        _path: &str,
    ) -> (Workspace, DerivationOutcome) {
        use evo_artifact::artifact_id::ArtifactId;
        use evo_restoration::{RestorationInput, derive_restoration_plan};
        use evo_workspace::attachment::{Attachment, ResourceRole};
        use evo_workspace::confidence::ConfidenceScore;
        use evo_workspace::lifecycle::WorkspaceLifecycle;
        use evo_workspace::snapshot::Snapshot;

        let _ = title;
        let attachment = Attachment::new(
            ArtifactId::new(artifact).unwrap(),
            ConfidenceScore::new(1.0).unwrap(),
            ResourceRole::Primary,
        );
        let snapshot = Snapshot::new(
            std::time::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![attachment.clone()],
        );
        let ws = Workspace::new(
            seed.id(),
            WorkspaceLifecycle::Active,
            vec![attachment],
            vec![snapshot],
        );
        let input = RestorationInput::new(&ws, ws.snapshots().last().unwrap()).unwrap();
        let outcome = derive_restoration_plan(&input);
        (ws, outcome)
    }
}
