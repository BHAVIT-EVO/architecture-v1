//! Replay Corpus.
//!
//! A [`ReplayCorpus`] is the ordered canonical Observation history together
//! with the canonical Artifact history derived from it (IS-0013 §3).
//!
//! # IS-0013 Invariants
//!
//! - WR-1: Replay consumes only canonical computational primitives.
//! - WR-2: Replay preserves canonical Observation order.
//! - WR-3: Replay never modifies Observation history.
//! - WR-4: Replay never modifies Artifact Identity.
//!
//! The corpus is immutable after construction. Observations are stored in
//! the order they are supplied and are presented to Formation in that order
//! without reordering, skipping, or duplication (IS-0013 §5).

use evo_artifact::artifact::Artifact;
use evo_observation::observation::Observation;

// ── ReplayCorpus ──────────────────────────────────────────────────────────────

/// The ordered canonical Observation history together with the canonical
/// Artifact history derived from it.
///
/// The caller is responsible for ensuring that:
///
/// - Observations are supplied in canonical order (IS-0013 §5).
/// - Every Artifact referenced by an Observation is present in the Artifact
///   history.
/// - No Observation is duplicated.
/// - No Observation is omitted.
///
/// [`ReplayCorpus`] performs no reordering, deduplication, or validation
/// beyond confirming that neither slice is empty at construction time, which
/// would make Replay trivially vacuous.
///
/// # Invariants
///
/// - Observation order is the order supplied at construction.
/// - Immutable after construction (WR-1, WR-3, WR-4).
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayCorpus {
    /// Canonical Observation history in canonical order.
    observations: Vec<Observation>,
    /// Canonical Artifact history derived from the Observation history.
    artifacts: Vec<Artifact>,
}

impl ReplayCorpus {
    /// Constructs a [`ReplayCorpus`] from an ordered canonical Observation
    /// history and the canonical Artifact history derived from it.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::EmptyObservations`] if `observations` is empty.
    /// Returns [`CorpusError::EmptyArtifacts`] if `artifacts` is empty.
    ///
    /// An empty corpus cannot represent a complete Replay Execution as defined
    /// by IS-0013 §10: Replay SHALL complete only after every canonical
    /// Observation has been processed, and Partial Replay SHALL NOT constitute
    /// canonical Workspace understanding.
    pub fn new(
        observations: Vec<Observation>,
        artifacts: Vec<Artifact>,
    ) -> Result<Self, CorpusError> {
        if observations.is_empty() {
            return Err(CorpusError::EmptyObservations);
        }
        if artifacts.is_empty() {
            return Err(CorpusError::EmptyArtifacts);
        }
        Ok(Self {
            observations,
            artifacts,
        })
    }

    /// Returns the canonical Observation history in canonical order.
    ///
    /// The order of the returned slice is the canonical Observation order
    /// that Replay will present to Formation (IS-0013 §5).
    pub fn observations(&self) -> &[Observation] {
        &self.observations
    }

    /// Returns the canonical Artifact history.
    pub fn artifacts(&self) -> &[Artifact] {
        &self.artifacts
    }
}

// ── CorpusError ───────────────────────────────────────────────────────────────

/// Errors that can occur when constructing a [`ReplayCorpus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorpusError {
    /// The Observation history was empty.
    ///
    /// A Replay over an empty Observation history cannot satisfy IS-0013 §10:
    /// Replay SHALL complete only after every canonical Observation has been
    /// processed.
    EmptyObservations,

    /// The Artifact history was empty.
    ///
    /// Workspace Formation requires at least one canonical Artifact to
    /// evaluate (IS-0012, IS-0013 §4).
    EmptyArtifacts,
}

impl std::fmt::Display for CorpusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorpusError::EmptyObservations => {
                write!(f, "replay corpus must contain at least one canonical Observation")
            }
            CorpusError::EmptyArtifacts => {
                write!(f, "replay corpus must contain at least one canonical Artifact")
            }
        }
    }
}

impl std::error::Error for CorpusError {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use evo_artifact::artifact_id::ArtifactId;
    use evo_observation::evidence::{Evidence, FactValue, ObservedFact};
    use evo_observation::observation_id::ObservationId;
    use evo_observation::observation_schema::ObservationSchema;
    use evo_observation::provenance::{ObservationSource, Provenance};

    use std::collections::HashMap;
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

    #[test]
    fn corpus_constructs_successfully_with_valid_inputs() {
        let observations = vec![make_observation()];
        let artifacts = vec![make_artifact()];
        assert!(ReplayCorpus::new(observations, artifacts).is_ok());
    }

    #[test]
    fn corpus_rejects_empty_observations() {
        let result = ReplayCorpus::new(vec![], vec![make_artifact()]);
        assert!(matches!(
            result,
            Err(CorpusError::EmptyObservations)
       ));
    }

    #[test]
    fn corpus_rejects_empty_artifacts() {
        let result = ReplayCorpus::new(vec![make_observation()], vec![]);
        assert!(matches!(
            result,
            Err(CorpusError::EmptyArtifacts)
        ));}

    #[test]
    fn corpus_observations_returns_observations_in_supplied_order() {
        let o1 = make_observation();
        let o2 = make_observation();
        let corpus = ReplayCorpus::new(
            vec![o1.clone(), o2.clone()],
            vec![make_artifact()],
        )
        .unwrap();
        assert_eq!(corpus.observations()[0], o1);
        assert_eq!(corpus.observations()[1], o2);
    }

    #[test]
    fn corpus_artifacts_returns_all_artifacts() {
        let a1 = make_artifact();
        let corpus =
            ReplayCorpus::new(vec![make_observation()], vec![a1.clone()]).unwrap();
        assert_eq!(corpus.artifacts().len(), 1);
        assert_eq!(corpus.artifacts()[0], a1);
    }

    #[test]
    fn corpus_error_empty_observations_displays_message() {
        let msg = CorpusError::EmptyObservations.to_string();
        assert!(!msg.is_empty());
        assert!(msg.contains("Observation"));
    }

    #[test]
    fn corpus_error_empty_artifacts_displays_message() {
        let msg = CorpusError::EmptyArtifacts.to_string();
        assert!(!msg.is_empty());
        assert!(msg.contains("Artifact"));
    }

    #[test]
    fn corpus_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&CorpusError::EmptyObservations);
        takes_error(&CorpusError::EmptyArtifacts);
    }

    #[test]
    fn corpus_clone_produces_equal_length_slices() {
        let corpus =
            ReplayCorpus::new(vec![make_observation()], vec![make_artifact()]).unwrap();
        let cloned = corpus.clone();
        assert_eq!(corpus.observations().len(), cloned.observations().len());
        assert_eq!(corpus.artifacts().len(), cloned.artifacts().len());
    }
}
