//! The Evo visual system — tokens only.
//!
//! Presentation only. This module owns color, typography, spacing, radii,
//! shadows and the *material* definitions. Reusable layout and control
//! primitives live in [`crate::ui`]; screen composition lives in the screen
//! modules. Nothing here is canonical.
//!
//! # The material hierarchy is the design
//!
//! Evo has six semantic surfaces and each one is a visually distinct
//! material. They are deliberately *not* six variations of a white card:
//!
//! ```text
//! ENVIRONMENT   the window itself — warm neutral, top-lit. Most content
//!               sits directly on it with no container at all.
//! RAISED        the one primary object on a screen — lighter than the
//!               environment, soft downward shadow, generous padding.
//! RECESSED      contextual/secondary regions (the continuation panel) —
//!               *darker* than the environment, inset top edge, no shadow.
//!               Weight, not brightness, is what marks it as supporting.
//! QUIET         historical / related material — no fill at all. A hairline
//!               rule and reduced presence. History never gets a card.
//! NOTICE        transient system messages — a tinted band with a leading
//!               edge bar. Never shaped like content.
//! ACTION        the pinned Continue bar and primary controls — the most
//!               opaque material, lifted by an upward shadow and separated
//!               from content by a soft scrim rather than a hard rule.
//! ```
//!
//! Rule: at most one RAISED surface per screen, and RECESSED is never
//! stacked inside RAISED. Two translucent layers are never stacked.
//!
//! # Renderer limitation, stated rather than faked
//!
//! The current egui renderer cannot sample the desktop behind the window, so
//! genuine backdrop blur and live environment translucency are unavailable.
//! Rather than paint a decorative gradient and call it blur, the environment
//! is a warm neutral with a ~7-luminance-unit vertical wash that reads as
//! *light from above*. It is a lighting cue, not a translucency claim, and it
//! is painted in discrete bands because the renderer has no gradient
//! primitive. Every other layer earns its depth from opacity, shadow,
//! elevation, scale and spacing — all of which the renderer does support.

use egui::epaint::text::VariationCoords;
use egui::{
    Color32, CornerRadius, CursorIcon, FontData, FontDefinitions, FontFamily, FontId, FontTweak,
    Margin, RichText, Shadow, Stroke, Vec2,
};

// ────────────────────────────────────────────────────────────────────────
// Environment
// ────────────────────────────────────────────────────────────────────────

/// The environment material's base tone. Used as the window clear color so
/// the very first painted frame is already the environment.
pub const BG: Color32 = Color32::from_rgb(0xE9, 0xE6, 0xE1);
/// The environment at the top of the window (light source above).
pub const ENV_TOP: Color32 = Color32::from_rgb(0xEC, 0xE9, 0xE4);
/// The environment at the bottom of the window.
pub const ENV_BOTTOM: Color32 = Color32::from_rgb(0xE5, 0xE1, 0xDC);

/// The raised surface: the single primary object on a screen.
pub const RAISED: Color32 = Color32::from_rgb(0xFD, 0xFC, 0xFB);
/// The recessed surface: contextual and supporting regions. Darker than the
/// environment on purpose — supporting material recedes by weight.
pub const RECESSED: Color32 = Color32::from_rgb(0xE1, 0xDD, 0xD7);
/// A recessed well used for small inputs (search) — one step in, not a card.
pub const WELL: Color32 = Color32::from_rgb(0xE3, 0xDF, 0xD9);
/// The action material (Continue bar, sheets): near-opaque, warm white.
pub const ACTION_SURFACE: Color32 = Color32::from_rgb(0xFB, 0xFA, 0xF8);

/// Legacy alias retained for the small-input well.
pub fn surface_alt() -> Color32 {
    WELL
}

// ────────────────────────────────────────────────────────────────────────
// Ink — contrast-checked against the environment (≥ 4.5:1 for every tone
// that carries information).
// ────────────────────────────────────────────────────────────────────────

/// Primary text.
pub const TEXT: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1D);
/// Secondary text — explanations, provenance, supporting prose.
pub const MUTED: Color32 = Color32::from_rgb(0x56, 0x56, 0x5B);
/// Tertiary text — eyebrow labels and field names.
pub const TERTIARY: Color32 = Color32::from_rgb(0x63, 0x63, 0x6A);
/// Receded — decoration only (rules, empty marks). Never information.
pub const RECEDED: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x90);

/// Evo's accent: deep teal, used sparingly — the primary action, the mark of
/// a declared choice, focus. Evo must never look covered in colored buttons.
pub const ACCENT: Color32 = Color32::from_rgb(0x0A, 0x5E, 0x58);
/// Accent under the pointer.
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x0C, 0x6B, 0x64);
/// Accent while pressed.
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0x08, 0x52, 0x4D);
/// A faint accent tint for marked rows and notices.
pub fn accent_tint() -> Color32 {
    Color32::from_rgba_unmultiplied(10, 94, 88, 20)
}

/// READY / recorded / running.
pub const SUCCESS: Color32 = Color32::from_rgb(0x0F, 0x6B, 0x29);
pub fn success_tint() -> Color32 {
    Color32::from_rgba_unmultiplied(15, 107, 41, 22)
}
/// UNAVAILABLE / AMBIGUOUS / permission required.
pub const WARNING: Color32 = Color32::from_rgb(0x9A, 0x44, 0x00);
pub fn warning_tint() -> Color32 {
    Color32::from_rgba_unmultiplied(154, 68, 0, 22)
}
/// Rejected / failed. Distinct from the accent so a rejection can never read
/// as a success.
pub const ERROR: Color32 = Color32::from_rgb(0xC1, 0x12, 0x1F);
pub fn error_tint() -> Color32 {
    Color32::from_rgba_unmultiplied(193, 18, 31, 22)
}
/// UNSUPPORTED / UNKNOWN. A capability Evo does not have, or a fact its
/// record does not contain, is neither a success nor a failure — it must not
/// borrow the color of either.
pub fn neutral_tint() -> Color32 {
    Color32::from_rgba_unmultiplied(0x63, 0x63, 0x6A, 22)
}

/// The text color for a status line: success when the daemon accepted the
/// user's action, error when it rejected it.
pub fn status_color(accepted: bool) -> Color32 {
    if accepted { SUCCESS } else { ERROR }
}

/// Hairline rule.
pub fn border() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 18)
}
/// A stronger hairline, for hover and focus.
pub fn border_strong() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 34)
}
/// The inset top edge of a recessed surface — the shade a lowered plane
/// catches from the light above.
pub fn inset_edge() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 12)
}
/// The wash a row takes under the pointer. Extends past the text margins so
/// hover reads as a region, not a box.
pub fn hover_wash() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 12)
}

// ────────────────────────────────────────────────────────────────────────
// Spacing — one scale, 4px base. Every gap in the product comes from here.
// Layout problems are solved by choosing a step, never by a magic number.
// ────────────────────────────────────────────────────────────────────────

/// 4px — within a line (mark to label).
pub const S1: f32 = 4.0;
/// 8px — between tightly related lines.
pub const S2: f32 = 8.0;
/// 12px — inside a group.
pub const S3: f32 = 12.0;
/// 16px — between groups.
pub const S4: f32 = 16.0;
/// 24px — between blocks.
pub const S5: f32 = 24.0;
/// 32px — between sections; page margin.
pub const S6: f32 = 32.0;
/// 48px — between major regions.
pub const S7: f32 = 48.0;
/// 64px — the breathing room above a screen's first line.
pub const S8: f32 = 64.0;

/// Page margin.
pub const PAGE_PAD: f32 = S6;
/// The primary column measure.
pub const CONTENT_WIDTH: f32 = 700.0;
/// The measure prose wraps within, so long explanations stay readable even
/// when the column is wider.
pub const PROSE_WIDTH: f32 = 600.0;
/// Below this total content width the detail view stacks instead of
/// splitting. [`detail_split`] cannot honour both its minimum main measure
/// (360) and its minimum contextual measure (260) plus the gap below 652, so
/// the breakpoint sits above that floor with room to spare — and below the
/// content width of the default window, which is wide enough for both.
pub const STACK_BREAKPOINT: f32 = 760.0;
/// Minimum window width the layout is designed to degrade to.
pub const MIN_WINDOW_WIDTH: f32 = 420.0;
/// The pinned action strip's resting height.
pub const ACTION_BAR_HEIGHT: f32 = 56.0;
/// The height of the top chrome strip.
pub const HEADER_HEIGHT: f32 = 56.0;
/// Standard control height.
pub const CONTROL_HEIGHT: f32 = 34.0;
/// The padding inside a raised surface.
pub const RAISED_PAD: i8 = 28;
/// The padding inside a recessed surface.
pub const RECESSED_PAD: i8 = 22;

// The same scale expressed in `Margin` units, because egui margins are `i8`.
// These are mirrors, not new numbers: a test asserts they match `S1`–`S6`.
/// 4px margin.
pub const M1: i8 = 4;
/// 8px margin.
pub const M2: i8 = 8;
/// 12px margin.
pub const M3: i8 = 12;
/// 16px margin.
pub const M4: i8 = 16;
/// 24px margin.
pub const M5: i8 = 24;
/// 32px margin.
pub const M6: i8 = 32;

// Semantic gap aliases, all snapped to the scale above.
/// Between sections.
pub const GAP_SECTION: f32 = S6;
/// Between entries.
pub const GAP_ENTRY: f32 = S5;
/// Between subsections.
pub const GAP_SUBSECTION: f32 = S4;
/// Inside a group.
pub const GAP_INNER: f32 = S3;
/// Between lines.
pub const GAP_LINE: f32 = S2;
/// Within a line.
pub const GAP_TIGHT: f32 = S1;

// ────────────────────────────────────────────────────────────────────────
// Radii
// ────────────────────────────────────────────────────────────────────────

/// Window corner radius.
pub const CORNER_WINDOW: u8 = 12;
/// Large surfaces (raised, recessed regions).
pub const CORNER_SURFACE: u8 = 16;
/// Sheets.
pub const CORNER_SHEET: u8 = 18;
/// Controls.
pub const CORNER_CONTROL: u8 = 10;
/// Chips, notices, marks.
pub const CORNER_SMALL: u8 = 8;

// ────────────────────────────────────────────────────────────────────────
// Shadows — only RAISED, ACTION and sheets cast them. A recessed plane
// cannot cast a shadow, and history never lifts off the page.
// ────────────────────────────────────────────────────────────────────────

/// The primary raised object at rest.
pub fn raised_shadow() -> Shadow {
    Shadow {
        offset: [0, 3],
        blur: 18,
        spread: 0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, 24),
    }
}
/// The primary raised object under the pointer — it rises toward the finger.
pub fn raised_shadow_hover() -> Shadow {
    Shadow {
        offset: [0, 6],
        blur: 26,
        spread: 0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, 34),
    }
}
/// The pinned action strip: lifted, so its shadow falls *upward* onto the
/// content it floats above.
pub fn action_shadow() -> Shadow {
    Shadow {
        offset: [0, -4],
        blur: 20,
        spread: 0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, 20),
    }
}
/// A sheet: the furthest-forward material, so the deepest shadow.
pub fn sheet_shadow() -> Shadow {
    Shadow {
        offset: [0, 14],
        blur: 44,
        spread: 0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, 44),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Typography
//
// One typeface (the system UI font) with a fixed eight-role ramp. Each role
// fixes size, weight, tracking and leading *together* — hierarchy is built
// from the set, never from size alone.
//
// Tracking is size-specific, as it must be: large text reads too loose at
// its natural spacing and is tightened; small text and all-caps eyebrows are
// opened up. Leading moves inversely to size: tight on display, generous on
// body prose.
//
// egui's `.strong()` only changes color, so real weight comes from dedicated
// variable-font instances (wght 500/600/700) registered as named families.
// ────────────────────────────────────────────────────────────────────────

const FACE_REGULAR: &str = "evo-sf";
const FAMILY_MEDIUM: &str = "evo-medium";
const FAMILY_SEMIBOLD: &str = "evo-semibold";
const FAMILY_BOLD: &str = "evo-bold";

/// The eight text roles. Every piece of text in Evo is one of these.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// 30 Bold — the one sentence a screen exists to say.
    Display,
    /// 22 Semibold — a screen or region heading.
    Headline,
    /// 16.5 Semibold — a group title.
    Title,
    /// 15 Regular — the sentence under a heading.
    Lede,
    /// 13.5 Regular — body prose.
    Body,
    /// 13.5 Semibold — a body line that carries the weight of the group.
    BodyStrong,
    /// 12 Regular — provenance, reasons, counts.
    Caption,
    /// 10.5 Medium, opened up — an all-caps region label.
    Eyebrow,
}

impl Role {
    /// Point size.
    pub fn size(self) -> f32 {
        match self {
            Role::Display => 30.0,
            Role::Headline => 22.0,
            Role::Title => 16.5,
            Role::Lede => 15.0,
            Role::Body => 13.5,
            Role::BodyStrong => 13.5,
            Role::Caption => 12.0,
            Role::Eyebrow => 10.5,
        }
    }

    /// Extra letter spacing, in points. Negative tightens.
    pub fn tracking(self) -> f32 {
        match self {
            Role::Display => -0.6,
            Role::Headline => -0.35,
            Role::Title => -0.15,
            Role::Lede => -0.05,
            Role::Body | Role::BodyStrong => 0.0,
            Role::Caption => 0.1,
            Role::Eyebrow => 0.9,
        }
    }

    /// Line height as a multiple of size. Inversely related to size.
    pub fn leading(self) -> f32 {
        match self {
            Role::Display => 1.08,
            Role::Headline => 1.16,
            Role::Title => 1.26,
            Role::Lede => 1.44,
            Role::Body => 1.48,
            Role::BodyStrong => 1.44,
            Role::Caption => 1.42,
            Role::Eyebrow => 1.2,
        }
    }

    /// The weighted face this role is set in.
    pub fn font(self) -> FontId {
        match self {
            Role::Display => bold_font(self.size()),
            Role::Headline | Role::Title | Role::BodyStrong => title_font(self.size()),
            Role::Lede | Role::Body | Role::Caption => body_font(self.size()),
            Role::Eyebrow => tiny_font(self.size()),
        }
    }
}

/// Text in one of the eight roles, with that role's tracking and leading
/// applied. This is the only way text should be produced.
pub fn text(role: Role, value: impl Into<String>, color: Color32) -> RichText {
    RichText::new(value)
        .font(role.font())
        .color(color)
        .extra_letter_spacing(role.tracking())
        .line_height(Some(role.size() * role.leading()))
}

/// An all-caps region label. Callers pass normal text; the role's tracking
/// makes the caps legible.
pub fn eyebrow(value: &str, color: Color32) -> RichText {
    text(Role::Eyebrow, value.to_uppercase(), color)
}

/// Display voice — Bold (700).
pub fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY_BOLD.into()))
}
/// Title voice — Semibold (600).
pub fn title_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY_SEMIBOLD.into()))
}
/// Body voice — Regular.
pub fn body_font(size: f32) -> FontId {
    FontId::proportional(size)
}
/// Body-strong voice — Semibold (600).
pub fn strong_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY_SEMIBOLD.into()))
}
/// Eyebrow voice — Medium (500).
pub fn tiny_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY_MEDIUM.into()))
}
/// Bold voice — Bold (700).
pub fn bold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY_BOLD.into()))
}

/// Free-size display text. Prefer [`text`] with a [`Role`].
pub fn display(value: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(value)
        .font(display_font(size))
        .color(color)
        .extra_letter_spacing(Role::Display.tracking())
        .line_height(Some(size * Role::Display.leading()))
}
/// Free-size title text. Prefer [`text`] with a [`Role`].
pub fn title(value: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(value)
        .font(title_font(size))
        .color(color)
        .extra_letter_spacing(Role::Title.tracking())
        .line_height(Some(size * Role::Title.leading()))
}
/// Free-size body text. Prefer [`text`] with a [`Role`].
pub fn body(value: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(value)
        .font(body_font(size))
        .color(color)
        .line_height(Some(size * Role::Body.leading()))
}
/// Free-size body-strong text. Prefer [`text`] with a [`Role`].
pub fn strong(value: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(value)
        .font(strong_font(size))
        .color(color)
        .line_height(Some(size * Role::BodyStrong.leading()))
}
/// Free-size eyebrow text. Prefer [`eyebrow`].
pub fn tiny(value: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(value)
        .font(tiny_font(size))
        .color(color)
        .extra_letter_spacing(Role::Eyebrow.tracking())
}

// ────────────────────────────────────────────────────────────────────────
// Motion
//
// The renderer gives us a linear 0→1 progress; the felt behaviour comes from
// the curve applied to it. `settle` is critically damped — it arrives and
// stops, with no overshoot — which is the right default for anything the
// user did not throw. Response is ~0.35s for arrivals and ~0.2s for
// departures, so leaving is never slower than coming.
// ────────────────────────────────────────────────────────────────────────

/// Response time (seconds) for an arriving transition.
pub const RESPONSE_IN: f32 = 0.35;
/// Response time (seconds) for a departing transition — faster than arrival.
pub const RESPONSE_OUT: f32 = 0.2;
/// Response time (seconds) for hover and press feedback.
pub const RESPONSE_TOUCH: f32 = 0.12;

/// A critically damped arrival curve: fast at first, decelerating into the
/// target, no overshoot.
pub fn settle(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

// ────────────────────────────────────────────────────────────────────────
// Materials
// ────────────────────────────────────────────────────────────────────────

/// The raised surface frame: the single primary object on a screen.
pub fn raised_frame(hovered: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(RAISED)
        .stroke(Stroke::new(
            1.0,
            if hovered { border_strong() } else { border() },
        ))
        .corner_radius(CornerRadius::same(CORNER_SURFACE))
        .inner_margin(Margin::same(RAISED_PAD))
        .shadow(if hovered {
            raised_shadow_hover()
        } else {
            raised_shadow()
        })
}

/// The recessed surface frame: contextual and supporting regions. Darker
/// than the environment, no shadow — a lowered plane cannot cast one.
pub fn recessed_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(RECESSED)
        .stroke(Stroke::new(1.0, inset_edge()))
        .corner_radius(CornerRadius::same(CORNER_SURFACE))
        .inner_margin(Margin::same(RECESSED_PAD))
}

/// A small recessed well, for a single input.
pub fn well_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(WELL)
        .stroke(Stroke::new(1.0, inset_edge()))
        .corner_radius(CornerRadius::same(CORNER_CONTROL))
        .inner_margin(Margin::symmetric(12, 7))
}

/// The action-strip frame. Its shadow lifts upward because the strip floats
/// above the content rather than sitting under it.
pub fn action_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(ACTION_SURFACE)
        .shadow(action_shadow())
}

/// The sheet frame: the furthest-forward material.
pub fn sheet_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(ACTION_SURFACE)
        .stroke(Stroke::new(1.0, border()))
        .corner_radius(CornerRadius::same(CORNER_SHEET))
        .inner_margin(Margin::same(28))
        .shadow(sheet_shadow())
}

/// The tint and edge color for a transient notice of the given severity.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    /// Neutral information about Evo's own state.
    Info,
    /// Something is working, recorded, or granted.
    Good,
    /// Something is limited or needs the user's attention.
    Caution,
    /// Something failed or was refused.
    Bad,
    /// Neither: Evo lacks the capability, or its record lacks the fact.
    Neutral,
}

impl Severity {
    /// The notice band's fill.
    pub fn tint(self) -> Color32 {
        match self {
            Severity::Info => accent_tint(),
            Severity::Good => success_tint(),
            Severity::Caution => warning_tint(),
            Severity::Bad => error_tint(),
            Severity::Neutral => neutral_tint(),
        }
    }

    /// The notice's leading edge bar and status word color.
    pub fn ink(self) -> Color32 {
        match self {
            Severity::Info => ACCENT,
            Severity::Good => SUCCESS,
            Severity::Caution => WARNING,
            Severity::Bad => ERROR,
            Severity::Neutral => TERTIARY,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────
// Geometry
// ────────────────────────────────────────────────────────────────────────

/// The detail view's two-column split: `(main, gap, side)`. The main column
/// dominates; the contextual column is a supporting measure that never grows
/// into a navigation rail.
pub fn detail_split(total: f32) -> (f32, f32, f32) {
    let gap = S6;
    let side = ((total - gap) * 0.34).clamp(260.0, 330.0);
    let main = (total - gap - side).max(360.0);
    (main, gap, side)
}

// ────────────────────────────────────────────────────────────────────────
// Context installation
// ────────────────────────────────────────────────────────────────────────

/// Installs fonts, visuals and base styling on the egui context.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());

    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.window_corner_radius = CornerRadius::same(CORNER_WINDOW);
    visuals.window_stroke = Stroke::new(1.0, border());
    visuals.hyperlink_color = ACCENT;
    visuals.extreme_bg_color = WELL;
    visuals.faint_bg_color = WELL;
    visuals.selection.bg_fill = accent_tint();
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.interact_cursor = Some(CursorIcon::PointingHand);

    // Built-in widgets stay quiet: Evo's controls are painted by
    // `crate::ui`, and anything egui draws by itself must not compete. The
    // existing widget visuals are edited in place rather than replaced so
    // this never depends on the shape of egui's widget-visual struct.
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = Color32::TRANSPARENT;
        widget.weak_bg_fill = Color32::TRANSPARENT;
        widget.bg_stroke = Stroke::NONE;
        widget.corner_radius = CornerRadius::same(CORNER_CONTROL);
        widget.fg_stroke = Stroke::new(1.0, TEXT);
        widget.expansion = 0.0;
    }
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, ACCENT);
    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        // Every gap in Evo is explicit and comes from the spacing scale.
        // Implicit item spacing is what makes vertical rhythm drift, so it
        // is zero and the layout primitives own the rhythm instead.
        style.spacing.item_spacing = Vec2::new(0.0, 0.0);
        style.spacing.button_padding = Vec2::new(0.0, 0.0);
        style.spacing.interact_size = Vec2::new(0.0, 0.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, body_font(Role::Body.size()));
        style
            .text_styles
            .insert(egui::TextStyle::Small, body_font(Role::Caption.size()));
        style
            .text_styles
            .insert(egui::TextStyle::Button, title_font(Role::Body.size()));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, title_font(Role::Title.size()));
    });
}

/// Loads the platform UI face so Evo reads native. The face ships its own
/// optical sizing and legibility tuning, which is why it is preferred to any
/// bundled alternative.
fn load_system_face() -> Option<Vec<u8>> {
    const CANDIDATES: &[&str] = &["/System/Library/Fonts/SFNS.ttf", "/Library/Fonts/SFNS.ttf"];
    CANDIDATES.iter().find_map(|path| std::fs::read(path).ok())
}

/// The font set Evo asks egui for: the platform face at three real weights,
/// with egui's own proportional stack behind each of them.
///
/// Separate from [`install`] so the binding invariant below can be tested
/// without a live context — the named families are registered whether or not
/// the platform face could be read. A `FontFamily::Name` bound to no font data
/// resolves to nothing, and since most text in Evo is set in a weighted role,
/// a Mac whose system face is unreadable would otherwise show blank headings
/// on every screen. Losing the native face should cost nativeness, not the
/// interface.
fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let fallback: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    let system_face = load_system_face();
    if let Some(bytes) = &system_face {
        fonts.font_data.insert(
            FACE_REGULAR.to_string(),
            std::sync::Arc::new(FontData::from_owned(bytes.clone())),
        );
        // Real weight from the variable `wght` axis, registered as named
        // families so a role can ask for the face it needs.
        for (name, wght) in [
            (FAMILY_MEDIUM, 500.0),
            (FAMILY_SEMIBOLD, 600.0),
            (FAMILY_BOLD, 700.0),
        ] {
            let data = FontData::from_owned(bytes.clone()).tweak(FontTweak {
                coords: VariationCoords::new([(b"wght", wght)]),
                ..Default::default()
            });
            fonts
                .font_data
                .insert(name.to_string(), std::sync::Arc::new(data));
        }
        if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
            family.insert(0, FACE_REGULAR.to_string());
        }
    }

    for name in [FAMILY_MEDIUM, FAMILY_SEMIBOLD, FAMILY_BOLD] {
        let mut chain: Vec<String> = Vec::new();
        if system_face.is_some() {
            chain.push(name.to_string());
            chain.push(FACE_REGULAR.to_string());
        }
        chain.extend(fallback.iter().cloned());
        fonts.families.insert(FontFamily::Name(name.into()), chain);
    }
    fonts
}

// ────────────────────────────────────────────────────────────────────────
// Tests — the tokens' semantics, not their literals.
// ────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_weighted_family_resolves_to_at_least_one_real_font() {
        // This test passes on a machine with the platform face and on one
        // without it — that is the entire point. An unbound named family
        // renders nothing, and every heading, title and eyebrow in Evo asks
        // for one, so a missing binding would empty the interface rather than
        // merely making it look less native.
        let fonts = font_definitions();
        for name in [FAMILY_MEDIUM, FAMILY_SEMIBOLD, FAMILY_BOLD] {
            let family = FontFamily::Name(name.into());
            let chain = fonts
                .families
                .get(&family)
                .unwrap_or_else(|| panic!("{name} is not registered as a family"));
            assert!(!chain.is_empty(), "{name} is bound to no font data");
            for face in chain {
                assert!(
                    fonts.font_data.contains_key(face),
                    "{name} falls back to {face}, which has no font data"
                );
            }
        }
    }

    #[test]
    fn every_text_role_asks_for_a_registered_family() {
        // A role naming a family nobody registered is the same failure as an
        // unbound family, arrived at from the other side.
        let fonts = font_definitions();
        for role in [
            Role::Display,
            Role::Headline,
            Role::Title,
            Role::Lede,
            Role::Body,
            Role::BodyStrong,
            Role::Caption,
            Role::Eyebrow,
        ] {
            let family = role.font().family;
            assert!(
                fonts.families.contains_key(&family),
                "{role:?} asks for {family:?}, which is not registered"
            );
        }
    }

    #[test]
    fn accepted_status_uses_the_success_green() {
        assert_eq!(status_color(true), SUCCESS);
    }

    #[test]
    fn rejected_status_uses_the_error_color_never_the_success_green() {
        assert_eq!(status_color(false), ERROR);
        assert_ne!(status_color(false), SUCCESS);
        assert_ne!(status_color(false), ACCENT);
    }

    #[test]
    fn display_font_is_the_bold_face_not_the_plain_body_face() {
        let display = display_font(20.0);
        let body = body_font(20.0);
        assert_ne!(display.family, body.family);
        assert_eq!(display.size, body.size);
    }

    #[test]
    fn title_font_is_the_semibold_face_and_strong_shares_it() {
        assert_eq!(title_font(14.0).family, strong_font(14.0).family);
        assert_ne!(title_font(14.0).family, display_font(14.0).family);
    }

    #[test]
    fn success_and_error_are_distinct_from_the_accent() {
        assert_ne!(SUCCESS, ACCENT);
        assert_ne!(ERROR, ACCENT);
        assert_ne!(SUCCESS, ERROR);
    }

    #[test]
    fn only_lifted_materials_cast_shadows() {
        // The action strip floats above the content, so its shadow falls
        // upward.
        assert!(action_shadow().offset[1] < 0);
        // A raised object casts a soft shadow downward.
        assert!(raised_shadow().offset[1] > 0);
        // The furthest-forward material casts the deepest shadow.
        assert!(raised_shadow().blur < sheet_shadow().blur);
        // A recessed plane is a lowered plane: it has no shadow at all.
        let recessed = recessed_frame().shadow;
        assert_eq!(recessed.blur, 0);
        assert_eq!(recessed.offset, [0, 0]);
    }

    #[test]
    fn the_margin_constants_mirror_the_spacing_scale() {
        // Margins are the same scale in egui's `i8` units — never a second,
        // divergent set of numbers.
        assert_eq!(M1 as f32, S1);
        assert_eq!(M2 as f32, S2);
        assert_eq!(M3 as f32, S3);
        assert_eq!(M4 as f32, S4);
        assert_eq!(M5 as f32, S5);
        assert_eq!(M6 as f32, S6);
        assert_eq!(RAISED_PAD as f32 % 4.0, 0.0);
        assert_eq!(RECESSED_PAD as f32 % 2.0, 0.0);
    }

    #[test]
    fn spacing_uses_the_four_pixel_base() {
        for step in [S1, S2, S3, S4, S5, S6, S7, S8] {
            assert_eq!(step % 4.0, 0.0);
        }
        assert_eq!(PAGE_PAD % 4.0, 0.0);
        assert_eq!(GAP_SECTION % 4.0, 0.0);
        assert_eq!(ACTION_BAR_HEIGHT % 4.0, 0.0);
        assert_eq!(HEADER_HEIGHT % 4.0, 0.0);
    }

    #[test]
    fn the_spacing_scale_is_strictly_increasing() {
        let scale = [S1, S2, S3, S4, S5, S6, S7, S8];
        for pair in scale.windows(2) {
            assert!(pair[0] < pair[1]);
        }
    }

    #[test]
    fn the_recessed_surface_is_darker_than_the_environment() {
        // Supporting material recedes by weight, not by being another white
        // card. The contextual surface must be darker than the environment
        // it sits in, and the one raised object must be lighter.
        let luma = |color: Color32| {
            0.2126 * color.r() as f32 + 0.7152 * color.g() as f32 + 0.0722 * color.b() as f32
        };
        assert!(luma(RECESSED) < luma(ENV_BOTTOM));
        assert!(luma(RAISED) > luma(ENV_TOP));
    }

    #[test]
    fn the_environment_is_lit_from_above() {
        let luma = |color: Color32| {
            0.2126 * color.r() as f32 + 0.7152 * color.g() as f32 + 0.0722 * color.b() as f32
        };
        assert!(luma(ENV_TOP) > luma(ENV_BOTTOM));
    }

    #[test]
    fn tracking_is_size_specific_not_one_value_for_every_size() {
        // Large text is tightened; small and all-caps text is opened up.
        assert!(Role::Display.tracking() < 0.0);
        assert!(Role::Eyebrow.tracking() > 0.0);
        assert_eq!(Role::Body.tracking(), 0.0);
    }

    #[test]
    fn leading_moves_inversely_to_size() {
        assert!(Role::Display.size() > Role::Body.size());
        assert!(Role::Display.leading() < Role::Body.leading());
    }

    #[test]
    fn every_role_has_a_distinct_size_or_weight() {
        // Hierarchy is built from size *and* weight together, so two roles
        // may share a size only if their faces differ.
        let roles = [
            Role::Display,
            Role::Headline,
            Role::Title,
            Role::Lede,
            Role::Body,
            Role::BodyStrong,
            Role::Caption,
            Role::Eyebrow,
        ];
        for (index, role) in roles.iter().enumerate() {
            for other in &roles[index + 1..] {
                assert!(
                    role.size() != other.size() || role.font().family != other.font().family,
                    "{role:?} and {other:?} are visually identical"
                );
            }
        }
    }

    #[test]
    fn settle_is_critically_damped_and_never_overshoots() {
        assert_eq!(settle(0.0), 0.0);
        assert_eq!(settle(1.0), 1.0);
        // Monotone, decelerating, and always inside [0, 1] — no bounce.
        let mut previous = 0.0;
        let mut previous_step = f32::MAX;
        for index in 1..=20 {
            let t = index as f32 / 20.0;
            let value = settle(t);
            assert!(value >= previous);
            assert!(value <= 1.0);
            let step = value - previous;
            assert!(step <= previous_step + f32::EPSILON);
            previous_step = step;
            previous = value;
        }
    }

    #[test]
    fn leaving_is_never_slower_than_arriving() {
        assert!(RESPONSE_OUT < RESPONSE_IN);
        assert!(RESPONSE_TOUCH < RESPONSE_OUT);
    }

    #[test]
    fn every_severity_pairs_a_tint_with_a_distinct_ink() {
        let severities = [
            Severity::Info,
            Severity::Good,
            Severity::Caution,
            Severity::Bad,
            Severity::Neutral,
        ];
        for (index, severity) in severities.iter().enumerate() {
            // A tint alone never conveys state: the ink is what the status
            // word is set in, and it must differ per severity.
            assert_ne!(severity.tint(), severity.ink());
            for other in &severities[index + 1..] {
                assert_ne!(severity.ink(), other.ink());
            }
        }
    }
}
