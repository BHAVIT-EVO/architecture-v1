//! Deterministic, local-only metamorphic evaluation for work attribution.
//!
//! Each fixture is an event trace with semantic resource slots (two
//! interleaved works, one shared surface, one reading-only work, and unrelated
//! attention). The same trace is rendered with deliberately different opaque
//! labels: identifier-like strings, app/process-like strings, and
//! profession/multilingual text. The evaluator compares normalized
//! attribution and [`Standing`] results, never literal resource names.
//!
//! This binary has no filesystem, network, personal-history, model, or clock
//! input. A non-zero exit status is a failed synthetic contract; production
//! heuristics are intentionally not changed here to improve a score.

use evo_engagement::{
    Act, ActCharacter, Declarations, Engagement, EngagementParams, Reconstruction, Resource,
    ResourceRole, Standing,
};

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, SystemTime};

const PRESENCE_GAP: u64 = 2 * 60 * 60;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Slot {
    ALead,
    AHelper,
    BLead,
    BHelper,
    Shared,
    Ambient,
    ReadingLead,
    ReadingHelper,
    Leisure,
}

impl Slot {
    fn label(self) -> &'static str {
        match self {
            Self::ALead => "work A lead",
            Self::AHelper => "work A helper",
            Self::BLead => "work B lead",
            Self::BHelper => "work B helper",
            Self::Shared => "shared resource",
            Self::Ambient => "ambient resource",
            Self::ReadingLead => "reading lead",
            Self::ReadingHelper => "reading helper",
            Self::Leisure => "unrelated attention",
        }
    }
}

#[derive(Clone, Copy)]
enum LabelStyle {
    Opaque,
    ProcessLike,
    MultilingualProfession,
}

impl LabelStyle {
    fn name(self) -> &'static str {
        match self {
            Self::Opaque => "opaque identifiers",
            Self::ProcessLike => "app/process-like labels",
            Self::MultilingualProfession => "profession and multilingual text",
        }
    }
}

#[derive(Clone)]
struct Labels {
    by_slot: BTreeMap<Slot, String>,
    by_label: BTreeMap<String, Slot>,
}

impl Labels {
    fn for_style(style: LabelStyle) -> Self {
        let values: [(Slot, &str); 9] = match style {
            // No word in this set carries semantic meaning to the evaluator.
            // The strings intentionally have no shared vocabulary.
            LabelStyle::Opaque => [
                (Slot::ALead, "opaque:7f3a"),
                (Slot::AHelper, "opaque:b91e"),
                (Slot::BLead, "opaque:42cd"),
                (Slot::BHelper, "opaque:e084"),
                (Slot::Shared, "opaque:5ab2"),
                (Slot::Ambient, "opaque:c760"),
                (Slot::ReadingLead, "opaque:19de"),
                (Slot::ReadingHelper, "opaque:ad38"),
                (Slot::Leisure, "opaque:f2b6"),
            ],
            LabelStyle::ProcessLike => [
                (Slot::ALead, "proc://redactor/pid-17/window-cerulean"),
                (Slot::AHelper, "app://ledger/pid-23/surface-saffron"),
                (Slot::BLead, "proc://renderer/pid-31/window-umber"),
                (Slot::BHelper, "app://catalog/pid-47/surface-verdant"),
                (Slot::Shared, "process://browser/pid-59/window-neutral"),
                (Slot::Ambient, "daemon://status/pid-71/surface-ambient"),
                (Slot::ReadingLead, "proc://reader/pid-83/window-indigo"),
                (Slot::ReadingHelper, "app://archive/pid-97/surface-citrine"),
                (Slot::Leisure, "process://player/pid-109/window-leisure"),
            ],
            LabelStyle::MultilingualProfession => [
                (Slot::ALead, "contrat — rédacteur — основной"),
                (Slot::AHelper, "contrat — archiviste — вспомогательный"),
                (Slot::BLead, "航海 — cartographe — प्रमुख"),
                (Slot::BHelper, "航海 — ingénieur — सहायक"),
                (Slot::Shared, "atelier — navegador — общая"),
                (Slot::Ambient, "musique — compañero — סביב"),
                (Slot::ReadingLead, "論文 — الباحث — lecture"),
                (Slot::ReadingHelper, "論文 — bibliothécaire — قراءة"),
                (Slot::Leisure, "cinéma — दर्शक — фильм"),
            ],
        };

        let by_slot: BTreeMap<Slot, String> = values
            .into_iter()
            .map(|(slot, label)| (slot, label.to_string()))
            .collect();
        let by_label = by_slot
            .iter()
            .map(|(slot, label)| (label.clone(), *slot))
            .collect();
        Self { by_slot, by_label }
    }

    fn get(&self, slot: Slot) -> &str {
        self.by_slot
            .get(&slot)
            .map(String::as_str)
            .expect("every semantic slot has a label")
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Fingerprint {
    standing: Standing,
    members: BTreeSet<Slot>,
    roles: BTreeMap<Slot, ResourceRole>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Snapshot {
    engagements: Vec<Fingerprint>,
}

#[derive(Default)]
struct Metrics {
    passed: usize,
    total: usize,
    failures: Vec<String>,
}

impl Metrics {
    fn record(&mut self, label: impl Into<String>, passed: bool, detail: impl Into<String>) {
        self.total += 1;
        if passed {
            self.passed += 1;
        } else {
            self.failures
                .push(format!("{}: {}", label.into(), detail.into()));
        }
    }

    fn report(&self) {
        println!(
            "  metamorphic invariants: {}/{} ({:.1}%)",
            self.passed,
            self.total,
            if self.total == 0 {
                100.0
            } else {
                self.passed as f64 * 100.0 / self.total as f64
            }
        );
    }
}

fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

fn act(labels: &Labels, slot: Slot, schema: &'static str, seconds: u64) -> Act {
    Act::new(
        Resource::new(labels.get(slot), schema),
        ActCharacter::of_schema(schema),
        at(seconds),
    )
}

fn focus(labels: &Labels, slot: Slot, seconds: u64) -> Act {
    act(labels, slot, "OBS-WINDOW-FOCUS-GAINED", seconds)
}

fn visit(labels: &Labels, slot: Slot, seconds: u64) -> Act {
    act(labels, slot, "OBS-URL-NAVIGATED", seconds)
}

fn equivalent_trace(labels: &Labels) -> Vec<Act> {
    let mut acts = Vec::new();
    let mut moment = 0;

    // Each work is deliberately interleaved in every sitting. The sittings of
    // A and B are separated so the shared resource occurs in both bodies
    // without making the two works one uninterrupted activity.
    for _sitting in 0..3 {
        for _round in 0..5 {
            acts.push(focus(labels, Slot::ALead, moment));
            moment += 30;
            acts.push(focus(labels, Slot::Shared, moment));
            moment += 30;
            acts.push(visit(labels, Slot::AHelper, moment));
            moment += 30;
        }
        moment += PRESENCE_GAP;

        for _round in 0..5 {
            acts.push(focus(labels, Slot::BLead, moment));
            moment += 30;
            acts.push(focus(labels, Slot::Shared, moment));
            moment += 30;
            acts.push(visit(labels, Slot::BHelper, moment));
            moment += 30;
        }
        moment += PRESENCE_GAP;
        // A ubiquitous companion is intentionally unrelated to attribution and
        // is given its own sitting rather than being attached to B by adjacency.
        acts.push(focus(labels, Slot::Ambient, moment));
        moment += PRESENCE_GAP;
    }

    // A reading-only work: no save, commit, typing, or other production act.
    // It earns standing from repeated, deep attention to two resources alone.
    for _sitting in 0..3 {
        for _round in 0..5 {
            acts.push(visit(labels, Slot::ReadingLead, moment));
            moment += 30;
            acts.push(visit(labels, Slot::ReadingHelper, moment));
            moment += 30;
        }
        moment += PRESENCE_GAP;
        acts.push(focus(labels, Slot::Ambient, moment));
        moment += PRESENCE_GAP;
    }

    // Unrelated attention receives its own sitting and can be remembered, but
    // must never become a claimed body of work or a restore target.
    for _ in 0..3 {
        acts.push(focus(labels, Slot::Leisure, moment));
        moment += 90;
    }

    acts
}

fn snapshot(reconstruction: &Reconstruction, labels: &Labels) -> Snapshot {
    let mut engagements = reconstruction
        .set()
        .engagements()
        .iter()
        .map(|engagement| fingerprint(engagement, labels))
        .collect::<Vec<_>>();
    engagements.sort();
    Snapshot { engagements }
}

fn fingerprint(engagement: &Engagement, labels: &Labels) -> Fingerprint {
    let members = engagement
        .subjects()
        .into_iter()
        .filter_map(|subject| labels.by_label.get(&subject).copied())
        .collect();
    let roles = engagement
        .participants()
        .iter()
        .filter_map(|participant| {
            labels
                .by_label
                .get(participant.subject())
                .copied()
                .map(|slot| (slot, participant.role()))
        })
        .collect();
    Fingerprint {
        standing: engagement.standing(),
        members,
        roles,
    }
}

fn works_containing<'snapshot>(
    snapshot: &'snapshot Snapshot,
    slot: Slot,
) -> Vec<&'snapshot Fingerprint> {
    snapshot
        .engagements
        .iter()
        .filter(|engagement| engagement.standing.is_work() && engagement.members.contains(&slot))
        .collect()
}

fn same_work(snapshot: &Snapshot, left: Slot, right: Slot) -> bool {
    works_containing(snapshot, left)
        .iter()
        .any(|engagement| engagement.members.contains(&right))
}

fn has_restore_role(snapshot: &Snapshot, slot: Slot) -> bool {
    works_containing(snapshot, slot).iter().any(|engagement| {
        engagement
            .roles
            .get(&slot)
            .is_some_and(|role| role.opens_on_restore())
    })
}

fn evaluate_variant(
    style: LabelStyle,
    labels: &Labels,
    baseline: Option<&Snapshot>,
    metrics: &mut Metrics,
) -> Snapshot {
    let acts = equivalent_trace(labels);
    let declarations = Declarations::new();
    let forward = Reconstruction::build(acts.clone(), &declarations, EngagementParams::default());
    let mut reversed_acts = acts;
    reversed_acts.reverse();
    let reversed = Reconstruction::build(reversed_acts, &declarations, EngagementParams::default());
    let forward_snapshot = snapshot(&forward, labels);
    let reversed_snapshot = snapshot(&reversed, labels);

    metrics.record(
        format!("{} / replay determinism", style.name()),
        forward_snapshot == reversed_snapshot,
        "forward and reverse input order produced different normalized attribution",
    );

    let a_work = works_containing(&forward_snapshot, Slot::ALead);
    let b_work = works_containing(&forward_snapshot, Slot::BLead);
    let reading_work = works_containing(&forward_snapshot, Slot::ReadingLead);
    metrics.record(
        format!("{} / work A exists", style.name()),
        a_work.len() == 1 && a_work[0].standing == Standing::Restorable,
        format!("expected one Restorable work for {}", Slot::ALead.label()),
    );
    metrics.record(
        format!("{} / work B exists", style.name()),
        b_work.len() == 1 && b_work[0].standing == Standing::Restorable,
        format!("expected one Restorable work for {}", Slot::BLead.label()),
    );
    metrics.record(
        format!("{} / reading-only work stands", style.name()),
        reading_work.len() == 1 && reading_work[0].standing == Standing::Restorable,
        format!(
            "expected one Restorable work for {}",
            Slot::ReadingLead.label()
        ),
    );
    metrics.record(
        format!("{} / independent works stay separate", style.name()),
        !same_work(&forward_snapshot, Slot::ALead, Slot::BLead),
        "the two interleaved work leads were attributed to one work",
    );
    metrics.record(
        format!("{} / work A helper attribution", style.name()),
        same_work(&forward_snapshot, Slot::ALead, Slot::AHelper)
            && !same_work(&forward_snapshot, Slot::BLead, Slot::AHelper),
        "the first helper did not stay with its semantic work",
    );
    metrics.record(
        format!("{} / work B helper attribution", style.name()),
        same_work(&forward_snapshot, Slot::BLead, Slot::BHelper)
            && !same_work(&forward_snapshot, Slot::ALead, Slot::BHelper),
        "the second helper did not stay with its semantic work",
    );
    metrics.record(
        format!("{} / shared resource overlap", style.name()),
        same_work(&forward_snapshot, Slot::ALead, Slot::Shared)
            && same_work(&forward_snapshot, Slot::BLead, Slot::Shared),
        "the shared resource was not attributed to both works",
    );
    metrics.record(
        format!("{} / reading-only pair attribution", style.name()),
        same_work(&forward_snapshot, Slot::ReadingLead, Slot::ReadingHelper),
        "reading-only resources were not attributed together",
    );
    metrics.record(
        format!("{} / continuation roles", style.name()),
        [Slot::ALead, Slot::BLead, Slot::ReadingLead]
            .into_iter()
            .all(|slot| has_restore_role(&forward_snapshot, slot)),
        "one of the three semantic works lost its restore-capable role",
    );
    metrics.record(
        format!("{} / unrelated attention withheld", style.name()),
        works_containing(&forward_snapshot, Slot::Leisure).is_empty(),
        "unrelated attention became a claimed work",
    );
    metrics.record(
        format!("{} / ambient surface not opened", style.name()),
        forward_snapshot.engagements.iter().all(|engagement| {
            !engagement.standing.is_work()
                || engagement
                    .roles
                    .get(&Slot::Ambient)
                    .is_none_or(|role| !role.opens_on_restore())
        }),
        "ambient attention received a restore-opening role",
    );

    if let Some(baseline) = baseline {
        metrics.record(
            format!("{} / metamorphic attribution equivalence", style.name()),
            &forward_snapshot == baseline,
            "renaming labels changed normalized members, roles, or standing",
        );
    }

    println!(
        "  {:<32} engagements={} works={} A={} B={} reading={}",
        style.name(),
        forward_snapshot.engagements.len(),
        forward_snapshot
            .engagements
            .iter()
            .filter(|engagement| engagement.standing.is_work())
            .count(),
        a_work.len(),
        b_work.len(),
        reading_work.len(),
    );

    forward_snapshot
}

fn main() {
    let styles = [
        LabelStyle::Opaque,
        LabelStyle::ProcessLike,
        LabelStyle::MultilingualProfession,
    ];
    let mut metrics = Metrics::default();
    let mut baseline = None;

    println!("METAMORPHIC LOCAL-ONLY ENGAGEMENT EVALUATION");
    println!("  fixtures: interleaving + shared resource + reading-only + unrelated attention");
    for style in styles {
        let labels = Labels::for_style(style);
        let snapshot = evaluate_variant(style, &labels, baseline.as_ref(), &mut metrics);
        if baseline.is_none() {
            baseline = Some(snapshot);
        }
    }

    metrics.report();
    if metrics.failures.is_empty() {
        println!("  result: PASS (synthetic invariants only; not real-user accuracy)");
    } else {
        println!("  result: FAIL");
        for failure in metrics.failures {
            println!("    - {failure}");
        }
        std::process::exit(1);
    }
}
