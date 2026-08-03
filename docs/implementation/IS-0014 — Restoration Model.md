IS-0014 — Restoration Model

Status: Frozen

⸻

1. Purpose

This specification defines the canonical computational object produced by the Restoration layer.

The Restoration layer transforms canonical Workspace understanding into a canonical Restoration Plan that enables a user to continue previously interrupted work with minimal cognitive reload.

The Restoration layer SHALL compute restoration understanding.

It SHALL NOT execute restoration.

⸻

2. Scope

This specification defines:

* Restoration Plan
* Resume Point
* Context Chain
* Blockers
* Next Step
* Restoration invariants
* Restoration replayability

This specification does NOT define:

* operating-system automation
* application launching
* browser tab restoration
* window positioning
* desktop interaction
* user interface
* voice interface
* replay triggering
* restoration scheduling
* restoration algorithms

⸻

3. Definitions

Restoration

The deterministic computation of a continuation strategy from canonical Workspace understanding.

⸻

Restoration Plan

The canonical computational object produced by the Restoration layer.

A Restoration Plan represents one coherent strategy for resuming one Workspace.

⸻

Resume Point

The canonical cognitive entry point into a Workspace.

A Resume Point answers:

Where should the user continue thinking?

⸻

Context Chain

The ordered supporting context required to understand the Resume Point.

⸻

Blocker

An unresolved condition preventing immediate continuation.

⸻

Next Step

The immediate action that naturally follows the Resume Point.

⸻

4. Restoration Plan

A Restoration Plan SHALL:

* correspond to exactly one Workspace;
* contain exactly one Resume Point;
* contain one ordered Context Chain;
* contain zero or more Blockers;
* contain exactly one Next Step.

A Restoration Plan is immutable after construction.

⸻

RP-1

A Restoration Plan SHALL be derived exclusively from canonical lower-layer computational objects.

⸻

RP-2

A Restoration Plan SHALL preserve Workspace identity.

WorkspaceId SHALL NOT change during Restoration.

⸻

RP-3

A Restoration Plan SHALL NOT modify Workspace understanding.

⸻

RP-4

Construction SHALL be deterministic.

Given identical canonical inputs, identical Restoration rules SHALL produce identical Restoration Plans.

⸻

5. Resume Point

Resume Point represents the first cognitive step required to continue work.

Resume Point is not:

* an application;
* a window;
* a browser tab;
* a monitor layout;
* an operating-system action.

Resume Point represents understanding rather than interface state.

⸻

RSP-1

Exactly one Resume Point SHALL exist.

⸻

RSP-2

Resume Point SHALL belong to exactly one Workspace.

⸻

RSP-3

Resume Point SHALL reference canonical Artifacts only.

⸻

RSP-4

Resume Point SHALL NOT reference raw Observations.

⸻

6. Context Chain

Context Chain contains the minimum supporting context necessary to understand the Resume Point.

Context Chain SHALL be ordered.

Context Chain SHALL NOT contain unrelated Artifacts.

Context Chain SHALL NOT duplicate Artifacts.

⸻

CC-1

Ordering SHALL be deterministic.

⸻

CC-2

Artifacts SHALL appear at most once.

⸻

CC-3

Every Artifact SHALL belong to the same Workspace.

⸻

CC-4

Context Chain SHALL minimize cognitive reload rather than maximize historical completeness.

⸻

7. Blockers

A Blocker represents unresolved work preventing immediate continuation.

Examples include:

* compilation failures;
* merge conflicts;
* failing tests;
* unresolved dependencies;
* incomplete work.

Blockers are descriptive.

They SHALL NOT prescribe solutions.

⸻

BL-1

Zero or more Blockers MAY exist.

⸻

BL-2

Blockers SHALL originate from canonical Workspace understanding.

⸻

BL-3

Restoration SHALL NOT invent Blockers.

⸻

8. Next Step

Next Step represents the immediate continuation following the Resume Point.

Exactly one Next Step SHALL exist.

⸻

NS-1

Next Step SHALL belong to the same Workspace.

⸻

NS-2

Next Step SHALL immediately follow the Resume Point.

⸻

NS-3

Next Step SHALL NOT depend on implementation-specific execution mechanisms.

⸻

9. Restoration Invariants

RM-1

Restoration SHALL consume canonical Workspace understanding only.

⸻

RM-2

Restoration SHALL NOT modify Observation history.

⸻

RM-3

Restoration SHALL NOT modify Artifact Identity.

⸻

RM-4

Restoration SHALL NOT modify Workspace understanding.

⸻

RM-5

Restoration SHALL compute.

It SHALL NOT execute.

⸻

RM-6

Restoration SHALL preserve replayability.

⸻

RM-7

Restoration SHALL minimize cognitive reload.

This is the primary optimization objective of the Restoration layer.

⸻

RM-8

One Workspace SHALL produce one Restoration Plan.

⸻

10. Relationship to Replay

Workspace Replay reconstructs Workspace understanding.

Restoration consumes Workspace understanding.

Replay SHALL precede Restoration.

Restoration SHALL NOT invoke Replay.

⸻

11. Relationship to Future Execution

Execution consumes Restoration Plans.

Execution MAY:

* launch applications;
* restore browser tabs;
* restore windows;
* focus documents;
* interact with operating systems.

The Restoration layer SHALL NOT perform any execution.

Execution is outside the scope of this specification.

⸻

12. Architectural Rationale

The purpose of Restoration is not to recreate a desktop.

The purpose of Restoration is to recreate continuity of thought.

Users do not ask Evo to restore windows.

Users ask:

“Hey Evo, continue my work.”

A Restoration Plan is therefore a computation describing how work should be resumed, independent of how any particular platform chooses to present or execute that continuation.

By separating Restoration from execution, Evo remains deterministic, replayable, platform-independent, and capable of producing identical restoration understanding from identical canonical computational history.

⸻

Dependencies

Depends on

* Constitution
* Product
* Architecture
* RFC-0001 — Observation Contract
* RFC-0002 — Artifact Identity Contract
* RFC-0003 — Workspace Contract
* RFC-0004 — Interpretation Evolution Contract
* RFC-0006 — Restoration Contract
* IS-0004 — Artifact Model
* IS-0011 — Workspace Model
* IS-0012 — Workspace Formation
* IS-0013 — Workspace Replay

⸻

Referenced by (future)

* Execution Layer
* Desktop Automation
* Voice Interface
* UI Presentation Layer