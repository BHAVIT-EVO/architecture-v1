//! Retrieval: finding the work the person means, from what they say.
//!
//! "Hey Evo, open automation X" — the retrieval question. The answer is a
//! meaning match over the thread's own witnessed material (anchor names,
//! companion names, and the apps and hosts they were witnessed in), never over
//! anything the engine was told about applications or domains. When two works
//! are indistinguishable from the words alone, the honest answer is to ask,
//! not to guess: the margin rule from the design (§C.7, §C.9).
//!
//! Three outcomes, all explicit:
//! * `Matched(thread_id, score)` — the best candidate won by a margin.
//! * `Ambiguous(Vec<ThreadId>)` — two or more candidates the words cannot
//!   separate; the person must choose.
//! * `NoMatch` — nothing in the person's work answers to these words.
//!
//! # How it matches
//!
//! Each thread is described by a document — what the person made (anchors,
//! strongest first), the context that recurred around it (companions), and the
//! apps and hosts that material was witnessed in — and an embedding model
//! turns the query and every document into vectors. Cosine similarity is the
//! score. There is no tokenization, no stemming, no keyword weighting, and no
//! stop-word list: the model already understands filler ("hey, can you open…"),
//! paraphrase ("fixing the technology of Evo" ≈ "debugging Evo's engine"), and
//! synonyms, which is exactly why those machineries are gone. What a query
//! matches is learned entirely from the person's own witnessed material, as
//! embedded by a general model trained on language — not on this person's work,
//! and never on a list of application names.

use crate::types::{ResumeBundle, ThreadId, ThreadView};
use evo_semantic::{
    app_marker_name, url_host, EmbedError, Embedder, SemanticIndex, SemanticParams,
    SemanticResolution,
};
use std::collections::BTreeSet;

/// The retrieval decision for one query.
#[derive(Debug, Clone, PartialEq)]
pub enum Retrieval {
    /// One work won the similarity ranking by a margin. The score is cosine
    /// similarity, not a probability or proof of the person's intent.
    Matched { thread: ThreadId, score: f64 },
    /// The words match multiple works too closely to choose. The person
    /// must decide; the candidates are listed in canonical order.
    Ambiguous { candidates: Vec<ThreadId> },
    /// Nothing the person has worked on answers to these words.
    NoMatch,
}

/// The retrieval margin: the runner-up must reach this fraction of the
/// winner before the engine considers them indistinguishable and asks.
pub const RETRIEVAL_MARGIN: f64 = 0.85;

/// The minimum cosine similarity for a semantic match: scope selection and
/// [`retrieve`] treat anything below this as no match at all.
///
/// Calibrated 2026-09-28 against a real embedding model (all-MiniLM-L6-v2)
/// on bhvt's own example phrases: true matches scored 0.32–0.60, unrelated
/// queries scored ≤ 0.19 against every thread. Re-validate against real user
/// data before moving it: a bad signal here costs a user.
pub const RETRIEVAL_FLOOR: f64 = 0.25;

/// The minimum cosine similarity for a trigger to restore a work *without
/// asking* — a stricter bar than [`RETRIEVAL_FLOOR`], because the failure
/// modes are not symmetric: a missed restore costs one glance at a carousel
/// of the nearest works, a wrong one costs the person their context.
///
/// Measured 2026-10-14 against a real log (this machine's 30 works,
/// all-MiniLM-L6-v2, the production document composition: witnessed names |
/// next step | structural material; see
/// `cargo run -p evo-daemon --example measure_vague_retrieval`):
///
/// * clear requests scored 0.35–0.77: "benchmark arena" 0.720, "custom email
///   banner for investor outreach" 0.768, "the overleaf paper" 0.521, "the
///   codex docs page" 0.349;
/// * realistic vague phrasing scored at most 0.353: "I was working on the
///   pitch thing" 0.353, "the thing with the website" 0.306, "that doc I had
///   open yesterday" 0.275, "what I was looking at earlier" 0.205;
/// * unrelated controls scored at most 0.23 ("quantum computing lecture")
///   even as the nearest five works.
///
/// The bands interleave at ~0.35 (the strongest measured vague phrase sits a
/// few thousandths above the weakest measured clear request), so no floor can
/// separate them; 0.40 sits above both, keeping those marginal cases on the
/// carousel side while leaving the strong hits (0.52+) real headroom. The
/// verbatim-only 2026-09-28 calibration did not see this: vague phrases at
/// 0.25–0.35 auto-restored under it. Re-validate against more real phrasing
/// before moving it.
pub const RETRIEVAL_RESTORE_FLOOR: f64 = 0.40;

/// How much of a thread's material describes it: the strongest anchors, then
/// the weightiest companions. A cap, not a ranking — the embedding sees the
/// most identifying material first and an enormous thread cannot drown it in
/// noise.
const MAX_ANCHOR_TERMS: usize = 8;
const MAX_COMPANION_TERMS: usize = 8;

/// The text a thread is matched by.
///
/// What the person made (anchors, strongest first), then the context that
/// recurred around it (companions, weightiest first), then the apps and hosts
/// that material was witnessed in — so "the Muse chat" can meet a thread whose
/// material lives at `claude.ai`, without anything ever naming an application.
/// Every app and host here was witnessed; none was listed.
///
/// Public so callers (and tests) can see exactly what a query is matched
/// against: retrieval has no hidden vocabulary.
pub fn thread_document(thread: &ThreadView) -> String {
    thread_document_with_surface(thread, &[])
}

/// The text a thread is matched by, with the caller's natural-language
/// surface folded in ahead of the structural material.
///
/// The caller owns the words — in the voice path the understanding layer
/// supplies the work's next step ("Continue in …", "Unsaved changes in …") —
/// while placement is this function's business: natural language first,
/// locators after, so a vague spoken phrase ("that doc I had open") meets a
/// sentence the way people talk before it meets a path, a URL, or an opaque
/// id. The surface adds vocabulary, never membership: it cannot introduce a
/// target, only describe one the caller already holds.
///
/// Measured 2026-10-14 on a real log: one next-step line per work kept every
/// strong hit's ranking, improved a near-tie's order ("the claude chat about
/// the banner" put the banner chat first without breaking the tie's
/// ambiguity), and pushed no vague phrase above [`RETRIEVAL_RESTORE_FLOOR`].
/// Long narrative boilerplate measured the other way — it diluted true hits
/// (0.415 → 0.224) — so keep the surface short and specific. Empty and
/// whitespace-only lines are skipped and never leave separators behind.
pub fn thread_document_with_surface(thread: &ThreadView, surface: &[&str]) -> String {
    let material = thread_material(thread);
    let mut document = String::new();
    for line in surface {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !document.is_empty() {
            document.push_str(" | ");
        }
        document.push_str(line);
    }
    if document.is_empty() || material.is_empty() {
        document.push_str(&material);
        document
    } else {
        format!("{document} | {material}")
    }
}

/// The structural material alone: anchors, companions, the apps and hosts
/// they were witnessed in.
fn thread_material(thread: &ThreadView) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for anchor in thread.anchors.iter().take(MAX_ANCHOR_TERMS) {
        parts.push(anchor.resource.as_str());
    }
    let mut companions: Vec<(&str, f64)> = thread
        .companions
        .iter()
        .map(|(resource, weight)| (resource.as_str(), *weight))
        .collect();
    companions.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.0.cmp(right.0))
    });
    for (resource, _) in companions.iter().take(MAX_COMPANION_TERMS) {
        parts.push(resource);
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

/// Retrieves the thread the person most plausibly means from a natural phrase.
///
/// Scores the query against every thread's document by meaning, not by shared
/// tokens. Returns an error when the embedding machinery fails — a broken
/// model is a broken system, never a "no match".
pub fn retrieve(
    query: &str,
    threads: &[ThreadView],
    embedder: &dyn Embedder,
) -> Result<Retrieval, EmbedError> {
    let params = SemanticParams {
        margin: RETRIEVAL_MARGIN,
        floor: RETRIEVAL_FLOOR,
    };
    let index = SemanticIndex::build(
        threads
            .iter()
            .map(|thread| (thread.id, thread_document(thread))),
        embedder,
        params,
    )?;
    match index.resolve(query, embedder)? {
        SemanticResolution::Matched { id, score } => Ok(Retrieval::Matched { thread: id, score }),
        SemanticResolution::Ambiguous { candidates } => Ok(Retrieval::Ambiguous { candidates }),
        SemanticResolution::NoMatch => Ok(Retrieval::NoMatch),
    }
}

/// Scope selection, separate from permission to execute a restore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeResolution {
    /// No scope was requested. Exact duplicates collapse while preserving the
    /// bundle's first-occurrence order, including its initial resume target.
    Unscoped { resources: Vec<String> },
    /// One resource was selected by exact identity or retrieval evidence.
    /// Similarity is evidence, not proof of the person's intent.
    Matched { resources: Vec<String> },
    /// Alternatives for the person to choose, never an executable restore set.
    Ambiguous { candidates: Vec<String> },
    /// A scope was requested but no resource was supported. Do not widen it.
    NoMatch,
}

/// Resolve a named surface within the bundle, without executing anything.
///
/// An exact full locator is checked before trimming input, preserving spaces
/// that belong to a path. Otherwise input is trimmed: empty input is explicitly
/// unscoped, then an exact locator is checked again before semantic matching
/// and (only on semantic NoMatch) distinctive substring evidence. Cosine scores
/// and their margin are ranking heuristics, not probabilities or authorization.
///
/// Resource identity is the exact witnessed string: duplicates collapse.
/// Unscoped resources retain bundle order; scoped candidates use canonical
/// locator order. Case-sensitive paths and URL queries are not conflated. A
/// nonempty query never falls back to the full bundle. Model errors propagate
/// rather than enabling the substring fallback.
pub fn resolve_scope(
    bundle: &ResumeBundle,
    surface_query: &str,
    embedder: &dyn Embedder,
) -> Result<ScopeResolution, EmbedError> {
    let mut seen = BTreeSet::new();
    let mut resources: Vec<String> = bundle
        .restore_set
        .iter()
        .filter(|resource| seen.insert(resource.as_str()))
        .cloned()
        .collect();
    if !surface_query.is_empty() {
        if let Some(resource) = resources
            .iter()
            .find(|resource| resource.as_str() == surface_query)
        {
            return Ok(ScopeResolution::Matched {
                resources: vec![resource.clone()],
            });
        }
    }
    let query = surface_query.trim();
    if query.is_empty() {
        return Ok(ScopeResolution::Unscoped { resources });
    }
    resources.sort();
    if let Some(resource) = resources.iter().find(|resource| resource.as_str() == query) {
        return Ok(ScopeResolution::Matched {
            resources: vec![resource.clone()],
        });
    }
    let params = SemanticParams {
        margin: RETRIEVAL_MARGIN,
        floor: RETRIEVAL_FLOOR,
    };
    let index = SemanticIndex::build(
        resources
            .iter()
            .enumerate()
            .map(|(position, resource)| (position, resource.clone())),
        embedder,
        params,
    )?;
    match index.resolve(query, embedder)? {
        SemanticResolution::Matched { id, .. } => Ok(ScopeResolution::Matched {
            resources: vec![resources[id].clone()],
        }),
        SemanticResolution::Ambiguous { candidates } => {
            let mut candidates: Vec<String> = candidates
                .into_iter()
                .map(|position| resources[position].clone())
                .collect();
            candidates.sort();
            Ok(ScopeResolution::Ambiguous { candidates })
        }
        SemanticResolution::NoMatch => Ok(substring_scope(&resources, query)),
    }
}

/// Compatibility projection of [`resolve_scope`]. Unresolved scope is always
/// empty; callers needing to distinguish ambiguity from no match should use
/// the explicit API. Errors still propagate.
pub fn scope_restore_set(
    bundle: &ResumeBundle,
    surface_query: &str,
    embedder: &dyn Embedder,
) -> Result<Vec<String>, EmbedError> {
    match resolve_scope(bundle, surface_query, embedder)? {
        ScopeResolution::Unscoped { resources } | ScopeResolution::Matched { resources } => {
            Ok(resources)
        }
        ScopeResolution::Ambiguous { .. } | ScopeResolution::NoMatch => Ok(Vec::new()),
    }
}

/// Fallback surface matching by distinctive substring.
///
/// Tokens occurring in every distinct resource contribute no evidence. Scheme
/// prefixes and punctuation alone are not surface names. All remaining token
/// matches must support just one resource to narrow; conflicting evidence asks
/// the person even if one alternative has more matching tokens. There is no
/// vocabulary list or repetition-based vote that can manufacture a winner.
fn substring_scope(resources: &[String], query: &str) -> ScopeResolution {
    let tokens: BTreeSet<String> = query
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|token| token.chars().any(char::is_alphanumeric))
        .collect();
    let lowered: Vec<String> = resources
        .iter()
        .map(|resource| {
            // Recognize URI schemes structurally, including opaque app: URIs.
            let content = match resource.split_once(':') {
                Some((scheme, rest))
                    if scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                        && scheme
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) =>
                {
                    rest
                }
                _ => resource,
            };
            content.to_lowercase()
        })
        .collect();
    let mut supported = BTreeSet::new();
    for token in tokens {
        let matches: Vec<usize> = lowered
            .iter()
            .enumerate()
            .filter(|(_, name)| name.contains(token.as_str()))
            .map(|(position, _)| position)
            .collect();
        if matches.len() < resources.len() {
            supported.extend(matches);
        }
    }
    let candidates: Vec<String> = supported
        .into_iter()
        .map(|position| resources[position].clone())
        .collect();
    match candidates.len() {
        0 => ScopeResolution::NoMatch,
        1 => ScopeResolution::Matched {
            resources: candidates,
        },
        _ => ScopeResolution::Ambiguous { candidates },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Anchor, AnchorKind};
    use std::collections::BTreeMap;

    /// Deterministic stand-in for a real model: the test prescribes the
    /// vectors, pinning the *wiring* (documents, fallback, errors) rather than
    /// any model's opinions about language. Real-model behavior — paraphrase,
    /// accents, synonyms — is covered by the voice pipeline's integration
    /// tests, where the actual model runs.
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

    fn thread_with_anchor(id: ThreadId, resource: &str, strength: f64) -> ThreadView {
        ThreadView {
            id,
            status: crate::Status::Active,
            episodes: Vec::new(),
            anchors: vec![Anchor {
                resource: resource.to_string(),
                kind: AnchorKind::Mutation,
                strength,
            }],
            companions: Vec::new(),
            contested: Vec::new(),
            fork: None,
        }
    }

    fn doc(thread: &ThreadView) -> String {
        thread_document(thread)
    }

    /// With no surface lines the enriched composition is byte-identical to
    /// the structural document — existing callers cannot drift.
    #[test]
    fn an_empty_surface_keeps_the_structural_document() {
        let thread = thread_with_anchor(1, "file:///work/report.md", 100.0);
        assert_eq!(
            thread_document(&thread),
            thread_document_with_surface(&thread, &["", "   "])
        );
    }

    /// The caller's natural-language surface leads; the locators follow.
    #[test]
    fn natural_language_surface_precedes_locators() {
        let thread = thread_with_anchor(1, "https://claude.ai/chat/abc", 300.0);
        let document = thread_document_with_surface(&thread, &["Continue in the banner chat"]);
        assert!(
            document.starts_with("Continue in the banner chat | "),
            "natural language leads the document: {document:?}"
        );
        assert!(document.contains("https://claude.ai/chat/abc"), "{document:?}");
        assert!(document.contains("claude.ai"), "{document:?}");
    }

    /// The trigger's restore bar is strictly stricter than the matching bar:
    /// no caller can accidentally auto-restore on match-floor evidence.
    #[test]
    fn restore_floor_is_stricter_than_the_match_floor() {
        assert!(RETRIEVAL_FLOOR < RETRIEVAL_RESTORE_FLOOR, "{RETRIEVAL_FLOOR} vs {RETRIEVAL_RESTORE_FLOOR}");
        assert!(RETRIEVAL_RESTORE_FLOOR < 1.0);
    }

    #[test]
    fn a_clear_name_matches_its_thread() {
        let threads = vec![
            thread_with_anchor(1, "file:///automation-x/main.py", 500.0),
            thread_with_anchor(2, "file:///automation-y/main.py", 500.0),
            thread_with_anchor(3, "https://claude.ai/chat/abc", 300.0),
        ];
        let embedder = StubEmbedder::new(&[
            (doc(&threads[0]).as_str(), &[1.0, 0.0, 0.0]),
            (doc(&threads[1]).as_str(), &[0.0, 1.0, 0.0]),
            (doc(&threads[2]).as_str(), &[0.0, 0.0, 1.0]),
            ("automation x", &[0.95, 0.31, 0.0]),
        ]);
        let result =
            retrieve("automation x", &threads, &embedder).expect("the stub embeds everything");
        assert!(
            matches!(result, Retrieval::Matched { thread: 1, .. }),
            "{result:?}"
        );
    }

    #[test]
    fn an_indistinguishable_pair_asks() {
        let threads = vec![
            thread_with_anchor(1, "file:///project-a/main.rs", 400.0),
            thread_with_anchor(2, "file:///project-b/main.rs", 400.0),
        ];
        let embedder = StubEmbedder::new(&[
            (doc(&threads[0]).as_str(), &[1.0, 0.1, 0.0]),
            (doc(&threads[1]).as_str(), &[1.0, -0.1, 0.0]),
            ("main", &[1.0, 0.0, 0.0]),
        ]);
        let result = retrieve("main", &threads, &embedder).expect("the stub embeds everything");
        assert!(
            matches!(result, Retrieval::Ambiguous { .. }),
            "two equally strong matches must ask, not guess: {result:?}"
        );
    }

    #[test]
    fn no_match_says_so() {
        let threads = vec![thread_with_anchor(1, "file:///work/report.md", 100.0)];
        let embedder = StubEmbedder::new(&[
            (doc(&threads[0]).as_str(), &[1.0, 0.0, 0.0]),
            ("quantum computing", &[0.0, 0.0, 1.0]),
        ]);
        let result =
            retrieve("quantum computing", &threads, &embedder).expect("the stub embeds everything");
        assert!(matches!(result, Retrieval::NoMatch), "{result:?}");
    }

    /// The old code needed a stop-word list so that "hey", "can", "you",
    /// "open", "my", "please" didn't poison matching. Meaning-based matching
    /// has no such list: the whole phrase embeds, filler and all.
    #[test]
    fn filler_words_do_not_break_matching() {
        let threads = vec![thread_with_anchor(
            1,
            "file:///presentation/deck.key",
            300.0,
        )];
        let embedder = StubEmbedder::new(&[
            (doc(&threads[0]).as_str(), &[1.0, 0.0]),
            ("hey can you open my presentation please", &[0.95, 0.31]),
        ]);
        let result = retrieve(
            "hey can you open my presentation please",
            &threads,
            &embedder,
        )
        .expect("the stub embeds everything");
        assert!(
            matches!(result, Retrieval::Matched { thread: 1, .. }),
            "{result:?}"
        );
    }

    /// A query naming an app meets the thread through the hosts its material
    /// was witnessed at — no application is ever named in code.
    #[test]
    fn a_named_app_meets_its_witnessed_host() {
        let threads = vec![
            thread_with_anchor(1, "https://claude.ai/chat/abc", 300.0),
            thread_with_anchor(2, "file:///notes/todo.md", 300.0),
        ];
        let document = doc(&threads[0]);
        assert!(
            document.contains("claude.ai"),
            "the thread document must carry where its material was witnessed, got {document:?}"
        );
        let embedder = StubEmbedder::new(&[
            (document.as_str(), &[1.0, 0.0]),
            (doc(&threads[1]).as_str(), &[0.0, 1.0]),
            ("the Muse chat", &[0.95, 0.31]),
        ]);
        let result =
            retrieve("the Muse chat", &threads, &embedder).expect("the stub embeds everything");
        assert!(
            matches!(result, Retrieval::Matched { thread: 1, .. }),
            "{result:?}"
        );
    }

    /// A broken model is reported, never laundered into "no match".
    #[test]
    fn embedder_failure_is_an_error() {
        let threads = vec![thread_with_anchor(1, "file:///work/report.md", 100.0)];
        let embedder = StubEmbedder::new(&[]);
        assert!(
            matches!(
                retrieve("report", &threads, &embedder),
                Err(EmbedError::Refused(_))
            ),
            "an embedding failure must surface as an error"
        );
    }

    fn bundle() -> ResumeBundle {
        ResumeBundle {
            resume_point: "https://claude.ai/chat/abc".into(),
            resume_reason: "test".into(),
            restore_set: vec![
                "https://claude.ai/chat/abc".into(),
                "file:///evo/src/main.rs".into(),
                "app:terminal".into(),
            ],
            open_deltas: vec![],
        }
    }

    #[test]
    fn scoped_restore_filters_to_the_named_surface() {
        let bundle = bundle();
        let embedder = StubEmbedder::new(&[
            (bundle.restore_set[0].as_str(), &[1.0, 0.0, 0.0]),
            (bundle.restore_set[1].as_str(), &[0.0, 1.0, 0.0]),
            (bundle.restore_set[2].as_str(), &[0.0, 0.0, 1.0]),
            ("just the claude chat", &[0.95, 0.31, 0.0]),
        ]);
        let scoped = scope_restore_set(&bundle, "just the claude chat", &embedder)
            .expect("the stub embeds everything");
        assert_eq!(scoped, vec!["https://claude.ai/chat/abc".to_string()]);
    }

    #[test]
    fn an_empty_scope_query_returns_everything() {
        let bundle = bundle();
        let embedder = StubEmbedder::new(&[]);
        let scoped = scope_restore_set(&bundle, "", &embedder).expect("empty queries never embed");
        assert_eq!(scoped.len(), 3);
    }

    /// Unrecognized scope must refuse rather than silently widen the request.
    #[test]
    fn an_unmatched_scope_query_restores_nothing() {
        let bundle = bundle();
        let embedder = StubEmbedder::new(&[
            (bundle.restore_set[0].as_str(), &[1.0, 0.0, 0.0, 0.0]),
            (bundle.restore_set[1].as_str(), &[0.0, 1.0, 0.0, 0.0]),
            (bundle.restore_set[2].as_str(), &[0.0, 0.0, 1.0, 0.0]),
            ("xyzzy plugh", &[0.0, 0.0, 0.0, 1.0]),
        ]);
        assert_eq!(
            resolve_scope(&bundle, "xyzzy plugh", &embedder),
            Ok(ScopeResolution::NoMatch)
        );
        let scoped = scope_restore_set(&bundle, "xyzzy plugh", &embedder)
            .expect("the stub embeds everything");
        assert!(scoped.is_empty());
    }

    /// The substring fallback keys on distinctiveness, not on a word list:
    /// "claude" appears in one resource and decides; common words decide
    /// nothing.
    #[test]
    fn substring_fallback_is_distinctive_not_listed() {
        let bundle = bundle();
        // A stub whose vectors force the semantic path to miss, so the
        // fallback decides.
        let embedder = StubEmbedder::new(&[
            (bundle.restore_set[0].as_str(), &[1.0, 0.0]),
            (bundle.restore_set[1].as_str(), &[1.0, 0.0]),
            (bundle.restore_set[2].as_str(), &[1.0, 0.0]),
            ("claude", &[0.0, 1.0]),
        ]);
        let scoped =
            scope_restore_set(&bundle, "claude", &embedder).expect("the stub embeds everything");
        assert_eq!(scoped, vec!["https://claude.ai/chat/abc".to_string()]);
    }
}
