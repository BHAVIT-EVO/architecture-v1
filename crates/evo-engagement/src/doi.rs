//! Degree of Interest (DOI): ranking what matters within a task.
//!
//! From Kersten & Murphy, "Using Task Context to Improve Programmer
//! Productivity" (FSE 2006), field-validated with 16 industry programmers
//! (significant edit-ratio increase).
//!
//! # The algorithm
//!
//! ```text
//! interest = 0
//! for each event targeting element w:
//!     interest += SCALING(KIND)          # edit/input-active = 2, view/focus = 1
//!     currDecay = |events since decayStart| × decay_per_event
//!     if interest < currDecay:           # decayed out; re-anchor
//!         decayStart = event
//!         interest = SCALING(KIND)
//! DOI(w) = interest − |events since decayStart| × decay_per_event
//! ```
//!
//! - **Drop elements with DOI ≤ 0** from the task context.
//! - **Decay is per focus-switch event, not wall-clock** — robust to idle
//!   gaps; ported directly to Evo's focus stream.
//! - Map "edit" → input-active episodes (weight 2×), "view" → passive
//!   focus (weight 1×).
//!
//! # Why this matters for Evo
//!
//! When restoring work, Evo should restore the **core** (what the user was
//! actually doing) immediately, and offer the rest as one-tap available.
//! DOI ranks resources by their degree of interest within the task context,
//! so the restore policy knows what the core is.
//!
//! # No hardcoding
//!
//! Constants are from the published Mylyn implementation, not guesses about
//! applications or users. The 2:1 edit-over-view ratio is independently
//! replicated in ReflAction (Prante et al., SIGIR 2010) and TaskTracker's
//! FolderPredictor.

use std::collections::BTreeMap;

/// Interest added per input-active event (edit).
///
/// From Mylyn: edit/input-active = 2. The user is actively working with
/// this resource, not just looking at it.
pub const SCALING_EDIT: f64 = 2.0;

/// Interest added per passive focus event (view).
///
/// From Mylyn: view/focus = 1. The user is looking at this resource.
pub const SCALING_VIEW: f64 = 1.0;

/// Interest decayed per event since the decay anchor.
///
/// From Mylyn: 0.1 per event. (Soh et al. variant uses 0.017.)
/// Decay is per focus-switch event, not wall-clock — robust to idle gaps.
pub const DECAY_PER_EVENT: f64 = 0.1;

/// How a resource was interacted with, for DOI scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionKind {
    /// Input-active: typing, editing, deliberate action. Weight 2×.
    Edit,
    /// Passive focus: viewing, reading. Weight 1×.
    View,
}

impl InteractionKind {
    fn scaling(self) -> f64 {
        match self {
            InteractionKind::Edit => SCALING_EDIT,
            InteractionKind::View => SCALING_VIEW,
        }
    }
}

/// Degree of Interest tracker for one task context.
///
/// Tracks interest per resource subject. Interest increases on interaction,
/// decays per event. Resources with DOI ≤ 0 are dropped from the context.
#[derive(Debug, Clone, Default)]
pub struct DegreeOfInterest {
    /// Current interest per subject.
    interests: BTreeMap<String, f64>,
    /// Event count at the decay anchor per subject.
    /// When interest decays below the current decay, we re-anchor.
    decay_anchors: BTreeMap<String, u64>,
    /// Total events seen (for decay calculation).
    event_count: u64,
}

impl DegreeOfInterest {
    /// Creates an empty DOI tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an interaction with a resource.
    ///
    /// `subject` is the canonical resource subject.
    /// `kind` is Edit (input-active) or View (passive focus).
    pub fn record(&mut self, subject: &str, kind: InteractionKind) {
        self.event_count += 1;
        let scaling = kind.scaling();

        let interest = self.interests.entry(subject.to_string()).or_insert(0.0);
        let anchor = self
            .decay_anchors
            .entry(subject.to_string())
            .or_insert(self.event_count);

        // Current decay: events since anchor × decay_per_event.
        let events_since_anchor = self.event_count.saturating_sub(*anchor);
        let curr_decay = events_since_anchor as f64 * DECAY_PER_EVENT;

        *interest += scaling;

        // If decayed out, re-anchor: reset interest to just this event's scaling.
        if *interest < curr_decay {
            *anchor = self.event_count;
            *interest = scaling;
        }
    }

    /// The current DOI for a subject, or 0.0 if not tracked or decayed out.
    pub fn doi(&self, subject: &str) -> f64 {
        let Some(interest) = self.interests.get(subject) else {
            return 0.0;
        };
        let Some(anchor) = self.decay_anchors.get(subject) else {
            return 0.0;
        };
        let events_since_anchor = self.event_count.saturating_sub(*anchor);
        let decay = events_since_anchor as f64 * DECAY_PER_EVENT;
        (interest - decay).max(0.0)
    }

    /// All subjects with positive DOI, ranked highest first.
    ///
    /// This is the "core" of the task context — what the user was actually
    /// doing, ordered by degree of interest.
    pub fn ranked(&self) -> Vec<(String, f64)> {
        let mut ranked: Vec<(String, f64)> = self
            .interests
            .keys()
            .map(|subject| (subject.clone(), self.doi(subject)))
            .filter(|(_, doi)| *doi > 0.0)
            .collect();
        // Sort by DOI descending, then by subject for determinism.
        ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        ranked
    }

    /// The top N subjects by DOI. Used for staged restore: core now, rest offered.
    pub fn top(&self, n: usize) -> Vec<(String, f64)> {
        self.ranked().into_iter().take(n).collect()
    }

    /// Whether a subject is in the task context (DOI > 0).
    pub fn contains(&self, subject: &str) -> bool {
        self.doi(subject) > 0.0
    }

    /// How many resources are in the context.
    pub fn len(&self) -> usize {
        self.ranked().len()
    }

    /// Whether the context is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_weights_more_than_view() {
        let mut doi = DegreeOfInterest::new();
        doi.record("doc-a", InteractionKind::View);
        doi.record("doc-b", InteractionKind::Edit);
        // doc-b should have higher DOI: 2.0 vs 1.0 (minus decay).
        assert!(doi.doi("doc-b") > doi.doi("doc-a"));
    }

    #[test]
    fn decay_drops_stale_resources() {
        let mut doi = DegreeOfInterest::new();
        doi.record("old-doc", InteractionKind::View);
        // Many events on other resources decay the old one out.
        for i in 0..50 {
            doi.record(&format!("new-doc-{i}"), InteractionKind::View);
        }
        // old-doc should be decayed out (DOI ≤ 0).
        assert!(!doi.contains("old-doc"));
    }

    #[test]
    fn ranked_orders_by_interest() {
        let mut doi = DegreeOfInterest::new();
        doi.record("low", InteractionKind::View);
        doi.record("high", InteractionKind::Edit);
        doi.record("high", InteractionKind::Edit);
        doi.record("mid", InteractionKind::View);
        doi.record("mid", InteractionKind::View);

        let ranked = doi.ranked();
        assert_eq!(ranked[0].0, "high");
        // high: 4.0, mid: 2.0, low: 1.0 (minus decay, but order preserved)
        assert!(ranked[0].1 > ranked[1].1);
        assert!(ranked[1].1 > ranked[2].1);
    }

    #[test]
    fn top_limits_results() {
        let mut doi = DegreeOfInterest::new();
        for i in 0..10 {
            doi.record(&format!("doc-{i}"), InteractionKind::Edit);
        }
        assert_eq!(doi.top(3).len(), 3);
        assert_eq!(doi.top(100).len(), 10);
    }

    #[test]
    fn reanchor_on_decay_out() {
        let mut doi = DegreeOfInterest::new();
        doi.record("doc", InteractionKind::View);
        // Decay it out with many other events.
        for i in 0..100 {
            doi.record(&format!("other-{i}"), InteractionKind::View);
        }
        assert!(!doi.contains("doc"));
        // Re-touching re-anchors with fresh interest.
        doi.record("doc", InteractionKind::Edit);
        assert!(doi.contains("doc"));
        assert!(doi.doi("doc") > 0.0);
    }
}
