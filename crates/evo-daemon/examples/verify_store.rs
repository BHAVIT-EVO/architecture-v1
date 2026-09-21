//! Read-only verification that a canonical storage root loads correctly
//! through the real persistence boundary. Never writes to the store.
//!
//! Workspaces are no longer read from a log. Under the current model they are a
//! projection of the canonical Observation history, so the only thing this
//! example can verify about them is that the projection runs over whatever is
//! actually in the store and reports what it derived.

use evo_daemon::persistence::{load_artifact_subjects, load_persisted_observations};
use evo_daemon::workspace_replay::replay_workspaces_from_observations;

fn main() {
    let root = evo_storage::canonical_storage_root();

    let observations = load_persisted_observations(&root).expect("observations load");
    let subjects = load_artifact_subjects(&root).expect("subjects load");
    let workspaces = replay_workspaces_from_observations(&observations);

    println!(
        "observations={} derived_workspaces={} artifact_subjects={}",
        observations.len(),
        workspaces.len(),
        subjects.len()
    );
    for workspace in &workspaces {
        println!(
            "workspace={} snapshots={} artifacts={}",
            workspace.id(),
            workspace.snapshots().len(),
            workspace.attachments().len()
        );
    }
    for (artifact, subject) in &subjects {
        println!("artifact={} subject={}", artifact, subject);
    }
}
