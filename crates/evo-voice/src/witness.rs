//! Execution-witness tagging: which subjects Evo opened itself.
//!
//! When the voice loop (or any execution path) restores a body of work, the
//! OS opens/raises windows. macOS capture then witnesses those windows as
//! ordinary focus observations — but they are not the person's acts, they are
//! Evo's. Fed back into the reconstruction they would corrupt grouping: the
//! engine would learn Evo's own restores as fresh user work.
//!
//! [`RestoredSubjects`] is the narrow contract between the producer (the
//! daemon's voice handler, which knows exactly what `evo_execution::restore`
//! opened from its [`crate::TriggerRestoreOutcome`]) and the consumer (the
//! daemon's capture pipeline, which marks matching observations with
//! [`evo_observation::Provenance::set_execution_origin`] so
//! `evo-engagement::interpret` skips them — audit history, never engagement
//! evidence).
//!
//! Matching is by witnessed subject string (URL or file path — the
//! document-grain subjects capture resolves), compared exactly on both sides.
//! Reserve before dispatch and reconcile only that operation's receipt.
//! TTL matching is correlation, not causal proof: genuine input must hand off
//! the subject immediately. Very slow effects may outlive this window.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long a restored subject counts as execution-witnessed.
///
/// A physical parameter: app-launch plus focus latency. Generous enough that
/// a slow app opening still lands inside the window; short enough that the
/// person's own later return to the same resource is theirs, not Evo's.
pub const RESTORED_SUBJECT_TTL: Duration = Duration::from_secs(30);

/// Hard bound including concurrent in-flight operations. Refuse reservations
/// at capacity rather than evict another operation's live suppression.
pub const MAX_RESTORE_RESERVATIONS: usize = 1024;

/// One original subject and its exact executor-classified locator, at most.
const MAX_RESTORE_ALIASES: usize = 2;

/// Opaque, single-use operation identity. Completion cannot recreate an entry
/// removed by handoff/expiry or remove a different operation's reservation.
#[derive(Debug)]
pub struct RestoreReservation(u64);

#[derive(Debug)]
struct Entry {
    aliases: Vec<String>,
    marked_at: Instant,
}

/// Subjects Evo's execution recently opened or raised.
///
/// Shared between the restore path (which marks) and the capture path (which
/// checks). `Send`-safe so it can live behind a `Mutex` shared across the
/// daemon's threads.
#[derive(Debug, Default)]
pub struct RestoredSubjects {
    marked: HashMap<u64, Entry>,
    next_operation: u64,
}

impl RestoredSubjects {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserves one exact subject before dispatch. `None` means no effect
    /// should be dispatched (empty subject, capacity or exhausted IDs).
    /// Callers synchronize this registry, then release the lock for the effect.
    pub fn reserve(&mut self, subject: &str) -> Option<RestoreReservation> {
        self.reserve_aliases(&[subject])
    }

    /// Atomically reserves one operation's explicit exact identities. No URI
    /// decoding, trimming or transitive alias inference. Duplicate identities
    /// collapse; empty identities or more than two distinct aliases refuse the
    /// entire reservation. Capacity counts operations, not aliases.
    pub fn reserve_aliases(&mut self, subjects: &[&str]) -> Option<RestoreReservation> {
        self.evict_expired();
        if subjects.is_empty() || self.marked.len() >= MAX_RESTORE_RESERVATIONS {
            return None;
        }
        let mut aliases: Vec<String> = Vec::new();
        for &subject in subjects {
            if subject.trim().is_empty() {
                return None;
            }
            if !aliases.iter().any(|alias| alias == subject) {
                if aliases.len() == MAX_RESTORE_ALIASES {
                    return None;
                }
                aliases.push(subject.to_string());
            }
        }
        let operation = self.next_operation.checked_add(1)?;
        self.next_operation = operation;
        self.marked.insert(
            operation,
            Entry {
                aliases,
                marked_at: Instant::now(),
            },
        );
        Some(RestoreReservation(operation))
    }

    /// Reconciles just this attempt. Success extends its TTL; failure releases
    /// just its reservation. A late receipt after handoff/expiry is a no-op.
    pub fn complete(&mut self, reservation: RestoreReservation, success: bool) {
        self.evict_expired();
        if success {
            if let Some(entry) = self.marked.get_mut(&reservation.0) {
                entry.marked_at = Instant::now();
            }
        } else {
            self.marked.remove(&reservation.0);
        }
    }

    /// The person's input/declaration supersedes all existing suppression for
    /// this subject, including in-flight attempts. Every matching operation
    /// loses all its aliases. New attempts get new IDs, never reviving old ones.
    pub fn handoff(&mut self, subject: &str) {
        self.evict_expired();
        self.marked
            .retain(|_, entry| !entry.aliases.iter().any(|alias| alias == subject));
    }

    /// Compatibility for callers that only have post-dispatch successes.
    ///
    /// The producer passes the `opened` and `raised` subjects from the
    /// restore outcome. Empty and whitespace-only subjects are ignored —
    /// they can never match a witnessed subject. Capacity failures are ignored
    /// by this legacy API; new dispatchers must use `reserve` and fail closed.
    pub fn mark(&mut self, subjects: impl IntoIterator<Item = impl AsRef<str>>) {
        for subject in subjects {
            if let Some(reservation) = self.reserve(subject.as_ref()) {
                self.complete(reservation, true);
            }
        }
    }

    /// Whether this exact subject has a pending or successful execution
    /// reservation within the TTL window (not a verified causal attribution).
    ///
    /// A hit does not consume the entry: one restore may produce several
    /// observations (focus, URL navigation) for the same subject.
    pub fn is_execution(&mut self, subject: &str) -> bool {
        self.evict_expired();
        self.marked
            .values()
            .any(|entry| entry.aliases.iter().any(|alias| alias == subject))
    }

    /// Expire even abandoned in-flight reservations (e.g. executor panicked).
    fn evict_expired(&mut self) {
        let now = Instant::now();
        self.marked.retain(|_, entry| {
            now.saturating_duration_since(entry.marked_at) < RESTORED_SUBJECT_TTL
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marked_subjects_hit() {
        let mut registry = RestoredSubjects::new();
        registry.mark(["https://example.com/doc", "file:///Users/p/report.md"]);
        assert!(registry.is_execution("https://example.com/doc"));
        assert!(registry.is_execution("file:///Users/p/report.md"));
    }

    #[test]
    fn unmarked_subjects_miss() {
        let mut registry = RestoredSubjects::new();
        registry.mark(["https://example.com/doc"]);
        assert!(!registry.is_execution("https://other.example/"));
        assert!(!registry.is_execution(""));
    }

    #[test]
    fn identities_are_exact_including_meaningful_slashes_and_whitespace() {
        let mut registry = RestoredSubjects::new();
        registry.mark(["https://example.com/doc/"]);
        assert!(!registry.is_execution("https://example.com/doc"));
        assert!(!registry.is_execution(" https://example.com/doc/"));
        assert!(registry.is_execution("https://example.com/doc/"));
        registry.mark(["/work/a "]);
        assert!(!registry.is_execution("/work/a"));
        // No second file-URI decoder: the executor's decoder is private.
        // Preserve identity rather than guessing equivalence/remote hosts.
        registry.mark(["file://remote/work/a", "file:///work/a%20b"]);
        assert!(!registry.is_execution("/work/a b"));
        assert!(!registry.is_execution("/work/a"));
    }

    #[test]
    fn whitespace_only_subjects_are_ignored() {
        let mut registry = RestoredSubjects::new();
        registry.mark(["   ", ""]);
        assert!(!registry.is_execution(""));
        assert!(!registry.is_execution("   "));
    }

    #[test]
    fn a_hit_does_not_consume_the_entry() {
        let mut registry = RestoredSubjects::new();
        registry.mark(["file:///Users/p/report.md"]);
        assert!(registry.is_execution("file:///Users/p/report.md"));
        // A restore produces several observations per subject (focus, URL);
        // the second one must still match.
        assert!(registry.is_execution("file:///Users/p/report.md"));
    }

    #[test]
    fn expired_entries_stop_matching() {
        let mut registry = RestoredSubjects::new();
        registry.mark(["https://example.com/doc"]);
        // Age the entry past the TTL by hand (the clock is real; the entry is not).
        for entry in registry.marked.values_mut() {
            entry.marked_at -= RESTORED_SUBJECT_TTL + Duration::from_secs(1);
        }
        assert!(!registry.is_execution("https://example.com/doc"));
    }

    #[test]
    fn expired_entries_are_evicted() {
        let mut registry = RestoredSubjects::new();
        registry.mark(["https://example.com/old"]);
        for entry in registry.marked.values_mut() {
            entry.marked_at -= RESTORED_SUBJECT_TTL + Duration::from_secs(1);
        }
        registry.mark(["https://example.com/new"]);
        assert_eq!(registry.marked.len(), 1);
        assert!(registry.is_execution("https://example.com/new"));
    }

    #[test]
    fn overlapping_attempt_failure_cannot_remove_other_attempt() {
        let mut registry = RestoredSubjects::new();
        let a = registry.reserve("a").unwrap();
        let b = registry.reserve("a").unwrap();
        assert!(registry.is_execution("a"));
        registry.complete(b, true);
        registry.complete(a, false);
        assert!(registry.is_execution("a"));
        let failed = registry.reserve("failed").unwrap();
        registry.complete(failed, false);
        assert!(!registry.is_execution("failed"));
    }

    #[test]
    fn handoff_defeats_late_success_and_failure_without_affecting_new_attempt() {
        let mut registry = RestoredSubjects::new();
        let a = registry.reserve("a").unwrap();
        let b = registry.reserve("a").unwrap();
        registry.mark(["other"]);
        registry.handoff("a");
        registry.complete(a, true);
        assert!(!registry.is_execution("a"));
        assert!(registry.is_execution("other"));
        let new = registry.reserve("a").unwrap();
        registry.complete(b, false);
        assert!(registry.is_execution("a"));
        registry.complete(new, false);
        assert!(!registry.is_execution("a"));
    }

    #[test]
    fn completion_extends_only_live_reservations() {
        let mut registry = RestoredSubjects::new();
        let live = registry.reserve("live").unwrap();
        registry.marked.get_mut(&live.0).unwrap().marked_at -= Duration::from_secs(20);
        let before = registry.marked[&live.0].marked_at;
        let id = live.0;
        registry.complete(live, true);
        assert!(registry.marked[&id].marked_at > before);
        let expired = registry.reserve("expired").unwrap();
        registry.marked.get_mut(&expired.0).unwrap().marked_at -= RESTORED_SUBJECT_TTL;
        registry.complete(expired, true);
        assert!(!registry.is_execution("expired"));
    }

    #[test]
    fn completed_alias_handoff_releases_whole_operation_only() {
        let aliases = ["file:///work/a b.md", "/work/a b.md"];
        for handoff in aliases {
            let mut registry = RestoredSubjects::new();
            let operation = registry.reserve_aliases(&aliases).unwrap();
            registry.complete(operation, true);
            registry.mark(["unrelated"]);
            registry.handoff(handoff);
            for alias in aliases {
                assert!(!registry.is_execution(alias));
            }
            assert!(registry.is_execution("unrelated"));
        }
    }

    #[test]
    fn old_alias_completion_cannot_touch_fresh_overlapping_operation() {
        let aliases = ["file:///work/a b.md", "/work/a b.md"];
        for success in [false, true] {
            let mut registry = RestoredSubjects::new();
            let old = registry.reserve_aliases(&aliases).unwrap();
            registry.handoff(aliases[1]);
            let fresh = registry.reserve_aliases(&aliases).unwrap();
            let before = registry.marked[&fresh.0].marked_at;
            registry.complete(old, success);
            assert_eq!(registry.marked.len(), 1);
            assert_eq!(
                registry.marked[&fresh.0].marked_at, before,
                "old success cannot renew fresh TTL"
            );
            registry.complete(fresh, false);
            assert!(!registry.is_execution(aliases[0]));
            assert!(!registry.is_execution(aliases[1]));
        }
    }

    #[test]
    fn alias_validation_and_capacity_are_atomic_and_count_operations() {
        let mut registry = RestoredSubjects::new();
        for invalid in [&[][..], &["a", ""][..], &["a", "b", "c"][..]] {
            assert!(registry.reserve_aliases(invalid).is_none());
            assert!(registry.marked.is_empty());
        }
        let deduped = registry.reserve_aliases(&["a", "a", "b"]).unwrap();
        assert_eq!(registry.marked[&deduped.0].aliases, ["a", "b"]);
        registry.complete(deduped, false);
        for _ in 0..MAX_RESTORE_RESERVATIONS - 1 {
            registry.reserve("unrelated").unwrap();
        }
        let last = registry.reserve_aliases(&["file:///a", "/a"]).unwrap();
        assert_eq!(registry.marked.len(), MAX_RESTORE_RESERVATIONS);
        assert!(registry.reserve_aliases(&["file:///b", "/b"]).is_none());
        assert!(!registry.is_execution("file:///b"));
        assert!(!registry.is_execution("/b"));
        assert!(registry.is_execution("unrelated"));
        registry.complete(last, false);
        assert!(!registry.is_execution("file:///a"));
        assert!(!registry.is_execution("/a"));
    }

    #[test]
    fn capacity_refuses_without_evicting_in_flight_operations() {
        let mut registry = RestoredSubjects::new();
        let first = registry.reserve("first").unwrap();
        for _ in 1..MAX_RESTORE_RESERVATIONS {
            assert!(registry.reserve("other").is_some());
        }
        assert!(registry.reserve("overflow").is_none());
        assert_eq!(registry.marked.len(), MAX_RESTORE_RESERVATIONS);
        assert!(registry.is_execution("first"));
        registry.complete(first, false);
        assert!(registry.reserve("new").is_some());
    }
}
