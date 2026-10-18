//! The macOS notch surface: a panel pinned at the notch (DynamicNotch's
//! geometry), rendering the [`crate::notch::NotchEngine`] content.
//! Concept-level port from jackson-storm's DynamicNotch (GPL-3.0); the
//! raw-C bridge pattern follows the repo's own AppKit capture bridges
//! precedent. No Swift copied.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::notch::{ContentId, NotchEngine, OutcomeState, SurfaceDecision, decide_surface};

/// How the surface presents this session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotchMode {
    /// Skip the notch surface entirely.
    Hidden,
    /// Auto: own the built-in physical notch; ordinary windows are irrelevant.
    Auto,
    /// Always render the floating card (never touch the strip).
    Float,
    /// Always claim the strip.
    Own,
}

impl NotchMode {
    fn decision(
        self,
        hardware_notch: Option<(f64, f64)>,
        foreign_notch_windows: usize,
    ) -> Option<SurfaceDecision> {
        hardware_notch?;
        match self {
            Self::Hidden => None,
            Self::Float => Some(SurfaceDecision::Float),
            Self::Auto => Some(decide_surface(foreign_notch_windows)),
            Self::Own => Some(SurfaceDecision::Own),
        }
    }

    fn from_env() -> Self {
        match std::env::var("EVO_NOTCH_MODE")
            .ok()
            .map(|v| v.to_ascii_lowercase())
            .as_deref()
        {
            Some("hidden") => Self::Hidden,
            Some("float") => Self::Float,
            Some("own") => Self::Own,
            _ => Self::Auto,
        }
    }
}

// ─── Channel: driver events in, UI decisions out ────────────────────────────

/// What the surface thread tells the main process (click actions).
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceAction {
    /// The person clicked an option chip: dispatch via the voice socket
    /// (same path as an AmbiguityPanel pick).
    Choose(String),
    /// The focused voice choice, including an occurrence capability when the
    /// daemon asked the person to choose among witnessed surfaces.
    ChooseCandidate(evo_voice::VoiceCandidate),
    /// Dismiss/collapse the surface (swipe or Escape).
    Dismiss,
    /// Open the main Evo window (rack deep-link). Retained for
    /// compatibility: the notch's listening chip now opens the browse
    /// carousel instead, and the agent owns that round-trip.
    OpenMain,
    /// Show the works-browse carousel (the listening card's Browse chip).
    /// The agent owns the daemon round-trip; the surface only reports intent.
    Browse,
    /// Resume the most recent work (the listening card's Restore chip).
    /// The agent picks the first work from the daemon's listing.
    ContinueRecent,
    /// One carousel step (±1) from a chip click or a drag swipe.
    ChoiceStep(isize),
}

/// Everything the surface thread consumes.
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceInput {
    /// One driver event (WakeGesture/Listening/HeardCommand/Outcome).
    Voice(evo_voice::VoiceEvent),
    /// The rack count (n works) for the idle live activity.
    Rack(usize),
    /// A works listing to browse: opens the browse carousel. An empty
    /// listing dismisses any open browse (the engine decides).
    Works(Vec<crate::notch::BrowseEntry>),
    /// Set/clear a scripted layout override for verification.
    VerifyCase(Option<VerifyCase>),
    /// One carousel step from a chip click or a drag swipe.
    ChoiceStep(isize),
    /// Collapse the current surface and return to hidden idle.
    Dismiss,
    /// Chip hover state change (index, or None to clear).
    Hover(Option<usize>),
    /// Never consume this: parked channel token.
    Never,
    /// Shut down.
    Quit,
}

/// The three layouts the demo/verify path exercises, painted exactly as
/// driver events would paint them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyCase {
    /// Legacy verification selector. Idle is intentionally empty now; the
    /// host orders its transparent panel out instead of painting a mark.
    IdleGlyph,
    /// Expanded listening surface with a partial transcript.
    Listening,
    /// The outcome card.
    Outcome,
    /// The candidate-choice carousel.
    Choice,
    /// The works-browse carousel (the rack as a resumable list).
    Browse,
}

/// The handles the app keeps after spawning the surface thread.
pub struct NotchSurface {
    pub input: mpsc::Sender<SurfaceInput>,
    pub actions: mpsc::Receiver<SurfaceAction>,
    /// What the probe decided when the thread started.
    pub decision: SurfaceDecision,
}

impl NotchSurface {
    /// Sends one input; a dead surface thread is quietly ignored (the
    /// notch is auxiliary, never load-bearing).
    pub fn send(&self, input: SurfaceInput) {
        let _ = self.input.send(input);
    }
}

/// One restorable candidate the choice carousel offers, with the
/// action a click on its chip dispatches. Plain data — the window
/// layer never talks to the daemon itself.
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceCandidate {
    /// The stable work id (the daemon's CHOOSE key).
    pub work_id: String,
    /// What the work is about, from its own witnessed material.
    pub title: String,
    /// The daemon-provided work or occurrence selection capability.
    pub selection: evo_voice::VoiceSelection,
}

/// What a renderer would draw each frame: the engine's decisions as
/// plain data. The window layer is a dumb consumer of this state;
/// decisions live in the tested engine.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderState {
    /// Legacy idle marker. Normal idle never paints this field.
    pub idle_glyph: bool,
    /// True when a rack live activity is visible.
    pub rack: bool,
    /// The listening lifecycle: expanded surface with live capture.
    pub listening: bool,
    /// The newest partial transcript (listening or heard).
    pub partial: Option<String>,
    /// Outcome text retained by the engine for the result card.
    pub outcome_content: Option<OutcomeState>,
    /// True while an outcome card is showing.
    pub outcome: bool,
    /// The candidate carousel: the works the person swipes between.
    /// Empty when no candidates came back.
    pub choice: Option<ChoiceState>,
    /// The works-browse carousel (the rack as a resumable list), when one
    /// is open. Wins the surface over the choice/listening phases.
    pub browse: Option<BrowseState>,
    /// Index of the currently hovered chip (None = no hover).
    /// Used for hover highlight on action chips.
    pub hovered_chip: Option<usize>,
    /// The hardware notch width/height in points, from the safe-area
    /// probe (`auxiliaryTopLeftArea` + `auxiliaryTopRightArea`). The
    /// drawn shape expands from this exact rect so it reads as a
    /// continuation of the physical notch (the reference app's trick).
    pub notch_w: f64,
    pub notch_h: f64,
    /// The notch's left edge in canvas coordinates. This is derived from the
    /// selected screen frame and auxiliary safe-area geometry; it is not
    /// assumed to be the center of an arbitrary display.
    pub notch_x: f64,
    /// How far the expanded card has grown, 0.0 (just the notch band)
    /// to 1.0 (fully extended, with a small un-clamped overshoot on the
    /// open spring). The DynamicNotch springs (response 0.5 / damping
    /// 0.75) become this eased factor over ~350ms in the surface thread;
    /// the layout compresses content toward the band proportionally, so
    /// every layer stays inside the card mid-flight.
    pub expand: f64,
    /// How far the card has grown, driving spring-based geometry interpolation.
    /// Set by the surface thread as the eased progress of a phase transition.
    pub animating_direction: SpringDirection,
    /// The morph's content opacity: 1.0 at rest, and driven by
    /// [`transition_frame`] so content arrives late on a grow
    /// (smoothstep across 26%..89% of the transition) and leaves early
    /// on a shrink (1 -> 0 by 34%). The paint path multiplies every
    /// content layer's alpha by this; the `card` silhouette stays
    /// opaque and only its geometry morphs.
    pub content_alpha: f64,
    /// Seconds since this Listening phase began, driving the listening
    /// equalizer's bar scales (see [`wave_bar_scale`]). `None` in every
    /// other phase — and in tests / verification states, where the bars
    /// render at their deterministic minimum scale. The surface thread
    /// samples it once per wave frame while the engine is Listening and
    /// repaints at 24fps, the reference equalizer's own cadence.
    pub listening_t: Option<f64>,
    /// Where the surface is allowed to render. `Own` is anchored to the
    /// measured hardware notch; `Float` is offset below it so another notch
    /// owner is never covered.
    pub decision: SurfaceDecision,
    /// Scripted verify override (demo path); never set by normal driving.
    pub verify: Option<VerifyCase>,
}

/// The choice carousel's draw-ready state.
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceState {
    /// The candidates, in daemon rank order.
    pub candidates: Vec<ChoiceCandidate>,
    /// The focused candidate (0-based; wraps on step).
    pub index: usize,
}

/// The works-browse carousel's draw-ready state: the focused work,
/// already composed for display by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowseState {
    /// The focused work's daemon id (the CHOOSE key the Open chip sends).
    pub id: String,
    /// The focused work's title (the carousel's headline).
    pub title: String,
    /// The supporting line under the title (status · sittings · reason).
    pub subtitle: String,
    /// The focused work (0-based; wraps on step).
    pub index: usize,
    /// How many works the listing carries.
    pub count: usize,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            idle_glyph: false,
            rack: false,
            listening: false,
            partial: None,
            outcome_content: None,
            outcome: false,
            choice: None,
            browse: None,
            hovered_chip: None,
            notch_w: FALLBACK_NOTCH.0,
            notch_h: FALLBACK_NOTCH.1,
            // Negative means "center this compatibility state for its
            // supplied notch width". Live safe-area states always carry an
            // explicit canvas anchor.
            notch_x: -1.0,
            expand: 1.0,
            animating_direction: SpringDirection::Hover,
            content_alpha: 1.0,
            listening_t: None,
            decision: SurfaceDecision::Own,
            verify: None,
        }
    }
}

impl RenderState {
    /// Applies the engine's current state into draw-ready flags,
    /// including the notch metrics the surface probe measured.
    pub fn from_engine(engine: &NotchEngine, rack_count: Option<usize>) -> Self {
        Self::from_engine_with_notch(engine, rack_count, FALLBACK_NOTCH.0, FALLBACK_NOTCH.1)
    }

    /// Same, with live hardware notch metrics; increasingly the normal
    /// path on any machine with a notch.
    pub fn from_engine_with_notch(
        engine: &NotchEngine,
        rack_count: Option<usize>,
        notch_w: f64,
        notch_h: f64,
    ) -> Self {
        Self::from_engine_with_notch_and_decision(
            engine,
            rack_count,
            notch_w,
            notch_h,
            if engine.owns_strip {
                SurfaceDecision::Own
            } else {
                SurfaceDecision::Float
            },
        )
    }

    /// Same as [`Self::from_engine_with_notch`], with the platform ownership
    /// decision supplied by the window probe. Keeping this separate preserves
    /// the original constructor for callers and makes ownership explicit at
    /// the one boundary where it matters.
    pub fn from_engine_with_notch_and_decision(
        engine: &NotchEngine,
        rack_count: Option<usize>,
        notch_w: f64,
        notch_h: f64,
        decision: SurfaceDecision,
    ) -> Self {
        let visible = engine.model().map(|c| c.id);
        let choice = match &engine.voice.phase {
            crate::notch::VoicePhase::Choice { candidates, index } => Some(ChoiceState {
                candidates: candidates
                    .iter()
                    .map(|c| ChoiceCandidate {
                        work_id: c.work_id.clone(),
                        title: c.title.clone(),
                        selection: c.selection.clone(),
                    })
                    .collect(),
                index: *index,
            }),
            _ => None,
        };
        let browse = engine.browse().map(|session| BrowseState {
            id: session
                .focused()
                .map(|entry| entry.id.clone())
                .unwrap_or_default(),
            title: session
                .focused()
                .map(|entry| entry.title.clone())
                .unwrap_or_default(),
            subtitle: session
                .focused()
                .map(|entry| entry.subtitle.clone())
                .unwrap_or_default(),
            index: session.index,
            count: session.entries.len(),
        });
        Self {
            // EvoIdle is a legacy model identity only. Real idle is an empty
            // state, so the host orders its transparent panel out.
            idle_glyph: false,
            rack: visible == Some(ContentId::EvoRack) || rack_count.is_some_and(|n| n > 0),
            listening: engine.is_expanded(),
            partial: engine.voice.partial.clone(),
            outcome_content: engine.voice.outcome.clone(),
            outcome: matches!(engine.voice.phase, crate::notch::VoicePhase::Outcome)
                || visible == Some(ContentId::EvoOutcome),
            choice,
            browse,
            hovered_chip: None,
            notch_w,
            notch_h,
            notch_x: (CANVAS_SIZE.0 - notch_w) / 2.0,
            expand: 1.0,
            animating_direction: SpringDirection::Hover,
            content_alpha: 1.0,
            listening_t: None,
            decision,
            verify: None,
        }
    }

    /// Builds a state from the complete safe-area measurement. Unlike the
    /// width/height compatibility constructor, this preserves an asymmetric
    /// selected-screen anchor all the way into the layout.
    pub fn from_engine_with_geometry(
        engine: &NotchEngine,
        rack_count: Option<usize>,
        geometry: NotchGeometry,
        decision: SurfaceDecision,
    ) -> Self {
        let mut state = Self::from_engine_with_notch_and_decision(
            engine,
            rack_count,
            geometry.notch_w,
            geometry.notch_h,
            decision,
        );
        state.notch_x = geometry.canvas_notch_x();
        state
    }

    /// True when this state has pixels that should be ordered in front of
    /// other applications.  A transparent canvas with no layers is still
    /// hidden: AppKit must receive `orderOut:` rather than keeping a helper
    /// panel in the window list.
    pub fn is_visible(&self) -> bool {
        !surface_layouts(self).is_empty()
    }
}

/// A screen frame in the same logical point space as AppKit's `NSScreen`.
/// Kept platform-neutral so safe-area anchoring can be tested without opening
/// an AppKit window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenFrame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The measured notch geometry. `notch_x` is absolute in screen coordinates;
/// `canvas_notch_x` converts it to the transparent host canvas whose frame is
/// centered on this screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NotchGeometry {
    pub screen_frame: ScreenFrame,
    pub notch_x: f64,
    pub notch_w: f64,
    pub notch_h: f64,
}

impl NotchGeometry {
    /// Derive the physical cutout from the two auxiliary top safe areas and
    /// the top safe-area inset. The small four-point allowance is the same
    /// AppKit inset used by the reference geometry, while the screen frame
    /// remains the source of the anchor.
    pub fn from_safe_areas(
        screen_frame: ScreenFrame,
        auxiliary_left_w: f64,
        auxiliary_right_w: f64,
        safe_top: f64,
    ) -> Option<Self> {
        if screen_frame.w <= 0.0
            || screen_frame.h <= 0.0
            || safe_top <= 0.0
            || (auxiliary_left_w <= 0.0 && auxiliary_right_w <= 0.0)
        {
            return None;
        }
        let notch_w = screen_frame.w - (auxiliary_left_w + auxiliary_right_w) + 4.0;
        if notch_w <= 0.0 || notch_w >= screen_frame.w {
            return None;
        }
        Some(Self {
            screen_frame,
            notch_x: screen_frame.x + (screen_frame.w - notch_w) / 2.0,
            notch_w,
            notch_h: safe_top,
        })
    }

    /// Left edge of the physical notch in a centered `CANVAS_SIZE` host.
    pub fn canvas_notch_x(&self) -> f64 {
        self.notch_x - (self.screen_frame.x + (self.screen_frame.w - CANVAS_SIZE.0) / 2.0)
    }
}

/// The transcript is intentionally bounded in both dimensions. The returned
/// lines are ready for CoreText: at most two lines, with an ellipsis on the
/// final line whenever source text does not fit.
pub const MAX_TRANSCRIPT_LINES: usize = 2;

pub fn bounded_transcript(text: &str, width: f64, size: f64) -> Vec<String> {
    let max_chars = (width / (size * 0.60)).floor().max(1.0) as usize;
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::with_capacity(MAX_TRANSCRIPT_LINES);
    let mut current = String::new();
    let mut truncated = false;
    for word in words {
        let word_chars: Vec<char> = word.chars().collect();
        if word_chars.len() > max_chars {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                if lines.len() == MAX_TRANSCRIPT_LINES {
                    truncated = true;
                    break;
                }
            }
            let mut start = 0;
            while start < word_chars.len() && lines.len() < MAX_TRANSCRIPT_LINES {
                let end = (start + max_chars).min(word_chars.len());
                lines.push(word_chars[start..end].iter().collect());
                start = end;
            }
            if start < word_chars.len() {
                truncated = true;
            }
            if lines.len() == MAX_TRANSCRIPT_LINES {
                truncated = true;
                break;
            }
            continue;
        }
        let next_len = if current.is_empty() {
            word_chars.len()
        } else {
            current.chars().count() + 1 + word_chars.len()
        };
        if next_len <= max_chars {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            if lines.len() == MAX_TRANSCRIPT_LINES {
                truncated = true;
                break;
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() && lines.len() < MAX_TRANSCRIPT_LINES {
        lines.push(current);
    }
    if truncated && !lines.is_empty() {
        let last_index = lines.len() - 1;
        let chars: Vec<char> = lines[last_index].chars().collect();
        if max_chars <= 1 {
            lines[last_index] = "…".to_string();
        } else if chars.len() >= max_chars {
            lines[last_index] = chars[..max_chars - 1].iter().collect();
            lines[last_index].push('…');
        } else {
            lines[last_index].push('…');
        }
    }
    lines.truncate(MAX_TRANSCRIPT_LINES);
    lines
}

// ─── Layout: pure geometry + hit-testing (fully testable) ──────────────────

/// A paint/hit-test layer: rect + color + what clicking it does.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Layer {
    /// Logical points, origin top-left of the surface canvas.
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// RGBA 0..1
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
    /// Silhouette corner radii. Non-silhouette layers leave these at zero;
    /// the card interpolates them from the physical notch to its expanded
    /// Dynamic-Island shape.
    pub top_radius: f64,
    pub bottom_radius: f64,
    /// What a click on this layer produces (None = decorative).
    pub action: Option<OptionAction>,
    /// Stable id for verification & tests.
    pub id: &'static str,
}

/// What a click on a layer dispatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionAction {
    /// "Restore the last work" — the voice socket CHOOSE default pick.
    Restore,
    /// "Browse the rack" — the listening chip that opens the works
    /// carousel (the agent answers with the daemon's WORKS listing).
    OpenRack,
    /// Dismiss / collapse (goodbye, scarf).
    Dismiss,
    /// The candidate carousel's Next: advance the focused candidate.
    Next,
    /// The candidate carousel's Prev: step back.
    Prev,
}

impl OptionAction {
    pub fn label(&self) -> &'static str {
        match self {
            OptionAction::Restore => "restore",
            OptionAction::OpenRack => "browse",
            OptionAction::Dismiss => "dismiss",
            OptionAction::Next => "next",
            OptionAction::Prev => "prev",
        }
    }
}
/// The paint canvas size in points: small enough to sit inside the
/// screen-top band on a notched display while still having room for the
/// expanded card that grows downward from the hardware notch. The visible
/// shape is drawn by [`surface_layouts`], never the canvas itself.
pub const CANVAS_SIZE: (f64, f64) = (380.0, 320.0);

/// Fallback notch metrics when the safe-area probe is unavailable
/// (no hardware notch, or headless test runs): a slightly expanded
/// representative notch band so tests still render a real silhouette.
pub const FALLBACK_NOTCH: (f64, f64) = (150.0, 31.0);

/// Gap used when Evo must yield the hardware strip to another notch owner.
/// The floating card remains visually associated with the top edge without
/// pretending to be part of the physical notch.
pub const FLOAT_GAP: f64 = 12.0;

/// The silhouette is a single true black surface. Transparency belongs to the
/// canvas around it, never to the notch itself: a translucent rectangle reads
/// as a generic floating card on the built-in display.
const SURFACE: (f64, f64, f64, f64) = (0.0, 0.0, 0.0, 1.0);

/// DESIGN.md color tokens — single source of truth for notch visuals.
/// Environment: #15181A → #0F1214 (handled by transparent canvas + black card)
const INK_PRIMARY: (f64, f64, f64, f64) = (0.93, 0.92, 0.90, 1.0); // #ECEAE6
const INK_MUTED: (f64, f64, f64, f64) = (0.659, 0.667, 0.659, 1.0); // #A8AAA8        // muted ink
const TEAL_VOICE: (f64, f64, f64, f64) = (0.357, 0.541, 0.561, 1.0); // #5B8A8F — teal voice
const CARAMEL: (f64, f64, f64, f64) = (0.784, 0.608, 0.420, 1.0); // #C89B6B — caramel accent
/// Chip fill grammar, lifted from the reference apps: every chip is a
/// tinted-GLASS capsule, never a solid accent slab. The dark chips are
/// white at 8% (hover 12%); the caramel pick is the accent itself at
/// 20% (hover 30%). The label is the ink — "accent TEXT on tinted
/// glass" — so there is no dark on-accent ink anywhere any more.
const CHIP_BG: (f64, f64, f64, f64) = (1.0, 1.0, 1.0, 0.08);
const CHIP_BG_HOVER: (f64, f64, f64, f64) = (1.0, 1.0, 1.0, 0.12);
/// The caramel pick's glass opacity (Restore/Open): steady, then hover.
const CHIP_CARAMEL_FILL: f64 = 0.20;
const CHIP_CARAMEL_FILL_HOVER: f64 = 0.30;
/// The dark chips' label ink: white at 85% over their tinted glass.
const CHIP_INK_WHITE: f64 = 0.85;

/// Font role sizes matching DESIGN.md typography scale.
/// UI voice uses system SF via CTFontCreateUIFontForLanguage.
const FONT_UI_EYEBROW: f64 = 10.5; // semibold — teal voice labels
/// The eyebrow's letter-spacing, in points: small uppercase labels read
/// as machinery when tracked open. Applied as kCTKernAttributeName.
const FONT_UI_EYEBROW_TRACKING: f64 = 0.6;
const FONT_UI_TRANSCRIPT: f64 = 15.0; // regular — live partial transcript
const FONT_UI_CHOICE_TITLE: f64 = 15.0; // regular — candidate title
const FONT_UI_CHOICE_SUBTITLE: f64 = 11.0; // semibold — "work N of M · swipe or click"
const FONT_UI_OUTCOME_TITLE: f64 = 15.0; // regular
const FONT_UI_OUTCOME_DETAIL: f64 = 11.0; // medium (semibold) muted
const FONT_UI_CHIP: f64 = 12.5; // semibold — chip labels

/// The expanded card's silhouette radii, in points: the reference's
/// `cornerRadiusInsets.opened` pair (top 19 / bottom 24). The collapsed
/// state is the physical notch itself — square at the top so the overlay
/// is flush with the bezel, with the shallow 8pt bottom radius — and
/// `compress_to_expand` interpolates linearly between the two, so the
/// radius morphs over exactly the same 0→1 `expand` factor as the size.
const CARD_TOP_RADIUS: f64 = 19.0;
const CARD_BOTTOM_RADIUS: f64 = 24.0;
const CARD_COLLAPSED_TOP_RADIUS: f64 = 0.0;
const CARD_COLLAPSED_BOTTOM_RADIUS: f64 = 8.0;

/// The card's vertical rhythm, shared by every expanded phase. Measured
/// screenshots showed 44-70% of the card as dead black, so rows are packed
/// from the notch band with one inset at the top, one at the bottom, and a
/// fixed gap between rows - and `body_h` is the sum of what is actually
/// there, never a round number.
const CARD_INSET: f64 = 12.0;
/// Minimum vertical gap between two stacked rows.
const ROW_GAP: f64 = 6.0;
/// The chip row reads as a separate control band, so it gets a little more
/// air than two text rows do.
const CHIP_ROW_GAP: f64 = 10.0;
/// Chip tap-target metrics. The pill radius is always `CHIP_H / 2`.
/// The minimum width is the reference's capsule grammar — a centered
/// label with 12pt of horizontal padding on each side, floored at 72pt
/// (the reference's own chip floor) so a short verb keeps a full-size
/// tap target. The four-chip carousel row is the one place card
/// geometry forces narrower pills; it derives from the body there.
const CHIP_H: f64 = 28.0;
const CHIP_MIN_W: f64 = 72.0;

/// The minimum height a text row needs to hold its font's ascent box.
/// Rows shorter than this clip the ascenders flat: CoreText places the
/// baseline a full ascent below the row top, so a 15pt line in an 18pt
/// row loses the tops of its letters (measured on real screenshots).
fn text_row_h(size: f64) -> f64 {
    (size * 1.35).ceil()
}

/// The label a chip paints on the current surface. The works-browse
/// carousel relabels its caramel pick to `Open` and its dismiss chip to
/// `Close`; the listening card's rack chip now opens the browse, so its
/// verb is `Browse`. The candidate carousel keeps the original verbs.
fn chip_label(state: &RenderState, id: &str) -> Option<&'static str> {
    match id {
        "chip_restore" if state.browse.is_some() => Some("Open"),
        "chip_dismiss" if state.browse.is_some() => Some("Close"),
        "chip_restore" => Some("Restore"),
        "chip_rack" => Some("Browse"),
        "chip_dismiss" => Some("Dismiss"),
        "chip_prev" => Some("Prev"),
        "chip_next" => Some("Next"),
        _ => None,
    }
}

/// The fill of one chip in the reference's tinted-glass grammar,
/// returned as `(r, g, b, a)`: the caramel pick speaks CARAMEL on a
/// caramel-tinted capsule (Restore/Open), every other chip is white on
/// white-tinted glass, and hover raises only the glass opacity — never
/// to a solid slab. Shared by the listening, outcome and carousel rows
/// so all three can only ever disagree by id.
fn chip_fill(id: &str, hovered: bool) -> (f64, f64, f64, f64) {
    if id == "chip_restore" {
        let a = if hovered {
            CHIP_CARAMEL_FILL_HOVER
        } else {
            CHIP_CARAMEL_FILL
        };
        (CARAMEL.0, CARAMEL.1, CARAMEL.2, a)
    } else if hovered {
        CHIP_BG_HOVER
    } else {
        CHIP_BG
    }
}

/// Pure layout for one `RenderState` — the ground truth the ObjC view
/// paints and the demo asserts against. Colors chosen so each layer is
/// pixel-recognizable in the bitmap verification.
pub fn surface_layouts(state: &RenderState) -> Vec<Layer> {
    let (cw, _ch) = CANVAS_SIZE;
    // The notch band: the visible shape always contains this exact rect from
    // the selected screen's measured safe-area anchor, so whatever phase is
    // live, the drawn card reads as a continuation of the hardware notch.
    let (nw, nh) = (state.notch_w, state.notch_h);
    let nx = if state.notch_x >= 0.0 && state.notch_x <= (cw - nw).max(0.0) {
        state.notch_x
    } else {
        (cw - nw) / 2.0
    };

    // How far the expanded card reaches beyond the notch band per phase.
    // An ordinary idle state has no phase at all: returning an empty layout
    // is what lets the host order its transparent panel out.  IdleGlyph is
    // retained only for the explicit verification/forced-mode path, so a
    // normal idle frame can never grow a teal `e` or fallback pill.
    // Idle is not a visual state. Keep the legacy selector only so old verify
    // callers remain source-compatible, but never leave a pixel or helper
    // panel ordered above another app.
    if matches!(state.verify, Some(VerifyCase::IdleGlyph)) {
        return Vec::new();
    }
    let phase = state.verify.or_else(|| {
        if state.browse.is_some() {
            Some(VerifyCase::Browse)
        } else if state.choice.is_some() {
            Some(VerifyCase::Choice)
        } else if state.listening {
            Some(VerifyCase::Listening)
        } else if state.outcome {
            Some(VerifyCase::Outcome)
        } else {
            None
        }
    });

    let Some(phase) = phase else {
        // Rack content is a real compact activity, unlike idle.  It gets the
        // hardware notch band only; the quiet idle mark is never painted.
        if state.rack {
            return vec![notch_shape_layer(nx, 0.0, nw, nh, "band")];
        }
        return Vec::new();
    };

    // Every phase centers on the hardware notch's own center (`nx + nw/2`):
    // the band expands from where the notch actually is, never drifts
    // sideways. The expanded card shares this exact axis so expansion is one
    // continuous motion.
    let notch_center = nx + nw / 2.0;
    let centered = |w: f64| -> f64 { notch_center - w / 2.0 };

    let mut layers = match phase {
        VerifyCase::IdleGlyph => unreachable!("legacy idle verification is hidden"),
        VerifyCase::Listening => {
            // Expanded card: the notch band plus an extended body below
            // it, NotchShape-shaped (square top, rounded bottom). Rows
            // stack without overlap: header (mic + eyebrow) → transcript
            // → chip row, each on its own band of the body.
            let body_y = 0.0;
            let body_w = (nw + 120.0).min(cw - 16.0);
            let bx = centered(body_w);

            // Header row, one card inset below the notch band: the live
            // listening mark (mic ring + equalizer), then the "LISTENING"
            // eyebrow in the teal voice.
            let header_y = nh + CARD_INSET;
            let eyebrow_h = text_row_h(FONT_UI_EYEBROW);
            let transcript_line_h = text_row_h(FONT_UI_TRANSCRIPT);
            // The transcript row is reserved whether or not anything has been
            // heard: an empty partial paints the `say a command` prompt
            // (see `draw_layer_glyphs`), so the card is never dead black.
            let transcript_line_count = state
                .partial
                .as_deref()
                .filter(|text| !text.trim().is_empty())
                .map(|text| bounded_transcript(text, body_w - 36.0, FONT_UI_TRANSCRIPT).len())
                .unwrap_or(1)
                .clamp(1, MAX_TRANSCRIPT_LINES);
            let transcript_top = header_y + eyebrow_h + ROW_GAP;
            let transcript_h = transcript_line_count as f64 * transcript_line_h;
            let chip_h = CHIP_H;
            let chip_row_top = transcript_top + transcript_h + CHIP_ROW_GAP;
            // Sized from the rows that exist, plus the bottom inset.
            let body_h = chip_row_top + chip_h + CARD_INSET;
            let mut layers = vec![notch_shape_layer(bx, body_y, body_w, body_h, "card")];

            // The listening mark, at the header's left, vertically centered:
            // a 12pt mic RING — a 1.5pt caramel stroke over a caramel@0.18
            // track fill (the stroke is painted by the rasterizer, which
            // owns the silhouette passes) — with a 3pt caramel center dot.
            // This replaces the old breathing dot: the reference's live
            // capture grammar is ring + equalizer, never an alpha pulse.
            let ring_size = 12.0;
            let ring_x = bx + 18.0;
            let ring_y = header_y + (eyebrow_h - ring_size) / 2.0;
            layers.push(Layer {
                x: ring_x,
                y: ring_y,
                w: ring_size,
                h: ring_size,
                r: CARAMEL.0,
                g: CARAMEL.1,
                b: CARAMEL.2,
                a: CARAMEL.3 * 0.18,
                // Equal radii of half the side make `shape_contains` a
                // circle: the track fill under the ring stroke.
                top_radius: ring_size / 2.0,
                bottom_radius: ring_size / 2.0,
                action: None,
                id: "mic_ring",
            });
            layers.push(Layer {
                x: ring_x + ring_size / 2.0 - 1.5,
                y: ring_y + ring_size / 2.0 - 1.5,
                w: 3.0,
                h: 3.0,
                r: CARAMEL.0,
                g: CARAMEL.1,
                b: CARAMEL.2,
                a: CARAMEL.3,
                top_radius: 1.5,
                bottom_radius: 1.5,
                action: None,
                id: "mic_dot",
            });

            // The equalizer: six 2.5pt capsules on a 2pt gap right after
            // the ring, up to 16pt tall, breathing symmetrically about the
            // header's center line. Teal with the reference's per-bar alpha
            // ramp (0.55 first → 1.0 last), which approximates its
            // horizontal gradient. `listening_t` drives the scales; with no
            // clock (tests, verification) the bars hold the deterministic
            // minimum scale.
            let bar_center_y = ring_y + ring_size / 2.0;
            let bars_x = ring_x + ring_size + 4.0;
            for bar in 0..WAVE_BAR_COUNT {
                let scale = state
                    .listening_t
                    .map(|t| wave_bar_scale(bar, t))
                    .unwrap_or(WAVE_MIN_SCALE);
                let bar_h = WAVE_BAR_MAX_H * scale;
                let alpha = WAVE_ALPHA_FIRST
                    + (1.0 - WAVE_ALPHA_FIRST)
                        * (bar as f64 / (WAVE_BAR_COUNT.saturating_sub(1)).max(1) as f64);
                layers.push(Layer {
                    x: bars_x + bar as f64 * (WAVE_BAR_W + WAVE_BAR_GAP),
                    y: bar_center_y - bar_h / 2.0,
                    w: WAVE_BAR_W,
                    h: bar_h,
                    r: TEAL_VOICE.0,
                    g: TEAL_VOICE.1,
                    b: TEAL_VOICE.2,
                    a: TEAL_VOICE.3 * alpha,
                    top_radius: WAVE_BAR_RADIUS,
                    bottom_radius: WAVE_BAR_RADIUS,
                    action: None,
                    id: "wave_bar",
                });
            }

            // The eyebrow label follows the equalizer on the header line.
            let eyebrow_x =
                bars_x + WAVE_BAR_COUNT as f64 * (WAVE_BAR_W + WAVE_BAR_GAP) - WAVE_BAR_GAP + 8.0;
            layers.push(Layer {
                x: eyebrow_x,
                y: header_y,
                w: 90.0,
                h: eyebrow_h,
                r: TEAL_VOICE.0,
                g: TEAL_VOICE.1,
                b: TEAL_VOICE.2,
                a: TEAL_VOICE.3,
                action: None,
                id: "eyebrow",
                ..Default::default()
            });

            // Live transcript bucket: the body's biggest zone, between
            // the header and the chips, in light ink over the card.
            layers.push(Layer {
                x: bx + 18.0,
                y: transcript_top,
                w: body_w - 36.0,
                h: transcript_h,
                r: INK_PRIMARY.0,
                g: INK_PRIMARY.1,
                b: INK_PRIMARY.2,
                a: INK_PRIMARY.3,
                action: None,
                id: "transcript",
                ..Default::default()
            });

            // The three chips fit the minimum capsule width at the
            // fallback notch (3 × 72 + 2 × 8 = 232 ≤ 238), so this row
            // never has to trade the reference's 12pt label padding away.
            let chip_w = CHIP_MIN_W;

            // Action chips along the bottom of the expanded card:
            // restore (caramel glass), rack, dismiss — tap targets, in
            // Evo's tinted-glass control language.
            let hovered = state.hovered_chip;
            for (index, (action, id)) in [
                (Some(OptionAction::Restore), "chip_restore"),
                (Some(OptionAction::OpenRack), "chip_rack"),
                (Some(OptionAction::Dismiss), "chip_dismiss"),
            ]
            .into_iter()
            .enumerate()
            {
                let (r, g, b, a) = chip_fill(id, hovered == Some(index));
                layers.push(Layer {
                    x: bx + 16.0 + index as f64 * (chip_w + 8.0),
                    y: chip_row_top,
                    w: chip_w,
                    h: chip_h,
                    r,
                    g,
                    b,
                    a,
                    // A chip is a pill, like the reference's OK / Settings
                    // controls: half its height on both ends.
                    top_radius: chip_h / 2.0,
                    bottom_radius: chip_h / 2.0,
                    action,
                    id,
                });
            }
            layers
        }
        VerifyCase::Choice => carousel_layers(
            nw,
            nh,
            cw,
            notch_center,
            state.hovered_chip,
            state
                .choice
                .as_ref()
                .map(|c| c.candidates.len())
                .unwrap_or(0),
            state.choice.as_ref().map(|c| c.index).unwrap_or(0),
        ),
        VerifyCase::Browse => carousel_layers(
            nw,
            nh,
            cw,
            notch_center,
            state.hovered_chip,
            state.browse.as_ref().map(|b| b.count).unwrap_or(0),
            state.browse.as_ref().map(|b| b.index).unwrap_or(0),
        ),
        VerifyCase::Outcome => {
            // Outcome card, strictly top-to-bottom: eyebrow -> title ->
            // detail -> the Dismiss chip. Same NotchStyle silhouette as the
            // listening card so the collapse is one continuous motion.
            //
            // There is deliberately NO full-width accent rectangle any more.
            // The reference only ever puts an accent on text, a small ring,
            // or a pill; the old `outcome_band` painted a caramel slab and
            // then drew the eyebrow on top of it, and the title/detail ink
            // colors were filled as slabs too (measured 1.1:1 and 1.3:1
            // contrast - the "grey block / white block" on screen). The
            // accent now lives in the eyebrow's ink.
            let body_w = (nw + 120.0).min(cw - 16.0);
            let bx = centered(body_w);
            let eyebrow_h = text_row_h(FONT_UI_EYEBROW);
            let title_h = text_row_h(FONT_UI_OUTCOME_TITLE);
            let detail_h = text_row_h(FONT_UI_OUTCOME_DETAIL);
            // Each row starts one gap after the previous row ends, so no two
            // rows can overlap by construction.
            let eyebrow_y = nh + CARD_INSET;
            let title_y = eyebrow_y + eyebrow_h + ROW_GAP;
            let detail_y = title_y + title_h + ROW_GAP;
            let chip_row_top = detail_y + detail_h + CHIP_ROW_GAP;
            let body_h = chip_row_top + CHIP_H + CARD_INSET;
            let mut layers = vec![notch_shape_layer(bx, 0.0, body_w, body_h, "card")];

            // 1. The eyebrow, in the caramel accent voice.
            layers.push(Layer {
                x: bx + 16.0,
                y: eyebrow_y,
                w: body_w - 32.0,
                h: eyebrow_h,
                r: CARAMEL.0,
                g: CARAMEL.1,
                b: CARAMEL.2,
                a: CARAMEL.3,
                action: None,
                id: "eyebrow",
                ..Default::default()
            });
            // 2. The outcome title, primary ink.
            layers.push(Layer {
                x: bx + 16.0,
                y: title_y,
                w: body_w - 32.0,
                h: title_h,
                r: INK_PRIMARY.0,
                g: INK_PRIMARY.1,
                b: INK_PRIMARY.2,
                a: INK_PRIMARY.3,
                action: None,
                id: "outcome_title",
                ..Default::default()
            });
            // 3. The detail line, muted ink.
            layers.push(Layer {
                x: bx + 16.0,
                y: detail_y,
                w: body_w - 32.0,
                h: detail_h,
                r: INK_MUTED.0,
                g: INK_MUTED.1,
                b: INK_MUTED.2,
                a: INK_MUTED.3,
                action: None,
                id: "outcome_detail",
                ..Default::default()
            });
            // 4. The chip row: Dismiss alone, right-aligned on its own
            // band, as tinted glass like every other chip.
            let (r, g, b, a) = chip_fill("chip_dismiss", state.hovered_chip == Some(0));
            layers.push(Layer {
                x: bx + body_w - 16.0 - CHIP_MIN_W,
                y: chip_row_top,
                w: CHIP_MIN_W,
                h: CHIP_H,
                r,
                g,
                b,
                a,
                top_radius: CHIP_H / 2.0,
                bottom_radius: CHIP_H / 2.0,
                action: Some(OptionAction::Dismiss),
                id: "chip_dismiss",
            });
            layers
        }
    };
    if let Some(card) = layers.iter_mut().find(|layer| layer.id == "card") {
        // The closed physical notch has a square top edge so the overlay is
        // attached to the bezel. Expansion introduces the softer Dynamic
        // Island radii (the reference's 19/24), and `compress_to_expand`
        // interpolates these values alongside width and height.
        card.top_radius = CARD_TOP_RADIUS;
        card.bottom_radius = CARD_BOTTOM_RADIUS;
    }
    // The DynamicNotch springs (response 0.5 / damping 0.75) enter the
    // layout as one eased factor: at expand < 1 the card is shorter and
    // every row compresses toward the band proportionally, so nothing
    // escapes the card mid-flight and tap targets stay inside it. The
    // open spring is allowed its physical overshoot (up to
    // `EXPAND_OVERSHOOT_LIMIT`): the silhouette grows by the same
    // factor as everything it contains, so the "nothing escapes"
    // invariant holds at the overshoot peak too.
    compress_to_expand(
        &mut layers,
        notch_center,
        nw,
        nh,
        state.expand.clamp(0.0, EXPAND_OVERSHOOT_LIMIT),
        state.animating_direction,
    );
    if state.decision == SurfaceDecision::Float {
        let offset = nh + FLOAT_GAP;
        for layer in &mut layers {
            layer.y += offset;
        }
    }
    layers
}

/// The carousel geometry shared verbatim by the candidate-choice and
/// works-browse phases: the same card size math, eyebrow / title /
/// subtitle rows, pip strip, chip row and swipe zone. `count`/`index`
/// describe whichever carousel is live; the glyph painter decides which
/// copy (and which state) the rows speak.
fn carousel_layers(
    nw: f64,
    nh: f64,
    cw: f64,
    notch_center: f64,
    hovered: Option<usize>,
    count: usize,
    index: usize,
) -> Vec<Layer> {
    // The candidate carousel: the notch grows *more* than it
    // does for listening (the person is browsing, and gets a
    // bigger surface to read titles and hit chips). Rows again:
    // eyebrow → candidate title + subtitle → pips → chips.
    let body_h = nh + 132.0;
    let body_y = 0.0;
    let body_w = (nw + 120.0).min(cw - 16.0);
    let bx = notch_center - body_w / 2.0;
    let mut layers = vec![notch_shape_layer(bx, body_y, body_w, body_h, "card")];

    // The eyebrow, in the teal voice, right below the band.
    let header_y = nh + 8.0;
    layers.push(Layer {
        x: bx + 18.0,
        y: header_y,
        w: 90.0,
        h: 16.0,
        r: TEAL_VOICE.0,
        g: TEAL_VOICE.1,
        b: TEAL_VOICE.2,
        a: TEAL_VOICE.3,
        action: None,
        id: "eyebrow",
        ..Default::default()
    });

    // The focused candidate's title, centered on the card —
    // the biggest zone in the whole surface (this is what the
    // notch expands for).
    let choice_top = header_y + 24.0;
    layers.push(Layer {
        x: bx + 16.0,
        y: choice_top,
        w: body_w - 32.0,
        h: 24.0,
        r: INK_PRIMARY.0,
        g: INK_PRIMARY.1,
        b: INK_PRIMARY.2,
        a: INK_PRIMARY.3,
        action: None,
        id: "choice_title",
        ..Default::default()
    });
    // The subtitle under the title: work N of M · swipe or click.
    layers.push(Layer {
        x: bx + 16.0,
        y: choice_top + 28.0,
        w: body_w - 32.0,
        h: 14.0,
        r: INK_MUTED.0,
        g: INK_MUTED.1,
        b: INK_MUTED.2,
        a: INK_MUTED.3,
        action: None,
        id: "choice_subtitle",
        ..Default::default()
    });
    // The pips indicator (one stroke per candidate, focused on
    // lit): a count in glyphs, drawn as sparse strokes.
    let pip_zone_top = choice_top + 46.0;
    if count > 0 {
        // A daemon may return more candidates than can be represented
        // by individual pips. Keep the indicator inside the card;
        // the focused index itself remains fully canonical.
        let pip_count = count.min(8);
        let pip_gap = 6.0f64;
        let pip_w = ((body_w - 32.0 - pip_count.saturating_sub(1) as f64 * pip_gap)
            / pip_count as f64)
            .clamp(4.0, 10.0);
        let total_w = pip_count as f64 * pip_w + pip_count.saturating_sub(1) as f64 * pip_gap;
        let px0 = notch_center - total_w / 2.0;
        for i in 0..pip_count {
            let is_focused = i == index;
            layers.push(Layer {
                x: px0 + i as f64 * (pip_w + pip_gap),
                y: pip_zone_top,
                w: pip_w,
                h: 4.0,
                r: INK_PRIMARY.0,
                g: INK_PRIMARY.1,
                b: INK_PRIMARY.2,
                a: if is_focused { 1.0 } else { 0.35 },
                action: None,
                id: "choice_pip",
                ..Default::default()
            });
        }
    }

    // Chips: Restore (caramel glass — the pick), Prev / Next (the
    // carousel steps), Close. Restores dispatch the focused
    // candidate's work id; steps their own actions. Four chips
    // must fit the card at the fallback notch width, so the
    // width derives from the body and caps at the capsule minimum
    // (CHIP_MIN_W — the 3-chip rows get the full minimum, this row
    // is the documented geometry compromise).
    let chip_h = CHIP_H;
    let chip_row_top = body_h - chip_h - CARD_INSET;
    let chip_w = ((body_w - 32.0 - 3.0 * 8.0) / 4.0).min(CHIP_MIN_W);
    let chips: [(Option<OptionAction>, &str); 4] = [
        (Some(OptionAction::Restore), "chip_restore"),
        (Some(OptionAction::Prev), "chip_prev"),
        (Some(OptionAction::Next), "chip_next"),
        (Some(OptionAction::Dismiss), "chip_dismiss"),
    ];
    for (index, (action, id)) in chips.into_iter().enumerate() {
        let (r, g, b, a) = chip_fill(id, hovered == Some(index));
        layers.push(Layer {
            x: bx + 16.0 + index as f64 * (chip_w + 8.0),
            y: chip_row_top,
            w: chip_w,
            h: chip_h,
            r,
            g,
            b,
            a,
            // A chip is a pill, like the reference's OK / Settings
            // controls: half its height on both ends.
            top_radius: chip_h / 2.0,
            bottom_radius: chip_h / 2.0,
            action,
            id,
        });
    }
    layers
}

/// Compresses a full-extension layout toward the notch band by the
/// expand factor: the card shrinks its extension, rows scale their
/// offsets and heights proportionally. Pure geometry — the animation
/// driver in the surface thread only ever changes `RenderState.expand`.
fn compress_to_expand(
    layers: &mut [Layer],
    notch_center: f64,
    notch_w: f64,
    band_h: f64,
    ext: f64,
    direction: SpringDirection,
) {
    let progress = spring_progress(ext, direction);
    if (progress - 1.0).abs() < f64::EPSILON {
        return;
    }
    let Some(card) = layers.iter().find(|layer| layer.id == "card").copied() else {
        // The compact rack/verification band has no expanded body to
        // animate. Keep its physical notch rect exact.
        return;
    };
    let collapsed_x = notch_center - notch_w / 2.0;
    let width = notch_w + (card.w - notch_w) * progress;
    let x = collapsed_x + (card.x - collapsed_x) * progress;
    for layer in layers.iter_mut() {
        if layer.id == "card" {
            // The silhouette itself: its height IS the extension.
            layer.x = x;
            layer.w = width;
            layer.h = band_h + (layer.h - band_h) * progress;
            // Radii ride the same expand factor, from the square-top
            // notch (0 / 8) to the reference card (19 / 24).
            layer.top_radius =
                CARD_COLLAPSED_TOP_RADIUS + (CARD_TOP_RADIUS - CARD_COLLAPSED_TOP_RADIUS) * progress;
            layer.bottom_radius = CARD_COLLAPSED_BOTTOM_RADIUS
                + (layer.bottom_radius - CARD_COLLAPSED_BOTTOM_RADIUS) * progress;
        } else if layer.id != "band" {
            // Content rows: offsets and heights scale with the growth.
            layer.y = band_h + (layer.y - band_h) * progress;
            layer.h *= progress;
            // The reference expands outward from the closed notch in both
            // axes. Scale each content bucket around the physical notch
            // center so a mid-flight frame cannot become a detached pill.
            let layer_center = layer.x + layer.w / 2.0;
            let scaled_w = layer.w * progress;
            layer.x = notch_center + (layer_center - notch_center) * progress - scaled_w / 2.0;
            layer.w = scaled_w;
        }
    }
}

/// Which way a phase transition is moving. The springs are intentionally
/// different: opening should feel eager (slightly under-damped, soft
/// settle), closing should feel like the card is being absorbed by the
/// bezel (critically damped, no bounce).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpringDirection {
    /// The card is growing out of the notch band.
    Expand,
    /// The card is shrinking back into the notch band.
    Collapse,
    /// The card is hovering/animating in place (e.g. equalizer bars).
    Hover,
}

/// The ceiling the open path accepts for the expand spring's single
/// physical overshoot. Response 0.42 / damping 0.8 peaks at ~1.0152
/// (around 35% of the transition, comfortably inside the 350ms
/// window); this guard leaves headroom for retuning while keeping the
/// layout's "nothing escapes the card" invariant bounded.
pub const EXPAND_OVERSHOOT_LIMIT: f64 = 1.03;

/// The eased progress for a phase transition, as an analytic spring.
///
/// Expansion is the reference app's open spring
/// (`spring(response: 0.42, dampingFraction: 0.8)`): quick launch, soft
/// settle with slight overshoot — the "fast-out" feel that makes the
/// notch expansion read as responsive. The overshoot is deliberately
/// NOT clamped away: the probe rides up to ~1.015 mid-flight and the
/// layout accepts it (bounded by [`EXPAND_OVERSHOOT_LIMIT`]), then the
/// curve settles to exactly 1.0 at the end. Collapse is critically
/// damped (`response: 0.45, dampingFraction: 1.0`): it approaches 1
/// smoothly from below and never overshoots, so the retracting card
/// cannot visibly bounce past the notch band it is heading into.
pub fn eased_expand(t: f64, direction: SpringDirection) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t <= f64::EPSILON {
        return 0.0;
    }
    if (1.0 - t).abs() <= f64::EPSILON {
        return 1.0;
    }
    let (response, damping): (f64, f64) = match direction {
        SpringDirection::Expand => (0.42, 0.8),
        SpringDirection::Collapse => (0.45, 1.0),
        SpringDirection::Hover => (0.35, 0.7),
    };
    let omega_0 = std::f64::consts::TAU / response;
    let progress = if damping >= 1.0 {
        // Critically damped: the oscillating term degenerates (omega_d
        // and the damping/under ratio are both zero/undefined), so use
        // the closed form directly.
        1.0 - (1.0 + omega_0 * t) * (-omega_0 * t).exp()
    } else {
        let under = (1.0 - damping * damping).sqrt();
        let omega_d = omega_0 * under;
        let envelope = (-damping * omega_0 * t).exp();
        1.0 - envelope * ((omega_d * t).cos() + (damping / under) * (omega_d * t).sin())
    };
    match direction {
        // The open spring's overshoot rides through un-clamped; the bound
        // is a guard for retuning, not a shaping clamp.
        SpringDirection::Expand => progress.clamp(0.0, EXPAND_OVERSHOOT_LIMIT),
        // The close spring approaches 1 from below by construction; the
        // clamp encodes "collapse never bounces past the band".
        SpringDirection::Collapse => progress.clamp(0.0, 1.0),
        // Hover: gentle under-damped spring with no overshoot.
        SpringDirection::Hover => progress,
    }
}

/// Clean-room spring progress reimplementing Sapphire's motion grammar.
/// Maps normalized time [0,1] to a physics-based progress factor.
///
/// Uses the same spring table as Sapphire's AGPL-licensed motion grammar
/// but independently derived from first principles (simple harmonic oscillator).
///
/// Motion model: simple harmonic oscillator x'' + 2*zeta*omega*x' + omega^2*x = 0
///   omega = 2*pi / response   (angular frequency)
///   zeta = damping / 2        (damping ratio)
///
/// Spring table (Sapphire grammar, clean-room reimplement):
///   - Expand:    response=0.40, damping=0.55  (under-damped, soft settle)
///   - Collapse:  response=0.30, damping=0.98  (critically damped, no bounce)
///   - Hover:     response=0.25, damping=0.55  (responsive, subtle vibration)
pub fn spring_progress(t: f64, direction: SpringDirection) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t <= f64::EPSILON {
        return 0.0;
    }
    if (1.0 - t).abs() <= f64::EPSILON {
        return 1.0;
    }
    let (response, damping): (f64, f64) = match direction {
        SpringDirection::Expand => (0.40, 0.55),
        SpringDirection::Collapse => (0.30, 0.98),
        SpringDirection::Hover => (0.25, 0.55),
    };
    let omega_0 = std::f64::consts::TAU / response;
    let zeta = damping / 2.0;
    let progress = if zeta >= 1.0 {
        // Critically damped / over-damped: closed form directly.
        1.0 - (1.0 + omega_0 * t) * (-omega_0 * t).exp()
    } else {
        let under = (1.0 - zeta * zeta).sqrt();
        let omega_d = omega_0 * under;
        let envelope = (-zeta * omega_0 * t).exp();
        1.0 - envelope * ((omega_d * t).cos() + (zeta / under) * (omega_d * t).sin())
    };
    match direction {
        // The expand spring's overshoot rides through un-clamped;
        // the bound is a guard for retuning, not a shaping clamp.
        SpringDirection::Expand => progress,
        // The collapse spring approaches 1 from below by construction;
        // the clamp encodes "collapse never bounces past the band".
        SpringDirection::Collapse => progress.clamp(0.0, 1.0),
        // Hover: symmetric around the midpoint, no overshoot clamping.
        SpringDirection::Hover => progress,
    }
}

/// How far this state's card extends beyond the notch band, in points.
/// The animation driver compares this between phases to decide grow
/// vs. shrink (and whether anything needs animating at all).
pub fn card_extension(state: &RenderState) -> f64 {
    surface_layouts(state)
        .iter()
        .find(|l| l.id == "card")
        .map(|card| (card.h - state.notch_h).max(0.0))
        .unwrap_or(0.0)
}

// ─── The listening equalizer (pure motion) ─────────────────────────────────
//
// The bars reproduce DynamicNotch's `LightweightNowPlayingEqualizerView`
// exactly: six bars retargeted every 0.12s (`Metrics.timerInterval`) to a
// fresh random scale in [0.32, 1.0] (`minimumPlayingScale...1`), each hop
// animated ease-in-out over 0.2s (`Metrics.animationDuration` with the
// `.easeInEaseOut` timing function). The reference's RNG is replaced by a
// deterministic integer hash, so the same (tick, bar) always yields the
// same draw and the whole motion is a testable pure function.

/// The equalizer's bar count (`Metrics.barCount`).
pub const WAVE_BAR_COUNT: usize = 6;
/// The scale floor (`Metrics.minimumScale`), shared with the layout's
/// no-clock rest state.
pub const WAVE_MIN_SCALE: f64 = 0.32;
/// Per-bar geometry: 2.5pt wide, 2pt gap, 16pt at full scale, capsule
/// ends at half the width (the reference's default 16pt / 2pt grammar,
/// widened to the notch's bar weight).
const WAVE_BAR_W: f64 = 2.5;
const WAVE_BAR_GAP: f64 = 2.0;
const WAVE_BAR_MAX_H: f64 = 16.0;
const WAVE_BAR_RADIUS: f64 = 1.25;
/// The first bar's alpha; the ramp reaches 1.0 at the last bar and
/// approximates the reference's horizontal color gradient.
const WAVE_ALPHA_FIRST: f64 = 0.55;
/// The retarget cadence (`Metrics.timerInterval`).
const WAVE_TICK_SECS: f64 = 0.12;
/// The per-hop animation length (`Metrics.animationDuration`).
const WAVE_EASE_SECS: f64 = 0.2;

/// splitmix64: the small integer finalizer that turns a `(tick, bar)`
/// pair into the same pseudo-random draw on every machine and every run.
fn splitmix64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The deterministic target scale for one bar at one tick, uniform in
/// `[WAVE_MIN_SCALE, 1.0]` — the reproducible stand-in for the
/// reference's `CGFloat.random(in: minimumPlayingScale...1)`. The draw
/// is keyed by `splitmix64(tick * 31 + bar)`, so bars never share a
/// value and a given tick is stable across repaints.
fn wave_bar_target(tick: u64, bar: usize) -> f64 {
    let hashed = splitmix64(tick.wrapping_mul(31).wrapping_add(bar as u64));
    // The top 53 bits are a uniform fraction in [0, 1), exactly
    // representable in f64 — the same draw quality as the double-based
    // reference, without any float RNG state.
    let fraction = (hashed >> 11) as f64 / (1u64 << 53) as f64;
    WAVE_MIN_SCALE + fraction * (1.0 - WAVE_MIN_SCALE)
}

/// `CAMediaTimingFunction(name: .easeInEaseOut)` — the cubic bezier
/// (0.42, 0, 0.58, 1) the reference animates every bar hop with —
/// evaluated at x by bisection. Deterministic and exact to ~1e-9, so
/// tests can pin its shape.
fn ease_in_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    // Pin the endpoints exactly: the bisection below converges to
    // ~1e-20 at the endpoints, and a retarget boundary must hold the
    // previous target to the last bit (tests assert exact equality).
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let bezier_x = |u: f64| {
        3.0 * (1.0 - u) * (1.0 - u) * u * 0.42 + 3.0 * (1.0 - u) * u * u * 0.58 + u * u * u
    };
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..32 {
        let mid = (lo + hi) / 2.0;
        if bezier_x(mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let u = (lo + hi) / 2.0;
    // y(u) for the ease-in-out control points reduces to smoothstep.
    u * u * (3.0 - 2.0 * u)
}

/// The displayed scale of equalizer `bar` at `t` seconds into the
/// Listening phase — a pure function of the clock, no RNG state.
///
/// The clock is quantized into 0.12s ticks. Within tick `k` the bar
/// eases from the previous tick's target to tick `k`'s target over the
/// first 0.2s (progress clamped to 0..1), which is exactly the
/// reference's retarget-and-animate loop; a tick is shorter than the
/// ease, so the final hop also reproduces its characteristic snap to
/// the new target at retarget time. Tick 0 eases up from the floor,
/// making `t = 0` byte-identical to the no-clock rest state; a
/// non-finite or negative clock also holds that floor.
pub fn wave_bar_scale(bar: usize, t: f64) -> f64 {
    if !t.is_finite() || t <= 0.0 {
        return WAVE_MIN_SCALE;
    }
    let tick_index = (t / WAVE_TICK_SECS).floor();
    let tick = tick_index as u64;
    let progress = ((t - tick_index * WAVE_TICK_SECS) / WAVE_EASE_SECS).clamp(0.0, 1.0);
    let target = wave_bar_target(tick, bar);
    let previous = if tick == 0 {
        WAVE_MIN_SCALE
    } else {
        wave_bar_target(tick - 1, bar)
    };
    previous + (target - previous) * ease_in_out(progress)
}

/// Hermite smoothstep over [0, 1]: zero slope at both ends, so a fade
/// that rides it never starts or stops with a visible velocity step.
fn smoothstep01(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The content choreography window on a growing morph, as transition
/// progress: content is invisible through 26% (~+90ms of the 350ms
/// transition), smoothsteps in, and is fully present by 89% (~+310ms).
const CONTENT_ALPHA_IN: (f64, f64) = (0.26, 0.89);
/// On a shrinking morph the content is gone linearly by 34% (~120ms),
/// so the card contracts alone after the content has left.
const CONTENT_ALPHA_OUT: f64 = 0.34;

/// The content opacity for a mid-flight morph. Growing content arrives
/// late (hold, then smoothstep in); shrinking content leaves early
/// (linear 1 -> 0 across the first third), which is what makes the
/// close read as the card absorbing a finished surface rather than a
/// slab snapping shut.
fn transition_content_alpha(progress: f64, direction: SpringDirection) -> f64 {
    let t = progress.clamp(0.0, 1.0);
    match direction {
        SpringDirection::Expand => {
            smoothstep01((t - CONTENT_ALPHA_IN.0) / (CONTENT_ALPHA_IN.1 - CONTENT_ALPHA_IN.0))
        }
        SpringDirection::Collapse => (1.0 - t / CONTENT_ALPHA_OUT).clamp(0.0, 1.0),
        SpringDirection::Hover => t,
    }
}

/// Interpolate from the actually displayed frame. Comparing against the
/// previous *target* would reverse collapse after its first frame, because
/// that target is already idle while the outgoing card is still on screen.
fn transition_frame(from: &RenderState, target: &RenderState, progress: f64) -> RenderState {
    if progress >= 1.0 {
        return target.clone();
    }
    let from_extension = card_extension(from);
    let target_extension = card_extension(target);
    // Direction-aware spring: growing uses the eager under-damped open
    // spring, shrinking uses the smooth critically damped close spring.
    let growing = target_extension >= from_extension;
    let direction = if growing {
        SpringDirection::Expand
    } else {
        SpringDirection::Collapse
    };
    let extension =
        from_extension + (target_extension - from_extension) * eased_expand(progress, direction);
    let mut frame = if growing {
        target.clone()
    } else {
        from.clone()
    };
    frame.expand = 1.0;
    frame.animating_direction = direction;
    let full_extension = card_extension(&frame);
    frame.expand = if full_extension > 0.0 {
        extension / full_extension
    } else {
        1.0
    };
    // The content choreography rides the same progress: late-in on a
    // grow, early-out on a shrink. Static (non-transition) frames keep
    // the constructor's 1.0 untouched.
    frame.content_alpha = transition_content_alpha(progress, direction);
    frame
}

/// The NotchShape silhouette as a paint layer: square at the top (it
/// continues into the physical notch), rounded lower corners (the
/// reference's notch bottom radius). Solid black — the same treatment
/// the hardware notch gets from the bezel.
fn notch_shape_layer(x: f64, y: f64, w: f64, h: f64, id: &'static str) -> Layer {
    // A true black silhouette that reads as a continuation of the bezel,
    // rather than a generic translucent rectangle. The radii are assigned by
    // the phase layout and morphed by `compress_to_expand`.
    Layer {
        x,
        y,
        w,
        h,
        r: SURFACE.0,
        g: SURFACE.1,
        b: SURFACE.2,
        a: SURFACE.3,
        top_radius: CARD_COLLAPSED_TOP_RADIUS,
        bottom_radius: CARD_COLLAPSED_BOTTOM_RADIUS,
        action: None,
        id,
    }
}

/// Pure hit-test: the topmost layer whose rect contains `(x, y)`, with
/// the point in logical surface coordinates (0,0 at the top-left).
pub fn hit_test(layers: &[Layer], x: f64, y: f64) -> Option<&Layer> {
    layers.iter().rev().find(|layer| {
        x >= layer.x && x <= layer.x + layer.w && y >= layer.y && y <= layer.y + layer.h
    })
}

/// Canonical surface coordinates are top-left (the same orientation used by
/// `BitmapData`, layout, and hit-testing).  AppKit mouse locations and its
/// native unflipped view are bottom-left, so the boundary conversion is one
/// vertical reflection and nothing else.
pub fn appkit_to_bitmap_point(point: (f64, f64), height: f64) -> (f64, f64) {
    (point.0, height - point.1)
}

/// The inverse of [`appkit_to_bitmap_point`], useful for geometry tests and
/// for documenting the image draw transform's exact mapping.
pub fn bitmap_to_appkit_point(point: (f64, f64), height: f64) -> (f64, f64) {
    (point.0, height - point.1)
}

/// Spawns the surface thread (macOS only; honest `None` elsewhere).
pub fn spawn() -> Option<NotchSurface> {
    if !cfg!(target_os = "macos") {
        return None;
    }

    let mode = NotchMode::from_env();
    if mode == NotchMode::Hidden {
        return None;
    }

    // A live surface requires a physical notch in every mode. Synthetic
    // geometry is only for deterministic verification, never a fallback pill.
    let geometry = probe_notch_geometry();
    let notch = geometry.map(|g| (g.notch_w, g.notch_h));
    let foreign_notch_windows = if mode == NotchMode::Auto && geometry.is_some() {
        occupancy_probe()
    } else {
        0
    };
    let decision = mode.decision(notch, foreign_notch_windows)?;
    let geometry = geometry?;

    let (input_tx, input_rx) = mpsc::channel::<SurfaceInput>();
    let (action_tx, action_rx) = mpsc::channel::<SurfaceAction>();

    let render_state = std::sync::Arc::new(std::sync::Mutex::new(RenderState::default()));
    let thread_render_state = render_state.clone();

    let steps_tx = input_tx.clone();
    let thread = std::thread::Builder::new()
        .name("evo-notch-surface".to_string())
        .spawn(move || {
            surface_main(
                input_rx,
                action_tx,
                steps_tx,
                thread_render_state,
                decision,
                geometry,
            );
        })
        .ok();

    thread.map(|_| NotchSurface {
        input: input_tx,
        actions: action_rx,
        decision,
    })
}

/// Accumulates horizontal drag travel (points; negative = leftward) and
/// returns every whole ±1 carousel step it crosses, consuming the travel:
/// each 24pt of net movement flips one item (drag left = next, `+1`).
/// Pure arithmetic so the swipe math is testable without AppKit.
fn drag_steps(accum: &mut f64, delta_x: f64) -> Vec<isize> {
    const DRAG_STEP_PTS: f64 = 24.0;
    *accum += delta_x;
    let mut steps = Vec::new();
    while *accum <= -DRAG_STEP_PTS {
        *accum += DRAG_STEP_PTS;
        steps.push(1);
    }
    while *accum >= DRAG_STEP_PTS {
        *accum -= DRAG_STEP_PTS;
        steps.push(-1);
    }
    steps
}

/// The action a clicked layer dispatches on the current surface. Which
/// work the Open / Restore chip means: the focused work when the browse
/// carousel is up, the focused candidate in the choice carousel, else
/// the "continue the most recent work" intent (the agent picks the
/// daemon's first listing). The rack chip opens the browse; steps and
/// dismiss keep their uniform verbs.
fn dispatch_for_action(state: &RenderState, action: OptionAction) -> SurfaceAction {
    match action {
        OptionAction::Restore => {
            if let Some(browse) = state.browse.as_ref() {
                SurfaceAction::Choose(browse.id.clone())
            } else {
                state
                    .choice
                    .as_ref()
                    .and_then(|c| c.candidates.get(c.index))
                    .map(|c| {
                        SurfaceAction::ChooseCandidate(evo_voice::VoiceCandidate {
                            work_id: c.work_id.clone(),
                            title: c.title.clone(),
                            selection: c.selection.clone(),
                        })
                    })
                    .unwrap_or(SurfaceAction::ContinueRecent)
            }
        }
        OptionAction::OpenRack => SurfaceAction::Browse,
        OptionAction::Dismiss => SurfaceAction::Dismiss,
        OptionAction::Next => SurfaceAction::ChoiceStep(1),
        OptionAction::Prev => SurfaceAction::ChoiceStep(-1),
    }
}

/// Routes one carousel step to whichever list is live: an open browse
/// wins (explicit user intent), else the choice phase steps as before.
/// Extracted so tests exercise exactly the routing `surface_main` uses.
fn step_open_carousel(engine: &mut NotchEngine, delta: isize, now: Instant) {
    if engine.browse().is_some() {
        engine.browse_step(delta);
    } else {
        engine.choice_step(delta, now);
    }
}

#[cfg(target_os = "macos")]
fn surface_main(
    input_rx: mpsc::Receiver<SurfaceInput>,
    action_tx: mpsc::Sender<SurfaceAction>,
    steps_tx: mpsc::Sender<SurfaceInput>,
    render_state: std::sync::Arc<std::sync::Mutex<RenderState>>,
    decision: SurfaceDecision,
    geometry: NotchGeometry,
) {
    let mut engine = NotchEngine::new();
    engine.owns_strip = decision == SurfaceDecision::Own;
    let mut rack_count: Option<usize> = None;
    // Measure the hardware notch once (safe-area probe); the drawn card
    // is anchored to this exact rect so expansions read as the notch
    // growing, never as an overlay over it.
    let (notch_w, notch_h) = (geometry.notch_w, geometry.notch_h);
    eprintln!("EVO notch: hardware notch {notch_w:.0}x{notch_h:.0}pt acquired");
    let mut host = match EvoNotchHost::open_for_geometry(geometry) {
        Ok(host) => Some(host),
        Err(err) => {
            eprintln!("EVO notch surface unavailable: {err}");
            None
        }
    };
    if let Some(host) = host.as_mut() {
        host.inject_action_sender(action_tx.clone());
        // Drag swipes step the surface's own engine directly.
        host.inject_step_sender(steps_tx);
    }
    // ── The eased render loop ───────────────────────────────────
    // The direction-aware springs arrive here as one eased factor:
    // expansion eases with response 0.42 / damping 0.8 (fast off the
    // mark, soft settle), collapse with response 0.45 / damping 1.0
    // (smooth approach, no bounce). Frames are DEADLINE-ALIGNED at
    // 16_667us (~60fps): frame n of a transition is due at
    // `start + n * ANIM_FRAME` and the loop sleeps `due - now`, so the
    // frame period is the period itself — not the period plus the
    // frame's own work (~25ms ≈ 39fps before). Transcript-only updates
    // re-render once without touching the animation.
    const PHASE_TRANSITION: Duration = Duration::from_millis(350);
    const ANIM_FRAME: Duration = Duration::from_micros(16_667);
    // The listening equalizer's repaint cadence: 24fps — exactly the
    // reference's `preferredFrameRateRange` for its bars. The loop wakes
    // on this while the engine is Listening even with no input and no
    // engine deadline, so the bars actually breathe on screen.
    const WAVE_FRAME: Duration = Duration::from_micros(41_667);
    // Paint the initial hidden frame immediately so the transparent
    // bitmap is in the view before the panel can ever be ordered front;
    // idle is an ordered-out state, so nothing flashes while the surface
    // thread blocks on its input channel.
    let mut shown = RenderState::from_engine_with_geometry(&engine, rack_count, geometry, decision);
    if let Some(host) = host.as_ref() {
        host.render(&shown);
    }
    *render_state.lock().unwrap() = shown.clone();
    // Yield once so the main-thread render dispatch lands before we block
    // on the input channel — the first frame must be visible.
    std::thread::yield_now();
    std::thread::yield_now();
    // The live transition: (outgoing frame, deadline anchor, next frame
    // index). Frame n is due at `start + n * ANIM_FRAME`.
    let mut transition: Option<(RenderState, Instant, u32)> = None;
    let mut verify = None;
    // The equalizer clock: set when the phase enters Listening, cleared
    // when it leaves, so `listening_t` honestly reads "seconds since
    // listening started" and never jumps mid-listen.
    let mut listening_since: Option<Instant> = None;
    loop {
        // Deadline alignment: while animating, sleep exactly until frame
        // n's deadline (`start + n * ANIM_FRAME`); otherwise block on the
        // next input (the agent's heartbeat loop stays idle-cheap). If
        // the work overran the deadline the wait is zero, so the loop
        // renders immediately and the schedule skips forward — drift is
        // never accumulated into later frames.
        let wait = match transition.as_ref() {
            Some((_, start, frame_index)) => {
                Some((*start + ANIM_FRAME * *frame_index).saturating_duration_since(Instant::now()))
            }
            None => {
                let deadline_wait = engine
                    .deadline()
                    .map(|deadline| deadline.saturating_duration_since(Instant::now()));
                // The breather: while Listening, the surface repaints on
                // its own 24fps cadence (merged with any engine deadline,
                // whichever comes first). Every other phase keeps the old
                // behavior: block on input until a deadline exists.
                let wave_wait = (engine.voice.phase == crate::notch::VoicePhase::Listening)
                    .then_some(WAVE_FRAME);
                match (deadline_wait, wave_wait) {
                    (Some(deadline), Some(wave)) => Some(deadline.min(wave)),
                    (Some(deadline), None) => Some(deadline),
                    (None, Some(wave)) => Some(wave),
                    (None, None) => None,
                }
            }
        };
        let input = if let Some(wait) = wait {
            match input_rx.recv_timeout(wait) {
                Ok(input) => Some(input),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match input_rx.recv() {
                Ok(input) => Some(input),
                Err(_) => break,
            }
        };
        if let Some(input) = input {
            match input {
                SurfaceInput::Voice(event) => {
                    engine.voice_event(&event, Instant::now());
                }
                SurfaceInput::ChoiceStep(delta) => {
                    // A chip click or a drag swipe: focus moves, and the
                    // new title is what the next render speaks.
                    step_open_carousel(&mut engine, delta, Instant::now());
                }
                SurfaceInput::Works(entries) => {
                    // The agent's WORKS listing: the browse carousel is
                    // the surface now (an empty listing closes any open
                    // browse — the engine owns that rule).
                    engine.show_works(entries);
                }
                SurfaceInput::Dismiss => {
                    engine.dismiss(Instant::now());
                    rack_count = None;
                    verify = None;
                }
                SurfaceInput::Rack(count) => {
                    rack_count = Some(count);
                    if count > 0 {
                        engine.show_live(ContentId::EvoRack, crate::notch::priority::RACK, true);
                    } else {
                        engine.hide_live(ContentId::EvoRack);
                    }
                }
                SurfaceInput::VerifyCase(case) => {
                    verify = case;
                }
                SurfaceInput::Hover(idx) => {
                    // Update hover state directly on the shown render state
                    // (the engine doesn't track hover; it's purely visual)
                    shown.hovered_chip = idx;
                }
                SurfaceInput::Never => {}
                SurfaceInput::Quit => {
                    if let Some(host) = host.as_ref() {
                        host.order_out();
                    }
                    break;
                }
            }
        }
        engine.tick(Instant::now());
        let mut target =
            RenderState::from_engine_with_geometry(&engine, rack_count, geometry, decision);
        target.verify = verify;
        let now = Instant::now();
        let from_ext = card_extension(&shown);
        let to_ext = card_extension(&target);
        if (to_ext - from_ext).abs() > 0.5
            && transition.is_none()
            && target.verify.is_none()
            && shown.verify.is_none()
        {
            // Anchor the deadline grid at the transition's own start:
            // frame 0 is due immediately (the frame that starts the
            // motion), frame n >= 1 one ANIM_FRAME later each.
            transition = Some((shown.clone(), now, 0));
            // The morph's start gets one alignment haptic, in both
            // directions — the same "the surface moved" tick the
            // reference app performs when its notch grows or relaxes.
            haptic_alignment();
        }
        if let Some((_, start, frame_index)) = transition.as_ref() {
            // An input may wake the loop before this frame's deadline.
            // Rendering now would put an off-grid frame on screen and
            // pull every later deadline forward, so wait out the
            // remainder (the next iteration sleeps `due - now`) and let
            // the due frame carry the coalesced input.
            if now < *start + ANIM_FRAME * *frame_index {
                continue;
            }
        }
        let end_of_transition = transition.as_ref().is_some_and(|(_, start, _)| {
            now.duration_since(*start).as_secs_f64() >= PHASE_TRANSITION.as_secs_f64()
        });
        let mut frame = if end_of_transition {
            transition = None;
            target.clone()
        } else {
            match transition.as_mut() {
                Some((from, start, frame_index)) => {
                    let t =
                        now.duration_since(*start).as_secs_f64() / PHASE_TRANSITION.as_secs_f64();
                    *frame_index = frame_index.saturating_add(1);
                    transition_frame(from, &target, t)
                }
                None => target.clone(),
            }
        };
        // The equalizer clock rides the rendered frame (also mid
        // expand/collapse, where `transition_frame` would otherwise carry
        // the outgoing phase's state). Every other phase renders with
        // `listening_t == None`, which holds the bars at their minimum
        // scale — the same deterministic rest state tests render.
        if engine.voice.phase == crate::notch::VoicePhase::Listening {
            let since = *listening_since.get_or_insert(now);
            frame.listening_t = Some(now.duration_since(since).as_secs_f64());
        } else {
            listening_since = None;
        }
        if let Some(host) = host.as_ref() {
            host.render(&frame);
        }
        shown = target;
        *render_state.lock().unwrap() = frame;
    }
    if let Some(host) = host.as_ref() {
        host.order_out();
    }
}

// ─── The live pixel surface: NSPanel + EvoNotchView ─────────────────────────

#[cfg(target_os = "macos")]
use std::os::raw::{c_char, c_void};

/// The host owns an `NSPanel` with an `EvoNotchView` (an NSView
/// subclass registered at runtime) filling it. Rendering is immediate:
/// [`EvoNotchHost::render`] fills the panel's NSBitmapImageRep-backed
/// buffer from [`surface_layouts`], then marks it dirty so AppKit
/// composites the pixels — deterministic, no CA/WebView layering.
#[cfg(target_os = "macos")]
pub struct EvoNotchHost {
    panel: objc::Id,
    view: objc::Id,
    /// Shared verify hook: the demo asserts over this.
    state: std::sync::Arc<std::sync::Mutex<RenderState>>,
    /// Click actions out to the app (verified by the e2e demo).
    actions: Option<mpsc::Sender<SurfaceAction>>,
    /// Whether the panel is currently ordered front. `render` issues
    /// `orderFrontRegardless` / `orderOut:` only when this flips, instead
    /// of re-ordering the panel on every frame.
    visible: std::sync::atomic::AtomicBool,
}

#[cfg(target_os = "macos")]
pub(crate) mod objc {
    use super::*;

    use std::os::raw::{c_char, c_void};

    // Share one type identity for each ObjC runtime primitive with
    // presence::imp so the two crates' extern "C" declarations of the
    // same C symbol (class_addMethod etc.) agree at the type level and
    // the clashing_extern_declarations lint stays quiet.
    pub type Bool = crate::presence::imp::Bool;
    pub type Id = crate::presence::imp::Id;
    pub type Class = crate::presence::imp::Class;
    pub type Sel = crate::presence::imp::Sel;
    pub type Imp = crate::presence::imp::Imp;
    pub type Ivar = crate::presence::imp::Ivar;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct NsRect {
        pub x: f64,
        pub y: f64,
        pub w: f64,
        pub h: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct NsPoint {
        pub x: f64,
        pub y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct NsEdgeInsets {
        pub top: f64,
        pub left: f64,
        pub bottom: f64,
        pub right: f64,
    }

    #[link(name = "objc")]
    #[link(name = "Foundation", kind = "framework")]
    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        pub fn objc_getClass(name: *const c_char) -> Class;
        pub fn objc_allocateClassPair(
            superclass: Class,
            name: *const c_char,
            extra: usize,
        ) -> Class;
        pub fn objc_registerClassPair(cls: Class);
        pub fn class_addMethod(cls: Class, name: Sel, imp: Imp, types: *const c_char) -> Bool;
        pub fn class_addIvar(
            cls: Class,
            name: *const c_char,
            size: usize,
            align: u8,
            types: *const c_char,
        ) -> Bool;
        pub fn object_setInstanceVariable(obj: Id, name: *const c_char, value: *mut c_void)
        -> Ivar;
        pub fn object_getInstanceVariable(
            obj: Id,
            name: *const c_char,
            out: *mut *mut c_void,
        ) -> Ivar;
        pub fn objc_msgSend();
        pub fn sel_registerName(name: *const c_char) -> Sel;

        fn CGDataProviderCreateWithData(
            info: *const c_void,
            data: *const c_void,
            size: usize,
            release_data: Option<extern "C" fn(*const c_void, *const c_void, usize)>,
        ) -> CGDataProviderRef;
        fn CGDataProviderRelease(provider: CGDataProviderRef);
        fn CGImageCreate(
            width: usize,
            height: usize,
            bits_per_component: usize,
            bits_per_pixel: usize,
            bytes_per_row: usize,
            space: CGColorSpaceRef,
            bitmap_info: u32,
            provider: CGDataProviderRef,
            decode: *const *const f64,
            should_interpolate: Bool,
            intent: u32,
        ) -> CGImageRef;
        fn CGImageRelease(image: CGImageRef);
        fn CGContextSaveGState(context: CGContextRef);
        fn CGContextRestoreGState(context: CGContextRef);
        fn CGContextClearRect(context: CGContextRef, rect: NsRect);
        fn CGContextDrawImage(context: CGContextRef, rect: NsRect, image: CGImageRef);

        // GCD: AppKit windows must be created and ordered on the main
        // thread (NSInternalInconsistencyException otherwise), and the
        // notch surface lives on its own thread. `dispatch_get_main_queue`
        // is a C macro for the `_dispatch_main_q` singleton, so bind the
        // global directly.
        static _dispatch_main_q: c_void;
        fn dispatch_sync_f(
            queue: *const c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
        fn dispatch_async_f(
            queue: *const c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );

        fn pthread_main_np() -> i32;
        fn CGDisplayIsBuiltin(display: u32) -> u32;

        // CoreText: thread-safe glyph rasterization into our own
        // buffer. Used from the paint path (any thread); no AppKit
        // object is touched, so this is not an on_main concern.
        fn CFStringCreateWithCString(
            alloc: *const c_void,
            text: *const c_char,
            encoding: u32,
        ) -> CFStringRef;
        fn CFAttributedStringCreate(
            alloc: *const c_void,
            text: CFStringRef,
            attributes: CFDictionaryRef,
        ) -> CTAttributedStringRef;
        fn CFDictionaryCreateMutable(
            alloc: *const c_void,
            capacity: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> CFDictionaryRef;
        fn CFDictionarySetValue(dict: CFDictionaryRef, key: *const c_void, value: *const c_void);
        fn CTFontCreateUIFontForLanguage(
            ui_font: u8,
            size: f64,
            language: *const c_void,
        ) -> CTFontRef;
        /// Named-face lookup: the explicit fallback when the UI font type
        /// cannot be resolved.
        fn CTFontCreateWithName(name: CFStringRef, size: f64, matrix: *const c_void) -> CTFontRef;
        /// The font's real ascent in points. Used to place the baseline far
        /// enough below a row's top edge that no ascender is clipped.
        fn CTFontGetAscent(font: CTFontRef) -> f64;
        fn CTLineCreateWithAttributedString(text: CTAttributedStringRef) -> CTLineRef;
        fn CTLineDraw(line: CTLineRef, context: CGContextRef);
        /// The line's typographic width, used by the tabular-digit
        /// verification to measure real advances through the same
        /// attribute pipeline the paint path uses.
        fn CTLineGetTypographicBounds(
            line: CTLineRef,
            ascent: *mut f64,
            descent: *mut f64,
            leading: *mut f64,
        ) -> f64;
        /// Font-feature plumbing (tabular digits): copy the descriptor,
        /// apply the feature, materialize a font copy. Both +1.
        fn CTFontCopyFontDescriptor(font: CTFontRef) -> CTFontDescriptorRef;
        fn CTFontDescriptorCreateCopyWithFeature(
            original: CTFontDescriptorRef,
            feature_type: CFNumberRef,
            feature_selector: CFNumberRef,
        ) -> CTFontDescriptorRef;
        fn CTFontCreateCopyWithAttributes(
            font: CTFontRef,
            size: f64,
            matrix: *const c_void,
            attributes: CTFontDescriptorRef,
        ) -> CTFontRef;
        /// CFNumber for the feature identifiers and the kern value.
        fn CFNumberCreate(
            alloc: *const c_void,
            number_type: isize,
            value: *const c_void,
        ) -> CFNumberRef;
        fn CFRelease(object: *const c_void);
        // CGBitmapContext: an off-screen RGBA canvas we rasterize glyphs
        // into, then alpha-composite into the layer rects.
        fn CGBitmapContextCreate(
            data: *mut c_void,
            width: usize,
            height: usize,
            bits_per_component: usize,
            bytes_per_row: usize,
            space: CGColorSpaceRef,
            bitmap_info: u32,
        ) -> CGContextRef;
        fn CGContextSetRGBFillColor(context: CGContextRef, r: f64, g: f64, b: f64, a: f64);
        // NOTE: there is deliberately no CGContextScaleCTM binding. Every
        // surface in this module is top-left oriented and converts exactly
        // once at a named boundary; a Y reflection at draw time is what
        // caused the mirrored composite and the mirrored glyphs.
        fn CGContextTranslateCTM(context: CGContextRef, tx: f64, ty: f64);
        fn CGColorSpaceCreateDeviceRGB() -> CGColorSpaceRef;
        fn CGColorSpaceRelease(space: CGColorSpaceRef);
        fn CGColorCreate(space: CGColorSpaceRef, components: *const f64) -> CGColorRef;
        // NOTE: the attrs dictionary holds its values WITHOUT retaining
        // (null callbacks). Every Create/Copy object built for one line is
        // therefore owned by a `CfOwned` guard that lives exactly until the
        // line has been drawn — never released early (that is the
        // objc_retain SIGSEGV the old code documented), and never leaked:
        // the guards release on every return path, early returns included.
    }

    // Reused opaque type spellings inside the objc module.
    type CTAttributedStringRef = *const c_void;
    type CTLineRef = *const c_void;
    type CTFontRef = *const c_void;
    type CGColorRef = *const c_void;
    type CGContextRef = *mut c_void;
    type CGColorSpaceRef = *const c_void;
    type CGDataProviderRef = *const c_void;
    type CGImageRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFNumberRef = *const c_void;
    type CTFontDescriptorRef = *const c_void;

    /// True when this thread is macOS's main thread — where NSWindow
    /// creation is legal. `pthread_main_np` is documented to return 1 on
    /// the main thread and 0 otherwise.
    pub fn is_main_thread() -> bool {
        unsafe { pthread_main_np() == 1 }
    }

    /// Runs `f` on the macOS main thread and hands back its result,
    /// blocking this thread until the closure has run. Safe from any
    /// thread: already on main executes inline; otherwise dispatches
    /// to the main queue with `dispatch_sync_f`. `None` only when the
    /// main queue does not exist (no AppKit runloop to hop to).
    pub fn on_main<R: 'static>(f: impl FnOnce() -> R + 'static) -> Option<R> {
        // Carrying the closure and its result back across the C
        // trampoline needs a stable address both sides agree on: a
        // boxed cell the context pointer owns for the dispatch. The
        // pointer types involved are ObjC ids — deliberately not Send —
        // so the contract is documented here rather than in the type
        // system: exactly one block runs the closure once, then the
        // dispatching thread reclaims the box.
        #[repr(C)]
        struct State<R> {
            f: Option<Box<dyn FnOnce() -> R>>,
            out: Option<R>,
        }
        let state = Box::new(State::<R> {
            f: Some(Box::new(f)),
            out: None,
        });
        let ctx = Box::into_raw(state) as *mut c_void;
        unsafe extern "C" fn trampoline<R>(ctx: *mut c_void) {
            let state = unsafe { &mut *(ctx as *mut State<R>) };
            if let Some(f) = state.f.take() {
                state.out = Some(f());
            }
        }
        if is_main_thread() {
            unsafe { trampoline::<R>(ctx) };
            let state = unsafe { Box::from_raw(ctx as *mut State<R>) };
            return state.out;
        }
        unsafe {
            // The main queue singleton is always present (it ships with
            // libdispatch; an AppKit app is already pumping it).
            let queue = &_dispatch_main_q as *const c_void;
            let work: extern "C" fn(*mut c_void) =
                std::mem::transmute(trampoline::<R> as *const ());
            dispatch_sync_f(queue, ctx, work);
            let state = Box::from_raw(ctx as *mut State<R>);
            state.out
        }
    }

    /// Like [`on_main`], but never waits: the closure runs on the main
    /// queue whenever the runloop gets to it. The per-frame view
    /// invalidation uses this so the surface thread is not serialized
    /// behind AppKit — the main queue coalesces the dirty marks, and the
    /// actual `drawRect:` lands at the next display (vsync) instead of
    /// being flushed synchronously by the render loop. The same
    /// documented cross-thread contract as `on_main` applies, except that
    /// the main-queue block itself owns (and frees) the boxed cell; if the
    /// main queue never runs, the box leaks once, which is the bounded
    /// cost of an un-pumped AppKit session.
    pub fn on_main_async(f: impl FnOnce() + 'static) {
        #[repr(C)]
        struct State {
            f: Option<Box<dyn FnOnce()>>,
        }
        unsafe extern "C" fn trampoline(ctx: *mut c_void) {
            // Reclaim the box so an async closure is freed exactly once.
            let mut state = unsafe { Box::from_raw(ctx as *mut State) };
            if let Some(f) = state.f.take() {
                f();
            }
        }
        let state = Box::new(State {
            f: Some(Box::new(f)),
        });
        let ctx = Box::into_raw(state) as *mut c_void;
        unsafe {
            // The main queue singleton is always present (it ships with
            // libdispatch; an AppKit app is already pumping it).
            let queue = &_dispatch_main_q as *const c_void;
            let work: extern "C" fn(*mut c_void) = std::mem::transmute(trampoline as *const ());
            dispatch_async_f(queue, ctx, work);
        }
    }

    pub const IVAR_NAME: &[u8] = b"rustHost\0";
    pub const POINTER_TYPES: &[u8] = b"^v\0";
    // AppKit sends drawRect: with NSRect by value. Registering a pointer (and
    // an extra object argument) makes the runtime call the IMP with a
    // different ABI, so the view never reliably paints its transparent
    // bitmap.
    pub const VOID_SELF_NSRECT: &[u8] = b"v@:{NSRect=dddd}\0";
    pub const BOOL_SELF: &[u8] = b"c@:\0";
    /// `-constrainFrameRect:toScreen:` returns an NSRect by value and takes
    /// (NSRect, id). The struct spelling matches `VOID_SELF_NSRECT` so the
    /// runtime dispatches the IMP with the struct-return ABI the C signature
    /// `extern "C" fn(Id, Sel, NsRect, Id) -> NsRect` actually implements.
    pub const NSRECT_SELF_NSRECT_ID: &[u8] = b"{NSRect=dddd}@:{NSRect=dddd}@\0";

    pub fn sel(bytes: &'static [u8]) -> Sel {
        unsafe { sel_registerName(bytes.as_ptr() as *const c_char) }
    }

    pub fn class_of(name: &[u8]) -> Class {
        unsafe { objc_getClass(name.as_ptr() as *const c_char) }
    }

    /// Prefers the built-in notched display even when an external monitor is
    /// currently the macOS main screen. This mirrors the reference app's
    /// display selection instead of blindly trusting `mainScreen`.
    pub fn preferred_screen() -> Id {
        unsafe {
            let screen_class = class_of(b"NSScreen\0");
            if screen_class.is_null() {
                return std::ptr::null_mut();
            }
            let screens: Id = send0(screen_class as Id, sel(b"screens\0"));
            if !screens.is_null() {
                let count: usize = send0(screens, sel(b"count\0"));
                let mut first_notched: Id = std::ptr::null_mut();
                for index in 0..count {
                    let screen: Id = send1(screens, sel(b"objectAtIndex:\0"), index);
                    if screen.is_null() {
                        continue;
                    }
                    let left: NsRect = send0(screen, sel(b"auxiliaryTopLeftArea\0"));
                    let right: NsRect = send0(screen, sel(b"auxiliaryTopRightArea\0"));
                    let safe: NsEdgeInsets = send0(screen, sel(b"safeAreaInsets\0"));
                    if (left.w > 0.0 || right.w > 0.0) && safe.top > 0.0 {
                        if first_notched.is_null() {
                            first_notched = screen;
                        }
                        if is_builtin_screen(screen) {
                            return screen;
                        }
                    }
                }
                if !first_notched.is_null() {
                    return first_notched;
                }
            }
            send0(screen_class as Id, sel(b"mainScreen\0"))
        }
    }

    /// NSScreen does not expose its display id as a plain Rust field. The
    /// device description carries `NSScreenNumber`, which lets Auto prefer
    /// the Mac's built-in notched display when an external notched display is
    /// also connected.
    unsafe fn is_builtin_screen(screen: Id) -> bool {
        unsafe {
            let description: Id = send0(screen, sel(b"deviceDescription\0"));
            if description.is_null() {
                return false;
            }
            let string_class = class_of(b"NSString\0");
            if string_class.is_null() {
                return false;
            }
            let key: Id = send1(
                string_class as Id,
                sel(b"stringWithUTF8String:\0"),
                b"NSScreenNumber\0".as_ptr() as *const c_char,
            );
            if key.is_null() {
                return false;
            }
            let number: Id = send1(description, sel(b"objectForKey:\0"), key);
            if number.is_null() {
                return false;
            }
            let display_id: u32 = send0(number, sel(b"unsignedIntValue\0"));
            CGDisplayIsBuiltin(display_id) != 0
        }
    }

    pub unsafe fn send0<T>(receiver: Id, selector: Sel) -> T {
        unsafe {
            let f: extern "C" fn(Id, Sel) -> T = std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector)
        }
    }

    pub unsafe fn send1<T, A>(receiver: Id, selector: Sel, a: A) -> T {
        unsafe {
            let f: extern "C" fn(Id, Sel, A) -> T = std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, a)
        }
    }

    pub unsafe fn send2<T, A, B>(receiver: Id, selector: Sel, a: A, b: B) -> T {
        unsafe {
            let f: extern "C" fn(Id, Sel, A, B) -> T =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, a, b)
        }
    }

    #[allow(dead_code)]
    pub unsafe fn send3<T, A, B, C>(receiver: Id, selector: Sel, a: A, b: B, c: C) -> T {
        unsafe {
            let f: extern "C" fn(Id, Sel, A, B, C) -> T =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, a, b, c)
        }
    }

    #[allow(dead_code)]
    pub unsafe fn send4<T, A, B, C, D>(receiver: Id, selector: Sel, a: A, b: B, c: C, d: D) -> T {
        unsafe {
            let f: extern "C" fn(Id, Sel, A, B, C, D) -> T =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, a, b, c, d)
        }
    }

    #[allow(dead_code)]
    pub unsafe fn send_rect(
        receiver: Id,
        selector: Sel,
        rect: NsRect,
        mask: usize,
        backing: usize,
        defer: isize,
    ) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, NsRect, usize, usize, isize) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, rect, mask, backing, defer)
        }
    }

    /// The registered `EvoNotchView` class: NSView with paint hooks.
    pub fn register_view_class() -> Class {
        let existing = class_of(b"EvoNotchView\0");
        if !existing.is_null() {
            return existing;
        }
        let superclass = class_of(b"NSView\0");
        if superclass.is_null() {
            eprintln!("EVO notch: NSView class unavailable");
            return std::ptr::null_mut();
        }
        let cls = unsafe {
            objc_allocateClassPair(superclass, b"EvoNotchView\0".as_ptr() as *const c_char, 0)
        };
        if cls.is_null() {
            eprintln!("EVO notch: class allocation failed");
            return std::ptr::null_mut();
        }
        // Rust-side pointer: the RenderState + action sender live here.
        unsafe {
            if class_addIvar(
                cls,
                IVAR_NAME.as_ptr() as *const c_char,
                std::mem::size_of::<*mut c_void>(),
                3, // log2 alignment of pointer: 8 bytes
                POINTER_TYPES.as_ptr() as *const c_char,
            ) == 0
            {
                eprintln!("EVO notch: ivar registration failed");
                return std::ptr::null_mut();
            }
            // drawRect: → call through to the Rust painter.
            if class_addMethod(
                cls,
                sel(b"drawRect:\0"),
                {
                    let imp: Imp =
                        std::mem::transmute::<*const (), Imp>(view_draw_rect as *const ());
                    imp
                },
                VOID_SELF_NSRECT.as_ptr() as *const c_char,
            ) == 0
            {
                eprintln!("EVO notch: drawRect: registration failed");
                return std::ptr::null_mut();
            }
            // mouseDown: → dispatch the hit surface action.
            if class_addMethod(
                cls,
                sel(b"mouseDown:\0"),
                {
                    let imp: Imp =
                        std::mem::transmute::<*const (), Imp>(view_mouse_down as *const ());
                    imp
                },
                VOID_AT_AT_TYPES.as_ptr() as *const c_char,
            ) == 0
            {
                eprintln!("EVO notch: mouseDown: registration failed");
                return std::ptr::null_mut();
            }
            // mouseDragged: → the candidate carousel's swipe.
            if class_addMethod(
                cls,
                sel(b"mouseDragged:\0"),
                {
                    let imp: Imp =
                        std::mem::transmute::<*const (), Imp>(view_mouse_dragged as *const ());
                    imp
                },
                VOID_AT_AT_TYPES.as_ptr() as *const c_char,
            ) == 0
            {
                eprintln!("EVO notch: mouseDragged: registration failed");
                return std::ptr::null_mut();
            }
            // Keep the AppKit view in its native bottom-left coordinate
            // system.  BitmapData/layout/hit-testing are all top-left; the
            // single explicit boundary transform lives in draw_bitmap_to_current_context
            // and appkit_to_bitmap_point below.
            if class_addMethod(
                cls,
                sel(b"isFlipped\0"),
                {
                    let imp: Imp =
                        std::mem::transmute::<*const (), Imp>(view_is_flipped as *const ());
                    imp
                },
                BOOL_SELF.as_ptr() as *const c_char,
            ) == 0
            {
                eprintln!("EVO notch: isFlipped registration failed");
                return std::ptr::null_mut();
            }
            // A custom view must explicitly report that it is not opaque;
            // otherwise AppKit may composite the view's full backing bounds
            // as an opaque rectangle even though our bitmap has zero alpha
            // outside the notch shape.
            if class_addMethod(
                cls,
                sel(b"isOpaque\0"),
                {
                    let imp: Imp =
                        std::mem::transmute::<*const (), Imp>(view_is_opaque as *const ());
                    imp
                },
                BOOL_SELF.as_ptr() as *const c_char,
            ) == 0
            {
                eprintln!("EVO notch: isOpaque registration failed");
                return std::ptr::null_mut();
            }
            objc_registerClassPair(cls);
            cls
        }
    }

    /// The notch panel is its own nonactivating window, not a view embedded
    /// in Evo's document window. These overrides match the reference panel:
    /// it can never become key or main, even when the pointer crosses it.
    pub fn register_panel_class() -> Class {
        let existing = class_of(b"EvoNotchPanel\0");
        if !existing.is_null() {
            return existing;
        }
        let superclass = class_of(b"NSPanel\0");
        if superclass.is_null() {
            eprintln!("EVO notch: NSPanel class unavailable");
            return std::ptr::null_mut();
        }
        let cls = unsafe {
            objc_allocateClassPair(superclass, b"EvoNotchPanel\0".as_ptr() as *const c_char, 0)
        };
        if cls.is_null() {
            eprintln!("EVO notch: panel class allocation failed");
            return std::ptr::null_mut();
        }
        unsafe {
            for (selector, implementation) in [
                (
                    b"canBecomeKey\0".as_slice(),
                    panel_cannot_become_key as *const (),
                ),
                (
                    b"canBecomeMain\0".as_slice(),
                    panel_cannot_become_main as *const (),
                ),
            ] {
                let imp: Imp = std::mem::transmute(implementation);
                if class_addMethod(cls, sel(selector), imp, BOOL_SELF.as_ptr() as *const c_char)
                    == 0
                {
                    eprintln!("EVO notch: panel method registration failed");
                    return std::ptr::null_mut();
                }
            }
            // The notch overlay must be allowed to own the menu-bar band.
            let constrain: Imp =
                std::mem::transmute::<*const (), Imp>(panel_constrain_frame_rect as *const ());
            if class_addMethod(
                cls,
                sel(b"constrainFrameRect:toScreen:\0"),
                constrain,
                NSRECT_SELF_NSRECT_ID.as_ptr() as *const c_char,
            ) == 0
            {
                eprintln!("EVO notch: constrainFrameRect:toScreen: registration failed");
                return std::ptr::null_mut();
            }
            objc_registerClassPair(cls);
        }
        cls
    }

    pub const VOID_AT_AT_TYPES: &[u8] = b"v@:@\0";

    pub extern "C" fn view_is_flipped(_this: Id, _cmd: Sel) -> Bool {
        0 // native AppKit bottom-left; bitmap conversion is explicit once
    }

    pub extern "C" fn panel_cannot_become_key(_this: Id, _cmd: Sel) -> Bool {
        0
    }

    /// AppKit's own window constraining is what pushes a notch overlay below
    /// the menu bar: `-constrainFrameRect:toScreen:` silently slides any
    /// window whose top edge reaches above the menu-bar band down by exactly
    /// that band's height. The panel is created with its top edge at the
    /// screen top, yet the measured top landed at 32.5pt - the menu-bar
    /// height. Returning the proposed rect UNCHANGED is the standard
    /// requirement for a window that has to cover the physical notch (the
    /// same override boring.notch installs), and it is what puts the card's
    /// top edge back at screen y = 0.
    pub extern "C" fn panel_constrain_frame_rect(
        _this: Id,
        _cmd: Sel,
        proposed: NsRect,
        _screen: Id,
    ) -> NsRect {
        proposed
    }

    pub extern "C" fn panel_cannot_become_main(_this: Id, _cmd: Sel) -> Bool {
        0
    }

    pub extern "C" fn view_draw_rect(this: Id, _cmd: Sel, _rect: NsRect) {
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ =
                object_getInstanceVariable(this, IVAR_NAME.as_ptr() as *const c_char, &mut holder);
            let Some(host_ptr) = (if holder.is_null() {
                None
            } else {
                Some(holder as *mut HostBlob)
            }) else {
                return;
            };
            let scale = view_backing_scale(this);
            paint_view((*host_ptr).painter.clone(), scale);
        }
    }

    pub extern "C" fn view_is_opaque(_this: Id, _cmd: Sel) -> Bool {
        0
    }

    pub extern "C" fn view_mouse_down(this: Id, _cmd: Sel, event: Id) {
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ =
                object_getInstanceVariable(this, IVAR_NAME.as_ptr() as *const c_char, &mut holder);
            let Some(host_ptr) = (if holder.is_null() {
                None
            } else {
                Some(holder as *mut HostBlob)
            }) else {
                return;
            };
            handle_click((*host_ptr).painter.clone(), event);
        }
    }

    pub extern "C" fn view_mouse_dragged(this: Id, _cmd: Sel, event: Id) {
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ =
                object_getInstanceVariable(this, IVAR_NAME.as_ptr() as *const c_char, &mut holder);
            let Some(host_ptr) = (if holder.is_null() {
                None
            } else {
                Some(holder as *mut HostBlob)
            }) else {
                return;
            };
            handle_drag((*host_ptr).painter.clone(), event);
        }
    }

    #[allow(dead_code)]
    pub extern "C" fn view_mouse_entered(this: Id, _cmd: Sel, _event: Id) {
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ =
                object_getInstanceVariable(this, IVAR_NAME.as_ptr() as *const c_char, &mut holder);
            let Some(host_ptr) = (if holder.is_null() {
                None
            } else {
                Some(holder as *mut HostBlob)
            }) else {
                return;
            };
            let painter = (*host_ptr).painter.clone();
            // Find which chip is under the mouse on entry
            if let Some(steps) = painter.steps.as_ref() {
                let _ = steps.send(SurfaceInput::Hover(None)); // Will be updated by mouseMoved
            }
        }
    }

    #[allow(dead_code)]
    pub extern "C" fn view_mouse_exited(this: Id, _cmd: Sel, _event: Id) {
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ =
                object_getInstanceVariable(this, IVAR_NAME.as_ptr() as *const c_char, &mut holder);
            let Some(host_ptr) = (if holder.is_null() {
                None
            } else {
                Some(holder as *mut HostBlob)
            }) else {
                return;
            };
            let painter = (*host_ptr).painter.clone();
            if let Some(steps) = painter.steps.as_ref() {
                let _ = steps.send(SurfaceInput::Hover(None));
            }
        }
    }

    #[allow(dead_code)]
    pub extern "C" fn view_mouse_moved(this: Id, _cmd: Sel, event: Id) {
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ =
                object_getInstanceVariable(this, IVAR_NAME.as_ptr() as *const c_char, &mut holder);
            let Some(host_ptr) = (if holder.is_null() {
                None
            } else {
                Some(holder as *mut HostBlob)
            }) else {
                return;
            };
            let painter = (*host_ptr).painter.clone();
            let state = painter.state.lock().unwrap().clone();
            let layers = super::surface_layouts(&state);
            // Convert event location to bitmap coordinates
            let loc: (f64, f64) = {
                let f: extern "C" fn(Id, Sel) -> super::objc::NsPoint =
                    std::mem::transmute(super::objc::objc_msgSend as *const ());
                let p = f(event, super::objc::sel(b"locationInWindow\0"));
                (p.x, p.y)
            };
            let local = super::appkit_to_bitmap_point(loc, super::CANVAS_SIZE.1);
            // Hit-test against chip layers
            let chip_ids = [
                "chip_restore",
                "chip_rack",
                "chip_dismiss",
                "chip_prev",
                "chip_next",
            ];
            let mut hovered_idx = None;
            for (idx, chip_id) in chip_ids.iter().enumerate() {
                if let Some(layer) = layers.iter().find(|l| l.id == *chip_id) {
                    if local.0 >= layer.x
                        && local.0 <= layer.x + layer.w
                        && local.1 >= layer.y
                        && local.1 <= layer.y + layer.h
                    {
                        hovered_idx = Some(idx);
                        break;
                    }
                }
            }
            if let Some(steps) = painter.steps.as_ref() {
                let _ = steps.send(SurfaceInput::Hover(hovered_idx));
            }
        }
    }

    // The shared state between the view callbacks and Rust-side control.
    pub struct HostBlob {
        pub painter: Painter,
    }

    /// A painter: everything the view needs to draw + dispatch actions.
    #[derive(Clone)]
    pub struct Painter {
        pub state: std::sync::Arc<std::sync::Mutex<RenderState>>,
        pub actions: Option<mpsc::Sender<SurfaceAction>>,
        /// The surface thread's own input (ChoiceStep feedback): the
        /// drag swipe lives HERE, in the surface's engine, so the next
        /// render speaks the new candidate.
        pub steps: Option<mpsc::Sender<SurfaceInput>>,
        /// The last painted bitmap for pixel verification.
        pub bitmap: std::sync::Arc<std::sync::Mutex<Option<BitmapData>>>,
        /// Horizontal drag accumulation for the carousel swipe (points
        /// since the last whole step; leftward travel is negative).
        pub drag_accum: std::sync::Arc<std::sync::Mutex<f64>>,
    }

    /// An RGBA bitmap in row-major top-left logical pixel space.
    #[derive(Clone, PartialEq)]
    pub struct BitmapData {
        pub width: usize,
        pub height: usize,
        pub pixels: Vec<u8>,
    }

    impl BitmapData {
        pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
            let idx = (y * self.width + x) * 4;
            [
                self.pixels[idx],
                self.pixels[idx + 1],
                self.pixels[idx + 2],
                self.pixels[idx + 3],
            ]
        }
    }

    /// Fills `bitmap` from surface_layouts(state), then rasterizes real
    /// glyphs (system SF via CoreText) inside the text-bearing layer
    /// rects. Referenced from the drawRect: IMP and the demo.
    fn shape_contains(layer: &Layer, px: f64, py: f64) -> bool {
        // Rounding is a property of the layer, not of its id: the card and
        // the notch band carry the NotchShape radii, chips carry half their
        // height (a pill), and the mic dot carries half its side (a circle).
        // A layer with no radii is its plain rect.
        if layer.top_radius <= 0.0 && layer.bottom_radius <= 0.0 {
            return true;
        }
        let x = px - layer.x;
        let y = py - layer.y;
        let w = layer.w;
        let h = layer.h;
        if x < 0.0 || y < 0.0 || x >= w || y >= h {
            return false;
        }

        // Matches the reference NotchShape: a shallow top curve that
        // continues into the real bezel and fuller rounded lower corners.
        let top = layer.top_radius.min(w * 0.5).min(h * 0.5);
        let bottom = layer.bottom_radius.min(w * 0.5).min(h * 0.5);
        if x < top && y < top {
            let dx = x - top;
            let dy = y - top;
            return dx * dx + dy * dy <= top * top;
        }
        if x > w - top && y < top {
            let dx = x - (w - top);
            let dy = y - top;
            return dx * dx + dy * dy <= top * top;
        }
        if x < bottom && y > h - bottom {
            let dx = x - bottom;
            let dy = y - (h - bottom);
            return dx * dx + dy * dy <= bottom * bottom;
        }
        if x > w - bottom && y > h - bottom {
            let dx = x - (w - bottom);
            let dy = y - (h - bottom);
            return dx * dx + dy * dy <= bottom * bottom;
        }
        true
    }

    /// The expanded card's drop shadow, in points: black at 55%, blur
    /// ~12pt (the falloff's compact support), offset (0, 4). The card
    /// sits at the screen's top edge when expanded in place, so the
    /// shadow above it is clipped by the physical screen edge — correct,
    /// and unavoidable by construction.
    pub(crate) const CARD_SHADOW_ALPHA: f64 = 0.55;
    pub(crate) const CARD_SHADOW_BLUR: f64 = 12.0;
    pub(crate) const CARD_SHADOW_OFFSET_Y: f64 = 4.0;

    /// The dark chips' hairline rim: white at 12% (the reference's
    /// `stroke(Color.white.opacity(0.12), lineWidth: 1)`), inset one point
    /// inside the pill outline. The caramel Restore chip stays fill-only.
    const CHIP_RIM_INSET: f64 = 1.0;
    const CHIP_RIM_WHITE: f64 = 0.12;

    /// The dark pills that carry the hairline rim (by id).
    fn chip_has_rim(id: &str) -> bool {
        matches!(id, "chip_rack" | "chip_dismiss" | "chip_prev" | "chip_next")
    }

    /// Signed distance from `(px, py)` to the layer's silhouette in
    /// points: negative inside, positive outside. Exact for a rounded
    /// rect with one radius; the card's two radii (top/bottom) are each
    /// used on their own half, joined at the mid-line where both sides
    /// are straight.
    fn silhouette_distance(layer: &Layer, px: f64, py: f64) -> f64 {
        let w = layer.w.max(1.0);
        let h = layer.h.max(1.0);
        let half = if py - layer.y >= h * 0.5 {
            layer.bottom_radius
        } else {
            layer.top_radius
        };
        let r = if half.is_finite() {
            half.max(0.0).min(w * 0.5).min(h * 0.5)
        } else {
            0.0
        };
        let qx = (px - layer.x - w * 0.5).abs() - (w * 0.5 - r);
        let qy = (py - layer.y - h * 0.5).abs() - (h * 0.5 - r);
        let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
        outside + qx.max(qy).min(0.0) - r
    }

    /// Source-over blend of a flat source color (`src`, alpha `a`) into
    /// one premultiplied-in-float RGBA pixel at `pixels[idx..idx + 4]`.
    fn blend_pixel(pixels: &mut [u8], idx: usize, src: (f64, f64, f64), a: f64) {
        let dst_a = pixels[idx + 3] as f64 / 255.0;
        let out_a = a + dst_a * (1.0 - a);
        let mix = |d: u8, s: f64| -> u8 {
            ((s * a + d as f64 / 255.0 * dst_a * (1.0 - a)) / out_a.max(f64::EPSILON) * 255.0)
                .clamp(0.0, 255.0) as u8
        };
        pixels[idx] = mix(pixels[idx], src.0);
        pixels[idx + 1] = mix(pixels[idx + 1], src.1);
        pixels[idx + 2] = mix(pixels[idx + 2], src.2);
        pixels[idx + 3] = (out_a * 255.0) as u8;
    }

    /// Clamped pixel-space bounding box of a layer's rect, padded by
    /// `pad` points. Every pass below runs exactly inside it, so no pass
    /// can index outside the canvas.
    fn layer_pixel_box(
        layer: &Layer,
        canvas_w: usize,
        canvas_h: usize,
        scale: f64,
        pad: f64,
    ) -> (usize, usize, usize, usize) {
        let pad = pad.max(0.0).min(4096.0);
        let x0 = ((layer.x - pad) * scale).max(0.0).min(canvas_w as f64) as usize;
        let y0 = ((layer.y - pad) * scale).max(0.0).min(canvas_h as f64) as usize;
        let x1 = ((layer.x + layer.w + pad) * scale)
            .max(0.0)
            .min(canvas_w as f64) as usize;
        let y1 = ((layer.y + layer.h + pad) * scale)
            .max(0.0)
            .min(canvas_h as f64) as usize;
        (x0, y0, x1.max(x0), y1.max(y0))
    }

    /// Draws the card's soft drop shadow into `pixels`: the silhouette
    /// shifted down by [`CARD_SHADOW_OFFSET_Y`] with a quadratic falloff
    /// over [`CARD_SHADOW_BLUR`] points (coverage 1 at the silhouette
    /// edge, 0 at the support boundary, so there is no hard cut).
    ///
    /// This runs BEFORE the card's fill, directly into the canvas: the
    /// opaque card then composites over the shadow's interior, so the
    /// card's own pixels are never tinted and no shadow state can leak
    /// into the chip or glyph draws (there is no graphics state at all —
    /// the pass only writes the pixels it covers, and only black at 55%).
    fn paint_card_shadow(
        pixels: &mut [u8],
        canvas_w: usize,
        canvas_h: usize,
        layer: &Layer,
        scale: f64,
    ) {
        let reach = CARD_SHADOW_BLUR + CARD_SHADOW_OFFSET_Y;
        let (x0, y0, x1, y1) = layer_pixel_box(layer, canvas_w, canvas_h, scale, reach);
        for py in y0..y1 {
            for px in x0..x1 {
                let cx = (px as f64 + 0.5) / scale;
                let cy = (py as f64 + 0.5) / scale;
                let distance = silhouette_distance(layer, cx, cy - CARD_SHADOW_OFFSET_Y);
                let falloff = 1.0 - (distance / CARD_SHADOW_BLUR).clamp(0.0, 1.0);
                let alpha = CARD_SHADOW_ALPHA * falloff * falloff;
                if alpha <= 0.0 {
                    continue;
                }
                let idx = (py * canvas_w + px) * 4;
                blend_pixel(pixels, idx, (0.0, 0.0, 0.0), alpha);
            }
        }
    }

    /// Strokes the dark chips' hairline rim: pixels inside the pill but
    /// within [`CHIP_RIM_INSET`] points of its outline take a 10% white
    /// source-over pass on top of the fill. Drawn per chip, after that
    /// chip's fill and before any glyph compositing.
    fn stroke_chip_rim(
        pixels: &mut [u8],
        canvas_w: usize,
        canvas_h: usize,
        layer: &Layer,
        scale: f64,
        content_alpha: f64,
    ) {
        let inner = Layer {
            x: layer.x + CHIP_RIM_INSET,
            y: layer.y + CHIP_RIM_INSET,
            w: (layer.w - 2.0 * CHIP_RIM_INSET).max(0.0),
            h: (layer.h - 2.0 * CHIP_RIM_INSET).max(0.0),
            top_radius: (layer.top_radius - CHIP_RIM_INSET).max(0.0),
            bottom_radius: (layer.bottom_radius - CHIP_RIM_INSET).max(0.0),
            ..*layer
        };
        let (x0, y0, x1, y1) = layer_pixel_box(layer, canvas_w, canvas_h, scale, 0.0);
        for py in y0..y1 {
            for px in x0..x1 {
                let cx = (px as f64 + 0.5) / scale;
                let cy = (py as f64 + 0.5) / scale;
                if !shape_contains(layer, cx, cy) || shape_contains(&inner, cx, cy) {
                    continue;
                }
                let idx = (py * canvas_w + px) * 4;
                blend_pixel(pixels, idx, (1.0, 1.0, 1.0), CHIP_RIM_WHITE * content_alpha);
            }
        }
    }

    /// The expanded card's hairline: the reference draws a 1pt white at
    /// 12% `strokeBorder` just inside the silhouette (boring.notch's
    /// `NotchShape` stroke), which is what makes the card read as a
    /// distinct glass surface against the bezel. The caller gates it on
    /// `expand > 0.05`, exactly like the shadow: the collapsed silhouette
    /// IS the physical notch, whose top edge is the bezel itself.
    const CARD_HAIRLINE_INSET: f64 = 1.0;
    const CARD_HAIRLINE_WHITE: f64 = 0.12;

    fn stroke_card_hairline(
        pixels: &mut [u8],
        canvas_w: usize,
        canvas_h: usize,
        layer: &Layer,
        scale: f64,
    ) {
        let inner = Layer {
            x: layer.x + CARD_HAIRLINE_INSET,
            y: layer.y + CARD_HAIRLINE_INSET,
            w: (layer.w - 2.0 * CARD_HAIRLINE_INSET).max(0.0),
            h: (layer.h - 2.0 * CARD_HAIRLINE_INSET).max(0.0),
            top_radius: (layer.top_radius - CARD_HAIRLINE_INSET).max(0.0),
            bottom_radius: (layer.bottom_radius - CARD_HAIRLINE_INSET).max(0.0),
            ..*layer
        };
        let (x0, y0, x1, y1) = layer_pixel_box(layer, canvas_w, canvas_h, scale, 0.0);
        for py in y0..y1 {
            for px in x0..x1 {
                let cx = (px as f64 + 0.5) / scale;
                let cy = (py as f64 + 0.5) / scale;
                if !shape_contains(layer, cx, cy) || shape_contains(&inner, cx, cy) {
                    continue;
                }
                let idx = (py * canvas_w + px) * 4;
                blend_pixel(pixels, idx, (1.0, 1.0, 1.0), CARD_HAIRLINE_WHITE);
            }
        }
    }

    /// The mic ring's 1.5pt caramel stroke: the band between the ring
    /// circle and a circle inset by the stroke width takes a solid
    /// caramel source-over pass on top of the ring's caramel@0.18 track
    /// fill. Drawn per ring, after its fill, before the center dot.
    const MIC_RING_STROKE: f64 = 1.5;

    fn stroke_mic_ring(
        pixels: &mut [u8],
        canvas_w: usize,
        canvas_h: usize,
        layer: &Layer,
        scale: f64,
        content_alpha: f64,
    ) {
        let inner = Layer {
            x: layer.x + MIC_RING_STROKE,
            y: layer.y + MIC_RING_STROKE,
            w: (layer.w - 2.0 * MIC_RING_STROKE).max(0.0),
            h: (layer.h - 2.0 * MIC_RING_STROKE).max(0.0),
            top_radius: (layer.top_radius - MIC_RING_STROKE).max(0.0),
            bottom_radius: (layer.bottom_radius - MIC_RING_STROKE).max(0.0),
            ..*layer
        };
        let (x0, y0, x1, y1) = layer_pixel_box(layer, canvas_w, canvas_h, scale, 0.0);
        for py in y0..y1 {
            for px in x0..x1 {
                let cx = (px as f64 + 0.5) / scale;
                let cy = (py as f64 + 0.5) / scale;
                if !shape_contains(layer, cx, cy) || shape_contains(&inner, cx, cy) {
                    continue;
                }
                let idx = (py * canvas_w + px) * 4;
                blend_pixel(
                    pixels,
                    idx,
                    (CARAMEL.0, CARAMEL.1, CARAMEL.2),
                    CARAMEL.3 * content_alpha,
                );
            }
        }
    }

    /// The chip labels' ink, shared by the paint path and its tests:
    /// CARAMEL on the caramel-tinted pick (Restore/Open), white at 85%
    /// on every white-tinted chip. The old dark on-accent ink is gone —
    /// the reference never puts dark ink on an accent chip.
    pub(crate) fn chip_ink(id: &str) -> (f64, f64, f64) {
        if id == "chip_restore" {
            (CARAMEL.0, CARAMEL.1, CARAMEL.2)
        } else {
            (CHIP_INK_WHITE, CHIP_INK_WHITE, CHIP_INK_WHITE)
        }
    }

    /// The layers whose rect is a text bucket and nothing else: their color
    /// is the glyph ink, so filling the rect would paint a slab behind (and
    /// in the same color as) the text. Every other layer is a real fill.
    pub(crate) const PURE_TEXT_LAYERS: [&str; 6] = [
        "transcript",
        "choice_title",
        "choice_subtitle",
        "outcome_title",
        "outcome_detail",
        "eyebrow",
    ];

    /// Verification hook: the historical 1x paint path. The pixel tests
    /// assert against it byte-for-byte, and it stays exactly 1.0 so that
    /// scripted verification never depends on the display it happens to
    /// run on. Production paints through [`paint_into_scaled`] with the
    /// view's real backing scale; the two agree exactly at scale 1.0.
    #[allow(dead_code)]
    pub fn paint_into(state: &RenderState) -> BitmapData {
        paint_into_scaled(state, 1.0)
    }

    /// The production paint path: rasterizes the layout ONCE at the
    /// display's backing scale, so a 2x Retina panel gets (CANVAS_SIZE *
    /// scale) pixels that the drawRect: blit lands 1:1 — one image pixel
    /// per device pixel, with no upscale and no interpolation. Everything
    /// multiplies by the scale factor: layer rects, glyph buffer dims, the
    /// font size passed to CoreText, and the baselines; the final image is
    /// never resampled.
    ///
    /// The layer geometry itself stays in points: `surface_layouts` is the
    /// single source of truth, and hit-testing keeps consuming points.
    pub fn paint_into_scaled(state: &RenderState, scale: f64) -> BitmapData {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let w = (CANVAS_SIZE.0 * scale).round() as usize;
        let h = (CANVAS_SIZE.1 * scale).round() as usize;
        let mut bitmap = BitmapData {
            width: w,
            height: h,
            pixels: vec![0u8; w * h * 4],
        };
        for layer in surface_layouts(state) {
            // Content choreography: every CONTENT layer's alpha rides
            // `content_alpha` (late-in on a grow, early-out on a shrink).
            // The `card` silhouette never dims — it stays opaque and only
            // its geometry morphs, so the shape can never flash during a
            // morph. Static states carry 1.0, leaving every other paint
            // path byte-for-byte unchanged.
            let content_alpha = if layer.id == "card" {
                1.0
            } else {
                state.content_alpha.clamp(0.0, 1.0)
            };
            let mut layer = layer;
            layer.a *= content_alpha;
            // The expanded card's drop shadow, drawn first so the opaque
            // card fill composites over its interior: the shadow lives
            // strictly outside the silhouette and never tints the card,
            // a chip, or a glyph.
            if layer.id == "card" && state.expand > 0.05 {
                paint_card_shadow(&mut bitmap.pixels, w, h, &layer, scale);
            }
            // Only PURE-TEXT rows skip the fill. The old rule keyed off
            // `static_label(..).is_some()`, which is true for every chip and
            // for the eyebrow - so the chip pills lost their background
            // entirely (measured 2-3:1 contrast, effectively invisible) -
            // while `outcome_title` / `outcome_detail` were missing from the
            // list, so their INK colors were filled as full-width slabs with
            // the text drawn on top in the same color. Chips, the card, the
            // band, the mic dot and the pips all keep their fill, and their
            // glyphs are composited on top of it afterwards.
            let is_text_layer = PURE_TEXT_LAYERS.contains(&layer.id);
            if !is_text_layer {
                // Pure-fill layer: alpha-blend its flat color over what
                // is beneath it. Text layers skip this — glyphs paint
                // directly on the card background, so the placeholder
                // block never shows behind the text.
                let x0 = (layer.x * scale).max(0.0) as usize;
                let y0 = (layer.y * scale).max(0.0) as usize;
                let x1 = ((layer.x + layer.w) * scale).min(w as f64).max(0.0) as usize;
                let y1 = ((layer.y + layer.h) * scale).min(h as f64).max(0.0) as usize;
                for py in y0..y1 {
                    for px in x0..x1 {
                        // Device pixel center converted back to layer point
                        // space, so the radius/containment math stays in
                        // points (identical to the 1x path at scale 1.0).
                        let cx = (px as f64 + 0.5) / scale;
                        let cy = (py as f64 + 0.5) / scale;
                        if !shape_contains(&layer, cx, cy) {
                            continue;
                        }
                        let idx = (py * w + px) * 4;
                        let src_a = layer.a;
                        let dst_a = bitmap.pixels[idx + 3] as f64 / 255.0;
                        let out_a = src_a + dst_a * (1.0 - src_a);
                        let mix = |dst: u8, src: f64| -> u8 {
                            ((src * src_a + dst as f64 / 255.0 * dst_a * (1.0 - src_a))
                                / out_a.max(f64::EPSILON)
                                * 255.0) as u8
                        };
                        bitmap.pixels[idx] = mix(bitmap.pixels[idx], layer.r);
                        bitmap.pixels[idx + 1] = mix(bitmap.pixels[idx + 1], layer.g);
                        bitmap.pixels[idx + 2] = mix(bitmap.pixels[idx + 2], layer.b);
                        bitmap.pixels[idx + 3] = (out_a * 255.0) as u8;
                    }
                }
                // The expanded card's hairline silhouette rides on top
                // of the card fill, so every later pass composites over
                // it. Skipped at (or near) the collapsed notch, whose top
                // edge is the physical bezel itself.
                if layer.id == "card" && state.expand > 0.05 {
                    stroke_card_hairline(&mut bitmap.pixels, w, h, &layer, scale);
                }
                // The dark chips' hairline rim rides on top of their own
                // fill — after the fill, before the label — so nothing
                // else can pick it up. The caramel chip is skipped.
                if chip_has_rim(layer.id) {
                    stroke_chip_rim(&mut bitmap.pixels, w, h, &layer, scale, content_alpha);
                }
                // The mic ring's 1.5pt caramel stroke lies strictly
                // inside the ring fill (the track), and the center dot
                // is its own later layer, so the band can never cover it.
                if layer.id == "mic_ring" {
                    stroke_mic_ring(&mut bitmap.pixels, w, h, &layer, scale, content_alpha);
                }
            }
            // Real glyphs inside the layer rect. The transcript's text
            // is the live partial from this very RenderState, so it
            // updates with every render. Glyph ink dims with the rest of
            // the content — the rasterizer draws at full coverage, so the
            // choreography multiplies in here.
            draw_layer_glyphs(&mut bitmap, &layer, state, w, h, scale, content_alpha);
        }
        bitmap
    }

    /// Rasterizes and composites the glyphs for one layer, clipped to
    /// the layer rect. No-op for pure-fill layers.
    ///
    /// `scale` is the device/point ratio: the glyph buffer, the CoreText
    /// font size, and the layer origin are all in device pixels, while the
    /// layer rect and the advance estimator stay in points. At scale 1.0
    /// this is exactly the original 1x code path.
    fn draw_layer_glyphs(
        bitmap: &mut BitmapData,
        layer: &Layer,
        state: &RenderState,
        canvas_w: usize,
        canvas_h: usize,
        scale: f64,
        content_alpha: f64,
    ) {
        // Device-pixel origin: a float→usize cast saturates (never
        // wraps), and the explicit floor keeps a stray negative layout
        // coordinate from smuggling the origin past the canvas edge.
        let ox0 = (layer.x.max(0.0) * scale) as usize;
        let oy = (layer.y.max(0.0) * scale) as usize;
        let lw_i = ((layer.w.max(1.0)) * scale) as usize;
        let lw_i = lw_i.min(canvas_w.saturating_sub(ox0));
        let lh_i = (layer.h.max(1.0) * scale) as usize;
        let lh = lh_i.min(canvas_h.saturating_sub(oy));
        if lw_i == 0 || lh == 0 {
            return;
        }
        // (text, size in points, semibold, (r, g, b), tracking in points, tabular digits)
        let spec = match layer.id {
            "transcript" => {
                let text = state.partial.clone().unwrap_or_default();
                if text.trim().is_empty() {
                    // Nothing heard yet. The layout reserves this row either
                    // way, so paint the prompt in muted ink instead of
                    // leaving ~70% of the card dead black (measured).
                    (
                        "say a command".to_string(),
                        FONT_UI_TRANSCRIPT,
                        false,
                        (INK_MUTED.0, INK_MUTED.1, INK_MUTED.2),
                        0.0,
                        false,
                    )
                } else {
                    (
                        super::bounded_transcript(&text, layer.w, FONT_UI_TRANSCRIPT).join("\n"),
                        FONT_UI_TRANSCRIPT,
                        false,
                        (INK_PRIMARY.0, INK_PRIMARY.1, INK_PRIMARY.2),
                        0.0,
                        false,
                    )
                }
            }
            // The carousel's focused candidate title, in the same light
            // ink and size the transcript speaks. No candidate → no
            // glyphs (honest, same rule as the quiet transcript).
            "choice_title" => {
                let text = if let Some(browse) = state.browse.as_ref() {
                    browse.title.clone()
                } else {
                    state
                        .choice
                        .as_ref()
                        .and_then(|c| c.candidates.get(c.index))
                        .map(|c| c.title.clone())
                        .unwrap_or_default()
                };
                if text.trim().is_empty() {
                    return;
                }
                (
                    bounded_text(&text, layer.w, FONT_UI_CHOICE_TITLE),
                    FONT_UI_CHOICE_TITLE,
                    false,
                    (INK_PRIMARY.0, INK_PRIMARY.1, INK_PRIMARY.2),
                    0.0,
                    false,
                )
            }
            // The subtitle: "work N of M · swipe or click" in muted ink.
            "choice_subtitle" => {
                // The browse carousel's subtitle is the focused work's
                // own supporting line, truncated to the row; the choice
                // carousel speaks its position in the list.
                if let Some(browse) = state.browse.as_ref() {
                    if browse.subtitle.trim().is_empty() {
                        return;
                    }
                    (
                        bounded_text(&browse.subtitle, layer.w, FONT_UI_CHOICE_SUBTITLE),
                        FONT_UI_CHOICE_SUBTITLE,
                        true,
                        (INK_MUTED.0, INK_MUTED.1, INK_MUTED.2),
                        0.0,
                        false,
                    )
                } else {
                    let Some(choice) = state.choice.as_ref() else {
                        return;
                    };
                    if choice.candidates.is_empty() {
                        return;
                    }
                    let n = choice.index + 1;
                    let m = choice.candidates.len();
                    // Tabular digits: "work 8 of 9" must occupy the same
                    // advance box as "work 1 of 3", so the line never jitters
                    // sideways as the focused index changes.
                    (
                        format!("work {n} of {m} · swipe or click"),
                        FONT_UI_CHOICE_SUBTITLE,
                        true,
                        (INK_MUTED.0, INK_MUTED.1, INK_MUTED.2),
                        0.0,
                        true,
                    )
                }
            }
            "outcome_title" => {
                let text = state
                    .outcome_content
                    .as_ref()
                    .map(|outcome| outcome.title.clone())
                    .unwrap_or_else(|| "Done".into());
                (
                    bounded_text(&text, layer.w, FONT_UI_OUTCOME_TITLE),
                    FONT_UI_OUTCOME_TITLE,
                    false,
                    (INK_PRIMARY.0, INK_PRIMARY.1, INK_PRIMARY.2),
                    0.0,
                    false,
                )
            }
            "outcome_detail" => {
                let text = state
                    .outcome_content
                    .as_ref()
                    .map(|outcome| outcome.detail.clone())
                    .unwrap_or_default();
                if text.trim().is_empty() {
                    return;
                }
                (
                    bounded_text(&text, layer.w, FONT_UI_OUTCOME_DETAIL),
                    FONT_UI_OUTCOME_DETAIL,
                    // Medium weight: at 11pt the emphasized system face
                    // is the closest real match for the design study's
                    // medium detail line.
                    true,
                    (INK_MUTED.0, INK_MUTED.1, INK_MUTED.2),
                    0.0,
                    false,
                )
            }
            // The eyebrow label differs per phase (PICK A WORK during the
            // carousel). Its ink is whatever the LAYOUT assigned the layer -
            // the teal informational voice while listening or choosing, the
            // caramel accent on the outcome card - so the accent lives in
            // the text and never in a full-width band behind it.
            "eyebrow" => {
                let text = if state.browse.is_some() {
                    "YOUR WORK".to_string()
                } else if state.choice.is_some() {
                    "PICK A WORK".to_string()
                } else if let Some(outcome) = state.outcome_content.as_ref() {
                    outcome.eyebrow.clone()
                } else {
                    "LISTENING".to_string()
                };
                (
                    text,
                    FONT_UI_EYEBROW,
                    true,
                    (layer.r, layer.g, layer.b),
                    FONT_UI_EYEBROW_TRACKING,
                    false,
                )
            }
            // Chips: the reference's tinted-glass grammar — the label is
            // the ink (accent on the accent-tinted pick, white on the
            // white-tinted pills); there is no dark on-accent ink on this
            // surface any more.
            _ => match chip_label(state, layer.id) {
                Some(text) => {
                    let ink = chip_ink(layer.id);
                    (text.to_string(), FONT_UI_CHIP, true, ink, 0.0, false)
                }
                None => return,
            },
        };
        let (text, size, semibold, color, tracking, tabular) = spec;
        // A chip label centers inside its pill; every other row stays
        // left-aligned at its layer origin. The advance estimate is the same
        // 0.60 em heuristic `bounded_text` truncates with, so the two always
        // agree about what fits; the pad it derives is converted to device
        // pixels so the label stays centered in its pill at any scale.
        let x_pad = if layer.id.starts_with("chip_") {
            let estimate = text.chars().count() as f64 * size * 0.60;
            (((layer.w - estimate) * scale) / 2.0).max(0.0).floor() as usize
        } else {
            0
        };
        let lw = lw_i.saturating_sub(x_pad).max(1);
        let ox = ox0 + x_pad;
        // Defensive: layouts keep every text row inside the canvas, so
        // neither check can fire for a real state at any scale — but a
        // stray geometry must never turn into a slice-index panic.
        if ox.saturating_add(lw) > canvas_w || oy.saturating_add(lh) > canvas_h {
            return;
        }
        // The font size passed to CoreText is scaled right here — the
        // buffer is rasterized natively at device resolution, never an
        // upscaled 1x image.
        let Some(glyphs) = rasterize_text(
            &text,
            lw,
            lh,
            size * scale,
            semibold,
            color,
            tracking * scale,
            tabular,
        ) else {
            return;
        };
        // Composite: glyphs are premultiplied RGBA in a buffer of the
        // layer's size; source-over into the canvas at the layer origin.
        for gy in 0..lh {
            for gx in 0..lw {
                let src = (gy * lw + gx) * 4;
                let a = glyphs[src + 3] as f64 / 255.0 * content_alpha;
                if a <= 0.0 {
                    continue;
                }
                let dst = ((oy + gy) * canvas_w + ox + gx) * 4;
                let dst_a = bitmap.pixels[dst + 3] as f64 / 255.0;
                let out_a = a + dst_a * (1.0 - a);
                let mix = |d: u8, s: f64| -> u8 {
                    ((s * a + d as f64 / 255.0 * dst_a * (1.0 - a)) / out_a.max(f64::EPSILON)
                        * 255.0) as u8
                };
                bitmap.pixels[dst] = mix(bitmap.pixels[dst], glyphs[src] as f64 / 255.0);
                bitmap.pixels[dst + 1] =
                    mix(bitmap.pixels[dst + 1], glyphs[src + 1] as f64 / 255.0);
                bitmap.pixels[dst + 2] =
                    mix(bitmap.pixels[dst + 2], glyphs[src + 2] as f64 / 255.0);
                bitmap.pixels[dst + 3] = (out_a * 255.0) as u8;
            }
        }
    }

    fn current_graphics_context() -> Option<CGContextRef> {
        unsafe {
            let context_class = class_of(b"NSGraphicsContext\0");
            if context_class.is_null() {
                return None;
            }
            let context: Id = send0(context_class as Id, sel(b"currentContext\0"));
            if context.is_null() {
                return None;
            }
            let port: CGContextRef = send0(context, sel(b"CGContext\0"));
            (!port.is_null()).then_some(port)
        }
    }

    fn draw_bitmap_to_current_context(bitmap: &BitmapData) {
        let Some(context) = current_graphics_context() else {
            return;
        };
        unsafe {
            let color_space = CGColorSpaceCreateDeviceRGB();
            if color_space.is_null() {
                return;
            }
            let provider = CGDataProviderCreateWithData(
                std::ptr::null(),
                bitmap.pixels.as_ptr() as *const c_void,
                bitmap.pixels.len(),
                None,
            );
            if provider.is_null() {
                CGColorSpaceRelease(color_space);
                return;
            }
            let image = CGImageCreate(
                bitmap.width,
                bitmap.height,
                8,
                32,
                bitmap.width * 4,
                color_space,
                1 | (4 << 12),
                provider,
                std::ptr::null(),
                // shouldInterpolate = NO: the bitmap is drawn at exactly
                // one image pixel per device pixel, so it must never be
                // resampled (and if a rect ever mismatched, nearest is the
                // honest failure mode for text).
                0,
                0,
            );
            if image.is_null() {
                CGDataProviderRelease(provider);
                CGColorSpaceRelease(color_space);
                return;
            }

            // Destination rect: the view's bounds in POINTS (the view is
            // exactly CANVAS_SIZE). The bitmap is device-scaled, so on a
            // 2x display a 760x640 image lands in a 380x320pt rect — one
            // image pixel per device pixel, no CTM upscale.
            let rect = NsRect {
                x: 0.0,
                y: 0.0,
                w: CANVAS_SIZE.0,
                h: CANVAS_SIZE.1,
            };
            CGContextSaveGState(context);
            CGContextClearRect(context, rect);
            // Draw the image with NO extra reflection. This is the whole fix
            // for the upside-down composite, and it is provable from the two
            // conventions involved:
            //
            //   * A CGImage's rows are stored top-down (row 0 = top), and
            //     `CGContextDrawImage` maps that first row to the rect's
            //     MAXIMUM user-space Y.
            //   * The view reports `isFlipped == NO`, so AppKit hands us its
            //     native y-up context, where max-Y is the TOP of the view.
            //
            // One unmodified draw therefore lands `BitmapData` row 0 at the
            // top of the panel. The previous `translate(0, h) + scale(1, -1)`
            // pair moved max-Y to the BOTTOM of the view, which is why every
            // card composited mirrored: chips above the eyebrow, the caramel
            // band under the title, and the card's radii apparently swapped.
            //
            // Hit-testing is unaffected: `locationInWindow` is bottom-left
            // WINDOW space whatever the view reports its flippedness to be,
            // so `appkit_to_bitmap_point` remains the single vertical
            // reflection at that boundary.
            CGContextDrawImage(context, rect, image);
            CGContextRestoreGState(context);

            CGImageRelease(image);
            CGDataProviderRelease(provider);
            CGColorSpaceRelease(color_space);
        }
    }

    /// The drawRect: paint path — fill the bitmap at the view's backing
    /// scale, retain it for verification, and draw the same pixels into
    /// AppKit's current graphics context at 1:1.
    pub fn paint_view(painter: Painter, scale: f64) {
        let state = painter.state.lock().unwrap().clone();
        // AppKit invokes drawRect: through an `extern "C"` callback, so a
        // Rust panic would abort the entire headless agent instead of merely
        // dropping one malformed frame. Keep the voice loop alive even if a
        // third-party/CoreText bitmap reports an inconsistent row size.
        let Some(bitmap) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            paint_into_scaled(&state, scale)
        }))
        .ok()
        else {
            eprintln!("EVO notch: skipped a malformed paint frame");
            return;
        };
        draw_bitmap_to_current_context(&bitmap);
        // Move — never clone — the device-scale bitmap (760x640x4 bytes
        // at 2x) into the verification slot: a per-frame copy of that
        // buffer is exactly the kind of work the deadline pacing cannot
        // afford.
        *painter.bitmap.lock().unwrap() = Some(bitmap);
    }

    // ── Text drawing (CoreText) ───────────────────────────────────────
    //
    // The layout layer stays authoritative: `surface_layouts` returns
    // rects, and glyphs are rasterized *inside* those rects. CoreText
    // + CGBitmapContext are thread-safe, so this runs from the paint
    // path on any thread — the AppKit-facing on_main rule is not
    // touched.

    /// The view's display scale: `[[view window] backingScaleFactor]`,
    /// re-read per draw (a cheap msgSend). The panel is created on the
    /// built-in display, so 2.0 is the correct default while the view has
    /// no window yet — and on a 1x panel the reported factor is honoured,
    /// which keeps the paint pipeline correct rather than assuming Retina.
    pub(crate) fn view_backing_scale(view: Id) -> f64 {
        unsafe {
            let window: Id = send0(view, sel(b"window\0"));
            if window.is_null() {
                return 2.0;
            }
            let scale: f64 = send0(window, sel(b"backingScaleFactor\0"));
            if scale.is_finite() && scale > 0.0 {
                scale
            } else {
                2.0
            }
        }
    }

    /// The system SF face (what the reference draws with), cached per
    /// size across the process — the notch uses at most three sizes.
    /// CTFont objects are immutable and documented thread-safe; the
    /// newtype exists only to mark that promise for `Send`.
    #[derive(Clone, Copy)]
    struct SharedFont(CTFontRef);
    unsafe impl Send for SharedFont {}

    /// CoreText's `CTFontUIFontType` values (CTFont.h). Only the `SYSTEM`
    /// entries are the SF UI face. `USER` and `USER_FIXED_PITCH` resolve to
    /// the user's *document* fonts, and fixed-pitch is Courier - which is
    /// exactly what the old `if semibold { 1 } else { 0 }` pair requested,
    /// and what pixel forensics found on screen (a uniform ~6.5pt advance
    /// for every letter, crossbarred `I`, serifed `L`/`T` feet).
    #[allow(dead_code)]
    mod ct_ui_font {
        pub const USER: u8 = 0;
        pub const USER_FIXED_PITCH: u8 = 1;
        pub const SYSTEM: u8 = 2;
        pub const EMPHASIZED_SYSTEM: u8 = 3;
        pub const SMALL_SYSTEM: u8 = 4;
        pub const SMALL_EMPHASIZED_SYSTEM: u8 = 5;
    }

    /// CoreText font-feature identifiers (the SFNT feature registry) and
    /// the CFNumber encodings they are passed with.
    mod ct_feature {
        /// kNumberSpacingType — the AAT number-spacing feature.
        pub const NUMBER_SPACING: i32 = 6;
        /// kMonospacedNumbersSelector — tabular (equal-advance) digits.
        pub const MONOSPACED_NUMBERS: i32 = 0;
        /// kCFNumberIntType.
        pub const CF_NUMBER_INT: isize = 9;
        /// kCFNumberDoubleType.
        pub const CF_NUMBER_DOUBLE: isize = 13;
    }

    /// RAII owner for one +1 CoreFoundation object. The release happens on
    /// every return path — early returns and `?` included — as soon as the
    /// guard drops, which is exactly when the object is no longer needed
    /// (after the line has been drawn / measured).
    struct CfOwned(*const c_void);

    impl CfOwned {
        fn new(object: *const c_void) -> Option<Self> {
            if object.is_null() {
                None
            } else {
                Some(Self(object))
            }
        }

        fn raw(&self) -> *const c_void {
            self.0
        }
    }

    impl Drop for CfOwned {
        fn drop(&mut self) {
            // Guarded at construction: CFRelease(NULL) is a crash.
            unsafe { CFRelease(self.0) }
        }
    }

    /// One named-face lookup; only the `system_font` fallback path uses it.
    fn font_by_name(name: &str, size: f64) -> CTFontRef {
        let cf_name = make_cf_string(name);
        if cf_name.is_null() {
            return std::ptr::null();
        }
        let font = unsafe { CTFontCreateWithName(cf_name, size, std::ptr::null()) };
        // CTFontCreateWithName only reads the name; we own this string.
        unsafe { CFRelease(cf_name) };
        font
    }

    fn system_font(size: f64, semibold: bool) -> CTFontRef {
        use std::collections::HashMap;
        use std::sync::Mutex;
        static CACHE: Mutex<Option<HashMap<(u64, bool), SharedFont>>> = Mutex::new(None);
        let key = ((size * 10.0) as u64, semibold);
        let mut guard = CACHE.lock().unwrap();
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(font) = cache.get(&key) {
            return font.0;
        }
        // Semibold labels take the emphasized system face; CoreText has no
        // separate semibold UI type, and Bold reads as Semibold at 10-11pt.
        let ui_font = if semibold {
            ct_ui_font::EMPHASIZED_SYSTEM
        } else {
            ct_ui_font::SYSTEM
        };
        let mut font_ref =
            unsafe { CTFontCreateUIFontForLanguage(ui_font, size, std::ptr::null()) };
        // Never fail silently again: a null UI font is how the whole surface
        // ended up rendering in a substituted face without anyone noticing.
        if font_ref.is_null() {
            eprintln!(
                "EVO notch: CTFontCreateUIFontForLanguage(type={ui_font}, {size}pt) returned null; falling back to .AppleSystemUIFont"
            );
            font_ref = font_by_name(".AppleSystemUIFont", size);
        }
        if font_ref.is_null() {
            eprintln!(
                "EVO notch: .AppleSystemUIFont unavailable at {size}pt; falling back to Helvetica"
            );
            font_ref = font_by_name("Helvetica", size);
        }
        if font_ref.is_null() {
            eprintln!("EVO notch: no usable font at {size}pt - this surface will draw no text");
        }
        cache.insert(key, SharedFont(font_ref));
        font_ref
    }

    /// Verification hook: the resolved CTFont for one role, as an opaque
    /// pointer. Non-null is the contract - a null font means CoreText
    /// resolved nothing and the surface would silently draw no text. Only
    /// the tests call this; the paint path uses `system_font` directly.
    #[allow(dead_code)]
    pub(crate) fn resolved_font_ptr(size: f64, semibold: bool) -> *const c_void {
        system_font(size, semibold)
    }

    /// Rasterizes `text` into a fresh RGBA buffer sized to the layer
    /// rect, drawing with `color`. Returns `None` for empty/blank text
    /// (nothing to draw — callers skip the blend). The buffer is
    /// bottom-up-free: CoreText draws top-down in a flipped context by
    /// scaling Y negatively, so row 0 is the top row (same orientation
    /// as `BitmapData`).
    /// Returns a single-line label that fits its paint bucket. CoreText
    /// clips at the bitmap edge, so truncating before rasterization is what
    /// keeps long transcripts, titles and outcomes visually complete rather
    /// than silently cutting off their final glyphs.
    fn bounded_text(text: &str, width: f64, size: f64) -> String {
        let max_chars = (width / (size * 0.60)).floor().max(1.0) as usize;
        let chars: Vec<char> = text.chars().collect();
        if chars.len() <= max_chars {
            return text.to_owned();
        }
        if max_chars <= 1 {
            return "…".to_owned();
        }
        let mut result: String = chars[..max_chars - 1].iter().collect();
        result.push('…');
        result
    }

    /// The tabular-digit copy of the resolved system face, cached per
    /// size/weight exactly like `system_font` (a second, equally bounded
    /// one-shot cache; the base cache is untouched). Built through the
    /// public font-feature API: copy the font's descriptor, apply
    /// `kNumberSpacingType` / `kMonospacedNumbersSelector`, materialize a
    /// font copy carrying it. The probe on this machine confirms the
    /// result: '1' and '8' both advance 7.1418pt at 10.5pt, while the
    /// default (proportional) face gives 5.4653 / 7.3777 — so "work N of
    /// M" cannot shift width as N changes.
    fn tabular_system_font(size: f64, semibold: bool) -> CTFontRef {
        use std::collections::HashMap;
        use std::sync::Mutex;
        static CACHE: Mutex<Option<HashMap<(u64, bool), SharedFont>>> = Mutex::new(None);
        let key = ((size * 10.0) as u64, semibold);
        let mut guard = CACHE.lock().unwrap();
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(font) = cache.get(&key) {
            return font.0;
        }
        let base = system_font(size, semibold);
        if base.is_null() {
            cache.insert(key, SharedFont(base));
            return base;
        }
        let feature_type: i32 = ct_feature::NUMBER_SPACING;
        let feature_selector: i32 = ct_feature::MONOSPACED_NUMBERS;
        let font = unsafe {
            let descriptor = CfOwned::new(CTFontCopyFontDescriptor(base));
            let type_number = CfOwned::new(CFNumberCreate(
                std::ptr::null(),
                ct_feature::CF_NUMBER_INT,
                &feature_type as *const i32 as *const c_void,
            ));
            let selector_number = CfOwned::new(CFNumberCreate(
                std::ptr::null(),
                ct_feature::CF_NUMBER_INT,
                &feature_selector as *const i32 as *const c_void,
            ));
            let derived = match (descriptor, type_number, selector_number) {
                (Some(descriptor), Some(type_number), Some(selector_number)) => {
                    CfOwned::new(CTFontDescriptorCreateCopyWithFeature(
                        descriptor.raw(),
                        type_number.raw(),
                        selector_number.raw(),
                    ))
                }
                _ => None,
            };
            match derived {
                Some(derived) => {
                    let copied =
                        CTFontCreateCopyWithAttributes(base, size, std::ptr::null(), derived.raw());
                    // A null copy falls back to the base face (the label
                    // must still draw); either way the cache owns it from
                    // here, exactly like `system_font`.
                    if copied.is_null() { base } else { copied }
                }
                None => base,
            }
        };
        cache.insert(key, SharedFont(font));
        font
    }

    /// One attributed CoreText line plus every +1 object that must
    /// outlive its drawing. Field order IS the release order: the line
    /// goes first, then the attribute objects it references without
    /// retaining (the attrs dictionary is built with null callbacks, so
    /// a release before the line is done is the historical
    /// objc_retain segfault).
    struct AttributedLine {
        line: CfOwned,
        /// Cache-owned; never released here.
        font: CTFontRef,
        /// Pure ownership: keeps every attribute object alive until the
        /// line has been drawn. Never read.
        _keepalive: Vec<CfOwned>,
    }

    /// Builds one attributed CoreText line with the surface's exact
    /// attribute set: `NSFont` (system SF, optionally the tabular-digit
    /// variant), `NSColor` when a color is supplied, and `NSKern` (the
    /// eyebrow tracking) when `tracking > 0`. String-attribute keys are
    /// the attribute names CoreText publishes: "NSFont"
    /// (kCTFontAttributeName), "NSColor" (kCTForegroundColorAttributeName)
    /// and "NSKern" (kCTKernAttributeName, verified against the SDK).
    fn attributed_line(
        text: &str,
        size: f64,
        semibold: bool,
        color: Option<(f64, f64, f64)>,
        tracking: f64,
        tabular: bool,
    ) -> Option<AttributedLine> {
        unsafe {
            let ctext = CfOwned::new(make_cf_string(text))?;
            let font = if tabular {
                tabular_system_font(size, semibold)
            } else {
                system_font(size, semibold)
            };
            if font.is_null() {
                // `system_font` already warned through every fallback.
                return None;
            }
            let attrs = CfOwned::new(CFDictionaryCreateMutable(
                std::ptr::null(),
                3,
                std::ptr::null(),
                std::ptr::null(),
            ))?;
            let font_key = CfOwned::new(make_cf_string("NSFont"))?;
            CFDictionarySetValue(attrs.raw(), font_key.raw(), font);
            let color_parts = match color {
                Some((r, g, b)) => {
                    // Foreground color MUST be a string attribute: without
                    // it CoreText renders the glyphs in black, alpha-only —
                    // the color lanes stay zero even when the context fill
                    // color is set (verified with a standalone probe).
                    let space = CfOwned::new(CGColorSpaceCreateDeviceRGB())?;
                    let comps = [r, g, b, 1.0f64];
                    let glyph_color = CfOwned::new(CGColorCreate(space.raw(), comps.as_ptr()))?;
                    let color_key = CfOwned::new(make_cf_string("NSColor"))?;
                    CFDictionarySetValue(attrs.raw(), color_key.raw(), glyph_color.raw());
                    Some((space, glyph_color, color_key))
                }
                None => None,
            };
            let kern_parts = if tracking > 0.0 && tracking.is_finite() {
                let mut kern_value = tracking;
                let kern_number = CfOwned::new(CFNumberCreate(
                    std::ptr::null(),
                    ct_feature::CF_NUMBER_DOUBLE,
                    &mut kern_value as *mut f64 as *const c_void,
                ))?;
                let kern_key = CfOwned::new(make_cf_string("NSKern"))?;
                CFDictionarySetValue(attrs.raw(), kern_key.raw(), kern_number.raw());
                Some((kern_number, kern_key))
            } else {
                None
            };
            let astr = CfOwned::new(CFAttributedStringCreate(
                std::ptr::null(),
                ctext.raw(),
                attrs.raw(),
            ))?;
            let line = CfOwned::new(CTLineCreateWithAttributedString(astr.raw()))?;
            let mut keepalive = vec![ctext, attrs, font_key, astr];
            if let Some((space, glyph_color, color_key)) = color_parts {
                keepalive.push(space);
                keepalive.push(glyph_color);
                keepalive.push(color_key);
            }
            if let Some((kern_number, kern_key)) = kern_parts {
                keepalive.push(kern_number);
                keepalive.push(kern_key);
            }
            Some(AttributedLine {
                line,
                font,
                _keepalive: keepalive,
            })
        }
    }

    /// Verification hook: the typographic advance of one line, measured
    /// through the exact attribute pipeline the paint path uses, for the
    /// tabular-digit and kern tests.
    #[allow(dead_code)]
    pub(crate) fn measured_advance(
        text: &str,
        size: f64,
        semibold: bool,
        tracking: f64,
        tabular: bool,
    ) -> f64 {
        let Some(attributed) = attributed_line(text, size, semibold, None, tracking, tabular)
        else {
            return f64::NAN;
        };
        unsafe {
            CTLineGetTypographicBounds(
                attributed.line.raw(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }
    }

    /// Rasterizes `text` into a fresh RGBA buffer sized to the layer
    /// rect, drawing with `color`. Returns `None` for empty/blank text
    /// (nothing to draw — callers skip the blend). The buffer is
    /// top-down: row 0 is the top row (same orientation as `BitmapData`).
    /// `tracking` adds letter-spacing in points (0 = none); `tabular`
    /// selects the equal-advance digit font.
    fn rasterize_text(
        text: &str,
        width: usize,
        height: usize,
        size: f64,
        semibold: bool,
        color: (f64, f64, f64),
        tracking: f64,
        tabular: bool,
    ) -> Option<Vec<u8>> {
        let lines: Vec<&str> = text.split('\n').filter(|line| !line.is_empty()).collect();
        if lines.is_empty() || width == 0 || height == 0 {
            return None;
        }
        if lines.len() == 1 {
            return rasterize_single_line(
                lines[0], width, height, size, semibold, color, tracking, tabular,
            );
        }
        let line_height = (height / lines.len()).max(1);
        let mut combined = vec![0u8; width * height * 4];
        let mut drew = false;
        for (line_index, line) in lines.iter().enumerate() {
            let Some(raster) = rasterize_single_line(
                line,
                width,
                line_height,
                size,
                semibold,
                color,
                tracking,
                tabular,
            ) else {
                continue;
            };
            drew = true;
            let dst_y = line_index * line_height;
            for row in 0..line_height {
                let dst = (dst_y + row) * width * 4;
                let src = row * width * 4;
                // Defensive: a mid-collapse frame compresses every row
                // toward zero height, and the 1px clamp in
                // `draw_layer_glyphs` can then hand this combiner a tall
                // line list in a too-short buffer. Copy only what fits —
                // never index past either buffer.
                let row_bytes = width.saturating_mul(4);
                let Some(dst_row) = combined.get_mut(dst..dst.saturating_add(row_bytes)) else {
                    break;
                };
                let Some(src_row) = raster.get(src..src.saturating_add(row_bytes)) else {
                    break;
                };
                dst_row.copy_from_slice(src_row);
            }
        }
        drew.then_some(combined)
    }

    fn rasterize_single_line(
        text: &str,
        width: usize,
        height: usize,
        size: f64,
        semibold: bool,
        color: (f64, f64, f64),
        tracking: f64,
        tabular: bool,
    ) -> Option<Vec<u8>> {
        if text.trim().is_empty() || width == 0 || height == 0 {
            return None;
        }
        // Every Create/Copy object for this line lives in `keepalive` (or
        // is the line guard itself) and is released when this function
        // returns — including every `?` above and below. Nothing here is
        // released while the line still needs it: that was the old
        // objc_retain segfault.
        let attributed = attributed_line(text, size, semibold, Some(color), tracking, tabular)?;

        unsafe {
            // Off-screen RGBA canvas to rasterize into.
            let space = CfOwned::new(CGColorSpaceCreateDeviceRGB())?;
            // kCGImageAlphaPremultipliedLast (1) | kCGBitmapByteOrder32Big
            // (4 << 12) → bytes in memory are premultiplied RGBA.
            let bitmap_info: u32 = 1 | (4 << 12);
            let mut buf = vec![0u8; width * height * 4];
            let ctx = CfOwned::new(CGBitmapContextCreate(
                buf.as_mut_ptr() as *mut c_void,
                width,
                height,
                8,
                width * 4,
                space.raw(),
                bitmap_info,
            ))?;
            // Place the baseline WITHOUT reflecting the context.
            //
            // A CGBitmapContext is y-up while its memory rows are stored
            // top-down, so `memory_row = height - user_y`. CoreText glyph
            // outlines are y-up too: drawn in this native space, ascenders
            // land at smaller row indices, i.e. visually up - exactly the
            // top-left orientation `BitmapData` promises.
            //
            // The baseline must sit a full ascent below the row's top edge or
            // CoreText shears the ascenders flat against the row boundary.
            // Ask the font for its real ascent rather than guessing;
            // `text_row_h` sizes every row to hold ascent + descent.
            let ascent = CTFontGetAscent(attributed.font).max(size * 0.82);
            let baseline_from_top = ascent.min(height as f64);
            // The guard erases the CGContextRef mutability; the handle is
            // valid while `ctx` is alive, and drawing is the only use.
            let context = ctx.raw() as *mut c_void;
            CGContextTranslateCTM(context, 0.0, height as f64 - baseline_from_top);
            CGContextSetRGBFillColor(context, color.0, color.1, color.2, 1.0);
            CTLineDraw(attributed.line.raw(), context);
            // The context paints synchronously into `buf`; copy it out
            // before the guards release the context at return.
            Some(buf)
        }
    }

    /// Helper holding the CString dance in one place.
    ///
    /// The encoding is `0x0800_0100` (NSUTF16BigEndianStringEncoding),
    /// verified against the surface's actual strings: `0x0800_0600`
    /// silently fails on multibyte text — the carousel's "·" subtitle
    /// returns a null CFString, and that null handed into
    /// CFAttributedStringCreate segfaults. `0x0800_0100` accepts both
    /// ASCII and multibyte, so one encoding covers the whole surface.
    fn make_cf_string(text: &str) -> CFStringRef {
        let c = std::ffi::CString::new(text).ok();
        match c {
            Some(c) => unsafe {
                CFStringCreateWithCString(
                    std::ptr::null(),
                    c.as_ptr(),
                    0x0800_0100, // NSUTF16BigEndianStringEncoding (see doc above)
                )
            },
            None => std::ptr::null(),
        }
    }

    /// The mouseDown: action path — hit-test and dispatch.
    pub fn handle_click(painter: Painter, event: Id) {
        unsafe {
            // Convert the NSEvent location from AppKit's native bottom-left
            // space into the canonical top-left bitmap/layout space.
            let loc: (f64, f64) = {
                let f: extern "C" fn(Id, Sel) -> NsPoint =
                    std::mem::transmute(objc_msgSend as *const ());
                let p = f(event, sel(b"locationInWindow\0"));
                (p.x, p.y) // NSPoint reads x/y from the struct return.
            };
            let local = appkit_to_bitmap_point(loc, CANVAS_SIZE.1);
            let state = painter.state.lock().unwrap().clone();
            let layers = surface_layouts(&state);
            if let Some(layer) = hit_test(&layers, local.0, local.1)
                && let Some(action) = layer.action
            {
                let action = match action {
                    OptionAction::Dismiss => {
                        if let Some(steps) = painter.steps.as_ref() {
                            let _ = steps.send(SurfaceInput::Dismiss);
                        }
                        dispatch_for_action(&state, action)
                    }
                    OptionAction::Next | OptionAction::Prev => {
                        // Chip step: drive the surface's own engine too,
                        // so the carousel flips on the click itself.
                        if let Some(steps) = painter.steps.as_ref() {
                            let _ = steps.send(SurfaceInput::ChoiceStep(
                                if action == OptionAction::Next { 1 } else { -1 },
                            ));
                        }
                        dispatch_for_action(&state, action)
                    }
                    _ => dispatch_for_action(&state, action),
                };
                if let Some(sender) = painter.actions.as_ref() {
                    let _ = sender.send(action);
                }
            }
        }
    }

    /// The mouseDragged: swipe path — accumulating horizontal drag
    /// over the carousel zone; every 24pt of net travel flips one
    /// candidate (drag left = next), with wraparound. Drag outside the
    /// carousel zone is inert. Steps travel down the same action path
    /// the Next chip uses (ChoiceStep), so the app sees one uniform
    /// consume path.
    pub fn handle_drag(painter: Painter, event: Id) {
        unsafe {
            let loc: (f64, f64) = {
                let f: extern "C" fn(Id, Sel) -> NsPoint =
                    std::mem::transmute(objc_msgSend as *const ());
                let p = f(event, sel(b"locationInWindow\0"));
                (p.x, p.y)
            };
            let local = appkit_to_bitmap_point(loc, CANVAS_SIZE.1);
            let state = painter.state.lock().unwrap().clone();
            if state.choice.is_none() && state.browse.is_none() {
                // Not a carousel moment: reset any half-accumulated
                // travel so a later drag starts from zero.
                *painter.drag_accum.lock().unwrap() = 0.0;
                return;
            }
            let layers = surface_layouts(&state);
            // Swipe zone: the focused item's title rect (the choice and
            // browse carousels share the row).
            let Some(zone) = layers.iter().find(|l| l.id == "choice_title") else {
                return;
            };
            let in_zone = local.0 >= zone.x
                && local.0 <= zone.x + zone.w
                && local.1 >= zone.y
                && local.1 <= zone.y + zone.h;
            if !in_zone {
                return;
            }
            // deltaX: points since the previous drag event (leftward
            // travel is negative). Accumulate until a whole 24pt step.
            let dx: f64 = {
                let f: extern "C" fn(Id, Sel) -> f64 =
                    std::mem::transmute(objc_msgSend as *const ());
                f(event, sel(b"deltaX\0"))
            };
            let mut accum = painter.drag_accum.lock().unwrap();
            for delta in drag_steps(&mut accum, dx) {
                // The engine step: the surface's own input, so the
                // very next render speaks the new focused work. The
                // engine routes it to the browse when one is open.
                if let Some(steps) = painter.steps.as_ref() {
                    let _ = steps.send(SurfaceInput::ChoiceStep(delta));
                }
                // The app notification (uniform consume path).
                if let Some(sender) = painter.actions.as_ref() {
                    let _ = sender.send(SurfaceAction::ChoiceStep(delta));
                }
            }
        }
    }
}

// Keep public names a flat re-export so the demo can use them.
#[cfg(target_os = "macos")]
pub use objc::{BitmapData, HostBlob, Painter};

#[cfg(target_os = "macos")]
impl EvoNotchHost {
    /// Opens the NSPanel over the notch with an `EvoNotchView` filling it.
    /// AppKit windows may only be instantiated on the main thread, and
    /// the notch surface runs on its own thread — this hops there
    /// transparently (blocking until the construction completes).
    pub fn open() -> Result<Self, String> {
        let geometry = probe_notch_geometry()
            .ok_or_else(|| "NSScreen preferred display has no hardware notch".to_string())?;
        Self::open_for_geometry(geometry)
    }

    /// Opens against the exact screen/safe-area measurement already captured
    /// by the surface thread. This prevents probing one display and creating
    /// the panel on another when the selected screen changes.
    pub fn open_for_geometry(geometry: NotchGeometry) -> Result<Self, String> {
        match objc::on_main(move || Self::open_main(geometry)) {
            Some(result) => result,
            None => Err("main queue unavailable for the notch surface".into()),
        }
    }

    /// The actual window construction; only ever runs on the main thread.
    ///
    /// Geometry lifted from the reference app: a generous transparent
    /// canvas top-anchored at the screen top and horizontally centered,
    /// above the menu-bar level (`mainMenu + 3`). The *visible* content
    /// is painted to continue the hardware notch, whose exact rect the
    /// probe reads from `auxiliaryTopLeftArea`/`...RightArea`.
    fn open_main(geometry: NotchGeometry) -> Result<Self, String> {
        use objc::*;
        let (w, h) = CANVAS_SIZE;
        unsafe {
            let panel_class = register_panel_class();
            if panel_class.is_null() {
                return Err("EvoNotchPanel class unavailable".into());
            }
            let screen_frame = geometry.screen_frame;
            // Top-anchored, centered (OverlayWindowLayout.topAnchoredFrame).
            let x = (screen_frame.x + screen_frame.w / 2.0 - w / 2.0).floor();
            // NSWindow y is bottom-left origin; the canvas hangs exactly from
            // the screen top, matching the reference panel's frame origin.
            let y = screen_frame.y + screen_frame.h - h;

            let allocated: Id = send0(panel_class as Id, sel(b"alloc\0"));
            let panel = send_rect(
                allocated,
                sel(b"initWithContentRect:styleMask:backing:defer:\0"),
                NsRect { x, y, w, h },
                (1 << 4) | (1 << 7) | (1 << 13),
                // utilityWindow | nonactivatingPanel | HUDWindow; borderless is 0
                2, // NSBackingStoreBuffered
                0, // defer: NO
            );
            if panel.is_null() {
                return Err("NSPanel init failed".into());
            }
            // Keep this independent from normal document/browser windows.
            // The notch host is a nonactivating overlay and must remain above
            // the reference uses main-menu + 3: high enough for a notch
            // overlay without turning the surface into a screen-saver-level
            // window that obscures unrelated system UI.
            send1::<(), isize>(panel, sel(b"setLevel:\0"), 27);
            // Match DynamicNotch's floating overlay panel. In particular,
            // isFloatingPanel keeps the nonactivating panel in the overlay
            // composition path instead of letting AppKit treat it as a
            // normal document window.
            send1::<(), i8>(panel, sel(b"setFloatingPanel:\0"), 1);
            send1::<(), i8>(panel, sel(b"setOpaque:\0"), 0);
            send1::<(), i8>(panel, sel(b"setHasShadow:\0"), 0);
            send1::<(), i8>(panel, sel(b"setHidesOnDeactivate:\0"), 0);
            send1::<(), i8>(panel, sel(b"setMovable:\0"), 0);
            send1::<(), i8>(panel, sel(b"setReleasedWhenClosed:\0"), 0);
            send1::<(), isize>(panel, sel(b"setTitleVisibility:\0"), 1);
            send1::<(), i8>(panel, sel(b"setTitlebarAppearsTransparent:\0"), 1);
            send1::<(), i8>(panel, sel(b"setAcceptsMouseMovedEvents:\0"), 1);
            send1::<(), i8>(panel, sel(b"setIgnoresMouseEvents:\0"), 0);
            // canJoinAllSpaces | stationary | ignoresCycle | fullScreenAuxiliary
            send1::<(), usize>(panel, sel(b"setCollectionBehavior:\0"), 1 | 16 | 64 | 256);
            // NSWindowAnimationBehaviorNone.
            send1::<(), isize>(panel, sel(b"setAnimationBehavior:\0"), 1);

            // Fully transparent window background: the visible shape is
            // what we paint, never the window fill (the reference sets
            // `backgroundColor = .clear` for the same reason).
            let clear: Id = send0(class_of(b"NSColor\0") as Id, sel(b"clearColor\0"));
            if !clear.is_null() {
                send1::<(), Id>(panel, sel(b"setBackgroundColor:\0"), clear);
            }

            // The view subclass instance.
            let view_class = register_view_class();
            if view_class.is_null() {
                return Err("could not register EvoNotchView".into());
            }
            let view_raw: Id = send0(view_class as Id, sel(b"alloc\0"));
            let view = {
                let f: extern "C" fn(Id, Sel, NsRect) -> Id =
                    std::mem::transmute(objc_msgSend as *const ());
                f(
                    view_raw,
                    sel(b"initWithFrame:\0"),
                    NsRect {
                        x: 0.0,
                        y: 0.0,
                        w,
                        h,
                    },
                )
            };
            if view.is_null() {
                return Err("EvoNotchView init failed".into());
            }
            // Keep both the NSView and its backing CALayer clear. The bitmap
            // paint path clears every frame, but AppKit still composites a
            // layer-backed view's default background unless it is explicitly
            // disabled.
            send1::<(), i8>(view, sel(b"setWantsLayer:\0"), 0);
            // Add tracking area for mouse entered/exited/moved events.
            // NSTrackingMouseEnteredAndExited | NSTrackingMouseMoved | NSTrackingActiveInActiveApp
            let tracking_options: usize = 0x1 | 0x2 | 0x40;
            let tracking_area_class = class_of(b"NSTrackingArea\0");
            if !tracking_area_class.is_null() {
                let tracking_alloc: Id = send0(tracking_area_class as Id, sel(b"alloc\0"));
                let tracking_area: Id = send4(
                    tracking_alloc,
                    sel(b"initWithRect:options:owner:userInfo:\0"),
                    NsRect {
                        x: 0.0,
                        y: 0.0,
                        w,
                        h,
                    },
                    tracking_options,
                    view,
                    std::ptr::null_mut::<c_void>(),
                );
                if !tracking_area.is_null() {
                    send1::<(), Id>(view, sel(b"addTrackingArea:\0"), tracking_area);
                }
            }
            let _: () = send1(panel, sel(b"setContentView:\0"), view);

            // Park the Rust-side painter in the view's ivar.
            let painter = Painter {
                state: std::sync::Arc::new(std::sync::Mutex::new(RenderState::default())),
                actions: None,
                steps: None,
                bitmap: std::sync::Arc::new(std::sync::Mutex::new(None)),
                drag_accum: std::sync::Arc::new(std::sync::Mutex::new(0.0)),
            };
            let blob = Box::into_raw(Box::new(HostBlob {
                painter: painter.clone(),
            }));
            let _ = object_setInstanceVariable(
                view,
                IVAR_NAME.as_ptr() as *const c_char,
                blob as *mut c_void,
            );

            Ok(Self {
                panel,
                view,
                state: painter.state.clone(),
                actions: None,
                visible: std::sync::atomic::AtomicBool::new(false),
            })
        }
    }

    /// Hands one `RenderState` to the view and marks it dirty. The dirty
    /// mark is dispatched to the main queue ASYNCHRONOUSLY — no
    /// `displayIfNeeded`, no per-frame panel ordering — so AppKit
    /// coalesces the actual `drawRect:` into the next vsync instead of
    /// the render loop forcing a synchronous flush per frame. Ordering
    /// the panel in or out is the rarer event and only happens when
    /// `is_visible()` actually changes.
    pub fn render(&self, state: &RenderState) {
        *self.state.lock().unwrap() = state.clone();
        let view = self.view;
        let visible = state.is_visible();
        let visibility_changed = self
            .visible
            .swap(visible, std::sync::atomic::Ordering::SeqCst)
            != visible;
        objc::on_main_async(move || unsafe {
            // BOOL goes as the argument (1 = YES); "YES" is never part of
            // a selector name — `-setNeedsDisplay:` is NSView's method.
            objc::send1::<(), i8>(view, objc::sel(b"setNeedsDisplay:\0"), 1);
        });
        if visibility_changed {
            let panel = self.panel;
            objc::on_main(move || unsafe {
                if visible {
                    // A plain no-arg method: no `:` suffix, no argument.
                    objc::send0::<()>(panel, objc::sel(b"orderFrontRegardless\0"));
                } else {
                    // Empty/idle is an ordered-out state, not a transparent
                    // panel left in the z-order above other applications.
                    objc::send1::<(), objc::Id>(
                        panel,
                        objc::sel(b"orderOut:\0"),
                        std::ptr::null_mut(),
                    );
                }
            });
        }
    }

    /// Orders the host out without requiring a synthetic render frame.
    /// Used when the input channel closes or the surface quits.
    pub fn order_out(&self) {
        self.visible
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let panel = self.panel;
        objc::on_main(move || unsafe {
            objc::send1::<(), objc::Id>(panel, objc::sel(b"orderOut:\0"), std::ptr::null_mut());
        });
    }

    /// Routes click actions to the given sender (the app's pump).
    /// Sets BOTH the host handle and the view's live painter — the
    /// painter is what mouseDown/mouseDragged actually read, so a chip
    /// click is real UI behavior, not a demo-only path.
    pub fn inject_action_sender(&mut self, sender: mpsc::Sender<SurfaceAction>) {
        self.actions = Some(sender.clone());
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ = objc::object_getInstanceVariable(
                self.view,
                objc::IVAR_NAME.as_ptr() as *const c_char,
                &mut holder,
            );
            let blob = holder as *mut objc::HostBlob;
            if !blob.is_null() {
                (*blob).painter.actions = Some(sender);
            }
        }
    }

    /// Wires the surface thread's own input into the view's painter so
    /// drag swipes (ChoiceStep) reach the engine that renders the
    /// carousel. Called by `spawn` after the host opens.
    pub fn inject_step_sender(&mut self, steps: mpsc::Sender<SurfaceInput>) {
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ = objc::object_getInstanceVariable(
                self.view,
                objc::IVAR_NAME.as_ptr() as *const c_char,
                &mut holder,
            );
            let blob = holder as *mut objc::HostBlob;
            if !blob.is_null() {
                (*blob).painter.steps = Some(steps);
            }
        }
    }

    /// Offscreen pixel verification: the last painted bitmap.
    pub fn last_bitmap(&self) -> Option<BitmapData> {
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ = objc::object_getInstanceVariable(
                self.view,
                objc::IVAR_NAME.as_ptr() as *const c_char,
                &mut holder,
            );
            let blob = holder as *mut objc::HostBlob;
            let scale = objc::view_backing_scale(self.view);
            objc::paint_view((*blob).painter.clone(), scale);
            (*blob).painter.bitmap.lock().unwrap().clone()
        }
    }

    /// Sends a synthetic click into the view's action path, exactly like
    /// a real mouseDown would dispatch (the demo uses this to verify the
    /// click handling without inventing CGEvents).
    pub fn synthetic_click(&self, x: f64, y: f64) {
        use objc::*;
        unsafe {
            let mut holder: *mut c_void = std::ptr::null_mut();
            let _ = objc::object_getInstanceVariable(
                self.view,
                IVAR_NAME.as_ptr() as *const c_char,
                &mut holder,
            );
            let blob = holder as *mut HostBlob;
            let painter = (*blob).painter.clone();
            let state = painter.state.lock().unwrap().clone();
            let layers = surface_layouts(&state);
            if let Some(layer) = hit_test(&layers, x, y)
                && let Some(action) = layer.action
            {
                let dispatch = match action {
                    OptionAction::Dismiss => {
                        if let Some(sender) = painter.steps.as_ref() {
                            let _ = sender.send(SurfaceInput::Dismiss);
                        }
                        dispatch_for_action(&state, action)
                    }
                    _ => dispatch_for_action(&state, action),
                };
                if let Some(sender) = self.actions.as_ref() {
                    let _ = sender.send(dispatch.clone());
                }
                if let Some(sender) = painter.actions.as_ref() {
                    let _ = sender.send(dispatch);
                }
            }
        }
    }

    pub fn panel_id(&self) -> usize {
        self.panel as usize
    }
}

// ─── Safe-area probe: the hardware notch's exact rect ─────────────────────

/// Reads the complete hardware notch geometry from the preferred selected
/// display's safe areas. The same frame is later used for the host window, so
/// the painted silhouette and its transparent canvas share one anchor.
#[cfg(target_os = "macos")]
pub fn probe_notch_geometry() -> Option<NotchGeometry> {
    objc::on_main(|| unsafe {
        let main: objc::Id = objc::preferred_screen();
        if main.is_null() {
            return None;
        }
        let frame: objc::NsRect = objc::send0(main, objc::sel(b"frame\0"));
        let left: objc::NsRect = objc::send0(main, objc::sel(b"auxiliaryTopLeftArea\0"));
        let right: objc::NsRect = objc::send0(main, objc::sel(b"auxiliaryTopRightArea\0"));
        let safe: objc::NsEdgeInsets = objc::send0(main, objc::sel(b"safeAreaInsets\0"));
        NotchGeometry::from_safe_areas(
            ScreenFrame {
                x: frame.x,
                y: frame.y,
                w: frame.w,
                h: frame.h,
            },
            left.w,
            right.w,
            safe.top,
        )
    })
    .flatten()
}

#[cfg(not(target_os = "macos"))]
pub fn probe_notch_geometry() -> Option<NotchGeometry> {
    None
}

/// Compatibility view of the safe-area probe for callers that only need the
/// physical width and height.
pub fn probe_notch() -> Option<(f64, f64)> {
    probe_notch_geometry().map(|geometry| (geometry.notch_w, geometry.notch_h))
}

/// One alignment haptic when a phase transition starts — the small
/// "the surface is moving" confirmation the reference app gives on a
/// Force Touch trackpad. Runs on the main thread (that is where AppKit
/// wants feedback performed), obeys the system haptic setting, and is
/// silently a no-op on Macs without a haptic device (the system
/// reserves the right to suppress the request). Every step is
/// null-guarded, so the call is panic-safe from the surface thread.
#[cfg(target_os = "macos")]
fn haptic_alignment() {
    objc::on_main(|| unsafe {
        let manager_class = objc::class_of(b"NSHapticFeedbackManager\0");
        if manager_class.is_null() {
            return;
        }
        let performer: objc::Id =
            objc::send0(manager_class as objc::Id, objc::sel(b"defaultPerformer\0"));
        if performer.is_null() {
            return;
        }
        // NSHapticFeedbackPatternAlignment = 1.
        // NSHapticFeedbackPerformanceTimeNow = 1: fire immediately, no
        // draw-completion sync (performFeedbackPattern:performanceTime:
        // takes NSInteger pattern, NSUInteger time).
        objc::send2::<(), isize, isize>(
            performer,
            objc::sel(b"performFeedbackPattern:performanceTime:\0"),
            1,
            1,
        );
    });
}

// ─── Occupancy probe (CGWindowList, name-only, no data retention) ───────────

/// Returns true when `window` is an OS-blessed chrome name we must not treat
/// as a competing notch app.
fn is_os_chrome(owner: &str) -> bool {
    matches!(
        owner,
        "Window Server" | "Dock" | "SystemUIServer" | "Control Center"
    )
}

/// Ordinary browser/Evo document windows can overlap the top edge without
/// owning the physical notch. A foreign surface must both be elevated and
/// identify itself as a known notch utility. This is intentionally an
/// allowlist: an unrelated HUD, menu, permission prompt, or window-manager
/// overlay must never push Evo away from the physical camera cutout.
fn is_known_notch_owner(owner: &str) -> bool {
    let normalized = owner.to_ascii_lowercase();
    [
        "dynamicnotch",
        "boring notch",
        "boringnotch",
        "notchnook",
        "topnotch",
        "notchbar",
    ]
    .iter()
    .any(|name| normalized.contains(name))
}

fn is_foreign_notch_window(owner: &str, layer: i32, alpha: f64, pid: i32, my_pid: i32) -> bool {
    alpha > 0.01
        && pid != my_pid
        && layer >= 20
        && !is_os_chrome(owner)
        && is_known_notch_owner(owner)
}

/// Counts non-Evo windows overlapping the notch strip — centered top
/// ~600×60 points on the main display. Reads the on-screen window list
/// at the name/dictionary level (no pixel capture, no retention). The
/// decision logic lives in [`crate::notch::decide_surface`], fully
/// tested there.
///
/// On any error the probe returns 0 (treat the strip as free — the
/// honest default for machines without a hardware notch at all).
#[cfg(target_os = "macos")]
pub fn occupancy_probe() -> usize {
    use std::os::raw::{c_char, c_void};

    // Signed the same way as pod_surface's declarations so the linker
    // sees one ABI spelling for each CoreFoundation symbol.
    type CFTypeRef = *const c_void;
    type CFArrayRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFNumberRef = *const c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGWindowListCopyWindowInfo(option: u32, window_id: u32) -> CFArrayRef;
        fn CFArrayGetCount(array: CFArrayRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CFArrayRef, index: isize) -> CFDictionaryRef;
        fn CFDictionaryGetValue(dict: CFDictionaryRef, key: CFStringRef) -> CFTypeRef;
        fn CFStringCreateWithCString(
            alloc: CFTypeRef,
            c_str: *const c_char,
            encoding: u32,
        ) -> CFStringRef;
        fn CFStringGetCString(
            the_string: CFStringRef,
            buffer: *mut c_char,
            buffer_size: isize,
            encoding: u32,
        ) -> i8;
        fn CFNumberGetValue(number: CFNumberRef, kind: isize, value: *mut c_void) -> i8;
        fn CFRelease(type_: CFTypeRef);
        // (mutable in-params below use local *mut spellings)
        fn CGRectMakeWithDictionaryRepresentation(dict: CFDictionaryRef, rect: *mut CGRect) -> i8;
        fn CGRectIntersectsRect(a: CGRect, b: CGRect) -> i8;
        fn CGMainDisplayID() -> u32;
        fn CGDisplayBounds(display: u32) -> CGRect;
        fn CGGetActiveDisplayList(
            max_displays: u32,
            active_displays: *mut u32,
            display_count: *mut u32,
        ) -> i32;
        fn CGDisplayIsBuiltin(display: u32) -> u32;
    }

    // kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements
    const OPTION: u32 = (1 << 0) | (1 << 4);
    const NULL_WINDOW: u32 = 0;
    const UTF8: u32 = 0x0800_0600;
    // kCFNumberFloat64Type / kCFNumberSInt32Type
    const FLOAT64: isize = 6;
    const SINT32: isize = 3;

    /// One dictionary lookup with a freshly created name key. The key is
    /// +1 and released right after the lookup (CFDictionaryGetValue only
    /// borrows it), so probing leaks nothing. A null key can only mean
    /// allocation failure; the lookup then honestly finds nothing.
    fn lookup(dict: CFDictionaryRef, bytes: &[u8]) -> CFTypeRef {
        unsafe {
            let name = CFStringCreateWithCString(
                std::ptr::null_mut(),
                bytes.as_ptr() as *const c_char,
                UTF8,
            );
            if name.is_null() {
                return std::ptr::null();
            }
            let value = CFDictionaryGetValue(dict, name);
            CFRelease(name);
            value
        }
    }

    unsafe {
        // Auto targets the built-in notched display even when macOS has an
        // external monitor selected as `mainScreen`.
        let mut displays = [0u32; 16];
        let mut display_count = 0u32;
        let display = if CGGetActiveDisplayList(
            displays.len() as u32,
            displays.as_mut_ptr(),
            &mut display_count,
        ) == 0
        {
            displays[..display_count.min(displays.len() as u32) as usize]
                .iter()
                .copied()
                .find(|display| CGDisplayIsBuiltin(*display) != 0)
                .unwrap_or_else(|| CGMainDisplayID())
        } else {
            CGMainDisplayID()
        };
        let bounds = CGDisplayBounds(display);
        let (screen_x, screen_y, screen_w) = (bounds.x, bounds.y, bounds.w);
        if screen_w <= 0.0 {
            return 0;
        }
        let my_pid = std::process::id() as i32;

        let list = CGWindowListCopyWindowInfo(OPTION, NULL_WINDOW);
        if list.is_null() {
            return 0;
        }
        let count = CFArrayGetCount(list);
        let mut overlapping = 0usize;
        for i in 0..count {
            let dict = CFArrayGetValueAtIndex(list, i);
            if dict.is_null() {
                continue;
            }

            // Transparent helper panels don't visibly occupy the strip.
            let alpha_ptr = lookup(dict, b"kCGWindowAlpha\0") as CFNumberRef;
            let mut alpha = 1.0f64;
            if !alpha_ptr.is_null() {
                CFNumberGetValue(alpha_ptr, FLOAT64, &mut alpha as *mut f64 as *mut c_void);
            }

            // Read the owner once so OS chrome and ordinary app windows can
            // be excluded by the same ownership predicate.
            let owner_ref = lookup(dict, b"kCGWindowOwnerName\0") as CFStringRef;
            let owner = if !owner_ref.is_null() {
                let mut buf = [0u8; 128];
                if CFStringGetCString(owner_ref, buf.as_mut_ptr() as *mut c_char, 128, UTF8) != 0 {
                    std::ffi::CStr::from_ptr(buf.as_ptr() as *const std::os::raw::c_char)
                        .to_string_lossy()
                        .into_owned()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // Skip our own windows and ordinary document windows.  A browser
            // or Evo window at layer 0 can touch the top edge but cannot own
            // the hardware notch; a real notch host is an elevated overlay.
            let pid_ptr = lookup(dict, b"kCGWindowOwnerPID\0") as CFNumberRef;
            let mut pid = -1i32;
            if !pid_ptr.is_null() {
                CFNumberGetValue(pid_ptr, SINT32, &mut pid as *mut i32 as *mut c_void);
            }
            let layer_ptr = lookup(dict, b"kCGWindowLayer\0") as CFNumberRef;
            let mut layer = 0i32;
            if !layer_ptr.is_null() {
                CFNumberGetValue(layer_ptr, SINT32, &mut layer as *mut i32 as *mut c_void);
            }
            if !is_foreign_notch_window(&owner, layer, alpha, pid, my_pid) {
                continue;
            }

            // kCGWindowBounds is the CG window-bounds dictionary;
            // CGRectMakeWithDictionaryRepresentation parses it to points
            // in top-left coordinates (same space as our strip).
            let bounds_ptr = lookup(dict, b"kCGWindowBounds\0") as CFDictionaryRef;
            let mut rect = CGRect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            };
            if bounds_ptr.is_null()
                || CGRectMakeWithDictionaryRepresentation(bounds_ptr, &mut rect) == 0
            {
                continue;
            }
            let strip_min_x = screen_w / 2.0 - 300.0;
            let strip = CGRect {
                x: screen_x + strip_min_x,
                y: screen_y,
                w: 600.0,
                h: 60.0,
            };
            if CGRectIntersectsRect(strip, rect) != 0 {
                overlapping += 1;
            }
        }
        CFRelease(list);
        overlapping
    }
}

#[cfg(not(target_os = "macos"))]
pub fn occupancy_probe() -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_layout_is_completely_empty_even_for_legacy_verify_state() {
        let state = RenderState {
            idle_glyph: true,
            verify: Some(VerifyCase::IdleGlyph),
            ..Default::default()
        };
        let layers = surface_layouts(&state);
        assert!(layers.is_empty(), "idle never paints Evo pixels");
        assert!(!state.is_visible(), "idle orders the host out");
    }

    #[test]
    fn normal_idle_is_empty_and_cannot_paint_an_idle_mark() {
        let state = RenderState::default();
        let layers = surface_layouts(&state);
        assert!(layers.is_empty(), "idle orders the host out entirely");
        assert!(!state.is_visible());
        assert!(layers.iter().all(|layer| layer.id != "idle_mark"));
    }

    #[test]
    fn safe_area_geometry_anchors_the_silhouette_to_the_selected_screen() {
        let geometry = NotchGeometry::from_safe_areas(
            ScreenFrame {
                x: -1440.0,
                y: 0.0,
                w: 1728.0,
                h: 1117.0,
            },
            700.0,
            700.0,
            37.0,
        )
        .expect("valid notch safe areas");
        assert_eq!(geometry.notch_w, 332.0);
        assert_eq!(geometry.notch_x, -742.0);
        let state = RenderState {
            listening: true,
            notch_w: geometry.notch_w,
            notch_h: geometry.notch_h,
            notch_x: geometry.canvas_notch_x(),
            ..Default::default()
        };
        let card = surface_layouts(&state)
            .into_iter()
            .find(|layer| layer.id == "card")
            .expect("expanded silhouette");
        assert!((card.x + card.w / 2.0 - state.notch_x - state.notch_w / 2.0).abs() < 1e-9);
        assert_eq!(card.top_radius, CARD_TOP_RADIUS);
        assert_eq!(card.bottom_radius, CARD_BOTTOM_RADIUS);
        assert_eq!(card.top_radius, 19.0, "the reference's opened top radius");
        assert_eq!(
            card.bottom_radius, 24.0,
            "the reference's opened bottom radius"
        );
    }

    #[test]
    fn transcript_is_at_most_two_lines_and_each_line_fits_width() {
        let lines = bounded_transcript(
            "this is a deliberately long live transcript that must collapse cleanly",
            120.0,
            14.0,
        );
        let max_chars = (120.0_f64 / (14.0_f64 * 0.60_f64)).floor() as usize;
        assert!(!lines.is_empty());
        assert!(lines.len() <= MAX_TRANSCRIPT_LINES);
        assert!(lines.iter().all(|line| line.chars().count() <= max_chars));
        assert!(lines.last().is_some_and(|line| line.ends_with('…')));
    }

    #[test]
    fn expanded_corner_radii_collapse_to_attached_notch_shape() {
        let state = RenderState {
            listening: true,
            expand: 0.0,
            ..Default::default()
        };
        let card = surface_layouts(&state)
            .into_iter()
            .find(|layer| layer.id == "card")
            .expect("collapsed silhouette");
        assert_eq!(card.w, state.notch_w);
        assert_eq!(card.h, state.notch_h);
        assert_eq!(card.top_radius, CARD_COLLAPSED_TOP_RADIUS);
        assert_eq!(card.bottom_radius, CARD_COLLAPSED_BOTTOM_RADIUS);
        assert_eq!(
            card.top_radius, 0.0,
            "square top edge, flush with the bezel"
        );
        assert_eq!(card.bottom_radius, 8.0);
    }

    #[test]
    fn corner_radii_interpolate_linearly_with_expand() {
        // 0 → 19 on top (square at the closed notch) and 8 → 24 on the
        // bottom, on exactly the same expand factor the size rides. A
        // mid-flight frame must sit on the linear path, not snap.
/// Radii now progress through Sapphire's motion grammar spring
        // (response 0.40, damping 0.55 for expand): the effective progress is
        // physics-based with controlled overshoot. Mid-flight frames may briefly
        // exceed the final target before settling to exactly 1.0.
        for (expand, top, bottom) in [
            (0.0, 0.0, 8.0),
            (0.25, 42.76, 44.01),
            (0.5, 9.55, 16.04),
            (1.0, 19.0, 24.0),
        ] {
            let state = RenderState {
                listening: true,
                expand,
                animating_direction: SpringDirection::Expand,
                ..Default::default()
            };
            let card = surface_layouts(&state)
                .into_iter()
                .find(|layer| layer.id == "card")
                .expect("card");
            assert!(
                (card.top_radius - top).abs() < 1e-9,
                "top radius at expand={expand} is {top}, got {}",
                card.top_radius
            );
            assert!(
                (card.bottom_radius - bottom).abs() < 1e-9,
                "bottom radius at expand={expand} is {bottom}, got {}",
                card.bottom_radius
            );
        }
    }

    #[test]
    fn surface_layouts_listening_has_three_chips_and_transcript() {
        let state = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            ..Default::default()
        };
        let layers = surface_layouts(&state);
        let chips: Vec<_> = layers
            .iter()
            .filter(|l| l.id.starts_with("chip_"))
            .collect();
        assert_eq!(chips.len(), 3, "restore/rack/dismiss chips");
        assert!(layers.iter().any(|l| l.id == "transcript"));
        assert!(layers.iter().any(|l| l.id == "mic_ring"));
        // The card covers the hardware notch rect: it is at least as
        // tall and expands from the notch band's own center inside the
        // canvas.
        let card = layers.iter().find(|l| l.id == "card").expect("card");
        assert!(card.h >= state.notch_h);
        let (cw, _ch) = CANVAS_SIZE;
        let (nw, _nh) = (state.notch_w, state.notch_h);
        let card_center = card.x + card.w / 2.0;
        let notch_center = (cw - nw) / 2.0 + nw / 2.0;
        assert!(
            (card_center - notch_center).abs() < 1.0,
            "centered on the notch"
        );
        assert!(
            layers
                .iter()
                .find(|layer| layer.id == "transcript")
                .is_some_and(
                    |layer| layer.h <= MAX_TRANSCRIPT_LINES as f64 * text_row_h(FONT_UI_TRANSCRIPT)
                ),
            "transcript is capped at two ascent-safe lines"
        );
    }

    #[test]
    fn floating_decision_offsets_the_surface_below_the_hardware_strip() {
        let state = RenderState {
            listening: true,
            decision: SurfaceDecision::Float,
            ..Default::default()
        };
        let layers = surface_layouts(&state);
        let card = layers
            .iter()
            .find(|layer| layer.id == "card")
            .expect("card");
        assert_eq!(
            card.y,
            state.notch_h + FLOAT_GAP,
            "yielded surfaces start below the physical notch"
        );
        assert!(
            layers
                .iter()
                .all(|layer| layer.y >= state.notch_h + FLOAT_GAP),
            "a floating surface never paints into the owned strip"
        );
    }

    #[test]
    fn carousel_pips_stay_bounded_for_large_candidate_sets() {
        let candidates = (0..32)
            .map(|index| ChoiceCandidate {
                work_id: index.to_string(),
                title: format!("Candidate {index}"),
                selection: evo_voice::VoiceSelection::Work,
            })
            .collect();
        let state = RenderState {
            choice: Some(ChoiceState {
                candidates,
                index: 31,
            }),
            ..Default::default()
        };
        let layers = surface_layouts(&state);
        let card = layers
            .iter()
            .find(|layer| layer.id == "card")
            .expect("card");
        let pips: Vec<_> = layers
            .iter()
            .filter(|layer| layer.id == "choice_pip")
            .collect();
        assert_eq!(pips.len(), 8, "large carousels use a bounded progress cue");
        assert!(pips.iter().all(|pip| {
            pip.x >= card.x
                && pip.x + pip.w <= card.x + card.w
                && pip.y >= card.y
                && pip.y + pip.h <= card.y + card.h
        }));
    }

    #[test]
    fn hit_test_returns_topmost_under_point_and_none_outside() {
        let state = RenderState {
            listening: true,
            ..Default::default()
        };
        let layers = surface_layouts(&state);
        let card = layers.iter().find(|l| l.id == "card").expect("card");
        let transcript = layers
            .iter()
            .find(|l| l.id == "transcript")
            .expect("transcript");
        // Center of the transcript bucket — primary tap target zone.
        let (tx, ty) = (transcript.x + 10.0, transcript.y + 5.0);
        let hit = hit_test(&layers, tx, ty).expect("transcript hit");
        assert_eq!(hit.id, "transcript");
        assert!(hit_test(&layers, -1.0, -1.0).is_none(), "outside");
        assert!(
            hit_test(&layers, CANVAS_SIZE.0 + 10.0, CANVAS_SIZE.1 + 10.0).is_none(),
            "outside expanded canvas"
        );
        // Any chip is tappable at its center.
        let chip = layers
            .iter()
            .find(|l| l.id.starts_with("chip_") && l.action.is_some())
            .expect("chip");
        let (cx, cy) = (chip.x + card.x.abs() + 4.0, chip.y + chip.h / 2.0);
        let chip_hit = hit_test(&layers, cx, cy).expect("chip hit");
        assert!(chip_hit.id.starts_with("chip_"), "got {}", chip_hit.id);
        assert!(chip_hit.action.is_some());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn paint_into_produces_recognizable_pixels_per_case() {
        use objc::paint_into;

        // Listening: the header's live mark is the mic RING — a 3pt
        // caramel center dot inside a caramel@0.18 track, banded by a
        // 1.5pt caramel stroke — followed by the equalizer. Every sample
        // point is read from the layout instead of a hard-coded
        // coordinate (the card is content-sized, so rows move with the
        // font metrics).
        let listening = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            verify: Some(VerifyCase::Listening),
            ..Default::default()
        };
        let listening_layers = surface_layouts(&listening);
        let mic = listening_layers
            .iter()
            .find(|layer| layer.id == "mic_ring")
            .expect("mic ring")
            .clone();
        let dot = listening_layers
            .iter()
            .find(|layer| layer.id == "mic_dot")
            .expect("mic dot")
            .clone();
        let bmp = paint_into(&listening);
        let center = (mic.x + mic.w / 2.0, mic.y + mic.h / 2.0);
        let is_ring_caramel = |p: [u8; 4]| -> bool {
            p[3] > 200 && p[0] > 180 && (130..180).contains(&p[1]) && (80..130).contains(&p[2])
        };
        // The center dot is the caramel live mark (R~200, G~155, B~107).
        let dot_px = bmp.pixel(center.0 as usize, center.1 as usize);
        assert!(
            is_ring_caramel(dot_px),
            "the ring's center dot is caramel, got {dot_px:?}"
        );
        assert!(
            (dot.x + dot.w / 2.0 - center.0).abs() < 1e-9
                && (dot.y + dot.h / 2.0 - center.1).abs() < 1e-9,
            "the dot is centered in the ring"
        );
        // A pixel inside the stroke band (radius 4.5..6) is solid
        // caramel; a pixel in the track (radius 2..4, outside the dot) is
        // the dim caramel@0.18 fill. Finding them by radius keeps the
        // test independent of the exact header metrics.
        let mut band_px = None;
        let mut track_px = None;
        for py in mic.y as usize..(mic.y + mic.h) as usize {
            for px in mic.x as usize..(mic.x + mic.w) as usize {
                let dx = px as f64 + 0.5 - center.0;
                let dy = py as f64 + 0.5 - center.1;
                let radius = (dx * dx + dy * dy).sqrt();
                if band_px.is_none() && (4.5..6.0).contains(&radius) {
                    band_px = Some(bmp.pixel(px, py));
                }
                if track_px.is_none() && (2.0..4.0).contains(&radius) {
                    track_px = Some(bmp.pixel(px, py));
                }
            }
        }
        let band_px = band_px.expect("a stroke-band pixel inside the ring rect");
        let track_px = track_px.expect("a track pixel inside the ring rect");
        assert!(
            is_ring_caramel(band_px),
            "the 1.5pt ring stroke is caramel, got {band_px:?}"
        );
        assert!(
            track_px[3] > 200
                && track_px[0] > track_px[1]
                && track_px[1] > track_px[2]
                && track_px[0] < 100,
            "the ring track is dim caramel over the card, got {track_px:?}"
        );
        assert!(
            band_px[0] > track_px[0] + 40,
            "the stroke reads clearly over its own track ({band_px:?} vs {track_px:?})"
        );
        // The card's background at top-left of the canvas is transparent
        // (outside the notch band) — alpha 0, nothing painted there.
        let bg = bmp.pixel(4, 200);
        assert_eq!(bg[3], 0, "canvas stays transparent outside the card");
        // Inside the card, near the bottom center: dark card fill.
        let inside = bmp.pixel(bmp.width / 2, 80);
        assert!(inside[3] > 200, "card painted inside, got {inside:?}");
        assert_eq!(
            (bmp.width, bmp.height),
            (CANVAS_SIZE.0 as usize, CANVAS_SIZE.1 as usize)
        );

        // Outcome: the caramel accent is INK on the eyebrow, never a
        // full-width band. The old assertion sampled the middle of the
        // `outcome_band` slab; that layer is gone on purpose (a full-width
        // accent rectangle measured 1.1:1 contrast against its own label).
        let outcome = RenderState {
            outcome: true,
            outcome_content: Some(OutcomeState {
                eyebrow: "RESTORED".into(),
                title: "Writing".into(),
                detail: "Opened 1 artifact".into(),
            }),
            verify: Some(VerifyCase::Outcome),
            ..Default::default()
        };
        let outcome_layers = surface_layouts(&outcome);
        assert!(
            outcome_layers
                .iter()
                .all(|layer| layer.id != "outcome_band"),
            "the outcome card has no full-width accent rectangle"
        );
        let eyebrow = outcome_layers
            .iter()
            .find(|layer| layer.id == "eyebrow")
            .expect("outcome eyebrow");
        let bmp = paint_into(&outcome);
        let is_caramel = |p: [u8; 4]| -> bool {
            p[3] > 200 && p[0] > 150 && p[1] > 100 && p[2] < 140 && p[0] > p[2] + 30
        };
        let mut caramel_ink = 0usize;
        let mut row_total = 0usize;
        for py in eyebrow.y as usize..(eyebrow.y + eyebrow.h) as usize {
            for px in eyebrow.x as usize..(eyebrow.x + eyebrow.w) as usize {
                row_total += 1;
                if is_caramel(bmp.pixel(px, py)) {
                    caramel_ink += 1;
                }
            }
        }
        assert!(caramel_ink > 0, "the eyebrow label is caramel ink");
        assert!(
            caramel_ink * 4 < row_total,
            "caramel is sparse glyph strokes, not a slab ({caramel_ink}/{row_total})"
        );
        // Past the end of the label the row is bare card, which is what
        // proves there is no accent rectangle behind the text.
        let tail = bmp.pixel(
            (eyebrow.x + eyebrow.w - 6.0) as usize,
            (eyebrow.y + eyebrow.h / 2.0) as usize,
        );
        assert!(
            !is_caramel(tail) && tail[3] > 200,
            "the eyebrow row's tail is bare card, got {tail:?}"
        );

        // Idle: no visible Evo pixels at all. The host receives this empty
        // layout and orders its transparent panel out.
        let idle = RenderState {
            idle_glyph: true,
            verify: Some(VerifyCase::IdleGlyph),
            ..Default::default()
        };
        let bmp = paint_into(&idle);
        assert!(bmp.pixels.chunks_exact(4).all(|pixel| pixel[3] == 0));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn paint_into_draws_real_glyph_pixels_in_the_transcript_rect() {
        // With a partial transcript present, the transcript layer must
        // carry actual glyph pixels (light text) inside its rect — not
        // just the flat placeholder fill.
        use objc::paint_into;

        let listening = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            ..Default::default()
        };
        let layers = surface_layouts(&listening);
        let transcript = layers
            .iter()
            .find(|l| l.id == "transcript")
            .expect("transcript layer")
            .clone();
        let bmp = paint_into(&listening);

        // Inside the transcript rect the card fill is dark; glyph ink is
        // light (#ECEAE6). Count *light* pixels — they exist only where
        // glyphs were rasterized, so this asserts real text, not the
        // placeholder fill.
        let is_light = |p: [u8; 4]| -> bool { p[3] > 0 && p[0] > 140 && p[1] > 140 };
        let mut lit = 0usize;
        let mut total = 0usize;
        for py in transcript.y as usize..(transcript.y + transcript.h) as usize {
            for px in transcript.x as usize..(transcript.x + transcript.w) as usize {
                total += 1;
                if is_light(bmp.pixel(px, py)) {
                    lit += 1;
                }
            }
        }
        // Glyphs occupy a slice of the rect: real text coverage is
        // sparse; a full light fill would be the placeholder bug.
        assert!(lit > 0, "glyph pixels present in the transcript rect");
        assert!(
            lit * 5 < total,
            "text is sparse glyphs, not a full light fill ({lit}/{total})"
        );

        // Without a partial the row still speaks: it paints the `say a
        // command` prompt in MUTED ink. The previous assertion required zero
        // glyphs here, which is what left ~70% of the listening card dead
        // black on screen. The prompt must be clearly muted, never primary
        // ink pretending to be a real transcript.
        let quiet = RenderState {
            listening: true,
            partial: None,
            ..Default::default()
        };
        let bmp = paint_into(&quiet);
        let quiet_transcript = surface_layouts(&quiet)
            .iter()
            .find(|l| l.id == "transcript")
            .expect("transcript layer")
            .clone();
        let mut quiet_lit = 0usize;
        let mut peak = 0u8;
        for py in quiet_transcript.y as usize..(quiet_transcript.y + quiet_transcript.h) as usize {
            for px in
                quiet_transcript.x as usize..(quiet_transcript.x + quiet_transcript.w) as usize
            {
                let p = bmp.pixel(px, py);
                peak = peak.max(p[0]);
                if p[3] > 0 && p[0] > 90 {
                    quiet_lit += 1;
                }
            }
        }
        assert!(quiet_lit > 0, "the quiet transcript paints its prompt");
        // INK_MUTED peaks near 168; INK_PRIMARY would peak near 237.
        assert!(
            (150..210).contains(&peak),
            "the prompt is muted ink, not a live transcript (peak {peak})"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn paint_into_draws_eyebrow_label_glyphs() {
        // The LISTENING eyebrow renders its label with glyphs, sparse
        // like real text, never a solid block.
        use objc::paint_into;
        let listening = RenderState {
            listening: true,
            partial: Some("x".into()),
            ..Default::default()
        };
        let layers = surface_layouts(&listening);
        let eyebrow = layers
            .iter()
            .find(|l| l.id == "eyebrow")
            .expect("eyebrow layer")
            .clone();
        let bmp = paint_into(&listening);
        // Eyebrow glyphs are dark ink on the dark card — count pixels
        // that are *brighter than the card fill* (the eyebrow color is
        // teal #5B8A8F ≤ marker: g/b channel above the card's 0.05 fill
        // scaled by the canvas but well below the light transcript ink).
        let has_teal = |p: [u8; 4]| -> bool { p[3] > 0 && p[1] > 90 && p[2] > 90 && p[0] < 140 };
        let mut lit = 0usize;
        let mut total = 0usize;
        for py in eyebrow.y as usize..(eyebrow.y + eyebrow.h) as usize {
            for px in eyebrow.x as usize..(eyebrow.x + eyebrow.w) as usize {
                total += 1;
                if has_teal(bmp.pixel(px, py)) {
                    lit += 1;
                }
            }
        }
        assert!(lit > 0, "eyebrow glyphs drawn");
        assert!(lit * 3 < total, "eyebrow is stroke-like, not a block");
    }

    fn carousel_state() -> RenderState {
        RenderState {
            choice: Some(ChoiceState {
                candidates: vec![
                    ChoiceCandidate {
                        work_id: "7".into(),
                        title: "Yesterday's notes".into(),
                        selection: evo_voice::VoiceSelection::Work,
                    },
                    ChoiceCandidate {
                        work_id: "9".into(),
                        title: "Morning writing".into(),
                        selection: evo_voice::VoiceSelection::Work,
                    },
                    ChoiceCandidate {
                        work_id: "11".into(),
                        title: "Claim drafts".into(),
                        selection: evo_voice::VoiceSelection::Work,
                    },
                ],
                index: 0,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn choice_layout_is_taller_than_listening_and_self_consistent() {
        let listening = RenderState {
            listening: true,
            ..Default::default()
        };
        let listening_card_h = surface_layouts(&listening)
            .iter()
            .find(|l| l.id == "card")
            .expect("listening card")
            .h;
        let state = carousel_state();
        let layers = surface_layouts(&state);
        let card = layers.iter().find(|l| l.id == "card").expect("choice card");
        assert!(
            card.h > listening_card_h,
            "the carousel expands a little more than listening ({:?} vs {:?})",
            card.h,
            listening_card_h
        );
        // The candidate title zone exists inside the card.
        let title = layers
            .iter()
            .find(|l| l.id == "choice_title")
            .expect("choice_title layer");
        assert!(title.x >= card.x && title.y >= card.y);
        assert!(title.x + title.w <= card.x + card.w);
        assert!(title.y + title.h <= card.y + card.h);
        // One pip per candidate.
        let pips: Vec<_> = layers.iter().filter(|l| l.id == "choice_pip").collect();
        assert_eq!(pips.len(), 3, "one pip per candidate");
        // Chips do not overlap the pip zone or each other: same row,
        // disjoint x ranges.
        let chips: Vec<_> = layers
            .iter()
            .filter(|l| l.id.starts_with("chip_") && l.action.is_some())
            .collect();
        assert_eq!(chips.len(), 4, "restore/prev/next/close");
        for pair in chips.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(
                a.x + a.w <= b.x || b.x + b.w <= a.x,
                "chips {} and {} overlap",
                a.id,
                b.id
            );
            assert!(a.y >= title.y + title.h, "chips sit below the title zone");
        }
        // Restore carries the caramel accent; steps and dismiss are dark.
        let restore = chips.iter().find(|l| l.id == "chip_restore").unwrap();
        assert!(restore.r > 0.7 && restore.g > 0.5, "restore is caramel");
    }

    #[test]
    fn choice_hit_test_routes_chips_and_the_title_zone_takes_clicks() {
        let state = carousel_state();
        let layers = surface_layouts(&state);
        // Restore chip center dispatches Restore.
        let restore = layers.iter().find(|l| l.id == "chip_restore").unwrap();
        let hit = hit_test(
            &layers,
            restore.x + restore.w / 2.0,
            restore.y + restore.h / 2.0,
        )
        .expect("restore hit");
        assert_eq!(hit.id, "chip_restore");
        assert_eq!(hit.action, Some(OptionAction::Restore));
        // Next chip center dispatches Next.
        let next = layers.iter().find(|l| l.id == "chip_next").unwrap();
        let hit =
            hit_test(&layers, next.x + next.w / 2.0, next.y + next.h / 2.0).expect("next hit");
        assert_eq!(hit.action, Some(OptionAction::Next));
        // The title zone exists under a point in its middle.
        let title = layers.iter().find(|l| l.id == "choice_title").unwrap();
        let hit = hit_test(&layers, title.x + title.w / 2.0, title.y + title.h / 2.0)
            .expect("title zone hit");
        assert_eq!(hit.id, "choice_title");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn paint_into_draws_real_glyph_pixels_in_the_choice_title_rect() {
        // The focused candidate's title is real text (light ink glyphs,
        // sparse), not a placeholder block — the same standard the
        // transcript test holds.
        use objc::paint_into;
        let state = carousel_state();
        let layers = surface_layouts(&state);
        let title = layers
            .iter()
            .find(|l| l.id == "choice_title")
            .expect("choice_title layer")
            .clone();
        let bmp = paint_into(&state);
        let is_light = |p: [u8; 4]| -> bool { p[3] > 0 && p[0] > 140 && p[1] > 140 };
        let mut lit = 0usize;
        let mut total = 0usize;
        for py in title.y as usize..(title.y + title.h) as usize {
            for px in title.x as usize..(title.x + title.w) as usize {
                total += 1;
                if is_light(bmp.pixel(px, py)) {
                    lit += 1;
                }
            }
        }
        assert!(lit > 0, "candidate title glyph pixels present");
        assert!(lit * 5 < total, "title is sparse glyphs, not a light fill");

        // No candidates → no glyphs (honest quiet case).
        let quiet = RenderState::default();
        let quiet_layers = surface_layouts(&quiet);
        assert!(
            quiet_layers.iter().all(|l| l.id != "choice_title"),
            "no carousel layout without candidates"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn paint_into_draws_the_choice_subtitle_glyphs() {
        // "work N of M · swipe or click" renders as muted glyphs: any
        // ink distinct from the card fill (0.04/0.05/0.06 ≈ 10-15) at
        // all — the detector only has to catch text-vs-nothing.
        use objc::paint_into;
        let state = carousel_state();
        let layers = surface_layouts(&state);
        let subtitle = layers
            .iter()
            .find(|l| l.id == "choice_subtitle")
            .expect("choice_subtitle layer")
            .clone();
        let bmp = paint_into(&state);
        // The subtitle bounds ship as floats (choice_top + 28.0);
        // rasterization clips at usize floors, so scan the rows around
        // the rect (floored bounds ±1 row) instead of assuming integer
        // alignment.
        let x0 = subtitle.x as usize;
        let x1 = (subtitle.x + subtitle.w) as usize;
        let y0 = subtitle.y as usize;
        let y1 = (subtitle.y + subtitle.h) as usize;
        let is_ink = |p: [u8; 4]| -> bool { p[3] > 0 && p[0] > 40 && p[1] > 40 };
        let mut lit = 0usize;
        let mut total = 0usize;
        for py in y0.saturating_sub(1)..(y1 + 1).min(CANVAS_SIZE.1 as usize) {
            for px in x0..x1.min(CANVAS_SIZE.0 as usize) {
                total += 1;
                if is_ink(bmp.pixel(px, py)) {
                    lit += 1;
                }
            }
        }
        assert!(lit > 0, "subtitle glyph pixels present");
        assert!(lit * 5 < total, "subtitle is sparse glyphs, not a fill");
    }

    /// The outcome card the glyph tests paint: a real eyebrow/title/detail
    /// triple so every row has text to draw.
    fn outcome_state() -> RenderState {
        RenderState {
            outcome: true,
            outcome_content: Some(OutcomeState {
                eyebrow: "RESTORED".into(),
                title: "Writing".into(),
                detail: "Opened 1 artifact".into(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn outcome_rows_are_strictly_ordered_and_never_overlap() {
        // BUG 7: the old layout stacked the band, chip, title and detail at
        // hand-picked offsets that overlapped by 4.5pt, and its row heights
        // were shorter than the font's ascent box.
        let state = outcome_state();
        let layers = surface_layouts(&state);
        let card = layers.iter().find(|l| l.id == "card").expect("card");
        let find = |id: &str| -> Layer {
            *layers
                .iter()
                .find(|l| l.id == id)
                .unwrap_or_else(|| panic!("{id} layer"))
        };
        let eyebrow = find("eyebrow");
        let title = find("outcome_title");
        let detail = find("outcome_detail");
        let chip = find("chip_dismiss");

        // 1. Strict top-to-bottom order.
        assert!(
            eyebrow.y < title.y && title.y < detail.y && detail.y < chip.y,
            "eyebrow {} < title {} < detail {} < chip {}",
            eyebrow.y,
            title.y,
            detail.y,
            chip.y
        );

        // 2. No two content rects intersect.
        let content: Vec<&Layer> = layers.iter().filter(|l| l.id != "card").collect();
        for (i, a) in content.iter().enumerate() {
            for b in content.iter().skip(i + 1) {
                let disjoint =
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                assert!(disjoint, "{} overlaps {}", a.id, b.id);
            }
        }

        // 3. Every text row is tall enough for its ascent box, and the gaps
        //    are at least ROW_GAP.
        for (layer, size) in [
            (eyebrow, FONT_UI_EYEBROW),
            (title, FONT_UI_OUTCOME_TITLE),
            (detail, FONT_UI_OUTCOME_DETAIL),
        ] {
            assert!(
                layer.h >= text_row_h(size),
                "{} is {} tall, needs {}",
                layer.id,
                layer.h,
                text_row_h(size)
            );
        }
        assert!(title.y - (eyebrow.y + eyebrow.h) >= ROW_GAP);
        assert!(detail.y - (title.y + title.h) >= ROW_GAP);
        assert!(chip.y - (detail.y + detail.h) >= ROW_GAP);

        // 4. Insets: at least a card inset below the notch band, exactly one
        //    below the chip row, and no dead space left over.
        assert!(eyebrow.y - state.notch_h >= CARD_INSET);
        assert!(
            (card.h - (chip.y + chip.h) - CARD_INSET).abs() < 1e-9,
            "the card ends one inset below the chip row (no dead space)"
        );

        // 5. The full-width accent slab is gone for good.
        assert!(layers.iter().all(|l| l.id != "outcome_band"));
    }

    #[test]
    fn every_chip_is_a_pill_and_the_mic_mark_is_round() {
        // BUG 5 / BUG 6: square chips and a 10x10 solid mic square. The
        // mic mark is now the reference's ring + center dot; both must
        // carry equal half-side radii so `shape_contains` paints circles.
        for state in [
            RenderState {
                listening: true,
                ..Default::default()
            },
            carousel_state(),
            outcome_state(),
        ] {
            for layer in surface_layouts(&state) {
                if layer.id.starts_with("chip_") {
                    assert!(
                        (layer.top_radius - layer.h / 2.0).abs() < 1e-9
                            && (layer.bottom_radius - layer.h / 2.0).abs() < 1e-9,
                        "{} is a pill (r={}/{}, h={})",
                        layer.id,
                        layer.top_radius,
                        layer.bottom_radius,
                        layer.h
                    );
                    assert!(
                        (layer.h - CHIP_H).abs() < 1e-9,
                        "{} rides the 28pt capsule height",
                        layer.id
                    );
                }
                if layer.id == "mic_ring" || layer.id == "mic_dot" {
                    assert!(
                        (layer.top_radius - layer.w / 2.0).abs() < 1e-9
                            && (layer.bottom_radius - layer.h / 2.0).abs() < 1e-9,
                        "{} is a circle (r={}/{}, w={}, h={})",
                        layer.id,
                        layer.top_radius,
                        layer.bottom_radius,
                        layer.w,
                        layer.h
                    );
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn chips_are_tinted_glass_capsules_over_the_card() {
        // BUG 4.1 resurrected by the reference grammar: the old test
        // asserted a SOLID fill in the layer's own color. The redesign
        // makes every chip tinted glass — white@0.08 for dark chips and
        // caramel@0.20 for the pick — so the correct expectation is the
        // blended tint over the opaque card, never the raw layer color
        // and never the card fill itself. The ink intent is checked
        // separately through `objc::chip_ink`.
        use objc::paint_into;
        for state in [
            RenderState {
                listening: true,
                partial: Some("open my writing".into()),
                ..Default::default()
            },
            carousel_state(),
            outcome_state(),
        ] {
            let layers = surface_layouts(&state);
            let bmp = paint_into(&state);
            let chips: Vec<&Layer> = layers
                .iter()
                .filter(|l| l.id.starts_with("chip_"))
                .collect();
            assert!(!chips.is_empty(), "the phase has chips");
            for chip in chips {
                // Below the glyph band (the centered label sits at the
                // top of the 28pt pill), on the vertical center line: the
                // fill's tint and nothing else.
                let px = (chip.x + chip.w / 2.0) as usize;
                let py = (chip.y + chip.h - 3.0) as usize;
                let p = bmp.pixel(px, py);
                assert!(
                    p[3] > 200,
                    "{} reads opaque over the card at ({px},{py}), got {p:?}",
                    chip.id
                );
                // Source-over of the glass over the opaque black card.
                let expect = |c: f64| -> i32 { (chip.a * c * 255.0) as i32 };
                assert!(
                    (p[0] as i32 - expect(chip.r)).abs() <= 2
                        && (p[1] as i32 - expect(chip.g)).abs() <= 2
                        && (p[2] as i32 - expect(chip.b)).abs() <= 2,
                    "{} paints its own tint over the card, got {p:?} for a={} over ({}, {}, {})",
                    chip.id,
                    chip.a,
                    chip.r,
                    chip.g,
                    chip.b
                );
                // The glass is a tint of the card, never the card itself
                // and never a solid slab.
                assert!(
                    p[0] as u16 + p[1] as u16 + p[2] as u16 > 0 && p[0] < 250 && (chip.a < 1.0),
                    "{} is translucent glass (a={}), got {p:?}",
                    chip.id,
                    chip.a
                );
            }
        }
        // The ink intent: accent text on the caramel pick, white on the
        // dark pills — and never the deleted dark on-accent ink.
        assert_eq!(
            objc::chip_ink("chip_restore"),
            (CARAMEL.0, CARAMEL.1, CARAMEL.2)
        );
        for id in ["chip_rack", "chip_prev", "chip_next", "chip_dismiss"] {
            assert_eq!(
                objc::chip_ink(id),
                (CHIP_INK_WHITE, CHIP_INK_WHITE, CHIP_INK_WHITE)
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn outcome_text_rows_are_never_filled_as_slabs() {
        // BUG 4.2: `outcome_title` / `outcome_detail` were missing from the
        // text-layer list, so their INK colors were painted as full-width
        // rectangles with the glyphs drawn on top in the same color — the
        // grey slab (#A8AAA8) and cream slab (#ECEAE6) on screen.
        use objc::paint_into;
        let state = outcome_state();
        let layers = surface_layouts(&state);
        let bmp = paint_into(&state);
        let card_bg = bmp.pixel(
            (layers[0].x + layers[0].w / 2.0) as usize,
            (layers[0].y + layers[0].h - 4.0) as usize,
        );
        assert_eq!(card_bg, [0, 0, 0, 255], "the card is opaque black");

        for id in ["outcome_title", "outcome_detail"] {
            let row = layers.iter().find(|l| l.id == id).expect(id);
            // Well past the end of the label: inside the row rect, away from
            // every glyph stroke. A filled rect would read as ink here.
            let px = (row.x + row.w - 6.0) as usize;
            let py = (row.y + row.h / 2.0) as usize;
            let p = bmp.pixel(px, py);
            assert_eq!(
                p, card_bg,
                "{id} is a text bucket, not a filled slab: ({px},{py}) = {p:?}"
            );

            // The row does carry real ink somewhere — this is a
            // not-filled assertion, not a not-drawn one.
            let ink = (row.y as usize..(row.y + row.h) as usize)
                .flat_map(|y| (row.x as usize..(row.x + row.w) as usize).map(move |x| (x, y)))
                .filter(|(x, y)| bmp.pixel(*x, *y)[0] > 90)
                .count();
            let total = (row.w * row.h) as usize;
            assert!(ink > 0, "{id} draws glyphs");
            assert!(
                ink * 4 < total,
                "{id} is sparse glyphs, not a slab ({ink}/{total})"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_font_helper_resolves_a_real_system_face_for_both_weights() {
        // BUG 3: `CTFontCreateUIFontForLanguage(1, ..)` is
        // kCTFontUIFontUserFixedPitch — Courier. The helper must resolve a
        // non-null font for every role the surface draws, in both weights.
        for size in [
            FONT_UI_EYEBROW,
            FONT_UI_TRANSCRIPT,
            FONT_UI_CHOICE_TITLE,
            FONT_UI_CHOICE_SUBTITLE,
            FONT_UI_OUTCOME_TITLE,
            FONT_UI_OUTCOME_DETAIL,
            FONT_UI_CHIP,
        ] {
            for semibold in [false, true] {
                let font = objc::resolved_font_ptr(size, semibold);
                assert!(
                    !font.is_null(),
                    "CTFontRef for {size}pt semibold={semibold} is non-null"
                );
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_resolved_face_is_proportional_not_courier() {
        // The decisive forensic signature of the old bug was a uniform
        // ~6.5pt advance for every letter. A real system face is
        // proportional: eight `M`s must be far wider than eight `i`s.
        use objc::paint_into;
        let ink_width = |text: &str| -> f64 {
            let state = RenderState {
                listening: true,
                partial: Some(text.to_string()),
                ..Default::default()
            };
            let row = surface_layouts(&state)
                .into_iter()
                .find(|l| l.id == "transcript")
                .expect("transcript");
            let bmp = paint_into(&state);
            let mut max_x = row.x;
            for py in row.y as usize..(row.y + row.h) as usize {
                for px in row.x as usize..(row.x + row.w) as usize {
                    let p = bmp.pixel(px, py);
                    if p[0] as u16 + p[1] as u16 + p[2] as u16 > 180 {
                        max_x = max_x.max(px as f64);
                    }
                }
            }
            max_x - row.x
        };
        let narrow = ink_width("iiiiiiii");
        let wide = ink_width("MMMMMMMM");
        assert!(narrow > 0.0 && wide > 0.0, "both strings render");
        assert!(
            wide > narrow * 1.5,
            "proportional face: MMMMMMMM={wide} vs iiiiiiii={narrow} (a monospace \
             fallback would make these equal)"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rasterized_glyphs_are_right_side_up_in_bitmap_space() {
        // BUG 1's other half: the glyph rasterizer used to reflect its CTM,
        // which mirrored every glyph inside its row buffer. That only looked
        // correct because the AppKit blit reflected the whole canvas again.
        // `BitmapData` row 0 is the TOP, so a row of `T`s must carry more ink
        // in its upper half (the crossbars) than in its lower half (stems).
        use objc::paint_into;
        let state = RenderState {
            listening: true,
            partial: Some("TTTTTT".into()),
            ..Default::default()
        };
        let row = surface_layouts(&state)
            .into_iter()
            .find(|l| l.id == "transcript")
            .expect("transcript");
        let bmp = paint_into(&state);
        let y0 = row.y as usize;
        let y1 = (row.y + row.h) as usize;
        let mid = (y0 + y1) / 2;
        let ink_in = |from: usize, to: usize| -> usize {
            (from..to)
                .flat_map(|y| (row.x as usize..(row.x + row.w) as usize).map(move |x| (x, y)))
                .filter(|(x, y)| bmp.pixel(*x, *y)[0] > 120)
                .count()
        };
        let upper = ink_in(y0, mid);
        let lower = ink_in(mid, y1);
        assert!(
            upper > 0 && lower > 0,
            "the glyphs render ({upper}/{lower})"
        );
        assert!(
            upper > lower,
            "`T` crossbars sit in the TOP half of the row (upper={upper}, lower={lower}); \
             a mirrored raster would invert this"
        );
    }

    #[test]
    fn notch_mode_parses_env() {
        // SAFETY: single-threaded test binary for these assertions; the
        // variable is restored after the checks.
        let saved = std::env::var("EVO_NOTCH_MODE").ok();
        unsafe {
            std::env::set_var("EVO_NOTCH_MODE", "hidden");
            assert_eq!(NotchMode::from_env(), NotchMode::Hidden);
            std::env::set_var("EVO_NOTCH_MODE", "float");
            assert_eq!(NotchMode::from_env(), NotchMode::Float);
            std::env::set_var("EVO_NOTCH_MODE", "own");
            assert_eq!(NotchMode::from_env(), NotchMode::Own);
            std::env::remove_var("EVO_NOTCH_MODE");
        }
        assert_eq!(NotchMode::from_env(), NotchMode::Auto);
        if let Some(saved) = saved {
            unsafe {
                std::env::set_var("EVO_NOTCH_MODE", saved);
            }
        }
    }

    #[test]
    fn no_hardware_notch_never_selects_a_fallback_surface() {
        assert_eq!(NotchMode::Auto.decision(None, 0), None);
        assert_eq!(NotchMode::Own.decision(None, 0), None);
        assert_eq!(NotchMode::Float.decision(None, 0), None);
    }

    #[test]
    fn auto_owns_a_free_hardware_notch_and_only_yields_to_foreign_notch_ui() {
        let hardware_notch = Some(FALLBACK_NOTCH);
        assert_eq!(
            NotchMode::Auto.decision(hardware_notch, 0),
            Some(SurfaceDecision::Own)
        );
        assert_eq!(
            NotchMode::Auto.decision(hardware_notch, 1),
            Some(SurfaceDecision::Float)
        );
    }

    #[test]
    fn os_chrome_names_are_excluded_from_occupancy() {
        assert!(is_os_chrome("Window Server"));
        assert!(is_os_chrome("Dock"));
        assert!(!is_os_chrome("DynamicNotch"));
        assert!(!is_os_chrome("Safari"));
    }

    #[test]
    fn occupancy_distinguishes_foreign_notch_overlays_from_ordinary_windows() {
        // Browser/Evo document windows are layer 0 even when they touch the
        // screen top; they do not own the notch strip.
        assert!(!is_foreign_notch_window("Safari", 0, 1.0, 42, 7));
        assert!(!is_foreign_notch_window("Evo", 0, 1.0, 7, 7));
        // An elevated, visible, non-self overlay is a real foreign owner.
        assert!(is_foreign_notch_window("DynamicNotch", 25, 1.0, 42, 7));
        assert!(!is_foreign_notch_window("Window Server", 25, 1.0, 42, 7));
        assert!(!is_foreign_notch_window("DynamicNotch", 25, 0.0, 42, 7));
    }

    #[test]
    fn bitmap_and_appkit_transform_is_one_vertical_reflection() {
        // Use asymmetric coordinates so a second flip cannot accidentally
        // pass: the top and bottom edges must map to different AppKit rows.
        let height = 320.0;
        let top_left = (17.0, 23.0);
        let bottom_right = (91.0, 47.0);
        assert_eq!(bitmap_to_appkit_point(top_left, height), (17.0, 297.0));
        assert_eq!(bitmap_to_appkit_point(bottom_right, height), (91.0, 273.0));
        assert_eq!(appkit_to_bitmap_point((17.0, 297.0), height), top_left);
        assert_eq!(appkit_to_bitmap_point((91.0, 273.0), height), bottom_right);
    }

    #[test]
    fn dismissed_engine_state_has_no_layout_to_keep_the_panel_visible() {
        let mut engine = NotchEngine::new();
        engine.show_live(ContentId::EvoRack, crate::notch::priority::RACK, true);
        engine.voice_event(
            &evo_voice::VoiceEvent::WakeGesture { score: None },
            Instant::now(),
        );
        engine.dismiss(Instant::now());
        let state = RenderState::from_engine(&engine, None);
        assert!(surface_layouts(&state).is_empty());
        assert!(!state.is_visible());
    }

    #[test]
    fn legacy_idle_engine_state_orders_the_panel_out() {
        let mut engine = NotchEngine::new();
        engine.show_live(ContentId::EvoIdle, crate::notch::priority::IDLE, false);
        let state = RenderState::from_engine(&engine, None);

        assert!(surface_layouts(&state).is_empty());
        assert!(!state.is_visible());
    }

    #[test]
    fn render_state_maps_the_engine_model() {
        use crate::notch::{ContentId, NotchEngine};
        let mut engine = NotchEngine::new();
        engine.show_live(ContentId::EvoIdle, crate::notch::priority::IDLE, false);
        let state = RenderState::from_engine(&engine, None);
        assert!(!state.idle_glyph, "idle is intentionally hidden");
        assert!(!state.listening);

        engine.voice_event(
            &evo_voice::VoiceEvent::WakeGesture { score: Some(0.2) },
            Instant::now(),
        );
        let state = RenderState::from_engine(&engine, None);
        assert!(state.listening, "wake gesture expands the surface");
        assert!(!state.idle_glyph, "listening preempts the glyph");

        engine.voice_event(
            &evo_voice::VoiceEvent::Listening {
                partial: Some("open my".to_string()),
            },
            Instant::now(),
        );
        let state = RenderState::from_engine(&engine, None);
        assert_eq!(state.partial.as_deref(), Some("open my"));
    }

    #[test]
    fn eased_expand_is_monotonic_and_fast_out() {
        // Expansion: the reference app's open spring (response 0.42 /
        // damping 0.8) — eager launch, soft settle, and one un-clamped
        // physical overshoot that peaks at ~1.0152 around 35% of the
        // transition (t = 0.35 of 350ms ≈ +122ms).
        assert_eq!(
            eased_expand(0.0, SpringDirection::Expand),
            0.0,
            "starts collapsed"
        );
        assert_eq!(
            eased_expand(1.0, SpringDirection::Expand),
            1.0,
            "settles extended"
        );
        assert!(
            eased_expand(0.5, SpringDirection::Expand) > 0.5,
            "fast-out: more than halfway before halfway"
        );
        let mut prev = 0.0;
        let mut peak: f64 = 0.0;
        for step in 0..=1000 {
            let e = eased_expand(step as f64 / 1000.0, SpringDirection::Expand);
            // The reference spring is slightly under-damped; after its
            // single overshoot the return is gentle (less than a
            // thousandth per millistep). Do not turn that physical
            // spring into a visibly stepped animation just to enforce
            // strict monotonicity.
            assert!(e + 1e-3 >= prev, "near-monotonic at step {step}");
            prev = e;
            peak = peak.max(e);
        }
        // Un-clamped: the overshoot must actually ride through mid-flight
        // — the old [0, 1] clamp cut it off at exactly 1.0 and the open
        // read as a flat arrival — while staying inside the bound the
        // layout accepts (`EXPAND_OVERSHOOT_LIMIT`).
        assert!(
            peak > 1.0,
            "the open spring overshoots 1.0 mid-flight (peak {peak})"
        );
        assert!(
            peak <= EXPAND_OVERSHOOT_LIMIT,
            "the overshoot stays inside the layout's guarded bound (peak {peak})"
        );
        assert!(
            peak < 1.02,
            "the overshoot is subtle, not a bounce (peak {peak})"
        );
        // t >= 1 is still forced to EXACTLY 1.0, never the peak.
        assert_eq!(
            eased_expand(1.0, SpringDirection::Expand),
            1.0,
            "settles to exactly full extension"
        );
        assert_eq!(
            eased_expand(-1.0, SpringDirection::Expand),
            0.0,
            "clamped below"
        );
        assert_eq!(
            eased_expand(2.0, SpringDirection::Expand),
            1.0,
            "clamped above"
        );

        // Collapse: critically damped (response 0.45 / damping 1.0). It
        // approaches 1 from BELOW — a bounce past the notch band it is
        // retracting into would be visible — and, unlike the under-damped
        // expansion, it is strictly monotonic.
        assert_eq!(eased_expand(0.0, SpringDirection::Collapse), 0.0);
        assert_eq!(eased_expand(1.0, SpringDirection::Collapse), 1.0);
        let mut prev = 0.0;
        for step in 0..=100 {
            let e = eased_expand(step as f64 / 100.0, SpringDirection::Collapse);
            assert!(
                e >= prev,
                "critically damped collapse is strictly monotonic at step {step}"
            );
            assert!(
                e <= 1.0,
                "collapse approaches 1 without overshooting it (step {step}: {e})"
            );
            prev = e;
        }
        // The close spring settles rather than launches: at the midpoint
        // it trails the open spring, but it still arrives before the
        // nominal end of the transition.
        assert!(
            eased_expand(0.5, SpringDirection::Collapse)
                < eased_expand(0.5, SpringDirection::Expand),
            "collapse is the calmer curve at the midpoint"
        );
        assert!(
            eased_expand(0.9, SpringDirection::Collapse) > 0.9,
            "collapse still arrives before its nominal end"
        );
    }

    #[test]
    fn card_extension_measures_each_phase_s_growth() {
        assert_eq!(
            card_extension(&RenderState {
                idle_glyph: true,
                ..Default::default()
            }),
            0.0,
            "the idle band does not extend"
        );
        // Both cards are sized from the rows that actually exist (12pt
        // insets, 6pt between text rows, 10pt before the chip row), so the
        // expectation is the sum of those rows rather than a round constant.
        // The old 92.0 / 112.0 pair encoded the untidy layout that left
        // 44-70% of the card dead black.
        let listening_extension = card_extension(&RenderState {
            listening: true,
            ..Default::default()
        });
        let expected_listening = CARD_INSET
            + text_row_h(FONT_UI_EYEBROW)
            + ROW_GAP
            + text_row_h(FONT_UI_TRANSCRIPT)
            + CHIP_ROW_GAP
            + CHIP_H
            + CARD_INSET;
        assert!(
            (listening_extension - expected_listening).abs() < 1e-9,
            "empty listening is exactly its rows ({listening_extension} vs {expected_listening})"
        );
        assert!(
            card_extension(&carousel_state())
                > card_extension(&RenderState {
                    listening: true,
                    ..Default::default()
                }),
            "the carousel expands a little more than listening"
        );
        let outcome_extension = card_extension(&RenderState {
            outcome: true,
            ..Default::default()
        });
        let expected_outcome = CARD_INSET
            + text_row_h(FONT_UI_EYEBROW)
            + ROW_GAP
            + text_row_h(FONT_UI_OUTCOME_TITLE)
            + ROW_GAP
            + text_row_h(FONT_UI_OUTCOME_DETAIL)
            + CHIP_ROW_GAP
            + CHIP_H
            + CARD_INSET;
        assert!(
            (outcome_extension - expected_outcome).abs() < 1e-9,
            "the result card is exactly eyebrow + title + detail + chip row \
             ({outcome_extension} vs {expected_outcome})"
        );
    }

    #[test]
    fn expand_scales_the_card_and_keeps_every_layer_inside_it() {
        // Mid-flight frames: the card shrinks toward the band and the
        // rows compress proportionally — nothing escapes the card, so
        // hit-testing stays honest even at half-growth.
        for state in [
            RenderState {
                listening: true,
                ..Default::default()
            },
            carousel_state(),
            RenderState {
                outcome: true,
                ..Default::default()
            },
        ] {
            let full_h = surface_layouts(&state)
                .iter()
                .find(|l| l.id == "card")
                .expect("card")
                .h;
            // 1.0151 is the open spring's physical overshoot peak and
            // 1.03 is the layout's accepted bound: the card silhouette
            // grows by the very same factor as everything it contains,
            // so the "nothing escapes" invariant holds there too.
            for expand in [0.0, 0.25, 0.5, 0.9, 1.0151, EXPAND_OVERSHOOT_LIMIT] {
                let frame = RenderState {
                    expand,
                    ..state.clone()
                };
                let layers = surface_layouts(&frame);
                let card = layers.iter().find(|l| l.id == "card").expect("card");
                let expected_h = frame.notch_h + (full_h - frame.notch_h) * expand;
                assert!(
                    (card.h - expected_h).abs() < 1e-6,
                    "card height interpolates at expand={expand}"
                );
                for layer in &layers {
                    assert!(
                        layer.y >= -1e-6
                            && layer.x >= card.x - 1e-6
                            && layer.x + layer.w <= card.x + card.w + 1e-6
                            && layer.y + layer.h <= card.h + 1e-6,
                        "{} stays inside the card at expand={expand}",
                        layer.id
                    );
                }
            }
            // Full extension is byte-for-byte the unanimated layout.
            let full = RenderState {
                expand: 1.0,
                ..state.clone()
            };
            assert_eq!(surface_layouts(&full), surface_layouts(&state));
        }
    }

    #[test]
    fn expansion_starts_at_the_physical_notch_and_grows_outward() {
        let state = RenderState {
            listening: true,
            notch_w: 170.0,
            notch_h: 32.0,
            expand: 0.0,
            ..Default::default()
        };
        let collapsed = surface_layouts(&state);
        let collapsed_card = collapsed
            .iter()
            .find(|layer| layer.id == "card")
            .expect("collapsed card");
        let center = CANVAS_SIZE.0 / 2.0;
        assert_eq!(collapsed_card.w, state.notch_w);
        assert_eq!(collapsed_card.h, state.notch_h);
        assert!((collapsed_card.x + collapsed_card.w / 2.0 - center).abs() < f64::EPSILON);

        let expanded = surface_layouts(&RenderState {
            expand: 1.0,
            ..state
        });
        let expanded_card = expanded
            .iter()
            .find(|layer| layer.id == "card")
            .expect("expanded card");
        assert!(expanded_card.w > collapsed_card.w);
        assert!(expanded_card.h > collapsed_card.h);
        assert!((expanded_card.x + expanded_card.w / 2.0 - center).abs() < f64::EPSILON);
    }

    #[test]
    fn collapse_keeps_the_outgoing_card_until_it_orders_out() {
        let from = RenderState {
            listening: true,
            ..Default::default()
        };
        let target = RenderState::default();

        let mid = transition_frame(&from, &target, 0.1);
        assert!(card_extension(&mid) > 0.0);
        assert!(
            mid.is_visible(),
            "collapse remains painted during its shrink"
        );
        assert!(!transition_frame(&from, &target, 1.0).is_visible());
    }

    #[test]
    fn result_and_choice_collapse_without_detaching_or_leaking_pixels() {
        for from in [
            RenderState {
                outcome: true,
                ..Default::default()
            },
            carousel_state(),
        ] {
            let target = RenderState::default();
            let mid = transition_frame(&from, &target, 0.1);
            let card = surface_layouts(&mid)
                .into_iter()
                .find(|layer| layer.id == "card")
                .expect("outgoing card remains during collapse");
            assert!(mid.is_visible());
            assert!(card.h > mid.notch_h && card.h < surface_layouts(&from)[0].h);
            assert_eq!(transition_frame(&from, &target, 1.0), target);
        }
    }

    #[test]
    fn expand_transition_carries_the_open_springs_overshoot() {
        // The un-clamped open spring enters the frame itself: at the
        // overshoot peak (~35% of the transition) the card is slightly
        // PAST its target extension — the shape washes out and settles
        // back instead of arriving flat at exactly 1.0.
        let idle = RenderState::default();
        let target = RenderState {
            listening: true,
            ..Default::default()
        };
        let full = card_extension(&target);
        assert!(full > 0.0);
        let frame = transition_frame(&idle, &target, 0.35);
        assert!(
            frame.expand > 1.0,
            "mid-flight expand overshoots 1.0 (got {})",
            frame.expand
        );
        assert!(
            frame.expand <= EXPAND_OVERSHOOT_LIMIT,
            "the frame stays inside the layout's bound (got {})",
            frame.expand
        );
        assert!(
            card_extension(&frame) > full,
            "the painted card is past its target size at the peak \
             ({} vs {full})",
            card_extension(&frame)
        );
        // ... and the transition still lands at exactly full extension.
        let settled = transition_frame(&idle, &target, 1.0);
        assert_eq!(settled, target);
        assert_eq!(settled.expand, 1.0);
    }

    #[test]
    fn content_alpha_fades_in_late_and_out_early() {
        let idle = RenderState::default();
        let listening = RenderState {
            listening: true,
            ..Default::default()
        };
        // Static frames are never touched: a plain state paints content
        // at full strength, and the settled frame is the target itself.
        assert_eq!(idle.content_alpha, 1.0);
        assert_eq!(listening.content_alpha, 1.0);
        assert_eq!(transition_frame(&idle, &listening, 1.0).content_alpha, 1.0);

        // Growing: content holds back, then smoothsteps in across
        // [0.26, 0.89] of the transition (~90ms -> ~310ms of 350ms).
        let grow = |t: f64| transition_frame(&idle, &listening, t).content_alpha;
        assert_eq!(grow(0.0), 0.0, "content is not there at frame zero");
        assert_eq!(grow(0.26), 0.0, "still hidden at the window's start");
        let mid = grow(0.575); // (0.575 - 0.26) / 0.63 = 0.5
        assert!(
            (mid - 0.5).abs() < 1e-9,
            "smoothstep midpoint is 0.5 (got {mid})"
        );
        assert_eq!(grow(0.89), 1.0, "fully arrived by the window's end");
        assert_eq!(grow(0.95), 1.0, "stays at full opacity past the window");
        let mut prev = 0.0;
        for step in 0..=99 {
            let a = grow(step as f64 / 100.0);
            assert!(
                (0.0..=1.0).contains(&a),
                "alpha stays in bounds at step {step}: {a}"
            );
            assert!(
                a + 1e-9 >= prev,
                "growing content fades in monotonically (step {step})"
            );
            prev = a;
        }

        // Shrinking: content leaves in the first 34% (~120ms), so the
        // card contracts alone after the content has left.
        let shrink = |t: f64| transition_frame(&listening, &idle, t).content_alpha;
        assert_eq!(
            shrink(0.0),
            1.0,
            "content is fully there when the close starts"
        );
        assert!(
            (shrink(0.17) - 0.5).abs() < 1e-9,
            "linear midpoint is 0.5 (got {})",
            shrink(0.17)
        );
        assert_eq!(shrink(0.34), 0.0, "gone by the window's end");
        assert_eq!(shrink(0.5), 0.0, "stays gone afterwards");
        let mut prev = 1.0;
        for step in 0..=99 {
            let a = shrink(step as f64 / 100.0);
            assert!(
                (0.0..=1.0).contains(&a),
                "alpha stays in bounds at step {step}: {a}"
            );
            assert!(
                a <= prev + 1e-9,
                "shrinking content fades out monotonically (step {step})"
            );
            prev = a;
        }
    }

    /// The content choreography is a PAINT behavior: a mid-morph frame
    /// keeps the card silhouette opaque while every content layer —
    /// chips included — is dimmed by `RenderState::content_alpha`.
    #[cfg(target_os = "macos")]
    #[test]
    fn morph_paint_dimms_content_while_the_card_stays_opaque() {
        use objc::paint_into;
        // t = 0.575 of a grow is the smoothstep midpoint (alpha 0.5), and
        // the open spring is at ~1.001 of full extension there, so the
        // frame still carries the whole chip row.
        let full = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            ..Default::default()
        };
        let frame = transition_frame(&RenderState::default(), &full, 0.575);
        assert!((frame.content_alpha - 0.5).abs() < 1e-9);

        let full_paint = paint_into(&full);
        let dim_paint = paint_into(&frame);
        let layers = surface_layouts(&frame);
        let card = layers
            .iter()
            .find(|l| l.id == "card")
            .expect("card")
            .clone();
        let chip = layers
            .iter()
            .find(|l| l.id == "chip_dismiss")
            .expect("chip_dismiss")
            .clone();

        // The silhouette is untouched by the choreography: the very same
        // opaque black interior in both paints.
        let (ix, iy) = (
            (card.x + card.w / 2.0) as usize,
            (card.y + card.h - 4.0) as usize,
        );
        assert_eq!(full_paint.pixel(ix, iy), [0, 0, 0, 255]);
        assert_eq!(
            dim_paint.pixel(ix, iy),
            [0, 0, 0, 255],
            "the card NEVER dims mid-morph"
        );

        // A chip is content: white glass over the card at half alpha
        // reads at half the tint. Sampled below the glyph band, on the
        // center line — the same rule the chip-glass test uses.
        let (px, py) = (
            (chip.x + chip.w / 2.0) as usize,
            (chip.y + chip.h - 3.0) as usize,
        );
        let full_px = full_paint.pixel(px, py);
        let dim_px = dim_paint.pixel(px, py);
        let expect = |a: f64| (a * 255.0) as i32;
        assert!(
            (full_px[0] as i32 - expect(chip.a)).abs() <= 2,
            "full-strength chip tint over the card, got {full_px:?}"
        );
        assert!(
            (dim_px[0] as i32 - expect(chip.a * frame.content_alpha)).abs() <= 2,
            "the chip dims to half its tint mid-morph, got {dim_px:?}"
        );
        assert!(
            dim_px[0] < full_px[0],
            "the mid-morph chip is strictly darker ({dim_px:?} vs {full_px:?})"
        );
    }

    /// The device-scale paint path must rasterize glyphs AT the scale
    /// factor (CoreText size × scale, buffer dims × scale), not upscale a
    /// 1x image: a native 2x raster has solid stroke interiors, a 1x
    /// raster of the same line is mostly antialiased edge.
    ///
    /// Measured as the solid-ink ratio inside the transcript row:
    ///
    ///     pixels at >= 200/255 ink coverage
    ///     ---------------------------------
    ///       pixels with any (> 17/255) ink
    ///
    /// The transcript sits on the opaque black card, so the premultiplied
    /// ink byte encodes coverage (`R = 237 * coverage^2` through the
    /// source-over composite); the ink-coverage thresholds 200/255 and
    /// 17/255 therefore map to red 186 and 16. A full-ink pixel is R≈237.
    #[cfg(target_os = "macos")]
    #[test]
    fn paint_into_at_device_scale_has_crisp_glyphs() {
        use objc::paint_into_scaled;

        const SOLID_INK_R: u8 = 186; // ≈ coverage >= 200/255
        const ANY_INK_R: u8 = 16; // ≈ coverage > 17/255

        let state = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            ..Default::default()
        };
        let transcript = surface_layouts(&state)
            .into_iter()
            .find(|layer| layer.id == "transcript")
            .expect("transcript layer");

        let ink_counts = |bitmap: &BitmapData, scale: f64| -> (usize, usize) {
            let x0 = (transcript.x * scale) as usize;
            let y0 = (transcript.y * scale) as usize;
            let x1 = ((transcript.x + transcript.w) * scale) as usize;
            let y1 = ((transcript.y + transcript.h) * scale) as usize;
            let mut solid = 0usize;
            let mut ink = 0usize;
            for py in y0..y1.min(bitmap.height) {
                for px in x0..x1.min(bitmap.width) {
                    let r = bitmap.pixel(px, py)[0];
                    if r >= SOLID_INK_R {
                        solid += 1;
                    }
                    if r > ANY_INK_R {
                        ink += 1;
                    }
                }
            }
            (solid, ink)
        };

        let one = paint_into_scaled(&state, 1.0);
        let two = paint_into_scaled(&state, 2.0);
        // The device-scale bitmap is the full canvas at scale: output
        // pixels == canvas points * scale in both dimensions.
        assert_eq!(
            (one.width, one.height),
            (CANVAS_SIZE.0 as usize, CANVAS_SIZE.1 as usize)
        );
        assert_eq!((two.width, two.height), (one.width * 2, one.height * 2));

        let (solid_1x, ink_1x) = ink_counts(&one, 1.0);
        let (solid_2x, ink_2x) = ink_counts(&two, 2.0);
        assert!(ink_1x > 0 && ink_2x > 0, "both rasters draw real ink");
        let ratio_1x = solid_1x as f64 / ink_1x as f64;
        let ratio_2x = solid_2x as f64 / ink_2x as f64;
        eprintln!(
            "DEVICE SCALE: solid-ink ratio 1x={ratio_1x:.3} ({solid_1x}/{ink_1x}), \
             2x={ratio_2x:.3} ({solid_2x}/{ink_2x})"
        );
        assert!(
            ratio_2x >= 0.5,
            "native 2x glyphs are majority solid ink (ratio {ratio_2x})"
        );
        assert!(
            ratio_2x > ratio_1x,
            "native 2x is crisper than the 1x raster (2x={ratio_2x}, 1x={ratio_1x})"
        );
    }

    /// TEMPORARY measurement harness (removed before the pipeline rewrite
    /// lands): p50 cost of one 1x LISTENING `paint_into` in the current
    /// implementation. Run with:
    ///   cargo test --release -p evo-desktop --features evo-notch \
    ///     measure_paint_into_p50_baseline -- --nocapture
    #[cfg(target_os = "macos")]
    #[test]
    fn measure_paint_into_p50_baseline() {
        use objc::paint_into;
        let state = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            ..Default::default()
        };
        for _ in 0..20 {
            let _ = paint_into(&state);
        }
        let mut samples: Vec<u128> = Vec::with_capacity(300);
        for _ in 0..300 {
            let t = Instant::now();
            let _ = paint_into(&state);
            samples.push(t.elapsed().as_micros());
        }
        samples.sort();
        eprintln!(
            "MEASURE paint_into 1x LISTENING: p50={}us p95={}us p99={}us",
            samples[150], samples[285], samples[297]
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn occupancy_probe_returns_a_usable_count() {
        // The honest contract: a number, no panic, no side effects. On a
        // dev MacBook with no other notch app this is 0; the probe just
        // has to not lie in either direction.
        let count = occupancy_probe();
        assert!(count < 100, "unrealistic overlap count {count}");
    }

    /// The shadow smoke test: a full expanded-card render with the shadow
    /// enabled must not panic, must leave the card's interior byte-for-byte
    /// the opaque card color (the shadow is composited BEHIND the fill,
    /// never over it), must show the falloff just outside the silhouette,
    /// and must die out within its compact support.
    #[cfg(target_os = "macos")]
    #[test]
    fn card_shadow_sits_behind_the_card_and_outside_its_fill() {
        use objc::paint_into;
        let state = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            ..Default::default()
        };
        let card = surface_layouts(&state)
            .into_iter()
            .find(|layer| layer.id == "card")
            .expect("card");
        let bmp = paint_into(&state);
        let center_x = (card.x + card.w / 2.0) as usize;

        // Interior — exactly the card color at the top band, in the bottom
        // inset below the chip row, and close to the left edge, where a
        // mis-ordered shadow would tint first.
        for (px, py) in [
            (center_x, 4usize),
            (center_x, (card.h - 4.0) as usize),
            ((card.x + 2.0) as usize, (card.y + card.h / 2.0) as usize),
        ] {
            assert_eq!(
                bmp.pixel(px, py),
                [0, 0, 0, 255],
                "card interior at ({px},{py}) stays pure card black"
            );
        }

        // Just below the card: the offset shadow is visible, dark and soft.
        let below = bmp.pixel(center_x, (card.h + 6.0) as usize);
        assert!(below[3] > 0, "shadow visible below the card, got {below:?}");
        assert!(below[0] < 40, "shadow is dark, got {below:?}");
        assert!(
            below[3] < 200,
            "shadow is soft, never opaque, got {below:?}"
        );

        // Beside the card at mid-height: the blur spills sideways too.
        let beside = bmp.pixel((card.x - 6.0) as usize, (card.y + card.h / 2.0) as usize);
        assert!(
            beside[3] > 0,
            "shadow visible beside the card, got {beside:?}"
        );

        // Past the compact support (blur + offset): fully transparent.
        let far = bmp.pixel(
            center_x,
            (card.h + objc::CARD_SHADOW_BLUR + objc::CARD_SHADOW_OFFSET_Y + 4.0) as usize,
        );
        assert_eq!(far[3], 0, "shadow has compact support, got {far:?}");

        // Collapsed (expand <= 0.05): the gate is off, the spot that
        // carried the shadow is transparent again.
        let collapsed = RenderState {
            expand: 0.0,
            ..state.clone()
        };
        let collapsed_card = surface_layouts(&collapsed)
            .into_iter()
            .find(|layer| layer.id == "card")
            .expect("collapsed card");
        let collapsed_bmp = paint_into(&collapsed);
        let quiet = collapsed_bmp.pixel(
            (collapsed_card.x + collapsed_card.w / 2.0) as usize,
            (collapsed_card.h + 6.0) as usize,
        );
        assert_eq!(quiet[3], 0, "no shadow while collapsed, got {quiet:?}");
        eprintln!(
            "SHADOW: below={below:?} beside={beside:?} far={far:?} interior=0,0,0,255 \
             collapsed={quiet:?}"
        );
    }

    /// The expanded card's 1pt white@12% hairline rides just inside the
    /// silhouette — the reference's `strokeBorder` — and is skipped at
    /// the collapsed notch, whose top edge is the physical bezel.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_expanded_card_carries_a_one_point_hairline_but_the_notch_does_not() {
        use objc::paint_into;
        let state = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            ..Default::default()
        };
        let card = surface_layouts(&state)
            .into_iter()
            .find(|layer| layer.id == "card")
            .expect("card");
        let bmp = paint_into(&state);
        // Mid-height on the straight left edge: the pixel whose center
        // sits ~0.5pt inside the silhouette carries the hairline; 2.5pt
        // inside is bare card black (a 1pt stroke, not a wash).
        let mid_y = (card.y + card.h / 2.0) as usize;
        let edge = bmp.pixel((card.x + 0.5).floor() as usize, mid_y);
        let inner = bmp.pixel((card.x + 2.5).floor() as usize, mid_y);
        assert!(
            edge[3] > 200 && edge[0] > inner[0] + 15,
            "the card's left edge carries the white@12% hairline (edge {edge:?} vs {inner:?})"
        );
        assert_eq!(inner, [0, 0, 0, 255], "the hairline is exactly 1pt wide");
        // The bottom edge carries it too (rounded corners included).
        let mid_x = (card.x + card.w / 2.0) as usize;
        let bottom = bmp.pixel(mid_x, (card.y + card.h - 0.5).floor() as usize);
        assert!(
            bottom[3] > 200 && bottom[0] > 15,
            "the card's bottom edge carries the hairline, got {bottom:?}"
        );
        eprintln!("HAIRLINE: edge={edge:?} inner={inner:?} bottom={bottom:?}");

        // Collapsed (expand 0): the silhouette IS the physical notch and
        // the gate is off — every painted pixel is pure card black.
        let collapsed = RenderState {
            expand: 0.0,
            ..state.clone()
        };
        let collapsed_card = surface_layouts(&collapsed)
            .into_iter()
            .find(|layer| layer.id == "card")
            .expect("collapsed card");
        let collapsed_bmp = paint_into(&collapsed);
        let quiet = collapsed_bmp.pixel(
            (collapsed_card.x + 0.5).floor() as usize,
            (collapsed_card.y + collapsed_card.h / 2.0) as usize,
        );
        assert_eq!(quiet, [0, 0, 0, 255], "no hairline on the collapsed notch");
        eprintln!("HAIRLINE: collapsed edge={quiet:?} (bezel, no stroke)");
    }

    /// Tabular digits and eyebrow tracking must be measurably applied, not
    /// merely requested. Measured through the same attribute pipeline the
    /// paint path builds.
    #[cfg(target_os = "macos")]
    #[test]
    fn subtitle_digits_are_tabular_and_the_eyebrow_is_tracked() {
        use objc::paint_into;
        let one = objc::measured_advance("1", FONT_UI_CHOICE_SUBTITLE, true, 0.0, true);
        let eight = objc::measured_advance("8", FONT_UI_CHOICE_SUBTITLE, true, 0.0, true);
        assert!(one.is_finite() && one > 0.0, "the subtitle font measures");
        assert!(
            (one - eight).abs() < 1e-6,
            "tabular digits: '1'={one} '8'={eight} must advance equally"
        );
        // The feature is what does it: the default (proportional) system
        // face gives '1' a visibly narrower advance than '8'.
        let default_one = objc::measured_advance("1", FONT_UI_CHOICE_SUBTITLE, true, 0.0, false);
        let default_eight = objc::measured_advance("8", FONT_UI_CHOICE_SUBTITLE, true, 0.0, false);
        assert!(
            default_eight > default_one + 0.5,
            "the default face's digits are proportional ('1'={default_one}, '8'={default_eight})"
        );

        // End to end: the rendered subtitle's ink must not drift between
        // focused candidate 0 ("work 1 of 3") and 2 ("work 3 of 3") — with
        // proportional digits the wider '3' would push the extent ~2pt.
        let ink_extent = |index: usize| -> f64 {
            let mut state = carousel_state();
            if let Some(choice) = state.choice.as_mut() {
                choice.index = index;
            }
            let row = surface_layouts(&state)
                .into_iter()
                .find(|layer| layer.id == "choice_subtitle")
                .expect("subtitle row");
            let bmp = paint_into(&state);
            let mut max_x = row.x;
            for py in row.y as usize..(row.y + row.h) as usize {
                for px in row.x as usize..(row.x + row.w) as usize {
                    if bmp.pixel(px, py)[0] > 40 {
                        max_x = max_x.max(px as f64 + 0.5);
                    }
                }
            }
            max_x
        };
        let first = ink_extent(0);
        let third = ink_extent(2);
        assert!(first > 0.0, "the subtitle renders");
        assert!(
            (first - third).abs() <= 1.0,
            "tabular digits keep the subtitle width stable (work 1 of 3 ends at {first}, work 3 of 3 at {third})"
        );

        // The eyebrow's kern: CoreText applies kern after every glyph
        // (trailing kern included), so 9 glyphs x 0.6pt = 5.4pt.
        let plain = objc::measured_advance("LISTENING", FONT_UI_EYEBROW, true, 0.0, false);
        let tracked = objc::measured_advance(
            "LISTENING",
            FONT_UI_EYEBROW,
            true,
            FONT_UI_EYEBROW_TRACKING,
            false,
        );
        assert!(
            ((tracked - plain) - 5.4).abs() < 0.25,
            "eyebrow kerning adds ~0.6pt per glyph (delta {})",
            tracked - plain
        );
        eprintln!(
            "TABULAR/KERN: '1'={one:.4} '8'={eight:.4} (tabular); default '1'={default_one:.4} \
             '8'={default_eight:.4}; subtitle extent work1of3={first} work3of3={third}; \
             kern delta={:.4}pt over 9 glyphs",
            tracked - plain
        );
    }

    /// The dark chips carry a one-point white-12% rim inside the pill
    /// outline; the caramel Restore pill stays fill-only. Sampled at the
    /// pixel whose center sits ~0.5pt inside the left edge at mid-height
    /// (the rim band) against a pixel 4pt inside (the fill). Both sit
    /// below the glyph band, so the samples read the glass, not ink.
    #[cfg(target_os = "macos")]
    #[test]
    fn dark_chips_carry_a_hairline_rim_and_the_caramel_chip_stays_fill_only() {
        use objc::paint_into;
        let state = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            ..Default::default()
        };
        let layers = surface_layouts(&state);
        let bmp = paint_into(&state);
        for id in ["chip_rack", "chip_dismiss"] {
            let chip = layers.iter().find(|l| l.id == id).expect(id);
            let y = (chip.y + chip.h / 2.0) as usize;
            let rim = bmp.pixel((chip.x + 0.5).floor() as usize, y);
            let fill = bmp.pixel((chip.x + 4.0) as usize, y);
            assert!(rim[3] > 200 && fill[3] > 200, "{id} is opaque");
            assert!(
                rim[0] >= fill[0] + 15 && rim[1] >= fill[1] + 15 && rim[2] >= fill[2] + 15,
                "{id} rim is a 12% white lift over its fill (rim {rim:?} vs fill {fill:?})"
            );
            let inside = bmp.pixel((chip.x + 2.0) as usize, y);
            assert_eq!(inside, fill, "{id} rim is a hairline, not a wash");
            eprintln!("CHIP RIM: {id} rim={rim:?} fill={fill:?} 2pt-in={inside:?}");
        }
        let restore = layers
            .iter()
            .find(|l| l.id == "chip_restore")
            .expect("restore");
        let y = (restore.y + restore.h / 2.0) as usize;
        let edge = bmp.pixel((restore.x + 0.5).floor() as usize, y);
        let fill = bmp.pixel((restore.x + 4.0) as usize, y);
        assert_eq!(edge, fill, "caramel Restore stays fill-only (no rim)");
        eprintln!("CHIP RIM: caramel edge={edge:?} fill={fill:?} (fill-only)");
    }

    #[test]
    fn ease_in_out_is_the_reference_timing_function() {
        // CAMediaTimingFunction .easeInEaseOut — the curve every bar hop
        // animates with: pinned endpoints, symmetric about the midpoint,
        // slow at both ends.
        assert_eq!(ease_in_out(0.0), 0.0);
        assert_eq!(ease_in_out(1.0), 1.0);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6, "midpoint is exact");
        assert!(
            (ease_in_out(0.25) - (1.0 - ease_in_out(0.75))).abs() < 1e-6,
            "symmetric about the midpoint"
        );
        assert!(
            ease_in_out(0.25) < 0.25 && ease_in_out(0.75) > 0.75,
            "ease-in at the start, ease-out at the end"
        );
    }

    #[test]
    fn wave_bar_scale_is_deterministic_bounded_and_continuous() {
        // Same inputs, same output: the reference's RNG is an integer
        // hash here, so every (bar, tick) draw is reproducible.
        assert_eq!(wave_bar_scale(2, 1.234), wave_bar_scale(2, 1.234));
        assert_ne!(
            wave_bar_scale(0, 1.234),
            wave_bar_scale(5, 1.234),
            "each bar carries its own draw"
        );
        // The full domain at 1ms over six seconds: every value is inside
        // the reference's [0.32, 1.0] and no sample-to-sample step
        // exceeds what a 0.12s retarget can produce (the boundary snap of
        // an interrupted 0.2s ease stays well under 0.25).
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for bar in 0..WAVE_BAR_COUNT {
            let mut previous = wave_bar_scale(bar, 0.0);
            for step in 0..=6000 {
                let t = step as f64 / 1000.0;
                let value = wave_bar_scale(bar, t);
                assert!(value.is_finite(), "bar {bar} at {t}s is finite");
                assert!(
                    (WAVE_MIN_SCALE - 1e-9..=1.0 + 1e-9).contains(&value),
                    "bar {bar} at {t}s = {value} leaves [0.32, 1.0]"
                );
                min = min.min(value);
                max = max.max(value);
                assert!(
                    (value - previous).abs() <= 0.25,
                    "bar {bar} jumps {previous} -> {value} at t={t}s"
                );
                previous = value;
            }
        }
        // The floor and the ceiling are actually visited: the ramp is
        // the reference's full range, not a damped copy.
        assert!(min < 0.4, "bars dip to the floor (min {min})");
        assert!(max > 0.9, "bars rise to the ceiling (max {max})");

        // The interpolation contract at a tick boundary: t = 0.48 is
        // exactly tick 4 (0.48 / 0.12 is exact in f64), where progress
        // 0 holds the previous tick's target.
        assert_eq!(
            wave_bar_scale(1, 0.48),
            wave_bar_target(3, 1),
            "a fresh tick starts from the previous tick's target"
        );
        // Mid-hop the value sits strictly between the two targets: the
        // hop is a real ease, not a jump.
        let from = wave_bar_target(3, 1);
        let to = wave_bar_target(4, 1);
        assert!(from != to, "the hash gives distinct neighbours");
        let mid = wave_bar_scale(1, 0.48 + 0.1);
        assert!(
            mid > from.min(to) + 1e-9 && mid < from.max(to) - 1e-9,
            "mid-hop value {mid} eases between {from} and {to}"
        );

        // The no-clock anchors: t = 0 and every invalid clock hold the
        // floor — the same state the layout renders with
        // `listening_t == None`, so the seam into a live phase never
        // pops and no NaN can reach a layer.
        for t in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(wave_bar_scale(0, t), WAVE_MIN_SCALE, "t={t}");
            assert_eq!(wave_bar_scale(5, t), WAVE_MIN_SCALE, "t={t}");
        }
    }

    #[test]
    fn listening_equalizer_is_six_capsules_with_the_teal_alpha_ramp() {
        let state = RenderState {
            listening: true,
            listening_t: Some(0.5),
            ..Default::default()
        };
        let layers = surface_layouts(&state);
        let card = layers.iter().find(|l| l.id == "card").expect("card");
        let ring = layers
            .iter()
            .find(|l| l.id == "mic_ring")
            .expect("mic ring");
        assert_eq!((ring.w, ring.h), (12.0, 12.0), "the ring is 12pt");
        assert_eq!((ring.top_radius, ring.bottom_radius), (6.0, 6.0));
        assert_eq!((ring.r, ring.g, ring.b), (CARAMEL.0, CARAMEL.1, CARAMEL.2));
        assert!(
            (ring.a - CARAMEL.3 * 0.18).abs() < 1e-9,
            "the ring's track fill is caramel@0.18 (got {})",
            ring.a
        );
        let dot = layers.iter().find(|l| l.id == "mic_dot").expect("mic dot");
        assert_eq!((dot.w, dot.h), (3.0, 3.0), "the center dot is 3pt");
        assert!((dot.a - CARAMEL.3).abs() < 1e-9, "the dot is solid caramel");
        assert!(
            (dot.x + dot.w / 2.0 - (ring.x + ring.w / 2.0)).abs() < 1e-9
                && (dot.y + dot.h / 2.0 - (ring.y + ring.h / 2.0)).abs() < 1e-9,
            "the dot is centered in the ring"
        );
        let eyebrow = layers.iter().find(|l| l.id == "eyebrow").expect("eyebrow");
        assert!(
            (ring.y + ring.h / 2.0 - (eyebrow.y + eyebrow.h / 2.0)).abs() < 1e-9,
            "the ring is vertically centered in the header row"
        );

        let bars: Vec<&Layer> = layers.iter().filter(|l| l.id == "wave_bar").collect();
        assert_eq!(bars.len(), WAVE_BAR_COUNT, "six bars");
        assert!(
            (bars[0].x - (ring.x + ring.w + 4.0)).abs() < 1e-9,
            "the equalizer starts 4pt after the ring"
        );
        for (index, bar) in bars.iter().enumerate() {
            assert!(
                (bar.w - WAVE_BAR_W).abs() < 1e-9,
                "bar {index} is {}pt wide",
                bar.w
            );
            if index > 0 {
                assert!(
                    (bar.x - (bars[index - 1].x + WAVE_BAR_W + WAVE_BAR_GAP)).abs() < 1e-9,
                    "bar {index} follows its neighbour on the 2pt gap"
                );
            }
            assert!(
                bar.h >= WAVE_BAR_MAX_H * WAVE_MIN_SCALE - 1e-9 && bar.h <= WAVE_BAR_MAX_H + 1e-9,
                "bar {index} height {} stays inside [5.12, 16]pt",
                bar.h
            );
            assert!(
                (bar.y + bar.h / 2.0 - (ring.y + ring.h / 2.0)).abs() < 1e-9,
                "bar {index} breathes about the header center line"
            );
            assert!(
                (bar.top_radius - WAVE_BAR_RADIUS).abs() < 1e-9
                    && (bar.bottom_radius - WAVE_BAR_RADIUS).abs() < 1e-9,
                "bar {index} has capsule ends"
            );
            assert_eq!(
                (bar.r, bar.g, bar.b),
                (TEAL_VOICE.0, TEAL_VOICE.1, TEAL_VOICE.2),
                "bar {index} is teal"
            );
            let expected_alpha = WAVE_ALPHA_FIRST
                + (1.0 - WAVE_ALPHA_FIRST) * index as f64 / (WAVE_BAR_COUNT - 1) as f64;
            assert!(
                (bar.a - expected_alpha).abs() < 1e-9,
                "bar {index} alpha ramps 0.55 -> 1.0 (got {})",
                bar.a
            );
            assert!(
                bar.x >= card.x
                    && bar.x + bar.w <= card.x + card.w
                    && bar.y >= card.y
                    && bar.y + bar.h <= card.y + card.h,
                "bar {index} stays inside the card"
            );
        }
        // The clock drives real motion: at this instant the bars hold
        // visibly distinct heights.
        let tallest = bars.iter().map(|b| b.h).fold(0.0_f64, f64::max);
        let shortest = bars.iter().map(|b| b.h).fold(f64::INFINITY, f64::min);
        assert!(
            tallest - shortest > 0.5,
            "the bars are in motion (height span {})",
            tallest - shortest
        );

        // No clock: the deterministic rest state tests paint — every bar
        // at the minimum scale. Documented behaviour for `None`.
        let quiet = RenderState {
            listening: true,
            ..Default::default()
        };
        for bar in surface_layouts(&quiet)
            .iter()
            .filter(|l| l.id == "wave_bar")
        {
            assert!(
                (bar.h - WAVE_BAR_MAX_H * WAVE_MIN_SCALE).abs() < 1e-9,
                "listening_t = None holds the minimum scale (got {})",
                bar.h
            );
        }
        // A non-finite clock is pinned to the same honest rest state.
        let broken = RenderState {
            listening: true,
            listening_t: Some(f64::NAN),
            ..Default::default()
        };
        assert_eq!(surface_layouts(&broken), surface_layouts(&quiet));
    }

    #[test]
    fn the_wave_clock_moves_only_the_bars() {
        let steady = RenderState {
            listening: true,
            ..Default::default()
        };
        let live = RenderState {
            listening: true,
            listening_t: Some(1.7),
            ..Default::default()
        };
        let steady_layers = surface_layouts(&steady);
        let live_layers = surface_layouts(&live);
        assert_eq!(steady_layers.len(), live_layers.len());
        let mut moved = 0usize;
        for (a, b) in steady_layers.iter().zip(live_layers.iter()) {
            if a.id == "wave_bar" {
                assert_eq!(b.id, "wave_bar");
                assert_eq!(a.x, b.x, "bars never drift sideways");
                assert_eq!(a.w, b.w);
                assert_eq!(a.a, b.a);
                assert!(
                    (a.y + a.h / 2.0 - (b.y + b.h / 2.0)).abs() < 1e-9,
                    "the bars stay center-anchored while they breathe"
                );
                if (a.h - b.h).abs() > 1e-9 {
                    moved += 1;
                }
            } else {
                assert_eq!(a, b, "{} must not move with the wave clock", a.id);
            }
        }
        assert_eq!(
            moved, WAVE_BAR_COUNT,
            "at 1.7s every bar is off its no-clock rest height"
        );
    }

    // ── Works browse ─────────────────────────────────────────────────────

    fn browse_entries() -> Vec<crate::notch::BrowseEntry> {
        vec![
            crate::notch::BrowseEntry {
                id: "7".into(),
                title: "Yesterday's notes".into(),
                subtitle: "active · 3 sittings · You left off in Editor.".into(),
            },
            crate::notch::BrowseEntry {
                id: "9".into(),
                title: "Morning writing".into(),
                subtitle: "dormant · 12 sittings · Three sittings this week.".into(),
            },
            crate::notch::BrowseEntry {
                id: "11".into(),
                title: "Claim drafts".into(),
                subtitle: "active · 1 sitting · One sitting this week.".into(),
            },
        ]
    }

    fn browse_state() -> RenderState {
        RenderState {
            browse: Some(BrowseState {
                id: "9".into(),
                title: "Morning writing".into(),
                subtitle: "dormant · 12 sittings · Three sittings this week.".into(),
                index: 1,
                count: 3,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn browse_layout_reuses_the_carousel_geometry_with_its_own_copy() {
        let state = browse_state();
        let layers = surface_layouts(&state);
        // The browse carousel is the choice carousel's surface: same card
        // geometry, title/subtitle rows, pip strip and chip row.
        let card = layers.iter().find(|l| l.id == "card").expect("browse card");
        let choice_card = surface_layouts(&carousel_state())
            .into_iter()
            .find(|l| l.id == "card")
            .expect("choice card");
        assert_eq!(
            *card, choice_card,
            "browse shares the choice card geometry exactly"
        );

        // The eyebrow speaks in the teal voice; the title/subtitle rows
        // are inside the card.
        let eyebrow = layers.iter().find(|l| l.id == "eyebrow").expect("eyebrow");
        assert_eq!(
            (eyebrow.r, eyebrow.g, eyebrow.b),
            (TEAL_VOICE.0, TEAL_VOICE.1, TEAL_VOICE.2)
        );
        let title = layers
            .iter()
            .find(|l| l.id == "choice_title")
            .expect("title");
        assert!(title.w > 0.0 && title.h > 0.0);
        assert!(title.y >= card.y && title.y + title.h <= card.y + card.h);
        let subtitle = layers
            .iter()
            .find(|l| l.id == "choice_subtitle")
            .expect("subtitle");
        assert_eq!(
            (subtitle.r, subtitle.g, subtitle.b),
            (INK_MUTED.0, INK_MUTED.1, INK_MUTED.2),
            "the browse subtitle is muted ink"
        );

        // Pips: one per work (bounded at eight), the focused index lit.
        let pips: Vec<&Layer> = layers.iter().filter(|l| l.id == "choice_pip").collect();
        assert_eq!(pips.len(), 3, "one pip per work");
        for (index, pip) in pips.iter().enumerate() {
            let expected = if index == 1 { 1.0 } else { 0.35 };
            assert!((pip.a - expected).abs() < 1e-9, "pip {index} alpha");
        }

        // The four chips carry the browse verbs.
        let chips: Vec<&Layer> = layers
            .iter()
            .filter(|l| l.id.starts_with("chip_") && l.action.is_some())
            .collect();
        assert_eq!(chips.len(), 4, "open/prev/next/close");
        let labels: Vec<&str> = chips
            .iter()
            .map(|l| chip_label(&state, l.id).expect("browse chip label"))
            .collect();
        assert_eq!(labels, ["Open", "Prev", "Next", "Close"]);
        let open = chips.iter().find(|l| l.id == "chip_restore").expect("open");
        assert!(open.r > 0.7 && open.g > 0.5, "Open is the caramel pick");
    }

    #[test]
    fn browse_hit_test_routes_the_four_chips_and_open_is_restore() {
        let state = browse_state();
        let layers = surface_layouts(&state);
        for (id, action) in [
            ("chip_restore", OptionAction::Restore),
            ("chip_prev", OptionAction::Prev),
            ("chip_next", OptionAction::Next),
            ("chip_dismiss", OptionAction::Dismiss),
        ] {
            let chip = layers
                .iter()
                .find(|l| l.id == id)
                .unwrap_or_else(|| panic!("{id}"));
            let hit = hit_test(&layers, chip.x + chip.w / 2.0, chip.y + chip.h / 2.0)
                .unwrap_or_else(|| panic!("{id} hit"));
            assert_eq!(hit.id, id);
            assert_eq!(hit.action, Some(action), "{id} dispatches its own verb");
        }
        // The Open chip is the caramel `chip_restore` carrying Restore.
        let open = layers.iter().find(|l| l.id == "chip_restore").unwrap();
        assert_eq!(open.action, Some(OptionAction::Restore));
        // The title rect is the swipe zone, exactly like the choice.
        let title = layers
            .iter()
            .find(|l| l.id == "choice_title")
            .expect("title");
        let hit = hit_test(&layers, title.x + title.w / 2.0, title.y + title.h / 2.0)
            .expect("title zone hit");
        assert_eq!(hit.id, "choice_title");
    }

    #[test]
    fn browse_open_dispatches_the_focused_work_and_restore_means_continue_recent() {
        // The Open chip resolves the focused browse entry's work id...
        let state = browse_state();
        assert_eq!(
            dispatch_for_action(&state, OptionAction::Restore),
            SurfaceAction::Choose("9".into())
        );
        // ...the listening card's rack chip means Browse...
        let listening = RenderState {
            listening: true,
            ..Default::default()
        };
        assert_eq!(
            dispatch_for_action(&listening, OptionAction::OpenRack),
            SurfaceAction::Browse
        );
        // ...and its Restore chip is the continue-recent intent; the old
        // `__notch_restore__` sentinel is gone.
        assert_eq!(
            dispatch_for_action(&listening, OptionAction::Restore),
            SurfaceAction::ContinueRecent
        );
        // The choice carousel keeps dispatching its focused candidate.
        assert_eq!(
            dispatch_for_action(&carousel_state(), OptionAction::Restore),
            SurfaceAction::ChooseCandidate(evo_voice::VoiceCandidate::work(
                "7",
                "Yesterday's notes",
            ))
        );
        // Steps and dismiss keep their uniform verbs on every carousel.
        assert_eq!(
            dispatch_for_action(&state, OptionAction::Next),
            SurfaceAction::ChoiceStep(1)
        );
        assert_eq!(
            dispatch_for_action(&state, OptionAction::Prev),
            SurfaceAction::ChoiceStep(-1)
        );
        assert_eq!(
            dispatch_for_action(&state, OptionAction::Dismiss),
            SurfaceAction::Dismiss
        );
    }

    #[test]
    fn the_listening_chip_row_now_offers_browse() {
        let state = RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            ..Default::default()
        };
        assert_eq!(chip_label(&state, "chip_restore"), Some("Restore"));
        assert_eq!(chip_label(&state, "chip_rack"), Some("Browse"));
        assert_eq!(chip_label(&state, "chip_dismiss"), Some("Dismiss"));
        // The browse carousel relabels the caramel pick and the dismiss
        // chip while keeping their geometry and actions.
        let browse = browse_state();
        assert_eq!(chip_label(&browse, "chip_restore"), Some("Open"));
        assert_eq!(chip_label(&browse, "chip_dismiss"), Some("Close"));
        let layers = surface_layouts(&state);
        let rack = layers
            .iter()
            .find(|l| l.id == "chip_rack")
            .expect("rack chip");
        assert_eq!(rack.action, Some(OptionAction::OpenRack));
    }

    #[test]
    fn browse_drag_over_the_title_zone_steps_the_index_and_wraps() {
        let mut engine = NotchEngine::new();
        engine.show_works(browse_entries());
        let state = RenderState::from_engine(&engine, None);
        let layers = surface_layouts(&state);
        // The drag path finds its swipe zone by this id on both carousels.
        let zone = layers
            .iter()
            .find(|l| l.id == "choice_title")
            .expect("browse swipe zone");
        assert_eq!(
            hit_test(&layers, zone.x + zone.w / 2.0, zone.y + zone.h / 2.0).map(|l| l.id),
            Some("choice_title")
        );

        // 23pt of leftward travel is not a step yet; the 24th completes it.
        let mut accum = 0.0;
        assert!(drag_steps(&mut accum, -23.0).is_empty());
        assert_eq!(drag_steps(&mut accum, -1.0), vec![1]);
        // Steps route to the open browse (not the choice) and advance it.
        step_open_carousel(&mut engine, 1, Instant::now());
        assert_eq!(
            engine
                .browse()
                .and_then(crate::notch::BrowseSession::focused)
                .map(|work| work.id.as_str()),
            Some("9")
        );
        // Stepping past the end wraps forward to the first work.
        step_open_carousel(&mut engine, 1, Instant::now());
        step_open_carousel(&mut engine, 1, Instant::now());
        assert_eq!(
            engine
                .browse()
                .and_then(crate::notch::BrowseSession::focused)
                .map(|work| work.id.as_str()),
            Some("7"),
            "wraps forward"
        );
        // The render state follows the focused work.
        let state = RenderState::from_engine(&engine, None);
        let browse = state.browse.as_ref().expect("browse state");
        assert_eq!(browse.id, "7");
        assert_eq!(browse.title, "Yesterday's notes");
        assert_eq!(browse.index, 0);
        assert_eq!(browse.count, 3);

        // A rightward drag steps backward, wrapping to the last work.
        let mut accum = 0.0;
        assert_eq!(drag_steps(&mut accum, 25.0), vec![-1]);
        step_open_carousel(&mut engine, -1, Instant::now());
        assert_eq!(
            engine
                .browse()
                .and_then(crate::notch::BrowseSession::focused)
                .map(|work| work.id.as_str()),
            Some("11"),
            "wraps backward"
        );
    }

    #[test]
    fn browse_none_states_carry_no_browse_layout() {
        // The browse phase only exists on `browse: Some(..)`: an engine
        // without a session maps to None, and the compact rack state is
        // untouched.
        let mut engine = NotchEngine::new();
        engine.show_live(ContentId::EvoRack, crate::notch::priority::RACK, true);
        let state = RenderState::from_engine(&engine, Some(2));
        assert!(state.browse.is_none());
        let ids: Vec<&str> = surface_layouts(&state)
            .iter()
            .map(|layer| layer.id)
            .collect();
        assert_eq!(ids, vec!["band"], "a no-browse rack stays the compact band");

        let idle = RenderState {
            browse: None,
            ..Default::default()
        };
        assert!(surface_layouts(&idle).is_empty());
    }

    #[test]
    fn works_input_opens_the_browse_and_an_empty_listing_closes_it() {
        let mut engine = NotchEngine::new();
        // Route the listing through the input the surface consumes, the
        // same delegation surface_main performs.
        let (input_tx, input_rx) = std::sync::mpsc::channel::<SurfaceInput>();
        let _ = input_tx.send(SurfaceInput::Works(browse_entries()));
        drop(input_tx);
        while let Ok(input) = input_rx.recv() {
            if let SurfaceInput::Works(entries) = input {
                engine.show_works(entries);
            }
        }
        let state = RenderState::from_engine(&engine, None);
        let browse = state.browse.as_ref().expect("browse opens");
        assert_eq!(browse.id, "7", "focused on the first work");
        assert_eq!(browse.title, "Yesterday's notes");
        assert_eq!(
            browse.subtitle,
            "active · 3 sittings · You left off in Editor."
        );
        assert_eq!(browse.index, 0);
        assert_eq!(browse.count, 3);
        assert!(
            surface_layouts(&state).iter().any(|l| l.id == "card"),
            "the carousel paints"
        );

        // An empty listing is not content: it closes the open browse.
        let (input_tx, input_rx) = std::sync::mpsc::channel::<SurfaceInput>();
        let _ = input_tx.send(SurfaceInput::Works(Vec::new()));
        drop(input_tx);
        while let Ok(input) = input_rx.recv() {
            if let SurfaceInput::Works(entries) = input {
                engine.show_works(entries);
            }
        }
        let state = RenderState::from_engine(&engine, None);
        assert!(state.browse.is_none(), "an empty listing leaves no session");
        assert!(
            surface_layouts(&state).is_empty(),
            "an empty listing leaves no browse layout"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn paint_into_draws_the_browse_card_and_its_title_glyphs() {
        // The browse card paints without panicking and the focused work's
        // title is real light ink inside its rect, not a placeholder fill.
        use objc::paint_into;
        let state = browse_state();
        let layers = surface_layouts(&state);
        let title = layers
            .iter()
            .find(|l| l.id == "choice_title")
            .expect("browse title")
            .clone();
        let bmp = paint_into(&state);
        assert_eq!(
            (bmp.width, bmp.height),
            (CANVAS_SIZE.0 as usize, CANVAS_SIZE.1 as usize)
        );
        let is_light = |p: [u8; 4]| -> bool { p[3] > 0 && p[0] > 140 && p[1] > 140 };
        let mut lit = 0usize;
        let mut total = 0usize;
        for py in title.y as usize..(title.y + title.h) as usize {
            for px in title.x as usize..(title.x + title.w) as usize {
                total += 1;
                if is_light(bmp.pixel(px, py)) {
                    lit += 1;
                }
            }
        }
        assert!(lit > 0, "browse title glyph pixels present");
        assert!(lit * 5 < total, "title is sparse glyphs, not a light fill");
    }
}
