IS-0012 — Workspace Formation

Status: Superseded in full by RFC-0014 (Engagement Contract)

⸻

Supersession Record

This specification is superseded **in full**. It remains in the repository as the
record of a process that was implemented, shipped, and found to produce the wrong
product. Nothing in it is normative. Implementations MUST NOT follow it.

The old process. Formation was defined per Artifact: for each newly resolved
Artifact, discover a small set of candidate Workspaces, evaluate attachment
against each, and either attach to one or establish a new Workspace. Candidate
discovery was scoped to a small, recent, local set — never the full history
(Architecture §5, also amended).

Why it prevented the product from working. The process asks *does this Artifact
attach to a nearby Workspace*, which presupposes that the Workspace already
exists. At the first sighting of any resource, no Workspace exists and no
relational evidence exists either, so the only question the process could
actually answer was whether the resource had been seen before. Establishing a new
Workspace therefore reduced to recurrence, and everything a computer touches
twice satisfies recurrence. The observable result was one Workspace per resource:
a music player, a chat window, a search, a file the operating system rewrote —
each presented on Home as a body of work to return to. Home mirrored the
observation log.

This was not a defect in the attachment evaluation. Every stage did what this
specification told it to do. The specification asked a question that cannot
distinguish work from activity, and no improvement to the evaluation function
could have changed that, because the necessary evidence — how a resource relates
to *other* resources across a history — was excluded from the process by
construction.

What replaces it. A body of work is derived from relationships among resources
across the whole canonical Observation history, not one Artifact at a time
against a local window:

    Observations → Acts → Episodes → attention → affinity →
    grouping → significance → roles → naming → Workspace projection

A body of work is presented only when three independent conditions all hold —
more than one participant, a return by a person on more than one occasion, and at
least one sitting of sustained attention — none of which can be satisfied by a
single resource considered alone.

Normative authority: **RFC-0014 (Engagement Contract)**, Requirements 1–14.

Implementation contract: `evo-engagement` — its crate-level and module-level
documentation carries the stage-by-stage contract this specification used to hold,
including the evidential justification for every parameter (RFC-0014 Requirement
5). Projection into the Workspace model is `evo-workspace::projection`.

Behavioural coverage: `evo-daemon`'s `verify_scenarios` example, adversarial
scenarios A–O, plus the unit tests of the modules named above.

Dependent documents reconciled: RFC-0003 v2.0 (Amendments 1–4), RFC-0012
(superseded), IS-0011 (Amendment 1), IS-0013 (superseded), Architecture
(Amendment 1, §3–§9 and §12).

⸻

The superseded specification follows, unchanged, for the record.

⸻

Purpose

This specification defines the deterministic process by which canonical Artifact history is transformed into canonical Workspace understanding.

Workspace Formation SHALL recognize whether an Artifact contributes to an existing Workspace or establishes a new Workspace.

Workspace Formation SHALL produce canonical Workspace understanding exclusively from canonical computational primitives.

Workspace Formation SHALL NOT modify Observation history or Artifact Identity.

⸻

Scope

This specification defines:

* Workspace Candidate Discovery
* Attachment Evaluation
* Workspace Decision
* Attachment Construction
* Snapshot Construction
* Workspace Construction
* Workspace Integrity Verification

This specification does NOT define:

* Replay
* Learning
* Restoration
* Retrieval
* Persistence
* Scheduling
* Notification
* Synchronization

⸻

Definitions

Workspace Recognition

The deterministic process of recognizing whether an Artifact contributes to an existing Workspace or establishes a new Workspace.

⸻

Candidate Workspace

An existing Workspace considered during Workspace Recognition.

⸻

Attachment Evaluation

The deterministic evaluation of one Artifact against one Candidate Workspace.

Each evaluation produces exactly one ConfidenceScore.

⸻

Workspace Understanding

The canonical computational understanding represented by:

* Workspace
* Attachment set
* Snapshot history

⸻

Responsibilities

Workspace Formation SHALL:

* recognize Workspace continuity
* recognize Workspace boundaries
* construct Attachments
* construct Snapshots
* construct canonical Workspace understanding

Workspace Formation SHALL NOT:

* collect Observations
* resolve Artifact Identity
* infer semantic meaning
* infer user intent
* restore work
* retrieve Workspaces
* persist state
* learn recognition rules

⸻

Inputs

Workspace Formation SHALL consume only:

* canonical Observation
* canonical Artifact
* existing canonical Workspace understanding

Workspace Formation SHALL NOT consume raw capture events or non-canonical representations.

⸻

Outputs

Workspace Formation SHALL produce only:

* Workspace
* Attachment
* Snapshot

Workspace Formation SHALL NOT introduce intermediate canonical domain objects.

⸻

Formation Pipeline

Stage 1 — Candidate Discovery

Identify the finite candidate set of existing Workspaces that may explain the current Artifact.

Requirements:

* deterministic
* finite
* replayable
* identical inputs SHALL produce identical candidate sets

Workspace Candidate Discovery SHALL deterministically enumerate every existing Workspace eligible for Attachment Evaluation.

Candidate Discovery SHALL:

* produce a finite candidate set;
* remain deterministic;
* remain replayable;
* consume only canonical computational primitives.

The concrete enumeration algorithm remains an implementation detail.

⸻

Stage 2 — Attachment Evaluation

Attachment Evaluation SHALL evaluate the Artifact against every Candidate Workspace.

Every evaluation SHALL produce exactly one Confidence Score as defined by IS-0014.

Attachment Evaluation SHALL:

* remain deterministic;
* remain replayable;
* remain read-only;
* preserve Artifact Identity;
* preserve Workspace Identity.

Attachment Evaluation SHALL NOT:

* modify Workspaces;
* modify Artifacts;
* create Attachments;
* determine the final Workspace Decision.

⸻

Stage 3 — Workspace Decision

Exactly one outcome SHALL occur:

* Attach to an existing Workspace

or

* Recognize a new Workspace

or

* Attach nowhere — no Workspace is created.

No other outcome is permitted.

Workspace Decision SHALL select exactly one outcome from the completed Attachment Evaluations.

Exactly one of the following SHALL occur:

* attach to one existing Workspace;
* recognize one new Workspace;
* attach nowhere — no Workspace is created.

No other outcome is permitted.

Workspace Decision SHALL:

Workspace Decision SHALL compare every completed Attachment Evaluation.

Workspace Decision SHALL select exactly one outcome.

If exactly one Candidate Workspace is determined to best explain the Artifact under the deterministic Workspace Formation rules, the Artifact SHALL attach to that Workspace.

Otherwise, a new Workspace SHALL be recognized only when the Artifact carries witnessed continuity (RFC-0003 Req 5: formation explains the evolution of Artifact histories, never instantaneous state) — a prior content Observation of the same Artifact, canonical co-membership evidence (RFC-0012), or an explicit user declaration naming the Artifact (RFC-0011 WorkDesignated; RFC-0012 WorkGrouped; RFC-0013 ContinuationSurface — sufficient origination evidence when the Artifact belongs to no remembered Workspace). Without witnessed continuity, the Artifact attaches nowhere: the Observation and Artifact remain canonical, but no Workspace is created (ARCHITECTURE §5 third outcome).

Workspace Decision SHALL produce exactly one deterministic decision.

Workspace Decision SHALL remain replayable.

The concrete comparison algorithm remains an implementation detail.

Workspace Decision SHALL NOT:

* modify Artifacts;
* modify Workspaces;
* construct Attachments;
* construct Snapshots.

⸻

Stage 4 — Attachment Construction

Construct immutable Attachment records.

Attachments SHALL be constructed only after Workspace Decision.

Attachments SHALL NOT be modified after construction.

⸻

Stage 5 — Snapshot Construction

Snapshot Construction SHALL construct exactly one immutable Snapshot representing the resulting canonical Workspace understanding.

The Snapshot SHALL contain exactly:

* the resulting Workspace Identity;
* a newly assigned Snapshot Identity;
* the Snapshot Creation Point;
* the complete ordered Attachment Set resulting from Workspace Formation.

Snapshot Construction SHALL:

* preserve historical Snapshots;
* append exactly one new Snapshot;
* preserve the Attachment records produced by Attachment Construction;
* preserve canonical Attachment ordering;
* remain deterministic;
* produce a complete Snapshot;
* produce no additional canonical domain objects.

Snapshot Construction SHALL NOT:

* create Artifacts;
* modify Artifacts;
* create or modify Attachments;
* infer user intent;
* infer semantic meaning;
* perform Restoration;
* derive a Resume Point;
* derive a Context Chain;
* derive Blockers;
* derive a Next Step;
* execute any operating-system action.

Snapshot Construction SHALL treat the Attachment Set produced by Workspace Formation as authoritative for the Snapshot.

The Snapshot SHALL preserve the exact Workspace understanding established by the current Workspace Formation operation.

Historical Snapshots SHALL remain unchanged.

A new Snapshot SHALL NOT modify, replace, or reinterpret any previous Snapshot.

Snapshot Construction SHALL complete before Workspace Construction completes.

A Workspace Formation operation SHALL NOT publish a Workspace whose resulting Snapshot violates the Snapshot Model defined by IS-0011.

⸻

Stage 6 — Workspace Construction

Workspace Construction SHALL construct exactly one canonical Workspace.

The resulting Workspace SHALL consist exclusively of the canonical components defined by IS-0011:

* Workspace Identity
* Workspace Lifecycle
* Attachment Set
* Snapshot History

Existing Workspace Identity SHALL remain unchanged.

New Workspace Identity SHALL be assigned only when recognizing a previously unseen Workspace.

Workspace Identity generation SHALL depend exclusively upon canonical Workspace Formation outputs.

Workspace Identity generation SHALL remain deterministic.

Workspace Identity generation SHALL remain replayable.

Given identical canonical Workspace Formation outputs, identical Workspace Identities SHALL be generated.

The concrete identity generation algorithm remains an implementation detail.

Workspace Construction SHALL NOT introduce additional canonical components.

⸻

Stage 7 — Integrity Verification

Verify that the resulting Workspace satisfies every invariant defined by IS-0011.

Integrity verification SHALL succeed before Workspace Formation completes.

⸻

Determinism

Given:

* identical Observations
* identical Artifacts
* identical Workspace candidates
* identical Formation rules

Workspace Formation SHALL produce identical Workspace understanding.

Implementations SHALL NOT permit nondeterministic Workspace Formation.

⸻

Required Invariants

WF-1

Workspace Formation consumes only canonical computational primitives.

WF-2

Observation history remains immutable.

WF-3

Artifact Identity remains immutable.

WF-4

Attachments are immutable.

WF-5

Snapshots are append-only.

WF-6

Workspace Identity remains stable.

WF-7

Workspace Recognition is deterministic.

WF-8

Workspace understanding remains reproducible from canonical computational primitives.

WF-9

Snapshot Construction produces exactly one complete immutable Snapshot.

WF-10

Every Snapshot produced by Workspace Formation contains the resulting Workspace Identity.

WF-11

Every Snapshot produced by Workspace Formation contains the complete Attachment Set produced by that formation operation.

WF-12

Snapshot Construction does not create or modify Artifacts.

WF-13

Snapshot Construction does not create or modify Attachments.

WF-14

Snapshot Construction does not perform Restoration Derivation.

WF-15

Historical Snapshots remain unchanged.

WF-16

Given identical canonical Workspace Formation inputs and identical Formation rules, Snapshot Construction produces equivalent Snapshot contents.

⸻

Non-Responsibilities

Workspace Formation SHALL NOT:

* infer user intent
* infer semantic meaning
* perform retrieval
* perform restoration
* perform learning
* perform persistence
* perform synchronization
* perform scheduling
* modify Observation history
* modify Artifact Identity
* rewrite historical Snapshots
* invoke language models

⸻

Architectural Rationale

Workspace Formation transforms Artifact continuity into Workspace continuity.

It does not determine meaning.

It determines the persistent computational structure of work.

By separating Workspace Formation from Replay, Restoration, Learning, Retrieval, and Persistence, Workspace understanding remains deterministic, reproducible, and accountable to canonical computational primitives alone.

⸻

Dependencies

Depends on:

* Constitution
* Product
* Architecture
* RFC-0001 — Observation Contract
* RFC-0002 — Artifact Identity Contract
* RFC-0003 — Workspace Contract
* RFC-0004 — Interpretation Evolution Contract
* IS-0001 — Observation Model
* IS-0004 — Artifact Model
* IS-0011 — Workspace Model

Referenced by (future):

* IS-0013 — Workspace Replay
* Workspace Engine implementation
* evo-replay
* evo-restoration