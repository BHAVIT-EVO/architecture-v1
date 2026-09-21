//! Shared fixture builders. Times are minutes-from-midnight on a synthetic
//! day, rendered as epoch ms. Everything ends up as plain `Event`s; fixtures
//! are just stories told in the engine's own alphabet.

use evo_threads::*;

pub const BASE: u64 = 1_700_000_000_000;
pub const MIN: u64 = 60_000;

/// Minutes past midnight -> timestamp.
pub fn t(min: u64) -> u64 {
    BASE + min * MIN
}

pub fn ev(ts: u64, i: Interaction, r: &str) -> Event {
    Event {
        timestamp: ts,
        origin: Origin::Person,
        interaction: i,
        resource: r.to_string(),
        weight: None,
        detail: None,
    }
}

pub fn evw(ts: u64, i: Interaction, r: &str, w: u64) -> Event {
    Event {
        timestamp: ts,
        origin: Origin::Person,
        interaction: i,
        resource: r.to_string(),
        weight: Some(w),
        detail: None,
    }
}

pub fn sys(ts: u64, r: &str) -> Event {
    Event {
        timestamp: ts,
        origin: Origin::System,
        interaction: Interaction::Mutated,
        resource: r.to_string(),
        weight: None,
        detail: None,
    }
}

pub fn exe(ts: u64, i: Interaction, r: &str) -> Event {
    Event {
        timestamp: ts,
        origin: Origin::Execution,
        interaction: i,
        resource: r.to_string(),
        weight: None,
        detail: None,
    }
}

pub fn declare(ts: u64, d: Declaration) -> Event {
    Event {
        timestamp: ts,
        origin: Origin::Person,
        interaction: Interaction::Declared,
        resource: String::new(),
        weight: None,
        detail: Some(d),
    }
}

/// The thread holding `anchor` among its anchors, if any.
pub fn thread_with_anchor(e: &Engine, anchor: &str) -> Option<ThreadView> {
    e.threads()
        .into_iter()
        .find(|t| t.anchors.iter().any(|a| a.resource == anchor))
}

/// Every resource the engine currently holds as a thread anchor, anywhere.
pub fn all_anchor_resources(e: &Engine) -> Vec<String> {
    let mut v: Vec<String> = e
        .threads()
        .into_iter()
        .flat_map(|t| t.anchors.into_iter().map(|a| a.resource))
        .collect();
    v.sort();
    v
}

/// Everything the engine would reopen today, across all works.
pub fn all_restore_resources(e: &Engine) -> Vec<String> {
    let mut v: Vec<String> = e
        .threads()
        .into_iter()
        .flat_map(|t| e.resume_bundle(t.id).restore_set)
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every participant resource (with weight) in every episode of every thread.
pub fn all_thread_participants(e: &Engine) -> Vec<String> {
    let mut v = Vec::new();
    for t in e.threads() {
        for ep in t.episodes {
            for (r, w) in ep.participants {
                if w > 0.0 {
                    v.push(r);
                }
            }
        }
    }
    v.sort();
    v.dedup();
    v
}

/// True if resource appears anywhere in the engine's work surface: anchors,
/// companions, participants, restore sets, resume points.
pub fn leaks_into_work(e: &Engine, resource_prefix: &str) -> bool {
    let threads = e.threads();
    for t in &threads {
        if t.anchors.iter().any(|a| a.resource.starts_with(resource_prefix)) {
            return true;
        }
        if t.companions.iter().any(|(r, _)| r.starts_with(resource_prefix)) {
            return true;
        }
        if t.episodes.iter().any(|ep| {
            ep.participants
                .iter()
                .any(|(r, w)| *w > 0.0 && r.starts_with(resource_prefix))
        }) {
            return true;
        }
        let b = e.resume_bundle(t.id);
        if b.resume_point.starts_with(resource_prefix)
            || b.restore_set.iter().any(|r| r.starts_with(resource_prefix))
        {
            return true;
        }
    }
    false
}

/// Telling, in one pass, whether an interaction-class string shows up in
/// the engine's trace with a given payload fragment. Handy for auditing.
pub fn trace_has(e: &Engine, pred: impl Fn(&Decision) -> bool) -> bool {
    e.trace().iter().any(pred)
}
