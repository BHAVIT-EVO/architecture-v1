//! Live macOS producer for `URLNavigated`.
//!
//! This source witnesses navigations through Accessibility: it reads the URL
//! of the focused web content of the frontmost application and emits a
//! `URLNavigated` signal only when that URL changes. A URL transition is the
//! directly observable manifestation of a navigation; the old and new values
//! are both witnessed, never guessed.
//!
//! The signal's subject is the new URL — the platform-independent canonical
//! form required by the frozen OBS-URL-NAVIGATED schema. No browser, bundle
//! identifier, process, or tab identity is recorded, and none is required.
//! Applications that expose no web-area URL simply produce no signal.
//!
//! This source never interprets, ranks, or decides importance. It is also
//! careful not to emit on startup: only a witnessed transition from one URL
//! to another constitutes a navigation signal.

use crate::adapters::macos::MacOSSignal;
use crate::macos_event_source::MacOSEventSourceError;

use std::collections::HashMap;
use std::ffi::c_char;
use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

#[cfg(target_os = "macos")]
mod imp {
    use super::*;

    type Bool = i8;
    type Id = *mut Object;
    type Class = *mut objc_class;
    type Sel = *mut c_void;
    type CFTypeRef = *const c_void;
    type CFArrayRef = *const c_void;
    type CFStringRef = *const c_void;
    type AXUIElementRef = *const c_void;
    type AXError = i32;

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
        fn CFArrayGetCount(array: CFArrayRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CFArrayRef, index: isize) -> *const c_void;
        fn CFStringGetLength(value: CFStringRef) -> isize;
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
        fn CFRelease(cf: CFTypeRef);
        fn CFGetTypeID(cf: CFTypeRef) -> usize;
        fn CFStringGetTypeID() -> usize;
    }

    const SHARED_WORKSPACE_SEL: &[u8] = b"sharedWorkspace\0";
    const FRONTMOST_APPLICATION_SEL: &[u8] = b"frontmostApplication\0";
    const PROCESS_IDENTIFIER_SEL: &[u8] = b"processIdentifier\0";
    const NSWORKSPACE_CLASS: &[u8] = b"NSWorkspace\0";

    const AX_FOCUSED_WINDOW: &[u8] = b"AXFocusedWindow\0";
    const AX_CHILDREN: &[u8] = b"AXChildren\0";
    const AX_CONTENTS: &[u8] = b"AXContents\0";
    const AX_ROLE: &[u8] = b"AXRole\0";
    const AX_URL: &[u8] = b"AXURL\0";
    const AX_DOCUMENT: &[u8] = b"AXDocument\0";
    const AX_WEB_AREA_ROLE: &[u8] = b"AXWebArea\0";

    fn ax_attr(attribute: &'static [u8]) -> CFStringRef {
        // Each attribute is created once and lives for the process lifetime,
        // matching the event source's static attribute cache.
        match attribute {
            AX_FOCUSED_WINDOW => ax_focused_window(),
            AX_CHILDREN => ax_children(),
            AX_CONTENTS => ax_contents(),
            AX_ROLE => ax_role(),
            AX_URL => ax_url(),
            AX_DOCUMENT => ax_document(),
            _ => unreachable!("unknown attribute"),
        }
    }

    fn create_cfstring(bytes: &'static [u8]) -> CFStringRef {
        unsafe {
            CFStringCreateWithCString(ptr::null(), bytes.as_ptr() as *const c_char, CFSTRING_ENCODING_UTF8)
        }
    }

    fn ax_focused_window() -> CFStringRef {
        static VALUE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_FOCUSED_WINDOW) as usize) as CFStringRef
    }

    fn ax_children() -> CFStringRef {
        static VALUE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_CHILDREN) as usize) as CFStringRef
    }

    fn ax_contents() -> CFStringRef {
        static VALUE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_CONTENTS) as usize) as CFStringRef
    }

    fn ax_role() -> CFStringRef {
        static VALUE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_ROLE) as usize) as CFStringRef
    }

    fn ax_url() -> CFStringRef {
        static VALUE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_URL) as usize) as CFStringRef
    }

    fn ax_document() -> CFStringRef {
        static VALUE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_DOCUMENT) as usize) as CFStringRef
    }

    /// Keepalive for the live URL poller thread.
    pub struct MacOSURLPoller {
        stop: Arc<AtomicBool>,
        handle: Option<JoinHandle<()>>,
    }

    impl MacOSURLPoller {
        /// Starts a background thread that samples the frontmost application's
        /// focused web-area URL and emits `URLNavigated` on witnessed change.
        pub fn start<F>(callback: F) -> Result<Self, MacOSEventSourceError>
        where
            F: FnMut(MacOSSignal) + Send + 'static,
        {
            let stop = Arc::new(AtomicBool::new(false));
            let stop_flag = Arc::clone(&stop);
            let handle = thread::spawn(move || {
                let mut callback = callback;
                let mut last_url_by_pid: HashMap<i32, String> = HashMap::new();
                // Evo's own UI (and this process) is never polled: walking
                // the shell's own accessibility tree both observes ourselves
                // and — measured — aborts the process when the egui
                // accessibility tree is mid-update (a foreign exception no
                // Rust boundary can catch).
                let self_pid = std::process::id() as i32;
                let desktop_pid = crate::desktop_shell_pid();
                while !stop_flag.load(Ordering::Relaxed) {
                    if let Some(front) = frontmost_pid() {
                        if front == self_pid || Some(front) == desktop_pid {
                            thread::sleep(Duration::from_secs(2));
                            continue;
                        }
                    }
                    match focused_web_url() {
                        Some((pid, url)) => {
                            // A navigation is emitted only when a transition
                            // from one URL to another was witnessed. The first
                            // observation of a pid (daemon start, or switching
                            // back to an application) records a baseline and
                            // emits nothing: the URL may have changed while we
                            // were not watching, so no navigation was witnessed.
                            let transition =
                                witnessed_url_transition(last_url_by_pid.get(&pid).map(String::as_str), &url);
                            last_url_by_pid.insert(pid, url.clone());
                            if transition {
                                eprintln!("EVO-CAPTURE url: {url}");
                                callback(MacOSSignal::URLNavigated {
                                    subject: url,
                                    observed_at: SystemTime::now(),
                                });
                            }
                        }
                        None => {}
                    }
                    thread::sleep(Duration::from_secs(2));
                }
            });
            Ok(Self {
                stop,
                handle: Some(handle),
            })
        }
    }

    impl Drop for MacOSURLPoller {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    /// Reads the focused web-area URL of the frontmost application.
    ///
    /// Returns `(pid, url)`. The pid is used only internally to distinguish
    /// which browsing context the URL belongs to; it never enters the
    /// canonical signal.
    fn focused_web_url() -> Option<(i32, String)> {
        let pid = frontmost_pid()?;
        let url = unsafe {
            let app = AXUIElementCreateApplication(pid);
            if app.is_null() {
                return None;
            }
            let url = focused_window_url(app);
            CFRelease(app);
            url
        };
        url.map(|url| (pid, url))
    }

    /// The pid of the frontmost application, via NSWorkspace (the same
    /// witness the focus event source uses).
    fn frontmost_pid() -> Option<i32> {
        unsafe {
            let workspace = msg_send_id(
                objc_getClass(NSWORKSPACE_CLASS.as_ptr() as *const c_char) as Id,
                sel_from_bytes(SHARED_WORKSPACE_SEL),
            );
            if workspace.is_null() {
                return None;
            }
            let application = msg_send_id(workspace, sel_from_bytes(FRONTMOST_APPLICATION_SEL));
            if application.is_null() {
                return None;
            }
            Some(msg_send_i32(
                application,
                sel_from_bytes(PROCESS_IDENTIFIER_SEL),
            ))
        }
    }

    fn sel_from_bytes(bytes: &'static [u8]) -> Sel {
        unsafe { sel_registerName(bytes.as_ptr() as *const c_char) }
    }

    unsafe fn msg_send_id(receiver: Id, selector: Sel) -> Id {
        let raw = objc_msgSend as *const ();
        let function: extern "C" fn(Id, Sel) -> Id = std::mem::transmute(raw);
        function(receiver, selector)
    }

    unsafe fn msg_send_i32(receiver: Id, selector: Sel) -> i32 {
        let raw = objc_msgSend as *const ();
        let function: extern "C" fn(Id, Sel) -> i32 = std::mem::transmute(raw);
        function(receiver, selector)
    }

    unsafe fn focused_window_url(app: AXUIElementRef) -> Option<String> {
        unsafe {
            let mut window_value: CFTypeRef = ptr::null();
            if AXUIElementCopyAttributeValue(app, ax_attr(AX_FOCUSED_WINDOW), &mut window_value)
                != AX_SUCCESS
                || window_value.is_null()
            {
                return None;
            }

            let window = window_value as AXUIElementRef;
            // Two independent URL witnesses are accepted, both direct URL
            // attributes from the Accessibility tree — never a window title
            // and never a guess:
            //
            // 1. A real AXWebArea element whose AXURL is an http(s) URL (the
            //    focused web content, exposed by browsers that surface their
            //    web-area tree).
            // 2. The focused window's AXDocument attribute when it is an
            //    http(s) URL. Browsers such as Google Chrome and Safari
            //    expose the URL of the document being viewed there even when
            //    they do not surface a readable web-area tree. The window's
            //    own AXURL is deliberately never read: browsers report the
            //    window *title* there (e.g. "Example Domain - Google Chrome
            //    – …"), which is not a URL and must never be emitted.
            let found = find_web_area_url(window, 0).or_else(|| window_document_url(window));
            CFRelease(window_value);
            found
        }
    }

    /// The focused window's `AXDocument` value, when it is a real http(s)
    /// URL. `AXDocument` is the URL of the document the window is viewing, a
    /// directly witnessed fact — not a title and not inferred.
    unsafe fn window_document_url(window: AXUIElementRef) -> Option<String> {
        let value = copy_string_attribute(window, AX_DOCUMENT)?;
        if is_http_url(&value) {
            Some(value)
        } else {
            None
        }
    }

    /// Walks the accessibility tree (children, then contents) for a web area
    /// whose AXURL is a real http(s) URL. Depth and breadth are bounded so a
    /// hostile or broken tree cannot stall capture.
    unsafe fn find_web_area_url(element: AXUIElementRef, depth: usize) -> Option<String> {
        if depth > 4 {
            return None;
        }
        if role_is_web_area(element) {
            if let Some(url) = copy_string_attribute(element, AX_URL) {
                if is_http_url(&url) {
                    return Some(url);
                }
            }
            return None;
        }

        let mut children_value: CFTypeRef = ptr::null();
        if AXUIElementCopyAttributeValue(element, ax_attr(AX_CHILDREN), &mut children_value)
            == AX_SUCCESS
            && !children_value.is_null()
        {
            let array = children_value as CFArrayRef;
            let count = CFArrayGetCount(array).min(32);
            for index in 0..count {
                let item = CFArrayGetValueAtIndex(array, index) as AXUIElementRef;
                if let Some(url) = find_web_area_url(item, depth + 1) {
                    CFRelease(children_value);
                    return Some(url);
                }
            }
            CFRelease(children_value);
            return None;
        }

        let mut contents_value: CFTypeRef = ptr::null();
        if AXUIElementCopyAttributeValue(element, ax_attr(AX_CONTENTS), &mut contents_value)
            == AX_SUCCESS
            && !contents_value.is_null()
        {
            let array = contents_value as CFArrayRef;
            let count = CFArrayGetCount(array).min(32);
            for index in 0..count {
                let item = CFArrayGetValueAtIndex(array, index) as AXUIElementRef;
                if let Some(url) = find_web_area_url(item, depth + 1) {
                    CFRelease(contents_value);
                    return Some(url);
                }
            }
            CFRelease(contents_value);
        }
        None
    }

    fn is_http_url(value: &str) -> bool {
        value.starts_with("https://") || value.starts_with("http://")
    }

    /// Whether a URL observation is a witnessed navigation.
    ///
    /// A navigation signal is emitted only when the same pid was previously
    /// observed with a *different* URL — a transition from one URL to another
    /// was directly witnessed. The first observation of a pid is a baseline,
    /// never a navigation: the URL may have changed while Evo was not
    /// watching that application, so emitting would claim a witness that did
    /// not occur.
    fn witnessed_url_transition(prior: Option<&str>, url: &str) -> bool {
        matches!(prior, Some(prior) if prior != url)
    }

    unsafe fn role_is_web_area(element: AXUIElementRef) -> bool {
        copy_string_attribute(element, AX_ROLE).as_deref() == Some("AXWebArea")
    }

    unsafe fn copy_string_attribute(
        element: AXUIElementRef,
        attribute: &'static [u8],
    ) -> Option<String> {
        unsafe {
            let mut value: CFTypeRef = ptr::null();
            if AXUIElementCopyAttributeValue(element, ax_attr(attribute), &mut value) != AX_SUCCESS
                || value.is_null()
            {
                return None;
            }
            // The attribute value is a CFTypeRef: apps return non-strings
            // for URL-ish attributes (URL objects, AXValues). Reading a
            // non-string as a CFString is a foreign exception no Rust
            // boundary can catch — measured as a daemon abort on Safari's
            // tree. Only a genuine CFString (NSString toll-free included)
            // is read; anything else is honestly absent.
            let is_string = CFGetTypeID(value) == CFStringGetTypeID();
            let text = if is_string {
                cfstring_to_string(value as CFStringRef)
            } else {
                None
            };
            CFRelease(value);
            text
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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn http_urls_are_accepted() {
            assert!(is_http_url("https://example.com/doc"));
            assert!(is_http_url("http://example.com"));
        }

        #[test]
        fn non_urls_and_other_schemes_are_rejected() {
            assert!(!is_http_url("Example Domain - Google Chrome – BHAVIT"));
            assert!(!is_http_url("Example Domain"));
            assert!(!is_http_url(""));
            assert!(!is_http_url("file:///tmp/x"));
            assert!(!is_http_url("about:blank"));
            assert!(!is_http_url("javascript:void(0)"));
        }

        #[test]
        fn title_shaped_window_axurl_is_rejected() {
            // This is the exact value Chrome exposes as the window's AXURL;
            // it must never be treated as a navigation.
            let title = "Example Domain - Google Chrome – BHAVIT";
            assert!(!is_http_url(title));
        }

        #[test]
        fn first_observation_of_a_pid_is_never_a_navigation() {
            // Daemon start or an app-switch back to a browser: no transition
            // was witnessed, so no navigation signal may be emitted — the
            // URL may have changed while we were not watching it.
            assert!(!witnessed_url_transition(None, "https://example.com/doc"));
        }

        #[test]
        fn a_changed_url_is_a_witnessed_transition() {
            assert!(witnessed_url_transition(
                Some("https://example.com/one"),
                "https://example.com/two"
            ));
        }

        #[test]
        fn an_unchanged_url_is_not_a_navigation() {
            assert!(!witnessed_url_transition(
                Some("https://example.com/doc"),
                "https://example.com/doc"
            ));
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::*;

    pub struct MacOSURLPoller;

    impl MacOSURLPoller {
        pub fn start<F>(_callback: F) -> Result<Self, MacOSEventSourceError>
        where
            F: FnMut(MacOSSignal) + Send + 'static,
        {
            Err(MacOSEventSourceError::UnsupportedPlatform)
        }
    }
}

pub use imp::MacOSURLPoller;
