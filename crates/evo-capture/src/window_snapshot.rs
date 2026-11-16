//! Live macOS window snapshots for the
//! [`WindowRegistry`](evo_observation::window_registry::WindowRegistry).
//!
//! The registry is pure logic: it needs the platform to report every window
//! it can see. This module is the macOS platform half — it enumerates the
//! live window list through `CGWindowListCopyWindowInfo` (public CoreGraphics
//! API) and shapes each entry into the portable
//! [`SnapshotWindow`](evo_observation::window_registry::SnapshotWindow).
//!
//! What each field costs:
//! - window number / owner pid / owner name / layer: a single window-server
//!   round trip for the whole list — no per-window cost, no permission.
//! - bundle id: one `NSWorkspace.runningApplications` walk per snapshot,
//!   matched by pid — no per-window cost.
//! - title: `kCGWindowName`, which macOS hides without Screen Recording
//!   permission — may be empty. Titles are display-only; identity never uses
//!   them, and the focused window's authoritative title comes from the AX
//!   focus event itself.
//!
//! This snapshot is native-window provenance only. It deliberately carries no
//! browser profile/session/tab fields; those may enter capture only through an
//! explicitly installed direct browser-identity provider.
//!
//! Focus is resolved structurally by the caller
//! ([`select_focused`](evo_observation::window_registry::select_focused) —
//! frontmost layer-0 window of the focused pid), so no per-window
//! Accessibility reads are needed here at all.

use evo_observation::window_registry::SnapshotWindow;

/// Enumerates the live window list, front-to-back, as portable snapshot records.
///
/// Only normal windows (layer 0) are reported: menus, popups, the dock, and
/// floating panels live on other layers and are not focusable work surfaces.
/// Entries the platform cannot key (no window number or no owner pid) are
/// skipped — without a key there is no identity.
///
/// # Errors
///
/// Returns an error when the window list cannot be obtained at all. A failed
/// snapshot is honest unavailability: the caller records no window rather
/// than guessing.
pub fn snapshot_windows() -> Result<Vec<SnapshotWindow>, String> {
    imp::snapshot_windows()
}

/// Builds the registry's opaque platform key for one window.
///
/// `Some(bundle_id)` → `"bundle-id:window-number"`; `None`/empty → the
/// `"pid:window-number"` fallback, which cannot collide within a snapshot.
/// The registry never parses the key — it only needs stability for the
/// window's lifetime.
fn platform_key(bundle_id: Option<&str>, owner_pid: i32, window_number: i64) -> String {
    match bundle_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(bundle) => format!("{bundle}:{window_number}"),
        None => format!("pid:{owner_pid}:{window_number}"),
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{SnapshotWindow, platform_key};
    use std::collections::HashMap;
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_void};

    type Bool = i8;
    type Id = *mut Object;
    type Class = *mut objc_class;
    type Sel = *mut c_void;
    type CFTypeRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFArrayRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFIndex = isize;
    type CFTypeID = usize;

    #[repr(C)]
    pub struct Object {
        _priv: [u8; 0],
    }

    #[repr(C)]
    pub struct objc_class {
        _priv: [u8; 0],
    }

    // CGWindowListOption and CGWindowID are both uint32_t.
    const KCG_NULL_WINDOW_ID: u32 = 0;
    const KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1 << 0;
    const KCG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
    const KCF_NUMBER_SINT64_TYPE: i32 = 4;
    const CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;

    #[allow(clippy::duplicated_attributes)]
    #[link(name = "objc")]
    #[link(name = "Foundation", kind = "framework")]
    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "CoreGraphics", kind = "framework")]
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Class;
        fn objc_msgSend();
        fn sel_registerName(name: *const c_char) -> Sel;

        fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFArrayRef;
        fn CFArrayGetCount(the_array: CFArrayRef) -> CFIndex;
        fn CFArrayGetValueAtIndex(the_array: CFArrayRef, index: CFIndex) -> *const c_void;
        fn CFDictionaryGetValue(the_dict: CFDictionaryRef, key: *const c_void) -> *const c_void;
        fn CFNumberGetValue(number: CFTypeRef, number_type: i32, value: *mut c_void) -> Bool;
        fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;
        fn CFNumberGetTypeID() -> CFTypeID;
        fn CFStringGetTypeID() -> CFTypeID;
        fn CFStringGetLength(the_string: CFStringRef) -> isize;
        fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
        fn CFStringGetCString(
            the_string: CFStringRef,
            buffer: *mut c_char,
            buffer_size: isize,
            encoding: u32,
        ) -> Bool;
        fn CFRelease(cf: CFTypeRef);

        static kCGWindowNumber: CFStringRef;
        static kCGWindowOwnerPID: CFStringRef;
        static kCGWindowOwnerName: CFStringRef;
        static kCGWindowName: CFStringRef;
        static kCGWindowLayer: CFStringRef;
    }

    pub fn snapshot_windows() -> Result<Vec<SnapshotWindow>, String> {
        // The bundle-id map is built once per snapshot: one NSWorkspace walk,
        // not one lookup per window.
        let bundle_ids = running_app_bundle_ids();
        unsafe {
            let list = CGWindowListCopyWindowInfo(
                KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | KCG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS,
                KCG_NULL_WINDOW_ID,
            );
            if list.is_null() {
                return Err("CGWindowListCopyWindowInfo returned no window list".to_string());
            }
            let count = CFArrayGetCount(list);
            let mut out = Vec::new();
            for i in 0..count {
                let dict = CFArrayGetValueAtIndex(list, i) as CFDictionaryRef;
                if dict.is_null() {
                    continue;
                }
                // Layer 0 is a normal window. Anything else (menus, popups,
                // the dock, floating panels) is not a focusable work surface.
                // A missing layer is treated as normal — fail open for
                // identity: excluding a window the platform could not classify
                // would erase a real window, while including it is harmless.
                if let Some(window) = snapshot_entry(dict, &bundle_ids)
                    && window.layer == 0
                {
                    out.push(window);
                }
            }
            CFRelease(list as CFTypeRef);
            Ok(out)
        }
    }

    /// Shapes one window-list dictionary into a portable snapshot record.
    ///
    /// Returns `None` for entries without a stable key (no window number or
    /// no owner pid): without a key there is no identity, and guessing
    /// identity is how windows get merged.
    unsafe fn snapshot_entry(
        dict: CFDictionaryRef,
        bundle_ids: &HashMap<i32, String>,
    ) -> Option<SnapshotWindow> {
        unsafe {
            let number = dict_i64(dict, kCGWindowNumber)?;
            let owner_pid = i32::try_from(dict_i64(dict, kCGWindowOwnerPID)?).ok()?;
            let layer = dict_i64(dict, kCGWindowLayer).unwrap_or(0);
            let owner_name = dict_string(dict, kCGWindowOwnerName).unwrap_or_default();
            // May be empty without Screen Recording permission — display-only.
            let title = dict_string(dict, kCGWindowName).unwrap_or_default();
            let key = platform_key(
                bundle_ids.get(&owner_pid).map(String::as_str),
                owner_pid,
                number,
            );
            Some(SnapshotWindow {
                platform_key: key,
                app_name: owner_name,
                title,
                layer,
                owner_pid,
            })
        }
    }

    unsafe fn dict_value(dict: CFDictionaryRef, key: CFStringRef) -> Option<CFTypeRef> {
        unsafe {
            let value = CFDictionaryGetValue(dict, key as *const c_void);
            if value.is_null() {
                None
            } else {
                Some(value as CFTypeRef)
            }
        }
    }

    unsafe fn dict_i64(dict: CFDictionaryRef, key: CFStringRef) -> Option<i64> {
        unsafe {
            let value = dict_value(dict, key)?;
            if CFGetTypeID(value) != CFNumberGetTypeID() {
                return None;
            }
            let mut number: i64 = 0;
            if CFNumberGetValue(
                value,
                KCF_NUMBER_SINT64_TYPE,
                &mut number as *mut i64 as *mut c_void,
            ) == 0
            {
                return None;
            }
            Some(number)
        }
    }

    unsafe fn dict_string(dict: CFDictionaryRef, key: CFStringRef) -> Option<String> {
        unsafe {
            let value = dict_value(dict, key)?;
            if CFGetTypeID(value) != CFStringGetTypeID() {
                return None;
            }
            cfstring_to_string(value as CFStringRef)
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

    /// Maps owner pid → bundle id over the running applications.
    ///
    /// The window list reports pids, not bundle ids; pids are reused after an
    /// app quits, so the platform key uses the bundle id (stable across
    /// launches) with a pid fallback for processes that expose none.
    fn running_app_bundle_ids() -> HashMap<i32, String> {
        unsafe {
            let workspace = msg_send_id(class_id(c"NSWorkspace") as Id, sel(b"sharedWorkspace\0"));
            if workspace.is_null() {
                return HashMap::new();
            }
            let apps = msg_send_id(workspace, sel(b"runningApplications\0"));
            if apps.is_null() {
                return HashMap::new();
            }
            let count = msg_send_usize(apps, sel(b"count\0"));
            let mut out = HashMap::new();
            for i in 0..count {
                let app = msg_send_id1(apps, sel(b"objectAtIndex:\0"), i as Id);
                if app.is_null() {
                    continue;
                }
                let pid = msg_send_i32(app, sel(b"processIdentifier\0"));
                let bundle = msg_send_id(app, sel(b"bundleIdentifier\0"));
                if let Some(id) = nsstring_to_string(bundle)
                    && !id.trim().is_empty()
                {
                    out.insert(pid, id);
                }
            }
            out
        }
    }

    unsafe fn nsstring_to_string(value: Id) -> Option<String> {
        unsafe {
            if value.is_null() {
                return None;
            }
            let cstr = msg_send_id(value, sel(b"UTF8String\0"));
            if cstr.is_null() {
                return None;
            }
            let bytes = CStr::from_ptr(cstr as *const c_char);
            Some(bytes.to_string_lossy().into_owned())
        }
    }

    fn sel(bytes: &'static [u8]) -> Sel {
        unsafe { sel_registerName(bytes.as_ptr() as *const c_char) }
    }

    unsafe fn class_id(name: &'static CStr) -> Class {
        unsafe { objc_getClass(name.as_ptr()) }
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

    unsafe fn msg_send_usize(receiver: Id, selector: Sel) -> usize {
        unsafe {
            let function: extern "C" fn(Id, Sel) -> usize =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector)
        }
    }

    unsafe fn msg_send_i32(receiver: Id, selector: Sel) -> i32 {
        unsafe {
            let function: extern "C" fn(Id, Sel) -> i32 =
                std::mem::transmute(objc_msgSend as *const ());
            function(receiver, selector)
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::SnapshotWindow;

    pub fn snapshot_windows() -> Result<Vec<SnapshotWindow>, String> {
        Err("live window snapshots are only supported on macOS".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::platform_key;

    #[test]
    fn platform_key_prefers_the_bundle_id() {
        assert_eq!(
            platform_key(Some("com.example.app"), 100, 42),
            "com.example.app:42"
        );
    }

    #[test]
    fn platform_key_falls_back_to_pid_without_a_bundle_id() {
        assert_eq!(platform_key(None, 100, 42), "pid:100:42");
        assert_eq!(platform_key(Some(""), 100, 42), "pid:100:42");
        assert_eq!(platform_key(Some("   "), 100, 42), "pid:100:42");
    }
}
