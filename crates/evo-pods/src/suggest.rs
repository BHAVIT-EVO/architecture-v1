//! Suggestions: a loose window that strongly remembers a pod may be
//! *offered* to it. A suggestion is never a decision — it renders once,
//! it can be refused, and silence is absolute when evidence is missing or
//! contested.

use crate::config::PodConfig;
use crate::pod::{Pod, PodId};

#[derive(Debug, Clone, PartialEq)]
pub struct PodSuggestion {
    /// The pod that remembers this subject.
    pub pod: PodId,
    /// The subject witness (what to show: "add X to Y?").
    pub subject: String,
    /// The evidence weight behind it (ms of remembered attention).
    pub weight_ms: f64,
}

/// Offers at most one pod for a focused subject the bar doesn't own yet.
///
/// Rules:
/// * the subject must live in exactly one candidate pod's memory — two
///   competing pods contesting the same subject means silence;
/// * below the config floor, silence;
/// * never suggest into the active pod (that's already where the person is).
pub fn suggest(
    subject: &str,
    pods: &[Pod],
    active: Option<PodId>,
    config: &PodConfig,
) -> Option<PodSuggestion> {
    if subject.is_empty() {
        return None;
    }
    let mut candidates: Vec<&Pod> = pods
        .iter()
        .filter(|p| active.map(|a| a != p.id).unwrap_or(true))
        .filter(|p| {
            p.companions
                .iter()
                .any(|(s, w)| s == subject && *w >= config.suggest_min_weight_ms)
        })
        .collect();
    // One candidate only: contested memory is not a suggestion.
    if candidates.len() != 1 {
        return None;
    }
    let pod = candidates.remove(0);
    let weight = pod
        .companions
        .iter()
        .find(|(s, _)| s == subject)
        .map(|(_, w)| *w)
        .unwrap_or(0.0);
    Some(PodSuggestion {
        pod: pod.id,
        subject: subject.to_string(),
        weight_ms: weight,
    })
}
