//! The text-trigger pipeline: a spoken (or typed) phrase → a body of work.
//!
//! This is the core of the "Hey Evo" moat, minus the audio. A trigger phrase
//! like "i was fixing the technology of evo" is resolved by meaning against
//! everything the reconstruction witnessed, using the production embedder.
//!
//! The three honest outcomes:
//! - [`TriggerOutcome::Resolved`] — a clear winner; restore with confirmation.
//! - [`TriggerOutcome::Ambiguous`] — cannot separate candidates; ask "X or Y?".
//! - [`TriggerOutcome::NotFound`] — honest miss; offer recent work.
//!
//! An embedding failure is an error, never laundered into "not found".

use evo_engagement::{Engagement, Reconstruction};
use evo_retrieval::{Resolution, SemanticRetrieval};

use evo_semantic::Embedder;

/// What a trigger phrase resolved to.
#[derive(Debug)]
pub enum TriggerOutcome<'a> {
    /// A single body of work, clearly the best match.
    Resolved(&'a Engagement),
    /// Multiple candidates the resolver cannot separate. The caller should
    /// ask the person to choose — never guess.
    Ambiguous(Vec<&'a Engagement>),
    /// Nothing matched. The caller should say so honestly and offer recent work.
    NotFound,
}

/// Resolves a trigger phrase to a body of work, by meaning.
///
/// `trigger` is the transcribed utterance (or typed text for testing).
/// `reconstruction` is the current understanding of the person's work.
/// `embedder` is the production embedding backend.
///
/// Uses the probe-validated params (floor 0.25, margin 0.85) via
/// [`SemanticRetrieval::new`].
/// Whether a transcribed command asks a *question about the work* rather
/// than naming a work to restore.
///
/// "take me back to the evo stuff" names a body of work; "what am I working
/// on?" does not — it asks the engine to speak its rack. Routing on the
/// utterance's shape (grammar) keeps this domain-blind: no application,
/// domain, or command vocabulary appears here. False positives are cheap
/// (the ask-path answers honestly from witnessed facts); a false negative
/// just falls through to semantic resolution as before.
pub fn is_ask_intent(text: &str) -> bool {
    let normalized = normalize(text);
    let question_shaped = normalized.ends_with('?')
        || normalized.starts_with("what ")
        || normalized.starts_with("where ")
        || normalized.starts_with("whats ")
        || normalized.starts_with("what's ");
    if !question_shaped {
        return false;
    }
    let work_words = [
        "work", "working", "works", "doing", "leave", "left", "off", "project", "projects",
    ];
    if work_words.iter().any(|w| has_word(&normalized, w)) {
        return true;
    }
    // "where was i" carries no single work word — the phrase is the signal.
    normalized.contains("was i")
}

/// Whole-word containment.
fn has_word(haystack: &str, word: &str) -> bool {
    haystack.split(' ').any(|t| t == word)
}

/// Lowercase, strip punctuation (keeping apostrophes out via the whats
/// variants), collapse whitespace — the same normalization family the
/// wake spotter uses.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn resolve_trigger<'a>(
    trigger: &str,
    reconstruction: &'a Reconstruction,
    embedder: &dyn Embedder,
) -> Result<TriggerOutcome<'a>, String> {
    let retriever = SemanticRetrieval::new();
    match retriever
        .retrieve(trigger, reconstruction, embedder)
        .map_err(|e| format!("embedding failed: {e}"))?
    {
        Resolution::Resolved(engagement) => Ok(TriggerOutcome::Resolved(engagement)),
        Resolution::Ambiguous(candidates) => Ok(TriggerOutcome::Ambiguous(candidates)),
        Resolution::NotFound => Ok(TriggerOutcome::NotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn questions_about_work_are_ask_intent() {
        for s in [
            "what am i working on",
            "What am I working on?",
            "what was i doing",
            "where was i",
            "where did i leave off",
            "what are my works",
            "what's my current project",
            "what projects do i have",
        ] {
            assert!(is_ask_intent(s), "expected ask intent: {s:?}");
        }
    }

    #[test]
    fn restore_commands_are_not_ask_intent() {
        for s in [
            "take me back to the evo stuff",
            "open my writing",
            "resume the manuscript",
            "back to zcode",
            "the second one",
            "",
        ] {
            assert!(!is_ask_intent(s), "expected restore intent: {s:?}");
        }
    }

    #[test]
    fn what_without_work_words_falls_through() {
        // A question that names no work vocabulary is not an ask: semantic
        // resolution may still find the named thing.
        assert!(!is_ask_intent("what time is it"));
    }
}
