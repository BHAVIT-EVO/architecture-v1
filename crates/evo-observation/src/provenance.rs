//! Observation Provenance.
//!
//! Provenance permanently records how Evidence entered Evo.
//! Every Observation possesses exactly one [`Provenance`].
//! (Observation Model §4, OA-5; IS-0001 R-5, G-4, I-5)

use std::collections::HashMap;
use std::fmt;
use std::time::SystemTime;

// ── SourceError ───────────────────────────────────────────────────────────────

/// Errors that can occur when constructing an [`ObservationSource`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// The source name was empty.
    ///
    /// A source with no name cannot identify an observation origin.
    Empty,
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::Empty => write!(f, "observation source name must not be empty"),
        }
    }
}

impl std::error::Error for SourceError {}

// ── ObservationSource ─────────────────────────────────────────────────────────

/// Identifies the origin of an Observation — which system component or
/// observer produced the Evidence.
///
/// Examples: `"accessibility_api"`, `"file_system_watcher"`, `"input_monitor"`.
///
/// # Invariants
///
/// - The inner name is never empty.
/// - Immutable after construction.
///
/// # Responsibilities
///
/// - Carry a stable, human-readable label identifying the observation origin.
///
/// # Non-Responsibilities
///
/// - Does **not** know about observers, Evidence, Observations, or schemas.
/// - Does **not** validate that the named source actually exists.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObservationSource(String);

impl ObservationSource {
    /// Constructs an [`ObservationSource`] from the given name.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::Empty`] if `name` is empty after conversion.
    pub fn new(name: impl Into<String>) -> Result<Self, SourceError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SourceError::Empty);
        }
        Ok(Self(name))
    }

    /// Returns the source name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ObservationSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ── Provenance ────────────────────────────────────────────────────────────────

/// Permanently records how Evidence entered Evo.
///
/// Provenance preserves three of the four items the Observation Model
/// requires (Observation Model §4):
///
/// | Observation Model field | This type           |
/// |-------------------------|---------------------|
/// | origin                  | `source`            |
/// | chronology              | `observed_at`       |
/// | observation context     | `context`           |
/// # Invariants
///
/// - [`ObservationSource`] guarantees a non-empty origin at construction.
/// - `observed_at` is the wall-clock time the Evidence was acquired.
/// - `context` may be empty; an empty map is valid provenance context.
/// - All fields are immutable after construction.
///
/// # Responsibilities
///
/// - Permanently preserve the origin, chronology, and observation context
///   of exactly one Observation.
///
/// # Non-Responsibilities
///
/// - Does **not** validate Evidence.
/// - Does **not** store or interpret Evidence.
/// - Does **not** hold the Observation Schema (that is a sibling field).
/// - Does **not** persist itself.
/// - Does **not** assign Observation identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// Which observer or system component produced this Evidence.
    source: ObservationSource,
    /// Wall-clock moment at which the Evidence was acquired.
    observed_at: SystemTime,
    /// Open-ended key-value metadata about the observation circumstances
    /// (e.g. observer version, session identifier, host context).
    context: HashMap<String, String>,
}

impl Provenance {
    /// Constructs a [`Provenance`] record.
    ///
    /// Construction cannot fail: [`ObservationSource`] already guarantees
    /// non-emptiness, and the remaining fields are always-valid types.
    ///
    /// Pass an empty `HashMap` when no additional context is available.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::SystemTime;
    /// use std::collections::HashMap;
    /// use evo_observation::provenance::{ObservationSource, Provenance};
    ///
    /// let source = ObservationSource::new("accessibility_api").unwrap();
    /// let provenance = Provenance::new(source, SystemTime::now(), HashMap::new());
    /// ```
    pub fn new(
        source: ObservationSource,
        observed_at: SystemTime,
        context: HashMap<String, String>,
    ) -> Self {
        Self {
            source,
            observed_at,
            context,
        }
    }

    /// Returns the origin of this Observation.
    pub fn source(&self) -> &ObservationSource {
        &self.source
    }

    /// Returns the wall-clock time at which the Evidence was acquired.
    pub fn observed_at(&self) -> SystemTime {
        self.observed_at
    }

    /// Returns the observation context metadata.
    ///
    /// May be empty; an empty map is valid.
    pub fn context(&self) -> &HashMap<String, String> {
        &self.context
    }
}

impl fmt::Display for Provenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Provenance {{ source: {}, context_keys: {} }}",
            self.source,
            self.context.len()
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::SystemTime;

    // ── ObservationSource construction ────────────────────────────────────────

    #[test]
    fn source_valid_construction_succeeds() {
        assert!(ObservationSource::new("accessibility_api").is_ok());
    }

    #[test]
    fn source_empty_name_is_rejected() {
        assert_eq!(ObservationSource::new(""), Err(SourceError::Empty));
    }

    #[test]
    fn source_as_str_returns_name() {
        let src = ObservationSource::new("file_watcher").unwrap();
        assert_eq!(src.as_str(), "file_watcher");
    }

    #[test]
    fn source_display_equals_name() {
        let src = ObservationSource::new("input_monitor").unwrap();
        assert_eq!(src.to_string(), "input_monitor");
    }

    #[test]
    fn source_equal_names_are_equal() {
        let a = ObservationSource::new("accessibility_api").unwrap();
        let b = ObservationSource::new("accessibility_api").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn source_different_names_are_not_equal() {
        let a = ObservationSource::new("accessibility_api").unwrap();
        let b = ObservationSource::new("file_watcher").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn source_clone_produces_equal_value() {
        let original = ObservationSource::new("accessibility_api").unwrap();
        assert_eq!(original.clone(), original);
    }

    #[test]
    fn source_usable_as_hashmap_key() {
        use std::collections::HashMap;
        let src = ObservationSource::new("accessibility_api").unwrap();
        let mut map: HashMap<ObservationSource, u32> = HashMap::new();
        map.insert(src.clone(), 1);
        assert_eq!(map.get(&src), Some(&1));
    }

    // ── SourceError ───────────────────────────────────────────────────────────

    #[test]
    fn source_error_displays_meaningful_message() {
        let msg = SourceError::Empty.to_string();
        assert!(msg.contains("source") || msg.contains("empty") || msg.contains("name"));
    }

    #[test]
    fn source_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&SourceError::Empty);
    }

    // ── Provenance construction ───────────────────────────────────────────────

    #[test]
    fn provenance_constructs_with_empty_context() {
        let src = ObservationSource::new("accessibility_api").unwrap();
        let now = SystemTime::now();
        let _p = Provenance::new(src, now, HashMap::new());
        // construction must not panic
    }

    #[test]
    fn provenance_constructs_with_populated_context() {
        let src = ObservationSource::new("accessibility_api").unwrap();
        let now = SystemTime::now();
        let mut ctx = HashMap::new();
        ctx.insert("observer_version".to_string(), "0.1.0".to_string());
        ctx.insert("session_id".to_string(), "abc123".to_string());
        let p = Provenance::new(src, now, ctx);
        assert_eq!(p.context().len(), 2);
    }

    // ── Provenance accessors ──────────────────────────────────────────────────

    #[test]
    fn provenance_source_accessor_returns_correct_value() {
        let src = ObservationSource::new("file_watcher").unwrap();
        let p = Provenance::new(src.clone(), SystemTime::now(), HashMap::new());
        assert_eq!(p.source(), &src);
    }

    #[test]
    fn provenance_observed_at_accessor_returns_correct_value() {
        let src = ObservationSource::new("accessibility_api").unwrap();
        let now = SystemTime::now();
        let p = Provenance::new(src, now, HashMap::new());
        assert_eq!(p.observed_at(), now);
    }

    #[test]
    fn provenance_context_accessor_returns_map() {
        let src = ObservationSource::new("accessibility_api").unwrap();
        let mut ctx = HashMap::new();
        ctx.insert("k".to_string(), "v".to_string());
        let p = Provenance::new(src, SystemTime::now(), ctx);
        assert_eq!(p.context().get("k"), Some(&"v".to_string()));
    }

    #[test]
    fn provenance_empty_context_is_valid() {
        let src = ObservationSource::new("input_monitor").unwrap();
        let p = Provenance::new(src, SystemTime::now(), HashMap::new());
        assert!(p.context().is_empty());
    }

    // ── Provenance equality ───────────────────────────────────────────────────

    #[test]
    fn identical_provenances_are_equal() {
        let src_a = ObservationSource::new("accessibility_api").unwrap();
        let src_b = ObservationSource::new("accessibility_api").unwrap();
        let ts = SystemTime::UNIX_EPOCH;
        let p1 = Provenance::new(src_a, ts, HashMap::new());
        let p2 = Provenance::new(src_b, ts, HashMap::new());
        assert_eq!(p1, p2);
    }

    #[test]
    fn different_source_provenances_are_not_equal() {
        let ts = SystemTime::UNIX_EPOCH;
        let p1 = Provenance::new(
            ObservationSource::new("accessibility_api").unwrap(),
            ts,
            HashMap::new(),
        );
        let p2 = Provenance::new(
            ObservationSource::new("file_watcher").unwrap(),
            ts,
            HashMap::new(),
        );
        assert_ne!(p1, p2);
    }

    #[test]
    fn different_timestamp_provenances_are_not_equal() {
        let src_a = ObservationSource::new("accessibility_api").unwrap();
        let src_b = ObservationSource::new("accessibility_api").unwrap();
        let t1 = SystemTime::UNIX_EPOCH;
        let t2 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
        let p1 = Provenance::new(src_a, t1, HashMap::new());
        let p2 = Provenance::new(src_b, t2, HashMap::new());
        assert_ne!(p1, p2);
    }

    // ── Provenance clone ──────────────────────────────────────────────────────

    #[test]
    fn provenance_clone_produces_equal_value() {
        let src = ObservationSource::new("accessibility_api").unwrap();
        let mut ctx = HashMap::new();
        ctx.insert("k".to_string(), "v".to_string());
        let original = Provenance::new(src, SystemTime::UNIX_EPOCH, ctx);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // ── Provenance display ────────────────────────────────────────────────────

    #[test]
    fn provenance_display_is_non_empty() {
        let src = ObservationSource::new("accessibility_api").unwrap();
        let p = Provenance::new(src, SystemTime::now(), HashMap::new());
        assert!(!p.to_string().is_empty());
    }

    #[test]
    fn provenance_display_contains_source_name() {
        let src = ObservationSource::new("accessibility_api").unwrap();
        let p = Provenance::new(src, SystemTime::now(), HashMap::new());
        assert!(p.to_string().contains("accessibility_api"));
    }
}
