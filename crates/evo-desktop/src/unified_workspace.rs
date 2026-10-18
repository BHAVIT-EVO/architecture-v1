//! Native window grouping for one restored Evo work.
//!
//! A work is not an embedded browser or a Pod surface. It is a short-lived
//! arrangement session: remember the windows that existed before opening,
//! open the work normally through macOS, then place the newly appeared
//! windows into Setpiece-style normalized slots on the active display.

use std::collections::BTreeSet;

use evo_pods::layout::{Layout, LayoutEngine, LayoutId, Plan, WindowRef};
use evo_pods::surface::{PodSurface, PodWindow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    windows: BTreeSet<(i32, u32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceArrangement {
    pub layout: LayoutId,
    pub plan: Plan,
}

/// Capture the visible regular-app windows before Evo opens a work.
pub fn snapshot(surface: &mut impl PodSurface, protected_pid: i32) -> WorkspaceSnapshot {
    let mut windows = BTreeSet::new();
    if let Ok(apps) = surface.running_apps() {
        for app in apps
            .into_iter()
            .filter(|app| app.regular && app.pid != protected_pid)
        {
            if let Ok(app_windows) = surface.windows_of(app.pid) {
                windows.extend(
                    app_windows
                        .into_iter()
                        .filter(|window| !window.minimized)
                        .map(|window| (window.pid, window.window_id)),
                );
            }
        }
    }
    WorkspaceSnapshot { windows }
}

/// Arrange windows that appeared after `before` into a deterministic layout.
/// The call is intentionally non-destructive: overflow is reported by the
/// planner but is left alone instead of being minimized behind the user's back.
pub fn arrange_new_windows(
    surface: &mut impl PodSurface,
    before: &WorkspaceSnapshot,
    protected_pid: i32,
) -> Option<WorkspaceArrangement> {
    arrange_new_windows_with_layout(surface, before, protected_pid, None)
}

/// Same arrangement operation with an optional user-selected preset. `None`
/// keeps the automatic count-based choice.
pub fn arrange_new_windows_with_layout(
    surface: &mut impl PodSurface,
    before: &WorkspaceSnapshot,
    protected_pid: i32,
    preferred_layout: Option<LayoutId>,
) -> Option<WorkspaceArrangement> {
    let apps = match surface.running_apps() {
        Ok(apps) => apps,
        Err(err) => {
            eprintln!("EVO workspace: could not enumerate running apps: {err}");
            return None;
        }
    };
    let mut current = Vec::new();
    for app in apps
        .into_iter()
        .filter(|app| app.regular && app.pid != protected_pid)
    {
        current.extend(
            match surface.windows_of(app.pid) {
                Ok(windows) => windows,
                Err(err) => {
                    eprintln!("EVO workspace: could not enumerate windows for pid {}: {err}", app.pid);
                    continue;
                }
            }
                .into_iter()
                .filter(|window| {
                    !window.minimized && !before.windows.contains(&(window.pid, window.window_id))
                }),
        );
    }
    if current.is_empty() {
        return None;
    }

    let screen = match surface.screens().into_iter().next() {
        Some(screen) => screen,
        None => {
            eprintln!("EVO workspace: no usable display reported");
            return None;
        }
    };
    if !screen.usable() {
        return None;
    }
    let layout = preferred_layout
        .map(Layout::seed)
        .unwrap_or_else(|| layout_for_count(current.len()));
    let slots = layout.resolved_slots(&screen);
    let refs: Vec<WindowRef> = current.iter().map(window_ref).collect();
    let plan = LayoutEngine::plan(&refs, &slots, &[]);
    for placement in &plan.placements {
        if let Err(err) = surface.move_window(placement.pid, placement.window_id, placement.frame) {
            eprintln!(
                "EVO workspace: failed to move pid {} window {}: {err}",
                placement.pid, placement.window_id
            );
        }
    }
    if let Some(pid) = plan.placements.last().map(|placement| placement.pid) {
        let _ = surface.activate_app(pid);
    }
    Some(WorkspaceArrangement {
        layout: layout.id,
        plan,
    })
}

fn window_ref(window: &PodWindow) -> WindowRef {
    WindowRef {
        pid: window.pid,
        window_id: window.window_id,
        frame: window.frame,
    }
}

/// Setpiece's presets, chosen from the number of windows in this work.
/// HeroRails gives five to eight windows a composed arrangement instead of a
/// random cascade; larger sets keep the first eight tiled and leave overflow
/// visible for the person to place manually.
pub fn layout_for_count(count: usize) -> Layout {
    let id = match count {
        0 | 1 => LayoutId::Full,
        2 => LayoutId::Halves,
        3 => LayoutId::Thirds,
        4 => LayoutId::Quads,
        5..=8 => LayoutId::HeroRails,
        _ => LayoutId::HeroRails,
    };
    Layout::seed(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_pods::surface::Frame;

    #[test]
    fn maps_window_counts_to_setpiece_layout_presets() {
        assert_eq!(layout_for_count(1).id, LayoutId::Full);
        assert_eq!(layout_for_count(2).id, LayoutId::Halves);
        assert_eq!(layout_for_count(3).id, LayoutId::Thirds);
        assert_eq!(layout_for_count(4).id, LayoutId::Quads);
        assert_eq!(layout_for_count(6).id, LayoutId::HeroRails);
    }

    #[test]
    fn every_seed_stays_inside_a_screen() {
        let screen = Frame::new(0.0, 0.0, 1440.0, 900.0);
        for count in 1..=8 {
            for frame in layout_for_count(count).resolved_slots(&screen) {
                assert!(frame.usable());
                assert!(frame.x >= screen.x);
                assert!(frame.y >= screen.y);
                assert!(frame.x + frame.w <= screen.x + screen.w + 1e-6);
                assert!(frame.y + frame.h <= screen.y + screen.h + 1e-6);
            }
        }
    }
}
