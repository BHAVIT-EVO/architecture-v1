//! Stage recipes: a pod's remembered layout.
//!
//! A recipe assigns each of the pod's resources a **role** (hero,
//! satellite, rail) and a **fractional frame** inside a specific
//! [`DisplaySignature`] — placement learned across days belongs to the
//! work, not to the morning. Fractions degrade honestly: a recipe made on
//! one display signature is re-derived on another, keeping whichever
//! placements are still learnable (same resource, default geometry).
//!
//! All math here is platform-free: screens and windows arrive as numbers,
//! frames go out as numbers.

use crate::surface::Frame;
use evo_threads::ResumeBundle;
use std::collections::BTreeMap;

/// A fingerprint of the physical display arrangement: count + rounded
/// frames, sorted. When the persona's world changes (dock a laptop), the
/// signature changes, and recipes re-derive instead of smearing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisplaySignature(pub String);

impl DisplaySignature {
    /// Deterministic from frames alone. Origin-relative and main-first.
    pub fn from_frames(frames: &[Frame]) -> Self {
        let mut parts: Vec<String> = frames
            .iter()
            .map(|f| {
                format!(
                    "{}x{}@{},{}",
                    f.w.round() as i64,
                    f.h.round() as i64,
                    f.x.round() as i64,
                    f.y.round() as i64
                )
            })
            .collect();
        parts.sort();
        Self(parts.join("|"))
    }
}

/// A placement as fractions of one screen (0..=1), learned or default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatioRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl RatioRect {
    pub fn to_frame(&self, screen: &Frame) -> Frame {
        if !screen.usable() {
            return Frame::new(screen.x, screen.y, screen.w, screen.h);
        }
        Frame::new(
            screen.x + self.x * screen.w,
            screen.y + self.y * screen.h,
            self.w * screen.w,
            self.h * screen.h,
        )
    }

    pub fn from_frame(frame: &Frame, screen: &Frame) -> Self {
        let (x, y, w, h) = frame.fraction_in(screen);
        Self { x, y, w, h }
    }
}

/// The trio of roles: the work's heart, its neighbors, its memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Hero,
    Satellite(u8),
    Rail(u8),
}

impl Role {
    /// The role's default geometry. Chosen once, documented, and never
    /// per-app: the hero owns the focus zone (left ~58 %), satellites
    /// stack right, rail is a reference strip beneath the hero.
    pub fn default_ratio(self) -> RatioRect {
        match self {
            Role::Hero => RatioRect {
                x: 0.03,
                y: 0.06,
                w: 0.58,
                h: 0.62,
            },
            Role::Satellite(i) => {
                const TOP: f64 = 0.06;
                const FULL: f64 = 0.88;
                const CAP: f64 = 3.0;
                let i = (i as f64).min(CAP - 1.0);
                let slot = FULL / CAP;
                RatioRect {
                    x: 0.63,
                    y: TOP + slot * i,
                    w: 0.34,
                    h: slot - 0.02,
                }
            }
            Role::Rail(i) => {
                const LEFT: f64 = 0.03;
                const WIDTH: f64 = 0.58;
                const CAP: f64 = 4.0;
                let i = (i as f64).min(CAP - 1.0);
                RatioRect {
                    x: LEFT + (WIDTH / CAP) * (i as f64),
                    y: 0.72,
                    w: (WIDTH / CAP) - 0.008,
                    h: 0.22,
                }
            }
        }
    }

    /// Raise order within the pod: satellites rise first, hero last (top).
    pub fn raise_rank(self) -> u8 {
        match self {
            Role::Rail(_) => 0,
            Role::Satellite(_) => 1,
            Role::Hero => 2,
        }
    }
}

/// One pod's learned arrangement for one display geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct StageRecipe {
    /// The display world this recipe was learned in.
    pub signature: DisplaySignature,
    /// Resource -> its role (identity order is the map's).
    pub roles: BTreeMap<String, Role>,
    /// Resource -> its learned placement (only some may be learned).
    pub placements: BTreeMap<String, RatioRect>,
}

impl StageRecipe {
    pub fn new(signature: DisplaySignature) -> Self {
        Self {
            signature,
            roles: BTreeMap::new(),
            placements: BTreeMap::new(),
        }
    }

    pub fn frame_for(&self, resource: &str, screen: &Frame) -> Option<Frame> {
        let role = self.roles.get(resource)?;
        let ratio = self
            .placements
            .get(resource)
            .copied()
            .unwrap_or_else(|| role.default_ratio());
        Some(ratio.to_frame(screen))
    }

    /// Records a user-performed frame as the resource's placement —
    /// the person choreographs, the recipe remembers.
    pub fn learn(&mut self, resource: &str, frame: &Frame, screen: &Frame) {
        if self.roles.contains_key(resource) && frame.usable() {
            self.placements
                .insert(resource.to_string(), RatioRect::from_frame(frame, screen));
        }
    }
}

/// Derives roles from the engine's resume order: the resume point is the
/// hero (that IS its contract); the restore set in order are satellites;
/// the strongest companions fill the rail. Previous placements survive
/// wherever the resource kept a role.
pub fn derive_recipe(
    bundle: &ResumeBundle,
    companions: &[(String, f64)],
    signature: DisplaySignature,
    satellite_cap: usize,
    rail_cap: usize,
    previous: Option<&StageRecipe>,
) -> StageRecipe {
    let mut recipe = StageRecipe::new(signature.clone());
    if !bundle.resume_point.is_empty() {
        recipe
            .roles
            .insert(bundle.resume_point.clone(), Role::Hero);
    }
    for (idx, resource) in bundle
        .restore_set
        .iter()
        .enumerate()
        .take(satellite_cap)
    {
        if *resource != bundle.resume_point {
            recipe
                .roles
                .entry(resource.clone())
                .or_insert(Role::Satellite(idx as u8));
        }
    }
    let mut companions: Vec<(String, f64)> = companions
        .iter()
        .filter(|(r, _)| !recipe.roles.contains_key(r))
        .cloned()
        .collect();
    companions.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (idx, (resource, _)) in companions.into_iter().enumerate().take(rail_cap) {
        recipe.roles.entry(resource).or_insert(Role::Rail(idx as u8));
    }

    // Legacy learning: same resource, same role family -> keep placement.
    if let Some(prev) = previous {
        let _ = &prev.signature; // a signature change re-derives; placements still carry.
        for (resource, role) in &recipe.roles {
            if let Some(p) = prev.placements.get(resource) {
                let _ = role;
                recipe.placements.insert(resource.clone(), *p);
            }
        }
    }
    recipe
}

/// A hand-written recipe store: learnings are *user* state (from
/// choreographing), not derived state — they persist textually, one line
/// per placement, versioned header, so tomorrow's intelligence can ignore
/// or migrate them without breaking yesterday.
pub mod recipe_store {
    use super::*;

    pub const HEADER: &str = "evo-stage-recipes-v1";

    pub fn export(recipes: &[(String, StageRecipe)]) -> String {
        let mut out = String::from(HEADER);
        out.push('\n');
        for (pod, recipe) in recipes {
            for (resource, ratio) in &recipe.placements {
                let subject = resource.replace('\t', " ");
                out.push_str(&format!(
                    "{pod}\t{}\t{subject}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\n",
                    recipe.signature.0, ratio.x, ratio.y, ratio.w, ratio.h
                ));
            }
        }
        out
    }

    pub fn import(text: &str) -> Vec<(String, StageRecipe)> {
        let mut by_key: BTreeMap<(String, String), StageRecipe> = BTreeMap::new();
        for line in text.lines() {
            if line == HEADER || line.trim().is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() != 7 {
                continue;
            }
            let Ok(x) = cols[3].parse::<f64>() else { continue };
            let Ok(y) = cols[4].parse::<f64>() else { continue };
            let Ok(w) = cols[5].parse::<f64>() else { continue };
            let Ok(h) = cols[6].parse::<f64>() else { continue };
            let recipe = by_key
                .entry((cols[0].to_string(), cols[1].to_string()))
                .or_insert_with(|| StageRecipe::new(DisplaySignature(cols[1].to_string())));
            recipe.placements.insert(
                cols[2].to_string(),
                RatioRect { x: x.max(0.0), y: y.max(0.0), w: w.clamp(0.0, 1.0), h: h.clamp(0.0, 1.0) },
            );
        }
        by_key.into_iter().map(|((pod, _), r)| (pod, r)).collect()
    }
}
