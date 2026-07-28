# RFC-0003 — Workspace Formation Contract

Status: Accepted

Version: 1.0

Depends On:

- Constitution
- Cognitive Model
- Product
- Architecture
- Architectural Laws
- RFC-0000
- RFC-0001
- RFC-0002

---

# Abstract

This RFC defines the behavioral contract of Workspace Formation.

Workspace Formation is the process through which Evo constructs its current best explanation that multiple Artifact histories collectively describe the evolution of one coherent body of work.

A Workspace is not directly observed.

A Workspace is not a container.

A Workspace is an explanatory hypothesis derived from Artifact histories.

This RFC defines the guarantees every Workspace Formation process MUST preserve, independent of implementation.

---

# Motivation

Human work persists while the artifacts through which it is expressed continuously change.

Applications are replaced.

Files are renamed.

Repositories are reorganized.

Documents are rewritten.

Tools evolve.

Yet the work itself continues.

A computational system that wishes to restore work continuity therefore requires an object whose identity survives changes in the supporting artifacts.

Workspace exists to provide that continuity.

---

# Definition

A Workspace is Evo's current best explanatory hypothesis that a collection of Artifact histories collectively describe the evolution of one coherent body of work.

A Workspace is inferred.

It is never directly observed.

Artifacts do not belong to a Workspace.

Rather, Artifact histories provide evidence supporting a Workspace hypothesis.

Workspace identity exists independently of any individual supporting Artifact.

---

# Scope

This RFC defines:

- what a Workspace is;
- how Workspace identity behaves;
- what guarantees Workspace Formation provides.

This RFC intentionally does not define:

- formation algorithms;
- clustering techniques;
- similarity metrics;
- confidence calculations;
- ranking;
- retrieval;
- restoration;
- learning.

Those responsibilities belong to later specifications.

---

# Behavioral Contract

## Requirement 1 — Derived Explanation

Every Workspace MUST be derived exclusively from Artifact histories.

A Workspace MUST NOT exist independently of supporting Artifact evidence.

---

## Requirement 2 — Continuity, Not Similarity

Workspace Formation MUST explain continuity of evolving work.

Workspace Formation MUST NOT explain topical similarity, application similarity, temporal proximity, file organization, or any other isolated observational feature.

Similarity may contribute evidence.

Similarity MUST NEVER define Workspace identity.

---

## Requirement 3 — Explanatory Nature

A Workspace is an explanatory hypothesis.

A Workspace MUST NEVER be treated as a container of Artifacts.

Artifacts support a Workspace.

A Workspace does not own Artifacts.

---

## Requirement 4 — Identity Through Evolution

Workspace identity is defined by the continued evolution of one coherent body of work.

Workspace identity MUST remain independent of changes in individual supporting Artifacts.

Adding, removing, replacing, or modifying supporting Artifacts MUST NOT alone create or destroy Workspace identity.

---

## Requirement 5 — Historical Basis

Workspace Formation MUST explain the observed evolution of Artifact histories.

Workspace Formation MUST NOT be based solely upon instantaneous system state.

Historical continuity is fundamental.

Instantaneous state is supporting evidence only.

---

## Requirement 6 — Provisional Identity

Every Workspace represents Evo's current best explanatory hypothesis.

Workspace identity is never absolute.

Improved evidence or improved Formation MAY produce a different Workspace hypothesis.

Historical Observations and Artifacts MUST remain unchanged.

---

## Requirement 7 — Replayability

Workspace Formation MUST be fully reproducible from the canonical Observation history.

Replay MUST regenerate Workspace hypotheses rather than mutate historical evidence.

Replay improves explanation.

Replay does not rewrite history.

---

## Requirement 8 — Independence From Hidden State

Workspace Formation MUST depend only upon observable evidence.

Workspace Formation MUST NOT depend upon inferred user intention, subjective motivation, internal mental state, future knowledge, or information unavailable through the observer model defined by the Constitution.

---

## Requirement 9 — Independence From Higher Computation

Workspace Formation MUST occur independently of Retrieval, Restoration, Learning, or future computational layers.

Higher computational objects may consume Workspace hypotheses.

They MUST NOT define Workspace identity.

---

# Guarantees

Every compliant Workspace guarantees:

- derivation from Artifact histories;
- explanatory rather than container semantics;
- provisional identity;
- historical continuity;
- replayability;
- independence from hidden mental state;
- independence from higher computational systems.

---

# Forbidden Behavior

A Workspace MUST NEVER:

- represent objective truth;
- contain Artifacts;
- encode user intention;
- encode semantic purpose as observed fact;
- depend upon future information;
- overwrite historical Artifacts;
- overwrite historical Observations;
- become immutable;
- become canonical history.

Violation of any of these behaviors invalidates compliance with this RFC.

---

# Architectural Consequences

Workspace Formation establishes the first computational representation of work continuity.

Observation explains what was witnessed.

Artifact Identity explains which observations refer to the same external entity.

Workspace Formation explains why Artifact histories collectively describe one evolving body of work.

Every higher-level capability in Evo—including Retrieval, Restoration, Search, Knowledge Formation, and Learning—depends upon Workspace hypotheses rather than constructing independent interpretations of work continuity.

---

# Non-Goals

Workspace Formation is not responsible for:

- determining user goals;
- determining project ownership;
- determining task completion;
- determining restoration order;
- determining importance;
- determining relevance;
- predicting future work.

Workspace Formation answers only one computational question:

"Which Artifact histories are best explained as the evolution of one coherent body of work?"

---

# Compatibility

Any implementation conforms to this RFC if it preserves every behavioral guarantee defined herein.

No particular formation strategy is prescribed.

Behavioral compliance is independent of implementation.

---

# Rationale

Workspace is the first computational object that represents continuity rather than identity.

Unlike Observations, which witness facts, or Artifacts, which establish entity identity, Workspaces explain how work evolves across time despite continual change in the artifacts through which that work is expressed.

By treating Workspaces as explanatory hypotheses rather than containers, Evo preserves replayability, continual improvement, architectural flexibility, and strict separation between observed history and inferred understanding.

Workspace Formation therefore provides the computational bridge between observed history and the reconstruction of human work continuity while remaining permanently accountable to the evidence from which it was derived.