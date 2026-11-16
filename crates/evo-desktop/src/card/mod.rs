//! The card: one body of work, as a surface that floats on the stage.
//!
//! `EXPERIENCE.md` §1: UI floats on the stage as cards, 24px radius, soft
//! long shadow; Day warm white, Dusk deep green; text never sits directly on
//! the art. §6 gives the card its two sizes: one hero card for the most
//! recent work, and calm 56px rows for the rest.
//!
//! Everything a card offers lives in its "…" menu, never in a stack of text
//! links under the title.

use crate::rack::RackAction;
use crate::stage;
use crate::theme::{self, Role};
use crate::ui::{self, Control};

use evo_daemon::threads::DisplayThread;

/// What the "…" menu asked for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuChoice {
    Rename,
    Restore,
    MergeWith,
    NotWork,
    Hide,
}

/// The hero card: the most recent work, the one thing Home is asking for.
///
/// Left: a tile that feels like the work. Right: the title in the UI sans at
/// 28px semibold, one plain sentence, the age, and exactly one primary
/// action.
pub fn hero(ui: &mut egui::Ui, work: &DisplayThread, index: usize) -> Option<RackAction> {
    let dusk = theme::is_dusk();
    let mut clicked_open = false;
    let mut continue_clicked = false;

    let frame = egui::Frame::new()
        .fill(theme::card_fill())
        .stroke(egui::Stroke::new(1.0, stage::rim(dusk)))
        .corner_radius(egui::CornerRadius::same(theme::CARD_RADIUS))
        .inner_margin(egui::Margin::same(theme::M6))
        .shadow(stage::card_shadow(dusk));

    let width = ui.available_width();
    // The card's background is registered before its children, so the pill
    // and the menu sit on top of it: a click on either is never also a
    // click on the card.
    let (card_rect, card_response) =
        ui.allocate_exact_size(egui::vec2(width, 188.0), egui::Sense::click());
    let mut card_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(card_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    {
        card_ui.set_width(width);
        frame.show(&mut card_ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                tile(ui, work, 76.0);
                ui.add_space(theme::S5);
                // A bounded text block: a vertical layout asked for inside a
                // horizontal one would otherwise take every remaining point of
                // height in the panel and drag the card's edge down with it.
                let text_width = (ui.available_width() - theme::S4).max(200.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(text_width, 122.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.horizontal(|ui| {
                            // Room for the "…" that is anchored to the card's
                            // right edge, so the title never runs under it.
                            let title_width = (ui.available_width() - 44.0).max(140.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(title_width, 40.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.label(theme::title(&work.name, 28.0, theme::text_ink()));
                                },
                            );
                        });
                        ui.add_space(theme::S2);
                        ui.label(theme::text(Role::Lede, one_line(work), theme::muted_ink()));
                        ui.add_space(theme::S4);
                        ui.horizontal(|ui| {
                            ui.label(theme::data(
                                age_stamp(work),
                                Role::Data.size(),
                                theme::tertiary_ink(),
                            ));
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), 34.0),
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui::control(ui, Control::primary("Continue").pill())
                                        .clicked()
                                    {
                                        continue_clicked = true;
                                    }
                                },
                            );
                        });
                    },
                );
            });
        });
    }

    // The whole card opens the work; the pill and the menu keep their own
    // hit targets.
    let rect = card_rect;
    // The "…" belongs to the card's right edge, not to the title row's
    // layout: it is placed from the card's own rect, so no title length can
    // push it inward.
    menu_button_at(ui, work, card_menu_rect(rect, theme::M6));
    if card_response.clicked() {
        clicked_open = true;
    }

    if let Some(choice) = menu_popup(ui, work, rect) {
        return Some(map_choice(work, index, choice));
    }
    if continue_clicked {
        return Some(RackAction::Continue(index));
    }
    clicked_open.then_some(RackAction::Open(index))
}

/// A calm row: one title line, one meta line, Continue on hover.
pub fn row(ui: &mut egui::Ui, work: &DisplayThread, index: usize) -> Option<RackAction> {
    let dusk = theme::is_dusk();
    let mut clicked_open = false;
    let mut continue_clicked = false;

    // The row's surface is painted on a fixed 56px rect rather than framed:
    // frame height follows its content, and content height is exactly what
    // a row must not negotiate.
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, theme::ROW_HEIGHT), egui::Sense::click());
    // Geometry, not `response.hovered()`: the row's own text labels sit on
    // top of its background, and they would take the hover from it.
    let hovered = ui.rect_contains_pointer(rect);
    paint_row_surface(ui.painter(), rect, dusk);

    let inner = rect.shrink2(egui::vec2(theme::S5, theme::S2));
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.spacing_mut().item_spacing.x = theme::S4;
    tile(&mut row, work, 32.0);
    // 32 tile + gap + Continue's slot + the gap the anchored "…" keeps.
    let text_width = (inner.width() - 32.0 - theme::S4 - 120.0 - 34.0).max(160.0);
    row.allocate_ui_with_layout(
        egui::vec2(text_width, 40.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.add(
                egui::Label::new(theme::text(Role::Title, &work.name, theme::text_ink()))
                    .truncate(),
            );
            ui.add(
                egui::Label::new(theme::text(
                    Role::Caption,
                    format!("{}  ·  {}", age_stamp(work), meta(work)),
                    theme::tertiary_ink(),
                ))
                .truncate(),
            );
        },
    );
    row.allocate_ui_with_layout(
        egui::vec2(120.0, 32.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            if hovered && ui::control(ui, Control::quiet("Continue")).clicked() {
                continue_clicked = true;
            }
        },
    );
    menu_button_at(ui, work, row_menu_rect(rect));
    if response.clicked() {
        clicked_open = true;
    }

    if let Some(choice) = menu_popup(ui, work, rect) {
        return Some(map_choice(work, index, choice));
    }
    if continue_clicked {
        return Some(RackAction::Continue(index));
    }
    clicked_open.then_some(RackAction::Open(index))
}

fn map_choice(work: &DisplayThread, index: usize, choice: MenuChoice) -> RackAction {
    match choice {
        MenuChoice::Rename => RackAction::Rename(work.thread_id),
        MenuChoice::Restore => RackAction::RestoreSelected(work.thread_id),
        MenuChoice::MergeWith => RackAction::Merge(index),
        MenuChoice::NotWork => RackAction::NotWork(work.thread_id),
        MenuChoice::Hide => RackAction::Dismiss(work.thread_id),
    }
}

fn menu_state_id(work: &DisplayThread) -> egui::Id {
    egui::Id::new(("evo-card-menu-open", work.thread_id))
}

fn just_opened_id() -> egui::Id {
    egui::Id::new("evo-card-menu-just-opened")
}

/// The "…" slot on a card: flush to its right inner edge, level with the
/// title's first line.
fn card_menu_rect(card: egui::Rect, pad: i8) -> egui::Rect {
    let pad = pad as f32;
    egui::Rect::from_min_size(
        egui::pos2(card.right() - pad - 26.0, card.top() + pad + 8.0),
        egui::vec2(26.0, 24.0),
    )
}

/// The "…" slot on a row: flush to its right inner edge, vertically centred.
fn row_menu_rect(row: egui::Rect) -> egui::Rect {
    egui::Rect::from_center_size(
        egui::pos2(row.right() - theme::S5 - 13.0, row.center().y),
        egui::vec2(26.0, 24.0),
    )
}

/// The row's surface: 24px radius and the same downward shadow the hero
/// carries, so a row reads as a card on the stage rather than a stripe.
fn paint_row_surface(painter: &egui::Painter, rect: egui::Rect, dusk: bool) {
    let shadow = stage::card_shadow(dusk);
    // A card's shadow is one soft shape; the renderer has no blur primitive,
    // so it is built from a few rounded layers that grow and fade.
    for step in (1..=6).rev() {
        let grow = step as f32 * 2.0;
        let alpha = (shadow.color.a() as f32 * 0.16 / step as f32) as u8;
        let colour = egui::Color32::from_rgba_unmultiplied(
            shadow.color.r(),
            shadow.color.g(),
            shadow.color.b(),
            alpha,
        );
        painter.rect_filled(
            rect.translate(egui::vec2(0.0, shadow.offset[1] as f32 * 0.45))
                .expand(grow),
            egui::CornerRadius::same(theme::CARD_RADIUS),
            colour,
        );
    }
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(theme::CARD_RADIUS),
        theme::card_row_fill(),
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(theme::CARD_RADIUS),
        egui::Stroke::new(1.0, stage::rim(dusk)),
        egui::StrokeKind::Inside,
    );
}

/// The "…" control: three dots, no chrome until touched.
fn menu_button_at(ui: &mut egui::Ui, work: &DisplayThread, rect: egui::Rect) {
    let id = egui::Id::new(("evo-card-menu-button", work.thread_id));
    let response = ui.interact(rect, id, egui::Sense::click());
    let ink = if response.hovered() {
        theme::text_ink()
    } else {
        theme::tertiary_ink()
    };
    for offset in [-6.0, 0.0, 6.0] {
        ui.painter().circle_filled(
            egui::pos2(rect.center().x + offset, rect.center().y),
            1.6,
            ink,
        );
    }
    if response.clicked() {
        let open: bool = ui.data(|data| data.get_temp(menu_state_id(work)).unwrap_or(false));
        ui.data_mut(|data| data.insert_temp(menu_state_id(work), !open));
        // The click that opens the menu is the same click the outside-click
        // test would see; remember it so the menu survives its own opening.
        ui.data_mut(|data| data.insert_temp(just_opened_id(), true));
        // No `request_focus` here: focusing a widget that the accessibility
        // tree does not carry makes accesskit panic on macOS ("Focused ID is
        // not in the node list"), which takes the window down.
        let _ = id;
    }
}

/// The menu popover, painted above the card when it is open.
fn menu_popup(ui: &mut egui::Ui, work: &DisplayThread, card: egui::Rect) -> Option<MenuChoice> {
    let open: bool = ui.data(|data| data.get_temp(menu_state_id(work)).unwrap_or(false));
    if !open {
        return None;
    }
    let mut chosen: Option<MenuChoice> = None;
    let mut close = false;

    let anchor = egui::pos2(card.right() - theme::S6 - 196.0, card.top() + 64.0);
    let area = egui::Area::new(egui::Id::new(("evo-card-menu", work.thread_id)))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor)
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(theme::card_fill())
                .stroke(egui::Stroke::new(1.0, theme::border()))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(theme::M2))
                .shadow(egui::Shadow {
                    offset: [0, 14],
                    blur: 36,
                    spread: 0,
                    color: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 96),
                })
                .show(ui, |ui| {
                    // A menu is as wide as its longest word, not as wide as
                    // the area it floats in (which is unbounded).
                    ui.set_width(180.0);
                    for (label, choice) in [
                        ("Rename…", MenuChoice::Rename),
                        ("Restore…", MenuChoice::Restore),
                        ("This isn't work", MenuChoice::NotWork),
                        ("Hide from Home", MenuChoice::Hide),
                        ("Merge with…", MenuChoice::MergeWith),
                    ] {
                        if menu_item(ui, label) {
                            chosen = Some(choice);
                        }
                    }
                });
        });

    if let Some(choice) = chosen {
        ui.data_mut(|data| data.insert_temp(menu_state_id(work), false));
        if choice == MenuChoice::MergeWith {
            // Opening the merge panel in place is the whole of this choice;
            // the panel itself, not the menu, decides what gets grouped.
            ui.data_mut(|data| data.insert_temp(egui::Id::new("evo-home-merge-open"), true));
        }
        return Some(choice);
    }

    // A click outside the menu closes it, exactly like a popover — except
    // the click that opened it, which arrives in this same frame.
    let just_opened: bool = ui.data(|data| data.get_temp(just_opened_id()).unwrap_or(false));
    if just_opened {
        ui.data_mut(|data| data.insert_temp(just_opened_id(), false));
    } else if ui.input(|input| input.pointer.any_click())
        && ui
            .input(|input| input.pointer.interact_pos())
            .is_some_and(|at| !area.response.rect.contains(at))
    {
        close = true;
    }
    if close {
        ui.data_mut(|data| data.insert_temp(menu_state_id(work), false));
    }
    None
}

fn menu_item(ui: &mut egui::Ui, label: &str) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(8), theme::hover_wash());
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        Role::Body.font(),
        if response.hovered() {
            theme::text_ink()
        } else {
            theme::muted_ink()
        },
    );
    response.clicked()
}

/// The tile: a coloured plate with the work's initial, until a real app icon
/// or thumbnail exists for the work. The colour comes from the work's own
/// identity, so the same work always looks the same.
fn tile(ui: &mut egui::Ui, work: &DisplayThread, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let dusk = theme::is_dusk();
    let radius = if size > 40.0 { 20 } else { 10 };
    let variant = hash_name(&work.name) % 3;
    let plate = match variant {
        0 => theme::accent_tint(),
        1 => theme::neutral_tint(),
        _ => theme::card_row_fill(),
    };
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(radius), plate);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(radius),
        egui::Stroke::new(
            1.0,
            if dusk {
                theme::border()
            } else {
                stage::rim(false)
            },
        ),
        egui::StrokeKind::Inside,
    );
    let initial = work
        .name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "·".to_string());
    let ink = if variant == 0 {
        theme::accent()
    } else {
        theme::muted_ink()
    };
    // UI sans, not the display face: §6 keeps the display font for the
    // headline and the wordmark only.
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        initial,
        theme::title_font(rect.height() * 0.40),
        ink,
    );
}

/// One plain sentence about the work, in the product's words.
///
/// Never the understanding's own narrative or any engine vocabulary: those
/// strings are written for the model ("Input without an observed save",
/// "observed activity"), and §3's plain-word rulings do not allow them on a
/// card. The sentence says only what is true of every work Evo keeps.
fn one_line(work: &DisplayThread) -> String {
    if work.has_open_loop {
        "Evo kept the trail here, so you can pick it up where you left off."
    } else {
        "Evo watched this work and kept what you need to carry on."
    }
    .to_string()
}

/// The one meta phrase a row carries: what is still open, in plain words.
fn meta(work: &DisplayThread) -> String {
    let open = work
        .understanding
        .pending
        .iter()
        .filter(|step| !step.trim().is_empty())
        .count();
    match open {
        0 if work.has_open_loop => "one step open".to_string(),
        0 => "nothing open".to_string(),
        1 => "one step open".to_string(),
        n => format!("{n} steps open"),
    }
}

/// The age in the card's own words: "5 days ago", not "120h".
pub fn age_stamp(work: &DisplayThread) -> String {
    let Some(last_active_ms) = work.last_active_ms else {
        return String::new();
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let secs = now_ms.saturating_sub(last_active_ms) / 1000;
    if secs < 90 {
        return "just now".to_string();
    }
    let minutes = secs / 60;
    if minutes < 60 {
        return format!("{minutes} minutes ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return match hours {
            1 => "an hour ago".to_string(),
            _ => format!("{hours} hours ago"),
        };
    }
    let days = hours / 24;
    match days {
        1 => "yesterday".to_string(),
        _ => format!("{days} days ago"),
    }
}

/// Whether a work was touched in the last seven days — the line §6 draws
/// between "This week" and "Earlier".
pub fn this_week(work: &DisplayThread) -> bool {
    let Some(last_active_ms) = work.last_active_ms else {
        return false;
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    now_ms.saturating_sub(last_active_ms) < 7 * 24 * 60 * 60 * 1000
}

fn hash_name(name: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in name.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> DisplayThread {
        DisplayThread {
            thread_id: 1,
            name: "Work".into(),
            bundle: evo_threads::ResumeBundle {
                resume_point: String::new(),
                resume_reason: String::new(),
                restore_set: vec![],
                open_deltas: vec![],
            },
            is_work: true,
            engagement: 1.0,
            has_open_loop: false,
            urgency_reason: None,
            understanding: crate::understanding_fixture("Work"),
            last_active_ms: None,
            member_subjects: vec![],
            skipped_members: vec![],
        }
    }

    #[test]
    fn age_speaks_in_days_not_hours() {
        let day_ms = 24 * 60 * 60 * 1000;
        let mut work = fixture();
        work.last_active_ms = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                - 4 * day_ms,
        );
        assert_eq!(age_stamp(&work), "4 days ago");
        assert!(this_week(&work));
    }

    #[test]
    fn the_card_sentence_is_plain() {
        // No engine vocabulary reaches a card (§3).
        for work in [fixture()] {
            let line = one_line(&work).to_lowercase();
            for banned in ["observed", "episode", "input without", "pending"] {
                assert!(!line.contains(banned), "card sentence said {banned}");
            }
        }
    }
}
