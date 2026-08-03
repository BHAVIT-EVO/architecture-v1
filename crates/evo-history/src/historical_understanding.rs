//! Historical Understanding.
//!
//! `HistoricalUnderstanding` is the immutable record of the committed
//! understanding that justified an architectural action.
//!
//! It exists solely to preserve historical accountability (IS-0016 §1).
//!
//! # IS-0016 — Canonical Components (§4)
//!
//! A `HistoricalUnderstanding` consists of exactly:
//!
//! - `HistoryId` — stable identity (H-1, H-2).
//! - `WorkspaceId` — reference to the Workspace whose understanding was committed (H-3).
//! - `RestorationPlan` — the complete committed understanding (H-4, H-5).
//!
//! Nothing else belongs to the canonical object (IS-0016 §4, §10).
//!
//! # Design Goal (IS-0016 §3)
//!
//! `HistoricalUnderstanding` answers one question only:
//!
//! > *"What understanding actually justified this architectural behaviour?"*
//!
//! It is not responsible for reproducing behaviour.
//! It is not responsible for deciding behaviour.
//! It preserves the committed understanding exactly as it existed.
//!
//! # Replay (IS-0016 §9)
//!
//! Replay SHALL NOT modify a `HistoricalUnderstanding`.
//! Replay SHALL create a new `HistoricalUnderstanding`.
//! The previous `HistoricalUnderstanding` remains permanently unchanged (H-7, H-8).
//!
//! # IS-0016 Invariants Enforced
//!
//! - H-1:  Exactly one `HistoryId`.
//! - H-2:  `HistoryId` never changes.
//! - H-3:  References exactly one `WorkspaceId`.
//! - H-4:  Owns exactly one `RestorationPlan`.
//! - H-5:  `RestorationPlan` is immutable after construction.
//! - H-6:  `HistoricalUnderstanding` is immutable after construction.
//! - H-9:  Does not own a `Workspace`.
//! - H-10: Does not own an `Observation`.
//! - H-11: Does not own an `Artifact`.
//! - H-12: Does not own `Knowledge`.
//! - H-13: Does not duplicate `RestorationPlan` components.
//! - H-14: Preserves the `RestorationPlan` exactly as committed.
//!
//! # Public API (IS-0016 §13)
//!
//! Exactly as specified. No setters. No mutation.

use evo_restoration::RestorationPlan;
use evo_workspace::WorkspaceId;

use crate::history_id::HistoryId;

// ── HistoricalUnderstanding ───────────────────────────────────────────────────

/// The immutable record of the committed understanding that justified an
/// architectural action (IS-0016 §1).
///
/// Exists solely to preserve historical accountability.
///
/// # Canonical Components (IS-0016 §4)
///
/// - `HistoryId`: stable, unique identity (H-1, H-2).
/// - `WorkspaceId`: the Workspace whose understanding was committed (H-3).
/// - `RestorationPlan`: the complete committed understanding, frozen permanently (H-4, H-5).
///
/// # Invariants
///
/// - Immutable after construction (H-6).
/// - `RestorationPlan` is preserved exactly as committed (H-14).
/// - Never owns a `Workspace`, `Observation`, `Artifact`, or `Knowledge` (H-9 through H-12).
///
/// # Non-Responsibilities
///
/// - Does **not** reproduce behaviour.
/// - Does **not** decide behaviour.
/// - Does **not** duplicate `RestorationPlan` components (H-13).
/// - Does **not** modify the `RestorationPlan` after construction (H-5, H-14).
/// - Does **not** own the referenced `Workspace` (H-9).
#[derive(Debug, Clone, PartialEq)]
pub struct HistoricalUnderstanding {
    /// Stable, unique identity of this committed understanding (H-1, H-2).
    id: HistoryId,

    /// The Workspace whose understanding was committed (H-3).
    ///
    /// A reference only. `HistoricalUnderstanding` SHALL NEVER own a `Workspace` (H-9).
    workspace_id: WorkspaceId,

    /// The complete committed understanding, frozen permanently (H-4, H-5, H-14).
    restoration_plan: RestorationPlan,
}

impl HistoricalUnderstanding {
    /// Constructs an immutable `HistoricalUnderstanding`.
    ///
    /// Construction is infallible (IS-0016 §15). All invariants are satisfied
    /// by the types of the supplied arguments.
    ///
    /// # Parameters
    ///
    /// - `id`: the stable, unique identity of this committed understanding (H-1).
    /// - `workspace_id`: the Workspace whose understanding was committed (H-3).
    /// - `restoration_plan`: the complete committed understanding (H-4).
    ///
    /// # Guarantees
    ///
    /// - All components are immutable after construction (H-5, H-6).
    /// - The `RestorationPlan` is preserved exactly as supplied (H-14).
    /// - No component is modified, replaced, or partially reconstructed after
    ///   this call (H-14).
    pub fn new(
        id: HistoryId,
        workspace_id: WorkspaceId,
        restoration_plan: RestorationPlan,
    ) -> Self {
        Self {
            id,
            workspace_id,
            restoration_plan,
        }
    }

    /// Returns the stable, unique identity of this committed understanding (H-1, H-2).
    pub fn id(&self) -> &HistoryId {
        &self.id
    }

    /// Returns the `WorkspaceId` of the Workspace whose understanding was committed (H-3).
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Returns the committed `RestorationPlan`, frozen permanently (H-4, H-5, H-14).
    pub fn restoration_plan(&self) -> &RestorationPlan {
        &self.restoration_plan
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use evo_restoration::{Blocker, ContextChain, NextStep, ResumePoint};

    fn workspace_id() -> WorkspaceId {
        WorkspaceId::new()
    }

    fn restoration_plan(workspace_id: WorkspaceId) -> RestorationPlan {
        let resume_point =
            ResumePoint::new(workspace_id.clone(), artifact_id("history-test-rp-artifact"));
        let context_chain =
            ContextChain::new(vec![artifact_id("history-test-cc-artifact")]).unwrap();
        let blockers = vec![Blocker::new("unresolved test dependency").unwrap()];
        let next_step = NextStep::new(
            workspace_id.clone(),
            "continue implementing the historical understanding tests",
        ).unwrap();
        RestorationPlan::new(
            workspace_id,
            resume_point,
            context_chain,
            blockers,
            next_step,
        )
        .unwrap()
    }

    fn understanding() -> HistoricalUnderstanding {
        let wid = workspace_id();
        HistoricalUnderstanding::new(HistoryId::new(), wid.clone(), restoration_plan(wid))
    }

use evo_artifact::artifact_id::ArtifactId;    
#[cfg(test)]
    #[test]
    fn construction_and_accessors() {
        let id = HistoryId::new();
        let wid = workspace_id();
        let plan = restoration_plan(wid.clone());

        let hu = HistoricalUnderstanding::new(id.clone(), wid.clone(), plan.clone());

        assert_eq!(hu.id(), &id);
        assert_eq!(hu.workspace_id(), &wid);
        assert_eq!(hu.restoration_plan(), &plan);
    }

    fn artifact_id(label: &str) -> ArtifactId {
    ArtifactId::new(label).unwrap()
}
    #[test]
    fn construction_is_infallible() {
        // No Result returned — construction always succeeds (IS-0016 §15).
        let _ = understanding();
    }

    #[test]
    fn clone_is_equal_to_original() {
        let hu = understanding();
        assert_eq!(hu.clone(), hu);
    }

    #[test]
    fn history_id_is_stable() {
        // H-2: HistoryId never changes. Verified structurally — no &mut self methods exist.
        let hu = understanding();
        let id_first = hu.id().clone();
        let id_second = hu.id().clone();
        assert_eq!(id_first, id_second);
    }

    #[test]
    fn workspace_id_is_stable() {
        // H-3: references exactly one WorkspaceId.
        let wid = workspace_id();
        let hu = HistoricalUnderstanding::new(
            HistoryId::new(),
            wid.clone(),
            restoration_plan(wid.clone()),
        );
        assert_eq!(hu.workspace_id(), &wid);
    }

    #[test]
    fn restoration_plan_is_preserved_exactly() {
        // H-14: RestorationPlan is preserved exactly as committed.
        let wid = workspace_id();
        let plan = restoration_plan(wid.clone());
        let hu = HistoricalUnderstanding::new(HistoryId::new(), wid, plan.clone());
        assert_eq!(hu.restoration_plan(), &plan);
    }

    #[test]
    fn two_understandings_with_distinct_ids_are_not_equal() {
        // Replay creates new HistoricalUnderstanding with new HistoryId (IS-0016 §5, H-8).
        let wid = workspace_id();
        let plan_a = restoration_plan(wid.clone());
        let plan_b = restoration_plan(wid.clone());

        let hu_a = HistoricalUnderstanding::new(HistoryId::new(), wid.clone(), plan_a);
        let hu_b = HistoricalUnderstanding::new(HistoryId::new(), wid, plan_b);

        assert_ne!(hu_a, hu_b);
    }

    #[test]
    fn is_immutable_after_construction() {
        // H-6: HistoricalUnderstanding is immutable after construction.
        // The type system enforces this: no &mut self methods exist.
        let hu = understanding();
        let _ = hu.id();
        let _ = hu.workspace_id();
        let _ = hu.restoration_plan();
    }

    #[test]
    fn does_not_own_workspace_directly() {
        // H-9: HistoricalUnderstanding SHALL NEVER own a Workspace.
        // Structural test: workspace_id() returns &WorkspaceId, not &Workspace.
        let wid = workspace_id();
        let hu = HistoricalUnderstanding::new(
            HistoryId::new(),
            wid.clone(),
            restoration_plan(wid.clone()),
        );
        // Only WorkspaceId is accessible — no Workspace object is owned.
        let _: &WorkspaceId = hu.workspace_id();
    }

    #[test]
    fn restoration_plan_workspace_consistent_with_hu_workspace() {
        // The RestorationPlan references the same WorkspaceId as the HistoricalUnderstanding.
        let wid = workspace_id();
        let plan = restoration_plan(wid.clone());
        let hu = HistoricalUnderstanding::new(HistoryId::new(), wid.clone(), plan);

        assert_eq!(hu.workspace_id(), hu.restoration_plan().workspace_id());
    }
}
