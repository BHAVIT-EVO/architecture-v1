//! What Continue actually does — the door, end to end, without a display.
//!
//! These tests exercise the whole path the Continue button takes: a derived
//! outcome, the selective restoration derived from it, the plan decided from
//! that selection plus live OS state, and the per-step results the executor
//! actually returns. Nothing here touches a real window: the OS is a fake that
//! records every call, which is precisely how these tests can assert that Evo
//! did not perform something it claimed, and did not perform something it
//! never announced.
//!
//! The properties under test are the ones a user would notice if they broke:
//!
//! - Continue opens what the work continues across, and *only* that.
//! - Membership in a body of work is not a reason to open something.
//! - Something already open is revealed, not reopened.
//! - Everything Evo cannot do is reported as such, never as a success.
//! - One failure does not cost the user the rest of their restoration.
//! - The order is the same every time, and it starts where they left off.
//! - Executing changes nothing about what Evo understands the work to be.

use evo_artifact::artifact_id::ArtifactId;
use evo_execution::{
    BoundResource, Locator, LocatorKind, PlatformExecutor, PreflightSource, RestorationSelection,
    StepPlan, TargetStatus, WindowInfo, classify_locator, execute_step, plan_selection,
    select_restoration,
};
use evo_restoration::{ContextChain, DerivationOutcome, NextStep, RestorationPlan, ResumePoint};
use evo_workspace::WorkspaceId;
use evo_workspace::attachment::ResourceRole;

use std::cell::RefCell;
use std::collections::HashMap;

const SCHEMA_FILE: &str = "OBS-FILE-SAVED";
const SCHEMA_URL: &str = "OBS-URL-NAVIGATED";
const SCHEMA_WINDOW: &str = "OBS-WINDOW-FOCUS-GAINED";
const SCHEMA_COMMIT: &str = "OBS-COMMIT-MADE";

// ── The fake operating system ────────────────────────────────────────────────

/// A recording stand-in for macOS.
///
/// It reports exactly the live state a test dictates and records every action
/// asked of it, in order. Because it records rather than pretends, a test can
/// assert both what Evo reported *and* what Evo actually did — and catch the
/// two disagreeing, which is the only failure mode that matters here.
#[derive(Default)]
struct FakeOs {
    /// Every live window, as the OS would report it.
    windows: Vec<WindowInfo>,
    /// Path → the windows the OS says are displaying that file.
    documents: HashMap<String, Vec<WindowInfo>>,
    /// Paths the OS says exist.
    files: Vec<String>,
    /// Set when the platform cannot report its window list at all.
    windows_unreadable: Option<String>,
    /// Paths whose open the OS refuses.
    refuse_open: Vec<String>,
    /// Every action performed, in order performed.
    performed: RefCell<Vec<String>>,
}

impl FakeOs {
    fn with_window(mut self, title: &str, pid: i32, owner: &str) -> Self {
        self.windows.push(window(title, pid, owner));
        self
    }

    fn with_file(mut self, path: &str) -> Self {
        self.files.push(path.to_string());
        self
    }

    /// The OS reports this window as the one displaying this file.
    fn showing_document(mut self, path: &str, title: &str, pid: i32, owner: &str) -> Self {
        self.files.push(path.to_string());
        self.documents
            .entry(path.to_string())
            .or_default()
            .push(window(title, pid, owner));
        self
    }

    fn refusing_open(mut self, path: &str) -> Self {
        self.refuse_open.push(path.to_string());
        self
    }

    fn performed(&self) -> Vec<String> {
        self.performed.borrow().clone()
    }
}

impl PreflightSource for FakeOs {
    fn file_exists(&self, path: &str) -> bool {
        self.files.iter().any(|known| known == path)
    }

    fn windows(&self) -> Result<Vec<WindowInfo>, String> {
        match &self.windows_unreadable {
            Some(reason) => Err(reason.clone()),
            None => Ok(self.windows.clone()),
        }
    }

    fn windows_showing_document(&self, path: &str) -> Result<Vec<WindowInfo>, String> {
        Ok(self.documents.get(path).cloned().unwrap_or_default())
    }
}

impl PlatformExecutor for FakeOs {
    fn open_url(&self, url: &str) -> TargetStatus {
        self.performed.borrow_mut().push(format!("open_url {url}"));
        TargetStatus::Opened {
            detail: format!("opened URL {url}"),
        }
    }

    fn open_file(&self, path: &str) -> TargetStatus {
        self.performed
            .borrow_mut()
            .push(format!("open_file {path}"));
        if self.refuse_open.iter().any(|refused| refused == path) {
            return TargetStatus::Failed {
                reason: format!("the system refused to open {path}"),
            };
        }
        TargetStatus::Opened {
            detail: format!("opened file {path}"),
        }
    }

    fn focus_window(&self, title: &str) -> TargetStatus {
        self.performed
            .borrow_mut()
            .push(format!("focus_window {title}"));
        TargetStatus::Opened {
            detail: format!("focused window “{title}”"),
        }
    }

    fn focus_bound_window(&self, window: &WindowInfo) -> TargetStatus {
        self.performed.borrow_mut().push(format!(
            "focus_bound {} [{}]",
            window.title, window.owner_pid
        ));
        TargetStatus::Opened {
            detail: format!("focused window “{}”", window.title),
        }
    }
}

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn artifact(id: &str) -> ArtifactId {
    ArtifactId::new(id).expect("valid artifact id")
}

fn window(title: &str, pid: i32, owner: &str) -> WindowInfo {
    WindowInfo {
        title: title.to_string(),
        owner_pid: pid,
        owner_name: owner.to_string(),
    }
}

fn member(id: &str, role: ResourceRole) -> (ArtifactId, ResourceRole) {
    (artifact(id), role)
}

/// A Complete derivation whose Resume Point is `resume` and whose current
/// continuation surface is exactly `surface`, in the order the derivation
/// produced it.
fn outcome_for(resume: &str, surface: &[&str]) -> DerivationOutcome {
    let workspace_id = WorkspaceId::new();
    let plan = RestorationPlan::new_with_continuation_surface(
        workspace_id.clone(),
        ResumePoint::new(workspace_id.clone(), artifact(resume)),
        ContextChain::new(vec![]).expect("empty chain"),
        vec![],
        NextStep::new(workspace_id.clone(), "continue").expect("next step"),
        surface.iter().map(|id| artifact(id)).collect(),
    )
    .expect("valid plan");
    DerivationOutcome::Complete(plan)
}

/// Canonical (schema, subject) evidence per Artifact — what Evo witnessed, and
/// the only thing a locator is ever built from.
fn evidence(entries: &[(&str, &str, &str)]) -> HashMap<String, (String, String)> {
    entries
        .iter()
        .map(|(id, schema, subject)| (id.to_string(), (schema.to_string(), subject.to_string())))
        .collect()
}

/// The executable targets, resolved from that same canonical evidence.
fn locators(entries: &[(&str, &str, &str)]) -> HashMap<String, Locator> {
    entries
        .iter()
        .filter_map(|(id, schema, subject)| {
            let kind = classify_locator(schema, subject)?;
            Some((id.to_string(), Locator::new(artifact(id), kind)))
        })
        .collect()
}

/// Runs a whole plan, one step at a time, exactly as the surface does.
fn run_all(
    selection: &RestorationSelection,
    locators: &HashMap<String, Locator>,
    os: &FakeOs,
) -> Vec<(String, TargetStatus)> {
    plan_selection(selection, locators, os)
        .iter()
        .map(|step| {
            let attempt = execute_step(step, os, os);
            (
                attempt.artifact_id().as_str().to_string(),
                attempt.status().clone(),
            )
        })
        .collect()
}

fn opened(status: &TargetStatus) -> bool {
    matches!(status, TargetStatus::Opened { .. })
}

// ── 1. Eligible artifacts are selected correctly ─────────────────────────────

#[test]
fn continue_opens_exactly_what_the_work_continues_across() {
    // Five artifacts belong to this body of work. The user's declaration says
    // the work currently continues across two of them.
    let members = [
        member("artifact-editor", ResourceRole::Continuation),
        member("artifact-docs", ResourceRole::Primary),
        member("artifact-notes", ResourceRole::Primary),
        member("artifact-manual", ResourceRole::Supporting),
        member("artifact-chat", ResourceRole::Context),
    ];
    let facts = [
        ("artifact-editor", SCHEMA_FILE, "/repo/pipeline.py"),
        ("artifact-docs", SCHEMA_URL, "https://docs.dev/api"),
        ("artifact-notes", SCHEMA_FILE, "/repo/notes.md"),
        ("artifact-manual", SCHEMA_FILE, "/repo/manual.pdf"),
        ("artifact-chat", SCHEMA_WINDOW, "Chat"),
    ];
    let outcome = outcome_for("artifact-editor", &["artifact-editor", "artifact-docs"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    let os = FakeOs::default()
        .with_file("/repo/pipeline.py")
        .with_file("/repo/notes.md");
    let results = run_all(&selection, &locators(&facts), &os);

    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["artifact-editor", "artifact-docs"]);
    assert!(results.iter().all(|(_, status)| opened(status)));
    // And Evo touched the OS for exactly those two, nothing else.
    assert_eq!(
        os.performed(),
        vec![
            "open_file /repo/pipeline.py".to_string(),
            "open_url https://docs.dev/api".to_string(),
        ]
    );
}

// ── 2. Supporting and reference artifacts are not automatically opened ───────

#[test]
fn belonging_to_the_work_is_never_a_reason_to_open_something() {
    // Every artifact here is a real member of the work, and every one has a
    // perfectly openable target. Only the declared continuation opens: the
    // user does not get thirty windows because they pressed Continue.
    let members = [
        member("artifact-a-primary", ResourceRole::Continuation),
        member("artifact-b-support", ResourceRole::Supporting),
        member("artifact-c-reference", ResourceRole::Reference),
        member("artifact-d-context", ResourceRole::Context),
    ];
    let facts = [
        ("artifact-a-primary", SCHEMA_FILE, "/repo/main.rs"),
        ("artifact-b-support", SCHEMA_FILE, "/repo/spec.pdf"),
        ("artifact-c-reference", SCHEMA_URL, "https://ref.dev/x"),
        ("artifact-d-context", SCHEMA_WINDOW, "Slack"),
    ];
    let outcome = outcome_for("artifact-a-primary", &["artifact-a-primary"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    // The three withheld members are still fully described — nothing is lost,
    // they are simply not opened.
    assert_eq!(selection.withheld().len(), 3);
    for withheld in selection.withheld() {
        assert!(!withheld.disposition().opens_unasked());
        assert!(!withheld.reason().is_empty());
    }

    let os = FakeOs::default()
        .with_file("/repo/main.rs")
        .with_file("/repo/spec.pdf")
        .with_window("Slack", 900, "Slack");
    let results = run_all(&selection, &locators(&facts), &os);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "artifact-a-primary");
    assert_eq!(os.performed(), vec!["open_file /repo/main.rs".to_string()]);
}

// ── 3. Already open → focus/raise, never reopen ──────────────────────────────

#[test]
fn a_resource_the_os_says_is_already_open_is_revealed_not_reopened() {
    let members = [member("artifact-editor", ResourceRole::Continuation)];
    let facts = [("artifact-editor", SCHEMA_FILE, "/repo/pipeline.py")];
    let outcome = outcome_for("artifact-editor", &["artifact-editor"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    // The OS itself reports which window is displaying the file.
    let os = FakeOs::default().showing_document("/repo/pipeline.py", "pipeline.py", 501, "Code");
    let results = run_all(&selection, &locators(&facts), &os);

    assert!(opened(&results[0].1));
    // Focused the exact window the OS named — no reopen, and no re-resolution
    // by title, which is what would have turned a certainty into a guess.
    assert_eq!(
        os.performed(),
        vec!["focus_bound pipeline.py [501]".to_string()]
    );
}

#[test]
fn a_live_window_target_is_raised_by_the_window_the_os_named() {
    let members = [member("artifact-canvas", ResourceRole::Continuation)];
    let facts = [("artifact-canvas", SCHEMA_WINDOW, "Design Brief — Canvas")];
    let outcome = outcome_for("artifact-canvas", &["artifact-canvas"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    let os = FakeOs::default()
        .with_window("Design Brief — Canvas", 742, "Canvas")
        .with_window("Something Else", 743, "Notes");
    let results = run_all(&selection, &locators(&facts), &os);

    assert!(opened(&results[0].1));
    assert_eq!(
        os.performed(),
        vec!["focus_bound Design Brief — Canvas [742]".to_string()]
    );
}

// ── 4 & 5. A known file opens; a known URL opens ─────────────────────────────

#[test]
fn a_known_file_that_no_window_is_showing_is_opened() {
    let members = [member("artifact-file", ResourceRole::Continuation)];
    let facts = [("artifact-file", SCHEMA_FILE, "/repo/report.md")];
    let outcome = outcome_for("artifact-file", &["artifact-file"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    let os = FakeOs::default().with_file("/repo/report.md");
    let results = run_all(&selection, &locators(&facts), &os);

    assert!(opened(&results[0].1));
    assert_eq!(
        os.performed(),
        vec!["open_file /repo/report.md".to_string()]
    );
}

#[test]
fn a_known_url_is_opened_and_never_claimed_to_be_a_focused_tab() {
    let members = [member("artifact-url", ResourceRole::Continuation)];
    let facts = [("artifact-url", SCHEMA_URL, "https://docs.dev/api")];
    let outcome = outcome_for("artifact-url", &["artifact-url"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    // Even with a browser window open whose title looks related, no supported
    // generic API can witness which tab is showing a URL. Evo opens the URL and
    // lets the browser decide; it never claims to have focused a tab.
    let os = FakeOs::default().with_window("api — Browser", 301, "Browser");
    let results = run_all(&selection, &locators(&facts), &os);

    assert!(opened(&results[0].1));
    assert_eq!(
        os.performed(),
        vec!["open_url https://docs.dev/api".to_string()]
    );
}

// ── 6. Missing binding → honest failure, never a substituted target ──────────

#[test]
fn a_window_that_is_no_longer_open_is_reported_not_substituted() {
    let members = [member("artifact-gone", ResourceRole::Continuation)];
    let facts = [("artifact-gone", SCHEMA_WINDOW, "A Window Since Closed")];
    let outcome = outcome_for("artifact-gone", &["artifact-gone"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    // Other windows are open — none of them is this one.
    let os = FakeOs::default().with_window("Unrelated", 111, "Other");
    let results = run_all(&selection, &locators(&facts), &os);

    assert!(!opened(&results[0].1));
    assert!(matches!(
        results[0].1,
        TargetStatus::Unavailable { .. } | TargetStatus::Ambiguous { .. }
    ));
    // Nothing was performed. Evo did not open "something close enough".
    assert!(os.performed().is_empty());
}

#[test]
fn two_windows_with_the_same_title_are_ambiguous_never_a_coin_flip() {
    let members = [member("artifact-dup", ResourceRole::Continuation)];
    let facts = [("artifact-dup", SCHEMA_WINDOW, "untitled")];
    let outcome = outcome_for("artifact-dup", &["artifact-dup"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    let os = FakeOs::default()
        .with_window("untitled", 1, "A")
        .with_window("untitled", 2, "B");
    let results = run_all(&selection, &locators(&facts), &os);

    assert!(matches!(results[0].1, TargetStatus::Ambiguous { .. }));
    assert!(os.performed().is_empty());
}

// ── 7. A resource with no executable target → honest unavailable ─────────────

#[test]
fn a_commit_keeps_its_identity_and_is_honestly_unsupported() {
    // A commit is a repository object, not an OS-launchable resource. Evo says
    // so, with its own reason, and performs nothing.
    let members = [
        member("artifact-editor", ResourceRole::Continuation),
        member("artifact-commit", ResourceRole::Primary),
    ];
    let facts = [
        ("artifact-editor", SCHEMA_FILE, "/repo/main.rs"),
        ("artifact-commit", SCHEMA_COMMIT, "9f2c1ab"),
    ];
    let outcome = outcome_for("artifact-editor", &["artifact-editor", "artifact-commit"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    let os = FakeOs::default().with_file("/repo/main.rs");
    let results = run_all(&selection, &locators(&facts), &os);

    let commit = results
        .iter()
        .find(|(id, _)| id == "artifact-commit")
        .expect("the commit is reported, not dropped");
    match &commit.1 {
        TargetStatus::Unsupported { reason } | TargetStatus::Unavailable { reason } => {
            assert!(!reason.is_empty(), "the reason is stated, not implied");
        }
        other => panic!("a commit must never report as opened: {other:?}"),
    }
    // Only the file was acted on.
    assert_eq!(os.performed(), vec!["open_file /repo/main.rs".to_string()]);
}

#[test]
fn a_file_the_os_says_is_missing_is_reported_and_never_opened() {
    let members = [member("artifact-missing", ResourceRole::Continuation)];
    let facts = [("artifact-missing", SCHEMA_FILE, "/repo/deleted.md")];
    let outcome = outcome_for("artifact-missing", &["artifact-missing"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    let os = FakeOs::default();
    let results = run_all(&selection, &locators(&facts), &os);

    assert!(matches!(results[0].1, TargetStatus::Unavailable { .. }));
    assert!(os.performed().is_empty());
}

// ── 8. A shared artifact restores from either body of work ───────────────────

#[test]
fn a_shared_artifact_restores_from_both_bodies_of_work_it_belongs_to() {
    // One browser resource, two bodies of work. Neither owns it. Restoring it
    // for one never depends on, or disturbs, the other.
    let shared = ("artifact-shared", SCHEMA_URL, "https://tracker.dev/board");

    let work_x_members = [
        member("artifact-shared", ResourceRole::Continuation),
        member("artifact-x-file", ResourceRole::Primary),
    ];
    let x_facts = [shared, ("artifact-x-file", SCHEMA_FILE, "/x/main.rs")];
    let outcome_x = outcome_for("artifact-shared", &["artifact-shared", "artifact-x-file"]);
    let selection_x = select_restoration(&outcome_x, &work_x_members, &evidence(&x_facts));

    let work_y_members = [
        member("artifact-shared", ResourceRole::Primary),
        member("artifact-y-file", ResourceRole::Continuation),
    ];
    let y_facts = [shared, ("artifact-y-file", SCHEMA_FILE, "/y/main.rs")];
    let outcome_y = outcome_for("artifact-y-file", &["artifact-shared", "artifact-y-file"]);
    let selection_y = select_restoration(&outcome_y, &work_y_members, &evidence(&y_facts));

    let os_x = FakeOs::default().with_file("/x/main.rs");
    let results_x = run_all(&selection_x, &locators(&x_facts), &os_x);
    let os_y = FakeOs::default().with_file("/y/main.rs");
    let results_y = run_all(&selection_y, &locators(&y_facts), &os_y);

    // The same artifact is restorable from both, with the same act, and the
    // binding is a property of the resource — not of the work asking for it.
    for results in [&results_x, &results_y] {
        let shared_result = results
            .iter()
            .find(|(id, _)| id == "artifact-shared")
            .expect("the shared artifact is restored for this work too");
        assert!(opened(&shared_result.1));
    }
    assert!(
        os_x.performed()
            .contains(&"open_url https://tracker.dev/board".to_string())
    );
    assert!(
        os_y.performed()
            .contains(&"open_url https://tracker.dev/board".to_string())
    );

    // And the canonical role each body of work records for it is untouched by
    // either restoration: the artifact was never claimed by one of them.
    assert_eq!(work_x_members[0].1, ResourceRole::Continuation);
    assert_eq!(work_y_members[0].1, ResourceRole::Primary);
}

// ── 9. Partial restoration ───────────────────────────────────────────────────

#[test]
fn one_failure_does_not_cost_the_user_the_rest_of_the_restoration() {
    let members = [
        member("artifact-a-editor", ResourceRole::Continuation),
        member("artifact-b-broken", ResourceRole::Primary),
        member("artifact-c-docs", ResourceRole::Primary),
    ];
    let facts = [
        ("artifact-a-editor", SCHEMA_FILE, "/repo/main.rs"),
        ("artifact-b-broken", SCHEMA_FILE, "/repo/locked.bin"),
        ("artifact-c-docs", SCHEMA_URL, "https://docs.dev/api"),
    ];
    let outcome = outcome_for(
        "artifact-a-editor",
        &["artifact-a-editor", "artifact-b-broken", "artifact-c-docs"],
    );
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    let os = FakeOs::default()
        .with_file("/repo/main.rs")
        .with_file("/repo/locked.bin")
        .refusing_open("/repo/locked.bin");
    let results = run_all(&selection, &locators(&facts), &os);

    assert_eq!(results.len(), 3);
    assert!(opened(&results[0].1), "the first still opened");
    assert!(matches!(results[1].1, TargetStatus::Failed { .. }));
    assert!(
        opened(&results[2].1),
        "and the failure did not stop what came after it"
    );
    // The failure is stated with the system's own reason, not softened.
    let TargetStatus::Failed { reason } = &results[1].1 else {
        unreachable!("asserted above")
    };
    assert!(reason.contains("/repo/locked.bin"));
}

// ── 10. A plan is acted on once, never twice ─────────────────────────────────

#[test]
fn each_step_of_a_plan_is_performed_exactly_once() {
    let members = [
        member("artifact-a", ResourceRole::Continuation),
        member("artifact-b", ResourceRole::Primary),
    ];
    let facts = [
        ("artifact-a", SCHEMA_FILE, "/repo/a.rs"),
        ("artifact-b", SCHEMA_URL, "https://docs.dev/b"),
    ];
    let outcome = outcome_for("artifact-a", &["artifact-a", "artifact-b"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));
    let locators = locators(&facts);

    let os = FakeOs::default().with_file("/repo/a.rs");
    let steps = plan_selection(&selection, &locators, &os);

    // Drive the plan the way the surface does: advance by the number of
    // results already in hand, so the frontier — never a click — decides what
    // runs next. Extra ticks past the end perform nothing.
    let mut done = Vec::new();
    for _ in 0..(steps.len() + 5) {
        if done.len() < steps.len() {
            done.push(execute_step(&steps[done.len()], &os, &os));
        }
    }

    assert_eq!(done.len(), 2);
    assert_eq!(
        os.performed(),
        vec![
            "open_file /repo/a.rs".to_string(),
            "open_url https://docs.dev/b".to_string(),
        ],
        "the OS was asked for each target exactly once"
    );
}

// ── 11 & 13. Executing changes nothing about what Evo understands ────────────

#[test]
fn executing_changes_nothing_about_what_evo_understands_the_work_to_be() {
    let members = [
        member("artifact-editor", ResourceRole::Continuation),
        member("artifact-docs", ResourceRole::Primary),
        member("artifact-manual", ResourceRole::Supporting),
    ];
    let facts = [
        ("artifact-editor", SCHEMA_FILE, "/repo/main.rs"),
        ("artifact-docs", SCHEMA_URL, "https://docs.dev/api"),
        ("artifact-manual", SCHEMA_FILE, "/repo/manual.pdf"),
    ];
    let outcome = outcome_for("artifact-editor", &["artifact-editor", "artifact-docs"]);
    let evidence = evidence(&facts);
    let locators = locators(&facts);

    // Everything canonical, before.
    let surface_before: Vec<String> = outcome
        .continuation_surface()
        .iter()
        .map(|id| id.to_string())
        .collect();
    let selection_before = select_restoration(&outcome, &members, &evidence);

    let os = FakeOs::default()
        .with_file("/repo/main.rs")
        .showing_document("/repo/manual.pdf", "manual.pdf", 88, "Preview");
    let results = run_all(&selection_before, &locators, &os);
    assert!(results.iter().all(|(_, status)| opened(status)));

    // The same canonical inputs still derive the same understanding. The
    // execution layer holds no state that could leak back into it — including
    // the live window it just focused, and the file it just opened.
    let surface_after: Vec<String> = outcome
        .continuation_surface()
        .iter()
        .map(|id| id.to_string())
        .collect();
    assert_eq!(surface_before, surface_after);

    let selection_after = select_restoration(&outcome, &members, &evidence);
    assert_eq!(
        ids_of(&selection_before),
        ids_of(&selection_after),
        "restoration derivation is unchanged by having executed"
    );
    assert_eq!(
        selection_before.withheld().len(),
        selection_after.withheld().len()
    );
    // Membership and roles are exactly as canonically recorded. Restoring the
    // manual's live window did not promote it out of Supporting.
    assert_eq!(members[2].1, ResourceRole::Supporting);

    // And replaying the whole thing a second time produces the same plan from
    // the same canonical evidence — the previous run left nothing behind.
    let os2 = FakeOs::default()
        .with_file("/repo/main.rs")
        .showing_document("/repo/manual.pdf", "manual.pdf", 88, "Preview");
    let replayed = run_all(&selection_after, &locators, &os2);
    assert_eq!(
        results.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        replayed
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(os.performed(), os2.performed());
}

fn ids_of(selection: &RestorationSelection) -> Vec<String> {
    selection
        .restore_worthy()
        .iter()
        .map(|selected| selected.artifact_id().to_string())
        .collect()
}

// ── 12. The order is deterministic, and it starts where they left off ────────

#[test]
fn the_resume_point_comes_first_and_the_order_never_varies() {
    // The Resume Point is deliberately not the alphabetically-first id, so a
    // plan that merely sorted would fail this.
    let members = [
        member("artifact-a-docs", ResourceRole::Primary),
        member("artifact-b-notes", ResourceRole::Primary),
        member("artifact-z-resume", ResourceRole::Continuation),
    ];
    let facts = [
        ("artifact-a-docs", SCHEMA_URL, "https://docs.dev/a"),
        ("artifact-b-notes", SCHEMA_FILE, "/repo/notes.md"),
        ("artifact-z-resume", SCHEMA_FILE, "/repo/resume.rs"),
    ];
    // The derivation hands the surface over with the Resume Point first.
    let outcome = outcome_for(
        "artifact-z-resume",
        &["artifact-z-resume", "artifact-a-docs", "artifact-b-notes"],
    );
    let selection = select_restoration(&outcome, &members, &evidence(&facts));
    let locators = locators(&facts);

    let expected = {
        let os = FakeOs::default()
            .with_file("/repo/notes.md")
            .with_file("/repo/resume.rs");
        let order: Vec<String> = run_all(&selection, &locators, &os)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(
            order[0], "artifact-z-resume",
            "the user is put back where they left off first"
        );
        order
    };

    // Ten runs, same canonical inputs, same live state: same order every time.
    for _ in 0..10 {
        let os = FakeOs::default()
            .with_file("/repo/notes.md")
            .with_file("/repo/resume.rs");
        let order: Vec<String> = run_all(&selection, &locators, &os)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(order, expected);
    }
}

// ── The capability boundary, stated rather than hidden ───────────────────────

#[test]
fn a_platform_that_cannot_report_its_windows_says_so_and_opens_nothing() {
    let members = [member("artifact-window", ResourceRole::Continuation)];
    let facts = [("artifact-window", SCHEMA_WINDOW, "Some Window")];
    let outcome = outcome_for("artifact-window", &["artifact-window"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    let mut os = FakeOs::default();
    os.windows_unreadable = Some("Accessibility permission is not granted".to_string());
    let results = run_all(&selection, &locators(&facts), &os);

    match &results[0].1 {
        TargetStatus::Unavailable { reason } | TargetStatus::Failed { reason } => {
            assert!(
                reason.contains("Accessibility"),
                "the real reason reaches the user: {reason}"
            );
        }
        other => panic!("an unreadable window list must not read as success: {other:?}"),
    }
    assert!(os.performed().is_empty());
}

#[test]
fn a_file_open_in_several_windows_is_delegated_with_the_reason_recorded() {
    // Evo declines to choose between two equally valid windows. It hands the
    // file to the system exactly as Finder would, and the binding records why —
    // it does not silently pick one and call it a focus.
    let os = FakeOs::default()
        .showing_document("/repo/shared.md", "shared.md — A", 1, "Editor A")
        .showing_document("/repo/shared.md", "shared.md — B", 2, "Editor B");

    let binding = evo_execution::bind_resource(
        &artifact("artifact-shared-doc"),
        &LocatorKind::FilePath("/repo/shared.md".to_string()),
        &os,
    )
    .expect("a file always has an honest fallback");

    match binding.resource() {
        BoundResource::OpenFile { path, witnessed } => {
            assert_eq!(path, "/repo/shared.md");
            let reason = witnessed
                .reason()
                .expect("not choosing between two windows is a fact worth stating");
            assert!(reason.contains('2'), "the count reaches the user: {reason}");
        }
        other => panic!("two candidate windows must not become a focus: {other:?}"),
    }
}

#[test]
fn a_step_that_will_not_be_attempted_says_so_before_it_runs() {
    // The plan itself distinguishes "Evo will act on this" from "Evo will
    // report on this", so the surface can say which is which without guessing.
    let members = [
        member("artifact-editor", ResourceRole::Continuation),
        member("artifact-commit", ResourceRole::Primary),
    ];
    let facts = [
        ("artifact-editor", SCHEMA_FILE, "/repo/main.rs"),
        ("artifact-commit", SCHEMA_COMMIT, "abc1234"),
    ];
    let outcome = outcome_for("artifact-editor", &["artifact-editor", "artifact-commit"]);
    let selection = select_restoration(&outcome, &members, &evidence(&facts));

    let os = FakeOs::default().with_file("/repo/main.rs");
    let steps = plan_selection(&selection, &locators(&facts), &os);

    let editor = steps
        .iter()
        .find(|step| step.artifact_id().as_str() == "artifact-editor")
        .expect("the editor is planned");
    assert!(editor.will_attempt());
    assert!(matches!(editor.plan(), StepPlan::Attempt(_)));

    let commit = steps
        .iter()
        .find(|step| step.artifact_id().as_str() == "artifact-commit")
        .expect("the commit is planned as a refusal, not omitted");
    assert!(!commit.will_attempt());
    assert!(matches!(commit.plan(), StepPlan::Refuse(_)));
}
