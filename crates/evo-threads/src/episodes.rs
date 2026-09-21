//! Stage 1: from events to runs and episodes.
//!
//! Product statement: an episode is the smallest unit the stream can honestly
//! recover — a bounded interval where attention lived on a stable working set.
//! Interleaving is what interrupted people actually do, so boundaries are
//! decided by *content-tested resumption* and dip absorption; the clock only
//! ever plays fallback.

use crate::state::{CreditDetail, Detour, EpisodeState, Participant, ProdKind};
use crate::trace::Decision;
use crate::types::*;
use crate::Engine;

impl Engine {
    /// Production weight of a person-origin event, in the engine's credit
    /// currency. `CommandRan` earns nothing alone: it becomes identity only
    /// when a churn-passing mutation corroborates that it changed something.
    /// Download dedup lives in `feed` (normalized targets), not here.
    pub(crate) fn production_credit(&self, e: &Event) -> u64 {
        let cfg = &self.st.cfg;
        match e.interaction {
            Interaction::Typed => e.weight.unwrap_or(0) * cfg.typed_unit,
            Interaction::Saved => cfg.saved_credit,
            Interaction::Downloaded => cfg.downloaded_credit,
            _ => 0,
        }
    }

    /// Does this routing event count as "production" for the dip / boundary
    /// rules? A detour with production ends the episode; a detour without it
    /// may be absorbed. Commands are acts of work even before their mutations
    /// are seen.
    pub(crate) fn is_production_event(&self, e: &Event) -> bool {
        match e.interaction {
            Interaction::Typed => e.weight.unwrap_or(0) > 0,
            Interaction::Saved | Interaction::Downloaded | Interaction::CommandRan => true,
            _ => false,
        }
    }

    /// Route a person-origin event into the episode machinery.
    pub(crate) fn route(&mut self, e: &Event) {
        let gap_close = self.st.cfg.gap_close;
        let dip_max = self.st.cfg.dip_max;

        // The clock fallback runs before everything: silence is the only
        // boundary that needs no content test. The episode closes at its last
        // known activity, not now.
        if let Some(open_id) = self.st.open {
            let last = self
                .st
                .episodes
                .get(&open_id)
                .map(|ep| ep.last_person_ts)
                .unwrap_or(0);
            if e.timestamp.saturating_sub(last) > gap_close {
                // An excursion pending across the silence is a departure that
                // never returned: give its material the same content-first
                // chance as anything else. It must be lifted out BEFORE the
                // close — closing clears the detour as part of ending the
                // sitting, which would silently discard the buffered material.
                let pending = self.st.detour.take();
                self.close_episode_at(last);
                if let Some(d) = pending {
                    self.flush_detour_as_context(d.events);
                }
                self.route(e);
                return;
            }
        }

        if let Some(open_id) = self.st.open {
            let in_focus = self
                .st
                .episodes
                .get(&open_id)
                .map(|ep| ep.focus.contains(&e.resource))
                .unwrap_or(false);

            if in_focus {
                // A return while an excursion is pending resolves it.
                if let Some(d) = self.st.detour.take() {
                    let productionless = d.events.iter().all(|x| self.production_credit(x) == 0
                        && !matches!(x.interaction, Interaction::CommandRan));
                    let short = e.timestamp.saturating_sub(d.start) < dip_max;
                    if productionless && short {
                        // A notification, not a change of work: kept as context,
                        // voting nowhere.
                        let resources = detour_resource_list(&d);
                        let end = e.timestamp;
                        let start = d.start;
                        self.st.push(Decision::DipAbsorbed {
                            episode: open_id,
                            start,
                            end,
                            resources,
                        });
                        if let Some(ep) = self.st.episodes.get_mut(&open_id) {
                            ep.dips.push((start, end));
                        }
                    } else {
                        // A long or productive excursion is real context: it
                        // joins the working set it interrupted.
                        for b in &d.events {
                            self.feed(b);
                        }
                    }
                }
                self.feed(e);
                return;
            }

            // Production inside a warm productive session is session content:
            // a commit mid-flow belongs to the work in flow. But the FIRST
            // production of a sitting bounds the prelude before it — a
            // still-consumption episode ends where attention left, and the
            // new material stands up on its own. (A cold episode has already
            // been closed by the fallback above, so an isolated burst always
            // spawns cleanly.)
            if self.is_production_event(e) {
                if let Some(d) = self.st.detour.take() {
                    let open_produced = self
                        .st
                        .episodes
                        .get(&open_id)
                        .map(|ep| !ep.credits.is_empty())
                        .unwrap_or(false);
                    if !open_produced {
                        self.close_episode_at(d.start);
                        let mut buf = d.events;
                        buf.push(e.clone());
                        self.flush_detour_as_context(buf);
                        return;
                    }
                    for b in &d.events {
                        self.feed(b);
                    }
                }
                self.feed(e);
                return;
            }

            // Attention moved without production. An excursion has begun.
            // Resumption may claim the session only while the sitting has
            // shown no production of its own — inside productive flow a
            // returning reference tab is company, not a change of work.
            let open_produced = self
                .st
                .episodes
                .get(&open_id)
                .map(|ep| !ep.credits.is_empty())
                .unwrap_or(false);
            if self.st.detour.is_none() {
                if !open_produced {
                    // First contact: resumption before buffering — never
                    // spawn what a suspended episode can explain.
                    if let Some(target) = self.resume_match(&e.resource) {
                        self.close_episode_at(e.timestamp);
                        self.reopen(target, &e.resource);
                        self.feed(e);
                        return;
                    }
                }
                self.st.detour = Some(Detour {
                    start: e.timestamp,
                    events: vec![e.clone()],
                });
                return;
            }
            let d = self.st.detour.take().unwrap();
            let stale = e.timestamp.saturating_sub(d.start) >= dip_max;
            let resumes = if open_produced {
                None
            } else {
                self.resume_match(&e.resource)
            };
            if !stale && resumes.is_none() {
                let mut d = d;
                d.events.push(e.clone());
                self.st.detour = Some(d);
                return;
            }
            // The excursion either outlasted dip_max or landed on suspended
            // work. Close at the moment attention left, then rerun the
            // buffered material content-first: it will resume something else
            // or stand up on its own.
            self.close_episode_at(d.start);
            let mut buf = d.events;
            buf.push(e.clone());
            self.flush_detour_as_context(buf);
            return;
        }

        self.fresh_route(e);
    }

    fn fresh_route(&mut self, e: &Event) {
        if let Some(target) = self.resume_match(&e.resource) {
            self.reopen(target, &e.resource);
            self.feed(e);
        } else {
            self.spawn_episode(e.timestamp, e.resource.clone());
            self.feed(e);
        }
    }

    /// Buffered detour material becomes a context of its own — unless a
    /// suspended episode claims it first. Resumption is tested on every
    /// buffered resource, in order.
    /// Rerun an excursion's material through the full routing machinery,
    /// content tests first. Callers have already closed the episode it
    /// interrupted; each event resumes something else or stands up on its
    /// own — an excursion's leftovers are never silently grafted together.
    pub(crate) fn flush_detour_as_context(&mut self, buf: Vec<Event>) {
        for ev in buf {
            self.route(&ev);
        }
    }

    /// Feed one event into the open episode: discounted attention, focus-set
    /// maintenance, and production recording.
    pub(crate) fn feed(&mut self, e: &Event) {
        let cfg = self.st.cfg.clone();
        let now = e.timestamp;
        let mut credit = self.production_credit(e);
        // Downloads count once per normalized target. Pulling the same file
        // down three times is one landing, not three works' worth of anchors —
        // and it must never hijack the resume point.
        if e.interaction == Interaction::Downloaded && credit > 0 {
            let norm = normalize_download_target(&e.resource);
            match self.st.downloaded_targets.get(&norm) {
                Some(first_at) => {
                    let first_at = *first_at;
                    self.st.push(Decision::RepeatedDownload {
                        resource: e.resource.clone(),
                        normalized: norm,
                        first_at,
                    });
                    credit = 0;
                }
                None => {
                    self.st.downloaded_targets.insert(norm, e.timestamp);
                }
            }
        }
        let open_id = match self.st.open {
            Some(id) => id,
            None => return,
        };
        let credit_kind = prod_kind_of(e);
        let ep = match self.st.episodes.get_mut(&open_id) {
            Some(ep) => ep,
            None => return,
        };

        let dwell = match e.interaction {
            Interaction::Focused => e.weight.unwrap_or(cfg.focus_tick),
            // A navigation may carry the dwell the caller measured (ms in
            // `weight`); without one it is a witnessed passage, ticked.
            Interaction::Visited => e.weight.unwrap_or(cfg.visited_tick),
            _ => 0,
        };
        let part = ep
            .participants
            .entry(e.resource.clone())
            .or_insert_with(|| Participant {
                remaining_credit: cfg.max_attention_interval,
                first_contact: Some(now),
                ..Participant::default()
            });
        // Every person contact re-arms the dwell window; each accrual spends
        // it. A window left focused for the night can yield one interval only.
        part.remaining_credit = cfg.max_attention_interval;
        let credited = dwell.min(part.remaining_credit);
        part.remaining_credit = part.remaining_credit.saturating_sub(credited);
        part.attention += credited as f64;
        part.last_contact = now;

        ep.focus.insert(e.resource.clone());
        let gap = cfg.gap_close;
        let stale: Vec<String> = ep
            .focus
            .iter()
            .filter(|r| {
                *r != &e.resource
                    && ep
                        .participants
                        .get(*r)
                        .map(|p| now.saturating_sub(p.last_contact) > gap)
                        .unwrap_or(true)
            })
            .cloned()
            .collect();
        for r in stale {
            ep.focus.remove(&r);
        }
        ep.last_person_ts = now;
        ep.span.1 = now;
        if credit > 0 {
            if ep.first_production_ts.is_none() {
                ep.first_production_ts = Some(now);
            }
            let c = ep
                .credits
                .entry(e.resource.clone())
                .or_insert(CreditDetail {
                    amount: 0,
                    committed: 0,
                    last_kind: credit_kind,
                });
            c.amount += credit;
            c.last_kind = credit_kind;
        }
    }

    pub(crate) fn spawn_episode(&mut self, at: u64, resource: String) -> EpisodeId {
        let id = self.st.next_episode;
        self.st.next_episode += 1;
        self.st.episodes.insert(
            id,
            EpisodeState {
                id,
                span: (at, at),
                open: true,
                closed_at: at,
                assigned: None,
                parked: false,
                participants: Default::default(),
                credits: Default::default(),
                focus: Default::default(),
                dips: Vec::new(),
                last_person_ts: at,
                fork_candidates: None,
                first_production_ts: None,
            },
        );
        self.st.open = Some(id);
        self.st.push(Decision::EpisodeStarted {
            episode: id,
            at,
            resource,
        });
        id
    }

    /// Close the open episode. Assigned episodes commit their deltas; all
    /// others face the assignment cascade.
    pub(crate) fn close_episode_at(&mut self, at: u64) {
        let id = match self.st.open.take() {
            Some(id) => id,
            None => return,
        };
        self.st.detour = None;
        if let Some(ep) = self.st.episodes.get_mut(&id) {
            ep.open = false;
            ep.closed_at = at;
            ep.span.1 = at.max(ep.span.0);
        }
        self.st.push(Decision::EpisodeClosed {
            episode: id,
            at,
        });
        let assigned = self
            .st
            .episodes
            .get(&id)
            .and_then(|ep| ep.assigned)
            .is_some();
        if assigned {
            self.commit_episode(id);
        } else {
            self.assign_episode(id);
        }
    }

    /// The content test: does any recently-closed episode have this resource
    /// among its anchors (best) or its meaningfully-attended participants?
    fn resume_match(&self, resource: &str) -> Option<EpisodeId> {
        // Ambience never revives a context.
        if self
            .st
            .ledger
            .get(resource)
            .map(|l| l.is_passive())
            .unwrap_or(false)
        {
            return None;
        }
        let cfg = &self.st.cfg;
        let now = self.st.now;
        let mut best: Option<(u8, u64, std::cmp::Reverse<u64>)> = None;
        for ep in self.st.episodes.values() {
            if ep.open {
                continue;
            }
            if now.saturating_sub(ep.closed_at) > cfg.resume_window {
                continue;
            }
            let score = if ep.credits.contains_key(resource) {
                2u8
            } else if ep
                .participants
                .get(resource)
                .map(|p| p.attention >= cfg.min_meaningful_attention as f64)
                .unwrap_or(false)
            {
                1u8
            } else {
                continue;
            };
            let key = (score, ep.closed_at, std::cmp::Reverse(ep.id));
            if best.as_ref().map(|b| &key > b).unwrap_or(true) {
                best = Some(key);
            }
        }
        best.map(|(_, _, rid)| rid.0)
    }

    fn reopen(&mut self, id: EpisodeId, via: &str) {
        let at = self.st.now;
        if let Some(ep) = self.st.episodes.get_mut(&id) {
            ep.open = true;
            ep.closed_at = at;
        }
        self.st.open = Some(id);
        self.st.push(Decision::EpisodeReopened {
            episode: id,
            at,
            via: via.to_string(),
        });
    }

    /// The churn rule: a mutation counts as the person's production only when
    /// a person was actually at that resource shortly before. Everything else
    /// is background with zero evidence weight — this is the whole defense
    /// against a service rewriting forty files beside real work.
    pub(crate) fn apply_mutated(&mut self, e: &Event) {
        let cfg = self.st.cfg.clone();
        let contact = self
            .st
            .ledger
            .get(&e.resource)
            .and_then(|l| l.last_person_contact);
        let pass = contact
            .map(|t| e.timestamp.saturating_sub(t) <= cfg.churn_window)
            .unwrap_or(false);
        if !pass {
            self.st.push(Decision::EvidenceDropped {
                resource: e.resource.clone(),
                reason: "changed state with no person at it (background churn)".to_string(),
            });
            return;
        }
        let (credit, kind) = {
            let pending = self
                .st
                .ledger
                .get(&e.resource)
                .and_then(|l| l.pending_command);
            match pending {
                Some(cmd_ts) if e.timestamp.saturating_sub(cmd_ts) <= cfg.churn_window => {
                    (cfg.command_credit, ProdKind::CommandRan)
                }
                _ => (cfg.mutated_credit, ProdKind::Mutated),
            }
        };
        {
            let l = self.st.ledger_mut(&e.resource);
            l.pending_command = None;
            l.last_churn_passing_mutated = Some(e.timestamp);
        }
        // The mutation corroborates work only where the person already is;
        // it never moves attention on its own.
        if let Some(open_id) = self.st.open {
            let attended = self
                .st
                .episodes
                .get(&open_id)
                .map(|ep| {
                    ep.focus.contains(&e.resource) || ep.participants.contains_key(&e.resource)
                })
                .unwrap_or(false);
            if attended {
                if let Some(ep) = self.st.episodes.get_mut(&open_id) {
                    let c = ep
                        .credits
                        .entry(e.resource.clone())
                        .or_insert(CreditDetail {
                            amount: 0,
                            committed: 0,
                            last_kind: kind,
                        });
                    c.amount += credit;
                    c.last_kind = kind;
                }
                return;
            }
        }
        self.st.push(Decision::EvidenceDropped {
            resource: e.resource.clone(),
            reason: "mutated outside any attended context; kept as state, not work".to_string(),
        });
    }
}

fn prod_kind_of(e: &Event) -> ProdKind {
    match e.interaction {
        Interaction::Typed => ProdKind::Typed,
        Interaction::Saved => ProdKind::Saved,
        Interaction::Downloaded => ProdKind::Downloaded,
        Interaction::CommandRan => ProdKind::CommandRan,
        _ => ProdKind::Mutated,
    }
}

/// Canonical form of a download target for dedup purposes. The one naming
/// transformation allowed: a trailing copy counter — " (1)", " (2)" — sitting
/// immediately before the final extension is not a new file. (" (final)" and
/// any other parenthetical survive; only bare digits are treated as copies.)
fn normalize_download_target(p: &str) -> String {
    let Some(dot) = p.rfind('.') else {
        return p.to_string();
    };
    let (stem, ext) = p.split_at(dot);
    if let Some(open) = stem.rfind(" (") {
        if stem.ends_with(')') {
            let num = &stem[open + 2..stem.len() - 1];
            if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                return format!("{}{}", &stem[..open], ext);
            }
        }
    }
    p.to_string()
}

fn detour_resource_list(d: &Detour) -> Vec<String> {
    let mut v: Vec<String> = d.events.iter().map(|e| e.resource.clone()).collect();
    v.sort();
    v.dedup();
    v
}
