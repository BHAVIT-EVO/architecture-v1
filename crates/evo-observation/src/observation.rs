//! Canonical Observation Entity.
//!
//! This module defines the immutable `Observation` type.
//!
//! An `Observation` represents the canonical result of the Observation
//! acceptance pipeline after validation, canonicalization, and identity
//! assignment.
//!
//! An `Observation` permanently owns exactly one:
//!
//! - `ObservationId`
//! - `ObservationSchema`
//! - `Provenance`
//! - `Evidence`
//!
//! Once constructed, an `Observation` is immutable.

use crate::evidence::Evidence;
use crate::observation_id::ObservationId;
use crate::observation_schema::ObservationSchema;
use crate::provenance::Provenance;

/// The canonical immutable representation of an accepted Observation.
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    id: ObservationId,
    schema: ObservationSchema,
    provenance: Provenance,
    evidence: Evidence,
}

impl Observation {
    /// Constructs a new immutable `Observation`.
    ///
    /// The caller is responsible for ensuring that:
    ///
    /// - validation has completed;
    /// - canonicalization has completed;
    /// - identity has been assigned.
    ///
    /// Observation performs no additional processing.
    pub fn new(
        id: ObservationId,
        schema: ObservationSchema,
        provenance: Provenance,
        evidence: Evidence,
    ) -> Self {
        Self {
            id,
            schema,
            provenance,
            evidence,
        }
    }

    /// Returns the immutable Observation identifier.
    pub fn id(&self) -> &ObservationId {
        &self.id
    }

    /// Returns the Observation schema.
    pub fn schema(&self) -> &ObservationSchema {
        &self.schema
    }

    /// Returns the Observation provenance.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// Returns the Observation evidence.
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::evidence::{Evidence, FactValue, ObservedFact};
    use crate::provenance::{ObservationSource, Provenance};

    use std::collections::HashMap;
    use std::time::SystemTime;

    #[test]
    fn observation_construction_and_accessors() {
        let id = ObservationId::new();

        let schema = ObservationSchema::new("test-schema", 1).unwrap();

        let source = ObservationSource::new("test-source").unwrap();

        let provenance = Provenance::new(source, SystemTime::now(), HashMap::new());

        let fact = ObservedFact::new("key", FactValue::Text("value".into())).unwrap();

        let evidence = Evidence::new(vec![fact]);

        let observation = Observation::new(
            id.clone(),
            schema.clone(),
            provenance.clone(),
            evidence.clone(),
        );

        assert_eq!(observation.id(), &id);
        assert_eq!(observation.schema(), &schema);
        assert_eq!(observation.provenance(), &provenance);
        assert_eq!(observation.evidence(), &evidence);
    }
}
