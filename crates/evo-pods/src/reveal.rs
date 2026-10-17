//! Choreography: how a planned arrangement *arrives*.
//!
//! [`crate::layout`] decides *where* a work's windows go. This module decides
//! *in what order and over what time* they get there, so a six-window work
//! reads as one gesture instead of six rectangles jumping at once, and so a
//! window already sitting where it belongs never flashes.
//!
//! Adapted from **Setpiece** (<https://github.com/ChiFungHillmanChan/setpiece>,
//! MIT, Copyright (c) 2026 Hillman Chan — see `THIRD-PARTY-NOTICES.md`): its
//! `AnimationRunner` / `FrameInterpolator` model — per-window duration scaled
//! by `sqrt(distance / 600pt)` clamped to 70–140 %, its easing set (linear,
//! cubic ease-out, and the damped `1 − e^(−6t)·cos(8t)`), and its rule that a
//! work of seven or more windows is placed instantly because a staggered
//! animation at that size is noise, not choreography.
//!
//! Everything here is a decision over numbers: no OS calls, no clocks. The
//! host asks for [`Reveal::frames_at`] on each frame it draws and books every
//! window move itself.
//!
//! # The laws
//!
//! 1. **Order is the layout's own.** Steps start in slot order (the order
//!    [`crate::layout::Layout::slots`] plans into), never in z-order: the
//!    work's main slot leads.
//! 2. **A placement already reached is not animated.** A window within
//!    [`crate::layout::STICKY_TOLERANCE`] of its target adds no motion.
//! 3. **Endpoints are exact.** Sampling at or after a step's end returns the
//!    target frame bit-for-bit, so an interrupted host lands where the plan
//!    said, not one float away.
//! 4. **Reduced motion and big works win.** Either one collapses the whole
//!    reveal to zero duration; the decision to skip animation is never a
//!    per-window exception the host has to re-litigate.

use crate::layout::{Placement, Plan, STICKY_TOLERANCE, WindowRef, frames_approx_equal};
use crate::surface::Frame;

/// The distance at which a move takes its base duration. Setpiece's reference
/// travel, in points.
pub const REFERENCE_DISTANCE: f64 = 600.0;

/// Evo's base travel time before distance scaling.
pub const BASE_DURATION_MS: u64 = 320;

/// How much later each subsequent slot starts. Small enough to read as one
/// wave, long enough that the window order is legible.
pub const STAGGER_MS: u64 = 45;

/// At this many placements the whole work lands at once (Setpiece's own
/// threshold: seven windows are placed instantly).
pub const INSTANT_THRESHOLD: usize = 7;

/// The fastest a move may be relative to [`BASE_DURATION_MS`].
pub const MIN_DURATION_SCALE: f64 = 0.7;

/// The slowest a move may be relative to [`BASE_DURATION_MS`].
pub const MAX_DURATION_SCALE: f64 = 1.4;

// ─── Easing ─────────────────────────────────────────────────────────────────

/// The easing curves a reveal may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Curve {
    /// Constant speed. Honest for a window sliding between two near slots.
    Linear,
    /// Fast out, settled in — the default for a window changing slot.
    #[default]
    CubicEaseOut,
    /// Setpiece's damped spring approximation, `1 − e^(−6t)·cos(8t)`: a
    /// slight overshoot that reads as weight, pinned to exactly `1` at the
    /// end so a finished step lands on the plan's own frame. Used where a
    /// settle is wanted.
    Damped,
}

impl Curve {
    /// The curve's value at `t`, clamped into `0..=1`; a non-finite `t`
    /// samples at `0` rather than poisoning a frame.
    pub fn sample(self, t: f64) -> f64 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match self {
            Curve::Linear => t,
            Curve::CubicEaseOut => 1.0 - (1.0 - t).powi(3),
            // The formula lands a hair past 1 at t = 1; the endpoint is a
            // contract, so it is pinned rather than left 0.04 % adrift.
            Curve::Damped if t < 1.0 => 1.0 - (-6.0 * t).exp() * (8.0 * t).cos(),
            Curve::Damped => 1.0,
        }
    }
}

// ─── Options ────────────────────────────────────────────────────────────────

/// The reveal's tuning. The defaults are the documented grammar; a host that
/// honours the person's own motion preference sets [`Self::reduced_motion`]
/// instead of inventing a second code path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RevealOptions {
    /// The base travel time, before distance scaling.
    pub base_duration_ms: u64,
    /// The delay between consecutive slots.
    pub stagger_ms: u64,
    /// At this many placements everything lands at once.
    pub instant_threshold: usize,
    /// The person asked for less motion: every step is instantaneous.
    pub reduced_motion: bool,
    /// The easing every animated step uses.
    pub curve: Curve,
}

impl Default for RevealOptions {
    fn default() -> Self {
        Self {
            base_duration_ms: BASE_DURATION_MS,
            stagger_ms: STAGGER_MS,
            instant_threshold: INSTANT_THRESHOLD,
            reduced_motion: false,
            curve: Curve::CubicEaseOut,
        }
    }
}

// ─── Steps ──────────────────────────────────────────────────────────────────

/// One window's journey: where it is, where the plan puts it, and when.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RevealStep {
    /// The slot this step fills, in the layout's own order.
    pub slot: usize,
    pub pid: i32,
    pub window_id: u32,
    /// The frame the window was in when the reveal was made.
    pub from: Frame,
    /// The frame the plan asks for — exactly, at the step's end.
    pub to: Frame,
    /// Milliseconds after the reveal starts before this step begins.
    pub start_ms: u64,
    /// How long the step takes. Zero means "already there" (or instant mode).
    pub duration_ms: u64,
    pub curve: Curve,
}

impl RevealStep {
    /// When this step finishes, relative to the reveal's start.
    pub fn end_ms(&self) -> u64 {
        self.start_ms.saturating_add(self.duration_ms)
    }

    /// Whether this step asks for no motion at all.
    pub fn is_static(&self) -> bool {
        self.duration_ms == 0 || self.from == self.to
    }

    /// The eased progress at `elapsed_ms`: `0` before the start, `1` at or
    /// after the end, and the curve's own value in between.
    pub fn progress(&self, elapsed_ms: u64) -> f64 {
        if self.duration_ms == 0 || elapsed_ms >= self.end_ms() {
            return 1.0;
        }
        if elapsed_ms <= self.start_ms {
            return 0.0;
        }
        self.curve
            .sample((elapsed_ms - self.start_ms) as f64 / self.duration_ms as f64)
    }

    /// The window's frame at `elapsed_ms`. Endpoints are returned exactly, so
    /// the last sample is the plan's own target.
    pub fn frame_at(&self, elapsed_ms: u64) -> Frame {
        let progress = self.progress(elapsed_ms);
        if progress >= 1.0 {
            return self.to;
        }
        if progress <= 0.0 {
            return self.from;
        }
        self.lerp_at(progress)
    }

    fn lerp_at(&self, progress: f64) -> Frame {
        Frame::new(
            self.from.x + (self.to.x - self.from.x) * progress,
            self.from.y + (self.to.y - self.from.y) * progress,
            self.from.w + (self.to.w - self.from.w) * progress,
            self.from.h + (self.to.h - self.from.h) * progress,
        )
    }
}

// ─── Reveal ─────────────────────────────────────────────────────────────────

/// A whole work's arrival: the steps, in slot order, and whether the batch
/// was placed all at once.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reveal {
    /// Steps in slot order; a static step is a window that needed no motion
    /// (or that the caller never described).
    pub steps: Vec<RevealStep>,
    /// The batch decision: reduced motion, or a work at/over the threshold.
    pub instant: bool,
}

impl Reveal {
    /// The total time the reveal takes: the latest step end, or `0`.
    pub fn duration_ms(&self) -> u64 {
        self.steps.iter().map(RevealStep::end_ms).max().unwrap_or(0)
    }

    /// Whether there is nothing to do.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Every step's frame at `elapsed_ms`, aligned with [`Self::steps`].
    pub fn frames_at(&self, elapsed_ms: u64) -> Vec<Frame> {
        self.steps
            .iter()
            .map(|step| step.frame_at(elapsed_ms))
            .collect()
    }

    /// Whether `elapsed_ms` is at or past the reveal's end.
    pub fn is_complete(&self, elapsed_ms: u64) -> bool {
        elapsed_ms >= self.duration_ms()
    }
}

// ─── Engine ─────────────────────────────────────────────────────────────────

/// Turns a [`Plan`] into a [`Reveal`].
pub struct RevealEngine;

impl RevealEngine {
    /// Choreograph `plan` against the windows' `current` frames.
    ///
    /// * A placement whose window is already within
    ///   [`STICKY_TOLERANCE`] of its target adds no step: nothing moves, so
    ///   nothing should animate.
    /// * A placement with no matching [`WindowRef`] is placed at its target
    ///   immediately (a static step): inventing a start frame would animate a
    ///   window from nowhere.
    /// * Animated steps start in slot order, one [`RevealOptions::stagger_ms`]
    ///   apart, and take `base × sqrt(distance / 600pt)` clamped to 70–140 %.
    /// * `reduced_motion`, or a plan at/over `instant_threshold` placements,
    ///   collapses every step to zero duration and zero delay.
    pub fn reveal(plan: &Plan, current: &[WindowRef], options: &RevealOptions) -> Reveal {
        let instant = options.reduced_motion || plan.placements.len() >= options.instant_threshold;

        let mut ordered: Vec<&Placement> = plan.placements.iter().collect();
        ordered.sort_by_key(|placement| (placement.slot, placement.pid, placement.window_id));

        let mut steps = Vec::with_capacity(ordered.len());
        let mut animated = 0u64;
        for placement in ordered {
            let Some(current) = current.iter().find(|window| {
                window.pid == placement.pid && window.window_id == placement.window_id
            }) else {
                steps.push(static_step(placement, placement.frame, options.curve));
                continue;
            };
            if frames_approx_equal(&current.frame, &placement.frame, STICKY_TOLERANCE) {
                continue;
            }
            let (start_ms, duration_ms) = if instant {
                (0, 0)
            } else {
                let start_ms = animated.saturating_mul(options.stagger_ms);
                animated += 1;
                (
                    start_ms,
                    duration_for(&current.frame, &placement.frame, options.base_duration_ms),
                )
            };
            steps.push(RevealStep {
                slot: placement.slot,
                pid: placement.pid,
                window_id: placement.window_id,
                from: current.frame,
                to: placement.frame,
                start_ms,
                duration_ms,
                curve: options.curve,
            });
        }

        Reveal { steps, instant }
    }
}

/// A step that asks for no motion, from `from` to the placement's target.
fn static_step(placement: &Placement, from: Frame, curve: Curve) -> RevealStep {
    RevealStep {
        slot: placement.slot,
        pid: placement.pid,
        window_id: placement.window_id,
        from,
        to: placement.frame,
        start_ms: 0,
        duration_ms: 0,
        curve,
    }
}

/// The travel time for one move: `base × sqrt(distance / 600pt)`, clamped to
/// [`MIN_DURATION_SCALE`]..=[`MAX_DURATION_SCALE`]. Distance is measured
/// between the windows' origins — the part of a move a person actually tracks.
fn duration_for(from: &Frame, to: &Frame, base_ms: u64) -> u64 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let distance = (dx * dx + dy * dy).sqrt();
    let scale = if distance.is_finite() {
        (distance / REFERENCE_DISTANCE)
            .sqrt()
            .clamp(MIN_DURATION_SCALE, MAX_DURATION_SCALE)
    } else {
        MIN_DURATION_SCALE
    };
    (base_ms as f64 * scale).round() as u64
}
