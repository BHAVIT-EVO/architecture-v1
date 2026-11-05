//! Phase-3 scenario tests (PLAN-0001 §4): a user classification gates founding
//! and titling, and deletes nothing.
//!
//! `OBS-SURFACE-CLASSIFIED` is the person's own word about a surface — "this is
//! not work". The engine must honour it strictly, and only in the way RFC-0014
//! and Architecture §5 allow: a class never removes a subject from a body of
//! work (the non-deletion guarantee holds), it only stops the subject *founding*
//! or *naming* one. Evo ships no default class, so these tests are the only
//! place a class ever appears — exactly as on a real machine.

mod common;

use common::{all_anchor_resources, declare, ev, evw, t, thread_with_anchor};
use evo_threads::{Declaration, Decision, Engine, Interaction};

/// A resource that looks like leisure video. The engine knows nothing about
/// what it is — that is the point; the person classifies it, not Evo.
const VIDEO: &str = "https://www.youtube.com/watch?v=EsW2BoFAUs";

/// A resource the same machine really worked on, so the log is not one-sided.
const NOTES: &str = "file:///work/notes.md";

fn classify(ts: u64, resource: &str, class: &str) -> evo_threads::Event {
    declare(
        ts,
        Declaration::Classified {
            resource: resource.to_string(),
            class: class.to_string(),
        },
    )
}

/// A log that founds exactly one work on the video, closed by a later tap.
fn founding_log() -> Vec<evo_threads::Event> {
    vec![
        evw(t(100), Interaction::Focused, VIDEO, 120_000),
        evw(t(101), Interaction::Typed, VIDEO, 200),
        ev(t(102), Interaction::Saved, VIDEO),
        // The sitting bounds at the clock with a weight-1 tap, which founds
        // nothing of its own.
        evw(t(3_000), Interaction::Typed, NOTES, 1),
    ]
}

#[test]
fn a_later_classification_demotes_an_existing_anchor() {
    // First, the work exists and the video is what carries it.
    let before = Engine::replay(&founding_log());
    assert!(
        thread_with_anchor(&before, VIDEO).is_some(),
        "the work is founded on the video before any classification"
    );

    // Now the person says it is not work.
    let mut log = founding_log();
    log.push(classify(t(3_100), VIDEO, "NotWork"));
    let after = Engine::replay(&log);

    assert!(
        thread_with_anchor(&after, VIDEO).is_none(),
        "a classified surface must not anchor a work"
    );
    assert!(
        !all_anchor_resources(&after).contains(&VIDEO.to_string()),
        "no work carries the classified surface as an anchor"
    );

    // Non-deletion: the subject is still witnessed somewhere in the engine —
    // a class is not a quarantine.
    assert!(
        after.threads().iter().any(|thread| {
            thread.companions.iter().any(|(r, _)| r == VIDEO)
                || thread
                    .episodes
                    .iter()
                    .any(|ep| ep.participants.iter().any(|(r, _)| r == VIDEO))
        }),
        "a class must never remove the subject from the work"
    );

    // And the decision is on the record, inspectable like every other.
    assert!(after
        .trace()
        .iter()
        .any(|d| matches!(d, Decision::SurfaceClassified { resource, .. } if resource == VIDEO)));
}

#[test]
fn a_surface_classified_before_production_never_founds_a_work() {
    let mut log = vec![classify(t(50), VIDEO, "NotWork")];
    log.extend(founding_log());

    let e = Engine::replay(&log);
    assert!(
        thread_with_anchor(&e, VIDEO).is_none(),
        "a pre-classified surface must never become an anchor, however much it produces"
    );
    assert!(!all_anchor_resources(&e).contains(&VIDEO.to_string()));
}

#[test]
fn a_classification_outranks_a_later_anchor_declaration() {
    // The person classifies the video, then declares it one work with notes.
    // The class wins for the video; the declaration still anchors the other
    // side — the pair is not silently discarded.
    let log = vec![
        classify(t(50), VIDEO, "NotWork"),
        declare(
            t(60),
            Declaration::SameWork {
                a: VIDEO.to_string(),
                b: NOTES.to_string(),
            },
        ),
    ];
    let e = Engine::replay(&log);
    assert!(
        thread_with_anchor(&e, VIDEO).is_none(),
        "a classified surface cannot be declared into identity"
    );
    assert!(
        thread_with_anchor(&e, NOTES).is_some(),
        "the unclassified side of the declaration still anchors"
    );
}

#[test]
fn a_classification_is_ordered_and_idempotent() {
    // Replaying the same classification twice changes nothing, and a second
    // class on the same subject simply replaces the first.
    let mut log = founding_log();
    log.push(classify(t(3_100), VIDEO, "NotWork"));
    log.push(classify(t(3_200), VIDEO, "Media"));
    let once = Engine::replay(&log);

    let mut twice = log.clone();
    twice.push(classify(t(3_300), VIDEO, "Media"));
    let twice = Engine::replay(&twice);

    assert_eq!(once.threads(), twice.threads());
    assert!(thread_with_anchor(&once, VIDEO).is_none());
}
