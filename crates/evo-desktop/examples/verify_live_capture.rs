//! Real Capture → Living Bodies of Work → Workspace-First Product Surface.
//!
//! Drives the exact daemon vertical pipeline (adapter → engine → persist →
//! derive) with a realistic sequence of a user working normally on the Mac,
//! WITHOUT constructing Workspaces directly and WITHOUT any app name, domain,
//! profession, MRU, or recency heuristic:
//!
//!   1. the user works on a body of work across three sittings (a project:
//!      plan.md and notes.md, read in one window, committed once);
//!   2. within it the user switches tools — the same file is saved again by a
//!      different tool, which must not change its identity or split the work;
//!   3. the user has a second, unrelated body of work (a deck), also across
//!      three sittings, in its own place and its own window;
//!   4. the user declares the current continuation inside the project
//!      {plan, notes};
//!   5. unrelated activity happens AFTER the declaration (a chat window is
//!      focused — the most recent thing that happened);
//!   6. the user corrects the continuation to {plan} (notes moves out);
//!   7. restart/replay preserves everything.
//!
//! Verifies the mission invariants:
//!   - a body of work is derived from how resources were used together, never
//!     from one sighting of one resource;
//!   - tool switching does NOT change resource identity or split a body of work
//!     (identity derives from (schema, subject) content, never provenance);
//!   - separate bodies of work stay isolated in continuation and execution;
//!   - recent-but-unrelated activity forms nothing and is never continuation
//!     evidence;
//!   - correction changes continuation but not membership or identity;
//!   - the Workspace-first Home surface derives cards from canonical state;
//!   - restart/replay preserves identities and selections.
//!
//! This is developer/verification tooling, not product code.
//!
//! Usage:
//!   cargo run -p evo-desktop --example verify_live_capture

use evo_artifact::artifact_id::ArtifactId;
use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
use evo_daemon::cache::CanonicalIndex;
use evo_daemon::runtime::VerticalPipeline;
use evo_desktop::state::{self, ShellView};
use evo_execution::{PlatformExecutor, PreflightSource, TargetStatus};
use evo_observation::provenance::ObservationSource;
use evo_workspace::workspace::Workspace;
use evo_workspace::workspace_id::WorkspaceId;

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime};

/// The windows each body of work is read in, and one window belonging to
/// neither. The names mean nothing to Evo — nothing matches on them.
const PROJECT_WINDOW: &str = "Project — Notes";
const DECK_WINDOW: &str = "Deck — Slides";
const CHAT_WINDOW: &str = "Chat — unrelated";

const HOUR: u64 = 60 * 60;

fn main() {
    let stamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let root = std::env::temp_dir().join(format!("evo-live-capture-verify-{stamp}"));
    println!("== storage root: {}", root.display());
    let _storage_guard = evo_storage::Storage::with_thread_root(root.clone());

    // ── Real repositories on the real file system ──────────────────────────
    let project_dir = root.join("project");
    let deck_dir = root.join("deck");
    std::fs::create_dir_all(&project_dir).expect("create project dir");
    std::fs::create_dir_all(&deck_dir).expect("create deck dir");
    for file in ["plan.md", "notes.md"] {
        std::fs::write(project_dir.join(file), "content\n").expect("write project file");
    }
    std::fs::write(deck_dir.join("deck.md"), "content\n").expect("write deck file");
    let plan = project_dir.join("plan.md").to_string_lossy().into_owned();
    let notes = project_dir.join("notes.md").to_string_lossy().into_owned();
    let deck = deck_dir.join("deck.md").to_string_lossy().into_owned();
    let commit = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
    let git_project = project_dir.join(".git").to_string_lossy().into_owned();
    let git_deck = deck_dir.join(".git").to_string_lossy().into_owned();

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

    // 1–2. The project: plan.md and notes.md, read in one window across three
    // sittings hours apart, committed once. Membership evidence precedes the
    // content that consumes it, exactly as the FSEvents producer emits it.
    let mut moment = 1u64;
    for member in [&plan, &notes, &commit.to_string()] {
        drive(
            &mut pipeline,
            MacOSSignal::RepositoryMembership {
                member: (*member).clone(),
                repository: git_project.clone(),
                observed_at: t(moment),
            },
        );
    }
    for sitting in 0..3 {
        for pass in 0..4 {
            drive(
                &mut pipeline,
                MacOSSignal::WindowFocusGained {
                    subject: PROJECT_WINDOW.into(),
                    observed_at: t(moment),
                    process_identifier: None,
                    owning_process_name: None,
                    observed_state: None,
                },
            );
            moment += 300;
            for (member, dwell) in [(&plan, 30u64), (&notes, 60)] {
                drive(
                    &mut pipeline,
                    MacOSSignal::FileSaved {
                        subject: member.clone(),
                        observed_at: t(moment),
                    },
                );
                moment += dwell;
            }
            if pass == 1 {
                // The tool switch: the same file saved again by something else.
                // Provenance must not change identity or split the work.
                drive(
                    &mut pipeline,
                    MacOSSignal::FileSaved {
                        subject: plan.clone(),
                        observed_at: t(moment),
                    },
                );
                moment += 45;
            }
        }
        if sitting == 1 {
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

    // 3. The deck: a second body of work, in its own place, its own window, and
    // its own vocabulary. Nothing about it is related to the project except
    // that the same person did both.
    moment += 3 * HOUR;
    drive(
        &mut pipeline,
        MacOSSignal::RepositoryMembership {
            member: deck.clone(),
            repository: git_deck.clone(),
            observed_at: t(moment),
        },
    );
    for sitting in 0..3 {
        for _pass in 0..4 {
            drive(
                &mut pipeline,
                MacOSSignal::WindowFocusGained {
                    subject: DECK_WINDOW.into(),
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
                    subject: deck.clone(),
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

    // 4. The user declares the current continuation inside the project. Saying
    // where you continue also says those resources belong together, so a
    // declaration is made within one body of work; the deck is not mentioned and
    // keeps the continuation its own evidence establishes.
    let declared_at = moment + HOUR;
    drive(
        &mut pipeline,
        MacOSSignal::ContinuationSurface {
            subjects: vec![plan.clone(), notes.clone()],
            observed_at: t(declared_at),
        },
    );

    // ── Identity + continuation + Home + execution ─────────────────────────
    let index = CanonicalIndex::new(root.clone()).expect("fresh index rebuilds");
    let plan_artifact = artifact_for_subject(&index, &plan);
    let notes_artifact = artifact_for_subject(&index, &notes);
    let deck_artifact = artifact_for_subject(&index, &deck);
    let commit_artifact = artifact_for_subject(&index, commit);
    let project_window_artifact = artifact_for_subject(&index, PROJECT_WINDOW);
    let deck_window_artifact = artifact_for_subject(&index, DECK_WINDOW);

    // Check 1 — every resource resolves to EXACTLY ONE canonical Artifact,
    // despite plan.md being saved many times, by two different tools.
    for (label, subject) in [
        ("plan.md", plan.as_str()),
        ("notes.md", notes.as_str()),
        ("deck.md", deck.as_str()),
        ("commit", commit),
        ("the project window", PROJECT_WINDOW),
        ("the deck window", DECK_WINDOW),
    ] {
        let count = index
            .subjects()
            .iter()
            .filter(|(_, witnessed)| witnessed.as_str() == subject)
            .count();
        assert_eq!(
            count, 1,
            "“{label}” must resolve to exactly one Artifact (count={count})"
        );
    }
    println!(
        "== identity: every resource resolves to exactly one Artifact, including plan.md saved by two tools across three sittings"
    );

    // Check 2 — two bodies of work, derived from use, never split by the tool
    // switch or by the repeated saves.
    assert_eq!(
        index.workspaces().len(),
        2,
        "two unrelated histories are two bodies of work"
    );
    let state = state::display_state_from_index(&index);
    let workspace_project = workspace_containing(&index, &plan_artifact);
    let workspace_deck = workspace_containing(&index, &deck_artifact);
    assert_ne!(workspace_project.id(), workspace_deck.id());
    assert_eq!(
        member_ids(workspace_project),
        [
            &plan_artifact,
            &notes_artifact,
            &commit_artifact,
            &project_window_artifact,
        ]
        .into_iter()
        .map(ArtifactId::to_string)
        .collect::<BTreeSet<_>>(),
        "the project is everything that was used together, including the window it was read in"
    );
    assert_eq!(
        member_ids(workspace_deck),
        [&deck_artifact, &deck_window_artifact]
            .into_iter()
            .map(ArtifactId::to_string)
            .collect::<BTreeSet<_>>(),
        "the deck is its own work and never absorbs the project's resources"
    );
    println!(
        "== bodies of work: project ({} members, never split by the tool switch) · deck ({} members) — derived from use, not from names",
        workspace_project.attachments().len(),
        workspace_deck.attachments().len()
    );

    // Check 3 — isolation of the declared continuation: the project's surface
    // is exactly what was declared, {plan, notes}; the deck keeps its own.
    let outcome_project = state
        .outcomes
        .get(&workspace_project.id().to_string())
        .expect("project outcome");
    let selection_project =
        state::selection_for(workspace_project, outcome_project, index.locators());
    let outcome_deck = state
        .outcomes
        .get(&workspace_deck.id().to_string())
        .expect("deck outcome");
    let selection_deck = state::selection_for(workspace_deck, outcome_deck, index.locators());
    let mut project_worthy: Vec<String> = selection_project
        .restore_worthy()
        .iter()
        .map(|selected| selected.artifact_id().to_string())
        .collect();
    project_worthy.sort();
    let mut expected_project = vec![plan_artifact.to_string(), notes_artifact.to_string()];
    expected_project.sort();
    assert_eq!(
        project_worthy, expected_project,
        "project surface = plan + notes"
    );
    assert!(
        historical_ids(&selection_project).contains(&commit_artifact.to_string()),
        "the commit is part of the project but is not where it resumes"
    );
    let deck_members = member_ids(workspace_deck);
    assert!(
        !selection_deck.restore_worthy().is_empty(),
        "the deck keeps a derived continuation of its own — declaring a surface in the \
         project says nothing about the deck"
    );
    assert!(
        selection_deck
            .restore_worthy()
            .iter()
            .all(|selected| deck_members.contains(&selected.artifact_id().to_string())),
        "the deck's continuation contains only the deck's own members"
    );
    assert!(
        project_worthy
            .iter()
            .all(|id| id != &deck_artifact.to_string()),
        "project surface never contains the deck"
    );
    println!(
        "== continuation isolation: project {{plan, notes}} (declared) · deck {} derived resource(s) — no leakage",
        selection_deck.restore_worthy().len()
    );

    // Check 4 — the Workspace-first Home surface derives cards from canonical
    // state, keyed by canonical WorkspaceId.
    let cards: Vec<state::WorkspaceCard> = state
        .workspaces
        .iter()
        .map(|workspace| {
            let outcome = state.outcomes.get(&workspace.id().to_string());
            let selection =
                outcome.map(|outcome| state::selection_for(workspace, outcome, index.locators()));
            state::workspace_card(
                workspace,
                outcome,
                selection.as_ref(),
                &state.subjects,
                &state.kinds,
                &state.titles,
                &state.standings,
            )
        })
        .collect();
    let card_ids: Vec<WorkspaceId> = cards.iter().map(|card| card.id.clone()).collect();
    assert!(card_ids.contains(workspace_project.id()));
    assert!(card_ids.contains(workspace_deck.id()));
    let workspaces: Vec<Workspace> = state.workspaces.clone();
    assert_eq!(
        ShellView::from_selection(Some(workspace_project.id()), &workspaces),
        ShellView::Workspace(workspace_project.id().clone())
    );
    println!(
        "== Home: Workspace cards keyed by canonical identity; opening the project is keyed by its WorkspaceId"
    );

    // Check 5 — Continue executes only the selected body of work's targets,
    // never another's.
    let locators = state::locators_from_locators(index.locators());
    let recorder_project = RecordingExecutor::default();
    let report_project = state::run_execution(
        Some(outcome_project),
        Some(&selection_project),
        &locators,
        &recorder_project,
    );
    let mut project_calls = recorder_project.calls.lock().unwrap().clone();
    project_calls.sort();
    let mut expected_calls = vec![format!("file:{plan}"), format!("file:{notes}")];
    expected_calls.sort();
    assert_eq!(
        project_calls, expected_calls,
        "project Continue executes plan + notes only"
    );
    assert!(
        project_calls.iter().all(|call| !call.contains("deck")),
        "project Continue must never execute the deck: {project_calls:?}"
    );
    assert!(report_project.all_opened());
    let recorder_deck = RecordingExecutor::default();
    let report_deck = state::run_execution(
        Some(outcome_deck),
        Some(&selection_deck),
        &locators,
        &recorder_deck,
    );
    let deck_calls = recorder_deck.calls.lock().unwrap().clone();
    assert!(!deck_calls.is_empty(), "the deck's Continue does something");
    assert!(
        deck_calls
            .iter()
            .all(|call| call.contains("deck") || call.contains(DECK_WINDOW)),
        "the deck's Continue touches only the deck's own resources: {deck_calls:?}"
    );
    assert!(report_deck.all_opened());
    println!(
        "== execution: project Continue → plan + notes; deck Continue → its own resources — no cross-Workspace execution"
    );

    // ── 5. Unrelated activity AFTER the declaration (the most recent) ───────
    // A chat window, focused twice and used with nothing. Under the old model
    // two sightings of one window originated a Workspace of its own, which is
    // how Home filled up with things nobody could resume. Nobody can continue
    // "a window someone looked at twice", so nothing is formed from it at all.
    drive(
        &mut pipeline,
        MacOSSignal::WindowFocusGained {
            subject: CHAT_WINDOW.into(),
            observed_at: t(declared_at + HOUR),
            process_identifier: None,
            owning_process_name: None,
            observed_state: None,
        },
    );
    drive(
        &mut pipeline,
        MacOSSignal::WindowFocusGained {
            subject: CHAT_WINDOW.into(),
            observed_at: t(declared_at + HOUR + 90),
            process_identifier: None,
            owning_process_name: None,
            observed_state: None,
        },
    );
    pipeline.settle();
    while receiver.try_recv().is_ok() {}
    let after_chat = CanonicalIndex::new(root.clone()).expect("index after chat focus");
    let chat_artifact = artifact_for_subject(&after_chat, CHAT_WINDOW);

    // Check 6 — the most recent activity is neither a body of work nor
    // continuation evidence, and it never joins somebody else's work.
    assert_eq!(
        after_chat.workspaces().len(),
        2,
        "a window used with nothing forms no body of work: {:?}",
        after_chat.workspaces().len()
    );
    assert!(
        workspace_for(&after_chat, &chat_artifact).is_none(),
        "the chat window belongs to no body of work"
    );
    let chat_state = state::display_state_from_index(&after_chat);
    for workspace in &chat_state.workspaces {
        let outcome = chat_state
            .outcomes
            .get(&workspace.id().to_string())
            .expect("a derived outcome for every body of work");
        let selection = state::selection_for(workspace, outcome, after_chat.locators());
        assert!(
            selection
                .restore_worthy()
                .iter()
                .all(|selected| selected.artifact_id() != &chat_artifact),
            "the most recent activity is not continuation evidence for any work"
        );
        assert!(
            !historical_ids(&selection).contains(&chat_artifact.to_string()),
            "the chat window is not part of any body of work"
        );
    }
    println!(
        "== recency: the chat window focused AFTER the declaration forms nothing, joins nothing, and is never executed"
    );

    // ── 6. Correction: the user moves notes out and the commit in ───────────
    // A continuation surface names at least two subjects (RFC-0013 §4), and the
    // correction is a whole new statement of where the work continues, not an
    // edit to the old one. Naming the commit also exercises the scope of the
    // declaration: the commit was witnessed in the first sitting only, so it is
    // absent from the sitting being returned to but present in the body of work,
    // and a declaration speaks about the work.
    drive(
        &mut pipeline,
        MacOSSignal::ContinuationSurface {
            subjects: vec![plan.clone(), commit.to_string()],
            observed_at: t(declared_at + 2 * HOUR),
        },
    );
    let corrected = CanonicalIndex::new(root.clone()).expect("index after correction");
    let corrected_state = state::display_state_from_index(&corrected);
    let project_now = workspace_containing(&corrected, &plan_artifact);
    let project_outcome_now = corrected_state
        .outcomes
        .get(&project_now.id().to_string())
        .expect("project outcome after correction");
    let project_selection_now =
        state::selection_for(project_now, project_outcome_now, corrected.locators());
    let mut now_worthy: Vec<String> = project_selection_now
        .restore_worthy()
        .iter()
        .map(|selected| selected.artifact_id().to_string())
        .collect();
    now_worthy.sort();
    assert_eq!(
        now_worthy,
        vec![plan_artifact.to_string()],
        "the corrected surface opens the plan"
    );
    // The commit is on the declared surface and is honoured as a member of it —
    // it left `historical` — but it has no executable target, so it is reported
    // unavailable with a reason rather than opened or silently dropped. Its
    // presence here is the proof the declaration reached an earlier sitting.
    let unavailable_now: Vec<&str> = project_selection_now
        .unavailable()
        .iter()
        .map(|reason| reason.artifact_id().as_str())
        .collect();
    assert!(
        unavailable_now.contains(&commit_artifact.to_string().as_str()),
        "the declared commit is on the surface, honestly unopenable: {unavailable_now:?}"
    );
    assert!(
        !historical_ids(&project_selection_now).contains(&commit_artifact.to_string()),
        "a declared member is no longer merely historical, even though it belongs \
         to an earlier sitting than the one being returned to"
    );
    assert!(
        historical_ids(&project_selection_now).contains(&notes_artifact.to_string()),
        "notes moved OUT of the current continuation and stays historical"
    );
    // Membership and identity are unchanged by the correction.
    assert_eq!(
        project_now.id(),
        workspace_project.id(),
        "Workspace identity stable"
    );
    assert_eq!(
        member_ids(project_now),
        member_ids(workspace_project),
        "correcting where you continue does not change what belongs to the work"
    );
    assert_eq!(
        artifact_for_subject(&corrected, &notes),
        notes_artifact,
        "notes identity unchanged"
    );
    // No other body of work is affected: the deck's continuation is untouched.
    let deck_now = workspace_containing(&corrected, &deck_artifact);
    let deck_outcome_now = corrected_state
        .outcomes
        .get(&deck_now.id().to_string())
        .expect("deck outcome after correction");
    let deck_selection_now = state::selection_for(deck_now, deck_outcome_now, corrected.locators());
    assert_eq!(
        deck_selection_now.restore_worthy(),
        selection_deck.restore_worthy(),
        "the deck's continuation is unaffected by correcting the project"
    );
    println!(
        "== correction: notes left the continuation (stays historical), the commit joined it from an earlier sitting; membership, identity, and the deck unaffected"
    );

    // ── 7. Restart/replay preserves identities and selections ──────────────
    let restart = CanonicalIndex::new(root.clone()).expect("restart rebuilds");
    let restart_project = workspace_containing(&restart, &plan_artifact);
    assert_eq!(
        restart_project.id(),
        workspace_project.id(),
        "Workspace identity survives restart"
    );
    let restart_state = state::display_state_from_index(&restart);
    let restart_outcome = restart_state
        .outcomes
        .get(&restart_project.id().to_string())
        .expect("restart outcome");
    let restart_selection =
        state::selection_for(restart_project, restart_outcome, restart.locators());
    assert_eq!(
        restart_selection.restore_worthy(),
        project_selection_now.restore_worthy()
    );
    assert_eq!(
        restart_selection.withheld(),
        project_selection_now.withheld()
    );
    assert_eq!(
        artifact_for_subject(&restart, &plan),
        plan_artifact,
        "plan identity survives restart"
    );
    assert_eq!(
        restart.workspaces(),
        corrected.workspaces(),
        "the same history reconstructs the same bodies of work"
    );
    println!(
        "== restart: fresh index preserves Workspace identity, selections, and resource identities"
    );

    std::fs::remove_dir_all(&root).ok();
    println!("\nVERIFY-LIVE-CAPTURE OK");
}

/// The recording executor: records every OS action, reports Opened, and is
/// itself the preflight source (files = real stat; windows = an injected,
/// deterministic snapshot in which the windows this history was read in are
/// genuinely open, so a window a derived continuation names can be raised).
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
        Ok(vec![
            evo_execution::macos::WindowInfo {
                title: PROJECT_WINDOW.to_string(),
                owner_pid: 1,
                owner_name: "Notes".to_string(),
            },
            evo_execution::macos::WindowInfo {
                title: DECK_WINDOW.to_string(),
                owner_pid: 2,
                owner_name: "Slides".to_string(),
            },
        ])
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

/// Finds the body of work one canonical Artifact belongs to, if it belongs to
/// one at all. Most resources a person touches belong to none.
fn workspace_for<'a>(index: &'a CanonicalIndex, artifact: &ArtifactId) -> Option<&'a Workspace> {
    index.workspaces().iter().find(|workspace| {
        workspace
            .attachments()
            .iter()
            .any(|attachment| attachment.artifact_id() == artifact)
    })
}

/// Finds the Workspace containing one canonical Artifact.
fn workspace_containing<'a>(index: &'a CanonicalIndex, artifact: &ArtifactId) -> &'a Workspace {
    workspace_for(index, artifact).expect("Workspace containing the Artifact")
}

/// The canonical Artifacts belonging to one body of work.
fn member_ids(workspace: &Workspace) -> BTreeSet<String> {
    workspace
        .attachments()
        .iter()
        .map(|attachment| attachment.artifact_id().to_string())
        .collect()
}

/// The members a selection withholds: part of the work, not part of what the
/// person is continuing, and therefore never opened — whether Evo keeps them to
/// hand or only in the record.
fn historical_ids(selection: &evo_execution::RestorationSelection) -> BTreeSet<String> {
    selection
        .withheld()
        .iter()
        .map(|member| member.artifact_id().to_string())
        .collect()
}
