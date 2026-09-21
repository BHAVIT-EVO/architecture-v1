//! Episodes and the Attention Ledger.
//!
//! An **Episode** is one continuous stretch of witnessed presence: a sitting.
//! Episodes are separated by gaps longer than the presence horizon, because
//! beyond that gap Evo has no evidence the person was still there.
//!
//! The **Attention Ledger** records, per resource, how much attention Evo can
//! honestly claim was spent on it and how often the person returned to it.
//! Attention is the scarce quantity that distinguishes work from encounter,
//! and it is measured, never assumed.

use crate::params::EngagementParams;
use crate::resource::{ActCharacter, Resource};

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, SystemTime};

/// One witnessed act involving one resource at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Act {
    resource: Resource,
    character: ActCharacter,
    at: SystemTime,
}

impl Act {
    /// Builds an act from its canonical parts.
    pub fn new(resource: Resource, character: ActCharacter, at: SystemTime) -> Self {
        Self {
            resource,
            character,
            at,
        }
    }

    /// The resource this act involved.
    pub fn resource(&self) -> &Resource {
        &self.resource
    }

    /// How the act was witnessed.
    pub fn character(&self) -> ActCharacter {
        self.character
    }

    /// When the act was witnessed.
    pub fn at(&self) -> SystemTime {
        self.at
    }
}

/// One continuous stretch of witnessed presence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    acts: Vec<Act>,
}

impl Episode {
    /// The acts of this sitting, in canonical time order.
    pub fn acts(&self) -> &[Act] {
        &self.acts
    }

    /// When the sitting began.
    pub fn started_at(&self) -> Option<SystemTime> {
        self.acts.first().map(Act::at)
    }

    /// When the last witnessed act of the sitting occurred.
    pub fn ended_at(&self) -> Option<SystemTime> {
        self.acts.last().map(Act::at)
    }

    /// The distinct resource subjects witnessed during this sitting.
    pub fn subjects(&self) -> BTreeSet<String> {
        self.acts
            .iter()
            .map(|act| act.resource().subject().to_string())
            .collect()
    }

    /// Whether any act in this sitting is direct evidence a person was
    /// present. A sitting made entirely of incidental change is not evidence
    /// of presence at all — it is what a machine does while nobody is there.
    pub fn has_human_evidence(&self) -> bool {
        self.acts
            .iter()
            .any(|act| act.character().is_human_evidence())
    }
}

/// Splits a canonical, time-ordered act sequence into Episodes.
///
/// The input is sorted by witnessed moment first, so the result depends only
/// on the canonical evidence and never on arrival order. This is what makes
/// live processing and replay produce identical Episodes.
pub fn segment_episodes(mut acts: Vec<Act>, params: &EngagementParams) -> Vec<Episode> {
    // Stable canonical ordering: by moment, then by subject, so identical
    // evidence always segments identically regardless of input order.
    acts.sort_by(|left, right| {
        left.at
            .cmp(&right.at)
            .then_with(|| left.resource.subject().cmp(right.resource.subject()))
    });

    let mut episodes: Vec<Episode> = Vec::new();
    let mut current: Vec<Act> = Vec::new();

    for act in acts {
        match current.last() {
            Some(previous) => {
                let gap = act
                    .at
                    .duration_since(previous.at)
                    .unwrap_or(Duration::ZERO);
                if gap > params.presence_horizon {
                    episodes.push(Episode { acts: std::mem::take(&mut current) });
                }
            }
            None => {}
        }
        current.push(act);
    }
    if !current.is_empty() {
        episodes.push(Episode { acts: current });
    }
    episodes
}

/// What Evo can claim about the attention spent on one resource.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AttentionRecord {
    /// Total attention Evo can honestly claim was spent here.
    pub attention: Duration,
    /// Attention broken down by the sitting it was spent in.
    ///
    /// Kept separately because the total alone cannot distinguish a habit from
    /// work: forty three-second visits and one two-minute stretch have the same
    /// total, and only the second is someone working. Every judgement about
    /// depth is therefore made per sitting, never on the lifetime sum.
    pub per_episode: BTreeMap<usize, Duration>,
    /// How many acts involved this resource.
    pub acts: usize,
    /// How many acts involving this resource were direct human evidence.
    pub human_acts: usize,
    /// The indices of the Episodes in which this resource appeared.
    pub episodes: BTreeSet<usize>,
    /// The indices of the Episodes in which a *person* was witnessed acting on
    /// this resource — an act of attention or a deliberate, durable act.
    ///
    /// A subset of [`AttentionRecord::episodes`], and deliberately not the same
    /// question as attention: a person can bring a window forward as the last
    /// act of a sitting, or commit, and Evo will honestly measure no dwell at
    /// all. Kept per sitting because whether a person acted *during this body of
    /// work* is what matters, and the lifetime count cannot answer that.
    pub human_episodes: BTreeSet<usize>,
    /// The last moment this resource was witnessed.
    pub last_seen: Option<SystemTime>,
    /// The first moment this resource was witnessed.
    pub first_seen: Option<SystemTime>,
}

impl AttentionRecord {
    /// How many distinct sittings the person returned to this resource in.
    pub fn recurrence(&self) -> usize {
        self.episodes.len()
    }

    /// The attention spent on this resource during one sitting.
    pub fn attention_in(&self, episode: usize) -> Duration {
        self.per_episode
            .get(&episode)
            .copied()
            .unwrap_or(Duration::ZERO)
    }

    /// The most attention this resource received in any single sitting.
    pub fn deepest_sitting(&self) -> Duration {
        self.per_episode
            .values()
            .copied()
            .max()
            .unwrap_or(Duration::ZERO)
    }
}

/// The per-resource attention record for a whole canonical history.
#[derive(Debug, Clone, Default)]
pub struct AttentionLedger {
    records: BTreeMap<String, AttentionRecord>,
}

impl AttentionLedger {
    /// Builds the ledger from segmented Episodes.
    ///
    /// Attention accrues only to **attentional** acts, and only for the
    /// interval until the next act *in the same sitting* — and only when that
    /// interval is short enough to be one continuous stretch of attention
    /// ([`EngagementParams::max_interval_attention`]). The final act of a
    /// sitting accrues nothing, and neither does an act followed by a long
    /// silence: in both cases Evo did not witness how long the person stayed, so
    /// it declines to claim anything. This under-claims rather than inventing
    /// attention, which is the required direction of error.
    ///
    /// Incidental acts (a file changing) accrue **no** attention. A resource
    /// that only ever changes, and is never attended, therefore carries zero
    /// attention mass no matter how often it changes. This is the mechanism —
    /// with no path list, no application list, and no frequency cutoff — by
    /// which machine churn can never become a body of work.
    ///
    /// # Authority change: silence is not attention (RFC-0003, §17, §23)
    ///
    /// **The old rule.** An attentional act accrued
    /// `min(gap_to_next_act, max_interval_attention)`. The cap existed so an
    /// overnight gap would not credit eight hours of attention.
    ///
    /// **Why it prevented the product from working.** The cap did not stop
    /// silence becoming attention; it only bounded how much. Any act followed by
    /// a gap longer than the cap was credited the entire cap — ten full minutes
    /// of attention Evo never witnessed, awarded precisely *because* it witnessed
    /// nothing. On the real machine this is how a body of work nobody worked in
    /// reached Home: two windows, four sightings each, twenty-one seconds of
    /// actually-witnessed attention, and one 972-second silence before the
    /// person's next act elsewhere. The silence contributed 600 seconds, the
    /// group cleared the depth condition on the strength of it, and Evo presented
    /// as resumable work an occasion that never happened. Every threshold in the
    /// depth test is measured against this quantity, so a quantity that inflates
    /// on absence of evidence makes all of them meaningless.
    ///
    /// **The new rule.** An interval is credited only if it is itself evidence of
    /// continuous attention — that is, if it is no longer than
    /// `max_interval_attention`. A longer gap accrues nothing, exactly as the end
    /// of a sitting does, and for the same reason. The parameter now means what
    /// its name says: the longest gap between two witnessed acts that still
    /// counts as one stretch of attention. No new threshold is introduced and no
    /// resource, application or path is named; the rule is uniform, and the only
    /// direction it can err in is under-claiming.
    pub fn build(episodes: &[Episode], params: &EngagementParams) -> Self {
        let mut records: BTreeMap<String, AttentionRecord> = BTreeMap::new();

        for (episode_index, episode) in episodes.iter().enumerate() {
            let acts = episode.acts();
            for (position, act) in acts.iter().enumerate() {
                let subject = act.resource().subject().to_string();
                let record = records.entry(subject).or_default();
                record.acts += 1;
                if act.character().is_human_evidence() {
                    record.human_acts += 1;
                    record.human_episodes.insert(episode_index);
                }
                record.episodes.insert(episode_index);
                record.last_seen = Some(match record.last_seen {
                    Some(previous) if previous > act.at() => previous,
                    _ => act.at(),
                });
                record.first_seen = Some(match record.first_seen {
                    Some(previous) if previous < act.at() => previous,
                    _ => act.at(),
                });

                if act.character() == ActCharacter::Attentional {
                    if let Some(next) = acts.get(position + 1) {
                        let gap = next
                            .at()
                            .duration_since(act.at())
                            .unwrap_or(Duration::ZERO);
                        // A gap longer than one continuous stretch of attention
                        // is silence, and silence is not attention. Credited
                        // whole or not at all — never truncated into a claim.
                        if gap <= params.max_interval_attention {
                            record.attention += gap;
                            *record
                                .per_episode
                                .entry(episode_index)
                                .or_insert(Duration::ZERO) += gap;
                        }
                    }
                }
            }
        }

        Self { records }
    }

    /// The record for one resource subject, when it was witnessed.
    pub fn get(&self, subject: &str) -> Option<&AttentionRecord> {
        self.records.get(subject)
    }

    /// Every witnessed subject, in canonical order.
    pub fn subjects(&self) -> impl Iterator<Item = &String> {
        self.records.keys()
    }

    /// The total attention across every witnessed resource.
    pub fn total_attention(&self) -> Duration {
        self.records
            .values()
            .map(|record| record.attention)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::Resource;

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn focus(subject: &str, secs: u64) -> Act {
        Act::new(
            Resource::new(subject, "OBS-WINDOW-FOCUS-GAINED"),
            ActCharacter::Attentional,
            at(secs),
        )
    }

    fn save(subject: &str, secs: u64) -> Act {
        Act::new(
            Resource::new(subject, "OBS-FILE-SAVED"),
            ActCharacter::Incidental,
            at(secs),
        )
    }

    #[test]
    fn a_long_absence_starts_a_new_sitting() {
        let params = EngagementParams::default();
        let acts = vec![
            focus("a", 0),
            focus("b", 60),
            // Well beyond the presence horizon.
            focus("c", 60 + 60 * 60),
        ];
        let episodes = segment_episodes(acts, &params);
        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[0].acts().len(), 2);
        assert_eq!(episodes[1].acts().len(), 1);
    }

    #[test]
    fn segmentation_is_independent_of_input_order() {
        let params = EngagementParams::default();
        let forward = segment_episodes(
            vec![focus("a", 0), focus("b", 30), focus("c", 60)],
            &params,
        );
        let shuffled = segment_episodes(
            vec![focus("c", 60), focus("a", 0), focus("b", 30)],
            &params,
        );
        assert_eq!(forward, shuffled);
    }

    #[test]
    fn attention_accrues_only_until_the_next_act_in_the_sitting() {
        let params = EngagementParams::default();
        let episodes = segment_episodes(vec![focus("a", 0), focus("b", 120)], &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        assert_eq!(ledger.get("a").unwrap().attention, Duration::from_secs(120));
        // The final act of a sitting claims nothing.
        assert_eq!(ledger.get("b").unwrap().attention, Duration::ZERO);
    }

    // A gap longer than one continuous stretch of attention is silence, and
    // silence accrues nothing — not the cap, not a fraction of it. Crediting the
    // cap here is how a group that received twenty-one witnessed seconds cleared
    // a two-minute depth condition on a real machine.
    #[test]
    fn a_long_silence_accrues_no_attention_at_all() {
        let params = EngagementParams::default();
        // Focused, then nothing until just inside the presence horizon, so the
        // gap stays in one sitting and only the interval rule decides.
        let gap = params.presence_horizon.as_secs() - 60;
        assert!(
            gap > params.max_interval_attention.as_secs(),
            "the fixture must actually exercise a longer-than-continuous gap"
        );
        let episodes = segment_episodes(vec![focus("a", 0), focus("b", gap)], &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        assert_eq!(
            ledger.get("a").unwrap().attention,
            Duration::ZERO,
            "Evo witnessed a focus and then nothing; it claims nothing"
        );
    }

    // The boundary itself is attention: an interval exactly as long as one
    // continuous stretch is credited in full, so the rule is a threshold on
    // whether the interval is evidence, not a ceiling on how much it may claim.
    #[test]
    fn an_interval_at_the_boundary_is_credited_in_full() {
        let params = EngagementParams::default();
        let held = params.max_interval_attention.as_secs();
        let episodes = segment_episodes(vec![focus("a", 0), focus("b", held)], &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        assert_eq!(
            ledger.get("a").unwrap().attention,
            params.max_interval_attention
        );
    }

    /// The core anti-noise property, stated as an invariant: a resource that
    /// only ever *changes* and is never *attended* accumulates no attention,
    /// regardless of how many times it changes.
    #[test]
    fn incidental_change_never_accrues_attention_however_frequent() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        for step in 0..500 {
            acts.push(save("/some/machine/written/cache.bin", step * 2));
        }
        let episodes = segment_episodes(acts, &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        let record = ledger.get("/some/machine/written/cache.bin").unwrap();
        assert_eq!(record.acts, 500);
        assert_eq!(record.human_acts, 0);
        assert_eq!(
            record.attention,
            Duration::ZERO,
            "change without attention is not work, at any frequency"
        );
    }

    #[test]
    fn recurrence_counts_distinct_sittings() {
        let params = EngagementParams::default();
        let acts = vec![
            focus("doc", 0),
            focus("other", 60),
            focus("doc", 3 * 60 * 60),
            focus("other", 3 * 60 * 60 + 60),
        ];
        let episodes = segment_episodes(acts, &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        assert_eq!(ledger.get("doc").unwrap().recurrence(), 2);
    }

    #[test]
    fn a_sitting_of_pure_machine_change_carries_no_human_evidence() {
        let params = EngagementParams::default();
        let episodes = segment_episodes(vec![save("/a", 0), save("/b", 1)], &params);
        assert!(!episodes[0].has_human_evidence());
    }
}
