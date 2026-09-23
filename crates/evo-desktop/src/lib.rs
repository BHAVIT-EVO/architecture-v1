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
//! or motion constant is defined. `home`, `detail` and `shell` are the three
//! screens composed from them. `app` holds state and dispatch only, and `state`
//! is the read-only projection of canonical understanding they all consume.

pub mod app;
pub mod daemon;
pub mod detail;
pub mod engine;
pub mod home;
pub mod presence;
pub mod pods;
pub mod pod_surface;
pub mod podbar;
pub mod shell;
pub mod state;
pub mod theme;
pub mod ui;
pub mod websurface;
