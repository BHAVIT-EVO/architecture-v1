//! Fixture (b) — the ground-truth day, rendered from the founder's labeled
//! recall. Icon research → demo work → pitch deck → shopping → demo again →
//! a long film → pitch & waitlist interleaved within minutes → provider
//! research → a brief messaging burst to end the day.
//!
//! The assertions are the founder's own: at least 2 and at most 4 works;
//! the film and the shopping never exist as far as work is concerned; the
//! messaging burst hijacks nothing; each bundle reopens what the day
//! actually consisted of.

mod common;
use common::*;
use evo_threads::*;

const ICON_A: &str = "https://icons.example/set/a";
const ICON_B: &str = "https://icons.example/set/b";
const ICON_C: &str = "https://icons.example/set/c";
const GEMINI_DEMO: &str = "https://gemini.example/app/demo";
const CLAUDE_DEMO: &str = "https://claude.example/chat/demo";
const INDEX_HTML: &str = "file:///Users/founder/Desktop/indexxx.html";
const BUILDER: &str = "https://builder.example/editor/make";
const CLI: &str = "app:cli-tool";
const GEN_ZIP: &str = "file:///Users/founder/Downloads/gen-export.zip";
const PITCH: &str = "https://app.pitch.example/presentation/deck-911";
const SHOP_1: &str = "https://shop.example/laptop-1";
const SHOP_2: &str = "https://shop.example/laptop-2";
const MOVIE: &str = "https://movies.example/watch/some-film";
const FRAMER: &str = "https://framer.example/projects/waitlist-home";
const NOTION: &str = "https://www.notion.example/waitlist-plan";
const DRIBBBLE: &str = "https://dribbble.example/shots/waitlist";
const MARKET: &str = "https://marketplace.example/creators";
const YT: &str = "https://www.youtube.example/watch?v=launch-inspo";
const GEMINI_DEBUG: &str = "https://gemini.example/app/waitlist-debug";
const PROVIDER_A: &str = "https://providers.example/resend";
const PROVIDER_B: &str = "https://providers.example/loops";
const MSG: &str = "app:messaging";

fn fixture_b_events() -> Vec<Event> {
    let mut v = Vec::new();
    // 09:30–09:55 — browsing laptop icons. Context; nobody's production.
    v.push(ev(t(570), Interaction::Visited, ICON_A));
    v.push(ev(t(575), Interaction::Visited, ICON_B));
    v.push(ev(t(590), Interaction::Visited, ICON_A));
    v.push(ev(t(592), Interaction::Visited, ICON_C));
    // 09:55–11:40 — the demo: AI-chat surfaces with real typing, saves on
    // indexxx.html, an HTML builder, a CLI tool that visibly changed, and a
    // download.
    v.push(evw(t(595), Interaction::Focused, GEMINI_DEMO, 30_000));
    v.push(evw(t(596), Interaction::Typed, GEMINI_DEMO, 80));
    v.push(evw(t(600), Interaction::Focused, CLAUDE_DEMO, 30_000));
    v.push(evw(t(601), Interaction::Typed, CLAUDE_DEMO, 60));
    v.push(evw(t(609), Interaction::Typed, INDEX_HTML, 40));
    v.push(ev(t(610), Interaction::Saved, INDEX_HTML));
    v.push(ev(t(611), Interaction::Saved, INDEX_HTML));
    v.push(ev(t(613), Interaction::Saved, INDEX_HTML));
    v.push(evw(t(620), Interaction::Focused, BUILDER, 30_000));
    v.push(evw(t(621), Interaction::Typed, BUILDER, 60));
    v.push(ev(t(630), Interaction::Focused, CLI));
    v.push(ev(t(631), Interaction::CommandRan, CLI));
    v.push(sys(t(632), CLI)); // churn-passing: the person was right there
    v.push(ev(t(640), Interaction::Downloaded, GEN_ZIP));
    v.push(evw(t(650), Interaction::Focused, GEMINI_DEMO, 30_000));
    v.push(evw(t(651), Interaction::Typed, GEMINI_DEMO, 60));
    // 11:40–12:00 — the pitch deck surface. Consumption today; a work's
    // surface returning across occasions tomorrow (Typed capture pending).
    v.push(evw(t(700), Interaction::Focused, PITCH, 120_000));
    v.push(ev(t(702), Interaction::Visited, PITCH));
    v.push(evw(t(705), Interaction::Focused, PITCH, 120_000));
    v.push(evw(t(710), Interaction::Focused, PITCH, 60_000));
    // 12:00–12:10 — laptop shopping, once, never again.
    v.push(ev(t(720), Interaction::Visited, SHOP_1));
    v.push(ev(t(722), Interaction::Visited, SHOP_2));
    v.push(ev(t(725), Interaction::Visited, SHOP_1));
    // 12:10–12:50 — back to the demo.
    v.push(evw(t(730), Interaction::Focused, GEMINI_DEMO, 10_000));
    v.push(evw(t(731), Interaction::Typed, GEMINI_DEMO, 40));
    v.push(ev(t(740), Interaction::Saved, INDEX_HTML));
    v.push(evw(t(741), Interaction::Typed, INDEX_HTML, 20));
    v.push(evw(t(750), Interaction::Focused, CLAUDE_DEMO, 10_000));
    v.push(evw(t(751), Interaction::Typed, CLAUDE_DEMO, 30));
    // 12:50–13:45 — a film. Fifty-five minutes of focus, zero input, no
    // recurrence, no configuration with anything.
    v.push(evw(t(770), Interaction::Focused, MOVIE, 3_300_000));
    // 13:45–16:00 — pitch deck and waitlist work interleaved within minutes.
    v.push(evw(t(825), Interaction::Focused, PITCH, 60_000));
    v.push(ev(t(826), Interaction::Visited, PITCH));
    v.push(evw(t(830), Interaction::Focused, FRAMER, 30_000));
    v.push(evw(t(831), Interaction::Typed, FRAMER, 90));
    v.push(evw(t(833), Interaction::Focused, PITCH, 60_000));
    v.push(evw(t(835), Interaction::Focused, NOTION, 30_000));
    v.push(evw(t(836), Interaction::Typed, NOTION, 40));
    v.push(ev(t(840), Interaction::Visited, DRIBBBLE));
    v.push(evw(t(845), Interaction::Focused, FRAMER, 10_000));
    v.push(evw(t(846), Interaction::Typed, FRAMER, 60));
    v.push(ev(t(850), Interaction::Visited, MARKET));
    v.push(evw(t(855), Interaction::Focused, PITCH, 120_000));
    v.push(ev(t(860), Interaction::Visited, YT));
    // 16:00–21:00 — waitlist proper, including an AI-chat debugging session
    // (a different conversation surface; the engine neither knows nor cares).
    v.push(evw(t(960), Interaction::Focused, FRAMER, 30_000));
    v.push(evw(t(961), Interaction::Typed, FRAMER, 60));
    v.push(evw(t(980), Interaction::Focused, NOTION, 10_000));
    v.push(evw(t(981), Interaction::Typed, NOTION, 20));
    v.push(evw(t(1000), Interaction::Focused, GEMINI_DEBUG, 30_000));
    v.push(evw(t(1001), Interaction::Typed, GEMINI_DEBUG, 80));
    v.push(ev(t(1020), Interaction::Visited, YT));
    v.push(evw(t(1100), Interaction::Focused, FRAMER, 120_000));
    // 21:00–22:54 — provider research + framer returns.
    v.push(ev(t(1260), Interaction::Visited, PROVIDER_A));
    v.push(ev(t(1300), Interaction::Visited, PROVIDER_B));
    v.push(evw(t(1350), Interaction::Focused, FRAMER, 60_000));
    // 22:54 — a few messages to the cofounder, sent, done.
    v.push(ev(t(1374), Interaction::Focused, MSG));
    v.push(evw(t(1375), Interaction::Typed, MSG, 30));
    v.push(ev(t(1376), Interaction::Saved, MSG));
    v
}

#[test]
fn fixture_b_ground_truth_day() {
    let e = Engine::replay(&fixture_b_events());
    let threads = e.threads();
    assert!(
        (2..=4).contains(&threads.len()),
        "expected 2..=4 works, found {}",
        threads.len()
    );

    let demo = thread_with_anchor(&e, INDEX_HTML).expect("demo work");
    let waitlist = thread_with_anchor(&e, FRAMER).expect("waitlist work");
    assert_ne!(demo.id, waitlist.id);

    // The film does not exist as far as work is concerned.
    assert!(!leaks_into_work(&e, "https://movies.example"));
    assert!(trace_has(&e, |d| matches!(
        d,
        Decision::EpisodeParked { reason: ParkReason::ConsumptionOnly, .. }
    )));
    // Neither does the shopping, nor the pre-work icon browsing.
    assert!(!leaks_into_work(&e, "https://shop.example"));
    assert!(!leaks_into_work(&e, "https://icons.example"));

    // Interleaving within minutes did not smear pitch/waitlist into the demo.
    for ep in &demo.episodes {
        assert!(!ep.participants.iter().any(|(r, _)| r == FRAMER));
        assert!(!ep.participants.iter().any(|(r, _)| r == NOTION));
        assert!(!ep.participants.iter().any(|(r, _)| r == PITCH));
    }
    // The demo's AI-chat surface and the waitlist's debugging surface stayed
    // distinct works' evidence; nothing is contested here.
    assert!(demo.contested.is_empty());
    assert!(waitlist.contested.is_empty());
    assert!(waitlist.anchors.iter().any(|a| a.resource == GEMINI_DEBUG));
    assert!(!demo.anchors.iter().any(|a| a.resource == GEMINI_DEBUG));

    // The pitch surface, returning across occasions, stays inside the
    // waitlist work's identity — as a recurrence anchor or as company; what
    // it must never be is its own work, or the demo's.
    let pitch_in_waitlist = waitlist
        .companions
        .iter()
        .any(|(r, _)| r == PITCH)
        || waitlist.anchors.iter().any(|a| a.resource == PITCH);
    assert!(pitch_in_waitlist);
    assert!(!demo.anchors.iter().any(|a| a.resource == PITCH));
    assert!(!demo.companions.iter().any(|(r, _)| r == PITCH));

    // Restore bundles reopen what the day consisted of, no more than 5.
    let bd = e.resume_bundle(demo.id);
    assert!(bd.restore_set.len() <= 5);
    assert_eq!(bd.resume_point, INDEX_HTML);
    assert!(bd.restore_set.contains(&GEMINI_DEMO.to_string()));
    assert!(bd.restore_set.contains(&CLAUDE_DEMO.to_string()));
    assert!(!bd.restore_set.contains(&MOVIE.to_string()));

    let bw = e.resume_bundle(waitlist.id);
    assert!(bw.restore_set.len() <= 5);
    assert_eq!(bw.resume_point, FRAMER);
    assert!(bw.restore_set.contains(&NOTION.to_string()));
    assert!(!bw.restore_set.contains(&MOVIE.to_string()));

    // The messaging burst hijacked nothing: no work's resume point, no work's
    // anchors, no work's restore set mentions it.
    assert!(all_anchor_resources(&e).iter().all(|r| r != MSG));
    for tv in &threads {
        let b = e.resume_bundle(tv.id);
        assert_ne!(b.resume_point, MSG);
        assert!(!b.restore_set.contains(&MSG.to_string()));
    }

    // The founder's stated ideal next-morning surface for the overall work —
    // the thing being built, the AI-chat surfaces, the related tabs — is
    // reachable from the union of the bundles.
    let union = all_restore_resources(&e);
    for want in [INDEX_HTML, GEMINI_DEMO, CLAUDE_DEMO] {
        assert!(union.contains(&want.to_string()), "missing {want}");
    }

    // Open deltas report witnessed state only.
    assert!(bd
        .open_deltas
        .contains(&Delta::DownloadedNotYetUsed { path: GEN_ZIP.to_string() }));
    assert!(bd
        .open_deltas
        .contains(&Delta::UncommittedChanges { root: CLI.to_string() }));
    // The sent messages leave no dangling delta anywhere.
    for tv in &threads {
        assert!(!e.resume_bundle(tv.id).open_deltas.iter().any(|d| matches!(
            d,
            Delta::UnsavedEdits { resource } | Delta::UnfinishedDraftSurface { resource }
                if resource == MSG
        )));
    }
}

#[test]
fn fixture_b_is_deterministic() {
    let log = fixture_b_events();
    assert_eq!(Engine::replay(&log), Engine::replay(&log));
}

#[test]
fn fixture_b_settles_without_surprises() {
    // The daemon's nightly read: settle bounds the day's last sitting. The
    // messaging reflex is parked below the birth floor; nothing else moves.
    let mut e = Engine::replay(&fixture_b_events());
    assert_eq!(e.threads().len(), 2);
    e.settle(t(1_376) + 3 * 3_600_000);
    let threads = e.threads();
    assert_eq!(threads.len(), 2, "settling mints nothing new");
    assert!(trace_has(&e, |d| matches!(
        d,
        Decision::EpisodeParked { reason: ParkReason::WeakProduction, .. }
    )));
    for tv in &threads {
        assert_ne!(e.resume_bundle(tv.id).resume_point, MSG);
    }
}
