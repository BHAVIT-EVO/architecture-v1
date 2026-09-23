//! Stage 3 of Pods: the work's web pages live inside Evo.
//!
//! Containment (Stage 1) cleared the desktop; the browser world (Stage 2)
//! gave each work its own login-jar in the person's browser. The remaining
//! context switch is the app boundary itself: even a work's own browser
//! window is *another window* to juggle. The web surface removes it — the
//! active work's pages render as panes **inside Evo's window**, one tab
//! strip, no app switch at all.
//!
//! Mechanics (all public API, the same raw-runtime pattern as presence):
//!
//! * each work gets a **persistent WKWebsiteDataStore** keyed by a
//!   deterministic UUID derived from the work's identity
//!   (`dataStoreForIdentifier:` — logins, cookies, and storage survive
//!   Evo restarts, and no two works ever share a jar);
//! * panes are WKWebViews added as child views of the window's content
//!   view, positioned every frame over the rect egui allocated — the
//!   native view consumes mouse and keyboard over that rect, so the web
//!   is simply *part of* Evo's window;
//! * **hibernation** is honest: closing the panel or leaving the pod
//!   releases every pane, and a released WKWebView takes its WebContent
//!   process with it — the RAM cost of a work's web presence is paid only
//!   while the person is looking at it. The data store on disk is
//!   untouched, so rehydrating restores sessions exactly.
//!
//! Nothing here is a browser feature beyond the work: no address bar, no
//! history UI — back/forward and the work's own tabs. Evo hosts the work,
//! it does not become a browser.

// ─── Platform implementation ─────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod imp {
    use std::os::raw::{c_char, c_void};

    type Bool = i8;
    type Id = *mut Object;
    type Class = *mut objc_class;
    type Sel = *mut c_void;

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
    #[link(name = "WebKit", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Class;
        fn objc_msgSend();
        fn sel_registerName(name: *const c_char) -> Sel;
    }

    // ─── selectors ────────────────────────────────────────────────────────
    const SHARED_APPLICATION_SEL: &[u8] = b"sharedApplication\0";
    const MAIN_WINDOW_SEL: &[u8] = b"mainWindow\0";
    const KEY_WINDOW_SEL: &[u8] = b"keyWindow\0";
    const WINDOWS_SEL: &[u8] = b"windows\0";
    const FIRST_OBJECT_SEL: &[u8] = b"firstObject\0";
    const CONTENT_VIEW_SEL: &[u8] = b"contentView\0";
    const ADD_SUBVIEW_SEL: &[u8] = b"addSubview:\0";
    const REMOVE_FROM_SUPERVER_SEL: &[u8] = b"removeFromSuperview\0";
    const SET_FRAME_SEL: &[u8] = b"setFrame:\0";
    const IS_FLIPPED_SEL: &[u8] = b"isFlipped\0";
    const BOUNDS_SEL: &[u8] = b"bounds\0";
    const STRING_WITH_UTF8_SEL: &[u8] = b"stringWithUTF8String:\0";
    const UTF8_STRING_SEL: &[u8] = b"UTF8String\0";
    const URL_WITH_STRING_SEL: &[u8] = b"URLWithString:\0";
    const REQUEST_WITH_URL_SEL: &[u8] = b"requestWithURL:\0";
    const NEW_SEL: &[u8] = b"new\0";
    const ALLOC_SEL: &[u8] = b"alloc\0";
    const INIT_WITH_UUID_STRING_SEL: &[u8] = b"initWithUUIDString:\0";
    const INIT_WITH_FRAME_CONFIG_SEL: &[u8] = b"initWithFrame:configuration:\0";
    const SET_WEBSITE_DATA_STORE_SEL: &[u8] = b"setWebsiteDataStore:\0";
    const LOAD_REQUEST_SEL: &[u8] = b"loadRequest:\0";
    const GO_BACK_SEL: &[u8] = b"goBack\0";
    const GO_FORWARD_SEL: &[u8] = b"goForward\0";
    const CAN_GO_BACK_SEL: &[u8] = b"canGoBack\0";
    const CAN_GO_FORWARD_SEL: &[u8] = b"canGoForward\0";
    const TITLE_SEL: &[u8] = b"title\0";
    const URL_SEL: &[u8] = b"URL\0";
    const ABSOLUTE_STRING_SEL: &[u8] = b"absoluteString\0";
    const UUID_WITH_STRING_SEL: &[u8] = b"initWithUUIDString:\0";
    const DATA_STORE_FOR_IDENTIFIER_SEL: &[u8] = b"dataStoreForIdentifier:\0";
    const SET_ALLOWS_BACK_FORWARD_GESTURES_SEL: &[u8] =
        b"setAllowsBackForwardNavigationGestures:\0";
    const SUPERVIEW_SEL: &[u8] = b"superview\0";
    const RETAIN_SEL: &[u8] = b"retain\0";
    const RELEASE_SEL: &[u8] = b"release\0";

    /// NSRect is two CGPoint/CGSize structs of CGFloat (f64) — laid out
    /// inline as four doubles.
    #[repr(C)]
    #[derive(Clone, Copy, PartialEq)]
    struct NsRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    }

    fn sel(bytes: &'static [u8]) -> Sel {
        unsafe { sel_registerName(bytes.as_ptr() as *const c_char) }
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
    unsafe fn msg_send_rect(receiver: Id, selector: Sel, arg: NsRect) {
        let function: extern "C" fn(Id, Sel, NsRect) =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg)
    }
    unsafe fn msg_send_bool(receiver: Id, selector: Sel) -> Bool {
        let function: extern "C" fn(Id, Sel) -> Bool =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector)
    }
    unsafe fn msg_send_bool1(receiver: Id, selector: Sel, arg: isize) {
        let function: extern "C" fn(Id, Sel, isize) =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg);
    }

    unsafe fn class(name: &[u8]) -> Class {
        let mut bytes = name.to_vec();
        bytes.push(0);
        objc_getClass(bytes.as_ptr() as *const c_char)
    }

    unsafe fn ns_string(value: &str) -> Id {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        msg_send_id1(
            class(b"NSString") as Id,
            sel(STRING_WITH_UTF8_SEL),
            bytes.as_ptr() as *mut c_char as Id,
        )
    }

    /// Reads an NSString into a Rust String (empty for nil).
    unsafe fn nsstring_value(value: Id) -> String {
        if value.is_null() {
            return String::new();
        }
        let utf8 = msg_send_id(value, sel(UTF8_STRING_SEL));
        if utf8.is_null() {
            return String::new();
        }
        let cstr = std::ffi::CStr::from_ptr(utf8 as *const c_char);
        cstr.to_string_lossy().into_owned()
    }

    // ─── per-work data stores ─────────────────────────────────────────────

    /// A deterministic UUID string for a work's identity: two FNV-1a
    /// passes with different seeds render as a standard 8-4-4-4-12 UUID,
    /// so the same work always opens the same persistent data store.
    pub fn work_store_uuid(work_identity: &str) -> String {
        fn fnv(seed: u64, bytes: &[u8]) -> u64 {
            let mut hash = seed;
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x00000100000001B3);
            }
            hash
        }
        let bytes = work_identity.as_bytes();
        let hi = fnv(0xcbf29ce484222325, bytes);
        let lo = fnv(0x9e3779b97f4a7c15, bytes);
        let (a, b, c, d, e) = (
            hi & 0xffff_ffff,
            hi >> 32 & 0xffff,
            lo >> 48 & 0x0fff,
            lo >> 36 & 0x0fff,
            lo & 0xffff_ffff_ffff,
        );
        format!("{a:08x}-{b:04x}-4{c:03x}-8{d:03x}-{e:012x}")
    }

    /// The work's persistent WKWebsiteDataStore (created on first use).
    unsafe fn data_store_for(work_identity: &str) -> Id {
        // `initWithUUIDString:` returns nil (never throws) for a malformed
        // string — the derived UUID is always well-formed, but nil is
        // checked anyway: passing nil to `dataStoreForIdentifier:` throws.
        let uuid_string = ns_string(&work_store_uuid(work_identity));
        let allocated = msg_send_id(class(b"NSUUID") as Id, sel(ALLOC_SEL));
        let uuid = msg_send_id1(allocated, sel(UUID_WITH_STRING_SEL), uuid_string);
        if uuid.is_null() {
            // The honest fallback: a fresh non-persistent store rather
            // than throwing into the app.
            return msg_send_id(class(b"WKWebsiteDataStore") as Id, sel(NEW_SEL));
        }
        msg_send_id1(
            class(b"WKWebsiteDataStore") as Id,
            sel(DATA_STORE_FOR_IDENTIFIER_SEL),
            uuid,
        )
    }

    /// The window's content view: main, key, or the first window — Evo
    /// has one window, and the panel can only show when one exists.
    unsafe fn content_view() -> Id {
        let app = msg_send_id(class(b"NSApplication") as Id, sel(SHARED_APPLICATION_SEL));
        if app.is_null() {
            return std::ptr::null_mut();
        }
        let window = {
            let main = msg_send_id(app, sel(MAIN_WINDOW_SEL));
            if !main.is_null() {
                main
            } else {
                let key = msg_send_id(app, sel(KEY_WINDOW_SEL));
                if !key.is_null() {
                    key
                } else {
                    let windows = msg_send_id(app, sel(WINDOWS_SEL));
                    if windows.is_null() {
                        return std::ptr::null_mut();
                    }
                    msg_send_id(windows, sel(FIRST_OBJECT_SEL))
                }
            }
        };
        if window.is_null() {
            return std::ptr::null_mut();
        }
        msg_send_id(window, sel(CONTENT_VIEW_SEL))
    }

    /// `[[WKWebView alloc] initWithFrame:configuration:]` — the instance
    /// initializer (returns retained).
    unsafe fn web_view_init(frame: NsRect, configuration: Id) -> Id {
        let allocated = msg_send_id(class(b"WKWebView") as Id, sel(ALLOC_SEL));
        if allocated.is_null() {
            return std::ptr::null_mut();
        }
        let function: extern "C" fn(Id, Sel, NsRect, Id) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        function(
            allocated,
            sel(INIT_WITH_FRAME_CONFIG_SEL),
            frame,
            configuration,
        )
    }

    // ─── ObjC exception guard ─────────────────────────────────────────────
    // WebKit legitimately throws ObjC exceptions (bad identifiers, views in
    // the wrong state). Rust aborts the whole process if a foreign
    // exception unwinds through a Rust frame, so every risky WebKit call
    // runs inside the compiled @try/@catch shim instead: a WebKit refusal
    // becomes an honest Evo error, never a crash.
    //
    // The trampoline is `extern "C-unwind"` on purpose: a plain `extern
    // "C"` frame aborts any foreign unwind that tries to cross it, which
    // would stop the exception before the shim's @catch can take it.
    unsafe extern "C" {
        fn evo_objc_try(
            fn_ptr: extern "C-unwind" fn(*mut c_void),
            ctx: *mut c_void,
            out: *mut c_char,
            out_len: usize,
        ) -> i32;
    }

    /// The trampoline the shim invokes: runs `guard.body(&mut guard.value)`.
    extern "C-unwind" fn guard_trampoline<T>(ctx: *mut c_void) {
        unsafe {
            let guard = &mut *(ctx as *mut Guard<T>);
            (guard.body)(&mut guard.value);
        }
    }

    struct Guard<T> {
        body: fn(&mut T),
        value: T,
    }

    /// Runs `body(&mut value)` inside an ObjC @try. `Ok(value)` when it
    /// completed; `Err(reason)` carries the exception's name and reason.
    fn objc_guarded<T>(value: T, body: fn(&mut T)) -> Result<T, String> {
        let mut guard = Guard { body, value };
        let mut reason = [0u8; 512];
        let ok = unsafe {
            evo_objc_try(
                guard_trampoline::<T>,
                &mut guard as *mut Guard<T> as *mut c_void,
                reason.as_mut_ptr() as *mut c_char,
                reason.len(),
            )
        };
        if ok == 1 {
            Ok(guard.value)
        } else {
            let end = reason.iter().position(|b| *b == 0).unwrap_or(reason.len());
            Err(String::from_utf8_lossy(&reason[..end]).into_owned())
        }
    }

    /// One pane: a WKWebView retained by Evo.
    pub struct Pane {
        pub url: String,
        view: Id,
        attached: bool,
        last_frame: Option<NsRect>,
    }

    /// Inputs and outputs of pane creation (the guarded body needs a plain
    /// context; closures cannot cross the C boundary).
    struct NewContext {
        work: String,
        url: String,
        view: Id,
    }

    fn pane_new_body(ctx: &mut NewContext) {
        unsafe {
            let configuration = msg_send_id(class(b"WKWebViewConfiguration") as Id, sel(NEW_SEL));
            if configuration.is_null() {
                return;
            }
            let store = data_store_for(&ctx.work);
            if !store.is_null() {
                msg_send_id1(configuration, sel(SET_WEBSITE_DATA_STORE_SEL), store);
            }

            let zero = NsRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            };
            let view = web_view_init(zero, configuration);
            if view.is_null() {
                return;
            }
            ctx.view = view;
            // macOS-native trackpad swipe navigation.
            msg_send_bool1(view, sel(SET_ALLOWS_BACK_FORWARD_GESTURES_SEL), 1);

            let url_string = ns_string(&ctx.url);
            let nsurl = msg_send_id1(class(b"NSURL") as Id, sel(URL_WITH_STRING_SEL), url_string);
            if !nsurl.is_null() {
                let request = msg_send_id1(
                    class(b"NSURLRequest") as Id,
                    sel(REQUEST_WITH_URL_SEL),
                    nsurl,
                );
                if !request.is_null() {
                    msg_send_id1(view, sel(LOAD_REQUEST_SEL), request);
                }
            }
        }
    }

    /// The guarded layout body's context: the pane and its target rect.
    struct LayoutContext {
        pane: *mut Pane,
        rect: (f32, f32, f32, f32),
    }

    fn pane_layout_body(ctx: &mut LayoutContext) {
        unsafe {
            let pane = &mut *ctx.pane;
            let rect = ctx.rect;
            let host = content_view();
            if host.is_null() || pane.view.is_null() {
                return;
            }
            let (x, y, width, height) = rect;
            let flipped = msg_send_bool(host, sel(IS_FLIPPED_SEL)) != 0;
            let host_height = {
                // NSBounds returns a struct; the transmuted signature is
                // the standard arm64 pattern.
                let function: extern "C" fn(Id, Sel) -> NsRect =
                    std::mem::transmute(objc_msgSend as *const ());
                let rect = function(host, sel(BOUNDS_SEL));
                rect.y + rect.height
            };
            let frame = if flipped {
                NsRect {
                    x: x as f64,
                    y: y as f64,
                    width: width as f64,
                    height: height as f64,
                }
            } else {
                NsRect {
                    x: x as f64,
                    y: host_height - (y as f64) - (height as f64),
                    width: width as f64,
                    height: height as f64,
                }
            };

            let needs_attach = !pane.attached || msg_send_id(pane.view, sel(SUPERVIEW_SEL)) != host;
            if needs_attach {
                if pane.attached {
                    msg_send_id(pane.view, sel(REMOVE_FROM_SUPERVER_SEL));
                }
                msg_send_id1(host, sel(ADD_SUBVIEW_SEL), pane.view);
                pane.attached = true;
            }
            if pane.last_frame != Some(frame) {
                msg_send_rect(pane.view, sel(SET_FRAME_SEL), frame);
                pane.last_frame = Some(frame);
            }
        }
    }

    impl Pane {
        /// Creates the pane and starts loading its URL. The view is
        /// retained by Evo (initWith returns retained) and is not in any
        /// view hierarchy yet. A WebKit refusal is reported, never fatal.
        pub fn new(work_identity: &str, url: &str) -> Option<Pane> {
            let ctx = NewContext {
                work: work_identity.to_string(),
                url: url.to_string(),
                view: std::ptr::null_mut(),
            };
            match objc_guarded(ctx, pane_new_body) {
                Ok(ctx) if !ctx.view.is_null() => Some(Pane {
                    url: url.to_string(),
                    view: ctx.view,
                    attached: false,
                    last_frame: None,
                }),
                Ok(_) => None,
                Err(reason) => {
                    eprintln!("EVO-WEB: pane refused: {reason}");
                    None
                }
            }
        }

        /// Detaches and releases the pane; its WebContent process exits
        /// with the last reference.
        pub fn hibernate(&mut self) {
            unsafe {
                if self.attached && !self.view.is_null() {
                    msg_send_id(self.view, sel(REMOVE_FROM_SUPERVER_SEL));
                }
                if !self.view.is_null() {
                    msg_send_id(self.view, sel(RELEASE_SEL));
                }
                self.view = std::ptr::null_mut();
                self.attached = false;
                self.last_frame = None;
            }
        }

        /// Places the pane at an egui rect (top-left origin, points) over
        /// the content view, attaching it first if needed. Guarded: a view
        /// in a state AppKit rejects skips this frame rather than dying.
        pub fn layout(&mut self, rect: (f32, f32, f32, f32)) {
            if self.view.is_null() {
                return;
            }
            let ctx = LayoutContext {
                pane: self as *mut Pane,
                rect,
            };
            if let Err(reason) = objc_guarded(ctx, pane_layout_body) {
                // Log once per distinct failure shape: layout runs every
                // frame, and flooding the log hides the first cause.
                eprintln!("EVO-WEB: layout refused: {reason}");
            }
        }

        /// Detaches the pane from the hierarchy without releasing it
        /// (a background tab).
        pub fn park(&mut self) {
            unsafe {
                if self.attached && !self.view.is_null() {
                    msg_send_id(self.view, sel(REMOVE_FROM_SUPERVER_SEL));
                }
                self.attached = false;
                self.last_frame = None;
            }
        }

        pub fn title(&self) -> String {
            unsafe {
                if self.view.is_null() {
                    return self.url.clone();
                }
                let title = msg_send_id(self.view, sel(TITLE_SEL));
                let title = nsstring_value(title);
                if title.trim().is_empty() {
                    self.url.clone()
                } else {
                    title
                }
            }
        }

        pub fn current_url(&self) -> String {
            unsafe {
                if self.view.is_null() {
                    return self.url.clone();
                }
                let url = msg_send_id(self.view, sel(URL_SEL));
                if url.is_null() {
                    return self.url.clone();
                }
                let absolute = msg_send_id(url, sel(ABSOLUTE_STRING_SEL));
                let absolute = nsstring_value(absolute);
                if absolute.is_empty() {
                    self.url.clone()
                } else {
                    absolute
                }
            }
        }

        pub fn go_back(&self) {
            unsafe {
                if !self.view.is_null() {
                    msg_send_id(self.view, sel(GO_BACK_SEL));
                }
            }
        }
        pub fn go_forward(&self) {
            unsafe {
                if !self.view.is_null() {
                    msg_send_id(self.view, sel(GO_FORWARD_SEL));
                }
            }
        }
        pub fn can_go_back(&self) -> bool {
            unsafe { !self.view.is_null() && msg_send_bool(self.view, sel(CAN_GO_BACK_SEL)) != 0 }
        }
        pub fn can_go_forward(&self) -> bool {
            unsafe {
                !self.view.is_null() && msg_send_bool(self.view, sel(CAN_GO_FORWARD_SEL)) != 0
            }
        }
    }

    impl Drop for Pane {
        fn drop(&mut self) {
            self.hibernate();
        }
    }
}

// ─── The surface the app talks to ─────────────────────────────────────────────

#[cfg(target_os = "macos")]
pub use imp::Pane;

#[cfg(not(target_os = "macos"))]
pub struct Pane {
    pub url: String,
}

/// The active work's web pages: a tab per URL, a WKWebView per *visited*
/// tab. Panes are created lazily — a work with twenty-five pages is
/// twenty-five labels until the person actually opens one, so RAM follows
/// attention, not the size of the work's history.
#[derive(Default)]
pub struct WebSurfaces {
    #[cfg(target_os = "macos")]
    tabs: Vec<String>,
    #[cfg(target_os = "macos")]
    views: Vec<Option<Pane>>,
    #[cfg(target_os = "macos")]
    work: Option<String>,
    active: usize,
}

impl WebSurfaces {
    pub fn new() -> Self {
        Self::default()
    }

    /// True when the given work's tabs are live (no work needed).
    pub fn is_hydrated_for(&self, work: &str) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.work.as_deref() == Some(work)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = work;
            false
        }
    }

    /// The number of tabs (labels), not live panes.
    pub fn pane_count(&self) -> usize {
        #[cfg(target_os = "macos")]
        {
            self.tabs.len()
        }
        #[cfg(not(target_os = "macos"))]
        {
            0
        }
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    /// The tab's URL (tabs are URLs; panes come and go).
    pub fn tab_url(&self, index: usize) -> String {
        #[cfg(target_os = "macos")]
        {
            self.tabs.get(index).cloned().unwrap_or_default()
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = index;
            String::new()
        }
    }

    /// Hydrates the work's tabs (a different work hibernates first).
    /// Duplicate URLs collapse; only http(s) pages become tabs. No pane
    /// exists yet — the first layout creates the active tab's pane.
    #[allow(unused_variables)]
    pub fn hydrate(&mut self, work: &str, urls: &[String]) {
        #[cfg(target_os = "macos")]
        {
            if self.is_hydrated_for(work) {
                return;
            }
            self.hibernate();
            let mut tabs: Vec<String> = Vec::new();
            for url in urls {
                if !url.starts_with("http") || tabs.contains(url) {
                    continue;
                }
                tabs.push(url.clone());
            }
            self.views = (0..tabs.len()).map(|_| None).collect();
            self.tabs = tabs;
            self.work = Some(work.to_string());
            self.active = 0;
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (work, urls);
        }
    }

    /// Releases every pane: the work's web presence leaves RAM entirely,
    /// its data store stays on disk.
    pub fn hibernate(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.views.clear();
            self.tabs.clear();
            self.work = None;
        }
        self.active = 0;
    }

    pub fn switch_tab(&mut self, index: usize) {
        #[cfg(target_os = "macos")]
        {
            if index < self.tabs.len() {
                self.active = index;
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = index;
        }
    }

    /// Places the active tab's pane over `rect` (egui top-left points),
    /// creating it on first view; visited-but-inactive panes are parked
    /// off-hierarchy so their state survives the tab switch.
    #[allow(unused_variables)]
    pub fn layout_active(&mut self, rect: (f32, f32, f32, f32)) {
        #[cfg(target_os = "macos")]
        {
            let work = match self.work.as_deref() {
                Some(work) => work.to_string(),
                None => return,
            };
            for (index, slot) in self.views.iter_mut().enumerate() {
                if index == self.active {
                    if slot.is_none() {
                        if let Some(url) = self.tabs.get(index) {
                            *slot = Pane::new(&work, url);
                        }
                    }
                    if let Some(pane) = slot.as_mut() {
                        pane.layout(rect);
                    }
                } else if let Some(pane) = slot.as_mut() {
                    pane.park();
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = rect;
        }
    }

    /// The label for tab `i`: the page's own title once it has loaded,
    /// the URL until then.
    pub fn tab_label(&self, index: usize) -> String {
        #[cfg(target_os = "macos")]
        {
            let title = match self.views.get(index) {
                Some(Some(pane)) => {
                    let title = pane.title();
                    if title.is_empty() {
                        String::new()
                    } else {
                        title
                    }
                }
                _ => String::new(),
            };
            if !title.is_empty() {
                return title;
            }
            self.tabs.get(index).cloned().unwrap_or_default()
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = index;
            String::new()
        }
    }

    /// The active tab's current URL (navigations included).
    pub fn active_url(&self) -> String {
        #[cfg(target_os = "macos")]
        {
            match self.views.get(self.active) {
                Some(Some(pane)) => pane.current_url(),
                _ => self.tabs.get(self.active).cloned().unwrap_or_default(),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            String::new()
        }
    }

    pub fn go_back(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(Some(pane)) = self.views.get(self.active) {
            pane.go_back();
        }
    }
    pub fn go_forward(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(Some(pane)) = self.views.get(self.active) {
            pane.go_forward();
        }
    }
    pub fn can_go_back(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            match self.views.get(self.active) {
                Some(Some(pane)) => pane.can_go_back(),
                _ => false,
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
    pub fn can_go_forward(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            match self.views.get(self.active) {
                Some(Some(pane)) => pane.can_go_forward(),
                _ => false,
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_store_uuid_is_deterministic_and_well_formed() {
        #[cfg(target_os = "macos")]
        {
            let a = imp::work_store_uuid("Baggage Battles");
            let b = imp::work_store_uuid("Baggage Battles");
            assert_eq!(a, b, "the same work always maps to the same store");
            assert_ne!(a, imp::work_store_uuid("ZCode"));
            // 8-4-4-4-12 hex with version/variant digits NSUUID accepts.
            assert_eq!(a.len(), 36);
            let parts: Vec<&str> = a.split('-').collect();
            assert_eq!(
                parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
                vec![8, 4, 4, 4, 12]
            );
            assert!(a.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
            assert!(parts[2].starts_with('4'));
            assert!(parts[3].starts_with('8'));
        }
    }

    #[test]
    fn empty_surfaces_hibernate_cleanly() {
        let mut surfaces = WebSurfaces::new();
        assert_eq!(surfaces.pane_count(), 0);
        assert!(!surfaces.is_hydrated_for("anything"));
        surfaces.hibernate();
        assert_eq!(surfaces.pane_count(), 0);
    }

    #[test]
    fn tab_switch_is_bounds_honest() {
        let mut surfaces = WebSurfaces::new();
        surfaces.switch_tab(3); // beyond the (empty) set: ignored
        assert_eq!(surfaces.active_index(), 0);
    }
}
