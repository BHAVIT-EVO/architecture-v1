//! evo-semantic — meaning-based matching over embedded text.
//!
//! # What this crate does
//!
//! Every "what did the person mean?" question in Evo — a spoken "Hey Evo, open
//! the thing I was doing about the membrane osteology", a typed trigger, a
//! "just the Z AI session" scoping request — lands here. An [`Embedder`] turns
//! text into a vector; [`SemanticIndex`] resolves a query against indexed
//! documents by cosine similarity and returns one of three honest answers:
//! one document clearly won, several are indistinguishable, or nothing matched.
//!
//! # What this crate does not do
//!
//! It does not embed anything itself. The model lives at the application
//! boundary (the voice pipeline), behind the [`Embedder`] trait, so this crate
//! stays pure, deterministic, and testable without a model download. It does
//! not tokenize, stem, weight keywords, or keep stop-word lists: the embedding
//! model already understands filler words, paraphrase, and synonyms, which is
//! exactly why those machineries are gone. "I was fixing the technology of
//! Evo", "I was debugging Evo's engine", and "working on Evo" resolve by
//! meaning, not by sharing the token "evo".
//!
//! # Contract
//!
//! - Resolution is a pure, deterministic function of the query, the documents,
//!   and the parameters: no clock, no randomness. Documents are held in
//!   canonical (id-sorted) order and ties break on that order, so the same
//!   history resolves the same query identically on every replay.
//! - A winner must clearly beat the runner-up — the runner-up reaching
//!   [`SemanticParams::margin`] of the winner's score makes the answer
//!   [`SemanticResolution::Ambiguous`], never a coin-flip presented as fact.
//! - A winner below [`SemanticParams::floor`] is not a winner at all:
//!   [`SemanticResolution::NoMatch`] is the honest answer to "I don't know
//!   what you mean".
//! - An embedding failure is reported as an error, never laundered into
//!   "no match": a broken model is a broken system, not an empty history.
//!
//! # No hardcoded categories
//!
//! Nothing here names an application, domain, file type, profession, or notion
//! of "productive" software, and it cannot acquire one: it sees only vectors.
//! What a query matches is learned entirely from the person's own witnessed
//! material, as embedded by a general model trained on language — not on Evo,
//! not on this person's work.

/// Which side of a retrieval pair is being embedded.
///
/// Some retrieval models are asymmetric: for example, multilingual E5 expects
/// `passage:` on indexed documents and `query:` on user queries. The default
/// [`Embedder`] hooks below remain symmetric, so existing fixture embedders and
/// other implementations do not need to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingRole {
    /// Text stored in the semantic index.
    Document,
    /// Text used to search the semantic index.
    Query,
}

/// Turns text into a fixed-dimension embedding vector.
///
/// Implemented at the application boundary, where the model lives. The
/// implementation must be a pure function of its input: the same text embeds
/// to the same vector every time, and every text embeds to the same number of
/// dimensions. Vectors must be nonempty, finite, and have nonzero norm.
/// [`SemanticIndex`] checks these contracts and reports violations rather than
/// matching across invalid or incompatible spaces.
///
/// Real models often need mutable session state (an ONNX runtime session, a
/// tokenizer cache). Keep this trait `&self` and put that mutability inside
/// the implementation (a `Mutex`, a session pool): the *output* must stay a
/// pure function of the input even when the machinery behind it is not.
pub trait Embedder {
    /// Embeds `text`. Fails on empty text and on model errors.
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError>;

    /// Whether this model's similarity scores have been validated for direct
    /// restore decisions under the caller's thresholds. Uncalibrated models
    /// may rank candidates but require a person's exact choice before action.
    /// Defaults to the historical model contract for existing implementations.
    fn allows_similarity_restore(&self) -> bool {
        true
    }

    /// Embeds indexed document text.
    ///
    /// This defaults to the historical symmetric [`Self::embed`] contract.
    /// Models with asymmetric document/query instructions can override this
    /// hook without breaking existing fixture embedders.
    fn embed_document(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed(text)
    }

    /// Embeds a user query.
    ///
    /// This defaults to the historical symmetric [`Self::embed`] contract.
    /// Models with asymmetric document/query instructions can override this
    /// hook without breaking existing fixture embedders.
    fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed(text)
    }

    /// Embeds text for an explicit retrieval role.
    fn embed_for_role(&self, role: EmbeddingRole, text: &str) -> Result<Vec<f32>, EmbedError> {
        match role {
            EmbeddingRole::Document => self.embed_document(text),
            EmbeddingRole::Query => self.embed_query(text),
        }
    }
}

/// Why an embedding failed.
///
/// Kept separate from "no match": these mean the understanding machinery is
/// broken, and the caller should say so rather than pretend the person said
/// nothing meaningful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbedError {
    /// The text, index parameters, or returned vector failed validation, or
    /// the model refused to embed. The string describes the refusal.
    Refused(String),
    /// The embedder returned vectors of inconsistent width — it is not a pure
    /// function of text, or two different models were mixed.
    DimensionMismatch { expected: usize, got: usize },
}

impl std::fmt::Display for EmbedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedError::Refused(text) => write!(f, "the embedder refused the text {text:?}"),
            EmbedError::DimensionMismatch { expected, got } => write!(
                f,
                "inconsistent embedding dimensions: expected {expected}, got {got}"
            ),
        }
    }
}

impl std::error::Error for EmbedError {}

/// The cosine similarity of two embedding vectors, in [-1, 1].
///
/// Accumulated in `f64` to avoid overflow/underflow of finite `f32` products. A zero
/// vector is similar to nothing (0.0) rather than NaN — emptiness is not a
/// direction. This permissive helper is not the index's validation boundary;
/// [`SemanticIndex`] rejects invalid vectors before scoring them. Similarity
/// is a ranking score, not a calibrated probability of intent.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    debug_assert_eq!(
        a.len(),
        b.len(),
        "cosine similarity needs equal-length vectors"
    );
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        let x = *x as f64;
        let y = *y as f64;
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        // Clamp: floating-point rounding can push a perfect match a hair past 1.
        (dot / denom).clamp(-1.0, 1.0)
    }
}

/// Tuning for semantic resolution. One place, documented, no magic elsewhere.
///
/// Empirical basis (2026-09-28, all-MiniLM-L6-v2, thread documents built from
/// witnessed anchors + app names + hosts, bhvt's own example phrases): true
/// matches scored 0.32–0.60, unrelated queries scored ≤ 0.19 against every
/// thread. The floor sits in that gap. Re-validate against real user data
/// before moving it: a bad signal here costs a user.
///
/// The floor is a per-caller decision bar, not a property of the model. A
/// caller that needs a stricter *confidence* bar — restoring a work without
/// asking, where a miss costs more than a glance at the candidates — passes a
/// higher floor and reads [`SemanticIndex::rank`] for the nearest documents
/// under it (the voice trigger measures this bar separately; see
/// `RETRIEVAL_RESTORE_FLOOR` in `evo-threads`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticParams {
    /// The runner-up must reach this *fraction* of the winner's score before
    /// the two are declared indistinguishable. Must be finite and in (0, 1];
    /// 1 admits only exact ties, and 0.85 keeps the historical contract.
    pub margin: f64,
    /// Absolute similarity below which the query matches nothing, however it
    /// ranks. Probe-measured: unrelated texts score ≤ 0.19 with the
    /// production embedding setup; paraphrases score ≥ 0.32. Must be finite
    /// and in (0, 1]: the ratio margin requires positive candidate scores.
    /// Below-floor documents are still available to callers that need the
    /// nearest ones ([`SemanticIndex::rank`]); below-floor is "not a match",
    /// not "no information".
    pub floor: f64,
}

impl Default for SemanticParams {
    fn default() -> Self {
        Self {
            margin: 0.85,
            floor: 0.25,
        }
    }
}

impl SemanticParams {
    fn validate(self) -> Result<(), EmbedError> {
        for (name, value) in [("margin", self.margin), ("floor", self.floor)] {
            if !value.is_finite() || value <= 0.0 || value > 1.0 {
                return Err(EmbedError::Refused(format!(
                    "invalid semantic {name}: expected a finite value in (0, 1], got {value}"
                )));
            }
        }
        Ok(())
    }
}

fn validate_vector(vector: &[f32], source: &str) -> Result<(), EmbedError> {
    let reason = if vector.is_empty() {
        Some("empty vector")
    } else if vector.iter().any(|value| !value.is_finite()) {
        Some("nonfinite vector component")
    } else if vector.iter().all(|value| *value == 0.0) {
        Some("zero-norm vector")
    } else {
        None
    };
    match reason {
        Some(reason) => Err(EmbedError::Refused(format!(
            "invalid {source} embedding: {reason}"
        ))),
        None => Ok(()),
    }
}

/// What a query resolved to.
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticResolution<TId> {
    /// One document clearly won the similarity ranking. The score is not a
    /// probability, proof of intent, or permission to act.
    Matched { id: TId, score: f64 },
    /// Two or more documents the query cannot separate, best-scoring first.
    /// The caller should ask *which one* rather than pick.
    Ambiguous { candidates: Vec<TId> },
    /// Nothing matched the query at all.
    NoMatch,
}

/// An embedder-backed index over a set of documents, resolved by meaning.
///
/// Built once per reconstruction (or per restore set); resolution itself is a
/// pure function of the query. Holds no clock, no randomness, no model.
#[derive(Debug, Clone)]
pub struct SemanticIndex<TId> {
    documents: Vec<(TId, Vec<f32>)>,
    params: SemanticParams,
}

impl<TId: Copy + Ord> SemanticIndex<TId> {
    /// Embeds every document and holds the index.
    ///
    /// Documents whose text is empty carry no meaning and are skipped — a body
    /// of work with no material simply isn't retrievable, rather than failing
    /// the whole index. Documents are stored in id order so resolution is
    /// independent of the order they were supplied in. Nonempty documents must
    /// have unique IDs. Invalid parameters or invalid embedding vectors return
    /// [`EmbedError::Refused`]; incompatible widths return a dimension error.
    pub fn build(
        documents: impl IntoIterator<Item = (TId, String)>,
        embedder: &dyn Embedder,
        params: SemanticParams,
    ) -> Result<Self, EmbedError> {
        params.validate()?;
        let mut embedded: Vec<(TId, Vec<f32>)> = Vec::new();
        let mut expected_dim: Option<usize> = None;
        let mut documents: Vec<(TId, String)> = documents
            .into_iter()
            .filter(|(_, text)| !text.trim().is_empty())
            .collect();
        documents.sort_by_key(|entry| entry.0);
        if documents.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(EmbedError::Refused("duplicate semantic document ID".into()));
        }
        for (id, text) in documents {
            let vector = embedder.embed_document(&text)?;
            validate_vector(&vector, "document")?;
            match expected_dim {
                None => expected_dim = Some(vector.len()),
                Some(dim) if dim != vector.len() => {
                    return Err(EmbedError::DimensionMismatch {
                        expected: dim,
                        got: vector.len(),
                    })
                }
                _ => {}
            }
            embedded.push((id, vector));
        }
        Ok(Self {
            documents: embedded,
            params,
        })
    }

    /// Ranks every indexed document against the query, best first, with no
    /// floor applied.
    ///
    /// [`Self::resolve`] is this ranking plus the decision rules. Callers use
    /// `rank` when they must still say *something* useful after nothing
    /// cleared the floor — the "nearest works" carousel after a vague spoken
    /// phrase — never to act: the nearest document to a phrase that matched
    /// nothing is still not a match, and this list is ranking evidence, not
    /// permission to restore.
    ///
    /// An empty query or empty index ranks nothing without consulting the
    /// embedder. Ties break on canonical id order so replays agree. Invalid
    /// query vectors return an error, never an empty ranking.
    pub fn rank(&self, query: &str, embedder: &dyn Embedder) -> Result<Vec<(TId, f64)>, EmbedError> {
        if query.trim().is_empty() || self.documents.is_empty() {
            return Ok(Vec::new());
        }
        let query_vector = embedder.embed_query(query)?;
        validate_vector(&query_vector, "query")?;
        let dim = self.documents[0].1.len();
        if query_vector.len() != dim {
            return Err(EmbedError::DimensionMismatch {
                expected: dim,
                got: query_vector.len(),
            });
        }

        let mut scored: Vec<(TId, f64)> = self
            .documents
            .iter()
            .map(|(id, vector)| (*id, cosine_similarity(&query_vector, vector)))
            .collect();

        // Best first; ties break on canonical id order, so replays agree.
        scored.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(left.0.cmp(&right.0))
        });
        Ok(scored)
    }

    /// Resolves a query against the indexed documents.
    ///
    /// An empty query matches nothing without consulting the embedder. Scores
    /// below [`SemanticParams::floor`] are dropped before the margin is
    /// applied, so "ambiguous" always means ambiguous between things that
    /// actually matched. Invalid query vectors return an error, never NoMatch.
    /// When the floor is a *confidence* bar (a restore without asking), the
    /// below-floor band is not meaningless: callers wanting the nearest
    /// documents even below it can read [`Self::rank`].
    pub fn resolve(
        &self,
        query: &str,
        embedder: &dyn Embedder,
    ) -> Result<SemanticResolution<TId>, EmbedError> {
        let scored: Vec<(TId, f64)> = self
            .rank(query, embedder)?
            .into_iter()
            .filter(|(_, score)| *score >= self.params.floor)
            .collect();

        if scored.is_empty() {
            return Ok(SemanticResolution::NoMatch);
        }

        let best = scored[0].1;
        let candidates: Vec<TId> = scored
            .iter()
            .filter(|(_, score)| *score >= self.params.margin * best)
            .map(|(id, _)| *id)
            .collect();

        if candidates.len() == 1 {
            Ok(SemanticResolution::Matched {
                id: candidates[0],
                score: best,
            })
        } else {
            Ok(SemanticResolution::Ambiguous { candidates })
        }
    }

    /// The resolution parameters this index was built with.
    pub fn params(&self) -> SemanticParams {
        self.params
    }

    /// How many documents carry meaning in this index.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether no document carried meaning.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

/// The host of a URL resource, structurally: the authority between the scheme
/// and the first path, port, or query separator. `None` for non-URLs.
///
/// Generic text surgery, not a list of known sites: `https://claude.ai/chat`
/// yields `claude.ai`, `file:///home/x` yields nothing. Used to let a query
/// that names where the work happened ("the claude chat") meet the material
/// it happened in, without ever naming an application.
pub fn url_host(resource: &str) -> Option<String> {
    let after_scheme = resource.split_once("://")?.1;
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .split('@')
        .next_back()
        .unwrap_or("");
    let host = authority
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if host.is_empty() || !host.contains('.') {
        return None;
    }
    Some(host)
}

/// The application named by an `app:<name>` resource marker, if any.
///
/// The marker is a structural convention of witnessed material, not a list of
/// known apps: any name the system witnessed works, including ones invented
/// after this code was written.
pub fn app_marker_name(resource: &str) -> Option<String> {
    let name = resource.strip_prefix("app:")?.trim().to_lowercase();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A deterministic stand-in for a real model: returns the vectors the test
    /// prescribes, so tests pin the *resolution logic* rather than any model's
    /// opinions about language.
    struct StubEmbedder {
        vectors: BTreeMap<String, Vec<f32>>,
    }

    impl StubEmbedder {
        fn new(pairs: &[(&str, &[f32])]) -> Self {
            Self {
                vectors: pairs
                    .iter()
                    .map(|(text, vector)| (text.to_string(), vector.to_vec()))
                    .collect(),
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

    struct AsymmetricEmbedder;

    impl Embedder for AsymmetricEmbedder {
        fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
            Err(EmbedError::Refused(
                "the symmetric hook must not be used".into(),
            ))
        }

        fn embed_document(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
            assert_eq!(text, "document text");
            Ok(vec![1.0, 0.0])
        }

        fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
            assert_eq!(text, "query text");
            Ok(vec![1.0, 0.0])
        }
    }

    fn params() -> SemanticParams {
        SemanticParams {
            margin: 0.85,
            floor: 0.35,
        }
    }

    #[test]
    fn semantic_index_uses_role_aware_hooks_without_changing_fixture_contracts() {
        let embedder = AsymmetricEmbedder;
        let index = SemanticIndex::build([(1u32, "document text".into())], &embedder, params())
            .expect("document hook should be used during indexing");
        assert_eq!(
            index.resolve("query text", &embedder),
            Ok(SemanticResolution::Matched { id: 1, score: 1.0 })
        );
        assert_eq!(
            embedder.embed_for_role(EmbeddingRole::Document, "document text"),
            Ok(vec![1.0, 0.0])
        );
        assert_eq!(
            embedder.embed_for_role(EmbeddingRole::Query, "query text"),
            Ok(vec![1.0, 0.0])
        );
    }

    /// A query close to document 1 and far from document 2 resolves to 1.
    #[test]
    fn a_clear_winner_matches() {
        let embedder = StubEmbedder::new(&[
            ("doc one", &[1.0, 0.0, 0.0]),
            ("doc two", &[0.0, 1.0, 0.0]),
            ("query near one", &[0.95, 0.31, 0.0]),
        ]);
        let index = SemanticIndex::build(
            [(1u32, "doc one".into()), (2u32, "doc two".into())],
            &embedder,
            params(),
        )
        .expect("the stub embeds everything");
        match index.resolve("query near one", &embedder) {
            Ok(SemanticResolution::Matched { id, score }) => {
                assert_eq!(id, 1);
                assert!(
                    score > 0.9,
                    "a near-duplicate should score high, got {score}"
                );
            }
            other => panic!("a clear winner must match, got {other:?}"),
        }
    }

    /// A query nearly equidistant from two documents is ambiguity, not a guess.
    #[test]
    fn a_close_runner_up_is_ambiguous() {
        let embedder = StubEmbedder::new(&[
            ("doc one", &[1.0, 0.1, 0.0]),
            ("doc two", &[1.0, -0.1, 0.0]),
            ("query between", &[1.0, 0.0, 0.0]),
        ]);
        let index = SemanticIndex::build(
            [(1u32, "doc one".into()), (2u32, "doc two".into())],
            &embedder,
            params(),
        )
        .expect("the stub embeds everything");
        match index.resolve("query between", &embedder) {
            Ok(SemanticResolution::Ambiguous { candidates }) => {
                assert_eq!(
                    candidates.len(),
                    2,
                    "both near-identical documents are candidates"
                );
            }
            other => panic!("a close runner-up must be ambiguous, got {other:?}"),
        }
    }

    /// A query far from every document matches nothing, however it ranks.
    #[test]
    fn a_distant_query_matches_nothing() {
        let embedder = StubEmbedder::new(&[
            ("doc one", &[1.0, 0.0, 0.0]),
            ("doc two", &[0.0, 1.0, 0.0]),
            ("far away", &[0.0, 0.0, 1.0]),
        ]);
        let index = SemanticIndex::build(
            [(1u32, "doc one".into()), (2u32, "doc two".into())],
            &embedder,
            params(),
        )
        .expect("the stub embeds everything");
        assert!(
            matches!(
                index.resolve("far away", &embedder),
                Ok(SemanticResolution::NoMatch)
            ),
            "a query orthogonal to every document must find nothing"
        );
    }

    /// Ranking keeps below-floor documents, best first: the nearest-work
    /// fallback needs honest evidence even when nothing matched.
    #[test]
    fn rank_keeps_below_floor_nearest_documents_in_order() {
        let embedder = StubEmbedder::new(&[
            ("doc one", &[1.0, 0.0, 0.0]),
            ("doc two", &[0.0, 1.0, 0.0]),
            ("near one", &[0.30, 0.20, 0.9327373]),
        ]);
        let index = SemanticIndex::build(
            [(1u32, "doc one".into()), (2u32, "doc two".into())],
            &embedder,
            params(),
        )
        .expect("the stub embeds everything");
        // The structural floor (0.35 in `params()`) drops both; the decision
        // says NoMatch...
        assert_eq!(
            index.resolve("near one", &embedder),
            Ok(SemanticResolution::NoMatch)
        );
        // ...while the ranking still knows which one is nearer, in order.
        let ranked = index.rank("near one", &embedder).expect("ranking works");
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].0, 1);
        assert_eq!(ranked[1].0, 2);
        assert!((ranked[0].1 - 0.30).abs() < 1e-4, "{ranked:?}");
        assert!((ranked[1].1 - 0.20).abs() < 1e-4, "{ranked:?}");
    }

    /// A tie ranks by canonical id order, the same rule resolution uses.
    #[test]
    fn rank_ties_break_on_canonical_id_order() {
        let embedder = StubEmbedder::new(&[
            ("a", &[1.0, 0.0]),
            ("b", &[1.0, 0.0]),
            ("same", &[1.0, 0.0]),
        ]);
        let index = SemanticIndex::build(
            [(2u32, "b".into()), (1u32, "a".into())],
            &embedder,
            params(),
        )
        .expect("the stub embeds everything");
        let ranked = index.rank("same", &embedder).expect("ranking works");
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].0, 1);
        assert_eq!(ranked[1].0, 2);
    }

    /// An empty query ranks nothing without reaching the embedder.
    #[test]
    fn rank_of_an_empty_query_is_empty_without_embedding() {
        struct PanickingEmbedder;
        impl Embedder for PanickingEmbedder {
            fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
                panic!("the embedder must not be consulted for an empty query")
            }
        }
        let index = SemanticIndex::build(
            [(1u32, "doc one".into())],
            &StubEmbedder::new(&[("doc one", &[1.0])]),
            params(),
        )
        .expect("build works");
        let ranked = index
            .rank("   ", &PanickingEmbedder)
            .expect("an empty query is not an error");
        assert!(ranked.is_empty());
    }

    /// Ranking reports the same explicit errors as resolution — a broken
    /// model is never a quietly empty ranking.
    #[test]
    fn rank_reports_the_same_explicit_errors_as_resolve() {
        let embedder = StubEmbedder::new(&[("a", &[1.0, 0.0]), ("wide", &[1.0, 0.0, 0.0])]);
        let index = SemanticIndex::build([(1u32, "a".into())], &embedder, params()).unwrap();
        assert_eq!(
            index.rank("wide", &embedder),
            Err(EmbedError::DimensionMismatch {
                expected: 2,
                got: 3
            })
        );
        assert_eq!(
            index.rank("missing", &embedder),
            Err(EmbedError::Refused("missing".into()))
        );
    }

    /// An empty query never reaches the embedder at all.
    #[test]
    fn an_empty_query_matches_nothing_without_embedding() {
        struct PanickingEmbedder;
        impl Embedder for PanickingEmbedder {
            fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
                panic!("the embedder must not be consulted for an empty query")
            }
        }
        let embedder = PanickingEmbedder;
        let index = SemanticIndex::build(
            [(1u32, "doc one".into())],
            &StubEmbedder::new(&[("doc one", &[1.0])]),
            params(),
        )
        .expect("build works");
        assert!(matches!(
            index.resolve("   ", &embedder),
            Ok(SemanticResolution::NoMatch)
        ));
    }

    /// Inconsistent vector widths fail the build loudly.
    #[test]
    fn dimension_mismatch_fails_loudly() {
        let embedder = StubEmbedder::new(&[("a", &[1.0, 0.0]), ("b", &[1.0, 0.0, 0.0])]);
        match SemanticIndex::build(
            [(1u32, "a".into()), (2u32, "b".into())],
            &embedder,
            params(),
        ) {
            Err(EmbedError::DimensionMismatch {
                expected: 2,
                got: 3,
            }) => {}
            other => panic!("mixed dimensions must fail loudly, got {other:?}"),
        }
    }

    /// A document that refuses to embed fails the build — a broken model is
    /// reported, never laundered into "no match".
    #[test]
    fn embedder_failure_is_an_error_not_a_no_match() {
        let embedder = StubEmbedder::new(&[("a", &[1.0])]);
        assert!(matches!(
            SemanticIndex::build(
                [(1u32, "a".into()), (2u32, "b".into())],
                &embedder,
                params()
            ),
            Err(EmbedError::Refused(_))
        ));
    }

    /// Documents with no text are skipped, not fatal.
    #[test]
    fn empty_documents_are_skipped() {
        let embedder = StubEmbedder::new(&[("a", &[1.0])]);
        let index =
            SemanticIndex::build([(1u32, "".into()), (2u32, "a".into())], &embedder, params())
                .expect("empty documents are skipped");
        assert_eq!(index.len(), 1);
    }

    /// Resolution does not depend on the order documents were supplied in.
    #[test]
    fn resolution_is_order_independent() {
        let embedder = StubEmbedder::new(&[
            ("doc one", &[1.0, 0.0]),
            ("doc two", &[0.0, 1.0]),
            ("query near one", &[0.9, 0.1]),
        ]);
        let forward = SemanticIndex::build(
            [(1u32, "doc one".into()), (2u32, "doc two".into())],
            &embedder,
            params(),
        )
        .expect("build works");
        let backward = SemanticIndex::build(
            [(2u32, "doc two".into()), (1u32, "doc one".into())],
            &embedder,
            params(),
        )
        .expect("build works");
        assert_eq!(
            forward.resolve("query near one", &embedder),
            backward.resolve("query near one", &embedder),
            "the same history must resolve the same query identically"
        );
    }

    /// A zero vector is similar to nothing — and never NaN.
    #[test]
    fn zero_vectors_are_not_similar_to_anything() {
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]), 1.0);
        assert!(cosine_similarity(&[-1.0, 0.0], &[1.0, 0.0]) < 0.0);
    }

    #[test]
    fn invalid_parameters_fail_even_for_an_empty_index() {
        let embedder = StubEmbedder::new(&[]);
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            0.0,
            -0.0,
            1.01,
        ] {
            for name in ["floor", "margin"] {
                let mut tuning = params();
                if name == "floor" {
                    tuning.floor = value;
                } else {
                    tuning.margin = value;
                }
                match SemanticIndex::build(Vec::<(u32, String)>::new(), &embedder, tuning) {
                    Err(EmbedError::Refused(reason)) => {
                        assert!(reason.contains(name), "{reason}");
                        assert!(reason.contains("(0, 1]"), "{reason}");
                    }
                    other => panic!("invalid {name} {value} must fail, got {other:?}"),
                }
            }
        }
    }

    #[test]
    fn parameter_boundaries_are_supported_without_empty_ambiguities() {
        let embedder = StubEmbedder::new(&[
            ("a", &[1.0, 0.0]),
            ("b", &[1.0, 0.0]),
            ("same", &[1.0, 0.0]),
            ("orthogonal", &[0.0, 1.0]),
        ]);
        for value in [f64::MIN_POSITIVE, 1.0] {
            let index = SemanticIndex::build(
                [(2u32, "b".into()), (1u32, "a".into())],
                &embedder,
                SemanticParams {
                    floor: value,
                    margin: value,
                },
            )
            .unwrap();
            assert_eq!(
                index.resolve("same", &embedder),
                Ok(SemanticResolution::Ambiguous {
                    candidates: vec![1, 2]
                })
            );
            assert_eq!(
                index.resolve("orthogonal", &embedder),
                Ok(SemanticResolution::NoMatch)
            );
        }
    }

    #[test]
    fn invalid_document_and_query_vectors_are_errors_not_misses() {
        let cases: &[(&[f32], &str)] = &[
            (&[], "empty vector"),
            (&[0.0, -0.0], "zero-norm vector"),
            (&[f32::NAN, 1.0], "nonfinite"),
            (&[1.0, f32::INFINITY], "nonfinite"),
            (&[f32::NEG_INFINITY, 1.0], "nonfinite"),
        ];
        for (vector, reason) in cases {
            let embedder = StubEmbedder::new(&[("bad", vector), ("good", &[1.0, 0.0])]);
            match SemanticIndex::build([(1u32, "bad".into())], &embedder, params()) {
                Err(EmbedError::Refused(message)) => {
                    assert!(message.contains("document"), "{message}");
                    assert!(message.contains(reason), "{message}");
                }
                other => panic!("{reason} must fail build, got {other:?}"),
            }
            let index = SemanticIndex::build([(1u32, "good".into())], &embedder, params()).unwrap();
            match index.resolve("bad", &embedder) {
                Err(EmbedError::Refused(message)) => {
                    assert!(message.contains("query"), "{message}");
                    assert!(message.contains(reason), "{message}");
                }
                other => panic!("{reason} must fail resolve, got {other:?}"),
            }
        }
    }

    #[test]
    fn query_dimension_mismatch_and_model_failure_are_explicit_errors() {
        let embedder = StubEmbedder::new(&[("a", &[1.0, 0.0]), ("wide", &[1.0, 0.0, 0.0])]);
        let index = SemanticIndex::build([(1u32, "a".into())], &embedder, params()).unwrap();
        assert_eq!(
            index.resolve("wide", &embedder),
            Err(EmbedError::DimensionMismatch {
                expected: 2,
                got: 3
            })
        );
        assert_eq!(
            index.resolve("missing", &embedder),
            Err(EmbedError::Refused("missing".into()))
        );
    }

    #[test]
    fn duplicate_document_ids_are_rejected_before_embedding() {
        let embedder = StubEmbedder::new(&[]);
        for documents in [
            [(1u32, "a".into()), (1u32, "b".into())],
            [(1u32, "b".into()), (1u32, "a".into())],
        ] {
            match SemanticIndex::build(documents, &embedder, params()) {
                Err(EmbedError::Refused(reason)) => assert!(reason.contains("duplicate")),
                other => panic!("duplicate IDs must fail, got {other:?}"),
            }
        }
    }

    #[test]
    fn finite_extreme_vectors_retain_direction_without_overflow_or_underflow() {
        // Include the smallest subnormal: a norm computed in f32 would vanish.
        for value in [f32::MAX, f32::MIN_POSITIVE, f32::from_bits(1)] {
            let embedder = StubEmbedder::new(&[
                ("forward", &[value, value]),
                ("backward", &[-value, -value]),
                ("same", &[value, value]),
                ("opposite", &[-value, -value]),
                ("orthogonal", &[value, -value]),
            ]);
            let index = SemanticIndex::build(
                [(1u32, "forward".into()), (2u32, "backward".into())],
                &embedder,
                params(),
            )
            .unwrap();
            for (query, expected) in [("same", 1), ("opposite", 2)] {
                match index.resolve(query, &embedder).unwrap() {
                    SemanticResolution::Matched { id, score } => {
                        assert_eq!(id, expected);
                        assert!(score.is_finite() && (score - 1.0).abs() < 1e-12, "{score}");
                    }
                    other => panic!("extreme valid vectors must match, got {other:?}"),
                }
            }
            assert_eq!(
                index.resolve("orthogonal", &embedder),
                Ok(SemanticResolution::NoMatch)
            );
        }
        // Also exercise a large dynamic range between the two vectors.
        let score = cosine_similarity(&[f32::MAX, f32::MAX], &[f32::from_bits(1); 2]);
        assert!(score.is_finite() && (score - 1.0).abs() < 1e-12);
    }

    /// URL hosts are extracted structurally, never from a list of known sites.
    #[test]
    fn url_host_extraction_is_structural() {
        assert_eq!(
            url_host("https://claude.ai/chat/abc"),
            Some("claude.ai".to_string())
        );
        assert_eq!(
            url_host("http://localhost:3000/x?y=1"),
            None,
            "hosts without a dot are machinery, not places"
        );
        assert_eq!(url_host("file:///home/x/report.md"), None);
        assert_eq!(url_host("just a window title"), None);
        assert_eq!(
            url_host("https://user@docs.example.com:8443/p"),
            Some("docs.example.com".to_string())
        );
    }

    /// `app:` markers name whatever was witnessed, including unknown apps.
    #[test]
    fn app_marker_names_are_witnessed_not_listed() {
        assert_eq!(app_marker_name("app:Muse"), Some("muse".to_string()));
        assert_eq!(app_marker_name("app:"), None);
        assert_eq!(app_marker_name("file:///x"), None);
    }
}
