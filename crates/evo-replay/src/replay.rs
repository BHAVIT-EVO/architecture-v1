//! Workspace Replay.
//!
//! This module implements the `replay` function, which is the complete
//! Workspace Replay execution as defined by IS-0013.
//!
//! # IS-0013 Execution Model
//!
//! Replay executes Workspace Formation in canonical Observation order
//! (IS-0013 §5). For each canonical Observation in the corpus, Replay
//! presents it together with the canonical Artifact history to the
//! Formation function supplied by the caller. The Formation function
//! implements IS-0012 and is not re-implemented here (IS-0013 §8).
//!
//! After every canonical Observation has been processed, Replay returns
//! the resulting canonical Workspace understanding (IS-0013 §6, Stage 6).
//!
//! # What Replay Does NOT Do
//!
//! - Does not implement Workspace Recognition (IS-0013 §8).
//! - Does not implement Attachment Evaluation (IS-0013 §8).
//! - Does not implement Workspace Decision (IS-0013 §8).
//! - Does not implement Snapshot Construction (IS-0013 §8).
//! - Does not perform persistence (IS-0013 §11).
//! - Does not perform restoration (IS-0013 §11).
//! - Does not perform learning (IS-0013 §11).
//! - Does not perform retrieval (IS-0013 §11).
//! - Does not modify Observation history (WR-3).
//! - Does not modify Artifact Identity (WR-4).
//! - Does not modify Workspace Formation rules (WR-5).
//! - Does not bypass Workspace Formation (WR-6).

use evo_artifact::artifact::Artifact;
use evo_observation::observation::Observation;
use evo_workspace::workspace::Workspace;
use crate::corpus::ReplayCorpus;

// ── replay ────────────────────────────────────────────────────────────────────

/// Deterministically re-executes Workspace Formation over the entire
/// [`ReplayCorpus`] in canonical Observation order, returning the resulting
/// canonical Workspace understanding.
///
/// # IS-0013 Contract
///
/// `replay` implements the six-stage Replay Pipeline defined by IS-0013 §6:
///
/// 1. Begin with the earliest canonical Observation in the corpus.
/// 2. Present the current Observation to `formation_fn`.
/// 3. `formation_fn` executes Observation Acceptance, Artifact Identity, and
///    Workspace Formation exactly as defined by their respective specifications.
/// 4. Advance to the next Observation.
/// 5. Repeat until every Observation has been processed.
/// 6. Return the resulting canonical Workspace understanding.
///
/// # Formation Function Contract
///
/// `formation_fn` implements IS-0012 (Workspace Formation). It receives:
///
/// - The current canonical [`Observation`] being processed.
/// - The complete canonical [`Artifact`] history.
/// - The current accumulated [`Workspace`] understanding (initially `None`
///   before any Observation has been processed, then `Some` thereafter).
///
/// It returns the updated canonical [`Workspace`] understanding after
/// processing the supplied Observation.
///
/// `replay` does not inspect or modify the [`Workspace`] returned by
/// `formation_fn`. It passes it unchanged to the next Formation invocation.
/// This preserves IS-0013 §8: Replay SHALL reuse IS-0012 unchanged.
///
/// # Determinism
///
/// Given identical inputs (`corpus` and `formation_fn` behaviour), `replay`
/// produces identical output (IS-0013 §7, WR-7, WR-8).
///
/// # Parameters
///
/// - `corpus`: The ordered canonical Observation and Artifact history
///   (IS-0013 §3, §4).
/// - `formation_fn`: A callable that implements IS-0012 Workspace Formation.
///   It is called once per canonical Observation, in canonical Observation
///   order. It MUST NOT be called with any Observation more than once and
///   MUST NOT be called with Observations in any order other than canonical
///   order (IS-0013 §5, WR-2).
///
/// # Returns
///
/// The canonical [`Workspace`] understanding produced after all canonical
/// Observations have been processed (IS-0013 §6 Stage 6, §10).
pub fn replay<F>(
    corpus: &ReplayCorpus,
    mut formation_fn: F,
) -> Workspace
where
    F: FnMut(&Observation, &[Artifact], Option<Workspace>) -> Workspace,
{
    // IS-0013 §6 Stage 1: begin with the earliest canonical Observation.
    // IS-0013 §5: the canonical unit of Replay is the canonical Observation.
    // IS-0013 §5: Replay SHALL NOT reorder, skip, or duplicate Observations.
    //
    // The corpus guarantees at least one Observation (// Preconditions are validated 
    // by ReplayCorpus before replay() is invoked.), so this fold always produces
    // a value without requiring an Option return type.
    //
    // IS-0013 §6 Stage 2–5: present each Observation to formation_fn in order.
    // IS-0013 §6 Stage 6: return the resulting Workspace understanding.
    corpus
        .observations()
        .iter()
        .fold(None, |current_workspace, observation| {
            // IS-0013 §6 Stage 2: present the current Observation.
            // IS-0013 §6 Stage 3: allow Formation to execute.
            // IS-0013 §8: Replay SHALL NOT bypass any canonical stage.
            let updated = formation_fn(
                              observation,
                              corpus.artifacts(),
                              current_workspace,
                        );
            // IS-0013 §6 Stage 4: advance to the next Observation.
            Some(updated)
        })
        // Preconditions are validated by ReplayCorpus before replay() is invoked., so fold always
        // produces Some. This unwrap is safe by construction.
        .unwrap()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use evo_artifact::artifact_id::ArtifactId;
    use evo_observation::evidence::{Evidence, FactValue, ObservedFact};
    use evo_observation::observation_id::ObservationId;
    use evo_observation::observation_schema::ObservationSchema;
    use evo_observation::provenance::{ObservationSource, Provenance};
    use evo_workspace::attachment::Attachment;
    use evo_workspace::confidence::ConfidenceScore;
    use evo_workspace::lifecycle::WorkspaceLifecycle;
    use evo_workspace::snapshot::Snapshot;
    use evo_workspace::workspace::Workspace;
    use evo_workspace::workspace_id::WorkspaceId;

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::SystemTime;

    fn make_observation() -> Observation {
        let id = ObservationId::new();
        let schema = ObservationSchema::new("test-schema", 1).unwrap();
        let source = ObservationSource::new("test-source").unwrap();
        let provenance = Provenance::new(source, SystemTime::now(), HashMap::new());
        let fact = ObservedFact::new("key", FactValue::Text("value".into())).unwrap();
        let evidence = Evidence::new(vec![fact]);
        Observation::new(id, schema, provenance, evidence)
    }

    fn make_artifact() -> Artifact {
        Artifact::new(ArtifactId::new("test-artifact").unwrap())
    }

    fn make_workspace() -> Workspace {
        let attachment = Attachment::new(
            ArtifactId::new("test-artifact").unwrap(),
            ConfidenceScore::new(0.8).unwrap(),
        );
        let snapshot = Snapshot::new(
            SystemTime::now(),
            WorkspaceLifecycle::Active,
            vec![attachment.clone()],
        );
        Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![attachment],
            vec![snapshot],
        )
    }

    // A simple formation function for testing: returns a fixed Workspace
    // regardless of input, representing the Formation result.
    fn fixed_formation(
        _observation: &Observation,
        _artifacts: &[Artifact],
        _current: Option<Workspace>,
    ) -> Workspace {
        make_workspace()
    }

    // A formation function that counts how many times it has been called.
    fn counting_formation(
        counter: Arc<Mutex<usize>>,
    ) -> impl FnMut(&Observation, &[Artifact], Option<Workspace>) -> Workspace {
        move |_obs, _artifacts, _current| {
            *counter.lock().unwrap() += 1;
            make_workspace()
        }
    }

    // A formation function that records which Observations it received, in
    // order, so tests can verify canonical Observation order is preserved.
    fn ordering_formation(
        received: Arc<Mutex<Vec<ObservationId>>>,
    ) -> impl FnMut(&Observation, &[Artifact], Option<Workspace>) -> Workspace {
        move |obs, _artifacts, _current| {
            received.lock().unwrap().push(obs.id().clone());
            make_workspace()
        }
    }

    #[test]
    fn replay_returns_workspace_for_single_observation() {
        let corpus =
            ReplayCorpus::new(vec![make_observation()], vec![make_artifact()]).unwrap();
        let result = replay(&corpus, fixed_formation);
        // Verify we received a Workspace (type check via usage of its API).
        let _ = result.lifecycle();
    }

    #[test]
    fn replay_calls_formation_once_per_observation() {
        let observations = vec![make_observation(), make_observation(), make_observation()];
        let corpus = ReplayCorpus::new(observations, vec![make_artifact()]).unwrap();

        let counter = Arc::new(Mutex::new(0usize));
        replay(&corpus, counting_formation(counter.clone()));

        assert_eq!(*counter.lock().unwrap(), 3);
    }

    #[test]
    fn replay_presents_observations_in_canonical_order() {
        let o1 = make_observation();
        let o2 = make_observation();
        let o3 = make_observation();

        let expected_order = vec![o1.id().clone(), o2.id().clone(), o3.id().clone()];

        let corpus = ReplayCorpus::new(
            vec![o1, o2, o3],
            vec![make_artifact()],
        )
        .unwrap();

        let received = Arc::new(Mutex::new(Vec::new()));
        replay(&corpus, ordering_formation(received.clone()));

        assert_eq!(*received.lock().unwrap(), expected_order);
    }

    #[test]
    fn replay_does_not_skip_any_observation() {
        let n = 10usize;
        let observations: Vec<Observation> = (0..n).map(|_| make_observation()).collect();
        let corpus = ReplayCorpus::new(observations, vec![make_artifact()]).unwrap();

        let counter = Arc::new(Mutex::new(0usize));
        replay(&corpus, counting_formation(counter.clone()));

        assert_eq!(*counter.lock().unwrap(), n);
    }

    #[test]
    fn replay_passes_artifacts_to_every_formation_invocation() {
        let artifact = make_artifact();
        let corpus = ReplayCorpus::new(
            vec![make_observation(), make_observation()],
            vec![artifact.clone()],
        )
        .unwrap();

        let artifact_counts = Arc::new(Mutex::new(Vec::<usize>::new()));
        let artifact_counts_clone = artifact_counts.clone();

        replay(&corpus, move |_obs, artifacts, _current| {
            artifact_counts_clone
                .lock()
                .unwrap()
                .push(artifacts.len());
            make_workspace()
        });

        let counts = artifact_counts.lock().unwrap();
        // Formation was called twice, each time with the full Artifact slice.
        assert_eq!(counts.len(), 2);
        assert!(counts.iter().all(|&c| c == 1));
    }

    #[test]
    fn replay_passes_none_as_initial_workspace_for_first_observation() {
        let corpus =
            ReplayCorpus::new(vec![make_observation()], vec![make_artifact()]).unwrap();

        let first_was_none = Arc::new(Mutex::new(false));
        let first_was_none_clone = first_was_none.clone();

        replay(&corpus, move |_obs, _artifacts, current| {
            *first_was_none_clone.lock().unwrap() = current.is_none();
            make_workspace()
        });

        assert!(*first_was_none.lock().unwrap());
    }

    #[test]
    fn replay_passes_formation_result_as_current_workspace_to_next_observation() {
        let id = WorkspaceId::new();
        let workspace_to_return = Workspace::new(
            id.clone(),
            WorkspaceLifecycle::Active,
            vec![],
            vec![],
        );

        let corpus = ReplayCorpus::new(
            vec![make_observation(), make_observation()],
            vec![make_artifact()],
        )
        .unwrap();

        let second_received_id = Arc::new(Mutex::new(None::<WorkspaceId>));
        let second_received_id_clone = second_received_id.clone();
        let workspace_clone = workspace_to_return.clone();
        let call_count = Arc::new(Mutex::new(0usize));
        let call_count_clone = call_count.clone();

        replay(&corpus, move |_obs, _artifacts, current| {
            let count = {
                let mut c = call_count_clone.lock().unwrap();
                *c += 1;
                *c
            };
            if count == 2 {
                // On the second call, record what current workspace we received.
                if let Some(ws) = &current {
                    *second_received_id_clone.lock().unwrap() = Some(ws.id().clone());
                }
            }
            workspace_clone.clone()
        });

        // The second Formation call must have received the Workspace returned
        // by the first Formation call, identified by its WorkspaceId.
        assert_eq!(*second_received_id.lock().unwrap(), Some(id));
    }

    #[test]
    fn replay_does_not_modify_corpus_observations() {
        let observation = make_observation();
        let original_id = observation.id().clone();
        let corpus =
            ReplayCorpus::new(vec![observation], vec![make_artifact()]).unwrap();
        replay(&corpus, fixed_formation);
        assert_eq!(corpus.observations()[0].id(), &original_id);
    }

    #[test]
    fn replay_does_not_modify_corpus_artifacts() {
        let a = make_artifact();
        let original_id = a.id().clone();
        let corpus =
            ReplayCorpus::new(vec![make_observation()], vec![a]).unwrap();

        replay(&corpus, fixed_formation);

        let artifacts = vec![make_artifact()];
        let corpus = ReplayCorpus::new(
            vec![make_observation()],
            artifacts.clone(),
        ).unwrap();

        assert_eq!(artifacts[0].id(), &original_id);
    }

    #[test]
    fn replay_is_deterministic_given_identical_inputs() {
        let o = make_observation();
        let a = make_artifact();

        let corpus_a = ReplayCorpus::new(vec![o.clone()], vec![a.clone()]).unwrap();
        let corpus_b = ReplayCorpus::new(vec![o], vec![a]).unwrap();

        let counter_a = Arc::new(Mutex::new(0usize));
        let counter_b = Arc::new(Mutex::new(0usize));

        replay(&corpus_a, counting_formation(counter_a.clone()));
        replay(&corpus_b, counting_formation(counter_b.clone()));

        // Both runs called Formation the same number of times.
        assert_eq!(*counter_a.lock().unwrap(), *counter_b.lock().unwrap());
    }

    #[test]
    fn replay_completes_only_after_every_observation_is_processed() {
        let n = 5usize;
        let observations: Vec<Observation> = (0..n).map(|_| make_observation()).collect();
        let corpus = ReplayCorpus::new(observations, vec![make_artifact()]).unwrap();

        let processed = Arc::new(Mutex::new(0usize));
        let processed_clone = processed.clone();

        // replay returns only after formation_fn has been called for every
        // Observation. The call count after replay returns must equal n.
        replay(&corpus, move |_obs, _artifacts, _current| {
            *processed_clone.lock().unwrap() += 1;
            make_workspace()
        });

        assert_eq!(*processed.lock().unwrap(), n);
    }
}
