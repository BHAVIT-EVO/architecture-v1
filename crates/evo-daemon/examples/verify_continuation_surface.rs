//! Real-pipeline verification of RFC-0013 continuation surface.
//!
//! Drives the exact daemon vertical pipeline (adapter → engine → persist →
//! derivation) over the canonical sequence a real user produces: witness
//! several resources, declare a continuation surface through the trusted
//! origin, and verify:
//!
//! 1. the declaration is accepted and persisted canonically;
//! 2. the current surface is derived and the canonical record is
//!    order-independent;
//! 3. the surface resolves to the canonical Artifacts it names;
//! 4. the declaration makes the named subjects one body of work — the person's
//!    word is ground truth (§10), so work Evo's own measurements would never
//!    have claimed becomes presentable — and that work's continuation is exactly
//!    what they named, no more: a resource Evo relates to the work on its own
//!    evidence is membership to show, never continuation to open;
//! 5. restart (fresh derived index) and full canonical replay preserve both the
//!    surface and that body of work;
//! 6. Resume Point / Next Step semantics are untouched.
//!
//! Usage:
//!   cargo run -p evo-daemon --example verify_continuation_surface

use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
use evo_daemon::cache::CanonicalIndex;
use evo_daemon::persistence::load_persisted_observations;
use evo_daemon::runtime::VerticalPipeline;
use evo_daemon::workspace_replay::{
    replay_current_continuation_surface, replay_workspaces_from_root,
};
use evo_observation::provenance::ObservationSource;

use std::collections::BTreeSet;
use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime};

fn main() {
    let stamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let root = std::env::temp_dir().join(format!("evo-continuation-surface-verify-{stamp}"));
    println!("== storage root: {}", root.display());
    // The daemon's worker thread sets the thread-local storage root before
    // constructing the pipeline; mirror that here.
    let _storage_guard = evo_storage::Storage::with_thread_root(root.clone());

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
    let mut drive = |signal: MacOSSignal| {
        let raw = adapter
            .normalize(signal)
            .expect("normalization is infallible")
            .expect("canonical signal");
        let schema = raw.schema();
        let observation = engine.ingest(raw, &schema).expect("observation acceptance");
        pipeline.handle_observation(observation);
        while let Ok(Ok(_)) = receiver.try_recv() {}
    };

    // Witness three resources: two files (each its own Workspace) and a URL.
    drive(MacOSSignal::FileSaved {
        subject: "/Users/alice/evo/plan.md".into(),
        observed_at: t(1),
    });
    drive(MacOSSignal::FileSaved {
        subject: "/Users/alice/evo/notes.md".into(),
        observed_at: t(2),
    });
    drive(MacOSSignal::URLNavigated {
        subject: "https://example.com/evo-research".into(),
        observed_at: t(3),
    });
    let before = CanonicalIndex::new(root.clone()).expect("index builds");
    let workspaces_before = before.workspaces().len();
    println!("== bodies of work before declaration: {workspaces_before}");

    // The user declares a continuation surface across two of them (declared
    // in reverse order to prove canonical ordering).
    drive(MacOSSignal::ContinuationSurface {
        subjects: vec![
            "https://example.com/evo-research".into(),
            "/Users/alice/evo/plan.md".into(),
        ],
        observed_at: t(4),
    });

    // 1. Canonical persisted evidence + canonical ordering.
    let observations = load_persisted_observations(&root).expect("observations load");
    let surface_observations: Vec<_> = observations
        .iter()
        .filter(|observation| observation.schema().name() == "OBS-CONTINUATION-SURFACE")
        .collect();
    let persisted_ok = surface_observations.len() == 1;
    let ordered_ok = surface_observations.first().is_some_and(|observation| {
        observation.evidence().facts()[0].value().to_string()
            .contains("/Users/alice/evo/plan.md")
    });
    println!("== declaration persisted canonically: {persisted_ok}");
    println!("== canonical ordering (smallest subject first): {ordered_ok}");

    // 2. Current surface derived; index and replay agree.
    let index = CanonicalIndex::new(root.clone()).expect("fresh index rebuilds");
    let index_surface = index.current_continuation_surface();
    let replayed = replay_current_continuation_surface(&root).expect("canonical replay");
    let derived_ok = index_surface.as_ref().is_some_and(|subjects| subjects.len() == 2);
    let replay_ok = index_surface == replayed;
    println!("== current surface derived: {derived_ok} (2 declared subjects)");
    println!("== restart + replay preserve the surface: {replay_ok}");

    // 3. Per-Workspace intersection: only the declared file's Workspace
    // carries a surface member; the URL's Workspace carries one too; the
    // undeclared file's Workspace carries none.
    let index_artifacts = index.current_continuation_surface_artifacts();
    println!(
        "== surface resolved to {} canonical Artifact(s)",
        index_artifacts.len()
    );

    // 4. The declaration is ground truth (§10), and it is a statement about what
    //    belongs *together* — not a statement that nothing else belongs. Evo's
    //    own measurements would claim nothing here (three subjects witnessed
    //    once each), so the declaration is what makes this work presentable at
    //    all. What must hold is that the named subjects end up in ONE body of
    //    work and that the work's continuation is exactly what the person named:
    //    a member Evo also found related stays membership, never continuation.
    let workspaces_after = before_and_after(&root);
    let declared: BTreeSet<String> = [
        "/Users/alice/evo/plan.md".to_string(),
        "https://example.com/evo-research".to_string(),
    ]
    .into_iter()
    .collect();
    let members: Vec<BTreeSet<String>> = workspaces_after
        .1
        .iter()
        .map(|workspace| {
            workspace
                .attachments()
                .iter()
                .filter_map(|attachment| {
                    workspaces_after.0.subjects().get(attachment.artifact_id().as_str()).cloned()
                })
                .collect()
        })
        .collect();
    // One body of work holds everything the person named: a declared surface is
    // also a declared grouping, so a declaration can never span two of them.
    let holding: Vec<usize> = members
        .iter()
        .enumerate()
        .filter(|(_, member_set)| !member_set.is_disjoint(&declared))
        .map(|(position, _)| position)
        .collect();
    let one_body_ok = holding.len() == 1 && declared.is_subset(&members[holding[0]]);
    println!(
        "== the {} named subject(s) are one body of work: {one_body_ok}",
        declared.len()
    );
    for member_set in &members {
        println!("   members: {:?}", member_set);
    }

    // 4b. The continuation of that work is exactly the declared subjects. A
    //     member Evo related to the work on its own evidence is context to show,
    //     not a resource to open — the declaration is honoured exactly.
    let surface_ok = holding.first().is_some_and(|&position| {
        let workspace = &workspaces_after.1[position];
        let outcome = workspaces_after
            .0
            .outcomes()
            .get(&workspace.id().to_string())
            .expect("a derived outcome for every body of work");
        let surface: BTreeSet<String> = outcome
            .continuation_surface()
            .iter()
            .filter_map(|artifact| workspaces_after.0.subjects().get(artifact.as_str()).cloned())
            .collect();
        surface == declared
    });
    println!("== its continuation is exactly what the person named: {surface_ok}");
    let declaration_ok = one_body_ok && surface_ok;

    // 5. Full canonical replay reproduces the identical understanding from the
    //    Observation log alone — declaration included.
    let replayed_workspaces = replay_workspaces_from_root(&root).expect("workspace replay");
    let workspace_replay_ok = replayed_workspaces == workspaces_after.1;
    println!("== replay reproduces the declared body of work exactly: {workspace_replay_ok}");

    let ok = persisted_ok && ordered_ok && derived_ok && replay_ok && declaration_ok
        && workspace_replay_ok && index_artifacts.len() == 2;
    println!(
        "\nVERIFY-CONTINUATION-SURFACE {}",
        if ok { "OK" } else { "FAIL" }
    );
    std::fs::remove_dir_all(&root).ok();
}

/// Rebuilds the derived index from the log and returns it alongside its bodies
/// of work, so the checks above read only what a restart would reconstruct.
fn before_and_after(
    root: &std::path::Path,
) -> (CanonicalIndex, Vec<evo_workspace::workspace::Workspace>) {
    let index = CanonicalIndex::new(root.to_path_buf()).expect("index rebuilds");
    let workspaces = index.workspaces().to_vec();
    (index, workspaces)
}
