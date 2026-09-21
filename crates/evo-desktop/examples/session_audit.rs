//! Engineering-audit tool: inspect a real persisted Evo session and run the
//! Continue-My-Work path against it.
//!
//! This is developer/verification tooling, not product code. It reads the
//! same canonical storage boundary the desktop shell consumes and prints the
//! full persisted chain — Observation log, Artifact subjects, Workspaces,
//! Snapshots, derived Restoration outcomes — and, in `continue` mode, routes
//! the user's Resume request through the real Execution layer.
//!
//! Usage (storage root via `EVO_STORAGE_ROOT` or first argument):
//!   EVO_STORAGE_ROOT=/tmp/evo-audit-session cargo run -p evo-desktop --example session_audit -- inspect
//!   EVO_STORAGE_ROOT=/tmp/evo-audit-session cargo run -p evo-desktop --example session_audit -- continue

use evo_daemon::persistence::load_persisted_observations;
use evo_desktop::state::{self, load_display_state};
use evo_engagement::Standing;
use evo_execution::{Locator, LocatorKind, MacOSExecutor};
use evo_restoration::DerivationOutcome;
use evo_workspace::workspace::Workspace;

use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let storage_root = std::env::var_os("EVO_STORAGE_ROOT")
        .map(PathBuf::from)
        .or_else(|| std::env::args().nth(1).map(PathBuf::from))
        .expect("EVO_STORAGE_ROOT or a storage-root argument is required");
    // The mode may be the first or second positional argument depending on
    // whether the storage root came from the environment or the command line.
    let mode = std::env::args()
        .skip(1)
        .find(|value| value == "inspect" || value == "continue")
        .unwrap_or_else(|| "inspect".to_string());

    let state =
        load_display_state(&storage_root).unwrap_or_else(|err| panic!("load display state: {err}"));

    match mode.as_str() {
        "continue" => {
            let target = std::env::args()
                .skip(1)
                .find(|value| value != "inspect" && value != "continue");
            continue_mode(&state, target.as_deref());
        }
        "inspect" | _ => inspect_mode(&storage_root, &state),
    }
}

fn inspect_mode(storage_root: &std::path::Path, state: &state::DisplayState) {
    println!("=== EVO SESSION AUDIT — storage {}", storage_root.display());
    println!();

    // Observation log (IS-0001): the immutable evidence record.
    let observations = load_persisted_observations(storage_root)
        .unwrap_or_else(|err| panic!("load observation log: {err}"));
    println!(
        "Observation log: {} canonical observation(s) persisted",
        observations.len()
    );
    for observation in &observations {
        println!(
            "  [{}] {} · {} · fact={:?}",
            observation
                .provenance()
                .observed_at()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_else(|_| "pre-epoch".to_string()),
            observation.schema().name(),
            observation.provenance().source().as_str(),
            observation
                .evidence()
                .fact(observation.schema().canonical_fact_name().unwrap_or(""))
                .map(|fact| fact.value())
        );
    }
    println!();

    // The current designation (RFC-0011): the user's explicit statement of
    // where their work continues, derived from the persisted log.
    match &state.designation {
        Some((subject, time)) => println!(
            "Current designation: {subject} (witnessed at {})",
            state::local_time_label(*time)
        ),
        None => println!("Current designation: none — Evo does not guess a continuation"),
    }
    println!();

    // Workspaces: the reconstructed work-state record.
    println!(
        "Workspaces remembered: {} ({} displayed on the primary surface)",
        state.workspaces.len(),
        state
            .display
            .as_ref()
            .map(|w| w.id().to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    for workspace in &state.workspaces {
        print_workspace(
            workspace,
            &state.subjects,
            &state.kinds,
            &state.locators,
            &state.outcomes,
            &state.titles,
        );
    }
    println!();

    // The work surface (WORK-MODEL): of everything Evo remembers, what has
    // risen to work and what it keeps only by name. This is exactly the split
    // Home now draws — is_work() bodies are presented as work; Remembered
    // bodies stay findable by name but are held off the surface. Printed here
    // over the real persisted history, so the model is checked against it and
    // not against a fixture.
    println!("=== Work surface: what Home presents, and what it keeps by name ===");
    let standing_of = |workspace: &Workspace| -> Standing {
        state
            .standings
            .get(&workspace.id().to_string())
            .copied()
            .unwrap_or(Standing::Remembered)
    };
    let name_of = |workspace: &Workspace| -> String {
        state
            .titles
            .get(&workspace.id().to_string())
            .cloned()
            .unwrap_or_else(|| {
                state::workspace_surface_label(&state.subjects, &state.kinds, workspace)
            })
    };
    let work: Vec<&Workspace> = state
        .workspaces
        .iter()
        .filter(|workspace| standing_of(workspace).is_work())
        .collect();
    let remembered: Vec<&Workspace> = state
        .workspaces
        .iter()
        .filter(|workspace| !standing_of(workspace).is_work())
        .collect();
    println!(
        "  {} of {} bodies are work; {} are remembered (kept, findable by name, off the work surface)",
        work.len(),
        state.workspaces.len(),
        remembered.len()
    );
    println!("  -- presented as work --");
    if work.is_empty() {
        println!("     (none — nothing has risen to work)");
    }
    for workspace in &work {
        let standing = standing_of(workspace);
        println!(
            "     [{}] {} — {}",
            if standing.is_restorable() {
                "RES"
            } else {
                "CON"
            },
            name_of(workspace),
            standing.phrase()
        );
    }
    println!("  -- remembered, by name only --");
    if remembered.is_empty() {
        println!("     (none)");
    }
    for workspace in &remembered {
        println!(
            "     [REM] {} — {}",
            name_of(workspace),
            Standing::Remembered.phrase()
        );
    }
    println!();

    // Derived restoration understanding for the displayed workspace.
    match state.display.as_ref() {
        Some(_) => {
            println!("=== Restoration understanding for the displayed workspace ===");
            if let Some(outcome) = state.outcome.as_ref() {
                print_outcome(outcome, &state.subjects);
            } else {
                println!("No persisted derivation outcome for the displayed workspace yet.");
            }
        }
        None => println!("No displayed workspace: no continuation can be derived."),
    }
    println!();

    println!("=== What Evo can and cannot do with this record ===");
    let resumable = state
        .workspaces
        .iter()
        .filter(|workspace| {
            state
                .outcomes
                .get(&workspace.id().to_string())
                .and_then(|outcome| outcome.resume_point())
                .map(|rp| state.locators.contains_key(&rp.artifact_id().to_string()))
                .unwrap_or(false)
        })
        .count();
    println!("Workspaces with a derivable Resume Point AND an executable target: {resumable}");
    println!(
        "Workspaces whose derivation names missing continuation evidence (no Next Step under the frozen model): {}",
        state
            .outcomes
            .values()
            .filter(|outcome| outcome.next_step_missing().is_some())
            .count()
    );
}

fn continue_mode(state: &state::DisplayState, target: Option<&str>) {
    println!(
        "=== EVO CONTINUE — routing the user's Resume request through the real Execution layer ==="
    );
    let workspace = match target {
        Some(prefix) => state
            .workspaces
            .iter()
            .find(|workspace| workspace.id().to_string().starts_with(prefix))
            .expect("workspace prefix must match a remembered workspace"),
        None => state
            .display
            .as_ref()
            .expect("no displayed workspace; nothing to continue"),
    };
    let outcome = state
        .outcomes
        .get(&workspace.id().to_string())
        .expect("persisted derivation outcome must exist for the workspace");
    println!("Continuing workspace: {}", workspace.id());
    print_outcome(outcome, &state.subjects);

    let executor = MacOSExecutor::new();
    let report = state::run_execution(
        Some(outcome),
        state.selection.as_ref(),
        &state.locators,
        &executor,
    );
    println!();
    println!("--- execution report ---");
    for attempt in report.attempts() {
        println!(
            "artifact={} status={:?}",
            attempt.artifact_id().as_str(),
            attempt.status()
        );
    }
    println!("summary: {}", state::execution_result_line(&report));
}

fn print_workspace(
    workspace: &Workspace,
    subjects: &HashMap<String, String>,
    kinds: &HashMap<String, String>,
    locators: &HashMap<String, Locator>,
    outcomes: &HashMap<String, DerivationOutcome>,
    titles: &HashMap<String, String>,
) {
    // The name Home heads the card with, then the members it is made of. The
    // name comes first because that is the order a person reads it in, and
    // because whether it names the *work* is the thing worth auditing.
    println!(
        "Workspace {} · “{}” · {} · {} snapshot(s)",
        state::short_identity(&workspace.id().to_string()),
        titles
            .get(&workspace.id().to_string())
            .map(String::as_str)
            .unwrap_or("<no derived name>"),
        state::workspace_surface_label(subjects, kinds, workspace),
        workspace.snapshots().len()
    );
    for snapshot in workspace.snapshots() {
        let ids: Vec<String> = snapshot
            .attachments()
            .iter()
            .map(|attachment| attachment.artifact_id().as_str().to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        println!(
            "  snapshot at unix+{}s · attachments: {}",
            snapshot
                .captured_at()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            ids.join(", ")
        );
    }
    for id in state::distinct_artifacts_across_history(workspace) {
        let subject = state::subject_for(subjects, &id)
            .map(|s| format!("{s:?}"))
            .unwrap_or_else(|| "no witnessed subject".to_string());
        let kind = state::kind_for(kinds, &id)
            .map(|k| format!("{k} · "))
            .unwrap_or_default();
        let target = match locators.get(&id).map(|locator| locator.kind()) {
            Some(LocatorKind::WindowTitle(title)) => format!("focus window {title:?}"),
            Some(LocatorKind::FilePath(path)) => format!("open file {path:?}"),
            Some(LocatorKind::Url(url)) => format!("open URL {url:?}"),
            None => "no executable target (e.g. a commit hash)".to_string(),
        };
        println!(
            "  artifact {} · {kind}subject {} · {target}",
            state::short_artifact_id(&id),
            subject
        );
    }
    match outcomes.get(&workspace.id().to_string()) {
        Some(outcome) => {
            match outcome.resume_point() {
                Some(rp) => println!(
                    "  resume point: {}",
                    state::artifact_label(subjects, rp.artifact_id().as_str())
                ),
                None => {
                    if let Some(missing) = outcome.resume_point_missing() {
                        println!("  resume point missing: {}", missing.reason());
                    }
                }
            }
            println!(
                "  context chain: {} · blockers: {} · next step: {}",
                outcome.context_chain().artifacts().len(),
                outcome.blockers().len(),
                match outcome.next_step() {
                    Some(step) => format!("{step:?}"),
                    None => "not derivable under the frozen model".to_string(),
                }
            );
        }
        None => println!("  no persisted derivation outcome"),
    }
    println!();
}

fn print_outcome(outcome: &DerivationOutcome, subjects: &HashMap<String, String>) {
    match outcome.resume_point() {
        Some(rp) => println!(
            "Resume point: {} ({})",
            state::artifact_label(subjects, rp.artifact_id().as_str()),
            rp.artifact_id().as_str()
        ),
        None => {
            if let Some(missing) = outcome.resume_point_missing() {
                println!("Resume point missing: {}", missing.reason());
            }
        }
    }
    println!(
        "Context chain: {} artifact(s) · Blockers: {}",
        outcome.context_chain().artifacts().len(),
        outcome.blockers().len()
    );
    match outcome.next_step() {
        Some(step) => println!("Next step: {}", step.description()),
        None => {
            if let Some(missing) = outcome.next_step_missing() {
                println!("Next step missing: {}", missing.reason());
            }
        }
    }
    match outcome {
        DerivationOutcome::Complete(_) => println!("Derivation: complete RestorationPlan"),
        DerivationOutcome::Insufficient(_) => {
            println!("Derivation: insufficient (structured, never guessed)")
        }
    }
}
