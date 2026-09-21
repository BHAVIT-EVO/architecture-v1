//! Canonical replay: re-deriving understanding from the Observation log alone.
//!
//! Replay is the proof that nothing Evo understands is stored — that losing
//! every derived byte loses no understanding (ARCHITECTURE §2, §6; RFC-0003
//! Req 7 "Replayability"; IS-0011 Replayability: "Replay SHALL reconstruct
//! Workspace understanding exclusively from canonical lower-layer computational
//! objects").
//!
//! # What changed, and why
//!
//! **The old rule.** Replay re-ran the daemon's per-Observation formation
//! sequence: for every content Observation in append order, derive the Artifact
//! identity, run Workspace Formation against the growing Workspace list, upsert
//! by identity. Its correctness claim was "replay applies exactly the formation
//! sequence the live pipeline applies" — two independent implementations of one
//! algorithm, kept in step by hand.
//!
//! **Why it could not work.** Two implementations of a subtle fold drift, and
//! the drift is invisible until it produces a wrong Home. Worse, the algorithm
//! being mirrored was itself the defect: it decided membership one Observation
//! at a time, when whether two resources belong together depends on the whole
//! corpus (see [`crate::understanding`]).
//!
//! **The new rule.** Understanding is a pure projection of the canonical
//! history, so replay does not *mirror* the live derivation — it **is** the live
//! derivation, called on evidence read from disk instead of evidence
//! accumulated in memory. Replay-equivalence stops being a property maintained
//! by two code paths and becomes a property of there being one.
//!
//! What remains here is therefore thin, and deliberately so: read the canonical
//! log, hand it to [`crate::understanding`], return what it derived.

use crate::errors::DaemonError;
use crate::persistence::load_persisted_observations;
use crate::understanding::{self, Understanding};
use evo_engagement::EngagementParams;
use evo_observation::observation::Observation;
use evo_workspace::workspace::Workspace;

use std::path::Path;

/// Replays the complete Workspace understanding — bodies of work and the
/// Restoration outcome derived for each — from one storage root.
///
/// Read-only: canonical state is never modified, and nothing derived is written.
///
/// # Errors
///
/// Returns a [`DaemonError`] when the Observation log cannot be decoded.
pub fn replay_understanding_from_root(root: &Path) -> Result<Understanding, DaemonError> {
    let observations = load_persisted_observations(root)?;
    Ok(replay_understanding_from_observations(&observations))
}

/// Replays the complete Workspace understanding from canonical Observations.
///
/// This is [`understanding::derive`] over [`understanding::evidence_of`] — the
/// same two functions the live index calls. There is no second algorithm to
/// keep in agreement.
pub fn replay_understanding_from_observations(observations: &[Observation]) -> Understanding {
    understanding::derive(
        understanding::evidence_of(observations),
        EngagementParams::default(),
    )
}

/// Replays just the remembered bodies of work from one storage root.
///
/// # Errors
///
/// Returns a [`DaemonError`] when the Observation log cannot be decoded.
pub fn replay_workspaces_from_root(root: &Path) -> Result<Vec<Workspace>, DaemonError> {
    Ok(replay_understanding_from_root(root)?.into_parts().0)
}

/// Replays just the remembered bodies of work from canonical Observations.
pub fn replay_workspaces_from_observations(observations: &[Observation]) -> Vec<Workspace> {
    replay_understanding_from_observations(observations)
        .into_parts()
        .0
}

/// Replays the current Continuation Surface (RFC-0013) from one storage root:
/// the declared subjects of the latest valid OBS-CONTINUATION-SURFACE
/// declaration by canonical Observation Time (later append-order record wins on
/// equal time, mirroring RFC-0011 §5). Returns `None` when no valid declaration
/// exists.
///
/// # Errors
///
/// Returns a [`DaemonError`] when the Observation log cannot be decoded.
pub fn replay_current_continuation_surface(
    root: &Path,
) -> Result<Option<Vec<String>>, DaemonError> {
    let observations = load_persisted_observations(root)?;
    Ok(replay_continuation_surface_from_observations(&observations))
}

/// Re-derives the declared subjects of the current Continuation Surface
/// (RFC-0013) from canonical Observations.
///
/// Supersession is the rule stated in [`crate::understanding`]: the latest valid
/// declaration by canonical Observation Time is current, and on equal time the
/// later append-order record wins.
pub fn replay_continuation_surface_from_observations(
    observations: &[Observation],
) -> Option<Vec<String>> {
    understanding::declared_continuation_surface(observations)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::cache::CanonicalIndex;
    use crate::persistence::persist_observation;
    use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
    use evo_observation::provenance::ObservationSource;
    use evo_storage::Storage;

    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-replay-{label}-{nanos}"))
    }

    fn secs(n: u64) -> SystemTime {
        UNIX_EPOCH
            .checked_add(Duration::from_secs(n))
            .expect("a representable moment")
    }

    /// Drives one canonical signal through the real capture → acceptance →
    /// persistence path, exactly as the daemon does. Nothing derived is
    /// persisted, because nothing derived is canonical.
    fn drive(signal: MacOSSignal) {
        let source = ObservationSource::new("replay-test").expect("non-empty");
        let adapter = MacOSAdapter::new(source);
        let mut engine = CaptureEngine::new();
        let raw = adapter
            .normalize(signal)
            .expect("normalization is infallible")
            .expect("canonical signal");
        let schema = raw.schema();
        let observation = engine.ingest(raw, &schema).expect("observation acceptance");
        persist_observation(&observation).expect("observation persists");
    }

    fn focus(subject: &str, at: u64) -> MacOSSignal {
        MacOSSignal::WindowFocusGained {
            subject: subject.to_string(),
            observed_at: secs(at),
            process_identifier: None,
            owning_process_name: None,
            observed_state: None,
        }
    }

    fn saved(subject: &str, at: u64) -> MacOSSignal {
        MacOSSignal::FileSaved {
            subject: subject.to_string(),
            observed_at: secs(at),
        }
    }

    /// Two bodies of work, each coherent, interleaved in time and sharing no
    /// vocabulary. Returns the last moment used.
    fn drive_two_bodies_of_work() -> u64 {
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                drive(focus("Migration Plan — Editor", moment));
                moment += 200;
                drive(saved("/Users/alice/ops/migration-plan.md", moment));
                moment += 20;
                drive(focus("migration plan rollout — Terminal", moment));
                moment += 200;
            }
            // The other task, in the same sitting but with nothing in common.
            for _pass in 0..4 {
                drive(focus("Recipe Collection — Notes", moment));
                moment += 200;
                drive(saved("/Users/alice/kitchen/recipe-collection.txt", moment));
                moment += 20;
                drive(focus("recipe collection sourdough — Browser", moment));
                moment += 200;
            }
            moment += 4 * 60 * 60;
        }
        moment
    }

    /// Test L. Replaying the log reconstructs the equivalent structure — and
    /// under the projection model "equivalent" is "identical", because the live
    /// index and the replay are the same function of the same evidence.
    #[test]
    fn replay_reconstructs_exactly_what_the_live_index_holds() {
        let root = unique_root("equivalence");
        let _guard = Storage::with_thread_root(root.clone());

        let moment = drive_two_bodies_of_work();
        drive(MacOSSignal::WorkDesignated {
            subject: "/Users/alice/ops/migration-plan.md".into(),
            observed_at: secs(moment + 5),
        });

        let index = CanonicalIndex::new(&root).expect("index builds");
        let replayed = replay_understanding_from_root(&root).expect("replay succeeds");

        assert_eq!(index.workspaces(), replayed.workspaces());
        assert_eq!(index.outcomes(), replayed.outcomes());
        assert_eq!(
            replayed.workspaces().len(),
            2,
            "two unrelated tasks stay two bodies of work"
        );
    }

    /// Replay is deterministic and reads only.
    #[test]
    fn replay_is_deterministic_and_read_only() {
        let root = unique_root("deterministic");
        let _guard = Storage::with_thread_root(root.clone());

        drive_two_bodies_of_work();

        let observation_log = root.join("observation.log");
        let before = std::fs::read(&observation_log).expect("observation log readable");

        let first = replay_workspaces_from_root(&root).expect("first replay");
        let second = replay_workspaces_from_root(&root).expect("second replay");
        assert_eq!(first, second);

        let after = std::fs::read(&observation_log).expect("observation log readable");
        assert_eq!(before, after, "replay is read-only");

        // No derived state was created as a side effect of replaying.
        for derived in ["workspace.log", "restoration.log"] {
            assert!(
                !root.join(derived).exists(),
                "{derived} is not written: understanding is derived, never stored"
            );
        }
    }

    #[test]
    fn replay_of_empty_log_derives_no_workspaces() {
        let root = unique_root("empty");
        let _guard = Storage::with_thread_root(root.clone());
        assert!(
            replay_workspaces_from_root(&root)
                .expect("empty replay succeeds")
                .is_empty()
        );
    }

    /// A shared structural container is *location*, and location alone is not a
    /// task (test G). Two files in one repository, used at different times for
    /// different purposes, must not be folded into one body of work by the fact
    /// that they live in the same place.
    #[test]
    fn shared_location_alone_does_not_merge_two_tasks() {
        let root = unique_root("repository");
        let _guard = Storage::with_thread_root(root.clone());

        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                drive(focus("parser — Editor", moment));
                moment += 200;
                drive(saved("/Users/alice/project/parser.rs", moment));
                moment += 20;
                drive(focus("parser grammar reference — Browser", moment));
                moment += 200;
            }
            moment += 4 * 60 * 60;
            for _pass in 0..4 {
                drive(focus("telemetry — Editor", moment));
                moment += 200;
                drive(saved("/Users/alice/project/telemetry.rs", moment));
                moment += 20;
                drive(focus("telemetry dashboards — Browser", moment));
                moment += 200;
            }
            moment += 4 * 60 * 60;
        }
        // Both files are witnessed members of the same repository.
        for member in [
            "/Users/alice/project/parser.rs",
            "/Users/alice/project/telemetry.rs",
        ] {
            drive(MacOSSignal::RepositoryMembership {
                member: member.into(),
                repository: "/Users/alice/project".into(),
                observed_at: secs(moment),
            });
            moment += 1;
        }

        let replayed = replay_workspaces_from_root(&root).expect("replay succeeds");
        assert_eq!(
            replayed.len(),
            2,
            "two tasks in one repository stay two tasks"
        );
    }

    /// RFC-0013 differential requirement: the live index and a full replay must
    /// derive the identical current Continuation Surface, both as declared
    /// subjects and as resolved Artifacts.
    #[test]
    fn continuation_surface_replay_matches_the_live_index() {
        let root = unique_root("surface");
        let _guard = Storage::with_thread_root(root.clone());

        for (subject, at) in [("/w/a.md", 1), ("/w/b.md", 2), ("/w/c.md", 3)] {
            drive(saved(subject, at));
            drive(saved(subject, at + 10));
        }
        drive(MacOSSignal::ContinuationSurface {
            subjects: vec!["/w/a.md".into(), "/w/b.md".into()],
            observed_at: secs(40),
        });
        drive(MacOSSignal::ContinuationSurface {
            subjects: vec!["/w/b.md".into(), "/w/c.md".into()],
            observed_at: secs(50),
        });

        let index = CanonicalIndex::new(&root).expect("index builds");
        let replayed = replay_current_continuation_surface(&root).expect("replay succeeds");

        assert_eq!(index.current_continuation_surface(), replayed);
        assert_eq!(
            replayed,
            Some(vec!["/w/b.md".to_string(), "/w/c.md".to_string()]),
            "the latest declaration supersedes; canonical ascending order"
        );

        // Determinism.
        assert_eq!(
            replayed,
            replay_current_continuation_surface(&root).expect("second replay")
        );

        // And the resolved Artifact-level surface agrees.
        let observations = crate::persistence::load_persisted_observations(&root)
            .expect("observations load");
        let evidence = understanding::evidence_of(&observations);
        assert_eq!(
            evidence.surface,
            Some(index.current_continuation_surface_artifacts()),
        );
    }
}
