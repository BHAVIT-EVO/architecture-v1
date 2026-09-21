//! Stages 2–3: anchors, and episode-to-thread assignment.
//!
//! Product statements, in one place:
//! * Evidence has ranks and keeps them. Declarations override everything;
//!   uncontested anchors decide; configuration corroborates; co-presence,
//!   names, and folders decorate and never vote.
//! * A resource shared by two or more works votes for *neither*. That is
//!   what makes one browser, one terminal, or one repo serving two real works
//!   safe — its reality is never in question, only its jurisdiction.
//! * Under a near-tie the engine forks: alternatives are retained and shown.
//!   It never breaks a tie by guessing.
//! * Two established works merge only when the person says so. No function
//!   reachable from inference can merge threads; the merge code path takes a
//!   declaration or it does not run.

use crate::state::{EpisodeState, ThreadState};
use crate::trace::{Decision, ParkReason, Witness};
use crate::types::*;
use crate::Engine;
use std::collections::{BTreeMap, BTreeSet};

/// The computed witness set against live threads, before any decision.
struct Candidates {
    /// thread -> summed episode credit of uncontested anchors pointing at it.
    anchor_support: BTreeMap<ThreadId, u64>,
    /// thread -> witnesses collected for it.
    witnesses: BTreeMap<ThreadId, Vec<Witness>>,
    /// thread -> suppressed configuration weight.
    config_support: BTreeMap<ThreadId, f64>,
    /// production this episode owns that no live thread has ever anchored.
    new_production: u64,
    total_production: u64,
    pinned: BTreeSet<ThreadId>,
    forbidden: BTreeSet<ThreadId>,
}

impl Engine {
    pub(crate) fn assign_episode(&mut self, ep_id: EpisodeId) {
        let ep = match self.st.episodes.get(&ep_id) {
            Some(ep) => ep.clone(),
            None => return,
        };
        let cfg = self.st.cfg.clone();
        let c = self.compute_candidates(&ep);

        // 1. The person's word outranks the stream.
        let pins: Vec<ThreadId> = c
            .pinned
            .iter()
            .filter(|t| !c.forbidden.contains(t))
            .copied()
            .collect();
        if pins.len() == 1 {
            let t = pins[0];
            let mut w = c.witnesses.get(&t).cloned().unwrap_or_default();
            w.push(Witness::UserPin);
            self.finish_assign(&ep, t, w);
            return;
        }

        // 2. Uncontested anchors decide — when they dominate both rival anchor
        //    support and the episode's own new production. Old identity must
        //    outweigh new identity to claim the episode.
        let anchor_order: Vec<(ThreadId, u64)> = sorted_support(&c.anchor_support);
        if let Some(&(best_t, best_s)) = anchor_order.first() {
            let second_s = anchor_order.get(1).map(|x| x.1).unwrap_or(0);
            let dominates = (best_s as f64) >= cfg.anchor_margin_ratio * (second_s as f64).max(1.0)
                && (best_s as f64)
                    >= cfg.anchor_margin_ratio * (c.new_production as f64).max(1.0);
            if dominates && !c.forbidden.contains(&best_t) {
                let w = c.witnesses.get(&best_t).cloned().unwrap_or_default();
                self.finish_assign(&ep, best_t, w);
                return;
            }
            // A genuine tie between anchor-backed works: retain, never guess.
            if second_s > 0
                && (second_s as f64) * cfg.anchor_margin_ratio >= best_s as f64
            {
                let candidates: Vec<ThreadId> = anchor_order.iter().map(|x| x.0).collect();
                let all_w = flatten_witnesses(&c.witnesses);
                self.fork_episode(&ep, candidates, all_w);
                return;
            }
        }

        // 3. Configuration may carry an assignment — but only for a
        //    production-free episode. A productive episode's own production
        //    always outranks borrowed context; otherwise one shared browser
        //    could drag tomorrow's new work into yesterday's thread.
        if c.total_production == 0 {
            if let Some((t, ws)) = self.best_config_candidate(&ep, &c) {
                if !c.forbidden.contains(&t) {
                    self.finish_assign(&ep, t, ws);
                    return;
                }
            }
        }

        // 3b. A contested anchor votes for neither side. When it is the only
        //     old evidence in the episode and nothing else distinguishes its
        //     holders, the honest outcome is the fork — never a coin flip,
        //     and never a third work glued to the shared resource.
        if c.anchor_support.is_empty() && c.new_production == 0 {
            let mut contested: Vec<ThreadId> = c
                .witnesses
                .iter()
                .filter(|(_, ws)| {
                    ws.iter()
                        .any(|w| matches!(w, Witness::SharedContestedAnchor(_)))
                })
                .map(|(t, _)| *t)
                .filter(|t| !c.forbidden.contains(t))
                .collect();
            contested.sort();
            if contested.len() >= 2 {
                let all_w = flatten_witnesses(&c.witnesses);
                self.fork_episode(&ep, contested, all_w);
                return;
            }
        }

        // 4. Production the world has never seen founds a work — if it crosses
        //    the birth floor. Below the floor the episode is retained, not
        //    guessed into being and not guessed away.
        if c.total_production >= cfg.thread_birth_min_production {
            self.spawn_thread_for_episode(&ep, birth_witnesses(&ep, &c));
            return;
        }
        if c.total_production > 0 {
            self.park_episode(&ep, ParkReason::WeakProduction, flatten_witnesses(&c.witnesses));
            return;
        }

        // 5. A configuration that reforms across distinct sessions is a work
        //    even when nothing is ever "produced" — the web-app reality.
        if let Some(qualified) = self.recurrence_qualify(&ep) {
            self.spawn_recurrence_thread(&ep, qualified);
            return;
        }

        // 5b. **Workspace setup**: the person deliberately opened a set of
        //     surfaces — sustained attention (>3s) on each of several
        //     distinct resources in one sitting — intending to work later.
        //     The research (Recall's design, TaskTracer): this pattern is a
        //     setup, not consumption, and the system should recognize it
        //     immediately rather than waiting for production that hasn't
        //     happened yet. The engagement floor prevents the false
        //     positive: tabs bounced past in under 3 seconds are not setup,
        //     they are navigation.
        if self.is_workspace_setup(&ep) {
            let surfaces: Vec<String> = ep
                .participants
                .iter()
                .filter(|(_, p)| p.attention >= 3000.0)
                .map(|(r, _)| r.clone())
                .collect();
            self.spawn_recurrence_thread(&ep, surfaces);
            return;
        }

        // 6. Attention alone is not a work. Retained; never minted.
        self.park_episode(&ep, ParkReason::ConsumptionOnly, flatten_witnesses(&c.witnesses));
    }

    /// Whether an episode matches the workspace-setup pattern: several
    /// distinct surfaces, each with sustained attention (≥3s), in one
    /// sitting, with no production. This is "the person arranged their
    /// tools and intends to work" — the pattern that a 1-minute app-opening
    /// spree produces. The threshold: at least 3 distinct surfaces at ≥3s
    /// each (below that, it's just reading or navigation).
    fn is_workspace_setup(&self, ep: &EpisodeState) -> bool {
        // Production already handled upstream (rule 4): setup has none.
        if !ep.credits.is_empty() {
            return false;
        }
        let sustained: Vec<&String> = ep
            .participants
            .iter()
            .filter(|(_, p)| p.attention >= 3000.0)
            .map(|(r, _)| r)
            .collect();
        // At least 3 distinct surfaces with sustained attention each:
        // opening 2 things is a glance; 3+ arranged surfaces is a setup.
        sustained.len() >= 3
    }

    fn compute_candidates(&self, ep: &EpisodeState) -> Candidates {
        let cfg = &self.st.cfg;
        let mut anchor_support: BTreeMap<ThreadId, u64> = BTreeMap::new();
        let mut witnesses: BTreeMap<ThreadId, Vec<Witness>> = BTreeMap::new();
        let mut config_support: BTreeMap<ThreadId, f64> = BTreeMap::new();
        let mut new_production: u64 = 0;
        let mut total_production: u64 = 0;

        for (r, credit) in &ep.credits {
            total_production += credit.amount;
            match self.st.anchor_index.get(r) {
                Some(set) if !set.is_empty() => {
                    if set.len() == 1 {
                        let t = *set.iter().next().unwrap();
                        *anchor_support.entry(t).or_default() += credit.amount;
                        witnesses
                            .entry(t)
                            .or_default()
                            .push(Witness::SharedUncontestedAnchor(r.clone()));
                    } else {
                        // Recorded on every holder — and given zero support on
                        // each. Shared identity is real and decides nothing.
                        for t in set {
                            witnesses
                                .entry(*t)
                                .or_default()
                                .push(Witness::SharedContestedAnchor(r.clone()));
                        }
                    }
                }
                _ => new_production += credit.amount,
            }
        }

        for (r, p) in &ep.participants {
            if ep.credits.contains_key(r) {
                continue;
            }
            // A passive surface (only ever focused — never visited, never
            // produced on) is ambience: an always-on player or sidebar. It
            // may vote an episode into a work only where THIS thread
            // witnessed it mid-flow — consulted while the work was actually
            // producing — because that is the one fact that separates a
            // reference the person genuinely uses in the work from a window
            // that merely happened to be open beside everything. Restore
            // sets enforce the same caste via `is_ambient`.
            let passive = self
                .st
                .ledger
                .get(r)
                .map(|l| l.is_passive())
                .unwrap_or(false);
            if p.attention < cfg.min_meaningful_attention as f64 {
                continue;
            }
            let ubiquity = self.st.profile_thread_count(r);
            if ubiquity == 0 {
                continue;
            }
            let mut suppression = 1.0 / (1.0 + ubiquity as f64);
            if passive {
                let any_in_flow = self.st.threads.values().any(|t| {
                    t.in_flow_companions.contains(r)
                        && (t.anchors.contains_key(r) || t.companions.contains_key(r))
                });
                if !any_in_flow {
                    continue;
                }
                // Even an in-flow-earned passive surface votes with reduced
                // voice: it was witnessed inside one flow, not produced on.
                suppression *= 0.5;
            }
            let w = p.attention * suppression;
            for (tid, t) in &self.st.threads {
                if t.anchors.contains_key(r) || t.companions.contains_key(r) {
                    *config_support.entry(*tid).or_default() += w;
                    witnesses
                        .entry(*tid)
                        .or_default()
                        .push(Witness::ConfigOverlap {
                            resource: r.clone(),
                            weight: w,
                        });
                }
            }
        }

        // Informational: resumption within the day corroborates, never decides.
        for (tid, t) in &self.st.threads {
            let gap = ep.span.0.saturating_sub(t.last_activity);
            if gap <= cfg.resume_window {
                witnesses
                    .entry(*tid)
                    .or_default()
                    .push(Witness::SameDayResumption { gap_ms: gap });
            }
        }

        let mut pinned: BTreeSet<ThreadId> = BTreeSet::new();
        let mut forbidden: BTreeSet<ThreadId> = BTreeSet::new();
        let contains = |r: &str| ep.participants.contains_key(r) || ep.credits.contains_key(r);
        for (a, b) in &self.st.same_work {
            for (x, y) in [(a, b), (b, a)] {
                if contains(x) {
                    for (tid, t) in &self.st.threads {
                        if t.anchors.contains_key(y) || t.companions.contains_key(y) {
                            pinned.insert(*tid);
                        }
                    }
                }
            }
        }
        for (a, b) in &self.st.separate_work {
            for (x, y) in [(a, b), (b, a)] {
                if contains(x) {
                    for (tid, t) in &self.st.threads {
                        if t.anchors.contains_key(y) || t.companions.contains_key(y) {
                            forbidden.insert(*tid);
                            witnesses
                                .entry(*tid)
                                .or_default()
                                .push(Witness::UserNeverLink {
                                    a: a.clone(),
                                    b: b.clone(),
                                });
                        }
                    }
                }
            }
        }

        Candidates {
            anchor_support,
            witnesses,
            config_support,
            new_production,
            total_production,
            pinned,
            forbidden,
        }
    }

    /// Configuration rule, factored for reuse by the parked-episode fixpoint.
    /// Returns the winning thread with its witnesses when configuration alone
    /// clears both the weight floor and the margin.
    fn best_config_candidate(
        &self,
        _ep: &EpisodeState,
        c: &Candidates,
    ) -> Option<(ThreadId, Vec<Witness>)> {
        let cfg = &self.st.cfg;
        let mut order: Vec<(ThreadId, f64)> = c.config_support.iter().map(|(t, w)| (*t, *w)).collect();
        order.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        let &(bt, bw) = order.first()?;
        let second = order.get(1).map(|x| x.1).unwrap_or(0.0);
        if bw >= cfg.config_min_weight
            && bw >= cfg.config_margin_ratio * second.max(1.0)
        {
            Some((bt, c.witnesses.get(&bt).cloned().unwrap_or_default()))
        } else {
            None
        }
    }

    /// Assign with full provenance. This is the only way episodes join works.
    fn finish_assign(&mut self, ep: &EpisodeState, tid: ThreadId, witnesses: Vec<Witness>) {
        if let Some(e) = self.st.episodes.get_mut(&ep.id) {
            e.assigned = Some(tid);
            e.parked = false;
        }
        if let Some(t) = self.st.threads.get_mut(&tid) {
            t.episodes.insert(ep.id);
            t.last_activity = t.last_activity.max(ep.span.1);
        }
        self.commit_episode(ep.id);
        self.st.push(Decision::EpisodeAssigned {
            thread: tid,
            episode: ep.id,
            witnesses,
        });
        self.st.sweep_dirty = true;
    }

    fn park_episode(&mut self, ep: &EpisodeState, reason: ParkReason, witnesses: Vec<Witness>) {
        if let Some(e) = self.st.episodes.get_mut(&ep.id) {
            e.parked = true;
        }
        self.st.push(Decision::EpisodeParked {
            episode: ep.id,
            reason,
        });
        // The parked record keeps its witnesses for the audit trail even when
        // no decision consumed them.
        let _ = witnesses;
    }

    fn fork_episode(&mut self, ep: &EpisodeState, candidates: Vec<ThreadId>, witnesses: Vec<Witness>) {
        if let Some(e) = self.st.episodes.get_mut(&ep.id) {
            e.parked = true;
            e.fork_candidates = Some(candidates.clone());
        }
        for tid in &candidates {
            if let Some(t) = self.st.threads.get_mut(tid) {
                t.fork = Some(candidates.clone());
            }
        }
        self.st.push(Decision::Fork {
            episode: ep.id,
            candidates,
            witnesses,
        });
    }

    /// Birth through production. Spawning is cheap and safe: a mistaken work
    /// is one click from merging, while a mistaken merge is a breach of trust.
    fn spawn_thread_for_episode(&mut self, ep: &EpisodeState, witnesses: Vec<Witness>) {
        let tid = self.st.next_thread;
        self.st.next_thread += 1;
        self.st.threads.insert(
            tid,
            ThreadState {
                id: tid,
                born_at: ep.span.0,
                last_activity: ep.span.1,
                last_commit_ts: ep.span.0,
                episodes: BTreeSet::new(),
                anchors: BTreeMap::new(),
                companions: BTreeMap::new(),
                in_flow_companions: BTreeSet::new(),
                fork: None,
            },
        );
        self.st.push(Decision::ThreadBorn {
            thread: tid,
            at: ep.span.0,
            witnesses: witnesses.clone(),
        });
        self.finish_assign(ep, tid, witnesses);
    }

    /// Birth through reformation: no production anywhere, but a resource keeps
    /// re-forming across distinct occasions with meaningful attention and was
    /// never merely ubiquitous.
    fn spawn_recurrence_thread(&mut self, ep: &EpisodeState, qualified: Vec<String>) {
        let tid = self.st.next_thread;
        self.st.next_thread += 1;
        self.st.threads.insert(
            tid,
            ThreadState {
                id: tid,
                born_at: ep.span.0,
                last_activity: ep.span.1,
                last_commit_ts: ep.span.0,
                episodes: BTreeSet::new(),
                anchors: BTreeMap::new(),
                companions: BTreeMap::new(),
                in_flow_companions: BTreeSet::new(),
                fork: None,
            },
        );
        let witnesses: Vec<Witness> = qualified
            .iter()
            .map(|r| Witness::RecurrenceReforms {
                resource: r.clone(),
                episodes: self.st.cfg.min_anchor_recurrence,
            })
            .collect();
        self.st.push(Decision::ThreadBorn {
            thread: tid,
            at: ep.span.0,
            witnesses: witnesses.clone(),
        });
        let strengths: Vec<f64> = qualified.iter().map(|r| self.recurrence_strength(r)).collect();
        if let Some(t) = self.st.threads.get_mut(&tid) {
            for (r, s) in qualified.iter().zip(strengths.iter()) {
                t.anchors.insert(
                    r.clone(),
                    Anchor {
                        resource: r.clone(),
                        kind: AnchorKind::Recurrence,
                        strength: *s,
                    },
                );
            }
        }
        for r in &qualified {
            let kind_first = !self
                .st
                .anchor_index
                .get(r)
                .map(|s| s.contains(&tid))
                .unwrap_or(false);
            let set = self.st.anchor_index.entry(r.clone()).or_default();
            let grew = set.insert(tid);
            if grew && set.len() >= 2 {
                let threads: Vec<ThreadId> = set.iter().copied().collect();
                self.st.push(Decision::AnchorContested {
                    resource: r.clone(),
                    threads,
                });
            }
            if kind_first {
                let strength = self.recurrence_strength(r);
                self.st.push(Decision::AnchorPromoted {
                    thread: tid,
                    resource: r.clone(),
                    kind: AnchorKind::Recurrence,
                    strength,
                });
            }
        }
        // Claim every unassigned episode that carried the re-forming resource,
        // oldest first: the reformer and the occasions it re-formed from.
        let mut members: Vec<EpisodeId> = self
            .st
            .episodes
            .values()
            .filter(|e| {
                !e.open
                    && e.assigned.is_none()
                    && qualified.iter().any(|r| {
                        e.participants
                            .get(r)
                            .map(|p| p.attention >= self.st.cfg.min_meaningful_attention as f64)
                            .unwrap_or(false)
                    })
            })
            .map(|e| e.id)
            .collect();
        members.sort();
        for id in members {
            let member = self.st.episodes.get(&id).cloned();
            if let Some(m) = member {
                self.finish_assign(&m, tid, witnesses.clone());
            }
        }
        self.st.sweep_dirty = true;
    }

    fn recurrence_strength(&self, r: &str) -> f64 {
        let cfg = &self.st.cfg;
        let mut total_ms = 0.0f64;
        for ep in self.st.episodes.values() {
            if let Some(p) = ep.participants.get(r) {
                if p.attention >= cfg.min_meaningful_attention as f64 {
                    total_ms += p.attention;
                }
            }
        }
        let cap = cfg.recurrence_strength_cap_ratio * cfg.saved_credit as f64;
        (total_ms / 1000.0 * 0.5).min(cap)
    }

    /// Reformation test for a closed, unassigned, production-free episode:
    /// a meaningfully-attended, non-passive, non-ubiquitous resource shared
    /// with at least one other unassigned episode, on a different occasion.
    fn recurrence_qualify(&self, ep: &EpisodeState) -> Option<Vec<String>> {
        let cfg = &self.st.cfg;
        let mut qualified = Vec::new();
        for (r, p) in &ep.participants {
            if ep.credits.contains_key(r) || p.attention < cfg.min_meaningful_attention as f64 {
                continue;
            }
            if self
                .st
                .ledger
                .get(r)
                .map(|l| l.is_passive())
                .unwrap_or(false)
            {
                continue;
            }
            if self.st.profile_thread_count(r) > cfg.recurrence_anchor_max_threads {
                continue;
            }
            let others = self
                .st
                .episodes
                .values()
                .filter(|o| {
                    o.id != ep.id
                        && !o.open
                        && o.assigned.is_none()
                        && o
                            .participants
                            .get(r)
                            .map(|q| q.attention >= cfg.min_meaningful_attention as f64)
                            .unwrap_or(false)
                })
                .count();
            if others + 1 >= cfg.min_anchor_recurrence {
                qualified.push(r.clone());
            }
        }
        if qualified.is_empty() {
            None
        } else {
            qualified.sort();
            Some(qualified)
        }
    }

    /// Fold a closed (possibly re-closed) episode's deltas into its thread.
    /// Watermarks make re-commitment idempotent: only new attention and new
    /// production cross over.
    pub(crate) fn commit_episode(&mut self, ep_id: EpisodeId) {
        let cfg = self.st.cfg.clone();
        let tid = match self.st.episodes.get(&ep_id).and_then(|e| e.assigned) {
            Some(t) => t,
            None => return,
        };
        let (span_end, parts, credits) = match self.st.episodes.get(&ep_id) {
            Some(ep) => (
                ep.span.1,
                ep.participants.clone(),
                ep.credits.clone(),
            ),
            None => return,
        };

        // Companions decay across the silence since the last commit; anchors
        // never do.
        if let Some(t) = self.st.threads.get_mut(&tid) {
            let dt = span_end.saturating_sub(t.last_commit_ts) as f64;
            if dt > 0.0 && cfg.companion_halflife > 0 {
                let decay = 0.5f64.powf(dt / cfg.companion_halflife as f64);
                for v in t.companions.values_mut() {
                    *v *= decay;
                }
            }
            t.last_commit_ts = span_end;
            t.last_activity = span_end;
        }

        // The boundary between prelude and consultation: production first
        // appeared at this instant. A passive resource whose first contact
        // precedes it merely attended the sitting's opening; one first
        // touched after it was consulted while the work was producing —
        // and only that earns the right to vote for this thread later.
        let first_production = self
            .st
            .episodes
            .get(&ep_id)
            .and_then(|ep| ep.first_production_ts)
            .unwrap_or(u64::MAX);

        for (r, p) in &parts {
            if credits.contains_key(r) {
                continue;
            }
            let delta = p.attention - p.committed_attention;
            if delta <= 0.0 {
                continue;
            }
            let in_flow = p
                .first_contact
                .map(|fc| fc >= first_production)
                .unwrap_or(false);
            if let Some(t) = self.st.threads.get_mut(&tid) {
                *t.companions.entry(r.clone()).or_insert(0.0) += delta;
                if in_flow {
                    t.in_flow_companions.insert(r.clone());
                }
            }
            if let Some(ep) = self.st.episodes.get_mut(&ep_id) {
                if let Some(pp) = ep.participants.get_mut(r) {
                    pp.committed_attention = p.attention;
                }
            }
        }

        for (r, c) in &credits {
            let delta = c.amount - c.committed;
            if delta == 0 {
                continue;
            }
            let mut promoted_now = false;
            let mut new_strength = 0.0;
            if let Some(t) = self.st.threads.get_mut(&tid) {
                let entry = t.anchors.entry(r.clone()).or_insert(Anchor {
                    resource: r.clone(),
                    kind: AnchorKind::Mutation,
                    strength: 0.0,
                });
                entry.kind = AnchorKind::Mutation;
                entry.strength += delta as f64;
                promoted_now = delta > 0;
                new_strength = entry.strength;
            }
            if let Some(ep) = self.st.episodes.get_mut(&ep_id) {
                if let Some(cc) = ep.credits.get_mut(r) {
                    cc.committed = c.amount;
                }
            }
            let (was_size, new_size, holders) = {
                let was = self
                    .st
                    .anchor_index
                    .get(r)
                    .map(|s| s.len())
                    .unwrap_or(0);
                let set = self.st.anchor_index.entry(r.clone()).or_default();
                set.insert(tid);
                let holders: Vec<ThreadId> = set.iter().copied().collect();
                (was, set.len(), holders)
            };
            if promoted_now {
                self.st.push(Decision::AnchorPromoted {
                    thread: tid,
                    resource: r.clone(),
                    kind: AnchorKind::Mutation,
                    strength: new_strength,
                });
            }
            if new_size >= 2 && new_size != was_size {
                self.st.push(Decision::AnchorContested {
                    resource: r.clone(),
                    threads: holders,
                });
            }
        }

        self.promote_recurrence_anchors(tid);
        self.st.sweep_dirty = true;
    }

    /// A non-anchor resource that keeps re-forming across this thread's
    /// distinct episodes earns second-class identity — provided it is not a
    /// passive surface and not ambience shared by everything.
    fn promote_recurrence_anchors(&mut self, tid: ThreadId) {
        let cfg = self.st.cfg.clone();
        let episodes: Vec<EpisodeId> = self
            .st
            .threads
            .get(&tid)
            .map(|t| t.episodes.iter().copied().collect())
            .unwrap_or_default();
        let mut counts: BTreeMap<String, (usize, f64)> = BTreeMap::new();
        for id in &episodes {
            if let Some(ep) = self.st.episodes.get(id) {
                for (r, p) in &ep.participants {
                    if ep.credits.contains_key(r) {
                        continue;
                    }
                    if p.attention >= cfg.min_meaningful_attention as f64 {
                        let e = counts.entry(r.clone()).or_insert((0, 0.0));
                        e.0 += 1;
                        e.1 += p.attention;
                    }
                }
            }
        }
        for (r, (count, total)) in counts {
            if count < cfg.min_anchor_recurrence {
                continue;
            }
            let already = self
                .st
                .threads
                .get(&tid)
                .map(|t| t.anchors.contains_key(&r))
                .unwrap_or(false);
            if already {
                continue;
            }
            if self
                .st
                .ledger
                .get(&r)
                .map(|l| l.is_passive())
                .unwrap_or(false)
            {
                continue;
            }
            if self.st.profile_thread_count(&r) > cfg.recurrence_anchor_max_threads {
                continue;
            }
            let cap = cfg.recurrence_strength_cap_ratio * cfg.saved_credit as f64;
            let strength = (total / 1000.0 * 0.5).min(cap);
            if let Some(t) = self.st.threads.get_mut(&tid) {
                t.anchors.insert(
                    r.clone(),
                    Anchor {
                        resource: r.clone(),
                        kind: AnchorKind::Recurrence,
                        strength,
                    },
                );
            }
            let set = self.st.anchor_index.entry(r.clone()).or_default();
            let grew = set.insert(tid);
            if grew && set.len() >= 2 {
                let threads: Vec<ThreadId> = set.iter().copied().collect();
                self.st.push(Decision::AnchorContested {
                    resource: r.clone(),
                    threads,
                });
            }
            self.st.push(Decision::AnchorPromoted {
                thread: tid,
                resource: r,
                kind: AnchorKind::Recurrence,
                strength,
            });
        }
    }

    /// The parked fixpoint: after any commit or declaration, re-offer every
    /// retained episode the config rule and the reformation rule until nothing
    /// moves. Deterministic by construction: parked ids climb in order, and
    /// every iteration removes at least one episode from consideration.
    pub(crate) fn sweep_parked(&mut self) {
        self.st.sweep_dirty = false;
        loop {
            let parked: Vec<EpisodeId> = self
                .st
                .episodes
                .values()
                .filter(|e| !e.open && e.assigned.is_none() && e.parked)
                .map(|e| e.id)
                .collect();
            let mut progressed = false;
            for id in parked {
                let ep = match self.st.episodes.get(&id) {
                    Some(e) => e.clone(),
                    None => continue,
                };
                // A forked episode stays forked until the person arbitrates.
                // The fixpoint never resolves ambiguity by itself.
                if ep.fork_candidates.is_some() {
                    continue;
                }
                let c = self.compute_candidates(&ep);
                let pins: Vec<ThreadId> = c.pinned.iter().copied().collect();
                if pins.len() == 1 {
                    let t = pins[0];
                    let mut w = c.witnesses.get(&t).cloned().unwrap_or_default();
                    w.push(Witness::UserPin);
                    self.finish_assign(&ep, t, w);
                    progressed = true;
                    continue;
                }
                let production: u64 = ep.credits.values().map(|x| x.amount).sum();
                if production >= self.st.cfg.thread_birth_min_production {
                    self.spawn_thread_for_episode(&ep, birth_witnesses(&ep, &c));
                    progressed = true;
                    continue;
                }
                if production == 0 {
                    if let Some((t, ws)) = self.best_config_candidate(&ep, &c) {
                        if !c.forbidden.contains(&t) {
                            self.finish_assign(&ep, t, ws);
                            progressed = true;
                            continue;
                        }
                    }
                    if let Some(qualified) = self.recurrence_qualify(&ep) {
                        self.spawn_recurrence_thread(&ep, qualified);
                        progressed = true;
                        continue;
                    }
                }
            }
            if !progressed {
                break;
            }
        }
    }

    // ------------------------------------------------------------------
    // Declarations: the supreme court.
    // ------------------------------------------------------------------

    pub(crate) fn apply_declaration(&mut self, e: &Event) {
        let d = match &e.detail {
            Some(d) => d.clone(),
            None => return,
        };
        match d {
            Declaration::SameWork { a, b } => self.declare_same_work(a, b),
            Declaration::SeparateWork { a, b } => {
                let pair = canon(a, b);
                self.st.separate_work.insert(pair.clone());
                self.st.push(Decision::DeclarationApplied {
                    summary: format!(
                        "Recorded: `{}` and `{}` are separate works; inference will never put them together.",
                        pair.0, pair.1
                    ),
                });
                self.st.sweep_dirty = true;
            }
            Declaration::NotMine { resource } => self.quarantine(&resource),
        }
    }

    fn declare_same_work(&mut self, a: String, b: String) {
        let pair = canon(a.clone(), b.clone());
        self.st.same_work.insert(pair);
        let ta = self.thread_holding(&a);
        let tb = self.thread_holding(&b);
        match (ta, tb) {
            (Some(x), Some(y)) if x == y => {
                self.st.push(Decision::DeclarationApplied {
                    summary: format!(
                        "`{a}` and `{b}` already live in the same work; nothing to do."
                    ),
                });
            }
            (Some(x), Some(y)) => {
                // THE merge path. Here, and nowhere else in the system.
                let (survivor, absorbed) = if x < y { (x, y) } else { (y, x) };
                self.do_merge(survivor, absorbed);
                self.st.push(Decision::ThreadMerged {
                    survivor,
                    absorbed,
                    declaration: format!("`{a}` and `{b}` are the same work, said the person."),
                });
            }
            (Some(x), None) => self.declare_anchor(x, b, None),
            (None, Some(y)) => self.declare_anchor(y, a, None),
            (None, None) => {
                let tid = self.st.next_thread;
                self.st.next_thread += 1;
                let at = self.st.now;
                self.st.threads.insert(
                    tid,
                    ThreadState {
                        id: tid,
                        born_at: at,
                        last_activity: at,
                        last_commit_ts: at,
                        episodes: BTreeSet::new(),
                        anchors: BTreeMap::new(),
                        companions: BTreeMap::new(),
                in_flow_companions: BTreeSet::new(),
                        fork: None,
                    },
                );
                self.st.push(Decision::ThreadBorn {
                    thread: tid,
                    at,
                    witnesses: vec![Witness::UserPin],
                });
                self.declare_anchor(tid, a.clone(), None);
                self.declare_anchor(tid, b.clone(), None);
            }
        }
        self.st.sweep_dirty = true;
    }

    fn declare_anchor(&mut self, tid: ThreadId, resource: String, _at: Option<u64>) {
        let strength = self.st.cfg.declared_anchor_strength;
        if let Some(t) = self.st.threads.get_mut(&tid) {
            let entry = t.anchors.entry(resource.clone()).or_insert(Anchor {
                resource: resource.clone(),
                kind: AnchorKind::Mutation,
                strength: 0.0,
            });
            entry.kind = AnchorKind::Mutation;
            entry.strength = entry.strength.max(strength);
        }
        let set = self.st.anchor_index.entry(resource.clone()).or_default();
        set.insert(tid);
        if set.len() >= 2 {
            let threads: Vec<ThreadId> = set.iter().copied().collect();
            self.st.push(Decision::AnchorContested {
                resource: resource.clone(),
                threads,
            });
        }
        self.st.push(Decision::AnchorPromoted {
            thread: tid,
            resource,
            kind: AnchorKind::Mutation,
            strength,
        });
        self.st.push(Decision::DeclarationApplied {
            summary: "Recorded a declaration anchor; your word outranks the stream.".to_string(),
        });
    }

    fn thread_holding(&self, resource: &str) -> Option<ThreadId> {
        let mut hits: Vec<ThreadId> = self
            .st
            .threads
            .values()
            .filter(|t| t.anchors.contains_key(resource) || t.companions.contains_key(resource))
            .map(|t| t.id)
            .collect();
        hits.sort();
        hits.first().copied()
    }

    /// The only merge function in the engine. It is unreachable from
    /// inference: its sole caller is the declaration path above.
    fn do_merge(&mut self, survivor: ThreadId, absorbed: ThreadId) {
        let absorbed_state = match self.st.threads.remove(&absorbed) {
            Some(t) => t,
            None => return,
        };
        if let Some(s) = self.st.threads.get_mut(&survivor) {
            s.episodes.extend(absorbed_state.episodes.iter().copied());
            for (r, anch) in &absorbed_state.anchors {
                let entry = s.anchors.entry(r.clone()).or_insert(Anchor {
                    resource: r.clone(),
                    kind: anch.kind,
                    strength: 0.0,
                });
                entry.kind = entry.kind.max(anch.kind);
                entry.strength += anch.strength;
            }
            for (r, w) in &absorbed_state.companions {
                *s.companions.entry(r.clone()).or_insert(0.0) += *w;
            }
            for r in &absorbed_state.in_flow_companions {
                s.in_flow_companions.insert(r.clone());
            }
            s.last_activity = s.last_activity.max(absorbed_state.last_activity);
            s.last_commit_ts = s.last_commit_ts.max(absorbed_state.last_commit_ts);
            s.fork = merge_fork(s.fork.take(), absorbed_state.fork);
        }
        for set in self.st.anchor_index.values_mut() {
            if set.remove(&absorbed) {
                set.insert(survivor);
            }
        }
        for ep in self.st.episodes.values_mut() {
            if let Some(f) = &mut ep.fork_candidates {
                for t in f.iter_mut() {
                    if *t == absorbed {
                        *t = survivor;
                    }
                }
                f.sort();
                f.dedup();
            }
        }
        for t in self.st.threads.values_mut() {
            if let Some(f) = &mut t.fork {
                for x in f.iter_mut() {
                    if *x == absorbed {
                        *x = survivor;
                    }
                }
                f.sort();
                f.dedup();
            }
        }
    }

    fn quarantine(&mut self, resource: &str) {
        self.st.quarantined.insert(resource.to_string());
        let mut stripped_threads = Vec::new();
        for (tid, t) in self.st.threads.iter_mut() {
            let had = t.anchors.remove(resource).is_some() | t.companions.remove(resource).is_some();
            if had {
                stripped_threads.push(*tid);
            }
        }
        if let Some(set) = self.st.anchor_index.get_mut(resource) {
            set.retain(|_| false);
        }
        self.st.anchor_index.remove(resource);
        for ep in self.st.episodes.values_mut() {
            ep.participants.remove(resource);
            ep.credits.remove(resource);
            ep.focus.remove(resource);
        }
        self.st.ledger.remove(resource);
        let _ = stripped_threads;
        self.st.push(Decision::ResourceQuarantined {
            resource: resource.to_string(),
        });
        self.st.sweep_dirty = true;
    }
}

fn canon(a: String, b: String) -> (String, String) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn sorted_support(m: &BTreeMap<ThreadId, u64>) -> Vec<(ThreadId, u64)> {
    let mut v: Vec<(ThreadId, u64)> = m.iter().map(|(t, s)| (*t, *s)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

fn flatten_witnesses(m: &BTreeMap<ThreadId, Vec<Witness>>) -> Vec<Witness> {
    m.values().flatten().cloned().collect()
}

fn birth_witnesses(ep: &EpisodeState, c: &Candidates) -> Vec<Witness> {
    let mut w = vec![Witness::NewProduction {
        weight: c.total_production,
    }];
    // Note honestly which existing anchors the newborn is about to share —
    // sharing is recorded, and decides nothing.
    for (t, ws) in &c.witnesses {
        let _ = t;
        for x in ws {
            if let Witness::SharedContestedAnchor(_) = x {
                w.push(x.clone());
            }
        }
    }
    let _ = ep;
    w
}

fn merge_fork(a: Option<Vec<ThreadId>>, b: Option<Vec<ThreadId>>) -> Option<Vec<ThreadId>> {
    match (a, b) {
        (None, None) => None,
        (Some(x), None) | (None, Some(x)) => Some(x),
        (Some(mut x), Some(y)) => {
            x.extend(y);
            x.sort();
            x.dedup();
            Some(x)
        }
    }
}
