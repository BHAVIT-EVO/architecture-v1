//! Containment: what entering a pod does to everything that isn't the pod.
//!
//! IS-0022's hide/park is preserved as modes; **Dim** (default) veils
//! foreign on-screen windows instead of removing them — the mind reads one
//! work at a time while nothing ever leaves the desktop. The rules are
//! absolute:
//!
//! * under-containment — a doubtful window stays fully visible;
//! * protection — Evo's own processes and system surfaces are never veiled,
//!   parked, or hidden by Evo;
//! * receipts — every verb the plan issues lands in the lease (see
//!   [`crate::lease`]) so leaving replays backwards exactly.

use crate::config::ContainMode;
use crate::pod::PodSurfaces;
use crate::stage::StageRecipe;
use crate::surface::{Frame, PodApp, PodWindow};

/// Why a window belongs to the pod — evidence kind, in match-strength
/// order: documents are produced work, pages are attended work, titles
/// are the coarsest witness.
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

    pub fn belongs(self) -> bool {
        self.rank() >= WindowMatch::Title.rank()
    }
}

/// Matches a live window against the work's own evidence, strongest rule
/// first. Nothing by application category.
pub fn window_match(window: &PodWindow, surfaces: &PodSurfaces) -> WindowMatch {
    if let Some(doc) = &window.ax_document {
        let doc_norm = strip_file_scheme(doc);
        for known in &surfaces.documents {
            let known_norm = strip_file_scheme(known);
            if doc_norm == known_norm {
                return WindowMatch::Document;
            }
        }
        for known in &surfaces.urls {
            if doc == known {
                return WindowMatch::Page;
            }
        }
    }
    if !window.title.is_empty() && surfaces.titles.iter().any(|t| t == &window.title) {
        return WindowMatch::Title;
    }
    WindowMatch::None
}

fn strip_file_scheme(s: &str) -> &str {
    s.strip_prefix("file://").unwrap_or(s)
}

/// Where the activation leaves windows of this pod that the recipe knows.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    pub pid: i32,
    pub window_id: u32,
    /// The frame to apply (None = raise only; the pod never guesses a
    /// placement for a window the recipe can't locate).
    pub frame: Option<Frame>,
    /// The resource this window matched, when any (recipe lookup key).
    pub resource: Option<String>,
    pub raise_rank: u8,
}

/// Every verb one activation would issue — the lease records this same
/// list as its receipt; tests assert this plan, not side effects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActivationPlan {
    /// Pod windows to arrange/raise, strongest last (hero lands on top).
    pub arrange: Vec<Arrangement>,
    /// Foreign windows of mixed apps to minimize (Park mode).
    pub park: Vec<(i32, u32)>,
    /// Foreign on-screen windows to veil (Dim mode).
    pub veil: Vec<(i32, u32, Frame)>,
    /// Foreign regular apps to hide (Hide mode).
    pub hide_apps: Vec<i32>,
    /// Pod's own apps that were hidden and must come back.
    pub unhide_apps: Vec<i32>,
    /// Pod app to make frontmost (hero's owner).
    pub activate: Option<i32>,
}

/// A plan is honestly impossible only when the surface can't be read —
/// an empty pod gets an empty plan (nothing moves, nothing veils; the
/// card said it had no surfaces).
#[derive(Debug, Clone, PartialEq)]
pub enum PlanError {
    Surface(String),
}

/// Computes the activation for `pod` in `mode`.
///
/// - `windows_all` already spans every visible regular app (providers may
///   pre-filter; the plan itself won't act on apps it wasn't shown except
///   through belonging evidence members).
/// - `protected` pids are never touched, even hidden=false & foreign.
/// `screen` is the pod's stage (the main screen's usable frame): recipe
/// ratios materialize THERE — once, per enter. Materializing a ratio per
/// window's own rect instead (an earlier mistake) staged windows onto
/// themselves rather than onto the stage.
pub fn plan_activation(
    surfaces: &PodSurfaces,
    recipe: Option<&StageRecipe>,
    windows_all: &[PodWindow],
    apps: &[PodApp],
    protected: &[i32],
    mode: ContainMode,
    screen: Option<&Frame>,
) -> ActivationPlan {
    let mut plan = ActivationPlan::default();

    // Pod membership by app: owns a belonging window, or witnessed app name.
    let mut pod_pid: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
    for app in apps {
        if surfaces.apps.iter().any(|n| n == &app.name) {
            pod_pid.insert(app.pid);
        }
    }

    // Own windows: arrange per recipe (or raise untouched), ranked so the
    // hero ends frontmost.
    let mut own: Vec<Arrangement> = Vec::new();
    for w in windows_all {
        if protected.contains(&w.pid) || w.minimized && mode == ContainMode::Park {
            continue;
        }
        let m = window_match(w, surfaces);
        if !m.belongs() {
            continue;
        }
        pod_pid.insert(w.pid);
        let resource = resource_of(w, surfaces);
        let frame = recipe.and_then(|r| match (&resource, screen) {
            (Some(res), Some(screen)) if screen.usable() => {
                r.frame_for(res, screen).map(|f| fit(f, &w.frame))
            }
            _ => None,
        });
        own.push(Arrangement {
            pid: w.pid,
            window_id: w.window_id,
            frame,
            resource,
            raise_rank: m.rank(),
        });
    }
    own.sort_by(|a, b| {
        a.raise_rank
            .cmp(&b.raise_rank)
            .then(a.window_id.cmp(&b.window_id))
    });
    plan.arrange = own;
    plan.activate = plan.arrange.last().map(|a| a.pid);

    // Foreign windows: veil or park (never both), under-containment only.
    for w in windows_all {
        if protected.contains(&w.pid) || w.minimized {
            continue;
        }
        if window_match(w, surfaces).belongs() {
            continue;
        }
        let owner_regular = apps.iter().find(|a| a.pid == w.pid).map(|a| a.regular).unwrap_or(true);
        if !owner_regular {
            continue;
        }
        match mode {
            ContainMode::Dim => {
                if w.frame.usable() {
                    plan.veil.push((w.pid, w.window_id, w.frame));
                }
            }
            ContainMode::Park => plan.park.push((w.pid, w.window_id)),
            ContainMode::Hide => {}
        }
    }

    // App-level moves.
    for app in apps {
        if protected.contains(&app.pid) {
            continue;
        }
        if mode == ContainMode::Hide && app.regular && !app.hidden && !pod_pid.contains(&app.pid) {
            plan.hide_apps.push(app.pid);
        }
        if pod_pid.contains(&app.pid) && app.hidden {
            plan.unhide_apps.push(app.pid);
        }
    }
    plan
}

/// The resource this window would choreograph to: its document, its page,
/// else its title.
fn resource_of(window: &PodWindow, surfaces: &PodSurfaces) -> Option<String> {
    if let Some(doc) = &window.ax_document {
        let doc_norm = strip_file_scheme(doc);
        for known in &surfaces.documents {
            if strip_file_scheme(known) == doc_norm {
                return Some(known.clone());
            }
        }
        for known in &surfaces.urls {
            if known == doc {
                return Some(known.clone());
            }
        }
    }
    if !window.title.is_empty() && surfaces.titles.iter().any(|t| t == &window.title) {
        return Some(window.title.clone());
    }
    None
}

/// Never place windows fully off the screen they live on: a recipe can
/// describe a region the app's window can't satisfy (E.g. Electron
/// min-heights), so clamp to at least 160px in screen, in-bounds or
/// adjacently aligned.
fn fit(target: Frame, current: &Frame) -> Frame {
    let w = target.w.max(160.0f64.min(target.w));
    let h = target.h.max(120.0f64.min(target.h));
    let (base_x, base_y) = if current.usable() {
        (current.x, current.y)
    } else {
        (target.x, target.y)
    };
    // Keep the anchor corner nearest the current position.
    let x = if (base_x - target.x).abs() < 40.0 { base_x } else { target.x };
    Frame::new(x, target.y, w, h)
}
