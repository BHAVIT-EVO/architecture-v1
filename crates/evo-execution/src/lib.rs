//! Evo Execution Layer.
//!
//! This crate implements the Architecture's Restoration Execution layer
//! (ARCHITECTURE.md §4 pipeline stage 8, §5 Restoration Engine; IS-0019 §11;
//! IS-0021 §20): a separate architectural layer that consumes a canonical
//! derived Restoration Plan / Resume Point and performs the restoration,
//! reporting per-item success or failure explicitly.
//!
//! # Contractual position
//!
//! - Execution consumes Restoration Plans (IS-0019 §11). It does NOT derive
//!   restoration understanding, and it does NOT modify canonical history,
//!   Workspace state, or Artifact identity.
//! - Execution MAY inspect current operating-system state while executing a
//!   Restoration Plan, but that state SHALL NOT become an input to Restoration
//!   Derivation (IS-0021 §Section 7: "A future execution layer MAY inspect
//!   current operating-system state when executing a RestorationPlan, but
//!   that state SHALL NOT become an input to Restoration Derivation.").
//! - Execution never guesses an executable target. Every target originates
//!   from canonical evidence (the witnessed subject of a canonical
//!   Observation, classified deterministically by its frozen schema), and the
//!   platform executor refuses — never substitutes — when the target cannot be
//!   resolved safely.
//!
//! # Epistemic boundary
//!
//! - Observation = witnessed fact (canonical Observation log).
//! - Derivation = deterministic interpretation (evo-restoration).
//! - Execution = action taken because derivation justified it (this crate).
//!
//! This crate never fabricates actions and never reports success for a
//! restoration that did not occur. The canonical `execute` boundary in
//! `evo-restoration` remains the honest refusal gate for the frozen model;
//! this crate is the downstream layer that performs when a plan exists.
//!
//! # The binding boundary
//!
//! [`binding`] is the one place where a canonical Artifact meets a live native
//! resource. It is explicitly **non-canonical and rebuildable**: it holds no
//! identity, is never persisted, is reconstructed by live probing on every
//! execution, and never becomes the source of truth for what the work is. The
//! whole layering is:
//!
//! ```text
//! WorkHypothesis → Workspace → RestorationPlan → binding → native resource
//!                  └─────── canonical ───────┘   └─ rebuildable ─┘
//! ```
//!
//! Nothing to the right of the arrow ever flows back to the left.

pub mod binding;
pub mod engine;
pub mod locator;
// The macOS executor module self-gates its platform-specific implementation
// (`#[cfg(not(target_os = "macos"))]` fallbacks report honest unavailability),
// so it is compiled on every platform and the window abstractions it defines
// are available to the platform-neutral preflight module.
pub mod macos;
pub mod preflight;
pub mod resource;
pub mod restore;
pub mod room;
pub mod room_macos;
pub mod selection;

pub use binding::{
    BindingCapability, BindingFailure, BoundResource, LiveResourceBinding, WitnessedOpen,
    bind_resource,
};
pub use engine::{
    ExecutionReport, PlatformExecutor, RestorationStep, StepPlan, TargetAttempt, TargetStatus,
    execute, execute_selection, execute_step, ordered_attempt_targets, ordered_targets,
    plan_request, plan_selection,
};
pub use locator::{Locator, LocatorKind, classify_locator};
pub use macos::MacOSExecutor;
pub use macos::{WindowInfo, WindowSelectionError, WindowSource, select_unique_window};
pub use preflight::{
    PreflightOutcome, PreflightSource, PreflightStatus, preflight_identity, preflight_selection,
};
pub use resource::{ResourceIdentity, classify_resource_identity};
pub use restore::{RestoreAction, RestoreOutcome, RestoreTarget, launch_app, plan, restore};
pub use selection::{
    Disposition, RestorationSelection, SelectedResource, UnavailableReason, WithheldMember,
    select_restoration,
};
