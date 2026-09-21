//! QA layout probe — the real screens, laid out headlessly, reported as facts.
//!
//! A development instrument, never part of the app. `qa_capture` produces
//! pictures but needs a window; this machine has no WindowServer session and no
//! Metal device, so no window can exist here. egui's layout and text shaping
//! are pure CPU, though, and [`egui::Context::run_ui`] is the very call eframe
//! makes to hand `App::ui` its `Ui` — so running the screens through it yields
//! the same geometry the renderer would have drawn, without a display.
//!
//! What comes out is measurements rather than pixels: every text run with its
//! resolved font, size, colour and box; every filled rect with its fill, stroke
//! and shadow blur; and the derived checks that answer the questions a
//! screenshot would have been used to answer — is anything clipped, do the left
//! edges line up, are the vertical gaps on the spacing scale, how many distinct
//! surface colours are actually on screen, and how many borders.

use egui::epaint::{ClippedShape, Shape};
use egui::{Color32, Context, FontFamily, Pos2, RawInput, Rect};

use evo_daemon::cache::CanonicalIndex;
use evo_desktop::daemon::{DaemonManager, DaemonStatus};
use evo_desktop::{detail, home, shell, state, theme, ui};
use evo_engagement::Standing;
use evo_execution::{
    ExecutionReport, Locator, PreflightOutcome, RestorationSelection, preflight_selection,
};
use evo_restoration::DerivationOutcome;
use evo_workspace::workspace::Workspace;
use evo_workspace::workspace_id::WorkspaceId;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Passes to run before reading the layout, so every entry transition and
/// scroll-edge fade has finished animating.
const PASSES: usize = 90;
/// The base of the spacing scale. A gap that is not a multiple of this is
/// off-grid by the theme's own definition.
const GRID: f32 = 4.0;

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

#[derive(Clone, Copy)]
enum Body {
    Home { query: &'static str },
    Detail(usize),
    Settings,
    FirstRun(u8),
    Empty,
    Permission,
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
            name: "02-home-notice",
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
    ]
}

// ── canonical state, loaded the way the shell loads it ─────────────────────

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
    let index = CanonicalIndex::new(root.to_path_buf()).expect("build canonical index");
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

/// The user-authored transient state the screens borrow mutably.
#[derive(Default)]
struct Transient {
    query: String,
    kind_filter: state::WorkKindFilter,
    period_filter: state::WorkPeriodFilter,
    memo: home::Memo,
    selected: Option<WorkspaceId>,
    designation_status: Option<(String, bool)>,
    continuation_draft: Option<BTreeSet<String>>,
    continuation_status: Option<(String, bool)>,
    grouping_status: Option<(String, bool)>,
    execution_result: Option<ExecutionReport>,
    continue_phase: detail::ContinuePhase,
    settings_open: bool,
}

/// The composition below mirrors `EvoApp::ui` — same environment, same header,
/// same body rect, same capture notice, same screen call.
fn compose(
    ui: &mut egui::Ui,
    shot: Shot,
    canon: &Canon,
    tr: &mut Transient,
    daemon: &mut DaemonManager,
    storage_root: &Path,
) {
    let status = shot.cap.status();
    let max = ui.max_rect();
    ui::paint_environment(ui.painter(), max);

    if let Body::FirstRun(step) = shot.body {
        let mut pending = Some(step);
        shell::first_run(ui, step, &mut pending, daemon);
        return;
    }

    let header_rect = egui::Rect::from_min_max(
        max.min,
        egui::pos2(max.max.x, max.min.y + theme::HEADER_HEIGHT),
    );
    let body_rect = egui::Rect::from_min_max(
        egui::pos2(max.min.x, max.min.y + theme::HEADER_HEIGHT),
        max.max,
    );
    shell::header(ui, header_rect, &status);

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
                Body::Empty => home::empty(ui, true),
                Body::Permission => home::permission_required(ui, None, || {}),
                Body::StorageError => shell::storage_error(
                    ui,
                    "canonical storage could not be read: observation.log is unreadable",
                ),
                Body::Home { .. } | Body::Settings => {
                    let mut retrieval = home::Retrieval {
                        query: &mut tr.query,
                        kind: &mut tr.kind_filter,
                        period: &mut tr.period_filter,
                        memo: &mut tr.memo,
                    };
                    home::screen(
                        ui,
                        canon.workspaces.as_slice(),
                        canon.cards.as_slice(),
                        &canon.subjects,
                        &canon.kinds,
                        false,
                        &mut tr.selected,
                        &mut retrieval,
                    );
                }
                Body::Detail(n) => {
                    let Some(workspace) = canon.workspaces.get(n) else {
                        home::empty(ui, true);
                        return;
                    };
                    let canonical = detail::Canonical {
                        workspace,
                        outcome: canon.outcomes.get(&workspace.id().to_string()),
                        selection: canon.selections.get(n).and_then(Option::as_ref),
                        preflight: canon.preflights.get(n).map(Vec::as_slice).unwrap_or(&[]),
                        subjects: &canon.subjects,
                        kinds: &canon.kinds,
                        titles: &canon.titles,
                        standings: &canon.standings,
                        locators: &canon.locators,
                        designation: canon.designation.as_ref(),
                    };
                    let mut transient = detail::Transient {
                        storage_root,
                        selected: &mut tr.selected,
                        designation_status: &mut tr.designation_status,
                        continuation_draft: &mut tr.continuation_draft,
                        continuation_status: &mut tr.continuation_status,
                        grouping_status: &mut tr.grouping_status,
                        execution_result: &mut tr.execution_result,
                        continue_phase: &mut tr.continue_phase,
                    };
                    detail::screen(ui, &canonical, canon.cards.as_slice(), &mut transient);
                }
                Body::FirstRun(_) => {}
            }
        },
    );

    shell::settings(ui, &mut tr.settings_open, &status, daemon, storage_root);
}

// ── extracting facts from the laid-out shapes ──────────────────────────────

struct TextRun {
    pos: Pos2,
    size: egui::Vec2,
    clip: Rect,
    font_size: f32,
    family: String,
    weight: f32,
    color: Color32,
    elided: bool,
    text: String,
}

struct RectRun {
    rect: Rect,
    fill: Color32,
    stroke_width: f32,
    stroke_color: Color32,
    blur: f32,
    radius: u8,
}

#[derive(Default)]
struct Facts {
    texts: Vec<TextRun>,
    rects: Vec<RectRun>,
    lines: usize,
    paths: usize,
    circles: usize,
}

fn hex(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}@{a}")
    }
}

/// Perceived lightness, for judging whether two surfaces actually differ.
fn luminance(c: Color32) -> f32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    let f = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
}

fn contrast(a: Color32, b: Color32) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    (hi + 0.05) / (lo + 0.05)
}

fn collect(shape: &Shape, clip: Rect, facts: &mut Facts) {
    match shape {
        Shape::Vec(shapes) => {
            for shape in shapes {
                collect(shape, clip, facts);
            }
        }
        Shape::Rect(r) => facts.rects.push(RectRun {
            rect: r.rect,
            fill: r.fill,
            stroke_width: r.stroke.width,
            stroke_color: r.stroke.color,
            blur: r.blur_width,
            radius: r.corner_radius.nw,
        }),
        Shape::Text(t) => {
            let section = t.galley.job.sections.first();
            let (font_size, family, weight) = section
                .map(|s| {
                    let f = &s.format.font_id;
                    let name = match &f.family {
                        FontFamily::Proportional => "Proportional".to_string(),
                        FontFamily::Monospace => "Monospace".to_string(),
                        FontFamily::Name(n) => n.to_string(),
                    };
                    // The theme drives SF Pro's variable weight axis; reading it
                    // back is the only way to tell a Semibold run from a Regular
                    // one, since both resolve through the same face. `wght` is
                    // the only axis the theme sets, so the first coordinate is
                    // the weight.
                    let weight = s
                        .format
                        .coords
                        .as_ref()
                        .first()
                        .map(|&(_, value)| value)
                        .unwrap_or(0.0);
                    (f.size, name, weight)
                })
                .unwrap_or((0.0, "?".into(), 0.0));
            let color = t
                .override_text_color
                .or_else(|| section.map(|s| s.format.color))
                .unwrap_or(t.fallback_color);
            facts.texts.push(TextRun {
                pos: t.pos,
                size: t.galley.rect.size(),
                clip,
                font_size,
                family,
                weight,
                color,
                elided: t.galley.elided,
                text: t.galley.text().to_string(),
            });
        }
        Shape::LineSegment { .. } => facts.lines += 1,
        Shape::Path(_) => facts.paths += 1,
        Shape::Circle(_) => facts.circles += 1,
        _ => {}
    }
}

fn round1(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

fn report(shot: Shot, facts: &Facts) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "══════════════════════════════════════════════════");
    let _ = writeln!(out, "SCREEN {}   {}x{} pt", shot.name, shot.w, shot.h);
    let _ = writeln!(out, "══════════════════════════════════════════════════");
    let _ = writeln!(
        out,
        "shapes: {} text, {} rect, {} line, {} path, {} circle",
        facts.texts.len(),
        facts.rects.len(),
        facts.lines,
        facts.paths,
        facts.circles
    );

    // ── 1. clipping ────────────────────────────────────────────────────────
    let mut clipped = Vec::new();
    for t in &facts.texts {
        let box_ = Rect::from_min_size(t.pos, t.size);
        let over_r = box_.max.x - t.clip.max.x;
        let over_b = box_.max.y - t.clip.max.y;
        if t.elided || over_r > 0.5 || over_b > 0.5 {
            clipped.push((t, round1(over_r.max(0.0)), round1(over_b.max(0.0))));
        }
    }
    let _ = writeln!(out, "\n── CLIPPED / ELIDED TEXT ({}) ──", clipped.len());
    for (t, r, b) in &clipped {
        let _ = writeln!(
            out,
            "  {:<7} elided={} overflow right={} bottom={}  {:?}",
            format!("{}pt", round1(t.font_size)),
            t.elided,
            r,
            b,
            truncate(&t.text, 70)
        );
    }

    // ── 2. typography actually used ────────────────────────────────────────
    // Weight is keyed on the family name, not on `TextFormat.coords`. The theme
    // registers one named family per weight (`evo-bold`, `evo-semibold`,
    // `evo-medium`, `Proportional`) instead of setting a `wght` axis per run,
    // so `coords` is empty on every run and grouping by size alone silently
    // merges Body (13.5 regular) with BodyStrong (13.5 semibold).
    let mut ramp: BTreeMap<String, usize> = BTreeMap::new();
    for t in &facts.texts {
        let axis = if t.weight > 0.0 {
            format!(" coords wght={}", round1(t.weight))
        } else {
            String::new()
        };
        let key = format!("{:>5}pt  {:<14}{axis}", round1(t.font_size), t.family);
        *ramp.entry(key).or_default() += 1;
    }
    let _ = writeln!(out, "\n── TYPE RAMP IN USE ({} distinct) ──", ramp.len());
    for (key, count) in &ramp {
        let _ = writeln!(out, "  {key}  x{count}");
    }

    // ── 3. ink colours ─────────────────────────────────────────────────────
    let mut inks: BTreeMap<String, usize> = BTreeMap::new();
    for t in &facts.texts {
        *inks.entry(hex(t.color)).or_default() += 1;
    }
    let _ = writeln!(out, "\n── INK COLOURS ({}) ──", inks.len());
    for (color, count) in &inks {
        let _ = writeln!(out, "  {color}  x{count}");
    }

    // ── 4. surfaces ────────────────────────────────────────────────────────
    // Only surfaces big enough to read as a surface; hairlines and marks are
    // counted separately below.
    let mut fills: BTreeMap<String, (usize, f32)> = BTreeMap::new();
    for r in &facts.rects {
        if r.fill.a() == 0 || r.rect.width() < 24.0 || r.rect.height() < 12.0 {
            continue;
        }
        let entry = fills.entry(hex(r.fill)).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 = entry.1.max(r.rect.width() * r.rect.height());
    }
    let _ = writeln!(out, "\n── SURFACE FILLS ({} distinct) ──", fills.len());
    for (color, (count, area)) in &fills {
        let _ = writeln!(
            out,
            "  {color}  x{count:<4} largest={:.0}px²  contrast-vs-BG {:.2}:1",
            area,
            contrast(parse_hex(color), theme::BG)
        );
    }

    // ── 4b. every painted rect, in paint order ─────────────────────────────
    // Needed to tell "these two things are stacked" from "these two things
    // overlap": a fill's position is the only way to check that a widget
    // actually advanced the cursor for the one after it. Environment bands are
    // dropped because there are ~18 of them on every screen and they are
    // reported above.
    let _ = writeln!(out, "\n── PAINTED RECTS (excluding environment bands) ──");
    for r in &facts.rects {
        let band = r.rect.width() >= 300.0 && (r.rect.height() - 46.0).abs() < 1.0;
        if band || r.fill.a() == 0 {
            continue;
        }
        let _ = writeln!(
            out,
            "  {:>5.0}x{:<5.0} at ({:>6.1},{:>6.1})  fill {:<12} r={:<3} stroke={:.2}px {}",
            r.rect.width(),
            r.rect.height(),
            r.rect.min.x,
            r.rect.min.y,
            hex(r.fill),
            r.radius,
            r.stroke_width,
            hex(r.stroke_color)
        );
    }

    // ── 5. borders and shadows ─────────────────────────────────────────────
    let bordered: Vec<&RectRun> = facts
        .rects
        .iter()
        .filter(|r| r.stroke_width > 0.0 && r.stroke_color.a() > 0)
        .collect();
    let shadowed: Vec<&RectRun> = facts.rects.iter().filter(|r| r.blur > 0.0).collect();
    let hairlines = facts
        .rects
        .iter()
        .filter(|r| r.fill.a() > 0 && r.rect.height() <= 1.5 && r.rect.width() > 24.0)
        .count();
    let _ = writeln!(
        out,
        "\n── EDGES: {} stroked rect(s), {} shadowed rect(s), {} hairline fill(s), {} line segment(s)",
        bordered.len(),
        shadowed.len(),
        hairlines,
        facts.lines
    );
    for r in bordered.iter().take(14) {
        let _ = writeln!(
            out,
            "    stroke {:.2}px {} on {:.0}x{:.0} at ({:.0},{:.0}) r={}",
            r.stroke_width,
            hex(r.stroke_color),
            r.rect.width(),
            r.rect.height(),
            r.rect.min.x,
            r.rect.min.y,
            r.radius
        );
    }
    for r in shadowed.iter().take(10) {
        let _ = writeln!(
            out,
            "    shadow blur={:.1} {} on {:.0}x{:.0} at ({:.0},{:.0})",
            r.blur,
            hex(r.fill),
            r.rect.width(),
            r.rect.height(),
            r.rect.min.x,
            r.rect.min.y
        );
    }

    // ── 6. horizontal alignment ────────────────────────────────────────────
    let mut lefts: BTreeMap<i32, usize> = BTreeMap::new();
    for t in &facts.texts {
        *lefts.entry(t.pos.x.round() as i32).or_default() += 1;
    }
    let _ = writeln!(out, "\n── TEXT LEFT EDGES ({} distinct x) ──", lefts.len());
    let mut edges: Vec<(i32, usize)> = lefts.into_iter().collect();
    edges.sort_by_key(|(x, _)| *x);
    let listed: Vec<String> = edges.iter().map(|(x, n)| format!("{x}(x{n})")).collect();
    let _ = writeln!(out, "  {}", listed.join("  "));

    // ── 7. vertical rhythm ─────────────────────────────────────────────────
    // Gaps between consecutive text blocks that share a left edge, which is
    // what the eye reads as a column's rhythm.
    let dominant = edges.iter().max_by_key(|(_, n)| *n).map(|(x, _)| *x);
    if let Some(column) = dominant {
        let mut ys: Vec<(f32, f32, &str)> = facts
            .texts
            .iter()
            .filter(|t| (t.pos.x.round() as i32) == column)
            .map(|t| (t.pos.y, t.size.y, t.text.as_str()))
            .collect();
        ys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let _ = writeln!(
            out,
            "\n── VERTICAL RHYTHM in the main column (x={column}, {} blocks) ──",
            ys.len()
        );
        let mut off_grid = 0;
        for pair in ys.windows(2) {
            let (y0, h0, t0) = pair[0];
            let (y1, _, t1) = pair[1];
            let gap = y1 - (y0 + h0);
            let on = (gap / GRID - (gap / GRID).round()).abs() < 0.02 || gap < 0.0;
            if !on {
                off_grid += 1;
            }
            let _ = writeln!(
                out,
                "  gap {:>7}  {}  {:?} → {:?}",
                round1(gap),
                if on { "  " } else { "OFF" },
                truncate(t0, 28),
                truncate(t1, 28)
            );
        }
        let _ = writeln!(out, "  off-grid gaps: {off_grid}");
    }

    // ── 8. every text run, in paint order ──────────────────────────────────
    let _ = writeln!(out, "\n── ALL TEXT ({}) ──", facts.texts.len());
    for t in &facts.texts {
        let _ = writeln!(
            out,
            "  ({:>6.1},{:>6.1}) {:>5.0}x{:<5.0} {:>5.1}pt {:<14} {:<10} {:?}",
            t.pos.x,
            t.pos.y,
            t.size.x,
            t.size.y,
            t.font_size,
            t.family,
            hex(t.color),
            truncate(&t.text, 78)
        );
    }

    out
}

fn parse_hex(value: &str) -> Color32 {
    let body = value.trim_start_matches('#');
    let base = body.split('@').next().unwrap_or(body);
    let byte = |i: usize| u8::from_str_radix(&base[i..i + 2], 16).unwrap_or(0);
    if base.len() >= 6 {
        Color32::from_rgb(byte(0), byte(2), byte(4))
    } else {
        Color32::BLACK
    }
}

fn truncate(value: &str, at: usize) -> String {
    let flat = value.replace('\n', "⏎");
    if flat.chars().count() <= at {
        flat
    } else {
        let kept: String = flat.chars().take(at).collect();
        format!("{kept}…")
    }
}

fn main() {
    let out_dir = std::env::temp_dir().join("evo-qa-layout");
    std::fs::create_dir_all(&out_dir).expect("create output directory");
    let storage_root: PathBuf = state::canonical_storage_root();
    let canon = load(&storage_root);
    println!(
        "QA-LAYOUT storage={} workspaces={} out={}",
        storage_root.display(),
        canon.workspaces.len(),
        out_dir.display()
    );

    // The manager is constructed but never spawned into: every screen is told
    // its capture state explicitly, so the banners can each be inspected.
    let mut daemon = DaemonManager::new(storage_root.clone());
    daemon.kill();

    let mut summary = String::new();
    for shot in shots() {
        let ctx = Context::default();
        theme::install(&ctx);
        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(shot.w, shot.h));
        let mut tr = Transient {
            query: match shot.body {
                Body::Home { query } => query.to_string(),
                _ => String::new(),
            },
            settings_open: matches!(shot.body, Body::Settings),
            selected: match shot.body {
                Body::Detail(n) => canon.workspaces.get(n).map(|w| w.id().clone()),
                _ => None,
            },
            ..Default::default()
        };

        let mut facts = Facts::default();
        for pass in 0..PASSES {
            let input = RawInput {
                screen_rect: Some(screen),
                time: Some(pass as f64 / 60.0),
                predicted_dt: 1.0 / 60.0,
                ..Default::default()
            };
            let output = ctx.run_ui(input, |ui| {
                compose(ui, shot, &canon, &mut tr, &mut daemon, &storage_root);
            });
            if pass + 1 == PASSES {
                facts = Facts::default();
                for ClippedShape { clip_rect, shape } in &output.shapes {
                    collect(shape, *clip_rect, &mut facts);
                }
            }
        }

        let text = report(shot, &facts);
        let path = out_dir.join(format!("{}.txt", shot.name));
        std::fs::write(&path, &text).expect("write report");
        let clipped = facts
            .texts
            .iter()
            .filter(|t| {
                t.elided
                    || (t.pos.x + t.size.x) - t.clip.max.x > 0.5
                    || (t.pos.y + t.size.y) - t.clip.max.y > 0.5
            })
            .count();
        let _ = writeln!(
            summary,
            "{:<20} {:>5}x{:<5} texts={:<4} rects={:<4} clipped={:<3} strokes={:<3} shadows={}",
            shot.name,
            shot.w,
            shot.h,
            facts.texts.len(),
            facts.rects.len(),
            clipped,
            facts
                .rects
                .iter()
                .filter(|r| r.stroke_width > 0.0 && r.stroke_color.a() > 0)
                .count(),
            facts.rects.iter().filter(|r| r.blur > 0.0).count()
        );
    }

    println!("\n── SUMMARY ──\n{summary}");
}
