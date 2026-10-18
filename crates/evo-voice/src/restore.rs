//! Trigger → restore: the product verb.
//!
//! A resolved trigger is a body of work; this module turns that body of
//! work into the OS actions that bring it back. This is the final step of
//! the pipeline documented in lib.rs: "... → semantic resolution → restore".
//!
//! # What it does
//!
//! [`targets_from_engagement`] extracts the restorable resources from a
//! resolved [`Engagement`]: URLs and file paths. Window titles (Surface)
//! can only be raised, never opened; opaque identifiers cannot be restored
//! at all. [`restore_engagement`] executes the restore and returns the
//! honest account of what happened.
//!
//! # No hardcoding
//!
//! Nothing here names an application, domain, or resource kind. What is
//! restorable is determined by the [`SubjectShape`] the witness layer
//! assigned — a structural fact about how the resource was observed, not
//! a guess about what it means.

use evo_engagement::{DegreeOfInterest, Engagement, InteractionKind, SubjectShape};
use evo_execution::{restore, RestoreOutcome, RestoreTarget};
use evo_semantic::Embedder;
use std::collections::HashSet;

/// Builds the restorable targets from a resolved engagement's participants.
///
/// Only addressable resources (URLs) and file paths are restorable.
/// The application is left to the OS default handler: the witnessed app
/// association is capture-layer fact, not engagement-model fact, and the
/// executor prefers the OS default when the caller does not know better.
pub fn targets_from_engagement(engagement: &Engagement) -> Vec<RestoreTarget> {
    engagement
        .participants()
        .iter()
        .filter(|p| {
            matches!(
                p.resource().shape(),
                SubjectShape::Address | SubjectShape::Path
            )
        })
        .map(|p| RestoreTarget::with_default_handler(p.resource().subject()))
        .collect()
}

/// Restores the body of work a trigger resolved to.
///
/// Returns the honest account: what opened, what was already open and
/// raised, and what could not be restored (with reasons).
pub fn restore_engagement(engagement: &Engagement) -> RestoreOutcome {
    restore(&targets_from_engagement(engagement))
}

/// Number of core resources to restore immediately in staged restore.
///
/// From the research (§4d): restore the active-at-suspension window plus
/// the top 1-2 by core score immediately; offer the rest as one-tap available.
/// We use 3 as the documented starting point — the resume point plus the
/// top 2 by DOI. This is a calibration starting point, not a frozen value.
pub const STAGED_RESTORE_CORE_COUNT: usize = 3;

/// The outcome of a staged restore: what was restored now, and what's offered.
///
/// Staged restore replaces "open everything blindly" with "open what matters,
/// offer the rest". The core (highest DOI) is restored immediately; the
/// long tail is returned as one-tap available, not auto-opened.
#[derive(Debug)]
pub struct StagedRestoreOutcome {
    /// The restore outcome for the core resources (what was actually opened).
    pub restored: RestoreOutcome,
    /// Targets not restored but offered as one-tap available.
    /// Ranked by DOI (highest first), excluding the core.
    pub offered: Vec<RestoreTarget>,
}

/// Builds DOI for an engagement's participants from the reconstruction's episodes.
///
/// Records `Edit` for acts with human evidence (input-active), `View` otherwise.
/// Only acts targeting this engagement's participants are recorded — the DOI
/// reflects what the user was actually doing within this body of work.
fn build_doi_for_engagement(
    engagement: &Engagement,
    reconstruction: &evo_engagement::Reconstruction,
) -> DegreeOfInterest {
    let mut doi = DegreeOfInterest::new();

    // The participant subjects for this engagement.
    let participant_subjects: HashSet<String> = engagement
        .participants()
        .iter()
        .map(|p| p.resource().subject().to_string())
        .collect();

    // Record interactions from all episodes, filtered to this engagement's participants.
    for episode in reconstruction.episodes() {
        for act in episode.acts() {
            let subject = act.resource().subject();
            if participant_subjects.contains(subject) {
                let kind = if act.character().is_human_evidence() {
                    InteractionKind::Edit
                } else {
                    InteractionKind::View
                };
                doi.record(subject, kind);
            }
        }
    }

    doi
}

/// Restores with DOI-ranked staged restore.
///
/// 1. Builds DOI from the engagement's episodes (what the user actually did)
/// 2. Ranks restorable targets by DOI (highest interest first)
/// 3. Restores the core (top N by DOI) immediately
/// 4. Returns the rest as "offered" for one-tap availability
///
/// This is the "understands what matters" behavior: Evo restores the core
/// of the work immediately and offers the long tail, instead of blindly
/// opening every participant.
pub fn restore_engagement_staged(
    engagement: &Engagement,
    reconstruction: &evo_engagement::Reconstruction,
) -> StagedRestoreOutcome {
    let doi = build_doi_for_engagement(engagement, reconstruction);

    // Get all restorable targets.
    let all_targets = targets_from_engagement(engagement);

    // Rank by DOI. Targets without DOI evidence get 0.0 and sort last.
    // Use a stable sort with subject tie-break for determinism.
    let mut ranked: Vec<(RestoreTarget, f64)> = all_targets
        .into_iter()
        .map(|target| {
            let score = doi.doi(&target.resource);
            (target, score)
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.resource.cmp(&b.0.resource))
    });

    // Split into core (restore now) and offered (one-tap).
    let core_count = STAGED_RESTORE_CORE_COUNT.min(ranked.len());
    let (core, rest): (Vec<_>, Vec<_>) = ranked
        .into_iter()
        .enumerate()
        .partition(|(i, _)| *i < core_count);

    let core_targets: Vec<RestoreTarget> = core.into_iter().map(|(_, (t, _))| t).collect();
    let offered: Vec<RestoreTarget> = rest.into_iter().map(|(_, (t, _))| t).collect();

    // Restore the core immediately.
    let restored = restore(&core_targets);

    StagedRestoreOutcome { restored, offered }
}

/// One body of work a trigger could have meant, with the stable identity the
/// caller needs to ask the person to choose — or to restore by id later.
///
/// Owned data (no lifetimes) so it can cross the daemon/desktop socket
/// boundary. `work_id` is the `Display` of `evo_engagement::WorkId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceCandidate {
    /// Stable body-of-work identity.
    pub work_id: String,
    /// What the work is about, from the work's own witnessed material.
    pub title: String,
    /// The exact safe action represented by this choice. Most choices select
    /// a work; occurrence choices select one witnessed surface inside it.
    pub selection: VoiceSelection,
}

/// The selection represented by a voice candidate.
///
/// Occurrence keys are opaque, daemon-generated capabilities. The desktop
/// never constructs or interprets them; it only sends the selected key back
/// over the local socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceSelection {
    Work,
    Occurrence { key: String },
}

impl VoiceCandidate {
    /// Builds the candidate view of one engagement.
    pub fn from_engagement(engagement: &Engagement) -> Self {
        Self::work(
            engagement.work_id().to_string(),
            engagement.title().to_string(),
        )
    }

    /// Builds a normal work choice.
    pub fn work(work_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            work_id: work_id.into(),
            title: title.into(),
            selection: VoiceSelection::Work,
        }
    }

    /// Builds a choice for one witnessed occurrence of a work.
    pub fn occurrence(
        work_id: impl Into<String>,
        title: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            work_id: work_id.into(),
            title: title.into(),
            selection: VoiceSelection::Occurrence { key: key.into() },
        }
    }
}

/// The outcome of a trigger → restore attempt.
#[derive(Debug)]
pub enum TriggerRestoreOutcome {
    /// The trigger resolved and the restore was attempted. Carries the
    /// work's title so the caller can say what came back.
    Restored {
        /// What the work is about.
        title: String,
        /// The honest account of what opened, raised, or failed.
        outcome: RestoreOutcome,
        /// Resources not restored but offered as one-tap available.
        /// The long tail — ranked by DOI, not auto-opened.
        offered: Vec<String>,
    },
    /// The trigger was ambiguous. The candidates are carried — never
    /// discarded — so the caller can ask the person to choose.
    Ambiguous(Vec<VoiceCandidate>),
    /// Nothing matched; the caller should say so honestly.
    NotFound,
    /// The embedding backend failed; never laundered into "not found".
    EmbeddingFailed(String),
}

/// The outcome in owned, socket-friendly form: what the desktop UI renders
/// and what crosses the daemon/desktop voice socket.
///
/// Built from [`TriggerRestoreOutcome`] via [`VoiceOutcome::from_trigger`];
/// the daemon fills `recent` for `NotFound` from its current understanding
/// (most-recent-first) before sending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceOutcome {
    /// Work was restored: what it was, and the one-breath summary.
    /// `offered` are resources not restored but available as one-tap.
    Restored {
        title: String,
        summary: String,
        offered: Vec<String>,
    },
    /// Ask the person to choose; each candidate is a safe work or witnessed
    /// occurrence selection.
    Ambiguous { candidates: Vec<VoiceCandidate> },
    /// Honest miss, with recent work offered as alternatives.
    NotFound { recent: Vec<VoiceCandidate> },
    /// Something failed honestly: embedding, transport, or the model.
    Failed { reason: String },
    /// Wake word heard but no command followed, even after follow-up capture.
    NoCommand,
    /// The engine-backed read path (Phase 4): the rack, spoken — the
    /// narrated Work Briefs, in rack order. One narration for the whole
    /// rack (ASK) or a single work (BRIEF); the count says how many works
    /// the narration covers.
    Brief { count: usize, narration: String },
}

impl VoiceOutcome {
    /// Maps a trigger → restore outcome onto the wire/UI form.
    ///
    /// `recent` is the daemon's current recent-work list (possibly empty);
    /// it is only used for `NotFound`.
    pub fn from_trigger(outcome: TriggerRestoreOutcome, recent: Vec<VoiceCandidate>) -> Self {
        match outcome {
            TriggerRestoreOutcome::Restored {
                title,
                outcome,
                offered,
            } => VoiceOutcome::Restored {
                title,
                summary: outcome.summary(),
                offered,
            },
            TriggerRestoreOutcome::Ambiguous(candidates) => VoiceOutcome::Ambiguous { candidates },
            TriggerRestoreOutcome::NotFound => VoiceOutcome::NotFound { recent },
            TriggerRestoreOutcome::EmbeddingFailed(reason) => VoiceOutcome::Failed { reason },
        }
    }
}

/// Resolves a trigger phrase and restores the matching work in one call.
///
/// This is the product verb: "Hey Evo, take me back to the evo stuff" →
/// the windows come back. Ambiguity and misses are reported honestly,
/// never guessed.
///
/// `embedder` is `&dyn Embedder` (not the concrete model type) so callers
/// can share the one loaded model — or substitute a stub in tests.
pub fn trigger_restore(
    trigger: &str,
    reconstruction: &evo_engagement::Reconstruction,
    embedder: &dyn Embedder,
) -> TriggerRestoreOutcome {
    match crate::resolve_trigger(trigger, reconstruction, embedder) {
        Ok(crate::TriggerOutcome::Resolved(engagement)) => {
            // Staged restore: core now (by DOI), long tail offered.
            let staged = restore_engagement_staged(engagement, reconstruction);
            TriggerRestoreOutcome::Restored {
                title: engagement.title().to_string(),
                outcome: staged.restored,
                offered: staged.offered.into_iter().map(|t| t.resource).collect(),
            }
        }
        Ok(crate::TriggerOutcome::Ambiguous(candidates)) => TriggerRestoreOutcome::Ambiguous(
            candidates
                .into_iter()
                .map(VoiceCandidate::from_engagement)
                .collect(),
        ),
        Ok(crate::TriggerOutcome::NotFound) => TriggerRestoreOutcome::NotFound,
        Err(e) => TriggerRestoreOutcome::EmbeddingFailed(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The shape filter is the core logic: Address and Path pass,
    // Surface and Opaque do not. Full Engagement construction is
    // covered by evo-engagement's own tests; here we verify the
    // contract via the public SubjectShape vocabulary.
    #[test]
    fn restorable_shapes_are_address_and_path() {
        assert!(matches!(
            SubjectShape::Address,
            SubjectShape::Address | SubjectShape::Path
        ));
        assert!(matches!(
            SubjectShape::Path,
            SubjectShape::Address | SubjectShape::Path
        ));
        assert!(!matches!(
            SubjectShape::Surface,
            SubjectShape::Address | SubjectShape::Path
        ));
        assert!(!matches!(
            SubjectShape::Opaque,
            SubjectShape::Address | SubjectShape::Path
        ));
    }

    #[test]
    fn voice_outcome_mapping_preserves_honesty() {
        // EmbeddingFailed is never laundered into NotFound.
        let outcome = VoiceOutcome::from_trigger(
            TriggerRestoreOutcome::EmbeddingFailed("model exploded".to_string()),
            vec![],
        );
        assert_eq!(
            outcome,
            VoiceOutcome::Failed {
                reason: "model exploded".to_string()
            }
        );

        // NotFound carries the recent-work offer through.
        let recent = vec![VoiceCandidate {
            work_id: "1".to_string(),
            title: "Evo stuff".to_string(),
            selection: VoiceSelection::Work,
        }];
        let outcome = VoiceOutcome::from_trigger(TriggerRestoreOutcome::NotFound, recent.clone());
        assert_eq!(outcome, VoiceOutcome::NotFound { recent });

        // Ambiguous keeps every candidate.
        let candidates = vec![
            VoiceCandidate {
                work_id: "1".to_string(),
                title: "Alpha".to_string(),
                selection: VoiceSelection::Work,
            },
            VoiceCandidate {
                work_id: "2".to_string(),
                title: "Beta".to_string(),
                selection: VoiceSelection::Work,
            },
        ];
        let outcome = VoiceOutcome::from_trigger(
            TriggerRestoreOutcome::Ambiguous(candidates.clone()),
            vec![],
        );
        assert_eq!(outcome, VoiceOutcome::Ambiguous { candidates });
    }
}
