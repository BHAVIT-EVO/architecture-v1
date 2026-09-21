//! Capture runtime engine.
//!
//! The engine accepts normalized `RawEvent` values, constructs
//! `CandidateObservation` values through the Collector boundary, and forwards
//! them to the Observation acceptance pipeline. It performs no semantic
//! reasoning.

use crate::collector::Collector;
use crate::raw_event::RawEvent;
use evo_observation::accept::accept;
use evo_observation::errors::ObservationError;
use evo_observation::observation::Observation;
use evo_observation::observation_schema::ObservationSchema;

/// Platform-independent capture engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CaptureEngine;

impl CaptureEngine {
    /// Constructs a capture engine with no prior event state.
    pub fn new() -> Self {
        Self
    }

    /// Ingests one normalized event and forwards it into Observation acceptance.
    pub fn ingest(
        &mut self,
        raw_event: RawEvent,
        registered_schema: &ObservationSchema,
    ) -> Result<Observation, ObservationError> {
        let collector = Collector::new();
        let candidate = collector.translate(raw_event);
        accept(candidate, registered_schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw_event::RawEvent;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::observation_schema::ObservationSchema;
    use evo_observation::provenance::ObservationSource;
    use std::collections::HashMap;
    use std::time::SystemTime;

    fn schema() -> ObservationSchema {
        // OBS-WINDOW-FOCUS-GAINED/v2: the witnessed contract under which a
        // focus observation may carry owning-process provenance in
        // `Provenance::context` (BE-TRACE-0001 §3.1).
        ObservationSchema::window_focus_gained_v3()
    }

    fn raw_event(timestamp: SystemTime) -> RawEvent {
        let source = ObservationSource::new("macos_adapter").unwrap();
        let concept = ObservationConcept::WindowFocusGained {
            subject: "editor-window".into(),
        };
        RawEvent::new(source, timestamp, HashMap::new(), concept)
    }

    #[test]
    fn ingest_forwards_raw_event_into_observation_acceptance() {
        let mut engine = CaptureEngine::new();
        let schema = schema();
        let event = raw_event(SystemTime::UNIX_EPOCH);

        let result = engine.ingest(event.clone(), &schema).unwrap();

        let observation = result;
        assert_eq!(observation.schema(), &schema);
        assert_eq!(observation.provenance().source().as_str(), "macos_adapter");
        assert_eq!(observation.provenance().observed_at(), SystemTime::UNIX_EPOCH);
        assert_eq!(observation.evidence(), event.evidence());
        assert_eq!(
            observation.evidence().fact("WindowFocusGained").unwrap().value(),
            event.evidence().fact("WindowFocusGained").unwrap().value()
        );
    }

    #[test]
    fn ingest_does_not_suppress_duplicate_state() {
        let mut engine = CaptureEngine::new();
        let schema = schema();
        let first = raw_event(SystemTime::UNIX_EPOCH);
        let second = raw_event(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1));

        let first_result = engine.ingest(first, &schema).unwrap();
        let second_result = engine.ingest(second, &schema).unwrap();

        assert_eq!(first_result.provenance().observed_at(), SystemTime::UNIX_EPOCH);
        assert_eq!(
            second_result.provenance().observed_at(),
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1)
        );
    }
}

// EXECUTION PROVENANCE: observations captured during Evo.execute() should carry
// provenance context['evo_origin']='execution' so reconstruction distinguishes
// restoration-side-effects from genuine user activity. See provenance.rs.
