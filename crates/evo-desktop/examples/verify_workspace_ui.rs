//! Real-pipeline verification of the Workspace-first product surface.
//!
//! Drives the exact daemon vertical pipeline (adapter → engine → persist →
//! derive) over a canonical sequence a real user produces: TWO unrelated bodies
//! of work, each worked on across three sittings — A: a ledger reconciliation
//! over {a1, a2, a3} in one repository, B: roster planning over {b1, b2} in
//! another — with one continuation-surface declaration (RFC-0013) made inside A
//! over {a1, a2}, then verifies the Workspace-first behaviors:
//!
//! 1. the home surface presents BOTH bodies of work with canonical identity;
//! 2. opening A / opening B is keyed by canonical Workspace identity;
//! 3. A's current continuation contains only A's declared members, and B —
//!    which the declaration never mentions — keeps the continuation its own
//!    evidence establishes; members never leak between bodies of work;
//! 4. Continue for A never executes B's resources and vice versa;
//! 5. Back navigation returns to the Workspace list;
//! 6. a later declaration (supersession) updates that body of work's
//!    continuation view — A3 moves from related work into the current
//!    continuation, never guessed, only declared;
//! 7. restart (fresh derived index) preserves the derived selections.
//!
//! This is developer/verification tooling, not product code.
//!
//! Usage:
//!   cargo run -p evo-desktop --example verify_workspace_ui

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

/// The windows the two bodies of work are read in. Their names carry no meaning
/// to Evo — nothing matches on them — but they are what makes each history a
/// record of someone's attention rather than a list of file writes.
const LEDGER_EDITOR: &str = "ledger reconciliation — Editor";
const ROSTER_CONSOLE: &str = "roster planning — Console";

fn main() {
    let stamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let root = std::env::temp_dir().join(format!("evo-workspace-ui-verify-{stamp}"));
    println!("== storage root: {}", root.display());
    let _storage_guard = evo_storage::Storage::with_thread_root(root.clone());

    // ── Two real repositories on the real file system ──────────────────────
    let repo_a_dir = root.join("repo-a");
    let repo_b_dir = root.join("repo-b");
    std::fs::create_dir_all(&repo_a_dir).expect("create repo A");
    std::fs::create_dir_all(&repo_b_dir).expect("create repo B");
    for file in ["a1.md", "a2.md", "a3-old.md"] {
        std::fs::write(repo_a_dir.join(file), "content\n").expect("write A file");
    }
    for file in ["b1.md", "b2-old.md"] {
        std::fs::write(repo_b_dir.join(file), "content\n").expect("write B file");
    }
    let a1 = repo_a_dir.join("a1.md").to_string_lossy().into_owned();
    let a2 = repo_a_dir.join("a2.md").to_string_lossy().into_owned();
    let a3 = repo_a_dir.join("a3-old.md").to_string_lossy().into_owned();
    let b1 = repo_b_dir.join("b1.md").to_string_lossy().into_owned();
    let b2 = repo_b_dir.join("b2-old.md").to_string_lossy().into_owned();
    let git_a = repo_a_dir.join(".git").to_string_lossy().into_owned();
    let git_b = repo_b_dir.join(".git").to_string_lossy().into_owned();

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

    // Two bodies of work, each worked on across three sittings hours apart.
    // Membership evidence precedes the content it describes, exactly as the
    // FSEvents producer emits it. Nothing here declares what the work *is* —
    // both bodies of work are derived from how the resources were used.
    const HOUR: u64 = 60 * 60;
    let mut moment = 1u64;
    for member in [&a1, &a2, &a3] {
        drive(
            &mut pipeline,
            MacOSSignal::RepositoryMembership {
                member: (*member).clone(),
                repository: git_a.clone(),
                observed_at: t(moment),
            },
        );
    }
    // Body of work A: the ledger reconciliation. Read in an editor, with three
    // documents saved as the person works through them.
    for sitting in 0..3 {
        for _pass in 0..4 {
            drive(
                &mut pipeline,
                MacOSSignal::WindowFocusGained {
                    subject: LEDGER_EDITOR.into(),
                    observed_at: t(moment),
                    process_identifier: None,
                    owning_process_name: None,
                    observed_state: None,
                },
            );
            moment += 300;
            for (member, dwell) in [(&a1, 30u64), (&a2, 60), (&a3, 120)] {
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
        if sitting < 2 {
            moment += 3 * HOUR;
        }
    }
    // Body of work B: roster planning, witnessed later than A (recency must not
    // change A's continuation), sharing no vocabulary and no container with it.
    moment += 3 * HOUR;
    for member in [&b1, &b2] {
        drive(
            &mut pipeline,
            MacOSSignal::RepositoryMembership {
                member: (*member).clone(),
                repository: git_b.clone(),
                observed_at: t(moment),
            },
        );
    }
    for sitting in 0..3 {
        for _pass in 0..4 {
            drive(
                &mut pipeline,
                MacOSSignal::WindowFocusGained {
                    subject: ROSTER_CONSOLE.into(),
                    observed_at: t(moment),
                    process_identifier: None,
                    owning_process_name: None,
                    observed_state: None,
                },
            );
            moment += 300;
            for (member, dwell) in [(&b1, 30u64), (&b2, 120)] {
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
        if sitting < 2 {
            moment += 3 * HOUR;
        }
    }
    // A body of work is a claim about a whole history, so it is derived once the
    // evidence has stopped arriving — the same call the daemon's worker makes.
    pipeline.settle();
    while receiver.try_recv().is_ok() {}

    // One declaration, inside body of work A: A's surface becomes {a1, a2} and
    // a3 stays historical. Saying where you continue also says those resources
    // belong together, so a declaration never spans two bodies of work — and it
    // says nothing at all about B, which keeps the continuation its own
    // evidence establishes (RFC-0013 per-Workspace consumption).
    let declared_at = moment + HOUR;
    drive(
        &mut pipeline,
        MacOSSignal::ContinuationSurface {
            subjects: vec![a1.clone(), a2.clone()],
            observed_at: t(declared_at),
        },
    );

    // ── 1. Home: both Workspaces, canonical identity ───────────────────────
    let index = CanonicalIndex::new(root.clone()).expect("fresh index rebuilds");
    let a1_artifact = artifact_for_subject(&index, &a1);
    let b1_artifact = artifact_for_subject(&index, &b1);
    let a3_artifact = artifact_for_subject(&index, &a3);
    let b2_artifact = artifact_for_subject(&index, &b2);
    assert_eq!(index.workspaces().len(), 2, "two canonical Workspaces");
    let workspace_a = workspace_containing(&index, &a1_artifact);
    let workspace_b = workspace_containing(&index, &b1_artifact);
    assert_ne!(workspace_a.id(), workspace_b.id());
    let state = state::display_state_from_index(&index);
    assert_eq!(state.workspaces.len(), 2);
    println!(
        "== home: {} Workspaces (A={}, B={})",
        state.workspaces.len(),
        workspace_a.id(),
        workspace_b.id()
    );

    // The home surface derives one card per Workspace, keyed by canonical
    // identity, from canonical state only.
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
    assert!(card_ids.contains(workspace_a.id()));
    assert!(card_ids.contains(workspace_b.id()));
    let card_a = cards
        .iter()
        .find(|card| card.id == *workspace_a.id())
        .expect("card A");
    let card_b = cards
        .iter()
        .find(|card| card.id == *workspace_b.id())
        .expect("card B");
    assert_eq!(
        card_a.continuation_count, 2,
        "A card shows 2 resources in the current continuation"
    );
    assert!(
        card_b.continuation_count > 0,
        "B card shows the continuation its own evidence establishes, not an empty one \
         because the person happened to declare a surface in A"
    );
    println!(
        "== home cards reference canonical identity: A has {} current-continuation resource(s), B has {}",
        card_a.continuation_count, card_b.continuation_count
    );

    // ── 2. Opening A / B is keyed by canonical Workspace identity ──────────
    let workspaces: Vec<Workspace> = state.workspaces.clone();
    assert_eq!(
        ShellView::from_selection(Some(workspace_a.id()), &workspaces),
        ShellView::Workspace(workspace_a.id().clone())
    );
    assert_eq!(
        ShellView::from_selection(Some(workspace_b.id()), &workspaces),
        ShellView::Workspace(workspace_b.id().clone())
    );
    println!("== opening A opens A; opening B opens B (keyed by WorkspaceId)");

    // ── 3. Continuation isolation: A's surface only A's, B's only B's ──────
    let outcome_a = state
        .outcomes
        .get(&workspace_a.id().to_string())
        .expect("A outcome");
    let outcome_b = state
        .outcomes
        .get(&workspace_b.id().to_string())
        .expect("B outcome");
    let selection_a = state::selection_for(workspace_a, outcome_a, index.locators());
    let selection_b = state::selection_for(workspace_b, outcome_b, index.locators());
    assert_eq!(selection_a.restore_worthy().len(), 2, "A: a1 + a2");
    assert!(
        historical_ids(&selection_a).contains(&a3_artifact.to_string()),
        "A3 stays historical in A: {:?}",
        historical_ids(&selection_a)
    );
    // B was never mentioned by the declaration, so its continuation is derived
    // from its own evidence — never empty merely because someone declared a
    // surface somewhere else.
    assert!(
        !selection_b.restore_worthy().is_empty(),
        "B keeps a derived continuation of its own"
    );
    let b_members = member_ids(workspace_b);
    assert!(
        selection_b
            .restore_worthy()
            .iter()
            .all(|selected| b_members.contains(&selected.artifact_id().to_string())),
        "B's continuation contains only B's own members"
    );
    assert!(
        selection_a
            .restore_worthy()
            .iter()
            .all(|selected| selected.artifact_id() != &b1_artifact),
        "A's continuation never contains B's resources"
    );
    assert!(
        selection_b
            .restore_worthy()
            .iter()
            .all(|selected| selected.artifact_id() != &a1_artifact),
        "B's continuation never contains A's resources"
    );
    assert!(
        historical_ids(&selection_b).contains(&b2_artifact.to_string())
            || selection_b
                .restore_worthy()
                .iter()
                .any(|selected| selected.artifact_id() == &b2_artifact),
        "B2 is part of B either way, never lost: {:?}",
        historical_ids(&selection_b)
    );
    println!(
        "== A continuation: a1+a2 (a3 historical) · B continuation: {} derived resource(s) — no leakage",
        selection_b.restore_worthy().len()
    );

    // ── 4. Continue for A never executes B resources (and vice versa) ──────
    let locators = state::locators_from_locators(index.locators());
    let recorder_a = RecordingExecutor::default();
    let report_a =
        state::run_execution(Some(outcome_a), Some(&selection_a), &locators, &recorder_a);
    let calls_a = recorder_a.calls.lock().unwrap().clone();
    let mut calls_a_sorted = calls_a.clone();
    calls_a_sorted.sort();
    let mut expected_a = vec![format!("file:{a1}"), format!("file:{a2}")];
    expected_a.sort();
    assert_eq!(
        calls_a_sorted, expected_a,
        "A's Continue executes exactly a1+a2"
    );
    assert!(
        calls_a.iter().all(|call| !call.contains("repo-b")),
        "A's Continue must never execute B resources: {calls_a:?}"
    );
    assert!(report_a.all_opened());

    let recorder_b = RecordingExecutor::default();
    let report_b =
        state::run_execution(Some(outcome_b), Some(&selection_b), &locators, &recorder_b);
    let calls_b = recorder_b.calls.lock().unwrap().clone();
    assert!(!calls_b.is_empty(), "B's Continue does something");
    assert!(
        calls_b.iter().all(|call| !call.contains("repo-a")),
        "B's Continue must never execute A resources: {calls_b:?}"
    );
    assert!(
        calls_b
            .iter()
            .all(|call| call.contains("repo-b") || call.contains(ROSTER_CONSOLE)),
        "B's Continue touches only B's own resources: {calls_b:?}"
    );
    assert!(report_b.all_opened());
    println!(
        "== Continue for A: {} attempted · Continue for B: {} attempted — no cross-Workspace execution",
        calls_a.len(),
        calls_b.len()
    );

    // ── 5. Back navigation returns to the Workspace list ───────────────────
    assert_eq!(
        ShellView::from_selection(None, &workspaces),
        ShellView::Home,
        "Back returns to the primary surface — the Workspace list"
    );
    println!("== back navigation returns to the Workspace list");

    // ── 6. Supersession: a later declaration updates the continuation ──────
    // The user now declares A3 into the current continuation (the correction
    // loop at the canonical boundary): the declaration is the only thing that
    // changes continuation — recency never would.
    drive(
        &mut pipeline,
        MacOSSignal::ContinuationSurface {
            subjects: vec![a1.clone(), a2.clone(), a3.clone()],
            observed_at: t(declared_at + HOUR),
        },
    );
    let updated = CanonicalIndex::new(root.clone()).expect("rebuilt after supersession");
    let updated_state = state::display_state_from_index(&updated);
    let updated_a = workspace_containing(&updated, &artifact_for_subject(&updated, &a1));
    let updated_outcome_a = updated_state
        .outcomes
        .get(&updated_a.id().to_string())
        .expect("updated A outcome");
    let updated_selection_a =
        state::selection_for(updated_a, updated_outcome_a, updated.locators());
    let mut updated_a_worthy: Vec<String> = updated_selection_a
        .restore_worthy()
        .iter()
        .map(|selected| selected.artifact_id().to_string())
        .collect();
    updated_a_worthy.sort();
    let mut expected_updated = vec![
        a1_artifact.to_string(),
        artifact_for_subject(&updated, &a2).to_string(),
        artifact_for_subject(&updated, &a3).to_string(),
    ];
    expected_updated.sort();
    assert_eq!(
        updated_a_worthy, expected_updated,
        "the new declaration moves A3 from related work into the current continuation"
    );
    // Workspace identity and membership are unchanged by the declaration.
    assert_eq!(updated_a.id(), workspace_a.id());
    assert_eq!(
        member_ids(updated_a),
        member_ids(workspace_a),
        "declaring where you continue does not change what belongs to the work"
    );
    println!(
        "== supersession: declaring A3 moves it into A's continuation; membership and identity unchanged"
    );

    // ── 7. Restart preserves the derived selections ────────────────────────
    let restart = CanonicalIndex::new(root.clone()).expect("restart rebuilds");
    let restart_state = state::display_state_from_index(&restart);
    let restart_a = workspace_containing(&restart, &artifact_for_subject(&restart, &a1));
    let restart_outcome_a = restart_state
        .outcomes
        .get(&restart_a.id().to_string())
        .expect("restart A outcome");
    let restart_selection_a =
        state::selection_for(restart_a, restart_outcome_a, restart.locators());
    assert_eq!(
        restart_selection_a.restore_worthy(),
        updated_selection_a.restore_worthy()
    );
    assert_eq!(
        restart_selection_a.withheld(),
        updated_selection_a.withheld()
    );
    println!("== restart: fresh index preserves the derived selections");

    std::fs::remove_dir_all(&root).ok();
    println!("\nVERIFY-WORKSPACE-UI OK");
}

/// The recording executor: records every OS action, reports Opened, and is
/// itself the preflight source (files = real stat; windows = an injected,
/// deterministic snapshot in which the two windows this history was read in are
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
                title: LEDGER_EDITOR.to_string(),
                owner_pid: 1,
                owner_name: "Editor".to_string(),
            },
            evo_execution::macos::WindowInfo {
                title: ROSTER_CONSOLE.to_string(),
                owner_pid: 2,
                owner_name: "Console".to_string(),
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

/// The members a selection withholds: part of the work, not part of what the
/// person said they are continuing, and therefore never opened — whether Evo
/// keeps them to hand or only in the record.
fn historical_ids(selection: &evo_execution::RestorationSelection) -> BTreeSet<String> {
    selection
        .withheld()
        .iter()
        .map(|member| member.artifact_id().to_string())
        .collect()
}
