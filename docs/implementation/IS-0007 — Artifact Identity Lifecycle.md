IS-0007 — Artifact Identity Lifecycle

1. Purpose

Defines the complete lifecycle of Artifact Identity after acceptance.

This specification establishes how Artifact Identity is created, maintained, revised, replaced, superseded, merged, split, retired, and replayed while preserving the constitutional principles of Evidence Is Sacred, Interpretation Is Disposable, Replayability, and Epistemic Separation.

This specification governs Artifact Identity only.

It does not define identity inference algorithms.

It does not define similarity scoring.

It does not define Artifact Engine implementation.

⸻

2. Scope

Defines

* Artifact Identity creation
* Artifact Identity revision
* Artifact supersession
* Artifact merge
* Artifact split
* Artifact retirement
* Identity stability
* Identity replay invariants

Does NOT define

* similarity algorithms
* machine learning
* confidence computation
* Observation acceptance
* Workspace attachment
* Knowledge attachment
* Decision making

⸻

3. Definitions

Define

Identity Hypothesis

Current best explanation that one or more canonical Observations describe one External Entity.

⸻

Accepted Artifact

Identity Hypothesis that successfully completed IS-0005.

⸻

Active Artifact

Accepted Artifact currently considered Evo’s best explanation.

⸻

Superseded Artifact

Accepted Artifact replaced by a newer explanation.

Never deleted.

Never modified.

⸻

Retired Artifact

Artifact no longer considered valid.

Still replayable.

Still immutable.

⸻

4. Lifecycle States

Identity Hypothesis
↓
Candidate Artifact
↓
Accepted Artifact
↓
Active Artifact
↓
Superseded
        ↘
         Retired

No backward transitions.

⸻

Allowed Lifecycle Transitions

The following lifecycle transitions are permitted:

* Candidate Artifact → Accepted Artifact
* Accepted Artifact → Active Artifact
* Active Artifact → Superseded Artifact
* Active Artifact → Retired Artifact

No other lifecycle transitions are permitted.

Artifact Identity SHALL NOT revert to an earlier lifecycle state.

⸻

5. Identity Creation

Normative statements.

Exactly one ArtifactId.

Only IS-0005 Stage 3 may create it.

Never reused.

Never changed.

Never regenerated.

⸻

6. Identity Revision

The important section.

Identity may improve.

Identity never mutates.

Instead,

old Artifact

↓

new Artifact

↓

relationship established

Old Artifact remains immutable.

⸻

7. Supersession

Define

Artifact A superseded by Artifact B.

A still exists.

Replay may reconstruct A.

Higher layers always use newest active Artifact unless replay requests historical reconstruction.

⸻

8. Merge

Suppose

Artifact A

Artifact B

actually describe same External Entity.

Never mutate either.

Instead

create

Artifact C

The merged Artifact SHALL receive a new ArtifactId.

Neither predecessor Artifact SHALL be modified.

Relationship

A ----\
       \
        -> C
B ----/

A and B become superseded.

C active.

⸻

9. Split

Suppose

Artifact A

actually represented

two entities.

Never edit A.

Instead

A
↓
B
C

A superseded.

B active.

C active.

Every Artifact produced by a split SHALL receive a newly assigned ArtifactId.

The predecessor Artifact SHALL remain immutable and SHALL become superseded.

⸻

10. Retirement

Artifact may retire when

identity confidence collapses

or

replay determines it no longer valid.

Retirement never deletes.

Retirement never changes history.

⸻

11. Replay Invariants

Replay may

produce

different active Artifact graph.

Replay may

replace identities.

Replay may

merge.

Replay may

split.

Replay may

retire.

Replay may NOT

change Observations.

change ArtifactIds.

rewrite accepted history.

⸻

12. Required Guarantees

Artifact Identity SHALL

remain immutable.

Artifact Identity SHALL

remain replayable.

Artifact Identity SHALL

remain traceable.

Artifact Identity SHALL

remain accountable.

Artifact Identity SHALL

never overwrite evidence.

Artifact Identity SHALL

never be silently merged.

Artifact Identity SHALL

never disappear.

⸻

13. Architectural Invariants

These become the hard rules.

I-1

ArtifactId is permanent.

I-2

Artifact content is immutable.

I-3

Only replay may replace explanations.

I-4

Replacement creates new Artifact.

I-5

Merge creates new Artifact.

I-6

Split creates new Artifacts.

I-7

Observations are never modified.

I-8

Every Artifact remains replayable forever.

I-9

Superseded Artifacts remain queryable.

I-10

Higher layers never mutate Artifact Identity.

⸻

14. Rationale

Tie it back to

Law II

Law III

Law IV

Law VII

Law XVI

Law XVII

⸻

This is the entire lifecycle.

Notice something important.

There is no implementation.

No Rust.

No storage.

No graph.

No database.

Only behavior.

Exactly how an IS should read.

⸻

## Dependencies

This specification depends on the following architectural documents:

- Constitution
- Product
- Architecture
- Architectural Laws
- RFC-0000 — Computational Model
- RFC-0002 — Artifact
- IS-0004 — Artifact Model
- IS-0005 — Artifact Acceptance Pipeline
- IS-0006 — Identity Hypothesis

This specification SHALL NOT contradict any dependency listed above.

Subsequent specifications governing Artifact Replay and the Artifact Engine SHALL conform to this specification.