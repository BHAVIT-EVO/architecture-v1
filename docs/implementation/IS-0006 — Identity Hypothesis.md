IS-0006 — Identity Hypothesis

⸻

Purpose

An Identity Hypothesis represents the Artifact Engine’s transient computational belief that one or more canonical Observations describe the same underlying Artifact.

It exists solely to support Artifact Acceptance.

An Identity Hypothesis is neither evidence nor a canonical Artifact.

It is an intermediate computational object.

⸻

Ownership

The Artifact Engine exclusively owns Identity Hypotheses.

No other engine SHALL construct, modify, persist, or consume them.

⸻

Lifetime

Identity Hypotheses are created during Artifact construction.

They exist only within a single execution of the Artifact Engine.

They SHALL be destroyed after Artifact Acceptance completes, regardless of acceptance outcome.

Identity Hypotheses SHALL NEVER be persisted.

⸻

Identity

Identity Hypotheses SHALL NOT possess canonical Artifact Identity.

Identity Hypotheses SHALL NOT be externally referenced.

Identity Hypotheses SHALL NOT participate in Workspace formation.

Identity Hypotheses SHALL NOT participate in Knowledge formation.

⸻

Structure

Every Identity Hypothesis SHALL contain:

* one or more canonical Observations;
* exactly one proposed Artifact identity grouping.

Identity Hypotheses SHALL NOT contain:

* Artifact Identity;
* Workspace references;
* Knowledge references;
* Decision references;
* Restoration state.

⸻

Invariants

An Identity Hypothesis SHALL satisfy the following invariants.

1. It SHALL reference one or more canonical Observations.
2. Every referenced Observation SHALL possess canonical Observation Identity.
3. Every referenced Observation SHALL remain immutable.
4. Every Observation SHALL appear at most once within a single Identity Hypothesis.
5. An Identity Hypothesis SHALL represent exactly one proposed Artifact grouping.
6. Identity Hypotheses SHALL NOT reference one another.
7. Identity Hypotheses SHALL remain immutable after construction.
8. Identity Hypotheses SHALL exist only within one Artifact Engine execution.

⸻

Relationships

Canonical Observation(s)
            │
            ▼
Identity Hypothesis
            │
            ▼
Candidate Artifact
            │
            ▼
Artifact Acceptance
            │
            ▼
Canonical Artifact

⸻

Responsibilities

An Identity Hypothesis SHALL:

* aggregate canonical Observations;
* express one proposed Artifact grouping;
* provide the input required by Candidate Artifact construction.

⸻

Non-Responsibilities

An Identity Hypothesis SHALL NOT:

* determine acceptance;
* assign Artifact Identity;
* perform canonicalization;
* perform persistence;
* participate in replay;
* survive Artifact Acceptance.

⸻

Computational Layer

Identity Hypotheses belong exclusively to the Artifact computational layer.

They SHALL NEVER appear within:

* Workspace;
* Knowledge;
* History;
* Restoration.

⸻

Destruction

After Artifact Acceptance completes:

Accepted:

Identity Hypothesis
↓
destroy
↓
Canonical Artifact survives

Rejected:

Identity Hypothesis
↓
destroy
↓
nothing survives
