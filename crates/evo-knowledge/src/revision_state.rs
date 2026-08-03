//! revision_state.rs

use std::fmt;

/// Represents the current validity of a Knowledge constraint.
///
/// Follows IS-0015 §7: Revision State SHALL represent only the current validity.
/// This specification defines exactly three states: Current, Weakened, Invalidated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionState {
    /// The constraint is fully supported and actively applicable.
    Current,
    /// The constraint has contradictory evidence but remains provisionally applicable.
    Weakened,
    /// The constraint is no longer supported by evidence.
    Invalidated,
}

impl fmt::Display for RevisionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RevisionState::Current => write!(f, "Current"),
            RevisionState::Weakened => write!(f, "Weakened"),
            RevisionState::Invalidated => write!(f, "Invalidated"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_state_implements_required_traits() {
        let state1 = RevisionState::Current;
        let state2 = state1.clone();
        assert_eq!(state1, state2);
    }

    #[test]
    fn display_formatting() {
        assert_eq!(RevisionState::Current.to_string(), "Current");
        assert_eq!(RevisionState::Weakened.to_string(), "Weakened");
        assert_eq!(RevisionState::Invalidated.to_string(), "Invalidated");
    }
    
    #[test]
    fn distinct_states_are_not_equal() {
        assert_ne!(RevisionState::Current, RevisionState::Weakened);
        assert_ne!(RevisionState::Current, RevisionState::Invalidated);
        assert_ne!(RevisionState::Weakened, RevisionState::Invalidated);
    }
}