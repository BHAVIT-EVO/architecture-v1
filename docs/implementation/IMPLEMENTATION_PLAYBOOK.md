# Evo Implementation Playbook

Version: 1.0
Status: Canonical

---

# Purpose

This document defines how every implementation for Evo must be written.

It is intended for humans and AI coding assistants.

Architecture takes precedence over implementation.

If implementation contradicts architecture, STOP and explain the contradiction instead of inventing a solution.

---

# Engineering Philosophy

We are not building Rust crates.

We are implementing a model of reality.

Rust is only the implementation language.

Every type should represent a real concept in Evo.

Every file should have one responsibility.

Every module should have one reason to change.

---

# Authority Order

Always follow documents in this order.

1. Constitution
2. Product
3. Architecture
4. Observation Model
5. RFC
6. Implementation Specification (IS)
7. Existing code

Never violate a higher-level document.

---

# Core Principles

- Local-first
- Privacy-first
- Correctness over cleverness
- Explicit over implicit
- Immutable domain objects whenever possible
- No speculative abstractions
- No architecture invented during implementation
- Prefer silence over wrong assumptions

---

# Responsibilities

Every file must define exactly one concept.

Every public type must have:

- a purpose
- invariants
- documented responsibilities
- documented non-responsibilities

---

# Domain First

Before writing code, answer:

What real-world concept does this represent?

If that cannot be answered, the type probably should not exist.

---

# Single Responsibility

Do not mix responsibilities.

Example:

ObservationId

Responsible for:

- uniquely identifying an Observation

Not responsible for:

- persistence
- serialization strategy
- storage
- validation
- business logic

---

# Immutability

Accepted domain objects should be immutable.

Mutation should happen before acceptance whenever possible.

---

# Construction

Construct invalid states as little as possible.

Prefer types that guarantee correctness by construction.

---

# Validation

Validation answers only:

Is this valid?

Validation never repairs data.

---

# Canonicalization

Canonicalization transforms data into canonical form.

Canonicalization never decides validity.

---

# Integrity

Integrity computes fingerprints.

Nothing else.

---

# Error Handling

Use dedicated error types.

Avoid String errors.

Avoid anyhow in domain models.

Errors should communicate domain meaning.

---

# Public APIs

Public APIs should be small.

Avoid exposing implementation details.

Prefer opaque types.

---

# Dependencies

Every dependency must have a clear justification.

Do not introduce dependencies for convenience alone.

---

# Traits

Do not introduce traits until there is more than one legitimate implementation.

Avoid speculative abstraction.

---

# Builders

Do not create builders automatically.

Only introduce builders if construction genuinely requires them.

---

# Services

Avoid Service, Manager, Factory, Repository unless the architecture explicitly defines them.

---

# Storage

Domain models know nothing about:

- SQLite
- REST
- Axum
- JSON
- networking

Those belong elsewhere.

---

# Documentation

Every public type should have rustdoc.

Every public function should explain:

- what it does
- what guarantees it provides

---

# Testing

Tests verify invariants.

Tests should not depend on implementation details.

Test behavior, not code structure.

---

# Code Style

Prefer readable code over clever code.

Prefer explicit names.

Avoid unnecessary generics.

Avoid macros unless they significantly improve clarity.

Keep modules cohesive.

---

# If Unsure

Never invent architecture.

Instead:

Explain the ambiguity.

Explain the possible approaches.

Wait for architectural guidance.

