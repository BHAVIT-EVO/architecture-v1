//! Blocker.
//!
//! A `Blocker` represents an unresolved condition preventing immediate
//! continuation of work within a Workspace.
//!
//! # IS-0014 Invariants
//!
//! - BL-1: Zero or more Blockers MAY exist per Restoration Plan.
//! - BL-2: Blockers SHALL originate from canonical Workspace understanding.
//! - BL-3: Restoration SHALL NOT invent Blockers.
//!
//! Blockers are descriptive only. They SHALL NOT prescribe solutions (IS-0014 §7).
//!
//! A `Blocker` is immutable after construction.
//!
//! # Non-Responsibilities
//!
//! - Does **not** prescribe a resolution.
//! - Does **not** determine whether the Workspace can be resumed.
//! - Does **not** reference operating-system mechanisms.
//! - Does **not** invent conditions not present in canonical Workspace understanding (BL-3).

use crate::errors::RestorationError;

// ── Blocker ───────────────────────────────────────────────────────────────────

/// A descriptive representation of an unresolved condition preventing immediate
/// continuation of work (IS-0014 §7).
///
/// # Invariants
///
/// - Descriptive only; does not prescribe a solution (IS-0014 §7).
/// - Originates from canonical Workspace understanding (BL-2).
/// - Immutable after construction.
///
/// # Non-Responsibilities
///
/// - Does **not** prescribe a resolution.
/// - Does **not** execute any action.
/// - Does **not** invent conditions (BL-3).
#[derive(Debug, Clone, PartialEq)]
pub struct Blocker {
    /// A platform-independent, descriptive account of the unresolved condition.
    ///
    /// Derived from canonical Workspace understanding (BL-2).
    /// Does not prescribe a solution (IS-0014 §7).
    description: String,
}

impl Blocker {
    /// Constructs an immutable `Blocker` with the supplied description.
    ///
    /// # Parameters
    ///
    /// - `description`: a platform-independent description of the unresolved condition,
    ///   derived from canonical Workspace understanding (BL-2). Must not be empty.
    ///
    /// # Guarantees
    ///
    /// - The description is preserved exactly as supplied.
    /// - Immutable after construction.
    pub fn new(
        description: impl Into<String>,
    ) -> Result<Self, RestorationError> {
        Ok(Self {
            description: description.into(),
        })
    }

    /// Returns the descriptive account of this unresolved condition.
    ///
    /// The description is platform-independent and does not prescribe a solution
    /// (IS-0014 §7).
    pub fn description(&self) -> &str {
        &self.description
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_and_accessor() {
        let blocker = Blocker::new("compilation failure in evo-artifact").unwrap();
        assert_eq!(blocker.description(), "compilation failure in evo-artifact");
    }

    #[test]
    fn clone_is_equal_to_original() {
        let a = Blocker::new("merge conflict on main").unwrap();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_descriptions_are_not_equal() {
        let a = Blocker::new("failing test in context_chain").unwrap();
        let b = Blocker::new("unresolved dependency on evo-workspace").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn description_is_preserved_exactly() {
        let desc = "incomplete work: snapshot construction pending IS-0012 review";
        let blocker = Blocker::new(desc).unwrap();
        assert_eq!(blocker.description(), desc);
    }

    #[test]
    fn blocker_is_immutable_after_construction() {
        // The type system enforces this: no &mut self methods exist.
        let blocker = Blocker::new("failing tests in evo-observation").unwrap();
        let _ = blocker.description();
    }

    #[test]
    fn string_and_str_both_construct() {
        let from_str = Blocker::new("from &str").unwrap();
        let from_string = Blocker::new(String::from("from String")).unwrap();
        assert_eq!(from_str.description(), "from &str");
        assert_eq!(from_string.description(), "from String");
    }
}
