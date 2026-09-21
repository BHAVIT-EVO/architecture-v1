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
use crate::evidence::{FactValue, ObservedFact};
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

    if !schema.is_canonical() {
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
    let schema = candidate.schema();
    let expected_fact_name = schema
        .canonical_fact_name()
        .ok_or_else(|| ValidationError::UnknownSchema(schema.clone()))?;

    let facts = candidate.evidence().facts();

    // The co-membership schemas (RFC-0012, IS-0003 §4.1.6/§4.1.7) require the
    // witnessed fact plus one subject-valued value concept: Repository for
    // OBS-REPOSITORY-MEMBERSHIP, CoMember for OBS-WORK-GROUPED. The
    // continuation-surface schema (RFC-0013, IS-0003 §4.1.8) requires the
    // witnessed declaration fact plus at least one ContinuationSubject fact
    // (a valid surface names at least two distinct subjects). Every other
    // canonical schema carries exactly one canonical fact.
    let expected_second = match schema.name() {
        "OBS-REPOSITORY-MEMBERSHIP" => Some("Repository"),
        "OBS-WORK-GROUPED" => Some("CoMember"),
        _ => None,
    };
    if schema.name() == "OBS-CONTINUATION-SURFACE" {
        return validate_continuation_surface(facts, expected_fact_name);
    }
    if schema.name() == "OBS-INPUT-ACTIVITY" {
        return validate_input_activity(facts, expected_fact_name);
    }
    let expected_len = if expected_second.is_some() { 2 } else { 1 };
    if facts.len() != expected_len {
        return Err(ValidationError::InvalidStructure(format!(
            "evidence must contain exactly {} canonical fact(s)",
            expected_len
        )));
    }

    let fact = &facts[0];
    if fact.name() != expected_fact_name {
        return Err(ValidationError::InvalidStructure(
            "evidence does not express the required canonical concept".into(),
        ));
    }

    if let Some(second_name) = expected_second {
        let second = &facts[1];
        if second.name() != second_name {
            return Err(ValidationError::InvalidStructure(format!(
                "evidence does not express the required {second_name} value concept"
            )));
        }
        match second.value() {
            FactValue::Text(value) if !value.trim().is_empty() => {}
            _ => {
                return Err(ValidationError::InvalidStructure(format!(
                    "required {second_name} value information is missing"
                )))
            }
        }
    }

    match fact.value() {
        FactValue::Text(subject) if !subject.trim().is_empty() => Ok(()),
        _ => Err(ValidationError::InvalidStructure(
            "required subject information is missing".into(),
        )),
    }
}

/// Validates the structural rule of `OBS-CONTINUATION-SURFACE/v1` (RFC-0013,
/// IS-0003 §4.1.8):
///
/// - the declaration fact (`ContinuationSurface`) carries the Required
///   Subject — the lexicographically smallest canonical subject;
/// - every additional fact is a `ContinuationSubject` value fact;
/// - a valid declaration names at least two distinct subjects;
/// - subjects are non-empty and unique (a declaration SHALL NOT name a
///   subject twice);
/// - the declaration fact is first, and the Required Subject is the
///   lexicographically smallest subject (the canonical ordering rule).
fn validate_continuation_surface(
    facts: &[ObservedFact],
    expected_fact_name: &str,
) -> Result<(), ValidationError> {
    if facts.len() < 2 {
        return Err(ValidationError::InvalidStructure(
            "a continuation surface must name at least two distinct subjects".into(),
        ));
    }
    let declaration = &facts[0];
    if declaration.name() != expected_fact_name {
        return Err(ValidationError::InvalidStructure(
            "evidence does not express the required canonical concept".into(),
        ));
    }
    let required = match declaration.value() {
        FactValue::Text(subject) if !subject.trim().is_empty() => subject.as_str(),
        _ => {
            return Err(ValidationError::InvalidStructure(
                "required subject information is missing".into(),
            ))
        }
    };

    let mut subjects: Vec<&str> = Vec::with_capacity(facts.len());
    let mut seen: Vec<&str> = vec![required];
    for fact in &facts[1..] {
        if fact.name() != "ContinuationSubject" {
            return Err(ValidationError::InvalidStructure(
                "evidence does not express the required ContinuationSubject value concept".into(),
            ));
        }
        match fact.value() {
            FactValue::Text(subject) if !subject.trim().is_empty() => {
                if seen.contains(&subject.as_str()) {
                    return Err(ValidationError::InvalidStructure(
                        "a continuation surface must not name a subject twice".into(),
                    ));
                }
                seen.push(subject.as_str());
                subjects.push(subject.as_str());
            }
            _ => {
                return Err(ValidationError::InvalidStructure(
                    "required ContinuationSubject value information is missing".into(),
                ))
            }
        }
    }
    // Canonical ordering: the declaration fact carries the lexicographically
    // smallest subject; the remainder follow in ascending order. This makes
    // the canonical record order-independent (RFC-0013 Deterministic
    // encoding).
    if required != seen.iter().min().copied().expect("at least two subjects") {
        return Err(ValidationError::InvalidStructure(
            "the Required Subject of a continuation surface must be the lexicographically smallest subject".into(),
        ));
    }
    let mut tail: Vec<&str> = subjects.to_vec();
    tail.sort_unstable();
    if tail != subjects {
        return Err(ValidationError::InvalidStructure(
            "continuation surface subjects must be canonically ordered".into(),
        ));
    }
    Ok(())
}

/// Validates the structural rule of `OBS-INPUT-ACTIVITY/v1`:
///
/// - exactly four canonical facts, in order: the witnessed `InputActivity`
///   fact carrying a non-empty Text subject, then `Keys`, `Clicks`, and
///   `Scrolls`, each carrying a non-negative Integer count;
/// - at least one count must be positive. An all-zero bucket is not an
///   observation — absence of the record is the zero, so a candidate
///   asserting all zeros is malformed rather than empty.
fn validate_input_activity(
    facts: &[ObservedFact],
    expected_fact_name: &str,
) -> Result<(), ValidationError> {
    if facts.len() != 4 {
        return Err(ValidationError::InvalidStructure(
            "input activity evidence must contain exactly four canonical facts".into(),
        ));
    }
    if facts[0].name() != expected_fact_name {
        return Err(ValidationError::InvalidStructure(
            "evidence does not express the required canonical concept".into(),
        ));
    }
    match facts[0].value() {
        FactValue::Text(subject) if !subject.trim().is_empty() => {}
        _ => {
            return Err(ValidationError::InvalidStructure(
                "required subject information is missing".into(),
            ))
        }
    }
    let mut any_positive = false;
    for fact in &facts[1..] {
        let ok = match (fact.name(), fact.value()) {
            ("Keys", FactValue::Integer(n)) | ("Clicks", FactValue::Integer(n)) => {
                any_positive |= *n > 0;
                *n >= 0
            }
            ("Scrolls", FactValue::Integer(n)) => {
                any_positive |= *n > 0;
                *n >= 0
            }
            _ => false,
        };
        if !ok {
            return Err(ValidationError::InvalidStructure(format!(
                "the {} count must be a non-negative integer",
                fact.name()
            )));
        }
    }
    if !any_positive {
        return Err(ValidationError::InvalidStructure(
            "an all-zero input bucket is not an observation; absence is the zero".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::candidate::CandidateObservation;
    use crate::evidence::{Evidence, FactValue, ObservedFact};
    use crate::observation_language::ObservationConcept;
    use crate::observation_schema::ObservationSchema;
    use crate::provenance::{ObservationSource, Provenance};

    use std::collections::HashMap;
    use std::time::SystemTime;

    fn schema() -> ObservationSchema {
        ObservationSchema::window_focus_gained_v1()
    }

    fn provenance() -> Provenance {
        let source = ObservationSource::new("accessibility_api").unwrap();
        Provenance::new(source, SystemTime::UNIX_EPOCH, HashMap::new())
    }

    fn candidate(schema: ObservationSchema) -> CandidateObservation {
        let concept = ObservationConcept::WindowFocusGained {
            subject: "editor-window".into(),
        };
        CandidateObservation::new(schema, provenance(), concept.evidence())
    }

    fn input_candidate(keys: i64, clicks: i64, scrolls: i64) -> CandidateObservation {
        let concept = ObservationConcept::InputActivity {
            subject: "file:///work/report.md".into(),
            keys: keys as u64,
            clicks: clicks as u64,
            scrolls: scrolls as u64,
        };
        CandidateObservation::new(
            ObservationSchema::input_activity_v1(),
            provenance(),
            concept.evidence(),
        )
    }

    #[test]
    fn input_activity_with_counts_passes_validation() {
        let schema = ObservationSchema::input_activity_v1();
        let candidate = input_candidate(43, 2, 0);
        assert!(validate(&candidate, &schema).is_ok());
    }

    #[test]
    fn input_activity_all_zero_bucket_is_rejected() {
        // Absence of the observation is the zero; an all-zero bucket is not
        // a record and must not enter the pipeline.
        let schema = ObservationSchema::input_activity_v1();
        let candidate = input_candidate(0, 0, 0);
        assert!(validate(&candidate, &schema).is_err());
    }

    #[test]
    fn input_activity_negative_counts_are_rejected() {
        let schema = ObservationSchema::input_activity_v1();
        let candidate = input_candidate(-1, 0, 0);
        assert!(validate(&candidate, &schema).is_err());
    }

    #[test]
    fn input_activity_survives_full_acceptance() {
        use crate::accept::accept;
        let schema = ObservationSchema::input_activity_v1();
        let candidate = input_candidate(61, 4, 12);
        let observation = accept(candidate, &schema).expect("valid input activity is accepted");
        assert_eq!(observation.schema(), &schema);
        assert_eq!(
            observation.evidence().fact("Keys").unwrap().value(),
            &FactValue::Integer(61)
        );
    }

    #[test]
    fn matching_schema_passes_validation() {
        let schema = schema();

        let result = validate(&candidate(schema.clone()), &schema);

        assert!(result.is_ok());
    }

    #[test]
    fn different_schema_is_rejected() {
        let candidate_schema = ObservationSchema::window_focus_gained_v1();
        let supplied_schema = ObservationSchema::file_saved_v1();

        let result = validate(&candidate(candidate_schema.clone()), &supplied_schema);

        assert_eq!(
            result,
            Err(ValidationError::UnknownSchema(candidate_schema))
        );
    }

    #[test]
    fn schema_version_must_match() {
        let candidate_schema = ObservationSchema::window_focus_gained_v1();
        let supplied_schema = ObservationSchema::new("OBS-WINDOW-FOCUS-GAINED", 2).unwrap();

        let result = validate(&candidate(candidate_schema.clone()), &supplied_schema);

        assert_eq!(
            result,
            Err(ValidationError::UnknownSchema(candidate_schema))
        );
    }

    #[test]
    fn unsupported_schema_is_rejected() {
        let candidate_schema = ObservationSchema::new("application_activated", 1).unwrap();
        let result = validate(&candidate(candidate_schema.clone()), &candidate_schema);

        assert_eq!(
            result,
            Err(ValidationError::UnknownSchema(candidate_schema))
        );
    }

    #[test]
    fn valid_fact_passes_validation() {
        let schema = schema();

        let fact =
            ObservedFact::new("WindowFocusGained", FactValue::Text("editor-window".into()))
                .unwrap();
        let evidence = Evidence::new(vec![fact]);
        let candidate = CandidateObservation::new(schema.clone(), provenance(), evidence);

        assert!(validate(&candidate, &schema).is_ok());
    }

    #[test]
    fn empty_evidence_is_rejected() {
        let schema = schema();

        let candidate = CandidateObservation::new(schema.clone(), provenance(), Evidence::new(vec![]));

        assert_eq!(
            validate(&candidate, &schema),
            Err(ValidationError::InvalidStructure(
                "evidence must contain exactly 1 canonical fact(s)".into()
            ))
        );
    }

    #[test]
    fn co_membership_schemas_require_their_value_concept() {
        // OBS-REPOSITORY-MEMBERSHIP carries the witnessed fact plus the
        // Repository value concept (RFC-0012, IS-0003 §4.1.6).
        let schema = ObservationSchema::repository_membership_v1();
        let facts = vec![
            ObservedFact::new(
                "RepositoryMembership",
                FactValue::Text("/repo/a.rs".into()),
            )
            .unwrap(),
            ObservedFact::new("Repository", FactValue::Text("/repo/.git".into())).unwrap(),
        ];
        let candidate = CandidateObservation::new(schema.clone(), provenance(), Evidence::new(facts));
        assert!(validate(&candidate, &schema).is_ok());

        // Missing the value concept → rejected.
        let partial = CandidateObservation::new(
            schema.clone(),
            provenance(),
            Evidence::new(vec![ObservedFact::new(
                "RepositoryMembership",
                FactValue::Text("/repo/a.rs".into()),
            )
            .unwrap()]),
        );
        assert!(validate(&partial, &schema).is_err());

        // OBS-WORK-GROUPED carries WorkGrouped + CoMember (IS-0003 §4.1.7).
        let grouped = ObservationSchema::work_grouped_v1();
        let grouped_facts = vec![
            ObservedFact::new("WorkGrouped", FactValue::Text("/repo/a.rs".into())).unwrap(),
            ObservedFact::new("CoMember", FactValue::Text("https://example.com".into())).unwrap(),
        ];
        let candidate = CandidateObservation::new(grouped.clone(), provenance(), Evidence::new(grouped_facts));
        assert!(validate(&candidate, &grouped).is_ok());
    }

    // ── Continuation surface (RFC-0013, IS-0003 §4.1.8) ─────────────────────

    fn surface_candidate(subjects: &[&str]) -> (ObservationSchema, CandidateObservation) {
        let schema = ObservationSchema::continuation_surface_v1();
        let concept = ObservationConcept::ContinuationSurface {
            subjects: subjects.iter().map(|subject| subject.to_string()).collect(),
        };
        let candidate = CandidateObservation::new(schema.clone(), provenance(), concept.evidence());
        (schema, candidate)
    }

    #[test]
    fn valid_continuation_surface_passes_validation() {
        let (schema, candidate) = surface_candidate(&["/tmp/a.md", "/tmp/b.md"]);
        assert!(validate(&candidate, &schema).is_ok());

        let (schema, candidate) = surface_candidate(&["/tmp/a.md", "/tmp/b.md", "/tmp/c.md"]);
        assert!(validate(&candidate, &schema).is_ok());
    }

    #[test]
    fn continuation_surface_with_single_subject_is_rejected() {
        let (schema, candidate) = surface_candidate(&["/tmp/a.md"]);
        let result = validate(&candidate, &schema);
        assert_eq!(
            result,
            Err(ValidationError::InvalidStructure(
                "a continuation surface must name at least two distinct subjects".into()
            ))
        );
    }

    #[test]
    fn continuation_surface_with_duplicate_subjects_is_rejected() {
        let schema = ObservationSchema::continuation_surface_v1();
        let facts = vec![
            ObservedFact::new("ContinuationSurface", FactValue::Text("/tmp/a.md".into())).unwrap(),
            ObservedFact::new("ContinuationSubject", FactValue::Text("/tmp/a.md".into())).unwrap(),
        ];
        let candidate = CandidateObservation::new(schema.clone(), provenance(), Evidence::new(facts));
        let result = validate(&candidate, &schema);
        assert_eq!(
            result,
            Err(ValidationError::InvalidStructure(
                "a continuation surface must not name a subject twice".into()
            ))
        );
    }

    #[test]
    fn continuation_surface_without_declaration_fact_is_rejected() {
        let schema = ObservationSchema::continuation_surface_v1();
        let facts = vec![
            ObservedFact::new("ContinuationSubject", FactValue::Text("/tmp/a.md".into())).unwrap(),
            ObservedFact::new("ContinuationSubject", FactValue::Text("/tmp/b.md".into())).unwrap(),
        ];
        let candidate = CandidateObservation::new(schema.clone(), provenance(), Evidence::new(facts));
        let result = validate(&candidate, &schema);
        assert!(matches!(result, Err(ValidationError::InvalidStructure(_))));
    }

    #[test]
    fn continuation_surface_requires_the_value_concept_for_extra_subjects() {
        let schema = ObservationSchema::continuation_surface_v1();
        let facts = vec![
            ObservedFact::new("ContinuationSurface", FactValue::Text("/tmp/a.md".into())).unwrap(),
            ObservedFact::new("WrongConcept", FactValue::Text("/tmp/b.md".into())).unwrap(),
        ];
        let candidate = CandidateObservation::new(schema.clone(), provenance(), Evidence::new(facts));
        let result = validate(&candidate, &schema);
        assert!(matches!(result, Err(ValidationError::InvalidStructure(_))));
    }

    #[test]
    fn continuation_surface_requires_canonical_ordering() {
        // The Required Subject must be the lexicographically smallest subject
        // (RFC-0013 Deterministic encoding). A declaration expressed in any
        // order must produce the identical canonical Observation.
        let schema = ObservationSchema::continuation_surface_v1();
        let facts = vec![
            ObservedFact::new("ContinuationSurface", FactValue::Text("/tmp/b.md".into())).unwrap(),
            ObservedFact::new("ContinuationSubject", FactValue::Text("/tmp/a.md".into())).unwrap(),
        ];
        let candidate = CandidateObservation::new(schema.clone(), provenance(), Evidence::new(facts));
        let result = validate(&candidate, &schema);
        assert!(matches!(result, Err(ValidationError::InvalidStructure(_))));

        // The canonical order is accepted and deterministic.
        let (schema, candidate) = surface_candidate(&["/tmp/b.md", "/tmp/a.md", "/tmp/c.md"]);
        assert!(validate(&candidate, &schema).is_ok());
        let declaration = candidate.evidence().facts()[0].value().clone();
        assert_eq!(declaration, FactValue::Text("/tmp/a.md".into()));
    }

    #[test]
    fn continuation_surface_subject_order_does_not_change_identity() {
        // Order-independence of the canonical record (RFC-0013): the concept
        // canonicalizes before building evidence, so reversed declaration
        // order produces byte-identical evidence.
        let first = ObservationConcept::ContinuationSurface {
            subjects: vec!["/tmp/b.md".into(), "/tmp/a.md".into()],
        };
        let second = ObservationConcept::ContinuationSurface {
            subjects: vec!["/tmp/a.md".into(), "/tmp/b.md".into()],
        };
        assert_eq!(first.evidence(), second.evidence());
    }

    #[test]
    fn continuation_surface_schema_is_canonical_and_reference_only() {
        let schema = ObservationSchema::continuation_surface_v1();
        assert!(schema.is_canonical());
        assert!(schema.is_reference_only());
        assert_eq!(schema.canonical_fact_name(), Some("ContinuationSurface"));
    }
}
