//! Retrieval: finding the work the person means, from what they say.
//!
//! "Hey Evo, open automation X" — the retrieval question. The answer is a
//! ranked match over the thread's own witnessed material (anchor names,
//! companion names, participant names), never over anything the engine
//! was told about applications or domains. When two works are
//! indistinguishable from the words alone, the honest answer is to ask,
//! not to guess: the margin rule from the design (§C.7, §C.9).
//!
//! Three outcomes, all explicit:
//! * `Matched(thread_id, score)` — the best candidate won by a margin.
//! * `Ambiguous(Vec<ThreadId>)` — two or more candidates the words cannot
//!   separate; the person must choose.
//! * `NoMatch` — nothing in the person's work answers to these words.

use crate::types::{ThreadId, ThreadView};
use std::collections::BTreeMap;

/// The retrieval decision for one query.
#[derive(Debug, Clone, PartialEq)]
pub enum Retrieval {
    /// One work clearly won. The margin was decisive.
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

/// The minimum score for any match at all. Below this, the words are
/// noise, and saying "I don't know what you mean" is the honest answer.
pub const RETRIEVAL_FLOOR: f64 = 0.25;

/// Retrieves the thread the person most plausibly means from a natural
/// phrase, scored over the thread's own witnessed material.
///
/// Scoring is deliberately simple and explainable: each token in the query
/// that appears as a substring of a resource name contributes that
/// resource's weight (anchors at full strength, companions discounted by
/// ubiquity). No embeddings, no model, no statistics beyond counting —
/// because the words the person uses are already the vocabulary of their
/// work, and matching them honestly is enough.
pub fn retrieve(query: &str, threads: &[ThreadView]) -> Retrieval {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|t: &str| t.trim().to_lowercase())
        .filter(|t| !t.is_empty() && !is_stop_word(t))
        .collect();
    if tokens.is_empty() {
        return Retrieval::NoMatch;
    }

    let mut scores: BTreeMap<ThreadId, f64> = BTreeMap::new();
    for thread in threads {
        let mut score = 0.0;
        // Anchors: what the person made — the strongest signal of what the
        // work is called, because they are where the work happened.
        for anchor in &thread.anchors {
            let name = anchor.resource.to_lowercase();
            for token in &tokens {
                if name.contains(token.as_str()) {
                    score += anchor.strength;
                }
            }
        }
        // Companions: recurring context, discounted by how many works
        // share them (a companion in one work says more than one in five).
        for (resource, weight) in &thread.companions {
            let name = resource.to_lowercase();
            for token in &tokens {
                if name.contains(token.as_str()) {
                    score += weight * 0.5;
                }
            }
        }
        if score > 0.0 {
            scores.insert(thread.id, score);
        }
    }

    if scores.is_empty() {
        return Retrieval::NoMatch;
    }

    let mut ranked: Vec<(ThreadId, f64)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (best_id, best_score) = ranked[0];
    if best_score < RETRIEVAL_FLOOR {
        return Retrieval::NoMatch;
    }

    let second = ranked.get(1).map(|x| x.1).unwrap_or(0.0);
    if second >= best_score * RETRIEVAL_MARGIN {
        let candidates: Vec<ThreadId> = ranked
            .iter()
            .take_while(|(_, s)| *s >= best_score * RETRIEVAL_MARGIN)
            .map(|(id, _)| *id)
            .collect();
        return Retrieval::Ambiguous { candidates };
    }

    Retrieval::Matched { thread: best_id, score: best_score }
}

/// Words that carry no meaning for retrieval.
fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "the" | "a" | "an" | "my" | "open" | "hey" | "evo" | "can" | "you" | "continue"
            | "resume" | "work" | "please" | "just" | "only" | "that" | "this" | "for" | "me"
            | "of" | "in" | "on" | "at" | "to" | "is" | "it" | "and" | "or" | "with" | "was"
    )
}

/// Scoped restore: the subset of a thread's restore set that belongs to a
/// named surface within the work.
///
/// "Hey Evo, just the Z AI session of my Evo work" — the person wants one
/// surface from a multi-surface work, not the whole neighborhood. The
/// surface is identified by the same kind of name matching retrieval uses:
/// a token that matches a resource in the thread's material.
pub fn scope_restore_set(bundle: &crate::ResumeBundle, surface_query: &str) -> Vec<String> {
    let tokens: Vec<String> = surface_query
        .split_whitespace()
        .map(|t: &str| t.trim().to_lowercase())
        .filter(|t| !t.is_empty() && !is_stop_word(t))
        .collect();
    if tokens.is_empty() {
        return bundle.restore_set.clone();
    }

    bundle
        .restore_set
        .iter()
        .filter(|resource| {
            let name = resource.to_lowercase();
            tokens.iter().any(|token| name.contains(token.as_str()))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnchorKind, Anchor};

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

    #[test]
    fn a_clear_name_matches_its_thread() {
        let threads = vec![
            thread_with_anchor(1, "file:///automation-x/main.py", 500.0),
            thread_with_anchor(2, "file:///automation-y/main.py", 500.0),
            thread_with_anchor(3, "https://claude.ai/chat/abc", 300.0),
        ];
        let result = retrieve("automation x", &threads);
        assert!(matches!(result, Retrieval::Matched { thread: 1, .. }), "{result:?}");
    }

    #[test]
    fn an_indistinguishable_pair_asks() {
        let threads = vec![
            thread_with_anchor(1, "file:///project-a/main.rs", 400.0),
            thread_with_anchor(2, "file:///project-b/main.rs", 400.0),
        ];
        let result = retrieve("main", &threads);
        assert!(
            matches!(result, Retrieval::Ambiguous { .. }),
            "two equally strong matches must ask, not guess: {result:?}"
        );
    }

    #[test]
    fn no_match_says_so() {
        let threads = vec![thread_with_anchor(1, "file:///work/report.md", 100.0)];
        let result = retrieve("quantum computing", &threads);
        assert!(matches!(result, Retrieval::NoMatch), "{result:?}");
    }

    #[test]
    fn stop_words_are_ignored() {
        let threads = vec![thread_with_anchor(1, "file:///presentation/deck.key", 300.0)];
        let result = retrieve("hey can you open my presentation please", &threads);
        assert!(matches!(result, Retrieval::Matched { thread: 1, .. }), "{result:?}");
    }

    #[test]
    fn scoped_restore_filters_to_the_named_surface() {
        let bundle = crate::ResumeBundle {
            resume_point: "https://claude.ai/chat/abc".into(),
            resume_reason: "test".into(),
            restore_set: vec![
                "https://claude.ai/chat/abc".into(),
                "file:///evo/src/main.rs".into(),
                "app:terminal".into(),
            ],
            open_deltas: vec![],
        };
        let scoped = scope_restore_set(&bundle, "just the claude chat");
        assert_eq!(scoped, vec!["https://claude.ai/chat/abc".to_string()]);
    }

    #[test]
    fn an_empty_scope_query_returns_everything() {
        let bundle = crate::ResumeBundle {
            resume_point: "a".into(),
            resume_reason: "test".into(),
            restore_set: vec!["a".into(), "b".into()],
            open_deltas: vec![],
        };
        let scoped = scope_restore_set(&bundle, "");
        assert_eq!(scoped.len(), 2);
    }
}
