//! Canonical Observation Language concepts.
//!
//! These are the frozen platform-independent concepts that the Collector may
//! translate into before a Candidate Observation enters Evo.

use crate::evidence::{Evidence, FactValue, ObservedFact};
use crate::observation_schema::ObservationSchema;

/// The frozen canonical Observation Language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationConcept {
    WindowFocusGained { subject: String },
    FileSaved { subject: String },
    URLNavigated { subject: String },
    CommitMade { subject: String },
    WorkDesignated { subject: String },
    /// The directly witnessed fact that entity `member` is a member of
    /// repository `repository` (RFC-0012, OBS-REPOSITORY-MEMBERSHIP/v1).
    RepositoryMembership {
        member: String,
        repository: String,
    },
    /// The directly witnessed declaration that subjects `first` and `second`
    /// are related work, canonically ordered (RFC-0012, OBS-WORK-GROUPED/v1).
    WorkGrouped { first: String, second: String },
    /// Directly witnessed input activity on a subject, as content-free
    /// counters aggregated over a fixed flush bucket (OBS-INPUT-ACTIVITY/v1).
    ///
    /// The subject is the surface the input landed on (document-grain where
    /// known, window-grain otherwise). The counts are aggregates only: what
    /// was typed, clicked, or scrolled is never observed, never recorded,
    /// and cannot be reconstructed from this observation. A count of zero is
    /// not reported — absence of the observation is the zero.
    InputActivity {
        subject: String,
        keys: u64,
        clicks: u64,
        scrolls: u64,
    },
    /// The directly witnessed user declaration that a set of already-witnessed
    /// subjects constitutes the current continuation of the user's work
    /// (RFC-0013, OBS-CONTINUATION-SURFACE/v1). The set is canonically ordered:
    /// the lexicographically smallest subject is the Required Subject; the
    /// remainder follow in ascending order.
    ContinuationSurface { subjects: Vec<String> },
}

impl ObservationConcept {
    /// Returns the canonical concept name.
    pub fn name(&self) -> &'static str {
        match self {
            ObservationConcept::WindowFocusGained { .. } => "WindowFocusGained",
            ObservationConcept::FileSaved { .. } => "FileSaved",
            ObservationConcept::URLNavigated { .. } => "URLNavigated",
            ObservationConcept::CommitMade { .. } => "CommitMade",
            ObservationConcept::WorkDesignated { .. } => "WorkDesignated",
            ObservationConcept::RepositoryMembership { .. } => "RepositoryMembership",
            ObservationConcept::WorkGrouped { .. } => "WorkGrouped",
            ObservationConcept::ContinuationSurface { .. } => "ContinuationSurface",
            ObservationConcept::InputActivity { .. } => "InputActivity",
        }
    }

    /// Returns the directly observed primary subject.
    pub fn subject(&self) -> &str {
        match self {
            ObservationConcept::WindowFocusGained { subject }
            | ObservationConcept::FileSaved { subject }
            | ObservationConcept::URLNavigated { subject }
            | ObservationConcept::CommitMade { subject }
            | ObservationConcept::WorkDesignated { subject } => subject,
            ObservationConcept::RepositoryMembership { member, .. } => member,
            ObservationConcept::WorkGrouped { first, .. } => first,
            ObservationConcept::InputActivity { subject, .. } => subject,
            ObservationConcept::ContinuationSurface { subjects } => {
                subjects.iter().min().expect("a valid surface names at least two subjects")
            }
        }
    }

    /// Returns the frozen schema associated with this concept.
    pub fn schema(&self) -> ObservationSchema {
        match self {
            ObservationConcept::WindowFocusGained { .. } => {
                // v2: the witnessing contract now carries owning-process
                // provenance in `Provenance::context` when the collector
                // observed it (BE-TRACE-0001 §2.1). The Evidence structure is
                // unchanged.
                ObservationSchema::window_focus_gained_v3()
            }
            ObservationConcept::FileSaved { .. } => ObservationSchema::file_saved_v1(),
            ObservationConcept::URLNavigated { .. } => ObservationSchema::url_navigated_v1(),
            ObservationConcept::CommitMade { .. } => ObservationSchema::commit_made_v1(),
            ObservationConcept::WorkDesignated { .. } => ObservationSchema::work_designated_v1(),
            ObservationConcept::RepositoryMembership { .. } => {
                ObservationSchema::repository_membership_v1()
            }
            ObservationConcept::WorkGrouped { .. } => ObservationSchema::work_grouped_v1(),
            ObservationConcept::ContinuationSurface { .. } => {
                ObservationSchema::continuation_surface_v1()
            }
            ObservationConcept::InputActivity { .. } => ObservationSchema::input_activity_v1(),
        }
    }

    /// Builds the canonical Evidence body for this concept.
    ///
    /// The co-membership concepts (RFC-0012) carry a second subject-valued
    /// fact: the repository identity of a member, and the co-member of a
    /// declared pair.
    pub fn evidence(&self) -> Evidence {
        match self {
            ObservationConcept::RepositoryMembership { member, repository } => Evidence::new(vec![
                ObservedFact::new("RepositoryMembership", FactValue::Text(member.clone()))
                    .expect("canonical concept names are always valid"),
                ObservedFact::new("Repository", FactValue::Text(repository.clone()))
                    .expect("canonical concept names are always valid"),
            ]),
            ObservationConcept::WorkGrouped { first, second } => Evidence::new(vec![
                ObservedFact::new("WorkGrouped", FactValue::Text(first.clone()))
                    .expect("canonical concept names are always valid"),
                ObservedFact::new("CoMember", FactValue::Text(second.clone()))
                    .expect("canonical concept names are always valid"),
            ]),
            ObservationConcept::InputActivity { subject, keys, clicks, scrolls } => {
                // The counters are aggregates over the flush bucket; the
                // subject is the surface they landed on. Nothing about the
                // *content* of any keystroke, click, or scroll exists here
                // or anywhere downstream — the count is the whole fact.
                Evidence::new(vec![
                    ObservedFact::new("InputActivity", FactValue::Text(subject.clone()))
                        .expect("canonical concept names are always valid"),
                    ObservedFact::new("Keys", FactValue::Integer(*keys as i64))
                        .expect("canonical concept names are always valid"),
                    ObservedFact::new("Clicks", FactValue::Integer(*clicks as i64))
                        .expect("canonical concept names are always valid"),
                    ObservedFact::new("Scrolls", FactValue::Integer(*scrolls as i64))
                        .expect("canonical concept names are always valid"),
                ])
            }
            ObservationConcept::ContinuationSurface { subjects } => {
                // Canonical ordering rule (RFC-0013): the lexicographically
                // smallest subject is the Required Subject (the
                // `ContinuationSurface` fact); every other subject follows as
                // a `ContinuationSubject` fact in ascending order. A
                // declaration expressed in any order therefore produces the
                // identical canonical Observation.
                let mut ordered: Vec<String> = subjects.clone();
                ordered.sort();
                let mut facts = Vec::with_capacity(ordered.len());
                let mut iterator = ordered.into_iter();
                let required = iterator
                    .next()
                    .expect("a valid surface names at least two subjects");
                facts.push(
                    ObservedFact::new("ContinuationSurface", FactValue::Text(required))
                        .expect("canonical concept names are always valid"),
                );
                for subject in iterator {
                    facts.push(
                        ObservedFact::new("ContinuationSubject", FactValue::Text(subject))
                            .expect("canonical concept names are always valid"),
                    );
                }
                Evidence::new(facts)
            }
            _ => {
                let fact = ObservedFact::new(
                    self.name(),
                    FactValue::Text(self.subject().to_string()),
                )
                .expect("canonical observation concept names are always valid");
                Evidence::new(vec![fact])
            }
        }
    }
}
