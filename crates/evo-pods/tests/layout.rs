//! Proofs for the unified work-space layout core (A1): regions, slots, the
//! twelve seeds, split trees with Setpiece's JSON shape, and the
//! sticky-then-fill planner.
//!
//! Everything here goes through the crate's public surface, because the
//! point of the core is that a host can plan a work's windows without
//! reaching into it.

use evo_pods::layout::{
    DEFAULT_BAND, DEFAULT_CENTER, DEFAULT_RATIO, DEFAULT_SIDE, LayoutEngine, LayoutId, LayoutNode,
    MAX_RATIO, MIN_RATIO, MIN_REGION_FRACTION, MIN_SLOT_SIDE, Plan, Region, STICKY_TOLERANCE, Slot,
    WindowRef, frames_approx_equal, hero_rails_slots, stack_slots,
};
use evo_pods::stage::RatioRect;
use evo_pods::stage::Role;
use evo_pods::surface::Frame;

// ─── Fixtures ───────────────────────────────────────────────────────────────

/// The screen most of these proofs plan on: the machine's own usable frame.
fn screen() -> Frame {
    Frame::new(0.0, 0.0, 1440.0, 900.0)
}

/// A screen that is not at the origin, to catch origin-relative mistakes.
fn offset_screen() -> Frame {
    Frame::new(-1920.0, 120.0, 1920.0, 1080.0)
}

fn window(pid: i32, window_id: u32, frame: Frame) -> WindowRef {
    WindowRef {
        pid,
        window_id,
        frame,
    }
}

fn ratio(x: f64, y: f64, w: f64, h: f64) -> RatioRect {
    RatioRect { x, y, w, h }
}

/// Every region worth resolving, including the hostile ones a stale
/// preference file can produce.
fn regions() -> Vec<Region> {
    vec![
        Region::Full,
        Region::left(),
        Region::right(),
        Region::center(),
        Region::Top,
        Region::Bottom,
        Region::Left(DEFAULT_SIDE),
        Region::Right(DEFAULT_SIDE),
        Region::Center(DEFAULT_CENTER),
        Region::Left(1.5),
        Region::Right(-3.0),
        Region::Center(0.0),
        Region::Left(f64::NAN),
        Region::Right(f64::INFINITY),
        Region::Center(f64::NEG_INFINITY),
    ]
}

fn epsilon() -> f64 {
    1e-9
}

// ─── Regions ────────────────────────────────────────────────────────────────

#[test]
fn every_region_resolves_inside_its_usable_frame() {
    for screen in [screen(), offset_screen(), Frame::new(5.0, 5.0, 300.0, 200.0)] {
        for region in regions() {
            let frame = region.frame(&screen);
            assert!(
                frame.x >= screen.x - epsilon() && frame.y >= screen.y - epsilon(),
                "{region:?} escaped the screen's origin"
            );
            assert!(
                frame.x + frame.w <= screen.x + screen.w + epsilon()
                    && frame.y + frame.h <= screen.y + screen.h + epsilon(),
                "{region:?} escaped the screen's extent"
            );
            assert!(frame.usable(), "{region:?} resolved to an unusable rect");
        }
    }
}

#[test]
fn region_defaults_and_clamps_are_the_documented_ones() {
    let screen = screen();
    assert_eq!(Region::Full.frame(&screen), screen);

    let left = Region::left().frame(&screen);
    assert_eq!(left.x, screen.x);
    assert_eq!(left.w, screen.w * DEFAULT_SIDE);
    assert_eq!(left.h, screen.h);

    let right = Region::right().frame(&screen);
    assert_eq!(right.x + right.w, screen.x + screen.w);
    assert_eq!(right.w, screen.w * DEFAULT_SIDE);

    let center = Region::center().frame(&screen);
    assert_eq!(center.w, screen.w * DEFAULT_CENTER);
    assert_eq!(
        center.x - screen.x,
        (screen.w - center.w) / 2.0,
        "a centred region is centred"
    );

    assert_eq!(Region::Top.frame(&screen).h, screen.h * DEFAULT_BAND);
    let bottom = Region::Bottom.frame(&screen);
    assert_eq!(bottom.y + bottom.h, screen.y + screen.h);
    assert_eq!(bottom.h, screen.h * DEFAULT_BAND);

    // Hostile fractions: clamped, never a sliver and never a NaN.
    assert_eq!(Region::Left(1.5).fraction(), 1.0);
    assert_eq!(Region::Left(1.5).frame(&screen), Region::Full.frame(&screen));
    assert_eq!(Region::Left(0.0).fraction(), MIN_REGION_FRACTION);
    assert_eq!(Region::Right(-3.0).fraction(), MIN_REGION_FRACTION);
    for hostile in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(Region::Left(hostile).fraction(), DEFAULT_SIDE);
        assert_eq!(Region::Center(hostile).fraction(), DEFAULT_SIDE);
        assert!(Region::Center(hostile).fraction().is_finite());
    }
    assert_eq!(Region::default(), Region::Full);

    // An unusable screen yields the screen itself, never a fabricated rect.
    let degenerate = Frame::new(0.0, 0.0, 0.0, 0.0);
    assert_eq!(Region::center().frame(&degenerate), degenerate);

    // Regions are what a work's preference file names: they serialize whole.
    for region in [
        Region::Full,
        Region::left(),
        Region::right(),
        Region::center(),
        Region::Top,
        Region::Bottom,
    ] {
        let json = serde_json::to_string(&region).expect("a region serializes");
        let back: Region = serde_json::from_str(&json).expect("a region deserializes");
        assert_eq!(back, region);
    }
}

// ─── Slots and seeds ────────────────────────────────────────────────────────

#[test]
fn twelve_seeds_have_stable_keys_names_and_slot_counts() {
    let seeds = LayoutId::all();
    assert_eq!(seeds.len(), 12, "the seed set is twelve layouts");

    let mut keys: Vec<&str> = seeds.iter().map(|id| id.key()).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), seeds.len(), "two seeds share a key");

    for id in seeds {
        assert_eq!(
            LayoutId::from_key(id.key()),
            Some(*id),
            "key {} does not round-trip",
            id.key()
        );
        assert!(!id.name().is_empty());
        let layout = id.seed();
        assert_eq!(layout.id, *id);
        assert!(!layout.slots().is_empty(), "{id:?} has no slots");
        let json = serde_json::to_string(id).expect("a seed id serializes");
        let back: LayoutId = serde_json::from_str(&json).expect("a seed id deserializes");
        assert_eq!(back, *id);
    }
    assert_eq!(LayoutId::from_key("not-a-layout"), None);

    // The documented mapping, layout by layout: ten adapted from Setpiece's
    // built-in presets, two Evo-native.
    let expected: [(LayoutId, usize); 12] = [
        (LayoutId::Full, 1),
        (LayoutId::Halves, 2),
        (LayoutId::Thirds, 3),
        (LayoutId::Quads, 4),
        (LayoutId::MainSide, 2),
        (LayoutId::LeftSplitRight, 3),
        (LayoutId::LeftRightSplit, 3),
        (LayoutId::MainSideVertical, 2),
        (LayoutId::HalvesVertical, 2),
        (LayoutId::ThirdsVertical, 3),
        (LayoutId::HeroRails, 8),
        (LayoutId::Stack, 4),
    ];
    for (id, count) in expected {
        assert_eq!(id.seed().slots().len(), count, "{id:?} slot count");
    }

    for id in LayoutId::all() {
        if matches!(*id, LayoutId::HeroRails | LayoutId::Stack) {
            assert!(
                id.seed().tree().is_none(),
                "{id:?} is explicit geometry, not a partition"
            );
            continue;
        }
        assert!(
            id.seed().tree().is_some(),
            "{id:?} should be tree-defined so a seam editor can adjust it"
        );
    }
}

#[test]
fn no_slot_escapes_the_unit_box_the_region_or_the_screen() {
    let screens = [
        screen(),
        offset_screen(),
        Frame::new(0.0, 0.0, 640.0, 400.0),
        Frame::new(10.0, -20.0, 100.0, 80.0),
    ];
    for id in LayoutId::all() {
        for region in regions() {
            let layout = id.seed().with_region(region);
            for slot in layout.slots() {
                let rect = slot.rect;
                assert!(
                    (0.0..=1.0).contains(&rect.x)
                        && (0.0..=1.0).contains(&rect.y)
                        && rect.x + rect.w <= 1.0 + epsilon()
                        && rect.y + rect.h <= 1.0 + epsilon(),
                    "{id:?} in {region:?} produced slot {:?} outside 0..=1",
                    rect
                );
                assert!(
                    rect.w >= MIN_SLOT_SIDE - epsilon() && rect.h >= MIN_SLOT_SIDE - epsilon(),
                    "{id:?} in {region:?} produced a sliver {:?}",
                    rect
                );
            }
            for screen in screens {
                let region_frame = region.frame(&screen);
                for frame in layout.resolved_slots(&screen) {
                    assert!(
                        frame.x >= region_frame.x - epsilon()
                            && frame.y >= region_frame.y - epsilon()
                            && frame.x + frame.w <= region_frame.x + region_frame.w + epsilon()
                            && frame.y + frame.h <= region_frame.y + region_frame.h + epsilon(),
                        "{id:?} in {region:?} put a frame outside its region on {screen:?}"
                    );
                    assert!(frame.usable());
                }
            }
        }
    }
}

#[test]
fn slot_construction_clamps_hostile_inputs() {
    let hostile = Slot::new(-5.0, 10.0, f64::INFINITY, f64::NAN);
    assert_eq!(hostile.rect.x, 0.0);
    assert_eq!(hostile.rect.y, 1.0 - MIN_SLOT_SIDE);
    // A side the file cannot express lands on the slot floor: a real, visible
    // slot, never an invented full-screen one.
    assert_eq!(hostile.rect.w, MIN_SLOT_SIDE);
    assert_eq!(hostile.rect.h, MIN_SLOT_SIDE);
    assert!(hostile.rect.x + hostile.rect.w <= 1.0);
    assert!(hostile.rect.y + hostile.rect.h <= 1.0);

    let overflowing = Slot::new(0.99, 0.99, 0.5, 0.5);
    assert!(overflowing.rect.x + overflowing.rect.w <= 1.0 + epsilon());
    assert!(overflowing.rect.y + overflowing.rect.h <= 1.0 + epsilon());
    assert!(overflowing.rect.w >= MIN_SLOT_SIDE);
    assert!(overflowing.rect.h >= MIN_SLOT_SIDE);

    let round_trip = Slot::from_frame(&Slot::new(0.25, 0.5, 0.5, 0.5).frame(&screen()), &screen());
    assert_eq!(round_trip, Slot::new(0.25, 0.5, 0.5, 0.5));
}

#[test]
fn hero_rails_seed_is_the_existing_stage_geometry() {
    let slots = hero_rails_slots();
    assert_eq!(slots[0].rect, Role::Hero.default_ratio());
    for index in 0..3u8 {
        assert_eq!(
            slots[1 + index as usize].rect,
            Role::Satellite(index).default_ratio(),
            "satellite {index} drifted from the stage geometry"
        );
    }
    for index in 0..4u8 {
        assert_eq!(
            slots[4 + index as usize].rect,
            Role::Rail(index).default_ratio(),
            "rail {index} drifted from the stage geometry"
        );
    }
    assert_eq!(LayoutId::HeroRails.seed().slots(), slots);
}

#[test]
fn stack_bands_are_separated_and_inside_the_box() {
    let slots = stack_slots();
    assert_eq!(slots.len(), 4);
    for pair in slots.windows(2) {
        let lower = pair[0].rect;
        let upper = pair[1].rect;
        assert!(
            lower.y + lower.h < upper.y + epsilon(),
            "stack bands touch: {lower:?} then {upper:?}"
        );
    }
    let last = slots[3].rect;
    assert!(last.y + last.h <= 1.0);
}

// ─── Split trees ────────────────────────────────────────────────────────────

#[test]
fn every_seed_layout_tree_round_trips_through_json() {
    for id in LayoutId::all() {
        let Some(tree) = id.seed().tree().cloned() else {
            continue;
        };
        let value = serde_json::to_value(&tree).expect("a tree serializes");
        assert!(value.get("kind").is_some(), "{id:?} lost its kind tag");
        assert_eq!(
            value.get("ratio").is_some(),
            !matches!(tree, LayoutNode::Leaf),
            "{id:?} ratio presence"
        );
        if let serde_json::Value::Object(fields) = &value {
            let mut names: Vec<&str> = fields.keys().map(String::as_str).collect();
            names.sort_unstable();
            match tree {
                LayoutNode::Leaf => assert_eq!(names, ["kind"]),
                _ => assert_eq!(names, ["first", "kind", "ratio", "second"]),
            }
        }

        let text = serde_json::to_string(&tree).expect("a tree serializes to text");
        let back: LayoutNode = serde_json::from_str(&text).expect("a tree deserializes");
        assert_eq!(back, tree, "{id:?} did not round-trip");
        assert_eq!(back.flatten(), tree.flatten(), "{id:?} slots drifted");
        assert_eq!(back.leaf_count(), tree.flatten().len());

        let through_value: LayoutNode = serde_json::from_value(value).expect("value round-trip");
        assert_eq!(through_value, tree);
    }
}

#[test]
fn setpiece_shaped_json_deserializes_into_the_documented_slots() {
    // The exact shape Setpiece writes: kind / ratio / first / second.
    let json = r#"{
        "kind": "hSplit",
        "ratio": 0.7,
        "first": {"kind": "leaf"},
        "second": {
            "kind": "vSplit",
            "ratio": 0.5,
            "first": {"kind": "leaf"},
            "second": {"kind": "leaf"}
        }
    }"#;
    let tree: LayoutNode = serde_json::from_str(json).expect("Setpiece-shaped JSON loads");
    let slots = tree.flatten();
    assert_eq!(slots.len(), 3);
    assert_eq!(slots[0].rect, ratio(0.0, 0.0, 1.0, 0.7));
    assert_eq!(slots[1].rect.x, 0.0);
    assert_eq!(slots[1].rect.y, 0.7);
    assert_eq!(slots[1].rect.w, 0.5);
    assert_eq!(slots[2].rect.x, 0.5);
    assert_eq!(slots[2].rect.y, 0.7);
    assert_eq!(slots[2].rect.w, 0.5);
    for slot in &slots[1..] {
        assert!(
            (slot.rect.h - 0.3).abs() < 1e-9,
            "the bottom band is the remaining 30%: {:?}",
            slot.rect
        );
    }
    assert_eq!(tree.ratio(), 0.7);
    assert_eq!(LayoutNode::leaf().ratio(), DEFAULT_RATIO);
}

#[test]
fn split_ratios_clamp_on_construction_and_on_deserialization() {
    assert_eq!(LayoutNode::hsplit(0.0, LayoutNode::leaf(), LayoutNode::leaf()).ratio(), MIN_RATIO);
    assert_eq!(LayoutNode::hsplit(1.7, LayoutNode::leaf(), LayoutNode::leaf()).ratio(), MAX_RATIO);
    assert_eq!(LayoutNode::vsplit(-0.5, LayoutNode::leaf(), LayoutNode::leaf()).ratio(), MIN_RATIO);
    assert_eq!(
        LayoutNode::vsplit(f64::NAN, LayoutNode::leaf(), LayoutNode::leaf()).ratio(),
        DEFAULT_RATIO
    );

    for (written, expected) in [(0.0, MIN_RATIO), (1.0, MAX_RATIO), (2.0, MAX_RATIO), (-1.0, MIN_RATIO), (0.4, 0.4)] {
        let json = format!(
            r#"{{"kind":"vSplit","ratio":{written},"first":{{"kind":"leaf"}},"second":{{"kind":"leaf"}}}}"#
        );
        let tree: LayoutNode = serde_json::from_str(&json).expect("clamped ratio loads");
        assert_eq!(tree.ratio(), expected, "ratio {written} clamped wrongly");
        let slots = tree.flatten();
        assert_eq!(slots.len(), 2);
        for slot in slots {
            assert!(
                slot.rect.w >= MIN_SLOT_SIDE - epsilon() && slot.rect.h >= MIN_SLOT_SIDE - epsilon(),
                "a clamped split produced a sliver: {slot:?}"
            );
            assert!(slot.rect.x + slot.rect.w <= 1.0 + epsilon());
        }
    }

    // A ratio the file cannot even express arrives as the default, not a panic.
    let hostile = r#"{"kind":"hSplit","ratio":null,"first":{"kind":"leaf"},"second":{"kind":"leaf"}}"#;
    let err = serde_json::from_str::<LayoutNode>(hostile);
    assert!(err.is_err(), "null ratio must be refused, never guessed");
}

// ─── Planning ───────────────────────────────────────────────────────────────

#[test]
fn sticky_pass_preserves_a_window_already_in_its_slot() {
    let screen = screen();
    let slots = LayoutId::Halves.seed().resolved_slots(&screen);
    let windows = vec![
        // Frontmost, but sitting in the *second* slot: it stays there.
        window(1, 10, slots[1]),
        window(2, 20, slots[0]),
        // Not placed yet: fills the only slot left.
        window(3, 30, Frame::new(500.0, 500.0, 320.0, 240.0)),
    ];
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    assert_eq!(plan.slot_for(1, 10), Some(1));
    assert_eq!(plan.slot_for(2, 20), Some(0));
    assert_eq!(plan.slot_for(3, 30), None, "no slot is left for the third");
    assert_eq!(plan.placements.len(), 2);
    assert_eq!(plan.to_minimize.len(), 1, "the extra window is reported, not hidden");
    assert_eq!(plan.to_minimize[0].window_id, 30);
    assert!(plan.empty_slots.is_empty());
    // A sticky match is placed at the slot's target rect, not the drifted one.
    let drifted = window(4, 40, Frame::new(slots[0].x + 6.0, slots[0].y - 4.0, slots[0].w + 9.0, slots[0].h));
    let plan = LayoutEngine::plan(&[drifted], &slots, &[]);
    assert_eq!(plan.placements[0].slot, 0);
    assert_eq!(plan.placements[0].frame, slots[0]);
}

#[test]
fn sticky_match_stops_at_the_documented_tolerance() {
    let screen = screen();
    let slots = LayoutId::Halves.seed().resolved_slots(&screen);
    let inside = Frame::new(
        slots[0].x + STICKY_TOLERANCE - 0.1,
        slots[0].y,
        slots[0].w + STICKY_TOLERANCE - 0.1,
        slots[0].h,
    );
    assert!(frames_approx_equal(&inside, &slots[0], STICKY_TOLERANCE));
    let outside = Frame::new(
        slots[0].x + STICKY_TOLERANCE + 5.0,
        slots[0].y,
        slots[0].w,
        slots[0].h,
    );
    assert!(!frames_approx_equal(&outside, &slots[0], STICKY_TOLERANCE));

    // Inside tolerance: preserved in slot 0. Outside: filled by z-order.
    let plan = LayoutEngine::plan(&[window(1, 1, inside)], &slots, &[]);
    assert_eq!(plan.placements[0].slot, 0);
    let plan = LayoutEngine::plan(&[window(1, 1, outside)], &slots, &[]);
    assert_eq!(plan.placements[0].slot, 0, "the fill pass takes the lowest free slot");
    assert_eq!(plan.placements[0].frame, slots[0]);
}

#[test]
fn fill_pass_assigns_in_z_order_to_the_lowest_slots() {
    let screen = screen();
    let slots = LayoutId::Quads.seed().resolved_slots(&screen);
    let windows: Vec<WindowRef> = (0..4)
        .map(|index| window(1, index as u32, Frame::new(700.0, 700.0, 400.0, 300.0)))
        .collect();
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    assert_eq!(
        plan.placements.iter().map(|p| p.slot).collect::<Vec<_>>(),
        vec![0, 1, 2, 3],
        "z-order must fill ascending slots"
    );
    for placement in &plan.placements {
        assert_eq!(placement.frame, slots[placement.slot]);
    }
    assert!(plan.to_minimize.is_empty());
    assert!(plan.empty_slots.is_empty());
    assert!(!plan.is_empty());
    assert_eq!(Plan::default().empty_slots.len(), 0);
    assert!(Plan::default().is_empty());
}

#[test]
fn overflow_windows_are_reported_for_minimization_in_z_order() {
    let screen = screen();
    let slots = LayoutId::Thirds.seed().resolved_slots(&screen);
    let windows: Vec<WindowRef> = (0..5)
        .map(|index| window(7, index as u32, Frame::new(50.0, 50.0, 300.0, 200.0)))
        .collect();
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    assert_eq!(plan.placements.len(), 3);
    assert_eq!(
        plan.placements.iter().map(|p| p.window_id).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        plan.to_minimize.iter().map(|w| w.window_id).collect::<Vec<_>>(),
        vec![3, 4],
        "the overflow is the tail of the z-order"
    );
    // The overflow keeps the frame the caller needs to book a receipt.
    assert_eq!(plan.to_minimize[0].frame, Frame::new(50.0, 50.0, 300.0, 200.0));
    assert!(plan.empty_slots.is_empty());
}

#[test]
fn empty_slots_are_reported_when_windows_run_out() {
    let screen = screen();
    let slots = LayoutId::Thirds.seed().resolved_slots(&screen);
    let plan = LayoutEngine::plan(&[window(3, 9, Frame::new(10.0, 10.0, 100.0, 100.0))], &slots, &[]);
    assert_eq!(plan.placements.len(), 1);
    assert_eq!(plan.placements[0].slot, 0);
    assert_eq!(plan.empty_slots, vec![1, 2]);
    assert!(plan.to_minimize.is_empty());
}

#[test]
fn no_windows_and_no_slots_are_both_honest() {
    let screen = screen();
    let slots = LayoutId::Halves.seed().resolved_slots(&screen);
    let plan = LayoutEngine::plan(&[], &slots, &[]);
    assert!(plan.placements.is_empty());
    assert!(plan.to_minimize.is_empty());
    assert_eq!(plan.empty_slots, vec![0, 1]);
    assert!(plan.is_empty());

    let windows = vec![window(1, 1, Frame::new(0.0, 0.0, 100.0, 100.0))];
    let plan = LayoutEngine::plan(&windows, &[], &[]);
    assert!(plan.placements.is_empty());
    assert_eq!(plan.to_minimize.len(), 1);
    assert!(plan.empty_slots.is_empty());
}

#[test]
fn alternate_slot_geometry_keeps_a_work_in_place_when_the_dock_moved() {
    let before = Frame::new(0.0, 0.0, 1440.0, 900.0);
    // The Dock moved off this display: the same layout resolves 30 pt lower.
    let after = Frame::new(0.0, 30.0, 1440.0, 870.0);
    let layout = LayoutId::Halves.seed();
    let before_slots = layout.resolved_slots(&before);

    let windows = vec![
        window(1, 1, before_slots[1]),
        window(2, 2, before_slots[0]),
    ];
    let plan = LayoutEngine::plan_layout(&windows, &layout, &after, &[before]);
    assert_eq!(plan.slot_for(1, 1), Some(1), "the work kept its side");
    assert_eq!(plan.slot_for(2, 2), Some(0));
    let after_slots = layout.resolved_slots(&after);
    assert_eq!(plan.placements[0].frame, after_slots[1], "the slot moved, the work did not");

    // Without the alternate geometry the windows are simply filled in
    // z-order: the front window takes slot 0.
    let plan = LayoutEngine::plan_layout(&windows, &layout, &after, &[]);
    assert_eq!(plan.slot_for(1, 1), Some(0));
    assert_eq!(plan.slot_for(2, 2), Some(1));

    // A short alternate slice adds no candidates; a window matching neither
    // primary nor alternate geometry is filled normally.
    let plan = LayoutEngine::plan(
        &[window(1, 1, Frame::new(700.0, 400.0, 200.0, 200.0))],
        &before_slots,
        &[Frame::new(0.0, 0.0, 1.0, 1.0)],
    );
    assert_eq!(plan.placements[0].slot, 0);
}

#[test]
fn a_centred_region_plans_inside_its_own_band() {
    let screen = screen();
    let layout = LayoutId::Quads.seed().with_region(Region::center());
    let region_frame = Region::center().frame(&screen);
    let slots = layout.resolved_slots(&screen);
    assert_eq!(slots.len(), 4);
    for frame in &slots {
        assert!(frame.x >= region_frame.x - epsilon());
        assert!(frame.x + frame.w <= region_frame.x + region_frame.w + epsilon());
    }
    let windows: Vec<WindowRef> = (0..4)
        .map(|index| window(1, index as u32, Frame::new(9999.0, 9999.0, 10.0, 10.0)))
        .collect();
    let plan = LayoutEngine::plan(&windows, &slots, &[]);
    assert_eq!(plan.placements.len(), 4);
    for placement in &plan.placements {
        assert!(placement.frame.x >= region_frame.x - epsilon());
    }
}
