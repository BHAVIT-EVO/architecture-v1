//! Observation Schema identity.
//!
//! Every Observation conforms to exactly one immutable [`ObservationSchema`].
//! Schemas evolve by versioning. Previously accepted Observations never
//! migrate to newer schemas. (Observation Model, OA-6)

use std::fmt;

// ── Error ────────────────────────────────────────────────────────────────────

/// Errors that can occur when constructing an [`ObservationSchema`].
///
/// Kept in this module because the error belongs to schema construction.
/// Higher-layer error aggregation (e.g. `errors.rs`) may re-export or wrap
/// this type; that decision belongs to those layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// The schema name was empty.
    ///
    /// A schema with no name cannot identify anything and violates the
    /// requirement that identity be stable and meaningful across observations.
    EmptyName,
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::EmptyName => {
                write!(f, "observation schema name must not be empty")
            }
        }
    }
}

impl std::error::Error for SchemaError {}

// ── ObservationSchema ─────────────────────────────────────────────────────────

/// Identifies an Observation Schema by name and version.
///
/// An [`ObservationSchema`] is the stable identity under which a body of
/// Evidence was observed. Every Observation conforms to exactly one
/// [`ObservationSchema`], and that conformance is permanent.
///
/// Schemas evolve by incrementing their version. A schema at version 2 is a
/// distinct identity from the same name at version 1. Previously accepted
/// Observations never migrate to a newer version.
///
/// # Invariants
///
/// - `name` is never empty.
/// - `version` is any [`u32`]; zero is a valid initial version.
/// - Once constructed, both fields are immutable.
///
/// # Responsibilities
///
/// - Identify an Observation Schema by name and version.
/// - Provide stable equality and hashing so schemas can be used as lookup keys.
///
/// # Non-Responsibilities
///
/// - Does **not** validate evidence structure against the schema.
/// - Does **not** describe the fields or shape of evidence.
/// - Does **not** store, persist, register, or serialize schemas.
/// - Does **not** know about any registry of known schemas.
/// - Does **not** know about any Observation, Candidate, or Evidence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObservationSchema {
    /// Human-readable schema name. Never empty.
    name: String,
    /// Schema version. Incremented when the schema evolves.
    version: u32,
}

impl ObservationSchema {
    /// Constructs an [`ObservationSchema`] with the given `name` and `version`.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaError::EmptyName`] if `name` is empty after conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use evo_observation::observation_schema::ObservationSchema;    ///
    /// let schema = ObservationSchema::new("app_focus", 1).unwrap();
    /// assert_eq!(schema.name(), "app_focus");
    /// assert_eq!(schema.version(), 1);
    /// ```
    pub fn new(name: impl Into<String>, version: u32) -> Result<Self, SchemaError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SchemaError::EmptyName);
        }
        Ok(Self { name, version })
    }

    /// Returns the schema name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the schema version.
    pub fn version(&self) -> u32 {
        self.version
    }
}

/// Displays the schema as `name/version`.
///
/// This format is stable and suitable for logging and diagnostics.
/// It is not a serialization format; storage representation is
/// defined by the persistence layer.
///
/// # Example
///
/// ```
/// use evo_observation::observation_schema::ObservationSchema;
///
/// let schema = ObservationSchema::new("app_focus", 2).unwrap();
/// assert_eq!(schema.to_string(), "app_focus/2");
/// ```
impl fmt::Display for ObservationSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.name, self.version)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn valid_construction_succeeds() {
        let schema = ObservationSchema::new("app_focus", 1);
        assert!(schema.is_ok());
    }

    #[test]
    fn empty_name_is_rejected() {
        let result = ObservationSchema::new("", 0);
        assert_eq!(result, Err(SchemaError::EmptyName));
    }

    #[test]
    fn version_zero_is_valid() {
        // Zero is a legitimate initial version — no minimum is imposed.
        let schema = ObservationSchema::new("window_switch", 0).unwrap();
        assert_eq!(schema.version(), 0);
    }

    #[test]
    fn version_max_u32_is_valid() {
        let schema = ObservationSchema::new("window_switch", u32::MAX).unwrap();
        assert_eq!(schema.version(), u32::MAX);
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    #[test]
    fn name_accessor_returns_correct_value() {
        let schema = ObservationSchema::new("file_open", 3).unwrap();
        assert_eq!(schema.name(), "file_open");
    }

    #[test]
    fn version_accessor_returns_correct_value() {
        let schema = ObservationSchema::new("file_open", 3).unwrap();
        assert_eq!(schema.version(), 3);
    }

    // ── Immutability via API surface ──────────────────────────────────────────
    //
    // There are no setters. The only way to get a different schema is to
    // construct a new one. This test documents that invariant by confirming
    // there is no mutation path through the public API.

    #[test]
    fn schema_fields_are_not_mutable_through_public_api() {
        let schema = ObservationSchema::new("app_focus", 1).unwrap();
        // name() and version() return by value / shared reference — no &mut path.
        let _name: &str = schema.name();
        let _version: u32 = schema.version();
        // If this test compiles, the invariant holds.
    }

    // ── Equality ──────────────────────────────────────────────────────────────

    #[test]
    fn same_name_and_version_are_equal() {
        let a = ObservationSchema::new("app_focus", 1).unwrap();
        let b = ObservationSchema::new("app_focus", 1).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_version_is_not_equal() {
        // Schema v1 and v2 are distinct identities (OA-6 / schema versioning).
        let v1 = ObservationSchema::new("app_focus", 1).unwrap();
        let v2 = ObservationSchema::new("app_focus", 2).unwrap();
        assert_ne!(v1, v2);
    }

    #[test]
    fn different_name_same_version_is_not_equal() {
        let a = ObservationSchema::new("app_focus", 1).unwrap();
        let b = ObservationSchema::new("window_switch", 1).unwrap();
        assert_ne!(a, b);
    }

    // ── Hashing ───────────────────────────────────────────────────────────────

    #[test]
    fn equal_schemas_hash_identically() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let a = ObservationSchema::new("app_focus", 1).unwrap();
        let b = ObservationSchema::new("app_focus", 1).unwrap();

        let hash_of = |s: &ObservationSchema| {
            let mut h = DefaultHasher::new();
            s.hash(&mut h);
            h.finish()
        };

        assert_eq!(hash_of(&a), hash_of(&b));
    }

    #[test]
    fn schema_usable_as_hashmap_key() {
        use std::collections::HashMap;

        let schema = ObservationSchema::new("app_focus", 1).unwrap();
        let mut map: HashMap<ObservationSchema, &str> = HashMap::new();
        map.insert(schema.clone(), "registered");

        assert_eq!(map.get(&schema), Some(&"registered"));
    }

    // ── Clone ─────────────────────────────────────────────────────────────────

    #[test]
    fn clone_produces_equal_schema() {
        let original = ObservationSchema::new("file_open", 5).unwrap();
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // ── Display ───────────────────────────────────────────────────────────────

    #[test]
    fn display_format_is_name_slash_version() {
        let schema = ObservationSchema::new("app_focus", 2).unwrap();
        assert_eq!(schema.to_string(), "app_focus/2");
    }

    #[test]
    fn display_format_version_zero() {
        let schema = ObservationSchema::new("window_switch", 0).unwrap();
        assert_eq!(schema.to_string(), "window_switch/0");
    }

    // ── SchemaError ───────────────────────────────────────────────────────────

    #[test]
    fn schema_error_displays_meaningful_message() {
        let msg = SchemaError::EmptyName.to_string();
        assert!(!msg.is_empty());
        // The message should not be opaque — it must convey the reason.
        assert!(msg.contains("name") || msg.contains("empty"));
    }

    #[test]
    fn schema_error_implements_std_error() {
        // Verify the trait bound compiles — no assertion needed.
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&SchemaError::EmptyName);
    }
}
