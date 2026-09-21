//! Real macOS event-source → derived-understanding verification.
//!
//! This harness exercises the ACTUAL capture runtime — not injected signals:
//!
//!   REAL macOS OS EVENT
//!       ↓
//!   actual capture source (FSEvents / real git / real sockets)
//!       ↓
//!   LiveCapturePipeline
//!       ↓
//!   Observation  (canonical, persisted, append-only)
//!       ↓
//!   Artifact identity → Acts → Episodes → attention → affinity →
//!   grouping → significance → roles → Workspace projection
//!       ↓
//!   the Workspace-first surface (Home / Detail / sidebar)
//!
//! It starts `Runtime::start_vertical_runtime_with_storage_root` over a
//! disposable storage root and then performs REAL filesystem operations in a
//! disposable directory that the real FSEvents source watches: file saves, a
//! second write mechanism on the same file (tool switch), a real `git commit`
//! (witnessed via `.git/logs/HEAD`), and real user declarations through the
//! daemon's trusted UNIX sockets.
//!
//! ## Honesty contract
//!
//!   - NO `MacOSSignal` values are synthesized anywhere in this harness.
//!   - Every stage that depends on a capability the OS may withhold —
//!     Accessibility (window focus and URL navigation), a working `git`, a
//!     bindable UNIX socket — reports BLOCKED and continues. It never fakes the
//!     signal, and it never asserts over evidence it did not obtain. A stage
//!     that could not run is reported as not run.
//!   - The daemon never witnesses its own writes (storage lives outside the
//!     watched directory, and the composition boundary suppresses storage-root
//!     paths in any case).
//!
//! ## What this harness asserts, and what it deliberately does not
//!
//! Amended for RFC-0014. This harness previously asserted that two file saves
//! and one commit form a three-member Workspace, which was the behaviour of the
//! superseded Workspace Formation pipeline (IS-0012): a Workspace was created
//! at the first sighting of a resource and Artifacts attached to it afterwards.
//!
//! Under the engagement model a body of work must satisfy three independent
//! necessary conditions (RFC-0014 R2): Relationship, Return, and Depth. A file
//! save is an `Incidental` act — a person saving a draft and a formatter
//! rewriting the file are witnessed identically — so writes alone establish no
//! human presence, and a single commit is one human act, not a return.
//! Therefore, when Accessibility is unavailable and the only real evidence this
//! machine can produce is file writes plus a commit, the correct derived result
//! is *no body of work*. That is what this harness now asserts, and it is the
//! honest answer rather than a fabricated one: Evo did not witness anyone
//! sitting with these files, so it declines to claim they were working on them.
//! The engagement model's positive convergence case is proved where real
//! attention evidence can be constructed —
//! `evo_engagement::engagement::tests::heterogeneous_resources_of_one_task_converge`
//! and `evo-daemon`'s `verify_scenarios` (scenarios A–O). The corresponding
//! negative case is pinned by
//! `changes_plus_one_deliberate_act_are_not_yet_a_body_of_work`.
//!
//! What remains genuinely verifiable here, over real OS events, is the part of
//! the vertical path that does not depend on attention capture: that real
//! FSEvents writes become canonical Observations, that a witnessed subject
//! resolves to a stable canonical Artifact, that changing the *tool* that wrote
//! a file never splits its identity, that a restart over the same storage root
//! reconstructs the same identities and the same derived understanding, and
//! that the shell's surface is a projection of that understanding rather than a
//! separate parallel state.
//!
//! This is developer/verification tooling, not product code.
//!
//! Usage:
//!   cargo run -p evo-desktop --example verify_real_capture
//! Keep the disposable roots for evo_doctor:
//!   EVO_REAL_CAPTURE_KEEP=1 cargo run -p evo-desktop --example verify_real_capture
//! Override the watched scratch directory (must be FSEvents-visible):
//!   EVO_VERIFY_SCRATCH=... cargo run -p evo-desktop --example verify_real_capture

use evo_artifact::artifact_id::ArtifactId;
use evo_daemon::cache::CanonicalIndex;
use evo_daemon::continuation::{ContinuationResponse, submit_continuation_surface};
use evo_daemon::daemon_status::{CaptureStatus, read_capture_status};
use evo_daemon::runtime::Runtime;
use evo_desktop::state::{self, ShellView};

use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

/// A stage outcome. `Blocked` is a first-class result, not a failure: it means
/// the OS withheld a capability, and the honest report is that the stage did
/// not run.
enum Stage {
    Ok(String),
    Blocked(String),
}

struct Report {
    checks: Vec<String>,
    blocked: Vec<String>,
}

impl Report {
    fn new() -> Self {
        Self {
            checks: Vec::new(),
            blocked: Vec::new(),
        }
    }

    fn record(&mut self, stage: Stage) {
        match stage {
            Stage::Ok(detail) => {
                println!("== {detail}");
                self.checks.push(detail);
            }
            Stage::Blocked(detail) => {
                println!("== BLOCKED — {detail}");
                self.blocked.push(detail);
            }
        }
    }
}

fn main() {
    let stamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let home = PathBuf::from(std::env::var_os("HOME").expect("HOME is set"));

    // The watched directory must be a path the real FSEvents collector accepts,
    // because that is the whole point of this harness: a write somewhere the
    // collector ignores would verify nothing. `is_user_visible_path` is the
    // collector's own scope predicate, so the check is against the real rule
    // rather than a restatement of it. `target/` is inside the checkout (hence
    // under the home directory and non-hidden) and conventionally scratch, so a
    // verification run leaves nothing behind anywhere a person keeps work.
    let test_dir = std::env::var_os("EVO_VERIFY_SCRATCH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target").join(format!("evo-real-capture-{stamp}")));
    let test_dir = std::path::absolute(&test_dir).unwrap_or(test_dir);
    if !evo_capture::macos_fsevents::is_user_visible_path(
        &test_dir.to_string_lossy(),
        &home.to_string_lossy(),
    ) {
        panic!(
            "the watched scratch path {} is outside the collector's capture scope \
             (it must sit under $HOME with no hidden or Library component); a write \
             there would not be witnessed, so the run would verify nothing",
            test_dir.display()
        );
    }

    let repo_a = test_dir.join("repo-a");
    let repo_b = test_dir.join("repo-b");
    let storage = std::env::temp_dir().join(format!("evo-real-capture-storage-{stamp}"));
    assert!(
        !storage.starts_with(&home),
        "the storage root must live outside the FSEvents-watched home directory"
    );
    std::fs::create_dir_all(&repo_a).expect("create repo-a");
    std::fs::create_dir_all(&repo_b).expect("create repo-b");
    println!("== storage root: {}", storage.display());
    println!(
        "== watched test dir (witnessed by real FSEvents): {}",
        test_dir.display()
    );

    let plan_path = repo_a.join("plan.md");
    let notes_path = repo_a.join("notes.md");
    let report_path = repo_b.join("report.md");
    let plan = plan_path.to_string_lossy().into_owned();
    let notes = notes_path.to_string_lossy().into_owned();
    let report_subject = report_path.to_string_lossy().into_owned();

    let mut report = Report::new();

    // ── Start the REAL capture runtime ─────────────────────────────────────
    let runtime = Runtime::new();
    let (_handle, boundaries) = runtime
        .start_vertical_runtime_with_storage_root(storage.clone())
        .expect("the real vertical runtime starts");
    // The FSEvents stream (0.5s latency) starts on its own thread; give it
    // time to go live before the first write so nothing is missed.
    thread::sleep(Duration::from_millis(2500));

    // Honest capture-status report (never assumed, read from the runtime's own
    // operational report).
    let attention_capture = match read_capture_status(&storage) {
        Some(CaptureStatus::Full) => {
            println!("== capture status: FULL (window-focus active)");
            true
        }
        Some(CaptureStatus::Partial { detail }) => {
            println!("== capture status: PARTIAL — {detail}");
            false
        }
        None => {
            println!("== capture status: none written");
            false
        }
    };

    // ── Real file capture (FSEvents → FileSaved) ───────────────────────────
    std::fs::write(&plan_path, "v1\n").expect("real write plan.md");
    let index = match wait_for_index(&storage, Duration::from_secs(25), |i| {
        i.subjects().values().any(|s| s == &plan)
    }) {
        Some(index) => index,
        None => {
            report.record(Stage::Blocked(
                "real FSEvents never delivered a write of plan.md within 25s; file capture \
                 is unavailable on this machine, so no later stage can be verified"
                    .to_string(),
            ));
            finish(&mut report, &test_dir, &storage, attention_capture);
            let _ = boundaries;
            return;
        }
    };
    let plan_artifact = artifact_for_subject(&index, &plan);
    report.record(Stage::Ok(format!(
        "real FSEvents captured plan.md → canonical Artifact {plan_artifact}"
    )));

    std::fs::write(&notes_path, "n1\n").expect("real write notes.md");
    let index = wait_for_index(&storage, Duration::from_secs(25), |i| {
        i.subjects().values().any(|s| s == &notes)
    })
    .expect("notes.md is witnessed once plan.md was");
    let notes_artifact = artifact_for_subject(&index, &notes);
    report.record(Stage::Ok(format!(
        "real FSEvents captured notes.md → canonical Artifact {notes_artifact}"
    )));

    // ── Tool switch on the SAME file ───────────────────────────────────────
    // Second write of plan.md through a DIFFERENT real mechanism (append-mode
    // write via OpenOptions instead of truncate-and-write). The canonical
    // subject is the same path — the tool is never an identity input.
    {
        use std::io::Write as _;
        let mut handle = std::fs::OpenOptions::new()
            .append(true)
            .open(&plan_path)
            .expect("open plan.md");
        handle
            .write_all(b"v2\n")
            .expect("real append write to plan.md");
    }
    let index = wait_for_index(&storage, Duration::from_secs(25), |i| {
        i.subjects()
            .iter()
            .filter(|(_, w)| w.as_str() == plan)
            .count()
            == 1
    })
    .expect("plan.md still resolves to exactly one subject after the second write");
    assert_eq!(
        artifact_for_subject(&index, &plan),
        plan_artifact,
        "a tool switch must never change canonical identity"
    );
    report.record(Stage::Ok(
        "tool switch: plan.md re-saved by a second real mechanism → the SAME Artifact, no split"
            .to_string(),
    ));

    // ── Real git commit (FSEvents → CommitMade via .git/logs/HEAD) ─────────
    // Every git invocation is pinned to the scratch repository. Without this, a
    // failed `git init` leaves no `.git` in the scratch directory and git's
    // ordinary discovery walks *up* the tree until it finds one — which, for a
    // scratch path inside a checkout, is the real repository. A verification
    // tool that commits the user's actual working tree and then reports that
    // repository's HEAD as the commit it "triggered" is both a destructive
    // action and fabricated evidence.
    let commit = real_commit(&repo_a, &test_dir);
    let commit_artifact = match commit {
        Some(hash) => match wait_for_index(&storage, Duration::from_secs(25), |i| {
            i.subjects().values().any(|s| s == &hash)
        }) {
            Some(index) => {
                let artifact = artifact_for_subject(&index, &hash);
                report.record(Stage::Ok(format!(
                    "real git commit captured via .git/logs/HEAD → canonical Artifact {artifact}"
                )));
                Some(artifact)
            }
            None => {
                report.record(Stage::Blocked(
                    "a real commit was made but FSEvents did not deliver .git/logs/HEAD \
                     within 25s"
                        .to_string(),
                ));
                None
            }
        },
        None => {
            report.record(Stage::Blocked(
                "no real commit could be made in the scratch repository (git unavailable, \
                 or writes to .git are denied in this environment); the commit-capture \
                 stage did not run and no commit signal was synthesized"
                    .to_string(),
            ));
            None
        }
    };

    // ── What the evidence so far does, and does not, establish ─────────────
    // RFC-0014 R2. Writes are Incidental; a commit is one Deliberate act. With
    // no attention capture there is no Return and no Depth, so the honest
    // derived result is no body of work. Asserting this is the point: it is the
    // behaviour that stops observation history from being mirrored into Home.
    std::fs::write(&report_path, "r1\n").expect("real write report.md");
    let index = wait_for_index(&storage, Duration::from_secs(25), |i| {
        i.subjects().values().any(|s| s == &report_subject)
    })
    .expect("report.md is witnessed");
    let report_artifact = artifact_for_subject(&index, &report_subject);
    report.record(Stage::Ok(format!(
        "real FSEvents captured report.md in a second directory → Artifact {report_artifact}"
    )));

    let derived = index.workspaces().len();
    let witnessed = index.subjects().len();
    if attention_capture {
        // Accessibility is granted, so focus evidence exists and this machine
        // may legitimately derive work. Nothing about the count can be asserted
        // — it depends on what the person was actually doing — so it is
        // reported, not checked.
        report.record(Stage::Ok(format!(
            "attention capture is active: {witnessed} witnessed subject(s) → {derived} \
             body/bodies of work (count depends on real activity; reported, not asserted)"
        )));
    } else {
        assert_eq!(
            derived, 0,
            "with no attention capture the only evidence is Incidental writes plus at most \
             one Deliberate commit, which satisfies neither Return nor Depth (RFC-0014 R2); \
             {witnessed} witnessed subject(s) must yield no body of work, got {derived}"
        );
        report.record(Stage::Ok(format!(
            "{witnessed} witnessed subject(s), no attention evidence → 0 bodies of work: \
             changes alone are never work, and Home does not mirror the Observation log"
        )));
    }

    // ── Real declaration through the daemon's trusted socket ───────────────
    // Declaration is ground truth and outranks inference (RFC-0014 R11), so a
    // declared continuation surface is the one path that can establish work
    // from this evidence. RFC-0013 requires at least two distinct witnessed
    // subjects.
    let declared = submit_real_declaration(&storage, &[plan.clone(), notes.clone()]);
    let declared_workspace = match declared {
        Ok(()) => {
            match wait_for_index(&storage, Duration::from_secs(15), |i| {
                !i.workspaces().is_empty()
            }) {
                Some(index) => {
                    let workspace = workspace_containing_opt(&index, &plan_artifact)
                        .expect("the declared surface establishes a body of work over plan.md");
                    let members: Vec<String> = workspace
                        .attachments()
                        .iter()
                        .map(|a| a.artifact_id().to_string())
                        .collect();
                    assert!(
                        members.iter().any(|id| id == &plan_artifact.to_string())
                            && members.iter().any(|id| id == &notes_artifact.to_string()),
                        "the declared subjects are members of the declared body of work"
                    );
                    assert!(
                        !members.iter().any(|id| id == &report_artifact.to_string()),
                        "real activity in an undeclared directory must not be pulled into a \
                         declared body of work by recency alone"
                    );
                    report.record(Stage::Ok(format!(
                        "real declaration (socket): {{plan, notes}} → one body of work, \
                         identity {}, and report.md stayed out of it",
                        workspace.id()
                    )));
                    Some(workspace.id().clone())
                }
                None => {
                    report.record(Stage::Blocked(
                        "the declaration was accepted but no body of work appeared within 15s"
                            .to_string(),
                    ));
                    None
                }
            }
        }
        Err(reason) => {
            report.record(Stage::Blocked(format!(
                "the daemon's continuation socket was unreachable ({reason}); the \
                 declaration stage did not run and no declaration was simulated"
            )));
            None
        }
    };

    // ── Restart / replay over the SAME storage root ────────────────────────
    // Identity and the derived understanding must both survive a restart, and
    // the second reading must equal the first (RFC-0014 R12). This is the check
    // that the understanding is a function of the log and not of process state.
    let before_ids: Vec<String> = index
        .workspaces()
        .iter()
        .map(|w| w.id().to_string())
        .collect();
    drop(_handle);
    thread::sleep(Duration::from_millis(300));
    let runtime_again = Runtime::new();
    let (_handle2, _boundaries2) = runtime_again
        .start_vertical_runtime_with_storage_root(storage.clone())
        .expect("the runtime restarts on the same storage root");
    thread::sleep(Duration::from_millis(500));
    drop(_handle2);

    let replayed = CanonicalIndex::new(storage.clone()).expect("replay rebuilds");
    assert_eq!(
        artifact_for_subject(&replayed, &plan),
        plan_artifact,
        "plan.md identity is stable across a restart"
    );
    assert_eq!(
        artifact_for_subject(&replayed, &notes),
        notes_artifact,
        "notes.md identity is stable across a restart"
    );
    if let Some(ref artifact) = commit_artifact {
        assert_eq!(
            artifact_for_subject(&replayed, &real_commit_subject(&replayed, artifact)),
            *artifact,
            "the commit's identity is stable across a restart"
        );
    }
    let after_ids: Vec<String> = replayed
        .workspaces()
        .iter()
        .map(|w| w.id().to_string())
        .collect();
    assert_eq!(
        after_ids, before_ids,
        "replaying the Observation log must reconstruct the same bodies of work, in the \
         same canonical order"
    );
    if let Some(ref id) = declared_workspace {
        assert!(
            after_ids.iter().any(|seen| seen == &id.to_string()),
            "the declared body of work survives a restart"
        );
    }
    report.record(Stage::Ok(format!(
        "restart: Artifact identities stable, and the derived understanding re-derived \
         identically from the log ({} body/bodies of work, same identities)",
        after_ids.len()
    )));

    // ── The Workspace-first surface over the replayed state ────────────────
    // The shell must be a projection of the derived understanding: every card
    // it shows is a derived body of work, and opening one is keyed by canonical
    // identity. When nothing was derived, Home must show nothing rather than
    // falling back to witnessed subjects.
    let display = state::display_state_from_index(&replayed);
    let cards: Vec<state::WorkspaceCard> = display
        .workspaces
        .iter()
        .map(|workspace| {
            let outcome = display.outcomes.get(&workspace.id().to_string());
            let selection = outcome
                .map(|outcome| state::selection_for(workspace, outcome, replayed.locators()));
            state::workspace_card(
                workspace,
                outcome,
                selection.as_ref(),
                &display.subjects,
                &display.kinds,
                &display.titles,
                &display.standings,
            )
        })
        .collect();
    assert_eq!(
        cards.len(),
        after_ids.len(),
        "Home shows exactly the derived bodies of work — no more, and no fallback rows"
    );
    for card in &cards {
        assert!(
            after_ids.iter().any(|id| id == &card.id.to_string()),
            "every Home card is keyed by a derived Workspace identity"
        );
    }
    if let Some(first) = display.workspaces.first() {
        assert_eq!(
            ShellView::from_selection(Some(first.id()), &display.workspaces),
            ShellView::Workspace(first.id().clone()),
            "opening a body of work is keyed by its canonical WorkspaceId"
        );
    }
    report.record(Stage::Ok(format!(
        "Workspace-first surface: {} Home card(s), each keyed by a derived Workspace \
         identity, with {} witnessed subject(s) in the log — the surface reflects the \
         understanding, not the history",
        cards.len(),
        replayed.subjects().len()
    )));

    finish(&mut report, &test_dir, &storage, attention_capture);
    let _ = boundaries;
}

/// Prints the honest source summary and cleans up the disposable roots.
fn finish(report: &mut Report, test_dir: &Path, storage: &Path, attention_capture: bool) {
    println!("\n── REAL sources exercised ──");
    println!("  FSEvents file saves, real persistence, real derivation, real restart replay");
    if attention_capture {
        println!("  window-focus: ACTIVE (Accessibility granted)");
    } else {
        println!(
            "  WINDOW CAPTURE BLOCKED — Accessibility permission unavailable; no focus \
             signal was injected"
        );
        println!(
            "  URL CAPTURE BLOCKED — the URL poller requires Accessibility; no navigation \
             signal was produced"
        );
    }
    println!(
        "\n{} check(s) passed over real OS events",
        report.checks.len()
    );
    if report.blocked.is_empty() {
        println!("0 stage(s) blocked");
    } else {
        println!(
            "{} stage(s) BLOCKED and therefore not verified:",
            report.blocked.len()
        );
        for blocked in &report.blocked {
            println!("  - {blocked}");
        }
    }

    if std::env::var_os("EVO_REAL_CAPTURE_KEEP").is_some() {
        println!(
            "== keeping roots for evo_doctor: storage={} test_dir={}",
            storage.display(),
            test_dir.display()
        );
    } else {
        let _ = std::fs::remove_dir_all(test_dir);
        let _ = std::fs::remove_dir_all(storage);
    }

    if report.blocked.is_empty() {
        println!("\nVERIFY-REAL-CAPTURE OK");
    } else {
        println!(
            "\nVERIFY-REAL-CAPTURE PARTIAL — every stage that ran passed; blocked stages are listed above"
        );
        std::process::exit(2);
    }
}

// ── helpers ────────────────────────────────────────────────────────────────

/// Attempts a real commit in a scratch repository, returning the commit hash.
///
/// Returns `None` when git is unavailable or the environment denies the writes
/// this needs, so the caller can report the stage as blocked. Nothing is
/// synthesized on failure.
fn real_commit(repo: &Path, ceiling: &Path) -> Option<String> {
    let git_dir = repo.join(".git");
    let template = ceiling.join("empty-git-template");
    std::fs::create_dir_all(&template).ok()?;
    let run = |args: &[&str], pinned: bool| -> Option<std::process::Output> {
        let mut command = Command::new("git");
        command
            .args(args)
            .current_dir(repo)
            // Stop git's upward discovery walk, so no command can ever resolve
            // to the real repository this scratch path sits inside.
            .env("GIT_CEILING_DIRECTORIES", ceiling)
            // A verification run must not read the developer's own git identity
            // or hooks, so it cannot depend on machine-specific configuration.
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", ceiling);
        if pinned {
            command.env("GIT_DIR", &git_dir).env("GIT_WORK_TREE", repo);
        }
        let output = command.output().ok()?;
        if !output.status.success() {
            println!(
                "  git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            return None;
        }
        Some(output)
    };
    // `init` is the one command that must not be given GIT_DIR, which would
    // make it initialise the pinned path rather than the repository directory.
    run(
        &[
            "init",
            "-b",
            "main",
            "-q",
            "--template",
            &template.to_string_lossy(),
        ],
        false,
    )?;
    // If init did not actually produce the repository, every later command
    // would resolve somewhere else. Refuse rather than risk that.
    if !git_dir.is_dir() {
        println!("  git init produced no .git directory in the scratch repository");
        return None;
    }
    run(&["add", "-A"], true)?;
    run(
        &[
            "-c",
            "user.name=Evo",
            "-c",
            "user.email=evo@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            "real capture commit",
        ],
        true,
    )?;
    let output = run(&["rev-parse", "HEAD"], true)?;
    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // The reflog is what FSEvents witnesses, so a commit with no reflog entry
    // would be a commit this harness cannot honestly claim was captured.
    if hash.is_empty() || !git_dir.join("logs").join("HEAD").is_file() {
        println!("  the commit produced no reflog entry, so it cannot be witnessed");
        return None;
    }
    Some(hash)
}

/// Recovers the witnessed subject behind an Artifact, for the restart check.
fn real_commit_subject(index: &CanonicalIndex, artifact: &ArtifactId) -> String {
    index
        .subjects()
        .iter()
        .find(|(id, _)| id.as_str() == artifact.to_string())
        .map(|(_, witnessed)| witnessed.clone())
        .expect("the Artifact resolves back to its witnessed subject")
}

/// Polls the persisted canonical index until the predicate holds, returning
/// `None` on timeout so the caller can report a blocked stage rather than
/// panicking on a capability the OS withheld.
fn wait_for_index<F>(root: &Path, timeout: Duration, pred: F) -> Option<CanonicalIndex>
where
    F: Fn(&CanonicalIndex) -> bool,
{
    let start = Instant::now();
    loop {
        if let Ok(index) = CanonicalIndex::new(root.to_path_buf()) {
            if pred(&index) {
                return Some(index);
            }
        }
        if start.elapsed() > timeout {
            return None;
        }
        thread::sleep(Duration::from_millis(200));
    }
}

/// Submits a real declaration through the daemon's socket, retrying briefly
/// while the listener comes up. Returns the reason on failure; never simulates
/// an accepted declaration.
fn submit_real_declaration(root: &Path, subjects: &[String]) -> Result<(), String> {
    let start = Instant::now();
    loop {
        match submit_continuation_surface(root, subjects) {
            Ok(ContinuationResponse::Accepted) => return Ok(()),
            Ok(ContinuationResponse::Rejected(reason)) => {
                return Err(format!("rejected: {reason}"));
            }
            Err(err) => {
                if start.elapsed() > Duration::from_secs(8) {
                    return Err(err.to_string());
                }
                thread::sleep(Duration::from_millis(150));
            }
        }
    }
}

/// Resolves the canonical Artifact a witnessed subject established.
fn artifact_for_subject(index: &CanonicalIndex, subject: &str) -> ArtifactId {
    index
        .subjects()
        .iter()
        .find(|(_, witnessed)| witnessed.as_str() == subject)
        .map(|(id, _)| ArtifactId::new(id.clone()).expect("canonical artifact id"))
        .expect("witnessed subject resolves to its canonical Artifact")
}

fn workspace_containing_opt<'a>(
    index: &'a CanonicalIndex,
    artifact: &ArtifactId,
) -> Option<&'a evo_workspace::workspace::Workspace> {
    index.workspaces().iter().find(|workspace| {
        workspace
            .attachments()
            .iter()
            .any(|attachment| attachment.artifact_id() == artifact)
    })
}
