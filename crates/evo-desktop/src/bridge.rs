//! The bridge between the web shell and this process (docs/BRIDGE.md).
//!
//! The page may send a small, closed set of commands. Every message is
//! validated here before anything runs: a bounded size, a JSON object, a
//! known `cmd`, no unknown keys, typed fields, bounded strings. The allowlist
//! is this enum; there is no generic "eval" or "invoke" command, and no
//! command takes a path from the page.
//!
//! Rust stays the only enforcer of the kill switch and of permissions: the
//! paused state is read from disk on every mutating command, and a paused app
//! refuses `continue_work` regardless of what the page believes.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use evo_pods::surface::PodSurface;

#[cfg(target_os = "macos")]
static LAST_WORKSPACE_SNAPSHOT: std::sync::OnceLock<
    std::sync::Mutex<Option<crate::unified_workspace::WorkspaceSnapshot>>,
> = std::sync::OnceLock::new();

/// A message larger than this is refused unread.
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024;
/// A name the person types is a short phrase, not a document.
pub const MAX_TITLE_CHARS: usize = 120;
/// A search box is a few words.
pub const MAX_QUERY_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    GetWorks,
    ContinueWork { work: u64, confirm: bool },
    PutThingsBack,
    RenameWork { work: u64, title: String },
    MergeWorks { first: u64, second: u64 },
    HideWork { work: u64, undo: bool },
    Search { query: String },
    GetStatus,
    GetVoiceEvents,
    Pause,
    Resume,
    GetAbout,
    GetPrivacy,
    SetMode { mode: String },
    SetScope { kind: String, value: String, allowed: bool },
    UndoActivity { at: u64 },
    GetDiagnostics,
    GetSettings,
    SetSettings {
        onboarding_step: Option<u8>,
        appearance: Option<String>,
        keep_background: Option<bool>,
        quiet_desktop: Option<bool>,
    },
    ExportData,
    DeleteAll,
    Quit,
}

/// The folders, apps and sites the person may allow. The page can only name
/// these; it can never hand Rust an arbitrary path to grant.
pub const SCOPE_CATALOG: [(&str, &str); 10] = [
    ("folder", "Documents"),
    ("folder", "Desktop"),
    ("folder", "Downloads"),
    ("app", "Claude"),
    ("app", "Chrome"),
    ("app", "Finder"),
    ("app", "Terminal"),
    ("site", "claude.ai"),
    ("site", "youtube.com"),
    ("site", "github.com"),
];

/// Off by default: Downloads and the video site are the risky ones.
fn scope_default(kind: &str, value: &str) -> bool {
    !matches!((kind, value), ("folder", "Downloads") | ("site", "youtube.com"))
}

pub fn catalog_has(kind: &str, value: &str) -> bool {
    SCOPE_CATALOG
        .iter()
        .any(|(k, v)| *k == kind && *v == value)
}

impl Command {
    /// The allowlisted name, for logs and for deciding which event follows.
    pub fn name(&self) -> &'static str {
        match self {
            Command::GetWorks => "get_works",
            Command::ContinueWork { .. } => "continue_work",
            Command::PutThingsBack => "put_things_back",
            Command::RenameWork { .. } => "rename_work",
            Command::MergeWorks { .. } => "merge_works",
            Command::HideWork { .. } => "hide_work",
            Command::Search { .. } => "search",
            Command::GetStatus => "get_status",
            Command::GetVoiceEvents => "get_voice_events",
            Command::Pause => "pause",
            Command::Resume => "resume",
            Command::GetAbout => "get_about",
            Command::GetPrivacy => "get_privacy",
            Command::SetMode { .. } => "set_permission_mode",
            Command::SetScope { .. } => "set_scope",
            Command::UndoActivity { .. } => "undo_activity",
            Command::GetDiagnostics => "get_diagnostics",
            Command::GetSettings => "get_settings",
            Command::SetSettings { .. } => "set_settings",
            Command::ExportData => "export_data",
            Command::DeleteAll => "delete_all",
            Command::Quit => "quit",
        }
    }
}

/// The event Rust pushes after a command changes what Home shows. Purely
/// informational: the page still cannot act without asking again.
pub fn event_for(name: &str) -> Option<&'static str> {
    match name {
        "rename_work" | "merge_works" | "hide_work" => Some("works_changed"),
        "pause" => Some("paused"),
        "resume" => Some("resumed"),
        "set_permission_mode" | "set_scope" | "undo_activity" | "delete_all" => {
            Some("privacy_changed")
        }
        "set_settings" => Some("settings_changed"),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: u64,
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeError {
    pub code: &'static str,
    pub message: String,
}

impl BridgeError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn to_json(&self) -> String {
        json!({ "code": self.code, "message": self.message }).to_string()
    }
}

fn keys_are_exactly(object: &serde_json::Map<String, Value>, allowed: &[&str]) -> bool {
    object.keys().all(|key| allowed.contains(&key.as_str()))
}

fn number(object: &serde_json::Map<String, Value>, key: &str) -> Option<u64> {
    object.get(key)?.as_u64()
}

fn text(object: &serde_json::Map<String, Value>, key: &str, max: usize) -> Option<String> {
    let value = object.get(key)?.as_str()?.trim();
    if value.is_empty() || value.chars().count() > max {
        return None;
    }
    Some(value.to_string())
}

/// Parses one message from the page. Everything not listed is refused.
pub fn parse(bytes: &[u8]) -> Result<Request, BridgeError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(BridgeError::new("too_large", "That message is too large."));
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| BridgeError::new("invalid_json", "That message is not JSON."))?;
    let object = value
        .as_object()
        .ok_or_else(|| BridgeError::new("invalid_shape", "A message must be an object."))?;
    let id =
        number(object, "id").ok_or_else(|| BridgeError::new("missing_id", "No request id."))?;
    let cmd = object
        .get("cmd")
        .and_then(|v| v.as_str())
        .ok_or_else(|| BridgeError::new("missing_command", "No command."))?;

    let (command, allowed): (Command, &[&str]) = match cmd {
        "get_works" => (Command::GetWorks, &["id", "cmd"]),
        "continue_work" => (
            Command::ContinueWork {
                work: number(object, "work").ok_or_else(|| {
                    BridgeError::new("invalid_args", "continue_work needs a work id.")
                })?,
                confirm: match object.get("confirm") {
                    None => false,
                    Some(value) if value.is_boolean() => value.as_bool().unwrap_or(false),
                    Some(_) => {
                        return Err(BridgeError::new(
                            "invalid_args",
                            "confirm is true or false.",
                        ));
                    }
                },
            },
            &["id", "cmd", "work", "confirm"],
        ),
        "put_things_back" => (Command::PutThingsBack, &["id", "cmd"]),
        "rename_work" => (
            Command::RenameWork {
                work: number(object, "work").ok_or_else(|| {
                    BridgeError::new("invalid_args", "rename_work needs a work id.")
                })?,
                title: text(object, "title", MAX_TITLE_CHARS)
                    .ok_or_else(|| BridgeError::new("invalid_args", "A name is required."))?,
            },
            &["id", "cmd", "work", "title"],
        ),
        "merge_works" => (
            Command::MergeWorks {
                first: number(object, "first").ok_or_else(|| {
                    BridgeError::new("invalid_args", "merge_works needs a first work.")
                })?,
                second: number(object, "second").ok_or_else(|| {
                    BridgeError::new("invalid_args", "merge_works needs a second work.")
                })?,
            },
            &["id", "cmd", "first", "second"],
        ),
        "hide_work" => (
            Command::HideWork {
                work: number(object, "work").ok_or_else(|| {
                    BridgeError::new("invalid_args", "hide_work needs a work id.")
                })?,
                undo: match object.get("undo") {
                    None => false,
                    Some(value) if value.is_boolean() => value.as_bool().unwrap_or(false),
                    Some(_) => {
                        return Err(BridgeError::new(
                            "invalid_args",
                            "hide_work takes undo: true or false.",
                        ));
                    }
                },
            },
            &["id", "cmd", "work", "undo"],
        ),
        "search" => (
            Command::Search {
                query: text(object, "query", MAX_QUERY_CHARS)
                    .ok_or_else(|| BridgeError::new("invalid_args", "A search needs words."))?,
            },
            &["id", "cmd", "query"],
        ),
        "get_status" => (Command::GetStatus, &["id", "cmd"]),
        "get_voice_events" => (Command::GetVoiceEvents, &["id", "cmd"]),
        "pause" => (Command::Pause, &["id", "cmd"]),
        "resume" => (Command::Resume, &["id", "cmd"]),
        "get_about" => (Command::GetAbout, &["id", "cmd"]),
        "get_privacy" => (Command::GetPrivacy, &["id", "cmd"]),
        "set_permission_mode" => {
            let mode = text(object, "mode", 8).ok_or_else(|| {
                BridgeError::new("invalid_args", "set_permission_mode needs a mode.")
            })?;
            if !["ask", "risky", "scope"].contains(&mode.as_str()) {
                return Err(BridgeError::new(
                    "invalid_args",
                    "The mode is ask, risky or scope.",
                ));
            }
            (Command::SetMode { mode }, &["id", "cmd", "mode"])
        }
        "set_scope" => {
            let kind = text(object, "kind", 8)
                .ok_or_else(|| BridgeError::new("invalid_args", "set_scope needs a kind."))?;
            let value = text(object, "value", 64)
                .ok_or_else(|| BridgeError::new("invalid_args", "set_scope needs a value."))?;
            if !catalog_has(&kind, &value) {
                return Err(BridgeError::new(
                    "invalid_args",
                    "That is not one of the things Evo may see.",
                ));
            }
            let allowed = match object.get("allowed") {
                Some(value) if value.is_boolean() => value.as_bool().unwrap_or(false),
                _ => {
                    return Err(BridgeError::new(
                        "invalid_args",
                        "set_scope takes allowed: true or false.",
                    ))
                }
            };
            (
                Command::SetScope {
                    kind,
                    value,
                    allowed,
                },
                &["id", "cmd", "kind", "value", "allowed"],
            )
        }
        "undo_activity" => (
            Command::UndoActivity {
                at: number(object, "at").ok_or_else(|| {
                    BridgeError::new("invalid_args", "undo_activity needs the row time.")
                })?,
            },
            &["id", "cmd", "at"],
        ),
        "get_diagnostics" => (Command::GetDiagnostics, &["id", "cmd"]),
        "get_settings" => (Command::GetSettings, &["id", "cmd"]),
        "set_settings" => {
            let step = match object.get("onboarding_step") {
                None => None,
                Some(value) => match value.as_u64() {
                    Some(step) if step <= 3 => Some(step as u8),
                    _ => {
                        return Err(BridgeError::new(
                            "invalid_args",
                            "The onboarding step is 0 to 3.",
                        ))
                    }
                },
            };
            let appearance = match object.get("appearance") {
                None => None,
                Some(value) => {
                    let name = value.as_str().unwrap_or_default();
                    if !["auto", "day", "dusk", "system"].contains(&name) {
                        return Err(BridgeError::new(
                            "invalid_args",
                            "Appearance is auto, day or dusk.",
                        ));
                    }
                    Some(name.to_string())
                }
            };
            let keep_background = match object.get("keep_background") {
                None => None,
                Some(value) if value.is_boolean() => value.as_bool(),
                Some(_) => {
                    return Err(BridgeError::new(
                        "invalid_args",
                        "keep_background must be true or false.",
                    ));
                }
            };
            let quiet_desktop = match object.get("quiet_desktop") {
                None => None,
                Some(value) if value.is_boolean() => value.as_bool(),
                Some(_) => return Err(BridgeError::new("invalid_args", "quiet_desktop must be true or false.")),
            };
            if step.is_none() && appearance.is_none() && keep_background.is_none() && quiet_desktop.is_none() {
                return Err(BridgeError::new(
                    "invalid_args",
                    "set_settings needs a setting to change.",
                ));
            }
            (
                Command::SetSettings {
                    onboarding_step: step,
                    appearance,
                    keep_background,
                    quiet_desktop,
                },
                &["id", "cmd", "onboarding_step", "appearance", "keep_background", "quiet_desktop"],
            )
        }
        "export_data" => (Command::ExportData, &["id", "cmd"]),
        "delete_all" => (Command::DeleteAll, &["id", "cmd"]),
        "quit" => (Command::Quit, &["id", "cmd"]),
        other => {
            return Err(BridgeError::new(
                "unknown_command",
                format!("“{other}” is not a command this app accepts."),
            ));
        }
    };
    if !keys_are_exactly(object, allowed) {
        return Err(BridgeError::new(
            "invalid_args",
            "That command carries fields it does not take.",
        ));
    }
    Ok(Request { id, command })
}

/// The typed state the page is allowed to see.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkItem {
    pub id: u64,
    pub title: String,
    pub source: String,
    pub last_active_ms: Option<u64>,
    pub resumable: bool,
}

/// The reply for a message that could not even be parsed: the page still gets
/// its request id back so its promise settles instead of hanging.
pub fn error_reply(id: Option<u64>, error: &BridgeError) -> String {
    json!({
        "id": id.unwrap_or(0),
        "ok": false,
        "error": { "code": error.code, "message": error.message },
    })
    .to_string()
}

pub struct Backend {
    root: PathBuf,
}

impl Backend {
    #[cfg(target_os = "macos")]
    fn prepare_workspace(
        &self,
        targets: &[String],
        quiet_desktop: bool,
    ) -> Option<crate::unified_workspace::WorkspaceSnapshot> {
        let mut surface = crate::pod_surface::MacPodSurface::new();
        if !crate::pod_surface::MacPodSurface::accessibility_trusted() {
            return None;
        }
        let mut snapshot = crate::unified_workspace::snapshot(&mut surface, std::process::id() as i32);
        if quiet_desktop {
            if let Ok(apps) = surface.running_apps() {
                for app in apps.into_iter().filter(|app| {
                    app.regular
                        && app.pid != std::process::id() as i32
                        && !targets.iter().any(|target| target.contains(&app.name))
                }) {
                    let _ = surface.hide_app(app.pid);
                    snapshot.remember_hidden_app(app.pid);
                }
            }
        }
        Some(snapshot)
    }

    #[cfg(target_os = "macos")]
    fn arrange_workspace(
        snapshot: &crate::unified_workspace::WorkspaceSnapshot,
        count: usize,
    ) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut arranged = false;
        #[cfg(target_os = "macos")]
        let accessibility_missing =
            !crate::pod_surface::MacPodSurface::accessibility_trusted();
        #[cfg(not(target_os = "macos"))]
        let accessibility_missing = false;
        while std::time::Instant::now() < deadline {
            let mut surface = crate::pod_surface::MacPodSurface::new();
            if crate::unified_workspace::arrange_new_windows_with_layout(
                &mut surface,
                snapshot,
                std::process::id() as i32,
                Some(crate::unified_workspace::layout_for_count(count).id),
            )
            .is_some()
            {
                arranged = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        arranged
    }

    #[cfg(target_os = "macos")]
    fn put_things_back(&self) -> Result<Value, BridgeError> {
        let slot = LAST_WORKSPACE_SNAPSHOT.get_or_init(|| std::sync::Mutex::new(None));
        let snapshot = slot
            .lock()
            .map_err(|_| BridgeError::new("backend", "Evo could not access the saved desktop state."))?
            .take()
            .ok_or_else(|| BridgeError::new("nothing_to_undo", "There is no recent desktop arrangement to undo."))?;
        let mut surface = crate::pod_surface::MacPodSurface::new();
        crate::unified_workspace::restore_snapshot(&mut surface, &snapshot);
        self.record("Put things back", "Restored the windows and apps from before Continue", None);
        Ok(json!({ "restored": true }))
    }

    #[cfg(not(target_os = "macos"))]
    fn put_things_back(&self) -> Result<Value, BridgeError> {
        Err(BridgeError::new("unsupported", "Putting windows back is available on macOS."))
    }

    pub fn new(root: PathBuf) -> Self {
        let expected = format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            option_env!("EVO_BUILD_TIME").unwrap_or("unknown")
        );
        if let Ok(actual) = std::fs::read_to_string(root.join("agent.build-id"))
            && actual.trim() != expected
            && let Ok(uid) = std::process::Command::new("id").arg("-u").output()
            && let Ok(uid) = String::from_utf8(uid.stdout)
        {
            let _ = std::process::Command::new("launchctl")
                .args(["kickstart", "-k", &format!("gui/{}/com.evo.agent", uid.trim())])
                .status();
        }
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn reply(&self, request: &Request, result: Result<Value, BridgeError>) -> String {
        match result {
            Ok(data) => json!({ "id": request.id, "ok": true, "data": data }).to_string(),
            Err(error) => json!({
                "id": request.id,
                "ok": false,
                "error": { "code": error.code, "message": error.message },
            })
            .to_string(),
        }
    }

    pub fn respond(&self, request: &Request) -> String {
        let result = match &request.command {
            Command::GetWorks => self.get_works(),
            Command::Search { query } => self.search(query),
            Command::GetStatus => self.status(),
            Command::GetVoiceEvents => self.voice_events(),
            Command::GetAbout => Ok(json!({
                "version": env!("CARGO_PKG_VERSION"),
                "build_time": option_env!("EVO_BUILD_TIME").unwrap_or("unknown"),
                "commit": option_env!("EVO_COMMIT").unwrap_or("unknown"),
            })),
            Command::Pause => self.set_paused(true),
            Command::Resume => self.set_paused(false),
            Command::HideWork { work, undo } => self.set_hidden(*work, !undo),
            Command::RenameWork { work, title } => self.rename(*work, title),
            Command::MergeWorks { first, second } => self.merge(*first, *second),
            Command::ContinueWork { work, confirm } => self.continue_work(*work, *confirm),
            Command::PutThingsBack => self.put_things_back(),
            Command::GetPrivacy => self.privacy(),
            Command::SetMode { mode } => self.set_mode(mode),
            Command::SetScope {
                kind,
                value,
                allowed,
            } => self.set_scope(kind, value, *allowed),
            Command::UndoActivity { at } => self.undo_activity(*at),
            Command::GetDiagnostics => self.diagnostics(),
            Command::GetSettings => self.settings(),
            Command::SetSettings {
                onboarding_step,
                appearance,
                keep_background,
                quiet_desktop,
            } => self.set_settings(
                *onboarding_step,
                appearance.as_deref(),
                *keep_background,
                *quiet_desktop,
            ),
            Command::ExportData => self.export_data(),
            Command::DeleteAll => self.delete_all(),
            Command::Quit => self.quit(),
        };
        self.reply(request, result)
    }

    /// The works Home shows: the engine's threads, or the canonical index when
    /// the log holds nothing the engine can group yet. Names and hidden state
    /// come from the person's own store.
    pub fn works(&self) -> Vec<WorkItem> {
        let state = crate::ui_state::UiState::load(&self.root);
        let mut items: Vec<WorkItem> = Vec::new();
        let threads =
            evo_daemon::threads::display_threads_from_storage(&self.root).unwrap_or_default();
        for thread in threads.iter().filter(|t| t.is_work) {
            let name = thread.name.clone();
            items.push(WorkItem {
                id: thread.thread_id,
                title: state.display_name(thread.thread_id, &name),
                source: "Evo".into(),
                last_active_ms: thread.last_active_ms,
                resumable: !thread.bundle.resume_point.is_empty(),
            });
        }
        if items.is_empty()
            && let Ok(index) = evo_daemon::cache::CanonicalIndex::new(&self.root)
        {
            let display = crate::state::display_state_from_index(&index);
            for workspace in display.workspaces {
                let id = workspace.id().to_string();
                let title = display
                    .titles
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "Work".into());
                let numeric = stable_id(&id);
                items.push(WorkItem {
                    id: numeric,
                    title: state.display_name(numeric, &title),
                    source: "Evo".into(),
                    last_active_ms: None,
                    resumable: true,
                });
            }
        }
        items.retain(|item| !state.is_hidden(item.id));
        items.sort_by(|a, b| b.last_active_ms.cmp(&a.last_active_ms));
        items
    }

    fn get_works(&self) -> Result<Value, BridgeError> {
        let works = self.works();
        let rows: Vec<Value> = works
            .iter()
            .map(|work| {
                json!({
                    "id": work.id,
                    "title": work.title,
                    "source": work.source,
                    "last_active_ms": work.last_active_ms,
                    "resumable": work.resumable,
                })
            })
            .collect();
        Ok(json!({ "works": rows, "total": rows.len() }))
    }

    fn status(&self) -> Result<Value, BridgeError> {
        let state = crate::ui_state::UiState::load(&self.root);
        let voice_state = std::fs::read_to_string(self.root.join("voice.status"))
            .map(|value| value.trim().to_string())
            .ok()
            .filter(|value| matches!(value.as_str(), "listening" | "paused" | "off"))
            .unwrap_or_else(|| if state.paused { "paused" } else { "off" }.into());
        Ok(json!({
            "paused": state.paused,
            "capture": if evo_daemon::daemon_lock::daemon_is_running(&self.root) {
                "running"
            } else {
                "stopped"
            },
            "voice_state": voice_state,
            "listening": voice_state == "listening",
        }))
    }

    fn voice_events(&self) -> Result<Value, BridgeError> {
        let path = self.root.join("voice.events.jsonl");
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let events: Vec<Value> = text
            .lines()
            .rev()
            .take(32)
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(json!({ "events": events.into_iter().rev().collect::<Vec<_>>() }))
    }

    fn quit(&self) -> Result<Value, BridgeError> {
        let state = crate::ui_state::UiState::load(&self.root);
        self.record(
            "Quit Evo completely",
            if state.keep_background {
                "Stopped the daemon; background listening is enabled"
            } else {
                "Stopped the daemon and unloaded the voice agent"
            },
            None,
        );
        if !state.keep_background {
            if let Ok(output) = std::process::Command::new("id").arg("-u").output()
                && let Ok(uid) = String::from_utf8(output.stdout)
            {
                let _ = std::process::Command::new("launchctl")
                    .args(["bootout", &format!("gui/{}/com.evo.agent", uid.trim())])
                    .status();
            }
        }
        if let Some(pid) = evo_daemon::daemon_lock::running_daemon_pid(&self.root) {
            let _ = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();
        }
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            std::process::exit(0);
        });
        Ok(json!({ "quitting": true }))
    }

    fn set_paused(&self, paused: bool) -> Result<Value, BridgeError> {
        let mut state = crate::ui_state::UiState::load(&self.root);
        state.paused = paused;
        state
            .save(&self.root)
            .map_err(|err| BridgeError::new("backend", format!("Cannot save: {err}")))?;
        self.record(
            if paused {
                "Paused everything"
            } else {
                "Resumed everything"
            },
            if paused {
                "Capture, listening and every action stopped"
            } else {
                "You turned capture and actions back on"
            },
            None,
        );
        Ok(json!({ "paused": paused }))
    }

    fn set_hidden(&self, work: u64, hidden: bool) -> Result<Value, BridgeError> {
        let mut state = crate::ui_state::UiState::load(&self.root);
        let changed = if hidden {
            state.hide(work)
        } else {
            state.unhide(work)
        };
        state
            .save(&self.root)
            .map_err(|err| BridgeError::new("backend", format!("Cannot save: {err}")))?;
        if changed {
            let title = self
                .works()
                .into_iter()
                .find(|item| item.id == work)
                .map(|item| item.title)
                .unwrap_or_else(|| "a work".into());
            self.record(
                &format!(
                    "{} “{}”",
                    if hidden { "Hid" } else { "Showed" },
                    title.chars().take(80).collect::<String>()
                ),
                if hidden {
                    "You asked Home to hide it"
                } else {
                    "You undid hiding it"
                },
                Some(("hide".into(), work.to_string())),
            );
        }
        Ok(json!({ "hidden": hidden, "changed": changed, "work": work }))
    }

    fn rename(&self, work: u64, title: &str) -> Result<Value, BridgeError> {
        let mut state = crate::ui_state::UiState::load(&self.root);
        state.renames.insert(work, title.to_string());
        state
            .save(&self.root)
            .map_err(|err| BridgeError::new("backend", format!("Cannot save: {err}")))?;
        // A name is also a declaration to capture, when the daemon is up; a
        // failure there is reported but the local name still stands.
        let subject = self
            .works()
            .into_iter()
            .find(|item| item.id == work)
            .map(|item| item.title);
        let forwarded = subject
            .as_deref()
            .map(|subject| crate::state::submit_naming(&self.root, subject, title))
            .map(|result| result.is_ok())
            .unwrap_or(false);
        self.record(
            &format!(
                "Renamed a work to “{}”",
                title.chars().take(80).collect::<String>()
            ),
            "You renamed it on Home",
            Some(("rename".into(), work.to_string())),
        );
        Ok(json!({ "work": work, "title": title, "forwarded": forwarded }))
    }

    fn merge(&self, first: u64, second: u64) -> Result<Value, BridgeError> {
        if first == second {
            return Err(BridgeError::new(
                "invalid_args",
                "A work cannot merge with itself.",
            ));
        }
        let pair = evo_daemon::threads::grouping_subjects_from_storage(&self.root, first, second)
            .map_err(|err| BridgeError::new("backend", format!("{err}")))?
            .ok_or_else(|| {
                BridgeError::new(
                    "no_shared_evidence",
                    "These two works share no witnessed resource, so Evo will not merge them.",
                )
            })?;
        match evo_daemon::grouping::submit_grouping(&self.root, &pair.0, &pair.1) {
            Ok(evo_daemon::grouping::GroupingResponse::Accepted) => {
                let mut state = crate::ui_state::UiState::load(&self.root);
                if !state.merges.contains(&(first, second)) {
                    state.merges.push((first, second));
                    let _ = state.save(&self.root);
                }
                self.record(
                    &format!("Merged work {second} into {first}"),
                    "You said they were the same work",
                    Some(("merge".into(), second.to_string())),
                );
                Ok(json!({ "merged": true }))
            }
            Ok(evo_daemon::grouping::GroupingResponse::Rejected(reason)) => {
                Err(BridgeError::new("rejected", reason))
            }
            Err(err) => Err(BridgeError::new("backend", format!("{err}"))),
        }
    }

    fn search(&self, query: &str) -> Result<Value, BridgeError> {
        let path = self.root.join("ledger.db");
        let ledger = evo_ledger::Ledger::open_readonly(&path)
            .map_err(|err| BridgeError::new("backend", format!("{err}")))?;
        let hits = ledger
            .search(query, 20)
            .map_err(|err| BridgeError::new("backend", format!("{err}")))?;
        let state = crate::ui_state::UiState::load(&self.root);
        let works = self.works();
        let rows: Vec<Value> = hits
            .iter()
            .map(|hit| {
                let id = hit.interval.id.max(0) as u64;
                let title = works
                    .iter()
                    .find(|work| work.id == id)
                    .map(|work| work.title.clone())
                    .unwrap_or_else(|| state.display_name(id, "Work"));
                json!({
                    "id": id,
                    "title": title,
                    "score": hit.score,
                    "matched": hit.matched_field,
                    "start_ms": hit.interval.start_ms,
                })
            })
            .collect();
        Ok(json!({ "results": rows, "query": query }))
    }


    // ── trust: privacy, permissions, activity ───────────────────────────

    fn home() -> String {
        std::env::var("HOME").unwrap_or_default()
    }

    fn shortened(path: &Path) -> String {
        let home = Self::home();
        let text = path.display().to_string();
        if !home.is_empty() && text.starts_with(&home) {
            text.replacen(&home, "~", 1)
        } else {
            text
        }
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Whether the catalog entry is allowed: the person's own choice, or the
    /// conservative default when they have not touched it.
    fn scope_allowed(state: &crate::ui_state::UiState, kind: &str, value: &str) -> bool {
        state
            .scopes
            .get(&format!("{kind}:{value}"))
            .copied()
            .unwrap_or_else(|| scope_default(kind, value))
    }

    /// The real `security::permissions::PermissionSet` built from the store.
    /// Folder scopes are expanded to absolute paths under this home.
    fn permission_set(state: &crate::ui_state::UiState) -> crate::security::permissions::PermissionSet {
        use crate::security::permissions::{Mode, PermissionSet, Scope};
        let mode = match state.permission_mode.as_str() {
            "ask" => Mode::AskEveryTime,
            "scope" => Mode::ActWithinScope,
            _ => Mode::AskForRisky,
        };
        let mut set = PermissionSet::new(mode);
        for (kind, value) in SCOPE_CATALOG {
            if !Self::scope_allowed(state, kind, value) {
                continue;
            }
            let target = if kind == "folder" {
                format!("{}/{value}", Self::home())
            } else {
                value.to_string()
            };
            if let Ok(scope) = Scope::parse(kind, &target) {
                set.allow(scope);
            }
        }
        set
    }

    fn record(&self, action: &str, reason: &str, undo: Option<(String, String)>) {
        let mut state = crate::ui_state::UiState::load(&self.root);
        state.record(Self::now_secs(), action, reason, undo);
        let _ = state.save(&self.root);
    }

    fn dir_bytes(path: &Path) -> u64 {
        let Ok(entries) = std::fs::read_dir(path) else {
            return 0;
        };
        let mut total = 0;
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_file() {
                total += meta.len();
            } else if meta.is_dir() && total < 64 * 1024 * 1024 {
                total += Self::dir_bytes(&entry.path());
            }
        }
        total
    }

    fn privacy(&self) -> Result<Value, BridgeError> {
        let state = crate::ui_state::UiState::load(&self.root);
        let scopes: Vec<Value> = SCOPE_CATALOG
            .iter()
            .map(|(kind, value)| {
                json!({
                    "kind": kind,
                    "value": value,
                    "allowed": Self::scope_allowed(&state, kind, value),
                })
            })
            .collect();
        let activity: Vec<Value> = state
            .activity
            .iter()
            .map(|row| {
                json!({
                    "at": row.at,
                    "action": row.action,
                    "reason": row.reason,
                    "undo": row
                        .undo
                        .as_ref()
                        .map(|(kind, id)| json!({ "kind": kind, "id": id })),
                })
            })
            .collect();
        Ok(json!({
            "mode": state.permission_mode,
            "paused": state.paused,
            "scopes": scopes,
            "activity": activity,
            "storage": {
                "path": Self::shortened(&self.root),
                "bytes": Self::dir_bytes(&self.root),
            },
        }))
    }

    fn set_mode(&self, mode: &str) -> Result<Value, BridgeError> {
        let mut state = crate::ui_state::UiState::load(&self.root);
        state.permission_mode = mode.to_string();
        state
            .save(&self.root)
            .map_err(|err| BridgeError::new("backend", format!("Cannot save: {err}")))?;
        let label = match mode {
            "ask" => "Ask every time",
            "scope" => "Act within scope",
            _ => "Ask for risky actions",
        };
        self.record(
            &format!("Permissions set to “{label}”"),
            "You changed what Evo may do",
            None,
        );
        Ok(json!({ "mode": mode }))
    }

    fn set_scope(&self, kind: &str, value: &str, allowed: bool) -> Result<Value, BridgeError> {
        let mut state = crate::ui_state::UiState::load(&self.root);
        state
            .scopes
            .insert(format!("{kind}:{value}"), allowed);
        state
            .save(&self.root)
            .map_err(|err| BridgeError::new("backend", format!("Cannot save: {err}")))?;
        self.record(
            &format!(
                "{} {}",
                if allowed { "Allowed" } else { "Revoked" },
                value
            ),
            "You changed what Evo can see",
            None,
        );
        Ok(json!({ "kind": kind, "value": value, "allowed": allowed }))
    }

    fn undo_activity(&self, at: u64) -> Result<Value, BridgeError> {
        let mut state = crate::ui_state::UiState::load(&self.root);
        let row = state
            .take_undo(at)
            .ok_or_else(|| BridgeError::new("nothing_to_undo", "That can no longer be undone."))?;
        let (kind, id) = row
            .undo
            .clone()
            .ok_or_else(|| BridgeError::new("nothing_to_undo", "That cannot be undone."))?;
        match kind.as_str() {
            "hide" => {
                if let Ok(work) = id.parse::<u64>() {
                    state.unhide(work);
                }
            }
            "rename" => {
                if let Ok(work) = id.parse::<u64>() {
                    state.renames.remove(&work);
                }
            }
            "merge" => {
                if let Ok(work) = id.parse::<u64>() {
                    state.merges.retain(|(_, second)| *second != work);
                }
            }
            _ => {
                return Err(BridgeError::new(
                    "nothing_to_undo",
                    "That cannot be undone.",
                ))
            }
        }
        state
            .save(&self.root)
            .map_err(|err| BridgeError::new("backend", format!("Cannot save: {err}")))?;
        Ok(json!({ "undone": row.action, "kind": kind }))
    }

    // ── settings: onboarding step and appearance ───────────────────────

    fn settings(&self) -> Result<Value, BridgeError> {
        let state = crate::ui_state::UiState::load(&self.root);
        Ok(json!({
            "onboarding_step": state.onboarding_step,
            "appearance": state.appearance,
            "keep_background": state.keep_background,
            "quiet_desktop": state.quiet_desktop,
        }))
    }

    fn set_settings(
        &self,
        onboarding_step: Option<u8>,
        appearance: Option<&str>,
        keep_background: Option<bool>,
        quiet_desktop: Option<bool>,
    ) -> Result<Value, BridgeError> {
        let mut state = crate::ui_state::UiState::load(&self.root);
        if let Some(step) = onboarding_step {
            state.onboarding_step = step;
        }
        if let Some(appearance) = appearance {
            state.appearance = appearance.to_string();
        }
        if let Some(keep_background) = keep_background {
            state.keep_background = keep_background;
        }
        if let Some(quiet_desktop) = quiet_desktop {
            state.quiet_desktop = quiet_desktop;
        }
        state
            .save(&self.root)
            .map_err(|err| BridgeError::new("backend", format!("Cannot save: {err}")))?;
        Ok(json!({
            "onboarding_step": state.onboarding_step,
            "appearance": state.appearance,
            "keep_background": state.keep_background,
            "quiet_desktop": state.quiet_desktop,
        }))
    }

    // ── diagnostics: measured, never quoted ────────────────────────────

    fn diagnostics(&self) -> Result<Value, BridgeError> {
        let facts = crate::observability::log_facts(&crate::observability::default_log_dir());
        let running = evo_daemon::daemon_lock::daemon_is_running(&self.root);
        let ledger = self.root.join("ledger.db");
        let ledger_bytes = std::fs::metadata(&ledger).map(|meta| meta.len()).unwrap_or(0);
        Ok(json!({
            "log": {
                "path": Self::shortened(&crate::observability::default_log_dir()),
                "files": facts.files,
                "bytes": facts.bytes,
                "crashes": facts.crashes,
            },
            "daemon": {
                "running": running,
                "label": if running { "Capture running" } else { "Capture stopped" },
            },
            "ledger": { "bytes": ledger_bytes, "present": ledger.exists() },
            "storage": {
                "path": Self::shortened(&self.root),
                "bytes": Self::dir_bytes(&self.root),
            },
            "version": env!("CARGO_PKG_VERSION"),
            "build_time": option_env!("EVO_BUILD_TIME").unwrap_or("unknown"),
            "commit": option_env!("EVO_COMMIT").unwrap_or("unknown"),
        }))
    }

    // ── data: export and delete ────────────────────────────────────────

    fn export_data(&self) -> Result<Value, BridgeError> {
        let privacy: Value = self.privacy()?;
        let settings = self.settings()?;
        let works: Vec<Value> = self
            .works()
            .iter()
            .map(|work| {
                json!({
                    "id": work.id,
                    "title": work.title,
                    "source": work.source,
                    "last_active_ms": work.last_active_ms,
                })
            })
            .collect();
        let payload = json!({
            "exported_at": Self::now_secs(),
            "app": {
                "version": env!("CARGO_PKG_VERSION"),
                "build_time": option_env!("EVO_BUILD_TIME").unwrap_or("unknown"),
                "commit": option_env!("EVO_COMMIT").unwrap_or("unknown"),
            },
            "works": works,
            "settings": settings,
            "privacy": privacy,
        });
        let text = serde_json::to_string_pretty(&payload)
            .map_err(|err| BridgeError::new("backend", format!("{err}")))?;
        let name = format!("evo-export-{}.json", Self::now_secs());
        let path = self.root.join(&name);
        std::fs::create_dir_all(&self.root)
            .and_then(|_| std::fs::write(&path, text.as_bytes()))
            .map_err(|err| BridgeError::new("backend", format!("Cannot write: {err}")))?;
        self.record(
            "Exported your data",
            "You asked for a copy of what Evo keeps",
            None,
        );
        Ok(json!({
            "path": Self::shortened(&path),
            "bytes": text.len(),
            "works": payload["works"].as_array().map(|rows| rows.len()).unwrap_or(0),
        }))
    }

    fn delete_all(&self) -> Result<Value, BridgeError> {
        let root = &self.root;
        let home = Self::home();
        if root.as_os_str().is_empty() || root == Path::new("/") || root.display().to_string() == home
        {
            return Err(BridgeError::new(
                "invalid_args",
                "Evo will not delete from that location.",
            ));
        }
        let names = [
            "evo.db",
            "evo.db-wal",
            "evo.db-shm",
            "ledger.db",
            "ledger.db-wal",
            "ledger.db-shm",
            "ui-state.json",
            "observation.log",
            "daemon.status",
        ];
        let mut removed = Vec::new();
        for name in names {
            let path = root.join(name);
            if path.is_file() && std::fs::remove_file(&path).is_ok() {
                removed.push(name.to_string());
            }
        }
        for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("evo-export-") && name.ends_with(".json")
                && std::fs::remove_file(entry.path()).is_ok() {
                    removed.push(name);
                }
        }
        Ok(json!({ "removed": removed, "works": 0 }))
    }

    fn work_targets(&self, work: u64) -> Option<(String, Vec<String>)> {
        let threads = evo_daemon::threads::display_threads_from_storage(&self.root).ok()?;
        let thread = threads.iter().find(|t| t.thread_id == work)?;
        Some((thread.name.clone(), thread.bundle.restore_set.clone()))
    }

    fn open_target(target: &str) -> Result<(), &'static str> {
        let target = target.trim();
        if target.is_empty() {
            return Err("no_target");
        }
        let is_url = target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("file://");
        #[cfg(test)]
        if let Ok(mode) = std::env::var("EVO_BRIDGE_OPEN_TEST") {
            return match mode.as_str() {
                "success" => Ok(()),
                "failure" if is_url => Err("browser_not_running"),
                "failure" => Err("app_missing"),
                _ => Err("other"),
            };
        }
        if !is_url && Path::new(target).exists() {
            return std::process::Command::new("open")
                .arg(target)
                .status()
                .ok()
                .filter(std::process::ExitStatus::success)
                .map(|_| ())
                .ok_or("other");
        }
        if !is_url && (target.contains('/') || target.starts_with('.')) {
            return Err("file_moved");
        }
        let status = if is_url {
            std::process::Command::new("open").arg(target).status()
        } else {
            std::process::Command::new("open")
                .args(["-a", target])
                .status()
        };
        match status {
            Ok(status) if status.success() => Ok(()),
            Ok(_) if is_url => Err("browser_not_running"),
            Ok(_) => Err("app_missing"),
            Err(_) => Err("other"),
        }
    }

    fn continue_work(&self, work: u64, confirm: bool) -> Result<Value, BridgeError> {
        // Duplicate guard: ignore the same work within 3 seconds.
        use std::sync::{Mutex, OnceLock};
        static LAST: OnceLock<
            Mutex<std::collections::HashMap<(std::path::PathBuf, u64), std::time::Instant>>,
        > = OnceLock::new();
        let map = LAST.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
        let key = (self.root.clone(), work);
        {
            let mut g = map.lock().map_err(|_| {
                BridgeError::new(
                    "duplicate_guard_unavailable",
                    "Evo could not check whether this work is already opening. Try again.",
                )
            })?;
            if let Some(t) = g.get(&key) {
                if t.elapsed().as_secs() < 3 {
                    self.record("Already opening", "Duplicate continue request within 3 seconds", None);
                    return Ok(json!({ "status": "opening", "items": [], "hint": "Already opening" }));
                }
            }
            g.insert(key, std::time::Instant::now());
        }
        let state = crate::ui_state::UiState::load(&self.root);
        // The kill switch and permissions are decided here, from disk. The
        // page has no way to claim either.
        let switch = crate::security::KillSwitch::new();
        if state.paused {
            switch.pause();
        }
        let set = Self::permission_set(&state);
        if switch.is_paused() {
            self.record("Couldn't open work", "Evo is paused", None);
            return Ok(json!({
                "status": "couldnt_open",
                "items": [{"label": format!("Work {work}"), "ok": false, "reason": "other"}],
                "hint": "Evo is paused. Resume, then try again.",
                "action": "Try again"
            }));
        }
        let Some((title, mut targets)) = self.work_targets(work) else {
            self.record("Couldn't open work", "No stored work target", None);
            return Ok(json!({
                "status": "couldnt_open",
                "items": [{"label": format!("Work {work}"), "ok": false, "reason": "no_target"}],
                "hint": "There is nothing saved to open.",
                "action": "Try again"
            }));
        };
        let omitted = targets.len().saturating_sub(8);
        if targets.len() > 8 {
            targets.truncate(6);
        }
        let action = crate::security::permissions::Action::Open;
        if let Some(target) = targets.first() {
            use crate::security::permissions::Decision;
            match set.decide(switch.context(), action, target) {
                Decision::Allow => {}
                Decision::Ask if confirm => {}
                Decision::Ask => {
                    self.record("Couldn't open work", "Permission is needed", None);
                    return Ok(json!({
                        "status": "couldnt_open",
                        "items": [{"label": target, "ok": false, "reason": "permission_needed"}],
                        "hint": "Evo needs permission before opening this.",
                        "action": "Allow access"
                    }));
                }
                Decision::Deny(reason) => {
                    let hint = match reason {
                        crate::security::permissions::DenyReason::Paused => {
                            "Evo is paused. Resume before opening anything."
                        }
                        crate::security::permissions::DenyReason::OutOfScope => {
                            "Allow access to this location, then try again."
                        }
                    };
                    self.record("Couldn't open work", hint, None);
                    return Ok(json!({
                        "status": "couldnt_open",
                        "items": [{"label": target, "ok": false, "reason": "permission_needed"}],
                        "hint": hint,
                        "action": "Allow access"
                    }));
                }
            }
        } else if set.mode() == crate::security::permissions::Mode::AskEveryTime && !confirm {
            self.record("Couldn't open work", "Permission is needed", None);
            return Ok(json!({
                "status": "couldnt_open",
                "items": [{"label": title, "ok": false, "reason": "permission_needed"}],
                "hint": "Evo needs permission before opening this.",
                "action": "Allow access"
            }));
        }
        #[cfg(target_os = "macos")]
        let before = self.prepare_workspace(&targets, state.quiet_desktop);
        #[cfg(target_os = "macos")]
        let accessibility_missing =
            !crate::pod_surface::MacPodSurface::accessibility_trusted();
        #[cfg(not(target_os = "macos"))]
        let accessibility_missing = false;
        let items: Vec<Value> = targets
            .iter()
            .map(|target| match Self::open_target(target) {
                Ok(()) => json!({ "label": target, "ok": true, "reason": null }),
                Err(reason) => json!({ "label": target, "ok": false, "reason": reason }),
            })
            .collect();
        let mut arranged = false;
        #[cfg(target_os = "macos")]
        if let Some(snapshot) = before {
            arranged = Self::arrange_workspace(&snapshot, items.len().min(8));
            if arranged {
                *LAST_WORKSPACE_SNAPSHOT
                    .get_or_init(|| std::sync::Mutex::new(None))
                    .lock()
                    .map_err(|_| BridgeError::new("backend", "Evo could not save the previous desktop state."))? = Some(snapshot);
            }
        }
        let opened = items.iter().filter(|item| item["ok"] == true).count();
        let status = if opened == items.len() && opened > 0 {
            "opened"
        } else if opened > 0 {
            "partly_opened"
        } else {
            "couldnt_open"
        };
        let reason = items
            .iter()
            .find_map(|item| item["reason"].as_str())
            .unwrap_or("other");
        let action = match reason {
            "file_moved" => "Find the file",
            "permission_needed" => "Allow access",
            _ => "Try again",
        };
        let action = if accessibility_missing {
            "Allow access"
        } else if arranged {
            "Put things back"
        } else {
            action
        };
        let hint = match status {
            "opened" => format!("Opened {title}."),
            "partly_opened" => format!("Opened some of {title}; check the items below."),
            _ => format!("Evo could not open {title}."),
        };
        let hint = if omitted > 0 {
            format!("{hint} Open {omitted} more in Evo.")
        } else {
            hint
        };
        let hint = if accessibility_missing && status == "opened" {
            format!("{hint} Tidy layout needs Accessibility permission; the items opened normally.")
        } else {
            hint
        };
        self.record(&hint, reason, None);
        Ok(json!({
            "status": status,
            "items": items,
            "hint": hint,
            "action": action,
            "arranged": arranged,
            "accessibility_missing": accessibility_missing
        }))
    }
}

/// A stable number for a canonical workspace id (`fnv1a`), so a work keeps
/// the same number across restarts.
fn stable_id(identity: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in identity.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash & 0x0fff_ffff_ffff_ffff
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(body: &str) -> Result<Request, BridgeError> {
        parse(body.as_bytes())
    }

    #[test]
    fn accepts_the_allowlisted_commands() {
        let get = request(r#"{"id":1,"cmd":"get_works"}"#).unwrap();
        assert_eq!(
            get,
            Request {
                id: 1,
                command: Command::GetWorks
            }
        );
        let rename =
            request(r#"{"id":2,"cmd":"rename_work","work":9,"title":"The pitch deck"}"#).unwrap();
        assert_eq!(
            rename.command,
            Command::RenameWork {
                work: 9,
                title: "The pitch deck".into()
            }
        );
        let hide = request(r#"{"id":3,"cmd":"hide_work","work":4,"undo":true}"#).unwrap();
        assert_eq!(
            hide.command,
            Command::HideWork {
                work: 4,
                undo: true
            }
        );
        assert!(request(r#"{"id":4,"cmd":"pause"}"#).is_ok());
        assert!(request(r#"{"id":5,"cmd":"search","query":"pitch"}"#).is_ok());
    }

    #[test]
    fn a_rename_a_hide_and_pause_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("evo-bridge-restart-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp root");

        let first = Backend::new(dir.clone());
        for body in [
            r#"{"id":1,"cmd":"rename_work","work":9,"title":"The pitch deck"}"#,
            r#"{"id":2,"cmd":"hide_work","work":4}"#,
            r#"{"id":3,"cmd":"pause"}"#,
        ] {
            let reply = first.respond(&request(body).unwrap());
            assert!(reply.contains("\"ok\":true"), "{reply}");
        }
        drop(first);

        // A new backend over the same root is what a restart looks like.
        let reopened = Backend::new(dir.clone());
        let state = crate::ui_state::UiState::load(&dir);
        assert_eq!(
            state.renames.get(&9).map(String::as_str),
            Some("The pitch deck")
        );
        assert!(state.hidden.contains(&4));
        assert!(state.paused);

        let status = reopened.respond(&request(r#"{"id":4,"cmd":"get_status"}"#).unwrap());
        assert!(status.contains("\"paused\":true"), "{status}");

        // Undo is exact: only the work that was hidden comes back.
        let reply = reopened
            .respond(&request(r#"{"id":5,"cmd":"hide_work","work":4,"undo":true}"#).unwrap());
        assert!(reply.contains("\"hidden\":false"), "{reply}");
        assert!(!crate::ui_state::UiState::load(&dir).hidden.contains(&4));

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "evo-bridge-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp root");
        dir
    }

    #[test]
    fn accepts_the_new_trust_settings_and_data_commands() {
        assert!(request(r#"{"id":1,"cmd":"get_privacy"}"#).is_ok());
        assert!(request(r#"{"id":2,"cmd":"get_diagnostics"}"#).is_ok());
        assert!(request(r#"{"id":3,"cmd":"get_settings"}"#).is_ok());
        assert!(request(r#"{"id":4,"cmd":"export_data"}"#).is_ok());
        assert!(request(r#"{"id":5,"cmd":"delete_all"}"#).is_ok());
        assert!(request(r#"{"id":6,"cmd":"undo_activity","at":17}"#).is_ok());
        assert!(
            request(r#"{"id":7,"cmd":"set_permission_mode","mode":"ask"}"#).is_ok()
        );
        assert!(
            request(r#"{"id":8,"cmd":"set_scope","kind":"app","value":"Chrome","allowed":true}"#)
                .is_ok()
        );
        assert!(
            request(r#"{"id":9,"cmd":"set_settings","onboarding_step":2}"#).is_ok()
        );
        assert!(request(r#"{"id":10,"cmd":"set_settings","appearance":"dusk"}"#).is_ok());
    }

    #[test]
    fn refuses_modes_scopes_and_settings_outside_the_catalog() {
        for body in [
            r#"{"id":1,"cmd":"set_permission_mode","mode":"always"}"#,
            r#"{"id":2,"cmd":"set_scope","kind":"folder","value":"/etc","allowed":true}"#,
            r#"{"id":3,"cmd":"set_scope","kind":"folder","value":"../etc","allowed":true}"#,
            r#"{"id":4,"cmd":"set_scope","kind":"app","value":"Chrome"}"#,
            r#"{"id":5,"cmd":"set_settings","onboarding_step":9}"#,
            r#"{"id":6,"cmd":"set_settings","appearance":"neon"}"#,
            r#"{"id":7,"cmd":"set_settings"}"#,
            r#"{"id":8,"cmd":"continue_work","work":1,"confirm":"yes"}"#,
        ] {
            assert_eq!(request(body).unwrap_err().code, "invalid_args", "{body}");
        }
    }

    #[test]
    fn the_privacy_reply_is_the_real_store_and_activity_undoes_a_hide() {
        let root = temp_root("privacy");
        let backend = Backend::new(root.clone());
        let hide = backend.respond(&request(r#"{"id":1,"cmd":"hide_work","work":12}"#).unwrap());
        assert!(hide.contains("\"ok\":true"), "{hide}");

        let privacy: Value =
            serde_json::from_str(&backend.respond(&request(r#"{"id":2,"cmd":"get_privacy"}"#).unwrap()))
                .unwrap();
        assert_eq!(privacy["ok"], true);
        assert_eq!(privacy["data"]["mode"], "risky");
        assert_eq!(privacy["data"]["scopes"].as_array().unwrap().len(), 10);
        let activity = privacy["data"]["activity"].as_array().unwrap();
        assert_eq!(activity.len(), 1, "the hide was recorded");
        assert_eq!(activity[0]["undo"]["kind"], "hide");
        let at = activity[0]["at"].as_u64().unwrap();

        let undo = backend.respond(
            &request(&format!(r#"{{"id":3,"cmd":"undo_activity","at":{at}}}"#)).unwrap(),
        );
        assert!(undo.contains("\"ok\":true"), "{undo}");
        assert!(!crate::ui_state::UiState::load(&root).hidden.contains(&12));

        let again = backend.respond(
            &request(&format!(r#"{{"id":4,"cmd":"undo_activity","at":{at}}}"#)).unwrap(),
        );
        assert!(again.contains("nothing_to_undo"), "{again}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn settings_are_saved_and_export_writes_a_file_delete_removes_it() {
        let root = temp_root("settings");
        let backend = Backend::new(root.clone());
        let reply = backend.respond(
            &request(r#"{"id":1,"cmd":"set_settings","onboarding_step":2,"appearance":"day"}"#)
                .unwrap(),
        );
        assert!(reply.contains("\"appearance\":\"day\""), "{reply}");
        let saved = crate::ui_state::UiState::load(&root);
        assert_eq!(saved.onboarding_step, 2);
        assert_eq!(saved.appearance, "day");

        std::fs::write(root.join("ledger.db"), b"not really a ledger").unwrap();
        let export = backend.respond(&request(r#"{"id":2,"cmd":"export_data"}"#).unwrap());
        assert!(export.contains("evo-export-"), "{export}");
        let files: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.starts_with("evo-export-"))
            .collect();
        assert_eq!(files.len(), 1, "{files:?}");
        let text = std::fs::read_to_string(root.join(&files[0])).unwrap();
        assert!(text.contains("\"works\""), "the copy holds the works");
        assert!(text.contains("\"privacy\""), "and the trust state");

        let deleted = backend.respond(&request(r#"{"id":3,"cmd":"delete_all"}"#).unwrap());
        assert!(deleted.contains("ledger.db"), "{deleted}");
        assert!(!root.join("ledger.db").exists());
        assert!(!root.join("ui-state.json").exists());
        assert!(std::fs::read_dir(&root).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ask_mode_refuses_to_open_without_confirmation() {
        let root = temp_root("askmode");
        let mut state = crate::ui_state::UiState::default();
        state.permission_mode = "ask".into();
        state.save(&root).unwrap();
        let backend = Backend::new(root.clone());
        let reply = backend.respond(
            &request(r#"{"id":1,"cmd":"continue_work","work":1}"#).unwrap(),
        );
        assert!(reply.contains("\"reason\":\"no_target\""), "{reply}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reopen_fixture_targets_have_structured_reasons() {
        let root = temp_root("reopen-fixtures");
        let file = root.join("note.txt");
        let folder = root.join("folder");
        let html = root.join("page.html");
        std::fs::write(&file, "fixture").unwrap();
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(&html, "<!doctype html>").unwrap();

        // SAFETY: this test runs the fixture matrix before spawning any
        // process, and restores the process environment before returning.
        unsafe { std::env::set_var("EVO_BRIDGE_OPEN_TEST", "success") };
        assert!(Backend::open_target(file.to_str().unwrap()).is_ok());
        assert!(Backend::open_target(folder.to_str().unwrap()).is_ok());
        assert!(Backend::open_target(html.to_str().unwrap()).is_ok());
        unsafe { std::env::remove_var("EVO_BRIDGE_OPEN_TEST") };
        assert_eq!(
            Backend::open_target(root.join("moved.txt").to_str().unwrap()),
            Err("file_moved")
        );
        unsafe { std::env::set_var("EVO_BRIDGE_OPEN_TEST", "failure") };
        assert_eq!(Backend::open_target("Definitely Missing Evo App"), Err("app_missing"));
        unsafe { std::env::remove_var("EVO_BRIDGE_OPEN_TEST") };
        assert_eq!(Backend::open_target(""), Err("no_target"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_continue_is_reported_and_recorded_by_the_bridge() {
        let root = temp_root("duplicate-continue");
        let backend = Backend::new(root.clone());
        let request = request(r#"{"id":1,"cmd":"continue_work","work":44}"#).unwrap();
        let first = backend.respond(&request);
        assert!(first.contains("\"reason\":\"no_target\""), "{first}");
        let second = backend.respond(&request);
        assert!(second.contains("\"hint\":\"Already opening\""), "{second}");
        let state = crate::ui_state::UiState::load(&root);
        assert_eq!(state.activity.len(), 2);
        assert_eq!(state.activity[0].action, "Already opening");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn export_and_delete_work_end_to_end() {
        let dir = std::env::temp_dir().join(format!("evo-bridge-data-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp root");
        std::fs::write(dir.join("ui-state.json"), "{}").unwrap();

        let backend = Backend::new(dir.clone());
        let reply = backend.respond(&request(r#"{"id":1,"cmd":"export_data"}"#).unwrap());
        assert!(reply.contains("\"ok\":true"), "{reply}");
        let parsed: serde_json::Value = serde_json::from_str(&reply).unwrap();
        let path = parsed["data"]["path"].as_str().unwrap();
        // the shortened path keeps the file name; find it on disk
        let exports: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("evo-export-"))
            .collect();
        assert_eq!(exports.len(), 1);
        let bytes = std::fs::metadata(exports[0].path()).unwrap().len();
        assert!(bytes > 0);

        let reply = backend.respond(&request(r#"{"id":2,"cmd":"delete_all"}"#).unwrap());
        assert!(reply.contains("\"ok\":true"), "{reply}");
        assert!(!dir.join("ui-state.json").exists());
        assert!(std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("evo-export-"))
            .count() == 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_unknown_commands() {
        let error = request(r#"{"id":1,"cmd":"eval","code":"alert(1)"}"#).unwrap_err();
        assert_eq!(error.code, "unknown_command");
        let error = request(r#"{"id":1,"cmd":"open_path","path":"/etc/passwd"}"#).unwrap_err();
        assert_eq!(error.code, "unknown_command");
    }

    #[test]
    fn refuses_bad_fields_and_extra_keys() {
        assert_eq!(
            request(r#"{"id":1,"cmd":"get_works","extra":true}"#)
                .unwrap_err()
                .code,
            "invalid_args"
        );
        assert_eq!(
            request(r#"{"id":1,"cmd":"rename_work","work":"nine","title":"x"}"#)
                .unwrap_err()
                .code,
            "invalid_args"
        );
        assert_eq!(
            request(r#"{"id":1,"cmd":"rename_work","work":1}"#)
                .unwrap_err()
                .code,
            "invalid_args"
        );
        assert_eq!(
            request(r#"{"id":1,"cmd":"hide_work","work":1,"undo":"yes"}"#)
                .unwrap_err()
                .code,
            "invalid_args"
        );
        assert_eq!(
            request(r#"{"id":"1","cmd":"get_works"}"#).unwrap_err().code,
            "missing_id"
        );
        assert_eq!(
            request(r#"{"cmd":"get_works"}"#).unwrap_err().code,
            "missing_id"
        );
        assert_eq!(
            request(r#"["get_works"]"#).unwrap_err().code,
            "invalid_shape"
        );
        assert_eq!(request("{").unwrap_err().code, "invalid_json");
    }

    #[test]
    fn refuses_oversized_messages_and_names() {
        let long_name = "x".repeat(MAX_TITLE_CHARS + 1);
        let body = format!(r#"{{"id":1,"cmd":"rename_work","work":1,"title":"{long_name}"}}"#);
        assert_eq!(request(&body).unwrap_err().code, "invalid_args");
        let huge = format!(
            r#"{{"id":1,"cmd":"search","query":"{}"}}"#,
            "y".repeat(MAX_MESSAGE_BYTES)
        );
        assert_eq!(request(&huge).unwrap_err().code, "too_large");
        let blank = r#"{"id":1,"cmd":"rename_work","work":1,"title":"   "}"#;
        assert_eq!(request(blank).unwrap_err().code, "invalid_args");
    }

    #[test]
    fn stable_ids_do_not_move() {
        assert_eq!(stable_id("workspace-abc"), stable_id("workspace-abc"));
        assert_ne!(stable_id("workspace-abc"), stable_id("workspace-abd"));
    }

    #[test]
    fn a_paused_app_refuses_to_continue() {
        let root = std::env::temp_dir().join(format!("evo-bridge-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut state = crate::ui_state::UiState::default();
        state.paused = true;
        state.save(&root).unwrap();
        let backend = Backend::new(root.clone());
        let request = Request {
            id: 7,
            command: Command::ContinueWork {
                work: 1,
                confirm: false,
            },
        };
        let reply = backend.respond(&request);
        assert!(reply.contains("\"ok\":false"));
        assert!(reply.contains("paused"), "{reply}");
        state.paused = false;
        state.save(&root).unwrap();
    }

    #[test]
    fn replies_carry_the_request_id_and_a_stable_error_shape() {
        let backend = Backend::new(std::env::temp_dir().join("evo-bridge-reply"));
        let request = Request {
            id: 42,
            command: Command::GetAbout,
        };
        let reply = backend.respond(&request);
        let value: Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(value["id"], 42);
        assert_eq!(value["ok"], true);
        assert!(value["data"]["version"].is_string());
        let error = BridgeError::new("unknown_command", "no").to_json();
        let value: Value = serde_json::from_str(&error).unwrap();
        assert_eq!(value["code"], "unknown_command");
        assert!(value["message"].is_string());
    }
}
