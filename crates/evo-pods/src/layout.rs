//! Layouts: how a work's windows occupy a space.
//!
//! A pod knew *roles* (hero / satellite / rail) but not a layout: there was
//! no named, normalized arrangement a work's windows are planned into, and
//! no way to change one without re-deriving the world. This module is that
//! missing piece, and it is a pure decision: screens, slots and window
//! frames arrive as numbers, and nothing here calls the OS.
//!
//! Adapted from **Setpiece** (<https://github.com/ChiFungHillmanChan/setpiece>,
//! MIT, Copyright (c) 2026 Hillman Chan — see `THIRD-PARTY-NOTICES.md`):
//! its normalized [`Slot`] rects resolved inside a display's usable frame,
//! its sticky-then-fill planning pass with a ~10 pt approximate match, its
//! overflow-to-minimize ending, its split tree with the `kind` / `ratio` /
//! `first` / `second` JSON shape, and its ten built-in presets. Evo adds the
//! [`Region`] (a layout can own part of a display and leave the rest of the
//! desktop free) and two native seeds: [`LayoutId::HeroRails`], which is the
//! existing [`crate::stage::Role`] geometry expressed as slots, and
//! [`LayoutId::Stack`] (an explicit slot list — its gutters mean the stack is
//! not a guillotine partition, so no tree can express it).
//!
//! # The laws
//!
//! 1. **Normalized, never pixel-locked.** Every rect is a ratio of its
//!    region, so a layout made on one display re-derives honestly on another.
//! 2. **Inside 0..=1, always.** Region fractions, slots and split ratios are
//!    clamped at construction *and* at deserialization; a slot can never
//!    point outside the box it was given.
//! 3. **The person's placement wins.** The sticky pass preserves a window
//!    already sitting in a slot (within [`STICKY_TOLERANCE`], including a
//!    slot's alternate geometry) before any later window can fill it.
//! 4. **Nothing silently vanishes.** Windows beyond the slot count are
//!    reported in [`Plan::to_minimize`]; the caller books the receipts.

use serde::{Deserialize, Deserializer, Serialize};

use crate::stage::RatioRect;
use crate::surface::Frame;

// ─── Regions ────────────────────────────────────────────────────────────────

/// The fraction of the usable width a side region takes by default.
pub const DEFAULT_SIDE: f64 = 0.6;
/// The fraction of the usable width a centred column takes by default.
pub const DEFAULT_CENTER: f64 = 0.7;
/// The fraction of the usable height a top/bottom band takes.
pub const DEFAULT_BAND: f64 = 0.6;
/// The smallest fraction a region may resolve to. Below this a region is no
/// longer a space; the clamp keeps a hostile or stale value from producing a
/// sliver nobody can use.
pub const MIN_REGION_FRACTION: f64 = 0.25;

/// A named fraction of a screen's usable frame that a layout owns.
///
/// The point of a region is that a work can occupy *part* of a display and
/// leave the rest of the desktop free: containment stays off (the desktop is
/// never hidden, dimmed or reordered), yet the work still reads as one space
/// because its windows share a bounded neighbourhood.
///
/// Fractions are clamped into `[`MIN_REGION_FRACTION`..=1.0]` when resolved;
/// a non-finite fraction resolves at its documented default. The primary
/// region is resolved per display, so shrinking a screen never smears a
/// layout off it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum Region {
    /// The whole usable frame.
    #[default]
    Full,
    /// The left band: `fraction` of the usable width, full height.
    Left(f64),
    /// The right band: `fraction` of the usable width, full height.
    Right(f64),
    /// A centred column: `fraction` of the usable width, full height.
    Center(f64),
    /// The top band: full width, [`DEFAULT_BAND`] of the usable height.
    Top,
    /// The bottom band: full width, [`DEFAULT_BAND`] of the usable height.
    Bottom,
}

impl Region {
    /// The left band at its documented default width ([`DEFAULT_SIDE`]).
    pub fn left() -> Self {
        Region::Left(DEFAULT_SIDE)
    }

    /// The right band at its documented default width ([`DEFAULT_SIDE`]).
    pub fn right() -> Self {
        Region::Right(DEFAULT_SIDE)
    }

    /// The centred column at its documented default width ([`DEFAULT_CENTER`]).
    pub fn center() -> Self {
        Region::Center(DEFAULT_CENTER)
    }

    /// This region's fraction of the axis it divides, as resolved. `Full`
    /// resolves at `1.0`; the bands at their band default.
    pub fn fraction(self) -> f64 {
        match self {
            Region::Full => 1.0,
            Region::Left(fraction) | Region::Right(fraction) | Region::Center(fraction) => {
                clamp_fraction(fraction)
            }
            Region::Top | Region::Bottom => DEFAULT_BAND,
        }
    }

    /// The rect this region owns inside `screen`. A screen that cannot be
    /// used (zero or non-finite area) yields the screen itself, the same
    /// honest fallback [`RatioRect::to_frame`] takes.
    pub fn frame(self, screen: &Frame) -> Frame {
        if !screen.usable() {
            return *screen;
        }
        let fraction = self.fraction();
        match self {
            Region::Full => *screen,
            Region::Left(_) => {
                Frame::new(screen.x, screen.y, screen.w * fraction, screen.h)
            }
            Region::Right(_) => Frame::new(
                screen.x + screen.w * (1.0 - fraction),
                screen.y,
                screen.w * fraction,
                screen.h,
            ),
            Region::Center(_) => {
                let width = screen.w * fraction;
                Frame::new(screen.x + (screen.w - width) / 2.0, screen.y, width, screen.h)
            }
            Region::Top => Frame::new(screen.x, screen.y, screen.w, screen.h * fraction),
            Region::Bottom => Frame::new(
                screen.x,
                screen.y + screen.h * (1.0 - fraction),
                screen.w,
                screen.h * fraction,
            ),
        }
    }
}

/// A fraction clamped into the usable range; non-finite values take the
/// documented default for their axis.
fn clamp_fraction(fraction: f64) -> f64 {
    if fraction.is_finite() {
        fraction.clamp(MIN_REGION_FRACTION, 1.0)
    } else {
        DEFAULT_SIDE
    }
}

// ─── Slots ──────────────────────────────────────────────────────────────────

/// The smallest side a slot may have (Setpiece's template floor, 5 %): a
/// slot is always a real target, never a line.
pub const MIN_SLOT_SIDE: f64 = 0.05;

/// One normalized slot *inside* the layout's region: the unit rect a window
/// is planned to fill, top-left origin, `0..=1` of the region.
///
/// Construction clamps: origins into `0..=(1 - MIN_SLOT_SIDE)` and sides
/// into `MIN_SLOT_SIDE..=(1 - origin)`, so `x + w <= 1` and `y + h <= 1`
/// hold by construction for every value a caller — or a JSON file — can hand
/// in, including infinities and NaN.
///
/// Slots are *derived* state: what persists is the split tree or the seed a
/// work chose, never a flattened slot list, so this type carries no serde
/// shape of its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub rect: RatioRect,
}

impl Slot {
    /// A slot from raw ratios, clamped into the unit box.
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        let x = clamp_origin(x);
        let y = clamp_origin(y);
        Self {
            rect: RatioRect {
                x,
                y,
                w: clamp_side(w, x),
                h: clamp_side(h, y),
            },
        }
    }

    /// The absolute frame this slot asks for inside `region`.
    pub fn frame(&self, region: &Frame) -> Frame {
        self.rect.to_frame(region)
    }

    /// The slot a frame currently occupies inside `region`. The inverse of
    /// [`Slot::frame`] for usable inputs; a frame that cannot be expressed
    /// yields a clamped slot rather than a panic.
    pub fn from_frame(frame: &Frame, region: &Frame) -> Self {
        let ratio = RatioRect::from_frame(frame, region);
        Self::new(ratio.x, ratio.y, ratio.w, ratio.h)
    }
}

/// An origin clamped so a side of [`MIN_SLOT_SIDE`] always fits after it.
fn clamp_origin(origin: f64) -> f64 {
    let value = if origin.is_finite() { origin } else { 0.0 };
    value.clamp(0.0, 1.0 - MIN_SLOT_SIDE)
}

/// A side clamped between the slot floor and the room left by its origin.
/// A side that cannot be expressed (non-finite) lands on the floor.
fn clamp_side(side: f64, origin: f64) -> f64 {
    let room = 1.0 - origin;
    let value = if side.is_finite() { side } else { MIN_SLOT_SIDE };
    value.clamp(MIN_SLOT_SIDE, room)
}

// ─── Split trees ────────────────────────────────────────────────────────────

/// The smallest split ratio a seam may take. A seam can never fully collapse
/// a slot away: the editor that will drag these seams cannot lose a window.
pub const MIN_RATIO: f64 = 0.05;
/// The largest split ratio a seam may take.
pub const MAX_RATIO: f64 = 0.95;
/// Where a non-finite ratio lands.
pub const DEFAULT_RATIO: f64 = 0.5;

/// A guillotine split tree: the door a seam-drag editor walks through later
/// without a rewrite. Serialized with Setpiece's own JSON shape — a `kind`
/// discriminator plus `ratio` / `first` / `second`:
///
/// ```json
/// {"kind":"hSplit","ratio":0.7,
///  "first":{"kind":"leaf"},
///  "second":{"kind":"vSplit","ratio":0.5,
///            "first":{"kind":"leaf"},"second":{"kind":"leaf"}}}
/// ```
///
/// Ratios are clamped to `[`MIN_RATIO`..=`MAX_RATIO`]` on construction *and*
/// on deserialization, and a non-finite ratio lands at [`DEFAULT_RATIO`], so
/// a hostile or stale file can never produce a zero-area slot.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum LayoutNode {
    /// One slot.
    #[serde(rename = "leaf")]
    Leaf,
    /// A horizontal split: `first` on top, `second` below.
    #[serde(rename = "hSplit")]
    HSplit {
        ratio: f64,
        #[serde(rename = "first")]
        top: Box<LayoutNode>,
        #[serde(rename = "second")]
        bottom: Box<LayoutNode>,
    },
    /// A vertical split: `first` on the left, `second` on the right.
    #[serde(rename = "vSplit")]
    VSplit {
        ratio: f64,
        #[serde(rename = "first")]
        left: Box<LayoutNode>,
        #[serde(rename = "second")]
        right: Box<LayoutNode>,
    },
}

impl LayoutNode {
    /// One slot.
    pub fn leaf() -> Self {
        LayoutNode::Leaf
    }

    /// `first` on top of `second`, `ratio` of the height going to `first`.
    pub fn hsplit(ratio: f64, first: LayoutNode, second: LayoutNode) -> Self {
        LayoutNode::HSplit {
            ratio: clamp_ratio(ratio),
            top: Box::new(first),
            bottom: Box::new(second),
        }
    }

    /// `first` left of `second`, `ratio` of the width going to `first`.
    pub fn vsplit(ratio: f64, first: LayoutNode, second: LayoutNode) -> Self {
        LayoutNode::VSplit {
            ratio: clamp_ratio(ratio),
            left: Box::new(first),
            right: Box::new(second),
        }
    }

    /// This tree's ratio, clamped (leaves have none and report
    /// [`DEFAULT_RATIO`]).
    pub fn ratio(&self) -> f64 {
        match self {
            LayoutNode::Leaf => DEFAULT_RATIO,
            LayoutNode::HSplit { ratio, .. } | LayoutNode::VSplit { ratio, .. } => *ratio,
        }
    }

    /// The slots this tree asks for, depth-first in reading order: a
    /// horizontal split flattens top-first, a vertical split left-first —
    /// the order a work's windows are planned into.
    pub fn flatten(&self) -> Vec<Slot> {
        let mut slots = Vec::new();
        self.flatten_into(RatioRect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 }, &mut slots);
        slots
    }

    /// How many slots this tree asks for.
    pub fn leaf_count(&self) -> usize {
        match self {
            LayoutNode::Leaf => 1,
            LayoutNode::HSplit { top, bottom, .. } => top.leaf_count() + bottom.leaf_count(),
            LayoutNode::VSplit { left, right, .. } => left.leaf_count() + right.leaf_count(),
        }
    }

    fn flatten_into(&self, rect: RatioRect, slots: &mut Vec<Slot>) {
        match self {
            LayoutNode::Leaf => slots.push(Slot::new(rect.x, rect.y, rect.w, rect.h)),
            LayoutNode::HSplit {
                ratio,
                top,
                bottom,
            } => {
                let first_h = rect.h * ratio;
                let remaining = rect.h - first_h;
                top.flatten_into(
                    RatioRect {
                        x: rect.x,
                        y: rect.y,
                        w: rect.w,
                        h: first_h,
                    },
                    slots,
                );
                bottom.flatten_into(
                    RatioRect {
                        x: rect.x,
                        y: rect.y + first_h,
                        w: rect.w,
                        h: remaining,
                    },
                    slots,
                );
            }
            LayoutNode::VSplit {
                ratio,
                left,
                right,
            } => {
                let first_w = rect.w * ratio;
                let remaining = rect.w - first_w;
                left.flatten_into(
                    RatioRect {
                        x: rect.x,
                        y: rect.y,
                        w: first_w,
                        h: rect.h,
                    },
                    slots,
                );
                right.flatten_into(
                    RatioRect {
                        x: rect.x + first_w,
                        y: rect.y,
                        w: remaining,
                        h: rect.h,
                    },
                    slots,
                );
            }
        }
    }
}

/// A ratio clamped into the seam range; non-finite lands at the default.
fn clamp_ratio(ratio: f64) -> f64 {
    if ratio.is_finite() {
        ratio.clamp(MIN_RATIO, MAX_RATIO)
    } else {
        DEFAULT_RATIO
    }
}

/// Setpiece's field names, kept exactly: `kind` / `ratio` / `first` /
/// `second`. Deserialization is manual so the clamp in [`clamp_ratio`] is
/// the only way a ratio can enter the type.
#[derive(Deserialize)]
#[serde(tag = "kind")]
enum RawLayoutNode {
    #[serde(rename = "leaf")]
    Leaf,
    #[serde(rename = "hSplit")]
    HSplit {
        ratio: f64,
        first: Box<RawLayoutNode>,
        second: Box<RawLayoutNode>,
    },
    #[serde(rename = "vSplit")]
    VSplit {
        ratio: f64,
        first: Box<RawLayoutNode>,
        second: Box<RawLayoutNode>,
    },
}

impl RawLayoutNode {
    fn clamped(self) -> LayoutNode {
        match self {
            RawLayoutNode::Leaf => LayoutNode::Leaf,
            RawLayoutNode::HSplit {
                ratio,
                first,
                second,
            } => LayoutNode::hsplit(ratio, first.clamped(), second.clamped()),
            RawLayoutNode::VSplit {
                ratio,
                first,
                second,
            } => LayoutNode::vsplit(ratio, first.clamped(), second.clamped()),
        }
    }
}

impl<'de> Deserialize<'de> for LayoutNode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(RawLayoutNode::deserialize(deserializer)?.clamped())
    }
}

// ─── Seeds ──────────────────────────────────────────────────────────────────

/// The twelve seeds: ten adapted from Setpiece's built-in presets (their
/// names and slot counts), plus two Evo-native ones.
///
/// The key of a seed is stable text — it is what a work's saved preference
/// will name, so a renamed layout never silently changes what a work opens
/// into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LayoutId {
    /// One window fills the region.
    Full,
    /// Two equal columns.
    Halves,
    /// Three equal columns.
    Thirds,
    /// A 2×2 grid.
    Quads,
    /// 70 % main column, 30 % side column.
    MainSide,
    /// A split left column beside a full-height right column.
    LeftSplitRight,
    /// A full-height left column beside a split right column.
    LeftRightSplit,
    /// 70 % top row, 30 % bottom row.
    MainSideVertical,
    /// Two equal rows.
    HalvesVertical,
    /// Three equal rows.
    ThirdsVertical,
    /// Evo-native: the existing [`crate::stage::Role`] geometry — hero,
    /// three satellite bands, four rail strips — as slots 0..=7.
    HeroRails,
    /// Evo-native: four stacked bands, one per terminal.
    Stack,
}

impl LayoutId {
    /// Every seed, in the order a picker should show them.
    pub const ALL: [LayoutId; 12] = [
        LayoutId::Full,
        LayoutId::Halves,
        LayoutId::Thirds,
        LayoutId::Quads,
        LayoutId::MainSide,
        LayoutId::LeftSplitRight,
        LayoutId::LeftRightSplit,
        LayoutId::MainSideVertical,
        LayoutId::HalvesVertical,
        LayoutId::ThirdsVertical,
        LayoutId::HeroRails,
        LayoutId::Stack,
    ];

    /// Every seed, in picker order.
    pub fn all() -> &'static [LayoutId] {
        &Self::ALL
    }

    /// The stable text key (what a saved preference names).
    pub fn key(self) -> &'static str {
        match self {
            LayoutId::Full => "full",
            LayoutId::Halves => "halves",
            LayoutId::Thirds => "thirds",
            LayoutId::Quads => "quads",
            LayoutId::MainSide => "main-side",
            LayoutId::LeftSplitRight => "left-split-right",
            LayoutId::LeftRightSplit => "left-right-split",
            LayoutId::MainSideVertical => "main-side-vertical",
            LayoutId::HalvesVertical => "halves-vertical",
            LayoutId::ThirdsVertical => "thirds-vertical",
            LayoutId::HeroRails => "hero-rails",
            LayoutId::Stack => "stack",
        }
    }

    /// The seed a stable key names, if any.
    pub fn from_key(key: &str) -> Option<LayoutId> {
        LayoutId::ALL.into_iter().find(|id| id.key() == key)
    }

    /// The human name (English, sentence case).
    pub fn name(self) -> &'static str {
        match self {
            LayoutId::Full => "Full",
            LayoutId::Halves => "Halves",
            LayoutId::Thirds => "Thirds",
            LayoutId::Quads => "Quads",
            LayoutId::MainSide => "Main + Side",
            LayoutId::LeftSplitRight => "Left split + right",
            LayoutId::LeftRightSplit => "Left + right split",
            LayoutId::MainSideVertical => "Main + side (vertical)",
            LayoutId::HalvesVertical => "Halves (vertical)",
            LayoutId::ThirdsVertical => "Thirds (vertical)",
            LayoutId::HeroRails => "Hero + rails",
            LayoutId::Stack => "Stack",
        }
    }

    /// This seed as a layout, in the [`Region::Full`] region.
    pub fn seed(self) -> Layout {
        Layout::seed(self)
    }
}

/// What a layout's slots come from: a split tree (adjustable later) or an
/// explicit slot list (geometry that is not a guillotine partition).
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutShape {
    /// A split tree, flattened depth-first in reading order.
    Tree(LayoutNode),
    /// An explicit slot list, in planning order.
    Slots(Vec<Slot>),
}

/// A named arrangement: the seeds, plus whatever a saved preference asks to
/// change about the region.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub id: LayoutId,
    pub name: &'static str,
    pub region: Region,
    pub shape: LayoutShape,
}

impl Layout {
    /// A seed in its default region.
    pub fn seed(id: LayoutId) -> Layout {
        let (shape, region) = match id {
            LayoutId::Full => (LayoutShape::Tree(LayoutNode::leaf()), Region::Full),
            LayoutId::Halves => (
                LayoutShape::Tree(LayoutNode::vsplit(0.5, LayoutNode::leaf(), LayoutNode::leaf())),
                Region::Full,
            ),
            LayoutId::Thirds => (
                LayoutShape::Tree(LayoutNode::vsplit(
                    1.0 / 3.0,
                    LayoutNode::leaf(),
                    LayoutNode::vsplit(0.5, LayoutNode::leaf(), LayoutNode::leaf()),
                )),
                Region::Full,
            ),
            LayoutId::Quads => (
                LayoutShape::Tree(LayoutNode::hsplit(
                    0.5,
                    LayoutNode::vsplit(0.5, LayoutNode::leaf(), LayoutNode::leaf()),
                    LayoutNode::vsplit(0.5, LayoutNode::leaf(), LayoutNode::leaf()),
                )),
                Region::Full,
            ),
            LayoutId::MainSide => (
                LayoutShape::Tree(LayoutNode::vsplit(0.7, LayoutNode::leaf(), LayoutNode::leaf())),
                Region::Full,
            ),
            LayoutId::LeftSplitRight => (
                LayoutShape::Tree(LayoutNode::vsplit(
                    0.5,
                    LayoutNode::hsplit(0.5, LayoutNode::leaf(), LayoutNode::leaf()),
                    LayoutNode::leaf(),
                )),
                Region::Full,
            ),
            LayoutId::LeftRightSplit => (
                LayoutShape::Tree(LayoutNode::vsplit(
                    0.5,
                    LayoutNode::leaf(),
                    LayoutNode::hsplit(0.5, LayoutNode::leaf(), LayoutNode::leaf()),
                )),
                Region::Full,
            ),
            LayoutId::MainSideVertical => (
                LayoutShape::Tree(LayoutNode::hsplit(0.7, LayoutNode::leaf(), LayoutNode::leaf())),
                Region::Full,
            ),
            LayoutId::HalvesVertical => (
                LayoutShape::Tree(LayoutNode::hsplit(0.5, LayoutNode::leaf(), LayoutNode::leaf())),
                Region::Full,
            ),
            LayoutId::ThirdsVertical => (
                LayoutShape::Tree(LayoutNode::hsplit(
                    1.0 / 3.0,
                    LayoutNode::leaf(),
                    LayoutNode::hsplit(0.5, LayoutNode::leaf(), LayoutNode::leaf()),
                )),
                Region::Full,
            ),
            LayoutId::HeroRails => (LayoutShape::Slots(hero_rails_slots()), Region::Full),
            LayoutId::Stack => (LayoutShape::Slots(stack_slots()), Region::Full),
        };
        Layout {
            id,
            name: id.name(),
            region,
            shape,
        }
    }

    /// The same layout in another region.
    pub fn with_region(mut self, region: Region) -> Layout {
        self.region = region;
        self
    }

    /// The layout's slots, in planning order.
    pub fn slots(&self) -> Vec<Slot> {
        match &self.shape {
            LayoutShape::Tree(node) => node.flatten(),
            LayoutShape::Slots(slots) => slots.clone(),
        }
    }

    /// The split tree, when this layout has one.
    pub fn tree(&self) -> Option<&LayoutNode> {
        match &self.shape {
            LayoutShape::Tree(node) => Some(node),
            LayoutShape::Slots(_) => None,
        }
    }

    /// The absolute slot frames this layout asks for on `screen`: the region
    /// resolved inside the screen's usable frame, then every slot inside the
    /// region.
    pub fn resolved_slots(&self, screen: &Frame) -> Vec<Frame> {
        let region = self.region.frame(screen);
        self.slots().iter().map(|slot| slot.frame(&region)).collect()
    }
}

/// The hero/satellite/rail geometry as slots: the same formulas
/// [`crate::stage::Role::default_ratio`] uses, so a work that already learned
/// this arrangement keeps it (a test pins the two together).
pub fn hero_rails_slots() -> Vec<Slot> {
    const HERO: (f64, f64, f64, f64) = (0.03, 0.06, 0.58, 0.62);
    const SATELLITES: usize = 3;
    const RAILS: usize = 4;
    let mut slots = vec![Slot::new(HERO.0, HERO.1, HERO.2, HERO.3)];
    for index in 0..SATELLITES {
        let band = 0.88 / SATELLITES as f64;
        slots.push(Slot::new(
            0.63,
            0.06 + band * index as f64,
            0.34,
            band - 0.02,
        ));
    }
    for index in 0..RAILS {
        let width = 0.58 / RAILS as f64;
        slots.push(Slot::new(
            0.03 + width * index as f64,
            0.72,
            width - 0.008,
            0.22,
        ));
    }
    slots
}

/// The stack geometry: four full-width bands, separated by a hairline gutter
/// so the windows read as separate objects rather than one wall. The gutter
/// is why this seed carries explicit slots instead of a split tree.
pub fn stack_slots() -> Vec<Slot> {
    const BANDS: usize = 4;
    const GUTTER: f64 = 0.006;
    (0..BANDS)
        .map(|index| {
            let band = 1.0 / BANDS as f64;
            Slot::new(0.0, band * index as f64, 1.0, band - GUTTER)
        })
        .collect()
}

// ─── Planning ───────────────────────────────────────────────────────────────

/// How close a window's frame must be to a slot's frame to count as already
/// placed. Setpiece's own tolerance, in points.
pub const STICKY_TOLERANCE: f64 = 10.0;

/// One live window as the planner sees it: identity, and where it is now.
/// The slice order a caller passes to [`LayoutEngine::plan`] *is* the
/// z-order, frontmost first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowRef {
    pub pid: i32,
    pub window_id: u32,
    pub frame: Frame,
}

/// Where one window should be. Nothing has happened yet: this is a decision,
/// and the caller books the receipts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub slot: usize,
    pub pid: i32,
    pub window_id: u32,
    pub frame: Frame,
}

/// An arrangement decision: what to place where, what is left over, and what
/// stays empty.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Plan {
    /// One entry per placed window, in z-order.
    pub placements: Vec<Placement>,
    /// Windows beyond the slot count, in z-order: the caller minimizes these
    /// (Setpiece's overflow rule) and books the receipts.
    pub to_minimize: Vec<WindowRef>,
    /// Slot indices nothing was placed into, ascending.
    pub empty_slots: Vec<usize>,
}

impl Plan {
    /// The slot a window was given, if any.
    pub fn slot_for(&self, pid: i32, window_id: u32) -> Option<usize> {
        self.placements
            .iter()
            .find(|p| p.pid == pid && p.window_id == window_id)
            .map(|p| p.slot)
    }

    /// Whether the plan would move nothing, minimize nothing and fill
    /// nothing new.
    pub fn is_empty(&self) -> bool {
        self.placements.is_empty() && self.to_minimize.is_empty()
    }
}

/// The planner. Two passes over the windows in z-order — sticky, then fill —
/// and an honest ending for whatever is left.
pub struct LayoutEngine;

impl LayoutEngine {
    /// Plan `windows_in_z_order` (frontmost first) into `slots`.
    ///
    /// * **Sticky pass.** A window whose current frame already matches a free
    ///   slot (within [`STICKY_TOLERANCE`]) keeps that slot. `alternating_frames`
    ///   carries each slot's *alternate* geometry — the same slot resolved on
    ///   another tiling frame (the Dock moved, Stage Manager ran) — indexed by
    ///   slot; a window matching one is preserved too. A short slice is fine:
    ///   missing alternates simply add no candidates.
    /// * **Fill pass.** Remaining windows, still in z-order, take the lowest
    ///   free slot indices.
    /// * **Overflow.** Anything left is reported for minimization.
    pub fn plan(
        windows_in_z_order: &[WindowRef],
        slots: &[Frame],
        alternating_frames: &[Frame],
    ) -> Plan {
        let mut taken: Vec<bool> = vec![false; slots.len()];
        let mut assigned: Vec<Option<usize>> = vec![None; windows_in_z_order.len()];

        for (index, window) in windows_in_z_order.iter().enumerate() {
            if let Some(slot) = first_free_slot(slots, alternating_frames, &taken, &window.frame) {
                taken[slot] = true;
                assigned[index] = Some(slot);
            }
        }
        for (index, _window) in windows_in_z_order.iter().enumerate() {
            if assigned[index].is_some() {
                continue;
            }
            if let Some(slot) = taken.iter().position(|claimed| !claimed) {
                taken[slot] = true;
                assigned[index] = Some(slot);
            }
        }

        let mut plan = Plan::default();
        let mut leftover: Vec<WindowRef> = Vec::new();
        for (index, window) in windows_in_z_order.iter().enumerate() {
            match assigned[index] {
                Some(slot_index) => plan.placements.push(Placement {
                    slot: slot_index,
                    pid: window.pid,
                    window_id: window.window_id,
                    frame: slots[slot_index],
                }),
                None => leftover.push(*window),
            }
        }
        plan.to_minimize = leftover;
        plan.empty_slots = taken
            .iter()
            .enumerate()
            .filter_map(|(index, claimed)| (!claimed).then_some(index))
            .collect();
        plan
    }

    /// Plan `windows` into `layout` as resolved on `screen`.
    ///
    /// `alternating_screens` are other tiling frames the same layout could be
    /// resolved on (Setpiece's sticky re-apply after the Dock moved). The
    /// first one that resolves the same slot count supplies the alternates;
    /// anything else is ignored, because a different shape is not the same
    /// slot wearing a new frame.
    pub fn plan_layout(
        windows: &[WindowRef],
        layout: &Layout,
        screen: &Frame,
        alternating_screens: &[Frame],
    ) -> Plan {
        let slots = layout.resolved_slots(screen);
        let alternating = alternating_screens
            .iter()
            .map(|alternate| layout.resolved_slots(alternate))
            .find(|alternate| alternate.len() == slots.len())
            .unwrap_or_default();
        Self::plan(windows, &slots, &alternating)
    }
}

/// The first slot this frame already sits in: primary geometry first, then
/// the slot's alternate geometry. Slot order decides when more than one
/// candidate matches, so the result is deterministic.
fn first_free_slot(
    slots: &[Frame],
    alternating: &[Frame],
    taken: &[bool],
    frame: &Frame,
) -> Option<usize> {
    let matching = |target: &Frame| frames_approx_equal(frame, target, STICKY_TOLERANCE);
    let primary = (0..slots.len()).find(|index| !taken[*index] && matching(&slots[*index]));
    primary.or_else(|| {
        (0..slots.len()).find(|index| {
            !taken[*index]
                && alternating
                    .get(*index)
                    .is_some_and(matching)
        })
    })
}

/// Whether two frames agree on every edge within `tolerance` points.
pub fn frames_approx_equal(a: &Frame, b: &Frame, tolerance: f64) -> bool {
    (a.x - b.x).abs() <= tolerance
        && (a.y - b.y).abs() <= tolerance
        && (a.w - b.w).abs() <= tolerance
        && (a.h - b.h).abs() <= tolerance
}
