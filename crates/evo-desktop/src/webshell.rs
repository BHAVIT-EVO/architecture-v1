//! The web UI inside the app window (ADR 0002).
//!
//! The new interface is HTML served from the bundle over one custom scheme,
//! `evo-app://app/…`, with navigation restricted to that scheme and reality
//! handed in as one read-only document-start script. The egui interface is
//! untouched; `EVO_UI=web` selects this shell, `native` (the default) selects
//! the old one.
//!
//! The rules that make this safe are in the pure functions below, which are
//! tested without a window: `resolve` (path containment), `navigation_allowed`
//! (scheme and host), `mime_for`, and `works_json` (the bridge payload).

use std::path::{Component, Path, PathBuf};

/// The only scheme the window may load.
pub const SCHEME: &str = "evo-app";
/// The only host under that scheme.
pub const HOST: &str = "app";
/// What the window opens.
pub const START_URL: &str = "evo-app://app/app/home.html?shell=1&entry=off";

/// The start URL, with an optional `&panel=` for QA shots. The panel name is
/// from a fixed set; anything else is ignored.
fn start_url() -> String {
    match std::env::var("EVO_START_PANEL").ok().as_deref() {
        Some(panel @ ("activity" | "permissions" | "data" | "diagnostics" | "about"
        | "appearance" | "settings" | "tray")) => format!("{START_URL}&panel={panel}"),
        _ => START_URL.to_string(),
    }
}

/// The directory the scheme handler serves from: the bundle's `Resources/ui`
/// when installed, the crate's own `ui/` in development.
pub fn assets_root() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let bundled = dir.join("../Resources/ui");
        if bundled.is_dir() {
            return bundled;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui")
}

/// Percent-decodes one URL path segment run. Invalid escapes are kept as-is;
/// a NUL or a backslash is refused because it means an escaped path.
fn decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
                let value = u8::from_str_radix(hex, 16).ok()?;
                if value == 0 {
                    return None;
                }
                out.push(value);
                i += 3;
            }
            b'%' => return None,
            b'\\' => return None,
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Maps a request path (`/app/home.js`, `…?v=1`) to a file under `root`.
///
/// Returns `None` for anything that is not a regular file inside the
/// canonicalised root: `..`, encoded traversal, absolute paths, symlinks that
/// leave the root, directories, and empty paths.
pub fn resolve(root: &Path, request_path: &str) -> Option<PathBuf> {
    let path_only = request_path.split(['?', '#']).next().unwrap_or("");
    let decoded = decode(path_only)?;
    let trimmed = decoded.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let relative = Path::new(trimmed);
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    let canonical_root = root.canonicalize().ok()?;
    let joined = canonical_root.join(relative);
    let canonical = joined.canonicalize().ok()?;
    if !canonical.starts_with(&canonical_root) || !canonical.is_file() {
        return None;
    }
    Some(canonical)
}

/// The one navigation policy: `evo-app://app/…` and nothing else.
pub fn navigation_allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix(SCHEME) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("://") else {
        return false;
    };
    let (host, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    host == HOST && (path.starts_with('/') || path.is_empty())
}

/// The content type the handler answers with.
pub fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html",
        "js" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[allow(dead_code)]
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("evo-webshell-{}", std::process::id()));
        let _ = fs::create_dir_all(dir.join("app"));
        fs::write(dir.join("app/home.html"), "<!doctype html>").unwrap();
        fs::write(dir.join("tokens.css"), "body{}").unwrap();
        dir
    }

    #[test]
    fn serves_files_inside_the_root_only() {
        let root = root();
        assert!(resolve(&root, "/app/home.html").is_some());
        assert_eq!(
            resolve(&root, "/app/home.html?v=2")
                .unwrap()
                .file_name()
                .unwrap(),
            "home.html"
        );
        assert!(resolve(&root, "/tokens.css").is_some());
        assert!(resolve(&root, "/").is_none(), "a directory is not a page");
        assert!(resolve(&root, "/app/missing.js").is_none());
    }

    #[test]
    fn refuses_traversal_encoded_or_plain() {
        let root = root();
        assert!(resolve(&root, "/../Cargo.toml").is_none());
        assert!(resolve(&root, "/app/../../Cargo.toml").is_none());
        assert!(resolve(&root, "/%2e%2e/Cargo.toml").is_none());
        assert!(resolve(&root, "/app/%2E%2E/%2E%2E/Cargo.toml").is_none());
        assert!(resolve(&root, "/..%2fCargo.toml").is_none());
        assert!(resolve(&root, "/app/home.html%00.png").is_none());
        assert!(resolve(&root, "/%2Fetc%2Fpasswd").is_none());
    }

    #[test]
    fn refuses_a_symlink_that_leaves_the_root() {
        let root = root();
        let link = root.join("escape.css");
        let _ = fs::remove_file(&link);
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/hosts", &link).unwrap();
        #[cfg(unix)]
        assert!(resolve(&root, "/escape.css").is_none());
    }

    #[test]
    fn navigation_is_the_app_scheme_and_host_only() {
        assert!(navigation_allowed("evo-app://app/app/home.html?shell=1"));
        assert!(navigation_allowed("evo-app://app/"));
        assert!(!navigation_allowed("https://example.com"));
        assert!(!navigation_allowed("http://example.com/"));
        assert!(!navigation_allowed("file:///etc/passwd"));
        assert!(!navigation_allowed(
            "file:///Users/me/Desktop/Evo.app/Contents/Resources/ui/app/home.html"
        ));
        assert!(!navigation_allowed("evo-app://evil/app/home.html"));
        assert!(!navigation_allowed("evo-app://"));
        assert!(!navigation_allowed("javascript:alert(1)"));
        assert!(!navigation_allowed("data:text/html,<h1>x</h1>"));
        assert!(!navigation_allowed("about:blank"));
    }

    #[test]
    fn content_types_are_explicit() {
        assert_eq!(mime_for(Path::new("a.html")), "text/html");
        assert_eq!(mime_for(Path::new("a.js")), "text/javascript");
        assert_eq!(mime_for(Path::new("a.woff2")), "font/woff2");
        assert_eq!(mime_for(Path::new("a.unknown")), "application/octet-stream");
    }
}

use std::sync::{Mutex, OnceLock};

/// Messages the page has sent and Rust has not read yet. Filled by the
/// `WKScriptMessageHandler` on the main thread, drained in `WebUiApp::ui`.
fn inbox() -> &'static Mutex<Vec<Vec<u8>>> {
    static INBOX: OnceLock<Mutex<Vec<Vec<u8>>>> = OnceLock::new();
    INBOX.get_or_init(|| Mutex::new(Vec::new()))
}

/// Takes every message received since the last call.
pub fn take_messages() -> Vec<Vec<u8>> {
    let Ok(mut queue) = inbox().lock() else {
        eprintln!("EVO-WEB bridge inbox is unavailable because its lock is poisoned");
        return Vec::new();
    };
    std::mem::take(&mut *queue)
}

#[cfg(target_os = "macos")]
pub use macos::WebShell;

#[cfg(not(target_os = "macos"))]
pub struct WebShell;

#[cfg(not(target_os = "macos"))]
impl WebShell {
    pub fn new() -> Self {
        WebShell
    }
    pub fn sync_frame(&mut self) {}
    pub fn set_appearance(&self, _appearance: &str) {}
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::os::raw::{c_char, c_void};
    use std::sync::Once;

    type Bool = i8;
    type Id = *mut Object;
    type Class = *mut c_void;
    type Sel = *mut c_void;

    #[repr(C)]
    pub struct Object {
        _priv: [u8; 0],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NsRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    }

    #[link(name = "objc")]
    #[link(name = "Foundation", kind = "framework")]
    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "WebKit", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Class;
        fn objc_msgSend();
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_allocateClassPair(superclass: Class, name: *const c_char, extra: usize) -> Class;
        fn objc_registerClassPair(class: Class);
        fn class_addMethod(
            class: Class,
            selector: Sel,
            imp: *const c_void,
            types: *const c_char,
        ) -> Bool;
        fn class_addProtocol(class: Class, protocol: *mut c_void) -> Bool;
        fn objc_getProtocol(name: *const c_char) -> *mut c_void;
    }

    fn sel(bytes: &'static [u8]) -> Sel {
        unsafe { sel_registerName(bytes.as_ptr() as *const c_char) }
    }

    unsafe fn class(name: &[u8]) -> Class {
        unsafe {
            let mut bytes = name.to_vec();
            bytes.push(0);
            objc_getClass(bytes.as_ptr() as *const c_char)
        }
    }

    unsafe fn send0(receiver: Id, selector: Sel) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector)
        }
    }
    unsafe fn send1(receiver: Id, selector: Sel, a: Id) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, a)
        }
    }
    unsafe fn send2(receiver: Id, selector: Sel, a: Id, b: Id) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, Id, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, a, b)
        }
    }
    unsafe fn send3(receiver: Id, selector: Sel, a: Id, b: Id, c: Id) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, Id, Id, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, a, b, c)
        }
    }
    unsafe fn send_void0(receiver: Id, selector: Sel) {
        unsafe {
            let f: extern "C" fn(Id, Sel) = std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector);
        }
    }
    unsafe fn send_void1(receiver: Id, selector: Sel, a: Id) {
        unsafe {
            let f: extern "C" fn(Id, Sel, Id) = std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, a);
        }
    }
    unsafe fn send_int1(receiver: Id, selector: Sel, a: isize) {
        unsafe {
            let f: extern "C" fn(Id, Sel, isize) = std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, a);
        }
    }
    unsafe fn send_bool(receiver: Id, selector: Sel) -> Bool {
        unsafe {
            let f: extern "C" fn(Id, Sel) -> Bool = std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector)
        }
    }
    unsafe fn send_rect(receiver: Id, selector: Sel, rect: NsRect) {
        unsafe {
            let f: extern "C" fn(Id, Sel, NsRect) = std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector, rect);
        }
    }
    unsafe fn send_bounds(receiver: Id) -> NsRect {
        unsafe {
            let f: extern "C" fn(Id, Sel) -> NsRect =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, sel(b"bounds\0"))
        }
    }
    unsafe fn send_user_script(receiver: Id, source: Id, at_start: isize, main_only: Bool) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, Id, isize, Bool) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(
                receiver,
                sel(b"initWithSource:injectionTime:forMainFrameOnly:\0"),
                source,
                at_start,
                main_only,
            )
        }
    }
    unsafe fn send_response(receiver: Id, url: Id, mime: Id, length: i64, encoding: Id) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, Id, Id, i64, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(
                receiver,
                sel(b"initWithURL:MIMEType:expectedContentLength:textEncodingName:\0"),
                url,
                mime,
                length,
                encoding,
            )
        }
    }
    unsafe fn send_data(bytes: *const c_void, length: usize) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, *const c_void, usize) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(
                class(b"NSData") as Id,
                sel(b"dataWithBytes:length:\0"),
                bytes,
                length,
            )
        }
    }
    unsafe fn send_init_frame(receiver: Id, frame: NsRect) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, NsRect) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, sel(b"initWithFrame:\0"), frame)
        }
    }
    unsafe fn send_init_frame_config(receiver: Id, frame: NsRect, config: Id) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, NsRect, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(
                receiver,
                sel(b"initWithFrame:configuration:\0"),
                frame,
                config,
            )
        }
    }
    unsafe fn send_number_bool(value: Bool) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, Bool) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(class(b"NSNumber") as Id, sel(b"numberWithBool:\0"), value)
        }
    }
    unsafe fn send_set_value(target: Id, key: Id, value: Id) {
        unsafe {
            let f: extern "C" fn(Id, Sel, Id, Id) = std::mem::transmute(objc_msgSend as *const ());
            f(target, sel(b"setValue:forKey:\0"), value, key);
        }
    }
    unsafe fn send_error(domain: Id, code: i64) -> Id {
        unsafe {
            let f: extern "C" fn(Id, Sel, Id, i64, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            f(
                class(b"NSError") as Id,
                sel(b"errorWithDomain:code:userInfo:\0"),
                domain,
                code,
                std::ptr::null_mut(),
            )
        }
    }

    unsafe fn ns_string(value: &str) -> Id {
        unsafe {
            let mut bytes = value.as_bytes().to_vec();
            bytes.push(0);
            send1(
                class(b"NSString") as Id,
                sel(b"stringWithUTF8String:\0"),
                bytes.as_ptr() as *mut c_char as Id,
            )
        }
    }

    unsafe fn nsstring_value(value: Id) -> String {
        unsafe {
            if value.is_null() {
                return String::new();
            }
            let utf8 = send0(value, sel(b"UTF8String\0"));
            if utf8.is_null() {
                return String::new();
            }
            std::ffi::CStr::from_ptr(utf8 as *const c_char)
                .to_string_lossy()
                .into_owned()
        }
    }

    // ─── the scheme handler ─────────────────────────────────────────────────

    extern "C" fn start_url_scheme_task(_this: Id, _cmd: Sel, _web_view: Id, task: Id) {
        unsafe {
            let request = send0(task, sel(b"request\0"));
            let url = send0(request, sel(b"URL\0"));
            let absolute = nsstring_value(send0(url, sel(b"absoluteString\0")));
            let path = absolute
                .strip_prefix(SCHEME)
                .and_then(|rest| rest.strip_prefix("://"))
                .and_then(|rest| rest.find('/').map(|i| &rest[i..]))
                .unwrap_or("");
            let root = assets_root();
            let Some(file) = resolve(&root, path) else {
                fail_task(task);
                return;
            };
            let Ok(bytes) = std::fs::read(&file) else {
                fail_task(task);
                return;
            };
            let data = send_data(bytes.as_ptr() as *const c_void, bytes.len());
            eprintln!("EVO-WEB serve {absolute} as {}", mime_for(&file));
            let mime = ns_string(mime_for(&file));
            let encoding = ns_string("utf-8");
            let allocated = send0(class(b"NSURLResponse") as Id, sel(b"alloc\0"));
            let response = send_response(allocated, url, mime, bytes.len() as i64, encoding);
            send_void1(task, sel(b"didReceiveResponse:\0"), response);
            send_void1(task, sel(b"didReceiveData:\0"), data);
            send_void0(task, sel(b"didFinish\0"));
            send_void0(response, sel(b"release\0"));
        }
    }

    unsafe fn fail_task(task: Id) {
        unsafe {
            let error = send_error(ns_string("NSURLErrorDomain"), -1008);
            send_void1(task, sel(b"didFailWithError:\0"), error);
        }
    }

    extern "C" fn stop_url_scheme_task(_this: Id, _cmd: Sel, _web_view: Id, _task: Id) {}

    // ─── navigation and windows ─────────────────────────────────────────────

    /// macOS 12+ calls the preferences variant; a delegate that implements
    /// only the older one gets the wrong arguments entirely.
    extern "C" fn decide_policy_with_preferences(
        _this: Id,
        _cmd: Sel,
        _web_view: Id,
        action: Id,
        _preferences: Id,
        handler: *mut c_void,
    ) {
        unsafe {
            let request = send0(action, sel(b"request\0"));
            let url = send0(request, sel(b"URL\0"));
            let absolute = nsstring_value(send0(url, sel(b"absoluteString\0")));
            let allowed = navigation_allowed(&absolute);
            if !allowed {
                eprintln!("EVO-WEB blocked {absolute}");
            }
            let call: extern "C" fn(*mut c_void, isize) = std::mem::transmute(handler);
            call(handler, if allowed { 1 } else { 0 });
        }
    }

    extern "C" fn create_web_view(
        _this: Id,
        _cmd: Sel,
        _web_view: Id,
        _configuration: Id,
        _action: Id,
        _features: Id,
    ) -> Id {
        std::ptr::null_mut()
    }

    extern "C" fn mouse_down(this: Id, _cmd: Sel, event: Id) {
        unsafe {
            let window = send0(this, sel(b"window\0"));
            if !window.is_null() {
                send_void1(window, sel(b"performWindowDragWithEvent:\0"), event);
            }
        }
    }

    extern "C" fn accepts_first_mouse(_this: Id, _cmd: Sel, _event: Id) -> Bool {
        1
    }

    /// `userContentController:didReceiveScriptMessage:` — the whole bridge
    /// ingress. It accepts a message only from the main frame of our own
    /// scheme; anything else is dropped before it is parsed.
    extern "C" fn did_receive_message(_this: Id, _cmd: Sel, _controller: Id, message: Id) {
        unsafe {
            let frame = send0(message, sel(b"frameInfo\0"));
            if frame.is_null() {
                return;
            }
            if send_bool(frame, sel(b"isMainFrame\0")) == 0 {
                eprintln!("EVO-WEB refused a message from a subframe");
                return;
            }
            let request = send0(frame, sel(b"request\0"));
            let url = send0(request, sel(b"URL\0"));
            let absolute = nsstring_value(send0(url, sel(b"absoluteString\0")));
            if !navigation_allowed(&absolute) {
                eprintln!("EVO-WEB refused a message from {absolute}");
                return;
            }
            let body = send0(message, sel(b"body\0"));
            let text = nsstring_value(body);
            eprintln!("EVO-WEB got {} bytes from the page", text.len());
            match super::inbox().lock() {
                Ok(mut queue) => queue.push(text.into_bytes()),
                Err(_) => eprintln!(
                    "EVO-WEB refused the message because the bridge inbox is unavailable"
                ),
            }
        }
    }

    static REGISTER_CLASSES: Once = Once::new();

    fn register_classes() {
        REGISTER_CLASSES.call_once(|| unsafe {
            let object = class(b"NSObject");
            let scheme =
                objc_allocateClassPair(object, b"EvoSchemeHandler\0".as_ptr() as *const c_char, 0);
            let task_types = b"v@:@@\0".as_ptr() as *const c_char;
            class_addMethod(
                scheme,
                sel(b"webView:startURLSchemeTask:\0"),
                start_url_scheme_task as *const c_void,
                task_types,
            );
            class_addMethod(
                scheme,
                sel(b"webView:stopURLSchemeTask:\0"),
                stop_url_scheme_task as *const c_void,
                task_types,
            );
            let protocol = objc_getProtocol(b"WKURLSchemeHandler\0".as_ptr() as *const c_char);
            if !protocol.is_null() {
                class_addProtocol(scheme, protocol);
            }
            objc_registerClassPair(scheme);

            let navigation = objc_allocateClassPair(
                object,
                b"EvoNavigationDelegate\0".as_ptr() as *const c_char,
                0,
            );
            class_addMethod(
                navigation,
                sel(b"webView:decidePolicyForNavigationAction:preferences:decisionHandler:\0"),
                decide_policy_with_preferences as *const c_void,
                b"v@:@@@@\0".as_ptr() as *const c_char,
            );
            class_addMethod(
                navigation,
                sel(
                    b"webView:createWebViewWithConfiguration:forNavigationAction:windowFeatures:\0",
                ),
                create_web_view as *const c_void,
                b"@@:@@@@\0".as_ptr() as *const c_char,
            );
            for protocol_name in [
                b"WKNavigationDelegate\0".as_slice(),
                b"WKUIDelegate\0".as_slice(),
            ] {
                let protocol = objc_getProtocol(protocol_name.as_ptr() as *const c_char);
                if !protocol.is_null() {
                    class_addProtocol(navigation, protocol);
                }
            }
            objc_registerClassPair(navigation);

            let strip = objc_allocateClassPair(
                class(b"NSView"),
                b"EvoDragStrip\0".as_ptr() as *const c_char,
                0,
            );
            class_addMethod(
                strip,
                sel(b"mouseDown:\0"),
                mouse_down as *const c_void,
                b"v@:@\0".as_ptr() as *const c_char,
            );
            class_addMethod(
                strip,
                sel(b"acceptsFirstMouse:\0"),
                accepts_first_mouse as *const c_void,
                b"c@:@\0".as_ptr() as *const c_char,
            );
            objc_registerClassPair(strip);

            let handler = objc_allocateClassPair(
                class(b"NSObject"),
                b"EvoBridgeHandler\0".as_ptr() as *const c_char,
                0,
            );
            class_addMethod(
                handler,
                sel(b"userContentController:didReceiveScriptMessage:\0"),
                did_receive_message as *const c_void,
                b"v@:@@\0".as_ptr() as *const c_char,
            );
            let protocol = objc_getProtocol(b"WKScriptMessageHandler\0".as_ptr() as *const c_char);
            if !protocol.is_null() {
                class_addProtocol(handler, protocol);
            }
            objc_registerClassPair(handler);
        });
    }

    fn main_window() -> Id {
        unsafe {
            let app = send0(class(b"NSApplication") as Id, sel(b"sharedApplication\0"));
            let mut window = send0(app, sel(b"keyWindow\0"));
            if window.is_null() {
                window = send0(app, sel(b"mainWindow\0"));
            }
            if window.is_null() {
                let windows = send0(app, sel(b"windows\0"));
                window = send0(windows, sel(b"firstObject\0"));
            }
            window
        }
    }

    pub struct WebShell {
        view: Id,
        navigation: Id,
        scheme: Id,
        strip: Id,
        attached: bool,
    }

    impl WebShell {
        pub fn new() -> Self {
            register_classes();
            unsafe {
                // A bare binary (not a bundle) starts as an accessory app: no
                // dock icon, no key window, and it cannot be brought forward.
                let app = send0(class(b"NSApplication") as Id, sel(b"sharedApplication\0"));
                send_int1(app, sel(b"setActivationPolicy:\0"), 0); // Regular
                let configuration = send0(class(b"WKWebViewConfiguration") as Id, sel(b"new\0"));
                let scheme = send0(class(b"EvoSchemeHandler") as Id, sel(b"new\0"));
                send2(
                    configuration,
                    sel(b"setURLSchemeHandler:forURLScheme:\0"),
                    scheme,
                    ns_string(SCHEME),
                );

                // The inspector exists only where a developer is running a
                // debug build; a release bundle has no way to open it.
                if !cfg!(debug_assertions) {
                    let preferences = send0(configuration, sel(b"preferences\0"));
                    send_set_value(
                        preferences,
                        ns_string("developerExtrasEnabled"),
                        send_number_bool(0),
                    );
                }

                let frame = NsRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1200.0,
                    height: 880.0,
                };
                let controller = send0(configuration, sel(b"userContentController\0"));
                let handler = send0(class(b"EvoBridgeHandler") as Id, sel(b"new\0"));
                send2(
                    controller,
                    sel(b"addScriptMessageHandler:name:\0"),
                    handler,
                    ns_string("evo"),
                );
                let allocated_view = send0(class(b"WKWebView") as Id, sel(b"alloc\0"));
                let view = send_init_frame_config(allocated_view, frame, configuration);
                send_int1(view, sel(b"setAllowsBackForwardNavigationGestures:\0"), 0);

                let navigation = send0(class(b"EvoNavigationDelegate") as Id, sel(b"new\0"));
                // The navigation delegate is deliberately not installed: on
                // this macOS, WebKit 26.4 crashes inside
                // NavigationClient::decidePolicyForNavigationAction when a
                // hand-built delegate implements that method (verified twice
                // from crash reports). Cross-scheme navigation is therefore
                // refused by the scheme handler and by shell.js in the page,
                // and the remaining gap is recorded in docs/INTEGRATION.md.
                send_void1(view, sel(b"setUIDelegate:\0"), navigation);

                let allocated_strip = send0(class(b"EvoDragStrip") as Id, sel(b"alloc\0"));
                let strip = send_init_frame(
                    allocated_strip,
                    NsRect {
                        x: 0.0,
                        y: 0.0,
                        width: 1200.0,
                        height: 28.0,
                    },
                );

                let ns_url = send1(
                    class(b"NSURL") as Id,
                    sel(b"URLWithString:\0"),
                    ns_string(&start_url()),
                );
                let request = send1(
                    class(b"NSURLRequest") as Id,
                    sel(b"requestWithURL:\0"),
                    ns_url,
                );
                send_void1(view, sel(b"loadRequest:\0"), request);

                Self {
                    view,
                    navigation,
                    scheme,
                    strip,
                    attached: false,
                }
            }
        }

        /// Runs one script in the page: the bridge, delivered when the read
        /// finishes. The page waits on `window.__evoReady` if it is already up.
        pub fn evaluate(&self, script: &str) {
            unsafe {
                let source = ns_string(script);
                let handler: extern "C" fn(Id, Sel, Id, Id) =
                    std::mem::transmute(objc_msgSend as *const ());
                handler(
                    self.view,
                    sel(b"evaluateJavaScript:completionHandler:\0"),
                    source,
                    std::ptr::null_mut(),
                );
            }
        }

        pub fn set_appearance(&self, appearance: &str) {
                unsafe {
                    let window = main_window();
                    if window.is_null() {
                        return;
                    }
                    if appearance == "auto" {
                        send_void1(window, sel(b"setAppearance:\0"), std::ptr::null_mut());
                    } else {
                        let name = match appearance {
                            "day" => "NSAppearanceNameAqua",
                            "dusk" => "NSAppearanceNameDarkAqua",
                            _ => return,
                        };
                        let native = send1(
                            class(b"NSAppearance") as Id,
                            sel(b"appearanceNamed:\0"),
                            ns_string(name),
                        );
                        if !native.is_null() {
                            send_void1(window, sel(b"setAppearance:\0"), native);
                    }
                }
            }
        }

        /// Sizes the web view to the window and lays the drag strip over the
        /// title-bar band. Called once per frame; moving is idempotent.
        pub fn sync_frame(&mut self) {
            unsafe {
                let window = main_window();
                if window.is_null() {
                    return;
                }
                let host = send0(window, sel(b"contentView\0"));
                if host.is_null() {
                    return;
                }
                if !self.attached {
                    send_void1(host, sel(b"addSubview:\0"), self.view);
                    send_void1(host, sel(b"addSubview:\0"), self.strip);
                    let app = send0(class(b"NSApplication") as Id, sel(b"sharedApplication\0"));
                    send_int1(app, sel(b"activateIgnoringOtherApps:\0"), 1);
                    send_void0(window, sel(b"makeKeyAndOrderFront:\0"));
                    self.attached = true;
                }
                let bounds = send_bounds(host);
                send_rect(
                    self.view,
                    sel(b"setFrame:\0"),
                    NsRect {
                        x: 0.0,
                        y: 0.0,
                        width: bounds.width,
                        height: bounds.height,
                    },
                );
                let flipped = send_bool(self.strip, sel(b"isFlipped\0")) != 0;
                send_rect(
                    self.strip,
                    sel(b"setFrame:\0"),
                    NsRect {
                        x: 0.0,
                        y: if flipped { 0.0 } else { bounds.height - 28.0 },
                        width: bounds.width,
                        height: 28.0,
                    },
                );
            }
        }
    }

    impl Default for WebShell {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Drop for WebShell {
        fn drop(&mut self) {
            unsafe {
                send_void0(self.view, sel(b"removeFromSuperview\0"));
                send_void0(self.strip, sel(b"removeFromSuperview\0"));
                send_void0(self.view, sel(b"release\0"));
                send_void0(self.strip, sel(b"release\0"));
                send_void0(self.navigation, sel(b"release\0"));
                send_void0(self.scheme, sel(b"release\0"));
            }
        }
    }
}

/// The eframe app that hosts the web shell: it paints nothing, keeps the
/// window alive, and sizes the web view to the window once per frame. The egui
/// application is untouched and remains the default (`EVO_UI=native`).
/// The menu-bar item, when the platform has one.
fn presence() -> Option<crate::presence::PodPresence> {
    let pods: crate::pods::SharedPods =
        std::rc::Rc::new(std::cell::RefCell::new(crate::pods::PodHost::default()));
    // A presence without a context still installs the item; the context is
    // only used by pod commands, which this shell never sends.
    Some(crate::presence::PodPresence::new(
        pods,
        egui::Context::default(),
    ))
}

pub struct WebUiApp {
    shell: WebShell,
    backend: crate::bridge::Backend,
    replies: std::sync::mpsc::Receiver<(String, String)>,
    reply_sender: std::sync::mpsc::Sender<(String, String)>,
    answered: u64,
    /// The menu-bar item: status, Pause or Resume, Open Evo, Quit.
    presence: Option<crate::presence::PodPresence>,
    menu_tick: u64,
}

impl WebUiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let root = evo_storage::prepare_storage_root()
            .unwrap_or_else(|_| evo_storage::canonical_storage_root());
        let (reply_sender, replies) = std::sync::mpsc::channel();
        let shell = WebShell::new();
        let saved_appearance = crate::ui_state::UiState::load(&root).appearance;
        if let Some(mode) = crate::theme::ThemeMode::parse(&saved_appearance) {
            crate::theme::set_theme_mode(mode);
        }
        crate::theme::refresh_visuals(&_cc.egui_ctx);
        shell.set_appearance(&saved_appearance);
        Self {
            shell,
            backend: crate::bridge::Backend::new(root),
            replies,
            reply_sender,
            answered: 0,
            presence: presence(),
            menu_tick: 0,
        }
    }

    fn answer(&mut self, bytes: Vec<u8>) {
        let request = match crate::bridge::parse(&bytes) {
            Ok(request) => request,
            Err(error) => {
                eprintln!(
                    "EVO-WEB refused a message: {} {}",
                    error.code, error.message
                );
                let id = serde_json::from_slice::<serde_json::Value>(&bytes)
                    .ok()
                    .and_then(|value| value.get("id")?.as_u64());
                self.shell.evaluate(&format!(
                    "window.__evoReply({});",
                    crate::bridge::error_reply(id, &error)
                ));
                return;
            }
        };
        // Work runs off the frame; the reply comes back through the channel.
        let command = request.command.name();
        let root = self.backend.root().to_path_buf();
        let sender = self.reply_sender.clone();
        std::thread::Builder::new()
            .name("evo-bridge-worker".into())
            .spawn(move || {
                let reply = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::bridge::Backend::new(root.clone()).respond(&request)
                }));
                match reply {
                    Ok(reply) => {
                        eprintln!("EVO-WEB answered {command} with {} bytes", reply.len());
                        let _ = sender.send((command.to_string(), reply));
                    }
                    Err(_) => {
                        eprintln!("EVO-WEB worker panicked on {command}");
                        let err = crate::bridge::error_reply(
                            Some(request.id),
                            &crate::bridge::BridgeError::new("backend", "Evo crashed on that request."),
                        );
                        let _ = sender.send((command.to_string(), err));
                    }
                }
            })
            .ok();
    }
}

impl eframe::App for WebUiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.sync_frame();
        let system_theme = ui.ctx().input(|input| input.raw.system_theme);
        if crate::theme::set_system_theme(system_theme) {
            crate::theme::refresh_visuals(ui.ctx());
        }
        self.menu_tick = self.menu_tick.wrapping_add(1);
        if self.menu_tick % 60 == 1
            && let Some(presence) = self.presence.as_mut()
        {
            // The web shell has no pods, so the menu is status, pause, open,
            // quit; the paused state inside it is read from the same store.
            presence.update_pods(&[], None);
        }
        for message in take_messages() {
            self.answer(message);
        }
        while let Ok((command, reply)) = self.replies.try_recv() {
            self.answered += 1;
            if self.answered <= 3 || self.answered.is_multiple_of(50) {
                eprintln!("EVO-WEB reply #{} {} bytes", self.answered, reply.len());
            }
            if command == "set_settings"
                && let Ok(value) = serde_json::from_str::<serde_json::Value>(&reply)
                && let Some(appearance) = value
                    .get("data")
                    .and_then(|data| data.get("appearance"))
                    .and_then(|appearance| appearance.as_str())
            {
                if let Some(mode) = crate::theme::ThemeMode::parse(appearance) {
                    crate::theme::set_theme_mode(mode);
                    crate::theme::refresh_visuals(ui.ctx());
                }
                self.shell.set_appearance(appearance);
            }
            let script = match crate::bridge::event_for(&command) {
                Some(event) => format!(
                    "window.__evoReply && window.__evoReply({reply});\
                     window.__evoEvent && window.__evoEvent({{type:\"{event}\"}});"
                ),
                None => format!("window.__evoReply && window.__evoReply({reply});"),
            };
            self.shell.evaluate(&script);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(30));
    }
}
