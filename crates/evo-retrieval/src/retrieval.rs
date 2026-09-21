//! Scoring a trigger phrase against reconstructed threads.
//!
//! # What this answers
//!
//! "Open the thing I was doing about the membrane osteology" — a phrase, typed
//! or spoken, that has to land on one body of work out of everything Evo has
//! ever witnessed. That includes work the person never returned to: "the PDF I
//! read once last Tuesday" is a real request and its answer is a
//! [`Standing::Remembered`](evo_engagement::Standing) thread, not a body of
//! work. So retrieval scores against *every* reconstructed thread
//! ([`EngagementSet::engagements`](evo_engagement::EngagementSet::engagements)),
//! not only the ones Home presents.
//!
//! # How it scores — WORK-MODEL §3
//!
//! A thread's vocabulary is the witnessed names of its members. Each token a
//! member contributes is weighted by that member's
//! [`specificity`](evo_engagement::Participant::specificity) in the thread, so a
//! resource that sits beside everything (a music player, a chat window) barely
//! adds to any thread's vocabulary and cannot pull a trigger toward work it was
//! only incidentally near. Across threads, each token is weighted by its inverse
//! document frequency: a token that appears in eight threads is nearly useless
//! for telling them apart, and a token that appears in one is decisive.
//!
//! ```text
//! score(thread, trigger) =
//!     Σ over trigger tokens t:  idf(t) · Σ over members m with t in name(m): specificity(m, thread)
//! ```
//!
//! Every quantity is a count, a ratio of counts, or an IDF over witnessed
//! strings — no model, no embedding, no vocabulary list. The same history scores
//! identically on every replay: iteration is over sorted collections, no clock
//! or randomness is read, and ties break on the reconstruction's own canonical
//! order.
//!
//! # What it refuses to guess
//!
//! When the best-scoring thread does not clearly beat the runner-up — the
//! runner-up is within [`EngagementParams::retrieval_margin`] of it — retrieval
//! returns [`Resolution::Ambiguous`] with the contenders rather than presenting
//! a coin-flip as a conclusion. Asking "which one?" is the honest answer, and it
//! is the same reflex as everywhere else in this system: represent the
//! uncertainty instead of destroying it.
//!
//! # No hardcoded categories
//!
//! This module contains no application name, domain, extension, profession, or
//! notion of "productive" software, and it cannot acquire one: it sees only the
//! tokens of witnessed subjects and the specificity the reconstruction measured.
//! What a trigger matches is learned entirely from the person's own history.

use std::collections::BTreeMap;

use evo_engagement::{Engagement, EngagementParams, Reconstruction, WorkId, tokenize_subject};

/// A retrieval boundary value.
///
/// Holds no resolver, index, or derived state: retrieval is a pure function of a
/// trigger and a reconstruction, so there is nothing to store between calls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Retrieval;

/// Owned, replayable retrieval material for a body of work.
#[derive(Debug, Clone, PartialEq)]
struct IndexedWork {
    id: WorkId,
    vocabulary: BTreeMap<String, f64>,
}

/// Retrieval index retained by the daemon after reconstruction.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RetrievalIndex {
    works: Vec<IndexedWork>,
    margin: f64,
}

/// Stable result suitable for API/UI boundaries that outlive Reconstruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkResolution {
    Resolved(WorkId),
    Ambiguous(Vec<WorkId>),
    NotFound,
}

/// What a trigger resolved to.
///
/// Borrows the matched threads from the reconstruction that was searched — the
/// caller already holds it, and retrieval adds no state of its own.
#[derive(Debug)]
pub enum Resolution<'a> {
    /// One thread stands clearly above the rest. Safe to act on.
    Resolved(&'a Engagement),
    /// Two or more threads the evidence cannot separate, best-scoring first.
    /// The caller should ask *which one* rather than pick.
    Ambiguous(Vec<&'a Engagement>),
    /// No thread's vocabulary matched the trigger at all.
    NotFound,
}

impl Resolution<'_> {
    /// Whether the trigger landed on exactly one body of work.
    pub fn is_resolved(&self) -> bool {
        matches!(self, Resolution::Resolved(_))
    }

    /// Stable identity of the resolved body of work, when unambiguous.
    pub fn work_id(&self) -> Option<WorkId> {
        match self { Resolution::Resolved(work) => Some(work.work_id()), _ => None }
    }
}

impl Retrieval {
    /// Constructs a retrieval boundary value.
    pub fn new() -> Self {
        Self
    }

    /// Resolves a trigger phrase against everything the reconstruction witnessed.
    ///
    /// Scores the trigger against every reconstructed thread — bodies of work and
    /// what was merely remembered alike — and returns the clear winner, the set
    /// of candidates it cannot separate, or nothing. See the module documentation
    /// for the scoring and [`Resolution`] for the outcomes.
    pub fn retrieve<'a>(&self, trigger: &str, reconstruction: &'a Reconstruction) -> Resolution<'a> {
        resolve(
            trigger,
            reconstruction.set().engagements(),
            reconstruction.params(),
        )
    }

    /// Resolves a trigger and returns the stable body-of-work identity when the
    /// evidence clearly separates one candidate from the others.
    pub fn retrieve_work_id(&self, trigger: &str, reconstruction: &Reconstruction) -> Option<WorkId> {
        self.retrieve(trigger, reconstruction).work_id()
    }
}

impl RetrievalIndex {
    pub fn from_reconstruction(reconstruction: &Reconstruction) -> Self {
        let works = reconstruction
            .set()
            .engagements()
            .iter()
            .map(|work| IndexedWork { id: work.work_id(), vocabulary: thread_vocabulary(work) })
            .collect();
        Self { works, margin: reconstruction.params().retrieval_margin }
    }

    pub fn resolve(&self, trigger: &str) -> WorkResolution {
        let query = tokenize_subject(trigger);
        if query.is_empty() || self.works.is_empty() { return WorkResolution::NotFound; }

        let mut document_frequency: BTreeMap<&str, usize> = BTreeMap::new();
        for work in &self.works {
            for token in work.vocabulary.keys() {
                *document_frequency.entry(token.as_str()).or_insert(0) += 1;
            }
        }
        let total = self.works.len() as f64;
        let mut scored: Vec<(WorkId, f64)> = self.works.iter().filter_map(|work| {
            let score = query.iter().filter_map(|term| {
                let weight = work.vocabulary.get(term)?;
                let df = *document_frequency.get(term.as_str())?;
                Some(((total + 1.0) / df as f64).ln() * weight)
            }).sum::<f64>();
            (score > 0.0).then_some((work.id, score))
        }).collect();
        if scored.is_empty() { return WorkResolution::NotFound; }
        scored.sort_by(|left, right| right.1.partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal).then_with(|| left.0.cmp(&right.0)));
        let best = scored[0].1;
        let contenders: Vec<WorkId> = scored.iter()
            .filter(|(_, score)| *score > self.margin * best)
            .map(|(id, _)| *id).collect();
        if contenders.len() == 1 { WorkResolution::Resolved(contenders[0]) }
        else { WorkResolution::Ambiguous(contenders) }
    }
}

/// The vocabulary of one thread: each token in a member's witnessed name,
/// weighted by the sum of the specificities of the members that carry it.
///
/// A member that belongs mostly to other work contributes little of its
/// vocabulary here, which is what stops a ubiquitous resource from lending its
/// tokens to every thread it was ever beside.
fn thread_vocabulary(thread: &Engagement) -> BTreeMap<String, f64> {
    let mut vocabulary: BTreeMap<String, f64> = BTreeMap::new();
    for participant in thread.participants() {
        let specificity = participant.specificity();
        if specificity <= 0.0 {
            continue;
        }
        for token in participant.resource().tokens() {
            *vocabulary.entry(token).or_insert(0.0) += specificity;
        }
    }
    vocabulary
}

/// The pure scorer behind [`Retrieval::retrieve`].
///
/// Kept separate from the boundary type so the whole of retrieval is a function
/// of its inputs: the trigger, the threads, and the margin.
fn resolve<'a>(
    trigger: &str,
    threads: &'a [Engagement],
    params: &EngagementParams,
) -> Resolution<'a> {
    let query = tokenize_subject(trigger);
    if query.is_empty() || threads.is_empty() {
        return Resolution::NotFound;
    }

    let vocabularies: Vec<BTreeMap<String, f64>> = threads.iter().map(thread_vocabulary).collect();

    // Document frequency: in how many threads each token appears at all. A token
    // present only through a low-specificity member still counts here — that is
    // exactly why a ubiquitous term ends up with almost no IDF.
    let mut document_frequency: BTreeMap<&str, usize> = BTreeMap::new();
    for vocabulary in &vocabularies {
        for token in vocabulary.keys() {
            *document_frequency.entry(token.as_str()).or_insert(0) += 1;
        }
    }
    let total = threads.len() as f64;

    // Score each thread. Smoothed IDF — ln((N + 1) / df) — so a token in every
    // thread still carries a little and, crucially, a single-thread history does
    // not collapse every score to zero (ln(1) == 0).
    let mut scored: Vec<(usize, f64)> = Vec::new();
    for (index, vocabulary) in vocabularies.iter().enumerate() {
        let mut score = 0.0;
        for term in &query {
            let Some(weight) = vocabulary.get(term) else {
                continue;
            };
            let Some(&df) = document_frequency.get(term.as_str()) else {
                continue;
            };
            let idf = ((total + 1.0) / df as f64).ln();
            score += idf * weight;
        }
        if score > 0.0 {
            scored.push((index, score));
        }
    }

    if scored.is_empty() {
        return Resolution::NotFound;
    }

    // Best first; equal scores break on the reconstruction's own canonical order
    // (index ascending), so the outcome is deterministic across replays.
    scored.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.0.cmp(&right.0))
    });

    let best = scored[0].1;
    // Every thread the evidence cannot separate from the best: those within the
    // margin of it. The best is always here (margin < 1). If nothing else is,
    // the winner is clear.
    let contenders: Vec<&Engagement> = scored
        .iter()
        .filter(|(_, score)| *score > params.retrieval_margin * best)
        .map(|(index, _)| &threads[*index])
        .collect();

    if contenders.len() <= 1 {
        Resolution::Resolved(&threads[scored[0].0])
    } else {
        Resolution::Ambiguous(contenders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use evo_engagement::{EngagementParams, Reconstruction};
    use evo_observation::observation::Observation;
    use evo_observation::observation_id::ObservationId;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::provenance::{ObservationSource, Provenance};

    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn observe(concept: ObservationConcept, secs: u64) -> Observation {
        let source = ObservationSource::new("retrieval-test").expect("a valid source");
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

    /// One body of work, worked across three sittings by interleaving two loci
    /// that share distinctive vocabulary, with a save between them. `distinct`
    /// is a token that appears in this work's names and (by construction of the
    /// caller) in no other.
    fn work(
        observations: &mut Vec<Observation>,
        start_sitting: u64,
        editor: &str,
        sheet: &str,
        path: &str,
    ) {
        for sitting in start_sitting..start_sitting + 3 {
            let mut moment = sitting * 24 * 60 * 60; // a day apart: no interleaving across works
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

    /// Two distinct bodies of work that share the generic words "report" and
    /// "draft" but differ on "quarterly"/"figures" versus "annual"/"budget".
    fn two_reports() -> Vec<Observation> {
        let mut observations = Vec::new();
        work(
            &mut observations,
            0,
            "quarterly report draft — Editor",
            "quarterly figures — Sheet",
            "/home/x/quarterly-report.txt",
        );
        work(
            &mut observations,
            10,
            "annual report draft — Editor",
            "annual budget — Sheet",
            "/home/x/annual-report.txt",
        );
        observations
    }

    fn reconstruct(observations: &[Observation]) -> Reconstruction {
        Reconstruction::from_observations(observations, EngagementParams::default())
    }

    /// The distinctive vocabulary of one work resolves to it alone.
    #[test]
    fn a_distinctive_trigger_resolves_to_one_body_of_work() {
        let reconstruction = reconstruct(&two_reports());
        let retrieval = Retrieval::new();

        match retrieval.retrieve("quarterly", &reconstruction) {
            Resolution::Resolved(engagement) => assert!(
                engagement
                    .subjects()
                    .iter()
                    .any(|subject| subject.contains("quarterly")),
                "\"quarterly\" must land on the quarterly work, got {:?}",
                engagement.subjects()
            ),
            other => panic!("a distinctive trigger must resolve, got {other:?}"),
        }

        match retrieval.retrieve("annual budget", &reconstruction) {
            Resolution::Resolved(engagement) => assert!(
                engagement
                    .subjects()
                    .iter()
                    .any(|subject| subject.contains("annual")),
                "\"annual budget\" must land on the annual work, got {:?}",
                engagement.subjects()
            ),
            other => panic!("a distinctive trigger must resolve, got {other:?}"),
        }
    }

    /// A word shared by two bodies of work at comparable strength must not
    /// resolve confidently to one — Evo asks which.
    #[test]
    fn a_shared_word_is_ambiguous_not_a_confident_guess() {
        let reconstruction = reconstruct(&two_reports());
        let retrieval = Retrieval::new();

        match retrieval.retrieve("report", &reconstruction) {
            Resolution::Ambiguous(candidates) => {
                assert!(
                    candidates.len() >= 2,
                    "a word both works share cannot pick one of them"
                );
            }
            Resolution::NotFound => {
                // Also acceptable: if "report" is common enough its IDF is
                // negligible, refusing is honest too. What must never happen is
                // a confident single answer.
            }
            Resolution::Resolved(engagement) => panic!(
                "a shared word resolved confidently to {:?} — that is a coin-flip presented as fact",
                engagement.subjects()
            ),
        }
    }

    /// A trigger that matches nothing is not forced onto the nearest thread.
    #[test]
    fn an_unmatched_trigger_finds_nothing() {
        let reconstruction = reconstruct(&two_reports());
        let retrieval = Retrieval::new();
        assert!(
            matches!(
                retrieval.retrieve("xyzzy nonexistent plugh", &reconstruction),
                Resolution::NotFound
            ),
            "a trigger sharing no vocabulary with any thread must find nothing"
        );
    }

    /// An empty trigger, or an empty history, finds nothing rather than panicking
    /// or guessing.
    #[test]
    fn degenerate_inputs_find_nothing() {
        let retrieval = Retrieval::new();

        let populated = reconstruct(&two_reports());
        assert!(matches!(
            retrieval.retrieve("", &populated),
            Resolution::NotFound
        ));
        assert!(matches!(
            retrieval.retrieve("   ", &populated),
            Resolution::NotFound
        ));

        let empty = reconstruct(&[]);
        assert!(matches!(
            retrieval.retrieve("quarterly", &empty),
            Resolution::NotFound
        ));
    }

    /// The same history resolves the same trigger identically every time — the
    /// property that lets live and replayed state agree.
    #[test]
    fn resolution_is_deterministic() {
        let observations = two_reports();
        let first = reconstruct(&observations);
        let second = reconstruct(&observations);
        let retrieval = Retrieval::new();

        let a = retrieval.retrieve("quarterly figures", &first);
        let b = retrieval.retrieve("quarterly figures", &second);
        match (a, b) {
            (Resolution::Resolved(left), Resolution::Resolved(right)) => {
                assert_eq!(left.subjects(), right.subjects());
            }
            other => panic!("a distinctive trigger must resolve identically, got {other:?}"),
        }
    }

    #[test]
    fn retrieval_is_copy_clone_and_default() {
        let a = Retrieval;
        let b = a;
        let c = a;
        let d: Retrieval = Default::default();
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);
    }
}
