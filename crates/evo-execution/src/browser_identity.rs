//! Portable, opt-in execution of directly witnessed browser surface identity.
//!
//! A URL is a constraint on an identified tab, never a way to find or recreate
//! one. This seam has no URL opener, browser scripting, or platform dependency.
//! Production support is deliberately [`BrowserSurfaceCapability::Unsupported`]
//! until an adapter can honor every required token and inspect global focus.
//! Live handles and evidence are execution-time state, never canonical history.

use crate::locator::{LocatorKind, classify_subject};
use crate::restore::{DispatchResult, TargetReceipt, VerificationResult};
use evo_observation::surface_identity::SurfaceIdentity;

const MAX_TOKEN_LENGTH: usize = 4096;
const UNSUPPORTED: &str =
    "browser profile/tab identity adapter unavailable; exact constraints cannot be honored";

/// Support must be declared explicitly, not inferred from readable URLs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BrowserSurfaceCapability {
    #[default]
    Unsupported,
    /// The adapter retains exact tab handles, witnesses their process/window/
    /// profile/tab tokens and URL, and can inspect the globally focused tab.
    ExactIdentity,
}

/// Directly read evidence from one retained tab, not a title or URL search.
/// Unknown attributes stay absent. Profile and tab tokens are opaque and are
/// compared byte-for-byte; the capture registry's `window_id` is not a handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserTabEvidence {
    pub process_id: i32,
    pub window_identifier: Option<String>,
    pub browser_profile: Option<String>,
    pub browser_tab: Option<String>,
    pub url: Option<String>,
}

/// An adapter must opt in to exact identity. Default methods fail closed.
///
/// `Tab` must retain the specific live tab through inspection and dispatch.
/// Tokens must not be reused within a process lifetime; stale handles must
/// refuse rather than bind to a replacement. No method may select by URL,
/// title, index, recency, or reconstruct an absent tab. If a provider cannot
/// satisfy that contract, its capability must remain `Unsupported`.
pub trait BrowserSurfaceAdapter {
    type Tab;

    fn capability(&self) -> BrowserSurfaceCapability {
        BrowserSurfaceCapability::Unsupported
    }

    /// Read the same opaque process-start token as the witnessed identity.
    fn process_started_at(&self, _pid: i32) -> Result<String, String> {
        Err(UNSUPPORTED.into())
    }

    /// Complete enumeration of retained tabs owned by this process. An
    /// incomplete or inaccessible enumeration is an error, not an empty list.
    fn tabs(&self, _pid: i32) -> Result<Vec<Self::Tab>, String> {
        Err(UNSUPPORTED.into())
    }

    fn evidence(&self, _tab: &Self::Tab) -> Result<BrowserTabEvidence, String> {
        Err(UNSUPPORTED.into())
    }

    /// Focus this retained tab and its owning window/application. This must
    /// check handle lifetime and must never open or navigate a URL instead.
    fn focus_tab(&self, _tab: &Self::Tab) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }

    /// Retain the globally focused tab, checking the focused application's
    /// PID. Browser-local active-tab state alone is not a focus witness.
    fn focused_tab(&self, _pid: i32) -> Result<Self::Tab, String> {
        Err(UNSUPPORTED.into())
    }
}

/// The production default: no browser support is claimed or attempted.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnsupportedBrowserSurfaceAdapter;

impl BrowserSurfaceAdapter for UnsupportedBrowserSurfaceAdapter {
    type Tab = ();
}

/// Conservative production entry point, with no browser side effects.
pub fn raise_browser_surface(surface: &SurfaceIdentity, url: &str) -> TargetReceipt {
    raise_browser_surface_with_adapter(surface, url, &UnsupportedBrowserSurfaceAdapter)
}

/// Inspect, focus, and independently verify one exact witnessed browser tab.
///
/// Complete profile/tab/native-window evidence is required. Missing or
/// duplicate profile/tab tokens refuse even if a URL or window would appear
/// to disambiguate them. Inspection failure, restart, or URL mismatch never
/// widens into window-only focus, navigation, or URL reopening. A successful
/// dispatch remains distinct from a verified process/window/profile/tab/URL
/// postcondition, just as it does for native identity restores.
pub fn raise_browser_surface_with_adapter<A: BrowserSurfaceAdapter + ?Sized>(
    surface: &SurfaceIdentity,
    url: &str,
    adapter: &A,
) -> TargetReceipt {
    let prepared = (|| {
        if adapter.capability() != BrowserSurfaceCapability::ExactIdentity {
            return Err(UNSUPPORTED.into());
        }
        let expected = RequiredIdentity::new(surface, url)?;
        check_process(adapter, surface)?;
        let tabs = adapter.tabs(surface.process_id)?;
        let mut selected = None;
        let mut matches = 0;
        for (index, tab) in tabs.iter().enumerate() {
            let evidence = adapter.evidence(tab)?;
            // Missing tokens on another candidate can conceal a duplicate.
            // URL/window evidence is deliberately not used to select a tab.
            let profile = required_token(evidence.browser_profile.as_deref(), "live profile")?;
            let token = required_token(evidence.browser_tab.as_deref(), "live tab")?;
            if evidence.process_id != surface.process_id {
                return Err("browser tab enumeration has an unexpected process owner".into());
            }
            if profile == expected.profile && token == expected.tab {
                matches += 1;
                selected = Some(index);
            }
        }
        let index = match (matches, selected) {
            (1, Some(index)) => index,
            (0, _) => return Err("no live tab has the witnessed profile/tab tokens".into()),
            _ => {
                return Err(format!(
                    "{matches} live tabs have the witnessed profile/tab tokens; exact identity is ambiguous"
                ));
            }
        };
        // Re-read this same handle immediately before dispatch. Never search
        // for another tab after the selected identity/resource has changed.
        check_evidence(&adapter.evidence(&tabs[index])?, surface, &expected)?;
        check_process(adapter, surface)?;
        Ok((tabs, index, expected))
    })();
    let (tabs, index, expected) = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => return refused_receipt(url, reason),
    };

    if let Err(reason) = adapter.focus_tab(&tabs[index]) {
        return TargetReceipt {
            resource: url.into(),
            dispatch: DispatchResult::Failed { reason },
            verification: VerificationResult::Unverified {
                reason: "browser focus dispatch was refused; no fallback was attempted".into(),
            },
        };
    }

    let postcondition = (|| {
        check_process(adapter, surface)?;
        let focused = adapter.focused_tab(surface.process_id)?;
        check_evidence(&adapter.evidence(&focused)?, surface, &expected)?;
        check_process(adapter, surface)
    })();
    TargetReceipt {
        resource: url.into(),
        dispatch: DispatchResult::RaiseAccepted,
        verification: match postcondition {
            Ok(()) => VerificationResult::Verified,
            Err(reason) => VerificationResult::Failed { reason },
        },
    }
}

struct RequiredIdentity<'a> {
    profile: &'a str,
    tab: &'a str,
    window: &'a str,
    url: &'a str,
}

impl<'a> RequiredIdentity<'a> {
    fn new(surface: &'a SurfaceIdentity, url: &'a str) -> Result<Self, String> {
        if !surface.is_valid() {
            return Err("invalid witnessed browser surface identity".into());
        }
        if url.trim().is_empty()
            || url.contains('\0')
            || !matches!(classify_subject(url), LocatorKind::Url(_))
        {
            return Err("browser identity restoration requires the witnessed exact URL".into());
        }
        Ok(Self {
            profile: required_token(surface.browser_profile.as_deref(), "witnessed profile")?,
            tab: required_token(surface.browser_tab.as_deref(), "witnessed tab")?,
            window: required_token(
                surface.window_identifier.as_deref(),
                "witnessed native window",
            )?,
            url,
        })
    }
}

fn required_token<'a>(value: Option<&'a str>, name: &str) -> Result<&'a str, String> {
    value
        .filter(|token| {
            !token.trim().is_empty() && token.len() <= MAX_TOKEN_LENGTH && !token.contains('\0')
        })
        .ok_or_else(|| format!("{name} identity token is missing or unreadable"))
}

fn check_process<A: BrowserSurfaceAdapter + ?Sized>(
    adapter: &A,
    surface: &SurfaceIdentity,
) -> Result<(), String> {
    if adapter.process_started_at(surface.process_id)? != surface.process_started_at {
        return Err(
            "browser process lifetime changed; witnessed PID was reused or restarted".into(),
        );
    }
    Ok(())
}

fn check_evidence(
    evidence: &BrowserTabEvidence,
    surface: &SurfaceIdentity,
    expected: &RequiredIdentity<'_>,
) -> Result<(), String> {
    if evidence.process_id != surface.process_id {
        return Err("browser tab process differs from the witnessed process".into());
    }
    let profile = required_token(evidence.browser_profile.as_deref(), "live profile")?;
    let tab = required_token(evidence.browser_tab.as_deref(), "live tab")?;
    let window = required_token(evidence.window_identifier.as_deref(), "live native window")?;
    if profile != expected.profile || tab != expected.tab || window != expected.window {
        return Err("browser profile/tab/window differs from the witnessed exact identity".into());
    }
    if evidence.url.as_deref() != Some(expected.url) {
        return Err("identified browser tab does not show the witnessed exact URL".into());
    }
    Ok(())
}

fn refused_receipt(url: &str, reason: String) -> TargetReceipt {
    TargetReceipt {
        resource: url.into(),
        dispatch: DispatchResult::NotDispatched { reason },
        verification: VerificationResult::Unverified {
            reason: "no browser identity action was dispatched".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    const URL: &str = "https://example.test/work?exact=1#section";

    fn surface() -> SurfaceIdentity {
        SurfaceIdentity {
            capture_session: "capture-browser-fixture".into(),
            window_id: "window-1".into(),
            process_id: 17,
            process_started_at: format!("{:016x}", 1_791_000_000.5f64.to_bits()),
            window_identifier: Some("native-window-a".into()),
            browser_window: Some("browser-window-a".into()),
            browser_profile: Some("profile-a".into()),
            browser_session: Some("session-a".into()),
            browser_tab: Some("tab-a".into()),
        }
    }

    fn evidence() -> BrowserTabEvidence {
        let surface = surface();
        BrowserTabEvidence {
            process_id: surface.process_id,
            window_identifier: surface.window_identifier,
            browser_profile: surface.browser_profile,
            browser_tab: surface.browser_tab,
            url: Some(URL.into()),
        }
    }

    /// Retained fake handles are indices, never resolved by URL. There is
    /// deliberately no opener or navigator in the seam or fake effect log.
    struct FakeAdapter {
        capability: BrowserSurfaceCapability,
        candidates: Vec<BrowserTabEvidence>,
        selected_reread: Option<BrowserTabEvidence>,
        focused: Result<BrowserTabEvidence, String>,
        process_reads: Cell<usize>,
        evidence_reads: Cell<usize>,
        restart_on_read: Option<usize>,
        enumeration_error: Option<String>,
        focus_error: Option<String>,
        effects: RefCell<Vec<usize>>,
    }

    impl Default for FakeAdapter {
        fn default() -> Self {
            Self {
                capability: BrowserSurfaceCapability::ExactIdentity,
                candidates: vec![evidence()],
                selected_reread: None,
                focused: Ok(evidence()),
                process_reads: Cell::new(0),
                evidence_reads: Cell::new(0),
                restart_on_read: None,
                enumeration_error: None,
                focus_error: None,
                effects: RefCell::new(vec![]),
            }
        }
    }

    impl BrowserSurfaceAdapter for FakeAdapter {
        type Tab = usize;

        fn capability(&self) -> BrowserSurfaceCapability {
            self.capability
        }

        fn process_started_at(&self, pid: i32) -> Result<String, String> {
            assert_eq!(pid, surface().process_id);
            let read = self.process_reads.get() + 1;
            self.process_reads.set(read);
            if self.restart_on_read.is_some_and(|restart| read >= restart) {
                Ok(format!("{:016x}", 1_791_000_001.0f64.to_bits()))
            } else {
                Ok(surface().process_started_at)
            }
        }

        fn tabs(&self, pid: i32) -> Result<Vec<usize>, String> {
            assert_eq!(pid, surface().process_id);
            match &self.enumeration_error {
                Some(reason) => Err(reason.clone()),
                None => Ok((0..self.candidates.len()).collect()),
            }
        }

        fn evidence(&self, tab: &usize) -> Result<BrowserTabEvidence, String> {
            let read = self.evidence_reads.get();
            self.evidence_reads.set(read + 1);
            if *tab == usize::MAX {
                return self.focused.clone();
            }
            if read >= self.candidates.len() {
                if let Some(changed) = &self.selected_reread {
                    return Ok(changed.clone());
                }
            }
            Ok(self.candidates[*tab].clone())
        }

        fn focus_tab(&self, tab: &usize) -> Result<(), String> {
            self.effects.borrow_mut().push(*tab);
            match &self.focus_error {
                Some(reason) => Err(reason.clone()),
                None => Ok(()),
            }
        }

        fn focused_tab(&self, pid: i32) -> Result<usize, String> {
            assert_eq!(pid, surface().process_id);
            self.focused
                .as_ref()
                .map(|_| usize::MAX)
                .map_err(Clone::clone)
        }
    }

    fn attempt(adapter: &FakeAdapter) -> TargetReceipt {
        raise_browser_surface_with_adapter(&surface(), URL, adapter)
    }

    fn assert_refused(receipt: &TargetReceipt, adapter: &FakeAdapter) {
        assert_eq!(receipt.resource, URL);
        assert!(matches!(
            receipt.dispatch,
            DispatchResult::NotDispatched { .. }
        ));
        assert!(matches!(
            receipt.verification,
            VerificationResult::Unverified { .. }
        ));
        assert!(
            adapter.effects.borrow().is_empty(),
            "must not focus, open, or navigate a fallback"
        );
    }

    #[test]
    fn unsupported_is_the_production_and_trait_default() {
        struct InertAdapter;
        impl BrowserSurfaceAdapter for InertAdapter {
            type Tab = ();
        }
        assert_eq!(
            BrowserSurfaceCapability::default(),
            BrowserSurfaceCapability::Unsupported
        );
        assert_eq!(
            InertAdapter.capability(),
            BrowserSurfaceCapability::Unsupported
        );
        let default_receipt = raise_browser_surface(&surface(), URL);
        assert_eq!(
            default_receipt,
            raise_browser_surface_with_adapter(&surface(), URL, &InertAdapter)
        );
        assert!(matches!(
            default_receipt.dispatch,
            DispatchResult::NotDispatched { .. }
        ));
    }

    #[test]
    fn unsupported_does_not_inspect_or_dispatch_even_when_all_tokens_match() {
        let adapter = FakeAdapter {
            capability: BrowserSurfaceCapability::Unsupported,
            ..FakeAdapter::default()
        };
        assert_refused(&attempt(&adapter), &adapter);
        assert_eq!(adapter.process_reads.get(), 0);
        assert_eq!(adapter.evidence_reads.get(), 0);
    }

    #[test]
    fn exact_profile_tab_url_process_and_window_are_verified_after_focus() {
        let adapter = FakeAdapter::default();
        let receipt = attempt(&adapter);
        assert_eq!(receipt.resource, URL);
        assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
        assert_eq!(receipt.verification, VerificationResult::Verified);
        assert_eq!(*adapter.effects.borrow(), [0]);
        assert_eq!(adapter.process_reads.get(), 4);
        assert_eq!(adapter.evidence_reads.get(), 3);
    }

    #[test]
    fn tab_tokens_are_scoped_to_the_exact_profile_not_the_url() {
        let mut other_profile = evidence();
        other_profile.browser_profile = Some("profile-b".into());
        let mut other_tab = evidence();
        other_tab.browser_tab = Some("tab-b".into());
        let adapter = FakeAdapter {
            candidates: vec![other_profile, other_tab, evidence()],
            ..FakeAdapter::default()
        };
        let receipt = attempt(&adapter);
        assert_eq!(receipt.verification, VerificationResult::Verified);
        assert_eq!(*adapter.effects.borrow(), [2]);
    }

    #[test]
    fn unknown_witnessed_tokens_cannot_be_synthesized_from_live_tokens() {
        for missing in 0..3 {
            let mut target = surface();
            match missing {
                0 => target.browser_profile = None,
                1 => target.browser_tab = None,
                _ => target.window_identifier = None,
            }
            let adapter = FakeAdapter::default();
            assert_refused(
                &raise_browser_surface_with_adapter(&target, URL, &adapter),
                &adapter,
            );
            assert_eq!(adapter.process_reads.get(), 0);
        }
    }

    #[test]
    fn missing_or_unreadable_live_tokens_refuse_without_a_url_fallback() {
        for missing in 0..3 {
            for token in [None, Some(""), Some(" \n "), Some("invalid\0token")] {
                let mut live = evidence();
                let token = token.map(str::to_string);
                match missing {
                    0 => live.browser_profile = token,
                    1 => live.browser_tab = token,
                    _ => live.window_identifier = token,
                }
                let adapter = FakeAdapter {
                    candidates: vec![live],
                    ..FakeAdapter::default()
                };
                assert_refused(&attempt(&adapter), &adapter);
            }
        }
    }

    #[test]
    fn unreadable_other_candidate_may_hide_duplicate_identity_and_refuses() {
        for missing in 0..2 {
            let mut unknown = evidence();
            if missing == 0 {
                unknown.browser_profile = None;
            } else {
                unknown.browser_tab = None;
            }
            let adapter = FakeAdapter {
                candidates: vec![evidence(), unknown],
                ..FakeAdapter::default()
            };
            assert_refused(&attempt(&adapter), &adapter);
        }
    }

    #[test]
    fn duplicate_tokens_refuse_even_when_url_or_window_would_disambiguate() {
        for difference in 0..3 {
            let mut duplicate = evidence();
            match difference {
                0 => duplicate.url = Some("https://example.test/other".into()),
                1 => duplicate.window_identifier = Some("native-window-b".into()),
                _ => {}
            }
            let adapter = FakeAdapter {
                candidates: vec![evidence(), duplicate],
                ..FakeAdapter::default()
            };
            let receipt = attempt(&adapter);
            assert_refused(&receipt, &adapter);
            assert!(
                matches!(receipt.dispatch, DispatchResult::NotDispatched { reason } if reason.contains("ambiguous"))
            );
        }
    }

    #[test]
    fn absent_or_wrong_token_never_selects_another_tab_with_the_exact_url() {
        for candidates in [
            vec![],
            {
                let mut wrong = evidence();
                wrong.browser_tab = Some("tab-b".into());
                vec![wrong]
            },
            {
                let mut wrong = evidence();
                wrong.browser_profile = Some("profile-b".into());
                vec![wrong]
            },
        ] {
            let adapter = FakeAdapter {
                candidates,
                ..FakeAdapter::default()
            };
            assert_refused(&attempt(&adapter), &adapter);
        }
    }

    #[test]
    fn missing_or_nonidentical_url_refuses_without_normalization_or_navigation() {
        for url in [
            None,
            Some("https://example.test/work?exact=1"),
            Some("https://example.test/work?exact=1#other"),
            Some("https://EXAMPLE.test/work?exact=1#section"),
        ] {
            let mut live = evidence();
            live.url = url.map(str::to_string);
            let adapter = FakeAdapter {
                candidates: vec![live],
                ..FakeAdapter::default()
            };
            assert_refused(&attempt(&adapter), &adapter);
        }
    }

    #[test]
    fn wrong_native_window_or_process_refuses_despite_matching_tokens_and_url() {
        for difference in 0..2 {
            let mut live = evidence();
            if difference == 0 {
                live.window_identifier = Some("native-window-b".into());
            } else {
                live.process_id += 1;
            }
            let adapter = FakeAdapter {
                candidates: vec![live],
                ..FakeAdapter::default()
            };
            assert_refused(&attempt(&adapter), &adapter);
        }
    }

    #[test]
    fn retained_tab_mutation_before_dispatch_refuses_without_searching_again() {
        for difference in 0..5 {
            let mut changed = evidence();
            mutate(&mut changed, difference);
            let adapter = FakeAdapter {
                selected_reread: Some(changed),
                ..FakeAdapter::default()
            };
            assert_refused(&attempt(&adapter), &adapter);
            assert_eq!(adapter.evidence_reads.get(), 2);
        }
    }

    #[test]
    fn process_restart_before_inspection_or_dispatch_refuses_without_fallback() {
        for read in [1, 2] {
            let adapter = FakeAdapter {
                restart_on_read: Some(read),
                ..FakeAdapter::default()
            };
            assert_refused(&attempt(&adapter), &adapter);
        }
    }

    #[test]
    fn process_restart_after_dispatch_never_reports_verified_or_retries() {
        for read in [3, 4] {
            let adapter = FakeAdapter {
                restart_on_read: Some(read),
                ..FakeAdapter::default()
            };
            let receipt = attempt(&adapter);
            assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
            assert!(
                matches!(receipt.verification, VerificationResult::Failed { reason } if reason.contains("lifetime changed"))
            );
            assert_eq!(*adapter.effects.borrow(), [0]);
        }
    }

    fn mutate(live: &mut BrowserTabEvidence, difference: usize) {
        match difference {
            0 => live.process_id += 1,
            1 => live.window_identifier = Some("native-window-b".into()),
            2 => live.browser_profile = Some("profile-b".into()),
            3 => live.browser_tab = Some("tab-b".into()),
            _ => live.url = Some("https://example.test/other".into()),
        }
    }

    #[test]
    fn postcondition_mismatch_preserves_accepted_dispatch_without_claiming_success() {
        for difference in 0..5 {
            let mut focused = evidence();
            mutate(&mut focused, difference);
            let adapter = FakeAdapter {
                focused: Ok(focused),
                ..FakeAdapter::default()
            };
            let receipt = attempt(&adapter);
            assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
            assert!(matches!(
                receipt.verification,
                VerificationResult::Failed { .. }
            ));
            assert_eq!(
                *adapter.effects.borrow(),
                [0],
                "must not retry or reopen URL"
            );
        }
    }

    #[test]
    fn unreadable_focus_does_not_treat_browser_local_active_tab_as_success() {
        let adapter = FakeAdapter {
            focused: Err("browser is not the globally focused application".into()),
            ..FakeAdapter::default()
        };
        let receipt = attempt(&adapter);
        assert_eq!(receipt.dispatch, DispatchResult::RaiseAccepted);
        assert!(matches!(
            receipt.verification,
            VerificationResult::Failed { .. }
        ));
        assert_eq!(*adapter.effects.borrow(), [0]);
    }

    #[test]
    fn enumeration_errors_refuse_and_dispatch_errors_are_never_retried() {
        let adapter = FakeAdapter {
            enumeration_error: Some("incomplete tab enumeration".into()),
            ..FakeAdapter::default()
        };
        assert_refused(&attempt(&adapter), &adapter);
        let adapter = FakeAdapter {
            focus_error: Some("stale retained tab handle".into()),
            ..FakeAdapter::default()
        };
        let receipt = attempt(&adapter);
        assert!(matches!(receipt.dispatch, DispatchResult::Failed { .. }));
        assert!(matches!(
            receipt.verification,
            VerificationResult::Unverified { .. }
        ));
        assert_eq!(*adapter.effects.borrow(), [0]);
    }

    #[test]
    fn invalid_identity_and_non_url_resources_refuse_before_inspection() {
        let mut invalid = surface();
        invalid.window_id = "window-0".into();
        let adapter = FakeAdapter::default();
        assert_refused(
            &raise_browser_surface_with_adapter(&invalid, URL, &adapter),
            &adapter,
        );
        for resource in [
            "Report",
            "/work/report.md",
            "file:///work/report.md",
            "https://example.test/nul\0",
        ] {
            let adapter = FakeAdapter::default();
            let receipt = raise_browser_surface_with_adapter(&surface(), resource, &adapter);
            assert!(matches!(
                receipt.dispatch,
                DispatchResult::NotDispatched { .. }
            ));
            assert!(adapter.effects.borrow().is_empty());
            assert_eq!(adapter.process_reads.get(), 0);
        }
    }
}
