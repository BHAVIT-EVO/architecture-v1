//! The live macOS [`DesktopSurface`](crate::room::DesktopSurface).
//!
//! Everything here runs on the Accessibility permission Evo already holds
//! for capture — no private Spaces machinery, no SIP changes. The
//! mechanisms are the public ones AeroSpace proved sufficient for
//! workspace management:
//!
//! * app hide/unhide: `NSRunningApplication` (public AppKit, no extra
//!   permission; state preserved, instantly reversible);
//! * window park (minimize): the settable `AXMinimized` attribute;
//! * window raise: the `AXRaise` action;
//! * window identity: the stable system window id via
//!   `_AXUIElementGetWindow` (a private-but-exported HIServices symbol —
//!   the same call AeroSpace and Hammerspoon use; verified to link on
//!   this machine).

use crate::room::{DesktopSurface, RoomApp, RoomWindow};

#[cfg(target_os = "macos")]
mod imp {
    use super::{DesktopSurface, RoomApp, RoomWindow};
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
    type CFBooleanRef = *const c_void;
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
        fn AXUIElementSetAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: CFTypeRef,
        ) -> AXError;
        fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
        fn _AXUIElementGetWindow(element: AXUIElementRef, window_id: *mut u32) -> AXError;
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
        // kCFBooleanTrue / kCFBooleanFalse are global constants on modern
        // macOS (the accessor functions were removed); declared as extern
        // statics.
        static kCFBooleanTrue: CFBooleanRef;
        static kCFBooleanFalse: CFBooleanRef;
        fn CFBooleanGetValue(boolean: CFBooleanRef) -> Bool;
    }

    const KAX_WINDOWS_ATTRIBUTE: &[u8] = b"AXWindows\0";
    const KAX_TITLE_ATTRIBUTE: &[u8] = b"AXTitle\0";
    const KAX_DOCUMENT_ATTRIBUTE: &[u8] = b"AXDocument\0";
    const KAX_MINIMIZED_ATTRIBUTE: &[u8] = b"AXMinimized\0";
    const KAX_RAISE_ACTION: &[u8] = b"AXRaise\0";

    const SHARED_WORKSPACE_SEL: &[u8] = b"sharedWorkspace\0";
    const RUNNING_APPLICATIONS_SEL: &[u8] = b"runningApplications\0";
    const COUNT_SEL: &[u8] = b"count\0";
    const OBJECT_AT_INDEX_SEL: &[u8] = b"objectAtIndex:\0";
    const PROCESS_IDENTIFIER_SEL: &[u8] = b"processIdentifier\0";
    const LOCALIZED_NAME_SEL: &[u8] = b"localizedName\0";
    const IS_HIDDEN_SEL: &[u8] = b"isHidden\0";
    const ACTIVATION_POLICY_SEL: &[u8] = b"activationPolicy\0";
    const HIDE_SEL: &[u8] = b"hide\0";
    const UNHIDE_SEL: &[u8] = b"unhide\0";
    const ACTIVATE_WITH_OPTIONS_SEL: &[u8] = b"activateWithOptions:\0";

    /// `NSApplicationActivateIgnoringOtherApps`.
    const ACTIVATE_IGNORING_OTHER_APPS: usize = 1 << 1;
    /// `NSApplicationActivationPolicyRegular`.
    const ACTIVATION_POLICY_REGULAR: usize = 0;

    // ── DesktopSurface implementation ────────────────────────────────────

    pub fn running_apps() -> Result<Vec<RoomApp>, String> {
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
            let mut out = Vec::new();
            for i in 0..count {
                let app = msg_send_id1(apps, sel(OBJECT_AT_INDEX_SEL), i as Id);
                if app.is_null() {
                    continue;
                }
                let policy = msg_send_usize(app, sel(ACTIVATION_POLICY_SEL));
                let Some(name) = optional_nsstring(app, sel(LOCALIZED_NAME_SEL)) else {
                    continue;
                };
                out.push(RoomApp {
                    pid: msg_send_i32(app, sel(PROCESS_IDENTIFIER_SEL)),
                    name,
                    hidden: msg_send_bool(app, sel(IS_HIDDEN_SEL)) != 0,
                    regular: policy == ACTIVATION_POLICY_REGULAR,
                });
            }
            Ok(out)
        }
    }

    pub fn windows_of(pid: i32) -> Result<Vec<RoomWindow>, String> {
        let mut windows = Vec::new();
        let owner = app_name(pid).unwrap_or_default();
        unsafe {
            with_app_windows(pid, |window| {
                windows.push(RoomWindow {
                    pid,
                    owner: owner.clone(),
                    title: window.title.clone(),
                    ax_document: window.document.clone(),
                    window_id: window.window_id,
                    minimized: window.minimized,
                });
            })
            .map_err(|reason| format!("cannot read windows of {owner}: {reason}"))?;
        }
        Ok(windows)
    }

    pub fn hide_app(pid: i32) -> Result<(), String> {
        with_running_app(pid, |app| unsafe {
            // The BOOL return of `hide`/`unhide` is advisory only: both lie
            // with NO when called from a non-foreground process while the
            // request still lands asynchronously. The room controller's
            // lease is the record of truth, so the return value is ignored
            // here and only a missing application is an error.
            let _ = msg_send_bool(app, sel(HIDE_SEL));
            Ok(())
        })
    }

    pub fn unhide_app(pid: i32) -> Result<(), String> {
        with_running_app(pid, |app| unsafe {
            let _ = msg_send_bool(app, sel(UNHIDE_SEL));
            Ok(())
        })
    }

    pub fn minimize_window(pid: i32, window_id: u32) -> Result<(), String> {
        set_minimized(pid, window_id, true)
    }

    pub fn unminimize_window(pid: i32, window_id: u32) -> Result<(), String> {
        set_minimized(pid, window_id, false)
    }

    pub fn raise_window(pid: i32, window_id: u32) -> Result<(), String> {
        let mut outcome: Option<Result<(), String>> = None;
        unsafe {
            with_app_windows(pid, |window| {
                if outcome.is_none() && window.window_id == window_id && !window.element.is_null() {
                    let raise_action = cfstring(KAX_RAISE_ACTION);
                    let err = AXUIElementPerformAction(window.element, raise_action);
                    CFRelease(raise_action);
                    outcome = Some(if err == AX_SUCCESS {
                        Ok(())
                    } else {
                        Err("the window could not be raised".to_string())
                    });
                }
            })?;
        }
        outcome.unwrap_or_else(|| Err("the window is no longer open".to_string()))
    }

    pub fn activate_app(pid: i32) -> Result<(), String> {
        with_running_app(pid, |app| unsafe {
            let ok = msg_send_usize1(
                app,
                sel(ACTIVATE_WITH_OPTIONS_SEL),
                ACTIVATE_IGNORING_OTHER_APPS,
            );
            if ok != 0 {
                Ok(())
            } else {
                Err("the application could not be activated".to_string())
            }
        })
    }

    // ── Internals ────────────────────────────────────────────────────────

    struct LiveWindow {
        window_id: u32,
        title: String,
        document: Option<String>,
        minimized: bool,
        /// Valid only inside `with_app_windows`, while the owning CFArray
        /// is alive.
        element: AXUIElementRef,
    }

    /// Runs `f` for every window of the application. The elements handed
    /// to `f` are owned by the AXWindows array and die with it, so no
    /// element may escape the closure.
    unsafe fn with_app_windows<F>(pid: i32, mut f: F) -> Result<(), String>
    where
        F: FnMut(&LiveWindow),
    {
        let element = AXUIElementCreateApplication(pid);
        if element.is_null() {
            return Err("cannot address the application".to_string());
        }
        let windows_attr = cfstring(KAX_WINDOWS_ATTRIBUTE);
        let mut windows_value: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(element, windows_attr, &mut windows_value);
        CFRelease(windows_attr);
        if err != AX_SUCCESS || windows_value.is_null() {
            CFRelease(element as CFTypeRef);
            // An app with no windows (or a dying app) is an empty answer,
            // not an error.
            return Ok(());
        }

        let array = windows_value as CFArrayRef;
        let count = CFArrayGetCount(array);
        for i in 0..count {
            let window_element = CFArrayGetValueAtIndex(array, i) as AXUIElementRef;
            let mut window_id: u32 = 0;
            let _ = _AXUIElementGetWindow(window_element, &mut window_id);
            let title =
                copy_string_attribute(window_element, KAX_TITLE_ATTRIBUTE).unwrap_or_default();
            let document = copy_string_attribute(window_element, KAX_DOCUMENT_ATTRIBUTE);
            let minimized = copy_boolean_attribute(window_element, KAX_MINIMIZED_ATTRIBUTE);
            f(&LiveWindow {
                window_id,
                title,
                document,
                minimized,
                element: window_element,
            });
        }

        CFRelease(windows_value);
        CFRelease(element as CFTypeRef);
        Ok(())
    }

    fn set_minimized(pid: i32, window_id: u32, minimized: bool) -> Result<(), String> {
        let mut outcome: Option<Result<(), String>> = None;
        unsafe {
            with_app_windows(pid, |window| {
                if outcome.is_none() && window.window_id == window_id && !window.element.is_null() {
                    let attr = cfstring(KAX_MINIMIZED_ATTRIBUTE);
                    let value = if minimized {
                        kCFBooleanTrue
                    } else {
                        kCFBooleanFalse
                    };
                    let err = AXUIElementSetAttributeValue(window.element, attr, value);
                    CFRelease(attr);
                    outcome = Some(if err == AX_SUCCESS {
                        Ok(())
                    } else {
                        Err("the window could not be parked".to_string())
                    });
                }
            })?;
        }
        outcome.unwrap_or_else(|| Err("the window is no longer open".to_string()))
    }

    /// Finds the `NSRunningApplication` for a pid and runs `f` on it.
    fn with_running_app<F>(pid: i32, f: F) -> Result<(), String>
    where
        F: FnOnce(Id) -> Result<(), String>,
    {
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
                    return f(app);
                }
            }
            Err("the application is no longer running".to_string())
        }
    }

    fn app_name(pid: i32) -> Option<String> {
        unsafe {
            let workspace = shared_workspace();
            if workspace.is_null() {
                return None;
            }
            let apps = msg_send_id(workspace, sel(RUNNING_APPLICATIONS_SEL));
            if apps.is_null() {
                return None;
            }
            let count = msg_send_usize(apps, sel(COUNT_SEL));
            for i in 0..count {
                let app = msg_send_id1(apps, sel(OBJECT_AT_INDEX_SEL), i as Id);
                if !app.is_null() && msg_send_i32(app, sel(PROCESS_IDENTIFIER_SEL)) == pid {
                    return optional_nsstring(app, sel(LOCALIZED_NAME_SEL));
                }
            }
            None
        }
    }

    // ── Objective-C / CoreFoundation plumbing ───────────────────────────

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

    unsafe fn copy_boolean_attribute(element: AXUIElementRef, attribute: &'static [u8]) -> bool {
        let attr = cfstring(attribute);
        let mut value: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(element, attr, &mut value);
        CFRelease(attr);
        if err != AX_SUCCESS || value.is_null() {
            return false;
        }
        let boolean = CFBooleanGetValue(value as CFBooleanRef) != 0;
        CFRelease(value);
        boolean
    }

    unsafe fn shared_workspace() -> Id {
        msg_send_id(class_id(b"NSWorkspace\0") as Id, sel(SHARED_WORKSPACE_SEL))
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

    unsafe fn msg_send_bool(receiver: Id, selector: Sel) -> Bool {
        let function: extern "C" fn(Id, Sel) -> Bool =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector)
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
}

#[cfg(target_os = "macos")]
pub use imp::*;

/// The live macOS desktop surface.
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, Default)]
pub struct MacDesktopSurface;

#[cfg(target_os = "macos")]
impl MacDesktopSurface {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "macos")]
impl DesktopSurface for MacDesktopSurface {
    fn running_apps(&self) -> Result<Vec<RoomApp>, String> {
        imp::running_apps()
    }
    fn windows_of(&self, pid: i32) -> Result<Vec<RoomWindow>, String> {
        imp::windows_of(pid)
    }
    fn hide_app(&mut self, pid: i32) -> Result<(), String> {
        imp::hide_app(pid)
    }
    fn unhide_app(&mut self, pid: i32) -> Result<(), String> {
        imp::unhide_app(pid)
    }
    fn minimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
        imp::minimize_window(pid, window_id)
    }
    fn unminimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
        imp::unminimize_window(pid, window_id)
    }
    fn raise_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
        imp::raise_window(pid, window_id)
    }
    fn activate_app(&mut self, pid: i32) -> Result<(), String> {
        imp::activate_app(pid)
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MacDesktopSurface;

#[cfg(not(target_os = "macos"))]
impl MacDesktopSurface {
    pub fn new() -> Self {
        Self
    }
}
