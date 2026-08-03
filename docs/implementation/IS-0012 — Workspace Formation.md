IS-0012 — Workspace Formation

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

This specification deliberately does not define discovery algorithms.

⸻

Stage 2 — Attachment Evaluation

Evaluate the Artifact against every Candidate Workspace.

Requirements:

* every evaluation SHALL produce exactly one ConfidenceScore
* Workspace state SHALL NOT be modified during evaluation

⸻

Stage 3 — Workspace Decision

Exactly one outcome SHALL occur:

* Attach to an existing Workspace

or

* Recognize a new Workspace

No other outcome is permitted.

⸻

Stage 4 — Attachment Construction

Construct immutable Attachment records.

Attachments SHALL be constructed only after Workspace Decision.

Attachments SHALL NOT be modified after construction.

⸻

Stage 5 — Snapshot Construction

Construct an immutable Snapshot representing the current Workspace understanding.

Historical Snapshots SHALL remain unchanged.

Snapshot History SHALL remain append-only.

⸻

Stage 6 — Workspace Construction

Construct canonical Workspace understanding.

Existing Workspace Identity SHALL remain unchanged.

New Workspace Identity SHALL be assigned only when recognizing a previously unseen Workspace.

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