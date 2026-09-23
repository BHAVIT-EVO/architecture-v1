//! Wrapper bundles: per-pod macOS app identities.
//!
//! For Dock and Cmd-Tab to show *the work* instead of the app, macOS needs
//! a real app bundle per identity — an established recipe in the wild:
//! a child `.app` with its own `CFBundleIdentifier`, whose executable is a
//! stub script that execs the canonical binary with the pod's isolation
//! args; ad-hoc signed so Gatekeeper treats it as our local artwork.
//!
//! This module generates only **text** (pure, testable): the plist and the
//! launcher script. Materializing the bundle (mkdir, chmod, codesign,
//! lsregister) is [`crate::macos`] under the right OS.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapperSpec {
    /// "Evo Pods — invoice-sync"
    pub label: String,
    /// Inverted-dns id seed: pod-respecting, lowercase, semver-free.
    pub bundle_id: String,
    /// The canonical binary inside the wrapped app (`/Applications/X.app/Contents/MacOS/x`).
    pub target_binary: String,
    /// Isolation args (from the InstancePolicy + pod profile dir).
    pub target_args: Vec<String>,
}

/// Lowercases, strips to [a-z0-9-], never empty, always dns-safe.
pub fn normalize_bundle_id(host_bundle_prefix: &str, pod_name: &str) -> String {
    let mut slug: String = pod_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "work" } else { slug };
    format!("{host_bundle_prefix}.{slug}")
}

/// A minimal, explicit executable shell stub.
pub fn launcher_script(spec: &WrapperSpec) -> String {
    let mut s = String::from("#!/bin/sh\n# Evo pod launcher — execs the canonical app with this pod's isolation.\nexec");
    s.push_str(&format!(" \"{}\"", spec.target_binary));
    for arg in &spec.target_args {
        s.push_str(&format!(" \"{arg}\""));
    }
    s.push_str(" \"$@\"\n");
    s
}

/// The Info.plist: a plain GUI app identity (shows in Dock/Cmd-Tab),
/// carrying nothing it doesn't need. XML produced by hand: the tree is
/// fixed, escaping is explicit, and plist correctness is test-asserted.
pub fn info_plist(spec: &WrapperSpec) -> String {
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" ",
            "\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
            "<plist version=\"1.0\">\n<dict>\n",
            "\t<key>CFBundleExecutable</key>\n\t<string>launcher</string>\n",
            "\t<key>CFBundleIdentifier</key>\n\t<string>{bid}</string>\n",
            "\t<key>CFBundleName</key>\n\t<string>{name}</string>\n",
            "\t<key>CFBundleDisplayName</key>\n\t<string>{name}</string>\n",
            "\t<key>CFBundlePackageType</key>\n\t<string>APPL</string>\n",
            "\t<key>LSMinimumSystemVersion</key>\n\t<string>12.0</string>\n",
            "</dict>\n</plist>\n"
        ),
        bid = escape_xml(&spec.bundle_id),
        name = escape_xml(&spec.label),
    )
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Where a pod's wrapper bundle lives (host-provided root, e.g.
/// `~/Library/Application Support/Evo/pods/<pod>/`).
pub fn wrapper_path(root: &str, spec: &WrapperSpec) -> String {
    format!("{}/{}.app", root.trim_end_matches('/'), spec.label)
}
