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
use crate::preflight::PreflightSource;

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
    /// across every running application. Re-deriving it by title would
    /// downgrade a certainty into an ambiguity.
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
    use super::WindowInfo;
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
    const CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;

    #[repr(C)]
    pub struct Object {
        _priv: [u8; 0],
    }

    #[repr(C)]
    pub struct objc_class {
        _priv: [u8; 0],
    }

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
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
        fn CFRelease(cf: CFTypeRef);
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
    const KAX_RAISE_ACTION: &[u8] = b"AXRaise\0";
    const AX_TRUSTED_CHECK_OPTION_PROMPT: &[u8] = b"AXTrustedCheckOptionPrompt\0";

    const SHARED_WORKSPACE_SEL: &[u8] = b"sharedWorkspace\0";
    const OPEN_URL_SEL: &[u8] = b"openURL:\0";
    const OPEN_FILE_SEL: &[u8] = b"openFile:\0";
    const URL_WITH_STRING_SEL: &[u8] = b"URLWithString:\0";
    const RUNNING_APPLICATIONS_SEL: &[u8] = b"runningApplications\0";
    const PROCESS_IDENTIFIER_SEL: &[u8] = b"processIdentifier\0";
    const LOCALIZED_NAME_SEL: &[u8] = b"localizedName\0";
    const ACTIVATE_WITH_OPTIONS_SEL: &[u8] = b"activateWithOptions:\0";
    const OBJECT_AT_INDEX_SEL: &[u8] = b"objectAtIndex:\0";
    const COUNT_SEL: &[u8] = b"count\0";

    /// `NSApplicationActivateIgnoringOtherApps`.
    const ACTIVATE_IGNORING_OTHER_APPS: usize = 1 << 1;

    // ── Public helpers used by the executor ──────────────────────────────

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
                    continue;
                }
                let pid = msg_send_i32(app, sel(PROCESS_IDENTIFIER_SEL));
                let name = optional_nsstring(app, sel(LOCALIZED_NAME_SEL));
                collect_app_windows(pid, name.as_deref().unwrap_or(""), &mut windows);
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
                    continue;
                }
                let pid = msg_send_i32(app, sel(PROCESS_IDENTIFIER_SEL));
                let name = optional_nsstring(app, sel(LOCALIZED_NAME_SEL));
                collect_document_windows(pid, name.as_deref().unwrap_or(""), path, &mut windows);
            }
            Ok(windows)
        }
    }

    /// Raises the unique matched window and activates its application.
    pub fn raise_and_activate(window: &WindowInfo) -> Result<(), String> {
        unsafe {
            let element = AXUIElementCreateApplication(window.owner_pid);
            if element.is_null() {
                return Err("cannot address the owning application".to_string());
            }
            let windows_attr = cfstring(KAX_WINDOWS_ATTRIBUTE);
            let mut windows_value: CFTypeRef = ptr::null();
            let err = AXUIElementCopyAttributeValue(element, windows_attr, &mut windows_value);
            CFRelease(windows_attr);
            if err != AX_SUCCESS || windows_value.is_null() {
                CFRelease(element as CFTypeRef);
                return Err(format!(
                    "cannot read windows of {} (Accessibility permission may be missing)",
                    window.owner_name
                ));
            }

            // Find the window element whose title matches exactly. The
            // element is owned by the CFArray, so the raise action MUST be
            // performed while `windows_value` is still alive.
            let array = windows_value as CFArrayRef;
            let count = CFArrayGetCount(array);
            let mut matched: AXUIElementRef = ptr::null_mut();
            for i in 0..count {
                let item = CFArrayGetValueAtIndex(array, i);
                let window_element = item as AXUIElementRef;
                let title_attr = cfstring(KAX_TITLE_ATTRIBUTE);
                let mut title_value: CFTypeRef = ptr::null();
                if AXUIElementCopyAttributeValue(window_element, title_attr, &mut title_value)
                    == AX_SUCCESS
                    && !title_value.is_null()
                {
                    let title = cfstring_to_string(title_value as CFStringRef).unwrap_or_default();
                    CFRelease(title_value);
                    if title == window.title {
                        matched = window_element;
                        CFRelease(title_attr);
                        break;
                    }
                }
                CFRelease(title_attr);
            }

            if matched.is_null() {
                CFRelease(windows_value);
                CFRelease(element as CFTypeRef);
                return Err(format!("the window “{}” is no longer open", window.title));
            }

            // Raised while the array (which owns the element) is alive.
            let raise_action = cfstring(KAX_RAISE_ACTION);
            let raise_err = AXUIElementPerformAction(matched, raise_action);
            CFRelease(raise_action);
            let raise_ok = raise_err == AX_SUCCESS;
            CFRelease(windows_value);
            CFRelease(element as CFTypeRef);

            if !raise_ok {
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

    unsafe fn collect_app_windows(pid: i32, owner_name: &str, out: &mut Vec<WindowInfo>) {
        let element = AXUIElementCreateApplication(pid);
        if element.is_null() {
            return;
        }
        let windows_attr = cfstring(KAX_WINDOWS_ATTRIBUTE);
        let mut windows_value: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(element, windows_attr, &mut windows_value);
        CFRelease(windows_attr);
        if err != AX_SUCCESS || windows_value.is_null() {
            CFRelease(element as CFTypeRef);
            return;
        }

        let array = windows_value as CFArrayRef;
        let count = CFArrayGetCount(array);
        for i in 0..count {
            let item = CFArrayGetValueAtIndex(array, i);
            let window_element = item as AXUIElementRef;
            let title_attr = cfstring(KAX_TITLE_ATTRIBUTE);
            let mut title_value: CFTypeRef = ptr::null();
            if AXUIElementCopyAttributeValue(window_element, title_attr, &mut title_value)
                == AX_SUCCESS
                && !title_value.is_null()
            {
                let title = cfstring_to_string(title_value as CFStringRef).unwrap_or_default();
                CFRelease(title_value);
                if !title.trim().is_empty() {
                    out.push(WindowInfo {
                        title,
                        owner_pid: pid,
                        owner_name: owner_name.to_string(),
                    });
                }
            }
            CFRelease(title_attr);
        }

        CFRelease(windows_value);
        CFRelease(element as CFTypeRef);
    }

    /// Collects one application's windows whose `AXDocument` resolves to
    /// exactly `path`.
    unsafe fn collect_document_windows(
        pid: i32,
        owner_name: &str,
        path: &str,
        out: &mut Vec<WindowInfo>,
    ) {
        let element = AXUIElementCreateApplication(pid);
        if element.is_null() {
            return;
        }
        let windows_attr = cfstring(KAX_WINDOWS_ATTRIBUTE);
        let mut windows_value: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(element, windows_attr, &mut windows_value);
        CFRelease(windows_attr);
        if err != AX_SUCCESS || windows_value.is_null() {
            CFRelease(element as CFTypeRef);
            return;
        }

        let array = windows_value as CFArrayRef;
        let count = CFArrayGetCount(array);
        for i in 0..count {
            let window_element = CFArrayGetValueAtIndex(array, i) as AXUIElementRef;
            let Some(document) = copy_string_attribute(window_element, KAX_DOCUMENT_ATTRIBUTE)
            else {
                continue;
            };
            // The window is only a match if the OS's own document value
            // resolves to the exact canonical path.
            if crate::binding::document_path(&document).as_deref() != Some(path) {
                continue;
            }
            let title =
                copy_string_attribute(window_element, KAX_TITLE_ATTRIBUTE).unwrap_or_default();
            out.push(WindowInfo {
                title,
                owner_pid: pid,
                owner_name: owner_name.to_string(),
            });
        }

        CFRelease(windows_value);
        CFRelease(element as CFTypeRef);
    }

    /// Reads one string-valued Accessibility attribute of an element.
    unsafe fn copy_string_attribute(
        element: AXUIElementRef,
        attribute: &'static [u8],
    ) -> Option<String> {
        let attr = cfstring(attribute);
        let mut value: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(element, attr, &mut value);
        CFRelease(attr);
        if err != AX_SUCCESS || value.is_null() {
            return None;
        }
        let string = cfstring_to_string(value as CFStringRef);
        CFRelease(value);
        string
    }

    unsafe fn shared_workspace() -> Id {
        msg_send_id(class_id(b"NSWorkspace\0") as Id, sel(SHARED_WORKSPACE_SEL))
    }

    unsafe fn ns_string(value: &str) -> Id {
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

    unsafe fn optional_nsstring(receiver: Id, selector: Sel) -> Option<String> {
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

    unsafe fn cfstring(bytes: &'static [u8]) -> CFStringRef {
        CFStringCreateWithCString(
            ptr::null(),
            bytes.as_ptr() as *const c_char,
            CFSTRING_ENCODING_UTF8,
        )
    }

    unsafe fn cfstring_to_string(value: CFStringRef) -> Option<String> {
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

    fn sel(bytes: &'static [u8]) -> Sel {
        unsafe { sel_registerName(bytes.as_ptr() as *const c_char) }
    }

    unsafe fn class_id(name: &'static [u8]) -> Class {
        objc_getClass(name.as_ptr() as *const c_char)
    }

    unsafe fn msg_send_id(receiver: Id, selector: Sel) -> Id {
        let function: extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector)
    }

    unsafe fn msg_send_id1(receiver: Id, selector: Sel, arg: Id) -> Id {
        let function: extern "C" fn(Id, Sel, Id) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg)
    }

    unsafe fn msg_send_bool1(receiver: Id, selector: Sel, arg: Id) -> Bool {
        let function: extern "C" fn(Id, Sel, Id) -> Bool =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg)
    }

    unsafe fn msg_send_usize(receiver: Id, selector: Sel) -> usize {
        let function: extern "C" fn(Id, Sel) -> usize =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector)
    }

    unsafe fn msg_send_usize1(receiver: Id, selector: Sel, arg: usize) -> usize {
        let function: extern "C" fn(Id, Sel, usize) -> usize =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg)
    }

    unsafe fn msg_send_i32(receiver: Id, selector: Sel) -> i32 {
        let function: extern "C" fn(Id, Sel) -> i32 =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector)
    }

    const UTF8_STRING_SEL: &[u8] = b"UTF8String\0";
    const STRING_WITH_UTF8_SEL: &[u8] = b"stringWithUTF8String:\0";
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::{WindowInfo, WindowSource};
    use crate::engine::TargetStatus;

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

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
    fn executor_reports_opened_for_unique_match() {
        let executor = MacOSExecutor::with_source(FakeSource(vec![window("B", 100, "App")]));
        let status = executor.focus_window("B");
        // Opening paths (raise + activate) can't run in unit tests without a
        // real window; on macOS the OS call may legitimately fail, so we only
        // assert it is not Unavailable/Ambiguous due to selection.
        assert!(
            matches!(
                status,
                TargetStatus::Opened { .. } | TargetStatus::Failed { .. }
            ),
            "selection must succeed; actual: {status:?}"
        );
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
}
