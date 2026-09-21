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

4. Canonical Vocabulary

The Observation Language SHALL define the canonical concepts used to express directly witnessed observations.

The initial vocabulary consists only of the following four observation concepts:

4.1 Window Focus Gained

WindowFocusGained

Represents the directly witnessed fact that a window gained focus.

The concept SHALL describe only the occurrence of the focus change and the window to which the observation refers.

It SHALL NOT imply:

* user intent;
* application purpose;
* task membership;
* Workspace membership;
* importance;
* relevance;
* activity classification;
* semantic interpretation.

⸻

4.2 File Saved

FileSaved

Represents the directly witnessed fact that a file was saved.

The concept SHALL describe only the occurrence of the save event and the file to which the observation refers.

It SHALL NOT imply:

* why the file was saved;
* what the user intended;
* whether the file is important;
* Workspace membership;
* task membership;
* semantic meaning of the file;
* any interpretation of the work being performed.

⸻

4.3 URL Navigated

URLNavigated

Represents the directly witnessed fact that navigation occurred to a URL.

The concept SHALL describe only the observed navigation event and the URL to which the observation refers.

It SHALL NOT imply:

* why the URL was visited;
* what the user was researching;
* whether the page was read;
* whether the URL is relevant;
* task membership;
* Workspace membership;
* semantic interpretation of the navigation.

⸻

4.4 Commit Made

CommitMade

Represents the directly witnessed fact that a commit was made.

The concept SHALL describe only the observed commit event and the commit to which the observation refers.

It SHALL NOT imply:

* why the commit was made;
* the purpose of the commit;
* whether the commit represents completion of a task;
* task membership;
* Workspace membership;
* importance;
* semantic interpretation.

⸻

4.5 Work Designated

WorkDesignated

Represents the directly witnessed fact that the user declared that their work continues at a specific subject.

The concept SHALL describe only the declaration act and the subject the user designated.

It SHALL NOT imply:

* that the user will actually perform further work at the subject;
* any prediction of future behavior;
* that the work is unfinished;
* Workspace membership;
* importance, priority, or relevance;
* any classification or interpretation of the work;
* task membership;
* semantic interpretation.

The declaration is a directly witnessed act through a trusted capture origin (the user's own explicit action in Evo). Its truth does not require interpretation: the user performed the declaration, and Evo witnessed it. The concept SHALL NOT be used to record an inference about what the user intends.

⸻

4.6 Repository Membership

RepositoryMembership

Represents the directly witnessed fact that an entity F (a file, directory, or commit) is a member of a repository R.

The concept SHALL describe only the witnessed membership relationship between the member entity and the repository identity.

It SHALL NOT imply:

* user intent;
* that the member is important;
* that the repository is one unique body of work;
* task membership;
* Workspace membership;
* that the member should be restored;
* semantic interpretation.

The concept records a witnessed structural fact, never Evo's interpretation. The member subject is the entity the fact concerns; the repository identity is a canonical subject-valued value.

⸻

4.7 Work Grouped

WorkGrouped

Represents the directly witnessed fact that the user explicitly declared two already-witnessed subjects to be related work.

The concept SHALL describe only the declaration act and the two subjects the user declared related.

It SHALL NOT imply:

* that the user will perform further work at either subject;
* any prediction of future behavior;
* importance, priority, or relevance;
* Workspace membership;
* task membership;
* semantic interpretation.

The declaration is a directly witnessed act through a trusted capture origin (the user's own explicit action in Evo). It records the act itself, never an inference about the user's intent or about whether the subjects actually belong to one body of work.

⸻

4.8 Continuation Surface

ContinuationSurface

Represents the directly witnessed fact that the user explicitly declared that a set of already-witnessed subjects currently constitutes the continuation of their work.

The concept SHALL describe only the declaration act and the declared set of subjects (canonically ordered: the lexicographically smallest subject is the Required Subject; the remainder follow in ascending order).

It SHALL NOT imply:

* that the work is unfinished;
* that any declared subject is important, urgent, or restorable now;
* that any declared subject should be opened, focused, or executed;
* Workspace membership (a surface declaration does not establish membership — that is Workspace Formation's question);
* task membership;
* that the declared subjects are the same Artifact;
* any relationship between the subjects beyond their joint declaration;
* semantic interpretation.

The declaration is a directly witnessed act through a trusted capture origin (the user's own explicit action in Evo). It records the act itself, never an inference about the user's intent or about whether the subjects should be restored. The concept SHALL NOT be used to record recency, frequency, focus, co-membership, or any derived relevance: the declaration is exactly what the user declared.

⸻

5. Canonical Concept Semantics

Canonical Observation Language concepts SHALL be platform-independent.

A canonical concept SHALL describe the semantic fact observed by Evo rather than the mechanism through which a platform exposed that fact.

For example, a platform-specific signal such as an operating-system focus notification MAY be translated into WindowFocusGained.

The platform-specific signal itself SHALL NOT become part of the canonical Observation Language.

The Observation Language SHALL NOT contain platform-specific concepts such as:

* operating-system notification types;
* operating-system event types;
* platform-specific window objects;
* process objects;
* browser-specific event types;
* filesystem API types;
* version-control implementation types.

Platform-specific capture mechanisms belong to the Collector boundary and SHALL be translated before entering the canonical Observation Language.

⸻

6. Direct-Witness Constraint

Every canonical Observation Language concept SHALL represent information that Evo directly witnessed.

Canonical concepts SHALL NOT represent information inferred from one or more observations.

In particular, the Observation Language SHALL NOT contain concepts representing:

* user intent;
* task identification;
* Workspace membership;
* importance;
* relevance;
* activity classification;
* summaries;
* predictions;
* recommendations;
* semantic interpretation.

A concept that requires interpretation to establish its truth SHALL NOT be a canonical Observation Language concept.

This requirement preserves the separation between witnessed information and inferred information established by the architecture.

⸻

7. Observation Context

Canonical concepts MAY contain the minimum subject information necessary to express the directly witnessed fact.

Such subject information SHALL identify what the observation directly concerns.

Subject information SHALL NOT constitute Artifact Identity.

Artifact Identity SHALL remain the responsibility of the Artifact layer.

The Observation Language SHALL therefore not define Artifact identity algorithms, identity resolution rules, or stable Artifact identifiers.

⸻

8. Provenance and Temporal Information

Timestamp and Provenance are properties of an Observation and its acceptance contract.

They are not canonical Observation Language concepts.

The Observation Language SHALL NOT introduce separate concepts for:

* Observation timestamp;
* Provenance;
* Observation identity;
* Schema identity.

These are governed by the Observation Model and Observation Acceptance specifications.

In particular, a timestamp SHALL describe when the observation occurred or was recorded according to the applicable Observation and Provenance contract; it SHALL NOT become part of the semantic vocabulary merely because every Observation possesses one.

⸻

9. Vocabulary Evolution

The canonical vocabulary MAY be extended in future versions.

A new canonical concept SHALL be introduced only when:

1. it represents a directly observable fact;
2. it has stable platform-independent semantics;
3. it cannot be adequately represented by an existing canonical concept;
4. its addition is justified by the architecture or demonstrated system requirements.

A platform-specific signal alone SHALL NOT justify creation of a new canonical concept.

Existing canonical concepts SHALL NOT silently change semantic meaning.

A semantic change to an existing concept SHALL require a new version according to the applicable language and schema evolution rules.

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