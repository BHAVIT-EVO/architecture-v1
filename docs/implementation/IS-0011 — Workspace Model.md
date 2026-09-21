IS-0011 — Workspace Model

Status: Frozen, with Amendment 1

Depends On:

* Constitution
* Product
* Architecture
* RFC-0000 — Computational Model
* RFC-0002 — Artifact
* RFC-0003 — Workspace (v2.0)
* RFC-0014 — Engagement Contract
* IS-0004 — Artifact Model
* IS-0007 — Artifact Identity Lifecycle
* IS-0008 — Artifact Replay
* IS-0009 — Artifact Engine
* IS-0010 — Identity Derivation

⸻

Amendment Record

Amendment 1 — introduced by RFC-0014 (Engagement Contract), consequential to
Architecture Amendment 1 and RFC-0003 v2.0.

Most of this specification survives unchanged, including every rule about what an
Attachment may not imply and what a Snapshot may not contain. Four rules change.

A1.1 — A Workspace is not persistent.

Old rule (§3, Definitions, Workspace): a Workspace is listed as "persistent".

Why it prevented the product from working: persistence obliges the Workspace to
be created at a moment, and the only available moment is the first sighting of a
resource — before any relational evidence exists. The row then outlives every
better reading of the evidence. See the Architecture Amendment Record for the
full mechanism.

New rule: a Workspace is *derived on read* and never written. The property
"persistent" is replaced by "projected".

A1.2 — A Snapshot is derived per sitting, not created on a trigger.

Old rule (§3 and §5): a Snapshot is a canonical historical record created at a
point in time, immutable once written, ordered by its creation point.

Why it changed: a trigger-fired record makes restoration state depend on when
the process happened to be running rather than on what was witnessed, and it
cannot be reproduced by replay because the triggers are not in the log.

New rule: exactly one Snapshot is derived per Episode (sitting) of the
Engagement, directly from that sitting's evidence. Snapshot ordering follows the
sitting's position in the evidence rather than a separate creation point.
Immutability is preserved and is now structural: a derived Snapshot cannot be
modified after creation because it is not stored at all — the next derivation
produces it again from the same log.

A1.3 — An Attachment carries a role.

Old rule: an Attachment carries an Artifact reference and a confidence score.

Why it changed: restoration must be selective, which requires knowing which
members matter to resuming. With no role on the Attachment, the only available
selection rule was "all members".

New rule: every Attachment carries exactly one role — Continuation, Primary,
Supporting, Reference, or Context (RFC-0014 Requirement 8). Role is importance
*to resuming* and is distinct from confidence, which remains strength of
membership. Both are present; neither substitutes for the other.

A1.4 — Workspace Identity derives from the founding sighting.

Old rule: Workspace Identity is established when the Workspace is created and
persists independently thereafter.

Why it changed: with nothing stored, identity has to be a function of the
evidence, or it would differ between two derivations of the same log and break
replay equivalence.

New rule: Workspace Identity is derived from the founding sighting of the
Engagement — the earliest evidence that grounds it — so the same log always
yields the same identity. Requirement 4 of RFC-0003 (identity independent of any
individual supporting Artifact) is preserved: the founding sighting grounds the
identity, and adding or removing other members does not change it.

Retained in force, unchanged: a Workspace is never directly observed; an
Attachment implies no ownership and is evidence only; a Snapshot contains no raw
Observations, no runtime state, no user interface state, no restoration
instructions, no execution instructions, and no inferred user intent. A Workspace
still has no name of its own — a name is a presentation-time claim about the
Engagement (RFC-0014 Requirement 9), not a field of the model.

Implementation: `evo-workspace` (model), `evo-workspace::projection`
(Engagement → Workspace), `evo-engagement` (roles and identity).

⸻

1. Purpose

This specification defines the canonical Workspace computational object.

A Workspace represents Evo’s current best explanation that a collection of Artifact histories collectively describe the evolution of one coherent body of work.

This specification defines:

* the Workspace domain model;
* Workspace identity;
* Workspace ownership;
* Workspace invariants;
* Workspace boundaries;
* Workspace relationships.

This specification does not define:

* Workspace Formation;
* Workspace Replay;
* Workspace Restoration;
* Workspace Retrieval;
* Workspace Learning;
* Workspace persistence;
* Workspace ranking.

⸻

2. Scope

This specification governs only the canonical Workspace domain object.

It defines:

* what a Workspace is;
* what a Workspace owns;
* what a Workspace references;
* what a Workspace SHALL preserve;
* what a Workspace SHALL NOT perform.

This specification does not define algorithms.

This specification does not define formation logic.

This specification does not define interpretation evolution.

⸻

3. Definitions

Workspace

A Workspace is Evo’s current best explanatory hypothesis that multiple Artifact histories collectively describe one coherent body of work.

A Workspace is:

* computational;
* derived;
* replayable;
* projected on read, never written (Amendment A1.1; superseded property: "persistent");
* accountable to canonical evidence.

A Workspace is never directly observed.

⸻

Attachment

An Attachment represents evidential membership between an Artifact and a Workspace.

An Attachment SHALL NOT imply ownership.

An Attachment SHALL remain evidence only.

⸻

Snapshot

A Snapshot represents one immutable historical understanding of a Workspace at a particular point in the Workspace's evolution.

A Snapshot is a canonical historical record of the Workspace understanding produced by Workspace Formation.

A Snapshot SHALL preserve:

* the Workspace Identity to which the understanding belongs;
* the canonical Attachment set representing the Workspace understanding at the time of Snapshot creation;
* the canonical ordering of those Attachments;
* the point at which the Snapshot was created.

A Snapshot SHALL reference canonical computational objects only.

A Snapshot SHALL NOT create a second Artifact Identity.

A Snapshot SHALL NOT create a second Workspace Identity.

A Snapshot SHALL NOT modify Artifacts.

A Snapshot SHALL NOT modify Attachments.

A Snapshot SHALL NOT contain raw Observations.

A Snapshot SHALL NOT contain runtime state.

A Snapshot SHALL NOT contain user interface state.

A Snapshot SHALL NOT contain restoration instructions.

A Snapshot SHALL NOT contain execution instructions.

A Snapshot SHALL NOT contain inferred user intent.

A Snapshot SHALL NOT contain semantic interpretation that is not already represented by canonical lower-layer Workspace understanding.

Snapshots SHALL NEVER be modified after creation.

⸻

Snapshot Model

Every canonical Snapshot SHALL contain exactly:

* one Workspace Identity;
* one Snapshot Identity;
* one Snapshot Creation Point;
* one ordered Attachment Set.

Snapshot Identity

Every Snapshot SHALL possess exactly one canonical Snapshot Identity.

Snapshot Identity SHALL:

* remain immutable;
* be unique within the Workspace;
* remain computationally meaningful only as a reference;
* NOT encode semantic meaning;
* NOT encode user intent;
* NOT encode Artifact Identity;
* NOT encode Workspace meaning.

Snapshot Creation Point

Every Snapshot SHALL possess exactly one canonical creation point.

The creation point SHALL establish the position of the Snapshot within the Workspace's Snapshot History.

The creation point SHALL be sufficient to deterministically order Snapshots.

Attachment Set

The Snapshot SHALL contain the immutable Attachment records representing the Workspace understanding at the time the Snapshot was created.

The Attachment Set SHALL preserve:

* Attachment Identity;
* referenced Artifact Identity;
* Attachment Confidence;
* canonical Attachment ordering.

Snapshot Construction SHALL NOT create or modify Attachments.

The Snapshot SHALL preserve the Attachment records produced by Workspace Formation.

Historical snapshots SHALL preserve their original Attachment Set even when later Workspace understanding evolves.

Snapshot Completeness

A Snapshot SHALL represent the complete canonical Workspace understanding produced by the corresponding Workspace Formation operation.

A partial Snapshot SHALL NOT constitute canonical Workspace understanding.

Snapshot Determinism

Given identical:

* canonical Workspace Formation inputs;
* canonical Attachment evaluations;
* canonical Workspace Formation rules;

Snapshot Construction SHALL produce equivalent Snapshot contents.

Snapshot Historical Integrity

A Snapshot SHALL remain permanently associated with the Workspace Identity under which it was created.

A later Snapshot SHALL NOT modify, replace, or reinterpret an earlier Snapshot.

Workspace evolution SHALL occur through creation of new Snapshots or new Workspaces according to the existing Workspace and Replay contracts.

⸻

Workspace Identity

Workspace Identity is the stable computational identity assigned to a Workspace.

Workspace Identity SHALL remain stable throughout the Workspace lifetime.

Workspace Identity SHALL NOT encode meaning.

Workspace Identity SHALL NOT encode user intent.

Workspace Identity SHALL NOT encode semantic interpretation.

⸻

4. Workspace Model

A canonical Workspace SHALL consist of exactly the following conceptual components:

* Workspace Identity
* Workspace Lifecycle
* Attachment Set
* Snapshot History

No additional canonical components exist.

Higher computational layers MAY derive additional views from a Workspace.

Those derived views SHALL NOT become part of the canonical Workspace model.

⸻

Workspace Identity

Every Workspace SHALL possess exactly one canonical Workspace Identity.

Workspace Identity SHALL:

* remain immutable;
* remain globally unique;
* remain computationally stable;
* remain semantically meaningless.

Workspace Identity SHALL NEVER change because Workspace understanding changes.

⸻

Workspace Lifecycle

Every Workspace SHALL possess exactly one Lifecycle State.

Lifecycle State describes the current computational state of the Workspace.

Lifecycle SHALL belong to the Workspace.

Lifecycle SHALL NOT belong to Attachments.

Lifecycle SHALL NOT belong to Snapshots.

The canonical lifecycle states are defined by the Workspace Model:

* Active — the Workspace represents the current canonical understanding of a body of work.
* Superseded — the Workspace has been replaced by a newer canonical understanding through replay or interpretation evolution, but remains preserved for historical accountability.

No additional lifecycle states exist in the canonical model.

⸻

Attachment Set

Every Workspace SHALL own exactly one Attachment Set.

The Attachment Set represents evidential relationships between the Workspace and Artifacts.

Attachments SHALL:

* reference Artifacts;
* carry confidence;
* remain immutable after creation.

Attachments SHALL NOT:

* own Artifacts;
* modify Artifacts;
* merge Artifacts;
* redefine Artifact Identity.

Attachments exist solely to express Workspace membership evidence.

⸻

Snapshot History

Every Workspace SHALL own exactly one Snapshot History.

Snapshot History preserves historical Workspace understanding.

Snapshots SHALL be:

* immutable;
* ordered chronologically;
* append-only.

Historical Snapshots SHALL NEVER be modified.

Replay SHALL create new Snapshots.

Replay SHALL NEVER overwrite existing Snapshots.

⸻
Great. Here’s the second half. I kept the same style and rigor as IS-0004/IS-0005.

⸻

5. Canonical Properties

Every canonical Workspace SHALL satisfy the following properties.

⸻

Persistence

A Workspace is a persisted computational object.

Persistence exists solely as an implementation optimization.

Workspace persistence SHALL NEVER alter Workspace semantics.

Workspace persistence SHALL NEVER become the source of truth.

The source of truth remains the canonical Observation history and the replay rules defined by the architecture.

⸻

Replayability

Every Workspace SHALL be replayable.

Replay SHALL reconstruct Workspace understanding exclusively from canonical lower-layer computational objects.

Replay SHALL NOT require historical Workspace persistence.

Replay SHALL produce identical Workspace understanding given:

* identical Observation history;
* identical Artifact history;
* identical derivation rules.

⸻

Accountability

Every Workspace SHALL remain permanently accountable to the Artifact histories from which it was derived.

No Workspace interpretation SHALL exist without supporting Artifact evidence.

Workspace evidence SHALL remain traceable throughout replay.

⸻

Immutability

The canonical Workspace object SHALL be immutable after creation.

Evolution of Workspace understanding SHALL occur through:

* new Snapshots;
* new Workspaces;
* replay.

Workspace mutation in place is prohibited.

⸻

6. Required Invariants

The following invariants SHALL hold for every canonical Workspace.

⸻

W-1

Every Workspace SHALL possess exactly one Workspace Identity.

⸻

W-2

Workspace Identity SHALL remain stable throughout the Workspace lifetime.

⸻

W-3

Workspace Identity SHALL NEVER encode semantic meaning.

⸻

W-4

Workspace SHALL own Attachments.

Workspace SHALL NOT own Artifacts.

⸻

W-5

Every Attachment SHALL reference exactly one Artifact.

⸻

W-6

Every Attachment SHALL contain exactly one Confidence Score.

⸻

W-7

Confidence SHALL represent evidential strength only.

Confidence SHALL NEVER represent importance, priority, or value.

⸻

W-8

Every Workspace SHALL own a Snapshot History.

⸻

W-9

Snapshots SHALL be immutable.

⸻

W-10

Snapshot History SHALL be append-only.

⸻

W-11

Historical Snapshots SHALL NEVER be modified.

⸻

W-12

Workspace understanding SHALL evolve through replay rather than mutation.

⸻

W-13

Every Workspace SHALL remain accountable to Artifact history.

⸻

W-14

Workspace SHALL remain independent of Retrieval.

⸻

W-15

Workspace SHALL remain independent of Restoration.

⸻

W-16

Workspace SHALL remain independent of Learning.

⸻

W-17

Workspace SHALL remain independent of Knowledge.

⸻

W-18

Workspace SHALL remain replayable.

⸻

W-19

A Workspace SHALL NEVER exist independently of supporting Artifact evidence. If all supporting Artifact evidence is removed through replay, the Workspace SHALL cease to exist in the replayed interpretation.

⸻

W-20

Every Snapshot SHALL possess exactly one Workspace Identity.

____


W-21

Every Snapshot SHALL possess exactly one Snapshot Identity.

____


W-22

Every Snapshot SHALL possess exactly one Snapshot Creation Point.

____


W-23

Every Snapshot SHALL contain exactly one ordered Attachment Set.

____


W-24

Snapshot Attachment Sets SHALL contain only canonical immutable Attachments.

____


W-25

Snapshot Construction SHALL NOT modify Artifact Identity.
____

W-26

Snapshot Construction SHALL NOT modify Attachment Identity.

____

W-27

Historical Snapshots SHALL preserve the Attachment Set that existed when they were created.

W-28

____

Snapshots SHALL NOT contain raw Observations or runtime state.

W-29

____

Snapshots SHALL NOT contain RestorationPlan semantics.

W-30

____

Snapshots SHALL NOT contain execution instructions.

W-31

____

Snapshots SHALL remain deterministic and replayable according to the Workspace Formation and Replay contracts.

____

W-32

A later Snapshot SHALL NOT modify or reinterpret an earlier Snapshot.

____

7. Non-Responsibilities

Workspace SHALL NOT perform any of the following responsibilities.

Workspace SHALL NOT:

* derive Artifact Identity;
* modify Observations;
* modify Artifacts;
* infer user intent;
* determine semantic meaning;
* perform Retrieval;
* perform Restoration;
* perform Learning;
* perform ranking;
* perform search;
* perform synchronization;
* execute replay;
* determine notification policy;
* own persistence infrastructure.

These responsibilities belong to higher architectural layers.

⸻

8. Architectural Rationale

The Workspace computational object exists to preserve the architectural separation between:

* evidence;
* identity;
* bodies of work.

Observation answers:

What happened?

Artifact answers:

What external entity was involved?

Workspace answers:

What coherent body of work best explains the evolution of these Artifact histories?

Workspace therefore becomes Evo’s first long-lived interpretation of work.

By making Workspace:

* replayable;
* immutable;
* accountable;
* evidence-based;

Evo ensures that improvements to Workspace understanding never require rewriting historical evidence.

Workspace evolution therefore becomes a consequence of replay rather than mutation.

This preserves the Computational Model defined by RFC-0000 and the Architectural Laws governing replay and interpretation.

⸻

9. Dependencies

This specification depends on:

* Constitution
* Product
* Architecture
* RFC-0000 — Computational Model
* RFC-0002 — Artifact
* RFC-0003 — Workspace
* IS-0004 — Artifact Model
* IS-0007 — Artifact Identity Lifecycle
* IS-0008 — Artifact Replay
* IS-0009 — Artifact Engine
* IS-0010 — Identity Derivation

Implementations SHALL NOT contradict any dependency listed above.

⸻

10. Out of Scope

This specification intentionally does NOT define:

* Workspace Formation
* Workspace Replay algorithms
* Workspace Restoration
* Workspace Retrieval
* Workspace Ranking
* Workspace Learning
* Workspace persistence implementation
* Attachment confidence computation
* Snapshot creation algorithms
* Replay triggering
* Workspace merge algorithms
* Workspace split algorithms

These behaviors belong to future specifications.

⸻

End of Specification