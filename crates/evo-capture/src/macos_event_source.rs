//! Real macOS event source.
//!
//! This source witnesses focused-window changes through Accessibility and emits
//! canonical `WindowFocusGained` signals only when macOS directly reports the
//! focused window for the frontmost application.

use crate::adapters::macos::MacOSSignal;
use evo_observation::surface_identity::{BrowserIdentityObservation, SurfaceIdentity};

/// Result of asking an explicitly installed local browser adapter for one
/// direct focused-surface witness.
///
/// `Unsupported` is intentional: generic macOS Accessibility does not expose
/// browser profile/tab tokens. `Unreadable` is distinct from unsupported but
/// has the same conservative capture behavior for this sample. Neither case
/// permits a URL, title, pid, or window id to stand in for browser identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserIdentityWitness {
    Unsupported,
    Unreadable,
    Observed(BrowserIdentityObservation),
}

/// Narrow capture seam for a local browser adapter.
///
/// Implementations must return tokens directly witnessed from the retained
/// browser surface. They must not synthesize profile/tab values from generic
/// macOS fields. The capture source owns the native process/window evidence;
/// the registry below only accepts a complete observation that agrees with it.
pub trait BrowserIdentityProvider {
    fn witness_focused_surface(&self, native: &SurfaceIdentity) -> BrowserIdentityWitness;
}

/// The production default. No browser automation or browser identity probing
/// is attempted unless a caller explicitly installs a provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnsupportedBrowserIdentityProvider;

impl BrowserIdentityProvider for UnsupportedBrowserIdentityProvider {
    fn witness_focused_surface(&self, _native: &SurfaceIdentity) -> BrowserIdentityWitness {
        BrowserIdentityWitness::Unsupported
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NativeSurfaceKey {
    process_id: i32,
    process_started_at: String,
    window_identifier: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BrowserTokenKey {
    profile: String,
    session: String,
    tab: String,
    browser_window: String,
}

/// Capture-side registry for direct browser witnesses.
///
/// The registry deliberately has no browser implementation. It is a small
/// ownership/ambiguity guard around an injected provider: incomplete or
/// contradictory readings are stripped as a unit, and one profile/tab pair
/// cannot silently bind to two native surfaces in the same capture registry.
pub struct BrowserIdentityRegistry {
    provider: Box<dyn BrowserIdentityProvider>,
    by_native_surface: std::collections::HashMap<NativeSurfaceKey, BrowserTokenKey>,
    by_browser_token: std::collections::HashMap<BrowserTokenKey, NativeSurfaceKey>,
}

impl Default for BrowserIdentityRegistry {
    fn default() -> Self {
        Self::unsupported()
    }
}

impl BrowserIdentityRegistry {
    pub fn new<P>(provider: P) -> Self
    where
        P: BrowserIdentityProvider + 'static,
    {
        Self {
            provider: Box::new(provider),
            by_native_surface: std::collections::HashMap::new(),
            by_browser_token: std::collections::HashMap::new(),
        }
    }

    pub fn unsupported() -> Self {
        Self::new(UnsupportedBrowserIdentityProvider)
    }

    /// Enriches one already-valid native capture identity, if and only if the
    /// provider directly witnessed a complete matching browser identity.
    pub fn enrich(&mut self, surface: SurfaceIdentity) -> SurfaceIdentity {
        let native = surface.without_browser_identity();
        let Some(native_key) = native_surface_key(&native) else {
            return native;
        };

        let witness = self.provider.witness_focused_surface(&native);
        let BrowserIdentityWitness::Observed(observation) = witness else {
            return native;
        };

        let enriched = native.with_browser_identity_observation(&observation);
        let (Some(profile), Some(session), Some(tab), Some(browser_window)) = (
            enriched.browser_profile.clone(),
            enriched.browser_session.clone(),
            enriched.browser_tab.clone(),
            enriched.browser_window.clone(),
        ) else {
            return native;
        };

        let browser_key = BrowserTokenKey {
            profile,
            session,
            tab,
            browser_window,
        };
        if self
            .by_browser_token
            .get(&browser_key)
            .is_some_and(|owner| owner != &native_key)
        {
            // Duplicate identity is ambiguous. Keep the prior ownership
            // record as a conservative guard, but never steal the token.
            return native;
        }

        self.forget_native(&native_key);
        self.by_native_surface
            .insert(native_key.clone(), browser_key.clone());
        self.by_browser_token.insert(browser_key, native_key);
        enriched
    }

    fn forget_native(&mut self, native_key: &NativeSurfaceKey) {
        let Some(browser_key) = self.by_native_surface.remove(native_key) else {
            return;
        };
        if self
            .by_browser_token
            .get(&browser_key)
            .is_some_and(|owner| owner == native_key)
        {
            self.by_browser_token.remove(&browser_key);
        }
    }
}

fn native_surface_key(surface: &SurfaceIdentity) -> Option<NativeSurfaceKey> {
    Some(NativeSurfaceKey {
        process_id: surface.process_id,
        process_started_at: surface.process_started_at.clone(),
        window_identifier: surface.window_identifier.clone()?,
    })
}

#[cfg(test)]
mod tests {
    use super::imp::pid_from_storage_root;
    use super::{
        BrowserIdentityProvider, BrowserIdentityRegistry, BrowserIdentityWitness,
        UnsupportedBrowserIdentityProvider, desktop_shell_pid, is_self_window,
    };
    use evo_observation::surface_identity::{BrowserIdentityObservation, SurfaceIdentity};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[test]
    fn self_window_matches_only_the_desktop_shell_pid() {
        assert!(is_self_window(42, Some(42)));
        assert!(!is_self_window(43, Some(42)));
        // No desktop shell declared (development launch): nothing is excluded.
        assert!(!is_self_window(42, None));
    }

    #[test]
    fn desktop_shell_pid_reads_only_a_valid_positive_env_value() {
        unsafe { std::env::set_var("EVO_DESKTOP_PID", "1234") };
        assert_eq!(desktop_shell_pid(), Some(1234));

        // An invalid env declaration falls through to the pid file — the
        // more durable declaration — whatever that holds on this machine.
        let file_result = {
            let home = std::env::var("HOME").unwrap_or_default();
            let root = std::path::Path::new(&home).join("Library/Application Support/evo/storage");
            pid_from_storage_root(&root)
        };
        unsafe { std::env::set_var("EVO_DESKTOP_PID", "not-a-number") };
        assert_eq!(desktop_shell_pid(), file_result);
        unsafe { std::env::set_var("EVO_DESKTOP_PID", "-5") };
        assert_eq!(desktop_shell_pid(), file_result);

        // No env declaration: the pid file decides — None over an empty
        // root (asserted without the machine's live state).
        unsafe { std::env::remove_var("EVO_DESKTOP_PID") };
        let empty_root = std::env::temp_dir().join("evo-test-empty-root");
        let _ = std::fs::create_dir_all(&empty_root);
        let _ = std::fs::remove_file(empty_root.join("desktop.pid"));
        assert_eq!(pid_from_storage_root(&empty_root), None);
    }

    #[test]
    fn pid_file_fallback_finds_a_live_shell_and_ignores_stale_ones() {
        let root = std::env::temp_dir().join("evo-test-pidfile-root");
        let _ = std::fs::create_dir_all(&root);
        // A stale (dead) pid is ignored: never exclude an unrelated
        // process that happens to reuse the number later.
        let _ = std::fs::write(root.join("desktop.pid"), "999999999");
        assert_eq!(pid_from_storage_root(&root), None);
        // A live pid — this very process — is found.
        let _ = std::fs::write(root.join("desktop.pid"), std::process::id().to_string());
        assert_eq!(
            pid_from_storage_root(&root),
            Some(std::process::id() as i32)
        );
        let _ = std::fs::remove_file(root.join("desktop.pid"));
    }

    fn surface(window_identifier: &str) -> SurfaceIdentity {
        SurfaceIdentity {
            capture_session: "capture-browser-test".into(),
            window_id: "window-1".into(),
            process_id: 123,
            process_started_at: format!("{:016x}", 1_791_000_000.5f64.to_bits()),
            window_identifier: Some(window_identifier.into()),
            browser_window: None,
            browser_profile: None,
            browser_session: None,
            browser_tab: None,
        }
    }

    fn browser_observation(window_identifier: &str) -> BrowserIdentityObservation {
        BrowserIdentityObservation {
            process_id: Some(123),
            process_started_at: Some(format!("{:016x}", 1_791_000_000.5f64.to_bits())),
            window_identifier: Some(window_identifier.into()),
            browser_window: Some("browser-window-1".into()),
            browser_profile: Some("profile-direct".into()),
            browser_session: Some("session-direct".into()),
            browser_tab: Some("tab-direct".into()),
        }
    }

    struct SyntheticProvider {
        witnesses: RefCell<VecDeque<BrowserIdentityWitness>>,
    }

    impl BrowserIdentityProvider for SyntheticProvider {
        fn witness_focused_surface(&self, _native: &SurfaceIdentity) -> BrowserIdentityWitness {
            self.witnesses
                .borrow_mut()
                .pop_front()
                .unwrap_or(BrowserIdentityWitness::Unreadable)
        }
    }

    #[test]
    fn unsupported_provider_is_explicit_and_preserves_native_capture_evidence() {
        let provider = UnsupportedBrowserIdentityProvider;
        assert_eq!(
            provider.witness_focused_surface(&surface("native-window-1")),
            BrowserIdentityWitness::Unsupported
        );
        let original = surface("native-window-1");
        let enriched = BrowserIdentityRegistry::unsupported().enrich(original.clone());
        assert_eq!(enriched, original);
        assert_eq!(enriched.browser_profile, None);
        assert_eq!(enriched.browser_tab, None);
        assert_eq!(enriched.process_started_at, original.process_started_at);
        assert_eq!(enriched.window_identifier, original.window_identifier);
        assert_eq!(enriched.browser_window, None);
        assert_eq!(enriched.browser_session, None);
    }

    #[test]
    fn synthetic_direct_browser_observation_is_attached_without_derivation() {
        let provider = SyntheticProvider {
            witnesses: RefCell::new(VecDeque::from([BrowserIdentityWitness::Observed(
                browser_observation("native-window-1"),
            )])),
        };
        let mut registry = BrowserIdentityRegistry::new(provider);
        let original = surface("native-window-1");
        let enriched = registry.enrich(original.clone());

        assert_eq!(enriched.capture_session, original.capture_session);
        assert_eq!(enriched.window_id, original.window_id);
        assert_eq!(enriched.process_id, original.process_id);
        assert_eq!(enriched.process_started_at, original.process_started_at);
        assert_eq!(enriched.window_identifier, original.window_identifier);
        assert_eq!(enriched.browser_window.as_deref(), Some("browser-window-1"));
        assert_eq!(enriched.browser_profile.as_deref(), Some("profile-direct"));
        assert_eq!(enriched.browser_session.as_deref(), Some("session-direct"));
        assert_eq!(enriched.browser_tab.as_deref(), Some("tab-direct"));
    }

    #[test]
    fn partial_and_duplicate_browser_observations_are_omitted_atomically() {
        let mut partial = browser_observation("native-window-1");
        partial.browser_tab = None;
        let provider = SyntheticProvider {
            witnesses: RefCell::new(VecDeque::from([
                BrowserIdentityWitness::Observed(browser_observation("native-window-1")),
                BrowserIdentityWitness::Observed(partial),
                BrowserIdentityWitness::Unreadable,
                BrowserIdentityWitness::Observed(browser_observation("native-window-2")),
            ])),
        };
        let mut registry = BrowserIdentityRegistry::new(provider);
        let first = registry.enrich(surface("native-window-1"));
        assert_eq!(first.browser_profile.as_deref(), Some("profile-direct"));
        assert_eq!(first.browser_tab.as_deref(), Some("tab-direct"));

        let partial = registry.enrich(surface("native-window-1"));
        assert_eq!(partial.browser_profile, None);
        assert_eq!(partial.browser_window, None);
        assert_eq!(partial.browser_session, None);
        assert_eq!(partial.browser_tab, None);
        assert_eq!(partial.process_started_at, first.process_started_at);
        assert_eq!(partial.window_identifier, first.window_identifier);

        let unreadable = registry.enrich(surface("native-window-1"));
        assert_eq!(unreadable.browser_profile, None);
        assert_eq!(unreadable.browser_window, None);
        assert_eq!(unreadable.browser_session, None);
        assert_eq!(unreadable.browser_tab, None);
        assert_eq!(unreadable.process_started_at, first.process_started_at);
        assert_eq!(unreadable.window_identifier, first.window_identifier);

        let duplicate = registry.enrich(surface("native-window-2"));
        assert_eq!(duplicate.browser_profile, None);
        assert_eq!(duplicate.browser_window, None);
        assert_eq!(duplicate.browser_session, None);
        assert_eq!(duplicate.browser_tab, None);
        assert_eq!(duplicate.process_started_at, first.process_started_at);
        assert_eq!(
            duplicate.window_identifier.as_deref(),
            Some("native-window-2")
        );
    }
}

/// Returns whether this process is trusted by macOS Accessibility (TCC).
///
/// Witnessing focused windows requires Accessibility permission; the daemon
/// checks this before attaching its observer and reports the honest
/// `AccessibilityPermissionRequired` error when it is not granted.
pub fn accessibility_permission_granted() -> bool {
    imp::accessibility_permission_granted()
}

/// Requests Accessibility permission through the macOS system dialog.
///
/// Calls `AXIsProcessTrustedWithOptions` with the trusted-check prompt
/// option so the user is offered the native System Settings grant. This is
/// the standard macOS way to ask for Accessibility; it never claims the
/// permission was granted.
pub fn prompt_for_accessibility_permission() {
    imp::prompt_for_accessibility_permission();
}

/// The process id of the Evo desktop shell, when the capture daemon was
/// spawned by it.
///
/// The desktop shell declares itself through `EVO_DESKTOP_PID` so the
/// capture boundary can exclude Evo's own UI from canonical evidence: Evo
/// witnessing its own focused window is Evo observing itself, not evidence
/// of the user's work. This is a composition-level self-reference boundary
/// (the same category as the storage-root exclusion): it is identity-based,
/// never a hardcoded application name, and never a semantic filter on the
/// user's activity. When the daemon is launched directly (development), no
/// variable is set and no exclusion applies.
pub fn desktop_shell_pid() -> Option<i32> {
    imp::desktop_shell_pid()
}

/// Whether `pid` belongs to Evo's own desktop shell (which owns Evo's UI
/// window) and therefore must not be witnessed as user work.
///
/// Pure and deterministic: Evo's own UI is the only excluded identity, and
/// only when the desktop shell actually spawned this capture process.
pub fn is_self_window(pid: i32, desktop_shell: Option<i32>) -> bool {
    desktop_shell == Some(pid)
}

#[cfg(target_os = "macos")]
pub(crate) mod imp {
    use super::{MacOSSignal, is_self_window};
    use evo_observation::observed_state::ObservedState;
    use evo_observation::surface_identity::SurfaceIdentity;
    use evo_observation::window_registry::{
        ObservedWindow, WindowId, WindowRegistry, select_focused,
    };

    use std::ffi::CStr;
    use std::fmt;
    use std::os::raw::{c_char, c_void};
    use std::ptr;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::SystemTime;

    type Bool = i8;
    type Id = *mut Object;
    type Class = *mut objc_class;
    type Sel = *mut c_void;
    type Ivar = *mut c_void;
    type AXObserverRef = *mut c_void;
    type AXUIElementRef = *mut c_void;
    type AXError = i32;
    type CFTypeRef = *const c_void;
    type CFRunLoopRef = *const c_void;
    type CFRunLoopSourceRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFTypeID = usize;
    type AXValueRef = *const c_void;

    const AX_SUCCESS: AXError = 0;
    const CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;
    const AX_VALUE_CF_RANGE_TYPE: i32 = 4;
    const CF_NUMBER_SINT64_TYPE: i32 = 4;

    #[repr(C)]
    struct CFRange {
        location: isize,
        length: isize,
    }

    #[repr(C)]
    pub struct Object {
        _priv: [u8; 0],
    }

    #[repr(C)]
    pub struct objc_class {
        _priv: [u8; 0],
    }

    #[allow(clippy::upper_case_acronyms)]
    type IMP = extern "C" fn(Id, Sel, Id);
    type AXObserverCallback =
        extern "C" fn(AXObserverRef, AXUIElementRef, CFStringRef, *mut c_void);

    #[allow(clippy::duplicated_attributes)]
    #[link(name = "objc")]
    #[link(name = "Foundation", kind = "framework")]
    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "ApplicationServices", kind = "framework")]
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Class;
        fn objc_allocateClassPair(
            superclass: Class,
            name: *const c_char,
            extra_bytes: usize,
        ) -> Class;
        fn objc_registerClassPair(cls: Class);
        fn class_addMethod(cls: Class, name: Sel, imp: IMP, types: *const c_char) -> Bool;
        fn class_addIvar(
            cls: Class,
            name: *const c_char,
            size: usize,
            alignment: u8,
            types: *const c_char,
        ) -> Bool;
        fn object_setInstanceVariable(
            obj: Id,
            name: *const c_char,
            value: *mut c_void,
        ) -> *mut Ivar;
        fn object_getInstanceVariable(
            obj: Id,
            name: *const c_char,
            out_value: *mut *mut c_void,
        ) -> *mut Ivar;
        fn objc_msgSend();

        fn AXIsProcessTrusted() -> Bool;
        fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> Bool;
        fn AXObserverCreate(
            pid: i32,
            callback: AXObserverCallback,
            out_observer: *mut AXObserverRef,
        ) -> AXError;
        fn AXObserverAddNotification(
            observer: AXObserverRef,
            element: AXUIElementRef,
            notification: CFStringRef,
            refcon: *mut c_void,
        ) -> AXError;
        fn AXObserverRemoveNotification(
            observer: AXObserverRef,
            element: AXUIElementRef,
            notification: CFStringRef,
        ) -> AXError;
        fn AXObserverGetRunLoopSource(observer: AXObserverRef) -> CFRunLoopSourceRef;
        fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut i32) -> AXError;
        fn AXUIElementGetTypeID() -> CFTypeID;
        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
        fn CFRunLoopRemoveSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
        fn CFStringGetLength(the_string: CFStringRef) -> isize;
        fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
        fn CFStringGetCString(
            the_string: CFStringRef,
            buffer: *mut c_char,
            buffer_size: isize,
            encoding: u32,
        ) -> Bool;
        fn CFStringCreateWithCString(
            alloc: CFTypeRef,
            c_str: *const c_char,
            encoding: u32,
        ) -> CFStringRef;
        fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;
        fn CFStringGetTypeID() -> CFTypeID;
        fn CFNumberGetTypeID() -> CFTypeID;
        fn CFNumberGetValue(number: CFTypeRef, number_type: i32, value: *mut c_void) -> Bool;
        fn AXValueGetType(value: AXValueRef) -> i32;
        fn AXValueGetValue(value: AXValueRef, value_type: i32, value_out: *mut c_void) -> Bool;
        fn CFRelease(cf: CFTypeRef);

        static kCFRunLoopDefaultMode: CFStringRef;
        static NSWorkspaceDidActivateApplicationNotification: Id;
        static NSWorkspaceApplicationKey: Id;
    }

    const OBSERVER_CLASS_NAME: &[u8] = b"EvoMacOSEventObserver\0";
    const CALLBACK_IVAR_NAME: &[u8] = b"rustCallback\0";
    const WORKSPACE_DID_ACTIVATE_SEL: &[u8] = b"workspaceDidActivate:\0";
    const DICTIONARY_WITH_OBJECT_FOR_KEY_SEL: &[u8] = b"dictionaryWithObject:forKey:\0";
    const NUMBER_WITH_BOOL_SEL: &[u8] = b"numberWithBool:\0";
    const AX_TRUSTED_CHECK_OPTION_PROMPT: &[u8] = b"AXTrustedCheckOptionPrompt\0";
    const ALLOC_SEL: &[u8] = b"alloc\0";
    const INIT_SEL: &[u8] = b"init\0";
    const RELEASE_SEL: &[u8] = b"release\0";
    const SHARED_WORKSPACE_SEL: &[u8] = b"sharedWorkspace\0";
    const FRONTMOST_APPLICATION_SEL: &[u8] = b"frontmostApplication\0";
    const NOTIFICATION_CENTER_SEL: &[u8] = b"notificationCenter\0";
    const ADD_OBSERVER_SEL: &[u8] = b"addObserver:selector:name:object:\0";
    const REMOVE_OBSERVER_SEL: &[u8] = b"removeObserver:name:object:\0";
    const USER_INFO_SEL: &[u8] = b"userInfo\0";
    const OBJECT_FOR_KEY_SEL: &[u8] = b"objectForKey:\0";
    const PROCESS_IDENTIFIER_SEL: &[u8] = b"processIdentifier\0";
    const RUNNING_APPLICATION_WITH_PROCESS_IDENTIFIER_SEL: &[u8] =
        b"runningApplicationWithProcessIdentifier:\0";
    const LAUNCH_DATE_SEL: &[u8] = b"launchDate\0";
    const TIME_INTERVAL_SINCE_1970_SEL: &[u8] = b"timeIntervalSince1970\0";
    #[allow(dead_code)]
    const UTF8_STRING_SEL: &[u8] = b"UTF8String\0";
    const AX_FOCUSED_WINDOW_CHANGED_NOTIFICATION: &[u8] = b"AXFocusedWindowChanged\0";
    const AX_FOCUSED_WINDOW_ATTRIBUTE: &[u8] = b"AXFocusedWindow\0";
    const AX_FOCUSED_UI_ELEMENT_ATTRIBUTE: &[u8] = b"AXFocusedUIElement\0";
    const AX_TITLE_ATTRIBUTE: &[u8] = b"AXTitle\0";
    const AX_DOCUMENT_ATTRIBUTE: &[u8] = b"AXDocument\0";
    const AX_ROLE_ATTRIBUTE: &[u8] = b"AXRole\0";
    const AX_IDENTIFIER_ATTRIBUTE: &[u8] = b"AXIdentifier\0";
    const AX_SELECTED_TEXT_RANGE_ATTRIBUTE: &[u8] = b"AXSelectedTextRange\0";
    const AX_INSERTION_POINT_LINE_NUMBER_ATTRIBUTE: &[u8] = b"AXInsertionPointLineNumber\0";
    const VOID_AT_AT_COLON_AT_TYPES: &[u8] = b"v@:@\0";
    const POINTER_TYPE: &[u8] = b"^v\0";
    const SPACEWORK_DIMENSION: u8 = 3;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MacOSEventSourceError {
        UnsupportedPlatform,
        AccessibilityPermissionRequired,
        RuntimeSetupFailed(&'static str),
    }

    impl fmt::Display for MacOSEventSourceError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MacOSEventSourceError::UnsupportedPlatform => {
                    write!(f, "macOS event source is only available on macOS")
                }
                MacOSEventSourceError::AccessibilityPermissionRequired => {
                    write!(
                        f,
                        "Accessibility permission is required to witness focused windows"
                    )
                }
                MacOSEventSourceError::RuntimeSetupFailed(step) => {
                    write!(f, "macOS event source setup failed: {}", step)
                }
            }
        }
    }

    impl std::error::Error for MacOSEventSourceError {}

    struct SignalSinkHolder {
        callback: Box<dyn FnMut(MacOSSignal)>,
        browser_identity_registry: super::BrowserIdentityRegistry,
        focus_attachment: Option<FocusAttachment>,
        /// The pid of Evo's own desktop shell, when this capture process was
        /// spawned by it. Evo's own UI window is excluded from capture at
        /// the composition boundary: witnessing it would be Evo observing
        /// itself, not observing the user's work.
        desktop_shell_pid: Option<i32>,
        /// The most recently emitted window subject. Used only to suppress
        /// duplicate emissions of the same window around attach-time races;
        /// it never influences which subject is emitted. The key includes the
        /// window id and freshly witnessed identity so same-title windows and
        /// restarted processes are real switches, never suppressed.
        last_emitted_focus: Option<FocusKey>,
        /// Stable window identity across snapshots. Fed on every focus
        /// emission — including suppressed duplicates — so open/close/title
        /// tracking stays current. Owned here, not in the daemon, because the
        /// dedup decision needs the window id before the signal exists.
        registry: WindowRegistry,
        /// Namespaces this registry's ids across capture restarts.
        capture_session: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FocusKey {
        process_identifier: Option<i32>,
        subject: String,
        window_id: Option<WindowId>,
        surface_identity: Option<SurfaceIdentity>,
    }

    struct FocusAttachment {
        observer: AXObserverRef,
        run_loop_source: CFRunLoopSourceRef,
        element: AXUIElementRef,
        pid: i32,
    }

    impl Drop for FocusAttachment {
        fn drop(&mut self) {
            unsafe {
                let _ = AXObserverRemoveNotification(
                    self.observer,
                    self.element,
                    ax_focused_window_changed_notification(),
                );
                CFRunLoopRemoveSource(
                    CFRunLoopGetCurrent(),
                    self.run_loop_source,
                    kCFRunLoopDefaultMode,
                );
                // The run loop source returned by AXObserverGetRunLoopSource is
                // owned by the observer: the observer retains it and releases it
                // when the observer is released. It must not be released
                // separately — doing so frees the source before the observer's
                // release invalidates it, crashing the process.
                CFRelease(self.element as CFTypeRef);
                CFRelease(self.observer as CFTypeRef);
                self.observer = ptr::null_mut();
                self.run_loop_source = ptr::null_mut();
                self.element = ptr::null_mut();
                self.pid = 0;
            }
        }
    }

    struct ObserverHandle {
        ptr: Id,
    }

    impl Drop for ObserverHandle {
        fn drop(&mut self) {
            unsafe {
                if !self.ptr.is_null() {
                    msg_send_void0(self.ptr, release_sel());
                    self.ptr = ptr::null_mut();
                }
            }
        }
    }

    pub struct MacOSEventSource {
        center: Id,
        observer: ObserverHandle,
        callback: *mut SignalSinkHolder,
    }

    impl MacOSEventSource {
        pub fn new<F>(
            desktop_shell: Option<i32>,
            callback: F,
        ) -> Result<Self, MacOSEventSourceError>
        where
            F: FnMut(MacOSSignal) + 'static,
        {
            Self::new_with_browser_identity_provider(
                desktop_shell,
                super::UnsupportedBrowserIdentityProvider,
                callback,
            )
        }

        /// Installs a local browser identity provider without changing the
        /// native capture path. The provider is consulted only after the
        /// retained AX window and process-lifetime evidence have been read.
        pub fn new_with_browser_identity_provider<P, F>(
            desktop_shell: Option<i32>,
            browser_identity_provider: P,
            callback: F,
        ) -> Result<Self, MacOSEventSourceError>
        where
            P: super::BrowserIdentityProvider + 'static,
            F: FnMut(MacOSSignal) + 'static,
        {
            unsafe {
                if AXIsProcessTrusted() == 0 {
                    return Err(MacOSEventSourceError::AccessibilityPermissionRequired);
                }

                let callback = Box::into_raw(Box::new(SignalSinkHolder {
                    callback: Box::new(callback),
                    browser_identity_registry: super::BrowserIdentityRegistry::new(
                        browser_identity_provider,
                    ),
                    focus_attachment: None,
                    desktop_shell_pid: desktop_shell,
                    last_emitted_focus: None,
                    registry: WindowRegistry::new(),
                    capture_session: new_capture_session(),
                }));

                let observer = create_observer(callback)?;
                let center = workspace_notification_center()?;

                let add_observer = add_observer_sel();
                let notification_name = NSWorkspaceDidActivateApplicationNotification;
                let nil: Id = ptr::null_mut();
                msg_send_void4(
                    center,
                    add_observer,
                    observer,
                    activate_selector(),
                    notification_name,
                    nil,
                );

                let source = Self {
                    center,
                    observer: ObserverHandle { ptr: observer },
                    callback,
                };
                source.bind_frontmost_application()?;
                Ok(source)
            }
        }

        unsafe fn bind_frontmost_application(&self) -> Result<(), MacOSEventSourceError> {
            unsafe {
                let pid = frontmost_application_pid()?;
                if let Some(pid) = pid {
                    let holder = &mut *self.callback;
                    holder.attach_to_pid(pid)?;
                }
                Ok(())
            }
        }
    }

    impl Drop for MacOSEventSource {
        fn drop(&mut self) {
            unsafe {
                if !self.center.is_null() && !self.observer.ptr.is_null() {
                    let remove_observer = remove_observer_sel();
                    let nil: Id = ptr::null_mut();
                    msg_send_void3(
                        self.center,
                        remove_observer,
                        self.observer.ptr,
                        NSWorkspaceDidActivateApplicationNotification,
                        nil,
                    );
                }

                if !self.callback.is_null() {
                    let _ = Box::from_raw(self.callback);
                    self.callback = ptr::null_mut();
                }
            }
        }
    }

    impl SignalSinkHolder {
        unsafe fn attach_to_pid(&mut self, pid: i32) -> Result<(), MacOSEventSourceError> {
            unsafe {
                // Evo must not witness its own UI: when the desktop shell spawned
                // this capture process, its window is Evo observing itself, not
                // the user's work. The skip is at the attachment boundary — no
                // observation is ever formed for Evo's own window.
                if is_self_window(pid, self.desktop_shell_pid) {
                    eprintln!("EVO-CAPTURE self window pid {pid} (Evo's own UI, not captured)");
                    return Ok(());
                }
                if self
                    .focus_attachment
                    .as_ref()
                    .map(|attachment| attachment.pid == pid)
                    .unwrap_or(false)
                {
                    return Ok(());
                }

                eprintln!("EVO-CAPTURE attach to pid {pid}");
                // Before detaching from the outgoing application, take one final
                // generic Accessibility reading. Ordinary duplicate focus events
                // are suppressed, but this boundary sample is intentionally not:
                // it is the strongest truthful local evidence of where the person
                // left that artifact after changing state inside it.
                if let Some(previous_element) = self
                    .focus_attachment
                    .as_ref()
                    .map(|previous| previous.element)
                {
                    self.emit_current_subject(previous_element, true);
                }
                self.focus_attachment.take();

                let element = AXUIElementCreateApplication(pid);
                if element.is_null() {
                    eprintln!("EVO-CAPTURE AXUIElementCreateApplication failed for pid {pid}");
                    return Err(MacOSEventSourceError::RuntimeSetupFailed(
                        "AXUIElementCreateApplication",
                    ));
                }

                let mut observer: AXObserverRef = ptr::null_mut();
                if AXObserverCreate(pid, focus_changed_callback, &mut observer) != AX_SUCCESS
                    || observer.is_null()
                {
                    CFRelease(element as CFTypeRef);
                    return Err(MacOSEventSourceError::RuntimeSetupFailed(
                        "AXObserverCreate",
                    ));
                }

                if AXObserverAddNotification(
                    observer,
                    element,
                    ax_focused_window_changed_notification(),
                    self as *mut SignalSinkHolder as *mut c_void,
                ) != AX_SUCCESS
                {
                    eprintln!("EVO-CAPTURE AXObserverAddNotification failed for pid {pid}");
                    CFRelease(observer as CFTypeRef);
                    CFRelease(element as CFTypeRef);
                    return Err(MacOSEventSourceError::RuntimeSetupFailed(
                        "AXObserverAddNotification focused window",
                    ));
                }

                let run_loop_source = AXObserverGetRunLoopSource(observer);
                if run_loop_source.is_null() {
                    CFRelease(observer as CFTypeRef);
                    CFRelease(element as CFTypeRef);
                    return Err(MacOSEventSourceError::RuntimeSetupFailed(
                        "AXObserverGetRunLoopSource",
                    ));
                }

                CFRunLoopAddSource(
                    CFRunLoopGetCurrent(),
                    run_loop_source,
                    kCFRunLoopDefaultMode,
                );
                self.focus_attachment = Some(FocusAttachment {
                    observer,
                    run_loop_source,
                    element,
                    pid,
                });
                eprintln!("EVO-CAPTURE attached to pid {pid}");
                // The AXFocusedWindowChanged notification may fire before the
                // observer registers for a newly activated application, so the
                // currently focused window is read directly and emitted here.
                self.emit_current_subject(element, false);
                Ok(())
            }
        }

        unsafe fn emit_current_subject(&mut self, element: AXUIElementRef, boundary_sample: bool) {
            unsafe {
                // AX callbacks may carry the changed window element. Always
                // sample through the subscribed application, including boundary
                // samples, so AXFocusedWindow is read from its proper owner.
                let element = self
                    .focus_attachment
                    .as_ref()
                    .map(|attachment| attachment.element)
                    .unwrap_or(element);
                match focused_window_state(element) {
                    Some((subject, mut state, window_identifier, window_pid)) => {
                        let process_identifier = self
                            .focus_attachment
                            .as_ref()
                            .map(|attachment| attachment.pid);
                        // Resolve the window id BEFORE the dedup check: the
                        // registry must be fed on every emission — including
                        // suppressed duplicates — so open/close/title tracking
                        // stays current. A failed snapshot yields None; the
                        // dedup then falls back to (pid, subject), degraded
                        // but never guessing.
                        let document_url = state.document_locator().map(|s| s.to_string());
                        let window_id = self.resolve_focused_window(
                            process_identifier,
                            &subject,
                            document_url.as_deref(),
                        );
                        // The registry's CG ordering heuristic is provenance
                        // only. The native token is read from this AX window,
                        // never synthesized from the registry, title, or URL.
                        // Re-read the process launch token on every sample,
                        // before dedup, rather than trusting a cached pid.
                        let process_started_at = process_identifier
                            .filter(|pid| Some(*pid) == window_pid)
                            .and_then(|pid| process_started_at(pid));
                        if let Some(identity) = captured_surface_identity(
                            &self.capture_session,
                            window_id,
                            process_identifier,
                            process_started_at,
                            window_identifier,
                        ) {
                            let identity = self.browser_identity_registry.enrich(identity);
                            state = state.with_surface_identity(identity);
                        }
                        let focus_key = FocusKey {
                            process_identifier,
                            subject: subject.clone(),
                            window_id,
                            surface_identity: state.surface_identity().cloned(),
                        };
                        if !boundary_sample && self.last_emitted_focus.as_ref() == Some(&focus_key)
                        {
                            eprintln!("EVO-CAPTURE focus unchanged: {subject:?} (suppressed)");
                            return;
                        }
                        eprintln!("EVO-CAPTURE focus: {subject:?}");
                        self.last_emitted_focus = Some(focus_key);
                        // The owning process is already in scope at the emit site
                        // (the FocusAttachment the observer is attached through);
                        // it is carried as provenance, never discarded
                        // (BE-TRACE-0001 §2.1). The application element's own
                        // title is the owning application's localized name — the
                        // human-stable identity of the process a bare pid
                        // cannot give downstream readers.
                        let owning_process_name = copy_attribute(element, ax_title_attribute())
                            .and_then(|value| {
                                let name = cfstring_value(value);
                                CFRelease(value);
                                name
                            })
                            .filter(|name: &String| !name.trim().is_empty());
                        // The window title must survive subject resolution: the
                        // composition boundary may re-key the subject to the
                        // document locator (browsers expose their URL there),
                        // and the title — the page name the person reads — would
                        // otherwise be lost exactly when the richest record is
                        // written.
                        state = state.with_window_title(subject.clone());
                        self.emit(MacOSSignal::WindowFocusGained {
                            subject,
                            observed_at: SystemTime::now(),
                            process_identifier,
                            owning_process_name,
                            observed_state: (!state.is_empty()).then_some(state),
                            window_id,
                        });
                    }
                    None => eprintln!("EVO-CAPTURE focus unreadable for attached app"),
                }
            }
        }

        /// Resolves the focused window's stable [`WindowId`] from a live
        /// snapshot of the platform window list.
        ///
        /// Called on every focus emission — before the dedup check — so the
        /// registry's open/close/title tracking stays current even when the
        /// emission is suppressed. Returns `None` when the snapshot fails or
        /// the focused process has no normal window; the caller then falls
        /// back to (pid, subject) dedup, degraded but never guessing.
        unsafe fn resolve_focused_window(
            &mut self,
            process_identifier: Option<i32>,
            subject: &str,
            document_url: Option<&str>,
        ) -> Option<WindowId> {
            let pid = process_identifier?;
            let snapshot = match crate::window_snapshot::snapshot_windows() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("EVO-CAPTURE window snapshot failed: {e}");
                    return None;
                }
            };
            let focused_idx = select_focused(&snapshot, pid);
            let at_ms = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let observed: Vec<ObservedWindow> = snapshot
                .into_iter()
                .enumerate()
                .filter_map(|(idx, w)| {
                    ObservedWindow::new(
                        w.platform_key,
                        w.app_name,
                        // The focused window's title is the AX title
                        // (authoritative); other windows keep the
                        // snapshot title.
                        if Some(idx) == focused_idx {
                            subject.to_string()
                        } else {
                            w.title
                        },
                        // URL only for the focused window, from the AX
                        // document locator. Per-window URL tracking for
                        // background windows is a future enhancement;
                        // None is honest.
                        if Some(idx) == focused_idx {
                            document_url.map(str::to_string)
                        } else {
                            None
                        },
                        Some(idx) == focused_idx,
                    )
                })
                .collect();
            let _diff = self.registry.observe_snapshot(at_ms, observed);
            self.registry.focused()
        }

        unsafe fn emit(&mut self, signal: MacOSSignal) {
            (self.callback)(signal);
        }
    }

    unsafe fn create_observer(
        callback: *mut SignalSinkHolder,
    ) -> Result<Id, MacOSEventSourceError> {
        unsafe {
            let cls = observer_class()?;
            let alloc = alloc_sel();
            let init = init_sel();
            let observer = msg_send_id(cls as Id, alloc);
            if observer.is_null() {
                return Err(MacOSEventSourceError::RuntimeSetupFailed("alloc observer"));
            }
            let observer = msg_send_id(observer, init);
            if observer.is_null() {
                return Err(MacOSEventSourceError::RuntimeSetupFailed("init observer"));
            }

            let ivar_name = CALLBACK_IVAR_NAME.as_ptr() as *const c_char;
            let _ = object_setInstanceVariable(observer, ivar_name, callback as *mut c_void);
            Ok(observer)
        }
    }

    unsafe fn observer_class() -> Result<Class, MacOSEventSourceError> {
        unsafe {
            static mut OBSERVER_CLASS: usize = 0;

            if OBSERVER_CLASS != 0 {
                return Ok(OBSERVER_CLASS as Class);
            }

            let class_name = OBSERVER_CLASS_NAME.as_ptr() as *const c_char;
            let existing = objc_getClass(class_name);
            if !existing.is_null() {
                OBSERVER_CLASS = existing as usize;
                return Ok(existing);
            }

            let superclass = objc_getClass(class_name_nsobject().as_ptr());
            if superclass.is_null() {
                return Err(MacOSEventSourceError::RuntimeSetupFailed(
                    "NSObject class lookup",
                ));
            }

            let cls = objc_allocateClassPair(superclass, class_name, 0);
            if cls.is_null() {
                return Err(MacOSEventSourceError::RuntimeSetupFailed(
                    "allocate observer class",
                ));
            }

            if class_addIvar(
                cls,
                CALLBACK_IVAR_NAME.as_ptr() as *const c_char,
                std::mem::size_of::<*mut c_void>(),
                SPACEWORK_DIMENSION,
                POINTER_TYPE.as_ptr() as *const c_char,
            ) == 0
            {
                return Err(MacOSEventSourceError::RuntimeSetupFailed(
                    "add callback ivar",
                ));
            }

            if class_addMethod(
                cls,
                activate_selector(),
                workspace_did_activate as IMP,
                VOID_AT_AT_COLON_AT_TYPES.as_ptr() as *const c_char,
            ) == 0
            {
                return Err(MacOSEventSourceError::RuntimeSetupFailed(
                    "add workspaceDidActivate: method",
                ));
            }

            objc_registerClassPair(cls);
            OBSERVER_CLASS = cls as usize;
            Ok(cls)
        }
    }

    extern "C" fn workspace_did_activate(this: Id, _cmd: Sel, notification: Id) {
        unsafe {
            let mut callback_ptr: *mut c_void = ptr::null_mut();
            let _ = object_getInstanceVariable(
                this,
                CALLBACK_IVAR_NAME.as_ptr() as *const c_char,
                &mut callback_ptr,
            );

            if callback_ptr.is_null() {
                return;
            }

            let holder = &mut *(callback_ptr as *mut SignalSinkHolder);
            match pid_from_workspace_notification(notification) {
                Some(pid) => {
                    eprintln!("EVO-CAPTURE app activated: pid {pid}");
                    let _ = holder.attach_to_pid(pid);
                }
                None => eprintln!("EVO-CAPTURE app activation notification without pid"),
            }
        }
    }

    extern "C" fn focus_changed_callback(
        _observer: AXObserverRef,
        element: AXUIElementRef,
        _notification: CFStringRef,
        refcon: *mut c_void,
    ) {
        unsafe {
            if refcon.is_null() {
                return;
            }

            let holder = &mut *(refcon as *mut SignalSinkHolder);
            holder.emit_current_subject(element, false);
        }
    }

    /// Reads only generic Accessibility attributes directly exposed for the
    /// focused occurrence. Title, document, and native window id come from the
    /// same retained AX window, not from separately sampled focus values.
    /// Failed or malformed reads remain absent, never inferred from old state.
    unsafe fn focused_window_state(
        app: AXUIElementRef,
    ) -> Option<(String, ObservedState, Option<String>, Option<i32>)> {
        unsafe {
            let focused_window = copy_attribute(app, ax_focused_window_attribute())?;
            if CFGetTypeID(focused_window) != AXUIElementGetTypeID() {
                CFRelease(focused_window);
                return None;
            }
            let window = focused_window as AXUIElementRef;
            let subject = copy_attribute(window, ax_title_attribute()).and_then(|value| {
                let title = cfstring_value(value);
                CFRelease(value);
                title.filter(|title| !title.trim().is_empty())
            });
            let Some(subject) = subject else {
                CFRelease(focused_window);
                return None;
            };
            let window_identifier =
                copy_attribute(window, ax_identifier_attribute()).and_then(|value| {
                    let identifier = cfstring_value(value);
                    CFRelease(value);
                    identifier.filter(|identifier| !identifier.trim().is_empty())
                });
            let mut pid = 0;
            let window_pid =
                (AXUIElementGetPid(window, &mut pid) == AX_SUCCESS && pid > 0).then_some(pid);
            let focused_element = copy_attribute(app, ax_focused_ui_element_attribute());
            let mut state = ObservedState::new();

            if let Some(value) = copy_attribute(window, ax_document_attribute()) {
                if let Some(document) = cfstring_value(value) {
                    state = state.with_document_locator(document);
                }
                CFRelease(value);
            }

            if let Some(element) = focused_element
                && CFGetTypeID(element) == AXUIElementGetTypeID()
            {
                let element = element as AXUIElementRef;
                if let Some(value) = copy_attribute(element, ax_role_attribute()) {
                    if let Some(role) = cfstring_value(value) {
                        state = state.with_focused_role(role);
                    }
                    CFRelease(value);
                }
                if let Some(value) = copy_attribute(element, ax_identifier_attribute()) {
                    if let Some(identifier) = cfstring_value(value) {
                        state = state.with_focused_identifier(identifier);
                    }
                    CFRelease(value);
                }
                if let Some(value) = copy_attribute(element, ax_selected_text_range_attribute()) {
                    if let Some((start, length)) = ax_range_value(value) {
                        state = state.with_selection(start, length);
                    }
                    CFRelease(value);
                }
                if let Some(value) =
                    copy_attribute(element, ax_insertion_point_line_number_attribute())
                {
                    if let Some(line) = cfnumber_u64(value) {
                        state = state.with_insertion_line(line);
                    }
                    CFRelease(value);
                }
            }

            if let Some(value) = focused_element {
                CFRelease(value);
            }
            CFRelease(focused_window);
            Some((subject, state, window_identifier, window_pid))
        }
    }

    fn new_capture_session() -> String {
        static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);
        let epoch_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        format!(
            "capture-{}-{}-{}",
            std::process::id(),
            epoch_nanos,
            NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
        )
    }

    fn captured_surface_identity(
        capture_session: &str,
        window_id: Option<WindowId>,
        process_id: Option<i32>,
        process_started_at: Option<String>,
        window_identifier: Option<String>,
    ) -> Option<SurfaceIdentity> {
        let identity = SurfaceIdentity {
            capture_session: capture_session.into(),
            window_id: window_id?.to_string(),
            process_id: process_id?,
            process_started_at: process_started_at?,
            window_identifier,
            // Generic AX does not witness native browser profile/tab ids.
            browser_window: None,
            browser_profile: None,
            browser_session: None,
            browser_tab: None,
        };
        identity.is_valid().then_some(identity)
    }

    /// Reads the real process launch date. The class method has a pid_t/i32
    /// argument; NSDate's epoch selector returns double/f64, not an object.
    unsafe fn process_started_at(pid: i32) -> Option<String> {
        unsafe {
            if pid <= 0 {
                return None;
            }
            let pool_class = class_id(c"NSAutoreleasePool");
            let application_class = class_id(c"NSRunningApplication");
            if pool_class.is_null() || application_class.is_null() {
                return None;
            }
            let pool = msg_send_id(msg_send_id(pool_class as Id, alloc_sel()), init_sel());
            if pool.is_null() {
                return None;
            }
            let token = (|| {
                // Both application and launchDate are borrowed/autoreleased
                // Objective-C objects. Neither is a CF Copy/Create result;
                // releasing them with CFRelease would over-release them.
                let application = msg_send_id_i32(
                    application_class as Id,
                    sel_from_bytes(RUNNING_APPLICATION_WITH_PROCESS_IDENTIFIER_SEL),
                    pid,
                );
                if application.is_null() {
                    return None;
                }
                let launch_date = msg_send_id(application, sel_from_bytes(LAUNCH_DATE_SEL));
                if launch_date.is_null() {
                    return None;
                }
                process_start_token(msg_send_f64(
                    launch_date,
                    sel_from_bytes(TIME_INTERVAL_SINCE_1970_SEL),
                ))
            })();
            // Release only the alloc/init-owned pool, after copying the token.
            msg_send_void0(pool, release_sel());
            token
        }
    }

    fn process_start_token(epoch: f64) -> Option<String> {
        (epoch.is_finite() && epoch > 0.0).then(|| format!("{:016x}", epoch.to_bits()))
    }

    #[cfg(test)]
    mod surface_tests {
        use super::*;
        use crate::CaptureEngine;
        use crate::adapters::macos::MacOSAdapter;
        use evo_observation::observation_schema::ObservationSchema;
        use evo_observation::provenance::ObservationSource;

        fn window_id() -> WindowId {
            "window-1".parse().unwrap()
        }

        fn identity(session: &str, pid: i32, native_id: Option<&str>) -> SurfaceIdentity {
            captured_surface_identity(
                session,
                Some(window_id()),
                Some(pid),
                process_start_token(1_791_000_000.5),
                native_id.map(str::to_string),
            )
            .unwrap()
        }

        fn focus_key(identity: SurfaceIdentity) -> FocusKey {
            FocusKey {
                process_identifier: Some(identity.process_id),
                subject: "Duplicate title".into(),
                window_id: Some(identity.window_id.parse().unwrap()),
                surface_identity: Some(identity),
            }
        }

        #[test]
        fn surface_capture_sessions_namespace_registry_ids() {
            let first_session = new_capture_session();
            let second_session = new_capture_session();
            assert_ne!(first_session, second_session);
            assert!(first_session.starts_with(&format!("capture-{}-", std::process::id())));
            let first = identity(&first_session, 123, Some("ax-window"));
            let second = identity(&second_session, 123, Some("ax-window"));
            assert_eq!(first.window_id, second.window_id);
            assert_ne!(first, second);
        }

        #[test]
        fn surface_start_token_preserves_real_epoch_bits_and_rejects_invalid_epochs() {
            let epoch = 1_791_000_000.123456f64;
            let token = process_start_token(epoch).unwrap();
            assert_eq!(token.len(), 16);
            assert_eq!(u64::from_str_radix(&token, 16).unwrap(), epoch.to_bits());
            assert!(
                token
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            );
            for invalid in [
                0.0f64,
                -0.0,
                -1.0,
                f64::NAN,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ] {
                assert_eq!(process_start_token(invalid), None);
            }
        }

        #[test]
        fn surface_capture_without_native_binding_does_not_invent_one() {
            let identity = identity("capture-session", 123, None);
            assert_eq!(identity.window_identifier, None);
            assert_eq!(identity.browser_profile, None);
            assert_eq!(identity.browser_tab, None);
            let mut state = ObservedState::new()
                .with_window_title("Duplicate title")
                .with_document_locator("https://example.test/same")
                .with_focused_identifier("editor-control")
                .with_surface_identity(identity.clone());
            assert_eq!(state.surface_identity(), Some(&identity));
            state = state
                .with_document_locator("https://example.test/changed")
                .with_window_title("Changed title");
            assert_eq!(state.surface_identity(), Some(&identity));
        }

        #[test]
        fn surface_capture_requires_all_provenance_and_never_drops_invalid_native_tokens() {
            let start = process_start_token(1_791_000_000.5);
            assert_eq!(
                captured_surface_identity("session", None, Some(123), start.clone(), None),
                None
            );
            assert_eq!(
                captured_surface_identity("session", Some(window_id()), None, start.clone(), None),
                None
            );
            assert_eq!(
                captured_surface_identity("session", Some(window_id()), Some(123), None, None),
                None
            );
            assert_eq!(
                captured_surface_identity(
                    "session",
                    Some(window_id()),
                    Some(123),
                    Some("invalid".into()),
                    None
                ),
                None
            );
            assert_eq!(
                captured_surface_identity(
                    "session",
                    Some(window_id()),
                    Some(123),
                    start,
                    Some("x".repeat(4097))
                ),
                None
            );
        }

        #[test]
        fn surface_dedup_distinguishes_two_processes_restarts_sessions_and_native_windows() {
            let original = identity("session-a", 123, Some("ax-window-a"));
            let original_key = focus_key(original.clone());
            assert_eq!(original_key, focus_key(original.clone()));
            assert_ne!(
                original_key,
                focus_key(identity("session-a", 124, Some("ax-window-a")))
            );
            assert_ne!(
                original_key,
                focus_key(identity("session-b", 123, Some("ax-window-a")))
            );
            assert_ne!(
                original_key,
                focus_key(identity("session-a", 123, Some("ax-window-b")))
            );
            let mut restarted = original.clone();
            restarted.process_started_at = process_start_token(1_791_000_001.5).unwrap();
            assert_ne!(original_key, focus_key(restarted));
            let mut another_window = original;
            another_window.window_id = "window-2".into();
            assert_ne!(original_key, focus_key(another_window));
        }

        #[test]
        fn surface_capture_uses_registry_ids_for_duplicate_titles_and_preserves_context_in_v3() {
            let mut registry = WindowRegistry::new();
            let make_windows = |second_focused: bool| {
                vec![
                    ObservedWindow::new(
                        "platform-a",
                        "Same app",
                        "Duplicate title",
                        Some("https://example.test/same".into()),
                        !second_focused,
                    )
                    .unwrap(),
                    ObservedWindow::new(
                        "platform-b",
                        "Same app",
                        "Duplicate title",
                        Some("https://example.test/same".into()),
                        second_focused,
                    )
                    .unwrap(),
                ]
            };
            registry.observe_snapshot(1, make_windows(false));
            let first_id = registry.focused().unwrap();
            registry.observe_snapshot(2, make_windows(true));
            let second_id = registry.focused().unwrap();
            assert_ne!(first_id, second_id);
            let adapter = MacOSAdapter::new(ObservationSource::new("surface-test").unwrap());
            for id in [first_id, second_id] {
                let identity = captured_surface_identity(
                    "session",
                    Some(id),
                    Some(123),
                    process_start_token(1_791_000_000.5),
                    None,
                )
                .unwrap();
                let state = ObservedState::new()
                    .with_window_title("Duplicate title")
                    .with_surface_identity(identity.clone());
                let raw = adapter
                    .normalize(MacOSSignal::WindowFocusGained {
                        subject: "Duplicate title".into(),
                        observed_at: SystemTime::UNIX_EPOCH,
                        process_identifier: Some(123),
                        owning_process_name: Some("Same app".into()),
                        observed_state: Some(state.clone()),
                        window_id: Some(id),
                    })
                    .unwrap()
                    .unwrap();
                assert_eq!(raw.schema(), ObservationSchema::window_focus_gained_v3());
                assert_eq!(ObservedState::from_context(raw.context()), Some(state));
                assert_eq!(
                    SurfaceIdentity::from_context(raw.context()),
                    Some(identity.clone())
                );
                assert_eq!(raw.context().get("window_id"), Some(&id.to_string()));
                let observation = CaptureEngine::new()
                    .ingest(raw, &ObservationSchema::window_focus_gained_v3())
                    .unwrap();
                assert_eq!(
                    SurfaceIdentity::from_context(observation.provenance().context()),
                    Some(identity)
                );
            }
        }
    }

    unsafe fn copy_attribute(element: AXUIElementRef, attribute: CFStringRef) -> Option<CFTypeRef> {
        unsafe {
            let mut value: CFTypeRef = ptr::null();
            (AXUIElementCopyAttributeValue(element, attribute, &mut value) == AX_SUCCESS
                && !value.is_null())
            .then_some(value)
        }
    }

    unsafe fn cfstring_value(value: CFTypeRef) -> Option<String> {
        unsafe {
            (CFGetTypeID(value) == CFStringGetTypeID())
                .then(|| cfstring_to_string(value as CFStringRef))
                .flatten()
        }
    }

    unsafe fn cfnumber_u64(value: CFTypeRef) -> Option<u64> {
        unsafe {
            if CFGetTypeID(value) != CFNumberGetTypeID() {
                return None;
            }
            let mut number = 0i64;
            if CFNumberGetValue(
                value,
                CF_NUMBER_SINT64_TYPE,
                &mut number as *mut i64 as *mut c_void,
            ) == 0
            {
                return None;
            }
            u64::try_from(number).ok()
        }
    }

    unsafe fn ax_range_value(value: CFTypeRef) -> Option<(u64, u64)> {
        unsafe {
            let value = value as AXValueRef;
            if AXValueGetType(value) != AX_VALUE_CF_RANGE_TYPE {
                return None;
            }
            let mut range = CFRange {
                location: 0,
                length: 0,
            };
            if AXValueGetValue(
                value,
                AX_VALUE_CF_RANGE_TYPE,
                &mut range as *mut CFRange as *mut c_void,
            ) == 0
            {
                return None;
            }
            Some((
                u64::try_from(range.location).ok()?,
                u64::try_from(range.length).ok()?,
            ))
        }
    }

    unsafe fn pid_from_workspace_notification(notification: Id) -> Option<i32> {
        unsafe {
            let user_info = msg_send_id(notification, user_info_sel());
            if user_info.is_null() {
                return None;
            }

            let workspace_app =
                msg_send_id1(user_info, object_for_key_sel(), NSWorkspaceApplicationKey);
            if workspace_app.is_null() {
                return None;
            }

            let pid = msg_send_i32(workspace_app, process_identifier_sel());
            Some(pid)
        }
    }

    unsafe fn workspace_notification_center() -> Result<Id, MacOSEventSourceError> {
        unsafe {
            let workspace = msg_send_id(
                class_id(class_name_nsworkspace()) as Id,
                shared_workspace_sel(),
            );
            if workspace.is_null() {
                return Err(MacOSEventSourceError::RuntimeSetupFailed("sharedWorkspace"));
            }
            let center = msg_send_id(workspace, notification_center_sel());
            if center.is_null() {
                return Err(MacOSEventSourceError::RuntimeSetupFailed(
                    "notificationCenter",
                ));
            }
            Ok(center)
        }
    }

    unsafe fn frontmost_application_pid() -> Result<Option<i32>, MacOSEventSourceError> {
        unsafe {
            let workspace = msg_send_id(
                class_id(class_name_nsworkspace()) as Id,
                shared_workspace_sel(),
            );
            if workspace.is_null() {
                return Err(MacOSEventSourceError::RuntimeSetupFailed("sharedWorkspace"));
            }

            let application = msg_send_id(workspace, frontmost_application_sel());
            if application.is_null() {
                return Ok(None);
            }

            let pid = msg_send_i32(application, process_identifier_sel());
            Ok(Some(pid))
        }
    }

    unsafe fn cfstring_to_string(value: CFStringRef) -> Option<String> {
        unsafe {
            if value.is_null() {
                return None;
            }

            let length = CFStringGetLength(value);
            if length <= 0 {
                return Some(String::new());
            }

            let max_size = CFStringGetMaximumSizeForEncoding(length, CFSTRING_ENCODING_UTF8) + 1;
            if max_size <= 0 {
                return None;
            }

            let mut buffer = vec![0u8; max_size as usize];
            let ok = CFStringGetCString(
                value,
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len() as isize,
                CFSTRING_ENCODING_UTF8,
            );
            if ok == 0 {
                return None;
            }

            let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
            String::from_utf8(buffer[..end].to_vec()).ok()
        }
    }

    unsafe fn class_name_nsobject() -> &'static CStr {
        c"NSObject"
    }

    unsafe fn class_name_nsworkspace() -> &'static CStr {
        c"NSWorkspace"
    }

    unsafe fn class_name_nsdictionary() -> &'static CStr {
        c"NSDictionary"
    }

    unsafe fn class_name_nsnumber() -> &'static CStr {
        c"NSNumber"
    }

    unsafe fn class_id(name: &'static CStr) -> Class {
        unsafe { objc_getClass(name.as_ptr()) }
    }

    unsafe fn msg_send_id(receiver: Id, selector: Sel) -> Id {
        unsafe {
            let raw = objc_msgSend as *const ();
            let function: extern "C" fn(Id, Sel) -> Id = std::mem::transmute(raw);
            function(receiver, selector)
        }
    }

    unsafe fn msg_send_id1(receiver: Id, selector: Sel, arg: Id) -> Id {
        unsafe {
            let raw = objc_msgSend as *const ();
            let function: extern "C" fn(Id, Sel, Id) -> Id = std::mem::transmute(raw);
            function(receiver, selector, arg)
        }
    }

    unsafe fn msg_send_id2(receiver: Id, selector: Sel, arg1: Id, arg2: Id) -> Id {
        unsafe {
            let raw = objc_msgSend as *const ();
            let function: extern "C" fn(Id, Sel, Id, Id) -> Id = std::mem::transmute(raw);
            function(receiver, selector, arg1, arg2)
        }
    }

    unsafe fn msg_send_void3(receiver: Id, selector: Sel, arg1: Id, arg2: Id, arg3: Id) {
        unsafe {
            let raw = objc_msgSend as *const ();
            let function: extern "C" fn(Id, Sel, Id, Id, Id) = std::mem::transmute(raw);
            function(receiver, selector, arg1, arg2, arg3)
        }
    }

    unsafe fn msg_send_void4(receiver: Id, selector: Sel, arg1: Id, arg2: Sel, arg3: Id, arg4: Id) {
        unsafe {
            let raw = objc_msgSend as *const ();
            let function: extern "C" fn(Id, Sel, Id, Sel, Id, Id) = std::mem::transmute(raw);
            function(receiver, selector, arg1, arg2, arg3, arg4)
        }
    }

    unsafe fn msg_send_void0(receiver: Id, selector: Sel) {
        unsafe {
            let raw = objc_msgSend as *const ();
            let function: extern "C" fn(Id, Sel) = std::mem::transmute(raw);
            function(receiver, selector)
        }
    }

    unsafe fn msg_send_i32(receiver: Id, selector: Sel) -> i32 {
        unsafe {
            let raw = objc_msgSend as *const ();
            let function: extern "C" fn(Id, Sel) -> i32 = std::mem::transmute(raw);
            function(receiver, selector)
        }
    }

    unsafe fn msg_send_id_i32(receiver: Id, selector: Sel, pid: i32) -> Id {
        unsafe {
            let function: extern "C" fn(Id, Sel, i32) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector, pid)
        }
    }

    unsafe fn msg_send_f64(receiver: Id, selector: Sel) -> f64 {
        unsafe {
            let function: extern "C" fn(Id, Sel) -> f64 =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector)
        }
    }

    pub(super) fn accessibility_permission_granted() -> bool {
        unsafe { AXIsProcessTrusted() != 0 }
    }

    pub(super) fn prompt_for_accessibility_permission() {
        unsafe {
            let prompt_key = create_cfstring(AX_TRUSTED_CHECK_OPTION_PROMPT);
            if prompt_key.is_null() {
                return;
            }
            let yes = msg_send_id1(
                class_id(class_name_nsnumber()) as Id,
                number_with_bool_sel(),
                true as i8 as Id,
            );
            let dict = msg_send_id2(
                class_id(class_name_nsdictionary()) as Id,
                dictionary_with_object_for_key_sel(),
                yes,
                prompt_key as Id,
            );
            // The prompt option asks macOS to surface the System Settings
            // Accessibility grant. The result is not treated as a grant: the
            // caller still checks AXIsProcessTrusted afterwards.
            let _ = AXIsProcessTrustedWithOptions(dict as CFTypeRef);
            CFRelease(prompt_key as CFTypeRef);
        }
    }

    pub(super) fn desktop_shell_pid() -> Option<i32> {
        // The shell declares itself either way: directly when it spawns the
        // daemon (env), or through the storage-root pid file when launchd
        // owns the daemon and no environment is inherited. Both are the
        // same identity-based self-reference boundary.
        if let Some(pid) = std::env::var("EVO_DESKTOP_PID")
            .ok()
            .and_then(|value| value.trim().parse::<i32>().ok())
            .filter(|pid| *pid > 0)
        {
            return Some(pid);
        }
        let Ok(home) = std::env::var("HOME") else {
            return None;
        };
        let storage_root =
            std::path::Path::new(&home).join("Library/Application Support/evo/storage");
        pid_from_storage_root(&storage_root)
    }

    /// The pid-file read, over an explicit storage root (testable without
    /// the machine's live state).
    pub(crate) fn pid_from_storage_root(storage_root: &std::path::Path) -> Option<i32> {
        let Ok(contents) = std::fs::read_to_string(storage_root.join("desktop.pid")) else {
            return None;
        };
        contents
            .trim()
            .parse::<i32>()
            .ok()
            .filter(|pid| *pid > 0)
            .filter(|pid| {
                // A stale pid file (crashed shell) must not exclude a live
                // unrelated process: only a still-running pid counts.
                std::process::Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false)
            })
    }

    fn shared_workspace_sel() -> Sel {
        sel_from_bytes(SHARED_WORKSPACE_SEL)
    }

    fn dictionary_with_object_for_key_sel() -> Sel {
        sel_from_bytes(DICTIONARY_WITH_OBJECT_FOR_KEY_SEL)
    }

    fn number_with_bool_sel() -> Sel {
        sel_from_bytes(NUMBER_WITH_BOOL_SEL)
    }

    fn frontmost_application_sel() -> Sel {
        sel_from_bytes(FRONTMOST_APPLICATION_SEL)
    }

    fn notification_center_sel() -> Sel {
        sel_from_bytes(NOTIFICATION_CENTER_SEL)
    }

    fn add_observer_sel() -> Sel {
        sel_from_bytes(ADD_OBSERVER_SEL)
    }

    fn remove_observer_sel() -> Sel {
        sel_from_bytes(REMOVE_OBSERVER_SEL)
    }

    fn user_info_sel() -> Sel {
        sel_from_bytes(USER_INFO_SEL)
    }

    fn object_for_key_sel() -> Sel {
        sel_from_bytes(OBJECT_FOR_KEY_SEL)
    }

    fn process_identifier_sel() -> Sel {
        sel_from_bytes(PROCESS_IDENTIFIER_SEL)
    }

    fn alloc_sel() -> Sel {
        sel_from_bytes(ALLOC_SEL)
    }

    fn init_sel() -> Sel {
        sel_from_bytes(INIT_SEL)
    }

    fn release_sel() -> Sel {
        sel_from_bytes(RELEASE_SEL)
    }

    fn activate_selector() -> Sel {
        sel_from_bytes(WORKSPACE_DID_ACTIVATE_SEL)
    }

    fn sel_from_bytes(bytes: &'static [u8]) -> Sel {
        unsafe { sel_registerName(bytes.as_ptr() as *const c_char) }
    }

    fn ax_focused_window_changed_notification() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_FOCUSED_WINDOW_CHANGED_NOTIFICATION) as usize)
            as CFStringRef
    }

    fn ax_focused_window_attribute() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_FOCUSED_WINDOW_ATTRIBUTE) as usize) as CFStringRef
    }

    fn ax_title_attribute() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_TITLE_ATTRIBUTE) as usize) as CFStringRef
    }

    fn ax_focused_ui_element_attribute() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_FOCUSED_UI_ELEMENT_ATTRIBUTE) as usize)
            as CFStringRef
    }

    fn ax_document_attribute() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_DOCUMENT_ATTRIBUTE) as usize) as CFStringRef
    }

    fn ax_role_attribute() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_ROLE_ATTRIBUTE) as usize) as CFStringRef
    }

    fn ax_identifier_attribute() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_IDENTIFIER_ATTRIBUTE) as usize) as CFStringRef
    }

    fn ax_selected_text_range_attribute() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_SELECTED_TEXT_RANGE_ATTRIBUTE) as usize)
            as CFStringRef
    }

    fn ax_insertion_point_line_number_attribute() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_INSERTION_POINT_LINE_NUMBER_ATTRIBUTE) as usize)
            as CFStringRef
    }

    fn create_cfstring(bytes: &'static [u8]) -> CFStringRef {
        unsafe {
            CFStringCreateWithCString(
                ptr::null(),
                bytes.as_ptr() as *const c_char,
                CFSTRING_ENCODING_UTF8,
            )
        }
    }

    #[link(name = "objc")]
    unsafe extern "C" {
        fn sel_registerName(name: *const c_char) -> Sel;
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::MacOSSignal;

    use std::fmt;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MacOSEventSourceError {
        UnsupportedPlatform,
    }

    impl fmt::Display for MacOSEventSourceError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "macOS event source is only available on macOS")
        }
    }

    impl std::error::Error for MacOSEventSourceError {}

    pub(super) fn accessibility_permission_granted() -> bool {
        false
    }

    pub(super) fn prompt_for_accessibility_permission() {}

    pub(super) fn desktop_shell_pid() -> Option<i32> {
        // Same contract as the macOS side: env declaration first, then the
        // storage-root pid file — the reads are platform-neutral.
        if let Some(pid) = std::env::var("EVO_DESKTOP_PID")
            .ok()
            .and_then(|value| value.trim().parse::<i32>().ok())
            .filter(|pid| *pid > 0)
        {
            return Some(pid);
        }
        let home = std::env::var("HOME").ok()?;
        let storage_root =
            std::path::Path::new(&home).join("Library/Application Support/evo/storage");
        pid_from_storage_root(&storage_root)
    }

    /// The pid-file read, over an explicit storage root (testable without
    /// the machine's live state); liveness is checked with `kill -0`,
    /// which is portable, exactly as on the macOS side.
    pub(crate) fn pid_from_storage_root(storage_root: &std::path::Path) -> Option<i32> {
        let contents = std::fs::read_to_string(storage_root.join("desktop.pid")).ok()?;
        contents
            .trim()
            .parse::<i32>()
            .ok()
            .filter(|pid| *pid > 0)
            .filter(|pid| {
                std::process::Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false)
            })
    }

    pub struct MacOSEventSource;

    impl MacOSEventSource {
        pub fn new<F>(
            _desktop_shell: Option<i32>,
            _callback: F,
        ) -> Result<Self, MacOSEventSourceError>
        where
            F: FnMut(MacOSSignal) + 'static,
        {
            Err(MacOSEventSourceError::UnsupportedPlatform)
        }

        pub fn new_with_browser_identity_provider<P, F>(
            _desktop_shell: Option<i32>,
            _browser_identity_provider: P,
            _callback: F,
        ) -> Result<Self, MacOSEventSourceError>
        where
            P: super::BrowserIdentityProvider + 'static,
            F: FnMut(MacOSSignal) + 'static,
        {
            Err(MacOSEventSourceError::UnsupportedPlatform)
        }
    }
}

pub use imp::{MacOSEventSource, MacOSEventSourceError};
