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
    println!("cargo:rerun-if-changed=objc/objc_try.mm");
    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("macos") {
        // ObjC compilation exists only on Apple targets; the WebKit surface
        // it guards is stub-compiled elsewhere (websurface.rs).
        return;
    }
    cc::Build::new()
        .file("objc/objc_try.mm")
        .flag("-fobjc-exceptions")
        .flag("-fcxx-exceptions")
        .flag("-fexceptions")
        .compile("evo_objc_try");
}
