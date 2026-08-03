# RFC-0007 — Retrieval Contract

**Status:** Accepted

**Version:** 1.0

**Depends On:**
- Constitution
- Cognitive Model
- Architectural Laws
- RFC-0000
- RFC-0001
- RFC-0002
- RFC-0003
- RFC-0004
- RFC-0005
- RFC-0006

---

# Abstract

This RFC defines how Evo resolves a trigger into the most appropriate current architectural understanding.

Retrieval is the process through which Evo determines which committed Workspace should become active in response to an external trigger.

Retrieval selects understanding.

It never creates, modifies, or improves understanding.

---

# Motivation

Evo may simultaneously possess many valid Workspaces.

Before assistance can occur, Evo must determine which Workspace best satisfies the current trigger.

Without Retrieval, Restoration has no architectural input and understanding cannot become useful assistance.

Retrieval therefore forms the bridge between user intent and architectural understanding.

---

# Definition

Retrieval is the architectural process that resolves a trigger into one or more candidate current Workspaces.

Its purpose is to activate the Workspace that most faithfully satisfies the trigger according to Evo's current understanding.

Retrieval never constructs new understanding.

Retrieval never modifies existing understanding.

Retrieval only resolves which current understanding should become active.

---

# Scope

This RFC defines:

- what Retrieval consumes;
- what Retrieval produces;
- how ambiguity is represented;
- the guarantees Retrieval must preserve.

This RFC intentionally does not define:

- ranking algorithms;
- semantic search;
- embeddings;
- vector databases;
- keyword matching;
- indexing strategies;
- scoring functions;
- user interfaces.

---

# Behavioral Contract

## Requirement 1 — Trigger-Driven

Retrieval SHALL operate only in response to an architectural trigger.

Triggers may originate from:

- explicit user requests;
- scheduled events;
- notifications;
- external APIs;
- system automation;
- any other architecturally valid source.

The source of the trigger does not alter Retrieval's behavioral guarantees.

---

## Requirement 2 — Current Understanding

Retrieval MUST resolve only against the current committed architectural understanding.

Historical Understanding MAY be consulted solely when historical explanation is explicitly requested.

Historical Understanding MUST NOT influence present retrieval.

---

## Requirement 3 — Resolution

Retrieval SHALL return the Workspace or set of Workspaces that most faithfully satisfy the trigger according to current understanding.

Retrieval MUST NOT manufacture certainty where current understanding is ambiguous.

---

## Requirement 4 — Preservation of Uncertainty

When current understanding does not uniquely satisfy a trigger, Retrieval SHALL preserve that ambiguity.

Ambiguity is architectural information.

It MUST NOT be discarded merely to produce a single answer.

---

## Requirement 5 — Architectural Purity

Retrieval MUST NOT:

- create Observations;
- modify Artifact identity;
- construct Workspaces;
- evolve Interpretation;
- modify Historical Understanding;
- alter Restoration.

Retrieval activates understanding.

It does not produce understanding.

---

## Requirement 6 — Explainability

Every Retrieval result MUST remain explainable using the current committed understanding.

If multiple Workspaces satisfy the trigger, Evo SHALL be able to explain why each satisfies the request.

---

## Requirement 7 — Determinism

Given identical:

- canonical Observations;
- current committed understanding;
- trigger;

Retrieval MUST produce behaviorally equivalent results.

Behavioral equivalence does not require identical implementation.

It requires identical architectural meaning.

---

# Guarantees

Every compliant implementation guarantees:

- Retrieval never modifies architectural knowledge;
- Retrieval activates current understanding;
- ambiguity is preserved rather than hidden;
- every result remains explainable;
- Retrieval remains independent of implementation strategy.

---

# Forbidden Behaviour

Retrieval MUST NEVER:

- construct new Workspaces;
- rewrite understanding;
- silently resolve ambiguity;
- depend upon implementation-specific assumptions;
- use future observations;
- treat historical understanding as current understanding.

Violation of any of these behaviours invalidates compliance with this RFC.

---

# Architectural Consequences

Retrieval establishes the architectural boundary between understanding and activation.

Understanding may exist without Retrieval.

Restoration cannot.

Every externally visible collaboration begins by activating the correct Workspace through Retrieval.

Retrieval therefore determines which understanding becomes available for collaboration.

---

# Identity Criterion

A Retrieval is identified by:

- the trigger that initiated it; and
- the committed understanding against which it was evaluated.

Different triggers over identical understanding constitute different Retrievals.

Changes in implementation do not change Retrieval identity.

---

Public Computational Contract

Retrieval is exposed to the remainder of the architecture solely through a single computational boundary.

Every compliant implementation SHALL provide a Retrieval service.

The Retrieval service SHALL consume:

* one architectural Trigger;
* the current committed Workspaces.

The Retrieval service SHALL produce:

* zero or more candidate WorkspaceIds.

Returned WorkspaceIds SHALL preserve the architectural ordering determined by the implementation.

Returned WorkspaceIds SHALL NOT duplicate the same Workspace.

An empty result is a valid architectural outcome.

It represents that no current committed Workspace satisfies the supplied Trigger.

Retrieval SHALL NOT:

* return Workspace objects;
* return Historical Understanding;
* modify any Workspace;
* construct any Workspace;
* expose implementation-specific ranking information;
* expose implementation-specific confidence scores.

Errors SHALL be limited to failures that prevent Retrieval from executing according to this RFC.

The absence of matching Workspaces SHALL NOT be considered an error.

The computational contract intentionally does not prescribe:

* public method signatures;
* search algorithms;
* ranking strategies;
* indexing structures;
* embedding models;
* storage engines.

Any implementation satisfying this computational contract remains compliant with this RFC.

---

# Non-Goals

Retrieval is not responsible for:

- collecting observations;
- resolving identity;
- forming Workspaces;
- evolving interpretation;
- preserving historical understanding;
- generating assistance;
- learning.

Retrieval answers one architectural question only:

> "Given this trigger, which current understanding should become active?"

---

# Compatibility

Any implementation conforms to this RFC if it preserves every behavioral guarantee defined herein.

No retrieval algorithm is prescribed.

No storage engine is prescribed.

No ranking strategy is prescribed.

Behavioral compatibility is independent of implementation.

---

# Rationale

Retrieval exists because architectural understanding alone is insufficient for collaboration.

Before Evo can assist, it must first determine which understanding is relevant to the present situation.

By separating Retrieval from Formation, Interpretation Evolution, and Restoration, Evo preserves a strict separation between:

- constructing understanding;
- selecting understanding;
- using understanding.

This separation allows each responsibility to evolve independently without violating the architectural contracts established by previous RFCs.

---

# Self-Critique

## Assumptions

This RFC assumes the existence of committed Workspaces as defined by RFC-0003 and RFC-0004.

## Deliberate Omissions

This RFC intentionally omits ranking methods, indexing structures, embedding models, search strategies, query parsing, caching, and implementation details.

These belong to implementation.

## Architectural Boundary

Retrieval activates architectural understanding.

It neither constructs understanding nor transforms understanding into user-visible assistance.

Those responsibilities belong to Formation and Restoration respectively.

---

# Architectural Law

> **Retrieval shall activate the understanding that most faithfully satisfies the current trigger while preserving uncertainty wherever certainty has not been earned.**