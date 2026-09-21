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

4.1 Initial Canonical Observation Schemas

The initial Observation Schema set SHALL contain exactly the following schemas.

⸻

4.1.1 OBS-WINDOW-FOCUS-GAINED/v1

Schema Identifier

OBS-WINDOW-FOCUS-GAINED

Schema Version

1

Observation Type

WindowFocusGained

Required Canonical Concept

WindowFocusGained

Required Subject

The window to which the directly witnessed focus-gain event refers.

Optional Canonical Concepts

None.

The schema SHALL NOT require application identity, process identity, window title, operating-system window identifiers, or other platform-specific values unless a future version explicitly establishes them as canonical concepts.

⸻

4.1.2 OBS-FILE-SAVED/v1

Schema Identifier

OBS-FILE-SAVED

Schema Version

1

Observation Type

FileSaved

Required Canonical Concept

FileSaved

Required Subject

The file to which the directly witnessed save event refers.

Optional Canonical Concepts

None.

The schema SHALL NOT require filesystem implementation details such as inode numbers, filesystem-specific identifiers, operating-system handles, editor-specific identifiers, or other platform-specific values.

⸻

4.1.3 OBS-URL-NAVIGATED/v1

Schema Identifier

OBS-URL-NAVIGATED

Schema Version

1

Observation Type

URLNavigated

Required Canonical Concept

URLNavigated

Required Subject

The URL to which the directly witnessed navigation event refers.

Optional Canonical Concepts

None.

The schema SHALL NOT require a particular browser, browser API, browser process, tab identifier, or other platform-specific implementation detail.

⸻

4.1.4 OBS-COMMIT-MADE/v1

Schema Identifier

OBS-COMMIT-MADE

Schema Version

1

Observation Type

CommitMade

Required Canonical Concept

CommitMade

Required Subject

The commit to which the directly witnessed commit event refers.

Optional Canonical Concepts

None.

The schema SHALL NOT require a particular version-control implementation, command-line interface, repository implementation, or platform-specific representation.

⸻

4.1.5 OBS-WORK-DESIGNATED/v1

Schema Identifier

OBS-WORK-DESIGNATED

Schema Version

1

Observation Type

WorkDesignated

Required Canonical Concept

WorkDesignated

Required Subject

The subject the user designated as the continuation of their work.

Optional Canonical Concepts

None.

The schema SHALL NOT require application identity, process identity, browser identity, window identifiers, or any other platform-specific value.

The schema records the directly witnessed declaration act itself. It SHALL NOT record an inference about user intent, and it SHALL NOT imply prediction, importance, priority, unfinishedness, or Workspace membership.

The schema SHALL NOT be used to establish a new external entity: a WorkDesignated observation references a subject other canonical Observations have already established (RFC-0011 §4). It remains a canonical Observation and is persisted, immutable, and replayable exactly like every other Observation.

⸻

4.1.6 OBS-REPOSITORY-MEMBERSHIP/v1

Schema Identifier

OBS-REPOSITORY-MEMBERSHIP

Schema Version

1

Observation Type

RepositoryMembership

Required Canonical Concept

RepositoryMembership

Required Subject

The member entity (file, directory, or commit) to which the directly witnessed membership fact refers.

Required Canonical Concept (value)

Repository — the repository identity the member belongs to, a canonical subject-valued value.

Optional Canonical Concepts

None.

The schema SHALL NOT require application identity, process identity, editor identity, or any other platform-specific value. The member subject is platform-typical text (a path or commit hash), exactly as in the frozen schemas; the repository identity is canonical opaque text that Formation compares for equality only and never interprets (RFC-0012).

The schema records the directly witnessed structural membership fact. It SHALL NOT record an inference about user intent, importance, relevance, or Workspace membership.

The schema SHALL NOT be used to establish a new external entity: a RepositoryMembership observation references the member subject and carries the repository identity as a canonical value; it never establishes an Artifact for either (RFC-0012 §Artifact Interaction). It remains a canonical Observation and is persisted, immutable, and replayable exactly like every other Observation.

⸻

4.1.7 OBS-WORK-GROUPED/v1

Schema Identifier

OBS-WORK-GROUPED

Schema Version

1

Observation Type

WorkGrouped

Required Canonical Concept

WorkGrouped

Required Subject

The first subject of the declared pair under the canonical pairing rule (the lexicographically smaller canonical subject string).

Required Canonical Concept (value)

CoMember — the second subject of the declared pair.

Optional Canonical Concepts

None.

The schema SHALL NOT require application, browser, or process identity.

The schema records the directly witnessed declaration act itself (RFC-0012). It SHALL NOT record an inference about user intent or about whether the subjects actually belong to one body of work.

The schema SHALL NOT be used to establish a new external entity: a WorkGrouped observation references subjects other canonical Observations have already established; it never establishes an Artifact for either (RFC-0012 §Artifact Interaction). It remains a canonical Observation and is persisted, immutable, and replayable exactly like every other Observation.

⸻

4.1.8 OBS-CONTINUATION-SURFACE/v1

Schema Identifier

OBS-CONTINUATION-SURFACE

Schema Version

1

Observation Type

ContinuationSurface

Required Subject

The first subject of the declared set under the canonical ordering rule (the lexicographically smallest canonical subject string).

Required Canonical Concept

ContinuationSurface — the witnessed declaration fact, carrying the Required Subject.

Required Canonical Concept (value)

ContinuationSubject — subject-valued; one fact per additional subject beyond the Required Subject, in ascending canonical order.

Cardinality

A valid declaration names at least two distinct subjects (the surface is the multi-resource continuation declaration; a single-subject continuation declaration is the RFC-0011 WorkDesignated contract). No upper bound.

Optional Canonical Concepts

None.

The schema SHALL NOT require application, browser, process, or editor identity. Subjects are platform-typical canonical text (paths, commit hashes, URL subjects), exactly as in the frozen schemas.

The schema records the directly witnessed declaration act itself (RFC-0013). It SHALL NOT record an inference about user intent, recency, frequency, focus, or relevance.

The schema SHALL NOT be used to establish a new external entity: a ContinuationSurface observation references subjects other canonical Observations have already established; it never establishes an Artifact for any declared subject (RFC-0013 §Artifact Interaction). It remains a canonical Observation and is persisted, immutable, and replayable exactly like every other Observation. The declared set is encoded in canonical order, so a declaration expressed in any order produces the identical canonical Observation (deterministic, order-independent encoding).

⸻

4.2 Schema Semantic Boundary

The schemas define the semantic structure required for each Observation type.

They SHALL NOT define:

* Collector behavior;
* operating-system signal sources;
* triggering mechanisms;
* buffering;
* rate limiting;
* deduplication;
* storage formats;
* serialization formats;
* implementation types;
* Artifact identity;
* Workspace membership;
* Knowledge;
* interpretation.

The Collector is responsible for translating a platform-specific signal into a Candidate Observation conforming to the applicable schema.

The Observation Acceptance module is responsible for validating and accepting or rejecting that Candidate Observation according to IS-0001.

⸻

4.3 Subject Identity Boundary

A schema’s Required Subject identifies what the observed fact concerns.

A Required Subject SHALL NOT be interpreted as canonical Artifact Identity.

Artifact Identity is established by the Artifact layer after Observation acceptance.

Therefore:
Observation
    ↓
Observed subject
    ↓
Artifact resolution
    ↓
Artifact Identity

The Observation Schema SHALL NOT perform or prescribe the Artifact resolution step.

---

# 5. Schema Evolution

Observation Schemas MAY evolve.

Adding optional canonical concepts is a compatible change.

Adding a required canonical concept is a breaking schema change and SHALL require a new Schema Version.

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