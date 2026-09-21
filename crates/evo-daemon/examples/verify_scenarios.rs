//! Adversarial verification suite (mission §25 P8, scenarios A–O).
//!
//! Drives the REAL vertical pipeline — MacOSAdapter → CaptureEngine →
//! Observation acceptance → Observation persistence → Artifact acceptance →
//! derived canonical index → Engagement inference → Workspace projection →
//! Restoration derivation — with canonical Observations, and then asserts what
//! Evo does and does not conclude.
//!
//! Every Observation is a canonical fact a real producer is capable of
//! witnessing (the frozen IS-0003 schemas). No fixture invents schema
//! semantics; the scenarios exercise the production path with real canonical
//! evidence, exactly as the daemon consumes it.
//!
//! **On the application names below.** Some scenarios witness subjects called
//! "Spotify Premium" or "WhatsApp". They appear here as *input*, never as
//! logic: nothing in Evo matches on them, and the mirror scenarios (B′, F′)
//! feed the very same application into a different *structure* of evidence and
//! get the opposite answer. That is the whole claim being tested — the verdict
//! follows the relationships in the evidence, not the identity of the
//! application. A grep of the production crates for any of these names finds
//! nothing.
//!
//! Every scenario runs in its own isolated storage root under the system
//! temporary directory. Scenario O proves the canonical root is untouched.
//!
//! Usage:
//!   cargo run -p evo-daemon --example verify_scenarios
//!
//! Exits 0 when every scenario holds, 1 otherwise.

use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
use evo_daemon::cache::CanonicalIndex;
use evo_daemon::persistence::load_persisted_observations;
use evo_daemon::runtime::{RestorationInputBoundary, VerticalPipeline};
use evo_daemon::workspace_replay::replay_workspaces_from_observations;
use evo_observation::provenance::ObservationSource;
use evo_restoration::{derive_restoration_plan, DerivationOutcome, RestorationInput};
use evo_storage::{Storage, StorageObjectKind};
use evo_workspace::attachment::ResourceRole;
use evo_workspace::workspace::Workspace;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ── Signal construction ────────────────────────────────────────────────────

fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH
        .checked_add(Duration::from_secs(secs))
        .expect("representable moment")
}

fn focus(subject: &str, secs: u64) -> MacOSSignal {
    MacOSSignal::WindowFocusGained {
        subject: subject.to_string(),
        observed_at: at(secs),
        process_identifier: None,
        owning_process_name: None,
        observed_state: None,
    }
}

fn visit(subject: &str, secs: u64) -> MacOSSignal {
    MacOSSignal::URLNavigated {
        subject: subject.to_string(),
        observed_at: at(secs),
    }
}

fn save(subject: &str, secs: u64) -> MacOSSignal {
    MacOSSignal::FileSaved {
        subject: subject.to_string(),
        observed_at: at(secs),
    }
}

fn member_of(member: &str, repository: &str, secs: u64) -> MacOSSignal {
    MacOSSignal::RepositoryMembership {
        member: member.to_string(),
        repository: repository.to_string(),
        observed_at: at(secs),
    }
}

// ── The derived understanding of one scenario ──────────────────────────────

/// Everything a scenario is allowed to look at: the bodies of work Evo derived,
/// and the witnessed subject behind each canonical Artifact identity. Read from
/// the derived index, which is built from the persisted Observation log — never
/// from anything the scenario held in memory.
struct Derived {
    root: PathBuf,
    workspaces: Vec<Workspace>,
    subjects: BTreeMap<String, String>,
    titles: BTreeMap<String, String>,
}

impl Derived {
    /// The witnessed subjects of one body of work, whatever their role.
    fn members(&self, workspace: &Workspace) -> BTreeSet<String> {
        workspace
            .attachments()
            .iter()
            .filter_map(|attachment| self.subjects.get(attachment.artifact_id().as_str()))
            .cloned()
            .collect()
    }

    /// The body of work containing a witnessed subject, if any.
    fn work_containing(&self, subject: &str) -> Option<&Workspace> {
        self.workspaces
            .iter()
            .find(|workspace| self.members(workspace).contains(subject))
    }

    /// The witnessed subjects of one body of work paired with the role Evo
    /// assigned them, in canonical role order.
    fn roles(&self, workspace: &Workspace) -> Vec<(String, ResourceRole)> {
        let mut roles: Vec<(String, ResourceRole)> = workspace
            .attachments()
            .iter()
            .filter_map(|attachment| {
                self.subjects
                    .get(attachment.artifact_id().as_str())
                    .map(|subject| (subject.clone(), attachment.role()))
            })
            .collect();
        roles.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
        roles
    }

    /// The Restoration outcome derived for the most recent sitting of a body of
    /// work — the same call the daemon makes.
    fn outcome(&self, workspace: &Workspace) -> DerivationOutcome {
        let snapshot = workspace
            .snapshots()
            .last()
            .expect("a projected body of work has at least one sitting");
        let input = RestorationInput::new(workspace, snapshot).expect("valid restoration input");
        derive_restoration_plan(&input)
    }

    /// The subject Evo would put the person back into.
    fn resume_subject(&self, workspace: &Workspace) -> Option<String> {
        self.outcome(workspace)
            .resume_point()
            .and_then(|point| self.subjects.get(point.artifact_id().as_str()).cloned())
    }

    /// What Evo calls this body of work — the name Home heads the card with.
    ///
    /// Read back from the replayed index like everything else here, so what a
    /// scenario inspects is the name the shipped Home would actually show.
    fn title(&self, workspace: &Workspace) -> String {
        self.titles
            .get(&workspace.id().to_string())
            .cloned()
            .unwrap_or_else(|| "<none>".to_string())
    }

    /// A one-line rendering for the report.
    fn describe(&self, workspace: &Workspace) -> String {
        let roles = self
            .roles(workspace)
            .into_iter()
            .map(|(subject, role)| format!("{role:?}:{subject}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "“{}” sittings={} [{}]",
            self.title(workspace),
            workspace.snapshots().len(),
            roles
        )
    }
}

/// Runs a list of canonical signals through the real pipeline in an isolated
/// storage root and returns the understanding Evo derived from the log it wrote.
fn run(scenario: &str, signals: &[MacOSSignal]) -> Derived {
    let root = std::env::temp_dir().join(format!(
        "evo-adversarial-{}-{scenario}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _guard = Storage::with_thread_root(root.clone());

    let source = ObservationSource::new("adversarial-suite").expect("non-empty");
    let adapter = MacOSAdapter::new(source);
    let mut engine = CaptureEngine::new();

    let (sender, receiver): (
        std::sync::mpsc::Sender<Result<RestorationInputBoundary, evo_daemon::DaemonError>>,
        std::sync::mpsc::Receiver<Result<RestorationInputBoundary, evo_daemon::DaemonError>>,
    ) = channel();
    let mut pipeline = VerticalPipeline::new(sender, root.clone()).expect("pipeline builds");

    for signal in signals {
        let raw = adapter
            .normalize(signal.clone())
            .expect("normalization is infallible")
            .expect("canonical signal");
        let schema = raw.schema();
        let observation = engine
            .ingest(raw, &schema)
            .expect("observation acceptance succeeds");
        pipeline.handle_observation(observation);
        while receiver.try_recv().is_ok() {}
    }
    // A body of work is a claim about a whole history, so it is derived once the
    // evidence has settled — the same call the daemon's worker makes when the
    // capture stream goes quiet.
    pipeline.settle();
    while receiver.try_recv().is_ok() {}

    // Read the understanding back through a *fresh* index built by full replay
    // of the persisted log. Nothing a scenario inspects was carried in memory.
    let index = CanonicalIndex::new(&root).expect("index rebuilds from the log");
    Derived {
        root,
        workspaces: index.workspaces().to_vec(),
        subjects: index
            .subjects()
            .iter()
            .map(|(id, subject)| (id.clone(), subject.clone()))
            .collect(),
        titles: index
            .titles()
            .iter()
            .map(|(id, title)| (id.clone(), title.clone()))
            .collect(),
    }
}

// ── Scenario evidence ──────────────────────────────────────────────────────

/// Repeated attention to a single resource, across many sittings, with real
/// dwell. The generous case: if attention alone could make a body of work, this
/// would be one.
fn attended_alone(subject: &str) -> Vec<MacOSSignal> {
    let mut signals = Vec::new();
    let mut moment = 0u64;
    for _sitting in 0..12 {
        for _pass in 0..4 {
            signals.push(focus(subject, moment));
            moment += 5 * 60;
        }
        moment += 2 * 60 * 60;
    }
    signals
}

/// A coherent body of work: a document, a browser page, and a terminal, used
/// with each other across several sittings.
fn coherent_task() -> Vec<MacOSSignal> {
    let mut signals = Vec::new();
    let mut moment = 0u64;
    for _sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus("internship assignment — Editor", moment));
            moment += 300;
            signals.push(save("/Users/alice/internship/assignment.md", moment));
            moment += 30;
            signals.push(visit("https://example.edu/internship/assignment-brief", moment));
            moment += 120;
            signals.push(focus("assignment analysis — Terminal", moment));
            moment += 180;
        }
        moment += 3 * 60 * 60;
    }
    signals
}

// ── Verdicts ───────────────────────────────────────────────────────────────

struct Report {
    lines: Vec<String>,
    failures: Vec<String>,
    canonical_root_records_before: usize,
}

impl Report {
    fn check(&mut self, label: &str, holds: bool, detail: String) {
        let mark = if holds { "PASS" } else { "FAIL" };
        self.lines.push(format!("  [{mark}] {label} — {detail}"));
        if !holds {
            self.failures.push(format!("{label}: {detail}"));
        }
    }

    fn scenario(&mut self, heading: &str) {
        self.lines.push(String::new());
        self.lines.push(heading.to_string());
    }
}

fn main() {
    let canonical_root = evo_storage::canonical_storage_root();
    let mut report = Report {
        lines: Vec::new(),
        failures: Vec::new(),
        canonical_root_records_before: count_records(&canonical_root, StorageObjectKind::Observation),
    };

    println!("== adversarial verification suite (scenarios A–O) ==");
    println!("canonical root (must stay untouched): {}", canonical_root.display());

    scenario_a_attention_alone(&mut report);
    scenario_b_messaging_alone_and_as_work(&mut report);
    scenario_c_a_url_is_not_a_workspace(&mut report);
    scenario_d_thirty_unrelated_resources(&mut report);
    scenario_e_convergence(&mut report);
    scenario_f_two_tasks_in_one_application(&mut report);
    scenario_g_two_tasks_in_one_repository(&mut report);
    scenario_h_return_to_earlier_work(&mut report);
    scenario_ij_selective_restoration(&mut report);
    scenario_kl_restart_and_replay(&mut report);
    scenario_m_unfamiliar_resource_types(&mut report);
    scenario_n_degradation_without_shared_vocabulary(&mut report);
    scenario_o_canonical_root_untouched(&mut report, &canonical_root);

    for line in &report.lines {
        println!("{line}");
    }
    if report.failures.is_empty() {
        println!("\nresult: OK — every adversarial scenario holds");
        std::process::exit(0);
    }
    println!("\nresult: {} scenario check(s) failed", report.failures.len());
    for failure in &report.failures {
        println!("  - {failure}");
    }
    std::process::exit(1);
}

/// **A.** A media player, attended for hours across a dozen sittings, is not a
/// body of work. Nothing was ever used *with* it, so there is no work for it to
/// be part of — a claim about relationship, which is why it needs no name.
fn scenario_a_attention_alone(report: &mut Report) {
    let derived = run("a-attention-alone", &attended_alone("Spotify Premium"));
    report.scenario("A. sustained attention to one resource, alone");
    report.check(
        "no body of work",
        derived.workspaces.is_empty(),
        format!(
            "{} derived (12 sittings, 5-minute dwells)",
            derived.workspaces.len()
        ),
    );
}

/// **B.** The same for a messaging app — and then **B′**, the case that proves
/// this is not a blacklist: the identical application, witnessed *with* a
/// document and a source, is part of the work and is presented as such.
fn scenario_b_messaging_alone_and_as_work(report: &mut Report) {
    let alone = run("b-messaging-alone", &attended_alone("WhatsApp"));
    report.scenario("B. sustained attention to a messaging app, alone");
    report.check(
        "no body of work",
        alone.workspaces.is_empty(),
        format!("{} derived", alone.workspaces.len()),
    );

    // B′ — the same application, used with the work.
    let mut signals = Vec::new();
    let mut moment = 0u64;
    for _sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus("WhatsApp", moment));
            moment += 240;
            signals.push(save("/Users/alice/handover/supplier-terms.md", moment));
            moment += 30;
            signals.push(focus("supplier terms — Editor", moment));
            moment += 300;
            signals.push(visit("https://supplier.example.com/terms", moment));
            moment += 120;
        }
        moment += 3 * 60 * 60;
    }
    let as_work = run("b-messaging-as-work", &signals);
    report.scenario("B′. the same application, witnessed with the work (anti-blacklist)");
    let work = as_work.work_containing("WhatsApp");
    report.check(
        "the same application is a member of the work",
        work.is_some(),
        match work {
            Some(work) => as_work.describe(work),
            None => format!(
                "not a member of any of the {} bodies of work derived",
                as_work.workspaces.len()
            ),
        },
    );
    // §15/§26. Being a member is not being the headline. The application belongs
    // to this work and is presented as such, but the work is named for what its
    // members have in common — not for the window that held the most attention.
    // Naming it after the application is the exact failure that made Home read
    // "Spotify Premium" over an afternoon of coursework, and no blacklist is
    // involved in avoiding it: the name is chosen by measured shared vocabulary.
    let named = work.map(|work| as_work.title(work));
    report.check(
        "the work is named for what its members share, not for the app in front",
        named
            .as_deref()
            .is_some_and(|title| title.to_lowercase().contains("supplier")),
        match &named {
            Some(title) => format!("titled “{title}”"),
            None => "no body of work to name".to_string(),
        },
    );
}

/// **C.** A URL existing is not a body of work. A search visited twice, with
/// nothing else, stays an observation.
fn scenario_c_a_url_is_not_a_workspace(report: &mut Report) {
    let derived = run(
        "c-random-search",
        &[
            visit("https://www.google.com/search?q=how+to+fix+a+tap", 0),
            visit("https://www.google.com/search?q=how+to+fix+a+tap", 600),
        ],
    );
    report.scenario("C. a random search, visited twice");
    report.check(
        "no body of work",
        derived.workspaces.is_empty(),
        format!("{} derived", derived.workspaces.len()),
    );
}

/// **D.** Thirty unrelated resources, each genuinely attended, must not produce
/// thirty bodies of work. Home is not a mirror of the observation log.
fn scenario_d_thirty_unrelated_resources(report: &mut Report) {
    let mut signals = Vec::new();
    let mut moment = 0u64;
    for index in 0..30 {
        let subject = format!("Unrelated Surface {index:02}");
        for _pass in 0..3 {
            signals.push(focus(&subject, moment));
            moment += 4 * 60;
        }
        moment += 2 * 60 * 60;
    }
    let derived = run("d-thirty-unrelated", &signals);
    report.scenario("D. thirty unrelated resources, each attended in its own sitting");
    report.check(
        "Home is not thirty bodies of work",
        derived.workspaces.len() < 30,
        format!("{} derived from 30 resources", derived.workspaces.len()),
    );
    report.check(
        "nothing here is work anyone could resume",
        derived.workspaces.is_empty(),
        format!("{} derived", derived.workspaces.len()),
    );
}

/// **E.** A coherent task spanning a document, a file, a browser page and a
/// terminal converges into ONE body of work — heterogeneous resources, no
/// repository, no application in common.
fn scenario_e_convergence(report: &mut Report) {
    let derived = run("e-convergence", &coherent_task());
    report.scenario("E. one task across an editor, a file, a URL and a terminal");
    report.check(
        "exactly one body of work",
        derived.workspaces.len() == 1,
        format!("{} derived", derived.workspaces.len()),
    );
    if let Some(work) = derived.workspaces.first() {
        let members = derived.members(work);
        report.check(
            "all four resources belong to it",
            members.len() == 4,
            format!("{} member(s): {}", members.len(), derived.describe(work)),
        );
        // §14/§15. Home has to call this something, and what it calls it must be a
        // name Evo saw. A displayed name is a slice of a witnessed subject — a
        // basename, an address without its machinery, or the subject itself — so
        // every title traces back to evidence and none of it is composed wording.
        let title = derived.title(work);
        report.check(
            "the name Home shows is traceable to a witnessed subject",
            !title.trim().is_empty()
                && derived
                    .subjects
                    .values()
                    .any(|subject| subject.contains(title.as_str())),
            format!("titled “{title}”"),
        );
    }
}

/// **F.** Two unrelated tasks carried out in the same application must not
/// collapse into one. The application is identical; only the relationships
/// differ.
fn scenario_f_two_tasks_in_one_application(report: &mut Report) {
    let mut signals = Vec::new();
    let mut moment = 0u64;
    // Task one: three sittings.
    for _sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus("tenancy agreement — Editor", moment));
            moment += 300;
            signals.push(save("/Users/alice/clients/tenancy-agreement.docx", moment));
            moment += 30;
            signals.push(visit("https://law.example.org/tenancy/statute", moment));
            moment += 120;
        }
        moment += 3 * 60 * 60;
    }
    // Task two: three later sittings in the same application, sharing nothing
    // with the first — not a resource, not a word, not a sitting.
    for _sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus("probate valuation — Editor", moment));
            moment += 300;
            signals.push(save("/Users/alice/estates/probate-valuation.docx", moment));
            moment += 30;
            signals.push(visit("https://revenue.example.org/probate/valuation", moment));
            moment += 120;
        }
        moment += 3 * 60 * 60;
    }
    let derived = run("f-two-tasks-one-application", &signals);
    report.scenario("F. two unrelated tasks in the same application");
    report.check(
        "two distinct bodies of work",
        derived.workspaces.len() == 2,
        format!("{} derived", derived.workspaces.len()),
    );
    let tenancy = derived.work_containing("tenancy agreement — Editor");
    let probate = derived.work_containing("probate valuation — Editor");
    let separate = match (tenancy, probate) {
        (Some(left), Some(right)) => left.id() != right.id(),
        _ => false,
    };
    report.check(
        "the two tasks are not the same body of work",
        separate,
        match (tenancy, probate) {
            (Some(left), Some(right)) => {
                format!("{} | {}", derived.describe(left), derived.describe(right))
            }
            _ => "at least one task was not derived at all".to_string(),
        },
    );
}

/// **G.** Two tasks inside one repository keep their distinction. Repository
/// membership is witnessed structural evidence, not an instruction to merge —
/// and the model is not built around it.
fn scenario_g_two_tasks_in_one_repository(report: &mut Report) {
    let mut signals = Vec::new();
    let mut moment = 0u64;
    let repository = "/Users/alice/src/platform";
    for path in [
        "/Users/alice/src/platform/billing/invoice.rs",
        "/Users/alice/src/platform/billing/tax.rs",
        "/Users/alice/src/platform/search/ranking.rs",
        "/Users/alice/src/platform/search/index.rs",
    ] {
        signals.push(member_of(path, repository, moment));
        moment += 1;
    }
    for _sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus("invoice tax rounding — Editor", moment));
            moment += 300;
            signals.push(save("/Users/alice/src/platform/billing/invoice.rs", moment));
            moment += 30;
            signals.push(save("/Users/alice/src/platform/billing/tax.rs", moment));
            moment += 60;
        }
        moment += 3 * 60 * 60;
    }
    for _sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus("search ranking recall — Editor", moment));
            moment += 300;
            signals.push(save("/Users/alice/src/platform/search/ranking.rs", moment));
            moment += 30;
            signals.push(save("/Users/alice/src/platform/search/index.rs", moment));
            moment += 60;
        }
        moment += 3 * 60 * 60;
    }
    let derived = run("g-two-tasks-one-repository", &signals);
    report.scenario("G. two tasks inside one repository");
    let billing = derived.work_containing("invoice tax rounding — Editor");
    let search = derived.work_containing("search ranking recall — Editor");
    let separate = match (billing, search) {
        (Some(left), Some(right)) => left.id() != right.id(),
        _ => false,
    };
    report.check(
        "shared repository membership does not fuse the two tasks",
        separate,
        match (billing, search) {
            (Some(left), Some(right)) => {
                format!("{} | {}", derived.describe(left), derived.describe(right))
            }
            _ => format!(
                "{} bodies of work derived; at least one task missing",
                derived.workspaces.len()
            ),
        },
    );
}

/// **H.** Returning to earlier work resumes from where that work was left off —
/// not from whatever happened most recently anywhere.
fn scenario_h_return_to_earlier_work(report: &mut Report) {
    let mut signals = Vec::new();
    let mut moment = 0u64;
    // Two sittings on the grant application.
    for _sitting in 0..2 {
        for _pass in 0..4 {
            signals.push(focus("grant application — Editor", moment));
            moment += 300;
            signals.push(save("/Users/alice/grants/application.md", moment));
            moment += 30;
            signals.push(visit("https://funder.example.org/grant/guidance", moment));
            moment += 120;
        }
        moment += 3 * 60 * 60;
    }
    // Three sittings on something else entirely.
    for _sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus("teaching timetable — Sheet", moment));
            moment += 300;
            signals.push(save("/Users/alice/teaching/timetable.numbers", moment));
            moment += 30;
            signals.push(visit("https://registry.example.edu/timetable", moment));
            moment += 120;
        }
        moment += 3 * 60 * 60;
    }
    // A return to the grant application, ending inside the document.
    for _pass in 0..4 {
        signals.push(visit("https://funder.example.org/grant/guidance", moment));
        moment += 120;
        signals.push(save("/Users/alice/grants/application.md", moment));
        moment += 30;
        signals.push(focus("grant application — Editor", moment));
        moment += 300;
    }
    let derived = run("h-return", &signals);
    report.scenario("H. returning to earlier work after working on something else");
    let grant = derived.work_containing("grant application — Editor");
    report.check(
        "the earlier work survived as its own body of work",
        grant.is_some(),
        format!("{} bodies of work derived", derived.workspaces.len()),
    );
    if let Some(grant) = grant {
        let resume = derived.resume_subject(grant);
        report.check(
            "it resumes inside itself, not in the work that interrupted it",
            resume
                .as_deref()
                .is_some_and(|subject| derived.members(grant).contains(subject)),
            format!("resume point: {}", resume.unwrap_or_else(|| "<none>".into())),
        );
    }
}

/// **I** and **J.** Restoration is selective: only the continuation context
/// opens, and everything else stays reachable rather than being thrown away.
fn scenario_ij_selective_restoration(report: &mut Report) {
    // The coherent task, plus twelve further resources present around it.
    let mut signals = coherent_task();
    let mut moment = 40 * 60 * 60;
    for index in 0..12 {
        let subject = format!("Peripheral Surface {index:02}");
        signals.push(focus("internship assignment — Editor", moment));
        moment += 120;
        signals.push(focus(&subject, moment));
        moment += 60;
    }
    let derived = run("ij-selective-restoration", &signals);
    report.scenario("I/J. selective restoration over a body of work with many resources");
    let Some(work) = derived.work_containing("internship assignment — Editor") else {
        report.check(
            "the work was derived",
            false,
            format!("{} bodies of work derived", derived.workspaces.len()),
        );
        return;
    };
    let roles = derived.roles(work);
    let opening: Vec<&(String, ResourceRole)> = roles
        .iter()
        .filter(|(_, role)| role.opens_on_restore())
        .collect();
    report.check(
        "restore opens a handful, not everything",
        !opening.is_empty() && opening.len() <= 4 && opening.len() < roles.len(),
        format!(
            "{} of {} member(s) open: {}",
            opening.len(),
            roles.len(),
            opening
                .iter()
                .map(|(subject, role)| format!("{role:?}:{subject}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    let kept: Vec<&(String, ResourceRole)> = roles
        .iter()
        .filter(|(_, role)| !role.opens_on_restore())
        .collect();
    report.check(
        "the rest stay part of the work without being opened",
        !kept.is_empty(),
        format!("{} member(s) held as context", kept.len()),
    );
    let chain = derived.outcome(work).context_chain().len();
    report.check(
        "supporting context is reachable in the Context Chain",
        chain > 0,
        format!("{chain} artifact(s) in the chain"),
    );
}

/// **K** and **L.** Restarting Evo and replaying the log both reconstruct the
/// same understanding — identity, membership, roles and sittings included.
fn scenario_kl_restart_and_replay(report: &mut Report) {
    let derived = run("kl-restart-replay", &coherent_task());
    report.scenario("K/L. restart, and full replay of the Observation log");

    let restarted = CanonicalIndex::new(&derived.root)
        .expect("the index rebuilds from the log after a restart")
        .workspaces()
        .to_vec();
    report.check(
        "a restart reconstructs the identical understanding",
        restarted == derived.workspaces,
        format!(
            "{} body/bodies of work before, {} after",
            derived.workspaces.len(),
            restarted.len()
        ),
    );

    // A name is derived, never stored, so a restart has to re-choose it — and
    // re-choose the same one, or Home would rename the person's work under them.
    let renamed: BTreeMap<String, String> = CanonicalIndex::new(&derived.root)
        .expect("the index rebuilds from the log after a restart")
        .titles()
        .iter()
        .map(|(id, title)| (id.clone(), title.clone()))
        .collect();
    report.check(
        "and calls each body of work by the same name",
        renamed == derived.titles,
        format!(
            "{}",
            derived
                .titles
                .values()
                .map(|title| format!("“{title}”"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );

    let observations =
        load_persisted_observations(&derived.root).expect("the Observation log loads");
    let replayed = replay_workspaces_from_observations(&observations);
    report.check(
        "replaying the log alone reconstructs the identical understanding",
        replayed == derived.workspaces,
        format!(
            "{} observation(s) replayed into {} body/bodies of work",
            observations.len(),
            replayed.len()
        ),
    );
}

/// **M.** Resource types Evo has never been taught about — a CAD drawing, a
/// PDF, a folder, a spreadsheet — form work with no application-specific code.
fn scenario_m_unfamiliar_resource_types(report: &mut Report) {
    let mut signals = Vec::new();
    let mut moment = 0u64;
    for _sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus("bracket revision c — Drafting", moment));
            moment += 300;
            signals.push(save("/Users/alice/parts/bracket-revision-c.step", moment));
            moment += 30;
            signals.push(focus("bracket tolerance sheet", moment));
            moment += 180;
            signals.push(save("/Users/alice/parts/bracket-tolerances.ods", moment));
            moment += 60;
        }
        moment += 3 * 60 * 60;
    }
    let derived = run("m-unfamiliar-types", &signals);
    report.scenario("M. resource types Evo was never taught about");
    report.check(
        "one body of work, with no code that knows what any of these are",
        derived.workspaces.len() == 1,
        match derived.workspaces.first() {
            Some(work) => derived.describe(work),
            None => "none derived".to_string(),
        },
    );
}

/// **N.** With no shared vocabulary and no shared container, the semantic
/// evidence is simply absent. Evo degrades to what it did witness — the person
/// moving between these resources — rather than inventing a relationship.
fn scenario_n_degradation_without_shared_vocabulary(report: &mut Report) {
    let mut signals = Vec::new();
    let mut moment = 0u64;
    for _sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus("Zephyr", moment));
            moment += 300;
            signals.push(focus("Quill", moment));
            moment += 300;
            signals.push(focus("Marbled", moment));
            moment += 180;
        }
        moment += 3 * 60 * 60;
    }
    let derived = run("n-no-vocabulary", &signals);
    report.scenario("N. no shared vocabulary, no shared container — only witnessed use");
    report.check(
        "work still forms from what was actually witnessed",
        derived.workspaces.len() == 1,
        match derived.workspaces.first() {
            Some(work) => derived.describe(work),
            None => "none derived".to_string(),
        },
    );
    if let Some(work) = derived.workspaces.first() {
        let resume = derived.resume_subject(work);
        report.check(
            "and it names itself from a resource it actually witnessed",
            resume
                .as_deref()
                .is_some_and(|subject| derived.members(work).contains(subject)),
            format!("resume point: {}", resume.unwrap_or_else(|| "<none>".into())),
        );
    }
}

/// **O.** None of the fabricated evidence above reached the canonical root.
/// Isolation is proven by counting the real log, not by reading the code.
fn scenario_o_canonical_root_untouched(report: &mut Report, canonical_root: &Path) {
    report.scenario("O. fabricated evidence never reaches the canonical root");
    let after = count_records(canonical_root, StorageObjectKind::Observation);
    report.check(
        "the canonical Observation log is byte-for-byte as many records as before",
        after == report.canonical_root_records_before,
        format!(
            "{} record(s) before, {} after",
            report.canonical_root_records_before, after
        ),
    );
    let refused = evo_storage::fabrication_root().is_err()
        || std::env::var_os("EVO_STORAGE_ROOT").is_some();
    report.check(
        "the storage boundary refuses fabrication against the canonical root",
        refused,
        "fabrication_root() requires an explicit non-canonical override".to_string(),
    );
}

fn count_records(root: &Path, kind: StorageObjectKind) -> usize {
    let _guard = Storage::with_thread_root(root.to_path_buf());
    Storage::new()
        .read_all(kind)
        .map(|records| records.len())
        .unwrap_or(0)
}
