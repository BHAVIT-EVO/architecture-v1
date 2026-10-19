//! Stable window identity.
//!
//! Capture used to be window-blind: it saw only the focused subject, keyed URL
//! state by process id, and could erase a switch between two same-title
//! windows during focus deduplication. Downstream scoring cannot recover state
//! it never witnessed, so identity has to be fixed at the point of capture.
//!
//! This module is the portable half of that fix. The platform layer (macOS
//! Accessibility APIs, and later other platforms) periodically reports the
//! windows it sees; [`WindowRegistry`] turns those snapshots into stable
//! [`WindowId`]s that survive title changes, tab switches, and focus moves.
//! What the platform must provide per window is documented on
//! [`ObservedWindow`]; everything else here is pure logic and runs anywhere.
//!
//! Nothing here names an application. `platform_key` is an opaque string the
//! platform builds from whatever identifies a window for its lifetime
//! (on macOS: the app's bundle id plus the window's CGWindowID). This crate
//! never parses it.

use std::collections::{HashMap, HashSet};

/// A stable identity for one window across its lifetime.
///
/// A `WindowId` is content-independent: the title can change, the tab can
/// switch, the URL can navigate, and the id stays the same. It dies only when
/// the window closes. Two windows with identical titles are two ids — a focus
/// move between them is a real switch, never deduplicated away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

impl WindowId {
    /// The id used when no window is focused. Never assigned to a real window.
    pub const NONE: WindowId = WindowId(0);
}

impl std::fmt::Display for WindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "window-{}", self.0)
    }
}

/// The `Provenance::context` key carrying a [`WindowId`] for a witnessed
/// focused window. Follows the same pattern as the owning-process pid key:
/// provenance of the witnessing, never Evidence.
pub const WINDOW_ID_CONTEXT_KEY: &str = "window_id";

/// Why a [`WindowId`] string failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowIdParseError;

impl std::fmt::Display for WindowIdParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid window id (expected \"window-<n>\")")
    }
}

impl std::error::Error for WindowIdParseError {}

impl std::str::FromStr for WindowId {
    type Err = WindowIdParseError;

    /// Parses the [`std::fmt::Display`] form back into a [`WindowId`].
    ///
    /// Only the canonical form parses: `"window-"` followed by a base-10
    /// `u64`. Anything else — including a bare number — is rejected, so a
    /// forged or misplaced context value can never alias a real id.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let digits = s.strip_prefix("window-").ok_or(WindowIdParseError)?;
        let n: u64 = digits.parse().map_err(|_| WindowIdParseError)?;
        Ok(WindowId(n))
    }
}

/// One window as the platform saw it in a single snapshot.
///
/// The platform must report *every* window it can see, not just the focused
/// one — capture is window-aware only if the platform enumerates all windows.
/// `platform_key` must be stable for the window's lifetime and never reused
/// for a different window while the old one is still alive; beyond that it is
/// opaque to this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedWindow {
    /// Opaque platform-stable key (e.g. `bundle-id:CGWindowID` on macOS).
    pub platform_key: String,
    /// Human-readable application name, for display only — never for matching.
    pub app_name: String,
    /// The window's current title.
    pub title: String,
    /// The URL visible in this window right now, if the platform can see one.
    /// Keyed per window: two windows of the same app can show different URLs.
    pub url: Option<String>,
    /// Whether this window is focused in this snapshot.
    pub is_focused: bool,
}

impl ObservedWindow {
    /// Builds an observed window. Refuses empty platform keys: without a key
    /// there is no identity, and guessing identity is how windows get merged.
    pub fn new(
        platform_key: impl Into<String>,
        app_name: impl Into<String>,
        title: impl Into<String>,
        url: Option<String>,
        is_focused: bool,
    ) -> Option<Self> {
        let platform_key = platform_key.into();
        if platform_key.trim().is_empty() {
            return None;
        }
        Some(Self {
            platform_key,
            app_name: app_name.into(),
            title: title.into(),
            url,
            is_focused,
        })
    }
}

/// One window as the platform enumerated it, before identity assignment.
///
/// This is the portable shape of a platform snapshot: the platform layer
/// (macOS `CGWindowListCopyWindowInfo`, later other platforms) fills these
/// plain fields with no logic, and the portable half turns them into
/// [`ObservedWindow`]s. The snapshot must arrive in front-to-back order —
/// the platform's enumeration order — because [`select_focused`] reads focus
/// from it structurally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotWindow {
    /// Opaque platform-stable key, e.g. `bundle-id:CGWindowID` on macOS.
    /// Never empty: the platform must skip windows it cannot key.
    pub platform_key: String,
    /// Human-readable application name, for display only — never for matching.
    pub app_name: String,
    /// The window's title as the platform saw it. May be empty when the
    /// platform cannot read titles (macOS hides them without Screen Recording
    /// permission); titles are display-only, never matched on.
    pub title: String,
    /// The platform window layer. `0` is a normal window; menus, popups, the
    /// dock, and floating panels live on other layers.
    pub layer: i64,
    /// The process identifier of the owning application.
    pub owner_pid: i32,
}

/// Selects which snapshot window holds the focus for `focused_pid`.
///
/// Pure structural rule, no titles, no application knowledge: the focused
/// window is the frontmost *normal* (layer 0) window of the focused process,
/// i.e. the first layer-0 window of `focused_pid` in the snapshot's
/// front-to-back order. Returns the snapshot index, or `None` when the
/// focused process has no normal window in this snapshot (e.g. focus sits on
/// a floating panel) — the caller then records no window rather than guessing.
///
/// The caller marks the returned index `is_focused` when building
/// [`ObservedWindow`]s; everything downstream keys off the resulting
/// [`WindowId`], never off this index.
pub fn select_focused(windows: &[SnapshotWindow], focused_pid: i32) -> Option<usize> {
    windows
        .iter()
        .position(|w| w.owner_pid == focused_pid && w.layer == 0)
}

/// What one window looked like across snapshots.
#[derive(Debug, Clone)]
pub struct WindowRecord {
    /// Stable id for this window's lifetime.
    pub id: WindowId,
    /// The platform key this record was created from. Opaque.
    pub platform_key: String,
    /// Display name of the owning application. Never used for matching.
    pub app_name: String,
    /// Latest title seen.
    pub title: String,
    /// Latest URL seen in this window, if any. Per-window: this is the fix
    /// for URL state that used to be keyed by process id.
    pub url: Option<String>,
    /// When the window was first seen (platform timestamp millis).
    pub first_seen: u64,
    /// When the window was last seen (platform timestamp millis).
    pub last_seen: u64,
    /// Whether the window has closed. Closed records are kept as tombstones
    /// so a later window reusing a platform key cannot inherit the old id.
    pub closed: bool,
}

/// What changed between two consecutive snapshots.
#[derive(Debug, Clone, Default)]
pub struct SnapshotDiff {
    /// Ids of windows seen for the first time.
    pub opened: Vec<WindowId>,
    /// Ids of windows that were alive and are now gone.
    pub closed: Vec<WindowId>,
    /// `(id, old_title, new_title)` for windows whose title changed.
    pub title_changed: Vec<(WindowId, String, String)>,
    /// `(id, old_url, new_url)` for windows whose visible URL changed.
    pub url_changed: Vec<(WindowId, Option<String>, Option<String>)>,
    /// `(was_focused, now_focused)` when focus moved. A move between two
    /// same-title windows is reported here — it is never deduplicated away.
    pub focus_changed: Option<(Option<WindowId>, WindowId)>,
}

/// Maintains stable window identity across platform snapshots.
///
/// The registry matches snapshot windows to live records by `platform_key`
/// only. It never matches by title, URL, or app name: titles change when tabs
/// switch, and two windows can share everything visible while being different
/// windows. Matching by anything weaker than the platform key is how
/// same-title windows used to collapse into one.
#[derive(Debug, Default)]
pub struct WindowRegistry {
    by_platform_key: HashMap<String, WindowId>,
    records: HashMap<WindowId, WindowRecord>,
    next_id: u64,
    focused: Option<WindowId>,
}

impl WindowRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Folds one platform snapshot into the registry and reports what changed.
    ///
    /// Snapshots must arrive in timestamp order. Windows absent from a
    /// snapshot are treated as closed; a platform key seen again after its
    /// window closed is treated as a *new* window, never a resurrection —
    /// the old record stays a tombstone so ids are never recycled.
    pub fn observe_snapshot(&mut self, at: u64, windows: Vec<ObservedWindow>) -> SnapshotDiff {
        let mut diff = SnapshotDiff::default();
        let mut seen_keys: HashSet<&str> = HashSet::with_capacity(windows.len());
        let mut focused_in_snapshot: Option<WindowId> = None;

        for window in &windows {
            seen_keys.insert(window.platform_key.as_str());
            let id = match self.by_platform_key.get(&window.platform_key) {
                Some(id) => *id,
                None => {
                    self.next_id += 1;
                    let id = WindowId(self.next_id);
                    self.by_platform_key.insert(window.platform_key.clone(), id);
                    self.records.insert(
                        id,
                        WindowRecord {
                            id,
                            platform_key: window.platform_key.clone(),
                            app_name: window.app_name.clone(),
                            title: window.title.clone(),
                            url: window.url.clone(),
                            first_seen: at,
                            last_seen: at,
                            closed: false,
                        },
                    );
                    diff.opened.push(id);
                    id
                }
            };

            if let Some(record) = self.records.get_mut(&id) {
                // A tombstoned record can never come back: if the platform key
                // was already mapped, the id above is the live one. This
                // branch only updates live records.
                if !record.closed {
                    if record.title != window.title {
                        diff.title_changed.push((
                            id,
                            std::mem::replace(&mut record.title, window.title.clone()),
                            window.title.clone(),
                        ));
                    }
                    if record.url != window.url {
                        diff.url_changed.push((
                            id,
                            std::mem::replace(&mut record.url, window.url.clone()),
                            window.url.clone(),
                        ));
                    }
                    record.app_name = window.app_name.clone();
                    record.last_seen = at;
                }
            }

            if window.is_focused {
                // If the platform reports several focused windows, the last
                // one wins; platforms report at most one.
                focused_in_snapshot = Some(id);
            }
        }

        // Windows that were alive and are absent from this snapshot closed.
        let mut closed_now: Vec<WindowId> = self
            .records
            .values()
            .filter(|record| !record.closed && !seen_keys.contains(record.platform_key.as_str()))
            .map(|record| record.id)
            .collect();
        closed_now.sort();
        for id in closed_now {
            if let Some(record) = self.records.get_mut(&id) {
                record.closed = true;
                record.last_seen = at;
            }
            diff.closed.push(id);
            // Unmap the platform key so a later window reusing it is treated
            // as a NEW window with a fresh id. The old record stays in
            // `records` as a tombstone — history can still name it, but no
            // new window ever inherits its identity.
            let key = self
                .records
                .get(&id)
                .map(|r| r.platform_key.clone())
                .unwrap_or_default();
            self.by_platform_key.remove(&key);
        }

        if focused_in_snapshot != self.focused {
            if let Some(now) = focused_in_snapshot {
                diff.focus_changed = Some((self.focused, now));
            }
            self.focused = focused_in_snapshot;
        }

        diff
    }

    /// The live record for `id`, if the window is still open.
    pub fn live(&self, id: WindowId) -> Option<&WindowRecord> {
        self.records.get(&id).filter(|record| !record.closed)
    }

    /// The record for `id`, whether open or closed.
    pub fn record(&self, id: WindowId) -> Option<&WindowRecord> {
        self.records.get(&id)
    }

    /// Ids of all currently open windows.
    pub fn open_windows(&self) -> Vec<WindowId> {
        let mut ids: Vec<WindowId> = self
            .records
            .values()
            .filter(|record| !record.closed)
            .map(|record| record.id)
            .collect();
        ids.sort();
        ids
    }

    /// The currently focused window, if any.
    pub fn focused(&self) -> Option<WindowId> {
        self.focused.and_then(|id| self.live(id).map(|_| id))
    }

    /// The URL currently visible in `id`'s window, per-window.
    ///
    /// This is the replacement for process-keyed URL state: callers ask what
    /// *this window* shows, not what the process last showed.
    pub fn window_url(&self, id: WindowId) -> Option<&str> {
        self.live(id).and_then(|record| record.url.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(key: &str, app: &str, title: &str, url: Option<&str>, focused: bool) -> ObservedWindow {
        ObservedWindow::new(key, app, title, url.map(str::to_string), focused).unwrap()
    }

    #[test]
    fn first_snapshot_opens_every_window() {
        let mut registry = WindowRegistry::new();
        let diff = registry.observe_snapshot(
            1000,
            vec![
                win(
                    "app1:1",
                    "Browser",
                    "Docs",
                    Some("https://docs.example.com/a"),
                    true,
                ),
                win(
                    "app1:2",
                    "Browser",
                    "Mail",
                    Some("https://mail.example.com"),
                    false,
                ),
            ],
        );
        assert_eq!(diff.opened.len(), 2);
        assert!(diff.closed.is_empty());
        assert_eq!(registry.open_windows().len(), 2);
        assert!(registry.focused().is_some());
    }

    #[test]
    fn same_title_windows_keep_distinct_ids_and_focus_moves_between_them() {
        // The old capture collapsed these: same app, same title, and focus
        // dedup erased the switch. The registry must not.
        let mut registry = WindowRegistry::new();
        registry.observe_snapshot(
            1000,
            vec![
                win("app1:1", "Browser", "Inbox", None, true),
                win("app1:2", "Browser", "Inbox", None, false),
            ],
        );
        let first_focused = registry.focused().unwrap();

        let diff = registry.observe_snapshot(
            2000,
            vec![
                win("app1:1", "Browser", "Inbox", None, false),
                win("app1:2", "Browser", "Inbox", None, true),
            ],
        );

        let second_focused = registry.focused().unwrap();
        assert_ne!(
            first_focused, second_focused,
            "two same-title windows are two ids"
        );
        assert_eq!(
            diff.focus_changed,
            Some((Some(first_focused), second_focused)),
            "the switch between same-title windows is witnessed, not deduplicated"
        );
        assert!(diff.opened.is_empty());
        assert!(diff.closed.is_empty());
    }

    #[test]
    fn title_change_keeps_identity() {
        let mut registry = WindowRegistry::new();
        registry.observe_snapshot(1000, vec![win("app1:1", "Browser", "Tab A", None, true)]);
        let id = registry.focused().unwrap();

        let diff =
            registry.observe_snapshot(2000, vec![win("app1:1", "Browser", "Tab B", None, true)]);

        assert_eq!(
            registry.focused(),
            Some(id),
            "a tab switch is not a new window"
        );
        assert_eq!(diff.title_changed.len(), 1);
        assert_eq!(diff.title_changed[0].0, id);
        assert!(diff.opened.is_empty());
    }

    #[test]
    fn urls_are_per_window_not_per_process() {
        // The old capture keyed URL transitions by PID: two windows of one
        // app clobbered each other's URL. Here each window keeps its own.
        let mut registry = WindowRegistry::new();
        registry.observe_snapshot(
            1000,
            vec![
                win(
                    "app1:1",
                    "Browser",
                    "Docs",
                    Some("https://docs.example.com/a"),
                    true,
                ),
                win(
                    "app1:2",
                    "Browser",
                    "Mail",
                    Some("https://mail.example.com"),
                    false,
                ),
            ],
        );
        let docs = registry.open_windows()[0];
        let mail = registry.open_windows()[1];

        registry.observe_snapshot(
            2000,
            vec![
                win(
                    "app1:1",
                    "Browser",
                    "Docs",
                    Some("https://docs.example.com/b"),
                    true,
                ),
                win(
                    "app1:2",
                    "Browser",
                    "Mail",
                    Some("https://mail.example.com"),
                    false,
                ),
            ],
        );

        assert_eq!(
            registry.window_url(docs),
            Some("https://docs.example.com/b")
        );
        assert_eq!(registry.window_url(mail), Some("https://mail.example.com"));
    }

    #[test]
    fn absent_windows_close_and_ids_are_never_recycled() {
        let mut registry = WindowRegistry::new();
        registry.observe_snapshot(1000, vec![win("app1:1", "Browser", "Docs", None, true)]);
        let id = registry.focused().unwrap();

        let diff = registry.observe_snapshot(2000, vec![]);
        assert_eq!(diff.closed, vec![id]);
        assert!(registry.live(id).is_none());
        // The tombstone remains: history can still name the window.
        assert!(registry.record(id).is_some());

        // A later window reusing the platform key is a NEW window.
        let diff =
            registry.observe_snapshot(3000, vec![win("app1:1", "Browser", "Docs", None, true)]);
        let new_id = registry.focused().unwrap();
        assert_ne!(new_id, id, "ids are never recycled");
        assert_eq!(diff.opened, vec![new_id]);
    }

    #[test]
    fn empty_platform_key_is_refused() {
        assert!(ObservedWindow::new("", "Browser", "T", None, false).is_none());
        assert!(ObservedWindow::new("   ", "Browser", "T", None, false).is_none());
    }

    #[test]
    fn focus_can_clear() {
        let mut registry = WindowRegistry::new();
        registry.observe_snapshot(1000, vec![win("app1:1", "Browser", "Docs", None, true)]);
        assert!(registry.focused().is_some());
        registry.observe_snapshot(2000, vec![win("app1:1", "Browser", "Docs", None, false)]);
        assert!(registry.focused().is_none());
    }
}
