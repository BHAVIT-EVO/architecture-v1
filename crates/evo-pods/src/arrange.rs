//! Arranging a work: which layout it opens into, and what the person's
//! drags teach it.
//!
//! [`crate::layout`] owns the geometry (slots, regions, the planning pass)
//! and [`crate::reveal`] owns how the move is choreographed. This module
//! owns the *choice*: given a work's live windows, its learned preference,
//! and the display, produce one [`Arrangement`].
//!
//! Three laws keep the choice honest:
//!
//! 1. **Automatic first, never guessed per-app.** With no pick, a work opens
//!    into the smallest seed that holds every window ([`auto_layout`]); the
//!    person's pick always wins over the table.
//! 2. **Learning is geometry, not identity.** A drag teaches one slot of one
//!    layout, measured *inside the layout's region* — so a Dock move or a
//!    narrower region carries the learning instead of smearing it.
//! 3. **Containment stays off.** Nothing here veils, parks, or hides a
//!    foreign window: a work occupies its slots and the rest of the desktop
//!    is exactly as the person left it.
//!
//! All math is platform-free: windows and screens arrive as numbers.

use std::collections::BTreeMap;

use crate::layout::{
    Layout, LayoutEngine, LayoutId, Plan, Region, Slot, WindowRef, frames_approx_equal,
};
use crate::stage::RatioRect;
use crate::surface::Frame;

/// The seed a work opens into when nobody has picked one.
///
/// "The smallest seed that holds every window, preferring the reading
/// order people actually use": a full frame for one, columns for two and
/// three, a grid for four, and the hero+rails geometry beyond that (its
/// eight slots are the widest seed, so nine or more windows overflow into
/// minimization — the documented ending, never a silent disappearance).
pub fn auto_layout(count: usize) -> LayoutId {
    match count {
        0 | 1 => LayoutId::Full,
        2 => LayoutId::Halves,
        3 => LayoutId::Thirds,
        4 => LayoutId::Quads,
        _ => LayoutId::HeroRails,
    }
}

/// What one work has learned about arranging: its pick, the region each
/// layout owns, and the slot frames the person dragged.
///
/// `Default` is the honest starting state: no pick (the count decides), every
/// layout full-frame, nothing learned.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArrangeMemory {
    /// The person's layout pick. `None` means "decide by window count".
    pub chosen: Option<LayoutId>,
    /// Region overrides, per layout. Absent = [`Region::Full`].
    pub regions: BTreeMap<LayoutId, Region>,
    /// Learned slot frames, per layout and slot index, as fractions of the
    /// layout's *region* (see [`Slot::from_frame`]).
    pub overrides: BTreeMap<LayoutId, BTreeMap<usize, Slot>>,
}

impl ArrangeMemory {
    pub fn new() -> Self {
        Self::default()
    }

    /// The layout a work with `count` windows opens into: the pick if there
    /// is one, else [`auto_layout`].
    pub fn layout_for(&self, count: usize) -> Layout {
        self.layout(self.chosen.unwrap_or_else(|| auto_layout(count)))
    }

    /// One seed, with the memory's region applied.
    pub fn layout(&self, id: LayoutId) -> Layout {
        let mut layout = id.seed();
        if let Some(region) = self.regions.get(&id) {
            layout = layout.with_region(*region);
        }
        layout
    }

    /// The layout's slots as absolute frames on `screen`, with learned slot
    /// frames substituted where the person dragged one.
    pub fn slots_for(&self, layout: &Layout, screen: &Frame) -> Vec<Frame> {
        let region = layout.region.frame(screen);
        let learned = self.overrides.get(&layout.id);
        layout
            .slots()
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                learned
                    .and_then(|overrides| overrides.get(&index))
                    .copied()
                    .unwrap_or(*slot)
                    .frame(&region)
            })
            .collect()
    }

    /// Picks a layout for this work. The next arrangement — and an active
    /// work's immediate re-tile — uses it.
    pub fn choose(&mut self, id: LayoutId) {
        self.chosen = Some(id);
    }

    /// Back to automatic choice by window count.
    pub fn clear_choice(&mut self) {
        self.chosen = None;
    }

    /// Narrows the layout to a region of the display.
    pub fn set_region(&mut self, id: LayoutId, region: Region) {
        self.regions.insert(id, region);
    }

    /// Forgets learned slot frames for one layout (the person asked for the
    /// seed geometry back).
    pub fn forget(&mut self, id: LayoutId) {
        self.overrides.remove(&id);
    }

    /// Learns a drag: the slot whose centre the window now sits nearest
    /// keeps the dragged frame. Measured inside the layout's region, so the
    /// learning is portable across Dock changes and screen sizes.
    ///
    /// Returns the slot index, or `None` when the layout has no slots or the
    /// screen/drag cannot be used (nothing is learned from a number that
    /// cannot be placed).
    pub fn learn_drag(
        &mut self,
        layout: &Layout,
        screen: &Frame,
        dragged: &Frame,
    ) -> Option<usize> {
        if !screen.usable() || !dragged.usable() {
            return None;
        }
        let slots = self.slots_for(layout, screen);
        let index = nearest_slot(&slots, dragged)?;
        let region = layout.region.frame(screen);
        let learned = quantize(Slot::from_frame(dragged, &region));
        self.overrides
            .entry(layout.id)
            .or_default()
            .insert(index, learned);
        Some(index)
    }
}

/// A learned ratio rounded to the store's precision (six decimals): well
/// below a pixel at any real display size, and it makes what the memory
/// holds and what the ledger writes the same number, so a restart changes
/// nothing.
pub const LEARNED_PRECISION: f64 = 1e-6;

fn quantize(slot: Slot) -> Slot {
    // Divide (never multiply by 1e-6): the quotient is correctly rounded, so
    // the number in memory is bit-for-bit the number the store writes and
    // reads back.
    let round = |value: f64| (value * 1e6).round() / 1e6;
    Slot::new(
        round(slot.rect.x),
        round(slot.rect.y),
        round(slot.rect.w),
        round(slot.rect.h),
    )
}

/// The slot whose centre is nearest `frame`'s centre. Slot order breaks ties,
/// so the result is deterministic.
fn nearest_slot(slots: &[Frame], frame: &Frame) -> Option<usize> {
    let (cx, cy) = centre(frame);
    slots
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let (sx, sy) = centre(slot);
            (index, (sx - cx).powi(2) + (sy - cy).powi(2))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(index, _)| index)
}

fn centre(frame: &Frame) -> (f64, f64) {
    (frame.x + frame.w / 2.0, frame.y + frame.h / 2.0)
}

/// One arrangement decision: the layout used, its resolved slots, and the
/// plan over them.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    pub layout: Layout,
    /// The absolute slot frames, in slot order (learned frames substituted).
    pub slots: Vec<Frame>,
    pub plan: Plan,
}

impl Arrangement {
    /// The slot a window was given, if any.
    pub fn slot_for(&self, pid: i32, window_id: u32) -> Option<usize> {
        self.plan.slot_for(pid, window_id)
    }
}

/// Arranges `windows` (frontmost first) into the work's layout on `screen`.
///
/// `alternates` are other tiling frames the same layout could resolve on
/// (the Dock moved, Stage Manager ran): the first one with the same slot
/// count supplies the sticky pass's alternate geometry, exactly as
/// [`LayoutEngine::plan`] documents.
pub fn arrange(
    windows: &[WindowRef],
    memory: &ArrangeMemory,
    screen: &Frame,
    alternates: &[Frame],
) -> Arrangement {
    let layout = memory.layout_for(windows.len());
    let slots = memory.slots_for(&layout, screen);
    let alternate = alternates
        .iter()
        .map(|frame| memory.slots_for(&layout, frame))
        .find(|candidate| candidate.len() == slots.len())
        .unwrap_or_default();
    let plan = LayoutEngine::plan(windows, &slots, &alternate);
    Arrangement {
        layout,
        slots,
        plan,
    }
}

/// One row of the preset picker: everything the UI needs to render a choice,
/// and nothing it would have to recompute.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Choice {
    pub id: LayoutId,
    pub name: &'static str,
    /// How many windows the seed holds without minimizing.
    pub slots: usize,
    /// Whether this seed is the one the work would use right now (the pick,
    /// or the automatic choice when nothing is picked).
    pub selected: bool,
    /// Whether every live window fits.
    pub fits: bool,
}

/// The picker's rows for a work with `count` live windows, in the seeds'
/// documented order.
pub fn choices(memory: &ArrangeMemory, count: usize) -> Vec<Choice> {
    let current = memory.chosen.unwrap_or_else(|| auto_layout(count));
    LayoutId::all()
        .iter()
        .map(|id| {
            let slots = id.seed().slots().len();
            Choice {
                id: *id,
                name: id.name(),
                slots,
                selected: *id == current,
                fits: slots >= count,
            }
        })
        .collect()
}

// ─── Persistence ────────────────────────────────────────────────────────────

/// Textual, versioned, one memory per work — the same doctrine as
/// [`crate::stage::recipe_store`]: learnings are user state, so they persist
/// as text a future version can migrate rather than as an opaque blob.
pub mod store {
    use super::*;

    pub const HEADER: &str = "evo-pod-arrangements-v1";

    /// The stable text for a region (`full`, `left:0.600`, ...).
    pub fn region_key(region: Region) -> String {
        match region {
            Region::Full => "full".to_string(),
            Region::Left(fraction) => format!("left:{fraction:.3}"),
            Region::Right(fraction) => format!("right:{fraction:.3}"),
            Region::Center(fraction) => format!("center:{fraction:.3}"),
            Region::Top => "top".to_string(),
            Region::Bottom => "bottom".to_string(),
        }
    }

    /// The region a stable key names, if any.
    pub fn region_from_key(key: &str) -> Option<Region> {
        let (side, fraction) = match key.split_once(':') {
            Some((side, fraction)) => (side, Some(fraction)),
            None => (key, None),
        };
        let fraction = || fraction.and_then(|value| value.parse::<f64>().ok());
        match side {
            "full" => Some(Region::Full),
            "left" => fraction().map(Region::Left),
            "right" => fraction().map(Region::Right),
            "center" => fraction().map(Region::Center),
            "top" => Some(Region::Top),
            "bottom" => Some(Region::Bottom),
            _ => None,
        }
    }

    pub fn export(entries: &[(String, ArrangeMemory)]) -> String {
        let mut out = String::from(HEADER);
        out.push('\n');
        for (pod, memory) in entries {
            if let Some(id) = memory.chosen {
                out.push_str(&format!("{pod}\tchosen\t{}\n", id.key()));
            }
            for (id, region) in &memory.regions {
                out.push_str(&format!(
                    "{pod}\tregion\t{}\t{}\n",
                    id.key(),
                    region_key(*region)
                ));
            }
            for (id, slots) in &memory.overrides {
                for (index, slot) in slots {
                    let rect: RatioRect = slot.rect;
                    out.push_str(&format!(
                        "{pod}\tslot\t{}\t{index}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\n",
                        id.key(),
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h
                    ));
                }
            }
        }
        out
    }

    pub fn import(text: &str) -> Vec<(String, ArrangeMemory)> {
        let mut by_key: BTreeMap<String, ArrangeMemory> = BTreeMap::new();
        for line in text.lines() {
            if line == HEADER || line.trim().is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            let Some(pod) = cols.first().filter(|pod| !pod.is_empty()) else {
                continue;
            };
            // A line is parsed before it can create a work: what cannot be
            // understood is skipped, never turned into an empty memory.
            let mut chosen = None;
            let mut region = None;
            let mut slot = None;
            match cols.as_slice() {
                [_, "chosen", key] => chosen = LayoutId::from_key(key),
                [_, "region", layout, value] => {
                    if let (Some(id), Some(region_value)) =
                        (LayoutId::from_key(layout), region_from_key(value))
                    {
                        region = Some((id, region_value));
                    }
                }
                [_, "slot", layout, index, x, y, w, h] => {
                    if let (Some(id), Ok(index), Ok(x), Ok(y), Ok(w), Ok(h)) = (
                        LayoutId::from_key(layout),
                        index.parse::<usize>(),
                        x.parse::<f64>(),
                        y.parse::<f64>(),
                        w.parse::<f64>(),
                        h.parse::<f64>(),
                    ) {
                        slot = Some((id, index, Slot::new(x, y, w, h)));
                    }
                }
                _ => {}
            }
            if chosen.is_none() && region.is_none() && slot.is_none() {
                continue;
            }
            let memory = by_key.entry((*pod).to_string()).or_default();
            if let Some(id) = chosen {
                memory.chosen = Some(id);
            }
            if let Some((id, value)) = region {
                memory.regions.insert(id, value);
            }
            if let Some((id, index, value)) = slot {
                memory.overrides.entry(id).or_default().insert(index, value);
            }
        }
        by_key.into_iter().collect()
    }
}

/// Whether two arranged frames agree within the planner's sticky tolerance —
/// a convenience for callers deciding if a re-arrange would move anything.
pub fn is_settled(arrangement: &Arrangement, windows: &[WindowRef]) -> bool {
    arrangement.plan.placements.iter().all(|placement| {
        windows
            .iter()
            .find(|window| window.pid == placement.pid && window.window_id == placement.window_id)
            .is_some_and(|window| {
                frames_approx_equal(
                    &window.frame,
                    &placement.frame,
                    crate::layout::STICKY_TOLERANCE,
                )
            })
    })
}
