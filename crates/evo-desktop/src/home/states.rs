//! Home's honest states: the screens shown when the work list cannot be.
//!
//! `EXPERIENCE.md` §1's stage and §6's Home are drawn by the module above;
//! these are the surfaces for "nothing yet" and "Evo cannot see" — the two
//! cases where the product's promise has to be explained instead of
//! exercised.

use crate::theme;
use crate::ui::{self, Control};

// ─── Permission screens ──────────────────────────────────────────────────────

pub fn permission_required(ui: &mut egui::Ui, detail: Option<&str>, open_settings: impl FnOnce()) {
    ui::gap(ui, theme::S5);
    ui::raised(ui, |ui| {
        ui.set_width(ui.available_width());
        ui::eyebrow(ui, "Permission required");
        ui::display(ui, "Evo needs Accessibility to see your work");
        ui::gap(ui, theme::S2);
        ui::caption(
            ui,
            detail.unwrap_or(
                "Open System Settings → Privacy & Security → Accessibility. \
                 Find Evo in the list and turn it ON. Then click 'Check again' \
                 below (or restart Evo) — the dialog alone does not grant it.",
            ),
        );
        ui::gap(ui, theme::S3);
        if ui::control(ui, Control::secondary("Open System Settings")).clicked() {
            open_settings();
        }
    });
    ui::gap(ui, theme::S5);
}

pub fn empty(ui: &mut egui::Ui, capturing: bool) {
    ui::gap(ui, theme::S5);
    ui::raised(ui, |ui| {
        ui.set_width(ui.available_width());
        ui::eyebrow(ui, if capturing { "Watching" } else { "Waiting" });
        ui::display(
            ui,
            if capturing {
                crate::copy::HOME_EMPTY_LEARNING
            } else {
                crate::copy::HOME_EMPTY_WAITING
            },
        );
        ui::gap(ui, theme::S2);
        ui::caption(
            ui,
            if capturing {
                "Use your computer normally. Evo observes which applications \
                 and resources you engage with, and begins to recognize \
                 bodies of work after a few sessions."
            } else {
                "The capture worker is not running. Restart Evo to begin \
                 capturing your work activity."
            },
        );
    });
    ui::gap(ui, theme::S5);
}
