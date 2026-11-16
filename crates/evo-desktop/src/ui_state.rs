//! The versioned local settings store (SPEC.md: "Backend changes").
//!
//! One JSON file beside the ledger holds the small, person-owned facts the
//! shell must remember across restarts: names given to works, pairings the
//! person confirmed, works they hid, the onboarding step, the appearance
//! choice, and whether the kill switch is on. Nothing here is evidence about
//! the work; it is the person's own word, so it lives in its own file.
//!
//! Unknown fields are ignored on read and a missing or corrupt file yields
//! defaults rather than an error: a settings file may never stop the app from
//! opening. Writes are atomic (temp file + rename) and carry a schema version.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 1;
const FILE_NAME: &str = "ui-state.json";

/// One line of the trust surface's history: what Evo did, why, and whether it
/// can still be undone. Newest first, bounded like `security::activity`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityRecord {
    /// Seconds since the Unix epoch.
    pub at: u64,
    pub action: String,
    pub reason: String,
    /// `(kind, id)` for the same three kinds the native Activity log knows.
    pub undo: Option<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiState {
    pub version: u32,
    pub renames: BTreeMap<u64, String>,
    pub merges: Vec<(u64, u64)>,
    pub hidden: Vec<u64>,
    pub onboarding_step: u8,
    pub appearance: String,
    pub paused: bool,
    pub keep_background: bool,
    pub quiet_desktop: bool,
    pub permission_mode: String,
    /// `"kind:value" -> allowed`, only for the scopes the person changed.
    pub scopes: BTreeMap<String, bool>,
    pub activity: Vec<ActivityRecord>,
}

/// The most recent activity rows kept.
pub const ACTIVITY_CAPACITY: usize = 100;

impl Default for UiState {
    fn default() -> Self {
        Self {
            version: VERSION,
            renames: BTreeMap::new(),
            merges: Vec::new(),
            hidden: Vec::new(),
            onboarding_step: 0,
            appearance: "system".into(),
            paused: false,
            keep_background: false,
            quiet_desktop: false,
            permission_mode: "risky".into(),
            scopes: BTreeMap::new(),
            activity: Vec::new(),
        }
    }
}

pub fn path_in(root: &Path) -> PathBuf {
    root.join(FILE_NAME)
}

impl UiState {
    /// Reads the store; any failure is "nothing saved yet".
    pub fn load(root: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path_in(root)) else {
            return Self::default();
        };
        Self::from_json(&text).unwrap_or_default()
    }

    /// The strict half: versions this build does not know are refused, so a
    /// newer file is never silently rewritten into an older shape.
    pub fn from_json(text: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(text).ok()?;
        let object = value.as_object()?;
        let version = object.get("version")?.as_u64()? as u32;
        if version != VERSION {
            return None;
        }
        let mut state = Self::default();
        if let Some(renames) = object.get("renames").and_then(|v| v.as_object()) {
            for (key, value) in renames {
                if let (Ok(id), Some(title)) = (key.parse::<u64>(), value.as_str()) {
                    state.renames.insert(id, title.to_string());
                }
            }
        }
        if let Some(merges) = object.get("merges").and_then(|v| v.as_array()) {
            for pair in merges {
                if let Some(items) = pair.as_array()
                    && items.len() == 2
                    && let (Some(a), Some(b)) = (items[0].as_u64(), items[1].as_u64())
                {
                    state.merges.push((a, b));
                }
            }
        }
        if let Some(hidden) = object.get("hidden").and_then(|v| v.as_array()) {
            state.hidden = hidden.iter().filter_map(|v| v.as_u64()).collect();
        }
        if let Some(step) = object.get("onboarding_step").and_then(|v| v.as_u64()) {
            state.onboarding_step = step.min(u8::MAX as u64) as u8;
        }
        if let Some(appearance) = object.get("appearance").and_then(|v| v.as_str()) {
            state.appearance = appearance.to_string();
        }
        if let Some(paused) = object.get("paused").and_then(|v| v.as_bool()) {
            state.paused = paused;
        }
        if let Some(keep_background) = object.get("keep_background").and_then(|v| v.as_bool()) {
            state.keep_background = keep_background;
        }
        if let Some(quiet_desktop) = object.get("quiet_desktop").and_then(|v| v.as_bool()) {
            state.quiet_desktop = quiet_desktop;
        }
        if let Some(mode) = object.get("permission_mode").and_then(|v| v.as_str())
            && !mode.is_empty()
            && mode.len() <= 16
        {
            state.permission_mode = mode.to_string();
        }
        if let Some(scopes) = object.get("scopes").and_then(|v| v.as_object()) {
            for (key, value) in scopes {
                if key.len() <= 96
                    && let Some(allowed) = value.as_bool()
                {
                    state.scopes.insert(key.clone(), allowed);
                }
            }
        }
        if let Some(activity) = object.get("activity").and_then(|v| v.as_array()) {
            for row in activity.iter().take(ACTIVITY_CAPACITY) {
                let Some(row) = row.as_object() else { continue };
                let at = row.get("at").and_then(|v| v.as_u64()).unwrap_or(0);
                let action = row.get("action").and_then(|v| v.as_str()).unwrap_or("");
                let reason = row.get("reason").and_then(|v| v.as_str()).unwrap_or("");
                if action.is_empty() {
                    continue;
                }
                let undo = row
                    .get("undo")
                    .and_then(|v| v.as_array())
                    .filter(|pair| pair.len() == 2)
                    .and_then(|pair| {
                        Some((pair[0].as_str()?.to_string(), pair[1].as_str()?.to_string()))
                    });
                state.activity.push(ActivityRecord {
                    at,
                    action: action.to_string(),
                    reason: reason.to_string(),
                    undo,
                });
            }
        }
        Some(state)
    }

    pub fn to_json(&self) -> String {
        let renames: Vec<String> = self
            .renames
            .iter()
            .map(|(id, title)| format!("\"{id}\":{}", json_string(title)))
            .collect();
        let merges: Vec<String> = self
            .merges
            .iter()
            .map(|(a, b)| format!("[{a},{b}]"))
            .collect();
        let hidden: Vec<String> = self.hidden.iter().map(|id| id.to_string()).collect();
        let scopes: Vec<String> = self
            .scopes
            .iter()
            .map(|(key, allowed)| format!("{}:{}", json_string(key), allowed))
            .collect();
        let activity: Vec<String> = self
            .activity
            .iter()
            .map(|row| {
                let undo = match &row.undo {
                    Some((kind, id)) => format!("[{},{}]", json_string(kind), json_string(id)),
                    None => "null".into(),
                };
                format!(
                    "{{\"at\":{},\"action\":{},\"reason\":{},\"undo\":{}}}",
                    row.at,
                    json_string(&row.action),
                    json_string(&row.reason),
                    undo
                )
            })
            .collect();
        format!(
            "{{\"version\":{},\"renames\":{{{}}},\"merges\":[{}],\"hidden\":[{}],\"onboarding_step\":{},\"appearance\":{},\"paused\":{},\"keep_background\":{},\"quiet_desktop\":{},\"permission_mode\":{},\"scopes\":{{{}}},\"activity\":[{}]}}",
            VERSION,
            renames.join(","),
            merges.join(","),
            hidden.join(","),
            self.onboarding_step,
            json_string(&self.appearance),
            self.paused,
            self.keep_background,
            self.quiet_desktop,
            json_string(&self.permission_mode),
            scopes.join(","),
            activity.join(",")
        )
    }

    pub fn save(&self, root: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(root)?;
        let target = path_in(root);
        let temp = target.with_extension("json.tmp");
        std::fs::write(&temp, self.to_json())?;
        std::fs::rename(&temp, &target)
    }

    /// Hiding is additive and idempotent; `undo` pops exactly that id once.
    pub fn hide(&mut self, id: u64) -> bool {
        if self.hidden.contains(&id) {
            return false;
        }
        self.hidden.push(id);
        true
    }

    pub fn unhide(&mut self, id: u64) -> bool {
        let before = self.hidden.len();
        self.hidden.retain(|hidden| *hidden != id);
        self.hidden.len() != before
    }

    pub fn is_hidden(&self, id: u64) -> bool {
        self.hidden.contains(&id)
    }

    /// Records one action, newest first, bounded.
    pub fn record(&mut self, at: u64, action: &str, reason: &str, undo: Option<(String, String)>) {
        self.activity.insert(
            0,
            ActivityRecord {
                at,
                action: action.chars().take(160).collect(),
                reason: reason.chars().take(160).collect(),
                undo,
            },
        );
        self.activity.truncate(ACTIVITY_CAPACITY);
    }

    /// Takes the row at `at` and hands back what to undo, once.
    pub fn take_undo(&mut self, at: u64) -> Option<ActivityRecord> {
        let index = self.activity.iter().position(|row| row.at == at)?;
        Some(self.activity.remove(index))
    }

    pub fn display_name(&self, id: u64, original: &str) -> String {
        self.renames
            .get(&id)
            .cloned()
            .unwrap_or_else(|| original.to_string())
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "evo-ui-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn names_hidden_works_and_pause_survive_a_round_trip() {
        let root = root();
        let mut state = UiState::default();
        state.renames.insert(4, "The pitch deck".into());
        state.merges.push((4, 9));
        state.hide(7);
        state.onboarding_step = 3;
        state.appearance = "dark".into();
        state.paused = true;
        state.save(&root).unwrap();

        let back = UiState::load(&root);
        assert_eq!(
            back.renames.get(&4).map(String::as_str),
            Some("The pitch deck")
        );
        assert_eq!(back.merges, vec![(4, 9)]);
        assert!(back.is_hidden(7));
        assert_eq!(back.onboarding_step, 3);
        assert_eq!(back.appearance, "dark");
        assert!(back.paused);
    }

    #[test]
    fn hiding_is_idempotent_and_undo_is_exact() {
        let mut state = UiState::default();
        assert!(state.hide(3));
        assert!(!state.hide(3));
        assert!(state.unhide(3));
        assert!(!state.unhide(3));
    }

    #[test]
    fn a_missing_file_is_defaults_not_an_error() {
        let state = UiState::load(Path::new("/nonexistent/evo/for-tests"));
        assert_eq!(state, UiState::default());
    }

    #[test]
    fn a_corrupt_or_future_file_is_refused() {
        assert!(UiState::from_json("{").is_none());
        assert!(UiState::from_json("{\"version\":99}").is_none());
        assert!(UiState::from_json("{\"version\":1,\"future\":true}").is_some());
    }
}
