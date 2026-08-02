//! Workspace.
//!
//! The canonical Workspace computational object.
//!
//! A Workspace represents Evo's current best explanation that a collection
//! of Artifact histories collectively describe one coherent body of work.
//!
//! Workspace is a persistent, replayable computational primitive.
//! It owns Attachments, Snapshot History, and Workspace Lifecycle.
//! It does not own Artifacts.

use crate::attachment::Attachment;
use crate::lifecycle::WorkspaceLifecycle;
use crate::snapshot::Snapshot;
use crate::workspace_id::WorkspaceId;

/// The canonical Workspace.
///
/// A Workspace is the first long-lived interpretation in Evo's computational
/// model. It groups Artifacts into a coherent body of work while remaining
/// fully accountable to Artifact history.
///
/// Workspace is immutable after construction. Evolution of Workspace
/// understanding occurs through replay, producing new Snapshots or new
/// Workspaces rather than mutating existing ones.
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    id: WorkspaceId,
    lifecycle: WorkspaceLifecycle,
    attachments: Vec<Attachment>,
    snapshots: Vec<Snapshot>,
}

impl Workspace {
    /// Constructs a new Workspace.
    pub fn new(
        id: WorkspaceId,
        lifecycle: WorkspaceLifecycle,
        attachments: Vec<Attachment>,
        snapshots: Vec<Snapshot>,
    ) -> Self {
        Self {
            id,
            lifecycle,
            attachments,
            snapshots,
        }
    }

    /// Returns the Workspace Identity.
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    /// Returns the current Lifecycle.
    pub fn lifecycle(&self) -> &WorkspaceLifecycle {
        &self.lifecycle
    }

    /// Returns the Attachment Set.
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }

    /// Returns the Snapshot History.
    pub fn snapshots(&self) -> &[Snapshot] {
        &self.snapshots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::attachment::Attachment;
    use crate::confidence::ConfidenceScore;
    use crate::lifecycle::WorkspaceLifecycle;
    use crate::snapshot::Snapshot;

    use evo_artifact::artifact_id::ArtifactId;

    use std::time::SystemTime;

    fn attachment() -> Attachment {
        Attachment::new(
            ArtifactId::new("artifact-1").unwrap(),
            ConfidenceScore::new(1.0).unwrap(),
        )
    }

    fn snapshot() -> Snapshot {
        Snapshot::new(
            SystemTime::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![attachment()],
        )
    }

    #[test]
    fn workspace_constructs_successfully() {
        let workspace = Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![attachment()],
            vec![snapshot()],
        );

        assert_eq!(workspace.lifecycle(), &WorkspaceLifecycle::Active);
        assert_eq!(workspace.attachments().len(), 1);
        assert_eq!(workspace.snapshots().len(), 1);
    }

    #[test]
    fn workspace_id_accessor_returns_identity() {
        let id = WorkspaceId::new();

        let workspace = Workspace::new(
            id.clone(),
            WorkspaceLifecycle::Active,
            vec![],
            vec![],
        );

        assert_eq!(workspace.id(), &id);
    }

    #[test]
    fn workspace_preserves_attachment_order() {
        let first = attachment();
        let second = attachment();

        let workspace = Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![first.clone(), second.clone()],
            vec![],
        );

        assert_eq!(workspace.attachments()[0], first);
        assert_eq!(workspace.attachments()[1], second);
    }

    #[test]
    fn workspace_preserves_snapshot_history() {
        let first = snapshot();
        let second = snapshot();

        let workspace = Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![],
            vec![first.clone(), second.clone()],
        );

        assert_eq!(workspace.snapshots()[0], first);
        assert_eq!(workspace.snapshots()[1], second);
    }

    #[test]
    fn workspace_accessors_are_read_only() {
        let workspace = Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![],
            vec![],
        );

        let _: &WorkspaceId = workspace.id();
        let _: &WorkspaceLifecycle = workspace.lifecycle();
        let _: &[Attachment] = workspace.attachments();
        let _: &[Snapshot] = workspace.snapshots();
    }
}