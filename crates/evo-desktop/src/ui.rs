//! Reusable layout, material and control primitives.
//!
//! Every screen composes from this module. Nothing here knows about Evo's
//! domain — these are the physical building blocks, so that spacing, text,
//! materials, motion and control sizing are decided **once** rather than
//! re-litigated at every call site with magic numbers.
//!
//! Three rules this module exists to enforce:
//!
//! 1. **A gap is always a step on the scale.** `egui`'s implicit item
//!    spacing is switched off in [`crate::theme::install`], so every vertical
//!    space in Evo is deliberate and comes from here.
//! 2. **Text is never clipped.** Controls measure their own label and
//!    allocate that width. When a row of controls cannot fit, it *reflows*
//!    into a column instead of truncating. When a single label cannot fit, it
//!    wraps and the control grows taller.
//! 3. **Feedback is immediate.** Presses register on pointer-*down*, not on
//!    release, and every state change is a short critically-damped
//!    transition rather than a hard cut.

use crate::theme::{self, Role, Severity};

use egui::{Color32, CornerRadius, Margin, Pos2, Rect, Response, RichText, Sense, Stroke, Vec2};

// ────────────────────────────────────────────────────────────────────────
// Color helpers
// ────────────────────────────────────────────────────────────────────────

/// Blends two **opaque** colors.
fn mix(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
    )
}

/// Fades an **opaque** color to a fraction of full opacity.
///
/// Only valid for opaque inputs: `Color32` stores premultiplied components,
/// so reading them back out of an already-translucent color would lose its
/// hue. Translucent tints are therefore never passed through here.
fn veil(color: Color32, t: f32) -> Color32 {
    debug_assert!(
        color.a() == 255,
        "veil() reads back rgb components and is only valid for opaque colors"
    );
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (255.0 * t.clamp(0.0, 1.0)).round() as u8,
    )
}

/// A neutral shade at a fraction of the given alpha. Black is safe to build
/// this way because premultiplication leaves its components at zero.
fn shade(alpha: u8, t: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, (alpha as f32 * t.clamp(0.0, 1.0)).round() as u8)
}

// ────────────────────────────────────────────────────────────────────────
// Environment and edges
// ────────────────────────────────────────────────────────────────────────

/// Paints the window environment: a warm neutral lit from above.
///
/// The renderer has no gradient primitive and cannot sample the desktop
/// behind the window, so this is a banded wash of about seven luminance
/// units — a lighting cue, deliberately *not* a claim of translucency. Call
/// this before any content so it sits behind everything.
pub fn paint_environment(painter: &egui::Painter, rect: Rect) {
    const BANDS: usize = 16;
    let band_height = rect.height() / BANDS as f32;
    for index in 0..BANDS {
        let t = index as f32 / (BANDS - 1) as f32;
        let band = Rect::from_min_size(
            egui::pos2(rect.min.x, rect.min.y + band_height * index as f32),
            // One pixel of overlap so no seam can appear between bands.
            Vec2::new(rect.width(), band_height + 1.0),
        );
        painter.rect_filled(
            band,
            CornerRadius::ZERO,
            mix(theme::ENV_TOP, theme::ENV_BOTTOM, t),
        );
    }
}

/// The soft edge where scrolling content passes beneath fixed chrome.
///
/// A hard one-pixel divider announces itself; a short fade lets the content
/// dissolve into the chrome instead. `anchor_top` fades downward from the top
/// edge (content sliding under a header); otherwise it fades upward from the
/// bottom edge (content sliding under the action strip).
pub fn paint_scroll_edge(painter: &egui::Painter, rect: Rect, color: Color32, anchor_top: bool) {
    const STEPS: usize = 8;
    let step = rect.height() / STEPS as f32;
    for index in 0..STEPS {
        let t = index as f32 / (STEPS - 1) as f32;
        // Opaque against the chrome, transparent where the content is free.
        let alpha = if anchor_top { 1.0 - t } else { t };
        let band = Rect::from_min_size(
            egui::pos2(rect.min.x, rect.min.y + step * index as f32),
            Vec2::new(rect.width(), step + 1.0),
        );
        painter.rect_filled(band, CornerRadius::ZERO, veil(color, alpha * alpha));
    }
}

// ────────────────────────────────────────────────────────────────────────
// Layout
// ────────────────────────────────────────────────────────────────────────

/// Vertical space, always a step from the scale.
pub fn gap(ui: &mut egui::Ui, amount: f32) {
    ui.add_space(amount);
}

/// A hairline rule spanning the available width.
pub fn hairline(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, theme::border());
}

/// A hairline that stops short of both margins, for separating peers in a
/// list without drawing a box around them.
pub fn hairline_inset(ui: &mut egui::Ui, inset: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    let rect = Rect::from_min_max(
        egui::pos2(rect.min.x + inset, rect.min.y),
        egui::pos2(rect.max.x - inset, rect.max.y),
    );
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, theme::border());
}

/// A centered column of at most `width`, so long lines never sprawl across a
/// wide window. Everything on Home and in the detail main column sits in one.
pub fn measure_column<R>(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let column = width.min(ui.available_width());
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.allocate_ui(Vec2::new(column, ui.available_height()), |ui| {
            // Content inside a centered column is still left-aligned: only
            // the column is centered, never the text.
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), add)
                .inner
        })
        .inner
    })
    .inner
}

/// The page inset. Every screen's content sits inside exactly one of these,
/// so the distance from the window edge to the first character is decided in
/// one place instead of being re-guessed per screen.
pub fn page<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .inner_margin(Margin::symmetric(theme::M6, theme::M5))
        .show(ui, add)
        .inner
}

/// A scrolling page body. The scroll region owns the full width — including
/// the scrollbar's lane — and the page inset sits inside it, so content never
/// shifts sideways when a scrollbar appears.
pub fn scroll<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| page(ui, add))
        .inner
}

/// Constrains a block to a readable measure without centering it.
pub fn constrained<R>(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.vertical(|ui| {
        ui.set_max_width(width.min(ui.available_width()));
        add(ui)
    })
    .inner
}

/// Whether `width` still fits in the current layout.
pub fn fits(ui: &egui::Ui, width: f32) -> bool {
    width <= ui.available_width() + 0.5
}

/// The rendered size of a string in a role.
///
/// `layout_no_wrap` does not apply the role's tracking, so positive tracking
/// is added back. Negative tracking is *not* subtracted: over-measuring is
/// harmless, under-measuring is what clips text.
pub fn measure(ui: &egui::Ui, role: Role, value: &str) -> Vec2 {
    let galley = ui
        .painter()
        .layout_no_wrap(value.to_owned(), role.font(), theme::TEXT);
    let tracking = (role.tracking() * value.chars().count() as f32).max(0.0);
    Vec2::new(galley.size().x + tracking, galley.size().y)
}

// ────────────────────────────────────────────────────────────────────────
// Text
// ────────────────────────────────────────────────────────────────────────

/// A single line of text in a role. Returns the response so callers can make
/// it interactive when they need to.
pub fn label(ui: &mut egui::Ui, role: Role, value: &str, color: Color32) -> Response {
    ui.label(theme::text(role, value, color))
}

/// The one sentence a screen exists to say.
pub fn display(ui: &mut egui::Ui, value: &str) {
    label(ui, Role::Display, value, theme::TEXT);
    gap(ui, theme::S3);
}

/// A screen or region heading.
pub fn headline(ui: &mut egui::Ui, value: &str) {
    label(ui, Role::Headline, value, theme::TEXT);
    gap(ui, theme::S3);
}

/// A group title.
pub fn title(ui: &mut egui::Ui, value: &str) {
    label(ui, Role::Title, value, theme::TEXT);
    gap(ui, theme::S2);
}

/// An all-caps region label. Sits above the thing it names, close enough that
/// the proximity does the work a box would otherwise do.
pub fn eyebrow(ui: &mut egui::Ui, value: &str) {
    ui.label(theme::eyebrow(value, theme::TERTIARY));
    gap(ui, theme::S2);
}

/// The supporting sentence under a heading.
pub fn lede(ui: &mut egui::Ui, value: &str) {
    constrained(ui, theme::PROSE_WIDTH, |ui| {
        ui.label(theme::text(Role::Lede, value, theme::MUTED));
    });
    gap(ui, theme::S4);
}

/// Body prose, wrapped to a readable measure.
pub fn prose(ui: &mut egui::Ui, value: &str) {
    constrained(ui, theme::PROSE_WIDTH, |ui| {
        ui.label(theme::text(Role::Body, value, theme::MUTED));
    });
    gap(ui, theme::S3);
}

/// A short explanatory line — a count, a reason, a source.
pub fn caption(ui: &mut egui::Ui, value: &str) {
    constrained(ui, theme::PROSE_WIDTH, |ui| {
        ui.label(theme::text(Role::Caption, value, theme::TERTIARY));
    });
    gap(ui, theme::S2);
}

/// How Evo knows something. Every derived claim carries one of these, which
/// is what lets the interface state a conclusion without asserting it.
pub fn provenance(ui: &mut egui::Ui, value: &str) {
    constrained(ui, theme::PROSE_WIDTH, |ui| {
        ui.label(theme::text(Role::Caption, value, theme::TERTIARY));
    });
    gap(ui, theme::S1);
}

/// A named value, stacked: the name above in the eyebrow voice, the value
/// below with the weight. Reading down a column of these gives the field
/// names for free without a table.
pub fn field(ui: &mut egui::Ui, name: &str, value: &str) {
    ui.label(theme::eyebrow(name, theme::TERTIARY));
    gap(ui, theme::S1);
    constrained(ui, theme::PROSE_WIDTH, |ui| {
        ui.label(theme::text(Role::BodyStrong, value, theme::TEXT));
    });
}

/// A single line that shortens with an ellipsis rather than overflowing —
/// used only for values Evo does not control the length of, such as a
/// resource identifier. Never used for a control label.
pub fn one_line(ui: &mut egui::Ui, role: Role, value: &str, color: Color32) -> Response {
    ui.add(egui::Label::new(theme::text(role, value, color)).truncate())
}

/// Text Evo does not have. Reads as a deliberate absence, not a broken
/// value: the same voice as a caption, at reduced presence.
pub fn unknown(ui: &mut egui::Ui, value: &str) {
    ui.scope(|ui| {
        ui.set_opacity(0.85);
        constrained(ui, theme::PROSE_WIDTH, |ui| {
            ui.label(theme::text(Role::Body, value, theme::TERTIARY));
        });
    });
    gap(ui, theme::S2);
}

// ────────────────────────────────────────────────────────────────────────
// Materials
// ────────────────────────────────────────────────────────────────────────

/// The single primary object on a screen. At most one per screen — a second
/// raised surface has nothing left to be raised *above*.
pub fn raised<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    theme::raised_frame(false).show(ui, add).inner
}

/// A contextual, supporting region: darker than the environment, lowered
/// into it, no shadow. Weight is what marks it as secondary — not a second
/// white card, and never a navigation rail.
pub fn recessed<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    theme::recessed_frame().show(ui, add).inner
}

/// Historical or related material: no fill at all, introduced by a hairline
/// and set back slightly. History is present without being offered.
pub fn quiet<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    hairline(ui);
    gap(ui, theme::S4);
    ui.scope(|ui| {
        ui.set_opacity(0.9);
        add(ui)
    })
    .inner
}

// ────────────────────────────────────────────────────────────────────────
// Status
// ────────────────────────────────────────────────────────────────────────

/// A status word.
///
/// The word is the indicator. Color reinforces it and never replaces it, so
/// the meaning survives both a monochrome display and a reader who does not
/// distinguish the hues. Sized by its own content, so it cannot clip.
pub fn status_word(ui: &mut egui::Ui, word: &str, severity: Severity) {
    egui::Frame::new()
        .fill(severity.tint())
        .corner_radius(CornerRadius::same(theme::CORNER_SMALL))
        .inner_margin(Margin::symmetric(theme::M2, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(word.to_uppercase())
                    .font(theme::strong_font(11.0))
                    .color(severity.ink())
                    .extra_letter_spacing(0.7),
            );
        });
}

/// A transient message about Evo's own state.
///
/// Shaped as a band with a leading edge bar, deliberately unlike content, so
/// a system message can never be mistaken for a body of work. Returns the
/// band's rect for callers that need to place something against it.
pub fn notice(ui: &mut egui::Ui, severity: Severity, word: &str, message: &str) -> Rect {
    notice_with(ui, severity, word, message, |_| {})
}

/// A notice with trailing actions (Grant permission, Retry, Dismiss).
pub fn notice_with(
    ui: &mut egui::Ui,
    severity: Severity,
    word: &str,
    message: &str,
    actions: impl FnOnce(&mut egui::Ui),
) -> Rect {
    let inner = egui::Frame::new()
        .fill(severity.tint())
        .corner_radius(CornerRadius::same(theme::CORNER_SMALL))
        .inner_margin(Margin::symmetric(theme::M4, theme::M3))
        .show(ui, |ui| {
            // A band spans its column. Without this the frame reports only the
            // width its longest wrapped row happened to need, so the same
            // notice measured 868pt on one screen and 901pt on another and its
            // right edge stopped short of the surface stacked beneath it.
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                status_word(ui, word, severity);
                ui.add_space(theme::S3);
                ui.label(theme::text(Role::Body, message, theme::TEXT));
            });
            actions(ui);
        });
    let rect = inner.response.rect;
    // The leading edge bar, inside the band's own left padding.
    let bar = Rect::from_min_size(
        rect.min + Vec2::new(0.0, 1.0),
        Vec2::new(2.0, rect.height() - 2.0),
    );
    ui.painter()
        .rect_filled(bar, CornerRadius::same(1), severity.ink());
    rect
}

// ────────────────────────────────────────────────────────────────────────
// Controls
// ────────────────────────────────────────────────────────────────────────

/// How much visual weight a control carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Emphasis {
    /// The one action a screen is asking for. Filled with the accent.
    Primary,
    /// A real alternative. A bordered surface, no fill.
    Secondary,
    /// An action that should be available without being offered — dismiss,
    /// back, cancel. Text only until touched.
    Quiet,
}

/// A control to be laid out. Built by [`Emphasis`] constructors so the call
/// site reads as intent rather than styling.
pub struct Control<'a> {
    label: &'a str,
    emphasis: Emphasis,
    enabled: bool,
    min_width: f32,
    salt: &'a str,
}

impl<'a> Control<'a> {
    /// The one action a screen is asking for.
    pub fn primary(label: &'a str) -> Self {
        Self::new(label, Emphasis::Primary)
    }

    /// A real alternative to the primary action.
    pub fn secondary(label: &'a str) -> Self {
        Self::new(label, Emphasis::Secondary)
    }

    /// Available without being offered.
    pub fn quiet(label: &'a str) -> Self {
        Self::new(label, Emphasis::Quiet)
    }

    fn new(label: &'a str, emphasis: Emphasis) -> Self {
        Self {
            label,
            emphasis,
            enabled: true,
            min_width: 0.0,
            salt: "",
        }
    }

    /// A disabled control stays visible and keeps its full label: the user
    /// should be able to read what is unavailable and, from the copy nearby,
    /// why.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A floor on width, for aligning a control with its neighbours.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// Distinguishes two controls that share a label within one screen.
    pub fn salt(mut self, salt: &'a str) -> Self {
        self.salt = salt;
        self
    }

    /// Horizontal padding either side of the label. More emphasis, more air.
    fn padding(&self) -> f32 {
        match self.emphasis {
            Emphasis::Primary => theme::S5,
            Emphasis::Secondary => theme::S4,
            Emphasis::Quiet => theme::S3,
        }
    }

    /// The width this control needs to show its whole label.
    fn intrinsic_width(&self, ui: &egui::Ui) -> f32 {
        let text = measure(ui, Role::BodyStrong, self.label);
        (text.x + self.padding() * 2.0).max(self.min_width)
    }
}

/// Lays out a control, sized to its own label.
///
/// The label is measured, the control is allocated exactly that much room,
/// and the text is centred inside it — so a label cannot be clipped by its
/// own button. If even the intrinsic width does not fit, the control wraps
/// its label and grows taller rather than losing characters.
pub fn control(ui: &mut egui::Ui, spec: Control<'_>) -> Response {
    let intrinsic = spec.intrinsic_width(ui);
    if !fits(ui, intrinsic) {
        return wrapped_control(ui, spec);
    }

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(intrinsic, theme::CONTROL_HEIGHT),
        if spec.enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );

    // Feedback on pointer-down, not on release: waiting for the click to
    // complete before acknowledging it is what makes an interface feel dead.
    let pressed = spec.enabled && response.is_pointer_button_down_on();
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("evo-control-hover"),
        spec.enabled && response.hovered(),
        theme::RESPONSE_TOUCH,
    );
    paint_control(ui, &spec, rect, hover, pressed);
    response
}

/// The fallback path for a control narrower than its label: the label wraps
/// and the control grows. Never reached at the designed minimum window size,
/// but it means the promise "text is never clipped" holds unconditionally.
fn wrapped_control(ui: &mut egui::Ui, spec: Control<'_>) -> Response {
    let (fill, ink, stroke) = resting_colors(&spec);
    let inner = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(theme::CORNER_CONTROL))
        .inner_margin(Margin::symmetric(theme::M3, theme::M2))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(theme::text(Role::BodyStrong, spec.label, ink));
        });
    ui.interact(
        inner.response.rect,
        ui.id().with(("evo-wrapped-control", spec.label, spec.salt)),
        if spec.enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    )
}

/// The fill, ink and border a control shows at rest.
fn resting_colors(spec: &Control<'_>) -> (Color32, Color32, Stroke) {
    match (spec.emphasis, spec.enabled) {
        (Emphasis::Primary, true) => (theme::ACCENT, Color32::WHITE, Stroke::NONE),
        (Emphasis::Primary, false) => (
            // A disabled primary keeps its shape so the screen's structure
            // does not shift when it becomes available.
            shade(theme::border().a(), 1.0),
            theme::TERTIARY,
            Stroke::NONE,
        ),
        (Emphasis::Secondary, true) => (
            theme::RAISED,
            theme::TEXT,
            Stroke::new(1.0, theme::border_strong()),
        ),
        (Emphasis::Secondary, false) => (
            Color32::TRANSPARENT,
            theme::RECEDED,
            Stroke::new(1.0, theme::border()),
        ),
        (Emphasis::Quiet, true) => (Color32::TRANSPARENT, theme::MUTED, Stroke::NONE),
        (Emphasis::Quiet, false) => (Color32::TRANSPARENT, theme::RECEDED, Stroke::NONE),
    }
}

fn paint_control(ui: &mut egui::Ui, spec: &Control<'_>, rect: Rect, hover: f32, pressed: bool) {
    let (fill, ink, stroke) = resting_colors(spec);
    // A press moves the control very slightly away from the finger, which
    // reads as the surface giving under pressure.
    let rect = if pressed { rect.shrink(1.0) } else { rect };
    let radius = CornerRadius::same(theme::CORNER_CONTROL);
    let painter = ui.painter();

    match spec.emphasis {
        Emphasis::Primary if spec.enabled => {
            let fill = if pressed {
                theme::ACCENT_PRESSED
            } else {
                mix(theme::ACCENT, theme::ACCENT_HOVER, hover)
            };
            painter.rect_filled(rect, radius, fill);
        }
        Emphasis::Quiet => {
            if hover > 0.0 {
                painter.rect_filled(rect, radius, shade(theme::hover_wash().a(), hover));
            }
        }
        _ => {
            painter.rect_filled(rect, radius, fill);
            if hover > 0.0 {
                painter.rect_filled(rect, radius, shade(theme::hover_wash().a(), hover * 0.6));
            }
            if stroke.width > 0.0 {
                painter.rect_stroke(rect, radius, stroke, egui::StrokeKind::Inside);
            }
        }
    }

    // The label is laid out again with its final color and positioned by its
    // own measured size. `Painter::galley` takes the galley's *top-left*, so
    // centering must subtract half the galley — passing the rect's centre
    // directly is what pushed labels off the right edge of their controls.
    let galley = ui
        .painter()
        .layout_no_wrap(spec.label.to_owned(), Role::BodyStrong.font(), ink);
    let origin: Pos2 = rect.center() - galley.size() * 0.5;
    ui.painter().galley(origin, galley, ink);
}

/// Lays out several controls as one decision.
///
/// They sit on a row while the row can show every label in full; when it
/// cannot, they stack full-width instead of shrinking. Reflowing keeps the
/// labels readable at any window size, which truncation would not.
pub fn control_row(ui: &mut egui::Ui, specs: Vec<Control<'_>>) -> Vec<Response> {
    debug_assert!(
        specs
            .iter()
            .filter(|spec| spec.emphasis == Emphasis::Primary)
            .count()
            <= 1,
        "a row of controls may offer at most one primary action"
    );
    let intrinsic: f32 = specs
        .iter()
        .map(|spec| spec.intrinsic_width(ui))
        .sum::<f32>()
        + theme::S3 * specs.len().saturating_sub(1) as f32;

    let mut responses = Vec::with_capacity(specs.len());
    if fits(ui, intrinsic) {
        ui.horizontal(|ui| {
            for (index, spec) in specs.into_iter().enumerate() {
                if index > 0 {
                    ui.add_space(theme::S3);
                }
                responses.push(control(ui, spec));
            }
        });
    } else {
        let width = ui.available_width();
        for (index, spec) in specs.into_iter().enumerate() {
            if index > 0 {
                gap(ui, theme::S2);
            }
            responses.push(control(ui, spec.min_width(width)));
        }
    }
    responses
}

/// A filter chip. Selected chips carry the accent; the rest are quiet.
/// Sized by its own content through a `Frame`, so it cannot clip.
pub fn chip(ui: &mut egui::Ui, label: &str, selected: bool) -> Response {
    let ink = if selected {
        theme::ACCENT
    } else {
        theme::MUTED
    };
    // A `Frame` reports its width only after its contents are laid out, so a
    // wrapping row never receives the overflow it breaks rows on: the chip
    // shrank to fit whatever was left instead of moving down, and the trailing
    // chips ended up off the edge. The label is measured up front and the row
    // is ended here instead.
    //
    // `available_width` cannot be used for this. In a wrapping layout egui
    // reports the width of a whole row there, not the width left on this one,
    // so every chip appears to fit. `available_rect_before_wrap` is the
    // cursor-relative figure.
    let intrinsic = measure(ui, Role::Caption, label).x + theme::M3 as f32 * 2.0 + 2.0;
    let left_on_row = ui.available_rect_before_wrap().width();
    let started = left_on_row < ui.max_rect().width() - 0.5;
    if intrinsic > left_on_row + 0.5 && started {
        ui.end_row();
    }
    let inner = egui::Frame::new()
        .fill(if selected {
            theme::accent_tint()
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::new(
            1.0,
            if selected {
                Color32::TRANSPARENT
            } else {
                theme::border()
            },
        ))
        .corner_radius(CornerRadius::same(theme::CORNER_SMALL))
        .inner_margin(Margin::symmetric(theme::M3, 5))
        .show(ui, |ui| {
            ui.label(theme::text(Role::Caption, label, ink));
        });
    ui.interact(
        inner.response.rect,
        ui.id().with(("evo-chip", label)),
        Sense::click(),
    )
}

/// The search field: a shallow well in the environment, not a raised card.
/// Search narrows an already-meaningful list, so it stays quiet.
pub fn search_field(
    ui: &mut egui::Ui,
    query: &mut String,
    placeholder: &str,
    salt: &str,
) -> Response {
    let mut response = None;
    theme::well_frame().show(ui, |ui| {
        ui.set_width(ui.available_width().min(320.0));
        ui.horizontal(|ui| {
            let field = ui.add(
                egui::TextEdit::singleline(query)
                    .id_salt(salt)
                    .hint_text(theme::text(Role::Body, placeholder, theme::TERTIARY))
                    .desired_width(ui.available_width() - 20.0)
                    .background_color(Color32::TRANSPARENT)
                    .frame(egui::Frame::NONE),
            );
            if !query.is_empty() {
                let clear = ui.add(
                    egui::Button::new(theme::text(Role::Body, "✕", theme::TERTIARY)).frame(false),
                );
                if clear.clicked() {
                    query.clear();
                }
            }
            response = Some(field);
        });
    });
    response.expect("the search field is always constructed")
}

// ────────────────────────────────────────────────────────────────────────
// Marks
//
// Two different declarations need two structurally different marks, so the
// difference is legible before any label is read: a round mark for the one
// place work continues from, a square mark for the several resources that
// come back with it.
// ────────────────────────────────────────────────────────────────────────

/// The round mark for a single, immediate, superseding choice.
pub fn origin_mark(ui: &mut egui::Ui, id: egui::Id, chosen: bool) -> Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
    let response = ui.interact(rect, id, Sense::click());
    let hover = ui.ctx().animate_bool_with_time(
        id.with("evo-origin-hover"),
        response.hovered(),
        theme::RESPONSE_TOUCH,
    );
    let center = rect.center();
    let painter = ui.painter();
    if chosen {
        painter.circle_filled(center, 8.0, theme::ACCENT);
        painter.circle_filled(center, 3.0, Color32::WHITE);
    } else {
        painter.circle_filled(center, 8.0, shade(theme::hover_wash().a(), hover));
        painter.circle_stroke(
            center,
            7.5,
            Stroke::new(
                1.5,
                mix(
                    Color32::from_rgb(0xB0, 0xAE, 0xAA),
                    Color32::from_rgb(0x7A, 0x78, 0x74),
                    hover,
                ),
            ),
        );
    }
    response
}

/// The square mark for accumulating a set.
///
/// Three states, because a draft is genuinely a third thing: filled when the
/// resource is part of the standing declaration, faint when the user has
/// added it but not yet declared it, and empty otherwise. A draft never
/// looks the same as something Evo has recorded.
pub fn set_mark(ui: &mut egui::Ui, id: egui::Id, member: bool, draft: bool) -> Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
    let response = ui.interact(rect, id, Sense::click());
    let hover = ui.ctx().animate_bool_with_time(
        id.with("evo-set-hover"),
        response.hovered(),
        theme::RESPONSE_TOUCH,
    );
    let box_rect = Rect::from_center_size(rect.center(), Vec2::splat(16.0));
    let radius = CornerRadius::same(5);
    let painter = ui.painter();
    match (member, draft) {
        (true, false) => {
            painter.rect_filled(box_rect, radius, theme::ACCENT);
            painter.text(
                box_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✓",
                theme::strong_font(11.0),
                Color32::WHITE,
            );
        }
        (true, true) => {
            // Pending, not canonical: present but visibly provisional.
            painter.rect_filled(
                box_rect,
                radius,
                Color32::from_rgba_unmultiplied(10, 94, 88, 38),
            );
            painter.rect_stroke(
                box_rect,
                radius,
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(10, 94, 88, 96)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                box_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✓",
                theme::strong_font(11.0),
                Color32::from_rgba_unmultiplied(10, 94, 88, 150),
            );
        }
        _ => {
            painter.rect_filled(box_rect, radius, shade(theme::hover_wash().a(), hover));
            painter.rect_stroke(
                box_rect,
                radius,
                Stroke::new(
                    1.5,
                    mix(
                        Color32::from_rgb(0xB0, 0xAE, 0xAA),
                        Color32::from_rgb(0x7A, 0x78, 0x74),
                        hover,
                    ),
                ),
                egui::StrokeKind::Inside,
            );
        }
    }
    response
}

// ────────────────────────────────────────────────────────────────────────
// Rows and disclosure
// ────────────────────────────────────────────────────────────────────────

/// An interactive row.
///
/// The hover wash extends past the text margins so it reads as a region
/// lighting up rather than a box appearing. The wash for frame *n* comes from
/// the hover state observed on frame *n − 1*, because a background must be
/// painted before the content that sits on it; the transition is animated,
/// so the single-frame handoff is not visible.
pub fn row<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> (Response, R) {
    let was_hovered: bool = ui.data(|data| data.get_temp(id).unwrap_or(false));
    let wash = ui.ctx().animate_bool_with_time(
        id.with("evo-row-wash"),
        was_hovered,
        theme::RESPONSE_TOUCH,
    );
    let inner = egui::Frame::new()
        .fill(shade(theme::hover_wash().a(), wash))
        .corner_radius(CornerRadius::same(theme::CORNER_SMALL))
        .inner_margin(Margin::symmetric(theme::M3, theme::M2))
        .show(ui, |ui| {
            // Full width, so the wash reads as the whole row lighting up.
            ui.set_width(ui.available_width());
            add(ui)
        });
    let response = ui.interact(inner.response.rect, id, Sense::click());
    let hovered = response.hovered();
    if hovered != was_hovered {
        ui.data_mut(|data| data.insert_temp(id, hovered));
    }
    (response, inner.inner)
}

/// A disclosure for material that is present but not currently offered.
///
/// Returns whether the section is open. The state is transient presentation
/// only: collapsing a section never changes what Evo has recorded.
pub fn disclosure(ui: &mut egui::Ui, id: egui::Id, label_text: &str) -> bool {
    let mut open: bool = ui.data(|data| data.get_temp(id).unwrap_or(false));
    let marker = if open { "▾" } else { "▸" };
    let response = ui.add(
        egui::Label::new(theme::text(
            Role::Caption,
            &format!("{marker}  {label_text}"),
            theme::MUTED,
        ))
        .sense(Sense::click()),
    );
    if response.clicked() {
        open = !open;
        ui.data_mut(|data| data.insert_temp(id, open));
    }
    open
}

/// A section: an eyebrow label and its content, separated from what came
/// before by section space. The rhythm belongs to the primitive so no screen
/// has to remember it.
pub fn section<R>(ui: &mut egui::Ui, label_text: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    gap(ui, theme::GAP_SECTION);
    eyebrow(ui, label_text);
    add(ui)
}

// ────────────────────────────────────────────────────────────────────────
// Motion
// ────────────────────────────────────────────────────────────────────────

/// A critically-damped 0→1 transition for a boolean, arriving faster than it
/// leaves and interruptible at any point: reversing mid-transition continues
/// from the value currently on screen rather than jumping.
pub fn transition(ui: &egui::Ui, id: egui::Id, on: bool) -> f32 {
    let response = if on {
        theme::RESPONSE_IN
    } else {
        theme::RESPONSE_OUT
    };
    theme::settle(ui.ctx().animate_bool_with_time(id, on, response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixing_is_endpoint_exact() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(255, 255, 255);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        // Clamped, never extrapolating past an endpoint.
        assert_eq!(mix(a, b, -1.0), a);
        assert_eq!(mix(a, b, 2.0), b);
    }

    #[test]
    fn veiling_an_opaque_color_keeps_its_hue() {
        let color = theme::ACTION_SURFACE;
        let faded = veil(color, 0.5);
        assert_eq!(faded.a(), 128);
        // The hue must survive: a scrim that turns grey would read as dirt.
        //
        // `Color32` stores *premultiplied* components, so the hue lives in the
        // unmultiplied reading and not in the stored bytes — reading `.r()`
        // back out of a half-transparent color yields half the channel, which
        // is the very trap `veil`'s own doc comment warns about. The check is
        // therefore the documented inverse, with the same 3/255 of tolerance
        // egui's own round-trip test allows for the multiply-then-unmultiply.
        let [r, g, b, _] = faded.to_srgba_unmultiplied();
        for (restored, original) in [(r, color.r()), (g, color.g()), (b, color.b())] {
            assert!(
                restored.abs_diff(original) <= 3,
                "veil() shifted a channel: {restored} is not {original}"
            );
        }
        // And it has not drifted toward neutral: this is a warm white, so the
        // channels stay in their warm order rather than converging on grey.
        assert!(r >= g && g >= b, "veil() flattened the hue to {r},{g},{b}");
    }

    #[test]
    fn shading_scales_alpha_and_stays_neutral() {
        let full = shade(20, 1.0);
        assert_eq!(full.a(), 20);
        assert_eq!(shade(20, 0.0).a(), 0);
        assert_eq!(shade(20, 0.5).a(), 10);
    }

    #[test]
    fn more_emphasis_earns_more_padding() {
        // Padding is a function of emphasis, so a primary action is never
        // the same size as a quiet one with the same label.
        let primary = Control::primary("Continue");
        let secondary = Control::secondary("Continue");
        let quiet = Control::quiet("Continue");
        assert!(primary.padding() > secondary.padding());
        assert!(secondary.padding() > quiet.padding());
    }

    #[test]
    fn controls_are_enabled_until_told_otherwise() {
        assert!(Control::primary("Continue").enabled);
        assert!(!Control::primary("Continue").enabled(false).enabled);
    }

    #[test]
    fn a_minimum_width_never_shrinks_a_control() {
        let spec = Control::secondary("Open").min_width(200.0);
        assert_eq!(spec.min_width, 200.0);
    }

    #[test]
    fn emphasis_constructors_agree_with_their_names() {
        assert_eq!(Control::primary("a").emphasis, Emphasis::Primary);
        assert_eq!(Control::secondary("a").emphasis, Emphasis::Secondary);
        assert_eq!(Control::quiet("a").emphasis, Emphasis::Quiet);
    }

    #[test]
    fn a_disabled_primary_control_never_keeps_the_accent_fill() {
        // A disabled action must not look pressable.
        let (fill, _, _) = resting_colors(&Control::primary("Continue").enabled(false));
        assert_ne!(fill, theme::ACCENT);
        let (fill, ink, _) = resting_colors(&Control::primary("Continue"));
        assert_eq!(fill, theme::ACCENT);
        assert_eq!(ink, Color32::WHITE);
    }

    #[test]
    fn the_quiet_emphasis_has_no_resting_fill_or_border() {
        let (fill, _, stroke) = resting_colors(&Control::quiet("Not now"));
        assert_eq!(fill, Color32::TRANSPARENT);
        assert_eq!(stroke.width, 0.0);
    }
}
