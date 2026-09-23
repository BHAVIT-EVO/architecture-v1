//! The Pod Bar: ⌥Space summons every work as itself.
//!
//! The bar is an overlay, never a mode: it floats above the current window,
//! takes no focus it cannot return, and leaves by the same door it entered
//! (Esc, ⌥Space again, a click outside, or choosing a pod). Every card is
//! painted only from what the engine witnessed — name, one honest reason
//! line, position-true stage geometry, and badges the deltas entitle us to.
//! There is no imagery, no prediction, and no badge the engine didn't earn.
//!
//! The surface is opaque and singular: one strip of cards that scrolls
//! sideways when the day outgrows the screen — never a clipped second row,
//! never the desktop bleeding through.

use crate::pods::PodCommand;
use evo_pods::pod::{Pod, PodBadge, PodResource};
use evo_pods::surface::PodWindow;

/// The hosted overlay state: visible or not, and which card the pointer is
/// over (for the pressed/hover tint only — never an invisible mode switch).
pub struct PodBar {
    pub visible: bool,
    pub hovered: Option<usize>,
    /// Add-to-Pod picker: which pod index is open for teaching.
    pub add_for: Option<usize>,
    /// Draft resource kind: 0 = app, 1 = file, 2 = folder, 3 = URL.
    pub draft_kind: usize,
    /// Draft resource value (text field contents).
    pub draft_text: String,
}

impl PodBar {
    pub fn new() -> Self {
        Self {
            visible: false,
            hovered: None,
            add_for: None,
            draft_kind: 0,
            draft_text: String::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if !self.visible {
            self.hovered = None;
            self.add_for = None;
            self.draft_text.clear();
        }
    }
}

/// One queued UI-level action back to the host (frame-flow, on the
/// renderer thread): the bar proposes, the runtime disposes.
pub enum PodBarAction {
    Command(PodCommand),
    Dismiss,
    /// The card's "+" danced: open the Add-to-Pod picker for this pod.
    OpenAdd(usize),
    /// The picker is done; back to the strip (the bar itself stays).
    CloseAdd,
}

/// What the active pod is currently doing to the rest of the world:
/// (split-app windows parked, alien apps hidden, own windows remembered).
/// Painted on the active card, derived from the live lease's receipts.
pub type StageStats = (usize, usize, usize);

// ─── Palette: the bar is a night surface above whatever day the user was in.
// Fully opaque by decision: a space switcher must read as its own place,
// never as a tint on the mess it replaces.
const BAR_BG: egui::Color32 = egui::Color32::from_rgb(9, 12, 17);
const BAR_EDGE: egui::Color32 = egui::Color32::from_rgb(54, 64, 76);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(16, 21, 28);
const CARD_ACTIVE_EDGE: egui::Color32 = egui::Color32::from_rgb(120, 168, 255);
const CARD_IDLE_EDGE: egui::Color32 = egui::Color32::from_rgb(38, 46, 56);
const TEXT_MAIN: egui::Color32 = egui::Color32::from_rgb(232, 236, 240);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(150, 160, 172);
const STAGE_WELL: egui::Color32 = egui::Color32::from_rgb(10, 13, 18);

/// Paints the bar as a centered overlay above the normal UI. Returns the
/// actions the host applies after paint (enter, dismiss). The bar owns no
/// state beyond visibility: every card is re-derived from `pods` each frame.
pub fn show(
    ctx: &egui::Context,
    bar: &mut PodBar,
    pods: &[Pod],
    active_id: Option<u64>,
    max_dial: usize,
    stage_stats: Option<StageStats>,
    open_inventory: &[PodWindow],
) -> Vec<PodBarAction> {
    let mut actions = Vec::new();
    if !bar.visible {
        return actions;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if bar.add_for.is_some() {
            actions.push(PodBarAction::CloseAdd);
        } else {
            actions.push(PodBarAction::Dismiss);
        }
        return actions;
    }

    let screen = ctx.viewport_rect();
    let card_w = 250.0f32;
    let card_h = 196.0f32;
    let gap = 10.0f32;
    let header_h = 58.0f32;
    let strip_chrome = 18.0f32; // room the scrollbar occupies when it exists
    // One strip: the width the cards honestly need, capped to the screen.
    let cards_w = pods.len() as f32 * card_w + pods.len().saturating_sub(1) as f32 * gap;
    let panel_w = (56.0 + cards_w.max(card_w)).min(screen.width() * 0.92);
    let panel_h = 28.0 + header_h + card_h + strip_chrome + 16.0;
    let panel_size = egui::vec2(panel_w, panel_h.min(screen.height() * 0.92));
    let panel_rect = egui::Rect::from_center_size(screen.center(), panel_size);

    // ONE area (not two): sibling areas at equal z-order once let the
    // backdrop swallow the panel's clicks. Inside a single area the
    // backdrop is created FIRST and the panel's widgets LATER, and egui
    // resolves overlapping hits newest-first — cards click, the dim
    // around them dismisses.
    egui::Area::new(egui::Id::new("evo-podbar"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            // Backdrop first: painted dim + one hit surface for the
            // outside-click dismissal.
            let (_, backdrop) =
                ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter().rect_filled(
                screen,
                0.0,
                egui::Color32::from_rgba_unmultiplied(4, 6, 9, 150),
            );

            // Panel above it: opaque frame, then every interactive card.
            ui.painter().rect_filled(panel_rect, 16.0, BAR_BG);
            ui.painter().rect_stroke(
                panel_rect,
                16.0,
                egui::Stroke::new(1.0, BAR_EDGE),
                egui::StrokeKind::Inside,
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(panel_rect.shrink2(egui::vec2(28.0, 14.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.scope(|ui| {
                // One-line chrome: name the instrument, then let the works
                // carry the weight. (Taller headers once ate a card-row.)
                egui::Grid::new("evo-podbar-head").num_columns(2).show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Pod bar")
                            .size(18.0)
                            .strong()
                            .color(TEXT_MAIN),
                    );
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new(
                                    "Each pod is its own space. \u{2325}1\u{2013}\u{2325}9 jump \u{b7} Esc closes \u{b7} when the day outruns the screen, the strip scrolls sideways.",
                                )
                                .size(10.5)
                                .color(TEXT_DIM),
                            );
                        },
                    );
                });
                ui.add_space(6.0);

                if pods.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "No works to show yet — pods appear as work accrues.",
                        )
                        .size(12.0)
                        .color(TEXT_DIM),
                    );
                    return;
                }

                let mut bar_actions: Vec<PodBarAction> = Vec::new();
                // The one sideways strip. egui hands vertical wheel input
                // to a horizontal-only scroll area, so the gesture the
                // hand already does (wheel / two-finger drag) scrolls it.
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for (index, pod) in pods.iter().enumerate() {
                                let card_rect = egui::Rect::from_min_size(
                                    ui.cursor().min,
                                    egui::vec2(card_w, card_h),
                                );
                                let active = active_id == Some(pod.id.0);
                                let stats =
                                    if active { stage_stats } else { None };
                                let hit = card(
                                    ui,
                                    card_rect,
                                    pod,
                                    index,
                                    active,
                                    index < max_dial,
                                    stats,
                                );
                                ui.allocate_rect(card_rect, egui::Sense::hover());
                                match hit {
                                    CardHit::Enter => {
                                        bar_actions
                                            .push(PodBarAction::Command(PodCommand::Enter(index)));
                                        bar_actions.push(PodBarAction::Dismiss);
                                    }
                                    CardHit::Add => {
                                        bar_actions.push(PodBarAction::OpenAdd(index));
                                    }
                                    CardHit::None => {}
                                }
                            }
                        });
                    });
                actions.extend(bar_actions);
            });

            if backdrop.clicked() {
                actions.push(PodBarAction::Dismiss);
            }
        });

    // The Add-to-Pod picker rides above the strip (Tooltip order: hits go
    // newest-first, so the picker's buttons eat clicks the strip can't).
    if let Some(add_index) = bar.add_for {
        if let Some(pod) = pods.get(add_index) {
            actions.extend(show_add_picker(ctx, bar, add_index, pod, open_inventory));
        } else {
            bar.add_for = None;
        }
    }

    actions
}

/// What one card tap meant (the "+" is a separate hit from entering).
enum CardHit {
    Enter,
    Add,
    None,
}

/// The Add-to-Pod picker for one pod: claim a witnessed open window, or
/// teach the pod a named thing it can re-open. Opaque, centered, one at a
/// time — the bar's entire vocabulary, for one work.
fn show_add_picker(
    ctx: &egui::Context,
    bar: &mut PodBar,
    add_index: usize,
    pod: &Pod,
    open_inventory: &[PodWindow],
) -> Vec<PodBarAction> {
    let mut actions: Vec<PodBarAction> = Vec::new();
    picker_area(ctx, |ui| {
        let tint = egui::Color32::from_rgb(pod.color.r, pod.color.g, pod.color.b);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Add to {}", pod.name))
                    .size(15.0)
                    .strong()
                    .color(TEXT_MAIN),
            );
        });
        ui.colored_label(
            TEXT_DIM,
            "Say what belongs to this work. It stays remembered — jump pods and it is here.",
        );
        ui.add_space(8.0);

        // What it already knows: one tap re-opens.
        if !pod.resources.is_empty() {
            ui.colored_label(TEXT_DIM, "this pod already knows:");
            ui.horizontal_wrapped(|ui| {
                for (res_index, resource) in pod.resources.iter().enumerate() {
                    let label = format!("{} {}", resource.kind_label(), resource.value());
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new(truncate(&label, 34)).size(10.5).color(TEXT_MAIN),
                        ))
                        .on_hover_text("open it now")
                        .clicked()
                    {
                        actions.push(PodBarAction::Command(PodCommand::OpenResource(
                            add_index, res_index,
                        )));
                    }
                }
            });
            ui.add_space(8.0);
        }

        // Section 1: claim one of the open windows.
        ui.label(main("An open window on the desktop:"));
        let list: Vec<&PodWindow> = open_inventory
            .iter()
            .filter(|w| !w.title.trim().is_empty() || w.ax_document.is_some())
            .take(10)
            .collect();
        if list.is_empty() {
            ui.colored_label(
                TEXT_DIM,
                "nothing on the desktop to claim (or the screen could not be read — check Accessibility).",
            );
        } else {
            egui::ScrollArea::vertical().max_height(130.0).show(ui, |ui| {
                for window in &list {
                    let title = window.title.trim();
                    let row_text = egui::RichText::new(if title.is_empty() {
                        "(untitled)".to_string()
                    } else {
                        truncate(title, 46)
                    })
                    .size(11.5)
                    .color(TEXT_MAIN);
                    let response = ui
                        .selectable_label(false, row_text)
                        .on_hover_text(format!("{} — say it is this pod's", window.owner));
                    if response.clicked() {
                        actions.push(PodBarAction::Command(PodCommand::ClaimWindow(
                            add_index,
                            (*window).clone(),
                        )));
                        actions.push(PodBarAction::CloseAdd);
                    }
                    ui.colored_label(TEXT_DIM, truncate(&window.owner, 40));
                }
            });
        }
        ui.add_space(10.0);

        // Section 2: anything else this pod should be able to open.
        ui.label(main("Anything else this pod should be able to open:"));
        ui.horizontal_wrapped(|ui| {
            for (kind_index, label) in ["app", "file", "folder", "URL"].iter().enumerate() {
                if ui
                    .selectable_label(bar.draft_kind == kind_index, *label)
                    .clicked()
                {
                    bar.draft_kind = kind_index;
                }
            }
        });
        ui.add(
            egui::TextEdit::singleline(&mut bar.draft_text)
                .hint_text(match bar.draft_kind {
                    0 => "app name, e.g. what the Dock calls it",
                    1 => "full path, e.g. /Users/you/work/notes.md",
                    2 => "full path, e.g. /Users/you/work/project",
                    _ => "https://…",
                })
                .desired_width(370.0),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let can_add = !bar.draft_text.trim().is_empty();
            if ui
                .add_enabled(can_add, egui::Button::new("Add to this pod"))
                .clicked()
            {
                let value = bar.draft_text.trim().to_string();
                let resource = match bar.draft_kind {
                    0 => PodResource::App(value),
                    1 => PodResource::File(value),
                    2 => PodResource::Folder(value),
                    _ => PodResource::Url(value),
                };
                actions.push(PodBarAction::Command(PodCommand::AddResource(
                    add_index, resource,
                )));
                bar.draft_text.clear();
            }
            if ui.button("Done").clicked() {
                actions.push(PodBarAction::CloseAdd);
            }
        });
    });
    actions
}

fn main(s: &str) -> egui::RichText {
    egui::RichText::new(s.to_string()).size(11.5).color(TEXT_MAIN)
}

fn truncate(s: &str, max: usize) -> String {
    let borrowed: String = s.chars().take(max).collect();
    if borrowed.chars().count() < s.chars().count() {
        format!("{borrowed}…")
    } else {
        borrowed
    }
}

/// The picker's framed area: opaque, centered, on top.
fn picker_area(ctx: &egui::Context, content: impl FnOnce(&mut egui::Ui)) {
    egui::Area::new(egui::Id::new("evo-podbar-add"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(BAR_BG)
                .stroke(egui::Stroke::new(1.0, BAR_EDGE))
                .corner_radius(12.0)
                .inner_margin(egui::Margin::symmetric(18, 14))
                .show(ui, |ui| {
                    ui.set_width(400.0);
                    content(ui);
                });
        });
}

/// One pod card: color rail, name, the engine's reason line, a position-true
/// miniature of the pod's stage, up to three witnessed badges, and the ⌥N
/// hint when one exists. When the card is the active stage, the right-top
/// speaks the containment receipts instead (parked / hidden / remembered).
/// The "+" at the bottom-right opens Add-to-Pod; the rest of the card
/// enters the pod.
fn card(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    pod: &Pod,
    index: usize,
    active: bool,
    dialable: bool,
    stage_stats: Option<StageStats>,
) -> CardHit {
    let response = ui.interact(rect, egui::Id::new(("evo-podbar-card", pod.id.0)), egui::Sense::click());
    let tint = pod_color(pod);
    let lift = if response.hovered() { 0.06 } else { 0.0 };
    let bg = mix(CARD_BG, tint, lift);
    ui.painter().rect_filled(rect, 10.0, bg);
    ui.painter().rect_stroke(
        rect,
        10.0,
        egui::Stroke::new(
            if active { 2.0 } else { 1.0 },
            if active { CARD_ACTIVE_EDGE } else { CARD_IDLE_EDGE },
        ),
        egui::StrokeKind::Inside,
    );

    // Color rail: the pod's identity color, always visible.
    let rail = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + 1.0, rect.min.y + 8.0),
        egui::pos2(rect.min.x + 5.0, rect.max.y - 8.0),
    );
    ui.painter().rect_filled(rail, 2.0, tint);

    let mut cursor_y = rect.min.y + 12.0;
    let text_x = rect.min.x + 16.0;
    let text_w = rect.width() - 28.0;

    // Name (one line, truncated by paint, honest).
    let name_galley = ui.painter().layout(
        pod.name.clone(),
        egui::FontId::proportional(14.0),
        TEXT_MAIN,
        text_w,
    );
    ui.painter().galley(egui::pos2(text_x, cursor_y), name_galley, TEXT_MAIN);
    cursor_y += 20.0;

    // The reason: the engine's sentence, exactly as given.
    let reason_galley = ui.painter().layout(
        pod.reason.clone(),
        egui::FontId::proportional(10.5),
        TEXT_DIM,
        text_w,
    );
    ui.painter().galley(egui::pos2(text_x, cursor_y), reason_galley, TEXT_DIM);
    cursor_y += 18.0;

    // Mini stage: position-true geometry of hero + satellites.
    let stage_rect = egui::Rect::from_min_size(
        egui::pos2(text_x, cursor_y),
        egui::vec2(text_w, 74.0),
    );
    ui.painter().rect_filled(stage_rect, 6.0, STAGE_WELL);
    paint_stage(ui.painter(), stage_rect, pod, tint);
    cursor_y += 80.0;

    // Badges: witnessed states only, costliest first, at most three.
    for badge in pod.badges.iter().take(3) {
        chip(ui.painter(), text_x, cursor_y, badge, tint);
        cursor_y += 16.0;
    }

    // Right-top corner: the receipts of containment while on stage, else
    // the dial hint (⌥N) when a hotkey exists for this index.
    let corner: Option<String> = match stage_stats {
        Some((parked, hidden, remembered)) => Some(format!(
            "on stage \u{b7} {parked} parked \u{b7} {hidden} hidden \u{b7} {remembered} remembered"
        )),
        None if dialable && index < 9 => Some(format!("\u{2325}{}", index + 1)),
        None => None,
    };
    if let Some(text) = corner {
        let galley = ui
            .painter()
            .layout_no_wrap(text, egui::FontId::proportional(10.0), TEXT_DIM);
        ui.painter().galley(
            egui::pos2(rect.max.x - galley.size().x - 10.0, rect.min.y + 12.0),
            galley,
            TEXT_DIM,
        );
    }

    // Add-to-Pod: a small "+" at the card's bottom-right. Its hit rect is
    // registered AFTER the card's, so egui's newest-first ordering means
    // "+" never becomes an accidental "enter this pod".
    let plus_rect = egui::Rect::from_min_size(
        egui::pos2(rect.max.x - 34.0, rect.max.y - 34.0),
        egui::vec2(26.0, 26.0),
    );
    let plus = ui.interact(
        plus_rect,
        egui::Id::new(("evo-podbar-add-btn", pod.id.0)),
        egui::Sense::click(),
    );
    let plus_bg = if plus.hovered() {
        mix(CARD_BG, tint, 0.35)
    } else {
        mix(CARD_BG, tint, 0.16)
    };
    ui.painter().rect_filled(plus_rect, 8.0, plus_bg);
    ui.painter().rect_stroke(
        plus_rect,
        8.0,
        egui::Stroke::new(1.0, mix(CARD_BG, tint, 0.6)),
        egui::StrokeKind::Inside,
    );
    let plus_galley = ui.painter().layout_no_wrap(
        "+".to_string(),
        egui::FontId::proportional(15.0),
        TEXT_MAIN,
    );
    ui.painter().galley(
        plus_rect.center() - plus_galley.size() / 2.0 - egui::vec2(0.0, 1.0),
        plus_galley,
        TEXT_MAIN,
    );

    if plus.clicked() {
        CardHit::Add
    } else if response.clicked() {
        CardHit::Enter
    } else {
        CardHit::None
    }
}

/// The card chip: kind label + the resource basename it concerns.
fn chip(painter: &egui::Painter, x: f32, y: f32, badge: &PodBadge, tint: egui::Color32) {
    let subject = badge
        .subject
        .rsplit('/')
        .next()
        .unwrap_or(&badge.subject)
        .chars()
        .take(22)
        .collect::<String>();
    let text = format!("{} · {}", badge.kind.label(), subject);
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(9.5), TEXT_DIM);
    let chip_rect = egui::Rect::from_min_size(
        egui::pos2(x, y),
        galley.size() + egui::vec2(10.0, 4.0),
    );
    painter.rect_filled(chip_rect, 7.0, mix(CARD_BG, tint, 0.18));
    painter.galley(chip_rect.min + egui::vec2(5.0, 2.0), galley, TEXT_MAIN);
}

/// The miniature stage: the pod's own geometry vocabulary. The hero (the
/// resume point, when it is among the restore set) takes the wide left;
/// up to three satellites stack on the right; heavier tint = nearer the
/// resume point. No imagery — position and proportion are the truth.
fn paint_stage(painter: &egui::Painter, rect: egui::Rect, pod: &Pod, tint: egui::Color32) {
    let items: Vec<&String> = pod
        .bundle
        .restore_set
        .iter()
        .filter(|item| *item != &pod.bundle.resume_point)
        .take(3)
        .collect();
    let has_hero = !pod.bundle.resume_point.is_empty();

    let well = rect.shrink(4.0);
    let hero_w = if has_hero { well.width() * 0.6 } else { 0.0 };

    if has_hero {
        let hero = egui::Rect::from_min_size(
            well.min,
            egui::vec2(hero_w, well.height()),
        );
        painter.rect_filled(hero, 4.0, mix(STAGE_WELL, tint, 0.45));
        painter.rect_stroke(
            hero,
            4.0,
            egui::Stroke::new(1.0, mix(STAGE_WELL, tint, 0.9)),
            egui::StrokeKind::Inside,
        );
    }

    let sat_x = well.min.x + hero_w + if has_hero { 4.0 } else { 0.0 };
    let sat_w = well.max.x - sat_x;
    if sat_w > 8.0 && !items.is_empty() {
        let slot_h = (well.height() - 3.0 * (items.len() as f32 - 1.0)) / items.len() as f32;
        for (slot, _) in items.iter().enumerate() {
            let sat = egui::Rect::from_min_size(
                egui::pos2(sat_x, well.min.y + slot as f32 * (slot_h + 3.0)),
                egui::vec2(sat_w, slot_h),
            );
            painter.rect_filled(sat, 4.0, mix(STAGE_WELL, tint, 0.22));
            painter.rect_stroke(
                sat,
                4.0,
                egui::Stroke::new(1.0, mix(STAGE_WELL, tint, 0.5)),
                egui::StrokeKind::Inside,
            );
        }
    }

    if !has_hero && items.is_empty() {
        // An honestly empty stage: nothing witnessed yet this episode.
        let galley = painter.layout_no_wrap(
            "nothing witnessed yet".to_string(),
            egui::FontId::proportional(9.0),
            TEXT_DIM,
        );
        painter.galley(
            rect.center() - galley.size() / 2.0,
            galley,
            TEXT_DIM,
        );
    }
}

/// The pod's identity color as an egui color.
fn pod_color(pod: &Pod) -> egui::Color32 {
    let (r, g, b) = (pod.color.r, pod.color.g, pod.color.b);
    egui::Color32::from_rgb(r, g, b)
}

/// Blend two colors — hover lifts and stage depth without a second palette.
fn mix(base: egui::Color32, tint: egui::Color32, amount: f32) -> egui::Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let blend = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount) as u8;
    egui::Color32::from_rgb(
        blend(base.r(), tint.r()),
        blend(base.g(), tint.g()),
        blend(base.b(), tint.b()),
    )
}
