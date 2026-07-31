use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

/// Immutable identifier for an Observation.
///
/// Observation IDs have no semantic meaning. They exist solely
/// to uniquely identify an Observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationId(Uuid);

impl ObservationId {
    /// Generates a new globally unique ObservationId.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Returns the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ObservationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ObservationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Uuid> for ObservationId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<ObservationId> for Uuid {
    fn from(value: ObservationId) -> Self {
        value.0
    }
}

impl FromStr for ObservationId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}
