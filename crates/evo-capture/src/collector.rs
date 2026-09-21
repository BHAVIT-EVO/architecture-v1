//! Collector boundary.
//!
//! The Collector converts normalized platform-adapter output into canonical
//! Candidate Observations. It does not accept Observations.

use crate::raw_event::RawEvent;
use evo_observation::candidate::CandidateObservation;

/// Collector for normalized platform-adapter events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Collector;

impl Collector {
    /// Constructs a Collector.
    pub fn new() -> Self {
        Self
    }

    /// Translates a normalized event into a Candidate Observation.
    pub fn translate(&self, raw_event: RawEvent) -> CandidateObservation {
        raw_event.into_candidate_observation()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::macos::{MacOSAdapter, MacOSSignal};
    use evo_observation::accept::accept;
    use evo_observation::evidence::FactValue;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::observation_schema::ObservationSchema;
    use evo_observation::provenance::ObservationSource;
    use std::collections::HashMap;
    use std::time::SystemTime;

    fn cases() -> Vec<(ObservationConcept, ObservationSchema, &'static str, &'static str)> {
        vec![
            (
                ObservationConcept::WindowFocusGained {
                    subject: "editor-window".into(),
                },
                ObservationSchema::window_focus_gained_v3(),
                "WindowFocusGained",
                "editor-window",
            ),
            (
                ObservationConcept::FileSaved {
                    subject: "/tmp/report.md".into(),
                },
                ObservationSchema::file_saved_v1(),
                "FileSaved",
                "/tmp/report.md",
            ),
            (
                ObservationConcept::URLNavigated {
                    subject: "https://example.com".into(),
                },
                ObservationSchema::url_navigated_v1(),
                "URLNavigated",
                "https://example.com",
            ),
            (
                ObservationConcept::CommitMade {
                    subject: "abc123".into(),
                },
                ObservationSchema::commit_made_v1(),
                "CommitMade",
                "abc123",
            ),
            (
                ObservationConcept::InputActivity {
                    subject: "file:///work/report.md".into(),
                    keys: 43,
                    clicks: 2,
                    scrolls: 0,
                },
                ObservationSchema::input_activity_v1(),
                "InputActivity",
                "file:///work/report.md",
            ),
        ]
    }

    #[test]
    fn translates_supported_canonical_events_into_candidate_observations() {
        let collector = Collector::new();

        for (concept, schema, fact_name, subject) in cases() {
            let raw_event = RawEvent::new(
                ObservationSource::new("macos_event_source").unwrap(),
                SystemTime::UNIX_EPOCH,
                HashMap::new(),
                concept.clone(),
            );

            let candidate = collector.translate(raw_event);

            assert_eq!(candidate.schema(), &schema);
            assert_eq!(
                candidate.provenance().source().as_str(),
                "macos_event_source"
            );
            assert_eq!(candidate.provenance().observed_at(), SystemTime::UNIX_EPOCH);
            assert!(candidate.provenance().context().is_empty());
            assert_eq!(
                candidate.evidence().fact(fact_name).unwrap().value(),
                &FactValue::Text(subject.into())
            );
        }
    }

    #[test]
    fn translated_candidates_enter_existing_acceptance_path() {
        let collector = Collector::new();

        for (concept, schema, fact_name, subject) in cases() {
            let raw_event = RawEvent::new(
                ObservationSource::new("macos_event_source").unwrap(),
                SystemTime::UNIX_EPOCH,
                HashMap::new(),
                concept,
            );

            let candidate = collector.translate(raw_event);
            let observation = accept(candidate, &schema).expect("valid candidate should be accepted");

            assert_eq!(observation.schema(), &schema);
            assert_eq!(observation.provenance().observed_at(), SystemTime::UNIX_EPOCH);
            assert_eq!(
                observation.evidence().fact(fact_name).unwrap().value(),
                &FactValue::Text(subject.into())
            );
        }
    }

    #[test]
    fn unsupported_activation_signal_stops_without_fabrication() {
        let adapter = MacOSAdapter::new(ObservationSource::new("macos_event_source").unwrap());
        let result = adapter.normalize(MacOSSignal::ApplicationActivated {
            bundle_identifier: None,
            application_name: None,
            process_identifier: None,
            observed_at: SystemTime::UNIX_EPOCH,
        });

        assert!(matches!(result, Ok(None)));
    }
}
