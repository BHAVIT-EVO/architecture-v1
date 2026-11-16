//! Compiles the ObjC++ exception guard (`objc/objc_try.mm`) into the
//! crate.
//!
//! WebKit throws — NSExceptions from its ObjC surface and, underneath,
//! C++ exceptions from the ObjC++ core. Rust aborts if any foreign
//! exception unwinds through a Rust frame, so every risky WebKit call
//! runs inside this @try/@catch shim: refusals become honest errors
//! (see `websurface.rs`). ObjC++ with both exception flags is what makes
//! `@catch (...)` actually catch the C++ ones.

fn main() {
    // About shows when this binary was built, not when the crate was written.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=EVO_BUILD_TIME={secs}");
    println!("cargo:rerun-if-changed=objc/objc_try.mm");
    cc::Build::new()
        .file("objc/objc_try.mm")
        .flag("-fobjc-exceptions")
        .flag("-fcxx-exceptions")
        .flag("-fexceptions")
        .compile("evo_objc_try");
}
