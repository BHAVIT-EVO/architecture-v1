//! `evo-agent-install` — installs the headless voice agent as a
//! per-user LaunchAgent so "Hey Evo" works from login onward, with or
//! without the Evo window ever being opened.
//!
//! Idempotent by construction: the plist is rewritten, any previous
//! registration is booted out (ignore failure — there may be none),
//! then the new one is bootstrapped. Nothing is installed silently;
//! this binary is the install step.
//!
//! ```text
//! ~/Library/LaunchAgents/com.evo.agent.plist
//!   Label: com.evo.agent
//!   Program: <absolute path of evo-agent beside this installer>
//!   RunAtLoad: true   KeepAlive: true
//!   stdout/stderr → ~/Library/Logs/evo-agent.log
//! ```

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

const LABEL: &str = "com.evo.agent";

fn main() -> ExitCode {
    match install() {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("evo-agent-install: {err}");
            ExitCode::FAILURE
        }
    }
}

fn install() -> Result<String, String> {
    // The agent binary sits beside this installer (same target dir).
    let exe = std::env::current_exe().map_err(|e| format!("cannot locate own binary: {e}"))?;
    let build_dir = exe
        .parent()
        .ok_or_else(|| "installer has no parent directory".to_string())?;
    let built_agent = build_dir.join("evo-agent");
    if !built_agent.is_file() {
        return Err(format!(
            "evo-agent not found beside the installer ({built_agent:?}); \
             build it with: cargo build --release -p evo-desktop \
             --features voice-wake,evo-notch,embeddings --bin evo-agent"
        ));
    }
    let built_daemon = build_dir.join("evo-daemon");
    if !built_daemon.is_file() {
        return Err(format!(
            "evo-daemon not found beside the installer ({built_daemon:?}); \
             build it with: cargo build --release -p evo-daemon"
        ));
    }

    let home = std::env::var("HOME").map_err(|_| "HOME unset".to_string())?;

    // Install the agent OUTSIDE the repo: a LaunchAgent launched from
    // inside ~/Desktop stalls forever in dyld (TCC blocks the open()
    // before main() even runs — verified live: sampled a launchd spawn
    // stuck in `open_with_subsystem`). Application Support is always
    // readable, and the install survives the repo moving.
    let install_dir = PathBuf::from(&home)
        .join("Library")
        .join("Application Support")
        .join("Evo")
        .join("agent");
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| format!("cannot create {}: {e}", install_dir.display()))?;
    let agent = install_dir.join("evo-agent");
    std::fs::copy(&built_agent, &agent)
        .map_err(|e| format!("cannot install {}: {e}", agent.display()))?;
    // DaemonManager resolves the worker beside the headless agent. Installing
    // only evo-agent leaves the microphone loop alive but guarantees every
    // command fails with a refused voice socket.
    let daemon = install_dir.join("evo-daemon");
    std::fs::copy(&built_daemon, &daemon)
        .map_err(|e| format!("cannot install {}: {e}", daemon.display()))?;
    install_runtime_library(build_dir, &agent, &daemon, &install_dir)?;

    // The voice pipeline loads its models from *beside the exe*: the
    // whisper model and the wake ONNX set. Copy them into the stable install
    // rather than symlinking back to a build directory; the background agent
    // must survive a repo cleanup, app replacement, and a moved checkout.
    let model_dirs = [
        build_dir.to_path_buf(),
        build_dir
            .parent()
            .map(|contents| contents.join("Resources"))
            .unwrap_or_default(),
        build_dir
            .parent()
            .map(|contents| contents.join("Resources/models"))
            .unwrap_or_default(),
    ];
    for model in [
        "whisper-base.en.bin",
        "melspectrogram.onnx",
        "embedding_model.onnx",
        "hey_evoh.onnx",
    ] {
        let Some(source) = model_dirs
            .iter()
            .map(|dir| dir.join(model))
            .find(|path| file_is_nonempty(path))
        else {
            return Err(format!(
                "required voice model {model} missing beside installer and in bundle Resources"
            ));
        };
        let target = install_dir.join(model);
        install_file(&source, &target)?;
    }

    install_embedding_assets(build_dir, &home)?;

    let agents_dir = PathBuf::from(&home).join("Library").join("LaunchAgents");
    std::fs::create_dir_all(&agents_dir)
        .map_err(|e| format!("cannot create {}: {e}", agents_dir.display()))?;
    let logs_dir = PathBuf::from(&home).join("Library").join("Logs");
    std::fs::create_dir_all(&logs_dir)
        .map_err(|e| format!("cannot create {}: {e}", logs_dir.display()))?;
    let out_log = logs_dir.join("evo-agent.log");
    let err_log = logs_dir.join("evo-agent.log");

    let plist_path = agents_dir.join(format!("{LABEL}.plist"));
    // There must be one daemon owner per storage root. If the older
    // always-on capture registration is still loaded, retire it before
    // installing the supervisor that owns the daemon child.
    let uid = user_id()?;
    let domain = format!("gui/{uid}");
    let capture_service = format!("{domain}/dev.evo.capture");
    let _ = Command::new("launchctl")
        .args(["bootout", &capture_service])
        .stderr(Stdio::null())
        .status();
    let capture_plist = agents_dir.join("dev.evo.capture.plist");
    let _ = std::fs::remove_file(capture_plist);

    let embedding_dir = PathBuf::from(&home)
        .join("Library")
        .join("Application Support")
        .join("Evo")
        .join("embedding");
    let plist = plist_xml(&agent, &out_log, &err_log, &embedding_dir)?;
    std::fs::write(&plist_path, plist)
        .map_err(|e| format!("cannot write {}: {e}", plist_path.display()))?;

    let service = format!("{domain}/{LABEL}");

    // Idempotent: remove any previous registration; failure just means
    // there was nothing to remove.
    let _ = Command::new("launchctl")
        .args(["bootout", &service])
        .status();

    // launchd can briefly retain the old job after bootout. Retry the
    // bootstrap instead of leaving the agent uninstalled on transient
    // `Input/output error` (exit 5).
    let mut bootstrap = None;
    for attempt in 0..5 {
        let status = Command::new("launchctl")
            .args(["bootstrap", &domain, &plist_path.to_string_lossy()])
            .status()
            .map_err(|e| format!("launchctl bootstrap failed to run: {e}"))?;
        if status.success() || attempt == 4 {
            bootstrap = Some(status);
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let bootstrap = bootstrap.expect("bootstrap attempts always produce a status");
    if !bootstrap.success() {
        return Err(format!(
            "launchctl bootstrap exited with {} (is this a GUI login session?)",
            describe_exit(&bootstrap)
        ));
    }

    Ok(format!(
        "installed {LABEL} → {} (logs: {}). \"Hey Evo\" works from now on, \
         window open or not. Remove with: launchctl bootout gui/{uid}/{LABEL}",
        plist_path.display(),
        out_log.display()
    ))
}

/// Installs the semantic-embedder assets (the five all-MiniLM-L6-v2
/// files fastembed loads) to the stable per-user location evo-voice's
/// offline loader checks as a last resort:
/// `~/Library/Application Support/Evo/embedding`.
///
/// Without this the daemon resolved its embedding cache relative to the
/// cwd — `/` under launchd — and every voice trigger failed honestly with
/// "voice model unavailable". Copy rather than symlink: the loader opens
/// these by path and plain copies survive a repo cleanup.
fn install_embedding_assets(build_dir: &Path, home: &str) -> Result<(), String> {
    const EMBEDDING_FILES: [&str; 5] = [
        "model.onnx",
        "tokenizer.json",
        "config.json",
        "special_tokens_map.json",
        "tokenizer_config.json",
    ];
    // Build-dir layout varies: Cargo uses target/release/model-metadata, while
    // a bundled app keeps the same files in Contents/Resources beside the
    // agent. Check both without falling back to a network/cache discovery.
    let candidates = [
        build_dir.join("model-metadata"),
        build_dir
            .parent()
            .map(|p| p.join("model-metadata"))
            .unwrap_or_default(),
        build_dir
            .parent()
            .map(|p| p.join("Resources"))
            .unwrap_or_default(),
    ];
    let Some(model_dir) = candidates.into_iter().find(|path| {
        path.is_dir()
            && EMBEDDING_FILES
                .iter()
                .all(|name| file_is_nonempty(&path.join(name)))
    }) else {
        return Err(format!(
            "required embedding assets missing beside installer; expected {}",
            build_dir.join("model-metadata").display()
        ));
    };
    let install_dir = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Evo")
        .join("embedding");
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| format!("cannot create {}: {e}", install_dir.display()))?;
    let mut installed = 0;
    for name in EMBEDDING_FILES {
        let source = model_dir.join(name);
        install_file(&source, &install_dir.join(name))?;
        installed += 1;
    }
    if installed < EMBEDDING_FILES.len() {
        return Err(format!(
            "{} holds {} of the 5 embedding assets; reinstall with all of them",
            model_dir.display(),
            installed
        ));
    }
    Ok(())
}

/// Copies a required runtime asset, replacing a previous independent copy.
fn install_file(source: &Path, target: &Path) -> Result<(), String> {
    if !file_is_nonempty(source) {
        return Err(format!(
            "cannot install empty or unreadable runtime asset {}",
            source.display()
        ));
    }
    if source != target {
        // Migrate the old symlink installation without following its target.
        // copy(source, symlink-to-source) truncates the source model itself.
        if std::fs::symlink_metadata(target)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            std::fs::remove_file(target).map_err(|e| {
                format!("cannot replace legacy model link {}: {e}", target.display())
            })?;
        }
        std::fs::copy(source, target)
            .map_err(|e| format!("cannot install {}: {e}", source.display()))?;
    }
    Ok(())
}

fn file_is_nonempty(path: &Path) -> bool {
    path.is_file()
        && std::fs::metadata(path)
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
}

/// Copies the ONNX Runtime dylib when either installed binary has a dynamic
/// dependency on it. The app build uses `@rpath/libonnxruntime.1.dylib`; the
/// installed agent keeps that same contract at `Evo/Frameworks`, one level
/// above `Evo/agent`, so both the agent and daemon resolve the same library.
fn install_runtime_library(
    build_dir: &Path,
    agent: &Path,
    daemon: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    let needs_runtime = binary_needs_onnx_runtime(agent)? || binary_needs_onnx_runtime(daemon)?;
    if !needs_runtime {
        return Ok(());
    }
    let Some(source) = find_runtime_library(build_dir) else {
        return Err(format!(
            "installed voice binaries require ONNX Runtime, but no libonnxruntime*.dylib was found near {}",
            build_dir.display()
        ));
    };
    // A LaunchAgent is not inside an app bundle, so there is no
    // `@executable_path/../Frameworks` directory for dyld to search. Keep
    // the runtime beside the installed binaries and add an executable-local
    // rpath to both consumers. This also makes the install independent of the
    // user's current working directory and of DYLD_* environment variables,
    // which launchd intentionally strips.
    let runtime = install_dir.join("libonnxruntime.1.dylib");
    install_file(&source, &runtime)?;
    for binary in [agent, daemon] {
        ensure_executable_rpath(binary)?;
    }
    Ok(())
}

fn ensure_executable_rpath(binary: &Path) -> Result<(), String> {
    let output = std::process::Command::new("otool")
        .args(["-l", &binary.to_string_lossy()])
        .output()
        .map_err(|e| format!("cannot inspect {} rpaths: {e}", binary.display()))?;
    if !output.status.success() {
        return Err(format!(
            "otool -l {} failed with {}",
            binary.display(),
            describe_exit(&output.status)
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    if text.lines().any(|line| line.contains("@executable_path")) {
        return Ok(());
    }
    let status = std::process::Command::new("install_name_tool")
        .args(["-add_rpath", "@executable_path", &binary.to_string_lossy()])
        .status()
        .map_err(|e| format!("cannot add rpath to {}: {e}", binary.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "install_name_tool failed for {} with {}",
            binary.display(),
            describe_exit(&status)
        ))
    }
}

fn binary_needs_onnx_runtime(binary: &Path) -> Result<bool, String> {
    let output = std::process::Command::new("otool")
        .arg("-L")
        .arg(binary)
        .output()
        .map_err(|e| {
            format!(
                "cannot inspect {} dependencies with otool: {e}",
                binary.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "otool -L {} failed with {}",
            binary.display(),
            describe_exit(&output.status)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.contains("libonnxruntime")))
}

fn find_runtime_library(build_dir: &Path) -> Option<PathBuf> {
    let mut directories = vec![build_dir.to_path_buf()];
    if let Some(parent) = build_dir.parent() {
        directories.extend([
            parent.to_path_buf(),
            parent.join("Frameworks"),
            parent.join("ort-link"),
        ]);
        if let Ok(entries) = std::fs::read_dir(parent) {
            directories.extend(entries.flatten().map(|entry| entry.path()).filter(|path| {
                path.is_dir()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("ort-"))
            }));
        }
    }
    directories
        .into_iter()
        .filter_map(|directory| std::fs::read_dir(directory).ok())
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            file_is_nonempty(path)
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("libonnxruntime") && name.ends_with(".dylib")
                    })
        })
}

/// The plist body; `&`-escape the paths so a home directory with XML
/// metacharacters can't corrupt the document.
fn plist_xml(
    agent: &Path,
    out_log: &Path,
    err_log: &Path,
    embedding_dir: &Path,
) -> Result<String, String> {
    let esc = |p: &Path| -> String {
        p.to_string_lossy()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{agent}</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>EVO_EMBEDDING_MODEL_DIR</key>
        <string>{embedding}</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{out}</string>
    <key>StandardErrorPath</key>
    <string>{err}</string>
</dict>
</plist>
"#,
        agent = esc(agent),
        embedding = esc(embedding_dir),
        out = esc(out_log),
        err = esc(err_log)
    ))
}

/// The real user id (the launchd gui domain is keyed by it).
fn user_id() -> Result<u32, String> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| format!("cannot read uid: {e}"))?;
    if !output.status.success() {
        return Err(format!("id -u exited {}", describe_exit(&output.status)));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("uid not numeric: {e}"))
}

fn describe_exit(status: &std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit {code}"),
        None => format!("signal {}", status.signal().unwrap_or(-1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_agent_plist_pins_the_embedding_model_location() {
        let plist = plist_xml(
            Path::new("/tmp/Evo/agent/evo-agent"),
            Path::new("/tmp/evo-agent.log"),
            Path::new("/tmp/evo-agent.log"),
            Path::new("/tmp/Evo/embedding"),
        )
        .unwrap();
        assert!(plist.contains("<key>EVO_EMBEDDING_MODEL_DIR</key>"));
        assert!(plist.contains("<string>/tmp/Evo/embedding</string>"));
        assert!(plist.contains("<key>KeepAlive</key>\n    <true/>"));
        assert!(!plist.contains("<key>KeepAlive</key>\n    <false/>"));
    }
}
