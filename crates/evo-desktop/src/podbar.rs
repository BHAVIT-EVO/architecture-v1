//! The Pod Bar: ⌥Space summons every work as itself.
//!
//! The bar is an overlay, never a mode: it floats above the current window,
//! takes no focus it cannot return, and leaves by the same door it entered
//! (Esc, ⌥Space again, a click outside, or choosing a pod). Every card is
//! painted only from what the engine witnessed — name, one honest reason
//! line, position-true stage geometry, and badges the deltas entitle us to.
//! There is no imagery, no prediction, and no badge the engine didn't earn.

use crate::pods::PodCommand;
use evo_pods::pod::{Pod, PodBadge};

/// The hosted overlay state: visible or not, and which card the pointer is
/// over (for the pressed/hover tint only — never an invisible mode switch).
pub struct PodBar {
    pub visible: bool,
    pub hovered: Option<usize>,
}

impl PodBar {
    pub fn new() -> Self {
        Self {
            visible: false,
            hovered: None,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if !self.visible {
            self.hovered = None;
        }
    }
}

/// One queued UI-level action back to the host (frame-flow, on the
/// renderer thread): the bar proposes, the runtime disposes.
pub enum PodBarAction {
    Command(PodCommand),
    Dismiss,
}

// ─── Palette: the bar is a night surface above whatever day the user was in.
const BAR_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(9, 12, 17, 238);
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
) -> Vec<PodBarAction> {
    let mut actions = Vec::new();
    if !bar.visible {
        return actions;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        actions.push(PodBarAction::Dismiss);
        return actions;
    }

    let screen = ctx.viewport_rect();
    let columns = pods.len().clamp(1, 4);
    let rows = pods.len().div_ceil(columns).max(1);
    let card_w = 250.0f32;
    let card_h = 196.0f32;
    let gap = 14.0f32;
    let panel_w = 56.0 + columns as f32 * card_w + (columns as f32 - 1.0) * gap;
    let panel_h = 108.0 + rows as f32 * card_h + (rows as f32 - 1.0) * gap;
    let panel_size = egui::vec2(
        panel_w.min(screen.width() * 0.92),
        panel_h.min(screen.height() * 0.90),
    );
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
                egui::Color32::from_rgba_unmultiplied(4, 6, 9, 128),
            );

            // Panel above it: frame, then every interactive card.
            ui.painter().rect_filled(panel_rect, 18.0, BAR_BG);
            ui.painter().rect_stroke(
                panel_rect,
                18.0,
                egui::Stroke::new(1.0, BAR_EDGE),
                egui::StrokeKind::Inside,
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(panel_rect.shrink(28.0))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.scope(|ui| {
                ui.label(
                    egui::RichText::new("Pod bar")
                        .size(20.0)
                        .strong()
                        .color(TEXT_MAIN),
                );
                ui.label(
                    egui::RichText::new(
                        "Your works, as themselves. \u{2325}1\u{2013}\u{2325}9 to switch, Esc to close.",
                    )
                    .size(11.0)
                    .color(TEXT_DIM),
                );
                ui.add_space(14.0);

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
                for chunk in pods.chunks(columns) {
                    ui.horizontal(|ui| {
                        for pod in chunk {
                            let index = pods
                                .iter()
                                .position(|candidate| candidate.id == pod.id)
                                .unwrap_or(0);
                            let card_rect = egui::Rect::from_min_size(
                                ui.cursor().min,
                                egui::vec2(card_w, card_h),
                            );
                            let clicked = card(
                                ui,
                                card_rect,
                                pod,
                                index,
                                active_id == Some(pod.id.0),
                                index < max_dial,
                            );
                            ui.allocate_rect(card_rect, egui::Sense::hover());
                            if clicked {
                                bar_actions.push(PodBarAction::Command(PodCommand::Enter(index)));
                                bar_actions.push(PodBarAction::Dismiss);
                            }
                            ui.add_space(gap);
                        }
                    });
                    ui.add_space(gap);
                }
                actions.extend(bar_actions);
            });

            if backdrop.clicked() {
                actions.push(PodBarAction::Dismiss);
            }
        });

    actions
}

/// One pod card: color rail, name, the engine's reason line, a position-true
/// miniature of the pod's stage, up to three witnessed badges, and the ⌥N
/// hint when one exists. Returns true when the card was clicked.
fn card(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    pod: &Pod,
    index: usize,
    active: bool,
    dialable: bool,
) -> bool {
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

    // The dial hint (⌥N) when a hotkey exists for this index.
    if dialable && index < 9 {
        let hint = format!("⌥{}", index + 1);
        let hint_galley = ui.painter().layout_no_wrap(
            hint,
            egui::FontId::proportional(11.0),
            TEXT_DIM,
        );
        let hint_pos = egui::pos2(
            rect.max.x - hint_galley.size().x - 10.0,
            rect.min.y + 10.0,
        );
        ui.painter().galley(hint_pos, hint_galley, TEXT_DIM);
    }

    response.clicked()
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
