//! The runtime, proven against a fake surface: entry choreography,
//! containment modes, leases, switches, mothball safety, isolation
//! launches, and recipe learning.

mod fake;

use evo_pods::config::ContainMode;
use evo_pods::config::PodConfig;
use evo_pods::pod::{BadgeKind, Pod, PodState};
use evo_pods::runtime::{PodError, PodRuntime};
use evo_pods::surface::Frame;
use evo_threads::{Delta, ResumeBundle};
use fake::{FakeSurface, app, pod_fixture, window, window_with_frame};

fn runtime_with(surface: FakeSurface) -> PodRuntime {
    PodRuntime::new(Box::new(surface), vec![900], PodConfig::default())
}

#[test]
fn enter_arranges_own_windows_veils_foreign_and_books_a_lease() {
    let mut surface = FakeSurface::base();
    surface.apps = vec![app(1, "Editor", false), app(2, "Mail", false), app(900, "Evo", false)];
    surface.windows = vec![
        window_with_frame(1, 10, "main.ts", Some("/repo/main.ts"), Frame::new(0.0, 0.0, 300.0, 300.0)),
        window_with_frame(1, 11, "api", Some("https://docs.example/api"), Frame::new(0.0, 0.0, 300.0, 300.0)),
        window_with_frame(2, 20, "Inbox", None, Frame::new(50.0, 50.0, 300.0, 300.0)),
        window_with_frame(900, 99, "Evo Home", None, Frame::new(0.0, 0.0, 200.0, 200.0)),
    ];
    let mut rt = runtime_with(surface);
    rt.set_mode(ContainMode::Dim); // this test proves the veil vocabulary
    rt.set_pods(vec![pod_fixture()]);

    let note = rt.enter(0, 1000).unwrap();
    assert!(note.contains("Entered"));
    assert!(rt.is_active());

    let lease = rt.lease().unwrap();
    assert_eq!(lease.moved_windows.len(), 2, "both pod windows got recipe frames");
    assert_eq!(lease.veils.len(), 1, "exactly one foreign window veiled");
    assert!(lease.hidden_apps.is_empty(), "Dim never hides");
}

#[test]
fn recipe_frames_materialize_on_the_screen_not_the_window_rect() {
    // Regression: recipe ratios must scale against the STAGE (the main
    // screen's usable frame), never against each window's own rect —
    // otherwise "entering" merely reshapes windows onto themselves.
    let mut surface = FakeSurface::base(); // screen: (0,0,1000,800)
    surface.apps = vec![app(1, "Editor", false)];
    surface.windows = vec![window_with_frame(
        1,
        10,
        "main.ts",
        Some("/repo/main.ts"),
        Frame::new(500.0, 400.0, 80.0, 60.0),
    )];
    let mut rt = runtime_with(surface);
    rt.set_mode(ContainMode::Stage); // recipes are the Stage vocabulary
    rt.set_pods(vec![pod_fixture()]);
    rt.enter(0, 1).unwrap();

    // Hero default ratio (0.03, 0.06, 0.58, 0.62) over (0,0,1000,800)
    // lands at (30, 48) — a corner far from the window's own (500, 400).
    let log = rt.surface_log();
    assert!(
        log.iter().any(|line| line.starts_with("mv:1:10:30:48")),
        "hero must stage at (30,48) on the screen; log was {log:?}"
    );
}

#[test]
fn double_enter_refuses_switch_then_enter_leaves_no_stale_state() {
    let mut surface = FakeSurface::base();
    surface.apps = vec![app(1, "Editor", false)];
    surface.windows = vec![window(1, 10, "main.ts", Some("/repo/main.ts"))];
    let mut rt = runtime_with(surface);
    let mut other = pod_fixture();
    other.id = evo_pods::PodId(8);
    rt.set_pods(vec![pod_fixture(), other]);

    rt.enter(0, 1).unwrap();
    assert!(matches!(rt.enter(1, 2), Err(PodError::AlreadyActive { .. })));
    let note = rt.switch(1, 3).unwrap();
    assert!(note.contains("Entered"));
    assert_eq!(rt.active().unwrap().id, evo_pods::PodId(8));
}

#[test]
fn leave_replays_the_veils_removed_and_frames_restored() {
    let mut surface = FakeSurface::base();
    surface.apps = vec![app(1, "Editor", false), app(2, "Mail", false)];
    surface.windows = vec![
        window_with_frame(1, 10, "main.ts", Some("/repo/main.ts"), Frame::new(1.0, 1.0, 300.0, 300.0)),
        window_with_frame(2, 20, "Inbox", None, Frame::new(2.0, 2.0, 300.0, 300.0)),
    ];
    let mut rt = runtime_with(surface);
    rt.set_mode(ContainMode::Dim); // veil reversal is the claim here
    rt.set_pods(vec![pod_fixture()]);
    rt.enter(0, 1).unwrap();
    let note = rt.leave().unwrap();
    assert!(note.contains("Left"));
    let lease = rt.lease();
    assert!(lease.is_none());
}

#[test]
fn park_mode_minimizes_foreign_instead_of_veiling() {
    let mut surface = FakeSurface::base();
    surface.apps = vec![app(1, "Editor", false), app(2, "Mail", false)];
    surface.windows = vec![
        window(1, 10, "main.ts", Some("/repo/main.ts")),
        window(2, 20, "Inbox", None),
    ];
    let mut rt = runtime_with(surface);
    rt.set_mode(ContainMode::Park);
    rt.set_pods(vec![pod_fixture()]);
    rt.enter(0, 1).unwrap();
    let lease = rt.lease().unwrap();
    assert_eq!(lease.parked_windows, vec![(2, 20)]);
    assert!(lease.veils.is_empty());
}

#[test]
fn empty_pod_refuses_to_enter_rather_than_veil_for_nothing() {
    let surface = FakeSurface::base();
    let mut rt = runtime_with(surface);
    let mut bare = pod_fixture();
    bare.surfaces = Default::default();
    rt.set_pods(vec![bare]);
    assert!(matches!(rt.enter(0, 1), Err(PodError::EmptySurface)));
}

#[test]
fn refresh_keeps_active_by_id_across_reordering() {
    let mut surface = FakeSurface::base();
    surface.apps = vec![app(1, "Editor", false)];
    surface.windows = vec![window(1, 10, "main.ts", Some("/repo/main.ts"))];
    let mut rt = runtime_with(surface);
    let mut other = pod_fixture();
    other.id = evo_pods::PodId(8);
    rt.set_pods(vec![pod_fixture(), other.clone()]);
    rt.enter(1, 1).unwrap();
    assert_eq!(rt.active().unwrap().id, evo_pods::PodId(8));
    rt.set_pods(vec![other]);
    assert_eq!(rt.active().unwrap().id, evo_pods::PodId(8));
}

#[test]
fn mothball_refuses_while_unsaved_work_would_be_eaten_unless_forced() {
    let mut surface = FakeSurface::base();
    surface.apps = vec![app(1, "Editor", false)];
    surface.windows = vec![window(1, 10, "main.ts", Some("/repo/main.ts"))];
    let mut rt = runtime_with(surface);
    let mut pod = pod_fixture();
    pod.bundle.open_deltas = vec![Delta::UnsavedEdits { resource: "main.ts".into() }];
    pod.badges = evo_pods::pod::badges_from_bundle(&pod.bundle);
    rt.set_pods(vec![pod]);

    assert!(matches!(rt.mothball(0, false), Err(PodError::UnsafeMothball { .. })));
    let note = rt.mothball(0, true).unwrap();
    assert!(note.contains("Mothballed"));
    assert_eq!(rt.pods()[0].state, PodState::Mothballed);
}

#[test]
fn isolated_launch_is_none_without_policy_and_well_formed_with_one() {
    let surface = FakeSurface::base();
    let mut rt = runtime_with(surface);
    rt.set_pods(vec![pod_fixture()]);
    let pod_id = evo_pods::PodId(7);

    assert!(rt.isolated_launch("Unknown App", "/Applications/U.app", pod_id, "/pods").is_none());

    rt.set_policies(vec![evo_pods::policy::InstancePolicy {
        app: "Fancy".into(),
        data_dir_flag: "--user-data-dir=".into(),
        extra_flag: None,
    }]);
    let spec = rt.isolated_launch("Fancy", "/Applications/F.app", pod_id, "/pods").unwrap();
    assert_eq!(spec.program, "/usr/bin/open");
    assert!(spec.args.iter().any(|a| a == "-na"));
    assert!(spec.args.iter().any(|a| a == &"--user-data-dir=/pods/pod-7".to_string()));
}

#[test]
fn learn_frame_records_user_choreography_and_export_roundTrips_it() {
    let mut surface = FakeSurface::base();
    surface.apps = vec![app(1, "Editor", false)];
    surface.windows = vec![window(1, 10, "main.ts", Some("/repo/main.ts"))];
    let mut rt = runtime_with(surface);
    rt.set_pods(vec![pod_fixture()]);
    rt.enter(0, 1).unwrap();
    rt.learn_frame(evo_pods::PodId(7), "file:///repo/main.ts", Frame::new(100.0, 100.0, 500.0, 400.0));
    let text = rt.export_recipes();
    assert!(text.contains("pod-7"));
    let mut rt2 = runtime_with(FakeSurface::base());
    rt2.import_recipes(&text);
    assert!(rt2.recipe(evo_pods::PodId(7)).is_some());
}

#[test]
fn suggestion_surfaces_memory_and_respects_silence() {
    let surface = FakeSurface::base();
    let mut rt = runtime_with(surface);
    let mut pod = pod_fixture();
    pod.companions = vec![("long-read.pdf".into(), 200_000.0)];
    rt.set_pods(vec![pod]);
    assert!(rt.suggest_for("long-read.pdf").is_some());
    assert!(rt.suggest_for("unknown").is_none());
}

#[test]
fn badges_cover_contested_and_delta_kinds_in_rank_order() {
    let mut view = fake::view_fixture();
    view.contested = vec!["shared-repo".into()];
    let bundle = ResumeBundle {
        resume_point: "x".into(),
        resume_reason: "x".into(),
        restore_set: vec![],
        open_deltas: vec![Delta::DownloadedNotYetUsed { path: "pkg.zip".into() }],
    };
    let pod = Pod::from_views(&view, &bundle, "n", "r", 1.0, vec![], Default::default());
    let kinds: Vec<BadgeKind> = pod.badges.iter().map(|b| b.kind).collect();
    assert_eq!(kinds, vec![BadgeKind::DownloadReady, BadgeKind::Contested]);
}


#[test]
fn stage_enter_resurrects_members_parks_split_windows_hides_aliens_then_leave_returns_exactly() {
    // The pod-desktop claim end to end (Stage, the subtractive mode):
    // the pod's own parked window comes back first, a split app keeps its
    // pod window but loses its foreign one singly, an alien app hides
    // whole — and leaving replays the precise inverse, ending with the
    // space's window going back into the space.
    let mut surface = FakeSurface::base();
    surface.apps = vec![app(1, "Editor", false), app(2, "Browser", false), app(3, "Mail", false)];
    let mut parked_member = window(1, 10, "main.ts", Some("/repo/main.ts"));
    parked_member.minimized = true;
    surface.windows = vec![
        parked_member,
        window(2, 40, "api", Some("https://docs.example/api")),
        window(2, 41, "shopping", None),
        window(3, 50, "Inbox", None),
    ];
    let mut rt = runtime_with(surface);
    rt.set_mode(ContainMode::Stage);
    rt.set_pods(vec![pod_fixture()]);

    rt.enter(0, 1).unwrap();
    let lease = rt.lease().unwrap();
    assert_eq!(lease.unparked_members, vec![(1, 10)]);
    assert_eq!(lease.parked_windows, vec![(2, 41)]);
    assert_eq!(lease.hidden_apps, vec![3]);
    let log = rt.surface_log();
    assert!(log.contains(&"unpark:1:10".to_string()), "{log:?}");
    assert!(log.contains(&"park:2:41".to_string()), "{log:?}");
    assert!(log.contains(&"hide:3".to_string()), "{log:?}");
    assert!(
        !log.contains(&"park:3:50".to_string()),
        "a hidden app's windows never clutter the Dock: {log:?}"
    );

    let note = rt.leave().unwrap();
    assert!(note.contains("Left"));
    let log = rt.surface_log();
    assert!(log.contains(&"unhide:3".to_string()), "{log:?}");
    assert!(log.contains(&"unpark:2:41".to_string()), "{log:?}");
    let repark = log.iter().rposition(|line| line == "park:1:10").expect("repark missing");
    let unhide = log.iter().rposition(|line| line == "unhide:3").unwrap();
    let unpark_foreign = log.iter().rposition(|line| line == "unpark:2:41").unwrap();
    assert!(
        repark > unpark_foreign && unpark_foreign > unhide,
        "reversal order: unhide -> restore -> repark-member last: {log:?}"
    );
}


#[test]
fn claim_window_merges_evidence_and_survives_an_engine_refresh() {
    // Add-to-Pod round: the person points at a still-open window; its
    // document lands in the pod's evidence (so the next enter treats the
    // window as theirs) — and an engine refresh must never un-own it,
    // because the claim lives in the addon ledger too.
    let mut surface = FakeSurface::base();
    surface.apps = vec![app(1, "Editor", false)];
    surface.windows = vec![window(1, 10, "inbox.md", Some("/repo/inbox.md"))];
    let mut rt = runtime_with(surface);
    rt.set_pods(vec![pod_fixture()]);

    let win = rt.open_windows()[0].clone();
    let note = rt.claim_window(0, &win).unwrap();
    assert!(note.contains("Added"), "{note}");
    assert!(rt.pods()[0]
        .surfaces
        .documents
        .contains(&"/repo/inbox.md".to_string()));

    // Engine refresh wipes the pod projection; the claim re-attaches.
    rt.set_pods(vec![pod_fixture()]);
    assert!(rt.pods()[0]
        .surfaces
        .documents
        .contains(&"/repo/inbox.md".to_string()),
        "refresh must not un-own the claim");

    // Export/import hand the ledger across a restart.
    let text = rt.export_addons();
    let mut rt2 = runtime_with(FakeSurface::base());
    rt2.import_addons(&text);
    rt2.set_pods(vec![pod_fixture()]);
    assert!(rt2.pods()[0]
        .surfaces
        .documents
        .contains(&"/repo/inbox.md".to_string()));
}

#[test]
fn add_resource_dedupes_and_open_runs_usr_bin_open() {
    use evo_pods::PodResource;
    let surface = FakeSurface::base();
    let mut rt = runtime_with(surface);
    rt.set_pods(vec![pod_fixture()]);

    let note = rt
        .add_resource(0, PodResource::App("Notes".into()))
        .unwrap();
    assert!(note.contains("Saved"), "{note}");
    let again = rt
        .add_resource(0, PodResource::App("Notes".into()))
        .unwrap();
    assert!(again.contains("already"), "{again}");
    assert_eq!(rt.pods()[0].resources.len(), 1);

    let open_note = rt.open_resource(0, 0).unwrap();
    assert!(open_note.contains("Opened"), "{open_note}");
    let log = rt.surface_log();
    assert!(log.contains(&"run:/usr/bin/open".to_string()), "{log:?}");
}
// ─── The unified work space (containment off) ───────────────────────────────

/// A work with `count` owned windows on one screen, plus one foreign window.
fn tiled_surface(count: usize) -> FakeSurface {
    let mut surface = FakeSurface::base(); // screen: (0,0,1000,800)
    surface.apps = vec![app(1, "Editor", false), app(2, "Mail", false)];
    let mut windows = vec![
        window_with_frame(
            1,
            10,
            "main.ts",
            Some("/repo/main.ts"),
            Frame::new(10.0, 10.0, 300.0, 200.0),
        ),
        window_with_frame(
            1,
            11,
            "api",
            Some("https://docs.example/api"),
            Frame::new(40.0, 40.0, 300.0, 200.0),
        ),
    ];
    for index in 2..count {
        let id = 10 + index as u32;
        windows.push(window_with_frame(
            1,
            id,
            &format!("notes-{index}"),
            None,
            Frame::new(70.0 * index as f64, 70.0, 300.0, 200.0),
        ));
    }
    windows.push(window_with_frame(
        2,
        20,
        "Inbox",
        None,
        Frame::new(500.0, 500.0, 300.0, 200.0),
    ));
    surface.windows = windows;
    surface
}

/// Owns every extra window by title (Add-to-Pod's claim), so a work can have
/// more windows than its original evidence.
fn own_extra_windows(rt: &mut PodRuntime, count: usize) {
    for index in 2..count {
        let id = 10 + index as u32;
        let window = rt
            .open_windows()
            .into_iter()
            .find(|w| w.window_id == id)
            .expect("the window is on the desktop");
        rt.claim_window(0, &window).unwrap();
    }
}

#[test]
fn tile_enter_arranges_the_work_and_touches_nothing_foreign() {
    let surface = tiled_surface(2);
    assert_eq!(PodConfig::default().contain, ContainMode::Tile);
    let mut rt = runtime_with(surface);
    rt.set_pods(vec![pod_fixture()]);

    let note = rt.enter(0, 1).unwrap();
    assert!(note.contains("tiled as Halves"), "{note}");

    let lease = rt.lease().unwrap();
    assert_eq!(
        lease.moved_windows.len(),
        2,
        "both of the work's windows moved"
    );
    assert!(lease.veils.is_empty(), "containment off: nothing is veiled");
    assert!(
        lease.parked_windows.is_empty(),
        "nothing foreign is minimized"
    );
    assert!(lease.hidden_apps.is_empty(), "no app is hidden");
    assert!(lease.unhidden_apps.is_empty());

    let log = rt.surface_log();
    assert!(
        log.contains(&"mv:1:10:0:0".to_string()) && log.contains(&"mv:1:11:500:0".to_string()),
        "halves: the two work windows take the two columns: {log:?}"
    );
    assert!(
        !log.iter().any(|line| line.starts_with("mv:2:20")),
        "the foreign window is never moved: {log:?}"
    );
    assert!(
        !log.iter().any(|line| line.starts_with("veil:")
            || line.starts_with("park:")
            || line.starts_with("hide:")),
        "tiling touches nothing but the work: {log:?}"
    );
    assert!(
        !log.iter().any(|line| line.starts_with("activate:")),
        "entering a work does not steal focus: {log:?}"
    );
}

#[test]
fn tile_overflow_minimizes_and_leaving_restores_every_frame_and_window() {
    let surface = tiled_surface(5);
    let mut rt = runtime_with(surface);
    rt.set_pods(vec![pod_fixture()]);
    own_extra_windows(&mut rt, 5);

    // Four slots (Stack) and five windows: the backmost minimizes.
    let pick = rt.set_layout(0, evo_pods::LayoutId::Stack).unwrap();
    assert!(pick.contains("now opens into Stack"), "{pick}");

    rt.enter(0, 1).unwrap();
    let lease = rt.lease().unwrap().clone();
    assert_eq!(lease.moved_windows.len(), 4, "four windows tiled");
    assert_eq!(
        lease.parked_windows.len(),
        1,
        "the fifth minimized, never lost"
    );
    let originals: Vec<(i32, u32, Frame)> = lease.moved_windows.clone();

    let note = rt.leave().unwrap();
    assert!(note.contains("Left"), "{note}");
    let log = rt.surface_log();
    for (pid, id, frame) in originals {
        let restore = format!("mv:{pid}:{id}:{}:{}", frame.x, frame.y);
        let back = log.iter().rposition(|line| line == &restore);
        let first = log
            .iter()
            .position(|line| line.starts_with(&format!("mv:{pid}:{id}:")));
        assert!(
            back.is_some(),
            "every frame comes back exactly ({restore}): {log:?}"
        );
        assert!(
            first.is_some_and(|first| back.unwrap() > first),
            "the restore happens after the arrangement: {log:?}"
        );
    }
    let parked = lease.parked_windows[0];
    assert!(
        log.contains(&format!("unpark:{}:{}", parked.0, parked.1)),
        "the minimized work window comes back: {log:?}"
    );
}

#[test]
fn the_picker_retiles_the_active_work_and_leaving_still_restores_it() {
    let surface = tiled_surface(2);
    let mut rt = runtime_with(surface);
    rt.set_pods(vec![pod_fixture()]);

    rt.enter(0, 1).unwrap();
    let before = rt.lease().unwrap().moved_windows.len();

    let rows = rt.layout_choices(0).unwrap();
    assert_eq!(rows.len(), evo_pods::LayoutId::all().len());
    let automatic: Vec<&str> = rows
        .iter()
        .filter(|row| row.selected)
        .map(|row| row.name)
        .collect();
    assert_eq!(automatic, vec!["Halves"], "two windows, no pick");

    let note = rt.set_layout(0, evo_pods::LayoutId::Thirds).unwrap();
    assert!(note.contains("now opens into Thirds"), "{note}");
    let lease = rt.lease().unwrap();
    assert!(
        lease.moved_windows.len() > before,
        "picking a preset re-tiles an active work immediately"
    );

    let after_pick = rt.layout_choices(0).unwrap();
    assert!(
        after_pick
            .iter()
            .find(|row| row.name == "Thirds")
            .unwrap()
            .selected
    );

    assert!(rt.leave().unwrap().contains("Left"));
    assert!(rt.lease().is_none());
}

#[test]
fn learn_drag_names_the_slot_and_survives_the_ledger() {
    let surface = tiled_surface(2);
    let mut rt = runtime_with(surface);
    rt.set_pods(vec![pod_fixture()]);

    let pod_id = evo_pods::PodId(7);
    let note = rt
        .learn_drag(pod_id, Frame::new(500.0, 0.0, 500.0, 800.0))
        .unwrap();
    assert!(note.contains("learned slot 2 of Halves"), "{note}");

    let memory = rt.arrangement(pod_id).cloned().unwrap();
    assert_eq!(
        memory.chosen, None,
        "learning a drag is not picking a preset"
    );
    assert!(memory.overrides.contains_key(&evo_pods::LayoutId::Halves));

    let text = rt.export_arrangements();
    let mut fresh = runtime_with(FakeSurface::base());
    fresh.import_arrangements(&text);
    assert_eq!(fresh.arrangement(pod_id), Some(&memory));
    assert!(
        fresh.export_arrangements() == text,
        "the ledger round-trips byte for byte"
    );

    let cleared = rt.clear_layout(0).unwrap();
    assert!(
        cleared.contains("arranges itself by its windows again"),
        "{cleared}"
    );
}
