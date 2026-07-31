//! Candidate Observation.
//!
//! A Candidate Observation represents the transient state of observational data
//! before it has been processed and accepted by Evo. (IS-0001 §5, §9)

use crate::evidence::Evidence;
use crate::observation_schema::ObservationSchema;
use crate::provenance::Provenance;

/// Collects the components required for the Observation acceptance pipeline.
///
/// `CandidateObservation` exists solely to transport an observation attempt into
/// the Observation module. It is the unvalidated, uncanonicalized, and
/// unpersisted precursor to a canonical Observation.
///
/// # Invariants
///
/// - Owns exactly one ObservationSchema, Provenance, and Evidence.
/// - Immutable after construction.
///
/// # Responsibilities
///
/// - Collect the components required for the Observation acceptance pipeline.
/// - Provide read access and ownership transfer of these components to the pipeline.
///
/// # Non-Responsibilities
///
/// - Does **not** validate that the evidence matches the schema.
/// - Does **not** canonicalize representation.
/// - Does **not** compute or verify integrity.
/// - Does **not** persist data.
/// - Does **not** assign an `ObservationId`.
/// - Does **not** infer or interpret meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateObservation {
    observation_schema: ObservationSchema,
    provenance: Provenance,
    evidence: Evidence,
}

impl CandidateObservation {
    /// Constructs a new `CandidateObservation` from its constituent parts.
    ///
    /// Construction cannot fail here because no cross-component validation
    /// (e.g., verifying `Evidence` against `ObservationSchema`) occurs at this
    /// stage. That is the responsibility of the acceptance pipeline.
    pub fn new(
        observation_schema: ObservationSchema,
        provenance: Provenance,
        evidence: Evidence,
    ) -> Self {
        Self {
            observation_schema,
            provenance,
            evidence,
        }
    }

    /// Returns a reference to the Observation Schema.
    pub fn schema(&self) -> &ObservationSchema {
        &self.observation_schema
    }

    /// Returns a reference to the Provenance record.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// Returns a reference to the observed Evidence.
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }

    /// Consumes the candidate and returns its constituent parts.
    ///
    /// This is provided for the acceptance pipeline to take ownership of the
    /// components without cloning when constructing the final canonical Observation.
    pub fn into_parts(self) -> (ObservationSchema, Provenance, Evidence) {
        (self.observation_schema, self.provenance, self.evidence)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Evidence, FactValue, ObservedFact};
    use crate::provenance::ObservationSource;
    use std::collections::HashMap;
    use std::time::SystemTime;

    // Helper to generate valid components for testing
    fn test_candidate_components() -> (ObservationSchema, Provenance, Evidence) {
        let observation_schema = ObservationSchema::new("test_schema", 1).unwrap();

        let source = ObservationSource::new("test_source").unwrap();
        let provenance = Provenance::new(source, SystemTime::now(), HashMap::new());

        let fact = ObservedFact::new("key", FactValue::Boolean(true)).unwrap();
        let evidence = Evidence::new(vec![fact]);

        (observation_schema, provenance, evidence)
    }

    #[test]
    fn candidate_constructs_successfully() {
        let (observation_schema, provenance, evidence) = test_candidate_components(); // Bug 6 fix
        let candidate = CandidateObservation::new(
            observation_schema.clone(),
            provenance.clone(),
            evidence.clone(),
        );

        assert_eq!(candidate.schema(), &observation_schema);
        assert_eq!(candidate.provenance(), &provenance);
        assert_eq!(candidate.evidence(), &evidence);
    }

    #[test]
    fn candidate_into_parts_returns_ownership() {
        let (observation_schema, provenance, evidence) = test_candidate_components(); // Bug 6 fix
        let candidate = CandidateObservation::new(
            observation_schema.clone(),
            provenance.clone(),
            evidence.clone(),
        );

        let (out_schema, out_prov, out_ev) = candidate.into_parts();

        assert_eq!(out_schema, observation_schema);
        assert_eq!(out_prov, provenance);
        assert_eq!(out_ev, evidence);
    }

    #[test]
    fn candidate_clone_produces_equal_value() {
        let (observation_schema, provenance, evidence) = test_candidate_components(); // Bug 6 fix
        let candidate = CandidateObservation::new(observation_schema, provenance, evidence);
        let cloned = candidate.clone();

        assert_eq!(candidate, cloned);
    }
}
