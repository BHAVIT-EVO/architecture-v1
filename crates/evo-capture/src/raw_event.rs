//! Canonical raw event representation used by Capture.
//!
//! A `RawEvent` is a normalized envelope of one frozen canonical Observation
//! Language concept. It does not decide meaning, importance, or ownership. It
//! is only a transport form between platform adapters and the Observation
//! acceptance pipeline.

use evo_observation::candidate::CandidateObservation;
use evo_observation::evidence::Evidence;
use evo_observation::observation_language::ObservationConcept;
use evo_observation::observation_schema::ObservationSchema;
use evo_observation::provenance::{ObservationSource, Provenance};

use std::collections::HashMap;
use std::time::SystemTime;

/// A canonical, platform-independent envelope of witnessed facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEvent {
    source: ObservationSource,
    observed_at: SystemTime,
    context: HashMap<String, String>,
    concept: ObservationConcept,
    evidence: Evidence,
}

impl RawEvent {
    /// Constructs a new `RawEvent`.
    pub fn new(
        source: ObservationSource,
        observed_at: SystemTime,
        context: HashMap<String, String>,
        concept: ObservationConcept,
    ) -> Self {
        let evidence = concept.evidence();
        Self {
            source,
            observed_at,
            context,
            concept,
            evidence,
        }
    }

    /// Returns the observation source.
    pub fn source(&self) -> &ObservationSource {
        &self.source
    }

    /// Returns the wall-clock time at which the event was observed.
    pub fn observed_at(&self) -> SystemTime {
        self.observed_at
    }

    /// Returns the raw observation context.
    pub fn context(&self) -> &HashMap<String, String> {
        &self.context
    }

    /// Returns the canonical Observation Language concept.
    pub fn concept(&self) -> &ObservationConcept {
        &self.concept
    }

    /// Returns the canonical Observation Schema for this raw event.
    pub fn schema(&self) -> ObservationSchema {
        self.concept.schema()
    }

    /// Returns the observed evidence.
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }

    /// Consumes the raw event and converts it into a `CandidateObservation`.
    pub fn into_candidate_observation(self) -> CandidateObservation {
        let schema = self.concept.schema();
        let provenance = Provenance::new(self.source, self.observed_at, self.context);
        CandidateObservation::new(schema, provenance, self.evidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_observation::observation_language::ObservationConcept;

    #[test]
    fn raw_event_preserves_components() {
        let source = ObservationSource::new("macos_adapter").unwrap();
        let concept = ObservationConcept::WindowFocusGained {
            subject: "editor-window".into(),
        };
        let mut context = HashMap::new();
        context.insert("session".into(), "1".into());

        let event = RawEvent::new(
            source.clone(),
            SystemTime::UNIX_EPOCH,
            context.clone(),
            concept.clone(),
        );

        assert_eq!(event.source(), &source);
        assert_eq!(event.observed_at(), SystemTime::UNIX_EPOCH);
        assert_eq!(event.context(), &context);
        assert_eq!(event.concept(), &concept);
        // The WindowFocusGained concept maps to v2 — the contract that may
        // carry owning-process provenance in context (BE-TRACE-0001 §3.1).
        assert_eq!(event.schema(), ObservationSchema::window_focus_gained_v3());
        assert_eq!(event.evidence(), &concept.evidence());
    }
}
