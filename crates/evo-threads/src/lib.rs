//! # evo-threads
//!
//! A deterministic, local, offline inference engine that reconstructs
//! *bodies of work* from a stream of computer-usage events, so a person can
//! resume any of them later with minimal mental reload.
//!
//! ## What this engine believes
//!
//! * **Attention is serial; work is not.** The person does one thing at a
//!   time, in interrupted bursts, across days. The engine therefore segments
//!   time into *episodes* of attention, and treats a work as the
//!   continuation of episodes — never as a set of resources. One browser,
//!   one terminal, one repo may serve many works without merging them.
//! * **Evidence has a caste system.** What you *made* (typed, saved,
//!   downloaded, changed-by-command) carries identity. What you kept
//!   returning to corroborates. What merely co-occurred decorates. Background
//!   mutation without a person present is churn and votes nowhere.
//! * **Ambiguity is retained, never guessed.** When two works tie for an
//!   episode, both alternatives are kept and shown. When attention has no
//!   production and no recurrence, nothing is minted — a 47,000-second film
//!   is still not a project.
//! * **Merging two established works happens only when the person says so.**
//!   The merge code path takes a declaration or it does not run; nothing in
//!   inference can reach it. Splitting is cheap; merging is sacred.
//! * **Everything is a pure fold over the log.** No clock but the last
//!   event's timestamp — or an explicit `settle(at)` tick the daemon injects
//!   at read time — no randomness, no iteration order that can leak.
//!   `Engine::replay(log)` == folding `apply` over the log, bit for bit.
//! * **This engine knows nothing about your tools.** No application names,
//!   no domains, no folder shapes, no languages. A resource is an opaque
//!   string that the person either made something on, returned to, or
//!   glanced at. That is all.

mod assign;
pub mod retrieval;
mod config;
mod continuation;
mod episodes;
mod state;
mod trace;
mod types;

pub use config::Config;
pub use trace::{render as render_decision, Decision, ParkReason, Witness};
pub use types::*;

use state::State;

/// The engine: an event-sourced pure function of its input log.
#[derive(Clone, Debug, PartialEq)]
pub struct Engine {
    pub(crate) st: State,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            st: State::new(Config::default()),
        }
    }

    pub fn with_config(cfg: Config) -> Self {
        Self {
            st: State::new(cfg),
        }
    }

    /// Feed one event. This is the complete input surface; there is no other
    /// way in. Declarations are events too — the person's word travels the
    /// same pipe as everything else, and outranks all of it.
    pub fn apply(&mut self, event: Event) {
        if !self.st.started {
            self.st.now = event.timestamp;
            self.st.started = true;
        }
        if event.timestamp > self.st.now {
            self.st.now = event.timestamp;
        }

        match event.interaction {
            Interaction::Declared => self.apply_declaration(&event),
            _ => {
                if self.st.quarantined.contains(&event.resource) {
                    self.st.push(Decision::EvidenceDropped {
                        resource: event.resource.clone(),
                        reason: "quarantined by declaration; permanently out of evidence"
                            .to_string(),
                    });
                } else {
                    match event.origin {
                        Origin::System => match event.interaction {
                            Interaction::Mutated => self.apply_mutated(&event),
                            _ => self.st.push(Decision::EvidenceDropped {
                                resource: event.resource.clone(),
                                reason: "system-origin act; state, not work".to_string(),
                            }),
                        },
                        Origin::Execution => {
                            // The product's own restores are provenance,
                            // never work. The resource is marked open so the
                            // restore set won't offer it back; the first real
                            // person contact performs the handoff.
                            let l = self.st.ledger_mut(&event.resource);
                            l.execution_open = true;
                            self.st.push(Decision::EvidenceDropped {
                                resource: event.resource.clone(),
                                reason: "restored by the product itself; awaiting your hand"
                                    .to_string(),
                            });
                        }
                        Origin::Person => self.apply_person(&event),
                    }
                }
            }
        }

        if self.st.sweep_dirty {
            self.sweep_parked();
        }
    }

    fn apply_person(&mut self, e: &Event) {
        let handoff;
        {
            let l = self.st.ledger_mut(&e.resource);
            handoff = l.execution_open;
            l.execution_open = false;
            // A mutation is never its own witness: person contact that would
            // corroborate churn must come from the person's attention acts,
            // not from the mutated event itself.
            if e.interaction != Interaction::Mutated {
                l.last_person_contact = Some(e.timestamp);
            }
            match e.interaction {
                Interaction::Focused => {
                    l.had_focus = true;
                    l.last_person_focus = Some(e.timestamp);
                }
                Interaction::Visited => l.had_visited = true,
                Interaction::Typed => {
                    l.had_typed = true;
                    if e.weight.unwrap_or(0) > 0 {
                        l.last_typed = Some(e.timestamp);
                    }
                }
                Interaction::Saved => {
                    l.had_saved = true;
                    l.last_saved = Some(e.timestamp);
                }
                Interaction::Downloaded => {
                    l.had_downloaded = true;
                    l.downloaded_at = Some(e.timestamp);
                    l.used_after_download = false;
                }
                Interaction::CommandRan => {
                    l.had_command = true;
                    l.pending_command = Some(e.timestamp);
                }
                _ => {}
            }
            // Touching a download after it landed closes the "unused" loop.
            let downloaded_before = l
                .downloaded_at
                .map(|d| d < e.timestamp)
                .unwrap_or(false);
            if downloaded_before
                && !l.used_after_download
                && matches!(
                    e.interaction,
                    Interaction::Focused
                        | Interaction::Visited
                        | Interaction::Typed
                        | Interaction::Saved
                )
            {
                l.used_after_download = true;
            }
        }
        if handoff {
            self.st.push(Decision::Handoff {
                resource: e.resource.clone(),
                at: e.timestamp,
                via: e.interaction,
            });
        }
        match e.interaction {
            Interaction::Mutated => self.apply_mutated(e),
            Interaction::Declared => {}
            _ => self.route(e),
        }
    }

    /// Advance the engine's clock with no new events — the daemon's read-side
    /// tick. Reads (`threads`, `resume_bundle`, `explain`) only ever show what
    /// has already been *decided*; a sitting still in progress is nothing yet,
    /// no matter how real it feels. The daemon calls this before reading: it
    /// applies exactly the cold boundary the next event would have forced —
    /// silence closes the sitting at its last act, pending excursions are
    /// re-routed content-first — and a call whose clock hasn't outlived the
    /// sitting is a no-op. Pure function of the log and `at`; the loop settles
    /// whatever the flush uncovers next.
    pub fn settle(&mut self, at: u64) {
        if !self.st.started || at <= self.st.now {
            return;
        }
        self.st.now = at;
        let gap_close = self.st.cfg.gap_close;
        loop {
            let Some(open_id) = self.st.open else { break };
            let last = self
                .st
                .episodes
                .get(&open_id)
                .map(|ep| ep.last_person_ts)
                .unwrap_or(0);
            if at.saturating_sub(last) <= gap_close {
                // The sitting itself is still warm; only a stale excursion
                // pending against it may be due.
                let stale_detour = self
                    .st
                    .detour
                    .as_ref()
                    .map(|d| at.saturating_sub(d.start) >= self.st.cfg.dip_max)
                    .unwrap_or(false);
                if !stale_detour {
                    break;
                }
                // Lifted before the close: closing clears the detour as part
                // of ending the sitting, which would discard the material.
                let d = self.st.detour.take().unwrap();
                self.close_episode_at(d.start);
                self.flush_detour_as_context(d.events);
                continue;
            }
            // Same ordering discipline as the gap fallback in `route`: the
            // excursion buffer comes out before the close consumes it.
            let pending = self.st.detour.take();
            self.close_episode_at(last);
            if let Some(d) = pending {
                self.flush_detour_as_context(d.events);
            }
            // The flushed context may itself leave a stale open episode
            // behind; settle it in turn.
        }
    }

    /// The audit surface: every decision, in order, with its witnesses. When
    /// a grouping is wrong, this log exists so nobody has to wonder where.
    pub fn trace(&self) -> Vec<Decision> {
        self.st.decisions.clone()
    }

    /// Replay a whole log. By construction this is a left fold of `apply`;
    /// the determinism invariants hold it to that.
    pub fn replay(events: &[Event]) -> Self {
        let mut e = Engine::new();
        for ev in events {
            e.apply(ev.clone());
        }
        e
    }
}
