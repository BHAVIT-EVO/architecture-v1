//! Evo Pods (IS-0023): a body of work as a first-class desktop object.
//!
//! Rooms (IS-0022, superseded) made a work a place you enter. Pods make a
//! work a *thing you switch between*: a card with a name, a color, a mini
//! stage, an honest one-line reason of where it resumes — and physical
//! gestures (drag a window onto a pod, merge two pods) that emit the
//! engine's declarations, because the physical correction IS the merge
//! path.
//!
//! # The laws (mirroring IS-0023)
//!
//! 1. **No hardcoded applications.** Surfaces come from the work's own
//!    witnessed evidence (documents, pages, titles, app names); launch
//!    behavior comes from an external [`policy::InstancePolicy`] file the
//!    user ships or imports. Code knows shapes, never products.
//! 2. **Silence over invention.** Badges render only from engine
//!    [`Delta`]s; suggestions only above their evidence floor; the bar
//!    never shows what the engine didn't say.
//! 3. **Under-containment.** Dim/park touches only windows provably not
//!    part of the active pod. A doubtful window stays fully visible.
//! 4. **Reversibility.** Every activation produces a [`lease::PodLease`];
//!    deactivation replays it in exact reverse order.
//! 5. **The pod bar is a view of the engine.** Pods re-derive from
//!    [`evo_threads`] views; the runtime itself keeps no grouping truth.

pub mod config;
pub mod dim;
pub mod gesture;
pub mod lease;
pub mod macos;
pub mod pod;
pub mod policy;
pub mod runtime;
pub mod stage;
pub mod suggest;
pub mod surface;
pub mod wrapper;

pub use config::ContainMode;
pub use pod::{Pod, PodBadge, PodClaim, PodColor, PodId, PodResource, PodState, PodSurfaces};
pub use surface::{CommandSpec, Frame, PodApp, PodSurface, PodWindow};
