# IS-0003 — Observation Schemas

**Status:** Frozen

**Version:** 1.0

**Depends On:**
- Constitution
- Product
- Architecture
- IS-0002 Observation Language

---

# 1. Purpose

An Observation Schema defines the canonical structure of a specific Observation type.

Observation Schemas compose the Observation Language into concrete Observation definitions.

Every Observation accepted by Evo SHALL conform to exactly one Observation Schema.

Observation Schemas provide stable structural contracts between Collectors and Consumers.

---

# 2. Scope

The Observation Schema SHALL:

- define one Observation type
- define required canonical concepts
- define optional canonical concepts
- define schema identity
- define schema version
- provide a stable structural contract

The Observation Schema SHALL NOT:

- perform validation
- perform interpretation
- infer meaning
- define platform-specific behavior
- define Collector behavior
- define storage formats
- define implementation details

---

# 3. Responsibilities

Every Observation Schema SHALL perform the following responsibilities.

### R-1 Observation Definition

Define exactly one Observation type.

---

### R-2 Required Canonical Concepts

Specify every canonical concept required by the Observation.

---

### R-3 Optional Canonical Concepts

Specify canonical concepts that MAY be present.

---

### R-4 Schema Identity

Provide exactly one immutable Schema Identifier.

---

### R-5 Schema Version

Provide exactly one Schema Version.

---

### R-6 Language Compatibility

Remain structurally compatible with the Observation Language.

---

# 4. Schema Structure

Every Observation Schema SHALL define:

- Schema Identifier
- Schema Version
- Required Canonical Concepts
- Optional Canonical Concepts

The concrete representation of a Schema is implementation-defined.

---

# 5. Schema Evolution

Observation Schemas MAY evolve.

Adding optional canonical concepts is a compatible change.

Removing required canonical concepts is a breaking change.

Changing the semantic meaning of an existing canonical concept is prohibited.

Breaking changes SHALL require a new Schema Version.

---

# 6. Guarantees

Every Observation Schema guarantees the following.

### G-1 Identity

Every Schema possesses exactly one immutable Schema Identifier.

---

### G-2 Version

Every Schema possesses exactly one Schema Version.

---

### G-3 Observation Conformance

Every Observation conforms to exactly one Observation Schema.

---

### G-4 Required Concepts

Every required canonical concept SHALL be present.

---

### G-5 Optional Concepts

Optional canonical concepts MAY be absent.

---

### G-6 Stable Semantics

The semantic meaning of a Schema remains stable throughout its lifetime.

---

# 7. Invariants

The following properties SHALL remain true throughout the lifetime of every Observation Schema.

### I-1

A Schema Identifier never changes.

---

### I-2

A Schema Version never changes.

---

### I-3

Every Observation conforms to exactly one Observation Schema.

---

### I-4

Collectors SHALL NOT redefine Observation Schemas.

---

### I-5

Consumers SHALL NOT infer or modify Observation Schema definitions.

---

### I-6

Observation Schemas remain platform-independent.

---

# 8. Conformance

An implementation conforms to IS-0003 if and only if all of the following are true.

1. Every Observation conforms to exactly one Observation Schema.
2. Every Observation Schema defines required and optional canonical concepts.
3. Every Observation Schema possesses exactly one immutable Schema Identifier.
4. Every Observation Schema possesses exactly one Schema Version.
5. Every Guarantee defined by this specification is upheld.
6. Every Invariant defined by this specification remains true.

Failure to satisfy any requirement constitutes non-conformance.

---

# End of Specification