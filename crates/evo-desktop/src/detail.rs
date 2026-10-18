//! The Workspace detail screen: one body of work, in focus.
//!
//! The screen answers a single question — *where does this work continue, and
//! can Evo reopen it?* — and everything on it is arranged around that answer.
//!
//! The screen is exactly three levels, in descending order of what the user
//! came for:
//!
//! The stage (DESIGN.md adoption map 1) is one full-bleed recessed canvas
//! carrying the work's identity: the serif initial as a watermark, the
//! display-voice title floating on the canvas, the standing of the evidence
//! in the teal voice. All functional material lives in three bottom-docked
//! panel-cards (adoption map 2), each one object in two states — a collapsed
//! strip that grows upward with the `settle()` easing, never modal:
//!
//! * **Overview** carries the answer: where this work continues, the next
//!   step, the blockers, the plan, and the in-flight transition and report.
//! * **Activity** carries what returns alongside the Resume Point, each with
//!   its canonical preflight word — read-only, a glance.
//! * **Record** carries the way to reconsider what comes back (RFC-0013), to
//!   relate other work (RFC-0012), and the whole witnessed record — present
//!   without being spread open by default.
//!
//! The idle Continue primary lives in the transport dock (adoption map 3),
//! so the stage itself offers no primary: the screen carries exactly one
//! primary control at a time, always in the same place.
//!
//! Two structures divide what the screen may read from what it may write.
//! [`Canonical`] is Evo's own record and is borrowed immutably: the screen can
//! project it and can never author it. [`Transient`] is the user's in-flight
//! state — a draft, a status line, a report they have not dismissed — and is
//! the only thing the screen mutates. Nothing in [`Transient`] is canonical
//! until the daemon accepts a declaration.
//!
//! Designation and Continuation Surface stay distinct, as RFC-0013 Negative
//! Case 10 requires, but the distinction is carried by *form* rather than by
//! vocabulary: a round mark that supersedes (where this work continues) and a
//! square mark that accumulates into a draft (what comes back with it). The
//! user never has to read the words "designation" or "surface" to act
//! correctly.

use crate::state;
use crate::theme::{self, Role, Severity};
use crate::ui::{self, Control};

use egui::Color32;

use evo_daemon::continuation::ContinuationResponse;
use evo_daemon::designation::DesignationResponse;
use evo_daemon::grouping::GroupingResponse;
use evo_engagement::Standing;
use evo_execution::{
    ExecutionReport, Locator, LocatorKind, PreflightOutcome, PreflightStatus, RestorationSelection,
    RestorationStep, StepPlan, TargetAttempt, TargetStatus, WithheldMember,
};
use evo_restoration::DerivationOutcome;
use evo_workspace::workspace::Workspace;
use evo_workspace::workspace_id::WorkspaceId;

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::{Instant, SystemTime};

/// The number of witnessed resources rendered in the full history list. A
/// bounded display window is presentation only: the record is complete, and
/// the count above the list always reports the true total.
pub const MAX_RENDERED_RESOURCES: usize = 50;

/// Where the Continue action is in its lifecycle.
///
/// `Idle` offers the primary control. `Preparing` is entered the instant the
/// user commits: the control is replaced by the restoring transition — which
/// *is* the lock against a second press, since there is no longer a button to
/// press — and it carries the moment it began so the shell can let the
/// transition paint before the restoration begins. `Executing` carries the
/// decided plan and the results that have actually come back so far, which is
/// what lets the screen show honest per-item progress: a row can only be marked
/// once `done` holds a real result for it.
///
/// The restoration itself is run by the shell (`app::EvoApp::logic`), one step
/// per frame, never inside the click handler. Structuring it this way is what
/// lets a future asynchronous or remote execution layer be a change in the
/// shell alone: the screen already treats Continue as "commit now, learn each
/// result as it arrives", which is exactly the shape an async boundary has.
#[derive(Clone, Default)]
pub enum ContinuePhase {
    /// At rest: the primary control is offered.
    #[default]
    Idle,
    /// The user has committed; the transition is showing and the shell will
    /// decide the plan once `since` has dwelled long enough to paint it.
    Preparing { since: Instant },
    /// The plan is decided and being acted on. `done` holds the results the
    /// executor has actually returned, in step order; every step beyond its
    /// length is still pending and is drawn as such.
    Executing {
        steps: Vec<RestorationStep>,
        done: Vec<TargetAttempt>,
    },
}

impl ContinuePhase {
    /// When the user committed, if a restoration is waiting to be decided.
    pub fn prepared_since(&self) -> Option<Instant> {
        match self {
            ContinuePhase::Preparing { since } => Some(*since),
            _ => None,
        }
    }

    /// Whether a decided plan is in flight.
    pub fn is_executing(&self) -> bool {
        matches!(self, ContinuePhase::Executing { .. })
    }

    /// Acts on exactly one planned step, and returns the finished report once
    /// every step has a real result.
    ///
    /// One step per call is what makes the progress the user sees honest: a row
    /// is marked because the executor returned a result for it, and the frame
    /// between calls is what paints it. The frontier is `done.len()`, so the
    /// same step can never be acted on twice — and because the phase is only
    /// left through this method, a second Continue cannot re-run a restoration
    /// that is already in flight. (The screen's own lock is stronger still: no
    /// Continue control exists outside `Idle`.)
    ///
    /// A phase that is not `Executing` performs nothing and reports nothing.
    pub fn advance<E>(&mut self, executor: &E) -> Option<ExecutionReport>
    where
        E: evo_execution::PlatformExecutor + evo_execution::PreflightSource,
    {
        let ContinuePhase::Executing { steps, done } = self else {
            return None;
        };
        if done.len() < steps.len() {
            let attempt = evo_execution::execute_step(&steps[done.len()], executor, executor);
            done.push(attempt);
            return None;
        }
        // Every step has a real result — or the plan was empty, in which case
        // the report says exactly that.
        let report = ExecutionReport::new(std::mem::take(done));
        *self = ContinuePhase::Idle;
        Some(report)
    }
}

/// Everything the detail screen may read. Borrowed immutably, because the
/// screen projects Evo's record and never authors it.
pub struct Canonical<'a> {
    /// The Workspace being viewed, identified canonically by its `WorkspaceId`.
    pub workspace: &'a Workspace,
    /// The replayed derivation for this Workspace, when one exists.
    pub outcome: Option<&'a DerivationOutcome>,
    /// What restoration derived from the current continuation.
    pub selection: Option<&'a RestorationSelection>,
    /// What the executor reports about the derived selection, right now.
    pub preflight: &'a [PreflightOutcome],
    /// Artifact identity → witnessed subject. Presentation only.
    pub subjects: &'a HashMap<String, String>,
    /// Artifact identity → witnessed kind. Presentation only.
    pub kinds: &'a HashMap<String, String>,
    /// Workspace identity → what that body of work is about. Presentation
    /// only; the name the work is called by, not a name for any resource in it.
    pub titles: &'a HashMap<String, String>,
    /// Workspace identity → Engagement-layer standing. The detail screen does
    /// not branch on it — opening a body of work by name is the same act
    /// whatever its standing — but it is threaded here so the canonical
    /// hand-off carries the whole derived record, not a convenient subset.
    pub standings: &'a HashMap<String, Standing>,
    /// Artifact identity → locator, for describing what `Continue` will try.
    pub locators: &'a HashMap<String, Locator>,
    /// The standing designation and when it was recorded, if there is one.
    pub designation: Option<&'a (String, SystemTime)>,
}

/// Everything the detail screen may write: the user's own in-flight state.
///
/// A canonical reload may replace derived state; it may never touch anything
/// in here. A draft the user is still editing, a status line they have not
/// read, and an execution report they have not dismissed all outlive a reload.
pub struct Transient<'a> {
    /// Where declarations are submitted.
    pub storage_root: &'a Path,
    /// Which Workspace the shell is showing, by canonical identity.
    pub selected: &'a mut Option<WorkspaceId>,
    /// The daemon's answer to the last designation, and whether it accepted.
    pub designation_status: &'a mut Option<(String, bool)>,
    /// Marks the user has changed but not yet declared. `None` means the
    /// marks shown are Evo's, not the user's.
    pub continuation_draft: &'a mut Option<BTreeSet<String>>,
    /// The daemon's answer to the last declaration, and whether it accepted.
    pub continuation_status: &'a mut Option<(String, bool)>,
    /// The daemon's answer to the last related-work declaration (RFC-0012
    /// WorkGrouped), and whether it accepted.
    pub grouping_status: &'a mut Option<(String, bool)>,
    /// The last execution's report. It stays until the user dismisses it.
    pub execution_result: &'a mut Option<ExecutionReport>,
    /// Where the Continue action is in its lifecycle. Writing `Preparing` here
    /// is how the click handler commits; the shell runs the restoration and
    /// returns it to `Idle`.
    pub continue_phase: &'a mut ContinuePhase,
}

/// Renders the stage for one body of work (DESIGN.md adoption map 1+2).
///
/// The stage is the reference grammar adopted, not copied: one full-bleed
/// recessed canvas carries the work's identity (the serif initial as a
/// watermark, the display-voice title floating on the canvas), and all
/// functional material lives in three bottom-docked panel-cards — Overview,
/// Activity, Record. Each panel is one object in two states: a collapsed
/// strip (eyebrow + glyph) that grows upward with the `settle()` easing into
/// a dense column holding the content the old scrolling screen carried.
/// Nothing is modal; the canvas never disappears behind a panel.
///
/// The idle Continue primary lives in the transport dock (DESIGN.md adoption
/// map 3), so the stage itself carries no primary control; the plan
/// line, the in-flight transition and the report still render here, in the
/// Overview panel, exactly as honest as before.
pub fn screen(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    siblings: &[state::WorkspaceCard],
    tr: &mut Transient<'_>,
) {
    let canvas = ui.available_rect_before_wrap();

    // Arrival: one settle() fade for the whole canvas per stage entry. Keyed
    // by the Workspace identity so stepping sideways into other work re-arrives.
    let stage_id = ui
        .id()
        .with(("evo-stage-arrival", canon.workspace.id().to_string()));
    let arrive = ui
        .ctx()
        .animate_bool_with_time(stage_id, true, theme::RESPONSE_IN);

    // Entry zoom (DESIGN.md Motion): the stage arrives as scale 0.97→1.0
    // with the same fade the chrome rides. The dock is composed by the
    // shell OUTSIDE this scope, so — exactly like the reference — the
    // transport stays put while the work zooms in behind it.
    let zoom = 0.97 + 0.03 * theme::settle(arrive);
    let painted = canvas.scale_from_center(zoom);
    let painter = ui.painter_at(painted);
    // The stage plane: recessed — the work recedes by weight and the
    // panels float on it. One flat fill; the renderer has no gradient
    // primitive and Evo does not fake one.
    painter.rect_filled(painted, 0.0, theme::RECESSED);

    // The elapsed time since entry, shared with the panel row so the three
    // strips can stagger their own arrivals off the same clock. The entry
    // instant is stored per workspace on first sight; a clock that moves
    // backwards (a fresh run) reads as a new entry, exactly like the
    // animation key above.
    let elapsed_id = egui::Id::new(("evo-stage-elapsed", canon.workspace.id().to_string()));
    let now = ui.input(|i| i.time);
    let entry = ui.data_mut(|data| match data.get_temp::<f64>(elapsed_id) {
        Some(prev) if prev <= now => prev,
        _ => {
            data.insert_temp(elapsed_id, now);
            now
        }
    });
    let elapsed = (now - entry) as f32;

    // The candidates for a continuation origin, paired with their subjects
    // in one pass — the same pairing rule the scrolling screen used, kept
    // here so the shown label and the written subject cannot drift apart.
    let candidates: Vec<(String, String)> =
        state::distinct_artifacts_in_latest_snapshot(canon.workspace)
            .into_iter()
            .filter_map(|artifact_id| {
                let subject = state::subject_for(canon.subjects, &artifact_id)?.to_string();
                Some((artifact_id, subject))
            })
            .collect();

    // ── Canvas chrome and identity (floating, not docked) ──────────────
    // The chrome is bounded to the painted plane's upper region and shares
    // the plane's zoom, so identity and canvas are one arriving object.
    let panels_top = canvas.bottom() - theme::ACTION_BAR_HEIGHT - theme::S4;
    let _chrome = ui
        .scope_builder(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(
                    painted.min,
                    egui::pos2(painted.max.x, panels_top),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.set_opacity(theme::settle(arrive));
                ui.add_space(theme::S5);
                ui.add_space(theme::PAGE_PAD);
                place_nav(ui, canon, siblings, tr);
                ui.add_space(theme::S7);
                ui.add_space(theme::PAGE_PAD);
                canvas_identity(ui, canon);
            },
        )
        .response
        .rect;

    // ── The panel-cards (adoption map 2) ────────────────────────────
    // One row of floating cards above the dock, inset from the window
    // edges by the page gutter. The slot reaches from just below the nav
    // row down to the dock clearance: an expanded panel grows OVER the
    // identity the way the reference's columns grow over the artwork — the
    // canvas recedes behind it, and the nav stays one gesture away (G3).
    // The panels' slot is cut from the painted plane, not the raw canvas,
    // so the row arrives with the zoom like everything else on the stage.
    let nav_bottom = theme::S5 + theme::PAGE_PAD + theme::CONTROL_HEIGHT;
    let slot = egui::Rect::from_min_max(
        egui::pos2(
            painted.left() + theme::PAGE_PAD,
            painted.top() + nav_bottom + theme::S2,
        ),
        egui::pos2(painted.right() - theme::PAGE_PAD, panels_top),
    );
    stage_panels(ui, canon, tr, &candidates, slot, elapsed);
}

/// The work's identity on the canvas: the serif initial as a watermark, the
/// display-voice title, the teal phase of the evidence, and the narrative.
/// This is adoption map 1 — the hero canvas built from the work's own
/// material. The lifecycle notice keeps its place: a superseded record
/// changes what the stage means, so it is stated.
fn canvas_identity(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    let card = state::workspace_card(
        canon.workspace,
        canon.outcome,
        canon.selection,
        canon.subjects,
        canon.kinds,
        canon.titles,
        canon.standings,
    );

    // The watermark initial: the largest quiet thing on the canvas, painted
    // like the artwork it stands in for. Decoration only — receded ink at
    // low alpha, never information. Anchored to the plane's own rect via
    // `painter_at`: inside the chrome scope the ambient painter would clip
    // the glyph to the flow cursor, and the watermark belongs to the whole
    // canvas, not to the line of text it decorates.
    let plane = ui.available_rect_before_wrap();
    let initial: String = card
        .title
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "\u{00B7}".to_string());
    ui.painter_at(plane).text(
        egui::pos2(plane.right() - theme::S7, plane.center().y),
        egui::Align2::RIGHT_CENTER,
        &initial,
        theme::display_font(160.0),
        Color32::from_rgba_unmultiplied(0xEC, 0xEA, 0xE6, 16),
    );
    // The standing the evidence supports, in the teal voice (informational
    // only — never success). The standing is the canonical, evidence-derived
    // fact the record has earned; the word comes from the engagement layer's
    // own vocabulary, never invented here.
    let standing = canon
        .standings
        .get(&canon.workspace.id().to_string())
        .copied()
        .unwrap_or(evo_engagement::Standing::Remembered);
    ui::eyebrow(ui, &format!("\u{25B8} {}", standing_label(standing)));
    ui::gap(ui, theme::S2);
    ui.label(theme::display(&card.title, 34.0, theme::TEXT));
    ui::gap(ui, theme::S3);

    // The narrative, in the lede voice, wrapped to a measure that leaves
    // the watermark its canvas. The block is bounded in width but
    // content-sized in height: a block that consumed the canvas's whole
    // remaining height would push the panel-cards' slot to zero.
    let measure = (ui.available_width() - theme::S8 * 2.0).min(theme::PROSE_WIDTH);
    let block = egui::Rect::from_min_size(
        ui.next_widget_position(),
        egui::vec2(measure, ui.available_height()),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(block)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            if state::distinct_artifacts_across_history(canon.workspace).is_empty() {
                ui::unknown(ui, "Evo has not witnessed anything in this work yet.");
            } else {
                ui::lede(ui, &card.description);
            }

            let lifecycle = state::lifecycle_label(canon.workspace.lifecycle());
            if lifecycle != "Active" {
                ui::gap(ui, theme::S3);
                ui::notice(
                    ui,
                    Severity::Caution,
                    lifecycle,
                    "Evo's record marks this grouping as superseded. Nothing has been \
                     removed — it remains exactly as it was witnessed.",
                );
            }
        },
    );
}

/// The three stage panels: Overview, Activity, Record.
///
/// Geometry, decided here once: the collapsed strip is 56px tall (the dock's
/// own height — one rhythm); an expanded panel grows to everything the slot
/// offers minus a 12px breathing margin, capped at [`theme::PANEL_MAX_HEIGHT`]
/// (480) — tall enough that the canvas recedes behind it, the reference's
/// defining move. The three strips share one row with equal columns, and
/// arrive on a 40ms-per-index stagger from the stage's shared arrival clock.
/// Only one panel is ever expanded; the expanded panel takes its full column
/// height upward, so its content reads as a table, never a squeezed box.
fn stage_panels(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    candidates: &[(String, String)],
    slot: egui::Rect,
    elapsed: f32,
) {
    let any_click = ui.input(|i| i.pointer.any_click());

    let overview_count = candidates.len();
    let activity_count = canon
        .selection
        .map(|selection| selection.restore_worthy().len() + selection.unavailable().len())
        .unwrap_or(0);

    // At most one panel expanded, owned as one stored index — a second
    // click can never leave two panels tall. The state is keyed by the
    // Workspace identity, so stepping sideways into other work arrives with
    // every panel collapsed: navigation resets it, a reload does not —
    // the same rule the shell's transient state follows.
    let open_panel_id = egui::Id::new(("evo-stage-open-panel", canon.workspace.id().to_string()));
    let mut open_panel = ui
        .data_mut(|data| data.get_temp::<usize>(open_panel_id))
        .unwrap_or(usize::MAX);
    if open_panel >= 3 {
        open_panel = usize::MAX;
    }

    let labels = ["Overview", "Activity", "Record"];
    let counts = [Some(overview_count), Some(activity_count), None];

    // Three equal floating cards on one row, a gutter between them. The row
    // is cut from the clamped slot directly rather than from the layout
    // cursor, so the panels always dock above the transport no matter how
    // much identity the canvas above them used — `ui.columns` would have
    // laid out from wherever the chrome left the cursor and ignored the
    // dock clearance entirely.
    let gutter = theme::S3;
    let column_width = (slot.width() - gutter * 2.0) / 3.0;
    let columns: Vec<egui::Rect> = (0..3usize)
        .map(|index| {
            let left = slot.left() + (column_width + gutter) * index as f32;
            egui::Rect::from_min_max(
                egui::pos2(left, slot.top()),
                egui::pos2(left + column_width, slot.bottom()),
            )
        })
        .collect();
    // A click anywhere outside the three cards — the canvas, the dock, a
    // gutter — collapses the open one: the same disarm rule the merge
    // two-step proved on the rack, without stealing clicks (a click inside
    // any card is that card's own business).
    let at = ui.input(|i| i.pointer.latest_pos());
    let collapse_elsewhere =
        any_click && !at.is_some_and(|at| columns.iter().any(|c| c.contains(at)));
    let mut rects: [Option<egui::Rect>; 3] = [None, None, None];
    for (index, column_rect) in columns.iter().enumerate() {
        let id = egui::Id::new(("evo-stage-panel", index));
        let is_open = open_panel == index;
        // Index n enters 40ms after n−1: one composed entrance.
        let arrive_in = elapsed - 0.04 * index as f32;
        let (final_open, panel_rect) = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(*column_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |column| {
                    let mut open = is_open;
                    let rect = stage_panel(
                        column,
                        id,
                        PanelIcon::of(index),
                        labels[index],
                        counts[index],
                        *column_rect,
                        &mut open,
                        collapse_elsewhere,
                        arrive_in,
                        |ui| match index {
                            0 => overview_body(ui, canon, &mut *tr, candidates),
                            1 => activity_body(ui, canon),
                            _ => record_body(ui, canon, &mut *tr),
                        },
                    );
                    (open, rect)
                },
            )
            .inner;
        if final_open != is_open {
            open_panel = if final_open { index } else { usize::MAX };
        }
        rects[index] = Some(panel_rect);
    }

    ui.data_mut(|data| data.insert_temp(open_panel_id, open_panel));
    // The headless harness reads the laid-out rects and the *resolved*
    // post-frame state back through egui's own data store — the rack's
    // proven seam. Written once here, after all three cards have had their
    // say, so a panel closed by another card's click reports its new state
    // in the same frame the click landed. The record tab rides the same
    // seam.
    if cfg!(test) {
        // The record tab is read OUTSIDE the write lock below — `record_tab`
        // itself takes the context's read lock, and re-entering it inside
        // `data_mut` would deadlock the very seam this block feeds.
        let tab_pos = record_tab(ui, canon.workspace.id()).position();
        ui.data_mut(|data| {
            for (index, rect) in rects.iter().enumerate() {
                if let Some(rect) = *rect {
                    data.insert_temp(stage_panel_widget_id(index).with("evo-test-rect"), rect);
                    data.insert_temp(
                        stage_panel_widget_id(index).with("evo-test-open"),
                        open_panel == index,
                    );
                }
            }
            data.insert_temp(
                egui::Id::new(("evo-test-record-tab", canon.workspace.id().to_string())),
                tab_pos,
            );
        });
    }
}

/// One panel-card in its two states (DESIGN.md adoption map 2).
///
/// Collapsed: a strip — eyebrow, live count in the data voice, expand glyph.
/// Expanded: the same strip grown upward into a column whose content is the
/// dense material this screen has always carried. The growth eases with
/// `settle()` over `RESPONSE_IN`; the collapse runs over `RESPONSE_OUT`, so
/// leaving is never slower than arriving. Expansion is NOT modal: any click
/// outside the open panel collapses it (the same disarm rule the merge
/// two-step proved on the rack). The glyph is the only text in the strip
/// that changes state — down when open, up when closed.
#[allow(clippy::too_many_arguments)]
fn stage_panel(
    ui: &mut egui::Ui,
    id: egui::Id,
    icon: PanelIcon,
    label: &'static str,
    count: Option<usize>,
    slot: egui::Rect,
    open: &mut bool,
    collapse_elsewhere: bool,
    arrive_in: f32,
    body: impl FnOnce(&mut egui::Ui),
) -> egui::Rect {
    // One frame of hover memory drives the light, like every Control.
    let was_hovered: bool = ui.data(|data| data.get_temp(id).unwrap_or(false));
    let hover = ui.ctx().animate_bool_with_time(
        id.with("evo-panel-hover"),
        was_hovered,
        theme::RESPONSE_TOUCH,
    );

    // The height animation: one 0→1 value per state, eased by settle().
    // Arriving takes RESPONSE_IN; leaving takes RESPONSE_OUT, so collapse is
    // never slower than expansion (DESIGN.md Motion).
    let expanded = ui.ctx().animate_bool_with_time(
        id.with("evo-panel-open"),
        *open,
        if *open {
            theme::RESPONSE_IN
        } else {
            theme::RESPONSE_OUT
        },
    );

    // Arrival: this panel's stagger-delayed clock, settle-eased. Negative
    // (not yet this panel's turn) clamps to a full recess.
    let arrive = theme::settle((arrive_in / theme::RESPONSE_IN).clamp(0.0, 1.0));

    let collapsed_h = theme::PANEL_STRIP_HEIGHT;
    // A real column: everything the slot offers minus a breathing margin
    // top and bottom — the reference's defining move is that the canvas
    // recedes behind the open panel, not that it peeks beside a strip.
    let expanded_h = (slot.height() - 2.0 * theme::S3).min(theme::PANEL_MAX_HEIGHT);
    let height = collapsed_h + (expanded_h - collapsed_h) * theme::settle(expanded);
    let rect = egui::Rect::from_min_size(
        egui::pos2(slot.left(), slot.bottom() - height),
        egui::vec2(slot.width(), height),
    );

    // The plane: raised, floating on the canvas. The hairline brightens
    // under the pointer or when open; a faint wash reads the hover. The
    // whole plane fades in behind its stagger — the shared `Painter` has no
    // global opacity, so the fill and the hairline carry the arrival alpha
    // explicitly, mixed toward the canvas they sit on.
    let painter = ui.painter_at(rect);
    let corner = egui::CornerRadius::same(theme::CORNER_SURFACE);
    let fill = mix_colors(theme::RECESSED, theme::RAISED, arrive);
    painter.rect_filled(rect, corner, fill);
    painter.rect_stroke(
        rect,
        corner,
        egui::Stroke::new(
            1.0,
            mix_colors(
                theme::RECESSED,
                if was_hovered || *open {
                    theme::border_strong()
                } else {
                    theme::border()
                },
                arrive,
            ),
        ),
        egui::StrokeKind::Inside,
    );
    if expanded > 0.01 {
        painter.rect_filled(
            rect,
            corner,
            Color32::from_rgba_unmultiplied(0, 0, 0, (18.0 * expanded) as u8),
        );
    }
    if hover > 0.01 && !*open {
        painter.rect_filled(
            rect,
            corner,
            Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, (12.0 * hover) as u8),
        );
    }

    // Click-elsewhere collapses (G4 round-trip; the merge disarm rule).
    // The attribution is computed once for all three cards by `stage_panels`,
    // so a click that opens one panel can never also be counted as an
    // elsewhere click that closes another.
    if collapse_elsewhere && *open {
        *open = false;
    }

    // The strip: identical in both states, so the object reads as one
    // object (G6). It is pinned to the panel's top edge and clipped to the
    // panel itself: the header stays exactly where it was while the column
    // grows upward beneath it, and it never paints over the canvas above —
    // a centering layout inside the full rect would have drifted the header
    // toward the middle of the expanded panel. The strip arrives with the
    // panel's stagger, and reads as an instrument row: icon, label, live
    // count in the data voice, state glyph.
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.set_clip_rect(rect);
            ui.set_opacity(arrive);
            let strip = egui::Rect::from_min_max(
                rect.min,
                egui::pos2(rect.max.x, rect.min.y + theme::PANEL_STRIP_HEIGHT),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(strip)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.add_space(theme::PAGE_PAD);
                    // The instrument icon: painter-drawn strokes in the
                    // label's own ink — the icon and the word are one voice,
                    // and no icon font is fabricated.
                    let ink = if *open { theme::TEXT } else { theme::MUTED };
                    panel_icon(ui, icon, ink);
                    ui.add_space(theme::S2);
                    ui.label(theme::eyebrow(label, ink));
                    if let Some(count) = count {
                        ui.add_space(theme::S2);
                        ui.label(theme::data(
                            count.to_string(),
                            Role::Data.size(),
                            theme::TERTIARY,
                        ));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(theme::PAGE_PAD);
                        ui.label(theme::text(
                            Role::Body,
                            if *open { "\u{2193}" } else { "\u{2191}" },
                            theme::TERTIARY,
                        ));
                    });
                },
            );
        },
    );

    let response = ui.interact(rect, id, egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!(
                "{label} panel, {}",
                if *open { "expanded" } else { "collapsed" }
            ),
        )
    });
    if response.clicked() {
        *open = !*open;
    }
    if response.hovered() != was_hovered {
        ui.data_mut(|data| data.insert_temp(id, response.hovered()));
    }

    // The body renders only once the panel is mostly grown — content
    // popping in while the plane is still a strip would read as a glitch.
    if expanded > 0.85 && rect.height() > theme::PANEL_BODY_TOP + theme::S4 {
        let content = egui::Rect::from_min_max(
            egui::pos2(
                rect.left() + theme::M6 as f32,
                rect.top() + theme::PANEL_BODY_TOP,
            ),
            egui::pos2(rect.right() - theme::M6 as f32, rect.bottom() - theme::S4),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(content)
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.set_clip_rect(content);
                ui::scroll(ui, |ui| {
                    body(ui);
                });
            },
        );
    }

    rect
}

/// Which instrument a stage panel reads as. The glyphs are drawn from line
/// segments, never fetched from an icon font — three shapes, three panels,
/// nothing fabricated.
#[derive(Clone, Copy)]
pub(crate) enum PanelIcon {
    /// Overview: three rows — the answer, itemized.
    Rows,
    /// Activity: vertical bars of different heights — things happening.
    Bars,
    /// Record: a ring — the witness mark, round like a round mark.
    Ring,
}

impl PanelIcon {
    fn of(index: usize) -> Self {
        match index {
            0 => PanelIcon::Rows,
            1 => PanelIcon::Bars,
            _ => PanelIcon::Ring,
        }
    }
}

/// Draws one panel instrument icon, 12px on the strip's centerline, from
/// line segments in the given ink. Stroke weight follows the eyebrow's
/// optical weight; every shape is two to four strokes.
fn panel_icon(ui: &mut egui::Ui, icon: PanelIcon, ink: Color32) {
    let side = 12.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
    let c = rect.center().y;
    let l = rect.left();
    let r = rect.right();
    let p = ui.painter_at(rect);
    let stroke = egui::Stroke::new(1.4, ink);
    match icon {
        PanelIcon::Rows => {
            // Three horizontal rows, the middle slightly shorter.
            for (i, dy) in [c - 3.5, c, c + 3.5].into_iter().enumerate() {
                let end = if i == 1 { r - 2.0 } else { r };
                p.line_segment([egui::pos2(l, dy), egui::pos2(end, dy)], stroke);
            }
        }
        PanelIcon::Bars => {
            // Vertical bars, different heights — a record of things happening.
            for (x, half) in [(l + 1.0, 2.0), (c, 4.0), (r - 1.0, 3.0)] {
                p.line_segment([egui::pos2(x, c - half), egui::pos2(x, c + half)], stroke);
            }
        }
        PanelIcon::Ring => {
            // The witness ring: round, like the round mark.
            p.circle_stroke(rect.center(), 4.5, stroke);
        }
    }
}

/// Linear mix between two colors, for arrival fades on painted materials.
fn mix_colors(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
        mix(from.a(), to.a()),
    )
}

/// The Overview panel body: where this work continues, the next step, the
/// blockers, the plan, and the transition/report. The idle Continue primary
/// lives in the transport dock; everything the dock's Continue commits to
/// is announced here, exactly as the old hero announced it.
fn overview_body(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    candidates: &[(String, String)],
) {
    match canon.outcome.and_then(|outcome| outcome.resume_point()) {
        Some(resume) => {
            ui::eyebrow(ui, "Continue from");
            designation_elsewhere(ui, canon, candidates);
            resume_answer(ui, canon, resume.artifact_id().as_str());
            if !candidates.is_empty() {
                ui::gap(ui, theme::S3);
                let open = ui::disclosure(
                    ui,
                    ui.id().with("evo-change-origin"),
                    "Change where this work continues",
                );
                if open {
                    origin_list(ui, canon, tr, candidates);
                } else {
                    designation_answer(ui, tr);
                }
            }
        }
        None => unresolved(ui, canon, tr, candidates),
    }

    next_step(ui, canon);
    blockers(ui, canon);

    // The plan announcement and the in-flight transition/report. The dock's
    // Continue commits by writing `Preparing`; this panel shows the same
    // lifecycle — one shared ContinuePhase, no new execution code.
    ui::gap(ui, theme::S4);
    ui::hairline(ui);
    ui::gap(ui, theme::S3);
    let plan = plan_line(canon);
    match &plan {
        Some(plan) => ui::caption(ui, &format!("Evo will attempt: {plan}")),
        None => {
            let declared_or_resumed = canon.outcome.is_some_and(|outcome| {
                !outcome.continuation_surface().is_empty() || outcome.resume_point().is_some()
            });
            if declared_or_resumed {
                ui::caption(
                    ui,
                    "Evo's record holds no resource it can open for this work.",
                );
            } else {
                ui::caption(
                    ui,
                    "Mark where this work continues, and Evo will be able to reopen it.",
                );
            }
        }
    }
    match &*tr.continue_phase {
        ContinuePhase::Preparing { .. } => {
            ui::gap(ui, theme::S3);
            ui::eyebrow(ui, "Preparing your workspace");
            for (index, (artifact_id, action)) in plan_targets(canon).iter().enumerate() {
                pending_line(ui, canon, index, artifact_id, action);
            }
        }
        ContinuePhase::Executing { steps, done } => {
            ui::gap(ui, theme::S3);
            ui::eyebrow(ui, "Restoring your work");
            for (index, step) in steps.iter().enumerate() {
                match done.get(index) {
                    Some(attempt) => attempt_line(ui, canon, index, attempt),
                    None => pending_line(
                        ui,
                        canon,
                        index,
                        step.artifact_id().as_str(),
                        &step_action(step),
                    ),
                }
            }
        }
        ContinuePhase::Idle => {}
    }
    if let Some(report) = tr.execution_result.as_ref() {
        ui::gap(ui, theme::S3);
        ui::eyebrow(ui, "What Evo did");
        if report.attempts().is_empty() {
            ui::unknown(
                ui,
                "Nothing was executed: the record referenced no resource.",
            );
        } else {
            ui::label(
                ui,
                Role::BodyStrong,
                &state::execution_result_line(report),
                theme::TEXT,
            );
            ui::gap(ui, theme::S2);
            for (index, attempt) in report.attempts().iter().enumerate() {
                attempt_line(ui, canon, index, attempt);
            }
        }
    }
}

/// The Activity panel body: what returns alongside the Resume Point, each
/// with its canonical preflight word — the old "used alongside" tier, read
/// in one glance.
fn activity_body(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    let Some(selection) = canon.selection else {
        ui::caption(
            ui,
            "Evo has not derived a continuation for this body of work yet.",
        );
        return;
    };

    let resume = canon
        .outcome
        .and_then(|outcome| outcome.resume_point())
        .map(|resume| resume.artifact_id().as_str().to_string());
    let statuses: HashMap<&str, &PreflightStatus> = canon
        .preflight
        .iter()
        .map(|outcome| (outcome.artifact_id().as_str(), outcome.status()))
        .collect();

    let mut shown = 0usize;
    for selected in selection.restore_worthy() {
        let artifact_id = selected.artifact_id().as_str();
        if resume.as_deref() == Some(artifact_id) {
            continue;
        }
        let status = statuses.get(artifact_id).copied();
        display_row(
            ui,
            canon,
            artifact_id,
            preflight_word(status, false),
            status.and_then(PreflightStatus::reason),
        );
        shown += 1;
    }
    for item in selection.unavailable() {
        let artifact_id = item.artifact_id().as_str();
        if resume.as_deref() == Some(artifact_id) {
            continue;
        }
        let status = statuses.get(artifact_id).copied();
        let reason = status
            .and_then(PreflightStatus::reason)
            .unwrap_or_else(|| item.reason());
        display_row(
            ui,
            canon,
            artifact_id,
            preflight_word(status, true),
            Some(reason),
        );
        shown += 1;
    }
    if shown == 0 {
        ui::caption(
            ui,
            "Only where you left off comes back — nothing else is in the \
             current continuation.",
        );
    }
}

/// The Record panel body: the reconsider material behind its own tab —
/// what returns with this work (RFC-0013), with the declared related work
/// and the declaration status right below, exactly as the old remembered
/// tier stacked them. The Related and Witnessed sections live on their own
/// tabs; the tab grammar is earned because these are genuinely different
/// sections, not a styling choice. State is egui temp data keyed by the
/// Workspace identity, the same reset rules the open-panel index follows.
fn record_body(ui: &mut egui::Ui, canon: &Canonical<'_>, tr: &mut Transient<'_>) {
    if state::distinct_artifacts_across_history(canon.workspace).is_empty() {
        ui::caption(
            ui,
            "Nothing to remember yet — Evo has not witnessed this work.",
        );
        return;
    }

    let tab = record_tab(ui, canon.workspace.id());
    record_tabs(ui, canon.workspace.id(), tab);
    match tab {
        RecordTab::Surface => reconsider(ui, canon, tr),
        RecordTab::Related => grouping(ui, canon, tr),
        RecordTab::Witnessed => witnessed_body(ui, canon),
    }
}

/// Which Record section is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordTab {
    /// What returns with this work (RFC-0013).
    Surface,
    /// What this work relates to (RFC-0012).
    Related,
    /// Everything Evo witnessed here.
    Witnessed,
}

impl RecordTab {
    const ALL: [RecordTab; 3] = [RecordTab::Surface, RecordTab::Related, RecordTab::Witnessed];
    fn label(self) -> &'static str {
        match self {
            RecordTab::Surface => "Surface",
            RecordTab::Related => "Related",
            RecordTab::Witnessed => "Witnessed",
        }
    }
    fn position(self) -> usize {
        match self {
            RecordTab::Surface => 0,
            RecordTab::Related => 1,
            RecordTab::Witnessed => 2,
        }
    }
}

/// The tab state, stored per workspace like the open-panel index.
fn record_tab(ui: &egui::Ui, workspace: &WorkspaceId) -> RecordTab {
    let id = egui::Id::new(("evo-stage-record-tab", workspace.to_string()));
    match ui.data(|data| data.get_temp::<usize>(id)).unwrap_or(0) {
        1 => RecordTab::Related,
        2 => RecordTab::Witnessed,
        _ => RecordTab::Surface,
    }
}

/// The mini tab strip under the Record panel's header: the active section
/// in the text ink with a teal underline (informational — a state, not a
/// promotion), the rest quiet. Each tab is a real widget with a stable id
/// and an accessible label.
fn record_tabs(ui: &mut egui::Ui, workspace: &WorkspaceId, active: RecordTab) {
    ui.horizontal(|ui| {
        for tab in RecordTab::ALL {
            let id = egui::Id::new(("evo-record-tab", tab.position(), workspace.to_string()));
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(
                    ui::measure(ui, Role::Caption, tab.label()).x + theme::S3,
                    24.0,
                ),
                egui::Sense::hover(),
            );
            // The stable id comes from this interact, not the allocation —
            // assistive tech and the harness aim at the same widget.
            let response = ui.interact(rect, id, egui::Sense::click());
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::SelectableLabel,
                    true,
                    tab.position() == active.position(),
                    format!("Record: {}", tab.label()),
                )
            });
            let is_active = tab == active;
            ui.painter().galley(
                rect.left_top() + egui::vec2(theme::S2, 4.0),
                ui.painter().layout_no_wrap(
                    tab.label().to_owned(),
                    Role::Caption.font(),
                    if is_active { theme::TEXT } else { theme::MUTED },
                ),
                Color32::WHITE,
            );
            if is_active {
                ui.painter().line_segment(
                    [
                        egui::pos2(rect.left() + theme::S2, rect.bottom() - 2.0),
                        egui::pos2(rect.right() - theme::S2, rect.bottom() - 2.0),
                    ],
                    egui::Stroke::new(2.0, theme::TEAL),
                );
            }
            if response.clicked() {
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new(("evo-stage-record-tab", workspace.to_string())),
                        tab.position(),
                    )
                });
            }
        }
    });
    ui::gap(ui, theme::S2);
}

/// The standing in one honest word for the stage's teal eyebrow.
fn standing_label(standing: evo_engagement::Standing) -> &'static str {
    match standing {
        evo_engagement::Standing::Remembered => "Remembered",
        evo_engagement::Standing::Continuable => "Continuable",
        evo_engagement::Standing::Restorable => "Restorable",
    }
}

// ────────────────────────────────────────────────────────────────────────
// Navigation between Places
// ────────────────────────────────────────────────────────────────────────

/// The way out of this Place, and the way across to another.
///
/// "All work" returns Home. The switcher steps sideways into another body of
/// work without going Home first — the Place-to-Place move the mental model
/// wants ("…Automation X → Automation Y"). It reuses the one navigation path
/// there is: setting the selected Workspace identity. Nothing about the target
/// Place is derived here; opening it derives its own state on the next frame,
/// exactly as a Home selection does.
fn place_nav(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    siblings: &[state::WorkspaceCard],
    tr: &mut Transient<'_>,
) {
    if ui::control(ui, Control::quiet("← All work")).clicked() {
        *tr.selected = None;
    }

    let current = canon.workspace.id();
    // Only work is offered as a sideways step. Remembered bodies stay findable
    // by name from Home; they are not thrown into the switcher (Home's own
    // work/remembered split is the authority on that).
    let others: Vec<&state::WorkspaceCard> = siblings
        .iter()
        .filter(|card| card.standing.is_work() && &card.id != current)
        .collect();
    if others.is_empty() {
        return;
    }

    ui::gap(ui, theme::S2);
    let open = ui::disclosure(
        ui,
        ui.id().with("evo-switch-work"),
        &format!("Switch to other work ({})", others.len()),
    );
    if !open {
        return;
    }
    ui::gap(ui, theme::S2);
    // A bounded set of doorways, not a directory. The rest stay one step away
    // through "All work" — the switcher is for stepping between the few things
    // you are actually moving among, never a second Home.
    const SWITCH_CAP: usize = 6;
    for card in others.iter().take(SWITCH_CAP) {
        let id = ui.id().with(("evo-switch", card.id.to_string()));
        let (row, _) = ui::row(ui, id, |ui| {
            ui.horizontal_top(|ui| {
                ui::one_line(ui, Role::Body, &card.title, theme::TEXT);
            });
        });
        if row.clicked() {
            *tr.selected = Some(card.id.clone());
        }
    }
    if others.len() > SWITCH_CAP {
        ui::gap(ui, theme::S2);
        ui::caption(
            ui,
            &format!(
                "{} more — open “All work” to find them.",
                others.len() - SWITCH_CAP
            ),
        );
    }
}

// ────────────────────────────────────────────────────────────────────────
// Identity
/// A designation that points outside this body of work. Stated where the
/// answer would otherwise be, because it explains why the answer is missing.
fn designation_elsewhere(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    candidates: &[(String, String)],
) {
    let Some((subject, recorded)) = canon.designation else {
        return;
    };
    if candidates.iter().any(|(_, known)| known == subject) {
        return;
    }
    ui::notice(
        ui,
        Severity::Caution,
        "Marked elsewhere",
        &format!(
            "You marked “{subject}” (at {}) as where you continue, but Evo's \
             record does not place it in this body of work.",
            state::local_time_label(*recorded)
        ),
    );
    ui::gap(ui, theme::S3);
}

/// The Resume Point: the largest, quietest thing on the screen.
fn resume_answer(ui: &mut egui::Ui, canon: &Canonical<'_>, artifact_id: &str) {
    let subject = state::subject_for(canon.subjects, artifact_id);
    match subject {
        Some(subject) => {
            ui::label(ui, Role::Display, subject, theme::TEXT);
            ui::gap(ui, theme::S2);
        }
        None => ui::unknown(ui, "Evo's record does not name this resource."),
    }

    if let Some(kind) = state::kind_for(canon.kinds, artifact_id) {
        ui::caption(ui, &sentence_case(kind));
    }

    // Why *this* one. Under IS-0021 §25.2 there are exactly two grounds: the
    // user marked it, or the record holds a single resource. Neither is
    // recency, and the sentence says so.
    let marked = canon
        .designation
        .is_some_and(|(designated, _)| Some(designated.as_str()) == subject);
    if let (true, Some((_, recorded))) = (marked, canon.designation) {
        ui::provenance(
            ui,
            &format!(
                "Because you marked it, at {}. Evo does not guess and does not rank.",
                state::local_time_label(*recorded)
            ),
        );
    } else {
        ui::provenance(
            ui,
            "Because Evo's record holds exactly one resource for this work. \
             Nothing was ranked or guessed.",
        );
    }
}

/// What Evo has derived as the step this work continues into.
fn next_step(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    let Some(next) = canon.outcome.and_then(|outcome| outcome.next_step()) else {
        return;
    };
    ui::gap(ui, theme::S5);
    ui::eyebrow(ui, "Next step");
    ui::label(ui, Role::BodyStrong, next.description(), theme::TEXT);
    ui::gap(ui, theme::S1);
    ui::provenance(
        ui,
        "Derived from what Evo witnessed. It is a note to you, not an action Evo will take.",
    );
}

/// Derived blockers.
///
/// These sit above the action rather than at the bottom of the screen: a
/// blocker changes what pressing `Continue` means, so it has to be readable
/// before the user reaches for it. They are set quietly — the heading carries
/// the meaning, so no per-line badge is needed.
fn blockers(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    let blockers = canon
        .outcome
        .map(|outcome| outcome.blockers())
        .unwrap_or(&[]);
    if blockers.is_empty() {
        return;
    }
    ui::gap(ui, theme::S5);
    ui::eyebrow(ui, "In the way");
    for blocker in blockers {
        ui::label(
            ui,
            Role::Body,
            &format!("— {}", blocker.description()),
            theme::TEXT,
        );
        ui::gap(ui, theme::S1);
    }
}

/// Whether the primary Continue act is ready.
///
/// Continue is enabled exactly when the shared ordering function — the same
/// one `state::run_execution` consumes — announces at least one target that
/// will actually be attempted (UI-VALIDATION-0001 B.9: the promise and the
/// act derive from one shared function). A declared Continuation Surface
/// (RFC-0013) is the restore candidate set even when derivation is
/// Insufficient with no Resume Point (IS-0021 §25.2), so the gate follows
/// the plan, never the Resume Point alone. Crate-visible because the
/// transport dock's gate is the same gate — one definition, two readers.
pub(crate) fn continue_ready(canon: &Canonical<'_>) -> bool {
    plan_line(canon).is_some()
}

/// One planned step that has no result yet: a pending mark, what it concerns,
/// and what Evo will try.
///
/// A gentle pulse on the mark, so the row reads as work underway rather than a
/// frozen list; it is driven by the shell's per-frame repaint request while a
/// restoration is in flight. Each row settles in a little after the last, so
/// the plan reads as a sequence being made ready rather than a block that
/// appeared at once.
fn pending_line(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    index: usize,
    artifact_id: &str,
    action: &str,
) {
    let label = state::subject_for(canon.subjects, artifact_id)
        .map(|_| state::describe_artifact(canon.kinds, canon.subjects, artifact_id))
        .unwrap_or_else(|| sentence_case(action));
    let reveal = ui.ctx().animate_bool_with_time(
        ui.id().with(("evo-prepare", index)),
        true,
        theme::RESPONSE_IN + 0.05 * index as f32,
    );
    let time = ui.input(|i| i.time) as f32;
    let pulse = 0.45 + 0.55 * (0.5 + 0.5 * (time * 3.2 + index as f32 * 0.7).sin());
    ui.scope(|ui| {
        ui.set_opacity(theme::settle(reveal));
        ui.horizontal_top(|ui| {
            ui.scope(|ui| {
                ui.set_opacity(pulse);
                ui::label(ui, Role::Body, "○", theme::MUTED);
            });
            ui.add_space(theme::S3);
            ui.vertical(|ui| {
                ui::one_line(ui, Role::Body, &label, theme::TEXT);
                ui::gap(ui, theme::S1);
                ui::label(ui, Role::Caption, action, theme::MUTED);
            });
        });
    });
    ui::gap(ui, theme::S3);
}

/// The ordered locator plan, in the executor's own order.
///
/// Describes exactly the targets `state::run_execution` will attempt, from
/// the same shared ordering function: a declared Continuation Surface is
/// announced as its restore-worthy members in canonical order, otherwise the
/// derived Resume Point first and the Context Chain after (BE-AUDIT-0001
/// §2.4). A refusal is not a plan, so there is no plan line to state; the
/// caller already says so honestly rather than guessing.
fn plan_line(canon: &Canonical<'_>) -> Option<String> {
    let targets = plan_targets(canon);
    if targets.is_empty() {
        None
    } else {
        Some(
            targets
                .into_iter()
                .map(|(_, action)| action)
                .collect::<Vec<_>>()
                .join(" · "),
        )
    }
}

/// The ordered targets Continue will attempt, each paired with the sentence
/// describing its action, in the executor's own order.
///
/// Empty when nothing resolves to an openable target. This is the single
/// source both the announced plan line and the restoring transition read from,
/// so the promise, the transition, and the act cannot drift apart
/// (UI-VALIDATION-0001 B.9): all three are this one function's output.
fn plan_targets(canon: &Canonical<'_>) -> Vec<(String, String)> {
    let Some(outcome) = canon.outcome else {
        return Vec::new();
    };
    let Ok(targets) = evo_execution::ordered_attempt_targets(Some(outcome), canon.selection) else {
        return Vec::new();
    };
    targets
        .iter()
        .filter_map(|artifact_id| {
            let locator = canon.locators.get(artifact_id.as_str())?;
            Some((
                artifact_id.as_str().to_string(),
                locator_action(locator.kind()),
            ))
        })
        .collect()
}

/// What Evo will try for a canonical executable target, in one place, so the
/// announcement, the transition, and the restoration in flight cannot describe
/// the same act in different words.
fn locator_action(kind: &LocatorKind) -> String {
    match kind {
        LocatorKind::WindowTitle(title) => format!("focus window “{title}”"),
        LocatorKind::FilePath(path) => format!("open file {path}"),
        LocatorKind::Url(url) => format!("open URL {url}"),
    }
}

/// What a decided step will try — the same words the announcement used.
///
/// A refused step will try nothing, and says so. Its executor-given reason
/// arrives as its result, not as a promise here.
fn step_action(step: &RestorationStep) -> String {
    match step.plan() {
        StepPlan::Attempt(kind) => locator_action(kind),
        StepPlan::Refuse(_) => "not attempted".to_string(),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Designation
// ────────────────────────────────────────────────────────────────────────

/// The screen when Evo has no Resume Point: honest about what it lacks, and
/// immediately actionable.
fn unresolved(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    candidates: &[(String, String)],
) {
    if candidates.is_empty() {
        ui::headline(ui, "Not enough witnessed yet");
        ui::prose(
            ui,
            "Evo remembers this body of work, but its record does not yet hold \
             a resource it could reopen. Keep working — Evo will record what it \
             witnesses.",
        );
        return;
    }

    ui::headline(ui, "You haven't marked where this continues");
    ui::prose(
        ui,
        "Evo witnessed several resources here and will not choose between them \
         for you. Mark the one your work continues from.",
    );
    origin_list(ui, canon, tr, candidates);
}

/// The candidates for a continuation origin.
///
/// The mark is round and exclusive: choosing one supersedes the last, which is
/// how RFC-0011 defines a designation. Nothing here is a draft — a designation
/// is recorded the moment it is made, and the reply says so.
fn origin_list(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    candidates: &[(String, String)],
) {
    ui::gap(ui, theme::S4);
    for (artifact_id, subject) in candidates {
        let chosen = canon
            .designation
            .is_some_and(|(designated, _)| designated == subject);
        let label = state::describe_artifact(canon.kinds, canon.subjects, artifact_id);
        let id = ui.id().with(("evo-origin", artifact_id.as_str()));
        let (row, mark) = ui::row(ui, id, |ui| {
            ui.horizontal_top(|ui| {
                let mark = ui::origin_mark(ui, id.with("mark"), chosen);
                ui.add_space(theme::S3);
                ui::one_line(
                    ui,
                    Role::Body,
                    &label,
                    if chosen { theme::TEXT } else { theme::MUTED },
                );
                mark
            })
            .inner
        });
        // `||` short-circuits, so exactly one record is submitted no matter
        // which of the two widgets egui awarded the click to.
        if (row.clicked() || mark.clicked()) && !chosen {
            *tr.designation_status = Some(record_designation(tr.storage_root, subject));
        }
    }
    designation_answer(ui, tr);
}

/// The daemon's reply to the last designation.
fn designation_answer(ui: &mut egui::Ui, tr: &mut Transient<'_>) {
    let Some((message, accepted)) = tr.designation_status.as_ref() else {
        return;
    };
    ui::gap(ui, theme::S3);
    let (severity, word) = if *accepted {
        (Severity::Good, "Recorded")
    } else {
        (Severity::Bad, "Not recorded")
    };
    ui::notice(ui, severity, word, message);
}

/// Submits a designation and reports, verbatim, what the daemon said.
fn record_designation(root: &Path, subject: &str) -> (String, bool) {
    match state::submit_designation(root, subject) {
        Ok(DesignationResponse::Accepted) => {
            ("Evo will continue from this resource.".to_string(), true)
        }
        Ok(DesignationResponse::Rejected(reason)) => {
            (format!("Evo could not record that: {reason}"), false)
        }
        Err(error) => (
            format!("Evo could not reach the capture worker: {error}"),
            false,
        ),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Execution
// ────────────────────────────────────────────────────────────────────────

/// One attempt, with its status word, what it concerned, and the executor's
/// own reason where it gave one.
///
/// The lines materialise in order, each a little later than the last, so the
/// report reads as a sequence of things that happened rather than a block that
/// appeared. The stagger is carried by the transition's response, so an
/// interrupted or re-run execution never has to wait for it to finish.
fn attempt_line(ui: &mut egui::Ui, canon: &Canonical<'_>, index: usize, attempt: &TargetAttempt) {
    let artifact_id = attempt.artifact_id().as_str();
    let (word, severity) = match attempt.status() {
        TargetStatus::Opened { .. } => ("Opened", Severity::Good),
        TargetStatus::Failed { .. } => ("Failed", Severity::Bad),
        TargetStatus::Unavailable { .. } => ("Unavailable", Severity::Caution),
        TargetStatus::Ambiguous { .. } => ("Ambiguous", Severity::Caution),
        TargetStatus::Unsupported { .. } => ("Unsupported", Severity::Neutral),
    };
    let reason = match attempt.status() {
        TargetStatus::Opened { detail } => detail.as_str(),
        TargetStatus::Failed { reason } => reason.as_str(),
        TargetStatus::Unavailable { reason } => reason.as_str(),
        TargetStatus::Ambiguous { reason } => reason.as_str(),
        TargetStatus::Unsupported { reason } => reason.as_str(),
    };
    let label = state::subject_for(canon.subjects, artifact_id)
        .map(|_| state::describe_artifact(canon.kinds, canon.subjects, artifact_id));

    let reveal = ui.ctx().animate_bool_with_time(
        ui.id().with(("evo-attempt", index)),
        true,
        theme::RESPONSE_IN + 0.05 * index as f32,
    );
    ui.scope(|ui| {
        ui.set_opacity(theme::settle(reveal));
        ui.horizontal_top(|ui| {
            ui::status_word(ui, word, severity);
            ui.add_space(theme::S3);
            ui.vertical(|ui| {
                match label {
                    Some(label) => {
                        ui::one_line(ui, Role::Body, &label, theme::TEXT);
                    }
                    None => {
                        ui::one_line(
                            ui,
                            Role::Body,
                            "A resource Evo's record does not name",
                            theme::MUTED,
                        );
                    }
                }
                if !reason.is_empty() {
                    ui::gap(ui, theme::S1);
                    ui::label(ui, Role::Caption, reason, theme::MUTED);
                }
            });
        });
    });
    ui::gap(ui, theme::S3);
}

// ────────────────────────────────────────────────────────────────────────
/// One member of the current continuation, shown for reading, not editing.
///
/// The disposition word is the canonical [`PreflightStatus::label`], never a
/// phrasing invented here; the reason is whatever canonical explanation exists.
/// There is no mark: changing what comes back is a deliberate act that lives in
/// the remembered tier, not something the calm glance offers.
fn display_row(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    artifact_id: &str,
    disposition: (&'static str, Severity),
    reason: Option<&str>,
) {
    if state::subject_for(canon.subjects, artifact_id).is_none() {
        return;
    }
    let label = state::describe_artifact(canon.kinds, canon.subjects, artifact_id);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui::one_line(ui, Role::Body, &label, theme::TEXT);
            ui::gap(ui, theme::S1);
            let (word, severity) = disposition;
            ui::status_word(ui, word, severity);
            if let Some(reason) = reason
                && !reason.is_empty()
            {
                ui::gap(ui, theme::S1);
                ui::label(ui, Role::Caption, reason, theme::MUTED);
            }
        });
    });
    ui::gap(ui, theme::S3);
}

// ────────────────────────────────────────────────────────────────────────
/// Reconsidering what comes back with this work (RFC-0013 Continuation
/// Surface).
///
/// The editable counterpart to the read-only "used alongside" tier: current
/// members can be unmarked, withheld membership can be marked in, and the whole
/// draft declared. It lives inside the quiet remembered tier because changing
/// what returns is a deliberate act, not the default glance — but it is never
/// more than one disclosure away, and the write path is fully wired.
fn reconsider(ui: &mut egui::Ui, canon: &Canonical<'_>, tr: &mut Transient<'_>) {
    let Some(selection) = canon.selection else {
        return;
    };

    ui::eyebrow(ui, "Reconsider what comes back");

    let current: BTreeSet<String> = state::surface_subjects(selection, canon.subjects)
        .into_iter()
        .collect();
    let statuses: HashMap<&str, &PreflightStatus> = canon
        .preflight
        .iter()
        .map(|outcome| (outcome.artifact_id().as_str(), outcome.status()))
        .collect();

    if current.is_empty() {
        ui::caption(
            ui,
            "Evo's record does not yet show a current continuation for this body \
             of work. Mark the resources it continues across — Evo never assumes \
             that where you continue from is everything that comes back with you.",
        );
    } else {
        ui::caption(ui, "Unmark anything that should not return with this work.");
        ui::gap(ui, theme::S3);

        // Reopenable first, then not — the per-row status word carries the
        // distinction, so no subheading is needed to repeat it.
        for selected in selection.restore_worthy() {
            let artifact_id = selected.artifact_id().as_str();
            let status = statuses.get(artifact_id).copied();
            member_row(
                ui,
                canon,
                tr,
                artifact_id,
                &current,
                Some(preflight_word(status, false)),
                status.and_then(PreflightStatus::reason),
            );
        }
        for item in selection.unavailable() {
            let artifact_id = item.artifact_id().as_str();
            let status = statuses.get(artifact_id).copied();
            let reason = status
                .and_then(PreflightStatus::reason)
                .unwrap_or_else(|| item.reason());
            member_row(
                ui,
                canon,
                tr,
                artifact_id,
                &current,
                Some(preflight_word(status, true)),
                Some(reason),
            );
        }
    }

    related(ui, canon, tr, selection, &current);
    declaration(ui, tr);
}

/// The user-declared related-work surface (RFC-0012 WorkGrouped).
///
/// The user may declare that this body of work is related to another
/// witnessed resource — a browser tab, a window, a file in another folder —
/// that Evo's canonical co-membership evidence cannot relate on its own. The
/// declaration is authoritative user evidence, exactly as RFC-0012 defines
/// WorkGrouped: it is never inferred, never scored, never guessed. The shell
/// only forwards the explicit declaration through the daemon's trusted
/// channel and reflects the daemon's honest response.
fn grouping(ui: &mut egui::Ui, canon: &Canonical<'_>, tr: &mut Transient<'_>) {
    // The anchor is the Workspace's own first witnessed resource in canonical
    // order — the same deterministic rule the Home card uses for its title.
    // The user then declares that another witnessed resource belongs with it.
    let Some(anchor) = state::distinct_artifacts_across_history(canon.workspace)
        .first()
        .and_then(|id| state::subject_for(canon.subjects, id))
    else {
        return;
    };
    let anchor = anchor.to_string();

    // Other witnessed resources: every subject Evo has witnessed that is NOT
    // a member of this Workspace. Presentation-only candidates; the daemon
    // re-validates both subjects against the canonical log before accepting.
    let own: BTreeSet<&str> = state::distinct_artifacts_across_history(canon.workspace)
        .iter()
        .filter_map(|id| state::subject_for(canon.subjects, id))
        .collect();
    let mut others: Vec<&str> = canon
        .subjects
        .values()
        .map(String::as_str)
        .filter(|subject| !own.contains(subject) && *subject != anchor)
        .collect();
    others.sort_unstable();
    others.dedup();
    if others.is_empty() {
        return;
    }

    ui::gap(ui, theme::S5);
    ui::hairline(ui);
    ui::gap(ui, theme::S4);
    ui::eyebrow(ui, "Related work — your call");
    ui::caption(
        ui,
        "Evo never guesses that two resources belong to the same body of work. \
         You can say so: declare that another witnessed resource is part of \
         this work.",
    );
    ui::gap(ui, theme::S3);
    // The candidate list is gated behind its own count, so opening "remembered"
    // never spills every other-workspace subject at once. Declaring related
    // work is a deliberate reach, not something the tier presents by default.
    let open = ui::disclosure(
        ui,
        ui.id().with("evo-group-list"),
        &format!("Show {}", plural(others.len(), "other resource")),
    );
    if open {
        ui::gap(ui, theme::S3);
        for other in others {
            let id = ui.id().with(("evo-group", other));
            let (row, button) = ui::row(ui, id, |ui| {
                ui.horizontal_top(|ui| {
                    ui::one_line(ui, Role::Body, other, theme::MUTED);

                    ui::control(ui, Control::quiet("Declare related"))
                })
                .inner
            });
            if row.clicked() || button.clicked() {
                *tr.grouping_status = Some(record_grouping(tr.storage_root, &anchor, other));
            }
        }
    }
    if let Some((message, accepted)) = tr.grouping_status.as_ref() {
        ui::gap(ui, theme::S3);
        let (severity, word) = if *accepted {
            (Severity::Good, "Recorded")
        } else {
            (Severity::Bad, "Not recorded")
        };
        ui::notice(ui, severity, word, message);
    }
}

/// Submits a WorkGrouped declaration (RFC-0012) and reports, verbatim, what
/// the daemon said.
fn record_grouping(root: &Path, first: &str, second: &str) -> (String, bool) {
    match state::submit_grouping(root, first, second) {
        Ok(GroupingResponse::Accepted) => (
            "Evo recorded that these resources belong to the same body of work.".to_string(),
            true,
        ),
        Ok(GroupingResponse::Rejected(reason)) => {
            (format!("Evo could not record that: {reason}"), false)
        }
        Err(error) => (
            format!("Evo could not reach the capture worker: {error}"),
            false,
        ),
    }
}

/// Membership that is deliberately outside the current continuation.
///
/// Two sections, because the selection layer derived two answers and they are
/// not the same answer. *Available if needed* is work: the person worked in it
/// or consulted it, and Evo is holding it back only because it is not the
/// continuation being returned to. *History only* is everything Evo happened to
/// witness while the work was going on. Showing them as one list is how a
/// restoration turns back into a list of everything that happened.
fn related(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    selection: &RestorationSelection,
    current: &BTreeSet<String>,
) {
    if selection.withheld().is_empty() {
        return;
    }
    ui::gap(ui, theme::S5);
    ui::hairline(ui);
    ui::gap(ui, theme::S4);

    let to_hand: Vec<&WithheldMember> = selection.available_if_needed().collect();
    let record: Vec<&WithheldMember> = selection.history_only().collect();

    if !to_hand.is_empty() {
        ui::eyebrow(ui, "Part of this work — not reopened");
        ui::caption(
            ui,
            "Evo will not reopen these, and has not put them away either. \
             Marking one moves it into the current continuation.",
        );
        withheld_rows(ui, canon, tr, &to_hand, current, "evo-related-to-hand");
    }

    if !record.is_empty() {
        if !to_hand.is_empty() {
            ui::gap(ui, theme::S4);
        }
        ui::eyebrow(ui, "Witnessed at the time — kept in the record");
        ui::caption(
            ui,
            "Nothing beyond having been there ties these to this work. They stay \
             findable, and you can still say otherwise.",
        );
        withheld_rows(ui, canon, tr, &record, current, "evo-related-record");
    }

    ui::gap(ui, theme::S2);
    ui::caption(
        ui,
        "Membership is never removed by time. Marking one moves it into the \
         current continuation; unmarking moves it out.",
    );
}

/// One collapsed section of withheld membership.
///
/// Each row shows the canonical [`evo_execution::Disposition::label`] and the
/// canonical reason the selection layer attached — never a phrasing invented
/// here.
fn withheld_rows(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    members: &[&WithheldMember],
    current: &BTreeSet<String>,
    id_salt: &'static str,
) {
    let open = ui::disclosure(
        ui,
        ui.id().with(id_salt),
        &format!("Show {}", plural(members.len(), "resource")),
    );
    if !open {
        return;
    }
    ui::gap(ui, theme::S3);
    for member in members {
        member_row(
            ui,
            canon,
            tr,
            member.artifact_id().as_str(),
            current,
            Some((member.disposition().label(), Severity::Neutral)),
            Some(member.reason()),
        );
    }
}

/// One member of this body of work.
///
/// `disposition` is the canonical word for what happens to this member — from
/// [`PreflightStatus::label`] for the current continuation, or
/// [`evo_execution::Disposition::label`] for withheld membership. Neither
/// vocabulary is invented here. `reason` is whatever canonical explanation
/// exists — never a phrasing of this module's own.
fn member_row(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    artifact_id: &str,
    current: &BTreeSet<String>,
    disposition: Option<(&'static str, Severity)>,
    reason: Option<&str>,
) {
    let Some(subject) = state::subject_for(canon.subjects, artifact_id) else {
        return;
    };
    let subject = subject.to_string();
    let label = state::describe_artifact(canon.kinds, canon.subjects, artifact_id);

    let declared = current.contains(&subject);
    let member = match tr.continuation_draft.as_ref() {
        Some(draft) => draft.contains(&subject),
        None => declared,
    };
    let drafted = tr.continuation_draft.is_some() && member != declared;

    let ink = if !member {
        theme::RECEDED
    } else if drafted {
        theme::MUTED
    } else {
        theme::TEXT
    };

    let id = ui.id().with(("evo-member", artifact_id));
    let (row, mark) = ui::row(ui, id, |ui| {
        ui.horizontal_top(|ui| {
            let mark = ui::set_mark(ui, id.with("mark"), member, drafted);
            ui.add_space(theme::S3);
            ui.vertical(|ui| {
                ui::one_line(ui, Role::Body, &label, ink);
                if let Some((word, severity)) = disposition {
                    ui::gap(ui, theme::S1);
                    ui::status_word(ui, word, severity);
                }
                if let Some(reason) = reason
                    && !reason.is_empty()
                {
                    ui::gap(ui, theme::S1);
                    ui::label(ui, Role::Caption, reason, theme::MUTED);
                }
            });
            mark
        })
        .inner
    });
    // `||` short-circuits, so exactly one toggle results no matter which of
    // the two widgets egui awarded the click to.
    if row.clicked() || mark.clicked() {
        toggle(tr, current, &subject);
    }
}

/// The status word for one member of the current continuation.
///
/// The word comes from [`PreflightStatus::label`] so the UI cannot drift from
/// the canonical `READY` / `UNAVAILABLE` / `AMBIGUOUS` / `UNSUPPORTED`
/// vocabulary. A missing entry is reported as unchecked rather than assumed
/// reopenable; `UNAVAILABLE` without an entry is not a guess, because
/// restoration itself placed the resource in the unavailable set.
fn preflight_word(
    status: Option<&PreflightStatus>,
    known_unavailable: bool,
) -> (&'static str, Severity) {
    match status {
        Some(status) => (
            status.label(),
            match status {
                PreflightStatus::Ready => Severity::Good,
                PreflightStatus::Unavailable { .. } | PreflightStatus::Ambiguous { .. } => {
                    Severity::Caution
                }
                PreflightStatus::Unsupported { .. } => Severity::Neutral,
            },
        ),
        None if known_unavailable => ("UNAVAILABLE", Severity::Caution),
        None => ("NOT CHECKED", Severity::Neutral),
    }
}

/// Moves one subject into or out of the draft.
///
/// The draft starts as a copy of what Evo currently holds, so the first mark
/// the user makes is an edit to Evo's understanding rather than a fresh empty
/// set. Nothing here is recorded.
fn toggle(tr: &mut Transient<'_>, current: &BTreeSet<String>, subject: &str) {
    *tr.continuation_status = None;
    let mut next = tr
        .continuation_draft
        .clone()
        .unwrap_or_else(|| current.clone());
    if next.contains(subject) {
        next.remove(subject);
    } else {
        next.insert(subject.to_string());
    }
    *tr.continuation_draft = Some(next);
}

/// The declaration control, offered only once there is something to declare.
fn declaration(ui: &mut egui::Ui, tr: &mut Transient<'_>) {
    ui::gap(ui, theme::S5);

    if tr.continuation_draft.is_some() {
        ui::caption(
            ui,
            "You've changed what's marked. Nothing is recorded until you declare it.",
        );
        if ui::control(ui, Control::secondary("Declare continuation")).clicked() {
            let subjects: Vec<String> = tr
                .continuation_draft
                .as_ref()
                .map(|draft| draft.iter().cloned().collect())
                .unwrap_or_default();
            let outcome = record_surface(tr.storage_root, &subjects);
            let accepted = outcome.1;
            *tr.continuation_status = Some(outcome);
            if accepted {
                *tr.continuation_draft = None;
            }
        }
    } else {
        ui::caption(
            ui,
            "Marking a resource changes what Evo would reopen. You declare when you're ready.",
        );
    }

    if let Some((message, accepted)) = tr.continuation_status.as_ref() {
        ui::gap(ui, theme::S3);
        let (severity, word) = if *accepted {
            (Severity::Good, "Recorded")
        } else {
            (Severity::Bad, "Not recorded")
        };
        ui::notice(ui, severity, word, message);
    }
}

/// Submits a Continuation Surface declaration and reports the daemon's answer.
///
/// The subjects are submitted in canonical order, which `BTreeSet` already
/// guarantees; the daemon canonicalises again regardless.
fn record_surface(root: &Path, subjects: &[String]) -> (String, bool) {
    match state::submit_continuation_surface(root, subjects) {
        Ok(ContinuationResponse::Accepted) => {
            ("Evo updated the current continuation.".to_string(), true)
        }
        Ok(ContinuationResponse::Rejected(reason)) => {
            (format!("Evo could not record that: {reason}"), false)
        }
        Err(error) => (
            format!("Evo could not reach the capture worker: {error}"),
            false,
        ),
    }
}

// ────────────────────────────────────────────────────────────────────────
// What Evo witnessed
// ────────────────────────────────────────────────────────────────────────

/// The evidence, rendered inside the remembered tier's quiet surface.
///
/// This is the deepest thing on the screen by design: it is why Evo believes
/// what it believes, which matters, but it is not what the user came for — so
/// it sits behind the remembered disclosure, never on its own surface (two
/// translucent layers must not stack). The latest witnessed moment is
/// presentation-only information about the record's most recent append — never
/// a reason anything is restored.
fn witnessed_body(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    ui::gap(ui, theme::S5);
    ui::hairline(ui);
    ui::gap(ui, theme::S4);
    ui::eyebrow(ui, "What Evo witnessed");

    match canon.workspace.snapshots().last() {
        Some(latest) => {
            ui::field(ui, "Latest moment", &state::snapshot_time_label(latest));
            ui::gap(ui, theme::S3);
            let recent = state::distinct_artifacts_in_latest_snapshot(canon.workspace);
            if recent.is_empty() {
                ui::unknown(ui, "Nothing was witnessed in that moment.");
            } else {
                for artifact_id in &recent {
                    ui::one_line(
                        ui,
                        Role::Body,
                        &format!(
                            "— {}",
                            state::describe_artifact(canon.kinds, canon.subjects, artifact_id)
                        ),
                        theme::MUTED,
                    );
                    ui::gap(ui, theme::S1);
                }
            }
        }
        None => {
            ui::unknown(
                ui,
                "Evo has not witnessed a moment in this body of work yet.",
            );
            return;
        }
    }

    let all = state::distinct_artifacts_across_history(canon.workspace);
    if all.is_empty() {
        return;
    }
    ui::gap(ui, theme::S5);
    ui::field(
        ui,
        "Across this work",
        &format!(
            "{}, over {}",
            plural(all.len(), "resource"),
            plural(canon.workspace.snapshots().len(), "moment")
        ),
    );
    ui::gap(ui, theme::S2);
    let open = ui::disclosure(ui, ui.id().with("evo-history"), "Show every resource");
    if !open {
        return;
    }
    ui::gap(ui, theme::S3);
    for artifact_id in all.iter().take(MAX_RENDERED_RESOURCES) {
        ui::one_line(
            ui,
            Role::Body,
            &format!(
                "— {}",
                state::describe_artifact(canon.kinds, canon.subjects, artifact_id)
            ),
            theme::MUTED,
        );
        ui::gap(ui, theme::S1);
    }
    if all.len() > MAX_RENDERED_RESOURCES {
        ui::gap(ui, theme::S2);
        ui::caption(
            ui,
            &format!(
                "Showing the first {MAX_RENDERED_RESOURCES} of {}. The record holds them all.",
                all.len()
            ),
        );
    }
}

// ────────────────────────────────────────────────────────────────────────
// Wording
// ────────────────────────────────────────────────────────────────────────

/// Capitalises a witnessed kind label for use at the start of a line.
///
/// Purely typographic: the canonical kind string is untouched.
/// The widget id the stage panel-card at `index` answers to. Global and
/// stable, like the rack card ids, so assistive tech and the headless
/// harness aim at the same object.
#[allow(dead_code)]
fn stage_panel_widget_id(index: usize) -> egui::Id {
    egui::Id::new(("evo-stage-panel", index))
}

fn sentence_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// `1 resource` / `2 resources`. Avoids the `(s)` that reads as machine output.
fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_label_is_capitalized_without_being_rewritten() {
        assert_eq!(sentence_case("focused window"), "Focused window");
        assert_eq!(sentence_case("saved file"), "Saved file");
        assert_eq!(sentence_case(""), "");
        // Already capitalized text survives untouched.
        assert_eq!(sentence_case("URL"), "URL");
    }

    // FIX 2 (BE-AUDIT-0001 §2.4): the announced plan line must describe
    // exactly what execution will attempt, in exactly the attempted order, on
    // both branches. This is the regression test whose absence let the
    // announcement and the act diverge (audit §13.2(a)).
    #[test]
    fn plan_line_describes_exactly_what_execution_will_attempt() {
        use evo_artifact::artifact_id::ArtifactId;
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};
        use evo_workspace::attachment::{Attachment, ResourceRole};
        use evo_workspace::confidence::ConfidenceScore;
        use evo_workspace::lifecycle::WorkspaceLifecycle;
        use evo_workspace::snapshot::Snapshot;
        use evo_workspace::workspace_id::WorkspaceId;
        use std::collections::HashMap;
        use std::str::FromStr;

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174950").unwrap();
        let snapshot = Snapshot::new(
            std::time::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![
                Attachment::new(
                    ArtifactId::new("artifact-zz").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    // These fixtures exercise announcement ordering, which
                    // reads the surface rather than the roles; Primary is the
                    // plain "the work happens here" role.
                    ResourceRole::Primary,
                ),
                Attachment::new(
                    ArtifactId::new("artifact-aa").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    ResourceRole::Primary,
                ),
            ],
        );
        let ws = Workspace::new(
            workspace_id.clone(),
            WorkspaceLifecycle::Active,
            snapshot.attachments().to_vec(),
            vec![snapshot],
        );

        // Surface branch: the Resume Point (artifact-zz) is announced first,
        // because "where the work continues" sorts before everything else
        // (IS-0019 RSP-1; plan_selection uses restoration_key).
        let plan = RestorationPlan::new_with_continuation_surface(
            workspace_id.clone(),
            ResumePoint::new(
                workspace_id.clone(),
                ArtifactId::new("artifact-zz").unwrap(),
            ),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
            vec![
                ArtifactId::new("artifact-zz").unwrap(),
                ArtifactId::new("artifact-aa").unwrap(),
            ],
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);
        let mut evidence = HashMap::new();
        evidence.insert(
            "artifact-zz".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/zz.md".to_string()),
        );
        evidence.insert(
            "artifact-aa".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/aa.md".to_string()),
        );
        let members: Vec<(ArtifactId, ResourceRole)> = ws
            .attachments()
            .iter()
            .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
            .collect();
        let selection = evo_execution::select_restoration(&outcome, &members, &evidence);
        let mut locators = HashMap::new();
        locators.insert(
            "artifact-zz".to_string(),
            Locator::new(
                ArtifactId::new("artifact-zz").unwrap(),
                LocatorKind::FilePath("/repo/zz.md".into()),
            ),
        );
        locators.insert(
            "artifact-aa".to_string(),
            Locator::new(
                ArtifactId::new("artifact-aa").unwrap(),
                LocatorKind::FilePath("/repo/aa.md".into()),
            ),
        );
        let canon = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: Some(&selection),
            preflight: &[],
            subjects: &HashMap::new(),
            kinds: &HashMap::new(),
            titles: &HashMap::new(),
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert_eq!(
            plan_line(&canon).expect("a declared surface announces a plan"),
            "open file /repo/zz.md · open file /repo/aa.md"
        );

        // Non-surface branch: Resume Point first, then the Context Chain in
        // its canonical order — which is not ascending ArtifactId here.
        let plan = RestorationPlan::new(
            workspace_id.clone(),
            ResumePoint::new(
                workspace_id.clone(),
                ArtifactId::new("artifact-mid").unwrap(),
            ),
            ContextChain::new(vec![
                ArtifactId::new("artifact-z").unwrap(),
                ArtifactId::new("artifact-a").unwrap(),
            ])
            .unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);
        let mut locators = HashMap::new();
        locators.insert(
            "artifact-mid".to_string(),
            Locator::new(
                ArtifactId::new("artifact-mid").unwrap(),
                LocatorKind::FilePath("/repo/mid.md".into()),
            ),
        );
        locators.insert(
            "artifact-z".to_string(),
            Locator::new(
                ArtifactId::new("artifact-z").unwrap(),
                LocatorKind::FilePath("/repo/z.md".into()),
            ),
        );
        locators.insert(
            "artifact-a".to_string(),
            Locator::new(
                ArtifactId::new("artifact-a").unwrap(),
                LocatorKind::FilePath("/repo/a.md".into()),
            ),
        );
        let canon = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: None,
            preflight: &[],
            subjects: &HashMap::new(),
            kinds: &HashMap::new(),
            titles: &HashMap::new(),
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert_eq!(
            plan_line(&canon).expect("a Resume Point announces a plan"),
            "open file /repo/mid.md · open file /repo/z.md · open file /repo/a.md"
        );
    }

    #[test]
    fn counts_read_as_english() {
        assert_eq!(plural(0, "resource"), "0 resources");
        assert_eq!(plural(1, "resource"), "1 resource");
        assert_eq!(plural(2, "moment"), "2 moments");
    }

    // The primary Continue act is enabled exactly when the shared ordering
    // function (the one run_execution consumes) announces at least one target
    // that will actually be attempted. A declared Continuation Surface
    // (RFC-0013) is the restore candidate set even when the derivation is
    // Insufficient with no Resume Point (IS-0021 §25.2: several eligible
    // candidates, no designation) — the surface path of run_execution
    // executes it. The gate must therefore follow the plan, not the Resume
    // Point (UI-VALIDATION-0001 B.9: the promise and the act derive from one
    // shared function).
    #[test]
    fn a_declared_surface_enables_continue_without_a_resume_point() {
        use evo_artifact::artifact_id::ArtifactId;
        use evo_restoration::{RestorationInput, derive_restoration_plan};
        use evo_workspace::attachment::{Attachment, ResourceRole};
        use evo_workspace::confidence::ConfidenceScore;
        use evo_workspace::lifecycle::WorkspaceLifecycle;
        use evo_workspace::snapshot::Snapshot;
        use evo_workspace::workspace_id::WorkspaceId;
        use std::collections::HashMap;
        use std::str::FromStr;

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174952").unwrap();
        let snapshot = Snapshot::new(
            std::time::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![
                Attachment::new(
                    ArtifactId::new("artifact-aa").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    // Announcement ordering reads the surface, not the roles.
                    ResourceRole::Primary,
                ),
                Attachment::new(
                    ArtifactId::new("artifact-zz").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    ResourceRole::Primary,
                ),
            ],
        );
        let ws = Workspace::new(
            workspace_id.clone(),
            WorkspaceLifecycle::Active,
            snapshot.attachments().to_vec(),
            vec![snapshot],
        );
        // Two candidates, no designation → Insufficient with no Resume Point
        // (IS-0021 §25.2); the declared surface still carries the
        // continuation (RFC-0013).
        let surface = vec![
            ArtifactId::new("artifact-aa").unwrap(),
            ArtifactId::new("artifact-zz").unwrap(),
        ];
        let snapshot = ws.snapshots().last().unwrap();
        let input =
            RestorationInput::new_with_continuation_surface(&ws, snapshot, None, Some(surface))
                .unwrap();
        let outcome = derive_restoration_plan(&input);
        assert!(outcome.resume_point().is_none());
        assert_eq!(outcome.continuation_surface().len(), 2);

        let mut evidence = HashMap::new();
        evidence.insert(
            "artifact-aa".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/aa.md".to_string()),
        );
        evidence.insert(
            "artifact-zz".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/zz.md".to_string()),
        );
        let members: Vec<(ArtifactId, ResourceRole)> = ws
            .attachments()
            .iter()
            .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
            .collect();
        let selection = evo_execution::select_restoration(&outcome, &members, &evidence);
        let mut locators = HashMap::new();
        locators.insert(
            "artifact-aa".to_string(),
            Locator::new(
                ArtifactId::new("artifact-aa").unwrap(),
                LocatorKind::FilePath("/repo/aa.md".into()),
            ),
        );
        locators.insert(
            "artifact-zz".to_string(),
            Locator::new(
                ArtifactId::new("artifact-zz").unwrap(),
                LocatorKind::FilePath("/repo/zz.md".into()),
            ),
        );
        let canon = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: Some(&selection),
            preflight: &[],
            subjects: &HashMap::new(),
            kinds: &HashMap::new(),
            titles: &HashMap::new(),
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        // The Continue gate follows the shared plan: a declared surface with
        // no Resume Point still announces both members, so Continue is ready
        // — the regression the gate previously failed (it required a Resume
        // Point and left the declared continuation dead).
        assert_eq!(
            plan_line(&canon).expect("a declared surface without a Resume Point announces"),
            "open file /repo/aa.md · open file /repo/zz.md"
        );
        assert!(continue_ready(&canon));
    }

    // Continue is ready only when something can actually be opened: no
    // outcome, an undeclared multi-candidate Workspace, or a declared surface
    // whose members are all unavailable produce no plan, so the button stays
    // off and the reason stays visible.
    #[test]
    fn continue_is_ready_only_when_something_can_be_opened() {
        use evo_artifact::artifact_id::ArtifactId;
        use evo_restoration::{RestorationInput, derive_restoration_plan};
        use evo_workspace::attachment::{Attachment, ResourceRole};
        use evo_workspace::confidence::ConfidenceScore;
        use evo_workspace::lifecycle::WorkspaceLifecycle;
        use evo_workspace::snapshot::Snapshot;
        use evo_workspace::workspace_id::WorkspaceId;
        use std::collections::HashMap;
        use std::str::FromStr;

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174953").unwrap();
        let snapshot = Snapshot::new(
            std::time::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![
                Attachment::new(
                    ArtifactId::new("artifact-aa").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    // Announcement ordering reads the surface, not the roles.
                    ResourceRole::Primary,
                ),
                Attachment::new(
                    ArtifactId::new("artifact-zz").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    ResourceRole::Primary,
                ),
            ],
        );
        let ws = Workspace::new(
            workspace_id.clone(),
            WorkspaceLifecycle::Active,
            snapshot.attachments().to_vec(),
            vec![snapshot],
        );
        let snapshot = ws.snapshots().last().unwrap();
        let empty: HashMap<String, String> = HashMap::new();
        let no_evidence: HashMap<String, (String, String)> = HashMap::new();
        let members: Vec<(ArtifactId, ResourceRole)> = ws
            .attachments()
            .iter()
            .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
            .collect();
        let locators: HashMap<String, Locator> = HashMap::new();

        // (a) No derived outcome at all → no plan, Continue off.
        let canon_none = Canonical {
            workspace: &ws,
            outcome: None,
            selection: None,
            preflight: &[],
            subjects: &empty,
            kinds: &empty,
            titles: &empty,
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert!(plan_line(&canon_none).is_none());
        assert!(!continue_ready(&canon_none));

        // (b) Two candidates, no surface, no designation → Insufficient, no
        // Resume Point, and nothing to announce (IS-0021 §25.2/§25.8).
        let input = RestorationInput::new(&ws, snapshot).unwrap();
        let outcome = derive_restoration_plan(&input);
        assert!(outcome.resume_point().is_none());
        let selection = evo_execution::select_restoration(&outcome, &members, &no_evidence);
        let canon_undeclared = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: Some(&selection),
            preflight: &[],
            subjects: &empty,
            kinds: &empty,
            titles: &empty,
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert!(plan_line(&canon_undeclared).is_none());
        assert!(!continue_ready(&canon_undeclared));

        // (c) A declared surface whose members are all a commit (no
        // executable target) → restore-worthy is empty, so nothing is
        // announced and Continue stays off.
        let surface = vec![ArtifactId::new("artifact-aa").unwrap()];
        let input =
            RestorationInput::new_with_continuation_surface(&ws, snapshot, None, Some(surface))
                .unwrap();
        let outcome = derive_restoration_plan(&input);
        let mut commit_evidence = HashMap::new();
        commit_evidence.insert(
            "artifact-aa".to_string(),
            (
                "OBS-COMMIT-MADE".to_string(),
                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
            ),
        );
        let selection = evo_execution::select_restoration(&outcome, &members, &commit_evidence);
        assert!(selection.restore_worthy().is_empty());
        let canon_commit = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: Some(&selection),
            preflight: &[],
            subjects: &empty,
            kinds: &empty,
            titles: &empty,
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert!(plan_line(&canon_commit).is_none());
        assert!(!continue_ready(&canon_commit));
    }

    #[test]
    fn the_status_vocabulary_comes_from_the_canonical_source() {
        // Not re-typed here: if the canonical vocabulary ever changes, the UI
        // follows it instead of quietly disagreeing with it.
        assert_eq!(
            preflight_word(Some(&PreflightStatus::Ready), false).0,
            PreflightStatus::Ready.label()
        );
        for status in [
            PreflightStatus::Unavailable {
                reason: "gone".to_string(),
            },
            PreflightStatus::Ambiguous {
                reason: "two matches".to_string(),
            },
            PreflightStatus::Unsupported {
                reason: "no handler".to_string(),
            },
        ] {
            assert_eq!(preflight_word(Some(&status), false).0, status.label());
            assert_ne!(preflight_word(Some(&status), false).0, "READY");
        }
    }

    #[test]
    fn a_missing_preflight_entry_is_never_reported_as_reopenable() {
        // Absence of evidence is reported as absence, not as readiness.
        assert_eq!(preflight_word(None, false).0, "NOT CHECKED");
        // Unless restoration itself already placed it out of reach.
        assert_eq!(preflight_word(None, true).0, "UNAVAILABLE");
        assert_ne!(preflight_word(None, false).0, "READY");
        assert_ne!(preflight_word(None, true).0, "READY");
    }

    #[test]
    fn an_unsupported_resource_borrows_neither_success_nor_failure() {
        let unsupported = PreflightStatus::Unsupported {
            reason: "no handler".to_string(),
        };
        let (_, severity) = preflight_word(Some(&unsupported), false);
        assert_eq!(severity, Severity::Neutral);
        assert_ne!(severity, Severity::Good);
        assert_ne!(severity, Severity::Bad);
    }

    #[test]
    fn a_status_word_exists_for_every_preflight_outcome() {
        // Every branch produces a word: the UI can never fall back to a bare
        // color, which is what IS-0019 and RFC-0006 R-2 forbid.
        for status in [
            None,
            Some(PreflightStatus::Ready),
            Some(PreflightStatus::Unavailable {
                reason: String::new(),
            }),
            Some(PreflightStatus::Ambiguous {
                reason: String::new(),
            }),
            Some(PreflightStatus::Unsupported {
                reason: String::new(),
            }),
        ] {
            let (word, _) = preflight_word(status.as_ref(), false);
            assert!(!word.is_empty());
        }
    }
}

// ────────────────────────────────────────────────────────────────────────
// Headless stage pins: the proven rack harness, aimed at the stage.
// The synthetic move/press/release travels the same egui input pipeline a
// desktop pointer event takes, so these pin the panel-cards' hit-testing,
// one-open-at-a-time state, click-elsewhere disarm, and the dock's Continue
// dispatch exactly as the shell runs them.
// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod stage_pins {
    use super::*;

    /// One single-resource workspace whose plan announces one target:
    /// enough for the Overview panel to carry a plan and for the dock's
    /// Continue to be enabled.
    fn fixture() -> (
        Workspace,
        DerivationOutcome,
        RestorationSelection,
        HashMap<String, String>,
        HashMap<String, Locator>,
    ) {
        use evo_artifact::artifact_id::ArtifactId;
        use evo_restoration::{RestorationInput, derive_restoration_plan};
        use evo_workspace::attachment::{Attachment, ResourceRole};
        use evo_workspace::confidence::ConfidenceScore;
        use evo_workspace::lifecycle::WorkspaceLifecycle;
        use evo_workspace::snapshot::Snapshot;
        use evo_workspace::workspace_id::WorkspaceId;
        use std::str::FromStr;

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174954").unwrap();
        let attachment = Attachment::new(
            ArtifactId::new("artifact-aa").unwrap(),
            ConfidenceScore::new(1.0).unwrap(),
            ResourceRole::Primary,
        );
        let snapshot = Snapshot::new(
            std::time::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![attachment.clone()],
        );
        let ws = Workspace::new(
            workspace_id,
            WorkspaceLifecycle::Active,
            vec![attachment],
            vec![snapshot],
        );
        let input = RestorationInput::new(&ws, ws.snapshots().last().unwrap()).unwrap();
        let outcome = derive_restoration_plan(&input);
        let members: Vec<(evo_artifact::artifact_id::ArtifactId, ResourceRole)> = ws
            .attachments()
            .iter()
            .map(|a| (a.artifact_id().clone(), a.role()))
            .collect();
        let evidence: HashMap<String, (String, String)> = HashMap::from([(
            "artifact-aa".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/aa.md".to_string()),
        )]);
        let selection = evo_execution::select_restoration(&outcome, &members, &evidence);
        let mut subjects = HashMap::new();
        subjects.insert("artifact-aa".to_string(), "Sprint notes".to_string());
        let mut locators = HashMap::new();
        locators.insert(
            "artifact-aa".to_string(),
            Locator::new(
                ArtifactId::new("artifact-aa").unwrap(),
                LocatorKind::FilePath("/repo/aa.md".into()),
            ),
        );
        (ws, outcome, selection, subjects, locators)
    }

    /// The whole screen's state, owned the way the shell owns it: the
    /// canonical half read-only, the transient half mutable.
    struct Stage {
        workspace: Workspace,
        outcome: DerivationOutcome,
        selection: RestorationSelection,
        subjects: HashMap<String, String>,
        kinds: HashMap<String, String>,
        titles: HashMap<String, String>,
        standings: HashMap<String, Standing>,
        locators: HashMap<String, Locator>,
        selected: Option<WorkspaceId>,
        designation_status: Option<(String, bool)>,
        continuation_draft: Option<BTreeSet<String>>,
        continuation_status: Option<(String, bool)>,
        grouping_status: Option<(String, bool)>,
        execution_result: Option<ExecutionReport>,
        continue_phase: ContinuePhase,
        storage: TempDir,
        /// The frame clock, advanced 1/60s per rendered frame so the
        /// `settle()` animations progress deterministically headless.
        clock: f64,
    }

    impl Stage {
        fn new() -> Self {
            let (workspace, outcome, selection, subjects, locators) = fixture();
            Self {
                workspace,
                outcome,
                selection,
                subjects,
                kinds: HashMap::new(),
                titles: HashMap::new(),
                standings: HashMap::new(),
                locators,
                selected: None,
                designation_status: None,
                continuation_draft: None,
                continuation_status: None,
                grouping_status: None,
                execution_result: None,
                continue_phase: ContinuePhase::default(),
                storage: TempDir::default(),
                clock: 0.0,
            }
        }
    }

    // A stand-in storage root: the designation write path is never exercised
    // by these pins, so the path only needs to exist and be cleaned up.
    struct TempDir(std::path::PathBuf);
    impl Default for TempDir {
        fn default() -> Self {
            let base = std::env::temp_dir().join(format!(
                "evo-stage-pins-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&base).unwrap();
            Self(base)
        }
    }
    impl TempDir {
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn raw_input(time: f64, events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        }
    }

    /// Renders one frame of the stage. The canonical and transient halves are
    /// split by destructuring — the same disjoint borrow the shell makes, so
    /// the screen can read the record while writing the user's in-flight state.
    fn frame(ctx: &egui::Context, stage: &mut Stage, events: Vec<egui::Event>) {
        frame_in(ctx, stage, 900.0, 700.0, events)
    }

    fn frame_in(ctx: &egui::Context, stage: &mut Stage, w: f32, h: f32, events: Vec<egui::Event>) {
        let now = stage.clock;
        stage.clock += 1.0 / 60.0;
        let Stage {
            workspace,
            outcome,
            selection,
            subjects,
            kinds,
            titles,
            standings,
            locators,
            selected,
            designation_status,
            continuation_draft,
            continuation_status,
            grouping_status,
            execution_result,
            continue_phase,
            storage,
            clock: _,
        } = stage;
        let canon = Canonical {
            workspace,
            outcome: Some(outcome),
            selection: Some(selection),
            preflight: &[],
            subjects,
            kinds,
            titles,
            standings,
            locators,
            designation: None,
        };
        let mut transient = Transient {
            storage_root: storage.path(),
            selected,
            designation_status,
            continuation_draft,
            continuation_status,
            grouping_status,
            execution_result,
            continue_phase,
        };
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(w, h),
            )),
            time: Some(now),
            events,
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| {
            screen(ui, &canon, &[], &mut transient);
        });
    }

    /// Reads a rect the stage stashed for the harness, if any.
    fn stashed_rect(ctx: &egui::Context, key: egui::Id) -> Option<egui::Rect> {
        ctx.data_mut(|data| data.get_temp::<egui::Rect>(key))
    }

    fn press_release_events(at: egui::Pos2) -> [Vec<egui::Event>; 3] {
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

    /// Where the pointer lands to work a panel's strip: the strip's own
    /// vertical center, read back from the laid-out panel rect.
    fn strip_center(ctx: &egui::Context, index: usize) -> egui::Pos2 {
        let rect = stashed_rect(ctx, stage_panel_widget_id(index).with("evo-test-rect"))
            .unwrap_or_else(|| panic!("panel {index} did not report its rect"));
        egui::pos2(
            rect.center().x,
            rect.top() + theme::PANEL_STRIP_HEIGHT * 0.5,
        )
    }

    fn stored_open(ctx: &egui::Context, index: usize) -> Option<bool> {
        ctx.data_mut(|data| data.get_temp(stage_panel_widget_id(index).with("evo-test-open")))
    }

    #[test]
    fn a_pointer_click_on_a_panel_strip_expands_it() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut stage = Stage::new();

        frame(&ctx, &mut stage, vec![]);
        // The strip reports itself as a labeled widget, like a rack card —
        // the accessible-label pin rides the same read.
        assert!(ctx.read_response(stage_panel_widget_id(0)).is_some());

        let at = strip_center(&ctx, 0);
        let [move_ev, down_ev, up_ev] = press_release_events(at);
        frame(&ctx, &mut stage, move_ev);
        frame(&ctx, &mut stage, down_ev);
        assert_eq!(
            stored_open(&ctx, 0),
            Some(false),
            "still collapsed on pointer-down: the toggle happens on release"
        );
        frame(&ctx, &mut stage, up_ev);

        assert_eq!(
            stored_open(&ctx, 0),
            Some(true),
            "the strip click opened the panel"
        );
        // Let the growth animation run to completion before reading the
        // rect: the click frame itself is still at height ≈ strip.
        for _ in 0..30 {
            frame(&ctx, &mut stage, vec![]);
        }
        let rect = stashed_rect(&ctx, stage_panel_widget_id(0).with("evo-test-rect")).unwrap();
        assert!(
            rect.height() > theme::PANEL_STRIP_HEIGHT + 40.0,
            "the panel grew upward past its strip, got {rect:?}"
        );
    }

    #[test]
    fn a_second_strip_click_collapses_and_only_one_panel_is_ever_open() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut stage = Stage::new();

        frame(&ctx, &mut stage, vec![]);
        // Open panel 0.
        let [m, d, u] = press_release_events(strip_center(&ctx, 0));
        frame(&ctx, &mut stage, m);
        frame(&ctx, &mut stage, d);
        frame(&ctx, &mut stage, u);
        assert_eq!(stored_open(&ctx, 0), Some(true));

        // Opening panel 1 closes panel 0: one stored index, never two tall.
        let [m, d, u] = press_release_events(strip_center(&ctx, 1));
        frame(&ctx, &mut stage, m);
        frame(&ctx, &mut stage, d);
        frame(&ctx, &mut stage, u);
        assert_eq!(stored_open(&ctx, 0), Some(false), "panel 0 was closed");
        assert_eq!(
            stored_open(&ctx, 1),
            Some(true),
            "panel 1 is the one open panel"
        );

        // A second click on the open strip collapses it again.
        let [m, d, u] = press_release_events(strip_center(&ctx, 1));
        frame(&ctx, &mut stage, m);
        frame(&ctx, &mut stage, d);
        frame(&ctx, &mut stage, u);
        assert_eq!(
            stored_open(&ctx, 1),
            Some(false),
            "the second click collapsed the panel"
        );
    }

    #[test]
    fn a_click_outside_the_panels_collapses_the_open_one() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut stage = Stage::new();

        frame(&ctx, &mut stage, vec![]);
        let [m, d, u] = press_release_events(strip_center(&ctx, 2));
        frame(&ctx, &mut stage, m);
        frame(&ctx, &mut stage, d);
        frame(&ctx, &mut stage, u);
        assert_eq!(stored_open(&ctx, 2), Some(true));

        // A click in the page margin — left of every card, whose full-height
        // rects own the whole slot — disarms the panel, the same rule the
        // merge two-step proved on the rack.
        let [m, d, u] = press_release_events(egui::pos2(16.0, 400.0));
        frame(&ctx, &mut stage, m);
        frame(&ctx, &mut stage, d);
        frame(&ctx, &mut stage, u);
        assert_eq!(
            stored_open(&ctx, 2),
            Some(false),
            "click-elsewhere collapsed the panel"
        );
    }

    #[test]
    fn the_dock_dispatches_continue_from_the_play_position() {
        use crate::dock::{self, DockAction, DockContext};

        let ctx = egui::Context::default();
        theme::install(&ctx);
        let rect = egui::Rect::from_min_max(
            egui::pos2(0.0, 700.0 - theme::ACTION_BAR_HEIGHT),
            egui::pos2(900.0, 700.0),
        );

        fn run(
            ctx: &egui::Context,
            rect: egui::Rect,
            continuable: bool,
            events: Vec<egui::Event>,
        ) -> Option<DockAction> {
            let phase = ContinuePhase::default();
            let report: Option<ExecutionReport> = None;
            let mut action = None;
            let _ = ctx.run_ui(raw_input(0.0, events), |ui| {
                action = dock::show(
                    ui,
                    rect,
                    &DockContext {
                        active_pod: None,
                        in_focus: true,
                        continuable,
                    },
                    &phase,
                    report.as_ref(),
                );
            });
            action
        }

        run(&ctx, rect, true, vec![]);
        // Aim at the real widget: the transport primary's laid-out rect,
        // stashed by the dock and read back from egui — the same
        // aim-by-widget discipline the rack pins use.
        let primary_at = stashed_rect(&ctx, egui::Id::new("evo-test-transport"))
            .unwrap_or_else(|| panic!("the transport primary did not report its rect"))
            .center();
        let [m, d, u] = press_release_events(primary_at);
        assert_eq!(run(&ctx, rect, true, m), None);
        assert_eq!(run(&ctx, rect, true, d), None, "no dispatch before release");
        assert_eq!(run(&ctx, rect, true, u), Some(DockAction::Continue));

        // The gate: a disabled Continue reports hover sense only, so the
        // same press-release sequence dispatches nothing — the honest
        // disabled state, pinned.
        let [m, d, u] = press_release_events(primary_at);
        run(&ctx, rect, false, m);
        run(&ctx, rect, false, d);
        assert_eq!(
            run(&ctx, rect, false, u),
            None,
            "a disabled Continue cannot be clicked"
        );
    }

    #[test]
    fn the_executing_dock_shows_progress_and_dispatches_nothing() {
        use crate::dock::{self, DockContext};

        let ctx = egui::Context::default();
        theme::install(&ctx);
        let rect = egui::Rect::from_min_max(
            egui::pos2(0.0, 700.0 - theme::ACTION_BAR_HEIGHT),
            egui::pos2(900.0, 700.0),
        );

        // A two-step restoration one step in: the pill and the teal
        // hairline fraction are showing, and no amount of clicking the
        // dock's transport dispatches anything — the lock is the absence
        // of the affordance.
        use evo_artifact::artifact_id::ArtifactId;
        use evo_execution::StepPlan;
        let steps = vec![
            RestorationStep::new(
                ArtifactId::new("artifact-aa").unwrap(),
                StepPlan::Attempt(LocatorKind::FilePath("/repo/aa.md".into())),
            ),
            RestorationStep::new(
                ArtifactId::new("artifact-bb").unwrap(),
                StepPlan::Attempt(LocatorKind::FilePath("/repo/bb.md".into())),
            ),
        ];
        let done = vec![evo_execution::TargetAttempt::new(
            ArtifactId::new("artifact-aa").unwrap(),
            evo_execution::TargetStatus::Opened {
                detail: "/repo/aa.md".to_string(),
            },
        )];
        let phase = ContinuePhase::Executing { steps, done };
        let mut action = None;
        let _ = ctx.run_ui(raw_input(0.0, vec![]), |ui| {
            action = dock::show(
                ui,
                rect,
                &DockContext {
                    active_pod: None,
                    in_focus: true,
                    continuable: true,
                },
                &phase,
                None,
            );
        });
        assert_eq!(action, None, "executing dock offers no action");

        // The transport spot (where the idle primary sits) dispatches
        // nothing while a phase is in flight: press-release on the pill.
        let pill_at = ctx
            .data_mut(|data| data.get_temp::<egui::Rect>(egui::Id::new("evo-test-dock-rect")))
            .map(|r| egui::pos2(r.right() - 60.0, r.center().y))
            .unwrap();
        let [m, d, u] = press_release_events(pill_at);
        for events in [m, d, u] {
            let mut action = None;
            let _ = ctx.run_ui(raw_input(0.0, events), |ui| {
                action = dock::show(
                    ui,
                    rect,
                    &DockContext {
                        active_pod: None,
                        in_focus: true,
                        continuable: true,
                    },
                    &phase,
                    None,
                );
            });
            assert_eq!(action, None, "no dispatch while executing");
        }
    }

    #[test]
    fn record_tabs_swap_the_shown_section_and_persist_across_frames() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut stage = Stage::new();

        frame(&ctx, &mut stage, vec![]);
        let tab_key = egui::Id::new(("evo-test-record-tab", stage.workspace.id().to_string()));
        let stored =
            |ctx: &egui::Context| -> Option<usize> { ctx.data_mut(|data| data.get_temp(tab_key)) };
        assert_eq!(stored(&ctx), Some(0), "Surface is the resting tab");

        // The tab strip lives inside the Record panel, which only renders
        // its body once expanded — open it and let the growth finish first.
        let [m, d, u] = press_release_events(strip_center(&ctx, 2));
        frame(&ctx, &mut stage, m);
        frame(&ctx, &mut stage, d);
        frame(&ctx, &mut stage, u);
        for _ in 0..30 {
            frame(&ctx, &mut stage, vec![]);
        }

        // A pointer press/release on the third tab (Witnessed) swaps the
        // shown section — the tab strip is a real widget, not a label.
        let response = ctx
            .read_response(egui::Id::new((
                "evo-record-tab",
                2usize,
                stage.workspace.id().to_string(),
            )))
            .unwrap_or_else(|| panic!("the Witnessed tab did not report its widget"));
        let [m, d, u] = press_release_events(response.rect.center());
        frame(&ctx, &mut stage, m);
        frame(&ctx, &mut stage, d);
        frame(&ctx, &mut stage, u);
        assert_eq!(
            stored(&ctx),
            Some(2),
            "the click moved the record tab to Witnessed"
        );

        // The choice survives frames (temp data), and switching back works.
        frame(&ctx, &mut stage, vec![]);
        assert_eq!(
            stored(&ctx),
            Some(2),
            "the tab choice persists across frames"
        );
        let response = ctx
            .read_response(egui::Id::new((
                "evo-record-tab",
                0usize,
                stage.workspace.id().to_string(),
            )))
            .unwrap_or_else(|| panic!("the Surface tab did not report its widget"));
        let [m, d, u] = press_release_events(response.rect.center());
        frame(&ctx, &mut stage, m);
        frame(&ctx, &mut stage, d);
        frame(&ctx, &mut stage, u);
        assert_eq!(stored(&ctx), Some(0), "and switching back to Surface works");
    }

    #[test]
    fn an_expanded_panel_grows_past_the_old_320_cap_when_the_slot_allows() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut stage = Stage::new();

        // A tall window: the stage slot must offer more than the old cap
        // before the cap can be pinned from above and below.
        frame_in(&ctx, &mut stage, 900.0, 980.0, vec![]);
        let at = {
            let rect = stashed_rect(&ctx, stage_panel_widget_id(0).with("evo-test-rect")).unwrap();
            egui::pos2(
                rect.center().x,
                rect.top() + theme::PANEL_STRIP_HEIGHT * 0.5,
            )
        };
        let [m, d, u] = press_release_events(at);
        frame_in(&ctx, &mut stage, 900.0, 980.0, m);
        frame_in(&ctx, &mut stage, 900.0, 980.0, d);
        frame_in(&ctx, &mut stage, 900.0, 980.0, u);
        // Let the growth animation run out on the staged clock.
        for _ in 0..40 {
            frame_in(&ctx, &mut stage, 900.0, 980.0, vec![]);
        }
        let rect = stashed_rect(&ctx, stage_panel_widget_id(0).with("evo-test-rect")).unwrap();
        // The old cap was 320; the new column must exceed it on a window
        // whose slot allows it, and never exceed the declared 480.
        assert!(
            rect.height() > 400.0,
            "the column reads as a full-height panel, got {rect:?}"
        );
        assert!(
            rect.height() <= theme::PANEL_MAX_HEIGHT,
            "but never beyond the declared cap, got {rect:?}"
        );
    }

    #[test]
    fn the_arrival_stagger_leaves_every_panel_fully_settled_and_clickable() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut stage = Stage::new();

        // Run the stage past the longest stagger (0.08s) plus one response.
        for _ in 0..45 {
            frame(&ctx, &mut stage, vec![]);
        }
        // Every strip answers a click after the choreography completes —
        // arrival animation never gates interaction.
        for index in 0..3usize {
            let [m, d, u] = press_release_events(strip_center(&ctx, index));
            frame(&ctx, &mut stage, m);
            frame(&ctx, &mut stage, d);
            frame(&ctx, &mut stage, u);
            assert_eq!(
                stored_open(&ctx, index),
                Some(true),
                "panel {index} opens after the arrival choreography"
            );
            // Close it again for the next iteration.
            let [m, d, u] = press_release_events(strip_center(&ctx, index));
            frame(&ctx, &mut stage, m);
            frame(&ctx, &mut stage, d);
            frame(&ctx, &mut stage, u);
        }
    }
}
