//! Regression tests for the three integration fixes, each traceable to a
//! failure observed on the founder's real corpus:
//! 1. `settle()` — the never-closing final episode (bit me twice in review).
//! 2. Download dedup — one file downloaded three times minted three anchors
//!    and hijacked the resume point of the top thread on the real corpus.
//! 3. Detour-buffer loss across a long silence — material buffered during an
//!    excursion was silently discarded when the gap fallback closed the
//!    episode, contradicting the code's own comment.
use evo_threads::*;

fn t(min: u64) -> u64 { min * 60_000 }

fn ev(ts: u64, origin: Origin, i: Interaction, r: &str) -> Event {
    Event { timestamp: ts, origin, interaction: i, resource: r.into(), weight: None, detail: None }
}

fn typing(ts: u64, r: &str) -> Event {
    Event { timestamp: ts, origin: Origin::Person, interaction: Interaction::Typed,
            resource: r.into(), weight: Some(300), detail: None }
}

/// settle() closes a stale open episode so threads() can see it.
#[test]
fn settle_makes_the_last_episode_visible() {
    let mut e = Engine::new();
    e.apply(ev(t(1), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(2), "report.md"));
    e.apply(ev(t(3), Origin::Person, Interaction::Saved, "report.md"));
    // No further events. Without settle, the episode stays open and
    // unassigned; threads() sees nothing.
    let before = e.threads().iter().filter(|x| !x.episodes.is_empty()).count();
    e.settle(t(60)); // an hour later, well past gap_close
    let after = e.threads().iter().filter(|x| !x.episodes.is_empty()).count();
    assert!(after > before, "settle must close and assign the stale episode: {before} -> {after}");
    // Idempotence: settling again changes nothing.
    let again = e.threads();
    e.settle(t(90));
    assert_eq!(e.threads().len(), again.len());
}

/// settle() within the gap does NOT close a live episode.
#[test]
fn settle_inside_the_gap_keeps_the_episode_open() {
    let mut e = Engine::new();
    e.apply(ev(t(1), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(2), "report.md"));
    e.settle(t(5)); // 3 minutes later, inside gap_close
    let open_episode_invisible = e.threads().iter().filter(|x| !x.episodes.is_empty()).count();
    assert_eq!(open_episode_invisible, 0, "a live sitting must not be closed early");
}

/// Repeated downloads of one resource credit once.
#[test]
fn repeated_downloads_credit_once() {
    let mut e = Engine::new();
    e.apply(ev(t(1), Origin::Person, Interaction::Focused, "browser"));
    e.apply(typing(t(2), "notes.md"));
    e.apply(ev(t(3), Origin::Person, Interaction::Saved, "notes.md"));
    e.apply(ev(t(4), Origin::Person, Interaction::Downloaded, "file.pdf"));
    e.apply(ev(t(5), Origin::Person, Interaction::Downloaded, "file.pdf"));
    e.apply(ev(t(6), Origin::Person, Interaction::Downloaded, "file.pdf"));
    e.settle(t(60));
    let threads = e.threads();
    let total_anchor_strength: f64 = threads.iter()
        .flat_map(|t| t.anchors.iter())
        .filter(|a| a.resource == "file.pdf")
        .map(|a| a.strength)
        .sum();
    assert_eq!(total_anchor_strength, 80.0,
        "one download credit (80) total, got {total_anchor_strength}");
}

/// The real-corpus failure: triple download must not dominate the resume set.
#[test]
fn repeated_downloads_do_not_hijack_the_resume_point() {
    let mut e = Engine::new();
    // Real work with real typing.
    e.apply(ev(t(1), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(2), "report.md"));
    e.apply(typing(t(4), "report.md"));
    e.apply(ev(t(6), Origin::Person, Interaction::Saved, "report.md"));
    // One file downloaded three times at the end of the day.
    e.apply(ev(t(50), Origin::Person, Interaction::Focused, "browser"));
    e.apply(ev(t(51), Origin::Person, Interaction::Downloaded, "doc.pdf"));
    e.apply(ev(t(52), Origin::Person, Interaction::Downloaded, "doc.pdf"));
    e.apply(ev(t(53), Origin::Person, Interaction::Downloaded, "doc.pdf"));
    e.settle(t(120));
    let top = e.threads().into_iter().find(|t| !t.episodes.is_empty());
    let Some(t) = top else { panic!("no thread minted") };
    let b = e.resume_bundle(t.id);
    // The dominant production is the typing+save (300+300+60=660 vs one 80
    // download credit), so the resume point must be the document, not the PDF.
    assert_eq!(b.resume_point, "report.md",
        "resume point hijacked by repeat downloads: {}", b.resume_point);
}

/// Material buffered during an excursion must survive a long silence:
/// the excursion was a departure, and its material deserves the content-first
/// re-route the code comment promises.
#[test]
fn detour_material_survives_a_long_gap() {
    let mut e = Engine::new();
    // Sitting 1: work on the report.
    e.apply(ev(t(1), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(2), "report.md"));
    e.apply(ev(t(3), Origin::Person, Interaction::Saved, "report.md"));
    // Excursion begins: attention moves to a different document, typed on.
    e.apply(ev(t(4), Origin::Person, Interaction::Focused, "editor"));
    e.apply(typing(t(5), "other-work.md"));
    // ... then silence long enough to cross gap_close with the detour pending.
    // The next event is a day later on unrelated material.
    e.apply(ev(t(60 * 26), Origin::Person, Interaction::Focused, "mail"));
    e.settle(t(60 * 27));
    // The excursion's production must exist somewhere: other-work.md was
    // typed on with real weight; discarding it would be silent loss.
    let typed_somewhere = e.threads().iter().any(|t| t.anchors.iter()
        .any(|a| a.resource == "other-work.md"));
    let in_trace = e.trace().iter().any(|d| {
        matches!(d, Decision::EpisodeAssigned { .. } | Decision::ThreadBorn { .. } | Decision::EpisodeParked { .. })
    });
    assert!(typed_somewhere || in_trace,
        "excursion material typed during a detour must survive the gap close");
    assert!(typed_somewhere,
        "'other-work.md' had real production; it must be an anchor of some thread");
}
