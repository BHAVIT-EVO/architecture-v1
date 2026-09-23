//! Developer probe: inspects what Accessibility exposes for URL reading in
//! the frontmost application (AXWebArea role, AXURL attribute), with result
//! codes at every step so failures are visible.
//!
//! Usage:
//!   cargo run -p evo-capture --example probe_url


#[cfg(target_os = "macos")]
mod platform_ref {
    #![allow(unused_imports, dead_code)]
    use std::ffi::c_char;
    use std::os::raw::c_void;
    use std::ptr;
}

#[cfg(target_os = "macos")]
use platform_ref::*;


#[cfg(target_os = "macos")]
type Bool = i8;
#[cfg(target_os = "macos")]
type Id = *mut Object;
#[cfg(target_os = "macos")]
type Class = *mut objc_class;
#[cfg(target_os = "macos")]
type Sel = *mut c_void;
#[cfg(target_os = "macos")]
type CFTypeRef = *const c_void;
#[cfg(target_os = "macos")]
type CFArrayRef = *const c_void;
#[cfg(target_os = "macos")]
type CFStringRef = *const c_void;
#[cfg(target_os = "macos")]
type AXUIElementRef = *const c_void;
#[cfg(target_os = "macos")]
type AXError = i32;

#[cfg(target_os = "macos")]
const AX_SUCCESS: AXError = 0;
#[cfg(target_os = "macos")]
const CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;

#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
struct Object {
    _priv: [u8; 0],
}

#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
struct objc_class {
    _priv: [u8; 0],
}

#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
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
}

#[cfg(target_os = "macos")]
const AX_FOCUSED_WINDOW: &[u8] = b"AXFocusedWindow\0";
#[cfg(target_os = "macos")]
const AX_CHILDREN: &[u8] = b"AXChildren\0";
#[cfg(target_os = "macos")]
const AX_CONTENTS: &[u8] = b"AXContents\0";
#[cfg(target_os = "macos")]
const AX_ROLE: &[u8] = b"AXRole\0";
#[cfg(target_os = "macos")]
const AX_URL: &[u8] = b"AXURL\0";
#[cfg(target_os = "macos")]
const AX_TITLE: &[u8] = b"AXTitle\0";
#[cfg(target_os = "macos")]
const AX_WEB_AREA_ROLE: &[u8] = b"AXWebArea\0";
#[cfg(target_os = "macos")]
const AX_GROUP_ROLE: &[u8] = b"AXGroup\0";

#[cfg(target_os = "macos")]
fn make_attr(name: &'static [u8]) -> CFStringRef {
    unsafe { CFStringCreateWithCString(ptr::null(), name.as_ptr() as *const c_char, CFSTRING_ENCODING_UTF8) }
}

#[cfg(target_os = "macos")]
fn attr(name: &'static [u8]) -> CFStringRef {
    match name {
        AX_FOCUSED_WINDOW => static_attr(AX_FOCUSED_WINDOW),
        AX_CHILDREN => static_attr(AX_CHILDREN),
        AX_CONTENTS => static_attr(AX_CONTENTS),
        AX_ROLE => static_attr(AX_ROLE),
        AX_URL => static_attr(AX_URL),
        AX_TITLE => static_attr(AX_TITLE),
        _ => make_attr(name),
    }
}

#[cfg(target_os = "macos")]
fn static_attr(name: &'static [u8]) -> CFStringRef {
    static mut CACHE: [usize; 6] = [0; 6];
    let index = match name {
        AX_FOCUSED_WINDOW => 0,
        AX_CHILDREN => 1,
        AX_CONTENTS => 2,
        AX_ROLE => 3,
        AX_URL => 4,
        AX_TITLE => 5,
        _ => 0,
    };
    unsafe {
        if CACHE[index] != 0 {
            return CACHE[index] as CFStringRef;
        }
        let value = CFStringCreateWithCString(
            ptr::null(),
            name.as_ptr() as *const c_char,
            CFSTRING_ENCODING_UTF8,
        );
        CACHE[index] = value as usize;
        value
    }
}

#[cfg(target_os = "macos")]
unsafe fn cf_to_string(value: CFStringRef) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let length = CFStringGetLength(value);
    if length <= 0 {
        return Some(String::new());
    }
    let max = CFStringGetMaximumSizeForEncoding(length, CFSTRING_ENCODING_UTF8) + 1;
    if max <= 0 {
        return None;
    }
    let mut buffer = vec![0u8; max as usize];
    if CFStringGetCString(
        value,
        buffer.as_mut_ptr() as *mut c_char,
        buffer.len() as isize,
        CFSTRING_ENCODING_UTF8,
    ) == 0
    {
        return None;
    }
    let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
    String::from_utf8(buffer[..end].to_vec()).ok()
}

#[cfg(target_os = "macos")]
unsafe fn copy_string(element: AXUIElementRef, name: &'static [u8]) -> Option<String> {
    let mut value: CFTypeRef = ptr::null();
    if AXUIElementCopyAttributeValue(element, attr(name), &mut value) != AX_SUCCESS
        || value.is_null()
    {
        return None;
    }
    let text = cf_to_string(value as CFStringRef);
    CFRelease(value);
    text
}

#[cfg(target_os = "macos")]
unsafe fn print_element(element: AXUIElementRef, depth: usize) -> bool {
    let role = copy_string(element, AX_ROLE).unwrap_or_default();
    let url = copy_string(element, AX_URL);
    let title = copy_string(element, AX_TITLE).unwrap_or_default();
    let pad = "  ".repeat(depth);
    println!("{pad}role={role:?} url={url:?} title={title:?}");
    url.is_some() && url.as_deref() != Some(title.as_str()) && url.as_deref().unwrap_or("").starts_with("http")
}

#[cfg(target_os = "macos")]
unsafe fn walk(element: AXUIElementRef, depth: usize) -> bool {
    if print_element(element, depth) {
        return true;
    }
    if depth >= 8 {
        return false;
    }
    let mut children: CFTypeRef = ptr::null();
    let child_result = AXUIElementCopyAttributeValue(element, attr(AX_CHILDREN), &mut children);
    if child_result == AX_SUCCESS && !children.is_null() {
        let array = children as CFArrayRef;
        let count = CFArrayGetCount(array);
        for i in 0..count {
            let item = CFArrayGetValueAtIndex(array, i) as AXUIElementRef;
            if walk(item, depth + 1) {
                CFRelease(children);
                return true;
            }
            if i >= 60 {
                break;
            }
        }
        CFRelease(children);
        return false;
    }
    let mut contents: CFTypeRef = ptr::null();
    let contents_result = AXUIElementCopyAttributeValue(element, attr(AX_CONTENTS), &mut contents);
    if contents_result == AX_SUCCESS && !contents.is_null() {
        let array = contents as CFArrayRef;
        let count = CFArrayGetCount(array);
        for i in 0..count {
            let item = CFArrayGetValueAtIndex(array, i) as AXUIElementRef;
            if walk(item, depth + 1) {
                CFRelease(contents);
                return true;
            }
            if i >= 60 {
                break;
            }
        }
        CFRelease(contents);
        return false;
    }
    // Try the element's own AXValue (e.g. the Safari address bar field).
    let mut value_value: CFTypeRef = ptr::null();
    let value_result = AXUIElementCopyAttributeValue(element, attr(b"AXValue\0"), &mut value_value);
    if value_result == AX_SUCCESS && !value_value.is_null() {
        if let Some(text) = cf_to_string(value_value as CFStringRef) {
            println!("{}value={:?}", "  ".repeat(depth), text);
            if text.starts_with("http") {
                CFRelease(value_value);
                return true;
            }
        }
        CFRelease(value_value);
    }
    false
}

#[cfg(target_os = "macos")]
unsafe fn msg_send_id(receiver: Id, selector: Sel) -> Id {
    let raw = objc_msgSend as *const ();
    let function: extern "C" fn(Id, Sel) -> Id = std::mem::transmute(raw);
    function(receiver, selector)
}

#[cfg(target_os = "macos")]
unsafe fn msg_send_i32(receiver: Id, selector: Sel) -> i32 {
    let raw = objc_msgSend as *const ();
    let function: extern "C" fn(Id, Sel) -> i32 = std::mem::transmute(raw);
    function(receiver, selector)
}

#[cfg(target_os = "macos")]
fn frontmost_pid() -> Option<i32> {
    unsafe {
        let workspace = msg_send_id(
            objc_getClass(b"NSWorkspace\0".as_ptr() as *const c_char) as Id,
            sel_registerName(b"sharedWorkspace\0".as_ptr() as *const c_char),
        );
        if workspace.is_null() {
            return None;
        }
        let app = msg_send_id(
            workspace,
            sel_registerName(b"frontmostApplication\0".as_ptr() as *const c_char),
        );
        if app.is_null() {
            return None;
        }
        Some(msg_send_i32(
            app,
            sel_registerName(b"processIdentifier\0".as_ptr() as *const c_char),
        ))
    }
}

#[cfg(target_os = "macos")]
fn main() {
    unsafe {
        let Some(pid) = frontmost_pid() else {
            println!("no frontmost application");
            return;
        };
        println!("frontmost pid: {pid}");
        let app = AXUIElementCreateApplication(pid);
        if app.is_null() {
            println!("cannot create app element");
            return;
        }
        let mut window_value: CFTypeRef = ptr::null();
        let window_result =
            AXUIElementCopyAttributeValue(app, attr(AX_FOCUSED_WINDOW), &mut window_value);
        println!("AXFocusedWindow result: {window_result}");
        if window_result != AX_SUCCESS || window_value.is_null() {
            println!("no focused window");
            CFRelease(app);
            return;
        }
        let found = walk(window_value as AXUIElementRef, 0);
        println!("real http url found: {found}");
        CFRelease(window_value);
        CFRelease(app);
    }
}

/// Off-macOS this target does not exist: it exercises macOS APIs directly.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("probe_url is a macOS diagnostic target; nothing to do on this platform.");
}
