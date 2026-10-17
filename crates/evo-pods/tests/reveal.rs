//! Proofs for the choreographed reveal (A2): step order, stagger, distance-
//! scaled durations, exact endpoints, the instant path, and the windows a
//! reveal must leave alone.
//!
//! Everything here goes through `evo_pods`' public surface, because the point
//! of the timeline is that a host can drive it without reaching inside.

use evo_pods::layout::{LayoutEngine, LayoutId, Placement, Plan, WindowRef};
use evo_pods::reveal::{
    BASE_DURATION_MS, Curve, MAX_DURATION_SCALE, MIN_DURATION_SCALE, RevealEngine, RevealOptions,
    RevealStep, STAGGER_MS,
};
use evo_pods::surface::Frame;

// ─── Fixtures ───────────────────────────────────────────────────────────────

fn screen() -> Frame {
    Frame::new(0.0, 0.0, 1440.0, 900.0)
}

fn window(pid: i32, window_id: u32, frame: Frame) -> WindowRef {
    WindowRef {
        pid,
        window_id,
        frame,
    }
}

/// A frame far from any slot, so nothing is sticky by accident.
fn offstage(index: u32) -> Frame {
    Frame::new(4000.0 + index as f64 * 40.0, 60.0, 300.0, 200.0)
}

/// One placement's reveal, for the tests that only care about duration.
fn single_step(target: Frame, from: Frame, options: &RevealOptions) -> RevealStep {
    let plan = Plan {
        placements: vec![Placement {
            slot: 0,
            pid: 1,
            window_id: 1,
            frame: target,
        }],
        ..Default::default()
    };
    let current = vec![window(1, 1, from)];
    let reveal = RevealEngine::reveal(&plan, &current, options);
    reveal.steps[0]
}

// ─── One step ───────────────────────────────────────────────────────────────

#[test]
fn a_window_animates_from_where_it_is_to_its_slot() {
    let slots = LayoutId::Halves.seed().resolved_slots(&screen());
    let before = Frame::new(2000.0, 40.0, 300.0, 200.0);
    let windows = vec![window(1, 10, before)];
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    let reveal = RevealEngine::reveal(&plan, &windows, &RevealOptions::default());

    assert!(!reveal.instant);
    assert_eq!(reveal.steps.len(), 1);
    let step = &reveal.steps[0];
    assert_eq!(step.slot, 0);
    assert_eq!(step.from, before);
    assert_eq!(step.to, slots[0]);
    assert_eq!(step.start_ms, 0);
    assert!(step.duration_ms > 0);

    // Endpoints are exact.
    assert_eq!(step.frame_at(0), before);
    assert_eq!(step.frame_at(step.end_ms()), slots[0]);

    // Mid-travel is strictly inside the segment: cubic ease-out is monotone.
    let middle = step.frame_at(step.duration_ms / 2);
    assert!(middle.x < before.x && middle.x > slots[0].x);
    assert!(middle.w > before.w && middle.w < slots[0].w);
    assert_eq!(step.progress(0), 0.0);
    assert_eq!(step.progress(step.end_ms()), 1.0);
}

#[test]
fn a_window_already_in_its_slot_is_not_animated() {
    let slots = LayoutId::Halves.seed().resolved_slots(&screen());
    let resting = window(2, 20, slots[1]);
    let mover = window(3, 30, offstage(0));
    let windows = vec![resting, mover];
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    assert_eq!(
        plan.slot_for(2, 20),
        Some(1),
        "the resting window is sticky"
    );

    let reveal = RevealEngine::reveal(&plan, &windows, &RevealOptions::default());
    assert_eq!(reveal.steps.len(), 1, "only the mover is choreographed");
    assert_eq!(reveal.steps[0].window_id, 30);
    assert_eq!(reveal.steps[0].slot, 0);

    // Everything already placed: nothing to animate, nothing to wait for.
    let settled: Vec<WindowRef> = slots
        .iter()
        .enumerate()
        .map(|(index, frame)| window(9, index as u32, *frame))
        .collect();
    let plan = LayoutEngine::plan(&settled, &slots, &[]);
    let reveal = RevealEngine::reveal(&plan, &settled, &RevealOptions::default());
    assert!(reveal.is_empty());
    assert_eq!(reveal.duration_ms(), 0);
    assert!(reveal.is_complete(0));
}

#[test]
fn a_window_the_caller_never_described_is_placed_at_once() {
    let target = LayoutId::Full.seed().resolved_slots(&screen())[0];
    let plan = Plan {
        placements: vec![Placement {
            slot: 0,
            pid: 7,
            window_id: 70,
            frame: target,
        }],
        ..Default::default()
    };
    // No current frames: the planner's target is all we know.
    let reveal = RevealEngine::reveal(&plan, &[], &RevealOptions::default());
    assert_eq!(reveal.steps.len(), 1);
    assert_eq!(reveal.steps[0].from, target);
    assert_eq!(reveal.steps[0].to, target);
    assert_eq!(reveal.steps[0].duration_ms, 0);
    assert!(reveal.steps[0].is_static());
    assert!(reveal.is_complete(0), "nothing to wait for");
    assert!(!reveal.instant, "one unknown window is not instant mode");
}

// ─── Order and stagger ──────────────────────────────────────────────────────

#[test]
fn stagger_follows_slot_order_and_the_main_slot_leads() {
    let slots = LayoutId::Quads.seed().resolved_slots(&screen());
    // Deliberately z-order-shuffled: the front window sits in slot 2.
    let plan = Plan {
        placements: vec![
            Placement {
                slot: 2,
                pid: 1,
                window_id: 10,
                frame: slots[2],
            },
            Placement {
                slot: 0,
                pid: 1,
                window_id: 11,
                frame: slots[0],
            },
            Placement {
                slot: 1,
                pid: 1,
                window_id: 12,
                frame: slots[1],
            },
        ],
        to_minimize: Vec::new(),
        empty_slots: vec![3],
    };
    let current = vec![
        window(1, 10, offstage(0)),
        window(1, 11, offstage(1)),
        window(1, 12, offstage(2)),
    ];

    let reveal = RevealEngine::reveal(&plan, &current, &RevealOptions::default());
    assert_eq!(
        reveal
            .steps
            .iter()
            .map(|step| (step.slot, step.window_id))
            .collect::<Vec<_>>(),
        vec![(0, 11), (1, 12), (2, 10)],
        "steps start in slot order, not z-order"
    );
    assert_eq!(
        reveal
            .steps
            .iter()
            .map(|step| step.start_ms)
            .collect::<Vec<_>>(),
        vec![0, STAGGER_MS, 2 * STAGGER_MS]
    );

    let slower = RevealOptions {
        stagger_ms: 100,
        ..RevealOptions::default()
    };
    let reveal = RevealEngine::reveal(&plan, &current, &slower);
    assert_eq!(
        reveal
            .steps
            .iter()
            .map(|step| step.start_ms)
            .collect::<Vec<_>>(),
        vec![0, 100, 200]
    );
    assert_eq!(
        reveal.duration_ms(),
        reveal.steps.iter().map(RevealStep::end_ms).max().unwrap()
    );
}

// ─── Duration ───────────────────────────────────────────────────────────────

#[test]
fn duration_scales_with_distance_and_stays_in_its_band() {
    let target = LayoutId::Halves.seed().resolved_slots(&screen())[0];
    let options = RevealOptions::default();
    let floor = (BASE_DURATION_MS as f64 * MIN_DURATION_SCALE).round() as u64;
    let ceiling = (BASE_DURATION_MS as f64 * MAX_DURATION_SCALE).round() as u64;

    let near = single_step(
        target,
        Frame::new(target.x + 30.0, target.y, target.w, target.h),
        &options,
    );
    let middle = single_step(
        target,
        Frame::new(target.x + 600.0, target.y, target.w, target.h),
        &options,
    );
    let far = single_step(
        target,
        Frame::new(target.x + 2000.0, target.y, target.w, target.h),
        &options,
    );

    for step in [near, middle, far] {
        assert!(
            (floor..=ceiling).contains(&step.duration_ms),
            "{} ms outside the {floor}..={ceiling} ms band",
            step.duration_ms
        );
    }
    assert!(near.duration_ms < middle.duration_ms);
    assert!(middle.duration_ms < far.duration_ms);
    assert_eq!(near.duration_ms, floor, "a short hop runs at the floor");
    assert_eq!(
        far.duration_ms, ceiling,
        "a cross-desktop trip runs at the cap"
    );
    assert_eq!(
        middle.duration_ms, BASE_DURATION_MS,
        "600 pt is the reference"
    );

    // A long vertical trip counts as well as a horizontal one.
    let vertical = single_step(
        target,
        Frame::new(target.x, target.y + 2000.0, target.w, target.h),
        &options,
    );
    assert_eq!(vertical.duration_ms, ceiling);
}

// ─── Instant path ───────────────────────────────────────────────────────────

#[test]
fn seven_windows_place_instantly_while_six_are_choreographed() {
    let slots = LayoutId::HeroRails.seed().resolved_slots(&screen());
    assert!(slots.len() >= 7);
    let windows: Vec<WindowRef> = (0..7)
        .map(|index| window(1, index, offstage(index)))
        .collect();
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    assert_eq!(plan.placements.len(), 7);

    let reveal = RevealEngine::reveal(&plan, &windows, &RevealOptions::default());
    assert!(reveal.instant, "seven windows land together");
    assert_eq!(reveal.duration_ms(), 0);
    assert!(reveal.is_complete(0));
    assert!(
        reveal
            .steps
            .iter()
            .all(|step| step.start_ms == 0 && step.duration_ms == 0)
    );
    let at_once = reveal.frames_at(0);
    for (step, frame) in reveal.steps.iter().zip(at_once) {
        assert_eq!(frame, step.to, "instant means the target, immediately");
    }

    let six = &windows[..6];
    let plan = LayoutEngine::plan(six, &slots, &[]);
    let reveal = RevealEngine::reveal(&plan, six, &RevealOptions::default());
    assert!(!reveal.instant);
    assert!(reveal.duration_ms() > 0);
    assert_eq!(reveal.steps.len(), 6);
}

#[test]
fn reduced_motion_places_everything_at_once() {
    let slots = LayoutId::Quads.seed().resolved_slots(&screen());
    let windows: Vec<WindowRef> = (0..3)
        .map(|index| window(1, index, offstage(index)))
        .collect();
    let plan = LayoutEngine::plan(&windows, &slots, &[]);

    let reveal = RevealEngine::reveal(
        &plan,
        &windows,
        &RevealOptions {
            reduced_motion: true,
            ..RevealOptions::default()
        },
    );
    assert!(reveal.instant);
    assert_eq!(reveal.duration_ms(), 0);
    assert!(reveal.is_complete(0));
    for (step, frame) in reveal.steps.iter().zip(reveal.frames_at(0)) {
        assert_eq!(frame, step.to);
    }
}

#[test]
fn the_threshold_and_base_duration_are_the_caller_s_choice() {
    let slots = LayoutId::Quads.seed().resolved_slots(&screen());
    let windows: Vec<WindowRef> = (0..3)
        .map(|index| window(1, index, offstage(index)))
        .collect();
    let plan = LayoutEngine::plan(&windows, &slots, &[]);

    let reveal = RevealEngine::reveal(
        &plan,
        &windows,
        &RevealOptions {
            instant_threshold: 3,
            ..RevealOptions::default()
        },
    );
    assert!(
        reveal.instant,
        "three placements reach a threshold of three"
    );

    let reveal = RevealEngine::reveal(
        &plan,
        &windows,
        &RevealOptions {
            base_duration_ms: 1000,
            ..RevealOptions::default()
        },
    );
    assert!(!reveal.instant);
    assert!(
        reveal.duration_ms() >= 1000,
        "the base duration is honoured"
    );
}

// ─── Sampling ───────────────────────────────────────────────────────────────

#[test]
fn sampling_is_aligned_with_steps_and_completes_exactly() {
    let slots = LayoutId::Quads.seed().resolved_slots(&screen());
    let windows: Vec<WindowRef> = (0..3)
        .map(|index| window(4, index, offstage(index)))
        .collect();
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    let reveal = RevealEngine::reveal(&plan, &windows, &RevealOptions::default());
    assert_eq!(reveal.steps.len(), 3);

    let start = reveal.frames_at(0);
    assert_eq!(
        start,
        reveal
            .steps
            .iter()
            .map(|step| step.from)
            .collect::<Vec<_>>(),
        "frame 0 is exactly where every window started"
    );
    let end = reveal.frames_at(reveal.duration_ms());
    assert_eq!(
        end,
        reveal.steps.iter().map(|step| step.to).collect::<Vec<_>>(),
        "the last frame is exactly the plan's target"
    );
    assert!(!reveal.is_complete(reveal.duration_ms() - 1));
    assert!(reveal.is_complete(reveal.duration_ms()));
    assert!(reveal.is_complete(reveal.duration_ms() + 10_000));

    let second = &reveal.steps[1];
    assert_eq!(
        second.frame_at(0),
        second.from,
        "a waiting window has not moved"
    );
    assert_eq!(
        second.frame_at(second.start_ms),
        second.from,
        "a step starts exactly where the window was"
    );

    // No frame leaves the rectangles it is travelling between: cubic
    // ease-out never overshoots.
    for step in &reveal.steps {
        for tick in 0..=step.end_ms() {
            let frame = step.frame_at(tick);
            let span =
                |a: f64, b: f64, value: f64| value >= a.min(b) - 1e-9 && value <= a.max(b) + 1e-9;
            assert!(span(step.from.x, step.to.x, frame.x));
            assert!(span(step.from.y, step.to.y, frame.y));
            assert!(span(step.from.w, step.to.w, frame.w));
            assert!(span(step.from.h, step.to.h, frame.h));
        }
    }
}

#[test]
fn an_empty_plan_reveals_nothing() {
    let reveal = RevealEngine::reveal(&Plan::default(), &[], &RevealOptions::default());
    assert!(reveal.is_empty());
    assert!(!reveal.instant);
    assert_eq!(reveal.duration_ms(), 0);
    assert!(reveal.frames_at(0).is_empty());
    assert!(reveal.is_complete(0));
}

#[test]
fn overflow_windows_are_not_choreographed() {
    let slots = LayoutId::Halves.seed().resolved_slots(&screen());
    let windows: Vec<WindowRef> = (0..4)
        .map(|index| window(1, index, offstage(index)))
        .collect();
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    assert_eq!(plan.placements.len(), 2);
    assert_eq!(plan.to_minimize.len(), 2);

    let reveal = RevealEngine::reveal(&plan, &windows, &RevealOptions::default());
    assert_eq!(reveal.steps.len(), 2, "minimized windows do not move");
    assert!(reveal.steps.iter().all(|step| {
        plan.to_minimize
            .iter()
            .all(|window| window.window_id != step.window_id)
    }));
}

// ─── Curves ─────────────────────────────────────────────────────────────────

#[test]
fn curves_hit_their_endpoints_and_stay_sane() {
    for curve in [Curve::Linear, Curve::CubicEaseOut, Curve::Damped] {
        assert_eq!(curve.sample(0.0), 0.0, "{curve:?} start");
        assert_eq!(curve.sample(1.0), 1.0, "{curve:?} end is exact");
        assert_eq!(curve.sample(f64::NAN), 0.0, "{curve:?} NaN");
        assert_eq!(curve.sample(-2.0), 0.0, "{curve:?} below range");
        assert_eq!(curve.sample(2.0), 1.0, "{curve:?} above range");
    }

    assert_eq!(Curve::Linear.sample(0.5), 0.5);
    assert_eq!(Curve::CubicEaseOut.sample(0.5), 0.875);
    assert!(
        Curve::CubicEaseOut.sample(0.25) > 0.25,
        "ease-out is front-loaded"
    );
    for tick in 0..100 {
        let a = Curve::CubicEaseOut.sample(tick as f64 / 100.0);
        let b = Curve::CubicEaseOut.sample((tick + 1) as f64 / 100.0);
        assert!(b >= a - 1e-12, "cubic ease-out must not go backwards");
    }

    // The damped curve is Setpiece's weight: a small overshoot, then settled.
    let mut maximum: f64 = 0.0;
    let mut minimum: f64 = 0.0;
    for tick in 0..=200 {
        let value = Curve::Damped.sample(tick as f64 / 200.0);
        maximum = maximum.max(value);
        minimum = minimum.min(value);
    }
    assert!(maximum > 1.0, "the damped curve overshoots");
    assert!(maximum < 1.15, "the overshoot stays bounded: {maximum}");
    assert!(
        minimum >= 0.0,
        "the damped curve never starts early: {minimum}"
    );
}

#[test]
fn the_default_curve_is_the_documented_one() {
    let options = RevealOptions::default();
    assert_eq!(options.curve, Curve::CubicEaseOut);
    assert_eq!(options.base_duration_ms, BASE_DURATION_MS);
    assert_eq!(options.stagger_ms, STAGGER_MS);
    assert!(!options.reduced_motion);
    assert_eq!(Curve::default(), Curve::CubicEaseOut);

    // A caller's curve reaches every step.
    let slots = LayoutId::Halves.seed().resolved_slots(&screen());
    let windows = vec![window(1, 0, offstage(0))];
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    let reveal = RevealEngine::reveal(
        &plan,
        &windows,
        &RevealOptions {
            curve: Curve::Linear,
            ..RevealOptions::default()
        },
    );
    assert_eq!(reveal.steps[0].curve, Curve::Linear);
}
