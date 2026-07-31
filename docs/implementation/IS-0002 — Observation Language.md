# IS-0002 — Observation Language

**Status:** Frozen

**Version:** 1.0

**Depends On:**
- Constitution
- Product
- Architecture

---

# 1. Purpose

The Observation Language defines Evo's canonical language for representing human work.

It provides a stable, platform-independent representation through which every Collector communicates observational information to the rest of the Evo architecture.

Every Observation accepted by Evo SHALL be expressed using the Observation Language.

The Observation Language exists to ensure that every downstream architectural layer interprets observational information using the same concepts, regardless of operating system, implementation, or collection mechanism.

---

# 2. Scope

The Observation Language SHALL:

- define Evo's canonical representation of observational information
- establish platform-independent concepts
- provide a stable language for Observation Schemas
- enable interoperability between Collectors and Consumers
- preserve long-term architectural compatibility

The Observation Language SHALL NOT:

- define Observation Schemas
- define storage formats
- define serialization
- define implementation details
- define platform-specific APIs
- perform interpretation
- infer meaning
- classify work

---

# 3. Core Concepts

The Observation Language consists of canonical concepts that describe human work independently of any operating system or implementation.

Canonical concepts SHALL represent semantics rather than platform-specific constructs.

Collectors SHALL translate platform-specific observations into canonical concepts before they enter Evo.

Consumers SHALL reason exclusively over canonical concepts.

---

# 4. Language Principles

### LP-1 Platform Independence

The Observation Language SHALL remain independent of any operating system.

No platform-specific concept SHALL become part of the Observation Language.

---

### LP-2 Canonical Representation

Every observational concept SHALL possess exactly one canonical representation.

Equivalent observations collected from different platforms SHALL produce the same canonical meaning.

---

### LP-3 Translation

Collectors translate platform-specific observations into the Observation Language.

Collectors SHALL NOT extend or redefine the Observation Language.

---

### LP-4 Semantic Representation

Canonical concepts SHALL describe work semantics rather than implementation details.

Implementation-specific identifiers SHALL remain outside the Observation Language.

---

### LP-5 Consumer Independence

Architectural layers consuming Observations SHALL depend only upon the Observation Language.

Consumers SHALL NOT require knowledge of platform-specific implementations.

---

### LP-6 Stability

The Observation Language SHALL evolve conservatively.

Backward compatibility SHALL be preserved whenever possible.

---

# 5. Language Evolution

The Observation Language MAY evolve through the introduction of additional canonical concepts.

Existing canonical concepts SHALL NOT change semantic meaning.

Breaking semantic changes SHALL require a new language version.

Observation Schemas SHALL remain compatible with the language version under which they were defined.

---

# 6. Invariants

The following properties SHALL always remain true.

### I-1

The Observation Language is platform-independent.

---

### I-2

Canonical concepts represent semantics rather than implementation.

---

### I-3

Collectors communicate with Evo exclusively through the Observation Language.

---

### I-4

Consumers reason exclusively over the Observation Language.

---

### I-5

Canonical concepts possess stable semantic meaning.

---

### I-6

The Observation Language evolves independently of Collector implementations.

---

# 7. Conformance

An implementation conforms to IS-0002 if and only if all of the following are true.

1. Every Observation entering Evo is expressed using the Observation Language.
2. No platform-specific concept appears within the Observation Language.
3. Collectors translate observations into canonical concepts before acceptance.
4. Consumers depend exclusively upon canonical concepts.
5. Every Language Principle defined by this specification is upheld.
6. Every Invariant defined by this specification remains true.

Failure to satisfy any requirement constitutes non-conformance.

---

# End of Specification