mod common;

use common::*;
use evo_threads::{
    CommitEvidence, Decision, Declaration, Engine, LifecycleEvidence, MemberState,
    MemberStateSource,
};

fn single_work() -> Engine {
    Engine::replay(&[
        evw(
            t(100),
            evo_threads::Interaction::Typed,
            "file:///work.md",
            200,
        ),
        ev(t(101), evo_threads::Interaction::Saved, "file:///work.md"),
        evw(
            t(1_500),
            evo_threads::Interaction::Typed,
            "file:///next.md",
            1,
        ),
    ])
}

#[test]
fn completed_member_is_not_a_default_resume_point_but_remains_in_trace() {
    let mut engine = single_work();
    let work = thread_with_anchor(&engine, "file:///work.md").expect("work should exist");

    engine.apply(declare(
        t(2_000),
        Declaration::MemberState {
            work: "file:///work.md".into(),
            member: "file:///work.md".into(),
            state: MemberState::Completed,
        },
    ));

    let plan = engine.activation_plan(work.id, false);
    assert!(plan.resources.is_empty());
    assert_eq!(plan.skipped.len(), 1);
    assert_eq!(plan.skipped[0].state, MemberState::Completed);
    assert!(engine.trace().iter().any(|decision| matches!(
        decision,
        Decision::MemberStateChanged { work, member, state }
            if work == "file:///work.md"
                && member == "file:///work.md"
                && *state == MemberState::Completed
    )));
}

#[test]
fn optional_member_is_only_omitted_in_reduced_mode() {
    let mut engine = single_work();
    let work = thread_with_anchor(&engine, "file:///work.md").expect("work should exist");

    engine.apply(declare(
        t(2_000),
        Declaration::MemberState {
            work: "file:///work.md".into(),
            member: "file:///work.md".into(),
            state: MemberState::Optional,
        },
    ));

    let normal = engine.activation_plan(work.id, false);
    let reduced = engine.activation_plan(work.id, true);
    assert!(normal.resources.contains(&"file:///work.md".to_string()));
    assert!(!reduced.resources.contains(&"file:///work.md".to_string()));
    assert!(reduced
        .skipped
        .iter()
        .any(|member| member.resource == "file:///work.md"));
}

#[test]
fn dismissal_hides_a_work_without_erasing_replay_state() {
    let mut engine = single_work();
    let work = thread_with_anchor(&engine, "file:///work.md").expect("work should exist");
    assert!(!engine.is_dismissed(work.id));

    engine.apply(declare(
        t(2_000),
        Declaration::WorkDismissed {
            work: "file:///work.md".into(),
        },
    ));

    assert!(engine.is_dismissed(work.id));
    assert_eq!(engine.threads().len(), 1, "dismissal must not delete work");
}

fn two_member_work() -> Engine {
    let mut engine = Engine::replay(&[
        evw(
            t(100),
            evo_threads::Interaction::Typed,
            "opaque-member-a",
            200,
        ),
        evw(
            t(101),
            evo_threads::Interaction::Typed,
            "opaque-member-b",
            200,
        ),
    ]);
    engine.settle(t(200));
    engine
}

#[test]
fn commit_evidence_is_ephemeral_and_derives_completed_from_an_opaque_relation() {
    let engine = two_member_work();
    let work = thread_with_anchor(&engine, "opaque-member-a").expect("work should exist");
    assert!(
        work.anchors
            .iter()
            .any(|anchor| anchor.resource == "opaque-member-b"),
        "fixture should have two members in one work"
    );

    let engine = engine.with_lifecycle_evidence(LifecycleEvidence::new([CommitEvidence::new(
        "opaque-commit",
        "opaque-member-b",
        t(300),
    )]));
    let plan = engine.activation_plan(work.id, false);
    assert!(!plan.resources.iter().any(|r| r == "opaque-member-b"));
    assert_eq!(
        plan.skipped
            .iter()
            .find(|member| member.resource == "opaque-member-b")
            .map(|member| member.source),
        Some(MemberStateSource::Derived)
    );

    // The relationship is a replay input, not a new canonical decision or a
    // persisted state mutation. Repeated reads produce the same trace.
    let first_trace = engine.trace();
    let second_trace = engine.trace();
    assert_eq!(first_trace, second_trace);
    assert!(first_trace.iter().any(|decision| matches!(
        decision,
        Decision::MemberStateDerived { member, commit, .. }
            if member == "opaque-member-b" && commit == "opaque-commit"
    )));
}

#[test]
fn declared_state_outranks_commit_derived_completion() {
    let mut engine =
        two_member_work().with_lifecycle_evidence(LifecycleEvidence::new([CommitEvidence::new(
            "opaque-commit",
            "opaque-member-b",
            t(300),
        )]));
    let work = thread_with_anchor(&engine, "opaque-member-a").expect("work should exist");
    engine.apply(declare(
        t(400),
        Declaration::MemberState {
            work: "opaque-member-a".into(),
            member: "opaque-member-b".into(),
            state: MemberState::Active,
        },
    ));

    let plan = engine.activation_plan(work.id, false);
    assert!(plan.resources.iter().any(|r| r == "opaque-member-b"));
    assert!(!plan
        .skipped
        .iter()
        .any(|member| member.resource == "opaque-member-b"));
    assert!(!engine.trace().iter().any(|decision| matches!(
        decision,
        Decision::MemberStateDerived { member, .. } if member == "opaque-member-b"
    )));
}

#[test]
fn stale_member_is_inferred_completed_only_after_a_sibling_returns() {
    let mut engine = Engine::replay(&[
        evw(
            t(100),
            evo_threads::Interaction::Typed,
            "opaque-member-a",
            200,
        ),
        evw(
            t(101),
            evo_threads::Interaction::Typed,
            "opaque-member-b",
            200,
        ),
        // Far enough away to create a later episode, but the anchor still
        // provides the deterministic assignment back to the same work.
        evw(
            t(1_000),
            evo_threads::Interaction::Typed,
            "opaque-member-a",
            200,
        ),
    ]);
    engine.settle(t(1_100));
    let work = thread_with_anchor(&engine, "opaque-member-a").expect("work should exist");
    let inferred_plan = engine.activation_plan(work.id, false);
    assert!(!inferred_plan
        .resources
        .iter()
        .any(|r| r == "opaque-member-b"));
    assert_eq!(
        inferred_plan
            .skipped
            .iter()
            .find(|member| member.resource == "opaque-member-b")
            .map(|member| member.source),
        Some(MemberStateSource::Inferred)
    );
    assert!(engine.trace().iter().any(|decision| matches!(
        decision,
        Decision::MemberStateInferred { member, sibling, .. }
            if member == "opaque-member-b" && sibling == "opaque-member-a"
    )));

    // A commit-derived state outranks that weaker inference for the same
    // member, while remaining below a declaration (covered above).
    engine.set_lifecycle_evidence(LifecycleEvidence::new([CommitEvidence::new(
        "opaque-commit",
        "opaque-member-b",
        t(900),
    )]));
    let derived_plan = engine.activation_plan(work.id, false);
    assert_eq!(
        derived_plan
            .skipped
            .iter()
            .find(|member| member.resource == "opaque-member-b")
            .map(|member| member.source),
        Some(MemberStateSource::Derived)
    );
    assert!(engine.trace().iter().any(|decision| matches!(
        decision,
        Decision::MemberStateDerived { member, commit, .. }
            if member == "opaque-member-b" && commit == "opaque-commit"
    )));
}
