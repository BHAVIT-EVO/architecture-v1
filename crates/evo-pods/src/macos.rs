//! Host-platform surface for pods.
//!
//! This crate is deliberately platform-pure: the real `PodSurface`
//! implementation lives in the host app (`evo-desktop`), which already
//! owns the Accessibility plumbing and the ObjC exception guard that makes
//! WebKit-safe FFI honest (`objc_try`). Keeping the OS legs in the host
//! also keeps the pods core compilable and testable everywhere — IS-0023's
//! logic is pure decisions; the host is the body.
//!
//! What exists here instead:
//! * the wrapper-bundle *materializer* — pure `std::fs`/process work that
//!   any POSIX host can perform and mock (no AppKit involved).

use crate::wrapper::{WrapperSpec, info_plist, launcher_script, wrapper_path};
use std::path::{Path, PathBuf};

/// Everything materialization did, so callers can report honestly and
/// leases could undo files if a host wanted that.
#[derive(Debug, Clone, PartialEq)]
pub struct WrapperOutcome {
    pub bundle_path: PathBuf,
    pub wrote_launcher: bool,
    pub wrote_plist: bool,
    /// codesign+lsregister attempted (skipped off-macOS).
    pub registered: bool,
}

/// Writes the wrapper bundle directory shell (plist + executable launcher)
/// under `root`. Returns the honest outcome. **No signing/registration is
/// attempted outside macOS** — the file artifacts are still useful for
/// inspection, and the host's quick-fix is to call
/// [`register_wrapper`] after.
#[cfg(not(target_os = "macos"))]
pub fn materialize_wrapper(root: &str, spec: &WrapperSpec) -> Result<WrapperOutcome, String> {
    write_bundle_files(root, spec, false)
}

/// macOS leg: writes the bundle, then ad-hoc signs and forces
/// LaunchServices to notice so Dock/Cmd-Tab reads the new identity at
/// once. Signing is `-` (ad-hoc): these bundles are local art by design;
/// proper Developer-ID happens at distribution, not per pod.
#[cfg(target_os = "macos")]
pub fn materialize_wrapper(root: &str, spec: &WrapperSpec) -> Result<WrapperOutcome, String> {
    let outcome = write_bundle_files(root, spec, true)?;
    Ok(outcome)
}

fn write_bundle_files(root: &str, spec: &WrapperSpec, try_register: bool) -> Result<WrapperOutcome, String> {
    use std::fs;
    use std::process::Command;

    let bundle = PathBuf::from(wrapper_path(root, spec));
    let macos_dir = bundle.join("Contents").join("MacOS");
    fs::create_dir_all(&macos_dir).map_err(|e| format!("create bundle dirs: {e}"))?;

    let plist_path = bundle.join("Contents").join("Info.plist");
    let launcher_path = macos_dir.join("launcher");
    fs::write(&plist_path, info_plist(spec)).map_err(|e| format!("write plist: {e}"))?;
    fs::write(&launcher_path, launcher_script(spec)).map_err(|e| format!("write launcher: {e}"))?;

    // Executable bit: via chmod (portable, no libc dependency).
    let st = Command::new("/bin/chmod")
        .arg("+x")
        .arg(&launcher_path)
        .status()
        .map_err(|e| format!("chmod spawn: {e}"))?;
    if !st.success() {
        return Err("chmod failed".into());
    }

    let mut registered = false;
    if try_register {
        let _ = Command::new("codesign")
            .args(["--force", "--deep", "--sign", "-"])
            .arg(&bundle)
            .status();
        let _ = Command::new("/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister")
            .arg("-f")
            .arg(&bundle)
            .status();
        registered = true;
    }

    Ok(WrapperOutcome {
        bundle_path: bundle,
        wrote_launcher: true,
        wrote_plist: true,
        registered,
    })
}

/// Root hint for per-pod assets: the daemon's storage parent. The desktop
/// resolves the true path; this crate stays path-naive elsewhere.
pub fn pods_root(storage_root: &Path) -> PathBuf {
    storage_root.join("pods")
}
