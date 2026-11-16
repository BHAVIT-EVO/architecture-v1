//! Phase-2: the Work Brief. `Engine::brief`/`briefs` assemble what a work
//! *is* from witnessed facts only — anchors, context, resume, open deltas —
//! and Phase 3's narration speaks from them. The contract pinned here:
//! complete for a produced work, honest-empty for an unknown id, ordered
//! exactly like the rack (`threads()`), and deterministic across replays.

mod common;

use common::{ev, evw, t};
use evo_threads::{Engine, Interaction};

fn manuscript_day() -> Vec<evo_threads::Event> {
    vec![
        evw(t(100), Interaction::Focused, "file:///manuscript.md", 1_000),
        evw(t(101), Interaction::Typed, "file:///manuscript.md", 300),
        ev(t(102), Interaction::Saved, "file:///manuscript.md"),
        // Consulted mid-flow: earns context, never identity.
        evw(
            t(103),
            Interaction::Focused,
            "https://ref.example/style-guide",
            60_000,
        ),
        evw(t(110), Interaction::Typed, "file:///manuscript.md", 80),
        ev(t(111), Interaction::Saved, "file:///manuscript.md"),
    ]
}

/// Replay + settle, the daemon read path's own two steps: the log folds,
/// then "now" is fixed one day past the last event — past `gap_close`, so
/// every sitting is closed and every work is minted — the same boundary the
/// committed-corpus gate and the regression binary use.
fn replay_settled(log: &[evo_threads::Event]) -> Engine {
    let mut e = Engine::replay(log);
    let last = log.last().map(|ev| ev.timestamp).unwrap_or(0);
    e.settle(last + 86_400_000);
    e
}

#[test]
fn brief_states_a_work_from_witnessed_facts_only() {
    let e = replay_settled(&manuscript_day());
    let briefs = e.briefs();
    assert_eq!(briefs.len(), 1, "one sitting with production is one work");
    let b = &briefs[0];
    assert!(
        b.anchors
            .iter()
            .any(|a| a.resource == "file:///manuscript.md"
                && a.kind == evo_threads::AnchorKind::Mutation),
        "the produced document is identity, and its kind is witnessed"
    );
    assert!(
        b.companions.iter().all(|r| r != "file:///manuscript.md"),
        "identity never double-reports as context"
    );
    assert_eq!(b.episodes, 1);
    assert_eq!(b.resume_point, "file:///manuscript.md");
    assert!(
        !b.resume_reason.is_empty(),
        "the one honest sentence exists"
    );
    assert!(
        b.contested.is_empty() && b.fork.is_none(),
        "an unambiguous work carries no ambiguity"
    );
}

#[test]
fn briefs_order_matches_the_rack_and_are_deterministic() {
    let mut log = manuscript_day();
    // A second, smaller work on another day.
    log.extend([
        evw(t(1500), Interaction::Focused, "file:///recipe.md", 1_000),
        evw(t(1501), Interaction::Typed, "file:///recipe.md", 120),
        ev(t(1502), Interaction::Saved, "file:///recipe.md"),
    ]);
    let e = replay_settled(&log);
    let briefs = e.briefs();
    let rack = e.threads();
    assert_eq!(briefs.len(), rack.len());
    for (b, tv) in briefs.iter().zip(rack.iter()) {
        assert_eq!(b.thread_id, tv.id, "briefs order == threads order");
    }
    // Replay determinism: same log, same briefs.
    let again = replay_settled(&log);
    assert_eq!(briefs, again.briefs());
}

#[test]
fn brief_of_unknown_id_is_honestly_empty() {
    let e = replay_settled(&manuscript_day());
    let b = e.brief(999);
    assert_eq!(b.thread_id, 999);
    assert!(b.anchors.is_empty());
    assert!(b.companions.is_empty());
    assert!(b.restore_set.is_empty());
    assert!(b.open_deltas.is_empty());
    assert!(b.resume_point.is_empty());
    assert_eq!(b.episodes, 0);
    assert!(b.fork.is_none());
}

#[test]
fn brief_carries_witnessed_open_deltas() {
    // Typed after the last save: the UnsavedEdits delta must reach the brief.
    let log = vec![
        evw(t(100), Interaction::Focused, "file:///notes.md", 1_000),
        ev(t(101), Interaction::Saved, "file:///notes.md"),
        evw(t(105), Interaction::Typed, "file:///notes.md", 50),
    ];
    let e = replay_settled(&log);
    let briefs = e.briefs();
    let has_unsaved = briefs
        .iter()
        .any(|b| b.open_deltas.iter().any(|d| matches!(d, evo_threads::Delta::UnsavedEdits { resource } if resource == "file:///notes.md")));
    assert!(has_unsaved, "the witnessed unsaved-edits loop is briefed");
}
