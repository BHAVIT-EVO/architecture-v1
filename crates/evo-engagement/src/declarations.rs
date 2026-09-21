//! User declarations: ground truth that outranks inference.
//!
//! Everything else in this crate is inference from witnessed evidence, and
//! inference can be wrong. When the person has told Evo something directly,
//! that statement wins. Evo never argues with it, never quietly overrides it,
//! and never requires it — declarations refine reconstruction, they are not a
//! precondition for it.
//!
//! Four declarations exist, all of them frozen Observation schemas:
//!
//! - **Designated** work (`OBS-WORK-DESIGNATED`) — "this is what I am working
//!   on". Makes a body of work presentable regardless of how little attention
//!   Evo happened to witness.
//! - **Grouped** work (`OBS-WORK-GROUPED`) — "these two things belong
//!   together". Overrides measured affinity outright.
//! - **Continuation surface** (`OBS-CONTINUATION-SURFACE`) — "this is where I
//!   continue". Forces those resources together and makes them primary.
//!   Unlike the others this one *supersedes*: "where I continue" is a statement
//!   about the present, so a later declaration replaces an earlier one rather
//!   than adding to it (RFC-0013 §5). The groupings it implies do not
//!   supersede — having said two things belong together stays said, and
//!   correcting where you continue is not a retraction of that.
//! - **Repository membership** (`OBS-REPOSITORY-MEMBERSHIP`) — a witnessed
//!   structural fact about where something lives. Note that this is treated as
//!   *location*, exactly like a parent directory, and location alone never
//!   forms a body of work. Two files in one repository are not one task.

use std::collections::{BTreeMap, BTreeSet};
use std::time::SystemTime;

/// Statements the person made, and structural facts that were witnessed
/// directly rather than inferred.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Declarations {
    designated: BTreeSet<String>,
    grouped: BTreeSet<(String, String)>,
    continuation: BTreeSet<String>,
    /// When the currently-held continuation surface was declared, so a later
    /// declaration can supersede it and an earlier one cannot.
    continuation_at: Option<SystemTime>,
    containers: BTreeMap<String, String>,
}

impl Declarations {
    /// An empty set of declarations. Reconstruction works fully without any.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records "this is what I am working on".
    pub fn designate(&mut self, subject: impl Into<String>) {
        self.designated.insert(subject.into());
    }

    /// Records "these two things belong together".
    pub fn group(&mut self, first: impl Into<String>, second: impl Into<String>) {
        let (first, second) = (first.into(), second.into());
        if first == second {
            return;
        }
        // Canonically ordered so the pair is identical however it was stated.
        if first < second {
            self.grouped.insert((first, second));
        } else {
            self.grouped.insert((second, first));
        }
    }

    /// Records "this set of resources is where I continue", witnessed at
    /// `witnessed_at`.
    ///
    /// Supersession follows the canonical rule (RFC-0011 §5, RFC-0013 §5): a
    /// later Observation Time replaces the held surface, and on equal time the
    /// later record wins — both paths that call this feed records in canonical
    /// append order, so "later call" is exactly the canonical tie-break. An
    /// out-of-order arrival with an earlier moment does not un-say the current
    /// declaration.
    ///
    /// The groupings the surface implies are recorded regardless of
    /// supersession, and accumulate: correcting where you continue does not
    /// retract having said these things belong together.
    pub fn continue_from(
        &mut self,
        subjects: impl IntoIterator<Item = String>,
        witnessed_at: SystemTime,
    ) {
        let subjects: Vec<String> = subjects.into_iter().collect();
        // A declared surface is also a declared grouping of its members.
        for (index, first) in subjects.iter().enumerate() {
            for second in subjects.iter().skip(index + 1) {
                self.group(first.clone(), second.clone());
            }
        }
        if subjects.is_empty() {
            return;
        }
        if self
            .continuation_at
            .is_some_and(|held| witnessed_at < held)
        {
            return;
        }
        self.continuation = subjects.into_iter().collect();
        self.continuation_at = Some(witnessed_at);
    }

    /// Records the witnessed structural container of a resource, overriding
    /// the container implied by its own name.
    pub fn contain(&mut self, member: impl Into<String>, container: impl Into<String>) {
        self.containers.insert(member.into(), container.into());
    }

    /// Whether the person named this resource as their work.
    pub fn is_designated(&self, subject: &str) -> bool {
        self.designated.contains(subject)
    }

    /// Whether the person named this resource as a place they continue.
    pub fn is_continuation(&self, subject: &str) -> bool {
        self.continuation.contains(subject)
    }

    /// Whether the person stated these two belong together.
    pub fn is_grouped(&self, left: &str, right: &str) -> bool {
        match left.cmp(right) {
            std::cmp::Ordering::Less => {
                self.grouped.contains(&(left.to_string(), right.to_string()))
            }
            std::cmp::Ordering::Greater => {
                self.grouped.contains(&(right.to_string(), left.to_string()))
            }
            std::cmp::Ordering::Equal => false,
        }
    }

    /// Whether the person made any statement about this resource.
    pub fn mentions(&self, subject: &str) -> bool {
        self.is_designated(subject)
            || self.is_continuation(subject)
            || self
                .grouped
                .iter()
                .any(|(left, right)| left == subject || right == subject)
    }

    /// The declared container of a resource, if one was witnessed.
    pub fn container_of(&self, subject: &str) -> Option<&String> {
        self.containers.get(subject)
    }

    /// Every declared pair, canonically ordered.
    pub fn groupings(&self) -> impl Iterator<Item = &(String, String)> {
        self.grouped.iter()
    }

    /// Every designated subject.
    pub fn designations(&self) -> impl Iterator<Item = &String> {
        self.designated.iter()
    }

    /// Every subject of the declared continuation surface.
    pub fn continuations(&self) -> impl Iterator<Item = &String> {
        self.continuation.iter()
    }

    /// When the held continuation surface was declared, if one was.
    ///
    /// Exposed so a derived rewriting of these declarations can carry the
    /// moment with the surface: the two are inseparable, because without the
    /// moment supersession cannot be applied.
    pub fn continuation_witnessed_at(&self) -> Option<SystemTime> {
        self.continuation_at
    }

    /// Every witnessed containment, as `(member, container)`.
    pub fn containments(&self) -> impl Iterator<Item = (&String, &String)> {
        self.containers.iter()
    }

    /// Whether the person has said nothing at all.
    pub fn is_empty(&self) -> bool {
        self.designated.is_empty()
            && self.grouped.is_empty()
            && self.continuation.is_empty()
            && self.containers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(secs))
            .expect("representable")
    }

    #[test]
    fn a_grouping_is_the_same_however_it_is_stated() {
        let mut declarations = Declarations::new();
        declarations.group("b", "a");
        assert!(declarations.is_grouped("a", "b"));
        assert!(declarations.is_grouped("b", "a"));
    }

    #[test]
    fn a_continuation_surface_also_groups_its_members() {
        let mut declarations = Declarations::new();
        declarations.continue_from(
            ["a".to_string(), "b".to_string(), "c".to_string()],
            at(10),
        );
        assert!(declarations.is_grouped("a", "c"));
        assert!(declarations.is_continuation("b"));
    }

    #[test]
    fn a_later_surface_supersedes_the_one_it_corrects() {
        let mut declarations = Declarations::new();
        declarations.continue_from(["a".to_string(), "b".to_string()], at(10));
        declarations.continue_from(["b".to_string(), "c".to_string()], at(20));

        // Where the person continues is now what they last said, not the union
        // of everything they have ever said.
        assert!(!declarations.is_continuation("a"));
        assert!(declarations.is_continuation("b"));
        assert!(declarations.is_continuation("c"));
        // Correcting the surface did not retract the relationship: they did
        // once say a and b belong together, and that stays said.
        assert!(declarations.is_grouped("a", "b"));
        assert_eq!(declarations.continuation_witnessed_at(), Some(at(20)));
    }

    #[test]
    fn a_surface_arriving_late_but_witnessed_earlier_does_not_win() {
        let mut declarations = Declarations::new();
        declarations.continue_from(["b".to_string(), "c".to_string()], at(20));
        // A record appended afterwards but witnessed before: canonical time
        // decides, not arrival.
        declarations.continue_from(["a".to_string(), "b".to_string()], at(10));

        assert!(!declarations.is_continuation("a"));
        assert!(declarations.is_continuation("c"));
        assert_eq!(declarations.continuation_witnessed_at(), Some(at(20)));
    }

    #[test]
    fn an_equally_timed_surface_is_decided_by_append_order() {
        let mut declarations = Declarations::new();
        declarations.continue_from(["a".to_string()], at(10));
        declarations.continue_from(["b".to_string()], at(10));

        assert!(!declarations.is_continuation("a"));
        assert!(declarations.is_continuation("b"));
    }

    #[test]
    fn nothing_is_required() {
        assert!(Declarations::new().is_empty());
    }
}
