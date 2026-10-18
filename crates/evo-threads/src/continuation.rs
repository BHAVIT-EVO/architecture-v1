//! Stage 4: continuation — where did you leave off, and what would it take to
//! put you back there?
//!
//! Product statements:
//! * Continuation reads the thread's last episode and the ledger — never the
//!   whole history. The museum stays closed; the neighborhood is enough.
//! * The resume point is the strongest thing you *made* in that episode.
//!   A brief terminal burst on a surface shared by many works is never
//!   allowed to become where you "left off".
//! * Open deltas report witnessed state only. When no rule fires, the reason
//!   is "Last seen: X" — observed, never invented.

use crate::state::{EpisodeState, ProdKind};
use crate::trace::{render, Decision};
use crate::types::*;
use crate::Engine;
use std::collections::BTreeSet;

impl Engine {
    /// All live works, best-supported first. "Live" includes Dormant: resting
    /// works are delisted, never deleted, and never re-numbered.
    pub fn threads(&self) -> Vec<ThreadView> {
        let mut scored: Vec<(f64, ThreadId)> = self
            .st
            .threads
            .keys()
            .map(|id| (self.thread_score(*id), *id))
            .collect();
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        scored.iter().map(|(_, id)| self.thread_view(*id)).collect()
    }

    /// Ranking for resumption: everything the person invested, weighted as the
    /// evidence ranks say — production at full weight, context discounted by
    /// how many works share it. Last-touched alone never wins this race.
    fn thread_score(&self, tid: ThreadId) -> f64 {
        let Some(t) = self.st.threads.get(&tid) else {
            return 0.0;
        };
        let mut s: f64 = t.anchors.values().map(|a| a.strength).sum();
        for (r, w) in &t.companions {
            let ub = self.st.profile_thread_count(r);
            s += w / (1.0 + ub as f64);
        }
        s
    }

    fn thread_view(&self, tid: ThreadId) -> ThreadView {
        let t = self
            .st
            .threads
            .get(&tid)
            .expect("thread view of missing thread");
        let episodes: Vec<EpisodeView> = t
            .episodes
            .iter()
            .filter_map(|id| self.st.episodes.get(id))
            .map(|ep| self.episode_view(ep))
            .collect();
        let contested: Vec<String> = t
            .anchors
            .keys()
            .filter(|r| {
                self.st
                    .anchor_index
                    .get(*r)
                    .map(|s| s.len() >= 2)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        let status = if self.st.now.saturating_sub(t.last_activity) > self.st.cfg.dormant_after {
            Status::Dormant
        } else {
            Status::Active
        };
        ThreadView {
            id: tid,
            status,
            episodes,
            anchors: sorted_anchors(t),
            companions: sorted_companions(t),
            contested,
            fork: t.fork.clone(),
        }
    }

    fn episode_view(&self, ep: &EpisodeState) -> EpisodeView {
        let now = self.st.now;
        let mut participants: Vec<(String, f64)> = ep
            .participants
            .iter()
            .map(|(r, p)| (r.clone(), p.attention))
            .collect();
        participants.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        EpisodeView {
            span: if ep.open { (ep.span.0, now) } else { ep.span },
            participants,
            anchors: ep.credits.keys().cloned().collect(),
            interruptions: ep.dips.clone(),
        }
    }

    pub fn resume_bundle(&self, thread_id: ThreadId) -> ResumeBundle {
        self.resume_bundle_with_mode(thread_id, false)
    }

    /// Derives the activation bundle with an explicit cognitive mode. The
    /// default remains the historical five-item cap; reduced mode uses the
    /// configured three-item cap and omits Optional members.
    pub fn resume_bundle_with_mode(&self, thread_id: ThreadId, reduced_mode: bool) -> ResumeBundle {
        let Some(t) = self.st.threads.get(&thread_id) else {
            return ResumeBundle {
                resume_point: String::new(),
                resume_reason: "Last seen: nothing.".to_string(),
                restore_set: vec![],
                open_deltas: vec![],
            };
        };
        let open_now = self.open_now_set();
        // Resume from the last episode that can still answer "what was I
        // doing?" — one with production or at least a genuinely non-ambient
        // surface. An ambient-only episode (music that merely attended the
        // work) is context, never a destination.
        let episode_worthy = |ep: &crate::state::EpisodeState| {
            !ep.credits.is_empty()
                || ep.participants.iter().any(|(r, _)| {
                    !self
                        .st
                        .ledger
                        .get(r)
                        .map(|l| l.is_passive())
                        .unwrap_or(false)
                })
        };
        let mut eps: Vec<&crate::state::EpisodeState> = t
            .episodes
            .iter()
            .filter_map(|id| self.st.episodes.get(id))
            .collect();
        eps.sort_by(|a, b| a.span.1.cmp(&b.span.1).then(a.id.cmp(&b.id)));
        let last_episode = eps
            .iter()
            .rev()
            .copied()
            .find(|ep| episode_worthy(ep))
            .or_else(|| eps.last().copied());

        let (resume_point, resume_reason) = match last_episode {
            Some(ep) => self.resume_point_of(t, ep),
            None => {
                // A work born of declarations, with no episodes yet.
                let point = sorted_anchors(t)
                    .into_iter()
                    .find(|anchor| !self.member_is_skipped(t, &anchor.resource, false))
                    .map(|a| a.resource.clone())
                    .unwrap_or_default();
                (point.clone(), format!("Last seen: {point}."))
            }
        };

        let restore_set = match last_episode {
            Some(ep) => self.restore_set_of(t, ep, &resume_point, &open_now, reduced_mode),
            None => {
                let mut v: Vec<String> = Vec::new();
                if !resume_point.is_empty() {
                    v.push(resume_point.clone());
                }
                v
            }
        };

        ResumeBundle {
            resume_point,
            resume_reason,
            restore_set,
            open_deltas: self.deltas_of(t),
        }
    }

    /// The executable subset of a resume bundle plus lifecycle decisions that
    /// kept members out. This is the projection consumed by restoration UI and
    /// execution; the evidence remains in the replay state.
    pub fn activation_plan(&self, thread_id: ThreadId, reduced_mode: bool) -> ActivationPlan {
        let bundle = self.resume_bundle_with_mode(thread_id, reduced_mode);
        let Some(thread) = self.st.threads.get(&thread_id) else {
            return ActivationPlan {
                resources: bundle.restore_set,
                skipped: Vec::new(),
            };
        };
        let mut skipped = Vec::new();
        for resource in self.thread_resources(thread) {
            if let Some(state) = self.member_state_for(thread, &resource) {
                if matches!(
                    state.state,
                    MemberState::Completed | MemberState::Superseded
                ) || (reduced_mode && state.state == MemberState::Optional)
                {
                    skipped.push(MemberStateView { resource, ..state });
                }
            }
        }
        skipped.sort_by(|a, b| a.resource.cmp(&b.resource));
        ActivationPlan {
            resources: bundle.restore_set,
            skipped,
        }
    }

    /// Dismissal changes only the default presentation surface. The work and
    /// all of its evidence remain available through replay and retrieval.
    pub fn is_dismissed(&self, thread_id: ThreadId) -> bool {
        self.st
            .threads
            .get(&thread_id)
            .map(|thread| self.thread_is_dismissed(thread))
            .unwrap_or(false)
    }

    fn thread_is_dismissed(&self, thread: &crate::state::ThreadState) -> bool {
        self.st.dismissed.iter().any(|work| {
            thread.anchors.contains_key(work)
                || thread.companions.contains_key(work)
                || thread.episodes.iter().any(|episode| {
                    self.st
                        .episodes
                        .get(episode)
                        .is_some_and(|ep| ep.participants.contains_key(work))
                })
        })
    }

    /// The strongest production of the last episode — with the trap filter:
    /// resources that are brief, terminal, and shared across many works are
    /// never where you "left off", no matter when they happened.
    fn resume_point_of(
        &self,
        t: &crate::state::ThreadState,
        ep: &EpisodeState,
    ) -> (String, String) {
        let cfg = &self.st.cfg;
        let mut candidates: Vec<(u64, String, ProdKind)> = ep
            .credits
            .iter()
            .filter(|(r, _)| !self.member_is_skipped(t, r, false))
            .filter(|(r, _)| !self.is_resume_trap(t, r, cfg.resume_exclusion_ubiquity))
            .map(|(r, c)| (c.amount, r.clone(), c.last_kind))
            .collect();
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        if let Some((_, r, kind)) = candidates.into_iter().next() {
            return (r.clone(), produce_reason(&r, kind));
        }
        // Second class: something you kept returning to, still inside this episode.
        let mut rec: Vec<(f64, String)> = t
            .anchors
            .values()
            .filter(|a| {
                !self.member_is_skipped(t, &a.resource, false)
                    && a.kind == AnchorKind::Recurrence
                    && ep.participants.contains_key(&a.resource)
            })
            .map(|a| (a.strength, a.resource.clone()))
            .collect();
        rec.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        if let Some((_, r)) = rec.into_iter().next() {
            return (
                r.clone(),
                format!("You kept returning to `{r}` across sessions."),
            );
        }
        // Last honesty: the final thing attention touched. Observed, not inferred.
        let mut by_contact: Vec<(u64, String)> = ep
            .participants
            .iter()
            .filter(|(r, _)| !self.member_is_skipped(t, r, false))
            .map(|(r, p)| (p.last_contact, r.clone()))
            .collect();
        by_contact.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        match by_contact.into_iter().next() {
            Some((_, r)) => (r.clone(), format!("Last seen: {r}.")),
            None => (String::new(), "Last seen: nothing.".to_string()),
        }
    }

    fn is_resume_trap(
        &self,
        _t: &crate::state::ThreadState,
        resource: &str,
        ubiquity_limit: usize,
    ) -> bool {
        let contested = self
            .st
            .anchor_index
            .get(resource)
            .map(|s| s.len() >= 2)
            .unwrap_or(false);
        contested || self.st.profile_thread_count(resource) >= ubiquity_limit
    }

    /// The neighborhood, not the museum: the last episode's anchors and its
    /// strongest *non-ambient* companions, capped, minus whatever is already
    /// open on the desk.
    fn restore_set_of(
        &self,
        t: &crate::state::ThreadState,
        ep: &EpisodeState,
        resume_point: &str,
        open_now: &BTreeSet<String>,
        reduced_mode: bool,
    ) -> Vec<String> {
        let cfg = &self.st.cfg;
        let total_threads = self.st.threads.len();
        let cap = if reduced_mode {
            cfg.reduced_restore_cap
        } else {
            cfg.restore_cap
        };
        let mut out: Vec<String> = Vec::new();
        let push = |out: &mut Vec<String>, r: &str| {
            if out.len() < cap
                && !r.is_empty()
                && !out.iter().any(|x| x == r)
                && !open_now.contains(r)
                && !self.st.quarantined.contains(r)
                && !self.member_is_skipped(t, r, reduced_mode)
            {
                out.push(r.to_string());
            }
        };
        push(&mut out, resume_point);

        let mut anchors: Vec<(u64, String)> = ep
            .credits
            .iter()
            .map(|(r, c)| (c.amount, r.clone()))
            .collect();
        anchors.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, r) in anchors {
            push(&mut out, &r);
        }

        let mut companions: Vec<(f64, String)> = ep
            .participants
            .iter()
            .filter(|(r, _)| !ep.credits.contains_key(*r))
            .filter(|(r, _)| !self.is_ambient(r, total_threads))
            // The neighborhood, not the museum. Two rules, one per caste:
            //
            // A companion that received *meaningful* attention (at least
            // `min_meaningful_attention`) is real context — the reference
            // document the person studied, the chat they typed into — and
            // rejoins the restore set regardless of when in the sitting it
            // was last touched.
            //
            // A companion that was merely *glanced at* (a 1-second visited
            // tick, a passing focus) is finished supporting work unless it
            // was still near the person at the end: the Google search that
            // served its purpose and was left behind.
            .filter(|(_, p)| {
                p.attention >= cfg.min_meaningful_attention as f64
                    || ep.span.1.saturating_sub(p.last_contact) <= cfg.restore_recency
            })
            .map(|(r, p)| {
                let ub = self.st.profile_thread_count(r);
                (p.attention / (1.0 + ub as f64), r.clone())
            })
            .collect();
        companions.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        for (_, r) in companions {
            push(&mut out, &r);
        }

        // A pure return visit carries no production of its own; the question
        // "what was I doing?" then answers from the work's identity — fill
        // the remaining slots from the thread's anchors, strongest first.
        if ep.credits.is_empty() {
            let mut identity: Vec<(f64, String)> = t
                .anchors
                .iter()
                .map(|(r, a)| (a.strength, r.clone()))
                .collect();
            identity.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.cmp(&b.1))
            });
            for (_, r) in identity {
                push(&mut out, &r);
            }
        }
        out
    }

    fn member_is_skipped(
        &self,
        thread: &crate::state::ThreadState,
        resource: &str,
        reduced_mode: bool,
    ) -> bool {
        match self.member_state_for(thread, resource) {
            Some(MemberStateView {
                state: MemberState::Completed | MemberState::Superseded,
                ..
            }) => true,
            Some(MemberStateView {
                state: MemberState::Optional,
                ..
            }) => reduced_mode,
            _ => false,
        }
    }

    fn thread_resources(&self, thread: &crate::state::ThreadState) -> BTreeSet<String> {
        let mut resources = BTreeSet::new();
        resources.extend(thread.anchors.keys().cloned());
        resources.extend(thread.companions.keys().cloned());
        for episode in &thread.episodes {
            if let Some(ep) = self.st.episodes.get(episode) {
                resources.extend(ep.participants.keys().cloned());
            }
        }
        resources
    }

    fn member_state_for(
        &self,
        thread: &crate::state::ThreadState,
        resource: &str,
    ) -> Option<MemberStateView> {
        let declared = self
            .st
            .member_states
            .iter()
            .filter(|((work, member), _)| {
                member == resource
                    && (thread.anchors.contains_key(work)
                        || thread.companions.contains_key(work)
                        || thread.episodes.iter().any(|episode| {
                            self.st
                                .episodes
                                .get(episode)
                                .is_some_and(|ep| ep.participants.contains_key(work))
                        }))
            })
            .map(|((work, _), state)| (work.clone(), *state))
            .min_by(|a, b| a.0.cmp(&b.0))
            .map(|(_, state)| MemberStateView {
                resource: resource.to_string(),
                state,
                source: MemberStateSource::Declared,
            });
        if declared.is_some() {
            return declared;
        }

        // The projection layer has already established the opaque
        // commit↔member relationship. The engine only applies the conservative
        // rule: a commit can complete an anchor whose last witnessed contact
        // is no later than that commit.
        if thread.anchors.contains_key(resource) {
            let last_contact = self.member_last_contact(thread, resource);
            if self
                .lifecycle_evidence
                .commits
                .iter()
                .filter(|evidence| evidence.member == resource)
                .any(|evidence| {
                    last_contact
                        .map(|contact| contact <= evidence.observed_at)
                        .unwrap_or(false)
                })
            {
                return Some(MemberStateView {
                    resource: resource.to_string(),
                    state: MemberState::Completed,
                    source: MemberStateSource::Derived,
                });
            }
        }

        // Promote the existing restore-recency boundary into an explicitly
        // labelled inference only when a sibling actually returned in a later
        // episode. A stale member by itself is never enough.
        if self.inferred_member_completed(thread, resource) {
            return Some(MemberStateView {
                resource: resource.to_string(),
                state: MemberState::Completed,
                source: MemberStateSource::Inferred,
            });
        }
        None
    }

    fn member_last_contact(
        &self,
        thread: &crate::state::ThreadState,
        resource: &str,
    ) -> Option<u64> {
        thread
            .episodes
            .iter()
            .filter_map(|episode| self.st.episodes.get(episode))
            .filter_map(|episode| episode.participants.get(resource))
            .map(|participant| participant.last_contact)
            .max()
    }

    fn latest_episode(
        &self,
        thread: &crate::state::ThreadState,
    ) -> Option<&crate::state::EpisodeState> {
        thread
            .episodes
            .iter()
            .filter_map(|episode| self.st.episodes.get(episode))
            .max_by(|a, b| a.span.1.cmp(&b.span.1).then(a.id.cmp(&b.id)))
    }

    fn inferred_member_completed(
        &self,
        thread: &crate::state::ThreadState,
        resource: &str,
    ) -> bool {
        let Some(latest) = self.latest_episode(thread) else {
            return false;
        };
        if latest.participants.contains_key(resource) {
            return false;
        }
        let resources = self.thread_resources(thread);
        let has_returning_sibling = latest
            .participants
            .keys()
            .any(|sibling| sibling != resource && resources.contains(sibling));
        if !has_returning_sibling {
            return false;
        }
        let Some(last_contact) = self.member_last_contact(thread, resource) else {
            return false;
        };
        latest.span.1.saturating_sub(last_contact) > self.st.cfg.restore_recency
    }

    /// Lifecycle decisions are read-time projections of the replay and its
    /// ephemeral evidence. They never append to the event-sourced trace, so
    /// repeated calls remain deterministic and side-effect free.
    pub(crate) fn lifecycle_decisions(&self) -> Vec<Decision> {
        let mut decisions = Vec::new();
        for thread in self.st.threads.values() {
            let work = thread
                .anchors
                .keys()
                .next()
                .cloned()
                .or_else(|| self.thread_resources(thread).into_iter().next());
            let Some(work) = work else {
                continue;
            };
            let resources = self.thread_resources(thread);
            for resource in resources {
                let Some(state) = self.member_state_for(thread, &resource) else {
                    continue;
                };
                match state.source {
                    MemberStateSource::Declared => {}
                    MemberStateSource::Derived => {
                        let evidence = self
                            .lifecycle_evidence
                            .commits
                            .iter()
                            .filter(|evidence| {
                                evidence.member == resource
                                    && self
                                        .member_last_contact(thread, &resource)
                                        .map(|contact| contact <= evidence.observed_at)
                                        .unwrap_or(false)
                            })
                            .min_by(|a, b| {
                                a.observed_at
                                    .cmp(&b.observed_at)
                                    .then(a.commit.cmp(&b.commit))
                            });
                        if let Some(evidence) = evidence {
                            decisions.push(Decision::MemberStateDerived {
                                work: work.clone(),
                                member: resource,
                                state: state.state,
                                commit: evidence.commit.clone(),
                                at: evidence.observed_at,
                            });
                        }
                    }
                    MemberStateSource::Inferred => {
                        let sibling = self
                            .latest_episode(thread)
                            .and_then(|episode| {
                                episode
                                    .participants
                                    .keys()
                                    .find(|sibling| sibling.as_str() != resource)
                            })
                            .cloned()
                            .unwrap_or_default();
                        decisions.push(Decision::MemberStateInferred {
                            work: work.clone(),
                            member: resource,
                            state: state.state,
                            sibling,
                        });
                    }
                }
            }
        }
        decisions
    }

    /// Ambient = can attend but may never be restored: passive surfaces (only
    /// ever focused — an always-on player, a sidebar) or resources that
    /// managed to get into every live work's company. Music links nothing.
    fn is_ambient(&self, resource: &str, total_threads: usize) -> bool {
        let passive = self
            .st
            .ledger
            .get(resource)
            .map(|l| l.is_passive())
            .unwrap_or(false);
        if passive {
            return true;
        }
        total_threads >= 2 && self.st.profile_thread_count(resource) >= total_threads
    }

    /// Resources currently on the desk: the open episode's surfaces, buffered
    /// detour material, Execution-restored surfaces awaiting you, and anything
    /// focused within the "already open" window.
    fn open_now_set(&self) -> BTreeSet<String> {
        let cfg = &self.st.cfg;
        let mut s = BTreeSet::new();
        if let Some(id) = self.st.open {
            if let Some(ep) = self.st.episodes.get(&id) {
                s.extend(ep.participants.keys().cloned());
            }
        }
        if let Some(d) = &self.st.detour {
            s.extend(d.events.iter().map(|e| e.resource.clone()));
        }
        for (r, l) in &self.st.ledger {
            if l.execution_open {
                s.insert(r.clone());
            }
            if let Some(f) = l.last_person_focus {
                if self.st.now.saturating_sub(f) <= cfg.open_now_window {
                    s.insert(r.clone());
                }
            }
        }
        s
    }

    /// Observable state deltas over the thread's profile, grouped by kind and
    /// sorted by resource within each group — a deterministic order the user
    /// can learn to read at a glance.
    fn deltas_of(&self, t: &crate::state::ThreadState) -> Vec<Delta> {
        let mut profile: BTreeSet<String> = BTreeSet::new();
        profile.extend(t.anchors.keys().cloned());
        profile.extend(t.companions.keys().cloned());
        for id in &t.episodes {
            if let Some(ep) = self.st.episodes.get(id) {
                profile.extend(ep.participants.keys().cloned());
            }
        }
        let mut unsaved = Vec::new();
        let mut uncommitted = Vec::new();
        let mut downloads = Vec::new();
        let mut drafts = Vec::new();
        for r in &profile {
            let Some(l) = self.st.ledger.get(r) else {
                continue;
            };
            match (l.last_typed, l.last_saved) {
                (Some(tp), Some(sv)) if tp > sv => unsaved.push(r.clone()),
                (Some(_), None) => drafts.push(r.clone()),
                _ => {}
            }
            let churn_after = l
                .last_churn_passing_mutated
                .map(|m| l.last_saved.map(|s| m > s).unwrap_or(true))
                .unwrap_or(false);
            if churn_after {
                uncommitted.push(r.clone());
            }
            if l.downloaded_at.is_some() && !l.used_after_download {
                downloads.push(r.clone());
            }
        }
        let mut out = Vec::new();
        out.extend(
            unsaved
                .into_iter()
                .map(|resource| Delta::UnsavedEdits { resource }),
        );
        out.extend(
            uncommitted
                .into_iter()
                .map(|root| Delta::UncommittedChanges { root }),
        );
        out.extend(
            downloads
                .into_iter()
                .map(|path| Delta::DownloadedNotYetUsed { path }),
        );
        out.extend(
            drafts
                .into_iter()
                .map(|resource| Delta::UnfinishedDraftSurface { resource }),
        );
        out
    }

    /// One sentence per decision this work has ever caused, in order. Trust
    /// comes from being able to read *why*.
    pub fn explain(&self, thread_id: ThreadId) -> Vec<String> {
        let episodes_of: BTreeSet<EpisodeId> = self
            .st
            .threads
            .get(&thread_id)
            .map(|t| t.episodes.clone())
            .unwrap_or_default();
        self.trace()
            .iter()
            .filter(|d| match d {
                Decision::ThreadBorn { thread, .. } => *thread == thread_id,
                Decision::EpisodeAssigned { thread, .. } => *thread == thread_id,
                Decision::AnchorPromoted { thread, .. } => *thread == thread_id,
                Decision::AnchorContested { threads, .. } => threads.contains(&thread_id),
                Decision::Fork { candidates, .. } => candidates.contains(&thread_id),
                Decision::ThreadMerged {
                    survivor, absorbed, ..
                } => *survivor == thread_id || *absorbed == thread_id,
                Decision::EpisodeReopened { episode, .. } => episodes_of.contains(episode),
                Decision::WorkDismissed { work } => self.thread_has_subject(thread_id, work),
                Decision::MemberStateChanged { work, .. } => {
                    self.thread_has_subject(thread_id, work)
                }
                Decision::MemberStateDerived { work, .. }
                | Decision::MemberStateInferred { work, .. } => {
                    self.thread_has_subject(thread_id, work)
                }
                _ => false,
            })
            .filter_map(render)
            .collect()
    }

    fn thread_has_subject(&self, thread_id: ThreadId, subject: &str) -> bool {
        self.st.threads.get(&thread_id).is_some_and(|thread| {
            thread.anchors.contains_key(subject)
                || thread.companions.contains_key(subject)
                || thread.episodes.iter().any(|episode| {
                    self.st
                        .episodes
                        .get(episode)
                        .is_some_and(|ep| ep.participants.contains_key(subject))
                })
        })
    }
}

fn produce_reason(resource: &str, kind: ProdKind) -> String {
    match kind {
        ProdKind::Typed => format!("Last edited `{resource}`."),
        ProdKind::Saved => format!("Last saved `{resource}`."),
        ProdKind::Downloaded => format!("Last downloaded `{resource}`."),
        ProdKind::CommandRan => format!("A command you ran last changed `{resource}`."),
        ProdKind::Mutated => format!("`{resource}` last changed while you were working in it."),
        ProdKind::Declared => format!("You declared `{resource}` part of this work."),
    }
}

fn sorted_anchors(t: &crate::state::ThreadState) -> Vec<Anchor> {
    let mut v: Vec<Anchor> = t.anchors.values().cloned().collect();
    v.sort_by(|a, b| {
        b.kind
            .cmp(&a.kind)
            .then(
                b.strength
                    .partial_cmp(&a.strength)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.resource.cmp(&b.resource))
    });
    v
}

fn sorted_companions(t: &crate::state::ThreadState) -> Vec<(String, f64)> {
    let mut v: Vec<(String, f64)> = t
        .companions
        .iter()
        .filter(|(r, _)| !t.anchors.contains_key(*r))
        .map(|(r, w)| (r.clone(), *w))
        .collect();
    v.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    v
}
