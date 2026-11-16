//! End-to-end journeys over the public security surface: the five paths a new
//! person actually walks — first run, first wake, opening a recent work,
//! pausing, and a refused permission.
//!
//! These are integration tests: they import `evo_desktop` the way the app does
//! and never reach into private state.

use evo_desktop::security::KillSwitch;
use evo_desktop::security::activity::{ActivityLog, Undo, UndoKind};
use evo_desktop::security::permissions::{
    Action, Decision, DenyReason, Mode, PermissionSet, Scope,
};
use std::path::Path;

const HOME: &str = "/Users/me";
const DOCS: &str = "/Users/me/Documents";
const NOTE: &str = "/Users/me/Documents/Q3 roadmap.md";

#[test]
fn first_run_starts_with_nothing_allowed_and_grants_one_scope_at_a_time() {
    // Day one: the default mode, no scopes. Every request is refused.
    let mut set = PermissionSet::default();
    assert_eq!(set.mode(), Mode::AskForRisky, "the calm default");
    assert_eq!(
        set.decide(KillSwitch::new().context(), Action::Open, NOTE),
        Decision::Deny(DenyReason::OutOfScope)
    );

    // Onboarding grants Documents. Opening is now quiet work.
    set.allow(Scope::parse("folder", DOCS).expect("a real folder"));
    assert_eq!(
        set.decide(KillSwitch::new().context(), Action::Open, NOTE),
        Decision::Allow
    );

    // Everything outside that one folder is still refused.
    assert_eq!(
        set.decide(
            KillSwitch::new().context(),
            Action::Read,
            "/Users/me/Desktop/secret.txt"
        ),
        Decision::Deny(DenyReason::OutOfScope)
    );
}

#[test]
fn first_wake_reaches_the_listener_then_opens_the_newest_work() {
    // The scope the wake-listener path needs: the work titles live in the
    // store, so the folder holding it is granted, nothing else.
    let mut set = PermissionSet::new(Mode::AskForRisky);
    set.allow(Scope::parse("folder", DOCS).expect("folder"));

    let switch = KillSwitch::new();
    assert!(!switch.is_paused());
    assert!(
        set.decide(switch.context(), Action::Read, NOTE)
            .is_allowed()
    );

    // The choice is remembered, then opened on confirmation.
    let mut log = ActivityLog::default();
    log.record(
        1_700_000_000,
        "Opened “Q3 roadmap”",
        "You said “Hey Evo”",
        None,
    );
    assert_eq!(log.len(), 1);
    assert!(
        set.decide(switch.context(), Action::Open, NOTE)
            .is_allowed()
    );
}

#[test]
fn opening_a_recent_work_asks_for_nothing_but_risky_work_asks() {
    let mut set = PermissionSet::new(Mode::AskForRisky);
    set.allow(Scope::parse("folder", DOCS).expect("folder"));
    let ctx = KillSwitch::new().context();

    assert_eq!(set.decide(ctx, Action::Open, NOTE), Decision::Allow);
    assert_eq!(set.decide(ctx, Action::Read, NOTE), Decision::Allow);
    assert_eq!(set.decide(ctx, Action::Write, NOTE), Decision::Ask);
    assert_eq!(set.decide(ctx, Action::Run, NOTE), Decision::Ask);

    // Act-within-scope is the mode that stops asking.
    set.set_mode(Mode::ActWithinScope);
    assert_eq!(set.decide(ctx, Action::Write, NOTE), Decision::Allow);
    // Ask-every-time asks even for opening.
    set.set_mode(Mode::AskEveryTime);
    assert_eq!(set.decide(ctx, Action::Open, NOTE), Decision::Ask);
}

#[test]
fn pausing_stops_every_journey_and_resuming_restores_them() {
    let switch = KillSwitch::new();
    let mut set = PermissionSet::new(Mode::ActWithinScope);
    set.allow(Scope::parse("folder", DOCS).expect("folder"));

    assert!(
        set.decide(switch.context(), Action::Open, NOTE)
            .is_allowed()
    );
    assert!(switch.toggle(), "first press pauses");
    assert_eq!(
        set.decide(switch.context(), Action::Open, NOTE),
        Decision::Deny(DenyReason::Paused)
    );
    assert_eq!(
        set.decide(switch.context(), Action::Run, NOTE),
        Decision::Deny(DenyReason::Paused)
    );

    switch.resume();
    assert!(
        set.decide(switch.context(), Action::Open, NOTE)
            .is_allowed()
    );
}

#[test]
fn a_refused_permission_stays_refused_and_can_be_undone_only_once() {
    // The user grants Documents, refuses Downloads.
    let mut set = PermissionSet::new(Mode::AskForRisky);
    set.allow(Scope::parse("folder", DOCS).expect("folder"));
    assert_eq!(
        set.decide(
            KillSwitch::new().context(),
            Action::Read,
            "/Users/me/Downloads/x.pdf"
        ),
        Decision::Deny(DenyReason::OutOfScope)
    );

    // A hidden work is reversible exactly once, and Activity records why.
    let mut log = ActivityLog::default();
    log.record(
        1_700_000_100,
        "Hid “Old draft”",
        "You asked Home to hide it",
        Some(Undo {
            kind: UndoKind::Hide,
            id: "old-draft".into(),
        }),
    );
    let undo = log.undo(0).expect("the hide is reversible");
    assert_eq!(undo.id, "old-draft");
    assert!(log.undo(0).is_none(), "and only once");
}

#[test]
fn sites_and_apps_are_scoped_as_narrowly_as_folders() {
    let mut set = PermissionSet::new(Mode::ActWithinScope);
    set.allow(Scope::parse("site", "claude.ai").expect("host"));
    set.allow(Scope::parse("app", "Claude").expect("app name"));
    let ctx = KillSwitch::new().context();

    assert!(set.decide(ctx, Action::Read, "docs.claude.ai").is_allowed());
    assert_eq!(
        set.decide(ctx, Action::Read, "claude.ai.evil.example"),
        Decision::Deny(DenyReason::OutOfScope)
    );
    assert!(set.decide(ctx, Action::Open, "Claude").is_allowed());
    assert_eq!(
        set.decide(ctx, Action::Open, "Terminal"),
        Decision::Deny(DenyReason::OutOfScope)
    );

    // Revoking takes effect immediately.
    assert!(set.revoke("site", "claude.ai"));
    assert_eq!(
        set.decide(ctx, Action::Read, "docs.claude.ai"),
        Decision::Deny(DenyReason::OutOfScope)
    );
}

#[test]
fn a_traversal_in_the_journey_is_refused_before_any_file_is_opened() {
    assert!(
        evo_desktop::security::paths::resolve_within(Path::new(DOCS), "../../etc/passwd").is_err(),
        "a work title cannot walk out of its folder"
    );
    assert!(
        evo_desktop::security::paths::resolve_within(Path::new(DOCS), "notes.md").is_ok(),
        "an ordinary relative work stays inside"
    );
    let _ = HOME;
}
