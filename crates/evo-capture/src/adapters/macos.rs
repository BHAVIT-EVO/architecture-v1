//! Minimal macOS capture adapter.
//!
//! This adapter is intentionally thin. It accepts structured macOS signals and
//! converts them into platform-independent `RawEvent` values when the signal
//! directly establishes one of the frozen canonical Observation concepts.

use crate::raw_event::RawEvent;
use evo_observation::observation_language::ObservationConcept;
use evo_observation::observed_state::ObservedState;
use evo_observation::provenance::ObservationSource;

use std::collections::HashMap;
use std::time::SystemTime;

/// The `Provenance::context` key that carries the owning process identifier
/// of a witnessed focused window (OBS-WINDOW-FOCUS-GAINED/v2).
///
/// The pid is provenance of the witnessing — which process owned the window
/// Evo observed — not an additional witnessed fact, so it rides in
/// `Provenance::context` rather than in the Evidence vector (BE-TRACE-0001
/// §3.1). The value is the decimal `pid` string. A v1 observation carries no
/// such key; absence means the owning process was not observed.
pub const OWNING_PROCESS_PID_CONTEXT_KEY: &str = "owning_process_pid";

/// The `Provenance::context` key carrying the owning application's localized
/// name (e.g. "Google Chrome") read from the application element's title at
/// witness time — the same provenance class as the pid. A pid is opaque to
/// every later reader; the name is what works, restore, and presentation
/// reason about. Absence means the name was not observed.
pub const OWNING_PROCESS_NAME_CONTEXT_KEY: &str = "owning_process_name";

/// Minimal structured macOS signal used by the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacOSSignal {
    /// A focused window was directly witnessed.
    ///
    /// `process_identifier` and `owning_process_name` identify the process
    /// that owned the focused window at witness time, when the collector
    /// observed it. They are provenance of the witnessing (BE-AUDIT-0001
    /// §8.6) and are carried into `Provenance::context`, never into
    /// Evidence.
    WindowFocusGained {
        subject: String,
        observed_at: SystemTime,
        process_identifier: Option<i32>,
        owning_process_name: Option<String>,
        observed_state: Option<ObservedState>,
    },
    /// A file save was directly witnessed.
    FileSaved {
        subject: String,
        observed_at: SystemTime,
    },
    /// A navigation to a URL was directly witnessed.
    URLNavigated {
        subject: String,
        observed_at: SystemTime,
    },
    /// A commit was directly witnessed.
    CommitMade {
        subject: String,
        observed_at: SystemTime,
    },
    /// The user explicitly declared that their work continues at a subject
    /// (RFC-0011, OBS-WORK-DESIGNATED). The declaration is a witnessed user
    /// act, not a capture signal.
    WorkDesignated {
        subject: String,
        observed_at: SystemTime,
    },
    /// The directly witnessed structural fact that a file, directory, or
    /// commit is a member of a repository (RFC-0012,
    /// OBS-REPOSITORY-MEMBERSHIP). Resolved at capture time; Formation never
    /// queries live git state.
    RepositoryMembership {
        member: String,
        repository: String,
        observed_at: SystemTime,
    },
    /// The directly witnessed act that the user explicitly declared two
    /// already-witnessed subjects to be related work (RFC-0012,
    /// OBS-WORK-GROUPED).
    WorkGrouped {
        first: String,
        second: String,
        observed_at: SystemTime,
    },
    /// The directly witnessed act that the user explicitly declared a set of
    /// already-witnessed subjects to constitute the current continuation of
    /// their work (RFC-0013, OBS-CONTINUATION-SURFACE). The declaration is a
    /// witnessed user act, not a capture signal. The set is canonicalized
    /// (ordered, deduplicated) by the daemon before acceptance.
    ContinuationSurface {
        subjects: Vec<String>,
        observed_at: SystemTime,
    },
    /// Directly witnessed input activity on a surface, as content-free
    /// counters aggregated over one flush bucket (OBS-INPUT-ACTIVITY/v1).
    ///
    /// `subject` is the focused surface the bucket is attributed to
    /// (document-grain where known). The counts are aggregates; what was
    /// typed, clicked, or scrolled was never observed. An all-zero bucket is
    /// never signalled — absence of the signal is the zero.
    InputActivity {
        subject: Option<String>,
        keys: u64,
        clicks: u64,
        scrolls: u64,
        observed_at: SystemTime,
    },
    /// Application activation observed by the operating system.
    ApplicationActivated {
        bundle_identifier: Option<String>,
        application_name: Option<String>,
        process_identifier: Option<u32>,
        observed_at: SystemTime,
    },
}

/// Thin adapter that converts macOS signals into canonical `RawEvent` values
/// only when the platform signal directly witnesses one of the frozen concepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacOSAdapter {
    source: ObservationSource,
}

impl MacOSAdapter {
    /// Constructs a macOS adapter for a specific observation source label.
    pub fn new(source: ObservationSource) -> Self {
        Self { source }
    }

    /// Resolves the engine-facing subject of a focus observation.
    ///
    /// Identity precedence (the capture layer's half of document-grain
    /// identity): a real document locator — the `AXDocument` attribute, a
    /// `file://` URL — outranks the window title, because one document is
    /// one resource regardless of which window showed it, while a title like
    /// "Visual Studio Code" names the application, not the work. When no
    /// locator exists the title stands, honestly coarse: a guessed fine
    /// identity would be worse than a stated coarse one.
    pub fn resolve_focus_subject(title: &str, document_locator: Option<String>) -> String {
        document_locator
            .filter(|locator| !locator.trim().is_empty())
            .unwrap_or_else(|| title.to_string())
    }

    /// Normalizes one structured macOS signal into a `RawEvent` when possible.
    pub fn normalize(&self, signal: MacOSSignal) -> Result<Option<RawEvent>, std::convert::Infallible> {
        match signal {
            MacOSSignal::WindowFocusGained {
                subject,
                observed_at,
                process_identifier,
                owning_process_name,
                observed_state,
            } => {
                let mut context = HashMap::new();
                if let Some(pid) = process_identifier {
                    context.insert(
                        OWNING_PROCESS_PID_CONTEXT_KEY.to_string(),
                        pid.to_string(),
                    );
                }
                if let Some(name) = owning_process_name {
                    context.insert(
                        OWNING_PROCESS_NAME_CONTEXT_KEY.to_string(),
                        name,
                    );
                }
                if let Some(state) = observed_state { state.write_context(&mut context); }
                Ok(Some(self.translate(
                    observed_at,
                    context,
                    ObservationConcept::WindowFocusGained { subject },
                )))
            }
            MacOSSignal::FileSaved {
                subject,
                observed_at,
            } => Ok(Some(self.translate(
                observed_at,
                HashMap::new(),
                ObservationConcept::FileSaved { subject },
            ))),
            MacOSSignal::URLNavigated {
                subject,
                observed_at,
            } => Ok(Some(self.translate(
                observed_at,
                HashMap::new(),
                ObservationConcept::URLNavigated { subject },
            ))),
            MacOSSignal::CommitMade {
                subject,
                observed_at,
            } => Ok(Some(self.translate(
                observed_at,
                HashMap::new(),
                ObservationConcept::CommitMade { subject },
            ))),
            MacOSSignal::WorkDesignated {
                subject,
                observed_at,
            } => Ok(Some(self.translate(
                observed_at,
                HashMap::new(),
                ObservationConcept::WorkDesignated { subject },
            ))),
            MacOSSignal::RepositoryMembership {
                member,
                repository,
                observed_at,
            } => Ok(Some(self.translate(
                observed_at,
                HashMap::new(),
                ObservationConcept::RepositoryMembership { member, repository },
            ))),
            MacOSSignal::WorkGrouped {
                first,
                second,
                observed_at,
            } => Ok(Some(self.translate(
                observed_at,
                HashMap::new(),
                ObservationConcept::WorkGrouped { first, second },
            ))),
            MacOSSignal::ContinuationSurface {
                subjects,
                observed_at,
            } => Ok(Some(self.translate(
                observed_at,
                HashMap::new(),
                ObservationConcept::ContinuationSurface { subjects },
            ))),
            MacOSSignal::ApplicationActivated { .. } => Ok(None),
            MacOSSignal::InputActivity {
                subject,
                keys,
                clicks,
                scrolls,
                observed_at,
            } => {
                // A bucket with no attributable surface carries nothing the
                // observation language can express: the subject is the
                // primary fact, and guessing one would fabricate identity.
                // Dropped, with the counts, in the same place every other
                // non-canonical signal lands.
                let Some(subject) = subject.filter(|s| !s.trim().is_empty()) else {
                    return Ok(None);
                };
                Ok(Some(self.translate(
                    observed_at,
                    HashMap::new(),
                    ObservationConcept::InputActivity { subject, keys, clicks, scrolls },
                )))
            }
        }
    }

    fn translate(
        &self,
        observed_at: SystemTime,
        context: HashMap<String, String>,
        concept: ObservationConcept,
    ) -> RawEvent {
        RawEvent::new(self.source.clone(), observed_at, context, concept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_observation::accept::accept;
    use evo_observation::observation_schema::ObservationSchema;

    #[test]
    fn adapter_normalizes_supported_canonical_signal_to_raw_event() {
        let source = ObservationSource::new("macos_adapter").unwrap();
        let adapter = MacOSAdapter::new(source.clone());

        let event = adapter
            .normalize(MacOSSignal::WindowFocusGained {
                subject: "editor-window".into(),
                observed_at: SystemTime::UNIX_EPOCH,
                process_identifier: Some(4242),
                owning_process_name: None,
                observed_state: Some(ObservedState::new()
                    .with_document_locator("/work/report.md")
                    .with_selection(12, 3)),
            })
            .unwrap();

        let raw_event = event.expect("canonical signal should normalize");
        assert_eq!(raw_event.concept(), &ObservationConcept::WindowFocusGained {
            subject: "editor-window".into(),
        });
        assert_eq!(raw_event.source(), &source);
        assert_eq!(raw_event.observed_at(), SystemTime::UNIX_EPOCH);
        assert_eq!(
            raw_event.context().get(OWNING_PROCESS_PID_CONTEXT_KEY).map(String::as_str),
            Some("4242")
        );
        assert_eq!(raw_event.schema(), ObservationSchema::window_focus_gained_v3());
        assert_eq!(
            ObservedState::from_context(raw_event.context()).and_then(|state| state.selection()),
            Some((12, 3))
        );
        assert_eq!(raw_event.evidence().fact("WindowFocusGained").unwrap().value(), &evo_observation::evidence::FactValue::Text("editor-window".into()));
    }

    #[test]
    fn adapter_window_focus_without_optional_provenance_is_still_v3() {
        // Optional pid and interface state remain absent rather than being
        // fabricated, while new focus observations use the current schema.
        let source = ObservationSource::new("macos_adapter").unwrap();
        let adapter = MacOSAdapter::new(source);

        let raw_event = adapter
            .normalize(MacOSSignal::WindowFocusGained {
                subject: "editor-window".into(),
                observed_at: SystemTime::UNIX_EPOCH,
                process_identifier: None,
                owning_process_name: None,
                observed_state: None,
            })
            .unwrap()
            .expect("canonical signal");
        assert_eq!(raw_event.schema(), ObservationSchema::window_focus_gained_v3());
        assert!(raw_event.context().is_empty());
    }

    #[test]
    fn adapter_pid_survives_acceptance_into_provenance_context() {
        // The full witness path: signal → normalize → accept. The owning pid
        // must survive into the accepted Observation's `Provenance::context`
        // — the exact provenance contract BE-TRACE-0001 §3.1 establishes —
        // and never enter the Evidence vector.
        let source = ObservationSource::new("macos_adapter").unwrap();
        let adapter = MacOSAdapter::new(source);

        let raw_event = adapter
            .normalize(MacOSSignal::WindowFocusGained {
                subject: "editor-window".into(),
                observed_at: SystemTime::UNIX_EPOCH,
                process_identifier: Some(7),
                owning_process_name: None,
                observed_state: None,
            })
            .unwrap()
            .expect("canonical signal");
        let schema = raw_event.schema();
        let observation = accept(raw_event.into_candidate_observation(), &schema)
            .expect("valid v2 candidate is accepted");

        assert_eq!(observation.schema(), &ObservationSchema::window_focus_gained_v3());
        assert_eq!(
            observation
                .provenance()
                .context()
                .get(OWNING_PROCESS_PID_CONTEXT_KEY)
                .map(String::as_str),
            Some("7")
        );
        // Evidence is unchanged: exactly one canonical fact, the subject.
        assert_eq!(
            observation.evidence().fact("WindowFocusGained").unwrap().value(),
            &evo_observation::evidence::FactValue::Text("editor-window".into())
        );
        assert_eq!(observation.evidence().facts().len(), 1);
    }

    #[test]
    fn adapter_normalizes_input_activity_with_document_grain_subject() {
        // OBS-INPUT-ACTIVITY/v1: counters on a subject. The subject here is
        // document-grain (what the focus source resolved); the counts are
        // the whole evidence, and the acceptance pipeline validates them.
        let source = ObservationSource::new("macos_adapter").unwrap();
        let adapter = MacOSAdapter::new(source);

        let raw_event = adapter
            .normalize(MacOSSignal::InputActivity {
                subject: Some("file:///Users/p/evo/src/main.rs".into()),
                keys: 61,
                clicks: 4,
                scrolls: 12,
                observed_at: SystemTime::UNIX_EPOCH,
            })
            .unwrap()
            .expect("canonical input signal");
        assert_eq!(raw_event.schema(), ObservationSchema::input_activity_v1());
        assert_eq!(
            raw_event.evidence().fact("InputActivity").unwrap().value(),
            &evo_observation::evidence::FactValue::Text("file:///Users/p/evo/src/main.rs".into())
        );
        assert_eq!(
            raw_event.evidence().fact("Keys").unwrap().value(),
            &evo_observation::evidence::FactValue::Integer(61)
        );
        let observation = accept(
            raw_event.into_candidate_observation(),
            &ObservationSchema::input_activity_v1(),
        )
        .unwrap();
        assert_eq!(observation.schema(), &ObservationSchema::input_activity_v1());
    }

    #[test]
    fn adapter_drops_input_activity_with_no_attributable_surface() {
        // A bucket that cannot be attributed to any surface carries nothing
        // the language can express; fabricating a subject would mint
        // identity from nothing.
        let source = ObservationSource::new("macos_adapter").unwrap();
        let adapter = MacOSAdapter::new(source);

        assert!(adapter
            .normalize(MacOSSignal::InputActivity {
                subject: None,
                keys: 30,
                clicks: 0,
                scrolls: 0,
                observed_at: SystemTime::UNIX_EPOCH,
            })
            .unwrap()
            .is_none());
        assert!(adapter
            .normalize(MacOSSignal::InputActivity {
                subject: Some("   ".into()),
                keys: 30,
                clicks: 0,
                scrolls: 0,
                observed_at: SystemTime::UNIX_EPOCH,
            })
            .unwrap()
            .is_none());
    }

    #[test]
    fn adapter_prefers_document_locator_for_focus_subject() {
        // Document-grain identity: when the focus observation carries a
        // document locator in its observed state, the engine-facing subject
        // is the document, not the window title. The title survives as
        // provenance context. This is the resolution rule the threads
        // engine's resource identity depends on: one document, one resource,
        // regardless of which window showed it.
        let raw_event = MacOSAdapter::resolve_focus_subject(
            "main.rs — evo — Visual Studio Code",
            Some("file:///Users/p/evo/src/main.rs".to_string()),
        );
        assert_eq!(raw_event, "file:///Users/p/evo/src/main.rs");

        // No locator: the title stands, honestly coarse.
        let title_only = MacOSAdapter::resolve_focus_subject("Terminal", None);
        assert_eq!(title_only, "Terminal");
    }

    #[test]
    fn adapter_does_not_promote_application_activation() {
        let source = ObservationSource::new("macos_adapter").unwrap();
        let adapter = MacOSAdapter::new(source.clone());

        let event = adapter
            .normalize(MacOSSignal::ApplicationActivated {
                bundle_identifier: Some("com.apple.dt.Xcode".into()),
                application_name: Some("Xcode".into()),
                process_identifier: Some(4321),
                observed_at: SystemTime::UNIX_EPOCH,
            })
            .unwrap();

        assert!(event.is_none());
        let _ = source;
    }

    #[test]
    fn adapter_normalizes_co_membership_signals_with_matching_schemas() {
        let source = ObservationSource::new("macos_adapter").unwrap();
        let adapter = MacOSAdapter::new(source);

        // OBS-REPOSITORY-MEMBERSHIP/v1 (RFC-0012): member + repository.
        let membership = adapter
            .normalize(MacOSSignal::RepositoryMembership {
                member: "/Users/alice/evo/src/a.rs".into(),
                repository: "/Users/alice/evo/.git".into(),
                observed_at: SystemTime::UNIX_EPOCH,
            })
            .unwrap()
            .expect("canonical membership signal");
        assert_eq!(membership.schema(), ObservationSchema::repository_membership_v1());
        assert_eq!(
            membership.evidence().fact("RepositoryMembership").unwrap().value(),
            &evo_observation::evidence::FactValue::Text("/Users/alice/evo/src/a.rs".into())
        );
        assert_eq!(
            membership.evidence().fact("Repository").unwrap().value(),
            &evo_observation::evidence::FactValue::Text("/Users/alice/evo/.git".into())
        );
        let observation =
            accept(membership.into_candidate_observation(), &ObservationSchema::repository_membership_v1())
                .unwrap();
        assert_eq!(observation.schema(), &ObservationSchema::repository_membership_v1());

        // OBS-WORK-GROUPED/v1 (RFC-0012): declared pair.
        let grouped = adapter
            .normalize(MacOSSignal::WorkGrouped {
                first: "/Users/alice/plan.md".into(),
                second: "https://example.com/research".into(),
                observed_at: SystemTime::UNIX_EPOCH,
            })
            .unwrap()
            .expect("canonical grouped signal");
        assert_eq!(grouped.schema(), ObservationSchema::work_grouped_v1());
        assert_eq!(
            grouped.evidence().fact("WorkGrouped").unwrap().value(),
            &evo_observation::evidence::FactValue::Text("/Users/alice/plan.md".into())
        );
        assert_eq!(
            grouped.evidence().fact("CoMember").unwrap().value(),
            &evo_observation::evidence::FactValue::Text("https://example.com/research".into())
        );
        let observation = accept(
            grouped.into_candidate_observation(),
            &ObservationSchema::work_grouped_v1(),
        )
        .unwrap();
        assert_eq!(observation.schema(), &ObservationSchema::work_grouped_v1());
    }

    #[test]
    fn adapter_normalizes_all_canonical_signals_with_matching_schemas() {
        let source = ObservationSource::new("macos_adapter").unwrap();
        let adapter = MacOSAdapter::new(source);
        let cases = [
            (
                MacOSSignal::WindowFocusGained {
                    subject: "editor-window".into(),
                    observed_at: SystemTime::UNIX_EPOCH,
                    process_identifier: None,
                    owning_process_name: None,
                    observed_state: None,
                },
                ObservationSchema::window_focus_gained_v3(),
                "WindowFocusGained",
                "editor-window",
            ),
            (
                MacOSSignal::FileSaved {
                    subject: "/tmp/report.md".into(),
                    observed_at: SystemTime::UNIX_EPOCH,
                },
                ObservationSchema::file_saved_v1(),
                "FileSaved",
                "/tmp/report.md",
            ),
            (
                MacOSSignal::URLNavigated {
                    subject: "https://example.com".into(),
                    observed_at: SystemTime::UNIX_EPOCH,
                },
                ObservationSchema::url_navigated_v1(),
                "URLNavigated",
                "https://example.com",
            ),
            (
                MacOSSignal::CommitMade {
                    subject: "abc123".into(),
                    observed_at: SystemTime::UNIX_EPOCH,
                },
                ObservationSchema::commit_made_v1(),
                "CommitMade",
                "abc123",
            ),
        ];

        for (signal, schema, fact_name, subject) in cases {
            let raw_event = adapter.normalize(signal).unwrap().expect("canonical signal");
            assert_eq!(raw_event.schema(), schema);
            assert_eq!(raw_event.evidence().fact(fact_name).unwrap().value(), &evo_observation::evidence::FactValue::Text(subject.into()));
            let observation = accept(raw_event.into_candidate_observation(), &schema).unwrap();
            assert_eq!(observation.schema(), &schema);
        }
    }
}
