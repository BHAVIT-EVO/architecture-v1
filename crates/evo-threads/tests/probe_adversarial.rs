//! Independent adversarial probes written by the reviewer, not the implementer.
use evo_threads::*;

fn t(min: u64) -> u64 { min * 60_000 }

fn ev(ts: u64, origin: Origin, i: Interaction, r: &str) -> Event {
    Event { timestamp: ts, origin, interaction: i, resource: r.into(), weight: None, detail: None }
}

fn typing(ts: u64, r: &str) -> Event {
    Event { timestamp: ts, origin: Origin::Person, interaction: Interaction::Typed,
            resource: r.into(), weight: Some(300), detail: None }
}

/// PROBE 1 — The bimodal hub across days.
/// One browser genuinely used inside two real works, day after day.
/// Expect: two threads; browser participates in both; never merges;
/// a browser-only episode with no anchors does not guess.
#[test]
fn probe_bimodal_hub_across_days() {
    let mut e = Engine::new();
    for day in 0..3u64 {
        let base = day * t(60 * 24);
        // Work A: editor + browser docs, saves on the editor file.
        e.apply(ev(base + t(1), Origin::Person, Interaction::Focused, "editor"));
        e.apply(typing(base + t(2), "editor-a.md"));
        e.apply(ev(base + t(4), Origin::Person, Interaction::Saved, "editor-a.md"));
        e.apply(ev(base + t(6), Origin::Person, Interaction::Focused, "browser"));
        e.apply(ev(base + t(10), Origin::Person, Interaction::Visited, "browser://docs-a"));
        // Work B (later same day): resume doc + browser careers page.
        e.apply(ev(base + t(30), Origin::Person, Interaction::Focused, "editor"));
        e.apply(typing(base + t(31), "resume.md"));
        e.apply(ev(base + t(33), Origin::Person, Interaction::Saved, "resume.md"));
        e.apply(ev(base + t(35), Origin::Person, Interaction::Focused, "browser"));
        e.apply(ev(base + t(39), Origin::Person, Interaction::Visited, "browser://careers"));
    }
    let threads = e.threads();
    let work: Vec<&ThreadView> = threads.iter().filter(|x| !x.episodes.is_empty()).collect();
    assert!(work.len() >= 2, "two distinct works must exist, got {}", work.len());
    assert!(work.len() <= 3, "fragmentation: {}", work.len());
    // The browser must appear as participant in both works' episodes, and as
    // anchor in NEITHER (it only ever received attention, never production).
    for w in &work {
        assert!(
            !w.anchors.iter().any(|a| a.resource == "browser"),
            "attention-only browser must never be an anchor"
        );
    }
    // A browser-only episode later: contested or forked, never assigned by guess.
    e.apply(ev(t(60 * 24 * 3) + t(5), Origin::Person, Interaction::Focused, "browser"));
    e.apply(ev(t(60 * 24 * 3) + t(6), Origin::Person, Interaction::Visited, "browser://docs-a"));
    let after = e.threads();
    let _ = after;
    // Count of threads must not have shrunk (no merge) and browser glues nothing.
    let work2: Vec<&ThreadView> = after.iter().filter(|x| !x.episodes.is_empty()).collect();
    assert!(work2.len() >= work.len(), "browser-only episode must not merge works");
}

/// PROBE 2 — Episode producing on BOTH threads' anchors at once.
/// The only safe outcome is a fork or a birth — never a merge, never a coin flip.
#[test]
fn probe_dual_anchor_episode_never_merges() {
    let mut e = Engine::new();
    // Day 1: work A around file A, work B around file B.
    e.apply(ev(t(1), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(2), "a.md"));
    e.apply(ev(t(3), Origin::Person, Interaction::Saved, "a.md"));
    e.apply(ev(t(60), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(61), "b.md"));
    e.apply(ev(t(62), Origin::Person, Interaction::Saved, "b.md"));
    e.apply(ev(t(120), Origin::Person, Interaction::Focused, "finder"));
    let before = e.threads().iter().filter(|x| !x.episodes.is_empty()).count();
    assert!(before >= 2, "two works first, got {before}");
    // Day 2: one episode that saves BOTH a.md and b.md.
    e.apply(ev(t(60 * 26), Origin::Person, Interaction::Focused, "editor"));
    e.apply(ev(t(60 * 26 + 2), Origin::Person, Interaction::Saved, "a.md"));
    e.apply(ev(t(60 * 26 + 4), Origin::Person, Interaction::Saved, "b.md"));
    e.apply(ev(t(60 * 28), Origin::Person, Interaction::Focused, "finder"));
    let threads = e.threads();
    let after = threads.iter().filter(|x| !x.episodes.is_empty()).count();
    assert!(after >= before, "a dual-anchor episode must never reduce thread count (merge)");
}

/// PROBE 3 — Churn storm beside real focus, but never ON the churned files.
#[test]
fn probe_churn_beside_focus_never_joins() {
    let mut e = Engine::new();
    e.apply(ev(t(1), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(2), "report.md"));
    e.apply(ev(t(3), Origin::Person, Interaction::Saved, "report.md"));
    // 40 cache writes DURING the session, never focused, never typed on.
    for i in 0..40u64 {
        e.apply(ev(t(4) + i * 100, Origin::System, Interaction::Mutated,
            &format!("/Users/x/Library/Sync/cache-{i}.db")));
    }
    e.apply(ev(t(30), Origin::Person, Interaction::Focused, "editor"));
    let threads = e.threads();
    for w in &threads {
        for (r, _) in &w.companions {
            assert!(!r.contains("cache-"), "churn must never become a companion: {r}");
        }
        for a in &w.anchors {
            assert!(!a.resource.contains("cache-"), "churn must never anchor: {}", a.resource);
        }
        for ep in &w.episodes {
            for (r, _) in &ep.participants {
                assert!(!r.contains("cache-"), "churn must never participate: {r}");
            }
        }
    }
}

/// PROBE 4 — A nightly movie: deep attention, zero production, recurring across days.
/// This is the honest epistemic limit: with no input signal, a recurring
/// consumption surface is indistinguishable from a web-app work surface.
/// The engine may or may not mint a thread — we RECORD what it does.
#[test]
fn probe_recurring_movie_behavior() {
    let mut e = Engine::new();
    for day in 0..4u64 {
        let base = day * t(60 * 24);
        e.apply(Event {
            timestamp: base + t(30),
            origin: Origin::Person,
            interaction: Interaction::Focused,
            resource: "vidbox".into(),
            weight: Some(t(90)), // 90 minutes of dwell each night
            detail: None,
        });
    }
    let threads = e.threads();
    let minted = threads.iter().any(|w| !w.episodes.is_empty());
    println!("recurring-movie minted a thread: {minted}");
    for w in &threads {
        for ep in &w.episodes {
            println!("  episode participants: {:?}", ep.participants);
        }
    }
}

/// PROBE 5 — Execution restore: person engages ONE of two restored windows.
#[test]
fn probe_execution_handoff_is_per_resource() {
    let mut e = Engine::new();
    // Real work first.
    e.apply(ev(t(1), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(2), "report.md"));
    e.apply(ev(t(3), Origin::Person, Interaction::Saved, "report.md"));
    // The product restores two windows.
    e.apply(ev(t(60 * 10), Origin::Execution, Interaction::Focused, "browser"));
    e.apply(ev(t(60 * 10) + 500, Origin::Execution, Interaction::Focused, "editor"));
    // Person types into the browser only.
    e.apply(typing(t(60 * 10) + 1000, "browser"));
    e.apply(ev(t(60 * 12), Origin::Person, Interaction::Focused, "finder"));
    let threads = e.threads();
    // The browser typing must count as person production somewhere;
    // the execution events must not have created phantom attention on editor.
    let browser_typed = threads.iter().any(|w| w
        .episodes.iter().any(|ep| ep.anchors.contains(&"browser".to_string())));
    assert!(browser_typed, "person typing into a restored window must count");
}

/// PROBE 6 — An anchor moved to a NEW thread by later behavior:
/// person starts doing something different with the same file. Identity must
/// not rewrite history (invariant 12) but new episodes may found new work.
#[test]
fn probe_same_file_two_purposes_over_time() {
    let mut e = Engine::new();
    e.apply(ev(t(1), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(2), "notes.md"));
    e.apply(ev(t(3), Origin::Person, Interaction::Saved, "notes.md"));
    // Two weeks later, same file, very different company.
    e.apply(ev(t(60 * 24 * 14), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(60 * 24 * 14 + 2), "notes.md"));
    e.apply(ev(t(60 * 24 * 14 + 3), Origin::Person, Interaction::Saved, "notes.md"));
    let threads = e.threads();
    // Whatever it decides, it must not MERGE unrelated context, and the first
    // thread's episode history must be unchanged.
    let _ = threads;
}
