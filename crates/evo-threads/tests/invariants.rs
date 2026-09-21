//! The fourteen invariants, written as executable product promises.
//! If any of these ever fails, the engine has broken its word to the user.

mod common;
use common::*;
use evo_threads::*;

// ----------------------------------------------------------------------
// 1. Determinism: same log, same state — bit for bit, fold for fold.
// ----------------------------------------------------------------------
#[test]
fn invariant_01_determinism_and_fold_equivalence() {
    let log = vec![
        evw(t(100), Interaction::Typed, "file:///a.md", 200),
        ev(t(101), Interaction::Saved, "file:///a.md"),
        evw(t(140), Interaction::Typed, "file:///b.md", 200),
        ev(t(141), Interaction::Saved, "file:///b.md"),
        evw(t(500), Interaction::Typed, "file:///a.md", 150),
        ev(t(600), Interaction::Saved, "file:///c.md"),
        sys(t(101), "file:///noise.bin"),
    ];
    let e1 = Engine::replay(&log);
    let e2 = Engine::replay(&log);
    assert_eq!(e1, e2, "two replays of the same log must be identical");
    assert_eq!(format!("{:?}", e1), format!("{:?}", e2));

    // Any slice, any fold split.
    for cut in 0..=log.len() {
        let mut manual = Engine::new();
        for ev in &log[..cut] {
            manual.apply(ev.clone());
        }
        let mut replayed_prefix = Engine::replay(&log[..cut]);
        assert_eq!(manual, replayed_prefix, "replay != fold(apply) at cut {cut}");
        for ev in &log[cut..] {
            manual.apply(ev.clone());
            replayed_prefix.apply(ev.clone());
        }
        assert_eq!(manual, e1, "refixing the tail must reproduce full state");
    }
}

// ----------------------------------------------------------------------
// 2. No System-origin sequence can create or extend a work.
// ----------------------------------------------------------------------
#[test]
fn invariant_02_system_origin_is_never_work() {
    let mut log = vec![
        sys(t(100), "file:///var/cache/svc/01.bin"),
        sys(t(101), "file:///var/cache/svc/02.bin"),
        Event {
            timestamp: t(102),
            origin: Origin::System,
            interaction: Interaction::Focused,
            resource: "app:finder".into(),
            weight: None,
            detail: None,
        },
        sys(t(103), "file:///var/cache/svc/03.bin"),
    ];
    let e = Engine::replay(&log);
    assert!(e.threads().is_empty());
    assert!(!trace_has(&e, |d| matches!(d, Decision::ThreadBorn { .. })));

    // And a long system-only tail cannot extend a real thread's evidence.
    log.push(evw(t(200), Interaction::Typed, "file:///real.md", 300));
    log.push(ev(t(201), Interaction::Saved, "file:///real.md"));
    for i in 0..30 {
        log.push(sys(t(300 + i), "file:///var/cache/svc/tail.bin"));
    }
    log.push(evw(t(400), Interaction::Typed, "file:///closer.md", 1));
    let e = Engine::replay(&log);
    assert_eq!(e.threads().len(), 1);
    let real = thread_with_anchor(&e, "file:///real.md").unwrap();
    for ep in &real.episodes {
        assert!(!ep.participants.iter().any(|(r, _)| r.contains("svc")));
    }
}

// ----------------------------------------------------------------------
// 3. The product never observes itself; the handoff is causal, not timed.
// ----------------------------------------------------------------------
#[test]
fn invariant_03_execution_is_silent_until_the_person_touches_it() {
    let prefix = vec![
        exe(t(100), Interaction::Focused, "https://restored.example/a"),
        sys(t(101), "file:///cache/x"),
    ];
    let e = Engine::replay(&prefix);
    assert!(
        e.threads().is_empty(),
        "a restore plus background noise must mint nothing"
    );

    // One second later (not eight), the person types into the restored thing.
    let mut log = prefix.clone();
    log.push(evw(t(101) + 1_000, Interaction::Typed, "https://restored.example/a", 120));
    // Hours later an unrelated tap: the sitting closes by the clock and the
    // work inside it stands up.
    log.push(evw(t(400), Interaction::Typed, "file:///unrelated.md", 1));
    let e = Engine::replay(&log);
    let threads = e.threads();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].anchors[0].resource, "https://restored.example/a");

    // The handoff is recorded, and it precedes the birth it enabled.
    let tr = e.trace();
    let handoff_pos = tr
        .iter()
        .position(|d| matches!(d, Decision::Handoff { resource, .. } if resource == "https://restored.example/a"))
        .expect("handoff must be recorded");
    let born_pos = tr
        .iter()
        .position(|d| matches!(d, Decision::ThreadBorn { .. }))
        .expect("thread must be born");
    assert!(handoff_pos < born_pos);
}

// ----------------------------------------------------------------------
// 4. Sharing any number of resources never merges two works.
//    Merging exists only in the declaration path.
// ----------------------------------------------------------------------
#[test]
fn invariant_04_no_merge_without_declaration() {
    let log = vec![
        ev(t(100), Interaction::Focused, "app:browser"),
        evw(t(101), Interaction::Focused, "https://shared.example/x", 120_000),
        evw(t(102), Interaction::Typed, "file:///doc-a.md", 200),
        ev(t(103), Interaction::Saved, "file:///doc-a.md"),
        // 15 minutes later: same browser, same shared page, *different* work.
        evw(t(500), Interaction::Typed, "file:///doc-b.md", 200),
        evw(t(501), Interaction::Focused, "https://shared.example/x", 120_000),
        ev(t(502), Interaction::Saved, "file:///doc-b.md"),
        // The day ends; the sitting bounds at the clock.
        evw(t(3_000), Interaction::Typed, "file:///next-day.md", 1),
    ];
    let e = Engine::replay(&log);
    assert_eq!(e.threads().len(), 2, "shared resources must not fuse works");
    assert!(thread_with_anchor(&e, "file:///doc-a.md").is_some());
    assert!(thread_with_anchor(&e, "file:///doc-b.md").is_some());
    assert!(!trace_has(&e, |d| matches!(d, Decision::ThreadMerged { .. })));

    // The declaration path — the only merge path in the system — works.
    let mut log2 = log.clone();
    log2.push(declare(
        t(3_100),
        Declaration::SameWork {
            a: "file:///doc-a.md".into(),
            b: "file:///doc-b.md".into(),
        },
    ));
    let e2 = Engine::replay(&log2);
    assert_eq!(e2.threads().len(), 1);
    assert!(trace_has(&e2, |d| matches!(d, Decision::ThreadMerged { survivor: 1, absorbed: 2, .. })));
}

// ----------------------------------------------------------------------
// 5. Forty churn mutations beside real work: zero participants, zero
//    threads, zero restore candidates.
// ----------------------------------------------------------------------
#[test]
fn invariant_05_machine_churn_is_quarantined_at_birth() {
    let mut log = vec![
        evw(t(100), Interaction::Typed, "file:///real/work.md", 300),
        ev(t(101), Interaction::Saved, "file:///real/work.md"),
    ];
    for i in 0..40 {
        log.push(sys(t(102) + i * 1_000, &format!("file:///var/lib/svc/state-{i:02}.json")));
    }
    // Even Person-origin Mutated with no adjacent person act on the resource
    // is background, not work.
    for i in 0..3 {
        log.push(Event {
            timestamp: t(103) + i * 1_000,
            origin: Origin::Person,
            interaction: Interaction::Mutated,
            resource: format!("file:///var/lib/svc/state-person-{i}.json"),
            weight: None,
            detail: None,
        });
    }
    // Hours later an unrelated tap bounds the sitting at the clock.
    log.push(evw(t(400), Interaction::Typed, "file:///unrelated.md", 1));
    let e = Engine::replay(&log);
    assert_eq!(e.threads().len(), 1);
    let real = thread_with_anchor(&e, "file:///real/work.md").unwrap();
    let bundle = e.resume_bundle(real.id);
    assert!(!bundle.restore_set.iter().any(|r| r.contains("svc")));
    assert!(!all_thread_participants(&e).iter().any(|r| r.contains("svc")));
    let churn_drops = e
        .trace()
        .iter()
        .filter(|d| matches!(d, Decision::EvidenceDropped { reason, .. } if reason.contains("background churn")))
        .count();
    assert_eq!(churn_drops, 43);
}

// ----------------------------------------------------------------------
// 6. The always-on companion window attends everything and links nothing.
// ----------------------------------------------------------------------
#[test]
fn invariant_06_companion_window_restores_nothing() {
    let log = vec![
        evw(t(100), Interaction::Focused, "app:music", 120_000),
        evw(t(101), Interaction::Typed, "file:///alpha.md", 200),
        ev(t(102), Interaction::Saved, "file:///alpha.md"),
        evw(t(500), Interaction::Typed, "file:///beta.md", 200),
        ev(t(501), Interaction::Saved, "file:///beta.md"),
        evw(t(502), Interaction::Focused, "app:music", 120_000),
        evw(t(600), Interaction::Typed, "file:///fleeting.md", 1),
    ];
    let e = Engine::replay(&log);
    assert_eq!(e.threads().len(), 2);
    for tv in e.threads() {
        let b = e.resume_bundle(tv.id);
        assert!(!b.restore_set.contains(&"app:music".to_string()));
        assert!(tv.anchors.iter().all(|a| a.resource != "app:music"));
    }
    assert!(!trace_has(&e, |d| matches!(
        d,
        Decision::EpisodeAssigned { witnesses, .. }
            if witnesses.iter().any(|w| matches!(w, Witness::ConfigOverlap { resource, .. } if resource == "app:music"))
    )));
}

// ----------------------------------------------------------------------
// 7. Deep attention without production or recurrence is not a work —
//    not even 47,000 seconds of it.
// ----------------------------------------------------------------------
#[test]
fn invariant_07_the_47000_second_movie_mints_nothing() {
    let log = vec![
        evw(t(100), Interaction::Focused, "https://cinema.example/watch/feature", 47_000_000),
        evw(t(800), Interaction::Typed, "file:///scribble.md", 50),
    ];
    let e = Engine::replay(&log);
    assert!(e.threads().is_empty(), "the film must never become a work");
    assert!(trace_has(&e, |d| matches!(
        d,
        Decision::EpisodeParked { reason: ParkReason::ConsumptionOnly, .. }
    )));
    assert!(!trace_has(&e, |d| matches!(d, Decision::ThreadBorn { .. })));
    assert!(!leaks_into_work(&e, "cinema.example"));
}

// ----------------------------------------------------------------------
// 8. Restore: at most five things, all from the last episode.
// ----------------------------------------------------------------------
#[test]
fn invariant_08_restore_is_capped_and_last_episode_only() {
    let mut log = vec![];
    for i in 1..=6u64 {
        log.push(evw(t(100 + i * 2), Interaction::Typed, &format!("file:///f{i}.md"), 100));
        log.push(ev(t(101 + i * 2), Interaction::Saved, &format!("file:///f{i}.md")));
    }
    // Long silence: beyond the resumption window, so what follows is a NEW
    // episode, attributed by the anchor it returns to.
    log.push(evw(t(400), Interaction::Typed, "file:///f1.md", 200));
    log.push(ev(t(401), Interaction::Saved, "file:///g1.md"));
    log.push(evw(t(600), Interaction::Typed, "file:///fleeting.md", 1));
    let e = Engine::replay(&log);
    assert_eq!(e.threads().len(), 1, "returning to an old anchor continues the work");
    let tv = e.threads().remove(0);
    assert_eq!(tv.episodes.len(), 2);
    let b = e.resume_bundle(tv.id);
    assert_eq!(b.resume_point, "file:///f1.md");
    assert!(b.restore_set.len() <= 5);
    let last: Vec<String> = tv.episodes[1]
        .participants
        .iter()
        .map(|(r, _)| r.clone())
        .chain(tv.episodes[1].anchors.clone())
        .collect();
    for r in &b.restore_set {
        assert!(last.contains(r), "{r} is not from the last episode");
    }
    for i in 2..=6 {
        assert!(
            !b.restore_set.contains(&format!("file:///f{i}.md")),
            "earlier-episode resources must stay in the museum"
        );
    }
}

// ----------------------------------------------------------------------
// 9. Thread IDs are issued once, never reused, never changed.
// ----------------------------------------------------------------------
#[test]
fn invariant_09_thread_ids_are_forever() {
    let mut log = vec![
        evw(t(100), Interaction::Typed, "file:///one.md", 200),
        ev(t(101), Interaction::Saved, "file:///one.md"),
        evw(t(500), Interaction::Typed, "file:///two.md", 200),
        ev(t(501), Interaction::Saved, "file:///two.md"),
        evw(t(600), Interaction::Typed, "file:///spacer.md", 1),
    ];
    let e = Engine::replay(&log);
    let ids: Vec<ThreadId> = e.threads().iter().map(|t| t.id).collect();
    assert_eq!(ids.len(), 2);

    log.push(declare(
        t(700),
        Declaration::SameWork {
            a: "file:///one.md".into(),
            b: "file:///two.md".into(),
        },
    ));
    log.push(evw(t(800), Interaction::Typed, "file:///three.md", 200));
    log.push(ev(t(801), Interaction::Saved, "file:///three.md"));
    log.push(evw(t(900), Interaction::Typed, "file:///spacer2.md", 1));
    let e = Engine::replay(&log);
    let ids2: Vec<ThreadId> = e.threads().iter().map(|t| t.id).collect();
    assert!(ids2.contains(&1), "the survivor keeps its identity");
    assert!(!ids2.contains(&2), "the absorbed id is retired, not recycled");
    assert!(ids2.contains(&3), "new births take fresh ids");
}

// ----------------------------------------------------------------------
// 10. A dip is absorbed: never splits, never votes.
// ----------------------------------------------------------------------
#[test]
fn invariant_10_dips_are_absorbed_not_attributed() {
    let detour_start = t(115);
    let back = t(115) + 90_000; // 90 seconds
    let log = vec![
        evw(t(100), Interaction::Typed, "file:///deep.md", 300),
        ev(t(110), Interaction::Saved, "file:///deep.md"),
        ev(detour_start, Interaction::Focused, "https://social.example/feed"),
        ev(detour_start + 20_000, Interaction::Visited, "https://social.example/feed"),
        evw(back, Interaction::Typed, "file:///deep.md", 20),
        evw(t(400), Interaction::Typed, "file:///fleeting.md", 1),
    ];
    let e = Engine::replay(&log);
    assert_eq!(e.threads().len(), 1, "a 90-second glance splits nothing");
    let tv = e.threads().remove(0);
    assert_eq!(tv.episodes.len(), 1, "the episode held through the detour");
    assert_eq!(tv.episodes[0].interruptions, vec![(detour_start, back)]);
    assert!(!tv.episodes[0].participants.iter().any(|(r, _)| r.contains("social")));
    assert!(!tv.companions.iter().any(|(r, _)| r.contains("social")));
    assert!(!trace_has(&e, |d| matches!(
        d,
        Decision::EpisodeAssigned { witnesses, .. }
            if witnesses.iter().any(|w| matches!(w, Witness::ConfigOverlap { resource, .. } if resource.contains("social")))
    )));
}

// ----------------------------------------------------------------------
// 11. A genuinely-shared anchor decides for neither side: other evidence
//     decides, or the episode forks.
// ----------------------------------------------------------------------
#[test]
fn invariant_11_contested_anchor_votes_for_neither() {
    let log = vec![
        // Work A: a doc and the shared repo.
        evw(t(100), Interaction::Typed, "file:///design.md", 200),
        ev(t(101), Interaction::Saved, "file:///design.md"),
        ev(t(102), Interaction::Saved, "file:///shared-repo"),
        // Work C: its own notes, and the same repo.
        evw(t(500), Interaction::Typed, "file:///notes.md", 120),
        ev(t(501), Interaction::Saved, "file:///notes.md"),
        ev(t(502), Interaction::Saved, "file:///shared-repo"),
        evw(t(520), Interaction::Typed, "file:///spacer.md", 1),
        // Much later: production on the shared repo, and nothing else.
        evw(t(900), Interaction::Typed, "file:///shared-repo", 60),
        ev(t(901), Interaction::Saved, "file:///shared-repo"),
        evw(t(1000), Interaction::Typed, "file:///spacer2.md", 1),
    ];
    let e = Engine::replay(&log);
    let a = thread_with_anchor(&e, "file:///design.md").unwrap();
    let c = thread_with_anchor(&e, "file:///notes.md").unwrap();
    assert_eq!(e.threads().len(), 2, "contestation must not mint a third work");
    assert!(a.contested.contains(&"file:///shared-repo".to_string()));
    assert!(c.contested.contains(&"file:///shared-repo".to_string()));
    assert!(trace_has(&e, |d| matches!(
        d,
        Decision::AnchorContested { resource, threads }
            if resource == "file:///shared-repo" && threads.contains(&a.id) && threads.contains(&c.id)
    )));
    // The repo-only episode went to NEITHER side; ambiguity was retained.
    assert!(trace_has(&e, |d| matches!(
        d,
        Decision::Fork { candidates, .. }
            if candidates.contains(&a.id) && candidates.contains(&c.id)
    )));
    assert_eq!(a.fork, Some(vec![a.id.min(c.id), a.id.max(c.id)]));
    assert!(!trace_has(&e, |d| matches!(
        d,
        Decision::EpisodeAssigned { witnesses, .. }
            if witnesses.iter().any(|w| matches!(w, Witness::SharedUncontestedAnchor(r) if r == "file:///shared-repo"))
                && witnesses.iter().all(|w| !matches!(w, Witness::UserPin))
    )) || !e
        .trace()
        .iter()
        .any(|d| matches!(d, Decision::EpisodeAssigned { .. })));
}

// ----------------------------------------------------------------------
// 12. Growth changes nothing that already exists.
// ----------------------------------------------------------------------
#[test]
fn invariant_12_growth_never_rewrites_history() {
    let seed = vec![
        evw(t(100), Interaction::Typed, "file:///core.md", 300),
        ev(t(101), Interaction::Saved, "file:///core.md"),
        evw(t(200), Interaction::Typed, "file:///spacer.md", 1),
    ];
    let before = Engine::replay(&seed);
    let view_before = before.threads();

    let mut grown = seed.clone();
    grown.push(evw(t(500), Interaction::Typed, "file:///core.md", 250));
    grown.push(ev(t(501), Interaction::Saved, "file:///new-appendix.md"));
    grown.push(evw(t(700), Interaction::Typed, "file:///spacer2.md", 1));
    let after = Engine::replay(&grown);
    let view_after = after.threads();

    assert_eq!(view_before.len(), 1);
    assert_eq!(view_after.len(), 1);
    assert_eq!(view_before[0].id, view_after[0].id, "the id is the work");
    assert_eq!(
        view_before[0].episodes[0], view_after[0].episodes[0],
        "the first episode's record is immutable"
    );
    assert!(view_after[0].anchors.iter().any(|a| a.resource == "file:///new-appendix.md"));
}

// ----------------------------------------------------------------------
// 13. Symmetry: renaming every resource renames the output, nothing more.
// ----------------------------------------------------------------------
#[test]
fn invariant_13_hidden_labels_do_not_matter() {
    let log = vec![
        evw(t(100), Interaction::Typed, "file:///alpha.md", 200),
        ev(t(101), Interaction::Saved, "file:///alpha.md"),
        evw(t(102), Interaction::Focused, "https://ref.example/a", 120_000),
        evw(t(500), Interaction::Typed, "file:///beta.md", 200),
        ev(t(501), Interaction::Saved, "file:///beta.md"),
        evw(t(600), Interaction::Focused, "https://film.example/x", 47_000_000),
        evw(t(700), Interaction::Typed, "file:///spacer.md", 1),
    ];
    let renamed: Vec<Event> = log
        .iter()
        .map(|e| {
            let mut e = e.clone();
            e.resource = format!("zz/{}", e.resource);
            e
        })
        .collect();
    let e1 = Engine::replay(&log);
    let e2 = Engine::replay(&renamed);
    assert_eq!(e1.threads().len(), e2.threads().len());
    assert_eq!(e1.trace().len(), e2.trace().len());
    let mut r1 = all_anchor_resources(&e1);
    let mut r2 = all_anchor_resources(&e2);
    r1.sort();
    r2.sort();
    let mapped: Vec<String> = r1.iter().map(|r| format!("zz/{r}")).collect();
    assert_eq!(mapped, r2);
}

// ----------------------------------------------------------------------
// 14. The last-thing-at-night trap: a brief terminal burst at day's end
//     moves no resume point.
// ----------------------------------------------------------------------
#[test]
fn invariant_14_the_whatsapp_trap() {
    let day = vec![
        evw(t(100), Interaction::Typed, "file:///work/report.md", 400),
        ev(t(101), Interaction::Saved, "file:///work/report.md"),
        evw(t(105), Interaction::Focused, "https://workdocs.example/spec", 120_000),
    ];
    let burst = vec![
        ev(t(1_180), Interaction::Focused, "app:messaging"),
        evw(t(1_181), Interaction::Typed, "app:messaging", 30),
        ev(t(1_182), Interaction::Saved, "app:messaging"),
        evw(t(1_190), Interaction::Typed, "file:///zzz.md", 5),
    ];
    let without = Engine::replay(&day);
    let _ = without; // a day that never closes ends watching, which is correct
    let mut full = day.clone();
    full.extend(burst);
    let with = Engine::replay(&full);

    let work = thread_with_anchor(&with, "file:///work/report.md")
        .expect("the day's real work exists");
    let b = with.resume_bundle(work.id);
    assert_eq!(b.resume_point, "file:///work/report.md");
    assert!(!b.restore_set.contains(&"app:messaging".to_string()));
    assert!(
        !with.threads().iter().any(|t| t
            .anchors
            .iter()
            .any(|a| a.resource == "app:messaging")),
        "a reflex must not mint a work"
    );
    // The burst is nowhere: no work's anchors, companions, restore sets, or
    // resume points mention it.
    for tv in with.threads() {
        let bb = with.resume_bundle(tv.id);
        assert_ne!(bb.resume_point, "app:messaging");
        assert!(!bb.restore_set.contains(&"app:messaging".to_string()));
        assert!(!tv.companions.iter().any(|(r, _)| r == "app:messaging"));
    }
    // And the head of the list is the work that earned it by investment,
    // not the surface that merely spoke last.
    assert_eq!(with.threads()[0].id, work.id);
}

// ----------------------------------------------------------------------
// Mechanism spot-checks (beyond the contract): configuration carries a
// production-free return; reformation founds a work without any production.
// ----------------------------------------------------------------------

#[test]
fn mechanism_config_carries_only_production_free_episodes() {
    let log = vec![
        evw(t(100), Interaction::Typed, "file:///manuscript.md", 300),
        ev(t(101), Interaction::Saved, "file:///manuscript.md"),
        // The guide is consulted mid-flow; the return after five minutes
        // keeps it as the working set's company.
        evw(t(102), Interaction::Focused, "https://ref.example/style-guide", 120_000),
        evw(t(107), Interaction::Typed, "file:///manuscript.md", 80),
        // Next session, far beyond resumption: pure reading at the same guide.
        evw(t(500), Interaction::Focused, "https://ref.example/style-guide", 120_000),
        // The day ends; the reading sitting bounds at the clock.
        evw(t(900), Interaction::Typed, "file:///next-day.md", 1),
    ];
    let e = Engine::replay(&log);
    assert_eq!(e.threads().len(), 1, "the reading belongs to the writing's thread");
    let tv = e.threads().remove(0);
    assert_eq!(tv.episodes.len(), 2);
    assert!(trace_has(&e, |d| matches!(
        d,
        Decision::EpisodeAssigned { witnesses, .. }
            if witnesses.iter().any(|w| matches!(w, Witness::ConfigOverlap { resource, .. } if resource == "https://ref.example/style-guide"))
    )));
}

#[test]
fn mechanism_reformation_founds_work_without_production() {
    // Day one: deep reading, no production. Nothing is minted.
    let day1 = vec![
        evw(t(100), Interaction::Focused, "https://journal.example/paper-7", 300_000),
        ev(t(101), Interaction::Visited, "https://journal.example/paper-7"),
        evw(t(150), Interaction::Typed, "file:///spacer.md", 1),
    ];
    let e1 = Engine::replay(&day1);
    assert!(e1.threads().is_empty(), "one session of reading is still reading");

    // Next day, the same reading returns on a separate occasion: the
    // configuration re-forms, and that *is* a work now — web-research reality.
    let mut day2 = day1.clone();
    day2.push(evw(t(1_600), Interaction::Focused, "https://journal.example/paper-7", 300_000));
    day2.push(ev(t(1_601), Interaction::Visited, "https://journal.example/paper-7"));
    day2.push(ev(t(1_660), Interaction::Visited, "https://journal.example/related-9"));
    day2.push(evw(t(1_700), Interaction::Typed, "file:///spacer2.md", 1));
    let e2 = Engine::replay(&day2);
    let papers = e2.threads();
    assert_eq!(papers.len(), 1, "reformed research becomes a work");
    let tv = papers.into_iter().next().unwrap();
    assert!(tv
        .anchors
        .iter()
        .any(|a| a.resource == "https://journal.example/paper-7"
            && a.kind == AnchorKind::Recurrence));
    assert_eq!(tv.episodes.len(), 2, "both occasions join the reformed work");
    let b = e2.resume_bundle(tv.id);
    assert_eq!(b.resume_point, "https://journal.example/paper-7");
}

// ----------------------------------------------------------------------
// A re-download of the same target is one landing, not three — and it must
// never hijack the resume point.
// ----------------------------------------------------------------------
#[test]
fn mechanism_a_redownload_counts_once() {
    let cfg = evo_threads::Config::default();
    let log = vec![
        evw(t(100), Interaction::Typed, "file:///reports/q3.md", 300),
        ev(t(101), Interaction::Saved, "file:///reports/q3.md"),
        ev(t(102), Interaction::Downloaded, "file:///dl/dataset.csv"),
        ev(t(103), Interaction::Downloaded, "file:///dl/dataset.csv"),
        // The same file again, renamed by the download surface.
        ev(t(104), Interaction::Downloaded, "file:///dl/dataset (1).csv"),
        // The day ends far later; the sitting bounds at the clock.
        evw(t(2_000), Interaction::Typed, "file:///next-day.md", 1),
    ];
    let e = Engine::replay(&log);
    assert_eq!(e.threads().len(), 1);
    let tv = &e.threads()[0];

    let dataset_anchors: Vec<_> = tv
        .anchors
        .iter()
        .filter(|a| a.resource.contains("dataset"))
        .collect();
    assert_eq!(dataset_anchors.len(), 1, "three landings, one anchor");
    assert_eq!(
        dataset_anchors[0].strength as f64, cfg.downloaded_credit as f64,
        "a download credits once"
    );

    // Strongest production keeps the resume point; the download corroborates.
    let b = e.resume_bundle(tv.id);
    assert_eq!(b.resume_point, "file:///reports/q3.md");
    assert!(b.restore_set.contains(&"file:///dl/dataset.csv".to_string()));

    let repeats = e
        .trace()
        .iter()
        .filter(|d| matches!(d, Decision::RepeatedDownload { .. }))
        .count();
    assert_eq!(repeats, 2, "both repeats are accounted for in the audit log");
}

// ----------------------------------------------------------------------
// The daemon's read-side tick: settle() bounds the last sitting exactly the
// way the next event would have — and a warm clock settles nothing.
// ----------------------------------------------------------------------
#[test]
fn mechanism_settle_bounds_the_last_sitting() {
    let day = vec![
        evw(t(100), Interaction::Typed, "file:///work/finale.md", 300),
        ev(t(101), Interaction::Saved, "file:///work/finale.md"),
    ];

    // Still mid-sitting: nothing is invented ahead of the evidence.
    let mut warm = Engine::replay(&day);
    warm.settle(t(103));
    assert!(warm.threads().is_empty(), "a warm clock settles nothing");

    // Three hours of silence later, the daemon asks what day it was.
    let mut e = Engine::replay(&day);
    let at = t(101) + 3 * 3_600_000;
    e.settle(at);
    let threads = e.threads();
    assert_eq!(threads.len(), 1, "after the boundary, the work stands up");
    assert_eq!(threads[0].anchors[0].resource, "file:///work/finale.md");
    let b = e.resume_bundle(threads[0].id);
    assert_eq!(b.resume_point, "file:///work/finale.md");
    assert!(!e.explain(threads[0].id).is_empty());

    // Settling again mints nothing new: no decisions, no change of view.
    let tr_len = e.trace().len();
    let view_before = format!("{:?}", e.threads());
    e.settle(at + 3_600_000);
    assert_eq!(e.trace().len(), tr_len, "a second settle is inert");
    assert_eq!(format!("{:?}", e.threads()), view_before);

    // And the clock is honest with itself: settling backwards is ignored.
    e.settle(t(100));
    assert_eq!(e.trace().len(), tr_len);
}

// ----------------------------------------------------------------------
// A terminal reflex settles as a reflex: once bounded, it is parked below
// the birth floor, not minted.
// ----------------------------------------------------------------------
#[test]
fn mechanism_settle_parks_a_terminal_burst() {
    let burst = vec![
        ev(t(1_374), Interaction::Focused, "app:messaging"),
        evw(t(1_375), Interaction::Typed, "app:messaging", 30),
        ev(t(1_376), Interaction::Saved, "app:messaging"),
    ];
    let mut e = Engine::replay(&burst);
    assert!(e.threads().is_empty(), "an unsettled burst is simply unseen");
    e.settle(t(1_376) + 3_600_000);
    assert!(e.threads().is_empty(), "a settled reflex is still not a work");
    assert!(trace_has(&e, |d| matches!(
        d,
        Decision::EpisodeParked { reason: ParkReason::WeakProduction, .. }
    )));
}
