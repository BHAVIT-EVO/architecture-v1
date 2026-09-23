//! The live macOS [`PodSurface`]: the pod system's body.
//!
//! Ported whole from IS-0022's room surface (same OS mechanics, proven on
//! this machine) and extended with the pod legs:
//!
//! * **veils** — translucent borderless `NSWindow`s placed over foreign
//!   windows (Dim mode): nothing hides, everything demotes visually;
//! * **screens** — `NSScreen` frames for stage/display signatures;
//! * **move** — `AXPosition`/`AXSize` writes for stage choreography;
//! * **quit** — `NSRunningApplication.terminate` for mothball;
//! * **run** — detached `std::process` launches for isolated instances.
//!
//! The stub below (off-macOS) keeps the crate linkable everywhere; the
//! runtime's logic is platform-pure and tested against fakes.

use evo_pods::surface::{CommandSpec, Frame, PodApp, PodSurface, PodWindow};

#[cfg(target_os = "macos")]
mod imp {
    use evo_pods::surface::{CommandSpec, Frame, PodApp, PodWindow};
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
        fn AXValueCreate(value_type: u32, value: *const c_void) -> CFTypeRef;
        fn AXValueGetValue(value: CFTypeRef, value_type: u32, out: *mut c_void) -> Bool;
    }

    const KAX_WINDOWS_ATTRIBUTE: &[u8] = b"AXWindows\0";
    const KAX_TITLE_ATTRIBUTE: &[u8] = b"AXTitle\0";
    const KAX_DOCUMENT_ATTRIBUTE: &[u8] = b"AXDocument\0";
    const KAX_MINIMIZED_ATTRIBUTE: &[u8] = b"AXMinimized\0";
    const KAX_RAISE_ACTION: &[u8] = b"AXRaise\0";
    const KAX_POSITION_ATTRIBUTE: &[u8] = b"AXPosition\0";
    const KAX_SIZE_ATTRIBUTE: &[u8] = b"AXSize\0";
    const KAX_VALUE_CG_POINT_TYPE: u32 = 1;
    const KAX_VALUE_CG_SIZE_TYPE: u32 = 2;

    const SCREENS_SEL: &[u8] = b"screens\0";
    const FRAME_SEL: &[u8] = b"frame\0";
    const TERMINATE_SEL: &[u8] = b"terminate\0";
    const INIT_WITH_CONTENT_RECT_SEL: &[u8] = b"initWithContentRect:styleMask:backing:defer:\0";
    const SET_LEVEL_SEL: &[u8] = b"setLevel:\0";
    const SET_OPAQUE_SEL: &[u8] = b"setOpaque:\0";
    const SET_IGNORES_MOUSE_SEL: &[u8] = b"setIgnoresMouseEvents:\0";
    const SET_BACKGROUND_COLOR_SEL: &[u8] = b"setBackgroundColor:\0";
    const COLOR_WITH_SRGBA_SEL: &[u8] = b"colorWithSRGBRed:green:blue:alpha:\0";
    const ORDER_FRONT_REGARDLESS_SEL: &[u8] = b"orderFrontRegardless\0";
    const ORDER_OUT_SEL: &[u8] = b"orderOut:\0";
    const SET_FRAME_DISPLAY_SEL: &[u8] = b"setFrame:display:\0";
    const SET_HAS_SHADOW_SEL: &[u8] = b"setHasShadow:\0";
    const RELEASE_SEL: &[u8] = b"release\0";
    /// Borderless (`NSWindowStyleMaskBorderless`); plain overlay, no chrome.
    const NS_BORDERLESS_STYLE_MASK: usize = 0;
    /// `NSBackingStoreBuffered`.
    const NS_BACKING_STORE_BUFFERED: usize = 2;
    /// kCGOverlayWindowLevelKey — above apps, below the drag layers.
    const CG_OVERLAY_WINDOW_LEVEL: isize = 102;

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

    // ── PodSurface implementation ────────────────────────────────────

    pub fn running_apps() -> Result<Vec<PodApp>, String> {
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
                out.push(PodApp {
                    pid: msg_send_i32(app, sel(PROCESS_IDENTIFIER_SEL)),
                    name,
                    hidden: msg_send_bool(app, sel(IS_HIDDEN_SEL)) != 0,
                    regular: policy == ACTIVATION_POLICY_REGULAR,
                });
            }
            Ok(out)
        }
    }

    pub fn windows_of(pid: i32) -> Result<Vec<PodWindow>, String> {
        let mut windows = Vec::new();
        let owner = app_name(pid).unwrap_or_default();
        unsafe {
            with_app_windows(pid, |window| {
                windows.push(PodWindow {
                    pid,
                    owner: owner.clone(),
                    title: window.title.clone(),
                    ax_document: window.document.clone(),
                    window_id: window.window_id,
                    minimized: window.minimized,
                    frame: window.frame.unwrap_or(Frame::new(0.0, 0.0, 0.0, 0.0)),
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
            // request still lands asynchronously. The pod runtime's
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

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct PlainPair {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NsRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    }

    struct LiveWindow {
        window_id: u32,
        title: String,
        document: Option<String>,
        minimized: bool,
        /// The window's readable global frame (position+size), if AX gave
        /// both legs. Pods never guess geometry — a missing half means a
        /// raise-only choreograph leaf.
        frame: Option<Frame>,
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
            let frame = copy_frame(window_element);
            f(&LiveWindow {
                window_id,
                title,
                document,
                minimized,
                frame,
                element: window_element,
            });
        }

        CFRelease(windows_value);
        CFRelease(element as CFTypeRef);
        Ok(())
    }


    /// All screens as global frames, main first (its frame contains the
    /// global origin).
    pub fn screens() -> Vec<Frame> {
        unsafe {
            let screens = msg_send_id(class_id(b"NSScreen\0") as Id, sel(SCREENS_SEL));
            if screens.is_null() {
                return Vec::new();
            }
            let count = msg_send_usize(screens, sel(COUNT_SEL));
            let mut frames = Vec::new();
            for i in 0..count {
                let screen = msg_send_id1(screens, sel(OBJECT_AT_INDEX_SEL), i as Id);
                if screen.is_null() {
                    continue;
                }
                let rect = msg_send_nsrect(screen, sel(FRAME_SEL));
                frames.push(Frame::new(rect.x, rect.y, rect.width, rect.height));
            }
            // Main first: the screen covering the global origin.
            if let Some(main) = frames.iter().position(|f| {
                f.x <= 0.0 && f.y <= 0.0 && f.x + f.w >= 0.0 && f.y + f.h >= 0.0
            }) {
                frames.swap(0, main);
            }
            frames
        }
    }

    /// Applies a global frame: AXPosition then AXSize (size last so apps
    /// whose position snaps to size can't steal the ending rectangle).
    pub fn move_window(pid: i32, window_id: u32, frame: Frame) -> Result<(), String> {
        if !frame.usable() {
            return Err("refusing to move a window to an unusable frame".to_string());
        }
        let mut outcome: Option<Result<(), String>> = None;
        unsafe {
            with_app_windows(pid, |window| {
                if outcome.is_none() && window.window_id == window_id && !window.element.is_null() {
                    let point = PlainPair { x: frame.x, y: frame.y };
                    let size = PlainPair { x: frame.w, y: frame.h };
                    let point_value = AXValueCreate(
                        KAX_VALUE_CG_POINT_TYPE,
                        &point as *const PlainPair as *const c_void,
                    );
                    let size_value = AXValueCreate(
                        KAX_VALUE_CG_SIZE_TYPE,
                        &size as *const PlainPair as *const c_void,
                    );
                    let pos_attr = cfstring(KAX_POSITION_ATTRIBUTE);
                    let size_attr = cfstring(KAX_SIZE_ATTRIBUTE);
                    let err_pos = AXUIElementSetAttributeValue(window.element, pos_attr, point_value);
                    let err_size = AXUIElementSetAttributeValue(window.element, size_attr, size_value);
                    CFRelease(pos_attr);
                    CFRelease(size_attr);
                    CFRelease(point_value);
                    CFRelease(size_value);
                    outcome = Some(if err_pos == AX_SUCCESS && err_size == AX_SUCCESS {
                        Ok(())
                    } else {
                        Err("the window refused its frame (some apps clamp moves)".to_string())
                    });
                }
            })?;
        }
        outcome.unwrap_or_else(|| Err("the window is no longer open".to_string()))
    }

    /// Mothball leg: soft-quit (the app may refuse for unsaved documents,
    /// which is exactly why the runtime refuses pods owing deltas).
    pub fn quit_app(pid: i32) -> Result<(), String> {
        with_running_app(pid, |app| unsafe {
            let _ = msg_send_bool(app, sel(TERMINATE_SEL));
            Ok(())
        })
    }

    /// Detached launch: the pod's isolated instance and wrapper lifecycles
    /// are fire-and-forget shells, executed through std for portability
    /// (no objc on this leg at all).
    pub fn run(cmd: &CommandSpec) -> Result<(), String> {
        let mut command = std::process::Command::new(&cmd.program);
        command.args(&cmd.args);
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command
            .spawn()
            .map(|_| ())
            .map_err(|err| format!("{err}"))
    }

    // ── the veil (Dim's body) ────────────────────────────────────────────

    static mut VEILS: Option<std::sync::Mutex<Vec<(u64, usize)>>> = None;
    use std::sync::atomic::{AtomicU64, Ordering};
    static VEIL_NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

    unsafe fn veil_store() -> &'static std::sync::Mutex<Vec<(u64, usize)>> {
        let veils = &mut *std::ptr::addr_of_mut!(VEILS);
        if veils.is_none() {
            *veils = Some(std::sync::Mutex::new(Vec::new()));
        }
        veils.as_ref().unwrap()
    }

    /// A translucent, borderless, mouse-transparent NSWindow covering
    /// exactly `frame`. Foreign windows stay technically present — Dim is
    /// a statement, never a disappearance.
    pub fn veil_open(frame: Frame, alpha: f32) -> Result<u64, String> {
        unsafe {
            let allocated = msg_send_id(class_id(b"NSWindow\0") as Id, sel(ALLOC_SEL_PATH));
            if allocated.is_null() {
                return Err("NSWindow allocation failed".to_string());
            }
            let rect = NsRect { x: frame.x, y: frame.y, width: frame.w, height: frame.h };
            let window = msg_send_rect4(
                allocated,
                sel(INIT_WITH_CONTENT_RECT_SEL),
                rect,
                NS_BORDERLESS_STYLE_MASK,
                NS_BACKING_STORE_BUFFERED,
                0,
            );
            if window.is_null() {
                return Err("NSWindow init failed".to_string());
            }
            let color_fn: extern "C" fn(Id, Sel, f64, f64, f64, f64) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            let background = color_fn(
                class_id(b"NSColor\0") as Id,
                sel(COLOR_WITH_SRGBA_SEL),
                0.02,
                0.03,
                0.05,
                f64::from(alpha.clamp(0.0, 1.0)),
            );
            let _ = msg_send_id1(window, sel(SET_BACKGROUND_COLOR_SEL), background);
            msg_send_void_isize(window, sel(SET_LEVEL_SEL), CG_OVERLAY_WINDOW_LEVEL);
            msg_send_void_bool(window, sel(SET_OPAQUE_SEL), 0);
            msg_send_void_bool(window, sel(SET_IGNORES_MOUSE_SEL), 1);
            msg_send_void_bool(window, sel(SET_HAS_SHADOW_SEL), 0);
            msg_send_void_id_nil(window, sel(ORDER_FRONT_REGARDLESS_SEL));

            let handle = VEIL_NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
            veil_store()
                .lock()
                .map_err(|_| "veil store poisoned".to_string())?
                .push((handle, window as usize));
            Ok(handle)
        }
    }

    pub fn veil_update(handle: u64, frame: Frame) -> Result<(), String> {
        unsafe {
            let store = veil_store();
            let window = store
                .lock()
                .map_err(|_| "veil store poisoned".to_string())?
                .iter()
                .find(|(h, _)| *h == handle)
                .map(|(_, w)| *w)
                .ok_or_else(|| "the veil is no longer open".to_string())?;
            let rect = NsRect { x: frame.x, y: frame.y, width: frame.w, height: frame.h };
            msg_send_rect_bool(window as Id, sel(SET_FRAME_DISPLAY_SEL), rect, 1);
            Ok(())
        }
    }

    pub fn veil_close(handle: u64) -> Result<(), String> {
        unsafe {
            let store = veil_store();
            let window = {
                let mut guard = store.lock().map_err(|_| "veil store poisoned".to_string())?;
                guard
                    .iter()
                    .position(|(h, _)| *h == handle)
                    .map(|i| guard.remove(i).1)
            }
            .ok_or_else(|| "the veil is no longer open".to_string())?;
            let _ = msg_send_id1(window as Id, sel(ORDER_OUT_SEL), ptr::null_mut());
            let _ = msg_send_id(window as Id, sel(RELEASE_SEL));
            Ok(())
        }
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


    const ALLOC_SEL_PATH: &[u8] = b"alloc\0";

    unsafe fn msg_send_nsrect(receiver: Id, selector: Sel) -> NsRect {
        let function: extern "C" fn(Id, Sel) -> NsRect = std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector)
    }

    unsafe fn msg_send_rect4(receiver: Id, selector: Sel, rect: NsRect, a: usize, b: usize, c: isize) -> Id {
        let function: extern "C" fn(Id, Sel, NsRect, usize, usize, isize) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, rect, a, b, c)
    }

    unsafe fn msg_send_rect_bool(receiver: Id, selector: Sel, rect: NsRect, flag: isize) {
        let function: extern "C" fn(Id, Sel, NsRect, isize) = std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, rect, flag);
    }

    unsafe fn msg_send_void_isize(receiver: Id, selector: Sel, arg: isize) {
        let function: extern "C" fn(Id, Sel, isize) = std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg);
    }

    unsafe fn msg_send_void_bool(receiver: Id, selector: Sel, arg: isize) {
        let function: extern "C" fn(Id, Sel, isize) = std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg);
    }

    unsafe fn msg_send_void_id_nil(receiver: Id, selector: Sel) {
        let function: extern "C" fn(Id, Sel) = std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector);
    }

    /// A window's frame: position + size as one value, only if AX gives
    /// both legs honestly.
    unsafe fn copy_frame(element: AXUIElementRef) -> Option<Frame> {
        let position = copy_point(element, KAX_POSITION_ATTRIBUTE, KAX_VALUE_CG_POINT_TYPE)?;
        let size = copy_point(element, KAX_SIZE_ATTRIBUTE, KAX_VALUE_CG_SIZE_TYPE)?;
        Some(Frame::new(position.x, position.y, size.x, size.y))
    }

    unsafe fn copy_point(element: AXUIElementRef, attribute: &'static [u8], value_type: u32) -> Option<PlainPair> {
        let attr = cfstring(attribute);
        let mut value: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(element, attr, &mut value);
        CFRelease(attr);
        if err != AX_SUCCESS || value.is_null() {
            return None;
        }
        let mut point = PlainPair { x: 0.0, y: 0.0 };
        let ok = AXValueGetValue(value, value_type, &mut point as *mut PlainPair as *mut c_void);
        CFRelease(value);
        if ok != 0 { Some(point) } else { None }
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
pub struct MacPodSurface;

#[cfg(target_os = "macos")]
impl MacPodSurface {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "macos")]
impl PodSurface for MacPodSurface {
    fn running_apps(&self) -> Result<Vec<PodApp>, String> {
        imp::running_apps()
    }
    fn windows_of(&self, pid: i32) -> Result<Vec<PodWindow>, String> {
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
    fn screens(&self) -> Vec<Frame> {
        imp::screens()
    }
    fn move_window(&mut self, pid: i32, window_id: u32, frame: Frame) -> Result<(), String> {
        imp::move_window(pid, window_id, frame)
    }
    fn quit_app(&mut self, pid: i32) -> Result<(), String> {
        imp::quit_app(pid)
    }
    fn run(&mut self, cmd: CommandSpec) -> Result<(), String> {
        imp::run(&cmd)
    }
    fn veil_open(&mut self, frame: Frame, alpha: f32) -> Result<u64, String> {
        imp::veil_open(frame, alpha)
    }
    fn veil_update(&mut self, handle: u64, frame: Frame) -> Result<(), String> {
        imp::veil_update(handle, frame)
    }
    fn veil_close(&mut self, handle: u64) -> Result<(), String> {
        imp::veil_close(handle)
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MacPodSurface;

#[cfg(not(target_os = "macos"))]
impl MacPodSurface {
    pub fn new() -> Self {
        Self
    }
}

/// Off-macOS: the trait is fully implemented, but every verb is an honest
/// "unsupported platform" — the pods logic above it is what gets tested.
#[cfg(not(target_os = "macos"))]
impl PodSurface for MacPodSurface {
    fn running_apps(&self) -> Result<Vec<PodApp>, String> {
        Err("pods require macOS".to_string())
    }
    fn windows_of(&self, _pid: i32) -> Result<Vec<PodWindow>, String> {
        Err("pods require macOS".to_string())
    }
    fn screens(&self) -> Vec<Frame> {
        Vec::new()
    }
    fn hide_app(&mut self, _pid: i32) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn unhide_app(&mut self, _pid: i32) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn quit_app(&mut self, _pid: i32) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn minimize_window(&mut self, _pid: i32, _window_id: u32) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn unminimize_window(&mut self, _pid: i32, _window_id: u32) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn raise_window(&mut self, _pid: i32, _window_id: u32) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn move_window(&mut self, _pid: i32, _window_id: u32, _frame: Frame) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn activate_app(&mut self, _pid: i32) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn veil_open(&mut self, _frame: Frame, _alpha: f32) -> Result<u64, String> {
        Err("pods require macOS".to_string())
    }
    fn veil_update(&mut self, _handle: u64, _frame: Frame) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn veil_close(&mut self, _handle: u64) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
    fn run(&mut self, _cmd: CommandSpec) -> Result<(), String> {
        Err("pods require macOS".to_string())
    }
}
