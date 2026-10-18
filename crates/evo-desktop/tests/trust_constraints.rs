//! The trust constraints, enforced rather than merely documented.
//!
//! Evo's promises to the person using it are narrow and absolute, and three of
//! them are structural enough to be tested rather than asserted in prose:
//!
//! * **It never sees the screen.** No screen recording, no window-image
//!   capture, no OCR over pixels.
//! * **It never listens.** No microphone, no ambient audio, no speech
//!   recognition — except inside the one opt-in feature ("Hey Evo"), which must
//!   stay opt-in and cannot become part of the default experience.
//! * **Nothing leaves the machine.** No HTTP or TLS client anywhere in the
//!   workspace. The only sockets Evo opens are local Unix domain sockets for
//!   daemon↔shell IPC.
//!
//! Plus the permission surface that follows from those: the shipped bundle may
//! declare Accessibility and, only when voice is explicitly enabled, the
//! microphone. Never a camera, never screen recording.
//!
//! These are deliberately coarse, manifest- and script-level checks. They cannot
//! prove the absence of a hand-rolled `std::net::TcpStream`, so
//! `no_raw_network_call_exists_in_source` closes that gap at the source level.
//! What they are good at is the failure mode that actually happens: someone adds
//! a dependency or a plist key for a feature and nobody notices that the product
//! just promised something it cannot keep.

use std::fs;
use std::path::{Path, PathBuf};

/// Crates that would give Evo a way to transmit anything it witnessed.
const FORBIDDEN_NETWORK_CRATES: &[&str] = &[
    "reqwest",
    "hyper",
    "ureq",
    "isahc",
    "attohttpc",
    "surf",
    "curl",
    "tungstenite",
    "rustls",
    "native-tls",
    "openssl",
    "async-std",
    "tokio",
];

/// Crates that would give Evo a view of the screen.
const FORBIDDEN_SCREEN_CRATES: &[&str] = &[
    "tesseract",
    "tesseract-rs",
    "leptess",
    "screenshots",
    "xcap",
    "screencapture",
    "screencapturekit",
    "screen-capture-kit",
    "ocrs",
    "rusty-ocr",
];

/// Crates that would give Evo ears. They are permitted — the voice feature
/// exists — but only as *optional* dependencies, so a default build cannot
/// acquire them by accident.
const AUDIO_CRATES: &[&str] = &["cpal", "whisper-rs", "coreaudio-rs", "oboe", "rubato"];

/// Embedding is a local, opt-in capability. Its current implementation has
/// optional hub/network support in its dependency tree, so it must not enter a
/// default build merely because voice transport is present.
const EMBEDDING_CRATES: &[&str] = &["fastembed"];

/// Usage-description keys that would ask macOS for a capability Evo does not
/// have. `NSMicrophoneUsageDescription` is deliberately not in this list: it is
/// governed separately in `the_microphone_is_declared_only_when_voice_is_on`.
const FORBIDDEN_PLIST_KEYS: &[&str] = &[
    "NSCameraUsageDescription",
    "NSScreenCaptureUsageDescription",
    "NSAppleEventUsageDescription",
    "NSDownloadsFolderUsageDescription",
    "NSDesktopFolderUsageDescription",
    "NSDocumentsFolderUsageDescription",
];

/// The workspace root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/evo-desktop has a repository root above it")
        .to_path_buf()
}

/// Every crate manifest in the workspace.
fn manifests(root: &Path) -> Vec<(String, String)> {
    let crates = root.join("crates");
    let mut out = Vec::new();
    for entry in fs::read_dir(&crates).expect("crates/ is readable") {
        let path = entry.expect("directory entry").path().join("Cargo.toml");
        if path.is_file() {
            let text = fs::read_to_string(&path).expect("manifest is readable");
            out.push((path.display().to_string(), text));
        }
    }
    assert!(
        out.len() > 5,
        "expected to find the workspace's crates, found {}",
        out.len()
    );
    out
}

/// Whether a manifest declares `name` as a dependency line, ignoring comments.
///
/// Matches the start of a TOML key, so `coral` does not match `cpal` and a
/// mention inside a comment does not count.
fn declares(manifest: &str, name: &str) -> Option<String> {
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if line.starts_with(&format!("{name} ="))
            || line.starts_with(&format!("\"{name}\" ="))
            || line.starts_with(&format!("{name}="))
        {
            return Some(line.to_string());
        }
    }
    None
}

#[test]
fn nothing_in_the_workspace_can_transmit_what_evo_witnessed() {
    // Constraint C3. The observation log is a record of everything the person
    // did; a single HTTP client dependency is all it would take for a future
    // feature to send it somewhere. Evo must not have the capability at all.
    let root = repo_root();
    let mut offenders = Vec::new();
    for (path, text) in manifests(&root) {
        for forbidden in FORBIDDEN_NETWORK_CRATES {
            if let Some(line) = declares(&text, forbidden) {
                offenders.push(format!("{path}: {line}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "Evo must have no way to transmit what it witnessed, but these manifests \
         declare a network or TLS client:\n  {}\n\n\
         If a feature genuinely needs the network, it does not belong in Evo — \
         the product promise is that information never leaves the machine.",
        offenders.join("\n  ")
    );
}

#[test]
fn nothing_in_the_workspace_can_see_the_screen() {
    // Constraint C1. This is the sharpest trust line in the product, and it is
    // the line every comparable app crosses. It is not negotiable: refusal is
    // permanent, not deferred.
    let root = repo_root();
    let mut offenders = Vec::new();
    for (path, text) in manifests(&root) {
        for forbidden in FORBIDDEN_SCREEN_CRATES {
            if let Some(line) = declares(&text, forbidden) {
                offenders.push(format!("{path}: {line}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "Evo never reads the screen, in any derived form, but these manifests \
         declare a screen-capture or OCR dependency:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn no_source_reaches_anywhere_but_this_machine() {
    // The manifest check above cannot catch a hand-rolled client, so this one is
    // source-level.
    //
    // A blanket ban on TCP would be wrong, and this test's first draft made
    // exactly that mistake: Evo talks to the *local* browser over the Chrome
    // DevTools protocol on a loopback port, which is how it reads tab titles and
    // URLs without ever seeing the screen
    // (`evo-execution/src/browser_macos.rs`). What must hold is the
    // **containment**, not the absence: a file that opens a socket must pin
    // itself to loopback, and no file may reach for an HTTP or TLS client at all.
    let root = repo_root();
    let crates = root.join("crates");
    let mut offenders = Vec::new();
    for (path, source) in rust_sources(&crates) {
        // This test names its own needles; it is not part of the product.
        if path.ends_with("trust_constraints.rs") {
            continue;
        }
        let code = code_without_comments(&source);
        for client in ["reqwest::", "ureq::", "hyper::", "attohttpc::", "isahc::"] {
            if code.contains(client) {
                offenders.push(format!("{path}: names an HTTP client ({client})"));
            }
        }
        let opens_socket = ["TcpStream", "TcpListener", "UdpSocket"]
            .iter()
            .any(|needle| code.contains(needle));
        if opens_socket && !code.contains("is_loopback") {
            offenders.push(format!(
                "{path}: opens a network socket without pinning to loopback \
                 (the file never calls `is_loopback`)"
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "Evo must reach nothing but this machine:\n  {}\n\n\
         A socket is acceptable only in a file that proves it is loopback-only. \
         Anything that can address a host outside this Mac is not.",
        offenders.join("\n  ")
    );
}

/// Source with `//` line comments removed, so a mention in prose does not read
/// as a use in code.
fn code_without_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `.rs` file under `dir`, as (display path, contents), skipping the
/// `target` tree and any generated output.
fn rust_sources(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs")
                && let Ok(text) = fs::read_to_string(&path)
            {
                out.push((path.display().to_string(), text));
            }
        }
    }
    out
}

#[test]
fn audio_capable_crates_are_optional_only() {
    // Constraint C2. The voice feature is permitted to exist; it is not
    // permitted to become something a default build acquires. Any audio or
    // speech dependency must therefore be declared `optional = true`, so
    // enabling it is an explicit act by whoever builds the app.
    let root = repo_root();
    for (path, text) in manifests(&root) {
        for crate_name in AUDIO_CRATES {
            if let Some(line) = declares(&text, crate_name) {
                assert!(
                    line.contains("optional = true"),
                    "{path} declares `{line}` without `optional = true`.\n\n\
                     Evo must never listen by default. An audio dependency that \
                     is not optional is compiled into every build, including the \
                     one a person installs without asking for voice."
                );
            }
        }
    }
}

#[test]
fn embedding_capability_is_opt_in_across_voice_wiring() {
    let root = repo_root();
    let voice_manifest = fs::read_to_string(root.join("crates/evo-voice/Cargo.toml"))
        .expect("evo-voice manifest is readable");
    for crate_name in EMBEDDING_CRATES {
        let line = declares(&voice_manifest, crate_name)
            .unwrap_or_else(|| panic!("evo-voice must declare `{crate_name}` explicitly"));
        assert!(
            line.contains("optional = true"),
            "evo-voice declares `{line}` without optional = true; semantic embeddings must be opt-in"
        );
    }
    assert!(
        voice_manifest
            .lines()
            .any(|line| line.trim_start().starts_with("embeddings =")
                && line.contains("dep:fastembed")),
        "evo-voice must expose fastembed only through its embeddings feature"
    );

    let daemon_manifest = fs::read_to_string(root.join("crates/evo-daemon/Cargo.toml"))
        .expect("evo-daemon manifest is readable");
    assert!(
        daemon_manifest
            .lines()
            .any(|line| line.trim_start().starts_with("embeddings =")
                && line.contains("evo-voice/embeddings")),
        "evo-daemon must propagate its embeddings feature to evo-voice"
    );

    let desktop_manifest = fs::read_to_string(root.join("crates/evo-desktop/Cargo.toml"))
        .expect("evo-desktop manifest is readable");
    assert!(
        desktop_manifest
            .lines()
            .any(|line| line.trim_start().starts_with("embeddings =")
                && line.contains("evo-daemon/embeddings")),
        "evo-desktop must propagate its embeddings feature to the daemon"
    );
    for (path, manifest) in [
        ("evo-voice", &voice_manifest),
        ("evo-daemon", &daemon_manifest),
        ("evo-desktop", &desktop_manifest),
    ] {
        let feature_block = manifest
            .split_once("[features]")
            .map(|(_, rest)| rest.split_once("\n[").map_or(rest, |(block, _)| block))
            .unwrap_or("");
        let default_line = feature_block
            .lines()
            .find(|line| line.trim_start().starts_with("default"))
            .unwrap_or("");
        assert!(
            !default_line.contains("embeddings"),
            "{path} must not enable embeddings by default: {default_line}"
        );
    }
}

#[test]
fn the_app_does_not_enable_voice_by_default() {
    // Constraint C4.1: never ask in advance. If `evo-desktop`'s default feature
    // set pulled in the voice pipeline, installing Evo would mean installing a
    // microphone listener, and the permission screen would be asking for
    // something the person never chose.
    let manifest = fs::read_to_string(repo_root().join("crates/evo-desktop/Cargo.toml"))
        .expect("evo-desktop manifest is readable");
    let default_features = manifest
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("default"))
        .take_while(|line| !line.trim_start().starts_with('['))
        .collect::<Vec<_>>()
        .join("\n");
    for voice_feature in ["voice-wake", "voice-audio"] {
        assert!(
            !default_features.contains(voice_feature),
            "evo-desktop enables `{voice_feature}` in its default features:\n{default_features}\n\n\
             Voice must stay behind an explicit opt-in."
        );
    }
}

#[test]
fn the_bundle_asks_for_the_smallest_permission_surface() {
    // Constraint C4. What the shipped app tells macOS is what the person will
    // be asked to grant, so the generated plist is part of the promise.
    let script = fs::read_to_string(repo_root().join("scripts/build-app.sh"))
        .expect("build-app.sh is readable");

    for key in FORBIDDEN_PLIST_KEYS {
        assert!(
            !script.contains(key),
            "scripts/build-app.sh declares {key}.\n\n\
             Evo reads window titles, file events and URLs. It does not see the \
             screen and it does not read your files by folder permission; asking \
             for those capabilities breaks the only trust story the product has."
        );
    }

    // The microphone key must exist only inside the explicit voice branch, and
    // the base plist must not carry it. Written as a structural check rather
    // than a count so a future edit that moves the key out of the branch fails.
    let plist_start = script
        .find("cat > \"$APP_DIR/Contents/Info.plist\"")
        .expect("the Info.plist heredoc exists");
    let plist_end = script[plist_start..]
        .find("\nPLIST\n")
        .map(|offset| plist_start + offset)
        .expect("the Info.plist heredoc is terminated");
    let base_plist = &script[plist_start..plist_end];
    assert!(
        !base_plist.contains("NSMicrophoneUsageDescription"),
        "the base Info.plist declares the microphone unconditionally.\n\n\
         A plist key is a statement to macOS and to the person reading Evo's \
         permissions, so it may not claim a capability the bundle does not use. \
         Insert it only when EVO_VOICE=on."
    );

    assert!(
        script.contains("EVO_VOICE=\"${EVO_VOICE:-off}\""),
        "scripts/build-app.sh must default voice to off, so the ordinary build \
         produces a bundle with no microphone in it."
    );
    assert!(
        script.contains("NSMicrophoneUsageDescription"),
        "the voice build must still declare the microphone, otherwise EVO_VOICE=on \
         produces an app whose microphone silently fails."
    );
}
