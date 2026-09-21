//! Deterministic helpers for Workspace projection.
//!
//! This module contains stable, internal-only mechanisms used to keep the
//! Workspace engine replayable without exposing implementation details.
//!
//! Nothing here reads the clock, allocates a random value, or depends on the
//! order in which Observations arrived. That is what allows the Workspace
//! projection to be recomputed from the canonical history and produce exactly
//! what it produced live (§19).

use crate::attachment::Attachment;
use crate::lifecycle::WorkspaceLifecycle;
use crate::snapshot::Snapshot;
use crate::workspace::Workspace;
use crate::workspace_id::WorkspaceId;

use evo_artifact::artifact_id::ArtifactId;
use evo_engagement::ResourceRole;

use std::time::{SystemTime, UNIX_EPOCH};

const FNV_OFFSET_A: u64 = 0xcbf29ce484222325;
const FNV_OFFSET_B: u64 = 0x8422_2325_cbf2_9ce4;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// The stable identity of the Workspace founded by a given witnessed sighting.
///
/// A pure function of the sighting string, so the same body of work recovers the
/// same identity every time the history is reconstructed — across restarts, and
/// across replay (W-2).
///
/// The caller supplies a *witnessed* name rather than a canonical one, for the
/// reason [`crate::projection`] documents at length: canonical names are learned
/// from the corpus and improve as it grows, so they are conclusions and not keys.
///
/// The hash is one-way and the identity decodes to nothing, which is what keeps
/// W-3 intact: identity is derived *from* a sighting without encoding it.
pub(crate) fn founding_identity(sighting: &str) -> WorkspaceId {
    let left = hash_bytes(
        hash_bytes(FNV_OFFSET_A, b"workspace-founded-by"),
        sighting.as_bytes(),
    );
    let right = hash_bytes(
        hash_bytes(FNV_OFFSET_B, b"workspace-founded-by"),
        sighting.as_bytes(),
    );
    WorkspaceId::from_u128((u128::from(left) << 64) | u128::from(right))
}

/// Whether a Workspace satisfies the invariants this crate guarantees.
///
/// # What changed, and why
///
/// This function used to require the last Snapshot's attachment set to equal the
/// Workspace's attachment set. That made sense while a Snapshot was taken every
/// time formation ran: the newest Snapshot *was* the current understanding, so
/// anything else was a bug.
///
/// It stopped making sense when the Snapshot History became the record of how the
/// work unfolded — one Snapshot per sitting, holding the members that were active
/// in that sitting. Under that meaning the last Snapshot describes the most recent
/// sitting, and the Workspace describes the whole body of work; requiring them to
/// match would forbid the ordinary and important case of returning to a piece of
/// work and touching only part of it. Enforcing the old equality would have meant
/// either flattening every sitting into one Snapshot (destroying the history §6
/// asks for) or attaching only what was last touched (destroying the record of
/// what else the work involves).
///
/// The invariant that survives is the one that was actually protecting anything:
/// **every Snapshot describes a subset of this Workspace**. A Snapshot naming an
/// Attachment the Workspace does not have would be an unaccountable claim, and
/// that is still rejected.
pub(crate) fn workspace_is_valid(workspace: &Workspace) -> bool {
    if workspace.attachments().is_empty() || workspace.snapshots().is_empty() {
        return false;
    }

    if workspace
        .attachments()
        .iter()
        .any(|attachment| !attachment_is_valid(attachment))
    {
        return false;
    }

    if workspace.snapshots().iter().any(|snapshot| {
        snapshot.attachments().is_empty()
            || snapshot
                .attachments()
                .iter()
                .any(|attachment| !attachment_is_valid(attachment))
    }) {
        return false;
    }

    // Every Snapshot must be accountable to the Workspace it belongs to.
    if workspace.snapshots().iter().any(|snapshot| {
        snapshot.attachments().iter().any(|attachment| {
            !workspace
                .attachments()
                .iter()
                .any(|owned| owned.artifact_id() == attachment.artifact_id())
        })
    }) {
        return false;
    }

    if workspace
        .snapshots()
        .windows(2)
        .any(|pair| pair[0].captured_at() > pair[1].captured_at())
    {
        return false;
    }

    let Some(last_snapshot) = workspace.snapshots().last() else {
        return false;
    };

    if last_snapshot.lifecycle() != workspace.lifecycle() {
        return false;
    }

    true
}

fn attachment_is_valid(attachment: &Attachment) -> bool {
    if attachment.artifact_id().as_str().trim().is_empty() {
        return false;
    }
    let confidence = attachment.confidence().value();
    confidence.is_finite() && (0.0..=1.0).contains(&confidence)
}

/// A stable digest of everything a Workspace asserts.
///
/// Two Workspaces with the same digest make the same claim: same lifecycle, same
/// Attachments with the same strengths and roles, same Snapshot History. Same
/// input, same digest, always — no clock and no ordering hazard.
///
/// This exists so persistence can tell whether a recomputed conclusion actually
/// differs from the one already recorded. Without it, an append-only audit would
/// gain an identical entry on every refresh, and a log of unchanged conclusions
/// is indistinguishable from a log of real ones.
pub(crate) fn understanding_digest(workspace: &Workspace) -> u128 {
    let left = digest_with_seed(FNV_OFFSET_A, workspace);
    let right = digest_with_seed(FNV_OFFSET_B, workspace);
    (u128::from(left) << 64) | u128::from(right)
}

fn digest_with_seed(seed: u64, workspace: &Workspace) -> u64 {
    let mut hash = hash_bytes(seed, b"workspace-understanding");
    hash = hash_lifecycle(hash, workspace.lifecycle());
    for attachment in workspace.attachments() {
        hash = hash_attachment(hash, attachment);
    }
    for snapshot in workspace.snapshots() {
        hash = hash_snapshot(hash, snapshot);
    }
    hash
}

fn hash_snapshot(mut hash: u64, snapshot: &Snapshot) -> u64 {
    hash = hash_bytes(hash, b"snapshot");
    hash = hash_time(hash, *snapshot.captured_at());
    hash = hash_lifecycle(hash, snapshot.lifecycle());

    for attachment in snapshot.attachments() {
        hash = hash_attachment(hash, attachment);
    }

    hash
}

fn hash_attachment(mut hash: u64, attachment: &Attachment) -> u64 {
    hash = hash_bytes(hash, b"attachment");
    hash = hash_artifact_id(hash, attachment.artifact_id());
    hash = hash_role(hash, attachment.role());
    hash_f32(hash, attachment.confidence().value())
}

fn hash_role(hash: u64, role: ResourceRole) -> u64 {
    match role {
        ResourceRole::Continuation => hash_bytes(hash, b"continuation"),
        ResourceRole::Primary => hash_bytes(hash, b"primary"),
        ResourceRole::Supporting => hash_bytes(hash, b"supporting"),
        ResourceRole::Reference => hash_bytes(hash, b"reference"),
        ResourceRole::Context => hash_bytes(hash, b"context"),
    }
}

fn hash_artifact_id(mut hash: u64, artifact_id: &ArtifactId) -> u64 {
    hash = hash_bytes(hash, b"artifact-id");
    hash_bytes(hash, artifact_id.as_str().as_bytes())
}

fn hash_lifecycle(hash: u64, lifecycle: &WorkspaceLifecycle) -> u64 {
    match lifecycle {
        WorkspaceLifecycle::Active => hash_bytes(hash, b"active"),
        WorkspaceLifecycle::Superseded => hash_bytes(hash, b"superseded"),
    }
}

fn hash_time(mut hash: u64, time: SystemTime) -> u64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            hash = hash_bytes(hash, b"time+");
            hash = hash_u64(hash, duration.as_secs());
            hash_u32(hash, duration.subsec_nanos())
        }
        Err(error) => {
            let duration = error.duration();
            hash = hash_bytes(hash, b"time-");
            hash = hash_u64(hash, duration.as_secs());
            hash_u32(hash, duration.subsec_nanos())
        }
    }
}

fn hash_f32(hash: u64, value: f32) -> u64 {
    hash_u32(hash, value.to_bits())
}

fn hash_u32(hash: u64, value: u32) -> u64 {
    hash_bytes(hash, &value.to_le_bytes())
}

fn hash_u64(hash: u64, value: u64) -> u64 {
    hash_bytes(hash, &value.to_le_bytes())
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::confidence::ConfidenceScore;
    use evo_engagement::ResourceRole;

    use std::time::Duration;

    fn attachment(id: &str, confidence: f32) -> Attachment {
        Attachment::new(
            ArtifactId::new(id).expect("a valid artifact id"),
            ConfidenceScore::new(confidence).expect("a valid confidence"),
            ResourceRole::Primary,
        )
    }

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn founding_identity_is_stable_for_the_same_subject() {
        assert_eq!(
            founding_identity("/Users/x/thesis/chapter-three.tex"),
            founding_identity("/Users/x/thesis/chapter-three.tex")
        );
    }

    #[test]
    fn founding_identity_differs_between_subjects() {
        assert_ne!(founding_identity("one"), founding_identity("two"));
    }

    /// The invariant that replaced the old last-Snapshot equality check: a
    /// Snapshot describing only part of the work is correct, because that is what
    /// a sitting is.
    #[test]
    fn a_snapshot_may_describe_only_part_of_the_workspace() {
        let workspace = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![attachment("a", 0.9), attachment("b", 0.5)],
            vec![
                Snapshot::new(
                    at(0),
                    WorkspaceLifecycle::Active,
                    vec![attachment("a", 0.9), attachment("b", 0.5)],
                ),
                Snapshot::new(at(100), WorkspaceLifecycle::Active, vec![attachment("a", 0.9)]),
            ],
        );
        assert!(workspace_is_valid(&workspace));
    }

    /// The invariant that survived: a Snapshot cannot claim something the
    /// Workspace does not have.
    #[test]
    fn a_snapshot_naming_an_unattached_artifact_is_rejected() {
        let workspace = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![attachment("a", 0.9)],
            vec![Snapshot::new(
                at(0),
                WorkspaceLifecycle::Active,
                vec![attachment("stranger", 0.9)],
            )],
        );
        assert!(!workspace_is_valid(&workspace));
    }

    #[test]
    fn out_of_order_snapshots_are_rejected() {
        let workspace = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![attachment("a", 0.9)],
            vec![
                Snapshot::new(at(100), WorkspaceLifecycle::Active, vec![attachment("a", 0.9)]),
                Snapshot::new(at(0), WorkspaceLifecycle::Active, vec![attachment("a", 0.9)]),
            ],
        );
        assert!(!workspace_is_valid(&workspace));
    }

    #[test]
    fn an_empty_snapshot_is_rejected() {
        let workspace = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![attachment("a", 0.9)],
            vec![Snapshot::new(at(0), WorkspaceLifecycle::Active, vec![])],
        );
        assert!(!workspace_is_valid(&workspace));
    }

    #[test]
    fn a_workspace_with_no_attachments_is_rejected() {
        let workspace = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![],
            vec![Snapshot::new(
                at(0),
                WorkspaceLifecycle::Active,
                vec![attachment("a", 0.9)],
            )],
        );
        assert!(!workspace_is_valid(&workspace));
    }

    #[test]
    fn a_lifecycle_mismatch_with_the_last_snapshot_is_rejected() {
        let workspace = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![attachment("a", 0.9)],
            vec![Snapshot::new(
                at(0),
                WorkspaceLifecycle::Superseded,
                vec![attachment("a", 0.9)],
            )],
        );
        assert!(!workspace_is_valid(&workspace));
    }

    /// The audit digest is content-sensitive, which is what makes it usable to
    /// tell whether a persisted conclusion still matches the live one.
    #[test]
    fn the_understanding_digest_reflects_content() {
        let one = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![attachment("a", 0.9)],
            vec![Snapshot::new(
                at(0),
                WorkspaceLifecycle::Active,
                vec![attachment("a", 0.9)],
            )],
        );
        let two = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![attachment("b", 0.9)],
            vec![Snapshot::new(
                at(0),
                WorkspaceLifecycle::Active,
                vec![attachment("b", 0.9)],
            )],
        );
        assert_ne!(understanding_digest(&one), understanding_digest(&two));
        assert_eq!(understanding_digest(&one), understanding_digest(&one.clone()));
    }

    /// A role change is a change in what Evo asserts, so it must change the
    /// digest — otherwise a body of work whose continuation point moved would be
    /// recorded as unchanged.
    #[test]
    fn the_understanding_digest_reflects_role() {
        let primary = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![attachment("a", 0.9)],
            vec![Snapshot::new(
                at(0),
                WorkspaceLifecycle::Active,
                vec![attachment("a", 0.9)],
            )],
        );
        let reference_attachment = Attachment::new(
            ArtifactId::new("a").expect("a valid artifact id"),
            ConfidenceScore::new(0.9).expect("a valid confidence"),
            ResourceRole::Reference,
        );
        let reference = Workspace::new(
            founding_identity("subject"),
            WorkspaceLifecycle::Active,
            vec![reference_attachment.clone()],
            vec![Snapshot::new(
                at(0),
                WorkspaceLifecycle::Active,
                vec![reference_attachment],
            )],
        );
        assert_ne!(
            understanding_digest(&primary),
            understanding_digest(&reference)
        );
    }
}
