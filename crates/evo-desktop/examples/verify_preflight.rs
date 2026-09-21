//! Real-pipeline verification of Executor Preflight + per-target outcomes.
//!
//! Drives the exact daemon vertical pipeline (adapter → engine → persist →
//! derive) over a canonical sequence a real user produces: a document review
//! worked on across three sittings — real files witnessed as saved inside one
//! repository, a witnessed-but-never-created file, a commit, and an undeclared
//! repo file — plus a *separate* body of work in an untitled window, and one
//! continuation-surface declaration (RFC-0013) over a subset of the first.
//!
//! Preflight is then exercised against the REAL file system (exact-path
//! stat) with an injected, deterministic window list (never real AX state):
//!
//! 1. an actually existing selected file → READY, and actually attempted;
//! 2. a missing selected file → UNAVAILABLE, never guessed;
//! 3. a renamed/moved file → UNAVAILABLE, no fuzzy recovery;
//! 4. a commit → UNSUPPORTED (identity preserved, no action);
//! 5. multiple matching windows → AMBIGUOUS, never arbitrarily selected;
//! 6. mixed selection → partial execution succeeds: READY targets are
//!    attempted, everything else is skipped, and every surface member
//!    receives an honest outcome;
//! 7. historical / recent-but-undeclared resources are never executed;
//! 8. restart (fresh derived index) preserves the selection and preflight.
//!
//! This is developer/verification tooling, not product code.
//!
//! Usage:
//!   cargo run -p evo-desktop --example verify_preflight

use evo_artifact::artifact_id::ArtifactId;
use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
use evo_daemon::cache::CanonicalIndex;
use evo_daemon::persistence::load_persisted_observations;
use evo_daemon::runtime::VerticalPipeline;
use evo_desktop::state::locators_from_locators;
use evo_execution::{
    PlatformExecutor, PreflightOutcome, PreflightSource, PreflightStatus, RestorationSelection,
    TargetStatus, execute_selection, preflight_selection,
};
use evo_observation::provenance::ObservationSource;
use evo_workspace::workspace::Workspace;

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime};

/// The window the repository work is read in. Its name means nothing to Evo —
/// nothing matches on it — but it is what makes the history a record of
/// someone's attention rather than a list of file writes.
const REVIEW_EDITOR: &str = "release notes review — Editor";
/// The window the *other*, unrelated body of work happens in. Two applications
/// really do have a window by this name, which is the ambiguity under test.
const WINDOW_TITLE: &str = "Untitled";

const HOUR: u64 = 60 * 60;

fn main() {
    let stamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let root = std::env::temp_dir().join(format!("evo-preflight-verify-{stamp}"));
    println!("== storage root: {}", root.display());
    let _storage_guard = evo_storage::Storage::with_thread_root(root.clone());

    // ── A real repository on the real file system ───────────────────────────
    let repo_dir = root.join("repo");
    let repo = repo_dir.join(".git").to_string_lossy().into_owned();
    let plan_path = repo_dir.join("plan.md");
    let notes_path = repo_dir.join("notes.md");
    let old_path = repo_dir.join("old.md");
    // archive.md is WITNESSED but never created: the honest missing case.
    let archive_path = repo_dir.join("archive.md");
    let commit = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
    std::fs::create_dir_all(&repo_dir).expect("create repo dir");
    for file in [&plan_path, &notes_path, &old_path] {
        std::fs::write(file, "content\n").expect("write real file");
    }
    let plan = plan_path.to_string_lossy().into_owned();
    let notes = notes_path.to_string_lossy().into_owned();
    let old = old_path.to_string_lossy().into_owned();
    let archive = archive_path.to_string_lossy().into_owned();

    // The unrelated work lives somewhere else entirely: another directory, no
    // repository, and no vocabulary in common with the release notes.
    let scratch_dir = root.join("jottings");
    std::fs::create_dir_all(&scratch_dir).expect("create scratch dir");
    let scratch_path = scratch_dir.join("shopping.md");
    std::fs::write(&scratch_path, "content\n").expect("write scratch file");
    let scratch = scratch_path.to_string_lossy().into_owned();

    // ── Drive the real daemon pipeline ─────────────────────────────────────
    let source = ObservationSource::new("user_continuation").expect("non-empty");
    let adapter = MacOSAdapter::new(source);
    let mut engine = CaptureEngine::new();
    let (sender, receiver) = channel::<Result<_, evo_daemon::DaemonError>>();
    let mut pipeline = VerticalPipeline::new(sender, root.clone()).expect("pipeline builds");

    let t = |secs: u64| {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(secs))
            .expect("representable")
    };
    // The pipeline is passed in rather than captured, so the history can be
    // settled part-way through — exactly as the daemon's worker does.
    let mut drive = |pipeline: &mut VerticalPipeline, signal: MacOSSignal| {
        let raw = adapter
            .normalize(signal)
            .expect("normalization is infallible")
            .expect("canonical signal");
        let schema = raw.schema();
        let observation = engine.ingest(raw, &schema).expect("observation acceptance");
        pipeline.handle_observation(observation);
        while receiver.try_recv().is_ok() {}
    };

    // Body of work one: the release notes review — plan, notes, archive (never
    // created), a commit, and old (deliberately never declared), read in an
    // editor across three sittings hours apart. RepositoryMembership evidence
    // precedes the content observations that consume it, exactly as the
    // FSEvents producer emits it. Nothing here declares what the work *is*.
    let mut moment = 1u64;
    for member in [&plan, &notes, &archive, &old, &commit.to_string()] {
        drive(
            &mut pipeline,
            MacOSSignal::RepositoryMembership {
                member: (*member).clone(),
                repository: repo.clone(),
                observed_at: t(moment),
            },
        );
    }
    for sitting in 0..3 {
        for _pass in 0..4 {
            drive(
                &mut pipeline,
                MacOSSignal::WindowFocusGained {
                    subject: REVIEW_EDITOR.into(),
                    observed_at: t(moment),
                    process_identifier: None,
                    owning_process_name: None,
                    observed_state: None,
                },
            );
            moment += 300;
            for (member, dwell) in [(&plan, 30u64), (&notes, 60), (&archive, 90), (&old, 120)] {
                drive(
                    &mut pipeline,
                    MacOSSignal::FileSaved {
                        subject: member.clone(),
                        observed_at: t(moment),
                    },
                );
                moment += dwell;
            }
        }
        if sitting == 1 {
            // The work was committed once, in the middle sitting.
            drive(
                &mut pipeline,
                MacOSSignal::CommitMade {
                    subject: commit.to_string(),
                    observed_at: t(moment),
                },
            );
            moment += 60;
        }
        if sitting < 2 {
            moment += 3 * HOUR;
        }
    }

    // Body of work two: something else entirely, in a window two applications
    // happen to give the same name. Separate container, separate vocabulary,
    // separate sittings — and never declared, so its continuation is whatever
    // its own evidence says it is.
    moment += 3 * HOUR;
    for sitting in 0..3 {
        for _pass in 0..4 {
            drive(
                &mut pipeline,
                MacOSSignal::WindowFocusGained {
                    subject: WINDOW_TITLE.into(),
                    observed_at: t(moment),
                    process_identifier: None,
                    owning_process_name: None,
                    observed_state: None,
                },
            );
            moment += 300;
            drive(
                &mut pipeline,
                MacOSSignal::FileSaved {
                    subject: scratch.clone(),
                    observed_at: t(moment),
                },
            );
            moment += 120;
        }
        if sitting < 2 {
            moment += 3 * HOUR;
        }
    }
    // A body of work is a claim about a whole history, so it is derived once the
    // evidence has stopped arriving — the same call the daemon's worker makes.
    pipeline.settle();
    while receiver.try_recv().is_ok() {}

    // The user declares the current continuation across plan, notes, archive
    // and the commit — old.md is deliberately excluded, and so is every part of
    // the unrelated work, which therefore keeps the continuation its own
    // evidence establishes.
    drive(
        &mut pipeline,
        MacOSSignal::ContinuationSurface {
            subjects: vec![
                plan.clone(),
                notes.clone(),
                archive.clone(),
                commit.to_string(),
            ],
            observed_at: t(moment + HOUR),
        },
    );

    // ── Persisted + derived canonical state ────────────────────────────────
    let observations = load_persisted_observations(&root).expect("observations load");
    println!(
        "== persisted observations: {} (content + reference evidence)",
        observations.len()
    );
    let index = CanonicalIndex::new(root.clone()).expect("fresh index rebuilds");

    let plan_artifact = artifact_for_subject(&index, &plan);
    let notes_artifact = artifact_for_subject(&index, &notes);
    let archive_artifact = artifact_for_subject(&index, &archive);
    let commit_artifact = artifact_for_subject(&index, commit);
    let old_artifact = artifact_for_subject(&index, &old);
    let editor_artifact = artifact_for_subject(&index, REVIEW_EDITOR);
    let window_artifact = artifact_for_subject(&index, WINDOW_TITLE);
    let scratch_artifact = artifact_for_subject(&index, &scratch);
    assert_eq!(
        index.workspaces().len(),
        2,
        "two unrelated histories are two bodies of work"
    );
    let repo_workspace = workspace_containing(&index, &plan_artifact);
    let window_workspace = workspace_containing(&index, &window_artifact);
    assert_ne!(repo_workspace.id(), window_workspace.id());
    // Everything the review was made of belongs to the review — including the
    // window it was read in and the file nobody ever declared.
    assert_eq!(
        member_ids(repo_workspace),
        [
            &plan_artifact,
            &notes_artifact,
            &archive_artifact,
            &commit_artifact,
            &old_artifact,
            &editor_artifact,
        ]
        .into_iter()
        .map(ArtifactId::to_string)
        .collect::<BTreeSet<_>>(),
        "the review's membership is what was used together, not what was declared"
    );
    assert!(
        !member_ids(repo_workspace).contains(&window_artifact.to_string()),
        "the unrelated window never joins the review"
    );
    println!(
        "== review body of work: {} members · unrelated window work: {} members",
        repo_workspace.attachments().len(),
        window_workspace.attachments().len()
    );

    // ── The preflight source: REAL file system + injected window list ───────
    // Files resolve by exact-path stat (real machine); windows use an
    // injected, deterministic list so the ambiguous/unavailable cases are
    // verifiable without real AX state.
    #[derive(Clone)]
    struct HarnessSource {
        windows: Vec<evo_execution::macos::WindowInfo>,
    }
    impl PreflightSource for HarnessSource {
        fn file_exists(&self, path: &str) -> bool {
            Path::new(path).exists()
        }
        fn windows(&self) -> Result<Vec<evo_execution::macos::WindowInfo>, String> {
            Ok(self.windows.clone())
        }
    }
    fn two_untitled_windows() -> Vec<evo_execution::macos::WindowInfo> {
        vec![
            evo_execution::macos::WindowInfo {
                title: WINDOW_TITLE.to_string(),
                owner_pid: 1,
                owner_name: "Notes".to_string(),
            },
            evo_execution::macos::WindowInfo {
                title: WINDOW_TITLE.to_string(),
                owner_pid: 2,
                owner_name: "TextEdit".to_string(),
            },
        ]
    }

    // ── 1–5. Preflight of the review against the real file system ──────────
    let repo_selection = selection_for(&index, repo_workspace);
    let preflight_source = HarnessSource {
        windows: two_untitled_windows(),
    };
    let outcomes: Vec<PreflightOutcome> = preflight_selection(&repo_selection, &preflight_source);
    let status_of: HashMap<String, PreflightStatus> = outcomes
        .iter()
        .map(|outcome| (outcome.artifact_id().to_string(), outcome.status().clone()))
        .collect();

    // plan.md actually exists on disk → READY.
    assert!(
        status_of[&plan_artifact.to_string()].is_ready(),
        "an actually existing selected file must be READY: {:?}",
        status_of[&plan_artifact.to_string()]
    );
    // notes.md actually exists → READY.
    assert!(status_of[&notes_artifact.to_string()].is_ready());
    // archive.md was declared but never created → UNAVAILABLE, never guessed.
    assert!(matches!(
        status_of[&archive_artifact.to_string()],
        PreflightStatus::Unavailable { .. }
    ));
    // The commit → UNSUPPORTED (identity preserved, no executable target).
    assert!(matches!(
        status_of[&commit_artifact.to_string()],
        PreflightStatus::Unsupported { .. }
    ));
    // old.md is historical (never declared): it must not be preflighted at
    // all — the outcomes cover only surface members. The same is true of the
    // editor window the review was read in.
    assert!(!status_of.contains_key(&old_artifact.to_string()));
    assert!(!status_of.contains_key(&editor_artifact.to_string()));
    println!(
        "== review preflight: plan=READY notes=READY archive=UNAVAILABLE commit=UNSUPPORTED (old + editor historical, not preflighted)"
    );

    // The unrelated work: nobody declared where it continues, so its
    // continuation is the place the work happens — the window — and two equally
    // valid windows carry that name, so it is AMBIGUOUS, never arbitrarily
    // picked. The file that was only ever saved beside it stays historical.
    let window_selection = selection_for(&index, window_workspace);
    let window_outcomes = preflight_selection(&window_selection, &preflight_source);
    let window_status = window_outcomes
        .iter()
        .find(|outcome| outcome.artifact_id() == &window_artifact)
        .expect("the window is where the unrelated work happens");
    assert!(
        matches!(window_status.status(), PreflightStatus::Ambiguous { .. }),
        "two windows with one title must be AMBIGUOUS: {:?}",
        window_status.status()
    );
    assert!(
        window_selection
            .withheld()
            .iter()
            .any(|member| member.artifact_id() == &scratch_artifact),
        "a file only ever saved beside the work is part of it, but not where it resumes"
    );
    println!("== unrelated work preflight: AMBIGUOUS (2 open windows titled “{WINDOW_TITLE}”)");

    // ── 6. Partial execution: READY attempted, everything else skipped ─────
    let recorder = RecordingExecutor::default();
    let locators = locators_from_locators(index.locators());
    let report = execute_selection(&repo_selection, &locators, &preflight_source, &recorder);
    let calls = recorder.calls.lock().unwrap().clone();
    // Exactly the two READY files were attempted — never the missing archive,
    // never the commit, never the historical old.md. (The attempt order is
    // canonical ascending ArtifactId, which is content-hash derived, so the
    // attempted set is compared, not a hardcoded path order.)
    let mut calls_sorted = calls.clone();
    calls_sorted.sort();
    let mut expected_calls = vec![format!("file:{plan}"), format!("file:{notes}")];
    expected_calls.sort();
    assert_eq!(calls_sorted, expected_calls);
    // Every surface member (4) received an honest per-target outcome.
    assert_eq!(
        report.attempts().len(),
        4,
        "attempts: {:?}",
        report
            .attempts()
            .iter()
            .map(|attempt| (
                attempt.artifact_id().as_str(),
                format!("{:?}", attempt.status())
            ))
            .collect::<Vec<_>>()
    );
    let attempt_of: HashMap<String, &TargetStatus> = report
        .attempts()
        .iter()
        .map(|attempt| (attempt.artifact_id().to_string(), attempt.status()))
        .collect();
    assert!(matches!(
        attempt_of[&plan_artifact.to_string()],
        TargetStatus::Opened { .. }
    ));
    assert!(matches!(
        attempt_of[&notes_artifact.to_string()],
        TargetStatus::Opened { .. }
    ));
    assert!(matches!(
        attempt_of[&archive_artifact.to_string()],
        TargetStatus::Unavailable { .. }
    ));
    assert!(matches!(
        attempt_of[&commit_artifact.to_string()],
        TargetStatus::Unsupported { .. }
    ));
    println!(
        "== partial execution: 2 attempted + opened, 1 unavailable, 1 unsupported; every surface member reported"
    );

    // ── 7. Restart preserves selection and preflight ───────────────────────
    // A fresh derived index (a full replay) reproduces the identical
    // selection; preflighting the re-derived selection against the same OS
    // snapshot reproduces the identical dispositions.
    let restart = CanonicalIndex::new(root.clone()).expect("restart rebuilds");
    let restarted_repo = workspace_containing(&restart, &plan_artifact);
    let restarted_selection = selection_for(&restart, restarted_repo);
    assert_eq!(
        restarted_selection.restore_worthy(),
        repo_selection.restore_worthy()
    );
    assert_eq!(
        restarted_selection.unavailable(),
        repo_selection.unavailable()
    );
    let restart_preflight = preflight_selection(&restarted_selection, &preflight_source);
    assert_eq!(
        restart_preflight
            .iter()
            .map(|outcome| (outcome.artifact_id().to_string(), outcome.status().clone()))
            .collect::<HashMap<_, _>>(),
        status_of
    );
    println!("== restart: fresh index preserves the selection and preflight dispositions");

    // ── 8. Renamed/moved file → UNAVAILABLE, no fuzzy recovery ─────────────
    // Move notes.md; the same canonical path must now preflight UNAVAILABLE,
    // and a second execution must not attempt it and must not substitute the
    // moved file.
    let moved = repo_dir.join("notes-renamed.md");
    std::fs::rename(&notes_path, &moved).expect("rename notes");
    let after_move = preflight_selection(&repo_selection, &preflight_source);
    let notes_status = after_move
        .iter()
        .find(|outcome| outcome.artifact_id() == &notes_artifact)
        .expect("notes remains a surface member");
    assert!(
        matches!(notes_status.status(), PreflightStatus::Unavailable { .. }),
        "a renamed file must be UNAVAILABLE, never fuzzy-resolved: {:?}",
        notes_status.status()
    );
    let recorder2 = RecordingExecutor::default();
    let report2 = execute_selection(&repo_selection, &locators, &preflight_source, &recorder2);
    let calls2 = recorder2.calls.lock().unwrap().clone();
    assert!(
        !calls2.iter().any(|call| call.contains("notes-renamed")),
        "Evo never substitutes the renamed file: {calls2:?}"
    );
    assert_eq!(calls2, vec![format!("file:{plan}")]);
    // The report itself must carry the honest per-target outcome for the
    // renamed member: UNAVAILABLE with the preflight reason, never a guessed
    // substitute and never a silent skip.
    let renamed_attempt = report2
        .attempts()
        .iter()
        .find(|attempt| attempt.artifact_id() == &notes_artifact)
        .expect("the renamed notes remains a surface member and is reported");
    assert!(
        matches!(
            renamed_attempt.status(),
            evo_execution::TargetStatus::Unavailable { .. }
        ),
        "the renamed notes must report UNAVAILABLE: {:?}",
        renamed_attempt.status()
    );
    println!("== renamed file: UNAVAILABLE after move; no fuzzy recovery, no substitution");

    std::fs::remove_dir_all(&root).ok();
    println!("\nVERIFY-PREFLIGHT OK");
}

/// The recording executor: records every OS action, reports Opened, and is
/// itself the preflight source (files = real stat; windows = injected).
#[derive(Debug, Default)]
struct RecordingExecutor {
    calls: std::sync::Mutex<Vec<String>>,
}

impl PlatformExecutor for RecordingExecutor {
    fn open_url(&self, url: &str) -> TargetStatus {
        self.calls.lock().unwrap().push(format!("url:{url}"));
        TargetStatus::Opened {
            detail: format!("opened {url}"),
        }
    }
    fn open_file(&self, path: &str) -> TargetStatus {
        self.calls.lock().unwrap().push(format!("file:{path}"));
        TargetStatus::Opened {
            detail: format!("opened {path}"),
        }
    }
    fn focus_window(&self, title: &str) -> TargetStatus {
        self.calls.lock().unwrap().push(format!("window:{title}"));
        TargetStatus::Opened {
            detail: format!("focused {title}"),
        }
    }
}

impl PreflightSource for RecordingExecutor {
    fn file_exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }
    fn windows(&self) -> Result<Vec<evo_execution::macos::WindowInfo>, String> {
        Ok(vec![])
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

/// Finds the Workspace containing one canonical Artifact.
fn workspace_containing<'a>(index: &'a CanonicalIndex, artifact: &ArtifactId) -> &'a Workspace {
    index
        .workspaces()
        .iter()
        .find(|workspace| {
            workspace
                .attachments()
                .iter()
                .any(|attachment| attachment.artifact_id() == artifact)
        })
        .expect("Workspace containing the Artifact")
}

/// The canonical Artifacts belonging to one body of work.
fn member_ids(workspace: &Workspace) -> BTreeSet<String> {
    workspace
        .attachments()
        .iter()
        .map(|attachment| attachment.artifact_id().to_string())
        .collect()
}

/// Computes the derived selective-restoration selection for one Workspace
/// from canonical inputs only, exactly as the desktop state helper does.
///
/// Membership comes from [`Workspace::members_across_history`] rather than being
/// extracted here, so "exactly as the desktop state helper does" is true because
/// it is the same code and not because two copies happen to agree. They did not
/// always: this helper read the Attachment Set while the desktop briefly read the
/// Snapshot History, which silently dropped any member no sitting was active in.
fn selection_for(index: &CanonicalIndex, workspace: &Workspace) -> RestorationSelection {
    let outcome = index
        .outcomes()
        .get(&workspace.id().to_string())
        .expect("a derived outcome for every body of work");
    let members = workspace.members_across_history();
    evo_execution::select_restoration(outcome, &members, index.locators())
}
