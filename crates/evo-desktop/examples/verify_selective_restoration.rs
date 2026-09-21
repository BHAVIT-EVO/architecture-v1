//! Real-pipeline verification of selective restoration (Resource Identity →
//! Executable Target → Selective Restoration).
//!
//! Drives the exact daemon vertical pipeline (adapter → engine → persist →
//! derive) over the canonical sequence a real person produces: two unrelated
//! bodies of work, each used across several sittings, then one declared
//! continuation surface over a subset of one of them (RFC-0013). It then
//! verifies the derived selection:
//!
//! 1. every witnessed signal is accepted and persisted, and statements about
//!    subjects are kept separate from witnessed content;
//! 2. the two bodies of work are derived from witnessed use and stay separate —
//!    the declaration narrows a body of work, it does not create one;
//! 3. the declared surface is canonical (ascending, deduplicated) and survives
//!    a full replay;
//! 4. only declared surface members are restore-worthy, a declared commit is
//!    honestly unavailable (real identity, no executable target — never
//!    guessed), and undeclared members are withheld — each with an explicit
//!    disposition: kept to hand, or kept only as record;
//! 5. the unrelated body of work receives none of the declaration's members,
//!    and keeps the continuation its own evidence establishes (per-Workspace
//!    intersection: a statement about one body of work is silence about every
//!    other, not an instruction to open nothing);
//! 6. restart (a fresh derived index, built by replaying the log) preserves the
//!    selection, the membership, and every Artifact identity;
//! 7. a corrected declaration supersedes the one before it: the resource the
//!    person dropped stops being restored and becomes historical membership —
//!    never deleted — and the resource they added becomes restore-worthy.
//!
//! Selection is not execution: nothing here opens, focuses or launches
//! anything. It answers what Evo *should* bring back.
//!
//! This is developer/verification tooling, not production code.
//!
//! Usage:
//!   cargo run -p evo-desktop --example verify_selective_restoration

use evo_artifact::artifact_id::ArtifactId;
use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
use evo_daemon::cache::CanonicalIndex;
use evo_daemon::persistence::load_persisted_observations;
use evo_daemon::runtime::VerticalPipeline;
use evo_daemon::workspace_replay::{
    replay_current_continuation_surface, replay_workspaces_from_root,
};
use evo_execution::RestorationSelection;
use evo_execution::select_restoration;
use evo_observation::provenance::ObservationSource;
use evo_workspace::attachment::ResourceRole;
use evo_workspace::workspace::Workspace;

use std::collections::BTreeSet;
use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// The contract review: a document, the counterparty's terms page, the editor it
// is read in, a terminal used to diff it, and one commit.
const CONTRACT_EDITOR: &str = "supplier contract — Editor";
const CONTRACT_FILE: &str = "/Users/alice/contracts/supplier-terms.md";
const CONTRACT_TERMS: &str = "https://supplier.example.com/terms";
const CONTRACT_DIFF: &str = "contract diff — Terminal";
const CONTRACT_COMMIT: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
const CONTRACT_REPOSITORY: &str = "/Users/alice/contracts/.git";

// A second, unrelated body of work, sharing no vocabulary and no container with
// the first. Evo must keep them apart on the evidence alone.
const TIMETABLE_SHEET: &str = "teaching timetable — Numbers";
const TIMETABLE_FILE: &str = "/Users/alice/teaching/timetable.numbers";
const TIMETABLE_REGISTRY: &str = "https://registry.example.edu/timetable";
const TIMETABLE_ROSTER: &str = "timetable roster — Console";

fn main() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let root = std::env::temp_dir().join(format!("evo-selective-restoration-verify-{stamp}"));
    println!("== storage root: {}", root.display());
    // The daemon's worker thread sets the thread-local storage root before
    // constructing the pipeline; mirror that here.
    let _storage_guard = evo_storage::Storage::with_thread_root(root.clone());

    let source = ObservationSource::new("user_continuation").expect("non-empty");
    let adapter = MacOSAdapter::new(source);
    let mut engine = CaptureEngine::new();
    let (sender, receiver) = channel::<Result<_, evo_daemon::DaemonError>>();
    let mut pipeline = VerticalPipeline::new(sender, root.clone()).expect("pipeline builds");

    let mut drive = |signals: &[MacOSSignal]| {
        for signal in signals {
            let raw = adapter
                .normalize(signal.clone())
                .expect("normalization is infallible")
                .expect("canonical signal");
            let schema = raw.schema();
            let observation = engine.ingest(raw, &schema).expect("observation acceptance");
            pipeline.handle_observation(observation);
            while receiver.try_recv().is_ok() {}
        }
        // A body of work is a claim about a whole history, so the pipeline
        // derives one only once evidence has stopped arriving — the same call
        // the daemon's worker makes on an idle tick.
        pipeline.settle();
        while receiver.try_recv().is_ok() {}
    };

    // ── The history ────────────────────────────────────────────────────────
    let (mut signals, contract_ended) = contract_review(0);
    let (timetable, timetable_ended) = teaching_timetable(contract_ended + 3 * HOUR);
    signals.extend(timetable);
    // The person declares where they continue: the document, the terms page,
    // and the commit. Declared in scrambled order to prove canonical ordering;
    // the editor window and the diff terminal are deliberately not named.
    let declared_at = timetable_ended + HOUR;
    signals.push(MacOSSignal::ContinuationSurface {
        subjects: vec![
            CONTRACT_COMMIT.into(),
            CONTRACT_TERMS.into(),
            CONTRACT_FILE.into(),
        ],
        observed_at: at(declared_at),
    });
    let witnessed = signals.len();
    drive(&signals);

    // ── 1. Canonical persistence ───────────────────────────────────────────
    let observations = load_persisted_observations(&root).expect("observations load");
    let reference = observations
        .iter()
        .filter(|observation| observation.schema().is_reference_only())
        .count();
    let content = observations.len() - reference;
    println!(
        "== persisted observations: {} of {witnessed} witnessed ({content} content, \
         {reference} statements about subjects)",
        observations.len()
    );
    assert_eq!(
        observations.len(),
        witnessed,
        "every witnessed signal is persisted, and nothing is invented"
    );
    // Two repository-membership facts and one continuation declaration. These
    // are statements *about* subjects: they must never count as attention.
    assert_eq!(reference, 3, "two memberships + one declaration");

    // ── 2. Two bodies of work, derived from witnessed use ──────────────────
    let index = CanonicalIndex::new(root.clone()).expect("fresh index rebuilds");
    let contract_file = artifact_for_subject(&index, CONTRACT_FILE);
    let contract_terms = artifact_for_subject(&index, CONTRACT_TERMS);
    let contract_editor = artifact_for_subject(&index, CONTRACT_EDITOR);
    let contract_diff = artifact_for_subject(&index, CONTRACT_DIFF);
    let contract_commit = artifact_for_subject(&index, CONTRACT_COMMIT);
    let timetable_file = artifact_for_subject(&index, TIMETABLE_FILE);
    let contract_work = workspace_containing(&index, &contract_file).clone();
    let timetable_work = workspace_containing(&index, &timetable_file).clone();
    println!(
        "== bodies of work derived: {} — contract review ({} members), teaching timetable ({} members)",
        index.workspaces().len(),
        contract_work.attachments().len(),
        timetable_work.attachments().len()
    );
    assert_eq!(
        index.workspaces().len(),
        2,
        "two unrelated bodies of work, neither absorbed into the other"
    );
    assert_ne!(contract_work.id(), timetable_work.id());
    for artifact in [
        &contract_terms,
        &contract_editor,
        &contract_diff,
        &contract_commit,
    ] {
        assert_eq!(
            workspace_containing(&index, artifact).id(),
            contract_work.id(),
            "everything used with the contract belongs to the contract work"
        );
    }

    // ── 3. The declared surface is canonical (ordered, deduplicated) ───────
    let surface = index
        .current_continuation_surface()
        .expect("declaration persisted and current");
    println!("== current surface: {surface:?}");
    assert!(
        surface.windows(2).all(|pair| pair[0] <= pair[1]),
        "canonical ascending order, whatever order it was stated in"
    );
    assert_eq!(surface.len(), 3);
    let replayed_surface = replay_current_continuation_surface(&root).expect("canonical replay");
    assert_eq!(index.current_continuation_surface(), replayed_surface);

    // ── 4. Selective restoration of the contract work ──────────────────────
    let selection = selection_for(&index, &contract_work);
    let worthy: BTreeSet<String> = selection
        .restore_worthy()
        .iter()
        .map(|selected| selected.artifact_id().to_string())
        .collect();
    println!(
        "== contract work: {} open now, {} unavailable, {} available if needed, {} history only",
        selection.restore_worthy().len(),
        selection.unavailable().len(),
        selection.available_if_needed().count(),
        selection.history_only().count()
    );
    assert!(
        worthy.contains(contract_file.as_str()) && worthy.contains(contract_terms.as_str()),
        "the declared document and terms page are restore-worthy: {worthy:?}"
    );
    assert_eq!(selection.restore_worthy().len(), 2);
    // The commit is a real resource with identity but no executable target.
    // Evo says so instead of guessing a way to reopen it.
    assert_eq!(selection.unavailable().len(), 1);
    assert_eq!(
        selection.unavailable()[0].artifact_id(),
        &contract_commit,
        "the unavailable member is the commit, honestly named"
    );
    assert!(selection.unavailable()[0].identity().is_some());
    assert!(
        selection.unavailable()[0]
            .reason()
            .contains("no executable target")
    );
    // The two windows were used with this work and remain part of it. They were
    // not declared, so they are context to be shown, not windows to be opened.
    let historical: BTreeSet<String> = selection
        .withheld()
        .iter()
        .map(|member| member.artifact_id().to_string())
        .collect();
    assert!(
        historical.contains(contract_editor.as_str())
            && historical.contains(contract_diff.as_str()),
        "undeclared members stay withheld membership: {historical:?}"
    );
    // Withheld is not one bucket. Every withheld member says which way it is
    // kept and why, and nothing withheld opens unasked.
    for member in selection.withheld() {
        assert!(
            !member.disposition().opens_unasked(),
            "nothing outside the continuation opens: {}",
            member.artifact_id()
        );
        assert!(
            !member.reason().is_empty(),
            "every withheld member states why: {}",
            member.artifact_id()
        );
    }
    // These two windows are places this work happened, so Evo keeps them to
    // hand rather than filing them away as things it merely saw.
    let to_hand: BTreeSet<String> = selection
        .available_if_needed()
        .map(|member| member.artifact_id().to_string())
        .collect();
    assert!(
        to_hand.contains(contract_editor.as_str()) && to_hand.contains(contract_diff.as_str()),
        "a place the work happened is offered, not buried: {to_hand:?}"
    );

    // ── 5. The unrelated body of work keeps its own continuation ───────────
    // Saying where you continue in one piece of work says nothing about any
    // other, so the timetable work is restored from its own evidence exactly as
    // it would be if no declaration existed anywhere in the log. What it must
    // never do is inherit a resource from the work the person actually spoke
    // about.
    let timetable_selection = selection_for(&index, &timetable_work);
    let timetable_members = members(&timetable_work);
    let timetable_worthy: BTreeSet<String> = timetable_selection
        .restore_worthy()
        .iter()
        .map(|selected| selected.artifact_id().to_string())
        .collect();
    assert!(
        !timetable_worthy.is_empty(),
        "the timetable work keeps the continuation its own evidence establishes"
    );
    assert!(
        timetable_worthy.is_subset(&timetable_members),
        "no cross-Workspace leak: {timetable_worthy:?} must all be members of this work"
    );
    for declared in [&contract_file, &contract_terms, &contract_commit] {
        assert!(
            !timetable_worthy.contains(declared.as_str()),
            "a resource declared in the contract work never reaches the timetable work"
        );
    }
    assert_eq!(
        timetable_worthy.len()
            + timetable_selection.unavailable().len()
            + timetable_selection.withheld().len(),
        timetable_members.len(),
        "every member is accounted for exactly once: opened, honestly unavailable, or withheld"
    );
    println!(
        "== teaching timetable: {} open now (its own evidence), {} available if needed, {} history only — nothing from the contract work",
        timetable_selection.restore_worthy().len(),
        timetable_selection.available_if_needed().count(),
        timetable_selection.history_only().count()
    );

    // ── 6. Restart + replay preserve the selection ─────────────────────────
    let restart = CanonicalIndex::new(root.clone()).expect("restart rebuilds");
    assert_eq!(
        selection_for(&restart, workspace_containing(&restart, &contract_file)),
        selection,
        "restart preserves the contract selection"
    );
    assert_eq!(
        selection_for(&restart, workspace_containing(&restart, &timetable_file)),
        timetable_selection,
        "restart preserves the unrelated selection"
    );
    assert_eq!(restart.workspaces(), index.workspaces());
    assert_eq!(restart.subjects(), index.subjects());
    let replayed = replay_workspaces_from_root(&root).expect("workspace replay");
    assert_eq!(
        replayed,
        index.workspaces(),
        "a full replay of the Observation log derives the identical understanding"
    );

    // ── 7. The human correction loop: a later declaration supersedes ────────
    // The person corrects where they continue: the commit drops out, the diff
    // terminal joins. Their latest word is ground truth — Evo neither argues
    // with it nor keeps honouring the statement it replaced.
    drive(&[MacOSSignal::ContinuationSurface {
        subjects: vec![
            CONTRACT_DIFF.into(),
            CONTRACT_TERMS.into(),
            CONTRACT_FILE.into(),
        ],
        observed_at: at(declared_at + HOUR),
    }]);
    let corrected = CanonicalIndex::new(root.clone()).expect("corrected index rebuilds");
    let corrected_work = workspace_containing(&corrected, &contract_file).clone();
    let after = selection_for(&corrected, &corrected_work);
    let worthy_after: BTreeSet<String> = after
        .restore_worthy()
        .iter()
        .map(|selected| selected.artifact_id().to_string())
        .collect();
    assert_eq!(
        worthy_after.len(),
        3,
        "the document, the terms page and the diff terminal: {worthy_after:?}"
    );
    assert!(worthy_after.contains(contract_diff.as_str()));
    assert!(
        after.unavailable().is_empty(),
        "the commit dropped out of the continuation, so nothing is unavailable"
    );
    let historical_after: BTreeSet<String> = after
        .withheld()
        .iter()
        .map(|member| member.artifact_id().to_string())
        .collect();
    assert!(
        historical_after.contains(contract_commit.as_str()),
        "the dropped commit becomes withheld membership, never deleted: {historical_after:?}"
    );
    // Supersession is canonical: the latest valid declaration is the current
    // one, not the union of everything ever declared.
    let corrected_surface = corrected
        .current_continuation_surface()
        .expect("corrected declaration is current");
    assert_eq!(corrected_surface.len(), 3);
    assert_eq!(
        corrected.current_continuation_surface(),
        replay_current_continuation_surface(&root).expect("canonical replay")
    );
    // The correction changed what Evo restores. It changed no membership and no
    // Artifact identity: the same resources still belong to the same work.
    assert_eq!(
        members(&corrected_work),
        members(&contract_work),
        "correcting the continuation does not rewrite what belongs to the work"
    );
    assert_eq!(corrected.subjects(), index.subjects());
    println!("== correction loop: supersession + per-Workspace intersection OK");

    println!("\nVERIFY-SELECTIVE-RESTORATION OK");
    std::fs::remove_dir_all(&root).ok();
}

// ── The witnessed history ──────────────────────────────────────────────────

const HOUR: u64 = 60 * 60;

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

fn save(subject: &str, secs: u64) -> MacOSSignal {
    MacOSSignal::FileSaved {
        subject: subject.to_string(),
        observed_at: at(secs),
    }
}

fn visit(subject: &str, secs: u64) -> MacOSSignal {
    MacOSSignal::URLNavigated {
        subject: subject.to_string(),
        observed_at: at(secs),
    }
}

/// Reviewing a supplier contract: read in an editor, saved, checked against the
/// counterparty's published terms, diffed in a terminal — three sittings, hours
/// apart, ending in one commit.
///
/// The repository-membership facts are emitted before the content Observations
/// that consume them, exactly as the real FSEvents producer emits them.
///
/// Returns the signals and the moment the last sitting ended.
fn contract_review(start: u64) -> (Vec<MacOSSignal>, u64) {
    let mut signals = vec![
        MacOSSignal::RepositoryMembership {
            member: CONTRACT_FILE.into(),
            repository: CONTRACT_REPOSITORY.into(),
            observed_at: at(start),
        },
        MacOSSignal::RepositoryMembership {
            member: CONTRACT_COMMIT.into(),
            repository: CONTRACT_REPOSITORY.into(),
            observed_at: at(start),
        },
    ];
    let mut moment = start;
    for sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus(CONTRACT_EDITOR, moment));
            moment += 300;
            signals.push(save(CONTRACT_FILE, moment));
            moment += 30;
            signals.push(visit(CONTRACT_TERMS, moment));
            moment += 120;
            signals.push(focus(CONTRACT_DIFF, moment));
            moment += 180;
        }
        if sitting == 2 {
            signals.push(MacOSSignal::CommitMade {
                subject: CONTRACT_COMMIT.into(),
                observed_at: at(moment),
            });
        } else {
            moment += 3 * HOUR;
        }
    }
    (signals, moment)
}

/// Building next term's teaching timetable: a spreadsheet, the registry page it
/// is checked against, a console used to validate the roster. Same shape of
/// work, no vocabulary and no container in common with the contract review.
///
/// Returns the signals and the moment the last sitting ended.
fn teaching_timetable(start: u64) -> (Vec<MacOSSignal>, u64) {
    let mut signals = Vec::new();
    let mut moment = start;
    for sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(focus(TIMETABLE_SHEET, moment));
            moment += 300;
            signals.push(save(TIMETABLE_FILE, moment));
            moment += 30;
            signals.push(visit(TIMETABLE_REGISTRY, moment));
            moment += 120;
            signals.push(focus(TIMETABLE_ROSTER, moment));
            moment += 180;
        }
        if sitting < 2 {
            moment += 3 * HOUR;
        }
    }
    (signals, moment)
}

// ── Reading the derived understanding ──────────────────────────────────────

/// Resolves the canonical Artifact a witnessed subject established.
fn artifact_for_subject(index: &CanonicalIndex, subject: &str) -> ArtifactId {
    index
        .subjects()
        .iter()
        .find(|(_, witnessed)| witnessed.as_str() == subject)
        .map(|(id, _)| ArtifactId::new(id.clone()).expect("canonical artifact id"))
        .unwrap_or_else(|| panic!("witnessed subject {subject:?} resolves to a canonical Artifact"))
}

/// Finds the body of work containing one canonical Artifact.
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
        .unwrap_or_else(|| panic!("a body of work contains Artifact {artifact}"))
}

/// The canonical Artifacts belonging to one body of work.
fn members(workspace: &Workspace) -> BTreeSet<String> {
    workspace
        .attachments()
        .iter()
        .map(|attachment| attachment.artifact_id().to_string())
        .collect()
}

/// Computes the derived selective-restoration selection for one body of work
/// from canonical inputs only (derived outcome + membership + resource
/// evidence), exactly as the desktop state helper does.
fn selection_for(index: &CanonicalIndex, workspace: &Workspace) -> RestorationSelection {
    let outcome = index
        .outcomes()
        .get(&workspace.id().to_string())
        .expect("derived restoration outcome");
    let members: Vec<(ArtifactId, ResourceRole)> = workspace
        .attachments()
        .iter()
        .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
        .collect();
    select_restoration(outcome, &members, index.locators())
}
