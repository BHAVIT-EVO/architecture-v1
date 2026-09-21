//! Fixture (a) — the scripted classic day.
//!
//! 9:00–11:30 work A (editor + terminal + docs; edits, commits)
//! 12:00–13:00 work B (browser form + resume doc; one save)
//! 14:00–16:00 work C (PDFs + notes; edits)
//! One browser in all three; one repo shared by A and C; a 90-second
//! notification detour inside A; an always-focused music player.

mod common;
use common::*;
use evo_threads::*;

fn fixture_a_events() -> Vec<Event> {
    let mut v = Vec::new();
    // ---- Work A, 9:00–11:00 (t=540..660) ----------------------------------
    v.push(evw(t(540), Interaction::Focused, "app:music", 120_000));
    v.push(ev(t(540), Interaction::Focused, "app:browser"));
    v.push(ev(t(541), Interaction::Focused, "file:///repo/src/main.rs"));
    v.push(evw(t(542), Interaction::Typed, "file:///repo/src/main.rs", 60));
    v.push(ev(t(543), Interaction::Saved, "file:///repo/src/main.rs"));
    // The browser, the docs, and the terminal join the working flow — and
    // leave evidence inside it.
    v.push(ev(t(555), Interaction::Focused, "app:browser"));
    v.push(ev(t(556), Interaction::Visited, "https://docs.example/rustbook"));
    v.push(evw(t(557), Interaction::Focused, "https://docs.example/rustbook", 180_000));
    v.push(ev(t(558), Interaction::Focused, "term:1"));
    v.push(ev(t(559), Interaction::CommandRan, "term:1"));
    v.push(ev(t(560), Interaction::CommandRan, "term:1"));
    v.push(evw(t(562), Interaction::Typed, "file:///repo/src/main.rs", 80));
    v.push(ev(t(563), Interaction::Saved, "file:///repo/src/main.rs"));
    // A 90-second notification detour inside the flow — absorbed, never
    // attributed.
    v.push(ev(t(565), Interaction::Focused, "https://social.example/feed"));
    v.push(ev(t(565) + 30_000, Interaction::Visited, "https://social.example/feed"));
    v.push(evw(t(566) + 30_000, Interaction::Typed, "file:///repo/src/main.rs", 60));
    // A commit, mid-flow: session content, not a new work.
    v.push(ev(t(570), Interaction::Focused, "file:///repo"));
    v.push(ev(t(571), Interaction::Saved, "file:///repo"));
    // Ambient music, then a second commit.
    v.push(evw(t(650), Interaction::Focused, "app:music", 120_000));
    v.push(ev(t(659), Interaction::Focused, "file:///repo"));
    v.push(ev(t(660), Interaction::Saved, "file:///repo"));
    // ---- Lunch: nothing until 12:00 (t=720) --------------------------------
    // ---- Work B, 12:00–13:00: a form, the browser, the resume, one save ----
    v.push(ev(t(720), Interaction::Visited, "https://careers.example/apply"));
    v.push(ev(t(721), Interaction::Focused, "app:browser"));
    v.push(evw(t(722), Interaction::Typed, "https://careers.example/apply", 40));
    v.push(ev(t(723), Interaction::Focused, "app:browser"));
    v.push(ev(t(724), Interaction::Focused, "file:///resume.pdf"));
    v.push(ev(t(725), Interaction::Saved, "file:///resume.pdf"));
    v.push(evw(t(745), Interaction::Focused, "app:music", 30_000));
    // ---- Work C, 14:00–16:00: papers, notes, the SAME repo -----------------
    v.push(ev(t(840), Interaction::Visited, "https://arxiv.example/abs/2401.0001"));
    v.push(ev(t(841), Interaction::Visited, "https://arxiv.example/abs/2402.0002"));
    v.push(evw(t(842), Interaction::Typed, "file:///notes.md", 100));
    v.push(ev(t(843), Interaction::Focused, "app:browser"));
    v.push(evw(t(845), Interaction::Focused, "app:music", 30_000));
    v.push(ev(t(850), Interaction::Saved, "file:///notes.md"));
    // The shared repo earns a commit inside the same sitting — same resource,
    // other work.
    v.push(ev(t(855), Interaction::Focused, "file:///repo"));
    v.push(ev(t(856), Interaction::Saved, "file:///repo"));
    // The day ends quietly; this closing event also bounds the last episode.
    v.push(evw(t(1_380), Interaction::Typed, "file:///journal.md", 40));
    v
}

#[test]
fn fixture_a_classic_day() {
    let e = Engine::replay(&fixture_a_events());
    let threads = e.threads();
    assert_eq!(threads.len(), 3, "exactly three bodies of work, no more");

    let a = thread_with_anchor(&e, "file:///repo/src/main.rs").expect("work A");
    let b = thread_with_anchor(&e, "file:///resume.pdf").expect("work B");
    let c = thread_with_anchor(&e, "file:///notes.md").expect("work C");
    assert!(a.id != b.id && b.id != c.id && a.id != c.id);

    // One browser served all three works; it stayed a resource, not a glue.
    for tv in [&a, &b, &c] {
        assert!(
            tv.episodes.iter().any(|ep| ep
                .participants
                .iter()
                .any(|(r, _)| r == "app:browser")),
            "the browser attends work {}",
            tv.id
        );
    }

    // The shared repo is anchored in A and C, displayed as contested on both,
    // and the threads never merged.
    assert!(a.anchors.iter().any(|x| x.resource == "file:///repo"));
    assert!(c.anchors.iter().any(|x| x.resource == "file:///repo"));
    assert!(a.contested.contains(&"file:///repo".to_string()));
    assert!(c.contested.contains(&"file:///repo".to_string()));
    assert!(!trace_has(&e, |d| matches!(d, Decision::ThreadMerged { .. })));

    // The music player attends everything and restores nothing.
    for tv in [&a, &b, &c] {
        let bundle = e.resume_bundle(tv.id);
        assert!(!bundle.restore_set.contains(&"app:music".to_string()));
    }
    assert!(all_anchor_resources(&e).iter().all(|r| r != "app:music"));

    // Restore sets are small, last-episode-only, and pointed at production.
    let ba = e.resume_bundle(a.id);
    assert!(ba.restore_set.len() <= 5);
    assert_eq!(ba.resume_point, "file:///repo/src/main.rs");
    assert!(ba.restore_set.contains(&"file:///repo/src/main.rs".to_string()));
    assert!(ba.restore_set.contains(&"file:///repo".to_string()));
    assert!(ba.restore_set.contains(&"https://docs.example/rustbook".to_string()));

    let bb = e.resume_bundle(b.id);
    assert!(bb.restore_set.len() <= 5);
    assert!(bb.restore_set.contains(&"file:///resume.pdf".to_string()));

    let bc = e.resume_bundle(c.id);
    assert!(bc.restore_set.len() <= 5);
    assert_eq!(bc.resume_point, "file:///notes.md");
    assert!(bc.restore_set.contains(&"file:///repo".to_string()));

    // The notification detour: kept as context, cast no votes.
    let a_ep = &a.episodes[0];
    assert_eq!(a_ep.interruptions.len(), 1);
    assert!(!a_ep.participants.iter().any(|(r, _)| r.contains("social")));

    // Every reasoning step is inspectable.
    assert!(!e.explain(a.id).is_empty());
    assert!(trace_has(&e, |d| matches!(
        d,
        Decision::AnchorContested { resource, .. } if resource == "file:///repo"
    )));
}

#[test]
fn fixture_a_is_deterministic() {
    let log = fixture_a_events();
    assert_eq!(Engine::replay(&log), Engine::replay(&log));
}
