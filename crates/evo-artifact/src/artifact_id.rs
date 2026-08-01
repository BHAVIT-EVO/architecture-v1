//! Artifact Identity.
//!
//! An Artifact represents Evo's current identity hypothesis about an external entity.
//! `ArtifactId` provides the stable computational reference for higher computational
//! layers. (IS-0004 R-3)
//!
//! This module defines only the value object. Identity inference and assignment
//! occur during Stage 3 of the Artifact Acceptance Pipeline (IS-0005).

use std::fmt;

// ── Error ────────────────────────────────────────────────────────────────────

/// Errors that can occur when constructing an [`ArtifactId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactIdError {
    /// The identity string was empty or contained only whitespace.
    ///
    /// An empty identifier cannot serve as a stable computational reference.
    Empty,
}

impl fmt::Display for ArtifactIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactIdError::Empty => write!(f, "artifact identifier must not be empty"),
        }
    }
}

impl std::error::Error for ArtifactIdError {}

// ── ArtifactId ───────────────────────────────────────────────────────────────

/// A stable computational reference to an Artifact.
///
/// `ArtifactId` answers the requirement that higher layers must reference Artifacts
/// rather than individual Observations when entity continuity is required
/// (RFC-0002 Requirement 4).
///
/// # Invariants
///
/// - The internal identifier is never empty or entirely whitespace.
/// - Immutable after construction.
///
/// # Responsibilities
///
/// - Provide a stable computational reference to an accepted Artifact.
/// - Provide equality and hashing for use as a stable computational identifier..
///
/// # Non-Responsibilities
///
/// - Does **not** generate its own value (assigned by the Acceptance Pipeline).
/// - Does **not** infer identity.
/// - Does **not** persist itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactId(String);

impl ArtifactId {
    /// Constructs an [`ArtifactId`] from the provided string representation.
    ///
    /// # Errors
    ///
    /// Returns [`ArtifactIdError::Empty`] if `id` is empty or only whitespace.
    pub fn new(id: impl Into<String>) -> Result<Self, ArtifactIdError> {
        let id_str = id.into();
        if id_str.trim().is_empty() {
            return Err(ArtifactIdError::Empty);
        }
        Ok(Self(id_str))
    }

    /// Returns the string slice of the identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // ── Construction ─────────────────────────────────────────────────────────

    #[test]
    fn valid_construction_succeeds() {
        let id = ArtifactId::new("art-12345");
        assert!(id.is_ok());
        assert_eq!(id.unwrap().as_str(), "art-12345");
    }

    #[test]
    fn empty_string_is_rejected() {
        let err = ArtifactId::new("").unwrap_err();
        assert_eq!(err, ArtifactIdError::Empty);
    }

    #[test]
    fn whitespace_only_string_is_rejected() {
        let err = ArtifactId::new("   ").unwrap_err();
        assert_eq!(err, ArtifactIdError::Empty);
    }

    // ── Equality ─────────────────────────────────────────────────────────────

    #[test]
    fn identical_ids_are_equal() {
        let id1 = ArtifactId::new("entity-A").unwrap();
        let id2 = ArtifactId::new("entity-A").unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn different_ids_are_not_equal() {
        let id1 = ArtifactId::new("entity-A").unwrap();
        let id2 = ArtifactId::new("entity-B").unwrap();
        assert_ne!(id1, id2);
    }

    // ── Cloning ──────────────────────────────────────────────────────────────

    #[test]
    fn clone_produces_identical_value() {
        let id1 = ArtifactId::new("stable-ref-99").unwrap();
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    // ── Hashing ──────────────────────────────────────────────────────────────

    #[test]
    fn equal_ids_produce_equal_hashes() {
        let id1 = ArtifactId::new("hash-target").unwrap();
        let id2 = ArtifactId::new("hash-target").unwrap();

        let mut hasher1 = DefaultHasher::new();
        id1.hash(&mut hasher1);

        let mut hasher2 = DefaultHasher::new();
        id2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn usable_as_hashmap_key() {
        use std::collections::HashMap;

        let id = ArtifactId::new("key-1").unwrap();
        let mut map = HashMap::new();
        map.insert(id.clone(), "value");

        assert_eq!(map.get(&id), Some(&"value"));
    }

    // ── Display ──────────────────────────────────────────────────────────────

    #[test]
    fn display_formats_correctly() {
        let id = ArtifactId::new("display-test").unwrap();
        assert_eq!(id.to_string(), "display-test");
    }

    // ── Error Formatting ─────────────────────────────────────────────────────

    #[test]
    fn error_displays_correctly() {
        let err = ArtifactIdError::Empty;
        assert_eq!(err.to_string(), "artifact identifier must not be empty");
    }

    #[test]
    fn error_implements_std_error() {
        fn accepts_std_error(_err: &dyn std::error::Error) {}
        accepts_std_error(&ArtifactIdError::Empty);
    }

    // ── Immutability ─────────────────────────────────────────────────────────

    #[test]
    fn id_is_immutable_after_construction() {
        let id = ArtifactId::new("immutable-test").unwrap();
        
        // Ensure there is no mutable accessor
        let reference: &str = id.as_str();
        assert_eq!(reference, "immutable-test");

        // If this test compiles, the struct successfully hides its internals
        // behind an immutable public API.
    }
}