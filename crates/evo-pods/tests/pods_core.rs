//! Platform-free proofs for the pod core: identity, stage math,
//! containment plans, lease reversal, gestures, suggestions, policies,
//! wrappers. Runtime behavior is proven in `runtime.rs` against a fake
//! surface.

mod fake;

use evo_pods::config::{ContainMode, PodConfig};
use evo_pods::dim::{self, WindowMatch};
use evo_pods::gesture::{self, PodGesture};
use evo_pods::lease::{LeaseStep, PodLease};
use evo_pods::policy;
use evo_pods::pod::{badges_from_bundle, BadgeKind, PodColor, PodSurfaces};
use evo_pods::stage::{self, DisplaySignature, Role};
use evo_pods::surface::Frame;
use evo_pods::suggest;
use evo_pods::wrapper;
use evo_threads::{Delta, ResumeBundle};
use fake::pod_fixture;

fn rt(x: f64, y: f64, w: f64, h: f64) -> Frame {
    Frame::new(x, y, w, h)
}

// ---- identity -----------------------------------------------------------

#[test]
fn color_is_deterministic_and_distinct_for_sequential_ids() {
    let colors: Vec<PodColor> = (1..=5u64).map(PodColor::from_thread).collect();
    for w in colors.windows(2) {
        assert_ne!(w[0], w[1], "sequential ids share a color");
    }
    assert_eq!(PodColor::from_thread(42), PodColor::from_thread(42));
}

#[test]
fn badges_come_only_from_engine_deltas() {
    let bundle = ResumeBundle {
        resume_point: "hero".into(),
        resume_reason: "test".into(),
        restore_set: vec![],
        open_deltas: vec![
            Delta::UnsavedEdits { resource: "a.rs".into() },
            Delta::DownloadedNotYetUsed { path: "x.zip".into() },
        ],
    };
    let badges = badges_from_bundle(&bundle);
    assert_eq!(badges.len(), 2);
    assert_eq!(badges[0].kind, BadgeKind::UnsavedEdits);
    assert_eq!(badges[1].kind, BadgeKind::DownloadReady);
}

// ---- stage math -----------------------------------------------------------

#[test]
fn roles_follow_resume_order_hero_first() {
    let pod = pod_fixture();
    let recipe = stage::derive_recipe(&pod.bundle, &pod.companions, DisplaySignature("s".into()), 3, 4, None);
    assert_eq!(recipe.roles.get("file:///repo/main.ts"), Some(&Role::Hero));
    assert_eq!(recipe.roles.get("https://docs.example/api"), Some(&Role::Satellite(0)));
    assert_eq!(recipe.roles.get("ref-notes.md"), Some(&Role::Rail(0)));
}

#[test]
fn placements_survive_more_controversial_rederive() {
    let mut recipe = stage::StageRecipe::new(DisplaySignature("s".into()));
    recipe.roles.insert("x".into(), Role::Hero);
    recipe.learn("x", &rt(10.0, 10.0, 500.0, 400.0), &rt(0.0, 0.0, 1000.0, 800.0));
    let bundle = ResumeBundle {
        resume_point: "x".into(),
        resume_reason: "r".into(),
        restore_set: vec!["y".into()],
        open_deltas: vec![],
    };
    let re = stage::derive_recipe(&bundle, &[], DisplaySignature("s2".into()), 3, 4, Some(&recipe));
    // Resource kept across a signature change: learned placement survives.
    assert!(re.placements.contains_key("x"));
    // ...but frame_for must never be consulted against the wrong signature.
    assert_eq!(re.signature, DisplaySignature("s2".into()));
}

#[test]
fn frame_math_roundtrips_through_fractions() {
    let screen = rt(100.0, 50.0, 1200.0, 800.0);
    let hero = Role::Hero.default_ratio().to_frame(&screen);
    let frac = evo_pods::stage::RatioRect::from_frame(&hero, &screen);
    let back = frac.to_frame(&screen);
    assert!((back.x - hero.x).abs() < 1e-6 && (back.w - hero.w).abs() < 1e-6);
}

#[test]
fn recipe_store_roundtrips_with_pod_names() {
    let mut recipe = stage::StageRecipe::new(DisplaySignature("sig".into()));
    recipe.roles.insert("r".into(), Role::Hero);
    recipe.placements.insert("r".into(), stage::RatioRect { x: 0.1, y: 0.1, w: 0.5, h: 0.5 });
    let text = stage::recipe_store::export(&[("pod-7".to_string(), recipe)]);
    let back = stage::recipe_store::import(&text);
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].0, "pod-7");
    assert!(back[0].1.placements.contains_key("r"));
}

// ---- containment -----------------------------------------------------------

#[test]
fn window_match_prefers_documents_over_pages_over_titles() {
    let surfaces = PodSurfaces {
        urls: vec!["https://tab.example".into()],
        documents: vec!["file:///repo/main.ts".into()],
        titles: vec!["Terminal".into()],
        apps: vec![],
        resource_apps: Default::default(),
    };
    let doc_win = fake::window(1, 10, "anything", Some("/repo/main.ts"));
    assert_eq!(dim::window_match(&doc_win, &surfaces), WindowMatch::Document);
    let page_win = fake::window(1, 11, "Tab", Some("https://tab.example"));
    assert_eq!(dim::window_match(&page_win, &surfaces), WindowMatch::Page);
    let title_win = fake::window(1, 12, "Terminal", None);
    assert_eq!(dim::window_match(&title_win, &surfaces), WindowMatch::Title);
    let foreign = fake::window(2, 13, "Else", None);
    assert_eq!(dim::window_match(&foreign, &surfaces), WindowMatch::None);
}

#[test]
fn dim_veils_foreign_and_arranges_own_under_containment() {
    let pod = pod_fixture();
    let screen = rt(0.0, 0.0, 1200.0, 800.0);
    let apps = vec![
        fake::app(1, "Editor", false),
        fake::app(2, "Mail", false),
        fake::app(3, "Evo", false),
    ];
    let hero = fake::window_with_frame(1, 10, "main.ts", Some("/repo/main.ts"), rt(0.0, 0.0, 1000.0, 800.0));
    let foreign = fake::window_with_frame(2, 20, "Inbox", None, rt(0.0, 0.0, 1000.0, 800.0));
    let protected = fake::window_with_frame(3, 30, "Evo Home", None, rt(0.0, 0.0, 1000.0, 800.0));
    let plan = dim::plan_activation(&pod.surfaces, None, &[hero, foreign, protected], &apps, &[3], ContainMode::Dim, Some(&screen));
    assert_eq!(plan.arrange.len(), 1);
    assert_eq!(plan.veil.len(), 1, "exactly the foreign window veils");
    assert_eq!(plan.veil[0].1, 20);
    assert!(plan.park.is_empty() && plan.hide_apps.is_empty());
}

#[test]
fn hide_mode_hides_only_foreign_regular_apps() {
    let pod = pod_fixture();
    let screen = rt(0.0, 0.0, 1200.0, 800.0);
    let apps = vec![fake::app(1, "Editor", false), fake::app(2, "Mail", false), fake::app(9, "System", false)];
    let mut system = apps.clone();
    system[2].regular = false;
    let hero = fake::window(1, 10, "main.ts", Some("/repo/main.ts"));
    let plan = dim::plan_activation(&pod.surfaces, None, &[hero], &system, &[], ContainMode::Hide, Some(&screen));
    assert_eq!(plan.hide_apps, vec![2]);
}

#[test]
fn stage_hides_alien_apps_and_parks_only_split_app_windows() {
    // The space semantic: an app witnessed both inside and outside the
    // work loses only its foreign windows (the platform's one per-window
    // verb); an app with no witness in the work hides whole, so its
    // windows never pile into the Dock.
    let surfaces = PodSurfaces {
        urls: vec!["https://tab.example".into()],
        documents: vec![],
        titles: vec![],
        apps: vec![],
        resource_apps: Default::default(),
    };
    let screen = rt(0.0, 0.0, 1200.0, 800.0);
    let apps = vec![fake::app(2, "Browser", false), fake::app(3, "Mail", false)];
    let pod_page = fake::window(2, 40, "Tab", Some("https://tab.example"));
    let foreign_same_app = fake::window(2, 41, "Other tab", None);
    let alien = fake::window(3, 50, "Inbox", None);
    let plan = dim::plan_activation(
        &surfaces, None, &[pod_page, foreign_same_app, alien], &apps, &[], ContainMode::Stage, Some(&screen));
    assert_eq!(plan.arrange.len(), 1, "only the witnessed page stages");
    assert_eq!(plan.park, vec![(2, 41)], "the split app's foreign window parks singly");
    assert_eq!(plan.hide_apps, vec![3], "the alien app hides whole — no Dock clutter");
    assert!(plan.veil.is_empty());
}

#[test]
fn stage_resurrects_the_pods_own_minimized_windows() {
    let pod = pod_fixture();
    let screen = rt(0.0, 0.0, 1200.0, 800.0);
    let apps = vec![fake::app(1, "Editor", false)];
    let mut parked_member = fake::window(1, 10, "main.ts", Some("/repo/main.ts"));
    parked_member.minimized = true;
    let plan = dim::plan_activation(
        &pod.surfaces, None, &[parked_member], &apps, &[], ContainMode::Stage, Some(&screen));
    assert_eq!(plan.unpark_members, vec![(1, 10)], "the space remembers its own parked window");
    assert_eq!(plan.arrange.len(), 1);
    assert!(plan.park.is_empty() && plan.hide_apps.is_empty(), "nothing foreign exists here");
}

#[test]
fn stage_is_the_default_containment() {
    assert_eq!(PodConfig::default().contain, ContainMode::Stage);
}

#[test]
fn reversal_reparks_members_last_after_frames_restore() {
    let mut lease = PodLease::new("w", 1);
    lease.parked_windows.push((2, 20));
    lease.moved_windows.push((1, 10, rt(1.0, 1.0, 100.0, 100.0)));
    lease.unparked_members.push((1, 10));
    let steps = lease.reversal();
    assert!(matches!(steps[0], LeaseStep::UnparkWindow { pid: 2, .. }));
    assert!(matches!(steps[1], LeaseStep::MoveWindowBack { pid: 1, .. }));
    assert!(
        matches!(steps[steps.len() - 1], LeaseStep::ReparkWindow { pid: 1, window_id: 10 }),
        "the space's windows return into the space LAST (moves need visibility)"
    );
}

// ---- leases -----------------------------------------------------------

#[test]
fn lease_reversal_is_exact_inverse_order() {
    let mut lease = PodLease::new("w", 1);
    lease.moved_windows.push((1, 10, rt(1.0, 1.0, 100.0, 100.0)));
    lease.parked_windows.push((2, 20));
    lease.veils.push(7);
    lease.hidden_apps.push(5);
    let steps = lease.reversal();
    assert!(matches!(steps[0], LeaseStep::CloseVeil { handle: 7 }));
    assert!(matches!(steps[1], LeaseStep::UnhideApp { pid: 5 }));
    assert!(matches!(steps[2], LeaseStep::UnparkWindow { pid: 2, .. }));
    assert!(matches!(steps[3], LeaseStep::MoveWindowBack { pid: 1, .. }));
}

// ---- gestures -----------------------------------------------------------

#[test]
fn merge_two_pods_emits_same_work_with_their_anchors() {
    let out = gesture::gesture_to_declaration(&PodGesture::MergePods {
        source_pod: 1,
        source_anchor: Some("a.rs".into()),
        target_pod: 2,
        target_anchor: Some("b.rs".into()),
    });
    assert_eq!(
        out,
        gesture::GestureOutcome::Declare(evo_threads::Declaration::SameWork {
            a: "a.rs".into(),
            b: "b.rs".into()
        })
    );
}

#[test]
fn merge_without_anchor_is_honestly_unbound() {
    let out = gesture::gesture_to_declaration(&PodGesture::MergePods {
        source_pod: 1,
        source_anchor: None,
        target_pod: 2,
        target_anchor: Some("b".into()),
    });
    assert!(matches!(out, gesture::GestureOutcome::Unbound(_)));
}

#[test]
fn eject_with_counterparty_separates_without_falls_to_quarantine() {
    let separated = gesture::gesture_to_declaration(&PodGesture::EjectWindow {
        subject: "stray".into(),
        source_pod: 1,
        source_anchor: Some("anchor".into()),
    });
    assert!(matches!(
        separated,
        gesture::GestureOutcome::Declare(evo_threads::Declaration::SeparateWork { .. })
    ));
    let quarantined = gesture::gesture_to_declaration(&PodGesture::EjectWindow {
        subject: "stray".into(),
        source_pod: 1,
        source_anchor: None,
    });
    assert!(matches!(
        quarantined,
        gesture::GestureOutcome::Declare(evo_threads::Declaration::NotMine { .. })
    ));
}

// ---- suggestions -----------------------------------------------------------

#[test]
fn suggest_only_above_floor_and_never_when_contested() {
    let mut pod_a = pod_fixture();
    pod_a.companions = vec![("subject".into(), 300_000.0)];
    let mut pod_b = pod_fixture();
    pod_b.id = evo_pods::PodId(99);
    pod_b.companions = vec![("subject".into(), 400_000.0)];
    let weak = pod_fixture();
    let config = PodConfig::default();

    assert!(suggest::suggest("subject", &[pod_a.clone(), weak], None, &config).is_some());
    assert!(suggest::suggest("subject", &[pod_a.clone(), pod_b], None, &config).is_none());
    assert!(suggest::suggest("subject", &[pod_a.clone()], Some(pod_a.id), &config).is_none());
    assert!(suggest::suggest("", &[pod_a], None, &config).is_none());
}

// ---- policy & wrapper (no hardcoding) -----------------------------------------------------------

#[test]
fn policies_parse_and_never_ship_by_default() {
    let text = "# evo-instance-policies-v1\napp=Fancy Editor|data_dir_flag=--user-data-dir=|extra_flag=--extensions-dir=\n";
    let policies = policy::parse_policies(text).unwrap();
    assert_eq!(policies.len(), 1);
    assert_eq!(
        policies[0].launch_args("/pods/pod-1"),
        vec!["--user-data-dir=/pods/pod-1", "--extensions-dir=/pods/pod-1"]
    );
    assert!(policy::default_policies().is_empty(), "no product knowledge ships in code");
    assert!(policy::parse_policies("bogus line").is_err());
}

#[test]
fn wrapper_generates_plist_launcher_and_identity_without_app_names() {
    let spec = wrapper::WrapperSpec {
        label: "Evo Pod — invoice-sync".into(),
        bundle_id: wrapper::normalize_bundle_id("dev.evo", "invoice-sync"),
        target_binary: "/Applications/X.app/Contents/MacOS/x".into(),
        target_args: vec!["--user-data-dir=/pods/pod-1".into()],
    };
    assert_eq!(spec.bundle_id, "dev.evo.invoice-sync");
    let plist = wrapper::info_plist(&spec);
    assert!(plist.contains("<string>dev.evo.invoice-sync</string>"));
    let script = wrapper::launcher_script(&spec);
    assert!(script.starts_with("#!/bin/sh"));
    assert!(script.contains("/Applications/X.app/Contents/MacOS/x"));
    assert!(script.contains("--user-data-dir=/pods/pod-1"));
}
