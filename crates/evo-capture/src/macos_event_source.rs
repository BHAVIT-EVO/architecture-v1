//! Real macOS event source.
//!
//! This source witnesses focused-window changes through Accessibility and emits
//! canonical `WindowFocusGained` signals only when macOS directly reports the
//! focused window for the frontmost application.

use crate::adapters::macos::MacOSSignal;

#[cfg(test)]
mod tests {
    use super::imp::pid_from_storage_root;
    use super::{desktop_shell_pid, is_self_window};

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
            let root = std::path::Path::new(&home)
                .join("Library/Application Support/evo/storage");
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
    use super::{is_self_window, MacOSSignal};
    use evo_observation::observed_state::ObservedState;

    use std::ffi::CStr;
    use std::fmt;
    use std::os::raw::{c_char, c_void};
    use std::ptr;
    use std::sync::OnceLock;
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

    type IMP = extern "C" fn(Id, Sel, Id);
    type AXObserverCallback = extern "C" fn(AXObserverRef, AXUIElementRef, CFStringRef, *mut c_void);

    #[link(name = "objc")]
    #[link(name = "Foundation", kind = "framework")]
    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "ApplicationServices", kind = "framework")]
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Class;
        fn objc_allocateClassPair(superclass: Class, name: *const c_char, extra_bytes: usize) -> Class;
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
        focus_attachment: Option<FocusAttachment>,
        /// The pid of Evo's own desktop shell, when this capture process was
        /// spawned by it. Evo's own UI window is excluded from capture at
        /// the composition boundary: witnessing it would be Evo observing
        /// itself, not observing the user's work.
        desktop_shell_pid: Option<i32>,
        /// The most recently emitted window subject. Used only to suppress
        /// duplicate emissions of the same window around attach-time races;
        /// it never influences which subject is emitted.
        last_emitted_focus: Option<(Option<i32>, String)>,
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
        pub fn new<F>(desktop_shell: Option<i32>, callback: F) -> Result<Self, MacOSEventSourceError>
        where
            F: FnMut(MacOSSignal) + 'static,
        {
            unsafe {
                if AXIsProcessTrusted() == 0 {
                    return Err(MacOSEventSourceError::AccessibilityPermissionRequired);
                }

                let callback = Box::into_raw(Box::new(SignalSinkHolder {
                    callback: Box::new(callback),
                    focus_attachment: None,
                    desktop_shell_pid: desktop_shell,
                    last_emitted_focus: None,
                }));

                let observer = create_observer(callback)?;
                let center = workspace_notification_center()?;

                let add_observer = add_observer_sel();
                let notification_name = NSWorkspaceDidActivateApplicationNotification;
                let nil: Id = ptr::null_mut();
                let _ = msg_send_void4(center, add_observer, observer, activate_selector(), notification_name, nil);

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
            let pid = frontmost_application_pid()?;
            if let Some(pid) = pid {
                let holder = &mut *self.callback;
                holder.attach_to_pid(pid)?;
            }
            Ok(())
        }
    }

    impl Drop for MacOSEventSource {
        fn drop(&mut self) {
            unsafe {
                if !self.center.is_null() && !self.observer.ptr.is_null() {
                    let remove_observer = remove_observer_sel();
                    let nil: Id = ptr::null_mut();
                    let _ = msg_send_void3(
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
            if let Some(previous_element) = self.focus_attachment.as_ref().map(|previous| previous.element) {
                self.emit_current_subject(previous_element, true);
            }
            self.focus_attachment.take();

            let element = AXUIElementCreateApplication(pid);
            if element.is_null() {
                eprintln!("EVO-CAPTURE AXUIElementCreateApplication failed for pid {pid}");
                return Err(MacOSEventSourceError::RuntimeSetupFailed("AXUIElementCreateApplication"));
            }

            let mut observer: AXObserverRef = ptr::null_mut();
            if AXObserverCreate(pid, focus_changed_callback, &mut observer) != AX_SUCCESS || observer.is_null() {
                CFRelease(element as CFTypeRef);
                return Err(MacOSEventSourceError::RuntimeSetupFailed("AXObserverCreate"));
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

            CFRunLoopAddSource(CFRunLoopGetCurrent(), run_loop_source, kCFRunLoopDefaultMode);
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

        unsafe fn emit_current_subject(&mut self, element: AXUIElementRef, boundary_sample: bool) {
            match focused_window_subject(element) {
                Some(subject) => {
                    let process_identifier = self.focus_attachment.as_ref().map(|attachment| attachment.pid);
                    if !boundary_sample
                        && self.last_emitted_focus.as_ref()
                            .is_some_and(|(pid, previous)| {
                                *pid == process_identifier && previous == &subject
                            })
                    {
                        eprintln!("EVO-CAPTURE focus unchanged: {subject:?} (suppressed)");
                        return;
                    }
                    eprintln!("EVO-CAPTURE focus: {subject:?}");
                    self.last_emitted_focus = Some((process_identifier, subject.clone()));
                    // The owning process is already in scope at the emit site
                    // (the FocusAttachment the observer is attached through);
                    // it is carried as provenance, never discarded
                    // (BE-TRACE-0001 §2.1). The application element's own
                    // title is the owning application's localized name — the
                    // human-stable identity of the process a bare pid
                    // cannot give downstream readers.
                    let owning_process_name =
                        copy_attribute(element, ax_title_attribute())
                            .and_then(|value| {
                                let name = cfstring_value(value);
                                CFRelease(value);
                                name
                            })
                            .filter(|name: &String| !name.trim().is_empty());
                    let mut state = observed_state(element).unwrap_or_default();
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
                    });
                }
                None => eprintln!("EVO-CAPTURE focus unreadable for attached app"),
            }
        }

        unsafe fn emit(&mut self, signal: MacOSSignal) {
            (self.callback)(signal);
        }
    }

    unsafe fn create_observer(callback: *mut SignalSinkHolder) -> Result<Id, MacOSEventSourceError> {
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

    unsafe fn observer_class() -> Result<Class, MacOSEventSourceError> {
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
            return Err(MacOSEventSourceError::RuntimeSetupFailed("NSObject class lookup"));
        }

        let cls = objc_allocateClassPair(superclass, class_name, 0);
        if cls.is_null() {
            return Err(MacOSEventSourceError::RuntimeSetupFailed("allocate observer class"));
        }

        if class_addIvar(
            cls,
            CALLBACK_IVAR_NAME.as_ptr() as *const c_char,
            std::mem::size_of::<*mut c_void>(),
            SPACEWORK_DIMENSION,
            POINTER_TYPE.as_ptr() as *const c_char,
        ) == 0
        {
            return Err(MacOSEventSourceError::RuntimeSetupFailed("add callback ivar"));
        }

        if class_addMethod(
            cls,
            activate_selector(),
            workspace_did_activate as IMP,
            VOID_AT_AT_COLON_AT_TYPES.as_ptr() as *const c_char,
        ) == 0
        {
            return Err(MacOSEventSourceError::RuntimeSetupFailed("add workspaceDidActivate: method"));
        }

        objc_registerClassPair(cls);
        OBSERVER_CLASS = cls as usize;
        Ok(cls)
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

    unsafe fn focused_window_subject(app: AXUIElementRef) -> Option<String> {
        let window_value = copy_attribute(app, ax_focused_window_attribute())?;

        let window = window_value as AXUIElementRef;
        let Some(title_value) = copy_attribute(window, ax_title_attribute()) else {
            CFRelease(window_value);
            return None;
        };

        let subject = cfstring_to_string(title_value as CFStringRef);
        CFRelease(title_value);
        CFRelease(window_value);

        subject.filter(|subject| !subject.trim().is_empty())
    }

    /// Reads only generic Accessibility attributes directly exposed for the
    /// focused occurrence. Every field is optional; failed or malformed reads
    /// remain absent rather than being inferred from an older observation.
    unsafe fn observed_state(app: AXUIElementRef) -> Option<ObservedState> {
        let focused_window = copy_attribute(app, ax_focused_window_attribute());
        let focused_element = copy_attribute(app, ax_focused_ui_element_attribute());
        let mut state = ObservedState::new();

        if let Some(window) = focused_window {
            if let Some(value) = copy_attribute(window as AXUIElementRef, ax_document_attribute()) {
                if let Some(document) = cfstring_value(value) {
                    state = state.with_document_locator(document);
                }
                CFRelease(value);
            }
        }

        if let Some(element) = focused_element {
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
            if let Some(value) = copy_attribute(element, ax_insertion_point_line_number_attribute()) {
                if let Some(line) = cfnumber_u64(value) {
                    state = state.with_insertion_line(line);
                }
                CFRelease(value);
            }
        }

        if let Some(value) = focused_element { CFRelease(value); }
        if let Some(value) = focused_window { CFRelease(value); }
        (!state.is_empty()).then_some(state)
    }

    unsafe fn copy_attribute(element: AXUIElementRef, attribute: CFStringRef) -> Option<CFTypeRef> {
        let mut value: CFTypeRef = ptr::null();
        (AXUIElementCopyAttributeValue(element, attribute, &mut value) == AX_SUCCESS
            && !value.is_null())
            .then_some(value)
    }

    unsafe fn cfstring_value(value: CFTypeRef) -> Option<String> {
        (CFGetTypeID(value) == CFStringGetTypeID())
            .then(|| cfstring_to_string(value as CFStringRef))
            .flatten()
    }

    unsafe fn cfnumber_u64(value: CFTypeRef) -> Option<u64> {
        if CFGetTypeID(value) != CFNumberGetTypeID() {
            return None;
        }
        let mut number = 0i64;
        if CFNumberGetValue(value, CF_NUMBER_SINT64_TYPE, &mut number as *mut i64 as *mut c_void) == 0 {
            return None;
        }
        u64::try_from(number).ok()
    }

    unsafe fn ax_range_value(value: CFTypeRef) -> Option<(u64, u64)> {
        let value = value as AXValueRef;
        if AXValueGetType(value) != AX_VALUE_CF_RANGE_TYPE {
            return None;
        }
        let mut range = CFRange { location: 0, length: 0 };
        if AXValueGetValue(value, AX_VALUE_CF_RANGE_TYPE, &mut range as *mut CFRange as *mut c_void) == 0 {
            return None;
        }
        Some((u64::try_from(range.location).ok()?, u64::try_from(range.length).ok()?))
    }

    unsafe fn pid_from_workspace_notification(notification: Id) -> Option<i32> {
        let user_info = msg_send_id(notification, user_info_sel());
        if user_info.is_null() {
            return None;
        }

        let workspace_app = msg_send_id1(user_info, object_for_key_sel(), NSWorkspaceApplicationKey);
        if workspace_app.is_null() {
            return None;
        }

        let pid = msg_send_i32(workspace_app, process_identifier_sel());
        Some(pid)
    }

    unsafe fn workspace_notification_center() -> Result<Id, MacOSEventSourceError> {
        let workspace = msg_send_id(class_id(class_name_nsworkspace()) as Id, shared_workspace_sel());
        if workspace.is_null() {
            return Err(MacOSEventSourceError::RuntimeSetupFailed("sharedWorkspace"));
        }
        let center = msg_send_id(workspace, notification_center_sel());
        if center.is_null() {
            return Err(MacOSEventSourceError::RuntimeSetupFailed("notificationCenter"));
        }
        Ok(center)
    }

    unsafe fn frontmost_application_pid() -> Result<Option<i32>, MacOSEventSourceError> {
        let workspace = msg_send_id(class_id(class_name_nsworkspace()) as Id, shared_workspace_sel());
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

    unsafe fn class_name_nsobject() -> &'static CStr {
        CStr::from_bytes_with_nul_unchecked(b"NSObject\0")
    }

    unsafe fn class_name_nsworkspace() -> &'static CStr {
        CStr::from_bytes_with_nul_unchecked(b"NSWorkspace\0")
    }

    unsafe fn class_name_nsdictionary() -> &'static CStr {
        CStr::from_bytes_with_nul_unchecked(b"NSDictionary\0")
    }

    unsafe fn class_name_nsnumber() -> &'static CStr {
        CStr::from_bytes_with_nul_unchecked(b"NSNumber\0")
    }

    unsafe fn class_id(name: &'static CStr) -> Class {
        objc_getClass(name.as_ptr())
    }

    unsafe fn msg_send_id(receiver: Id, selector: Sel) -> Id {
        let raw = objc_msgSend as *const ();
        let function: extern "C" fn(Id, Sel) -> Id = std::mem::transmute(raw);
        function(receiver, selector)
    }

    unsafe fn msg_send_id1(receiver: Id, selector: Sel, arg: Id) -> Id {
        let raw = objc_msgSend as *const ();
        let function: extern "C" fn(Id, Sel, Id) -> Id = std::mem::transmute(raw);
        function(receiver, selector, arg)
    }

    unsafe fn msg_send_id2(receiver: Id, selector: Sel, arg1: Id, arg2: Id) -> Id {
        let raw = objc_msgSend as *const ();
        let function: extern "C" fn(Id, Sel, Id, Id) -> Id = std::mem::transmute(raw);
        function(receiver, selector, arg1, arg2)
    }

    unsafe fn msg_send_void3(receiver: Id, selector: Sel, arg1: Id, arg2: Id, arg3: Id) -> () {
        let raw = objc_msgSend as *const ();
        let function: extern "C" fn(Id, Sel, Id, Id, Id) = std::mem::transmute(raw);
        function(receiver, selector, arg1, arg2, arg3)
    }

    unsafe fn msg_send_void4(receiver: Id, selector: Sel, arg1: Id, arg2: Sel, arg3: Id, arg4: Id) -> () {
        let raw = objc_msgSend as *const ();
        let function: extern "C" fn(Id, Sel, Id, Sel, Id, Id) = std::mem::transmute(raw);
        function(receiver, selector, arg1, arg2, arg3, arg4)
    }

    unsafe fn msg_send_void0(receiver: Id, selector: Sel) {
        let raw = objc_msgSend as *const ();
        let function: extern "C" fn(Id, Sel) = std::mem::transmute(raw);
        function(receiver, selector)
    }

    unsafe fn msg_send_i32(receiver: Id, selector: Sel) -> i32 {
        let raw = objc_msgSend as *const ();
        let function: extern "C" fn(Id, Sel) -> i32 = std::mem::transmute(raw);
        function(receiver, selector)
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
        let storage_root = std::path::Path::new(&home)
            .join("Library/Application Support/evo/storage");
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
        *VALUE.get_or_init(|| create_cfstring(AX_FOCUSED_UI_ELEMENT_ATTRIBUTE) as usize) as CFStringRef
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
        *VALUE.get_or_init(|| create_cfstring(AX_SELECTED_TEXT_RANGE_ATTRIBUTE) as usize) as CFStringRef
    }

    fn ax_insertion_point_line_number_attribute() -> CFStringRef {
        static VALUE: OnceLock<usize> = OnceLock::new();
        *VALUE.get_or_init(|| create_cfstring(AX_INSERTION_POINT_LINE_NUMBER_ATTRIBUTE) as usize) as CFStringRef
    }

    fn create_cfstring(bytes: &'static [u8]) -> CFStringRef {
        unsafe { CFStringCreateWithCString(ptr::null(), bytes.as_ptr() as *const c_char, CFSTRING_ENCODING_UTF8) }
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
        let storage_root = std::path::Path::new(&home)
            .join("Library/Application Support/evo/storage");
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
        pub fn new<F>(_desktop_shell: Option<i32>, _callback: F) -> Result<Self, MacOSEventSourceError>
        where
            F: FnMut(MacOSSignal) + 'static,
        {
            Err(MacOSEventSourceError::UnsupportedPlatform)
        }
    }
}

pub use imp::{MacOSEventSource, MacOSEventSourceError};
