//! evo-desktop — the Evo desktop shell.
//!
//! A macOS-native presentation and user-interaction layer for the canonical
//! Evo runtime.
//!
//! This crate owns NO canonical semantics:
//!
//! - Artifact identity stays in `evo-artifact`;
//! - Workspace identity and Snapshot semantics stay in `evo-workspace`;
//! - persistence semantics stay in `evo-storage` and the daemon's persistence
//!   boundary (`evo_daemon::persistence`);
//! - restoration derivation stays in `evo-restoration`;
//! - capture semantics stay in `evo-capture` / `evo-daemon`.
//!
//! The desktop shell consumes canonical Workspace/Snapshot understanding from
//! the daemon's persistence boundary and presents it in a native macOS
//! window. It never duplicates business logic and never fabricates state.

//! # Module layout
//!
//! `ui` and `theme` are the visual foundation: reusable layout, control and
//! surface primitives, and the single place any measurement, colour, type role
//! or motion constant is defined. `rack`, `dock`, `detail` and `shell` are the
//! surfaces composed from them (`home` keeps the empty and permission
//! states). `app` holds state and dispatch only, and `state` is the read-only
//! projection of canonical understanding they all consume.

pub mod app;
pub mod daemon;
pub mod voice;
pub mod pod_bar_drag;
pub mod workspace_layout;
pub mod unified_workspace;

/// A minimal honest `WorkUnderstanding` for interaction tests: only the
/// fields the rack renders from, with no invented evidence.
#[cfg(test)]
pub fn understanding_fixture(name: &str) -> evo_daemon::work_understanding::WorkUnderstanding {
    evo_daemon::work_understanding::WorkUnderstanding {
        title: name.to_string(),
        phase: evo_daemon::work_understanding::WorkPhase::Building,
        trail: vec![],
        completed: vec![],
        pending: vec![],
        next_step: String::new(),
        narrative: String::new(),
    }
}
pub mod detail;
pub mod dock;
pub mod engine;
pub mod home;
pub mod notch;
pub mod notch_host_verify;
pub mod notch_window;
pub mod pod_surface;
pub mod pods;
pub mod presence;
pub mod rack;
pub mod scoped_restore;
pub mod shell;
pub mod state;
pub mod theme;
pub mod ui;
