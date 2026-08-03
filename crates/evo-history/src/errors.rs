//! Domain error types for the evo-history crate.
//!
//! IS-0016 §15 defines that no construction failures exist for this crate.
//! `HistoricalUnderstanding` construction is infallible.
//!
//! Accordingly, `HistoricalError` is an uninhabited type. It exists only to
//! satisfy the public computational surface defined by IS-0016 §12 and to
//! preserve the option for future specifications to introduce fallible
//! construction without breaking the public API.

/// The error type for the evo-history crate.
///
/// Uninhabited. No construction failures exist (IS-0016 §15).
///
/// Future specifications MAY add variants without breaking existing callers.
#[derive(Debug, Clone, PartialEq)]
pub enum HistoricalError {}

impl std::fmt::Display for HistoricalError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl std::error::Error for HistoricalError {}
