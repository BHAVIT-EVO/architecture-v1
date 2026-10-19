//! Phase-2 scenario tests: the founder's shadow gates as engine scenarios.
//!
//! `crates/evo-ledger/src/thread_gate.rs` scores the engine over the real
//! log; these tests replay the same *stories* as synthetic fixtures, so the
//! gate semantics are pinned on every machine without the log. Each test is
//! named for the gate or rule it protects.
//!
//! The rule under test — **establishment precedence** (the engine mirror of
//! the ledger's `INTERLEAVE_DWELL_CAP_MS`): configuration may carry a
//! production-free sitting into a neighboring work only when the sitting is a
//! *fragment* of that work — every resource new to the work holds less than
//! the establishment floor (`Config::establishment_dwell_floor`, the ledger's
//! 120 s) of attention. Substantive attention on territory the work has never
//! carried is not a fragment: the sitting is never swallowed by the neighbor.
//! That is the "kundli chat stays out of the gear work" story, structural.
//!
//! Fixture mechanics (verified with the probe): an excursion from the
//! focused surface is buffered as a detour and rejoins the sitting when the
//! person returns to focused ground — a short excursion resolves as an
//! interruption, a long one feeds its material into the working set as
//! participants. A production-free tail far past the sitting bounds it
//! without adding production (production would move the assignment out of
//! the config path by design).

mod common;

use common::{ev, evw, t, t_sec};
use evo_threads::{Interaction, ThreadView};

const GEAR_TITLE: &str =
    "Unisex Hiking & Trekking Clothes | Arc'teryx United States - Google Chrome – BHAVIT";
const GEAR_URL: &str = "https://arcteryx.com/us/en/c/mens/shells";
const KUNDLI_CHAT: &str = "Complete kundli analysis and remedies - Claude - Google Chrome – BHAVIT";
const KUNDLI_SEARCH: &str = "https://www.google.com/search?q=kundli+remedies+for+saturn+transit";
const TREK_V1: &str =
    "https://www.youtube.com/watch?v=R7qSra0aNGo&list=PLiM-TFJI81R_X4HUrRDjwSJmK-MpqC1dW&index=7";
const TREK_V2: &str =
    "https://www.youtube.com/watch?v=TuyshY0fZ94&list=PLiM-TFJI81R_X4HUrRDjwSJmK-MpqC1dW&index=18";
const UNRELATED: &str = "https://www.youtube.com/watch?v=eswW2BoFAUs";

/// Minutes:seconds past midnight -> timestamp, for sitting-internal timing.
fn ts(min: u64, sec: u64) -> u64 {
    t_sec(min, sec)
}

fn thread_of(tv: &[ThreadView], resource: &str) -> Option<u64> {
    tv.iter()
        .find(|t| {
            t.anchors.iter().any(|a| a.resource == resource)
                || t.episodes
                    .iter()
                    .any(|ep| ep.participants.iter().any(|(r, _)| r == resource))
        })
        .map(|t| t.id)
}

fn anchors_gear_work() -> Vec<evo_threads::Event> {
    // Day 1: the gear work establishes — a browsing sitting on the store
    // with production on the gear page. The store URL leaves as the work's
    // known ground (a visited, produced-beside surface — neither an
    // always-on player nor fresh territory).
    vec![
        evw(t(600), Interaction::Focused, GEAR_URL, 1_000),
        ev(ts(600, 1), Interaction::Visited, GEAR_URL),
        evw(ts(610, 0), Interaction::Typed, GEAR_TITLE, 200),
        ev(ts(610, 5), Interaction::Saved, GEAR_TITLE),
    ]
}

/// Day 2: one production-free browsing sitting — back on the store (known
/// ground, long enough after the ubiquity discount to clear the config
/// floor), then an excursion through a kundli search into the kundli chat
/// (`chat_dwell_ms` on the chat). The return lands exactly `dip_max` (5 min)
/// after the excursion began: the excursion is stale, so its material joins
/// the sitting as participants instead of resolving as a discarded dip. A
/// far, brief, terminal tail bounds the sitting without production.
fn kundli_day(chat_dwell_ms: u64) -> Vec<evo_threads::Event> {
    let mut log = anchors_gear_work();
    log.extend([
        evw(ts(1500, 0), Interaction::Visited, GEAR_URL, 90_000),
        evw(ts(1500, 90), Interaction::Visited, KUNDLI_SEARCH, 60_000),
        evw(
            ts(1500, 150),
            Interaction::Visited,
            KUNDLI_CHAT,
            chat_dwell_ms,
        ),
        // 300 s after the excursion began: stale by exactly dip_max.
        evw(ts(1500, 390), Interaction::Visited, GEAR_URL, 30_000),
        evw(t(1600), Interaction::Typed, "file:///unrelated-day2.md", 1),
    ]);
    log
}

#[test]
fn scenario_brief_consultation_is_still_a_fragment() {
    // The chat holds 90 seconds — below the establishment floor, as is the
    // search. The whole sitting is a fragment of the gear work: every
    // resource new to the work is sub-floor, so configuration carries the
    // sitting in, the kundli material riding as company. No production ever
    // happened on it, so it can never anchor. The boundary under test is
    // the floor, not the mechanism.
    let e = evo_threads::Engine::replay(&kundli_day(90_000));
    let gear = thread_of(&e.threads(), GEAR_TITLE).expect("gear work exists");
    let gear_view = e.threads().into_iter().find(|t| t.id == gear).unwrap();
    assert!(
        gear_view
            .episodes
            .iter()
            .any(|ep| ep.participants.iter().any(|(r, _)| r == KUNDLI_CHAT)),
        "a sub-floor consultation rides borrowed context into the work"
    );
    assert!(
        gear_view.anchors.iter().all(|a| a.resource != KUNDLI_CHAT),
        "consulted company decorates; it never becomes identity"
    );
}

#[test]
fn scenario_substantive_sitting_is_not_a_fragment() {
    // The same sitting, but the chat holds 5 minutes — at the establishment
    // floor. That is substantive engagement with something else, not a
    // fragment of the gear work: the establishment-precedence filter refuses
    // the config carry, and the sitting never joins the gear work (it stands
    // on its own — parked or its own small work — never swallowed by the
    // neighbor). This is the "kundli chat stays out of the gear work" gate,
    // made structural.
    let e = evo_threads::Engine::replay(&kundli_day(300_000));
    let gear = thread_of(&e.threads(), GEAR_TITLE).expect("gear work exists");
    let gear_view = e.threads().into_iter().find(|t| t.id == gear).unwrap();
    assert_eq!(
        gear_view.episodes.len(),
        1,
        "the substantive sitting never joins the gear work"
    );
    assert!(
        gear_view
            .episodes
            .iter()
            .all(|ep| ep.participants.iter().all(|(r, _)| r != KUNDLI_CHAT))
            && gear_view.anchors.iter().all(|a| a.resource != KUNDLI_CHAT)
            && gear_view.companions.iter().all(|(r, _)| r != KUNDLI_CHAT),
        "the kundli chat is no part of the gear work — not company, not identity"
    );
}

#[test]
fn scenario_trek_playlist_is_one_work() {
    // Occasion 1 births the trek work (production on V1). Occasion 2 is one
    // browsing sitting — light production back at the already-anchored V1
    // (anchor support carries the occasion despite the new production),
    // with V2 watched in the middle and resolved back into the sitting by
    // the return. An unrelated video watched alone, days later, never joins
    // and never anchors.
    let log = vec![
        evw(t(300), Interaction::Focused, TREK_V1, 120_000),
        evw(ts(300, 125), Interaction::Typed, TREK_V1, 150),
        // Next day: one sitting — a V2 excursion that outlasts dip_max
        // (320 s), so its material joins the sitting, then the return.
        evw(ts(1500, 0), Interaction::Visited, TREK_V1, 40_000),
        evw(ts(1500, 40), Interaction::Typed, TREK_V1, 60),
        evw(ts(1500, 45), Interaction::Visited, TREK_V2, 320_000),
        evw(ts(1500, 365), Interaction::Visited, TREK_V1, 30_000),
        // Day 3: an unrelated video, alone, no production.
        evw(t(2500), Interaction::Focused, UNRELATED, 200_000),
        evw(t(2700), Interaction::Typed, "file:///day3.md", 1),
    ];
    let e = evo_threads::Engine::replay(&log);
    let trek = thread_of(&e.threads(), TREK_V1).expect("trek work exists");
    let trek_view = e.threads().into_iter().find(|t| t.id == trek).unwrap();
    assert_eq!(trek_view.episodes.len(), 2, "both occasions join one work");
    assert!(
        trek_view
            .episodes
            .iter()
            .any(|ep| ep.participants.iter().any(|(r, _)| r == TREK_V2)),
        "V2 is company of the trek work"
    );
    assert!(
        e.threads()
            .iter()
            .all(|t| t.anchors.iter().all(|a| a.resource != UNRELATED)),
        "the unrelated video anchors nothing"
    );
    assert!(
        e.threads().iter().all(|t| t
            .episodes
            .iter()
            .all(|ep| { ep.participants.iter().all(|(r, _)| r != UNRELATED) })),
        "the unrelated video joins no work"
    );
}

#[test]
fn scenario_gear_work_is_exactly_one() {
    // Two gear sittings on different days, both producing on the same page:
    // anchor support keeps them one work — not two.
    let mut log = anchors_gear_work();
    log.extend([
        evw(ts(1500, 0), Interaction::Focused, GEAR_TITLE, 90_000),
        evw(ts(1500, 90), Interaction::Typed, GEAR_TITLE, 120),
    ]);
    let e = evo_threads::Engine::replay(&log);
    let gear_threads = e
        .threads()
        .into_iter()
        .filter(|t| t.anchors.iter().any(|a| a.resource == GEAR_TITLE))
        .count();
    assert_eq!(gear_threads, 1, "the gear work is exactly one");
}
