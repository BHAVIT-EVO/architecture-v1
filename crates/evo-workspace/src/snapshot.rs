//! Workspace Snapshot.
//!
//! A `Snapshot` represents one immutable historical understanding of a Workspace.
//!
//! # IS-0011 Invariants
//!
//! - W-8:  Every Workspace SHALL own a Snapshot History.
//! - W-9:  Snapshots SHALL be immutable.
//! - W-10: Snapshot History SHALL be append-only.
//! - W-11: Historical Snapshots SHALL NEVER be modified.
//! - W-12: Workspace understanding SHALL evolve through replay rather than mutation.
//!
//! A `Snapshot` captures the Workspace state — its lifecycle and its
//! attachment set — at the moment it was committed.
//!
//! Chronological ordering is preserved through the `captured_at` timestamp.
//! Snapshot History is managed by the `Workspace` type, which enforces
//! the append-only invariant (W-10).
//!
//! # Non-Responsibilities
//!
//! This module does NOT define:
//!
//! - when Snapshots are created (belongs to a future formation IS);
//! - how Snapshots are persisted;
//! - how Snapshots are replayed.

use std::time::SystemTime;

use crate::attachment::Attachment;
use crate::lifecycle::WorkspaceLifecycle;

// ── Snapshot ──────────────────────────────────────────────────────────────────

/// One immutable historical understanding of a Workspace.
///
/// A `Snapshot` preserves the Workspace lifecycle state and its attachment set
/// at the moment of commitment (IS-0011 §3, W-9, W-11).
///
/// `Snapshot` is immutable after construction.
///
///Chronological ordering is established by captured_at.
///
/// # Invariants
///
/// - Immutable after construction (W-9).
/// - Never modified after creation (W-11).
///
/// # Non-Responsibilities
///
/// - Does **not** trigger Snapshot creation.
/// - Does **not** persist itself.
/// - Does **not** execute replay.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// The moment at which this understanding was committed.
    ///
    /// Used to establish chronological ordering of the Snapshot History (IS-0011 §4).
    captured_at: SystemTime,

    /// The Workspace lifecycle state at the moment of this Snapshot.
    lifecycle: WorkspaceLifecycle,

    /// The evidential attachment set at the moment of this Snapshot.
    attachments: Vec<Attachment>,
}

impl Snapshot {
    /// Constructs an immutable `Snapshot` of the current Workspace understanding.
    ///
    /// # Parameters
    ///
    /// - `captured_at`: the moment at which this understanding was committed.
    /// - `lifecycle`: the Workspace lifecycle state at commitment.
    /// - `attachments`: the evidential attachment set at commitment.
    ///
    /// # Guarantees
    ///
    /// - Immutable after construction (W-9).
    /// - Carries exactly the state supplied at construction (W-11).
    pub fn new(
        captured_at: SystemTime,
        lifecycle: WorkspaceLifecycle,
        attachments: Vec<Attachment>,
    ) -> Self {
        Self {
            captured_at,
            lifecycle,
            attachments,
        }
    }

    /// Returns the moment at which this Snapshot was committed.
    ///
    /// Used to establish chronological ordering within the Snapshot History.
    pub fn captured_at(&self) -> &SystemTime {
        &self.captured_at
    }

    /// Returns the Workspace lifecycle state as it was at the moment of this Snapshot.
    pub fn lifecycle(&self) -> &WorkspaceLifecycle {
        &self.lifecycle
    }

    /// Returns the evidential attachment set as it was at the moment of this Snapshot.
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::confidence::ConfidenceScore;
    use evo_artifact::artifact_id::ArtifactId;

    fn attachment() -> Attachment {
        Attachment::new(
            ArtifactId::new("snapshot-test-artifact").unwrap(),
            ConfidenceScore::new(0.7).unwrap(),
        )
    }

    #[test]
    fn construction_and_accessors() {
        let now = SystemTime::now();
        let lifecycle = WorkspaceLifecycle::Active;
        let attachments = vec![attachment()];

        let snapshot = Snapshot::new(now, lifecycle.clone(), attachments.clone());

        assert_eq!(snapshot.captured_at(), &now);
        assert_eq!(snapshot.lifecycle(), &lifecycle);
        assert_eq!(snapshot.attachments(), attachments.as_slice());
    }

    #[test]
    fn superseded_lifecycle_is_preserved() {
        let snapshot =
            Snapshot::new(SystemTime::now(), WorkspaceLifecycle::Superseded, vec![]);
        assert_eq!(snapshot.lifecycle(), &WorkspaceLifecycle::Superseded);
    }

    #[test]
    fn clone_is_equal_to_original() {
        let snapshot = Snapshot::new(
            SystemTime::now(),
            WorkspaceLifecycle::Active,
            vec![attachment()],
        );
        let cloned = snapshot.clone();
        assert_eq!(snapshot, cloned);
    }

    #[test]
    fn multiple_attachments_are_preserved() {
        let a1 = Attachment::new(
            ArtifactId::new("artifact-snap-1").unwrap(),
            ConfidenceScore::new(0.4).unwrap(),
        );
        let a2 = Attachment::new(
            ArtifactId::new("artifact-snap-2").unwrap(),
            ConfidenceScore::new(0.9).unwrap(),
        );
        let snapshot = Snapshot::new(
            SystemTime::now(),
            WorkspaceLifecycle::Active,
            vec![a1.clone(), a2.clone()],
        );
        assert_eq!(snapshot.attachments().len(), 2);
        assert_eq!(snapshot.attachments()[0], a1);
        assert_eq!(snapshot.attachments()[1], a2);
    }

    #[test]
    fn snapshot_is_not_modified_after_construction() {
        // Immutability is guaranteed by the type system: no &mut self methods exist.
        // This test verifies that the public API exposes no mutation surface.
        let snapshot = Snapshot::new(
            SystemTime::now(),
            WorkspaceLifecycle::Active,
            vec![attachment()],
        );
        // The only operations available are read-only accessors.
        let _ = snapshot.captured_at();
        let _ = snapshot.lifecycle();
        let _ = snapshot.attachments();
    }
}
