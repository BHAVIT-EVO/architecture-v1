//! Developer probe: checks whether Safari's address-bar text field exposes
//! the current URL through AXValue, and whether an AXWebArea exists anywhere.
//!
//! Usage:
//!   cargo run -p evo-capture --example probe_url2

use std::ffi::c_char;
use std::os::raw::c_void;
use std::ptr;

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
struct Object {
    _priv: [u8; 0],
}
#[repr(C)]
struct objc_class {
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

    static kCFBooleanTrue: CFTypeRef;
}

const AX_FOCUSED_WINDOW: &[u8] = b"AXFocusedWindow\0";
const AX_CHILDREN: &[u8] = b"AXChildren\0";
const AX_ROLE: &[u8] = b"AXRole\0";
const AX_URL: &[u8] = b"AXURL\0";
const AX_VALUE: &[u8] = b"AXValue\0";
const AX_TITLE: &[u8] = b"AXTitle\0";

fn attr(name: &'static [u8]) -> CFStringRef {
    static mut CACHE: [usize; 6] = [0; 6];
    let index = match name {
        AX_FOCUSED_WINDOW => 0,
        AX_CHILDREN => 1,
        AX_ROLE => 2,
        AX_URL => 3,
        AX_VALUE => 4,
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

/// Returns (web_area_url, address_bar_value).
unsafe fn scan(element: AXUIElementRef, depth: usize, hits: &mut Vec<(String, String)>) {
    if depth > 6 {
        return;
    }
    let role = copy_string(element, AX_ROLE).unwrap_or_default();
    let url = copy_string(element, AX_URL);
    let value = copy_string(element, AX_VALUE);
    let pad = "  ".repeat(depth);
    if let Some(url) = url {
        println!("{pad}role={role} AXURL={url:?}");
        if url.starts_with("http") {
            hits.push(("url".into(), url));
        }
    }
    if role == "AXTextField" || role == "AXWebArea" {
        if let Some(value) = &value {
            println!("{pad}role={role} AXValue={value:?}");
            if value.starts_with("http") {
                hits.push(("value".into(), value.clone()));
            }
        }
    }

    let mut children: CFTypeRef = ptr::null();
    if AXUIElementCopyAttributeValue(element, attr(AX_CHILDREN), &mut children) == AX_SUCCESS
        && !children.is_null()
    {
        let array = children as CFArrayRef;
        let count = CFArrayGetCount(array);
        for i in 0..count {
            let item = CFArrayGetValueAtIndex(array, i) as AXUIElementRef;
            scan(item, depth + 1, hits);
            if i >= 40 {
                break;
            }
        }
        CFRelease(children);
    }
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

fn main() {
    unsafe {
        let Some(pid) = frontmost_pid() else {
            println!("no frontmost");
            return;
        };
        println!("frontmost pid: {pid}");
        let app = AXUIElementCreateApplication(pid);

        // Chrome exposes its full web tree only when AXEnhancedUserInterface
        // is requested; this is a documented, permission-free switch.
        let enhanced = CFStringCreateWithCString(
            ptr::null(),
            b"AXEnhancedUserInterface\0".as_ptr() as *const c_char,
            CFSTRING_ENCODING_UTF8,
        );
        let set_result = AXUIElementSetAttributeValue(app, enhanced, kCFBooleanTrue);
        println!("AXEnhancedUserInterface set result: {set_result}");
        CFRelease(enhanced);

        let mut window_value: CFTypeRef = ptr::null();
        let result = AXUIElementCopyAttributeValue(app, attr(AX_FOCUSED_WINDOW), &mut window_value);
        println!("focused window result: {result}");
        if result != AX_SUCCESS || window_value.is_null() {
            return;
        }
        let mut hits = Vec::new();
        scan(window_value as AXUIElementRef, 0, &mut hits);
        println!("hits: {hits:?}");
        CFRelease(window_value);
        CFRelease(app);
    }
}
