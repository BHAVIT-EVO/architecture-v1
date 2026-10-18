//! Optional, directly witnessed identity of one application surface.
//!
//! Registry window ids are capture-session provenance, not native window
//! handles. Only a witnessed native window identifier may bind an action to a
//! window. Unknown browser profile/tab identifiers must remain absent.

use std::collections::HashMap;

use crate::window_registry::WindowId;

pub const CAPTURE_SESSION_KEY: &str = "observed_state.surface.capture_session";
pub const WINDOW_ID_KEY: &str = "observed_state.surface.window_id";
pub const PROCESS_ID_KEY: &str = "observed_state.surface.process_id";
pub const PROCESS_STARTED_AT_KEY: &str = "observed_state.surface.process_started_at";
pub const WINDOW_IDENTIFIER_KEY: &str = "observed_state.surface.window_identifier";
pub const BROWSER_WINDOW_KEY: &str = "observed_state.surface.browser_window";
pub const BROWSER_PROFILE_KEY: &str = "observed_state.surface.browser_profile";
pub const BROWSER_SESSION_KEY: &str = "observed_state.surface.browser_session";
pub const BROWSER_TAB_KEY: &str = "observed_state.surface.browser_tab";

/// One browser identity witness supplied by an explicitly installed local
/// adapter.
///
/// Every field is optional because an adapter may be unable to read one part
/// of a live surface. Capture treats such a reading as one incomplete
/// observation: it never fills a missing token from a pid, title, URL, window
/// id, or an earlier reading. The capture-session namespace remains the
/// surrounding [`SurfaceIdentity`] field rather than a browser-derived value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrowserIdentityObservation {
    pub process_id: Option<i32>,
    pub process_started_at: Option<String>,
    pub window_identifier: Option<String>,
    /// Browser-native window token, kept separate from the AX window token.
    pub browser_window: Option<String>,
    pub browser_profile: Option<String>,
    /// Browser context/session token, when the browser exposes one.
    pub browser_session: Option<String>,
    pub browser_tab: Option<String>,
}

impl BrowserIdentityObservation {
    /// Whether this observation contains the complete opaque evidence needed
    /// to attach browser constraints to a native surface.
    pub fn is_complete(&self) -> bool {
        self.process_id.is_some()
            && self.process_started_at.is_some()
            && self.window_identifier.is_some()
            && self.browser_window.is_some()
            && self.browser_profile.is_some()
            && self.browser_session.is_some()
            && self.browser_tab.is_some()
    }
}

const MAX_IDENTITY_FIELD: usize = 4096;
const CONTEXT_KEYS: [&str; 9] = [
    CAPTURE_SESSION_KEY,
    WINDOW_ID_KEY,
    PROCESS_ID_KEY,
    PROCESS_STARTED_AT_KEY,
    WINDOW_IDENTIFIER_KEY,
    BROWSER_WINDOW_KEY,
    BROWSER_PROFILE_KEY,
    BROWSER_SESSION_KEY,
    BROWSER_TAB_KEY,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceIdentity {
    pub capture_session: String,
    pub window_id: String,
    pub process_id: i32,
    /// Sixteen lowercase hex digits of the finite, positive Unix-epoch
    /// `NSDate::timeIntervalSince1970` f64 bits witnessed at process launch.
    pub process_started_at: String,
    /// The focused AX window's AXIdentifier, never its title or control id.
    pub window_identifier: Option<String>,
    /// Browser-native window identity, never substituted for `window_identifier`.
    pub browser_window: Option<String>,
    pub browser_profile: Option<String>,
    /// Browser-native session/context identity.
    pub browser_session: Option<String>,
    pub browser_tab: Option<String>,
}

impl SurfaceIdentity {
    /// Validates the whole identity without normalizing or dropping constraints.
    pub fn is_valid(&self) -> bool {
        bounded_nonempty(&self.capture_session)
            && bounded_nonempty(&self.window_id)
            && self
                .window_id
                .parse::<WindowId>()
                .is_ok_and(|id| id != WindowId::NONE && id.to_string() == self.window_id)
            && self.process_id > 0
            && valid_process_start(&self.process_started_at)
            && self
                .window_identifier
                .as_deref()
                .is_none_or(bounded_nonempty)
            && self.browser_window.as_deref().is_none_or(bounded_nonempty)
            && self.browser_profile.as_deref().is_none_or(bounded_nonempty)
            && self.browser_session.as_deref().is_none_or(bounded_nonempty)
            && self.browser_tab.as_deref().is_none_or(bounded_nonempty)
            && (self.browser_tab.is_none()
                || (self.browser_profile.is_some()
                    && self.browser_session.is_some()
                    && self.browser_window.is_some()))
    }

    /// Attaches browser tokens only when a provider directly witnessed the
    /// complete browser/native identity for this same surface.
    ///
    /// A failed, partial, malformed, or contradictory witness returns the
    /// native identity with both browser constraints absent. Native capture
    /// evidence is therefore retained without weakening an invalid browser
    /// witness into a guessed one.
    pub fn with_browser_identity_observation(
        &self,
        observation: &BrowserIdentityObservation,
    ) -> Self {
        let mut native = self.without_browser_identity();
        if !self.is_valid() || !observation.is_complete() {
            return native;
        }

        let Some(process_id) = observation.process_id else {
            return native;
        };
        let Some(process_started_at) = observation.process_started_at.as_deref() else {
            return native;
        };
        let Some(window_identifier) = observation.window_identifier.as_deref() else {
            return native;
        };
        let Some(browser_window) = observation.browser_window.as_deref() else {
            return native;
        };
        let Some(browser_profile) = observation.browser_profile.as_deref() else {
            return native;
        };
        let Some(browser_session) = observation.browser_session.as_deref() else {
            return native;
        };
        let Some(browser_tab) = observation.browser_tab.as_deref() else {
            return native;
        };

        if process_id != self.process_id
            || process_started_at != self.process_started_at
            || self.window_identifier.as_deref() != Some(window_identifier)
            || !bounded_nonempty(browser_window)
            || !bounded_nonempty(browser_profile)
            || !bounded_nonempty(browser_session)
            || !bounded_nonempty(browser_tab)
        {
            return native;
        }

        native.browser_profile = Some(browser_profile.to_string());
        native.browser_window = Some(browser_window.to_string());
        native.browser_session = Some(browser_session.to_string());
        native.browser_tab = Some(browser_tab.to_string());
        if native.is_valid() {
            native
        } else {
            // Keep the atomically stripped native identity if a future field
            // validation rule makes the assembled browser identity invalid.
            self.without_browser_identity()
        }
    }

    /// Removes browser constraints while preserving the native capture
    /// identity and its process launch evidence.
    pub fn without_browser_identity(&self) -> Self {
        let mut native = self.clone();
        native.browser_profile = None;
        native.browser_window = None;
        native.browser_session = None;
        native.browser_tab = None;
        native
    }

    /// Writes a complete identity, replacing any earlier surface context.
    /// Invalid identities are omitted as a whole, including optional fields.
    pub fn write_context(&self, context: &mut HashMap<String, String>) {
        Self::clear_context(context);
        if !self.is_valid() {
            return;
        }
        context.insert(CAPTURE_SESSION_KEY.into(), self.capture_session.clone());
        context.insert(WINDOW_ID_KEY.into(), self.window_id.clone());
        context.insert(PROCESS_ID_KEY.into(), self.process_id.to_string());
        context.insert(
            PROCESS_STARTED_AT_KEY.into(),
            self.process_started_at.clone(),
        );
        if let Some(value) = &self.window_identifier {
            context.insert(WINDOW_IDENTIFIER_KEY.into(), value.clone());
        }
        if let Some(value) = &self.browser_window {
            context.insert(BROWSER_WINDOW_KEY.into(), value.clone());
        }
        if let Some(value) = &self.browser_profile {
            context.insert(BROWSER_PROFILE_KEY.into(), value.clone());
        }
        if let Some(value) = &self.browser_session {
            context.insert(BROWSER_SESSION_KEY.into(), value.clone());
        }
        if let Some(value) = &self.browser_tab {
            context.insert(BROWSER_TAB_KEY.into(), value.clone());
        }
    }

    /// Reads recognized context keys atomically. A partial or malformed
    /// identity never becomes a less-constrained identity.
    pub fn from_context(context: &HashMap<String, String>) -> Option<Self> {
        let identity = Self {
            capture_session: context.get(CAPTURE_SESSION_KEY)?.clone(),
            window_id: context.get(WINDOW_ID_KEY)?.clone(),
            process_id: context.get(PROCESS_ID_KEY)?.parse().ok()?,
            process_started_at: context.get(PROCESS_STARTED_AT_KEY)?.clone(),
            window_identifier: context.get(WINDOW_IDENTIFIER_KEY).cloned(),
            browser_window: context.get(BROWSER_WINDOW_KEY).cloned(),
            browser_profile: context.get(BROWSER_PROFILE_KEY).cloned(),
            browser_session: context.get(BROWSER_SESSION_KEY).cloned(),
            browser_tab: context.get(BROWSER_TAB_KEY).cloned(),
        };
        identity.is_valid().then_some(identity)
    }

    pub(crate) fn clear_context(context: &mut HashMap<String, String>) {
        for key in CONTEXT_KEYS {
            context.remove(key);
        }
    }
}

fn bounded_nonempty(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_IDENTITY_FIELD && !value.contains('\0')
}

fn valid_process_start(value: &str) -> bool {
    if value.len() != 16
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return false;
    }
    let Ok(bits) = u64::from_str_radix(value, 16) else {
        return false;
    };
    let epoch = f64::from_bits(bits);
    epoch.is_finite() && epoch > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> SurfaceIdentity {
        SurfaceIdentity {
            capture_session: "capture-42-1791000000000000000".into(),
            window_id: "window-1".into(),
            process_id: 123,
            process_started_at: format!("{:016x}", 1_791_000_000.5f64.to_bits()),
            window_identifier: Some("native-window-1".into()),
            browser_window: Some("browser-window-a".into()),
            browser_profile: Some("profile-a".into()),
            browser_session: Some("session-a".into()),
            browser_tab: Some("tab-9".into()),
        }
    }

    #[test]
    fn complete_identity_round_trips_without_changing_tokens() {
        let original = identity();
        let mut context = HashMap::new();
        original.write_context(&mut context);
        assert_eq!(SurfaceIdentity::from_context(&context), Some(original));
        assert_eq!(context.len(), 9);
    }

    #[test]
    fn canonical_positive_window_id_is_required() {
        for invalid in [
            "window-0",
            "window-01",
            "window-+1",
            "1",
            "window--1",
            "window-18446744073709551616",
        ] {
            let mut value = identity();
            value.window_id = invalid.into();
            assert!(!value.is_valid(), "{invalid}");
        }
        let mut value = identity();
        value.window_id = format!("window-{}", u64::MAX);
        assert!(value.is_valid());
    }

    #[test]
    fn process_start_requires_positive_finite_epoch_bits_in_lowercase_hex() {
        let mut value = identity();
        for epoch in [
            0.0f64,
            -0.0,
            -1.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            value.process_started_at = format!("{:016x}", epoch.to_bits());
            assert!(!value.is_valid(), "{epoch}");
        }
        for invalid in [
            "1791000000",
            "41DAB052E0000000",
            "41dab052e000000",
            "41dab052e00000000",
            "41dab052e000000g",
        ] {
            value.process_started_at = invalid.into();
            assert!(!value.is_valid(), "{invalid}");
        }
    }

    #[test]
    fn required_and_present_optional_tokens_are_bounded_and_nonempty() {
        for invalid in [
            String::new(),
            " \n ".into(),
            "x".repeat(MAX_IDENTITY_FIELD + 1),
            "nul\0token".into(),
        ] {
            for field in 0..4 {
                let mut value = identity();
                match field {
                    0 => value.capture_session = invalid.clone(),
                    1 => value.window_identifier = Some(invalid.clone()),
                    2 => value.browser_profile = Some(invalid.clone()),
                    _ => value.browser_tab = Some(invalid.clone()),
                }
                assert!(!value.is_valid());
            }
        }
        for invalid_pid in [0, -1] {
            let mut value = identity();
            value.process_id = invalid_pid;
            assert!(!value.is_valid());
        }
        let mut value = identity();
        value.capture_session = "x".repeat(MAX_IDENTITY_FIELD);
        assert!(value.is_valid());
    }

    #[test]
    fn unknown_optional_tokens_remain_absent_and_tab_requires_profile() {
        let mut value = identity();
        value.browser_profile = None;
        assert!(!value.is_valid());
        value.browser_tab = None;
        value.window_identifier = None;
        assert!(value.is_valid());
        let mut context = HashMap::new();
        value.write_context(&mut context);
        assert_eq!(SurfaceIdentity::from_context(&context), Some(value));
        assert!(!context.contains_key(BROWSER_PROFILE_KEY));
        assert!(!context.contains_key(BROWSER_TAB_KEY));
        assert!(!context.contains_key(WINDOW_IDENTIFIER_KEY));
    }

    #[test]
    fn partial_or_corrupt_optional_context_rejects_the_entire_identity() {
        let original = identity();
        let mut context = HashMap::new();
        original.write_context(&mut context);
        for required in [
            CAPTURE_SESSION_KEY,
            WINDOW_ID_KEY,
            PROCESS_ID_KEY,
            PROCESS_STARTED_AT_KEY,
        ] {
            let mut partial = context.clone();
            partial.remove(required);
            assert_eq!(SurfaceIdentity::from_context(&partial), None);
        }
        for optional in [WINDOW_IDENTIFIER_KEY, BROWSER_PROFILE_KEY, BROWSER_TAB_KEY] {
            let mut corrupt = context.clone();
            corrupt.insert(optional.into(), "".into());
            assert_eq!(SurfaceIdentity::from_context(&corrupt), None);
        }
        context.remove(BROWSER_PROFILE_KEY);
        assert_eq!(SurfaceIdentity::from_context(&context), None);
    }

    #[test]
    fn context_replacement_cannot_inherit_old_optional_constraints() {
        let mut context = HashMap::new();
        identity().write_context(&mut context);
        let mut replacement = identity();
        replacement.browser_profile = None;
        replacement.browser_tab = None;
        replacement.window_identifier = None;
        replacement.write_context(&mut context);
        assert_eq!(
            SurfaceIdentity::from_context(&context),
            Some(replacement.clone())
        );
        replacement.process_id = 0;
        replacement.write_context(&mut context);
        assert!(context.is_empty());
    }

    #[test]
    fn complete_direct_browser_witness_preserves_native_and_capture_identity() {
        let original = identity();
        let observed = BrowserIdentityObservation {
            process_id: Some(original.process_id),
            process_started_at: Some(original.process_started_at.clone()),
            window_identifier: original.window_identifier.clone(),
            browser_window: Some("browser-window-direct".into()),
            browser_profile: Some("profile-direct".into()),
            browser_session: Some("session-direct".into()),
            browser_tab: Some("tab-direct".into()),
        };

        let enriched = original
            .without_browser_identity()
            .with_browser_identity_observation(&observed);
        assert_eq!(enriched.capture_session, original.capture_session);
        assert_eq!(enriched.window_id, original.window_id);
        assert_eq!(enriched.process_id, original.process_id);
        assert_eq!(enriched.process_started_at, original.process_started_at);
        assert_eq!(enriched.window_identifier, original.window_identifier);
        assert_eq!(enriched.browser_profile.as_deref(), Some("profile-direct"));
        assert_eq!(enriched.browser_tab.as_deref(), Some("tab-direct"));
        assert!(enriched.is_valid());
    }

    #[test]
    fn partial_or_contradictory_browser_witness_is_removed_atomically() {
        let original = identity();
        let native = original.without_browser_identity();
        let cases = [
            BrowserIdentityObservation {
                browser_profile: Some("profile-direct".into()),
                browser_tab: Some("tab-direct".into()),
                ..BrowserIdentityObservation::default()
            },
            BrowserIdentityObservation {
                process_id: Some(original.process_id + 1),
                process_started_at: Some(original.process_started_at.clone()),
                window_identifier: original.window_identifier.clone(),
                browser_window: Some("browser-window-direct".into()),
                browser_profile: Some("profile-direct".into()),
                browser_session: Some("session-direct".into()),
                browser_tab: Some("tab-direct".into()),
            },
            BrowserIdentityObservation {
                process_id: Some(original.process_id),
                process_started_at: Some(original.process_started_at.clone()),
                window_identifier: Some("other-native-window".into()),
                browser_window: Some("browser-window-direct".into()),
                browser_profile: Some("profile-direct".into()),
                browser_session: Some("session-direct".into()),
                browser_tab: Some("tab-direct".into()),
            },
        ];

        for observation in cases {
            let enriched = native.with_browser_identity_observation(&observation);
            assert_eq!(enriched, native);
            assert_eq!(enriched.browser_profile, None);
            assert_eq!(enriched.browser_tab, None);
            assert_eq!(enriched.process_started_at, original.process_started_at);
            assert_eq!(enriched.window_identifier, original.window_identifier);
        }
    }
}
