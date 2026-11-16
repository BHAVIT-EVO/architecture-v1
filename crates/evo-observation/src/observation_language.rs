//! Canonical Observation Language concepts.
//!
//! These are the frozen platform-independent concepts that the Collector may
//! translate into before a Candidate Observation enters Evo.

use crate::evidence::{Evidence, FactValue, ObservedFact};
use crate::observation_schema::ObservationSchema;

/// The frozen canonical Observation Language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationConcept {
    WindowFocusGained {
        subject: String,
    },
    FileSaved {
        subject: String,
    },
    URLNavigated {
        subject: String,
    },
    CommitMade {
        subject: String,
    },
    WorkDesignated {
        subject: String,
    },
    /// The directly witnessed fact that entity `member` is a member of
    /// repository `repository` (RFC-0012, OBS-REPOSITORY-MEMBERSHIP/v1).
    RepositoryMembership {
        member: String,
        repository: String,
    },
    /// The directly witnessed declaration that subjects `first` and `second`
    /// are related work, canonically ordered (RFC-0012, OBS-WORK-GROUPED/v1).
    WorkGrouped {
        first: String,
        second: String,
    },
    /// The directly witnessed act of the person naming a body of work
    /// (RFC-0014, OBS-WORK-NAMED/v1): `subject` is the canonical Workspace
    /// identity the name belongs to and `name` is exactly what the person
    /// wrote. A person's name for their own work is ground truth over every
    /// derived title (Law IX); it references the Workspace other
    /// Observations established and never mints one.
    WorkNamed {
        subject: String,
        name: String,
    },
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
    ContinuationSurface {
        subjects: Vec<String>,
    },
    /// The name a surface presented of itself (OBS-SURFACE-NAMED/v1): the
    /// directly witnessed page title of a web document, the document title of
    /// an editor, the window title of any focused surface. The subject is the
    /// surface the name belongs to — the URL when the surface was witnessed as
    /// a web document, the file path when it was witnessed as a document, the
    /// window title itself otherwise — and the name is exactly what the
    /// surface displayed, never truncated, never cleaned, never guessed. A
    /// name is presentation, not identity: it decorates and never votes.
    SurfaceNamed {
        subject: String,
        name: String,
    },
    /// The directly witnessed user declaration that a surface is of a class
    /// (OBS-SURFACE-CLASSIFIED/v1): the person's own word — "this is not
    /// work", "this is media" — about a subject other Observations have
    /// already established. `subject` is that established subject; `class` is
    /// exactly the class the person assigned. Evo ships no default class:
    /// silence is the default, and every classification here has the person's
    /// name on it. A classification gates founding and titling only — it never
    /// removes the subject from a body of work (the non-deletion guarantee
    /// holds); it only prevents it from anchoring or naming one. This
    /// references subjects other Observations established and never mints an
    /// Artifact.
    SurfaceClassified {
        subject: String,
        class: String,
    },
    /// The directly witnessed user declaration that a body of work should be
    /// dismissed from the default surface (Phase 5). The subject is a
    /// witnessed member that identifies the work; dismissal never deletes the
    /// work or its evidence.
    WorkDismissed {
        work: String,
    },
    /// The directly witnessed user declaration of one member's lifecycle
    /// state within a body of work (Phase 5/6). `work` and `member` are
    /// witnessed subjects, while `state` is the person's exact label.
    MemberState {
        work: String,
        member: String,
        state: String,
    },
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
            ObservationConcept::WorkNamed { .. } => "WorkNamed",
            ObservationConcept::ContinuationSurface { .. } => "ContinuationSurface",
            ObservationConcept::InputActivity { .. } => "InputActivity",
            ObservationConcept::SurfaceNamed { .. } => "SurfaceNamed",
            ObservationConcept::SurfaceClassified { .. } => "SurfaceClassified",
            ObservationConcept::WorkDismissed { .. } => "WorkDismissed",
            ObservationConcept::MemberState { .. } => "MemberState",
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
            ObservationConcept::WorkNamed { subject, .. } => subject,
            ObservationConcept::InputActivity { subject, .. } => subject,
            ObservationConcept::SurfaceNamed { subject, .. } => subject,
            ObservationConcept::SurfaceClassified { subject, .. } => subject,
            ObservationConcept::WorkDismissed { work } => work,
            ObservationConcept::MemberState { work, .. } => work,
            ObservationConcept::ContinuationSurface { subjects } => subjects
                .iter()
                .min()
                .expect("a valid surface names at least two subjects"),
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
            ObservationConcept::WorkNamed { .. } => ObservationSchema::work_named_v1(),
            ObservationConcept::ContinuationSurface { .. } => {
                ObservationSchema::continuation_surface_v1()
            }
            ObservationConcept::InputActivity { .. } => ObservationSchema::input_activity_v1(),
            ObservationConcept::SurfaceNamed { .. } => ObservationSchema::surface_named_v1(),
            ObservationConcept::SurfaceClassified { .. } => {
                ObservationSchema::surface_classified_v1()
            }
            ObservationConcept::WorkDismissed { .. } => ObservationSchema::work_dismissed_v1(),
            ObservationConcept::MemberState { .. } => ObservationSchema::member_state_v1(),
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
            ObservationConcept::WorkNamed { subject, name } => Evidence::new(vec![
                ObservedFact::new("WorkNamed", FactValue::Text(subject.clone()))
                    .expect("canonical concept names are always valid"),
                ObservedFact::new("Name", FactValue::Text(name.clone()))
                    .expect("canonical concept names are always valid"),
            ]),
            ObservationConcept::InputActivity {
                subject,
                keys,
                clicks,
                scrolls,
            } => {
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
            ObservationConcept::SurfaceNamed { subject, name } => {
                // The subject is the witnessed fact (what was named); the name
                // is the value concept (what it presented). Evidence order is
                // fixed: the witnessed fact first, exactly as the co-membership
                // concepts fix theirs.
                Evidence::new(vec![
                    ObservedFact::new("SurfaceNamed", FactValue::Text(subject.clone()))
                        .expect("canonical concept names are always valid"),
                    ObservedFact::new("Name", FactValue::Text(name.clone()))
                        .expect("canonical concept names are always valid"),
                ])
            }
            ObservationConcept::SurfaceClassified { subject, class } => {
                // The subject is the witnessed fact (what was classified); the
                // class is the value concept (what the person called it).
                // Evidence order is fixed: the witnessed fact first, exactly
                // as the co-membership concepts fix theirs.
                Evidence::new(vec![
                    ObservedFact::new("SurfaceClassified", FactValue::Text(subject.clone()))
                        .expect("canonical concept names are always valid"),
                    ObservedFact::new("Class", FactValue::Text(class.clone()))
                        .expect("canonical concept names are always valid"),
                ])
            }
            ObservationConcept::WorkDismissed { work } => Evidence::new(vec![
                ObservedFact::new("WorkDismissed", FactValue::Text(work.clone()))
                    .expect("canonical concept names are always valid"),
            ]),
            ObservationConcept::MemberState {
                work,
                member,
                state,
            } => Evidence::new(vec![
                ObservedFact::new("MemberState", FactValue::Text(work.clone()))
                    .expect("canonical concept names are always valid"),
                ObservedFact::new("Member", FactValue::Text(member.clone()))
                    .expect("canonical concept names are always valid"),
                ObservedFact::new("State", FactValue::Text(state.clone()))
                    .expect("canonical concept names are always valid"),
            ]),
            _ => {
                let fact =
                    ObservedFact::new(self.name(), FactValue::Text(self.subject().to_string()))
                        .expect("canonical observation concept names are always valid");
                Evidence::new(vec![fact])
            }
        }
    }
}
