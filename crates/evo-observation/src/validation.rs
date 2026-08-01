//! Validation of Candidate Observations.
//!
//! Determines whether a [`CandidateObservation`] satisfies the architectural
//! requirements to enter the Observation acceptance pipeline.
//! (IS-0001 §3 R-2; IS-0001 §10)
//!
//! Validation is **pure** and **deterministic**:
//! - it produces no side effects,
//! - it never mutates the candidate,
//! - it never persists, infers, or interprets,
//! - identical inputs always produce identical outputs.

use crate::candidate::CandidateObservation;
use crate::errors::ValidationError;
use crate::observation_schema::ObservationSchema;
// ── Public entry point ────────────────────────────────────────────────────────

/// Validates a [`CandidateObservation`] against the provided registry of known
/// schemas.
///
/// Runs three checks in sequence, returning the **first** failure encountered.
/// This matches IS-0001 §3 R-8: the result is exactly one of `Ok(())` or a
/// [`ValidationError`] — no partial acceptance.
///
/// | Order | Check | IS-0001 §10 failure mode |
/// |-------|-------|--------------------------|
/// | 1 | Schema existence | `Unknown Observation Schema` |
/// | 2 | Provenance completeness | `Missing required provenance` |
/// | 3 | Structural correctness | `Structural validation failure` |
///
/// Schema existence is checked first because it is the cheapest check and the
/// most likely early failure for candidates arriving with unregistered schemas.
///
/// # Schema Registry
///
/// `known_schemas` is the authoritative set of schemas registered with Evo.
/// Validation has no opinion about which schemas are valid — it only asks
/// whether the candidate's schema is a member of that set.
///
/// # Architectural Note — IS-0003 Structural Validation
///
/// Full structural validation (verifying that Evidence contains every required
/// canonical concept for this schema) requires IS-0003 schema specification
/// types that carry concept definitions alongside the schema identifier. Until
/// those types exist, structural validation verifies construction-time
/// invariants and serves as the integration point for future concept checks.
///
/// # Non-Responsibilities
///
/// - Does **not** canonicalize evidence or provenance.
/// - Does **not** assign [`ObservationId`].
/// - Does **not** compute or verify integrity.
/// - Does **not** persist anything.
/// - Does **not** interpret or infer meaning.
///
/// [`ObservationId`]: crate::observation_id::ObservationId
pub fn validate(
    candidate: &CandidateObservation,
    schema: &ObservationSchema,
) -> Result<(), ValidationError> {
    validate_schema(candidate, schema)?;
    validate_provenance(candidate)?;
    validate_structure(candidate)?;
    Ok(())
}

// ── Sub-checks ────────────────────────────────────────────────────────────────

fn validate_schema(
    candidate: &CandidateObservation,
    schema: &ObservationSchema,
) -> Result<(), ValidationError> {
    if candidate.schema() != schema {
        return Err(ValidationError::UnknownSchema(candidate.schema().clone()));
    }

    Ok(())
}

/// Verifies that required provenance fields are present and non-empty.
///
/// IS-0001 §10: "Missing required provenance" → rejection.
///
/// [`ObservationSource`] enforces a non-empty source name at construction time,
/// making this failure unreachable through normal value-object construction.
/// The check is explicit to:
/// - satisfy IS-0001 §10's requirement that missing provenance is detected here,
/// - document the provenance requirement in a single authoritative place,
/// - guard against future refactoring that might weaken construction invariants.
///
/// [`ObservationSource`]: crate::provenance::ObservationSource
fn validate_provenance(candidate: &CandidateObservation) -> Result<(), ValidationError> {
    let provenance = candidate.provenance();

    if provenance.source().as_str().is_empty() {
        return Err(ValidationError::MissingProvenance(
            "observation source must not be empty".into(),
        ));
    }

    Ok(())
}

/// Verifies structural correctness of the candidate's evidence.
///
/// IS-0001 §10: "Structural validation failure" → rejection.
///
/// # Checks currently enforced
///
/// - Every fact name is non-empty.
///   [`ObservedFact::new`] enforces this at construction time, so this failure
///   is unreachable through normal construction. The check is explicit for
///   the same reasons as [`validate_provenance`].
///
/// # Checks deferred to IS-0003 implementation
///
/// IS-0003 §3 R-2 requires that schemas define required canonical concepts,
/// and those concepts must be present in Evidence. Full required-concept
/// validation requires IS-0003 schema specification types that carry concept
/// definitions alongside the schema identifier. When those types are
/// implemented, this function will:
///
/// 1. Accept a schema specification (not just an identifier).
/// 2. Verify every required canonical concept is present in the Evidence.
/// 3. Verify optional concepts, if present, conform to the schema's definitions.
///
/// [`ObservedFact::new`]: crate::evidence::ObservedFact::new
fn validate_structure(candidate: &CandidateObservation) -> Result<(), ValidationError> {
    for fact in candidate.evidence().facts() {
        if fact.name().is_empty() {
            return Err(ValidationError::InvalidStructure(
                "evidence contains a fact with an empty name".into(),
            ));
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::candidate::CandidateObservation;
    use crate::evidence::{Evidence, FactValue, ObservedFact};
    use crate::observation_schema::ObservationSchema;
    use crate::provenance::{ObservationSource, Provenance};

    use std::collections::HashMap;
    use std::time::SystemTime;

    fn schema(name: &str, version: u32) -> ObservationSchema {
        ObservationSchema::new(name, version).unwrap()
    }

    fn provenance() -> Provenance {
        let source = ObservationSource::new("accessibility_api").unwrap();
        Provenance::new(source, SystemTime::UNIX_EPOCH, HashMap::new())
    }

    fn candidate(schema: ObservationSchema) -> CandidateObservation {
        CandidateObservation::new(schema, provenance(), Evidence::new(vec![]))
    }

    #[test]
    fn matching_schema_passes_validation() {
        let schema = schema("app_focus", 1);

        let result = validate(&candidate(schema.clone()), &schema);

        assert!(result.is_ok());
    }

    #[test]
    fn different_schema_is_rejected() {
        let candidate_schema = schema("app_focus", 1);
        let supplied_schema = schema("window_focus", 1);

        let result = validate(
            &candidate(candidate_schema.clone()),
            &supplied_schema,
        );

        assert_eq!(
            result,
            Err(ValidationError::UnknownSchema(candidate_schema))
        );
    }

    #[test]
    fn schema_version_must_match() {
        let candidate_schema = schema("app_focus", 1);
        let supplied_schema = schema("app_focus", 2);

        let result = validate(
            &candidate(candidate_schema.clone()),
            &supplied_schema,
        );

        assert_eq!(
            result,
            Err(ValidationError::UnknownSchema(candidate_schema))
        );
    }

    #[test]
    fn empty_evidence_is_structurally_valid() {
        let schema = schema("empty", 1);

        let result = validate(&candidate(schema.clone()), &schema);

        assert!(result.is_ok());
    }

    #[test]
    fn valid_fact_passes_validation() {
        let schema = schema("facts", 1);

        let fact =
            ObservedFact::new(
                "app_name",
                FactValue::Text("VS Code".into()),
            )
            .unwrap();

        let evidence = Evidence::new(vec![fact]);

        let candidate =
            CandidateObservation::new(schema.clone(), provenance(), evidence);

        assert!(validate(&candidate, &schema).is_ok());
    }
}