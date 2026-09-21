# IS-0014 — Confidence Score

Status: Frozen

Depends On:

* Constitution
* Product
* Architecture
* RFC-0003 — Workspace (v2.0)
* RFC-0014 — Engagement Contract (supersedes IS-0012 in full)
* IS-0011 — Workspace Model (as amended, Amendment 1)

---

## 1. Purpose

This specification defines the canonical Confidence Score computational object.

A Confidence Score represents the evidential strength supporting the attachment of one Artifact to one Workspace.

This specification defines:

* Confidence Score semantics;
* Confidence Score ownership;
* Confidence Score invariants;
* Confidence Score boundaries.

This specification does not define:

* confidence computation;
* Workspace Formation;
* Workspace Replay;
* Workspace Learning;
* Workspace Retrieval.

---

## 2. Scope

This specification governs only the canonical Confidence Score object.

It defines:

* what a Confidence Score represents;
* what a Confidence Score SHALL preserve;
* what a Confidence Score SHALL NOT represent.

This specification does not define algorithms.

---

## 3. Definitions

### Confidence Score

A Confidence Score represents the evidential strength supporting one Attachment between exactly one Artifact and exactly one Workspace.

Confidence Score is computational.

Confidence Score is deterministic.

Confidence Score is replayable.

Confidence Score is accountable to Artifact evidence.

Confidence Score is never directly observed.

---

## 4. Ownership

Confidence Score belongs exclusively to one Attachment.

Confidence Score SHALL NOT exist independently of an Attachment.

Confidence Score SHALL NOT be shared between Attachments.

---

## 5. Canonical Properties

Confidence Score SHALL:

* represent evidential strength only;
* remain deterministic;
* remain replayable;
* remain accountable to Artifact evidence.

Confidence Score SHALL NOT:

* represent importance;
* represent priority;
* represent user intent;
* represent semantic meaning;
* represent ranking;
* represent retrieval quality.

---

## 5A. Derivation Contract

Engagement Derivation is solely responsible for deriving Confidence Scores.

**Amended by RFC-0014.** Superseded clause: *"Workspace Formation is solely responsible for deriving Confidence Scores."* Workspace Formation no longer exists as a stage — IS-0012 is superseded in full and the Workspace is projected on read rather than formed on write (IS-0011 Amendment A1.1). The responsibility is unchanged in substance and has moved to the stage that now holds it.

Confidence Score derivation SHALL consume only:

* the canonical Artifact under evaluation;
* one Candidate Workspace;
* canonical computational objects owned by that Candidate Workspace;
* the relational evidence establishing a relationship between the Artifact under evaluation and the other participants in that body of work — temporal co-presence, interleaved attention, shared vocabulary, and shared containment, each normalized against ubiquity (RFC-0014 Requirements 3 and 4).

**Amended by RFC-0014.** Superseded clause: *"canonical co-membership evidence — accepted canonical Observations of the Co-Membership Evidence class (RFC-0012) — establishing a relationship between the Artifact under evaluation and canonical computational objects owned by that Candidate Workspace."*

Why it changed: the clause named repository co-membership as *the* relational input. It is Git-specific, so it yields no confidence at all for the majority of bodies of work, and it is identical for related and unrelated activities inside one repository, so it cannot separate them. See RFC-0012's Supersession Record. Shared containment survives as one relational signal among several, carrying no privileged weight and no knowledge of what kind of container it is.

Confidence Score derivation SHALL ALSO NOT consume application identity, domain, file extension, or profession, or any allowlist or blocklist of these (RFC-0014 Requirement 3).

Confidence Score derivation SHALL NOT consume:

* user state;
* runtime state;
* retrieval results;
* restoration state;
* language model output;
* learned behavior;
* non-canonical representations.

Confidence Score derivation SHALL produce exactly one Confidence Score for every Attachment Evaluation.

Given identical canonical inputs, identical Confidence Scores SHALL be produced.

This specification intentionally does not define the concrete derivation algorithm.

---

## 6. Required Invariants

CS-1

Every Attachment SHALL own exactly one Confidence Score.

CS-2

Every Confidence Score SHALL belong to exactly one Attachment.

CS-3

Confidence Score SHALL represent evidential strength only.

CS-4

Confidence Score SHALL remain immutable after Attachment construction.

CS-5

Confidence Score SHALL remain replayable.

CS-6

Confidence Score SHALL remain reproducible from canonical lower-layer computational objects.

---

## 7. Non-Responsibilities

Confidence Score SHALL NOT:

* determine Workspace membership;
* perform Workspace Recognition;
* determine Workspace Identity;
* perform Retrieval;
* perform Restoration;
* perform Learning;
* infer semantic meaning;
* infer user intent.

---

## 8. Architectural Rationale

Confidence Score exists solely to preserve the evidential strength associated with one Attachment.

Workspace Formation determines the value of a Confidence Score.

This specification intentionally does not define how Confidence Scores are computed.

Computation belongs exclusively to Workspace Formation.

---

## 9. Dependencies

This specification depends on:

* Constitution
* Product
* Architecture
* RFC-0003 — Workspace (v2.0)
* RFC-0014 — Engagement Contract (supersedes IS-0012 in full)
* IS-0011 — Workspace Model (as amended, Amendment 1)

Implementations SHALL NOT contradict any dependency listed above.