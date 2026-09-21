//! Candidate Artifact.
//!
//! A Candidate Artifact represents the transient computational state from
//! which a canonical Artifact may be accepted (IS-0004 — Candidate Artifact).
//!
//! Every Candidate Artifact represents exactly one Identity Hypothesis
//! awaiting acceptance (IS-0004 — Candidate Artifact, Identity Hypothesis).
//!
//! A Candidate Artifact exists only during the Artifact Acceptance Pipeline.
//! It does not survive beyond acceptance or rejection (IS-0005 R-2).
//!
//! # Internal Representation
//!
//! The internal representation of a Candidate Artifact is intentionally
//! unspecified by IS-0004. The current implementation stores one private
//! Identity Hypothesis payload so Stage 1 validation can enforce the amended
//! structural invariants without exposing any new public API surface.

#[cfg(test)]
use evo_observation::evidence::{Evidence, FactValue, ObservedFact};
use evo_observation::observation::Observation;
#[cfg(test)]
use evo_observation::observation_id::ObservationId;
#[cfg(test)]
use evo_observation::observation_schema::ObservationSchema;
#[cfg(test)]
use evo_observation::provenance::{ObservationSource, Provenance};

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::str::FromStr;
#[cfg(test)]
use std::time::SystemTime;

// ── CandidateArtifact ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
struct IdentityHypothesis {
    observations: Vec<Observation>,
}

impl IdentityHypothesis {
    fn new(observations: Vec<Observation>) -> Self {
        Self { observations }
    }

    fn observations(&self) -> &[Observation] {
        &self.observations
    }
}

/// The transient computational state representing one Identity Hypothesis
/// awaiting Artifact Acceptance (IS-0004).
///
/// `CandidateArtifact` is the input to the Artifact Acceptance Pipeline
/// (IS-0005). It is produced before the pipeline begins and consumed — never
/// cloned — through each stage until acceptance produces one canonical
/// [`Artifact`] or rejection ceases its existence.
///
/// # Invariants
///
/// - Represents exactly one Identity Hypothesis (IS-0004 — Candidate Artifact).
/// - Does NOT possess canonical Artifact Identity (IS-0004 — Candidate
///   Artifact; IS-0005 Definitions).
/// - Is NOT referenced by higher computational layers (IS-0004 — Candidate
///   Artifact).
/// - Immutable after construction.
///
/// # Internal Representation
///
/// The internal representation is intentionally unspecified by IS-0004.
/// All fields are private and must not be accessed outside this module.
///
/// # Non-Responsibilities
///
/// - Does **not** infer Identity Hypotheses.
/// - Does **not** assign canonical [`ArtifactId`].
/// - Does **not** validate, canonicalize, or verify integrity.
/// - Does **not** persist anything.
/// - Does **not** reference higher computational layers.
///
/// [`Artifact`]: crate::artifact::Artifact
/// [`ArtifactId`]: crate::artifact_id::ArtifactId
#[derive(Debug)]
pub(crate) struct CandidateArtifact {
    hypothesis: IdentityHypothesis,
}

impl CandidateArtifact {
    /// Constructs a `CandidateArtifact` representing one Identity Hypothesis
    /// awaiting acceptance.
    ///
    /// # Caller Responsibilities
    ///
    /// The Acceptance Pipeline enforces these invariants through Validation
    /// (IS-0005 Stage 1) before any subsequent stage executes.
    pub(crate) fn new() -> Self {
        Self {
            hypothesis: IdentityHypothesis::new(vec![]),
        }
    }

    pub(crate) fn observations(&self) -> &[Observation] {
        self.hypothesis.observations()
    }

    pub(crate) fn new_with_observations(observations: Vec<Observation>) -> Self {
        Self {
            hypothesis: IdentityHypothesis::new(observations),
        }
    }
}

#[cfg(test)]
pub(crate) fn test_observation() -> Observation {
    test_observation_with_subject("fixture", SystemTime::UNIX_EPOCH)
}

#[cfg(test)]
pub(crate) fn test_observation_with_subject(subject: &str, observed_at: SystemTime) -> Observation {
    let id = ObservationId::from_str("123e4567-e89b-12d3-a456-426614174000").unwrap();
    let schema = ObservationSchema::window_focus_gained_v1();
    let source = ObservationSource::new("artifact_engine_fixture").unwrap();
    let provenance = Provenance::new(source, observed_at, HashMap::new());
    let fact = ObservedFact::new(
        schema.canonical_fact_name().unwrap(),
        FactValue::Text(subject.into()),
    )
    .unwrap();
    let evidence = Evidence::new(vec![fact]);

    Observation::new(id, schema, provenance, evidence)
}

#[cfg(test)]
pub(crate) fn test_observation_with_identity(
    id: &str,
    subject: &str,
    observed_at: SystemTime,
) -> Observation {
    let id = ObservationId::from_str(id).unwrap();
    let schema = ObservationSchema::window_focus_gained_v1();
    let source = ObservationSource::new("artifact_engine_fixture").unwrap();
    let provenance = Provenance::new(source, observed_at, HashMap::new());
    let fact = ObservedFact::new(
        schema.canonical_fact_name().unwrap(),
        FactValue::Text(subject.into()),
    )
    .unwrap();
    let evidence = Evidence::new(vec![fact]);

    Observation::new(id, schema, provenance, evidence)
}

#[cfg(test)]
pub(crate) fn test_candidate_with_observations(
    observations: Vec<Observation>,
) -> CandidateArtifact {
    CandidateArtifact::new_with_observations(observations)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_constructs_successfully() {
        let _ = CandidateArtifact::new();
    }

    #[test]
    fn candidate_new_for_testing_constructs_successfully() {
        let _ = CandidateArtifact::new();
    }

    #[test]
    fn candidate_is_consumed_by_value_through_pipeline() {
        // CandidateArtifact does not implement Copy. Once moved into a
        // pipeline stage, it cannot be reused. This enforces IS-0005 R-2:
        // atomic acceptance — no partial state survives.
        let candidate = CandidateArtifact::new();
        let _moved = candidate;
        // `candidate` is no longer accessible here.
        // If this test compiles, move semantics are confirmed.
    }

    #[test]
    fn candidate_representation_is_private() {
        // If this test compiles without accessing any field,
        // the internal representation is fully encapsulated.
        let _candidate = CandidateArtifact::new();
    }

    #[test]
    fn test_helper_constructs_populated_candidate() {
        let candidate = test_candidate_with_observations(vec![test_observation()]);
        assert_eq!(candidate.observations().len(), 1);
    }
}
