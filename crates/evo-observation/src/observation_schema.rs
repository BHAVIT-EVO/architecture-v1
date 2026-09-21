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

    fn canonical(name: &'static str) -> Self {
        Self {
            name: name.to_string(),
            version: 1,
        }
    }

    /// Returns the frozen schema for `WindowFocusGained`.
    pub fn window_focus_gained_v1() -> Self {
        Self::canonical("OBS-WINDOW-FOCUS-GAINED")
    }

    /// Returns the v2 schema for `WindowFocusGained`.
    ///
    /// v2 marks the witnessed contract under which the observation also
    /// carries owning-process provenance in `Provenance::context` (the
    /// process that owned the focused window, BE-AUDIT-0001 §8.6 /
    /// BE-TRACE-0001 §2.1). The Evidence structure is unchanged: exactly one
    /// canonical fact (`WindowFocusGained` carrying the subject). v1
    /// observations remain canonical, readable, and interpretable; they
    /// simply carry no owning-process context.
    pub fn window_focus_gained_v2() -> Self {
        Self {
            name: "OBS-WINDOW-FOCUS-GAINED".to_string(),
            version: 2,
        }
    }

    /// v3 permits the directly witnessed, generic accessibility state defined
    /// by [`crate::observed_state::ObservedState`] in provenance context.
    pub fn window_focus_gained_v3() -> Self {
        Self { name: "OBS-WINDOW-FOCUS-GAINED".to_string(), version: 3 }
    }

    /// Returns the frozen schema for `FileSaved`.
    pub fn file_saved_v1() -> Self {
        Self::canonical("OBS-FILE-SAVED")
    }

    /// Returns the frozen schema for `URLNavigated`.
    pub fn url_navigated_v1() -> Self {
        Self::canonical("OBS-URL-NAVIGATED")
    }

    /// Returns the frozen schema for `CommitMade`.
    pub fn commit_made_v1() -> Self {
        Self::canonical("OBS-COMMIT-MADE")
    }

    /// Returns the frozen schema for `WorkDesignated` (RFC-0011, IS-0003
    /// §4.1.5).
    pub fn work_designated_v1() -> Self {
        Self::canonical("OBS-WORK-DESIGNATED")
    }

    /// Returns the frozen schema for `RepositoryMembership` (RFC-0012,
    /// IS-0003 §4.1.6).
    pub fn repository_membership_v1() -> Self {
        Self::canonical("OBS-REPOSITORY-MEMBERSHIP")
    }

    /// Returns the frozen schema for `WorkGrouped` (RFC-0012, IS-0003 §4.1.7).
    pub fn work_grouped_v1() -> Self {
        Self::canonical("OBS-WORK-GROUPED")
    }

    /// Returns the frozen schema for `ContinuationSurface` (RFC-0013,
    /// IS-0003 §4.1.8).
    pub fn continuation_surface_v1() -> Self {
        Self::canonical("OBS-CONTINUATION-SURFACE")
    }

    /// Returns the frozen schema for `InputActivity`: content-free input
    /// counters (keys, clicks, scrolls) aggregated over one flush bucket on
    /// one subject. The counts are the entire evidence; input content is
    /// never observed and cannot be reconstructed.
    pub fn input_activity_v1() -> Self {
        Self::canonical("OBS-INPUT-ACTIVITY")
    }

    /// Returns `true` when this schema is one of the frozen IS-0003 schemas.
    pub fn is_canonical(&self) -> bool {
        matches!(
            (self.name.as_str(), self.version),
            ("OBS-WINDOW-FOCUS-GAINED", 1)
                | ("OBS-WINDOW-FOCUS-GAINED", 2)
                | ("OBS-WINDOW-FOCUS-GAINED", 3)
                | ("OBS-FILE-SAVED", 1)
                | ("OBS-URL-NAVIGATED", 1)
                | ("OBS-COMMIT-MADE", 1)
                | ("OBS-WORK-DESIGNATED", 1)
                | ("OBS-REPOSITORY-MEMBERSHIP", 1)
                | ("OBS-WORK-GROUPED", 1)
                | ("OBS-CONTINUATION-SURFACE", 1)
                | ("OBS-INPUT-ACTIVITY", 1)
        )
    }

    /// Returns `true` when this schema is a reference-only evidence class
    /// (RFC-0011 §4, RFC-0012 §Artifact Interaction, RFC-0013 §Artifact
    /// Interaction): the Observation references canonical subjects other
    /// Observations have already established and SHALL NOT by itself
    /// establish an Artifact.
    pub fn is_reference_only(&self) -> bool {
        matches!(
            (self.name.as_str(), self.version),
            ("OBS-WORK-DESIGNATED", 1)
                | ("OBS-REPOSITORY-MEMBERSHIP", 1)
                | ("OBS-WORK-GROUPED", 1)
                | ("OBS-CONTINUATION-SURFACE", 1)
        )
    }

    /// Returns `true` when this schema carries the person's explicit word —
    /// a declaration made through a user channel, as opposed to a
    /// reference-only fact the capture layer witnessed on its own (a file's
    /// repository membership is observed by the file watcher, not spoken).
    ///
    /// The distinction matters for immediacy: a person's declaration is
    /// rare and intentional, so downstream derivation may settle on it at
    /// once; a machine-witnessed reference arrives in bursts with every
    /// build or sync and must never trigger per-record derivation.
    pub fn is_user_declaration(&self) -> bool {
        matches!(
            (self.name.as_str(), self.version),
            ("OBS-WORK-DESIGNATED", 1)
                | ("OBS-WORK-GROUPED", 1)
                | ("OBS-CONTINUATION-SURFACE", 1)
        )
    }

    /// Returns the canonical fact name associated with this schema.
    pub fn canonical_fact_name(&self) -> Option<&'static str> {
        match (self.name.as_str(), self.version) {
            ("OBS-WINDOW-FOCUS-GAINED", 1) => Some("WindowFocusGained"),
            ("OBS-WINDOW-FOCUS-GAINED", 2) => Some("WindowFocusGained"),
            ("OBS-WINDOW-FOCUS-GAINED", 3) => Some("WindowFocusGained"),
            ("OBS-FILE-SAVED", 1) => Some("FileSaved"),
            ("OBS-URL-NAVIGATED", 1) => Some("URLNavigated"),
            ("OBS-COMMIT-MADE", 1) => Some("CommitMade"),
            ("OBS-WORK-DESIGNATED", 1) => Some("WorkDesignated"),
            ("OBS-REPOSITORY-MEMBERSHIP", 1) => Some("RepositoryMembership"),
            ("OBS-WORK-GROUPED", 1) => Some("WorkGrouped"),
            ("OBS-CONTINUATION-SURFACE", 1) => Some("ContinuationSurface"),
            ("OBS-INPUT-ACTIVITY", 1) => Some("InputActivity"),
            _ => None,
        }
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
    fn window_focus_v2_is_canonical_not_reference_only_with_the_same_fact_name() {
        // OBS-WINDOW-FOCUS-GAINED/v2 (BE-TRACE-0001 §3.1): the v2 schema is a
        // canonical content schema — the pid rides in `Provenance::context`,
        // not in Evidence — so it must be canonical, must not be
        // reference-only, and must carry the same canonical fact name as v1
        // so identity derivation, locator classification, and subject
        // presentation keep working for v2 records.
        let v1 = ObservationSchema::window_focus_gained_v1();
        let v2 = ObservationSchema::window_focus_gained_v2();
        assert_ne!(v1, v2);
        assert_eq!(v1.name(), v2.name());
        assert_eq!(v1.version(), 1);
        assert_eq!(v2.version(), 2);
        assert!(v1.is_canonical());
        assert!(v2.is_canonical());
        assert!(!v1.is_reference_only());
        assert!(!v2.is_reference_only());
        assert_eq!(v1.canonical_fact_name(), Some("WindowFocusGained"));
        assert_eq!(v2.canonical_fact_name(), Some("WindowFocusGained"));
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
