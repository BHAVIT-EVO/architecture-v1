//! Observation Evidence.
//!
//! Evidence is the complete collection of directly observed facts produced by
//! exactly one Observation. It contains no interpretation and never changes
//! after acceptance. (Observation Model §3, OA-1, OA-2, OA-3; Law II)

use std::fmt;

// ── FactValue ─────────────────────────────────────────────────────────────────

/// A primitive value carried by an [`ObservedFact`].
///
/// The Observation Model defines no fixed fact types — the names of facts are
/// determined by Observation Schemas. This enum defines only the primitive
/// value representations that named facts may carry.
///
/// Adding a variant here is an architectural decision, not an implementation
/// convenience. These variants represent the primitive value types currently supported by the Observation Model. Future architectural revisions may introduce additional primitive value representations if required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactValue {
    /// A UTF-8 text value (app names, window titles, file paths, URLs).
    Text(String),
    /// A 64-bit signed integer (dwell times in µs, counts, byte offsets).
    Integer(i64),
    /// A boolean flag (is_focused, is_fullscreen, etc.).
    Boolean(bool),
    /// An opaque byte sequence for data that does not fit a typed primitive.
    ///
    /// Exists to keep `Text` semantically clean. Binary observation outputs
    /// must not be base64-encoded into `Text` — use this variant instead.
    Bytes(Vec<u8>),
}

impl fmt::Display for FactValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FactValue::Text(s) => write!(f, "\"{}\"", s),
            FactValue::Integer(n) => write!(f, "{}", n),
            FactValue::Boolean(b) => write!(f, "{}", b),
            FactValue::Bytes(b) => write!(f, "<{} bytes>", b.len()),
        }
    }
}

// ── FactError ─────────────────────────────────────────────────────────────────

/// Errors that can occur when constructing an [`ObservedFact`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactError {
    /// The fact name was empty.
    ///
    /// A fact without a name cannot be associated with a schema field and
    /// therefore cannot constitute valid observed evidence.
    EmptyName,
}

impl fmt::Display for FactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FactError::EmptyName => write!(f, "observed fact name must not be empty"),
        }
    }
}

impl std::error::Error for FactError {}

// ── ObservedFact ──────────────────────────────────────────────────────────────

/// An indivisible piece of information obtained directly through observation.
///
/// An [`ObservedFact`] pairs a schema-defined name with a primitive value.
/// It contains no inference. Its name is defined by the Observation Schema;
/// this type imposes no constraints on which names are valid beyond requiring
/// them to be non-empty.
///
/// # Invariants
///
/// - `name` is never empty.
/// - `value` is always a directly observed primitive — never inferred.
/// - Immutable after construction.
///
/// # Responsibilities
///
/// - Carry one directly observed, named, primitive fact.
///
/// # Non-Responsibilities
///
/// - Does **not** know which schema it belongs to.
/// - Does **not** validate that its name is schema-legal (that is validation's job).
/// - Does **not** interpret its value.
/// - Does **not** persist itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedFact {
    /// Schema-defined name identifying this fact. Never empty.
    name: String,
    /// The directly observed primitive value.
    value: FactValue,
}

impl ObservedFact {
    /// Constructs an [`ObservedFact`] with the given name and value.
    ///
    /// # Errors
    ///
    /// Returns [`FactError::EmptyName`] if `name` is empty after conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use evo_observation::evidence::{ObservedFact, FactValue};
    ///
    /// let fact = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
    /// assert_eq!(fact.name(), "app_name");
    /// ```
    pub fn new(name: impl Into<String>, value: FactValue) -> Result<Self, FactError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(FactError::EmptyName);
        }
        Ok(Self { name, value })
    }

    /// Returns the fact name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the fact value.
    pub fn value(&self) -> &FactValue {
        &self.value
    }
}

impl fmt::Display for ObservedFact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.name, self.value)
    }
}

// ── Evidence ──────────────────────────────────────────────────────────────────

/// The complete collection of directly observed facts from exactly one
/// Observation.
///
/// Evidence contains no interpretation. Every [`ObservedFact`] within it
/// originates from directly observed reality. Evidence is immutable after
/// construction and never changes after acceptance. (OA-1, OA-2, OA-3)
///
/// An empty collection is structurally valid. Whether a specific Observation
/// Schema permits empty Evidence is a concern for the validation layer, not
/// this type.
///
/// # Invariants
///
/// - All contained [`ObservedFact`]s were valid at construction time.
/// - No interpretation is present.
/// - Immutable after construction.
///
/// # Responsibilities
///
/// - Hold the complete, immutable body of directly observed facts for one
///   Observation.
/// - Provide read access to those facts by position and by name.
///
/// # Non-Responsibilities
///
/// - Does **not** validate fact names against a schema.
/// - Does **not** interpret, classify, rank, or summarize facts.
/// - Does **not** know about any Observation, Candidate, or Provenance.
/// - Does **not** persist itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    facts: Vec<ObservedFact>,
}

impl Evidence {
    /// Constructs an [`Evidence`] body from the given collection of facts.
    ///
    /// Construction cannot fail: every [`ObservedFact`] has already been
    /// validated at construction time.
    ///
    /// Pass an empty `Vec` only when the schema permits it; whether empty
    /// evidence is acceptable is enforced by the validation layer.
    ///
    /// # Examples
    ///
    /// ```
    /// use evo_observation::evidence::{Evidence, ObservedFact, FactValue};
    ///
    /// let fact = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
    /// let evidence = Evidence::new(vec![fact]);
    /// assert_eq!(evidence.len(), 1);
    /// ```
    pub fn new(facts: Vec<ObservedFact>) -> Self {
        Self { facts }
    }

    /// Returns all facts as a slice.
    pub fn facts(&self) -> &[ObservedFact] {
        &self.facts
    }

    /// Returns the number of facts in this Evidence body.
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    /// Returns `true` if this Evidence body contains no facts.
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Returns the first fact whose name matches `name`, or `None`.
    ///
    /// Performs a linear scan. If the schema guarantees unique fact names,
    /// this returns the only matching fact. If names are not unique, this
    /// returns the first match in insertion order.
    pub fn fact(&self, name: &str) -> Option<&ObservedFact> {
        self.facts.iter().find(|f| f.name() == name)
    }
}

impl fmt::Display for Evidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Evidence {{ facts: {} }}", self.facts.len())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── FactValue ─────────────────────────────────────────────────────────────

    #[test]
    fn fact_value_text_equality() {
        assert_eq!(
            FactValue::Text("Xcode".into()),
            FactValue::Text("Xcode".into())
        );
        assert_ne!(
            FactValue::Text("Xcode".into()),
            FactValue::Text("Safari".into())
        );
    }

    #[test]
    fn fact_value_integer_equality() {
        assert_eq!(FactValue::Integer(42), FactValue::Integer(42));
        assert_ne!(FactValue::Integer(42), FactValue::Integer(0));
    }

    #[test]
    fn fact_value_boolean_equality() {
        assert_eq!(FactValue::Boolean(true), FactValue::Boolean(true));
        assert_ne!(FactValue::Boolean(true), FactValue::Boolean(false));
    }

    #[test]
    fn fact_value_bytes_equality() {
        assert_eq!(
            FactValue::Bytes(vec![1, 2, 3]),
            FactValue::Bytes(vec![1, 2, 3])
        );
        assert_ne!(
            FactValue::Bytes(vec![1, 2, 3]),
            FactValue::Bytes(vec![4, 5, 6])
        );
    }

    #[test]
    fn fact_value_variants_are_not_equal_across_types() {
        // Text "42" is not the same as Integer 42 — distinct variants.
        assert_ne!(FactValue::Text("42".into()), FactValue::Integer(42));
    }

    #[test]
    fn fact_value_clone_produces_equal_value() {
        let v = FactValue::Text("Xcode".into());
        assert_eq!(v.clone(), v);
    }

    #[test]
    fn fact_value_display_text() {
        assert_eq!(FactValue::Text("Xcode".into()).to_string(), "\"Xcode\"");
    }

    #[test]
    fn fact_value_display_integer() {
        assert_eq!(FactValue::Integer(-7).to_string(), "-7");
    }

    #[test]
    fn fact_value_display_boolean_true() {
        assert_eq!(FactValue::Boolean(true).to_string(), "true");
    }

    #[test]
    fn fact_value_display_boolean_false() {
        assert_eq!(FactValue::Boolean(false).to_string(), "false");
    }

    #[test]
    fn fact_value_display_bytes_shows_length() {
        let display = FactValue::Bytes(vec![0u8; 5]).to_string();
        assert!(display.contains("5"));
    }

    #[test]
    fn fact_value_bytes_empty_is_valid() {
        let v = FactValue::Bytes(vec![]);
        assert_eq!(FactValue::Bytes(vec![]), v);
    }

    // ── ObservedFact construction ─────────────────────────────────────────────

    #[test]
    fn observed_fact_valid_construction_succeeds() {
        let f = ObservedFact::new("app_name", FactValue::Text("Xcode".into()));
        assert!(f.is_ok());
    }

    #[test]
    fn observed_fact_empty_name_is_rejected() {
        let result = ObservedFact::new("", FactValue::Boolean(true));
        assert_eq!(result, Err(FactError::EmptyName));
    }

    // ── ObservedFact accessors ────────────────────────────────────────────────

    #[test]
    fn observed_fact_name_accessor() {
        let f = ObservedFact::new("dwell_us", FactValue::Integer(5000)).unwrap();
        assert_eq!(f.name(), "dwell_us");
    }

    #[test]
    fn observed_fact_value_accessor() {
        let f = ObservedFact::new("is_focused", FactValue::Boolean(true)).unwrap();
        assert_eq!(f.value(), &FactValue::Boolean(true));
    }

    // ── ObservedFact immutability ─────────────────────────────────────────────

    #[test]
    fn observed_fact_has_no_mutation_path_through_public_api() {
        let f = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        let _: &str = f.name();
        let _: &FactValue = f.value();
        // No &mut path exists. If this compiles, the invariant holds.
    }

    // ── ObservedFact equality and clone ──────────────────────────────────────

    #[test]
    fn observed_facts_with_same_name_and_value_are_equal() {
        let a = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        let b = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn observed_facts_with_different_names_are_not_equal() {
        let a = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        let b = ObservedFact::new("window_title", FactValue::Text("Xcode".into())).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn observed_facts_with_different_values_are_not_equal() {
        let a = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        let b = ObservedFact::new("app_name", FactValue::Text("Safari".into())).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn observed_fact_clone_produces_equal_value() {
        let f = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        assert_eq!(f.clone(), f);
    }

    // ── ObservedFact display ──────────────────────────────────────────────────

    #[test]
    fn observed_fact_display_contains_name_and_value() {
        let f = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        let s = f.to_string();
        assert!(s.contains("app_name"));
        assert!(s.contains("Xcode"));
    }

    // ── FactError ─────────────────────────────────────────────────────────────

    #[test]
    fn fact_error_displays_meaningful_message() {
        let msg = FactError::EmptyName.to_string();
        assert!(msg.contains("name") || msg.contains("empty"));
    }

    #[test]
    fn fact_error_implements_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&FactError::EmptyName);
    }

    // ── Evidence construction ─────────────────────────────────────────────────

    #[test]
    fn evidence_constructs_with_empty_facts() {
        let e = Evidence::new(vec![]);
        assert!(e.is_empty());
    }

    #[test]
    fn evidence_constructs_with_single_fact() {
        let f = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        let e = Evidence::new(vec![f]);
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn evidence_constructs_with_multiple_facts() {
        let facts = vec![
            ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap(),
            ObservedFact::new("dwell_us", FactValue::Integer(12_000)).unwrap(),
            ObservedFact::new("is_focused", FactValue::Boolean(true)).unwrap(),
        ];
        let e = Evidence::new(facts);
        assert_eq!(e.len(), 3);
    }

    // ── Evidence accessors ────────────────────────────────────────────────────

    #[test]
    fn evidence_facts_returns_all_facts() {
        let f = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        let e = Evidence::new(vec![f.clone()]);
        assert_eq!(e.facts(), &[f]);
    }

    #[test]
    fn evidence_fact_lookup_by_name_returns_match() {
        let f = ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap();
        let e = Evidence::new(vec![f]);
        let found = e.fact("app_name").unwrap();
        assert_eq!(found.name(), "app_name");
    }

    #[test]
    fn evidence_fact_lookup_missing_name_returns_none() {
        let e = Evidence::new(vec![]);
        assert!(e.fact("nonexistent").is_none());
    }

    #[test]
    fn evidence_fact_lookup_returns_first_match() {
        // If a schema allows duplicate names, first match is returned.
        let f1 = ObservedFact::new("tag", FactValue::Text("alpha".into())).unwrap();
        let f2 = ObservedFact::new("tag", FactValue::Text("beta".into())).unwrap();
        let e = Evidence::new(vec![f1, f2]);
        let found = e.fact("tag").unwrap();
        assert_eq!(found.value(), &FactValue::Text("alpha".into()));
    }

    // ── Evidence immutability ─────────────────────────────────────────────────

    #[test]
    fn evidence_has_no_mutation_path_through_public_api() {
        let e = Evidence::new(vec![]);
        let _: &[ObservedFact] = e.facts();
        let _: usize = e.len();
        let _: bool = e.is_empty();
        // No &mut path exists. If this compiles, the invariant holds.
    }

    // ── Evidence equality and clone ───────────────────────────────────────────

    #[test]
    fn identical_evidence_bodies_are_equal() {
        let make = || {
            Evidence::new(vec![
                ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap(),
            ])
        };
        assert_eq!(make(), make());
    }

    #[test]
    fn evidence_with_different_facts_is_not_equal() {
        let a = Evidence::new(vec![
            ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap(),
        ]);
        let b = Evidence::new(vec![
            ObservedFact::new("app_name", FactValue::Text("Safari".into())).unwrap(),
        ]);
        assert_ne!(a, b);
    }

    #[test]
    fn evidence_clone_produces_equal_value() {
        let e = Evidence::new(vec![
            ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap(),
        ]);
        assert_eq!(e.clone(), e);
    }

    // ── Evidence display ──────────────────────────────────────────────────────

    #[test]
    fn evidence_display_is_non_empty() {
        let e = Evidence::new(vec![]);
        assert!(!e.to_string().is_empty());
    }

    #[test]
    fn evidence_display_reflects_fact_count() {
        let e = Evidence::new(vec![
            ObservedFact::new("app_name", FactValue::Text("Xcode".into())).unwrap(),
            ObservedFact::new("dwell_us", FactValue::Integer(5000)).unwrap(),
        ]);
        assert!(e.to_string().contains("2"));
    }
}
