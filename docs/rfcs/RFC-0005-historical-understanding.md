# RFC-0005 — Historical Understanding Contract

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

---

# Abstract

This RFC defines how Evo preserves the historical record of its own understanding.

As Interpretation Evolution improves, Evo's current understanding may change indefinitely. Historical understanding, however, must remain permanently available so that every committed architectural decision can always be explained using the understanding that actually existed when that decision was made.

Historical Understanding preserves accountability across time.

---

# Motivation

Observed history is immutable.

Current understanding evolves.

Without preserving historical understanding, improvements to Formation would cause past decisions to be explained using knowledge that did not exist when those decisions were made.

This would destroy architectural accountability.

Historical Understanding exists to preserve the reasoning context that actually justified committed architectural behaviour.

---

# Definition

Historical Understanding is the immutable historical record of the committed decision context that existed at a particular architectural moment.

It preserves what Evo understood at that time.

It does not preserve objective reality.

It does not preserve current understanding.

Historical Understanding exists solely to preserve historical accountability.

---

# Scope

This RFC defines:

- what Historical Understanding preserves;
- when Historical Understanding is created;
- what may never change after preservation;
- how historical explanations are produced.

This RFC intentionally does not define:

- Formation;
- Interpretation Evolution;
- Restoration;
- Retrieval;
- Learning;
- scheduling;
- implementation details.

---

# Behavioral Contract

## Requirement 1 — Commitment Boundary

Historical Understanding SHALL be created only when Evo commits an understanding that becomes architectural state.

Intermediate computation MUST NEVER become Historical Understanding.

Only committed understanding becomes architectural history.

---

## Requirement 2 — Immutability

Historical Understanding MUST NEVER be modified after creation.

Corrections, improvements, or future understanding MUST create new Historical Understanding rather than altering existing history.

Historical reasoning is immutable.

---

## Requirement 3 — Temporal Integrity

Every explanation of past architectural behaviour MUST be derived from the Historical Understanding that existed when that behaviour occurred.

Present understanding MUST NEVER be used to explain past decisions.

Historical explanations preserve temporal integrity.

---

## Requirement 4 — Independence From Current Understanding

Historical Understanding remains permanently valid regardless of future Interpretation Evolution.

Current understanding may change indefinitely.

Historical understanding does not.

---

## Requirement 5 — Historical Accountability

Every externally observable architectural behaviour MUST remain explainable using Historical Understanding.

Explanation MUST reference the committed understanding that actually justified the behaviour.

Historical accountability is permanent.

---

## Requirement 6 — Architectural Separation

Historical Understanding MUST remain separate from:

- canonical Observations;
- current Workspace;
- future interpretations;
- future Formation processes.

Historical Understanding records understanding.

It does not replace history.

It does not replace current interpretation.

---

## Requirement 7 — Replay Compatibility

Interpretation Evolution MUST NEVER rewrite Historical Understanding.

Replay constructs better current understanding.

Replay preserves historical understanding exactly as originally committed.

Replay improves explanation going forward.

It never alters historical reasoning.

---

# Guarantees

Every compliant implementation guarantees:

- historical reasoning is permanently preserved;
- historical explanations remain temporally correct;
- improvements never rewrite historical accountability;
- every committed architectural decision remains explainable;
- current understanding and historical understanding remain permanently separated.

---

# Forbidden Behaviour

Historical Understanding MUST NEVER:

- be rewritten;
- be regenerated using newer understanding;
- be silently updated;
- be merged with current understanding;
- depend upon future observations;
- explain historical behaviour using future reasoning.

Violation of any of these behaviours invalidates compliance with this RFC.

---

# Architectural Consequences

Historical Understanding establishes a permanent distinction between:

- what Evo currently believes; and
- what Evo believed when a historical architectural decision was committed.

This distinction enables long-term trust, auditability, reproducibility, and accountability while allowing Interpretation Evolution to continue indefinitely.

Without Historical Understanding, Replay would unintentionally rewrite reasoning history.

---

# Identity Criterion

Each Historical Understanding represents one committed architectural understanding.

Its identity is determined by the committed understanding it preserves.

Once created, that identity never changes.

Future understanding produces new Historical Understanding rather than modifying existing history.

---

# Non-Goals

Historical Understanding is not responsible for:

- determining current understanding;
- collecting observations;
- restoring work;
- retrieving workspaces;
- learning;
- ranking;
- prediction.

Historical Understanding answers one architectural question only:

> "What understanding actually justified this committed architectural behaviour?"

---

# Compatibility

Any implementation conforms to this RFC if it preserves every behavioural guarantee defined herein.

No storage mechanism is prescribed.

No serialization format is prescribed.

No persistence technology is prescribed.

Behavioural compatibility is independent of implementation.

---

# Rationale

History and understanding are fundamentally different.

History records what was witnessed.

Understanding records how those observations were interpreted.

As understanding evolves, history must remain unchanged.

Historical Understanding extends this principle by ensuring that reasoning history remains as immutable as observation history.

This allows Evo to improve forever without ever losing the ability to explain its past honestly.

---

# Self-Critique

## Assumptions

This RFC assumes immutable Observations and provisional Interpretation as defined by earlier RFCs.

## Deliberate Omissions

This RFC intentionally leaves storage format, replay scheduling, serialization, indexing, and implementation strategy unspecified.

## Architectural Boundary

Historical Understanding preserves committed reasoning.

It does not govern how reasoning is formed, improved, retrieved, or acted upon.

Those responsibilities belong to other RFCs.

---

# Architectural Law

> **Past behaviour shall always be explained by the understanding that actually produced it, never by understanding acquired later.**