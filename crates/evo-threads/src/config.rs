//! Every tunable in the engine, with the reason it exists.
//!
//! Product statement: these knobs bound *time and arithmetic*, never *meaning*.
//! None of them names an application, a domain, a folder shape, or a language.
//! If a default ever needs to differ per profession, the model — not the
//! constant — is wrong.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Fallback boundary for a run (ms, default 20 min). Gaps are the *last*
    /// resort for closing an episode; content-tested resumption is the first.
    /// Exists because attention sometimes genuinely goes away (lunch, sleep)
    /// and the engine must close without an event to tell it so.
    pub gap_close: u64,

    /// Maximum absorbed detour length (ms, default 5 min). A short,
    /// production-free excursion that returns to the working set is recorded
    /// as an interruption — context, never evidence. One that outlasts this
    /// bound is a genuine departure: the episode ends where attention left,
    /// and the departed material is re-routed content-first.
    /// Exists because a notification glance is not a change of work.
    pub dip_max: u64,

    /// Person-corroboration window for mutations (ms, default 120 s). A
    /// `Mutated` event counts as person production only if the same resource
    /// saw person focus or typing within this window before it. Exists because
    /// a background service rewriting forty cache files beside real work is
    /// indistinguishable from a save at the event level — but it never has a
    /// person attached to it.
    pub churn_window: u64,

    /// Dwell discount interval (ms, default 10 min). Per person-origin event on
    /// a resource, attention accrues at most this much. Exists because a window
    /// left focused overnight — or a 47,000-second movie — contributed one
    /// interval of attention, not the night.
    pub max_attention_interval: u64,

    /// Attention credited to a `Focused` event that carries no dwell weight
    /// (ms, default 1 s). Presence with no measurable dwell is recorded with a
    /// token tick: enough to be honest context, never enough to outrank
    /// measured work.
    pub focus_tick: u64,

    /// Attention credited to a bare `Visited` (ms, default 1 s). A navigation
    /// happened; how long the page mattered is unknown until focus says more.
    pub visited_tick: u64,

    /// Companion weight halflife (ms, default 3 days). Companions fade because
    /// context drifts; anchors do not decay because what you made doesn't stop
    /// being what you made.
    pub companion_halflife: u64,

    /// Resumption window for suspended episodes (ms, default 3 h). Within it,
    /// a returning run that touches an episode's anchor or meaningful
    /// participants *reopens* that episode rather than starting a new one.
    /// Exists because interrupted people resume, and resumption must be tested
    /// by content, not by the clock — the clock only bounds how long the offer
    /// stands.
    pub resume_window: u64,

    /// Hard cap on restore sets (default 5). Restoration reopens the
    /// neighborhood, not the museum.
    pub restore_cap: usize,

    /// How recent a companion's last contact must be (ms, default 30 min)
    /// for it to rejoin the restore set. The boundary between "still near
    /// you when you stopped" and "finished supporting work that served its
    /// purpose earlier in the sitting." A Google search tab consulted and
    /// left long before the episode ended is done; the chat the person was
    /// still typing in is not.
    pub restore_recency: u64,

    /// Minimum suppressed configuration weight (ms-of-attention units, default
    /// 30_000) before configuration alone may carry an assignment. Exists so
    /// one shared glance can never attach an episode to a work.
    pub config_min_weight: f64,

    /// Required dominance margins (default 1.5 / 1.2). Decisions require a
    /// winner by a margin; a near-tie forks (alternatives retained) instead of
    /// guessing. `anchor_margin_ratio` additionally requires an anchor
    /// candidate to dominate the episode's *new* production — this is what
    /// lets a long-standing shared resource stay put while genuinely new work
    /// is born around it.
    pub config_margin_ratio: f64,
    pub anchor_margin_ratio: f64,

    /// Distinct episodes on different occasions required before a merely
    /// re-attended resource may become a Recurrence anchor (default 2).
    pub min_anchor_recurrence: usize,

    /// Discounted attention that counts as "meaningful" (ms, default 30 s):
    /// below this, presence is a glance and cannot seed recurrence.
    pub min_meaningful_attention: u64,

    /// Minimum production weight for an episode to *birth* a thread (default
    /// 100 units: roughly one save plus a sentence of typing). Exists because
    /// identity must cost more than a reflex: a brief, terminal burst — a few
    /// words and a send at the end of the day — must not mint a work whose
    /// resume point is itself. Below the floor the episode is retained,
    /// unguessed, and may still be born later via recurrence or declaration.
    pub thread_birth_min_production: u64,

    /// Production credit schedule (units; the currency of "you made this").
    /// Typed counts its content-free weight; the rest are flat event credits.
    pub typed_unit: u64,        // multiplier per unit of Typed weight (default 1)
    pub saved_credit: u64,      // default 60
    pub downloaded_credit: u64, // default 80
    pub command_credit: u64,    // default 60 — a command that visibly changed its resource
    pub mutated_credit: u64,    // default 40 — a churn-passing mutation alone

    /// Strength granted to an anchor created by declaration (default 500):
    /// the person's word outweighs a session's typing, permanently.
    pub declared_anchor_strength: f64,

    /// Ceiling on a Recurrence anchor's strength, as a fraction of
    /// `saved_credit` (default 0.9). Returning often approaches — never reaches
    /// — the weight of having made something.
    pub recurrence_strength_cap_ratio: f64,

    /// Maximum thread-profile count for a resource to be promotable as a
    /// Recurrence anchor (default 2). Beyond that, recurrence is ubiquity,
    /// and ubiquity is ambience, not identity.
    pub recurrence_anchor_max_threads: usize,

    /// Inactivity after which a thread lists as Dormant (ms, default 2 days).
    /// Cosmetic only: identity, resumption and restore never consult it.
    pub dormant_after: u64,

    /// Window in which a focused resource counts as "already open" for
    /// restore-set exclusion (ms, default 10 min).
    pub open_now_window: u64,

    /// Profile-count at or above which a resource is excluded from resume-point
    /// candidacy (default 3): brief, terminal mutations on surfaces shared by
    /// many works never hijack where you "left off".
    pub resume_exclusion_ubiquity: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gap_close: 20 * 60_000,
            dip_max: 5 * 60_000,
            churn_window: 120_000,
            max_attention_interval: 10 * 60_000,
            focus_tick: 1_000,
            visited_tick: 1_000,
            companion_halflife: 3 * 86_400_000,
            resume_window: 3 * 3_600_000,
            restore_cap: 5,
            restore_recency: 30 * 60_000,
            config_min_weight: 30_000.0,
            config_margin_ratio: 1.5,
            anchor_margin_ratio: 1.2,
            min_anchor_recurrence: 2,
            min_meaningful_attention: 30_000,
            thread_birth_min_production: 100,
            typed_unit: 1,
            saved_credit: 60,
            downloaded_credit: 80,
            command_credit: 60,
            mutated_credit: 40,
            declared_anchor_strength: 500.0,
            recurrence_strength_cap_ratio: 0.9,
            recurrence_anchor_max_threads: 2,
            dormant_after: 2 * 86_400_000,
            open_now_window: 10 * 60_000,
            resume_exclusion_ubiquity: 3,
        }
    }
}
