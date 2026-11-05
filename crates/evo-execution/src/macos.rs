//! macOS Restoration Executor.
//!
//! Performs the OS actions of a Restoration Plan on macOS:
//!
//! - URL targets → `NSWorkspace openURL:` (default application);
//! - file targets → existence check then `NSWorkspace openFile:`;
//! - window-title targets → the currently open window with that exact title
//!   is raised and its application activated.
//!
//! # Honesty rules
//!
//! - The executor NEVER guesses which application owns a title: it queries
//!   the live window list at execution time and acts only on an exact,
//!   unique title match. Zero matches → [`TargetStatus::Unavailable`];
//!   multiple matches → [`TargetStatus::Ambiguous`].
//! - The executor NEVER substitutes a different resource.
//! - OS state is inspected only while executing (IS-0021 §7 permits this);
//!   it never becomes an input to Restoration Derivation.

use crate::engine::{PlatformExecutor, TargetStatus};
use crate::locator::{LocatorKind, classify_subject};
use crate::preflight::PreflightSource;
use crate::restore::{DispatchResult, VerificationResult};
use evo_observation::surface_identity::SurfaceIdentity;

/// Adapter handles must retain the exact AX element through dispatch and
/// verification. An adapter must never resolve a handle again by title.
pub trait IdentityAdapter {
    type Window;
    fn process_started_at(&self, pid: i32) -> Result<String, String>;
    fn windows(&self, pid: i32) -> Result<Vec<Self::Window>, String>;
    fn identifier(&self, window: &Self::Window) -> Result<Option<String>, String>;
    /// Directly read the requested kind's locator/title from this retained
    /// element. Missing evidence must remain absent, never be inferred from
    /// the window's other attributes or the requested resource.
    fn resource(&self, window: &Self::Window, kind: &LocatorKind)
    -> Result<Option<String>, String>;
    /// Retains the globally focused window, checking the focused app's PID.
    /// Identifier and resource postconditions must be read from this same handle.
    fn focused_window(&self, pid: i32) -> Result<Self::Window, String>;
    fn raise(&self, window: &Self::Window) -> Result<(), String>;
    fn activate(&self, pid: i32) -> Result<(), String>;
    /// Must check that `pid` is the globally focused application, then return
    /// its focused AX window's identifier; app-local focus alone is insufficient.
    fn focused_window_identifier(&self, pid: i32) -> Result<Option<String>, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityAttempt {
    pub dispatch: DispatchResult,
    pub verification: VerificationResult,
}

impl IdentityAttempt {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            dispatch: DispatchResult::NotDispatched {
                reason: reason.into(),
            },
            verification: VerificationResult::Unverified {
                reason: "no identity action was dispatched".into(),
            },
        }
    }
}

fn required_native_identifier(surface: &SurfaceIdentity) -> Result<&str, String> {
    if !surface.is_valid() {
        return Err("invalid witnessed surface identity".into());
    }
    // These are constraints, even when an adapter cannot interpret them.
    // They are never dropped to allow a window-only or URL action.
    if surface.browser_profile.is_some() || surface.browser_tab.is_some() {
        return Err(
            "browser profile/tab identity adapter unavailable; exact constraints cannot be honored"
                .into(),
        );
    }
    surface
        .window_identifier
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| "native window identity unavailable: no witnessed AXIdentifier".into())
}

fn unique_identifier_index(
    identifiers: &[Option<String>],
    identifier: &str,
) -> Result<usize, String> {
    if identifiers
        .iter()
        .any(|id| id.as_deref().is_none_or(|id| id.trim().is_empty()))
    {
        return Err(
            "cannot inspect every candidate AXIdentifier; exact identity is unavailable".into(),
        );
    }
    let matches: Vec<_> = identifiers
        .iter()
        .enumerate()
        .filter(|(_, id)| id.as_deref() == Some(identifier))
        .map(|(index, _)| index)
        .collect();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err("no live window has the witnessed AXIdentifier".into()),
        _ => Err(format!(
            "{} windows have the witnessed AXIdentifier; exact identity is ambiguous",
            matches.len()
        )),
    }
}

fn check_process(adapter: &impl IdentityAdapter, surface: &SurfaceIdentity) -> Result<(), String> {
    let live = adapter.process_started_at(surface.process_id)?;
    if live != surface.process_started_at {
        return Err(
            "process lifetime changed; witnessed PID may have been reused or restarted".into(),
        );
    }
    Ok(())
}

/// Window-only identity seam. This verifies focus, not restoration of a
/// particular document/tab. Production constrained resource restores use
/// [`raise_surface_resource_with_adapter`] instead.
///
/// A capture session namespaces provenance; it does not prevent an application
/// from reusing AXIdentifier after a window closes within the same process.
/// Process lifetime and observed identifiers cannot prove window lifetime.
pub fn raise_surface_with_adapter(
    surface: &SurfaceIdentity,
    adapter: &impl IdentityAdapter,
) -> IdentityAttempt {
    raise_surface_impl(surface, None, adapter)
}

/// Requires the requested resource on the identified window before dispatch,
/// and on the globally focused retained window afterward. Resource matching
/// never selects another window or widens into URL/file/title reopening.
pub fn raise_surface_resource_with_adapter(
    surface: &SurfaceIdentity,
    resource: &str,
    adapter: &impl IdentityAdapter,
) -> IdentityAttempt {
    raise_surface_impl(surface, Some(resource), adapter)
}

fn check_resource<A: IdentityAdapter>(
    adapter: &A,
    window: &A::Window,
    kind: &LocatorKind,
) -> Result<(), String> {
    let witnessed = adapter
        .resource(window, kind)?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "required native resource locator/title is unavailable".to_string())?;
    let matches = match kind {
        // Only the existing raw file URI/absolute path classification is
        // equivalent. No canonicalization, title resemblance, or browser
        // profile/tab inference is an acceptable locator witness.
        LocatorKind::FilePath(path) => {
            classify_subject(&witnessed) == LocatorKind::FilePath(path.clone())
        }
        LocatorKind::Url(url) => witnessed == *url,
        LocatorKind::WindowTitle(title) => witnessed == *title && !title.trim().is_empty(),
    };
    if matches {
        Ok(())
    } else {
        Err("identified window does not show the requested exact resource".into())
    }
}

fn raise_surface_impl(
    surface: &SurfaceIdentity,
    resource: Option<&str>,
    adapter: &impl IdentityAdapter,
) -> IdentityAttempt {
    let kind = resource.map(classify_subject);
    let prepared = (|| -> Result<_, String> {
        let identifier = required_native_identifier(surface)?;
        check_process(adapter, surface)?;
        let windows = adapter.windows(surface.process_id)?;
        let identifiers = windows
            .iter()
            .map(|window| adapter.identifier(window))
            .collect::<Result<Vec<_>, _>>()?;
        let index = unique_identifier_index(&identifiers, identifier)?;
        // The selected handle stays retained. Refuse mutation, rather than
        // searching for a replacement, immediately before dispatch.
        if adapter.identifier(&windows[index])?.as_deref() != Some(identifier) {
            return Err("selected AX window identity changed before dispatch".into());
        }
        if let Some(kind) = &kind {
            check_resource(adapter, &windows[index], kind)?;
        }
        check_process(adapter, surface)?;
        Ok((windows, index, identifier))
    })();
    let (windows, index, identifier) = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => return IdentityAttempt::unavailable(reason),
    };
    if let Err(reason) = adapter.raise(&windows[index]) {
        return IdentityAttempt {
            dispatch: DispatchResult::Failed { reason },
            verification: VerificationResult::Unverified {
                reason: "AXRaise was refused".into(),
            },
        };
    }
    let postcondition: Result<(), String> = (|| {
        // A restart after AXRaise must not activate a replacement process.
        check_process(adapter, surface)?;
        adapter.activate(surface.process_id)?;
        check_process(adapter, surface)?;
        let focused = if let Some(kind) = &kind {
            let window = adapter.focused_window(surface.process_id)?;
            let identifier = adapter.identifier(&window)?;
            check_resource(adapter, &window, kind)?;
            identifier
        } else {
            adapter.focused_window_identifier(surface.process_id)?
        };
        check_process(adapter, surface)?;
        if focused.as_deref() != Some(identifier) {
            return Err("focused AX window does not have the witnessed AXIdentifier".into());
        }
        Ok(())
    })();
    IdentityAttempt {
        dispatch: DispatchResult::RaiseAccepted,
        verification: match postcondition {
            Ok(()) => VerificationResult::Verified,
            Err(reason) => VerificationResult::Failed { reason },
        },
    }
}

// ── Window source abstraction (testable) ─────────────────────────────────────

/// One live window observed by the platform at execution time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    /// The window's exact title, as the OS reports it.
    pub title: String,
    /// The process identifier of the window's owning application.
    pub owner_pid: i32,
    /// The localized name of the owning application.
    pub owner_name: String,
}

/// The live window list at execution time.
///
/// Abstracted so the decision logic is testable without a real display.
pub trait WindowSource {
    /// Returns the currently open windows, or an error when the platform
    /// cannot witness them (e.g. missing Accessibility permission).
    fn windows(&self) -> Result<Vec<WindowInfo>, String>;
}

/// Why a window-title target could not be resolved uniquely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowSelectionError {
    /// No window with the exact title is currently open.
    NoMatch,
    /// More than one window with the exact title is currently open.
    MultipleMatches { count: usize },
}

/// Selects the unique window whose title exactly matches `title`.
///
/// Pure and deterministic given a window list. This is the entire selection
/// rule: exact title equality, uniqueness required. Nothing else — no
/// recency, no ordering, no focus history, no ranking.
pub fn select_unique_window<'a>(
    windows: &'a [WindowInfo],
    title: &str,
) -> Result<&'a WindowInfo, WindowSelectionError> {
    // An empty title has no identifying power and must never select a window.
    if title.trim().is_empty() {
        return Err(WindowSelectionError::NoMatch);
    }
    let matches: Vec<&WindowInfo> = windows
        .iter()
        .filter(|window| window.title == title)
        .collect();
    match matches.len() {
        0 => Err(WindowSelectionError::NoMatch),
        1 => Ok(matches[0]),
        count => Err(WindowSelectionError::MultipleMatches { count }),
    }
}

/// Last-moment check over *all* titles in the owning PID. An unreadable
/// element could be another match, so it cannot be silently skipped.
/// PID + title is still not a durable window identity: a replacement with
/// the same title, or a change after this check, cannot be distinguished.
#[cfg(any(target_os = "macos", test))]
fn unique_raise_index(titles: &[Option<String>], title: &str) -> Result<usize, String> {
    if title.trim().is_empty() {
        return Err("an empty title cannot identify a window to raise".to_string());
    }
    if titles.iter().any(Option::is_none) {
        return Err("cannot inspect every window title in the owning application".to_string());
    }
    let matches: Vec<_> = titles
        .iter()
        .enumerate()
        .filter(|(_, live)| live.as_deref() == Some(title))
        .map(|(index, _)| index)
        .collect();
    match matches.as_slice() {
        [] => Err(format!("the window “{title}” is no longer open")),
        [index] => Ok(*index),
        _ => Err(format!(
            "{} windows in the owning application carry the title “{title}”; cannot raise a unique window",
            matches.len()
        )),
    }
}

// ── macOS executor ───────────────────────────────────────────────────────────

/// The macOS Restoration Executor.
pub struct MacOSExecutor<S: WindowSource> {
    source: S,
}

impl MacOSExecutor<SystemWindowSource> {
    /// Constructs the executor over the live system window list.
    pub fn new() -> Self {
        Self {
            source: SystemWindowSource,
        }
    }
}

impl Default for MacOSExecutor<SystemWindowSource> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: WindowSource> MacOSExecutor<S> {
    /// Constructs the executor over an injected window source (for tests).
    pub fn with_source(source: S) -> Self {
        Self { source }
    }

    fn open_target(&self, kind: &str, value: &str) -> TargetStatus {
        match imp::open(kind, value) {
            Ok(()) => TargetStatus::Opened {
                detail: format!("opened {value}"),
            },
            Err(reason) => TargetStatus::Failed { reason },
        }
    }

    fn focus_window(&self, title: &str) -> TargetStatus {
        let windows = match self.source.windows() {
            Ok(windows) => windows,
            Err(reason) => {
                return TargetStatus::Unavailable {
                    reason: format!("cannot inspect open windows: {reason}"),
                };
            }
        };
        match select_unique_window(&windows, title) {
            Ok(window) => {
                let raise = imp::raise_and_activate(window);
                match raise {
                    Ok(()) => TargetStatus::Opened {
                        detail: format!(
                            "focused the open window “{}” (in {})",
                            window.title, window.owner_name
                        ),
                    },
                    Err(reason) => TargetStatus::Failed { reason },
                }
            }
            Err(WindowSelectionError::NoMatch) => TargetStatus::Unavailable {
                reason: format!(
                    "no window titled “{title}” is currently open; Evo does not guess \
                     which application owned it"
                ),
            },
            Err(WindowSelectionError::MultipleMatches { count }) => TargetStatus::Ambiguous {
                reason: format!(
                    "{count} open windows are titled “{title}”; Evo cannot choose between \
                     them without more canonical evidence"
                ),
            },
        }
    }
}

impl<S: WindowSource> PlatformExecutor for MacOSExecutor<S> {
    fn open_url(&self, url: &str) -> TargetStatus {
        self.open_target("url", url)
    }

    fn open_file(&self, path: &str) -> TargetStatus {
        // A missing file is an unavailable target: the resource is simply not
        // present. Evo never substitutes a different resource and never
        // claims the OS refused to open something that does not exist.
        if !imp::path_exists(path) {
            return TargetStatus::Unavailable {
                reason: format!(
                    "the file “{path}” no longer exists; Evo does not guess which resource \
                     to open"
                ),
            };
        }
        self.open_target("file", path)
    }

    fn focus_window(&self, title: &str) -> TargetStatus {
        self.focus_window(title)
    }

    /// Raises a window the binding layer already resolved uniquely.
    ///
    /// This deliberately does *not* go back through the title path: the
    /// binding may have identified this window by the file the OS reports it
    /// is displaying, and that window's title is not required to be unique
    /// across every running application. The native call does still require
    /// a unique title within its PID: `WindowInfo` has no durable AX handle.
    fn focus_bound_window(&self, window: &WindowInfo) -> TargetStatus {
        match imp::raise_and_activate(window) {
            Ok(()) => TargetStatus::Opened {
                detail: format!(
                    "focused the open window “{}” (in {})",
                    window.title, window.owner_name
                ),
            },
            Err(reason) => TargetStatus::Failed { reason },
        }
    }
}

impl<S: WindowSource> PreflightSource for MacOSExecutor<S> {
    fn file_exists(&self, path: &str) -> bool {
        imp::path_exists(path)
    }

    fn windows(&self) -> Result<Vec<WindowInfo>, String> {
        self.source.windows()
    }

    /// Asks the Accessibility API which live windows report this exact file as
    /// their `AXDocument`.
    ///
    /// This is the operating system's own account of which window is showing
    /// which file — not an inference from window titles, and not knowledge of
    /// any particular application. Applications that do not expose
    /// `AXDocument` simply never match, which degrades to opening the file: a
    /// missing answer is never read as "not open".
    fn windows_showing_document(&self, path: &str) -> Result<Vec<WindowInfo>, String> {
        imp::windows_showing_document(path)
    }
}

// ── System window source (live macOS window list via Accessibility) ──────────

/// Queries the live macOS window list.
///
/// Uses the Accessibility API (the same permission Evo requests for capture),
/// enumerating the windows of every running application and reading each
/// window's exact title.
#[derive(Debug, Clone, Copy, Default)]
#[cfg(target_os = "macos")]
pub struct SystemWindowSource;

#[cfg(target_os = "macos")]
impl WindowSource for SystemWindowSource {
    fn windows(&self) -> Result<Vec<WindowInfo>, String> {
        imp::system_windows()
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{
        IdentityAdapter, IdentityAttempt, LocatorKind, WindowInfo,
        raise_surface_resource_with_adapter, raise_surface_with_adapter, unique_raise_index,
    };
    use crate::restore::VerificationResult;
    use evo_observation::surface_identity::SurfaceIdentity;
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_void};
    use std::ptr;

    type Bool = i8;
    type Id = *mut Object;
    type Class = *mut objc_class;
    type Sel = *mut c_void;
    type CFTypeRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFArrayRef = *const c_void;
    type CFIndex = isize;
    type AXError = i32;
    type AXUIElementRef = *mut c_void;

    const AX_SUCCESS: AXError = 0;
    const AX_CANNOT_COMPLETE: AXError = -25204;
    const AX_ATTRIBUTE_UNSUPPORTED: AXError = -25205;
    const AX_NO_VALUE: AXError = -25212;
    const CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;

    #[repr(C)]
    pub struct Object {
        _priv: [u8; 0],
    }

    #[repr(C)]
    pub struct objc_class {
        _priv: [u8; 0],
    }

    #[allow(clippy::duplicated_attributes)]
    #[link(name = "objc")]
    #[link(name = "Foundation", kind = "framework")]
    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "ApplicationServices", kind = "framework")]
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Class;
        fn objc_msgSend();
        fn sel_registerName(name: *const c_char) -> Sel;

        fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut i32) -> AXError;
        fn AXUIElementGetTypeID() -> usize;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
        fn AXIsProcessTrusted() -> Bool;
        fn CFRelease(cf: CFTypeRef);
        fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
        fn CFGetTypeID(cf: CFTypeRef) -> usize;
        fn CFArrayGetTypeID() -> usize;
        fn CFStringGetTypeID() -> usize;
        fn CFURLGetTypeID() -> usize;
        fn CFURLGetString(url: CFTypeRef) -> CFStringRef;
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
        fn CFArrayGetCount(the_array: CFArrayRef) -> CFIndex;
        fn CFArrayGetValueAtIndex(the_array: CFArrayRef, index: CFIndex) -> *const c_void;
    }

    const KAX_WINDOWS_ATTRIBUTE: &[u8] = b"AXWindows\0";
    const KAX_TITLE_ATTRIBUTE: &[u8] = b"AXTitle\0";
    const KAX_DOCUMENT_ATTRIBUTE: &[u8] = b"AXDocument\0";
    const KAX_URL_ATTRIBUTE: &[u8] = b"AXURL\0";
    const KAX_IDENTIFIER_ATTRIBUTE: &[u8] = b"AXIdentifier\0";
    const KAX_FOCUSED_APPLICATION_ATTRIBUTE: &[u8] = b"AXFocusedApplication\0";
    const KAX_FOCUSED_WINDOW_ATTRIBUTE: &[u8] = b"AXFocusedWindow\0";
    const KAX_RAISE_ACTION: &[u8] = b"AXRaise\0";
    #[allow(dead_code)]
    const AX_TRUSTED_CHECK_OPTION_PROMPT: &[u8] = b"AXTrustedCheckOptionPrompt\0";

    const SHARED_WORKSPACE_SEL: &[u8] = b"sharedWorkspace\0";
    const FRONTMOST_APPLICATION_SEL: &[u8] = b"frontmostApplication\0";
    const OPEN_URL_SEL: &[u8] = b"openURL:\0";
    const OPEN_FILE_SEL: &[u8] = b"openFile:\0";
    const URL_WITH_STRING_SEL: &[u8] = b"URLWithString:\0";
    const RUNNING_APPLICATIONS_SEL: &[u8] = b"runningApplications\0";
    const PROCESS_IDENTIFIER_SEL: &[u8] = b"processIdentifier\0";
    const LOCALIZED_NAME_SEL: &[u8] = b"localizedName\0";
    const ACTIVATE_WITH_OPTIONS_SEL: &[u8] = b"activateWithOptions:\0";
    const OBJECT_AT_INDEX_SEL: &[u8] = b"objectAtIndex:\0";
    const COUNT_SEL: &[u8] = b"count\0";
    const RUNNING_APPLICATION_WITH_PID_SEL: &[u8] = b"runningApplicationWithProcessIdentifier:\0";
    const LAUNCH_DATE_SEL: &[u8] = b"launchDate\0";
    const TIME_INTERVAL_SINCE_1970_SEL: &[u8] = b"timeIntervalSince1970\0";

    /// `NSApplicationActivateIgnoringOtherApps`.
    const ACTIVATE_IGNORING_OTHER_APPS: usize = 1 << 1;

    // ── Public helpers used by the executor ──────────────────────────────

    struct NativeIdentityAdapter;

    impl IdentityAdapter for NativeIdentityAdapter {
        type Window = OwnedCf;

        fn process_started_at(&self, pid: i32) -> Result<String, String> {
            process_started_at(pid)
        }

        fn windows(&self, pid: i32) -> Result<Vec<OwnedCf>, String> {
            unsafe {
                require_accessibility()?;
                let windows = copy_app_windows(pid)?
                    .ok_or_else(|| "AXWindows unavailable for witnessed process".to_string())?;
                let mut retained = Vec::new();
                for i in 0..CFArrayGetCount(windows.0) {
                    let window = CFArrayGetValueAtIndex(windows.0, i);
                    if window.is_null() || CFGetTypeID(window) != AXUIElementGetTypeID() {
                        return Err("AXWindows contains an unreadable window element".into());
                    }
                    retained.push(OwnedCf::new(CFRetain(window))?);
                }
                Ok(retained)
            }
        }

        fn identifier(&self, window: &OwnedCf) -> Result<Option<String>, String> {
            unsafe { copy_string_attribute(window.0 as AXUIElementRef, KAX_IDENTIFIER_ATTRIBUTE) }
        }

        fn resource(&self, window: &OwnedCf, kind: &LocatorKind) -> Result<Option<String>, String> {
            let attribute = match kind {
                LocatorKind::FilePath(_) => KAX_DOCUMENT_ATTRIBUTE,
                LocatorKind::Url(_) => KAX_URL_ATTRIBUTE,
                LocatorKind::WindowTitle(_) => KAX_TITLE_ATTRIBUTE,
            };
            unsafe { copy_resource_attribute(window.0 as AXUIElementRef, attribute) }
        }

        fn focused_window(&self, pid: i32) -> Result<OwnedCf, String> {
            unsafe {
                let (focused_pid, window) = focused_window()?;
                if focused_pid != pid {
                    return Err("globally focused application is not the witnessed process".into());
                }
                Ok(window)
            }
        }

        fn raise(&self, window: &OwnedCf) -> Result<(), String> {
            unsafe {
                let action = OwnedCf::new(cfstring(KAX_RAISE_ACTION))?;
                let error = AXUIElementPerformAction(window.0 as AXUIElementRef, action.0);
                if error == AX_SUCCESS {
                    Ok(())
                } else {
                    Err(format!(
                        "AXRaise refused the exact window (Accessibility error {error})"
                    ))
                }
            }
        }

        fn activate(&self, pid: i32) -> Result<(), String> {
            activate_app(pid)
        }

        fn focused_window_identifier(&self, pid: i32) -> Result<Option<String>, String> {
            unsafe {
                let (focused_pid, window) = focused_window()?;
                if focused_pid != pid {
                    return Err("globally focused application is not the witnessed process".into());
                }
                copy_string_attribute(window.0 as AXUIElementRef, KAX_IDENTIFIER_ATTRIBUTE)
            }
        }
    }

    fn running_application(pid: i32) -> Result<Id, String> {
        unsafe {
            let app = msg_send_id_i32(
                class_id(b"NSRunningApplication\0") as Id,
                sel(RUNNING_APPLICATION_WITH_PID_SEL),
                pid,
            );
            if app.is_null() {
                Err("witnessed process is no longer running".into())
            } else {
                Ok(app)
            }
        }
    }

    fn process_started_at(pid: i32) -> Result<String, String> {
        unsafe {
            let app = running_application(pid)?;
            let date = msg_send_id(app, sel(LAUNCH_DATE_SEL));
            if date.is_null() {
                return Err("NSRunningApplication launchDate is unavailable".into());
            }
            let seconds = msg_send_f64(date, sel(TIME_INTERVAL_SINCE_1970_SEL));
            if !seconds.is_finite() {
                return Err("NSRunningApplication launchDate is unreadable".into());
            }
            Ok(format!("{:016x}", seconds.to_bits()))
        }
    }

    fn frontmost_application_pid() -> Result<i32, String> {
        unsafe {
            let workspace =
                msg_send_id(class_id(b"NSWorkspace\0") as Id, sel(SHARED_WORKSPACE_SEL));
            if workspace.is_null() {
                return Err("frontmost application unavailable".into());
            }
            let app = msg_send_id(workspace, sel(FRONTMOST_APPLICATION_SEL));
            if app.is_null() {
                return Err("frontmost application unavailable".into());
            }
            let pid = msg_send_i32(app, sel(PROCESS_IDENTIFIER_SEL));
            (pid > 0)
                .then_some(pid)
                .ok_or_else(|| "frontmost process identifier unavailable".into())
        }
    }

    fn check_scoped_focus(before: i32, observed: i32, after: i32) -> Result<i32, String> {
        if before <= 0 || observed != before || after != before {
            Err("frontmost application changed during scoped AX inspection".into())
        } else {
            Ok(before)
        }
    }

    /// macOS can return AXCannotComplete for the system-wide focused-app
    /// attribute while the exact frontmost application's AXFocusedWindow is
    /// readable. Scope the fallback to that one process and reject a focus
    /// race rather than enumerating or choosing another app by title.
    unsafe fn focused_window_from_frontmost_app() -> Result<(i32, OwnedCf), String> {
        unsafe {
            let before = frontmost_application_pid()?;
            let app = OwnedCf::new(AXUIElementCreateApplication(before) as CFTypeRef)?;
            let window = copy_attribute(app.0 as AXUIElementRef, KAX_FOCUSED_WINDOW_ATTRIBUTE)?
                .ok_or_else(|| "focused AX window unavailable".to_string())?;
            if CFGetTypeID(window.0) != AXUIElementGetTypeID() {
                return Err("focused AX window has an unreadable type".into());
            }
            let mut observed = 0;
            if AXUIElementGetPid(window.0 as AXUIElementRef, &mut observed) != AX_SUCCESS {
                return Err("focused AX window process identifier unavailable".into());
            }
            let pid = check_scoped_focus(before, observed, frontmost_application_pid()?)?;
            Ok((pid, window))
        }
    }

    /// Reads only the globally focused app/window, not a desktop-wide snapshot.
    unsafe fn focused_window() -> Result<(i32, OwnedCf), String> {
        unsafe {
            require_accessibility()?;
            let system = OwnedCf::new(AXUIElementCreateSystemWide() as CFTypeRef)?;
            let attribute = OwnedCf::new(cfstring(KAX_FOCUSED_APPLICATION_ATTRIBUTE))?;
            let mut value: CFTypeRef = ptr::null();
            let error =
                AXUIElementCopyAttributeValue(system.0 as AXUIElementRef, attribute.0, &mut value);
            let app = if error == AX_SUCCESS {
                OwnedCf::new(value)?
            } else {
                if !value.is_null() {
                    CFRelease(value);
                }
                if error == AX_CANNOT_COMPLETE {
                    return focused_window_from_frontmost_app();
                }
                return Err(format!(
                    "cannot inspect AXFocusedApplication (Accessibility error {error})"
                ));
            };
            if CFGetTypeID(app.0) != AXUIElementGetTypeID() {
                return Err("focused AX application has an unreadable type".into());
            }
            let mut pid = 0;
            if AXUIElementGetPid(app.0 as AXUIElementRef, &mut pid) != AX_SUCCESS || pid <= 0 {
                return Err("focused AX process identifier unavailable".into());
            }
            let window = copy_attribute(app.0 as AXUIElementRef, KAX_FOCUSED_WINDOW_ATTRIBUTE)?
                .ok_or_else(|| "focused AX window unavailable".to_string())?;
            if CFGetTypeID(window.0) != AXUIElementGetTypeID() {
                return Err("focused AX window has an unreadable type".into());
            }
            Ok((pid, window))
        }
    }

    pub fn raise_surface(surface: &SurfaceIdentity) -> IdentityAttempt {
        raise_surface_with_adapter(surface, &NativeIdentityAdapter)
    }

    pub fn raise_surface_resource(surface: &SurfaceIdentity, resource: &str) -> IdentityAttempt {
        raise_surface_resource_with_adapter(surface, resource, &NativeIdentityAdapter)
    }

    pub fn verify_resource(
        kind: &str,
        resource: &str,
        app: Option<&str>,
        expected: Option<&WindowInfo>,
    ) -> VerificationResult {
        if kind == "url" {
            return VerificationResult::Unverified { reason: "generic URL postcondition verification is unsupported without a browser tab adapter".into() };
        }
        let inspected: Result<bool, String> = (|| unsafe {
            let (pid, window) = focused_window()?;
            if expected.is_some_and(|window| window.owner_pid != pid) {
                return Ok(false);
            }
            let running = running_application(pid)?;
            if let Some(app) = app {
                let name = optional_nsstring(running, sel(LOCALIZED_NAME_SEL))
                    .ok_or_else(|| "focused application name unavailable".to_string())?;
                if name != app {
                    return Ok(false);
                }
            }
            let (attribute, value) = match kind {
                "file" => (KAX_DOCUMENT_ATTRIBUTE, resource),
                "title" => (KAX_TITLE_ATTRIBUTE, resource),
                _ => return Err("postcondition verification unsupported for target kind".into()),
            };
            let witnessed = copy_resource_attribute(window.0 as AXUIElementRef, attribute)?
                .ok_or_else(|| "focused AX resource attribute unavailable".to_string())?;
            let matches = if kind == "file" {
                crate::binding::document_path(&witnessed).as_deref() == Some(value)
            } else {
                witnessed == value
            };
            Ok(matches)
        })();
        match inspected {
            Ok(true) => VerificationResult::Verified,
            Ok(false) => VerificationResult::Failed {
                reason:
                    "globally focused AX window does not show the requested resource/application"
                        .into(),
            },
            Err(reason) => VerificationResult::Unverified { reason },
        }
    }

    /// Whether a file path currently exists (runtime check at execution time
    /// only — never an input to Restoration Derivation).
    pub fn path_exists(path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    /// Opens a URL or file through `NSWorkspace`.
    pub fn open(kind: &str, value: &str) -> Result<(), String> {
        unsafe {
            let workspace = shared_workspace();
            if workspace.is_null() {
                return Err("NSWorkspace unavailable".to_string());
            }
            let ok = match kind {
                "url" => {
                    let url = msg_send_id1(
                        class_id(b"NSURL\0") as Id,
                        sel(URL_WITH_STRING_SEL),
                        ns_string(value),
                    );
                    if url.is_null() {
                        return Err(format!("could not build NSURL from {value:?}"));
                    }
                    msg_send_bool1(workspace, sel(OPEN_URL_SEL), url)
                }
                "file" => msg_send_bool1(workspace, sel(OPEN_FILE_SEL), ns_string(value)),
                _ => return Err(format!("unsupported target kind {kind:?}")),
            };
            if ok != 0 {
                Ok(())
            } else {
                Err(format!("the operating system refused to open {value:?}"))
            }
        }
    }

    /// Enumerates live windows of every running application via Accessibility.
    pub fn system_windows() -> Result<Vec<WindowInfo>, String> {
        unsafe {
            require_accessibility()?;
            let workspace = shared_workspace();
            if workspace.is_null() {
                return Err("NSWorkspace unavailable".to_string());
            }
            let apps = msg_send_id(workspace, sel(RUNNING_APPLICATIONS_SEL));
            if apps.is_null() {
                return Err("cannot enumerate running applications".to_string());
            }
            let count = msg_send_usize(apps, sel(COUNT_SEL));
            let mut windows = Vec::new();
            for i in 0..count {
                let app = msg_send_id1(apps, sel(OBJECT_AT_INDEX_SEL), i as Id);
                if app.is_null() {
                    return Err("cannot inspect a running application".to_string());
                }
                let pid = msg_send_i32(app, sel(PROCESS_IDENTIFIER_SEL));
                let name = optional_nsstring(app, sel(LOCALIZED_NAME_SEL));
                collect_app_windows(pid, name.as_deref().unwrap_or(""), &mut windows)?;
            }
            Ok(windows)
        }
    }

    /// Enumerates the live windows the system itself reports are displaying
    /// the file at this exact path.
    ///
    /// The evidence is each window's Accessibility `AXDocument` attribute,
    /// which document-based applications populate with the URL of the file
    /// they are showing. Matching is exact on the resolved local path: no
    /// filename resemblance, no directory walking, no application knowledge.
    /// An application that does not expose `AXDocument` contributes nothing,
    /// which is honest — Evo then opens the file rather than claiming to know
    /// it was closed.
    pub fn windows_showing_document(path: &str) -> Result<Vec<WindowInfo>, String> {
        unsafe {
            require_accessibility()?;
            let workspace = shared_workspace();
            if workspace.is_null() {
                return Err("NSWorkspace unavailable".to_string());
            }
            let apps = msg_send_id(workspace, sel(RUNNING_APPLICATIONS_SEL));
            if apps.is_null() {
                return Err("cannot enumerate running applications".to_string());
            }
            let count = msg_send_usize(apps, sel(COUNT_SEL));
            let mut windows = Vec::new();
            for i in 0..count {
                let app = msg_send_id1(apps, sel(OBJECT_AT_INDEX_SEL), i as Id);
                if app.is_null() {
                    return Err("cannot inspect a running application".to_string());
                }
                let pid = msg_send_i32(app, sel(PROCESS_IDENTIFIER_SEL));
                let name = optional_nsstring(app, sel(LOCALIZED_NAME_SEL));
                collect_document_windows(pid, name.as_deref().unwrap_or(""), path, &mut windows)?;
            }
            Ok(windows)
        }
    }

    /// Raises the unique matched window and activates its application.
    pub fn raise_and_activate(window: &WindowInfo) -> Result<(), String> {
        unsafe {
            require_accessibility()?;
            let windows_value = copy_app_windows(window.owner_pid)?
                .ok_or_else(|| "cannot inspect windows of the owning application".to_string())?;

            // Scan the entire PID before acting, even for an upstream unique
            // document binding. The title need not be unique within that PID.
            // The array owns the AX elements and remains alive through raise.
            let array = windows_value.0 as CFArrayRef;
            let count = CFArrayGetCount(array);
            let mut titles = Vec::new();
            for i in 0..count {
                let window_element = CFArrayGetValueAtIndex(array, i) as AXUIElementRef;
                titles.push(copy_string_attribute(window_element, KAX_TITLE_ATTRIBUTE)?);
            }
            let index = unique_raise_index(&titles, &window.title)?;
            let matched = CFArrayGetValueAtIndex(array, index as CFIndex) as AXUIElementRef;

            // Raised while the array (which owns the element) is alive.
            let raise_action = OwnedCf::new(cfstring(KAX_RAISE_ACTION))?;
            if AXUIElementPerformAction(matched, raise_action.0) != AX_SUCCESS {
                return Err(format!(
                    "the window “{}” could not be raised (Accessibility permission may be missing)",
                    window.title
                ));
            }

            activate_app(window.owner_pid)
        }
    }

    fn activate_app(pid: i32) -> Result<(), String> {
        unsafe {
            let workspace = shared_workspace();
            if workspace.is_null() {
                return Err("NSWorkspace unavailable".to_string());
            }
            let apps = msg_send_id(workspace, sel(RUNNING_APPLICATIONS_SEL));
            if apps.is_null() {
                return Err("cannot enumerate running applications".to_string());
            }
            let count = msg_send_usize(apps, sel(COUNT_SEL));
            for i in 0..count {
                let app = msg_send_id1(apps, sel(OBJECT_AT_INDEX_SEL), i as Id);
                if app.is_null() {
                    continue;
                }
                if msg_send_i32(app, sel(PROCESS_IDENTIFIER_SEL)) == pid {
                    let ok = msg_send_usize1(
                        app,
                        sel(ACTIVATE_WITH_OPTIONS_SEL),
                        ACTIVATE_IGNORING_OTHER_APPS,
                    );
                    if ok != 0 {
                        return Ok(());
                    }
                    return Err("the application could not be activated".to_string());
                }
            }
            Err("the owning application is no longer running".to_string())
        }
    }

    // ── Objective-C / CoreFoundation plumbing ────────────────────────────

    /// Owns a Create/Copy reference, releasing it on every success/error path.
    struct OwnedCf(CFTypeRef);

    impl OwnedCf {
        fn new(value: CFTypeRef) -> Result<Self, String> {
            if value.is_null() {
                Err("Accessibility returned a null reference".to_string())
            } else {
                Ok(Self(value))
            }
        }
    }

    impl Drop for OwnedCf {
        fn drop(&mut self) {
            unsafe { CFRelease(self.0) }
        }
    }

    fn require_accessibility() -> Result<(), String> {
        if unsafe { AXIsProcessTrusted() } == 0 {
            Err("Accessibility permission is missing; cannot inspect windows".to_string())
        } else {
            Ok(())
        }
    }

    // Unsupported/no-value is an absent attribute. Other AX failures are
    // unreadable state, never evidence for an empty window/document list.
    unsafe fn copy_attribute(
        element: AXUIElementRef,
        attribute: &'static [u8],
    ) -> Result<Option<OwnedCf>, String> {
        unsafe {
            if element.is_null() {
                return Err("cannot inspect a null window element".to_string());
            }
            let attr = OwnedCf::new(cfstring(attribute))?;
            let mut value: CFTypeRef = ptr::null();
            let err = AXUIElementCopyAttributeValue(element, attr.0, &mut value);
            let owned = (!value.is_null()).then(|| OwnedCf(value));
            match err {
                AX_SUCCESS => owned
                    .map(Some)
                    .ok_or_else(|| "Accessibility returned no attribute value".to_string()),
                AX_ATTRIBUTE_UNSUPPORTED | AX_NO_VALUE => Ok(None),
                _ => Err(format!(
                    "cannot inspect {} (Accessibility error {err})",
                    String::from_utf8_lossy(&attribute[..attribute.len() - 1])
                )),
            }
        }
    }

    unsafe fn copy_app_windows(pid: i32) -> Result<Option<OwnedCf>, String> {
        unsafe {
            let element = OwnedCf::new(AXUIElementCreateApplication(pid) as CFTypeRef)?;
            let value = copy_attribute(element.0 as AXUIElementRef, KAX_WINDOWS_ATTRIBUTE)?;
            if let Some(value) = &value {
                if CFGetTypeID(value.0) != CFArrayGetTypeID() {
                    return Err("Accessibility returned a non-array window list".to_string());
                }
            }
            Ok(value)
        }
    }

    unsafe fn collect_app_windows(
        pid: i32,
        owner_name: &str,
        out: &mut Vec<WindowInfo>,
    ) -> Result<(), String> {
        unsafe {
            let Some(windows_value) = copy_app_windows(pid)? else {
                return Ok(());
            };

            let array = windows_value.0 as CFArrayRef;
            let count = CFArrayGetCount(array);
            for i in 0..count {
                let item = CFArrayGetValueAtIndex(array, i);
                let window_element = item as AXUIElementRef;
                if let Some(title) = copy_string_attribute(window_element, KAX_TITLE_ATTRIBUTE)? {
                    if !title.trim().is_empty() {
                        out.push(WindowInfo {
                            title,
                            owner_pid: pid,
                            owner_name: owner_name.to_string(),
                        });
                    }
                }
            }
            Ok(())
        }
    }

    /// Collects one application's windows whose `AXDocument` resolves to
    /// exactly `path`.
    unsafe fn collect_document_windows(
        pid: i32,
        owner_name: &str,
        path: &str,
        out: &mut Vec<WindowInfo>,
    ) -> Result<(), String> {
        unsafe {
            let Some(windows_value) = copy_app_windows(pid)? else {
                return Ok(());
            };

            let array = windows_value.0 as CFArrayRef;
            let count = CFArrayGetCount(array);
            for i in 0..count {
                let window_element = CFArrayGetValueAtIndex(array, i) as AXUIElementRef;
                let Some(document) = copy_string_attribute(window_element, KAX_DOCUMENT_ATTRIBUTE)?
                else {
                    continue;
                };
                // The window is only a match if the OS's own document value
                // resolves to the exact canonical path.
                if crate::binding::document_path(&document).as_deref() != Some(path) {
                    continue;
                }
                let title =
                    copy_string_attribute(window_element, KAX_TITLE_ATTRIBUTE)?.unwrap_or_default();
                out.push(WindowInfo {
                    title,
                    owner_pid: pid,
                    owner_name: owner_name.to_string(),
                });
            }

            Ok(())
        }
    }

    /// Locators may be actual CFURLs or raw strings. Read only the named
    /// attribute; a missing URL/document never becomes a title-based match.
    unsafe fn copy_resource_attribute(
        element: AXUIElementRef,
        attribute: &'static [u8],
    ) -> Result<Option<String>, String> {
        unsafe {
            let Some(value) = copy_attribute(element, attribute)? else {
                return Ok(None);
            };
            let string = if CFGetTypeID(value.0) == CFStringGetTypeID() {
                value.0
            } else if CFGetTypeID(value.0) == CFURLGetTypeID() && attribute != KAX_TITLE_ATTRIBUTE {
                CFURLGetString(value.0)
            } else {
                return Err("Accessibility returned an unreadable resource locator type".into());
            };
            cfstring_to_string(string)
                .map(Some)
                .ok_or_else(|| "cannot decode Accessibility resource locator".to_string())
        }
    }

    /// Reads one string-valued Accessibility attribute of an element.
    unsafe fn copy_string_attribute(
        element: AXUIElementRef,
        attribute: &'static [u8],
    ) -> Result<Option<String>, String> {
        unsafe {
            let Some(value) = copy_attribute(element, attribute)? else {
                return Ok(None);
            };
            if CFGetTypeID(value.0) != CFStringGetTypeID() {
                return Err("Accessibility returned a non-string attribute".to_string());
            }
            cfstring_to_string(value.0 as CFStringRef)
                .map(Some)
                .ok_or_else(|| "cannot decode Accessibility string attribute".to_string())
        }
    }

    unsafe fn shared_workspace() -> Id {
        unsafe { msg_send_id(class_id(b"NSWorkspace\0") as Id, sel(SHARED_WORKSPACE_SEL)) }
    }

    unsafe fn ns_string(value: &str) -> Id {
        unsafe {
            // The C buffer lives for the duration of the synchronous msg_send call;
            // NSString copies the bytes, so no dangling pointer escapes.
            let mut bytes = value.as_bytes().to_vec();
            bytes.push(0);
            msg_send_id1(
                class_id(b"NSString\0") as Id,
                sel(STRING_WITH_UTF8_SEL),
                bytes.as_ptr() as *mut c_char as Id,
            )
        }
    }

    unsafe fn optional_nsstring(receiver: Id, selector: Sel) -> Option<String> {
        unsafe {
            let value = msg_send_id(receiver, selector);
            if value.is_null() {
                return None;
            }
            let cstr = msg_send_id(value, sel(UTF8_STRING_SEL));
            if cstr.is_null() {
                return None;
            }
            let bytes = CStr::from_ptr(cstr as *const c_char);
            Some(bytes.to_string_lossy().into_owned())
        }
    }

    unsafe fn cfstring(bytes: &'static [u8]) -> CFStringRef {
        unsafe {
            CFStringCreateWithCString(
                ptr::null(),
                bytes.as_ptr() as *const c_char,
                CFSTRING_ENCODING_UTF8,
            )
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

    fn sel(bytes: &'static [u8]) -> Sel {
        unsafe { sel_registerName(bytes.as_ptr() as *const c_char) }
    }

    unsafe fn class_id(name: &'static [u8]) -> Class {
        unsafe { objc_getClass(name.as_ptr() as *const c_char) }
    }

    unsafe fn msg_send_id(receiver: Id, selector: Sel) -> Id {
        unsafe {
            let function: extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector)
        }
    }

    unsafe fn msg_send_id1(receiver: Id, selector: Sel, arg: Id) -> Id {
        unsafe {
            let function: extern "C" fn(Id, Sel, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector, arg)
        }
    }

    unsafe fn msg_send_bool1(receiver: Id, selector: Sel, arg: Id) -> Bool {
        unsafe {
            let function: extern "C" fn(Id, Sel, Id) -> Bool =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector, arg)
        }
    }

    unsafe fn msg_send_usize(receiver: Id, selector: Sel) -> usize {
        unsafe {
            let function: extern "C" fn(Id, Sel) -> usize =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector)
        }
    }

    unsafe fn msg_send_usize1(receiver: Id, selector: Sel, arg: usize) -> usize {
        unsafe {
            let function: extern "C" fn(Id, Sel, usize) -> usize =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector, arg)
        }
    }

    unsafe fn msg_send_i32(receiver: Id, selector: Sel) -> i32 {
        unsafe {
            let function: extern "C" fn(Id, Sel) -> i32 =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector)
        }
    }

    unsafe fn msg_send_id_i32(receiver: Id, selector: Sel, arg: i32) -> Id {
        unsafe {
            let function: extern "C" fn(Id, Sel, i32) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector, arg)
        }
    }

    unsafe fn msg_send_f64(receiver: Id, selector: Sel) -> f64 {
        unsafe {
            let function: extern "C" fn(Id, Sel) -> f64 =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector)
        }
    }

    const UTF8_STRING_SEL: &[u8] = b"UTF8String\0";
    const STRING_WITH_UTF8_SEL: &[u8] = b"stringWithUTF8String:\0";

    #[cfg(test)]
    mod scoped_focus_tests {
        use super::check_scoped_focus;

        #[test]
        fn exact_frontmost_pid_must_remain_the_owner_of_the_focused_ax_window() {
            assert_eq!(check_scoped_focus(42, 42, 42), Ok(42));
            for (before, window, after) in [(0, 42, 42), (42, 41, 42), (42, 42, 41)] {
                assert!(check_scoped_focus(before, window, after).is_err());
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::{IdentityAttempt, WindowInfo, WindowSource};
    use crate::engine::TargetStatus;
    use crate::restore::VerificationResult;
    use evo_observation::surface_identity::SurfaceIdentity;

    pub fn raise_surface(_surface: &SurfaceIdentity) -> IdentityAttempt {
        IdentityAttempt::unavailable("native surface identity execution is only supported on macOS")
    }

    pub fn raise_surface_resource(_surface: &SurfaceIdentity, _resource: &str) -> IdentityAttempt {
        IdentityAttempt::unavailable("native surface resource execution is only supported on macOS")
    }

    pub fn verify_resource(
        _kind: &str,
        _resource: &str,
        _app: Option<&str>,
        _window: Option<&WindowInfo>,
    ) -> VerificationResult {
        VerificationResult::Unverified {
            reason: "native postcondition inspection is only supported on macOS".into(),
        }
    }

    /// Non-macOS stub: opening is unsupported, honestly reported.
    pub fn open(_kind: &str, _value: &str) -> Result<(), String> {
        Err("execution is only supported on macOS".to_string())
    }

    pub fn path_exists(path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    pub fn raise_and_activate(_window: &WindowInfo) -> Result<(), String> {
        Err("execution is only supported on macOS".to_string())
    }

    /// Non-macOS stub: an error, never an empty list, because "this platform
    /// cannot tell which window shows a file" is not the same fact as "no
    /// window is showing it".
    pub fn windows_showing_document(_path: &str) -> Result<Vec<WindowInfo>, String> {
        Err("document inspection is only supported on macOS".to_string())
    }

    /// Non-macOS stub for the window-list query behind [`super::live_windows`].
    pub fn system_windows() -> Result<Vec<WindowInfo>, String> {
        Err("window inspection is only supported on macOS".to_string())
    }

    pub struct SystemWindowSource;

    impl WindowSource for SystemWindowSource {
        fn windows(&self) -> Result<Vec<WindowInfo>, String> {
            Err("window inspection is only supported on macOS".to_string())
        }
    }

    // Kept for API parity on non-macOS builds.
    #[allow(dead_code)]
    pub fn stub_status() -> TargetStatus {
        TargetStatus::Unavailable {
            reason: "execution is only supported on macOS".to_string(),
        }
    }
}

// On non-macOS, `SystemWindowSource` is defined inside `imp` and honestly
// reports unavailability; on macOS it is the top-level struct above.
#[cfg(not(target_os = "macos"))]
pub use imp::SystemWindowSource;

/// Thin, platform-honest wrappers over the platform implementation, for
/// callers (the restore executor) that act on bare resources rather than
/// through the full executor pipeline.
pub fn open_target(kind: &str, value: &str) -> Result<(), String> {
    imp::open(kind, value)
}

pub fn raise_window(window: &WindowInfo) -> Result<(), String> {
    imp::raise_and_activate(window)
}

pub fn windows_showing(path: &str) -> Result<Vec<WindowInfo>, String> {
    imp::windows_showing_document(path)
}

pub fn live_windows() -> Result<Vec<WindowInfo>, String> {
    imp::system_windows()
}

/// Window-only focus primitive; this does not verify a resource restoration.
/// Constrained restore targets must use [`raise_surface_resource`] instead.
pub fn raise_surface(surface: &SurfaceIdentity) -> IdentityAttempt {
    imp::raise_surface(surface)
}

pub fn raise_surface_resource(surface: &SurfaceIdentity, resource: &str) -> IdentityAttempt {
    imp::raise_surface_resource(surface, resource)
}

/// Inspect-only postcondition probe. Reads the focused AX window only; it
/// does not capture other windows, launch apps, or dispatch native actions.
pub fn verify_resource(
    kind: &str,
    resource: &str,
    app: Option<&str>,
    window: Option<&WindowInfo>,
) -> VerificationResult {
    imp::verify_resource(kind, resource, app, window)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    fn window(title: &str, pid: i32, owner: &str) -> WindowInfo {
        WindowInfo {
            title: title.to_string(),
            owner_pid: pid,
            owner_name: owner.to_string(),
        }
    }

    #[test]
    fn unique_title_match_selects_the_window() {
        let windows = vec![
            window("Design Brief — Canvas", 100, "Canvas"),
            window("Untitled", 200, "TextEdit"),
        ];
        let selected = select_unique_window(&windows, "Design Brief — Canvas").unwrap();
        assert_eq!(selected.owner_pid, 100);
        assert_eq!(selected.owner_name, "Canvas");
    }

    #[test]
    fn no_title_match_is_unavailable() {
        let windows = vec![window("Untitled", 200, "TextEdit")];
        assert_eq!(
            select_unique_window(&windows, "Design Brief — Canvas"),
            Err(WindowSelectionError::NoMatch)
        );
    }

    #[test]
    fn multiple_title_matches_are_ambiguous() {
        let windows = vec![
            window("Untitled", 200, "TextEdit"),
            window("Untitled", 300, "Notes"),
        ];
        assert_eq!(
            select_unique_window(&windows, "Untitled"),
            Err(WindowSelectionError::MultipleMatches { count: 2 })
        );
    }

    #[test]
    fn title_match_is_exact_and_case_sensitive() {
        let windows = vec![window("Design Brief", 100, "Canvas")];
        assert_eq!(
            select_unique_window(&windows, "design brief"),
            Err(WindowSelectionError::NoMatch)
        );
    }

    #[test]
    fn empty_title_never_selects() {
        let windows = vec![window("", 100, "Canvas")];
        assert_eq!(
            select_unique_window(&windows, ""),
            Err(WindowSelectionError::NoMatch)
        );
    }

    struct FakeSource(Vec<WindowInfo>);

    impl WindowSource for FakeSource {
        fn windows(&self) -> Result<Vec<WindowInfo>, String> {
            Ok(self.0.clone())
        }
    }

    struct FailingSource;

    impl WindowSource for FailingSource {
        fn windows(&self) -> Result<Vec<WindowInfo>, String> {
            Err("Accessibility permission is missing".to_string())
        }
    }

    #[test]
    fn native_raise_selects_only_a_unique_title_in_owning_pid() {
        let titles = [Some("A".into()), Some("B".into()), Some("C".into())];
        assert_eq!(unique_raise_index(&titles, "B"), Ok(1));
    }

    #[test]
    fn native_raise_counts_late_duplicates_even_after_an_initial_match() {
        // These are all windows in one PID, including unrelated documents
        // that acquired the same title after an upstream document binding.
        let titles = [Some("B".into()), Some("Other".into()), Some("B".into())];
        let err = unique_raise_index(&titles, "B").unwrap_err();
        assert!(err.contains("2 windows"));
        assert!(err.contains("cannot raise a unique window"));
    }

    #[test]
    fn native_raise_refuses_disappeared_or_unidentifiable_target() {
        assert!(
            unique_raise_index(&[Some("Other".into())], "B")
                .unwrap_err()
                .contains("no longer open")
        );
        assert!(unique_raise_index(&[], "B").is_err());
        assert!(
            unique_raise_index(&[Some("".into())], "")
                .unwrap_err()
                .contains("empty title")
        );
    }

    #[test]
    fn native_raise_refuses_unreadable_competitor_even_with_one_match() {
        for titles in [vec![Some("B".into()), None], vec![None, Some("B".into())]] {
            assert!(
                unique_raise_index(&titles, "B")
                    .unwrap_err()
                    .contains("cannot inspect every window title")
            );
        }
    }

    #[test]
    fn executor_reports_unavailable_for_missing_file() {
        let executor = MacOSExecutor::with_source(FakeSource(vec![]));
        let path = std::env::temp_dir()
            .join("evo-execution-definitely-missing-file")
            .to_string_lossy()
            .into_owned();
        let status = executor.open_file(&path);
        // A missing resource is Unavailable, never a substitute and never a
        // claim that the OS refused to open something that is not there.
        assert!(
            matches!(status, TargetStatus::Unavailable { .. }),
            "missing file must be Unavailable, got {status:?}"
        );
    }

    #[test]
    fn executor_reports_unavailable_when_source_fails() {
        let executor = MacOSExecutor::with_source(FailingSource);
        let status = executor.focus_window("B");
        assert!(matches!(
            status,
            TargetStatus::Unavailable { reason } if reason.contains("Accessibility")
        ));
    }

    #[derive(Clone)]
    struct IdentityWindow {
        handle: usize,
        title: String,
        identifier: Option<String>,
    }

    #[derive(Debug, PartialEq, Eq)]
    enum IdentityEffect {
        Raise(usize),
        Activate(i32),
    }

    struct FakeIdentityAdapter {
        starts: RefCell<VecDeque<Result<String, String>>>,
        windows: Result<Vec<IdentityWindow>, String>,
        changed_identifier: Option<Option<String>>,
        identifier_error: Option<String>,
        reads: Cell<usize>,
        raised: Result<(), String>,
        activated: Result<(), String>,
        focused: Result<Option<String>, String>,
        resource: Result<Option<String>, String>,
        resource_after_raise: Option<Result<Option<String>, String>>,
        effects: RefCell<Vec<IdentityEffect>>,
    }

    fn identity() -> SurfaceIdentity {
        SurfaceIdentity {
            capture_session: "capture-fixture".into(),
            window_id: "window-1".into(),
            process_id: 10,
            process_started_at: format!("{:016x}", 1_791_000_000.5f64.to_bits()),
            window_identifier: Some("wanted-window".into()),
            browser_window: None,
            browser_profile: None,
            browser_session: None,
            browser_tab: None,
        }
    }

    impl Default for FakeIdentityAdapter {
        fn default() -> Self {
            Self {
                starts: RefCell::new(VecDeque::from([Ok(identity().process_started_at)])),
                windows: Ok(vec![IdentityWindow {
                    handle: 1,
                    title: "Report".into(),
                    identifier: Some("wanted-window".into()),
                }]),
                changed_identifier: None,
                identifier_error: None,
                reads: Cell::new(0),
                raised: Ok(()),
                activated: Ok(()),
                focused: Ok(Some("wanted-window".into())),
                resource: Ok(None),
                resource_after_raise: None,
                effects: RefCell::new(vec![]),
            }
        }
    }

    impl IdentityAdapter for FakeIdentityAdapter {
        type Window = IdentityWindow;
        fn process_started_at(&self, pid: i32) -> Result<String, String> {
            assert_eq!(pid, 10);
            let mut starts = self.starts.borrow_mut();
            if starts.len() > 1 {
                starts.pop_front().unwrap()
            } else {
                starts.front().unwrap().clone()
            }
        }
        fn windows(&self, pid: i32) -> Result<Vec<IdentityWindow>, String> {
            assert_eq!(pid, 10);
            self.windows.clone()
        }
        fn identifier(&self, window: &IdentityWindow) -> Result<Option<String>, String> {
            if let Some(error) = &self.identifier_error {
                return Err(error.clone());
            }
            let reads = self.reads.get();
            self.reads.set(reads + 1);
            if reads >= self.windows.as_ref().unwrap().len() {
                if let Some(changed) = &self.changed_identifier {
                    return Ok(changed.clone());
                }
            }
            Ok(window.identifier.clone())
        }
        fn raise(&self, window: &IdentityWindow) -> Result<(), String> {
            self.effects
                .borrow_mut()
                .push(IdentityEffect::Raise(window.handle));
            self.raised.clone()
        }
        fn resource(
            &self,
            window: &IdentityWindow,
            kind: &LocatorKind,
        ) -> Result<Option<String>, String> {
            if !self.effects.borrow().is_empty() {
                if let Some(resource) = &self.resource_after_raise {
                    return resource.clone();
                }
            }
            match kind {
                LocatorKind::WindowTitle(_) => Ok(Some(window.title.clone())),
                _ => self.resource.clone(),
            }
        }
        fn focused_window(&self, pid: i32) -> Result<IdentityWindow, String> {
            assert_eq!(pid, 10);
            let focused = self
                .focused
                .clone()?
                .ok_or_else(|| "focused window unavailable".to_string())?;
            self.windows
                .as_ref()
                .map_err(Clone::clone)?
                .iter()
                .find(|window| window.identifier.as_deref() == Some(&focused))
                .cloned()
                .ok_or_else(|| "focused window is not the witnessed window".into())
        }
        fn activate(&self, pid: i32) -> Result<(), String> {
            self.effects
                .borrow_mut()
                .push(IdentityEffect::Activate(pid));
            self.activated.clone()
        }
        fn focused_window_identifier(&self, pid: i32) -> Result<Option<String>, String> {
            assert_eq!(pid, 10);
            self.focused.clone()
        }
    }

    #[test]
    fn identity_selects_retained_exact_element_between_same_title_windows() {
        let adapter = FakeIdentityAdapter {
            windows: Ok(vec![
                IdentityWindow {
                    handle: 2,
                    title: "Report".into(),
                    identifier: Some("other-window".into()),
                },
                IdentityWindow {
                    handle: 3,
                    title: "Report".into(),
                    identifier: Some("wanted-window".into()),
                },
            ]),
            ..FakeIdentityAdapter::default()
        };
        let windows = adapter.windows.as_ref().unwrap();
        assert_eq!(windows[0].title, windows[1].title);
        let attempt = raise_surface_with_adapter(&identity(), &adapter);
        assert_eq!(attempt.dispatch, DispatchResult::RaiseAccepted);
        assert_eq!(attempt.verification, VerificationResult::Verified);
        assert_eq!(
            *adapter.effects.borrow(),
            [IdentityEffect::Raise(3), IdentityEffect::Activate(10)]
        );
    }

    #[test]
    fn identity_refuses_pid_reuse_restart_and_missing_launch_date_before_action() {
        let original = identity().process_started_at;
        let replacement = format!("{:016x}", 1_791_000_010.5f64.to_bits());
        for starts in [
            vec![Ok(replacement.clone())],
            vec![Ok(original), Ok(replacement)],
            vec![Err("launchDate unavailable".into())],
        ] {
            let adapter = FakeIdentityAdapter {
                starts: RefCell::new(VecDeque::from(starts)),
                ..FakeIdentityAdapter::default()
            };
            assert!(matches!(
                raise_surface_with_adapter(&identity(), &adapter).dispatch,
                DispatchResult::NotDispatched { .. }
            ));
            assert!(adapter.effects.borrow().is_empty());
        }
    }

    #[test]
    fn identity_refuses_missing_unreadable_duplicate_or_unmatched_identifiers() {
        for identifiers in [
            vec![None],
            vec![Some("wanted-window"), None],
            vec![Some("wanted-window"), Some("")],
            vec![Some("wanted-window"), Some("wanted-window")],
            vec![Some("other-window")],
            vec![],
        ] {
            let windows = identifiers
                .into_iter()
                .enumerate()
                .map(|(handle, identifier)| IdentityWindow {
                    handle,
                    title: "Report".into(),
                    identifier: identifier.map(str::to_string),
                })
                .collect();
            let adapter = FakeIdentityAdapter {
                windows: Ok(windows),
                ..FakeIdentityAdapter::default()
            };
            assert!(matches!(
                raise_surface_with_adapter(&identity(), &adapter).dispatch,
                DispatchResult::NotDispatched { .. }
            ));
            assert!(adapter.effects.borrow().is_empty());
        }
        for adapter in [
            FakeIdentityAdapter {
                windows: Err("AXWindows unreadable".into()),
                ..FakeIdentityAdapter::default()
            },
            FakeIdentityAdapter {
                identifier_error: Some("AXIdentifier unreadable".into()),
                ..FakeIdentityAdapter::default()
            },
        ] {
            assert!(matches!(
                raise_surface_with_adapter(&identity(), &adapter).dispatch,
                DispatchResult::NotDispatched { .. }
            ));
            assert!(adapter.effects.borrow().is_empty());
        }
    }

    #[test]
    fn identity_missing_required_native_identifier_and_unsupported_browser_constraints_do_not_widen()
     {
        let mut missing = identity();
        missing.window_identifier = None;
        let mut profile = identity();
        profile.browser_profile = Some("profile-a".into());
        let mut tab = profile.clone();
        tab.browser_tab = Some("tab-a".into());
        let mut invalid = identity();
        invalid.process_started_at = "0000000000000000".into();
        for surface in [missing, profile, tab, invalid] {
            let adapter = FakeIdentityAdapter::default();
            let attempt = raise_surface_with_adapter(&surface, &adapter);
            assert!(matches!(
                attempt.dispatch,
                DispatchResult::NotDispatched { .. }
            ));
            assert_eq!(adapter.reads.get(), 0);
            assert!(adapter.effects.borrow().is_empty());
        }
    }

    #[test]
    fn identity_mutation_before_raise_is_refused_without_searching_for_a_replacement() {
        for changed in [None, Some("replacement-window".into())] {
            let adapter = FakeIdentityAdapter {
                changed_identifier: Some(changed),
                ..FakeIdentityAdapter::default()
            };
            let attempt = raise_surface_with_adapter(&identity(), &adapter);
            assert!(
                matches!(attempt.dispatch, DispatchResult::NotDispatched { reason } if reason.contains("changed before dispatch"))
            );
            assert!(adapter.effects.borrow().is_empty());
        }
    }

    #[test]
    fn identity_postcondition_failures_remain_dispatch_accepted_without_a_second_action() {
        for focused in [
            Ok(None),
            Ok(Some("wrong-window".into())),
            Err("focused AXIdentifier unreadable".into()),
            Err("globally focused app differs".into()),
        ] {
            let adapter = FakeIdentityAdapter {
                focused,
                ..FakeIdentityAdapter::default()
            };
            let attempt = raise_surface_with_adapter(&identity(), &adapter);
            assert_eq!(attempt.dispatch, DispatchResult::RaiseAccepted);
            assert!(matches!(
                attempt.verification,
                VerificationResult::Failed { .. }
            ));
            assert_eq!(
                *adapter.effects.borrow(),
                [IdentityEffect::Raise(1), IdentityEffect::Activate(10)]
            );
        }
    }

    #[test]
    fn identity_rechecks_process_after_raise_and_after_focused_inspection() {
        let original = identity().process_started_at;
        let replacement = format!("{:016x}", 1_791_000_010.5f64.to_bits());
        for changed_at in [2, 4] {
            let mut starts = vec![Ok(original.clone()); changed_at];
            starts.push(Ok(replacement.clone()));
            let adapter = FakeIdentityAdapter {
                starts: RefCell::new(VecDeque::from(starts)),
                ..FakeIdentityAdapter::default()
            };
            let attempt = raise_surface_with_adapter(&identity(), &adapter);
            assert_eq!(attempt.dispatch, DispatchResult::RaiseAccepted);
            assert!(
                matches!(attempt.verification, VerificationResult::Failed { reason } if reason.contains("process lifetime changed"))
            );
            assert_eq!(adapter.effects.borrow()[0], IdentityEffect::Raise(1));
            assert_eq!(
                adapter.effects.borrow().len(),
                if changed_at == 2 { 1 } else { 2 }
            );
        }
    }

    #[test]
    fn identity_raise_and_activation_refusals_have_distinct_receipts() {
        let adapter = FakeIdentityAdapter {
            raised: Err("AXRaise refused".into()),
            ..FakeIdentityAdapter::default()
        };
        let attempt = raise_surface_with_adapter(&identity(), &adapter);
        assert!(matches!(attempt.dispatch, DispatchResult::Failed { .. }));
        assert_eq!(*adapter.effects.borrow(), [IdentityEffect::Raise(1)]);
        let adapter = FakeIdentityAdapter {
            activated: Err("activation refused".into()),
            ..FakeIdentityAdapter::default()
        };
        let attempt = raise_surface_with_adapter(&identity(), &adapter);
        assert_eq!(attempt.dispatch, DispatchResult::RaiseAccepted);
        assert!(
            matches!(attempt.verification, VerificationResult::Failed { reason } if reason.contains("activation refused"))
        );
        assert_eq!(
            *adapter.effects.borrow(),
            [IdentityEffect::Raise(1), IdentityEffect::Activate(10)]
        );
    }

    #[test]
    fn constrained_resources_require_exact_document_or_url_before_dispatch() {
        for (requested, witnessed) in [
            ("/work/report.md", Some("/work/other.md")),
            ("/work/report.md", Some("https://example.com/report.md")),
            ("/work/report.md", None),
            (
                "https://example.com/exact",
                Some("https://example.com/other"),
            ),
            ("https://example.com/exact", Some("Report")),
            ("https://example.com/exact", None),
        ] {
            let adapter = FakeIdentityAdapter {
                resource: Ok(witnessed.map(str::to_string)),
                ..FakeIdentityAdapter::default()
            };
            let attempt = raise_surface_resource_with_adapter(&identity(), requested, &adapter);
            assert!(
                matches!(attempt.dispatch, DispatchResult::NotDispatched { .. }),
                "{attempt:?}"
            );
            assert!(adapter.effects.borrow().is_empty());
        }
        let adapter = FakeIdentityAdapter {
            resource: Err("AXDocument unreadable".into()),
            ..FakeIdentityAdapter::default()
        };
        assert!(matches!(
            raise_surface_resource_with_adapter(&identity(), "/work/report.md", &adapter).dispatch,
            DispatchResult::NotDispatched { .. }
        ));
        assert!(adapter.effects.borrow().is_empty());
    }

    #[test]
    fn constrained_resources_accept_only_exact_raw_file_uri_path_equivalence() {
        for (requested, witnessed) in [
            ("/work/report.md", "file:///work/report.md"),
            ("file:///work/report.md", "/work/report.md"),
            ("https://example.com/exact", "https://example.com/exact"),
        ] {
            let adapter = FakeIdentityAdapter {
                resource: Ok(Some(witnessed.into())),
                ..FakeIdentityAdapter::default()
            };
            let attempt = raise_surface_resource_with_adapter(&identity(), requested, &adapter);
            assert_eq!(
                attempt.verification,
                VerificationResult::Verified,
                "{attempt:?}"
            );
        }
        let adapter = FakeIdentityAdapter {
            resource: Ok(Some("/work/../work/report.md".into())),
            ..FakeIdentityAdapter::default()
        };
        assert!(matches!(
            raise_surface_resource_with_adapter(&identity(), "/work/report.md", &adapter).dispatch,
            DispatchResult::NotDispatched { .. }
        ));
        assert!(adapter.effects.borrow().is_empty());
    }

    #[test]
    fn switched_or_unreadable_resource_after_raise_is_accepted_but_verification_failed() {
        for (requested, before, after) in [
            (
                "/work/report.md",
                "/work/report.md",
                Ok(Some("/work/other.md".into())),
            ),
            (
                "https://example.com/exact",
                "https://example.com/exact",
                Ok(Some("https://example.com/other".into())),
            ),
            ("/work/report.md", "/work/report.md", Ok(None)),
            (
                "/work/report.md",
                "/work/report.md",
                Err("AXDocument unreadable after dispatch".into()),
            ),
        ] {
            let adapter = FakeIdentityAdapter {
                resource: Ok(Some(before.into())),
                resource_after_raise: Some(after),
                ..FakeIdentityAdapter::default()
            };
            let attempt = raise_surface_resource_with_adapter(&identity(), requested, &adapter);
            assert_eq!(attempt.dispatch, DispatchResult::RaiseAccepted);
            assert!(matches!(
                attempt.verification,
                VerificationResult::Failed { .. }
            ));
            assert_eq!(
                *adapter.effects.borrow(),
                [IdentityEffect::Raise(1), IdentityEffect::Activate(10)]
            );
        }
    }

    #[test]
    fn exact_document_under_identity_can_raise_between_duplicate_titles() {
        let adapter = FakeIdentityAdapter {
            windows: Ok(vec![
                IdentityWindow {
                    handle: 2,
                    title: "Report".into(),
                    identifier: Some("other-window".into()),
                },
                IdentityWindow {
                    handle: 3,
                    title: "Report".into(),
                    identifier: Some("wanted-window".into()),
                },
            ]),
            resource: Ok(Some("file:///work/report.md".into())),
            ..FakeIdentityAdapter::default()
        };
        let attempt = raise_surface_resource_with_adapter(&identity(), "/work/report.md", &adapter);
        assert_eq!(attempt.verification, VerificationResult::Verified);
        assert_eq!(
            *adapter.effects.borrow(),
            [IdentityEffect::Raise(3), IdentityEffect::Activate(10)]
        );
    }

    #[test]
    fn title_targets_require_exact_native_title_without_locator_substitution() {
        let adapter = FakeIdentityAdapter::default();
        assert_eq!(
            raise_surface_resource_with_adapter(&identity(), "Report", &adapter).verification,
            VerificationResult::Verified
        );
        let adapter = FakeIdentityAdapter::default();
        assert!(matches!(
            raise_surface_resource_with_adapter(&identity(), "report", &adapter).dispatch,
            DispatchResult::NotDispatched { .. }
        ));
        assert!(adapter.effects.borrow().is_empty());
        let adapter = FakeIdentityAdapter {
            resource_after_raise: Some(Ok(Some("Other Report".into()))),
            ..FakeIdentityAdapter::default()
        };
        let attempt = raise_surface_resource_with_adapter(&identity(), "Report", &adapter);
        assert_eq!(attempt.dispatch, DispatchResult::RaiseAccepted);
        assert!(matches!(
            attempt.verification,
            VerificationResult::Failed { .. }
        ));
    }
}
