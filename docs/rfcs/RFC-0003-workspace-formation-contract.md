# RFC-0003 — Workspace Formation Contract

Status: Accepted

Version: 2.0

Amended By: RFC-0014 (Engagement Contract) — see Amendment Record

Depends On:

- Constitution
- Cognitive Model
- Product
- Architecture
- Architectural Laws
- RFC-0000
- RFC-0001
- RFC-0002
- RFC-0014

---

# Amendment Record

## v2.0 — Requirement 1, Requirement 2, and Non-Goals

Three rules in v1.0 forbade behaviour the product requires. Each is recorded
below with the old rule, why it prevented Evo from working, and the rule that
replaces it. Nothing is removed silently; the superseded wording is quoted in
full.

### Amendment 1 — Requirement 2 no longer forbids temporal proximity

**Old rule (v1.0, Requirement 2).**

> "Workspace Formation MUST NOT explain topical similarity, application
> similarity, temporal proximity, file organization, or any other isolated
> observational feature."

**Why it prevented the product from working.** The prohibition conflated two
different things under one word. *Proximity as a sufficient condition* — "these
were seen near each other, therefore they are one Workspace" — is genuinely
wrong and remains forbidden. But *temporal structure as evidence* is the
strongest signal available that heterogeneous resources belong to one body of
work, and frequently the only one. A document called `assignment.md`, a terminal,
and a browser page about a library share no vocabulary, no directory, no
repository, and no application; the sole witnessed fact that relates them is
that a person moved between them, repeatedly, in one sitting. Forbidding
temporal reading therefore left nothing with which to relate anything, and the
only surviving basis for a Workspace was a property of one resource in
isolation — which is exactly the per-resource rule that made Home a list of
resources.

**New rule.** Temporal relationships between Acts ARE admissible evidence of
work continuity, subject to two bounds: proximity alone MUST NOT constitute a
body of work, and presence MUST NOT be inferred across an interval too long to
be one continuous stretch of attending. Normative statement: RFC-0014
Requirement 4.

The v1.0 prohibitions on topical similarity, application similarity, and file
organization *as sufficient conditions* are retained and strengthened: RFC-0014
Requirement 3 additionally forbids consulting application identity at all.

### Amendment 2 — Requirement 2's repository clause is withdrawn

**Old rule (v1.0, Requirement 2, final paragraph).**

> "Repository co-membership evidence, as defined by RFC-0012, is a relational
> continuity signal and MAY contribute evidence of Workspace continuity; it
> SHALL NEVER define Workspace identity alone."

**Why it prevented the product from working.** It elevated one Git-specific
relationship to named status in the formation contract. Most work is not in a
repository — browser and PDF and terminal, spreadsheet and notes, drawing and
folder — so a contract that names repository membership and nothing else makes
every non-Git body of work second-class. It also over-groups precisely where
separation matters most: two unrelated activities in one repository share the
signal completely.

**New rule.** Shared containment is one structural relationship among several,
carrying no privileged weight and no dependence on Git. RFC-0012 is superseded
in full by RFC-0014; see RFC-0014 "Architectural Consequences".

### Amendment 3 — Non-Goals no longer excludes importance and restoration order

**Old rule (v1.0, Non-Goals).**

> "Workspace Formation is not responsible for: … determining restoration order;
> determining importance; determining relevance …"

**Why it prevented the product from working.** Restoration must be selective:
opening all sixteen resources of a body of work hands the person back the exact
reload cost the product exists to remove. Selectivity requires knowing which
members matter to resuming — that is, importance. With importance excluded from
formation and no other layer holding the evidence, the only available selection
rule was "all members", and restoration opened everything.

**New rule.** Formation determines each participant's **role**, which is its
importance *to resuming* — distinct from strength of membership, and separately
derived. Normative statement: RFC-0014 Requirement 8. Restoration consumes roles
rather than computing importance itself (RFC-0014 Requirement 10), so the
separation of concerns the old Non-Goal protected is preserved: formation states
importance, restoration acts on it.

Still excluded, unchanged: determining user goals, project ownership, task
completion, and predicting future work.

### Amendment 4 — Requirement 1's evidence basis is widened

**Old rule (v1.0, Requirement 1).**

> "Every Workspace MUST be derived exclusively from Artifact histories."

**Why it prevented the product from working.** Read strictly, it admits only the
per-Artifact history as evidence and so excludes the relational and temporal
evidence a body of work is made of. It is also narrower than the invariant it
was protecting, which is that nothing may be invented — not that Artifact
history is the only permitted shape of derived evidence.

**New rule.** Every Workspace MUST be derived exclusively from the canonical
Observation history, through derived intermediate representations (Acts,
Episodes, the Attention Ledger, Engagements) that are themselves pure functions
of it. No Workspace may rest on anything not traceable to a canonical
Observation. The invariant is unchanged in force; only the permitted shape of
derived evidence is widened.

### Reconciliation of dependent documents

- **RFC-0012** — superseded in full (Status header updated).
- **Architecture** §3, §4, §5, §6, §7, §9, §12 — amended; see the Amendment
  Record in `ARCHITECTURE.md`.
- **IS-0011, IS-0012, IS-0013** — amended; see each document's Amendment
  Record.
- **RFC-0013** — unaffected: declaration remains ground truth (RFC-0014
  Requirement 11).

### Implementation contracts and tests

Implemented by `evo-engagement` (Acts, Episodes, attention, affinity, grouping,
roles, naming), `evo-workspace::projection` (Engagement → Workspace), and
`evo-daemon::understanding`. Behavioural coverage for each amended rule lives in
the adversarial scenario suite (`evo-daemon`'s `verify_scenarios` example,
scenarios A–O) and in the unit tests of the modules named above; Amendment 1 is
covered by scenarios E, F, and G, Amendment 3 by scenarios I and J.

---

# Abstract

This RFC defines the behavioral contract of Workspace Formation.

Workspace Formation is the process through which Evo constructs its current best explanation that multiple Artifact histories collectively describe the evolution of one coherent body of work.

A Workspace is not directly observed.

A Workspace is not a container.

A Workspace is an explanatory hypothesis derived from Artifact histories.

This RFC defines the guarantees every Workspace Formation process MUST preserve, independent of implementation.

---

# Motivation

Human work persists while the artifacts through which it is expressed continuously change.

Applications are replaced.

Files are renamed.

Repositories are reorganized.

Documents are rewritten.

Tools evolve.

Yet the work itself continues.

A computational system that wishes to restore work continuity therefore requires an object whose identity survives changes in the supporting artifacts.

Workspace exists to provide that continuity.

---

# Definition

A Workspace is Evo's current best explanatory hypothesis that a collection of Artifact histories collectively describe the evolution of one coherent body of work.

A Workspace is inferred.

It is never directly observed.

Artifacts do not belong to a Workspace.

Rather, Artifact histories provide evidence supporting a Workspace hypothesis.

Workspace identity exists independently of any individual supporting Artifact.

---

# Scope

This RFC defines:

- what a Workspace is;
- how Workspace identity behaves;
- what guarantees Workspace Formation provides.

This RFC intentionally does not define:

- formation algorithms;
- clustering techniques;
- similarity metrics;
- confidence calculations;
- ranking;
- retrieval;
- restoration;
- learning.

Those responsibilities belong to later specifications.

---

# Behavioral Contract

## Requirement 1 — Derived Explanation

Every Workspace MUST be derived exclusively from the canonical Observation
history, through derived intermediate representations that are themselves pure
functions of that history.

A Workspace MUST NOT exist independently of supporting canonical evidence.

Nothing in a Workspace may rest on anything not traceable to a canonical
Observation. (Amended in v2.0 — Amendment 4.)

---

## Requirement 2 — Continuity, Not Similarity

Workspace Formation MUST explain continuity of evolving work.

Workspace Formation MUST NOT treat topical similarity, application similarity,
temporal proximity, file organization, or any other single observational feature
as *sufficient* to establish a Workspace.

Similarity may contribute evidence.

Similarity MUST NEVER define Workspace identity.

Temporal relationships between Acts ARE admissible evidence of continuity,
bounded as RFC-0014 Requirement 4 defines: proximity alone establishes nothing,
and presence is never inferred across silence. (Amended in v2.0 — Amendment 1.)

Application identity MUST NOT be consulted at all (RFC-0014 Requirement 3).

---

## Requirement 3 — Explanatory Nature

A Workspace is an explanatory hypothesis.

A Workspace MUST NEVER be treated as a container of Artifacts.

Artifacts support a Workspace.

A Workspace does not own Artifacts.

---

## Requirement 4 — Identity Through Evolution

Workspace identity is defined by the continued evolution of one coherent body of work.

Workspace identity MUST remain independent of changes in individual supporting Artifacts.

Adding, removing, replacing, or modifying supporting Artifacts MUST NOT alone create or destroy Workspace identity.

---

## Requirement 5 — Historical Basis

Workspace Formation MUST explain the observed evolution of Artifact histories.

Workspace Formation MUST NOT be based solely upon instantaneous system state.

Historical continuity is fundamental.

Instantaneous state is supporting evidence only.

---

## Requirement 6 — Provisional Identity

Every Workspace represents Evo's current best explanatory hypothesis.

Workspace identity is never absolute.

Improved evidence or improved Formation MAY produce a different Workspace hypothesis.

Historical Observations and Artifacts MUST remain unchanged.

---

## Requirement 7 — Replayability

Workspace Formation MUST be fully reproducible from the canonical Observation history.

Replay MUST regenerate Workspace hypotheses rather than mutate historical evidence.

Replay improves explanation.

Replay does not rewrite history.

---

## Requirement 8 — Independence From Hidden State

Workspace Formation MUST depend only upon observable evidence.

Workspace Formation MUST NOT depend upon inferred user intention, subjective motivation, internal mental state, future knowledge, or information unavailable through the observer model defined by the Constitution.

---

## Requirement 9 — Independence From Higher Computation

Workspace Formation MUST occur independently of Retrieval, Restoration, Learning, or future computational layers.

Higher computational objects may consume Workspace hypotheses.

They MUST NOT define Workspace identity.

---

# Guarantees

Every compliant Workspace guarantees:

- derivation from the canonical Observation history;
- explanatory rather than container semantics;
- provisional identity;
- historical continuity;
- replayability;
- independence from hidden mental state;
- independence from higher computational systems.

---

# Forbidden Behavior

A Workspace MUST NEVER:

- represent objective truth;
- contain Artifacts;
- encode user intention;
- encode semantic purpose as observed fact;
- depend upon future information;
- overwrite historical Artifacts;
- overwrite historical Observations;
- become immutable;
- become canonical history.

Violation of any of these behaviors invalidates compliance with this RFC.

---

# Architectural Consequences

Workspace Formation establishes the first computational representation of work continuity.

Observation explains what was witnessed.

Artifact Identity explains which observations refer to the same external entity.

Workspace Formation explains why Artifact histories collectively describe one evolving body of work.

Every higher-level capability in Evo—including Retrieval, Restoration, Search, Knowledge Formation, and Learning—depends upon Workspace hypotheses rather than constructing independent interpretations of work continuity.

---

# Non-Goals

Workspace Formation is not responsible for:

- determining user goals;
- determining project ownership;
- determining task completion;
- predicting future work.

Workspace Formation DOES determine each participant's role — its importance to
resuming — which Restoration consumes rather than recomputes. (Amended in
v2.0 — Amendment 3; normative statement in RFC-0014 Requirements 8 and 10.)

Workspace Formation answers only one computational question:

"Which resources are best explained as the evolution of one coherent body of
work, and how much does each matter to resuming it?"

---

# Compatibility

Any implementation conforms to this RFC if it preserves every behavioral guarantee defined herein.

No particular formation strategy is prescribed.

Behavioral compliance is independent of implementation.

---

# Rationale

Workspace is the first computational object that represents continuity rather than identity.

Unlike Observations, which witness facts, or Artifacts, which establish entity identity, Workspaces explain how work evolves across time despite continual change in the artifacts through which that work is expressed.

By treating Workspaces as explanatory hypotheses rather than containers, Evo preserves replayability, continual improvement, architectural flexibility, and strict separation between observed history and inferred understanding.

Workspace Formation therefore provides the computational bridge between observed history and the reconstruction of human work continuity while remaining permanently accountable to the evidence from which it was derived.