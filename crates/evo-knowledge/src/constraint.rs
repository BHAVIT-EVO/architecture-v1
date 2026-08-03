//! Architectural Constraint.
//!
//! A `Constraint` represents the current reusable architectural understanding
//! encoded in a `Knowledge`.
//!
//! # IS-0015 Specification
//!
//! IS-0015 §5 defines:
//!
//! - The Constraint is the Knowledge. It is not evidence, not explanation, not history.
//! - Constraint SHALL be treated as an opaque computational object.
//! - This specification deliberately does not prescribe its internal representation.
//! - Constraint SHALL be immutable after construction.
//! - Revision SHALL replace the entire Constraint.
//! - Revision SHALL NOT mutate an existing Constraint.
//!
//! # IS-0015 Invariant
//!
//! - KI-2: Every Knowledge represents exactly one architectural constraint.
//!
//! # Opaque Representation
//!
//! IS-0015 §5 explicitly does not prescribe the internal representation of a
//! `Constraint`. This implementation represents it as an opaque content value.
//!
//! No structure is imposed on the content. No interpretation is performed.
//! The caller is responsible for supplying content that represents a valid
//! architectural constraint derived from canonical evidence.
//!
//! # Non-Responsibilities
//!
//! - Does **not** interpret its content.
//! - Does **not** validate architectural meaning.
//! - Does **not** reference canonical objects (evidence is held by `SupportingEvidence`).
//! - Does **not** mutate after construction.

// ── Constraint ────────────────────────────────────────────────────────────────

/// The current reusable architectural understanding encoded in a `Knowledge`.
///
/// Opaque by specification (IS-0015 §5). Immutable after construction.
///
/// Revision replaces the entire `Constraint`; it never mutates it in place (IS-0015 §5, §9).
///
/// # Invariants
///
/// - Represents exactly one architectural constraint (KI-2).
/// - Immutable after construction (IS-0015 §5).
/// - Opaque: internal representation is not prescribed by IS-0015 §5.
///
/// # Non-Responsibilities
///
/// - Does **not** hold supporting evidence.
/// - Does **not** hold revision history.
/// - Does **not** interpret its own content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    /// The opaque content of the architectural constraint.
    ///
    /// IS-0015 §5 does not prescribe the internal representation.
    /// The content is preserved exactly as supplied at construction.
    content: String,
}

impl Constraint {
    /// Constructs an immutable `Constraint` with the supplied opaque content.
    ///
    /// The content is not interpreted. It is preserved exactly as supplied.
    ///
    /// # Parameters
    ///
    /// - `content`: the opaque representation of the architectural constraint.
    ///
    /// # Guarantees
    ///
    /// - Content is preserved exactly as supplied.
    /// - Immutable after construction (IS-0015 §5).
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Returns the opaque content of this architectural constraint.
    ///
    /// The content is the complete, opaque representation of the constraint.
    /// No structural interpretation is imposed.
    pub fn content(&self) -> &str {
        &self.content
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_and_accessor() {
        let c = Constraint::new("prefer behavioral signals over app-name heuristics");
        assert_eq!(
            c.content(),
            "prefer behavioral signals over app-name heuristics"
        );
    }

    #[test]
    fn clone_is_equal_to_original() {
        let a = Constraint::new("dwell time above 30s indicates primary focus");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_contents_are_not_equal() {
        let a = Constraint::new("constraint-alpha");
        let b = Constraint::new("constraint-beta");
        assert_ne!(a, b);
    }

    #[test]
    fn content_is_preserved_exactly() {
        let content = "scroll depth > 0.8 correlates with sustained reading intent";
        let c = Constraint::new(content);
        assert_eq!(c.content(), content);
    }

    #[test]
    fn string_and_str_both_construct() {
        let a = Constraint::new("from &str");
        let b = Constraint::new(String::from("from String"));
        assert_eq!(a.content(), "from &str");
        assert_eq!(b.content(), "from String");
    }

    #[test]
    fn constraint_is_immutable_after_construction() {
        // The type system enforces this: no &mut self methods exist.
        let c = Constraint::new("immutability test");
        let _ = c.content();
    }
}