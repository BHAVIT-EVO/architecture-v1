//! Optional interface state directly witnessed with an Observation.
//!
//! This is deliberately small and application-agnostic. Every field maps to a
//! generic operating-system accessibility attribute. Missing fields mean the
//! state was not observable; callers must never infer or fabricate them.

use std::collections::HashMap;

pub const DOCUMENT_LOCATOR_KEY: &str = "observed_state.document_locator";
pub const WINDOW_TITLE_KEY: &str = "observed_state.window_title";
pub const FOCUSED_ROLE_KEY: &str = "observed_state.focused_role";
pub const FOCUSED_IDENTIFIER_KEY: &str = "observed_state.focused_identifier";
pub const SELECTION_START_KEY: &str = "observed_state.selection_start";
pub const SELECTION_LENGTH_KEY: &str = "observed_state.selection_length";
pub const INSERTION_LINE_KEY: &str = "observed_state.insertion_line";

const MAX_TEXT_FIELD: usize = 4096;

/// The minimum generic state useful for returning inside an artifact.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObservedState {
    document_locator: Option<String>,
    window_title: Option<String>,
    focused_role: Option<String>,
    focused_identifier: Option<String>,
    selection: Option<(u64, u64)>,
    insertion_line: Option<u64>,
}

impl ObservedState {
    pub fn new() -> Self { Self::default() }

    pub fn with_document_locator(mut self, value: impl Into<String>) -> Self {
        self.document_locator = bounded(value.into()); self
    }

    pub fn with_window_title(mut self, value: impl Into<String>) -> Self {
        self.window_title = bounded(value.into()); self
    }

    pub fn with_focused_role(mut self, value: impl Into<String>) -> Self {
        self.focused_role = bounded(value.into()); self
    }

    pub fn with_focused_identifier(mut self, value: impl Into<String>) -> Self {
        self.focused_identifier = bounded(value.into()); self
    }

    pub fn with_selection(mut self, start: u64, length: u64) -> Self {
        self.selection = Some((start, length)); self
    }

    pub fn with_insertion_line(mut self, line: u64) -> Self {
        self.insertion_line = Some(line); self
    }

    pub fn document_locator(&self) -> Option<&str> { self.document_locator.as_deref() }
    pub fn window_title(&self) -> Option<&str> { self.window_title.as_deref() }
    pub fn focused_role(&self) -> Option<&str> { self.focused_role.as_deref() }
    pub fn focused_identifier(&self) -> Option<&str> { self.focused_identifier.as_deref() }
    pub fn selection(&self) -> Option<(u64, u64)> { self.selection }
    pub fn insertion_line(&self) -> Option<u64> { self.insertion_line }
    pub fn is_empty(&self) -> bool {
        self.document_locator.is_none() && self.window_title.is_none()
            && self.focused_role.is_none() && self.focused_identifier.is_none()
            && self.selection.is_none() && self.insertion_line.is_none()
    }

    /// Adds the state to an Observation provenance context.
    pub fn write_context(&self, context: &mut HashMap<String, String>) {
        if let Some(value) = &self.document_locator { context.insert(DOCUMENT_LOCATOR_KEY.into(), value.clone()); }
        if let Some(value) = &self.window_title { context.insert(WINDOW_TITLE_KEY.into(), value.clone()); }
        if let Some(value) = &self.focused_role { context.insert(FOCUSED_ROLE_KEY.into(), value.clone()); }
        if let Some(value) = &self.focused_identifier { context.insert(FOCUSED_IDENTIFIER_KEY.into(), value.clone()); }
        if let Some((start, length)) = self.selection {
            context.insert(SELECTION_START_KEY.into(), start.to_string());
            context.insert(SELECTION_LENGTH_KEY.into(), length.to_string());
        }
        if let Some(line) = self.insertion_line { context.insert(INSERTION_LINE_KEY.into(), line.to_string()); }
    }

    /// Reads only recognized state keys. Invalid or partial values are ignored.
    pub fn from_context(context: &HashMap<String, String>) -> Option<Self> {
        let selection = match (parse_u64(context, SELECTION_START_KEY), parse_u64(context, SELECTION_LENGTH_KEY)) {
            (Some(start), Some(length)) => Some((start, length)),
            _ => None,
        };
        let state = Self {
            document_locator: context.get(DOCUMENT_LOCATOR_KEY).cloned().and_then(bounded),
            window_title: context.get(WINDOW_TITLE_KEY).cloned().and_then(bounded),
            focused_role: context.get(FOCUSED_ROLE_KEY).cloned().and_then(bounded),
            focused_identifier: context.get(FOCUSED_IDENTIFIER_KEY).cloned().and_then(bounded),
            selection,
            insertion_line: parse_u64(context, INSERTION_LINE_KEY),
        };
        (!state.is_empty()).then_some(state)
    }
}

fn bounded(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.len() <= MAX_TEXT_FIELD).then(|| value.to_string())
}

fn parse_u64(context: &HashMap<String, String>, key: &str) -> Option<u64> {
    context.get(key)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trips_through_canonical_context() {
        let state = ObservedState::new()
            .with_document_locator("/work/automation-x.ts")
            .with_focused_role("AXTextArea")
            .with_focused_identifier("editor")
            .with_selection(418, 12)
            .with_insertion_line(37);
        let mut context = HashMap::new();
        state.write_context(&mut context);
        assert_eq!(ObservedState::from_context(&context), Some(state));
    }

    #[test]
    fn missing_or_malformed_state_is_not_fabricated() {
        let mut context = HashMap::new();
        context.insert(SELECTION_START_KEY.into(), "12".into());
        context.insert(INSERTION_LINE_KEY.into(), "not-a-number".into());
        assert_eq!(ObservedState::from_context(&context), None);
    }
}
