//! Orchestration of the Observation Acceptance Pipeline.
//!
//! This module coordinates the transformation of a transient [`CandidateObservation`]
//! into a permanent, immutable [`Observation`]. It acts strictly as an orchestrator,
//! containing no business rules, validation logic, or canonicalization logic of its own.

use crate::candidate::CandidateObservation;
use crate::canonicalization::canonicalize;
use crate::errors::ObservationError;
use crate::integrity::verify_integrity;
use crate::observation::Observation;
use crate::observation_id::ObservationId;
use crate::observation_schema::ObservationSchema;
use crate::validation::validate;

/// Accepts a candidate observation through the standard pipeline.
///
/// Coordinates the sequential execution of validation, canonicalization,
/// deconstruction, identity assignment, and final entity assembly.
///
/// # Arguments
///
/// * `candidate` - The unvalidated, transient observation data.
/// * `registered_schema` - The authorized schema to validate the candidate against.
///
/// # Returns
///
/// A fully materialized, immutable `Observation` upon success, or an aggregate
/// `ObservationError` if any stage of the pipeline fails.
pub fn accept(
    candidate: CandidateObservation,
    registered_schema: &ObservationSchema,
) -> Result<Observation, ObservationError> {
    // 1. Validation
    // Borrow the candidate to validate it against the registered schema.
    // The `?` operator maps `ValidationError` into `ObservationError`.
    validate(&candidate, registered_schema)?;

    // 2. Canonicalization
    // Move ownership of the candidate into the canonicalization stage.
    // The `?` operator maps `CanonicalizationError` into `ObservationError`.
    let canonical_candidate = canonicalize(candidate)?;

    // 3. Deconstruction
    // Extract the successfully processed components.
    let (schema, provenance, evidence) = canonical_candidate.into_parts();

    // 4. Identity Assignment
    // // Assign an ObservationId.
    let id = ObservationId::new();

    // 5. Final Construction
    // Assemble the permanent, immutable entity.
    let observation = Observation::new(id, schema, provenance, evidence);

    verify_integrity(&observation)?;

    Ok(observation)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ValidationError;
    use crate::evidence::{Evidence, FactValue, ObservedFact};
    use crate::provenance::{ObservationSource, Provenance};
    use std::collections::HashMap;
    use std::time::SystemTime;

    // Helper to generate valid components for testing
    fn create_test_components(schema_name: &str) -> (ObservationSchema, Provenance, Evidence) {
        let schema = ObservationSchema::new(schema_name, 1).unwrap();

        let source = ObservationSource::new("test_source").unwrap();
        let provenance = Provenance::new(source, SystemTime::now(), HashMap::new());

        let fact = ObservedFact::new("key", FactValue::Boolean(true)).unwrap();
        let evidence = Evidence::new(vec![fact]);

        (schema, provenance, evidence)
    }

    #[test]
    fn accept_pipeline_succeeds_for_valid_candidate() {
        // Arrange
        let (schema, provenance, evidence) = create_test_components("test_schema");
        let candidate =
            CandidateObservation::new(schema.clone(), provenance.clone(), evidence.clone());

        // Act
        let result = accept(candidate, &schema);

        // Assert
        assert!(result.is_ok());
        let observation = result.unwrap();

        // Verify ID generation (accessible)
        let _id = observation.id();

        // Verify all components were preserved flawlessly
        assert_eq!(observation.schema(), &schema);
        assert_eq!(observation.provenance(), &provenance);
        assert_eq!(observation.evidence(), &evidence);
    }

    #[test]
    fn accept_pipeline_fails_validation_for_mismatched_schema() {
        // Arrange
        let (candidate_schema, provenance, evidence) = create_test_components("candidate_schema");
        let candidate = CandidateObservation::new(candidate_schema.clone(), provenance, evidence);

        let registered_schema = ObservationSchema::new("registered_schema", 1).unwrap();

        // Act
        let result = accept(candidate, &registered_schema);

        // Assert
        assert!(result.is_err());

        // Ensure the error propagated correctly through the ObservationError aggregate
        match result.unwrap_err() {
            ObservationError::Validation(ValidationError::UnknownSchema(err_schema)) => {
                assert_eq!(err_schema, candidate_schema);
            }
            _ => panic!("Expected ObservationError::Validation(UnknownSchema)"),
        }
    }
}
