//! The arrangement policy, proven through its public surface: the automatic
//! choice, the picker's rows, sticky planning with learned slot frames, the
//! honest overflow ending, and the textual memory that carries learnings
//! across restarts.

use evo_pods::arrange::{self, ArrangeMemory, auto_layout, choices, store};
use evo_pods::layout::{LayoutId, Region, STICKY_TOLERANCE, WindowRef, frames_approx_equal};
use evo_pods::surface::Frame;

fn screen() -> Frame {
    Frame::new(0.0, 0.0, 1600.0, 1000.0)
}

fn window(pid: i32, window_id: u32, frame: Frame) -> WindowRef {
    WindowRef {
        pid,
        window_id,
        frame,
    }
}

fn overlapping(a: &Frame, b: &Frame) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

#[test]
fn auto_layout_is_the_smallest_seed_that_holds_every_window() {
    let expected = [
        (0, LayoutId::Full),
        (1, LayoutId::Full),
        (2, LayoutId::Halves),
        (3, LayoutId::Thirds),
        (4, LayoutId::Quads),
        (5, LayoutId::HeroRails),
        (6, LayoutId::HeroRails),
        (7, LayoutId::HeroRails),
        (8, LayoutId::HeroRails),
        (9, LayoutId::HeroRails),
    ];
    for (count, id) in expected {
        assert_eq!(auto_layout(count), id, "{count} windows");
    }
    for count in 0..=8usize {
        let slots = auto_layout(count).seed().slots().len();
        assert!(
            slots >= count,
            "the automatic seed must hold every window: {count} windows, {slots} slots"
        );
    }
}

#[test]
fn three_windows_land_in_thirds_inside_the_usable_frame() {
    let windows = vec![
        window(1, 10, Frame::new(0.0, 0.0, 300.0, 300.0)),
        window(1, 11, Frame::new(200.0, 100.0, 300.0, 300.0)),
        window(2, 20, Frame::new(400.0, 200.0, 300.0, 300.0)),
    ];
    let arrangement = arrange::arrange(&windows, &ArrangeMemory::new(), &screen(), &[]);

    assert_eq!(arrangement.layout.id, LayoutId::Thirds);
    assert_eq!(arrangement.plan.placements.len(), 3);
    assert!(arrangement.plan.to_minimize.is_empty());
    assert!(arrangement.plan.empty_slots.is_empty());

    for placement in &arrangement.plan.placements {
        let frame = placement.frame;
        assert!(
            frame.x >= 0.0
                && frame.y >= 0.0
                && frame.x + frame.w <= screen().w
                && frame.y + frame.h <= screen().h,
            "a slot must stay inside the usable frame: {frame:?}"
        );
    }
    for (index, placement) in arrangement.plan.placements.iter().enumerate() {
        for other in arrangement.plan.placements.iter().skip(index + 1) {
            assert!(
                !overlapping(&placement.frame, &other.frame),
                "thirds must not overlap: {:?} vs {:?}",
                placement.frame,
                other.frame
            );
        }
    }
}

#[test]
fn a_settled_work_is_left_where_it_is() {
    let memory = ArrangeMemory::new();
    let first = arrange::arrange(
        &[
            window(1, 10, Frame::new(0.0, 0.0, 100.0, 100.0)),
            window(1, 11, Frame::new(500.0, 500.0, 100.0, 100.0)),
            window(2, 20, Frame::new(900.0, 900.0, 100.0, 100.0)),
        ],
        &memory,
        &screen(),
        &[],
    );
    // Apply the plan, then ask again: the sticky pass must recognize every
    // window as already in its slot, so nothing jumps a second time.
    let placed: Vec<WindowRef> = first
        .plan
        .placements
        .iter()
        .map(|placement| window(placement.pid, placement.window_id, placement.frame))
        .collect();
    let second = arrange::arrange(&placed, &memory, &screen(), &[]);
    assert!(arrange::is_settled(&second, &placed), "{second:?}");
    for placement in &second.plan.placements {
        let current = placed
            .iter()
            .find(|w| w.pid == placement.pid && w.window_id == placement.window_id)
            .expect("every window is still there");
        assert!(
            frames_approx_equal(&current.frame, &placement.frame, STICKY_TOLERANCE),
            "{current:?} was moved to {:?}",
            placement.frame
        );
    }
}

#[test]
fn overflow_beyond_the_layout_minimizes_the_backmost_and_never_vanishes() {
    let windows: Vec<WindowRef> = (1..=10)
        .map(|index| {
            window(
                1,
                index as u32,
                Frame::new(10.0 * index as f64, 10.0, 80.0, 60.0),
            )
        })
        .collect();
    let arrangement = arrange::arrange(&windows, &ArrangeMemory::new(), &screen(), &[]);

    assert_eq!(arrangement.layout.id, LayoutId::HeroRails);
    assert_eq!(
        arrangement.plan.placements.len(),
        arrangement.slots.len(),
        "the layout is filled exactly"
    );
    assert_eq!(
        arrangement
            .plan
            .to_minimize
            .iter()
            .map(|window| window.window_id)
            .collect::<Vec<_>>(),
        vec![9, 10],
        "z-order decides: the backmost windows minimize"
    );
    assert!(arrangement.plan.empty_slots.is_empty());
}

#[test]
fn a_picked_layout_wins_over_the_automatic_one() {
    let mut memory = ArrangeMemory::new();
    memory.choose(LayoutId::Stack);
    let windows = vec![
        window(1, 10, Frame::new(0.0, 0.0, 100.0, 100.0)),
        window(1, 11, Frame::new(300.0, 0.0, 100.0, 100.0)),
        window(1, 12, Frame::new(600.0, 0.0, 100.0, 100.0)),
        window(2, 20, Frame::new(900.0, 0.0, 100.0, 100.0)),
    ];
    let arrangement = arrange::arrange(&windows, &memory, &screen(), &[]);
    assert_eq!(arrangement.layout.id, LayoutId::Stack);
    assert_eq!(arrangement.plan.placements.len(), 4);
    assert!(arrangement.plan.to_minimize.is_empty());

    memory.clear_choice();
    let automatic = arrange::arrange(&windows, &memory, &screen(), &[]);
    assert_eq!(automatic.layout.id, LayoutId::Quads);
}

#[test]
fn a_region_keeps_the_work_on_its_side_of_the_screen() {
    let mut memory = ArrangeMemory::new();
    memory.set_region(LayoutId::Thirds, Region::left());
    let windows = vec![
        window(1, 10, Frame::new(0.0, 0.0, 100.0, 100.0)),
        window(1, 11, Frame::new(900.0, 0.0, 100.0, 100.0)),
        window(1, 12, Frame::new(1200.0, 0.0, 100.0, 100.0)),
    ];
    let arrangement = arrange::arrange(&windows, &memory, &screen(), &[]);
    let right_edge = screen().w * 0.6;
    for slot in &arrangement.slots {
        assert!(
            slot.x + slot.w <= right_edge + 0.001,
            "a left region owns the left 60 %: {slot:?}"
        );
        assert!(slot.y + slot.h <= screen().h + 0.001);
    }
}

#[test]
fn learn_drag_teaches_the_nearest_slot_and_the_next_arrangement_uses_it() {
    let mut memory = ArrangeMemory::new();
    let layout = memory.layout_for(3);
    let dragged = Frame::new(40.0, 60.0, 420.0, 700.0);
    let slot = memory
        .learn_drag(&layout, &screen(), &dragged)
        .expect("a usable drag on a usable screen teaches a slot");
    assert_eq!(slot, 0, "the drag sits nearest the first third");

    let windows = vec![
        window(1, 10, Frame::new(700.0, 100.0, 300.0, 300.0)),
        window(1, 11, Frame::new(700.0, 500.0, 300.0, 300.0)),
        window(2, 20, Frame::new(1200.0, 200.0, 300.0, 300.0)),
    ];
    let arrangement = arrange::arrange(&windows, &memory, &screen(), &[]);
    let front = arrangement.plan.placements[0];
    assert!(
        frames_approx_equal(&front.frame, &dragged, 0.5),
        "the learned frame is what the work re-opens into: {:?}",
        front.frame
    );
}

#[test]
fn a_learned_slot_is_measured_inside_its_region_so_a_narrower_region_carries_it() {
    let mut memory = ArrangeMemory::new();
    memory.set_region(LayoutId::Full, Region::Center(0.5));
    let layout = memory.layout_for(1);
    let dragged = Frame::new(500.0, 300.0, 300.0, 400.0); // inside the centred half
    assert_eq!(memory.learn_drag(&layout, &screen(), &dragged), Some(0));

    let center = Region::Center(0.5).frame(&screen());
    let learned = memory.slots_for(&layout, &screen())[0];
    assert!(
        learned.x >= center.x - 0.001 && learned.x + learned.w <= center.x + center.w + 0.001,
        "the learned slot stays inside its region: {learned:?} vs {center:?}"
    );
    assert!(
        (learned.w / center.w - dragged.w / center.w).abs() < 0.001,
        "the learning is a fraction of the region, not of the screen"
    );
}

#[test]
fn a_drag_on_no_usable_screen_learns_nothing() {
    let mut memory = ArrangeMemory::new();
    let layout = memory.layout_for(1);
    let dead = Frame::new(0.0, 0.0, 0.0, 0.0);
    assert_eq!(memory.learn_drag(&layout, &dead, &screen()), None);
    assert_eq!(
        memory.learn_drag(&layout, &screen(), &Frame::new(f64::NAN, 0.0, 100.0, 100.0)),
        None
    );
    assert_eq!(
        store::export(&[("pod-1".to_string(), memory.clone())]),
        format!("{}\n", store::HEADER),
        "nothing learned exports nothing but the versioned header"
    );
}

#[test]
fn the_picker_marks_the_choice_and_what_fits() {
    let mut memory = ArrangeMemory::new();
    let rows = choices(&memory, 3);
    assert_eq!(rows.len(), LayoutId::all().len());
    let selected: Vec<&str> = rows
        .iter()
        .filter(|row| row.selected)
        .map(|row| row.name)
        .collect();
    assert_eq!(selected, vec!["Thirds"], "the automatic choice is selected");
    for row in &rows {
        assert_eq!(row.fits, row.slots >= 3, "{} fits three windows", row.name);
    }

    memory.choose(LayoutId::Stack);
    let rows = choices(&memory, 3);
    assert!(
        rows.iter()
            .find(|row| row.id == LayoutId::Stack)
            .unwrap()
            .selected
    );
    assert!(
        !rows
            .iter()
            .find(|row| row.id == LayoutId::Thirds)
            .unwrap()
            .selected
    );
}

#[test]
fn the_memory_round_trips_through_its_text_form() {
    let mut memory = ArrangeMemory::new();
    memory.choose(LayoutId::MainSide);
    memory.set_region(LayoutId::MainSide, Region::right());
    let layout = memory.layout_for(2);
    assert_eq!(
        memory.learn_drag(&layout, &screen(), &Frame::new(10.0, 20.0, 400.0, 500.0)),
        Some(0)
    );

    let text = store::export(&[("pod-7".to_string(), memory.clone())]);
    assert!(text.starts_with(store::HEADER));
    let restored = store::import(&text);
    assert_eq!(restored, vec![("pod-7".to_string(), memory)]);
}

#[test]
fn the_text_form_skips_what_it_cannot_understand() {
    let text = format!(
        "{header}\npod-1\tchosen\tnot-a-layout\npod-1\tchosen\tquads\npod-1\tregion\tquads\tnowhere\npod-1\tslot\tquads\tzero\t1\t2\t3\t4\npod-1\tslot\tquads\t2\t0.1\t0.2\thalf\t0.4\npod-1\tslot\tquads\t3\t0.1\t0.2\t0.3\t0.4\ngarbage\n",
        header = store::HEADER
    );
    let imported = store::import(&text);
    assert_eq!(
        imported.len(),
        1,
        "one work, not one per line: {imported:?}"
    );
    let (pod, memory) = &imported[0];
    assert_eq!(pod, "pod-1");
    assert_eq!(memory.chosen, Some(LayoutId::Quads));
    assert!(
        memory.regions.is_empty(),
        "an unreadable region is not invented"
    );
    assert_eq!(
        memory
            .overrides
            .get(&LayoutId::Quads)
            .map(|slots| slots.len()),
        Some(1),
        "only the well-formed slot line survives"
    );
    assert_eq!(
        store::region_key(Region::center()),
        "center:0.700",
        "region keys are stable text"
    );
    assert_eq!(
        store::region_from_key("left:0.55"),
        Some(Region::Left(0.55))
    );
    assert_eq!(store::region_from_key("top"), Some(Region::Top));
    assert_eq!(store::region_from_key("sideways"), None);
}
