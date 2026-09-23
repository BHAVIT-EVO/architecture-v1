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
