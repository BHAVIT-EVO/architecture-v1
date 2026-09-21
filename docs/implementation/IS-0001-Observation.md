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

# 5A. Provenance

Every Candidate Observation SHALL contain exactly one Provenance record.

Provenance records the origin and temporal circumstances under which Evo witnessed the Evidence.

Provenance describes the observation event itself.

Provenance SHALL NOT contain semantic interpretation of the Evidence.

Provenance SHALL NOT contain inferred user intent, task membership, workspace membership, artifact identity, importance, confidence, or explanation.

## 5A.1 Provenance Components

Every Provenance record SHALL contain:

- Observation Source
- Observation Time
- Observation Context

The concrete representation of these components is implementation-defined unless otherwise specified by this specification.

### Observation Source

Observation Source identifies the trusted capture origin through which Evo witnessed the Evidence.

Observation Source SHALL identify the capture channel sufficiently to establish provenance.

Observation Source SHALL NOT assert semantic meaning about the Evidence.

Observation Source SHALL NOT be used to classify or rank the Evidence.

### Observation Time

Observation Time records when Evo witnessed the Evidence.

Observation Time SHALL provide a stable temporal position for the Observation.

Observation Time SHALL be preserved without modification after acceptance.

The representation of Observation Time SHALL provide sufficient precision to preserve the ordering required by the Observation Contract.

The Observation system SHALL NOT treat Observation Time as evidence of user intent or activity duration.

### Observation Context

Observation Context records directly observed circumstances necessary to understand the provenance of the Evidence at the time it was witnessed.

Observation Context SHALL contain only directly observed information available through the trusted observation channel.

Observation Context SHALL NOT contain interpretation, inferred state, confidence, classification, or explanation.

Observation Context MAY be empty when the trusted observation channel provides no additional directly observed context beyond Observation Source and Observation Time.

## 5A.2 Provenance and Observation Schema

Provenance SHALL identify the observation circumstances independently of the Observation Schema.

The Observation Schema Identifier remains a separate required component of the Candidate Observation.

Provenance SHALL NOT redefine, extend, or replace the Observation Schema.

## 5A.3 Provenance Immutability

Once an Observation is accepted:

- Provenance SHALL never change.
- Observation Source SHALL never change.
- Observation Time SHALL never change.
- Observation Context SHALL never change.

Any later information about an Observation SHALL be represented by additional Observations or derived computation.

## 5A.4 Provenance Preservation

The Observation Acceptance Pipeline SHALL preserve Provenance exactly as supplied by the Candidate Observation, except for representation canonicalization that does not alter its witnessed content.

Canonicalization SHALL NOT:

- add inferred provenance;
- remove provenance;
- reinterpret provenance;
- replace the Observation Source;
- replace the Observation Time;
- replace the Observation Context.

## 5A.5 Provenance Independence

Provenance SHALL remain meaningful independently of:

- Artifact identity;
- Workspace membership;
- Knowledge;
- Retrieval;
- Restoration;
- any other derived computation.

No derived computation may modify Provenance.

## 5A.6 Provenance Failure

A Candidate Observation SHALL be rejected if required Provenance is absent or structurally invalid.

A Candidate Observation SHALL NOT be repaired by inventing, estimating, or inferring missing Provenance.

## 5A.7 Privacy

Provenance SHALL contain no raw high-fidelity capture.

Provenance SHALL not retain screenshots, screen recordings, or equivalent visual ground truth.

All Provenance processing SHALL remain subject to Evo's local-first and privacy constraints.

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