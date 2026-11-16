//! Home — "Pick up where you left off." (`EXPERIENCE.md` §6)
//!
//! One backdrop, one Continue card, and calm rows under it. The top bar
//! carries the wordmark, a quiet status, search and the way into Settings.
//! Everything a card can do lives in its "…" menu.
//!
//! The screen offers one obvious next action — Continue on the most recent
//! work — and never more than one filled control.

use crate::card;
use crate::rack::RackAction;
use crate::theme::{self, Role};
use crate::ui::{self, Control};

use evo_daemon::threads::DisplayThread;

/// How many rows are shown before "Show N more".
const VISIBLE_ROWS: usize = 6;

pub mod states;

// The honest empty and permission states keep the call sites they always
// had; only their home moved.
pub use states::{empty, permission_required};

/// What Home asked the shell to do.
#[derive(Default)]
pub struct HomeRequest {
    pub action: Option<RackAction>,
    pub open_settings: bool,
}

/// Draws Home into the full body rect. `listening` is the daemon's own
/// report that a voice surface is live — never inferred from this window.
pub fn home(
    ui: &mut egui::Ui,
    works: &[DisplayThread],
    query: &mut String,
    voice_state: &str,
    note: Option<&str>,
    footer: impl FnOnce(&mut egui::Ui),
) -> HomeRequest {
    let mut request = HomeRequest::default();
    let dusk = theme::is_dusk();

    top_bar(ui, query, voice_state, &mut request);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(theme::M6, theme::M5))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let column = theme::HOME_WIDTH.min(ui.available_width());
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.allocate_ui(egui::vec2(column, ui.available_height()), |ui| {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                // A correction Evo owes the person is stated
                                // inside the page, under the bar, not over the
                                // window's title strip.
                                if let Some(note) = note {
                                    ui.label(theme::text(Role::Caption, note, theme::muted_ink()));
                                    ui.add_space(theme::S3);
                                }
                                body(ui, works, query, &mut request, dusk, footer)
                            });
                        });
                    });
                });
        });

    request
}

/// Home while the work list is still being read: the same stage, the same
/// headline position, one honest line. Never the old eyebrow-and-sentence
/// loading screen, which spoke a different type system.
pub fn loading(ui: &mut egui::Ui) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(theme::M6, theme::M5))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.allocate_ui(
                egui::vec2(theme::HOME_WIDTH.min(ui.available_width()), 200.0),
                |ui| {
                    ui.label(theme::display(
                        crate::copy::HOME_TITLE,
                        theme::HEADLINE_SIZE,
                        theme::text_ink(),
                    ));
                    ui.add_space(theme::S3);
                    ui.label(theme::text(
                        Role::Lede,
                        "Reading your work record…",
                        theme::muted_ink(),
                    ));
                    ui.add_space(theme::S4);
                    ui.label(theme::text(
                        Role::Caption,
                        "No work is shown until that read completes, so the list is never a guess.",
                        theme::tertiary_ink(),
                    ));
                },
            );
        });
}

/// The top bar: wordmark left; status, search and gear right.
fn top_bar(ui: &mut egui::Ui, query: &mut String, voice_state: &str, request: &mut HomeRequest) {
    let height = theme::HEADER_HEIGHT;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.add_space((height - theme::CONTROL_HEIGHT) * 0.5);
            ui.horizontal(|ui| {
                // The window is full-bleed, so the traffic lights own the
                // first ~84 points; the wordmark starts after them rather
                // than under them.
                ui.add_space(88.0);
                ui.label(theme::display_strong("evo", 26.0, theme::text_ink()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(theme::S6);
                    if gear(ui) {
                        request.open_settings = true;
                    }
                    ui.add_space(theme::S4);
                    ui.scope(|ui| {
                        ui.set_width(220.0);
                        ui::search_field(ui, query, crate::copy::HOME_SEARCH, "evo-home-search");
                    });
                    ui.add_space(theme::S5);
                    status(ui, voice_state);
                });
            });
        },
    );
}

/// The quiet status: a small dot and two words. Never a badge, never caps.
fn status(ui: &mut egui::Ui, voice_state: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(160.0, 18.0), egui::Sense::hover());
    let active = voice_state == "Listening";
    let dot = if active {
        theme::accent()
    } else {
        theme::receded_ink()
    };
    ui.painter()
        .circle_filled(egui::pos2(rect.left() + 3.0, rect.center().y), 2.6, dot);
    ui.painter().text(
        egui::pos2(rect.left() + 13.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        voice_state,
        body_font(12.5),
        theme::muted_ink(),
    );
}

/// A drawn gear: one icon, painted, no icon font and no glyph roulette.
fn gear(ui: &mut egui::Ui) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
    let ink = if response.hovered() {
        theme::text_ink()
    } else {
        theme::muted_ink()
    };
    let centre = rect.center();
    let painter = ui.painter();
    for index in 0..8 {
        let angle = index as f32 * std::f32::consts::TAU / 8.0;
        let dir = egui::vec2(angle.cos(), angle.sin());
        painter.circle_filled(centre + dir * 8.2, 1.9, ink);
    }
    painter.circle_stroke(centre, 5.6, egui::Stroke::new(1.4, ink));
    painter.circle_filled(centre, 2.0, ink);
    response.clicked()
}

/// The body: headline, count line, Continue card, groups, hint, coming next.
fn body(
    ui: &mut egui::Ui,
    works: &[DisplayThread],
    query: &mut String,
    request: &mut HomeRequest,
    dusk: bool,
    footer: impl FnOnce(&mut egui::Ui),
) {
    ui.add_space(theme::S5);
    ui.label(theme::display(
        crate::copy::HOME_TITLE,
        theme::HEADLINE_SIZE,
        theme::text_ink(),
    ));
    ui.add_space(theme::S2);

    let presentable: Vec<(usize, &DisplayThread)> = works
        .iter()
        .enumerate()
        .filter(|(_, work)| work.is_work)
        .collect();

    let needle = query.trim().to_lowercase();
    let visible: Vec<(usize, &DisplayThread)> = if needle.is_empty() {
        presentable.clone()
    } else {
        presentable
            .iter()
            .copied()
            .filter(|(_, work)| {
                work.name.to_lowercase().contains(&needle)
                    || work
                        .understanding
                        .narrative
                        .to_lowercase()
                        .contains(&needle)
            })
            .collect()
    };

    let week = visible
        .iter()
        .filter(|(_, work)| card::this_week(work))
        .count();
    ui.label(theme::text(
        Role::Lede,
        count_line(week, visible.len(), !needle.is_empty()),
        theme::muted_ink(),
    ));
    ui.add_space(theme::S6);

    if visible.is_empty() {
        empty_home(ui, works.is_empty(), !needle.is_empty());
        ui.add_space(theme::S5);
    } else {
        // The Continue card.
        let (hero_index, hero) = visible[0];
        let _ = hero_index;
        if let Some(action) = card::hero(ui, hero, hero_index) {
            request.action = Some(action);
        }
        ui.add_space(theme::S6);

        let rows = &visible[1..];

        // "Show N more" state, remembered across frames.
        let more_id = egui::Id::new("evo-home-show-more");
        let expanded: bool = ui.data(|data| data.get_temp(more_id).unwrap_or(false));
        let mut shown = 0usize;

        shown += group(ui, "This week", rows, shown, dusk, expanded, request);
        let _ = group(ui, "Earlier", rows, shown, dusk, expanded, request);

        if rows.len() > VISIBLE_ROWS && !expanded {
            ui.add_space(theme::S3);
            let hidden = rows.len() - VISIBLE_ROWS;
            if ui::control(ui, Control::quiet(&format!("Show {hidden} more"))).clicked() {
                ui.data_mut(|data| data.insert_temp(more_id, true));
            }
        } else if expanded && rows.len() > VISIBLE_ROWS {
            ui.add_space(theme::S3);
            if ui::control(ui, Control::quiet("Show less")).clicked() {
                ui.data_mut(|data| data.insert_temp(more_id, false));
            }
        }
    }

    ui.add_space(theme::S6);
    footer(ui);
    ui.add_space(theme::S6);
}

/// One recency group. Returns how many rows it drew.
fn group(
    ui: &mut egui::Ui,
    label: &str,
    rows: &[(usize, &DisplayThread)],
    already_shown: usize,
    dusk: bool,
    expanded: bool,
    request: &mut HomeRequest,
) -> usize {
    let in_group: Vec<(usize, &DisplayThread)> = rows
        .iter()
        .copied()
        .filter(|(_, work)| (label == "This week") == card::this_week(work))
        .collect();
    if in_group.is_empty() {
        return 0;
    }
    let budget = if expanded {
        in_group.len()
    } else {
        VISIBLE_ROWS.saturating_sub(already_shown)
    };
    if budget == 0 {
        return 0;
    }
    ui.label(theme::text(Role::Caption, label, theme::tertiary_ink()));
    ui.add_space(theme::S2);
    let mut drawn = 0;
    for (index, work) in in_group.iter().take(budget) {
        if let Some(action) = card::row(ui, work, *index) {
            request.action = Some(action);
        }
        drawn += 1;
        ui.add_space(theme::S2);
    }
    let _ = dusk;
    ui.add_space(theme::S3);
    drawn
}

fn count_line(week: usize, total: usize, searching: bool) -> String {
    if searching {
        return format!("{total} {} match.", plural(total, "work"));
    }
    if total == 0 {
        return "Nothing here yet.".to_string();
    }
    if week == 0 {
        return format!("{total} {} from before this week.", plural(total, "thing"));
    }
    format!("{week} {} from this week.", plural(week, "thing"))
}

fn plural(count: usize, word: &str) -> String {
    if count == 1 {
        word.to_string()
    } else {
        format!("{word}s")
    }
}

fn empty_home(ui: &mut egui::Ui, nothing_at_all: bool, searching: bool) {
    ui.add_space(theme::S2);
    ui.label(theme::text(
        Role::Lede,
        if searching {
            "Nothing here matches that. Try the name you gave it."
        } else if nothing_at_all {
            "Evo has not found any work yet. Use your computer normally; Evo watches and will show what it recognizes."
        } else {
            "Nothing to continue right now."
        },
        theme::muted_ink(),
    ));
}

/// The body text face at a free size, used for the status line only.
fn body_font(size: f32) -> egui::FontId {
    theme::body_font(size)
}

/// The stage is painted by the shell before any screen; this helper exists
/// so a caller that renders Home standalone still gets the backdrop.
pub fn paint_stage(ui: &egui::Ui) {
    crate::stage::paint(ui.painter(), ui.max_rect(), theme::is_dusk());
}

/// Whether the daemon reports a live voice surface. Read from
/// `voice.status`: the bar's status line must be the agent's own report,
/// never this window's guess.
pub fn voice_listening(root: &std::path::Path) -> bool {
    voice_status(root) == "Listening"
}

/// Reads the agent-owned microphone state for Home and the tray.
pub fn voice_status(root: &std::path::Path) -> &'static str {
    match std::fs::read_to_string(root.join("voice.status"))
        .ok()
        .and_then(|status| match status.trim() {
            "listening" => Some("Listening"),
            "paused" => Some("Paused"),
            "off" => Some("Off"),
            _ => None,
        }) {
        Some(state) => state,
        None => "Off",
    }
}
