//! Startup supervision of the headless voice agent.
//!
//! "Hey Evo" lives in `com.evo.agent`, a per-user LaunchAgent that outlives
//! the window. Nothing in the window owned its registration, so an agent that
//! was never bootstrapped (or that BTM disabled) stayed dead while the app
//! looked healthy. The app now checks the job on every launch and installs it
//! when it is missing.
//!
//! Bounds, from the mission: the `launchctl print` poll gives up after
//! [`PRINT_POLL`]. The install runs off the main thread so a cold bootstrap
//! never delays the window, and never runs under `EVO_TEST_HOME` (a test home
//! must not touch the real login session).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Label of the per-user job the headless agent runs as.
pub const LABEL: &str = "com.evo.agent";
/// How long to keep asking launchd whether the job came up.
pub const PRINT_POLL: Duration = Duration::from_secs(10);
/// How long the installer may take before it is abandoned.
pub const INSTALL_TIMEOUT: Duration = Duration::from_secs(20);
/// How long a liveness answer is reused, so a UI poll does not shell out to
/// `launchctl` per frame. The mission's budget for the heartbeat is 5 s.
pub const STATUS_CACHE: Duration = Duration::from_secs(5);

/// What the startup check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupOutcome {
    /// The job was already registered; nothing was changed.
    AlreadyRunning,
    /// The job was missing and the bundled installer registered it.
    Installed,
    /// Voice is switched off for this launch.
    Disabled,
}

impl StartupOutcome {
    pub fn describe(&self) -> &'static str {
        match self {
            StartupOutcome::AlreadyRunning => "agent already running",
            StartupOutcome::Installed => "agent installed and started",
            StartupOutcome::Disabled => "voice off; agent untouched",
        }
    }
}

/// `gui/<uid>/com.evo.agent`, the launchd target for this login session.
pub fn service_target(uid: &str) -> String {
    format!("gui/{}/{}", uid.trim(), LABEL)
}

/// Where the job's plist belongs.
pub fn plist_path(home: &Path) -> PathBuf {
    home.join("Library").join("LaunchAgents").join(format!("{LABEL}.plist"))
}

/// The installer ships beside the app's own binary (inside
/// `Contents/MacOS`, or in `target/release` for a plain build).
pub fn installer_candidates(exe: &Path) -> Vec<PathBuf> {
    let Some(dir) = exe.parent() else {
        return Vec::new();
    };
    let mut candidates = vec![dir.join("evo-agent-install")];
    if let Some(contents) = dir.parent() {
        candidates.push(contents.join("MacOS").join("evo-agent-install"));
        candidates.push(contents.join("Resources").join("evo-agent-install"));
    }
    candidates
}

/// The voice state the UI is allowed to show.
///
/// The agent owns the microphone, so the window reports the agent's own
/// status when there is one. What it must never do is claim "Off" while the
/// job is live, or claim "Listening" while nothing is registered — the bar
/// said "Off" for a running agent because `voice.status` is written only by
/// the in-process voice loop, which the LaunchAgent does not run.
pub fn derive_state(paused: bool, agent_running: bool, reported: Option<&str>) -> &'static str {
    if paused {
        return "paused";
    }
    if !agent_running {
        return "off";
    }
    match reported {
        Some("paused") => "paused",
        Some("off") => "off",
        _ => "listening",
    }
}

/// Whether the agent's job is live right now, cached for [`STATUS_CACHE`].
pub fn agent_is_running() -> bool {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<Option<(Instant, bool)>>> = OnceLock::new();
    if std::env::var_os("EVO_TEST_HOME").is_some() {
        return false;
    }
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock()
        && let Some((at, value)) = *guard
        && at.elapsed() < STATUS_CACHE
    {
        return value;
    }
    let running = current_uid()
        .map(|uid| job_is_registered(&service_target(&uid)))
        .unwrap_or(false);
    if let Ok(mut guard) = cache.lock() {
        *guard = Some((Instant::now(), running));
    }
    running
}

/// True when `launchctl print` reports the job as registered.
fn job_is_registered(target: &str) -> bool {
    Command::new("launchctl")
        .args(["print", target])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn current_uid() -> Result<String, String> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| format!("cannot read uid: {e}"))?;
    if !output.status.success() {
        return Err("id -u failed".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Checks the LaunchAgent and installs it when it is not there.
///
/// Returns an error only when the agent could not be brought up; the caller
/// logs it. A failure here never blocks the window.
pub fn ensure_agent_running() -> Result<StartupOutcome, String> {
    if std::env::var_os("EVO_TEST_HOME").is_some() {
        return Err("refusing to touch the login session under EVO_TEST_HOME".to_string());
    }
    if std::env::var("EVO_VOICE").map(|value| value == "off").unwrap_or(false) {
        return Ok(StartupOutcome::Disabled);
    }
    let uid = current_uid()?;
    let target = service_target(&uid);
    if job_is_registered(&target) {
        return Ok(StartupOutcome::AlreadyRunning);
    }

    let exe = std::env::current_exe().map_err(|e| format!("cannot locate own binary: {e}"))?;
    let installer = installer_candidates(&exe)
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "agent not running and no evo-agent-install beside {exe:?}; \
                 the window cannot start \"Hey Evo\" without it"
            )
        })?;

    let mut child = Command::new(&installer)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run {}: {e}", installer.display()))?;

    let started = Instant::now();
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() >= INSTALL_TIMEOUT => {
                timed_out = true;
                let _ = child.kill();
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(format!("cannot wait for the installer: {e}")),
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("cannot read installer output: {e}"))?;
    if timed_out {
        return Err(format!(
            "evo-agent-install did not finish within {}s",
            INSTALL_TIMEOUT.as_secs()
        ));
    }
    if !output.status.success() {
        return Err(format!(
            "evo-agent-install failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    // launchd registers the job asynchronously; ask until it answers or the
    // bounded poll runs out.
    let deadline = Instant::now() + PRINT_POLL;
    while Instant::now() < deadline {
        if job_is_registered(&target) {
            return Ok(StartupOutcome::Installed);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(format!(
        "evo-agent-install succeeded but `launchctl print {target}` still reports no job after {}s",
        PRINT_POLL.as_secs()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_the_login_domain_job() {
        assert_eq!(service_target("501\n"), "gui/501/com.evo.agent");
    }

    #[test]
    fn plist_lives_in_the_user_launch_agents() {
        assert_eq!(
            plist_path(Path::new("/Users/example")),
            PathBuf::from("/Users/example/Library/LaunchAgents/com.evo.agent.plist")
        );
    }

    #[test]
    fn the_bar_never_says_off_while_the_job_is_live() {
        assert_eq!(derive_state(false, true, None), "listening");
        assert_eq!(derive_state(false, true, Some("listening")), "listening");
        assert_eq!(derive_state(false, true, Some("paused")), "paused");
    }

    #[test]
    fn the_bar_never_says_listening_with_no_agent() {
        assert_eq!(derive_state(false, false, Some("listening")), "off");
        assert_eq!(derive_state(false, false, None), "off");
    }

    #[test]
    fn the_kill_switch_wins_over_a_live_agent() {
        assert_eq!(derive_state(true, true, Some("listening")), "paused");
    }

    #[test]
    fn the_installer_is_looked_for_beside_the_binary_and_in_the_bundle() {
        let candidates = installer_candidates(Path::new("/Applications/Evo.app/Contents/MacOS/Evo"));
        assert_eq!(
            candidates[0],
            PathBuf::from("/Applications/Evo.app/Contents/MacOS/evo-agent-install")
        );
        assert!(candidates.contains(&PathBuf::from(
            "/Applications/Evo.app/Contents/MacOS/evo-agent-install"
        )));
    }
}
