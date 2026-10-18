//! Unseen fictional workflow regressions, independent of personal-history data.
//!
//! Labels are supplied by production, declarations, qualified recurrence, or
//! their absence. Titles, professions, hosts and extensions do not supply them.
//! These deterministic scenarios test evidence handling, not real-user accuracy
//! or learned-model quality. Metrics count the labeled resource/work relations
//! below; ambiguous isolated episodes are checked separately, not counted as TP.

mod common;
use common::*;
use evo_threads::*;
use std::collections::BTreeSet;

struct Probe {
    resource: &'static str,
    /// A work is named by its independent identity anchor, not its app/domain.
    works: Vec<&'static str>,
}

struct Fixture {
    name: &'static str,
    events: Vec<Event>,
    probes: Vec<Probe>,
}

fn probe(resource: &'static str, works: &[&'static str]) -> Probe {
    Probe {
        resource,
        works: works.to_vec(),
    }
}

fn settled(events: &[Event]) -> Engine {
    let mut engine = Engine::replay(events);
    engine.settle(events.last().unwrap().timestamp + 86_400_000);
    engine
}

fn carries(thread: &ThreadView, resource: &str) -> bool {
    thread.anchors.iter().any(|a| a.resource == resource)
        || thread.companions.iter().any(|(r, _)| r == resource)
        || thread
            .episodes
            .iter()
            .any(|ep| ep.participants.iter().any(|(r, _)| r == resource))
}

const SCENE: &str = "file:///fictional/Café/escena-été.md";
const WATER: &str = "https://watershed.test/観測/水位";
const SHARED: &str = "https://community.test/共有/planificación";
const IMAGE: &str = "file:///fictional/写真/костюм-é.png";
const TABLE: &str = "file:///fictional/قياسات/medición.csv";

fn shared_work() -> Fixture {
    let mut events = Vec::new();
    // The shared planning surface is genuinely edited in each work. The two
    // primary artifacts receive stronger, independent production on each day.
    for day in [0, 1440] {
        for (minute, root, supporting) in [(100, SCENE, IMAGE), (500, WATER, TABLE)] {
            events.extend([
                evw(t(day + minute), Interaction::Typed, root, 240),
                ev(t(day + minute + 1), Interaction::Saved, root),
                ev(t(day + minute + 2), Interaction::Saved, SHARED),
                evw(
                    t(day + minute + 3),
                    Interaction::Visited,
                    supporting,
                    60_000,
                ),
                evw(t(day + minute + 9), Interaction::Typed, root, 80),
            ]);
        }
    }
    // Nothing distinguishes the work when only the contested surface is edited.
    events.extend([
        evw(t(3100), Interaction::Typed, SHARED, 120),
        ev(t(3101), Interaction::Saved, SHARED),
    ]);
    Fixture {
        name: "two repeated workflows edit the same planning surface",
        events,
        probes: vec![
            probe(SCENE, &[SCENE]),
            probe(WATER, &[WATER]),
            probe(SHARED, &[SCENE, WATER]),
            probe(IMAGE, &[SCENE]),
            probe(TABLE, &[WATER]),
        ],
    }
}

fn mixed_surfaces() -> Fixture {
    const ROOT: &str = "file:///fictional/أرشيف/récit-हिन्दी.md";
    const PHOTO: &str = "file:///fictional/写真/祖母-été.jpg";
    const PAGE: &str = "https://oral-archive.test/कथा/記録";
    Fixture {
        name: "oral archive file, photo and URL with accents and non-Latin scripts",
        events: vec![
            evw(t(100), Interaction::Typed, ROOT, 200),
            ev(t(101), Interaction::Saved, ROOT),
            evw(t(102), Interaction::Visited, PHOTO, 60_000),
            evw(t(103), Interaction::Visited, PAGE, 90_000),
            evw(t(108), Interaction::Typed, ROOT, 80),
        ],
        probes: vec![
            probe(ROOT, &[ROOT]),
            probe(PHOTO, &[ROOT]),
            probe(PAGE, &[ROOT]),
        ],
    }
}

fn attention_tabs() -> Fixture {
    const A: &str = "https://timetable.test/horaires-é";
    const B: &str = "https://garden.test/庭/花";
    const C: &str = "https://gallery.test/لوحات";
    const D: &str = "https://festival.test/संगीत";
    // A long return retains all four surfaces in one attention-only episode;
    // none has production, a declaration, or a second occasion of recurrence.
    Fixture {
        name: "four unrelated sustained tabs are not a declared setup",
        events: vec![
            evw(t_sec(100, 0), Interaction::Visited, A, 10_000),
            evw(t_sec(100, 10), Interaction::Visited, B, 20_000),
            evw(t_sec(100, 30), Interaction::Visited, C, 20_000),
            evw(t_sec(100, 50), Interaction::Visited, D, 20_000),
            evw(t_sec(105, 30), Interaction::Visited, A, 10_000),
        ],
        probes: vec![probe(A, &[]), probe(B, &[]), probe(C, &[]), probe(D, &[])],
    }
}

fn recurrence() -> Fixture {
    const RESOURCE: &str = "https://seed-bank.test/سجل/蕃茄";
    Fixture {
        name: "qualified production-free recurrence across different days",
        events: vec![
            evw(t(100), Interaction::Visited, RESOURCE, 90_000),
            evw(t(1540), Interaction::Visited, RESOURCE, 90_000),
        ],
        probes: vec![probe(RESOURCE, &[RESOURCE])],
    }
}

fn declaration() -> Fixture {
    const MENU: &str = "file:///fictional/cantina/menú-الثلاثاء.pdf";
    const ORDER: &str = "https://meal-coop.test/注文/火曜日";
    Fixture {
        name: "human co-membership without any production",
        events: vec![
            declare(
                t(99),
                Declaration::SameWork {
                    a: MENU.into(),
                    b: ORDER.into(),
                },
            ),
            evw(t(100), Interaction::Visited, MENU, 60_000),
            evw(t(101), Interaction::Visited, ORDER, 60_000),
            evw(t(107), Interaction::Visited, MENU, 60_000),
        ],
        probes: vec![probe(MENU, &[MENU]), probe(ORDER, &[MENU])],
    }
}

const RESTORED: &str = "https://ceramics.test/工房/émaillé";
const RESTORED_IMAGE: &str = "file:///fictional/陶器/glaçure.png";

fn execution_feedback(person_touches: bool) -> Fixture {
    let mut events = vec![
        exe(t(100), Interaction::Focused, RESTORED),
        exe(t(101), Interaction::Visited, RESTORED_IMAGE),
        exe(t(102), Interaction::Saved, RESTORED),
    ];
    if person_touches {
        events.push(evw(t(103), Interaction::Typed, RESTORED, 160));
    }
    Fixture {
        name: if person_touches {
            "real person input after restore"
        } else {
            "execution feedback alone"
        },
        events,
        probes: vec![
            probe(RESTORED, if person_touches { &[RESTORED] } else { &[] }),
            probe(RESTORED_IMAGE, &[]),
        ],
    }
}

const SAME_TITLE_A: &str = "window:fictional-session/101/title=Café — 計画";
const SAME_TITLE_B: &str = "window:fictional-session/202/title=Café — 計画";

fn same_title() -> Fixture {
    Fixture {
        name: "same displayed title with two different exact identity strings",
        events: vec![
            evw(t(100), Interaction::Typed, SAME_TITLE_A, 200),
            evw(t(500), Interaction::Typed, SAME_TITLE_B, 200),
            evw(t(1540), Interaction::Typed, SAME_TITLE_A, 160),
            evw(t(1940), Interaction::Typed, SAME_TITLE_B, 160),
        ],
        probes: vec![
            probe(SAME_TITLE_A, &[SAME_TITLE_A]),
            probe(SAME_TITLE_B, &[SAME_TITLE_B]),
        ],
    }
}

fn common_host() -> Fixture {
    const ROOT: &str = "https://shared-host.test/水/registro";
    const A: &str = "https://shared-host.test/театр/billets";
    const B: &str = "https://shared-host.test/رياضة/calendário";
    const C: &str = "https://shared-host.test/சமையல்/recette";
    Fixture {
        name: "common host is not relationship evidence",
        events: vec![
            evw(t(100), Interaction::Typed, ROOT, 200),
            evw(t(1540), Interaction::Visited, A, 60_000),
            evw(t(1541), Interaction::Visited, B, 60_000),
            evw(t(1542), Interaction::Visited, C, 60_000),
            evw(t(1547), Interaction::Visited, A, 60_000),
        ],
        probes: vec![
            probe(ROOT, &[ROOT]),
            probe(A, &[]),
            probe(B, &[]),
            probe(C, &[]),
        ],
    }
}

fn unqualified_recurrence(passive: bool) -> Fixture {
    const RESOURCE: &str = "https://weather-board.test/天気/été";
    let interaction = if passive {
        Interaction::Focused
    } else {
        Interaction::Visited
    };
    let dwell = if passive { 90_000 } else { 3_000 };
    Fixture {
        name: if passive {
            "passive recurrence does not qualify"
        } else {
            "brief recurrence does not qualify"
        },
        events: vec![
            evw(t(100), interaction, RESOURCE, dwell),
            evw(t(1540), interaction, RESOURCE, dwell),
        ],
        probes: vec![probe(RESOURCE, &[])],
    }
}

#[test]
fn fictional_workflow_relation_coverage_is_explicit_and_deterministic() {
    let fixtures = vec![
        shared_work(),
        mixed_surfaces(),
        attention_tabs(),
        recurrence(),
        declaration(),
        execution_feedback(false),
        execution_feedback(true),
        same_title(),
        common_host(),
        unqualified_recurrence(true),
        unqualified_recurrence(false),
    ];
    let mut tp = 0;
    let mut fp = 0;
    let mut missed_relations = 0;
    let mut positive_probes = 0;
    let mut positive_abstentions = 0;
    let mut negative_probes = 0;
    let mut safe_abstentions = 0;
    for fixture in &fixtures {
        let engine = settled(&fixture.events);
        assert_eq!(
            engine,
            settled(&fixture.events),
            "{} replay must be deterministic",
            fixture.name
        );
        let threads = engine.threads();
        let roots: BTreeSet<&str> = fixture
            .probes
            .iter()
            .flat_map(|p| p.works.iter().copied())
            .collect();
        for p in &fixture.probes {
            let expected: BTreeSet<String> = p.works.iter().map(|r| r.to_string()).collect();
            let mut actual = BTreeSet::new();
            for thread in threads.iter().filter(|thread| carries(thread, p.resource)) {
                let labels: Vec<&str> = roots
                    .iter()
                    .copied()
                    .filter(|root| thread.anchors.iter().any(|a| a.resource == *root))
                    .collect();
                if labels.is_empty() {
                    actual.insert(format!("unlabeled-thread:{}", thread.id));
                } else {
                    actual.extend(labels.into_iter().map(str::to_string));
                }
            }
            tp += actual.intersection(&expected).count();
            fp += actual.difference(&expected).count();
            missed_relations += expected.difference(&actual).count();
            if expected.is_empty() {
                negative_probes += 1;
                safe_abstentions += usize::from(actual.is_empty());
            } else {
                positive_probes += 1;
                positive_abstentions += usize::from(actual.is_empty());
            }
            assert_eq!(actual, expected, "{}: {}", fixture.name, p.resource);
        }
        // Every expected work exists separately; a fusion can otherwise hide
        // behind positive labels when a thread contains two independent roots.
        assert_eq!(
            threads.len(),
            roots.len(),
            "{} unexpected birth or fusion",
            fixture.name
        );
    }
    println!(
        "synthetic relation regression: fixtures={} positive_probes={} TP={} FP={} missed_relations={} positive_abstentions={}/{} safe_abstentions={}/{}; not real-user accuracy",
        fixtures.len(),
        positive_probes,
        tp,
        fp,
        missed_relations,
        positive_abstentions,
        positive_probes,
        safe_abstentions,
        negative_probes
    );
    assert_eq!(fp, 0);
    assert_eq!(missed_relations, 0);
    assert_eq!(positive_abstentions, 0);
    assert_eq!(safe_abstentions, negative_probes);
}

#[test]
fn a_shared_surface_only_episode_retains_both_works_as_alternatives() {
    let engine = settled(&shared_work().events);
    let scene = thread_with_anchor(&engine, SCENE).unwrap();
    let water = thread_with_anchor(&engine, WATER).unwrap();
    assert_ne!(scene.id, water.id);
    assert!(scene.contested.contains(&SHARED.into()));
    assert!(water.contested.contains(&SHARED.into()));
    assert!(engine
        .trace()
        .iter()
        .any(|d| matches!(d, Decision::Fork { candidates, .. }
        if candidates.contains(&scene.id) && candidates.contains(&water.id))));
    assert!(engine
        .threads()
        .iter()
        .all(|thread| thread.episodes.iter().all(|ep| ep.span.0 != t(3100))));
    assert!(!engine
        .trace()
        .iter()
        .any(|d| matches!(d, Decision::ThreadMerged { .. })));
}

#[test]
fn declared_membership_and_qualified_recurrence_have_distinct_witnesses() {
    let recurrence = recurrence();
    let first_occasion = settled(&recurrence.events[..1]);
    assert!(first_occasion.threads().is_empty());
    let engine = settled(&recurrence.events);
    assert_eq!(engine.threads()[0].episodes.len(), 2);
    assert!(engine.threads()[0]
        .anchors
        .iter()
        .all(|a| a.kind == AnchorKind::Recurrence));
    assert!(engine
        .trace()
        .iter()
        .any(|d| matches!(d, Decision::ThreadBorn { witnesses, .. }
        if witnesses.iter().any(|w| matches!(w, Witness::RecurrenceReforms { episodes: 2, .. })))));
    let declared = settled(&declaration().events);
    assert!(declared
        .trace()
        .iter()
        .any(|d| matches!(d, Decision::ThreadBorn { witnesses, .. }
        if witnesses.contains(&Witness::UserPin))));
}

#[test]
fn execution_feedback_is_not_person_activity() {
    assert!(settled(&execution_feedback(false).events)
        .threads()
        .is_empty());
    let engine = settled(&execution_feedback(true).events);
    assert!(engine.trace().iter().any(
        |d| matches!(d, Decision::Handoff { resource, via: Interaction::Typed, .. }
        if resource == RESTORED)
    ));
    assert_eq!(engine.threads()[0].anchors.len(), 1);
    assert_eq!(engine.threads()[0].anchors[0].resource, RESTORED);
    assert_eq!(engine.threads()[0].anchors[0].strength, 160.0);
}

#[test]
fn one_sitting_of_three_or_more_sustained_surfaces_never_implies_setup() {
    // These returns deliberately retain all resources in a single episode,
    // rather than accidentally testing separate one-resource episodes.
    for count in [3, 4, 6] {
        for dwell in [3_000, 30_000, 180_000] {
            let resources: Vec<_> = (0..count)
                .map(|i| format!("https://fictional-{i}.test/é/記録"))
                .collect();
            let mut events = Vec::new();
            for (i, resource) in resources.iter().enumerate() {
                events.push(evw(
                    t(100) + i as u64 * 30_000,
                    Interaction::Visited,
                    resource,
                    dwell,
                ));
            }
            events.push(evw(t(108), Interaction::Visited, &resources[0], dwell));
            let engine = settled(&events);
            assert_eq!(
                engine
                    .trace()
                    .iter()
                    .filter(|d| matches!(d, Decision::EpisodeStarted { .. }))
                    .count(),
                1,
                "the {count}-surface fixture must really be one sitting"
            );
            assert!(
                engine.threads().is_empty(),
                "{count} surfaces at {dwell} ms do not declare intent"
            );
            assert!(engine.trace().iter().any(|d| matches!(
                d,
                Decision::EpisodeParked {
                    reason: ParkReason::ConsumptionOnly,
                    ..
                }
            )));
            assert!(!engine.trace().iter().any(|d| matches!(
                d,
                Decision::ThreadBorn { .. } | Decision::AnchorPromoted { .. }
            )));
        }
    }
}

#[test]
fn later_production_does_not_swallow_a_substantive_parked_sitting() {
    const ROOT: &str = "file:///fictional/choeur/répétition.md";
    const KNOWN: &str = "https://archive.test/partition";
    const NEW: &str = "https://orchard.test/сад/яблоки";
    let mut events = vec![
        evw(t(100), Interaction::Typed, ROOT, 200),
        evw(t(101), Interaction::Visited, KNOWN, 90_000),
        evw(t(107), Interaction::Typed, ROOT, 80),
        // A known reference overlaps, but substantive attention is on a new
        // surface. The person's relation between them was never declared.
        evw(t(1540), Interaction::Visited, KNOWN, 90_000),
        evw(t(1541), Interaction::Visited, NEW, 300_000),
        evw(t(1547), Interaction::Visited, KNOWN, 30_000),
    ];
    let before = settled(&events);
    assert_eq!(before.threads().len(), 1);
    assert!(!carries(&before.threads()[0], NEW));
    events.push(evw(t(3100), Interaction::Typed, ROOT, 200));
    let after = settled(&events);
    assert_eq!(after.threads().len(), 1);
    assert!(
        !carries(&after.threads()[0], NEW),
        "parked sweep must obey the same fragment rule as initial assignment"
    );
}
