# RFC-0000 — The RFC System

**Status:** Accepted  
**Version:** 1.0

---

# Abstract

This RFC defines the RFC system used by Evo.

An RFC is not documentation.

An RFC is a permanent behavioral contract.

RFCs define the observable behavior of Evo independently of implementation.

They exist to ensure that Evo's behavior remains stable while its implementation continues to evolve.

---

# Purpose

The purpose of the RFC system is to provide precise, testable, implementation-independent behavioral specifications.

RFCs define what Evo must do.

They deliberately avoid defining how Evo must do it.

---

# Scope

This document governs every RFC in the Evo project.

It defines:

- what an RFC is;
- what an RFC may define;
- what an RFC must never define;
- the relationship between RFCs and the foundation documents;
- how RFCs evolve;
- how RFC quality is evaluated.

---

# Relationship to the Foundation

RFCs exist beneath Evo's foundation documents.

The dependency hierarchy is fixed.

```
Constitution
        ↓
Cognitive Model
        ↓
Product
        ↓
Architecture
        ↓
Architectural Laws
        ↓
RFCs
        ↓
Implementation
```

Every lower layer MUST remain consistent with every higher layer.

RFCs refine behavior.

They MUST NOT contradict the foundation.

---

# Definition

An RFC defines exactly one behavioral contract.

A behavioral contract specifies the externally observable behavior that every compliant implementation MUST exhibit.

Behavioral contracts are implementation-independent.

Different implementations that satisfy the same RFC SHOULD be behaviorally indistinguishable to users.

---

# Principles

## Principle I — Behavior Before Implementation

RFCs define observable behavior.

RFCs MUST NOT constrain implementation unless the implementation detail itself affects observable behavior.

---

## Principle II — One Contract Per RFC

Each RFC MUST define one behavioral contract.

A contract SHOULD have one clear responsibility.

Behavior that can evolve independently SHOULD be specified by separate RFCs.

---

## Principle III — Behavioral Sufficiency

An RFC MUST specify:

- everything necessary to guarantee the required behavior; and
- nothing unnecessary that restricts future implementations.

A contract MUST be complete without becoming prescriptive.

---

## Principle IV — Observable Behavior

Every externally observable behavior MUST be specified.

Implementation choices that do not affect observable behavior MUST remain unconstrained.

---

## Principle V — Testability

Every normative requirement MUST be testable.

If compliance cannot be verified through observation or testing, the requirement MUST be rewritten.

---

## Principle VI — Explainability

Every behavioral contract SHOULD include a rationale.

Rationale explains why the behavior exists.

Rationale is informative.

Normative requirements are authoritative.

Changing rationale MUST NOT change behavior.

---

## Principle VII — Stability

Accepted RFCs define stable behavioral contracts.

Behavior MUST NOT change through silent modification of an accepted RFC.

Behavioral changes require versioning or superseding the affected RFC.

History MUST remain honest.

---

## Principle VIII — Independence

RFCs SHOULD remain independent whenever practical.

An RFC MAY reference another RFC.

An RFC MUST NOT redefine behavior already specified by another accepted RFC.

---

# Normative Language

The following terms are normative throughout the RFC series.

- MUST
- MUST NOT
- SHALL
- SHALL NOT
- SHOULD
- SHOULD NOT
- MAY

All other language is explanatory.

---

# RFC Structure

Every RFC SHOULD contain the following sections where applicable.

- Abstract
- Motivation
- Scope
- Definitions
- Normative Requirements
- Behavioral Contract
- State Model
- Failure Cases
- Non-Goals
- Rationale

Normative requirements MUST remain clearly distinguishable from explanatory material.

---

# Acceptance Criteria

An RFC is considered complete when all of the following are true.

- It is internally consistent.
- It does not contradict any foundation document.
- Every normative requirement is testable.
- Observable behavior is completely specified.
- Implementation remains unconstrained wherever behavior does not require constraint.

---

# The Two-Team Test

Every RFC SHOULD satisfy the following thought experiment.

Two independent engineering teams receive only the accepted RFC.

The teams do not communicate.

Both independently implement the specification.

If users cannot distinguish the observable behavior of the two implementations, the RFC is sufficiently specified.

If observable behavior differs, the RFC is incomplete.

---

# Evolution

RFCs are intended to be stable.

Behavioral changes SHOULD be introduced through versioning or superseding existing RFCs.

Accepted behavior MUST NEVER be silently reinterpreted.

---

# Summary

RFCs are Evo's behavioral specification.

They define what Evo must do.

They intentionally avoid defining how Evo must do it.

Implementation may evolve indefinitely.

Behavioral contracts must remain stable.