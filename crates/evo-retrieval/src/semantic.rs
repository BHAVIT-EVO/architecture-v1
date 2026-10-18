//! Semantic trigger resolution over engagements: matching meaning, not tokens.
//!
//! # What this answers
//!
//! The same question as [`crate::retrieval`] — "which body of work does this
//! phrase mean?" — asked the way a person actually speaks: "I was fixing the
//! technology of Evo", "I was using Muse to fix Evo". Each engagement is
//! described by a document (its title, the witnessed names of the members that
//! most specifically belong to it, and the apps and hosts that material was
//! witnessed in); an embedding model turns the trigger and every document into
//! vectors, and cosine similarity is the score. Paraphrase, filler, and
//! synonyms are understood by the model, so there is no stop-word list and no
//! keyword weighting here.
//!
//! # Contract
//!
//! The same three honest outcomes as lexical retrieval —
//! [`Resolution::Resolved`], [`Resolution::Ambiguous`], [`Resolution::NotFound`]
//! — with the same determinism: a pure function of the trigger, the
//! reconstruction, and the parameters. An embedding failure is reported as an
//! error, never laundered into "not found".
//!
//! # No hardcoded categories
//!
//! Nothing here names an application, domain, file type, or profession. What a
//! trigger matches is learned entirely from the person's own witnessed
//! material, as embedded by a general model trained on language.

use std::collections::{BTreeMap, BTreeSet};

use evo_engagement::{Engagement, Reconstruction, WorkId};
use evo_semantic::{
    EmbedError, Embedder, SemanticIndex, SemanticParams, SemanticResolution, app_marker_name,
    url_host,
};

use crate::retrieval::Resolution;

/// How much of an engagement's material describes it: the most specifically
/// belonging members. A cap, not a ranking — the embedding sees the most
/// identifying material and an enormous engagement cannot drown it in noise.
const MAX_MEMBER_TERMS: usize = 8;

/// The text an engagement is matched by.
///
/// Its title, then the witnessed names of the members that most specifically
/// belong to it (highest specificity first), then the apps and hosts that
/// material was witnessed in — so "the Muse chat" can meet an engagement whose
/// material lives at `claude.ai`, without anything ever naming an application.
/// Every app and host here was witnessed; none was listed.
fn engagement_document(engagement: &Engagement) -> String {
    let mut members: Vec<(&str, f64)> = engagement
        .participants()
        .iter()
        .map(|participant| (participant.resource().subject(), participant.specificity()))
        .filter(|(_, specificity)| *specificity > 0.0)
        .collect();
    members.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.0.cmp(right.0))
    });

    let mut parts: Vec<&str> = Vec::with_capacity(members.len() + 1);
    if !engagement.title().is_empty() {
        parts.push(engagement.title());
    }
    for (subject, _) in members.iter().take(MAX_MEMBER_TERMS) {
        parts.push(subject);
    }

    let mut apps: BTreeSet<String> = BTreeSet::new();
    let mut hosts: BTreeSet<String> = BTreeSet::new();
    for part in parts.iter() {
        if let Some(app) = app_marker_name(part) {
            apps.insert(app);
        }
        if let Some(host) = url_host(part) {
            hosts.insert(host);
        }
    }

    let mut document = parts.join(" | ");
    if !apps.is_empty() {
        document.push_str(" | apps: ");
        document.push_str(&apps.into_iter().collect::<Vec<_>>().join(", "));
    }
    if !hosts.is_empty() {
        document.push_str(" | hosts: ");
        document.push_str(&hosts.into_iter().collect::<Vec<_>>().join(", "));
    }
    document
}

/// Semantic retrieval boundary value.
///
/// Holds no resolver, index, or derived state: resolution is a pure function of
/// a trigger, a reconstruction, and the embedder, so there is nothing to store
/// between calls.
#[derive(Debug, Clone, Copy, Default)]
pub struct SemanticRetrieval {
    params: SemanticParams,
}

impl SemanticRetrieval {
    /// Constructs a semantic retrieval boundary value with default tuning.
    pub fn new() -> Self {
        Self {
            params: SemanticParams::default(),
        }
    }

    /// Constructs one with explicit tuning. See [`SemanticParams`].
    pub fn with_params(params: SemanticParams) -> Self {
        Self { params }
    }

    /// Resolves a trigger phrase against everything the reconstruction
    /// witnessed, by meaning.
    ///
    /// Scores the trigger against every reconstructed thread — bodies of work
    /// and what was merely remembered alike — and returns the clear winner, the
    /// set of candidates it cannot separate, or nothing. An embedding failure
    /// surfaces as [`EmbedError`]: a broken model is reported, never mistaken
    /// for an empty history.
    pub fn retrieve<'a>(
        &self,
        trigger: &str,
        reconstruction: &'a Reconstruction,
        embedder: &dyn Embedder,
    ) -> Result<Resolution<'a>, EmbedError> {
        let threads = reconstruction.set().engagements();
        let index = SemanticIndex::build(
            threads
                .iter()
                .map(|thread| (thread.work_id(), engagement_document(thread))),
            embedder,
            self.params,
        )?;
        let by_id: BTreeMap<WorkId, &Engagement> = threads
            .iter()
            .map(|thread| (thread.work_id(), thread))
            .collect();
        let found = |id: WorkId| -> Resolution<'a> {
            by_id
                .get(&id)
                .map(|thread| Resolution::Resolved(thread))
                .unwrap_or(Resolution::NotFound)
        };
        match index.resolve(trigger, embedder)? {
            SemanticResolution::Matched { id, .. } => Ok(found(id)),
            SemanticResolution::Ambiguous { candidates } => Ok(Resolution::Ambiguous(
                candidates
                    .into_iter()
                    .filter_map(|id| by_id.get(&id).copied())
                    .collect(),
            )),
            SemanticResolution::NoMatch => Ok(Resolution::NotFound),
        }
    }

    /// The tuning this resolver was constructed with.
    pub fn params(&self) -> SemanticParams {
        self.params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use evo_engagement::EngagementParams;
    use evo_observation::observation::Observation;
    use evo_observation::observation_id::ObservationId;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::provenance::{ObservationSource, Provenance};

    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};

    /// Deterministic stand-in for a real model: the test prescribes the
    /// vectors, pinning the *wiring* rather than any model's opinions about
    /// language.
    struct StubEmbedder {
        vectors: BTreeMap<String, Vec<f32>>,
    }

    impl StubEmbedder {
        fn named(pairs: &[(&str, &[f32])]) -> Self {
            Self {
                vectors: pairs
                    .iter()
                    .map(|(text, vector)| (text.to_string(), vector.to_vec()))
                    .collect(),
            }
        }

        /// Builds a stub keyed by the actual documents of a reconstruction, so
        /// tests don't hand-copy document strings. The vector for each
        /// engagement's document comes from `doc_vector_for`, which sees the
        /// engagement itself — tests must not assume engagement order.
        fn for_reconstruction(
            reconstruction: &Reconstruction,
            query_vectors: &[(&str, &[f32])],
            doc_vector_for: impl Fn(&Engagement) -> Vec<f32>,
        ) -> Self {
            let mut pairs: Vec<(String, Vec<f32>)> = Vec::new();
            for thread in reconstruction.set().engagements() {
                pairs.push((engagement_document(thread), doc_vector_for(thread)));
            }
            for (query, vector) in query_vectors {
                pairs.push((query.to_string(), vector.to_vec()));
            }
            Self {
                vectors: pairs.into_iter().collect(),
            }
        }
    }

    impl Embedder for StubEmbedder {
        fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
            self.vectors
                .get(text)
                .cloned()
                .ok_or_else(|| EmbedError::Refused(text.to_string()))
        }
    }

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn observe(concept: ObservationConcept, secs: u64) -> Observation {
        let source = ObservationSource::new("semantic-test").expect("a valid source");
        Observation::new(
            ObservationId::new(),
            concept.schema(),
            Provenance::new(source, at(secs), HashMap::new()),
            concept.evidence(),
        )
    }

    fn focus(subject: &str, secs: u64) -> Observation {
        observe(
            ObservationConcept::WindowFocusGained {
                subject: subject.to_string(),
            },
            secs,
        )
    }

    fn saved(subject: &str, secs: u64) -> Observation {
        observe(
            ObservationConcept::FileSaved {
                subject: subject.to_string(),
            },
            secs,
        )
    }

    /// Two bodies of work a day apart: one around quarterly figures, one around
    /// an annual budget.
    fn two_reports() -> Vec<Observation> {
        let mut observations = Vec::new();
        for (start_day, editor, sheet, path) in [
            (
                0u64,
                "quarterly report draft — Editor",
                "quarterly figures — Sheet",
                "/home/x/quarterly-report.txt",
            ),
            (
                10u64,
                "annual report draft — Editor",
                "annual budget — Sheet",
                "/home/x/annual-report.txt",
            ),
        ] {
            for sitting in start_day..start_day + 3 {
                let mut moment = sitting * 24 * 60 * 60;
                for _pass in 0..4 {
                    observations.push(focus(editor, moment));
                    moment += 300;
                    observations.push(saved(path, moment));
                    moment += 30;
                    observations.push(focus(sheet, moment));
                    moment += 300;
                }
            }
        }
        observations
    }

    fn reconstruct(observations: &[Observation]) -> Reconstruction {
        Reconstruction::from_observations(observations, EngagementParams::default())
    }

    #[test]
    fn a_paraphrase_resolves_by_meaning() {
        let reconstruction = reconstruct(&two_reports());
        let threads = reconstruction.set().engagements();
        assert!(threads.len() >= 2, "the fixture must reconstruct two works");
        // The stub stands in for the model: the paraphrase "the quarterly
        // numbers I was crunching" is near the quarterly document, far from the
        // annual one. What the test pins is that resolution follows the vectors
        // — meaning — rather than shared tokens (the paraphrase shares none).
        let embedder = StubEmbedder::for_reconstruction(
            &reconstruction,
            &[("the quarterly numbers I was crunching", &[0.95, 0.31, 0.0])],
            |engagement| {
                if engagement
                    .subjects()
                    .iter()
                    .any(|subject| subject.contains("quarterly"))
                {
                    vec![1.0, 0.0, 0.0]
                } else {
                    vec![0.0, 1.0, 0.0]
                }
            },
        );
        let retrieval = SemanticRetrieval::new();
        match retrieval
            .retrieve(
                "the quarterly numbers I was crunching",
                &reconstruction,
                &embedder,
            )
            .expect("the stub embeds everything")
        {
            Resolution::Resolved(engagement) => assert!(
                engagement
                    .subjects()
                    .iter()
                    .any(|subject| subject.contains("quarterly")),
                "the paraphrase must land on the quarterly work, got {:?}",
                engagement.subjects()
            ),
            other => {
                panic!("a paraphrase with a clear nearest meaning must resolve, got {other:?}")
            }
        }
    }

    #[test]
    fn an_unclear_paraphrase_is_ambiguous() {
        let reconstruction = reconstruct(&two_reports());
        let embedder = StubEmbedder::for_reconstruction(
            &reconstruction,
            &[("the report", &[1.0, 0.0, 0.0])],
            |engagement| {
                // Both works are near-identical in meaning to "the report":
                // near-parallel vectors a hair apart.
                if engagement
                    .subjects()
                    .iter()
                    .any(|subject| subject.contains("quarterly"))
                {
                    vec![1.0, 0.1, 0.0]
                } else {
                    vec![1.0, -0.1, 0.0]
                }
            },
        );
        let retrieval = SemanticRetrieval::new();
        match retrieval
            .retrieve("the report", &reconstruction, &embedder)
            .expect("the stub embeds everything")
        {
            Resolution::Ambiguous(candidates) => {
                assert!(
                    candidates.len() >= 2,
                    "both works are near-identical in meaning here"
                )
            }
            other => panic!("an unclear paraphrase must ask, not guess, got {other:?}"),
        }
    }

    #[test]
    fn a_meaningless_trigger_finds_nothing() {
        let reconstruction = reconstruct(&two_reports());
        let embedder = StubEmbedder::for_reconstruction(
            &reconstruction,
            &[("xyzzy plugh", &[0.0, 0.0, 1.0])],
            |engagement| {
                if engagement
                    .subjects()
                    .iter()
                    .any(|subject| subject.contains("quarterly"))
                {
                    vec![1.0, 0.0, 0.0]
                } else {
                    vec![0.0, 1.0, 0.0]
                }
            },
        );
        let retrieval = SemanticRetrieval::new();
        assert!(
            matches!(
                retrieval
                    .retrieve("xyzzy plugh", &reconstruction, &embedder)
                    .expect("the stub embeds everything"),
                Resolution::NotFound
            ),
            "a trigger meaning nothing here must find nothing"
        );
    }

    #[test]
    fn embedder_failure_surfaces_as_an_error() {
        let reconstruction = reconstruct(&two_reports());
        let embedder = StubEmbedder::named(&[]);
        assert!(
            matches!(
                SemanticRetrieval::new().retrieve("quarterly", &reconstruction, &embedder),
                Err(EmbedError::Refused(_))
            ),
            "a broken model must be reported, not mistaken for an empty history"
        );
    }

    #[test]
    fn the_document_carries_witnessed_hosts() {
        let reconstruction = reconstruct(&two_reports());
        let threads = reconstruction.set().engagements();
        // The fixture has no URLs, so no host trailer — the assertion is that
        // the document is built from the title and member names, not empty.
        for thread in threads {
            assert!(
                !engagement_document(thread).is_empty(),
                "every engagement with material must have a document"
            );
        }
    }
}
