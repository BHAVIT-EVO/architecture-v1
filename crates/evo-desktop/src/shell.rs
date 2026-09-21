//! The shell around every screen: chrome, system states, and the two gates.
//!
//! Everything here is deliberately quieter than the work it surrounds. Evo's
//! subject is the user's work, not Evo — so the wordmark is small, the capture
//! state is one word, Settings is a text control rather than an icon, and the
//! header carries no fill, no border and no shadow. It sits *in* the
//! environment. The first surface that lifts off the background is the one
//! holding the answer.
//!
//! System messages take a shape content never takes: a tinted band with a
//! leading edge bar. That is what makes "Evo cannot see your work" unmistakably
//! a statement by Evo about itself, rather than something it found.
//!
//! Every word about capture comes from `DaemonStatus` — the worker's own
//! report. The shell never infers that capture is running, and never softens
//! a failure into a warning.

use crate::daemon::{DaemonManager, DaemonStatus};
use crate::theme::{self, Role, Severity};
use crate::ui::{self, Control};

use std::path::{Path, PathBuf};

/// The header band: who we are, what Evo is currently able to see, and the way
/// into Settings. Returns true if Settings was asked for.
///
/// No fill and no divider. A bar drawn under a header is a habit borrowed from
/// web layout; here the space below it does the separating, and the raised
/// surface's own edge finishes the job. The band owns its rect so the row sits
/// optically centred in it rather than pinned to the window's top edge.
pub fn header(ui: &mut egui::Ui, rect: egui::Rect, daemon_status: &DaemonStatus) -> bool {
    let mut open_settings = false;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui::gap(ui, (rect.height() - theme::CONTROL_HEIGHT) * 0.5);
            ui.horizontal(|ui| {
                ui.add_space(theme::PAGE_PAD);
                ui::label(ui, Role::Title, "Evo", theme::ACCENT);
                ui.add_space(theme::S4);
                let (word, severity) = capture_status(daemon_status);
                // The word is the indicator. A colored dot on its own would
                // ask the user to learn a legend, and would say nothing at all
                // in monochrome.
                ui::status_word(ui, word, severity);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(theme::PAGE_PAD);
                    if ui::control(ui, Control::quiet("Settings")).clicked() {
                        open_settings = true;
                    }
                });
            });
        },
    );
    open_settings
}

/// What the capture worker reports, as a word and a severity.
///
/// Both come from the worker's own state. `Unavailable` is Neutral rather than
/// an error: Evo not having a capability is not the same as Evo failing.
pub fn capture_status(daemon_status: &DaemonStatus) -> (&'static str, Severity) {
    match daemon_status {
        DaemonStatus::Capturing => ("Watching", Severity::Good),
        DaemonStatus::PartialCapture { .. } => ("Partial", Severity::Caution),
        DaemonStatus::PermissionRequired => ("Blocked", Severity::Caution),
        DaemonStatus::Unavailable => ("Not running", Severity::Neutral),
        DaemonStatus::Failed(_) => ("Stopped", Severity::Bad),
    }
}

/// The system notice under the header, when Evo has something to say about
/// its own ability to witness work. Silent when capture is simply running —
/// a healthy state needs no band.
pub fn capture_notice(ui: &mut egui::Ui, daemon_status: &DaemonStatus, daemon: &mut DaemonManager) {
    match daemon_status {
        DaemonStatus::Capturing => {}
        DaemonStatus::PermissionRequired => {
            ui::notice_with(
                ui,
                Severity::Caution,
                "Blocked",
                "Evo witnesses focused windows through macOS Accessibility. \
                 Until that permission is granted Evo cannot observe your work \
                 — and it will not pretend otherwise.",
                |ui| {
                    ui::gap(ui, theme::S3);
                    permission_actions(ui, daemon);
                },
            );
            ui::gap(ui, theme::S4);
        }
        DaemonStatus::PartialCapture { detail } => {
            ui::notice_with(
                ui,
                Severity::Caution,
                "Partial",
                &format!(
                    "{detail}. Evo is still recording file saves; it will not \
                     pretend to witness the windows you focus."
                ),
                |ui| {
                    ui::gap(ui, theme::S3);
                    permission_actions(ui, daemon);
                },
            );
            ui::gap(ui, theme::S4);
        }
        DaemonStatus::Unavailable => {
            ui::notice(
                ui,
                Severity::Neutral,
                "Not running",
                "Evo could not start its capture worker, so nothing new is being \
                 recorded. What Evo already remembers is unaffected.",
            );
            ui::gap(ui, theme::S4);
        }
        DaemonStatus::Failed(detail) => {
            ui::notice_with(
                ui,
                Severity::Bad,
                "Stopped",
                &format!("Evo's capture worker stopped: {detail}"),
                |ui| {
                    ui::gap(ui, theme::S3);
                    permission_actions(ui, daemon);
                },
            );
            ui::gap(ui, theme::S4);
        }
    }
}

/// The two real permission mechanisms: hand the user to macOS, then let them
/// ask Evo to look again. Neither one fabricates a granted state.
pub fn permission_actions(ui: &mut egui::Ui, daemon: &mut DaemonManager) {
    let responses = ui::control_row(
        ui,
        vec![
            Control::secondary("Open System Settings"),
            Control::quiet("Check again"),
        ],
    );
    if responses[0].clicked() {
        crate::daemon::open_accessibility_settings();
    }
    if responses[1].clicked() {
        daemon.restart();
    }
}

/// Evo could not read its own record.
///
/// The most important thing here is that the user is not looking at their work
/// — so nothing on this screen imitates work. No list, no surface, no action
/// that would suggest continuing is possible.
pub fn storage_error(ui: &mut egui::Ui, status: &str) {
    ui::scroll(ui, |ui| {
        ui::gap(ui, theme::S6);
        ui::measure_column(ui, theme::PROSE_WIDTH, |ui| {
            ui::eyebrow(ui, "Your work");
            ui::display(ui, "Evo could not read its record");
            ui::gap(ui, theme::S2);
            ui::lede(
                ui,
                "Nothing is shown because nothing could be read. Evo will not \
                 show a partial or remembered list as if it were current.",
            );
            ui::notice(ui, Severity::Bad, "Unreadable", status);
        });
    });
}

// ────────────────────────────────────────────────────────────────────────
// First run
// ────────────────────────────────────────────────────────────────────────

/// The first-run gate: three short moments on the bare environment — what Evo
/// is, that it stays on this Mac, and the permission it needs.
///
/// System-setup restraint. No surface lifts off the background, because there
/// is no work to raise yet; the type carries the whole screen. Progress is
/// three small marks, and the only controls are the ones that move you
/// forward or grant the real permission.
pub fn first_run(
    ui: &mut egui::Ui,
    step: u8,
    pending: &mut Option<u8>,
    daemon: &mut DaemonManager,
) {
    let top = ui.max_rect().height() * 0.16;
    ui::page(ui, |ui| {
        ui::gap(ui, top);
        ui::measure_column(ui, 460.0, |ui| {
            progress(ui, step);
            ui::gap(ui, theme::S5);
            let (heading, explanation) = match step {
                1 => (
                    "Evo remembers where you were working",
                    "Evo keeps a private, local record of what you do, so you can \
                     pick up exactly where you left off.",
                ),
                2 => (
                    "It all stays on this Mac",
                    "Evo watches which files, windows, and pages you use — entirely \
                     on this Mac. Nothing is sent anywhere, and nothing needs the \
                     internet to work.",
                ),
                _ => (
                    "Give Evo the permission it needs",
                    "Evo needs Accessibility permission to witness the windows you \
                     focus. You can grant it now, or later from Settings.",
                ),
            };
            ui::display(ui, heading);
            ui::gap(ui, theme::S2);
            ui::lede(ui, explanation);
            ui::gap(ui, theme::S2);
            match step {
                1 | 2 => {
                    if ui::control(ui, Control::primary("Continue")).clicked() {
                        *pending = Some(step + 1);
                    }
                }
                _ => {
                    permission_actions(ui, daemon);
                    ui::gap(ui, theme::S5);
                    // Deliberately the only primary control on the step, and
                    // below the permission actions: entering Evo without the
                    // permission is allowed, but it is not the suggestion.
                    if ui::control(ui, Control::primary("Enter Evo")).clicked() {
                        complete_first_run();
                        *pending = None;
                    }
                    ui::gap(ui, theme::S3);
                    ui::provenance(
                        ui,
                        "Without this permission Evo records file saves only, and \
                         will say so.",
                    );
                }
            }
        });
    });
}

/// Three small marks. The passed moments stay visible but quiet, so the screen
/// says how far along you are without turning into a stepper component.
fn progress(ui: &mut egui::Ui, step: u8) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(48.0, 8.0), egui::Sense::hover());
    let painter = ui.painter();
    for index in 0..3u8 {
        let center = egui::pos2(rect.min.x + 4.0 + index as f32 * 16.0, rect.center().y);
        if index + 1 == step {
            painter.circle_filled(center, 3.5, theme::ACCENT);
        } else if index + 1 < step {
            painter.circle_filled(center, 2.5, theme::RECEDED);
        } else {
            painter.circle_stroke(center, 2.5, egui::Stroke::new(1.0, theme::border_strong()));
        }
    }
}

/// Whether the first-run gate is still pending. `Some(1)` starts it.
pub fn first_run_pending() -> Option<u8> {
    match first_run_marker() {
        Some(path) if path.exists() => None,
        _ => Some(1),
    }
}

/// The completion marker lives outside canonical storage — Evo never writes a
/// presentation fact into its evidence log. Losing the marker only re-shows
/// the gate, which is honest rather than harmful.
fn first_run_marker() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Evo")
            .join("first_run_done"),
    )
}

/// Writes the completion marker, best-effort.
fn complete_first_run() {
    if let Some(path) = first_run_marker() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, "done");
    }
}

// ────────────────────────────────────────────────────────────────────────
// Settings
// ────────────────────────────────────────────────────────────────────────

/// Settings: a small sheet over a dimmed window, holding three facts and no
/// invented options.
///
/// Deliberately not a settings dashboard with a sidebar. Evo has almost
/// nothing to configure, and pretending otherwise would imply the product is
/// somewhere other than where it actually is — Home, a body of work, Continue.
/// The sheet dims what is behind it because it is modal; a panel that did not
/// block would use translucency and offset instead.
///
/// Called on every frame, open or closed, so the sheet leaves along the path
/// it arrived by instead of vanishing — and so a second opening animates like
/// the first rather than snapping into place.
pub fn settings(
    ui: &mut egui::Ui,
    open: &mut bool,
    daemon_status: &DaemonStatus,
    daemon: &mut DaemonManager,
    storage_root: &Path,
) {
    let reveal = ui::transition(ui, ui.id().with("evo-settings"), *open);
    if !*open && reveal <= 0.002 {
        return;
    }
    let max = ui.max_rect();

    // The scrim pushes the window back rather than hiding it: the user has not
    // left where they were, and can see that they have not.
    ui.painter().rect_filled(
        max,
        0.0,
        egui::Color32::from_rgba_unmultiplied(0x14, 0x12, 0x10, (52.0 * reveal) as u8),
    );

    let needs_permission = !matches!(daemon_status, DaemonStatus::Capturing);
    let height = if needs_permission { 400.0 } else { 320.0 };
    let width = 460.0f32.min(max.width() - theme::PAGE_PAD * 2.0).max(280.0);
    // The sheet arrives by materializing at scale rather than sliding: it has
    // no source edge to come from, so a slide would imply a direction that
    // does not exist.
    let scale = 0.98 + 0.02 * reveal;
    let rect =
        egui::Rect::from_center_size(max.center(), egui::vec2(width * scale, height * scale));

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.set_opacity(reveal);
            theme::sheet_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui::label(ui, Role::Headline, "Settings", theme::TEXT);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui::control(ui, Control::quiet("Done")).clicked() {
                            *open = false;
                        }
                    });
                });
                ui::gap(ui, theme::S5);

                let (word, severity) = capture_status(daemon_status);
                ui::eyebrow(ui, "Capture");
                ui.horizontal(|ui| {
                    ui::status_word(ui, word, severity);
                    ui.add_space(theme::S3);
                    ui::label(
                        ui,
                        Role::Body,
                        capture_sentence(daemon_status),
                        theme::MUTED,
                    );
                });
                if needs_permission {
                    ui::gap(ui, theme::S3);
                    permission_actions(ui, daemon);
                }

                ui::gap(ui, theme::S5);
                ui::hairline(ui);
                ui::gap(ui, theme::S5);

                ui::field(ui, "Where your record is kept", "On this Mac only");
                ui::gap(ui, theme::S1);
                ui::one_line(
                    ui,
                    Role::Caption,
                    &storage_root.display().to_string(),
                    theme::TERTIARY,
                );

                ui::gap(ui, theme::S5);
                ui::provenance(
                    ui,
                    "All processing and storage stay on this Mac. Evo makes no \
                     network requests.",
                );
            });
        },
    );

    // Escape leaves, because a modal the keyboard cannot dismiss is a trap.
    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        *open = false;
    }
}

/// One plain sentence for each capture state, for the Settings sheet.
fn capture_sentence(daemon_status: &DaemonStatus) -> &'static str {
    match daemon_status {
        DaemonStatus::Capturing => "Evo is recording what it witnesses.",
        DaemonStatus::PartialCapture { .. } => "File saves only — window focus is not permitted.",
        DaemonStatus::PermissionRequired => "Evo cannot observe your work yet.",
        DaemonStatus::Unavailable => "The capture worker is not running.",
        DaemonStatus::Failed(_) => "The capture worker stopped.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_capture_state_has_a_word_and_a_sentence() {
        // A state with no word would leave the header silent about a condition
        // the user needs to know, and silence reads as "everything is fine".
        let states = [
            DaemonStatus::Capturing,
            DaemonStatus::PartialCapture {
                detail: "test".to_string(),
            },
            DaemonStatus::PermissionRequired,
            DaemonStatus::Unavailable,
            DaemonStatus::Failed("test".to_string()),
        ];
        for state in &states {
            let (word, _) = capture_status(state);
            assert!(!word.is_empty());
            assert!(!capture_sentence(state).is_empty());
        }
    }

    #[test]
    fn only_a_running_capture_reads_as_good() {
        // Any state other than Capturing must not borrow the color of success:
        // that is what would let the UI imply Evo is witnessing work it is not.
        assert_eq!(capture_status(&DaemonStatus::Capturing).1, Severity::Good);
        for state in [
            DaemonStatus::PermissionRequired,
            DaemonStatus::Unavailable,
            DaemonStatus::Failed("test".to_string()),
            DaemonStatus::PartialCapture {
                detail: "test".to_string(),
            },
        ] {
            assert_ne!(capture_status(&state).1, Severity::Good);
        }
    }

    #[test]
    fn a_missing_home_directory_shows_the_gate_rather_than_skipping_it() {
        // If the marker cannot be located, the honest default is to explain
        // Evo again — never to silently assume consent was already given.
        assert!(first_run_marker().is_some() || first_run_pending() == Some(1));
    }
}
