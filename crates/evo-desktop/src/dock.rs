//! The dock: Evo's persistent bottom control panel (DESIGN.md: The dock).
//!
//! One strip, always at the bottom of the window, carrying the single
//! primary action and its live progress. It is the ACTION material: the
//! most opaque surface in the app, separated from the content above it by a
//! hairline, because it is the one piece of chrome the user is meant to
//! find without looking.
//!
//! The dock reports pure intent ([`DockAction`]); the shell owns every
//! effect, the way a screen module does. It never mutates state itself.

use crate::detail::ContinuePhase;
use crate::theme::{self, Role};
use crate::ui::{self, Control};

use evo_execution::ExecutionReport;

/// What the dock asks the shell to do.
#[derive(Debug, PartialEq, Eq)]
pub enum DockAction {
    /// Return to the rack (Home).
    Library,
    /// Continue the work currently in focus.
    Continue,
    /// Eject the active pod, restoring the desktop as it was.
    Eject,
    /// Dismiss the last execution report.
    DismissReport,
}

/// What the dock needs to know about the world, gathered by the shell.
pub struct DockContext {
    /// The name of the pod currently entered, if any.
    pub active_pod: Option<String>,
    /// Whether a stage is in focus (shows the Library control).
    pub in_focus: bool,
    /// Whether the focused work's plan announces at least one target Evo
    /// will actually attempt — the only condition under which Continue is
    /// offered (UI-VALIDATION-0001 B.9: the gate follows the plan, the same
    /// shared ordering function execution consumes). A disabled Continue
    /// keeps its shape so the dock's structure never shifts.
    pub continuable: bool,
}

/// Renders the dock into `rect`. Returns the action the user asked for, if any.
///
/// The transport cluster sits at the *play position* — right-aligned, the
/// fixed right end of the strip — so Continue is always exactly where a
/// transport control belongs and the left of the dock stays chrome-only
/// (DESIGN.md adoption map 3). A phase in flight renders as a pill in the
/// informational teal sitting on the spot the primary occupies when idle,
/// so the progress and the button are one object in two states. The step
/// counter is data-voice mono; the small bar under it only confirms what
/// the number already says.
pub fn show(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    ctx: &DockContext,
    phase: &ContinuePhase,
    report: Option<&ExecutionReport>,
) -> Option<DockAction> {
    let mut action = None;
    let painter = ui.painter_at(rect);

    // The ACTION material: near-opaque charcoal over the content. The strip
    // is structural, so a one-pixel top rule and the lighter fill are the
    // lift; the upward shadow belongs to floating frames, not this one.
    painter.rect_filled(rect, 0.0, theme::ACTION_SURFACE);

    // The top hairline is also the progress line: while a restoration is
    // executing, the teal fraction fills *the hairline itself*, left to
    // right — the reference's thin progress line, mapped honestly onto the
    // one rule this strip already had. No separate bar, no new execution
    // code: the same numbers the pill's counter reads drive the line.
    painter.line_segment(
        [
            rect.left_top() + egui::vec2(0.0, 0.5),
            rect.right_top() + egui::vec2(0.0, 0.5),
        ],
        egui::Stroke::new(1.0, theme::border()),
    );
    if let ContinuePhase::Executing { steps, done } = phase {
        let fraction = (done.len() as f32 / steps.len().max(1) as f32).clamp(0.0, 1.0);
        if fraction > 0.0 {
            painter.line_segment(
                [
                    rect.left_top() + egui::vec2(0.0, 0.5),
                    rect.left_top() + egui::vec2(rect.width() * fraction, 0.5),
                ],
                egui::Stroke::new(1.5, theme::TEAL),
            );
        }
    }

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            // Optical vertical centering, the same trick the header uses.
            ui.add_space((rect.height() - theme::CONTROL_HEIGHT) * 0.5);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(theme::PAGE_PAD);

                // The entered pod, with its exits. Pod chrome uses the
                // one control language like everything else.
                if let Some(pod) = &ctx.active_pod {
                    ui.label("\u{25CF}");
                    ui.strong(format!("In: {pod}"));
                    ui.add_space(theme::S2);
                    if ui::control(ui, Control::secondary("Eject")).clicked() {
                        action = Some(DockAction::Eject);
                    }
                    ui.add_space(theme::S2);
                }

                // The way back to the rack, ahead of the transport
                // cluster but left of it.
                if ctx.in_focus {
                    ui.add_space(theme::S4);
                    if ui::control(ui, Control::secondary("Library")).clicked() {
                        action = Some(DockAction::Library);
                    }
                }

                // The transport cluster: Continue in the play position,
                // with the phase pill sitting on the same spot while a
                // phase is in flight. A disabled primary keeps its
                // shape; the system control gives it only hover sense,
                // so it cannot be clicked by accident or on purpose.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(theme::PAGE_PAD);
                    transport(ui, ctx, phase, report, &mut action);
                });
            });
        },
    );

    // The headless harness reads the laid-out dock rect back through egui's
    // own data store — the rack's proven seam.
    if cfg!(test) {
        ui.data_mut(|data| data.insert_temp(egui::Id::new("evo-test-dock-rect"), rect));
    }
    action
}

/// The transport cluster: the primary Continue, or the phase it is in.
///
/// One object in two states (the same grammar as the stage panel-cards):
/// idle, a filled primary labelled `▶ Continue`; in flight, a pill in the
/// informational teal — the phase word, then the step count in the data
/// voice — with the finished-report state rendered as its own quiet pill,
/// not a disguised success color (informational, never celebratory). No
/// branch here executes anything: the shell owns every effect, exactly as
/// before. One primary at a time is structural — the idle primary and the
/// in-flight pill are the same object's two states, never shown together.
fn transport(
    ui: &mut egui::Ui,
    ctx: &DockContext,
    phase: &ContinuePhase,
    report: Option<&ExecutionReport>,
    action: &mut Option<DockAction>,
) {
    match phase {
        ContinuePhase::Idle if report.is_none() => {
            let response = ui::control(
                ui,
                Control::primary("\u{25B6} Continue").enabled(ctx.continuable),
            );
            // The headless harness aims at the play position by its real
            // laid-out rect — the same seam the dock's own rect uses.
            if cfg!(test) {
                ui.data_mut(|data| {
                    data.insert_temp(egui::Id::new("evo-test-transport"), response.rect)
                });
            }
            if response.clicked() {
                *action = Some(DockAction::Continue);
            }
        }
        ContinuePhase::Idle => {
            // A finished report: say so plainly, let it go.
            phase_pill(ui, "Continue completed");
            ui.add_space(theme::S2);
            if ui::control(ui, Control::quiet("Dismiss")).clicked() {
                *action = Some(DockAction::DismissReport);
            }
        }
        ContinuePhase::Preparing { .. } => {
            phase_pill(ui, "Preparing");
        }
        ContinuePhase::Executing { steps, done } => {
            // The pill and the count state the progress; the teal fraction
            // on the dock's top hairline (painted above, in `show`) is the
            // line the reference reads — one object, not a separate bar.
            phase_pill(ui, "Restoring");
            ui.add_space(theme::S2);
            ui.label(theme::data(
                format!("{} / {} steps", done.len(), steps.len().max(1)),
                Role::Data.size(),
                theme::TEXT,
            ));
        }
    }
}

/// The phase pill: raised fill, teal eyebrow voice, hairline border.
/// Teal is Evo's informational voice — a phase in flight is a state, not an
/// achievement, so it borrows neither the success green nor the accent.
/// Sized from its own measured galley, so the word can never clip.
fn phase_pill(ui: &mut egui::Ui, word: &str) {
    let text = theme::eyebrow(word, theme::TEAL);
    let galley =
        ui.painter()
            .layout_no_wrap(text.text().to_owned(), Role::Eyebrow.font(), theme::TEAL);
    let size = galley.size() + egui::vec2(theme::S3 * 2.0, theme::S1 * 1.5);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    let corner = egui::CornerRadius::same(theme::CORNER_CONTROL);
    painter.rect_filled(rect, corner, theme::RAISED);
    painter.rect_stroke(
        rect,
        corner,
        egui::Stroke::new(1.0, theme::border()),
        egui::StrokeKind::Inside,
    );
    let origin = rect.center() - galley.size() * 0.5;
    painter.galley(origin, galley, theme::TEAL);
}
