IS-0016 — Historical Understanding Model

Status: Proposed

Version: 1.0

Depends On

* Constitution
* Cognitive Model
* Architecture
* RFC-0005
* IS-0011 (Workspace)
* IS-0014 (Restoration)

⸻

1. Purpose

This specification defines the canonical computational model for Historical Understanding.

Historical Understanding is the immutable record of the committed understanding that justified an architectural action.

It exists solely to preserve historical accountability.

This specification defines the computational structure required to satisfy RFC-0005.

⸻

2. Scope

This specification defines:

* the canonical HistoricalUnderstanding object;
* its identity;
* its components;
* construction invariants;
* immutability;
* public computational surface.

This specification does not define:

* decision execution;
* replay;
* restoration;
* persistence;
* storage;
* retrieval;
* learning;
* scheduling.

⸻

3. Design Goal

Historical Understanding answers one question only:

“What understanding actually justified this architectural behaviour?”

It is not responsible for reproducing behaviour.

It is not responsible for deciding behaviour.

It simply preserves the committed understanding exactly as it existed.

⸻

4. Canonical Components

A HistoricalUnderstanding SHALL consist of exactly:

HistoryId
WorkspaceId
RestorationPlan

Nothing else belongs to the canonical object.

⸻

Why only these?

The RestorationPlan already represents the complete understanding required to continue work.

It already contains:

* ResumePoint
* ContextChain
* Blockers
* NextStep

RFC-0005 never says Historical Understanding stores a second copy of understanding.

It stores the committed understanding.

The committed understanding is already represented by RestorationPlan.

Duplicating those fields would violate the Architecture’s preference for single canonical representations.

⸻

5. Identity

Every HistoricalUnderstanding owns exactly one immutable HistoryId.

HistoryId SHALL

* never change
* never be reused
* uniquely identify one committed understanding

Replay SHALL create a new HistoricalUnderstanding.

Replay SHALL NEVER reuse an existing HistoryId.

⸻

6. Workspace Reference

HistoricalUnderstanding SHALL reference exactly one Workspace.

It SHALL reference only

WorkspaceId

It SHALL NEVER own a Workspace.

It SHALL NEVER duplicate Workspace state.

Historical Understanding records understanding.

Workspace remains the canonical interpretation.

⸻

7. Restoration Plan

HistoricalUnderstanding SHALL own exactly one immutable RestorationPlan.

That RestorationPlan SHALL be frozen permanently.

HistoricalUnderstanding SHALL NEVER modify it.

Replay SHALL construct a completely new HistoricalUnderstanding rather than replacing the RestorationPlan.

⸻

8. Immutability

After construction

HistoryId

WorkspaceId

RestorationPlan

shall all be immutable.

No mutating APIs shall exist.

⸻

9. Replay

Replay SHALL NOT modify HistoricalUnderstanding.

Replay SHALL create

Current Workspace
↓
New RestorationPlan
↓
New HistoricalUnderstanding

The previous HistoricalUnderstanding remains permanently unchanged.

⸻

10. Separation

HistoricalUnderstanding SHALL NEVER own

Observation

Artifact

Knowledge

Workspace

Decision

Learning

Replay state

Current interpretation

It references only

WorkspaceId

and owns only

RestorationPlan.

⸻

11. Invariants

H-1

Every HistoricalUnderstanding has exactly one HistoryId.

⸻

H-2

HistoryId never changes.

⸻

H-3

Every HistoricalUnderstanding references exactly one WorkspaceId.

⸻

H-4

HistoricalUnderstanding owns exactly one RestorationPlan.

⸻

H-5

RestorationPlan is immutable after construction.

⸻

H-6

HistoricalUnderstanding is immutable after construction.

⸻

H-7

Replay never modifies HistoricalUnderstanding.

⸻

H-8

Replay creates a new HistoricalUnderstanding.

⸻

H-9

HistoricalUnderstanding never owns Workspace.

⸻

H-10

HistoricalUnderstanding never owns Observation.

⸻

H-11

HistoricalUnderstanding never owns Artifact.

⸻

H-12

HistoricalUnderstanding never owns Knowledge.

⸻

H-13

HistoricalUnderstanding never duplicates RestorationPlan components.

⸻

H-14

HistoricalUnderstanding SHALL preserve the RestorationPlan exactly as committed. 
It SHALL NEVER modify, replace, or partially reconstruct that RestorationPlan after construction.

⸻

12. Public Computational Surface

HistoryId
HistoricalUnderstanding
HistoricalError

No additional public types are required.

⸻

13. HistoricalUnderstanding

Public API:

pub struct HistoricalUnderstanding
impl HistoricalUnderstanding {
    pub fn new(
        id: HistoryId,
        workspace_id: WorkspaceId,
        restoration_plan: RestorationPlan,
    ) -> Self;
    pub fn id(&self) -> &HistoryId;
    pub fn workspace_id(&self) -> &WorkspaceId;
    pub fn restoration_plan(&self) -> &RestorationPlan;
}

No setters.

No mutation.

⸻

14. HistoryId

pub struct HistoryId

Requirements

* immutable
* unique
* cloneable
* hashable
* comparable

Construction

HistoryId::new()

⸻

15. HistoricalError

Initially this specification requires no construction failures.

Accordingly

pub enum HistoricalError {}

(or the crate may omit an error type entirely if no fallible constructors exist).

This mirrors the approach taken in earlier IS documents: do not invent errors where no invariant can be violated.

⸻

16. Dependencies

The crate depends only on:

evo-workspace
evo-restoration

No dependency on

* evo-observation
* evo-artifact
* evo-knowledge
* evo-replay

is required.