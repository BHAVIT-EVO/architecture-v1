//! Workspace.
//!
//! The canonical Workspace computational object.
//!
//! A Workspace represents Evo's current best explanation that a collection
//! of Artifact histories collectively describe one coherent body of work.
//!
//! It is a body of work, never a resource. What makes a set of Artifacts one
//! Workspace is [`crate::projection`]; this module defines only what a Workspace
//! holds.
//!
//! Workspace is a persistent, replayable computational primitive.
//! It owns Attachments, Snapshot History, and Workspace Lifecycle.
//! It does not own Artifacts.

use crate::attachment::Attachment;
use crate::attachment::ResourceRole;
use crate::lifecycle::WorkspaceLifecycle;
use crate::snapshot::Snapshot;
use crate::workspace_id::WorkspaceId;

use evo_artifact::artifact_id::ArtifactId;

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

    /// Every distinct Artifact this Workspace claims as part of the work, each
    /// with the part it played, in the Workspace's own order of leadership.
    ///
    /// # Why the Attachment Set and not the Snapshot History
    ///
    /// This was got wrong once, on real history, and the shape of the mistake is
    /// worth keeping. Reading membership off the Snapshot History looks more
    /// thorough — it sounds like "everything, across all of time" — but the
    /// Snapshot History is deliberately *not* that. Every Snapshot holds the
    /// members active in one sitting, and the invariant this crate enforces is
    /// only that each Snapshot describes a **subset** of the Workspace. The
    /// union of those subsets is therefore allowed to be strictly smaller than
    /// the membership, and on real history it was: a resource attached to the
    /// work but active in none of its sittings appeared in no Snapshot, so a
    /// fold over Snapshots dropped it. Dropped from membership means dropped
    /// from selective restoration entirely — not opened, and, worse, not
    /// withheld either, so it was no longer recoverable at all. The Attachment
    /// Set is the Workspace's own answer to "what belongs to this work",
    /// which is the question membership asks.
    ///
    /// Roles are not folded across Snapshots either, and there is nothing to
    /// fold: a Snapshot carries the very Attachment the Workspace holds, so an
    /// Artifact's role is one value, not one per sitting. What this does still
    /// collapse is an Artifact named more than once in a single set — that
    /// cannot arise from projection, which attaches each Artifact once, but
    /// [`Workspace::new`] is public, and a Workspace claiming one Artifact twice
    /// would otherwise be offered to restoration twice. Duplicates keep the most
    /// important part claimed, the same rule Restoration Derivation applies.
    ///
    /// The Snapshot History is still swept, after the Attachment Set, purely so
    /// that a Workspace built by hand rather than by projection cannot hide a
    /// member from restoration in a Snapshot. Under the invariant that sweep
    /// finds nothing new; it exists so the failure mode above cannot return by a
    /// different door.
    ///
    /// # Why this lives on Workspace rather than in a caller
    ///
    /// This is the membership every downstream reading of a body of work has to
    /// agree on — in particular the input to selective restoration, which
    /// decides what gets opened and what is merely kept. When two callers
    /// extract membership their own way, the acceptance gate stops measuring
    /// what the product does, and a member dropped by one of them silently
    /// disappears from the withheld set instead of remaining recoverable. So the
    /// rule lives once, on the object that owns the membership, and returns
    /// [`ArtifactId`] rather than strings so no caller has to re-parse an
    /// identity that was already canonical.
    pub fn members_across_history(&self) -> Vec<(ArtifactId, ResourceRole)> {
        let mut distinct: Vec<(ArtifactId, ResourceRole)> = Vec::new();
        let snapshotted = self
            .snapshots
            .iter()
            .flat_map(|snapshot| snapshot.attachments());
        for attachment in self.attachments.iter().chain(snapshotted) {
            let id = attachment.artifact_id();
            let role = attachment.role();
            match distinct.iter_mut().find(|(seen, _)| seen == id) {
                // ResourceRole declares its variants in importance order.
                Some((_, seen_role)) if role < *seen_role => *seen_role = role,
                Some(_) => {}
                None => distinct.push((id.clone(), role)),
            }
        }
        distinct
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
            crate::attachment::ResourceRole::Primary,
        )
    }

    fn snapshot() -> Snapshot {
        Snapshot::new(
            SystemTime::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![attachment()],
        )
    }

    fn member(id: &str, role: crate::attachment::ResourceRole) -> Attachment {
        Attachment::new(
            ArtifactId::new(id).unwrap(),
            ConfidenceScore::new(1.0).unwrap(),
            role,
        )
    }

    /// The defect this pins was found on real history, not imagined.
    ///
    /// A resource can be attached to a body of work and active in none of its
    /// sittings — a page open beside the work that drew no attention. Every
    /// Snapshot holds only the members active in one sitting, so such a member
    /// appears in no Snapshot at all. Membership read off the Snapshot History
    /// therefore lost it, and losing it from membership lost it from selective
    /// restoration in the worst possible way: not opened, and not withheld
    /// either, so no longer recoverable by any route.
    #[test]
    fn membership_keeps_a_member_that_no_sitting_was_active_in() {
        use crate::attachment::ResourceRole;

        let worked_in = member("artifact-worked-in", ResourceRole::Primary);
        let merely_present = member("artifact-merely-present", ResourceRole::Context);

        // Exactly what projection produces: both attached, but only the member
        // that was active in the sitting appears in that sitting's Snapshot.
        let workspace = Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![worked_in.clone(), merely_present.clone()],
            vec![Snapshot::new(
                SystemTime::UNIX_EPOCH,
                WorkspaceLifecycle::Active,
                vec![worked_in.clone()],
            )],
        );

        let members = workspace.members_across_history();

        assert_eq!(
            members,
            vec![
                (worked_in.artifact_id().clone(), ResourceRole::Primary),
                (merely_present.artifact_id().clone(), ResourceRole::Context),
            ],
            "a member no sitting was active in must still be membership, or it \
             is neither opened nor withheld and cannot be recovered at all"
        );
    }

    /// A Workspace claiming one Artifact twice must not offer it to restoration
    /// twice, and must claim the most important part it was given. Projection
    /// cannot produce this, but [`Workspace::new`] is public.
    #[test]
    fn membership_collapses_one_artifact_to_its_most_important_part() {
        use crate::attachment::ResourceRole;

        let reference = member("artifact-1", ResourceRole::Reference);
        let continuation = member("artifact-1", ResourceRole::Continuation);

        let workspace = Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![reference.clone()],
            vec![Snapshot::new(
                SystemTime::UNIX_EPOCH,
                WorkspaceLifecycle::Active,
                vec![continuation],
            )],
        );

        assert_eq!(
            workspace.members_across_history(),
            vec![(
                reference.artifact_id().clone(),
                ResourceRole::Continuation
            )],
            "one Artifact is one member, holding the most important part claimed"
        );
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

        let workspace = Workspace::new(id.clone(), WorkspaceLifecycle::Active, vec![], vec![]);

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
