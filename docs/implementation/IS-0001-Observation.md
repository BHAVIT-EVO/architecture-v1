# IS-0001 — Observation

**Status:** Frozen

**Version:** 1.0

**Depends On:**
- Constitution
- Product
- Architecture
- Observation Model v1.0

---

# 1. Purpose

The Observation module is responsible for transforming a Candidate Observation into a Canonical Observation.

It SHALL validate, canonicalize, assign identity, verify integrity, preserve provenance, and durably persist accepted Observations.

The Observation module is the sole architectural entry point through which observational evidence enters Evo.

---

# 2. Scope

The Observation module SHALL:

- Accept Candidate Observations.
- Validate structural correctness.
- Canonicalize representation.
- Assign immutable identity.
- Preserve provenance.
- Verify integrity.
- Persist accepted Observations.
- Reject invalid Candidate Observations.

---

# 3. Responsibilities

The Observation module SHALL perform the following operations for every Candidate Observation.

### R-1 Acceptance

Accept a Candidate Observation for processing.

---

### R-2 Validation

Validate that the Candidate Observation conforms to exactly one known Observation Schema.

Observation Schemas are defined in IS-0003 and are composed from the canonical Observation Language defined in IS-0002.
---

### R-3 Canonicalization

Transform the Candidate Observation into its canonical representation.

Canonicalization SHALL preserve Evidence.

---

### R-4 Identity Assignment

Assign one immutable Observation Identity.

Identity SHALL remain stable for the lifetime of the Observation.

---

### R-5 Provenance Preservation

Preserve all provenance associated with the Observation.

---

### R-6 Integrity Verification

Verify that the constructed Observation satisfies every structural invariant required for acceptance before persistence.
This responsibility concerns architectural integrity, not cryptographic verification.
---

### R-7 Persistence

Durably persist the Observation.

Success SHALL NOT be reported before persistence completes.

---

### R-8 Acceptance Decision

Return exactly one result:

- Accepted Observation
- Rejection

No partial acceptance is permitted.

---

# 4. Non-Responsibilities

The Observation module SHALL NOT:

- infer meaning
- classify observations
- identify artifacts
- detect tasks
- build workspaces
- summarize evidence
- rank importance
- merge observations
- deduplicate observations
- repair evidence
- interpret evidence
- reason about observations

These responsibilities belong to higher architectural layers.

---

# 5. Inputs

The Observation module accepts exactly one input.

## Candidate Observation

A Candidate Observation SHALL contain:

- Evidence
- Provenance
- Observation Schema Identifier

The concrete representation is implementation-defined.
The Observation Schema Identifier references an Observation Schema defined by IS-0003.
The Observation Schema composes canonical Observation Language concepts defined by IS-0002.

---

# 6. Outputs

The Observation module produces exactly one outcome.

## Accepted Observation

An Accepted Observation SHALL satisfy every requirement defined by the Observation Model.

or

## Rejection

Rejected Candidate Observations SHALL NOT become part of Evo.

---

# 7. Guarantees

Upon successful acceptance, the Observation module guarantees the following.

### G-1 Identity

The Observation possesses exactly one immutable identity.

---

### G-2 Immutability

Observation and Evidence are immutable.

---

### G-3 Schema Conformance

Evidence conforms to exactly one immutable Observation Schema.
Schema conformance guarantees structural compatibility with Evo's canonical Observation Language.
---

### G-4 Provenance

Provenance has been permanently preserved.

---

### G-5 Canonical Representation

Canonicalization has successfully completed.

---

### G-6 Integrity

Integrity verification has succeeded.

---

### G-7 Durability

The Observation has been durably persisted.

---

# 8. Invariants

The following properties SHALL remain true throughout the lifetime of every accepted Observation.

### I-1

Accepted Observations never change.

---

### I-2

Evidence is never interpreted.

---

### I-3

Observation Identity never changes.

---

### I-4

Every Observation possesses exactly one Observation Schema.

---

### I-5

Every Observation possesses Provenance.

---

### I-6

Observation acceptance is atomic.

An Observation SHALL either be fully accepted or fully rejected.

---

### I-7

Acceptance is irreversible.

Accepted Observations SHALL never transition back to an earlier lifecycle state.

---

# 9. State Machine

Every Candidate Observation SHALL progress through exactly one of the following state transitions.

```
Candidate
    │
    ▼
Validation
    │
    ▼
Canonicalization
    │
    ▼
Identity Assignment
    │
    ▼
Integrity Verification
    │
    ▼
Persistence
    │
    ▼
Accepted
```

or

```
Candidate
    │
    ▼
Rejected
```

No additional states are permitted.

---

# 10. Failure Modes

The Observation module SHALL reject a Candidate Observation if any of the following occur.

- Unknown Observation Schema.
- Structural validation failure.
- Missing required provenance.
- Canonicalization failure.
- Integrity verification failure.
- Persistence failure.

Rejected Candidate Observations SHALL leave no observable side effects.

---

# 11. Conformance

An implementation conforms to IS-0001 if and only if all of the following are true.

1. Every accepted Observation satisfies the Observation Model.
2. Every Responsibility defined by this specification is implemented.
3. Every Guarantee defined by this specification is upheld.
4. Every Invariant defined by this specification remains true.
5. No Non-Responsibility is performed by the Observation module.
6. Success is never reported before durable persistence completes.

Failure to satisfy any requirement constitutes non-conformance.

---

# End of Specification