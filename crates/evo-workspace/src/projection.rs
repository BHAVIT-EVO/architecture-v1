//! Projecting reconstructed bodies of work onto Workspaces.
//!
//! This is the rule that decides what a Workspace *is*, and it lives here
//! because this crate owns the Workspace definition.
//!
//! # What replaced what
//!
//! The old rule was one line of arithmetic: an Artifact witnessed at least twice
//! originated a Workspace. It had the shape of a rule about work and the content
//! of a counter, and everything a computer touches twice satisfies it. On a real
//! machine it produced a Workspace for a music player, one for a chat window, one
//! for a search, one for a file the operating system rewrote — Home became the
//! Observation log with different typography. No amount of tuning that threshold
//! fixes it, because the quantity being counted is not evidence of work.
//!
//! The new rule is that a Workspace is the *projection of an Engagement*
//! ([`evo_engagement`]): a set of resources the evidence says were being used
//! together, for something, by a person who was present. Formation is no longer a
//! decision made per Observation; it is a pure function of the whole canonical
//! history. That is a strictly stronger claim than the old one and it is the
//! claim the product needs, because "there is work here worth returning to" is
//! exactly what Home asserts.
//!
//! # Why a projection and not an incremental fold
//!
//! Relatedness is a property of a *corpus*, not of an event. Whether two
//! resources are meaningfully associated depends on how ubiquitous each of them
//! is across everything else — a resource present beside all activity is related
//! to none of it — and ubiquity cannot be known from one Observation. An
//! incremental formation step would have to guess, and a guess that later turns
//! out wrong cannot be withdrawn from an append-only store.
//!
//! Recomputing from the history has the property that matters instead: live state
//! and replayed state are the same function of the same input, so §19
//! replay-equivalence holds by construction rather than by careful maintenance of
//! two code paths.
//!
//! # What this module deliberately does not do
//!
//! It contains no application names, no domains, no file extensions, no notion of
//! "productive" software, and no model of any profession. It cannot: it never
//! sees a resource's content or its application, only the structure
//! [`evo_engagement`] reconstructed. A rule of that kind would have to be added
//! here to exist, and there is nowhere in this file it would fit.

use std::collections::BTreeMap;

use evo_artifact::artifact_id::ArtifactId;
use evo_engagement::{Engagement, Episode, IdentityIndex, Participant, Reconstruction};

use crate::attachment::Attachment;
use crate::confidence::ConfidenceScore;
use crate::deterministic::workspace_is_valid;
use crate::lifecycle::WorkspaceLifecycle;
use crate::snapshot::Snapshot;
use crate::workspace::Workspace;
use crate::workspace_id::WorkspaceId;

/// Resolves a canonical resource subject to the Artifact that carries its
/// history.
///
/// A `BTreeMap` rather than a closure so the projection stays a pure,
/// inspectable function of its inputs — testable without a daemon, and
/// deterministic regardless of how the caller assembled it.
pub type SubjectArtifacts = BTreeMap<String, ArtifactId>;

/// Projects every reconstructed thread onto a Workspace.
///
/// "Every" is meant literally: bodies of work *and* the threads Evo witnessed
/// but did not claim as work ([`Standing::Remembered`]) both project. What keeps
/// a Remembered thread off Home is its standing, read by the caller — not its
/// absence from this list. Dropping it here instead would be silent loss: a
/// thread a person was witnessed at, gone from every surface, unfindable even by
/// name. The distinction the product draws is `50 remembered ≠ 50 opened`, and
/// that is a distinction in *standing*, not in *existence*.
///
/// Threads whose members have no corresponding Artifact are still skipped —
/// rather than represented by an empty Workspace: a Workspace with no
/// Attachments asserts something exists while offering nothing to restore *or*
/// retrieve, which is worse than saying nothing.
///
/// Order is preserved from the reconstruction, which is bodies of work first and
/// most-recently-active within each — the thing the person was just doing is the
/// thing they are most likely coming back to.
pub fn project_workspaces(
    reconstruction: &Reconstruction,
    artifacts: &SubjectArtifacts,
) -> Vec<Workspace> {
    project_engagements(reconstruction, artifacts)
        .into_iter()
        .map(|(_, workspace)| workspace)
        .collect()
}

/// Projects every reconstructed thread, keeping each Workspace paired with the
/// Engagement it came from.
///
/// Same rule, same order, same skipping as [`project_workspaces`] — this is that
/// function with the correspondence retained rather than discarded. In
/// particular it projects the *whole* record: [`Reconstruction::set`], not the
/// work-only [`Reconstruction::engagements`]. A Workspace carries no standing
/// (W-3), so the standing that decides whether Home presents a thread as work
/// lives on the paired [`Engagement`] — and it can only travel with the
/// Workspace if the Remembered threads are projected at all. Projecting only the
/// bodies of work would make [`Engagement::standing`] a distinction nothing
/// downstream could act on, because everything that reached the caller would
/// already be work.
///
/// # Why the pairing is exposed
///
/// A Workspace deliberately carries no name. Naming is a *reading* of the
/// evidence, not a canonical fact about the work, so a Workspace holds identity,
/// membership and history and nothing interpretive (W-3). But Home has to call
/// the work something, and the only place that can honestly say what a body of
/// work is *about* is the Engagement, which measured the vocabulary its members
/// share ([`Engagement::title`], and
/// [`Engagement::titled_by_shared_vocabulary`] for whether the choice rested on
/// that evidence at all). The standing travels the same way and for the same
/// reason: it is a reading of the evidence the Engagement already made.
///
/// Without this pairing the caller can only recover a name by picking one of the
/// Workspace's Artifacts, and no such pick is a name for the work: it is a name
/// for one resource inside it. That is exactly how Home came to call an
/// afternoon on a coursework assignment "Spotify Premium".
pub fn project_engagements<'a>(
    reconstruction: &'a Reconstruction,
    artifacts: &SubjectArtifacts,
) -> Vec<(&'a Engagement, Workspace)> {
    reconstruction
        .set()
        .engagements()
        .iter()
        .filter_map(|engagement| {
            project_workspace(
                engagement,
                reconstruction.episodes(),
                reconstruction.identity(),
                artifacts,
            )
            .map(|workspace| (engagement, workspace))
        })
        .collect()
}

/// Projects one body of work onto one Workspace.
///
/// Returns `None` when none of the members resolve to an Artifact, or when the
/// result would violate the Workspace invariants — the projection refuses to
/// emit an invalid Workspace rather than persisting one and hoping.
pub fn project_workspace(
    engagement: &Engagement,
    sittings: &[Episode],
    identity: &IdentityIndex,
    artifacts: &SubjectArtifacts,
) -> Option<Workspace> {
    let attachments = attachments_of(engagement, artifacts);
    if attachments.is_empty() {
        return None;
    }

    let snapshots = snapshots_of(engagement, sittings, artifacts, &attachments)?;
    let id = WorkspaceId::from_work_id(engagement.work_id().as_u128());
    let workspace = Workspace::new(id, WorkspaceLifecycle::Active, attachments, snapshots);
    workspace_is_valid(&workspace).then_some(workspace)
}

/// The Attachment set: one per member that resolves to an Artifact.
///
/// Every member is attached, whatever its role. That is the point of §4's
/// distinction between belonging and importance — a resource merely present
/// during the work still belongs to the record of it, and dropping it would
/// destroy the answer to "what else was I looking at". What its role decides is
/// whether Restoration *opens* it.
///
/// Attachment order follows participant order, which is by leadership: the places
/// the work continues come first. Two members that fold to the same Artifact
/// attach once, at the position of the more important of them, because a
/// Workspace claiming the same Artifact twice would double-count it in every
/// downstream reading.
fn attachments_of(engagement: &Engagement, artifacts: &SubjectArtifacts) -> Vec<Attachment> {
    let mut attachments: Vec<Attachment> = Vec::new();
    for participant in engagement.participants() {
        let Some(artifact_id) = artifacts.get(participant.subject()) else {
            continue;
        };
        if attachments
            .iter()
            .any(|existing| existing.artifact_id() == artifact_id)
        {
            continue;
        }
        attachments.push(Attachment::new(
            artifact_id.clone(),
            confidence_of(participant),
            participant.role(),
        ));
    }
    attachments
}

/// The evidential strength of one membership claim.
///
/// [`Participant::strength`] is exactly this quantity: the strongest measured
/// affinity between this resource and any locus the work formed around, `1.0`
/// when the person said so themselves or when the resource is itself such a
/// locus. It is the strength of *this* membership — a reference consulted while
/// working in one part of the body of work is strongly attached to that part,
/// and the workspace records that strength rather than diluting it into a mean
/// across the whole. It is *not* importance, which is what W-7 forbids
/// confidence from carrying and what [`Attachment::role`] carries instead.
///
/// The mean-to-the-rest ([`Participant::belonging`]) answered a different
/// question — group cohesion — and using it here understated a resource
/// genuinely part of one strand of a body of work that spans several.
fn confidence_of(participant: &Participant) -> ConfidenceScore {
    ConfidenceScore::new(participant.strength() as f32)
        .unwrap_or_else(|_| ConfidenceScore::new(0.0).expect("zero is a valid confidence"))
}

/// The Snapshot History: one Snapshot per sitting this body of work was going on
/// in, in chronological order.
///
/// This is what makes the Snapshot History a record of how the work unfolded
/// rather than a log of when Evo last recomputed. Each Snapshot holds the members
/// that were actually active in that sitting, so the history shows the work
/// starting, being interrupted, and being returned to — the structure §6 asks
/// for, read off the evidence instead of asserted by a threshold.
///
/// Each Snapshot is timed by when its sitting began, which is a witnessed moment.
/// Sittings are separated by a gap wider than the presence horizon and are
/// indexed in time order, so the resulting history is append-only ordered (W-10)
/// without needing a clamp, and the clock is never read — which is what keeps
/// live state and replayed state identical.
///
/// Sittings in which no member resolved to an Artifact are omitted; a Snapshot
/// with no Attachments would claim a sitting happened while being unable to say
/// what was in it.
fn snapshots_of(
    engagement: &Engagement,
    sittings: &[Episode],
    artifacts: &SubjectArtifacts,
    attachments: &[Attachment],
) -> Option<Vec<Snapshot>> {
    let by_artifact: BTreeMap<&str, &Attachment> = attachments
        .iter()
        .map(|attachment| (attachment.artifact_id().as_str(), attachment))
        .collect();

    let mut snapshots = Vec::new();
    for sitting in engagement.episodes() {
        let Some(began) = sittings.get(*sitting).and_then(Episode::started_at) else {
            continue;
        };
        let mut active: Vec<Attachment> = Vec::new();
        for participant in engagement.participants() {
            if !participant.episodes().contains(sitting) {
                continue;
            }
            let Some(artifact_id) = artifacts.get(participant.subject()) else {
                continue;
            };
            let Some(attachment) = by_artifact.get(artifact_id.as_str()) else {
                continue;
            };
            if active
                .iter()
                .any(|existing| existing.artifact_id() == artifact_id)
            {
                continue;
            }
            active.push((*attachment).clone());
        }
        if active.is_empty() {
            continue;
        }
        snapshots.push(Snapshot::new(began, WorkspaceLifecycle::Active, active));
    }

    if snapshots.is_empty() {
        // A body of work Evo chose to present always has at least one recorded
        // understanding. If no sitting could be attributed — which a declared
        // grouping between resources witnessed apart can produce — the whole
        // attachment set stands as the single understanding, timed by when the
        // work was last witnessed.
        let captured_at = engagement
            .last_active()
            .or_else(|| engagement.first_active())?;
        snapshots.push(Snapshot::new(
            captured_at,
            WorkspaceLifecycle::Active,
            attachments.to_vec(),
        ));
    }

    Some(snapshots)
}

/// The witnessed sighting a body of work is identified by.
///
/// The *first name Evo ever saw* for the earliest-founded **seed** — the locus
/// the person was witnessed working in that this work formed around earliest.
/// Ties on time break on canonical subject order.
///
/// # Why the earliest-founded seed rather than the earliest member
///
/// Identity must survive the work changing, and a peripheral member is the
/// wrong thing to key it on twice over. A body of work acquires and loses
/// peripheral resources constantly, so hashing the whole membership would mint a
/// new Workspace every time someone opened a reference page. But taking the
/// earliest *member* has a subtler failure: a reference page can be witnessed
/// before the work proper began, and the same page can sit beside several bodies
/// of work — so keying on it lets an incidental resource name the work, and lets
/// two bodies of work that share that page collide onto one identity. Seeds are
/// the resources the work was actually built around ([`Engagement::seeds`]); the
/// earliest of them is the stable, work-specific founder. A thread with no seeds
/// — a declared grouping between resources witnessed apart, or a group formed
/// from what no locus reached — has only its members to be identified by, so the
/// earliest member stands in.
///
/// # Why the witnessed name rather than the canonical one
///
/// This was found the hard way and it is worth recording. Identity first hashed
/// the founding member's *canonical* subject, and that is not stable — not because
/// of a bug, but because [`evo_engagement`] learns identity from the corpus. A
/// resource witnessed once as `thesis chapter three — Editor` is canonically that
/// whole string, because nothing yet shows `— Editor` to be decoration. Witness
/// `thesis appendix draft — Editor` later and the evidence arrives: `— Editor`
/// appears under two stems, so it is decoration, and the first resource's canonical
/// name folds to `thesis chapter three`. The conclusion improved and the key moved
/// with it, taking the Workspace's identity along.
///
/// A witnessed name has no such property. It is the exact string Evo saw at the
/// earliest moment it saw this thing at all, and appending to an append-only
/// history cannot change what came earliest. So identity is derived from the fact,
/// not from the interpretation of the fact — which is also the more honest place
/// for it to come from.
///
/// # Identity is derivation, not meaning
///
/// W-3 forbids Workspace Identity from encoding semantics, and a hash of a string
/// decodes to nothing. Nothing reads a Workspace's meaning out of its identity, and
/// the four channels through which a person states intent are all keyed by subject,
/// so identity changing could never discard what they said.
fn founding_sighting<'a>(
    engagement: &'a Engagement,
    identity: &'a IdentityIndex,
) -> Option<&'a str> {
    /// The earliest-witnessed of a set of participants: least `first_seen`, ties
    /// broken on canonical subject order so the choice is deterministic. A member
    /// with no witnessed time cannot have founded anything, so it sorts last.
    fn earliest<'p>(
        candidates: impl Iterator<Item = &'p Participant>,
    ) -> Option<&'p Participant> {
        candidates.min_by(|left, right| match (left.first_seen(), right.first_seen()) {
            (Some(left_at), Some(right_at)) => left_at
                .cmp(&right_at)
                .then_with(|| left.subject().cmp(right.subject())),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.subject().cmp(right.subject()),
        })
    }

    let founder = earliest(engagement.seeds())
        .or_else(|| earliest(engagement.participants().iter()))
        .map(Participant::subject)?;

    // A founder known only from a declaration has no founding sighting; its
    // canonical name is then all there is, and it is also all that can change.
    Some(identity.founding_name(founder).unwrap_or(founder))
}

#[cfg(test)]
mod tests {
    use super::*;

    use evo_engagement::{EngagementParams, Reconstruction, ResourceRole};
    use evo_observation::observation::Observation;
    use evo_observation::observation_id::ObservationId;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::provenance::{ObservationSource, Provenance};

    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn observe(concept: ObservationConcept, secs: u64) -> Observation {
        let source = ObservationSource::new("projection-test").expect("a valid source");
        Observation::new(
            ObservationId::new(),
            concept.schema(),
            Provenance::new(source, at(secs), HashMap::new()),
            concept.evidence(),
        )
    }

    fn focus(subject: &str, secs: u64) -> Observation {
        observe(
            ObservationConcept::WindowFocusGained {
                subject: subject.to_string(),
            },
            secs,
        )
    }

    fn saved(subject: &str, secs: u64) -> Observation {
        observe(
            ObservationConcept::FileSaved {
                subject: subject.to_string(),
            },
            secs,
        )
    }

    /// Every witnessed subject gets an Artifact, as the daemon would provide.
    fn artifacts_for(observations: &[Observation]) -> SubjectArtifacts {
        let (acts, _) = evo_engagement::interpret(observations);
        let reconstruction =
            Reconstruction::from_observations(observations, EngagementParams::default());
        let mut artifacts = SubjectArtifacts::new();
        for act in &acts {
            let witnessed = act.resource().subject();
            let canonical = reconstruction.identity().canonical(witnessed);
            artifacts
                .entry(canonical.to_string())
                .or_insert_with(|| ArtifactId::new(canonical).expect("a valid artifact id"));
        }
        artifacts
    }

    /// A coherent task across a document, a file and a terminal, worked in three
    /// sittings.
    fn coherent_history() -> Vec<Observation> {
        let mut observations = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                observations.push(focus("thesis chapter three — Editor", moment));
                moment += 300;
                observations.push(saved("/Users/x/thesis/chapter-three.tex", moment));
                moment += 30;
                observations.push(focus("thesis references — Reader", moment));
                moment += 300;
            }
            moment += 4 * 60 * 60;
        }
        observations
    }

    fn project(observations: &[Observation]) -> Vec<Workspace> {
        let reconstruction =
            Reconstruction::from_observations(observations, EngagementParams::default());
        let artifacts = artifacts_for(observations);
        project_workspaces(&reconstruction, &artifacts)
    }

    #[test]
    fn a_body_of_work_projects_onto_one_workspace() {
        let workspaces = project(&coherent_history());
        assert_eq!(workspaces.len(), 1, "one task, one Workspace");
        assert!(workspaces[0].attachments().len() >= 2);
    }

    /// The headline claim, restated for the standing model: a single resource,
    /// attended repeatedly, is not *work* — but neither is it lost. It projects
    /// to a Workspace held as [`Standing::Remembered`]: kept, findable by name,
    /// and off Home. Nothing about the resource is consulted to reach that
    /// conclusion — it holds for any resource, which is why no exclusion list is
    /// needed. Dropping it, which is what this test used to assert, was the
    /// silent loss the standing model exists to end.
    #[test]
    fn one_resource_attended_many_times_is_remembered_not_work() {
        let mut observations = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..6 {
            for _pass in 0..5 {
                observations.push(focus("Some Single Window", moment));
                moment += 600;
            }
            moment += 4 * 60 * 60;
        }
        let reconstruction =
            Reconstruction::from_observations(&observations, EngagementParams::default());
        let artifacts = artifacts_for(&observations);
        let projected = project_engagements(&reconstruction, &artifacts);
        assert!(
            !projected.is_empty(),
            "a lone resource is kept, not dropped — silent loss is the failure, not the fix"
        );
        assert!(
            projected
                .iter()
                .all(|(engagement, _)| !engagement.standing().is_work()),
            "a resource attended with nothing else is Remembered, never presented as work"
        );
    }

    /// Attachments carry both claims separately: belonging and importance.
    #[test]
    fn attachments_carry_role_and_belonging_independently() {
        let workspaces = project(&coherent_history());
        let workspace = &workspaces[0];
        assert!(
            workspace
                .attachments()
                .iter()
                .any(|attachment| attachment.role() == ResourceRole::Primary),
            "a body of work has somewhere to resume"
        );
        for attachment in workspace.attachments() {
            let confidence = attachment.confidence().value();
            assert!(
                (0.0..=1.0).contains(&confidence),
                "confidence stays a normalized evidential strength"
            );
        }
    }

    /// §12: Restoration opens the continuation point, not the whole set.
    #[test]
    fn fewer_attachments_open_on_restore_than_are_attached() {
        let mut observations = coherent_history();
        // A pile of things consulted once each, during the work.
        for (index, moment) in (0..12).map(|index| (index, 200 + index * 40)) {
            observations.push(focus(
                &format!("thesis lookup {index} — Reader"),
                moment as u64,
            ));
        }
        let workspaces = project(&observations);
        let workspace = workspaces
            .iter()
            .max_by_key(|workspace| workspace.attachments().len())
            .expect("the coherent task is still present");
        let opening = workspace
            .attachments()
            .iter()
            .filter(|attachment| attachment.opens_on_restore())
            .count();
        assert!(opening >= 1, "there is always somewhere to resume");
        assert!(
            opening < workspace.attachments().len(),
            "restoring must not reopen everything that was attached"
        );
    }

    /// Snapshot History is the work's own history: one understanding per sitting.
    #[test]
    fn snapshot_history_has_one_entry_per_sitting() {
        let observations = coherent_history();
        let reconstruction =
            Reconstruction::from_observations(&observations, EngagementParams::default());
        let artifacts = artifacts_for(&observations);
        let engagement = &reconstruction.engagements()[0];
        let workspace = project_workspace(
            engagement,
            reconstruction.episodes(),
            reconstruction.identity(),
            &artifacts,
        )
        .expect("the task projects");
        assert_eq!(
            workspace.snapshots().len(),
            engagement.episodes().len(),
            "the Snapshot History records how the work unfolded"
        );
        assert!(
            engagement.episodes().len() > 1,
            "this fixture is worked across several sittings"
        );
    }

    #[test]
    fn snapshot_history_is_chronologically_ordered() {
        let workspaces = project(&coherent_history());
        for workspace in &workspaces {
            for pair in workspace.snapshots().windows(2) {
                assert!(
                    pair[0].captured_at() <= pair[1].captured_at(),
                    "append-only history stays ordered (W-10)"
                );
            }
        }
    }

    /// §26 / test K: the same history must project to the same identity, or
    /// nothing survives a restart.
    #[test]
    fn identity_is_stable_across_recomputation() {
        let observations = coherent_history();
        let first = project(&observations);
        let second = project(&observations);
        assert_eq!(
            first.iter().map(Workspace::id).collect::<Vec<_>>(),
            second.iter().map(Workspace::id).collect::<Vec<_>>()
        );
    }

    /// Identity must also survive the work *growing* — which is the case that
    /// caught the original derivation out.
    ///
    /// Adding `thesis appendix draft — Editor` teaches the identity layer that
    /// `— Editor` is decoration, so the founding member's canonical name folds
    /// from `thesis chapter three — Editor` to `thesis chapter three`. Identity
    /// keyed on that canonical name moved; identity keyed on the founding
    /// *sighting* does not.
    #[test]
    fn identity_survives_the_work_acquiring_new_resources() {
        let observations = coherent_history();
        let before = project(&observations);

        let mut extended = observations.clone();
        let mut moment = 20_000u64;
        for _pass in 0..3 {
            extended.push(focus("thesis chapter three — Editor", moment));
            moment += 300;
            extended.push(focus("thesis appendix draft — Editor", moment));
            moment += 300;
        }
        let after = project(&extended);

        let before_id = before[0].id().clone();
        assert!(
            after.iter().any(|workspace| workspace.id() == &before_id),
            "a body of work that grew is still the same body of work"
        );
    }

    #[test]
    fn a_body_of_work_with_no_artifacts_projects_to_nothing() {
        let reconstruction =
            Reconstruction::from_observations(&coherent_history(), EngagementParams::default());
        let empty = SubjectArtifacts::new();
        assert!(
            project_workspaces(&reconstruction, &empty).is_empty(),
            "a Workspace with nothing to restore is worse than no Workspace"
        );
    }

    /// Two members folding to one Artifact must not attach twice.
    #[test]
    fn one_artifact_attaches_once() {
        let observations = coherent_history();
        let reconstruction =
            Reconstruction::from_observations(&observations, EngagementParams::default());
        let single = ArtifactId::new("one-artifact-for-everything").expect("valid");
        let artifacts: SubjectArtifacts = reconstruction.engagements()[0]
            .subjects()
            .into_iter()
            .map(|subject| (subject, single.clone()))
            .collect();
        let workspace = project_workspace(
            &reconstruction.engagements()[0],
            reconstruction.episodes(),
            reconstruction.identity(),
            &artifacts,
        )
        .expect("one attachment is still a projection");
        assert_eq!(workspace.attachments().len(), 1);
        for snapshot in workspace.snapshots() {
            assert_eq!(snapshot.attachments().len(), 1);
        }
    }

    /// Every projected Workspace satisfies the invariants the crate enforces.
    #[test]
    fn projected_workspaces_are_valid() {
        for workspace in project(&coherent_history()) {
            assert!(workspace_is_valid(&workspace));
        }
    }

    #[test]
    fn an_empty_history_projects_to_nothing() {
        assert!(project(&[]).is_empty());
    }
}
