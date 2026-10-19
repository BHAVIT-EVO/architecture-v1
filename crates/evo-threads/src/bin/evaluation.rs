//! Held-out, local-only evaluation for overlapping work and selective restore.
//!
//! This binary is deliberately self-contained: every workflow is synthetic and
//! lives in memory, and the only system under test is `evo_threads::Engine`.
//! There is no model, filesystem read, personal-history input, or network call.
//!
//! The four headline metrics are kept separate:
//!
//! * grouping usefulness — expected same-work relations are recovered;
//! * grouping safety — expected separations and non-work attention stay apart;
//! * restoration usefulness — required current surfaces are reopened; and
//! * restoration safety — stale, shared, ambient, and unrelated surfaces stay
//!   out of a selective restore.
//!
//! A non-zero exit status means a synthetic contract failed. This evaluator does
//! not tune production heuristics to make a score pass; a failure is evidence
//! for the owning engine workstream to investigate.

use evo_threads::{Engine, Event, Interaction, Origin, ThreadId, ThreadView};

const BASE: u64 = 1_700_000_000_000;
const MINUTE: u64 = 60_000;

#[derive(Clone, Copy)]
struct WorkSpec {
    label: &'static str,
    root: &'static str,
    must_restore: &'static [&'static str],
    must_not_restore: &'static [&'static str],
}

struct Case {
    name: &'static str,
    events: Vec<Event>,
    works: Vec<WorkSpec>,
    same_work: Vec<(&'static str, &'static str)>,
    separate_work: Vec<(&'static str, &'static str)>,
    absent_from_work: Vec<&'static str>,
}

#[derive(Default)]
struct Metrics {
    grouping_usefulness_pass: usize,
    grouping_usefulness_total: usize,
    grouping_safety_pass: usize,
    grouping_safety_total: usize,
    restoration_usefulness_pass: usize,
    restoration_usefulness_total: usize,
    restoration_safety_pass: usize,
    restoration_safety_total: usize,
    determinism_pass: usize,
    determinism_total: usize,
    failures: Vec<String>,
}

impl Metrics {
    fn record(&mut self, bucket: &str, passed: bool, detail: String) {
        let (pass, total) = match bucket {
            "grouping usefulness" => (
                &mut self.grouping_usefulness_pass,
                &mut self.grouping_usefulness_total,
            ),
            "grouping safety" => (
                &mut self.grouping_safety_pass,
                &mut self.grouping_safety_total,
            ),
            "restoration usefulness" => (
                &mut self.restoration_usefulness_pass,
                &mut self.restoration_usefulness_total,
            ),
            "restoration safety" => (
                &mut self.restoration_safety_pass,
                &mut self.restoration_safety_total,
            ),
            "determinism" => (&mut self.determinism_pass, &mut self.determinism_total),
            _ => panic!("unknown metric bucket: {bucket}"),
        };
        *total += 1;
        if passed {
            *pass += 1;
        } else {
            self.failures.push(format!("{bucket}: {detail}"));
        }
    }

    fn ratio(pass: usize, total: usize) -> String {
        if total == 0 {
            return "n/a".to_string();
        }
        format!(
            "{pass}/{total} ({:.1}%)",
            pass as f64 * 100.0 / total as f64
        )
    }
}

fn timestamp(minute: u64) -> u64 {
    BASE + minute * MINUTE
}

fn event(timestamp: u64, interaction: Interaction, resource: &str) -> Event {
    Event {
        timestamp,
        origin: Origin::Person,
        interaction,
        resource: resource.to_string(),
        weight: None,
        detail: None,
    }
}

fn weighted(timestamp: u64, interaction: Interaction, resource: &str, weight: u64) -> Event {
    Event {
        timestamp,
        origin: Origin::Person,
        interaction,
        resource: resource.to_string(),
        weight: Some(weight),
        detail: None,
    }
}

fn settled(events: &[Event]) -> Engine {
    let mut engine = Engine::replay(events);
    let last = events.last().map(|event| event.timestamp).unwrap_or(BASE);
    // Make the final sitting observable without relying on a wall clock.
    engine.settle(last + 86_400_000);
    engine
}

fn carries(thread: &ThreadView, resource: &str) -> bool {
    thread
        .anchors
        .iter()
        .any(|anchor| anchor.resource == resource)
        || thread
            .companions
            .iter()
            .any(|(candidate, _)| candidate == resource)
        || thread.episodes.iter().any(|episode| {
            episode
                .participants
                .iter()
                .any(|(candidate, _)| candidate == resource)
        })
}

fn thread_ids_for(threads: &[ThreadView], resource: &str) -> Vec<ThreadId> {
    threads
        .iter()
        .filter(|thread| carries(thread, resource))
        .map(|thread| thread.id)
        .collect()
}

fn bundle_for(engine: &Engine, threads: &[ThreadView], root: &str) -> Option<Vec<String>> {
    let ids = thread_ids_for(threads, root);
    if ids.len() != 1 {
        return None;
    }
    Some(engine.resume_bundle(ids[0]).restore_set)
}

fn contains_pair(threads: &[ThreadView], left: &str, right: &str) -> bool {
    let left_ids = thread_ids_for(threads, left);
    let right_ids = thread_ids_for(threads, right);
    left_ids.iter().any(|id| right_ids.contains(id))
}

fn print_case_header(case: &Case) {
    println!("\nCASE: {}", case.name);
    for work in &case.works {
        println!("  work: {} [{}]", work.label, work.root);
    }
}

fn evaluate_case(case: &Case, metrics: &mut Metrics) {
    print_case_header(case);

    let first = settled(&case.events);
    let second = settled(&case.events);
    metrics.record(
        "determinism",
        first == second,
        format!("{} replay differed", case.name),
    );

    let threads = first.threads();
    println!("  threads minted: {}", threads.len());

    for work in &case.works {
        let ids = thread_ids_for(&threads, work.root);
        let exists_once = ids.len() == 1;
        let not_duplicated = ids.len() <= 1;
        metrics.record(
            "grouping usefulness",
            exists_once,
            format!("{} root mapped to thread ids {ids:?}", work.root),
        );
        metrics.record(
            "grouping safety",
            not_duplicated,
            format!("{} root is ambiguous across thread ids {ids:?}", work.root),
        );

        let bundle = bundle_for(&first, &threads, work.root);
        for resource in work.must_restore {
            let passed = bundle
                .as_ref()
                .is_some_and(|restore_set| restore_set.iter().any(|item| item == resource));
            metrics.record(
                "restoration usefulness",
                passed,
                format!("{} must restore {resource}; bundle={bundle:?}", work.label),
            );
        }
        for resource in work.must_not_restore {
            let passed = bundle
                .as_ref()
                .is_none_or(|restore_set| !restore_set.iter().any(|item| item == resource));
            metrics.record(
                "restoration safety",
                passed,
                format!("{} must withhold {resource}; bundle={bundle:?}", work.label),
            );
        }

        if let Some(restore_set) = bundle {
            metrics.record(
                "restoration safety",
                restore_set.len() <= 5,
                format!("{} restore cap exceeded: {restore_set:?}", work.label),
            );
        }
    }

    for &(left, right) in &case.same_work {
        let passed = contains_pair(&threads, left, right);
        metrics.record(
            "grouping usefulness",
            passed,
            format!("expected {left} and {right} in one work"),
        );
    }
    for &(left, right) in &case.separate_work {
        let passed = !contains_pair(&threads, left, right);
        metrics.record(
            "grouping safety",
            passed,
            format!("expected {left} and {right} to stay separate"),
        );
    }
    for resource in &case.absent_from_work {
        let passed = thread_ids_for(&threads, resource).is_empty();
        metrics.record(
            "grouping safety",
            passed,
            format!("unrelated attention leaked into a work: {resource}"),
        );
    }

    for thread in &threads {
        println!(
            "  thread {}: anchors={} companions={} episodes={}",
            thread.id,
            thread.anchors.len(),
            thread.companions.len(),
            thread.episodes.len()
        );
    }
}

fn overlapping_workflow() -> Case {
    const A: &str = "file:///sintesis/Δ/rapport-final.md";
    const B: &str = "file:///مشاريع/東京/observations.md";
    const SHARED: &str = "https://atelier.test/共有/planificación";
    const MUSIC: &str = "app:ambient-music";
    const UNRELATED: &str = "https://cinema.test/фильм/long-form";

    let mut events = vec![
        // Work A: a produced artifact plus a shared planning surface.
        weighted(timestamp(100), Interaction::Focused, MUSIC, 120_000),
        weighted(timestamp(101), Interaction::Focused, A, 120_000),
        weighted(timestamp(102), Interaction::Typed, A, 240),
        event(timestamp(103), Interaction::Saved, A),
        weighted(timestamp(105), Interaction::Visited, SHARED, 60_000),
        // Six minutes away makes this a real shared excursion rather than an
        // absorbed dip; returning to A keeps it in the episode's context.
        weighted(timestamp(111), Interaction::Typed, A, 120),
        event(timestamp(112), Interaction::Saved, A),
        // Work B begins after a real temporal gap, using the same shared surface.
        weighted(timestamp(500), Interaction::Focused, MUSIC, 120_000),
        weighted(timestamp(501), Interaction::Focused, B, 120_000),
        weighted(timestamp(502), Interaction::Typed, B, 240),
        event(timestamp(503), Interaction::Saved, B),
        weighted(timestamp(505), Interaction::Visited, SHARED, 60_000),
        weighted(timestamp(511), Interaction::Typed, B, 120),
        event(timestamp(512), Interaction::Saved, B),
    ];
    // Long unrelated attention is a parked consumption episode, not a work.
    events.extend([
        weighted(timestamp(900), Interaction::Focused, UNRELATED, 300_000),
        weighted(timestamp(901), Interaction::Visited, UNRELATED, 60_000),
    ]);

    Case {
        name: "overlapping multilingual workflows with shared resources and unrelated attention",
        events,
        works: vec![
            WorkSpec {
                label: "rapport — تقرير — 報告",
                root: A,
                must_restore: &[A],
                must_not_restore: &[B, SHARED, MUSIC, UNRELATED],
            },
            WorkSpec {
                label: "observations — ملاحظات — 観測",
                root: B,
                must_restore: &[B],
                must_not_restore: &[A, SHARED, MUSIC, UNRELATED],
            },
        ],
        same_work: vec![(A, SHARED), (B, SHARED)],
        separate_work: vec![(A, B)],
        absent_from_work: vec![MUSIC, UNRELATED],
    }
}

fn reading_only_workflow() -> Case {
    const READING: &str = "https://biblioteca.test/مخطوطات/東京/été";
    const UNRELATED: &str = "https://cinema.test/кино/feature";

    let events = vec![
        // No typing or saving: only meaningful returns across separate days.
        weighted(timestamp(100), Interaction::Visited, READING, 90_000),
        weighted(timestamp(105), Interaction::Focused, READING, 90_000),
        weighted(timestamp(1540), Interaction::Visited, READING, 90_000),
        weighted(timestamp(1545), Interaction::Focused, READING, 90_000),
        weighted(timestamp(2000), Interaction::Focused, UNRELATED, 300_000),
    ];

    Case {
        name: "reading-only recurrence with multilingual labels and a temporal gap",
        events,
        works: vec![WorkSpec {
            label: "lecture — قراءة — 読書",
            root: READING,
            must_restore: &[READING],
            must_not_restore: &[UNRELATED],
        }],
        same_work: vec![],
        separate_work: vec![],
        absent_from_work: vec![UNRELATED],
    }
}

fn renamed_machine_workflow() -> Case {
    const ROOT: &str = "file:///vault/atelier/proposal.md";
    const OLD_APP: &str = "window:machine-alpha/app=EditorClassic/title=proposal";
    const NEW_APP: &str = "window:machine-beta/app=EditorNext/title=proposal";
    const MUSIC: &str = "app:ambient-music";

    let events = vec![
        // The stable artifact is the identity carrier; only its machine/app
        // surface changes between sittings.
        weighted(timestamp(100), Interaction::Focused, MUSIC, 120_000),
        weighted(timestamp(101), Interaction::Focused, ROOT, 120_000),
        weighted(timestamp(102), Interaction::Typed, ROOT, 320),
        event(timestamp(103), Interaction::Saved, ROOT),
        weighted(timestamp(105), Interaction::Visited, OLD_APP, 90_000),
        weighted(timestamp(111), Interaction::Typed, ROOT, 160),
        event(timestamp(112), Interaction::Saved, ROOT),
        // Next day: same artifact, renamed machine and application surface.
        weighted(timestamp(1540), Interaction::Focused, MUSIC, 120_000),
        weighted(timestamp(1541), Interaction::Focused, ROOT, 120_000),
        weighted(timestamp(1542), Interaction::Typed, ROOT, 320),
        event(timestamp(1543), Interaction::Saved, ROOT),
        weighted(timestamp(1545), Interaction::Visited, NEW_APP, 90_000),
        weighted(timestamp(1551), Interaction::Typed, ROOT, 160),
        event(timestamp(1552), Interaction::Saved, ROOT),
    ];

    Case {
        name: "stable work across machine/app renaming with selective current-surface restore",
        events,
        works: vec![WorkSpec {
            label: "proposal — प्रस्ताव — مشروع",
            root: ROOT,
            must_restore: &[ROOT, NEW_APP],
            must_not_restore: &[OLD_APP, MUSIC],
        }],
        same_work: vec![(ROOT, OLD_APP), (ROOT, NEW_APP)],
        separate_work: vec![],
        absent_from_work: vec![MUSIC],
    }
}

fn main() {
    let cases = vec![
        overlapping_workflow(),
        reading_only_workflow(),
        renamed_machine_workflow(),
    ];
    let mut metrics = Metrics::default();
    for case in &cases {
        evaluate_case(case, &mut metrics);
    }

    println!("\nHELD-OUT SYNTHETIC EVALUATION");
    println!(
        "  grouping usefulness:   {}",
        Metrics::ratio(
            metrics.grouping_usefulness_pass,
            metrics.grouping_usefulness_total
        )
    );
    println!(
        "  grouping safety:       {}",
        Metrics::ratio(metrics.grouping_safety_pass, metrics.grouping_safety_total)
    );
    println!(
        "  restoration usefulness: {}",
        Metrics::ratio(
            metrics.restoration_usefulness_pass,
            metrics.restoration_usefulness_total
        )
    );
    println!(
        "  restoration safety:     {}",
        Metrics::ratio(
            metrics.restoration_safety_pass,
            metrics.restoration_safety_total
        )
    );
    println!(
        "  deterministic replay:    {}",
        Metrics::ratio(metrics.determinism_pass, metrics.determinism_total)
    );

    if metrics.failures.is_empty() {
        println!("  result: PASS (synthetic contracts only; not real-user accuracy)");
    } else {
        println!("  result: FAIL");
        for failure in metrics.failures {
            println!("    - {failure}");
        }
        std::process::exit(1);
    }
}
