//! Metrics over the canonical observation log.
//!
//! PLAN-0001 treats the person's seven design schemas as product feedback,
//! rather than as machine activity.  The denominator remains every canonical
//! observation, including machine-written `OBS-SURFACE-NAMED` observations.

/// The PLAN-0001 minimum design-event rate, expressed per 1,000 observations.
pub const DESIGN_EVENT_RATE_FLOOR_PER_1000: u64 = 5;

/// User-authored design schemas counted in the PLAN-0001 numerator.
///
/// `OBS-SURFACE-NAMED` is intentionally absent: it is a machine observation,
/// not a user design event.
pub const DESIGN_EVENT_SCHEMAS: [&str; 7] = [
    "OBS-WORK-DESIGNATED",
    "OBS-WORK-GROUPED",
    "OBS-WORK-NAMED",
    "OBS-CONTINUATION-SURFACE",
    "OBS-SURFACE-CLASSIFIED",
    "OBS-WORK-DISMISSED",
    "OBS-MEMBER-STATE",
];

/// Canonical observation schemas recognized by the ledger log reader.
///
/// Window-focus has three canonical schema versions; the log stores the
/// schema name and version separately, so the version check lives in
/// [`is_canonical_schema`].
const CANONICAL_SCHEMA_NAMES: [&str; 14] = [
    "OBS-WINDOW-FOCUS-GAINED",
    "OBS-FILE-SAVED",
    "OBS-URL-NAVIGATED",
    "OBS-COMMIT-MADE",
    "OBS-WORK-DESIGNATED",
    "OBS-REPOSITORY-MEMBERSHIP",
    "OBS-WORK-GROUPED",
    "OBS-WORK-NAMED",
    "OBS-CONTINUATION-SURFACE",
    "OBS-INPUT-ACTIVITY",
    "OBS-SURFACE-NAMED",
    "OBS-SURFACE-CLASSIFIED",
    "OBS-WORK-DISMISSED",
    "OBS-MEMBER-STATE",
];

/// Counts used by the regression report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObservationMetrics {
    /// Number of records using a canonical observation schema.
    pub observation_count: usize,
    /// Number of records using one of [`DESIGN_EVENT_SCHEMAS`].
    pub design_event_count: usize,
}

impl ObservationMetrics {
    /// Constructs metrics from deterministic counts.
    pub const fn from_counts(observation_count: usize, design_event_count: usize) -> Self {
        Self {
            observation_count,
            design_event_count,
        }
    }

    /// Counts canonical and user-design schema names from an iterator.
    ///
    /// This helper keeps the floor tests independent of a machine-local
    /// historical log while using the same schema classification as the log
    /// reader.
    pub fn from_schema_names<I, S>(schema_names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut metrics = Self::default();
        for schema_name in schema_names {
            metrics.observe_schema(schema_name.as_ref(), 1);
        }
        metrics
    }

    /// Returns the design-event rate per 1,000 canonical observations.
    pub fn rate_per_1000(self) -> f64 {
        if self.observation_count == 0 {
            0.0
        } else {
            self.design_event_count as f64 * 1000.0 / self.observation_count as f64
        }
    }

    /// Returns whether this sample meets the PLAN-0001 floor.
    ///
    /// Integer arithmetic makes the boundary deterministic: exactly 5 design
    /// events per 1,000 observations passes, while an empty sample does not.
    pub fn meets_design_event_floor(self) -> bool {
        self.observation_count > 0
            && (self.design_event_count as u128) * 1000
                >= (self.observation_count as u128) * DESIGN_EVENT_RATE_FLOOR_PER_1000 as u128
    }

    /// Adds one schema record to these metrics when it is canonical.
    pub(crate) fn observe_schema(&mut self, schema_name: &str, schema_version: u32) {
        if !is_canonical_schema(schema_name, schema_version) {
            return;
        }
        self.observation_count += 1;
        if DESIGN_EVENT_SCHEMAS.contains(&schema_name) {
            self.design_event_count += 1;
        }
    }
}

/// Returns whether a schema identity is canonical in the observation model.
pub(crate) fn is_canonical_schema(schema_name: &str, schema_version: u32) -> bool {
    CANONICAL_SCHEMA_NAMES.contains(&schema_name)
        && if schema_name == "OBS-WINDOW-FOCUS-GAINED" {
            matches!(schema_version, 1..=3)
        } else {
            schema_version == 1
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_floor_is_reported_as_below() {
        let metrics = ObservationMetrics::from_counts(1_000, 4);

        assert_eq!(metrics.rate_per_1000(), 4.0);
        assert!(!metrics.meets_design_event_floor());
    }

    #[test]
    fn above_floor_is_reported_as_passing() {
        let metrics = ObservationMetrics::from_counts(1_000, 6);

        assert_eq!(metrics.rate_per_1000(), 6.0);
        assert!(metrics.meets_design_event_floor());
    }

    #[test]
    fn the_floor_boundary_is_inclusive_and_empty_samples_do_not_pass() {
        assert!(ObservationMetrics::from_counts(1_000, 5).meets_design_event_floor());
        assert!(!ObservationMetrics::from_counts(0, 0).meets_design_event_floor());
    }

    #[test]
    fn only_the_seven_user_schemas_are_design_events() {
        let mut schema_names = DESIGN_EVENT_SCHEMAS.to_vec();
        schema_names.extend([
            "OBS-SURFACE-NAMED",
            "OBS-WINDOW-FOCUS-GAINED",
            "OBS-NOT-CANONICAL",
        ]);

        let metrics = ObservationMetrics::from_schema_names(schema_names);

        assert_eq!(metrics.observation_count, 9);
        assert_eq!(metrics.design_event_count, 7);
    }
}
