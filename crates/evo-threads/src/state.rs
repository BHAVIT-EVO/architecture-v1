//! Internal engine state: event-sourced, fully deterministic.
//!
//! Product statement: state is a pure fold over the event log. Ordered maps
//! everywhere; no randomness; no iteration order that could ever leak into an
//! output. The same log replayed twice must be provably identical — not just
//! at the API surface, but bit for bit.

use crate::config::Config;
use crate::trace::Decision;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A resource's standing inside one episode. Attention here is already
/// dwell-discounted: a window left focused overnight contributed one interval.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Participant {
    pub attention: f64,
    pub last_contact: u64,
    /// When this resource first entered the episode. Distinguishes a
    /// surface that attended the sitting's opening from one first touched
    /// once the work was already producing.
    pub first_contact: Option<u64>,
    /// Watermark preventing double-counting when an episode is committed,
    /// reopened, and committed again.
    pub committed_attention: f64,
    pub remaining_credit: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ProdKind {
    Typed,
    Saved,
    Mutated,
    CommandRan,
    Downloaded,
    Declared,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct CreditDetail {
    pub amount: u64,
    pub committed: u64,
    pub last_kind: ProdKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct EpisodeState {
    pub id: EpisodeId,
    pub span: (u64, u64),
    pub open: bool,
    /// When the closure happened; drives the resumption window. The span's end
    /// is the attention boundary; `closed_at` is the decision instant.
    pub closed_at: u64,
    pub assigned: Option<ThreadId>,
    /// Parked episodes are retained, unguessed, and remain resumable and
    /// promotable. They are not threads and appear in no thread view.
    pub parked: bool,
    pub participants: BTreeMap<String, Participant>,
    /// Resources whose production was witnessed inside this episode — the
    /// episode-local record from which thread anchors are committed.
    pub credits: BTreeMap<String, CreditDetail>,
    /// The currently attended set (attention-recency pruned).
    pub focus: BTreeSet<String>,
    pub dips: Vec<(u64, u64)>,
    pub last_person_ts: u64,
    pub fork_candidates: Option<Vec<ThreadId>>,
    /// When this episode first saw person production, if it ever did. The
    /// boundary between prelude and consultation: a passive resource present
    /// only before this moment attended the sitting's opening; one first
    /// contacted after it was consulted while the work was producing.
    pub first_production_ts: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ThreadState {
    pub id: ThreadId,
    pub born_at: u64,
    pub last_activity: u64,
    pub last_commit_ts: u64,
    pub episodes: BTreeSet<EpisodeId>,
    /// Identity carriers. Anchors decay never; they are what the person made
    /// or deliberately returned to.
    pub anchors: BTreeMap<String, Anchor>,
    /// Context memory: decays with `companion_halflife`.
    pub companions: BTreeMap<String, f64>,
    /// Companions whose weight was earned while the episode was already
    /// producing — consulted mid-flow, not merely co-present when the sitting
    /// began or after it ended. A focus-only surface (a player, a sidebar)
    /// may only vote an episode into this work if it earned this flag;
    /// otherwise ambience could carry production-free sittings into whatever
    /// work it happened to attend. `BTreeSet`, not a weight, because the
    /// question is categorical: was this companionship witnessed *inside*
    /// the work's own flow, or beside it?
    pub in_flow_companions: BTreeSet<String>,
    pub fork: Option<Vec<ThreadId>>,
}

/// Per-resource factual ledger. Times are only ever compared; nothing here is
/// content, and nothing here is statistics.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct ResourceLedger {
    pub last_person_contact: Option<u64>,
    pub last_person_focus: Option<u64>,
    pub pending_command: Option<u64>,
    pub last_typed: Option<u64>,
    pub last_saved: Option<u64>,
    pub last_churn_passing_mutated: Option<u64>,
    pub downloaded_at: Option<u64>,
    pub used_after_download: bool,
    /// Set when Execution opens the resource; cleared by the first person
    /// contact (the handoff). While set, the resource is "already open" for
    /// restore purposes and contributes nothing else.
    pub execution_open: bool,
    // Presence-history flags defining the "passive surface" caste: a resource
    // that has only ever been focused (never visited, never produced on) is
    // ambience — an always-on player or sidebar — and may never be restored.
    pub had_focus: bool,
    pub had_visited: bool,
    pub had_typed: bool,
    pub had_saved: bool,
    pub had_downloaded: bool,
    pub had_command: bool,
}

impl ResourceLedger {
    pub fn is_passive(&self) -> bool {
        self.had_focus
            && !self.had_visited
            && !self.had_typed
            && !self.had_saved
            && !self.had_downloaded
            && !self.had_command
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct State {
    pub cfg: Config,
    pub now: u64,
    pub started: bool,
    pub next_thread: u64,
    pub next_episode: u64,
    pub threads: BTreeMap<ThreadId, ThreadState>,
    pub episodes: BTreeMap<EpisodeId, EpisodeState>,
    pub open: Option<EpisodeId>,
    pub detour: Option<Detour>,
    pub ledger: BTreeMap<String, ResourceLedger>,
    /// resource -> threads holding it as an anchor. Contestation lives here
    /// and is always derived, never cached into threads.
    pub anchor_index: BTreeMap<String, BTreeSet<ThreadId>>,
    pub quarantined: BTreeSet<String>,
    /// Canonical (sorted) declared pairs.
    pub same_work: BTreeSet<(String, String)>,
    pub separate_work: BTreeSet<(String, String)>,
    pub decisions: Vec<Decision>,
    /// Set whenever a commit or declaration may enable parked episodes;
    /// the promotion fixpoint runs at the end of `apply`.
    pub sweep_dirty: bool,
    /// Normalized download target -> first landing time. A re-download of the
    /// same target is state, not new production: identity costs once.
    pub downloaded_targets: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Detour {
    pub start: u64,
    pub events: Vec<Event>,
}

impl State {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            now: 0,
            started: false,
            next_thread: 1,
            next_episode: 1,
            threads: BTreeMap::new(),
            episodes: BTreeMap::new(),
            open: None,
            detour: None,
            ledger: BTreeMap::new(),
            anchor_index: BTreeMap::new(),
            quarantined: BTreeSet::new(),
            same_work: BTreeSet::new(),
            separate_work: BTreeSet::new(),
            decisions: Vec::new(),
            sweep_dirty: false,
            downloaded_targets: BTreeMap::new(),
        }
    }

    /// Threads whose profile (anchors or companions) contains the resource.
    /// The denominator of all ubiquity reasoning.
    pub fn profile_thread_count(&self, resource: &str) -> usize {
        self.threads
            .values()
            .filter(|t| t.anchors.contains_key(resource) || t.companions.contains_key(resource))
            .count()
    }

    pub fn ledger_mut(&mut self, resource: &str) -> &mut ResourceLedger {
        self.ledger.entry(resource.to_string()).or_default()
    }

    pub(crate) fn push(&mut self, d: Decision) {
        self.decisions.push(d);
    }
}
