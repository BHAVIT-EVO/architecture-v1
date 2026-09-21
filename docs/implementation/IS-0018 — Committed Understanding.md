# IS-0018 — Committed Understanding

Status: Frozen

Version: 1.0

Depends On

* Constitution
* Product
* Architecture
* Architectural Laws
* RFC-0005 — Historical Understanding Contract
* IS-0011 — Workspace Model

---

# 1. Purpose

This specification defines the canonical CommittedUnderstanding computational object.

CommittedUnderstanding represents the exact canonical understanding that has been committed for historical accountability.

It exists solely to preserve the understanding that justified an architectural action.

This specification defines the computational structure of CommittedUnderstanding.

It does not define Historical Commitment.

---

# 2. Scope

This specification defines:

* CommittedUnderstanding
* ownership
* canonical components
* immutability
* invariants
* public computational surface

This specification does NOT define:

* Historical Commitment
* Replay
* Restoration
* Retrieval
* Learning
* Persistence
* Scheduling

---

# 3. Canonical Components

CommittedUnderstanding SHALL consist of exactly:

* ResumePoint
* ContextChain
* Blockers
* NextStep

Nothing else belongs to the canonical object.

---

# 4. Ownership

CommittedUnderstanding SHALL belong exclusively to one HistoricalUnderstanding.

CommittedUnderstanding SHALL NOT exist independently once Historical Commitment has completed.

CommittedUnderstanding SHALL NEVER be shared between HistoricalUnderstanding objects.

---

# 5. Immutability

CommittedUnderstanding SHALL become immutable immediately after Historical Commitment.

No mutating APIs SHALL exist.

Replay SHALL construct a completely new CommittedUnderstanding.

Replay SHALL NEVER modify an existing CommittedUnderstanding.

---

# 6. Required Invariants

CU-1

CommittedUnderstanding contains exactly one ResumePoint.

---

CU-2

CommittedUnderstanding contains exactly one ContextChain.

---

CU-3

CommittedUnderstanding contains zero or more Blockers.

---

CU-4

CommittedUnderstanding contains exactly one NextStep.

---

CU-5

CommittedUnderstanding SHALL remain immutable after commitment.

---

CU-6

Replay SHALL create a new CommittedUnderstanding.

---

CU-7

CommittedUnderstanding SHALL preserve committed understanding exactly.

---

# 7. Non-Responsibilities

CommittedUnderstanding SHALL NOT own:

* Observation
* Artifact
* Workspace
* HistoricalUnderstanding
* Knowledge
* Learning
* Replay state

CommittedUnderstanding SHALL NOT perform:

* Restoration
* Replay
* Retrieval
* Historical Commitment

---

# 8. Public Computational Surface

CommittedUnderstanding

No additional public types are required.

---

# 9. Dependencies

Depends on:

* Constitution
* Product
* Architecture
* Architectural Laws
* RFC-0005
* IS-0011