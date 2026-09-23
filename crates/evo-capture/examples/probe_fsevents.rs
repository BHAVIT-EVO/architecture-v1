//! Developer probe: isolates FSEventStreamCreate parameter handling.
//!
//! Usage:
//!   cargo run -p evo-capture --example probe_fsevents


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
type CFTypeRef = *const c_void;
#[cfg(target_os = "macos")]
type CFArrayRef = *const c_void;
#[cfg(target_os = "macos")]
type CFStringRef = *const c_void;
#[cfg(target_os = "macos")]
type FSEventStreamRef = *const c_void;

#[cfg(target_os = "macos")]
const CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;

#[cfg(target_os = "macos")]
type FSEventStreamCallback = extern "C" fn(
    stream_ref: FSEventStreamRef,
    info: *mut c_void,
    num_events: usize,
    event_paths: *const c_void,
    event_flags: *const u32,
    event_ids: *const u64,
);

#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn FSEventStreamCreate(
        allocator: CFTypeRef,
        callback: FSEventStreamCallback,
        context: *mut FSEventStreamContext,
        paths_to_watch: CFArrayRef,
        since_when: u64,
        latency: f64,
        flags: u32,
    ) -> FSEventStreamRef;
    fn FSEventStreamRelease(stream: FSEventStreamRef);
    fn CFArrayCreate(
        allocator: CFTypeRef,
        values: *const *const c_void,
        num_values: isize,
        callbacks: *const c_void,
    ) -> CFArrayRef;
    fn CFArrayGetTypeID() -> u32;
    fn CFStringCreateWithCString(
        alloc: CFTypeRef,
        c_str: *const c_char,
        encoding: u32,
    ) -> CFStringRef;
    fn CFGetTypeID(cf: CFTypeRef) -> u32;
    fn CFRelease(cf: CFTypeRef);
}

#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
struct FSEventStreamContext {
    version: isize,
    info: *mut c_void,
    retain: *const c_void,
    release: *const c_void,
    copy_description: *const c_void,
}

#[cfg(target_os = "macos")]
extern "C" fn cb(
    _stream: FSEventStreamRef,
    _info: *mut c_void,
    _num: usize,
    _paths: *const c_void,
    _flags: *const u32,
    _ids: *const u64,
) {
}

#[cfg(target_os = "macos")]
fn main() {
    let home = std::env::var("HOME").unwrap();
    let c_path = std::ffi::CString::new(home.clone()).unwrap();
    unsafe {
        let cf_string =
            CFStringCreateWithCString(ptr::null(), c_path.as_ptr() as *const c_char, CFSTRING_ENCODING_UTF8);
        println!("cf_string: {:p}", cf_string);
        println!("cf_string typeid: 0x{:x}", CFGetTypeID(cf_string));

        let mut values = [cf_string as *const c_void];
        let array = CFArrayCreate(ptr::null(), values.as_mut_ptr(), 1, ptr::null());
        println!("array: {:p}", array);
        println!("array typeid: 0x{:x} (expect array typeid 0x{:x})", CFGetTypeID(array), CFArrayGetTypeID());

        let mut context = FSEventStreamContext {
            version: 0,
            info: ptr::null_mut(),
            retain: ptr::null(),
            release: ptr::null(),
            copy_description: ptr::null(),
        };
        let stream = FSEventStreamCreate(
            ptr::null(),
            cb,
            &mut context,
            array,
            u64::MAX,
            0.5,
            0x11, // UseCFTypes | FileEvents
        );
        println!("stream: {:p}", stream);
        if !stream.is_null() {
            FSEventStreamRelease(stream);
        }
        CFRelease(array);
        CFRelease(cf_string);
        println!("probe done");
    }
}

/// Off-macOS this target does not exist: it exercises macOS APIs directly.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("probe_fsevents is a macOS diagnostic target; nothing to do on this platform.");
}
