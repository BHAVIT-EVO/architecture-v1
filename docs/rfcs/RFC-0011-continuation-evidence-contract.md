# RFC-0011 — Continuation Evidence Contract (Work Designation)

**Status:** Accepted

**Version:** 1.1

---

## Amendment Record

### v1.1 — Declaration-triggered origination is retired

**Old rule (v1.0, §4).**

> "As an explicit user declaration it MAY justify the origination of a Workspace
> for the canonical Artifact it names when that Artifact belongs to no remembered
> Workspace (explicit user declaration is sufficient origination evidence and
> outranks every measurement …); it never re-originates an Artifact that already
> belongs to a Workspace."

**Why it prevented the product from working.** The clause existed to patch a
defect from the other side. Because Formation decided membership one Observation at
a time — without the corpus that determines it — it could not see that a declared
Resource belonged with anything, so the runtime had to *manufacture* a Workspace
for it by hand and persist the result. Two consequences followed. A declaration
became a Workspace-creating act, which is the one-resource-per-Workspace failure
reached by the shortest possible route: designate a file, get a body of work
containing that file alone. And the guard "when that Artifact belongs to no
remembered Workspace" made the outcome depend on *when* the user declared relative
to what had been persisted, so the same declaration over the same evidence gave
different structures depending on history — which Requirement 12's replay
equivalence forbids.

**New rule.** There is no origination step. Bodies of work are derived from the
whole accumulated corpus (RFC-0014 Requirements 1–7), so a declaration needs no
special path: it enters the derivation as ground truth and short-circuits the
significance test for the group it names, which both honours §10 ("the user's
judgement outranks inference") and keeps the result a function of the evidence
alone. A declaration still never establishes an Artifact — that half of the v1.0
rule is unchanged and load-bearing.

Nothing about the user's authority is reduced by this. A declared group is still
guaranteed to appear as a body of work; it simply arrives by being *recognised* in
the reconstruction rather than by being *written* beside it. ARCH-PROP-0001, which
proposed widening declaration-triggered origination instead of retiring it, is
recorded **NOT ADOPTED** for this reason.

**Reconciliation.** Architecture Amendment 1 (§3–§7, §9, §12) removes the
persisted Workspace this clause wrote to. RFC-0014 Requirement 11 states the
declaration guarantee at contract level. IS-0012 and IS-0013 are superseded in
full. `evo_daemon::runtime::VerticalPipeline` carries the same record at the point
of definition.

**Tests.** The declaration round-trip and grouping end-to-end tests in
`evo-daemon`, `evo-engagement`'s declaration short-circuit tests, and the
adversarial-scenario harness `verify_scenarios` (restart and replay equivalence,
K and L).

---

**Depends On:**
- Constitution
- Product
- Architecture
- Architectural Laws
- RFC-0000
- RFC-0001
- RFC-0002
- RFC-0003
- RFC-0010
- IS-0001
- IS-0002
- IS-0003
- IS-0021

---

# Abstract

RFC-0010 defines Work Continuity as a derived relationship that may be established
only from canonical evidence satisfying Requirements 1–7. It leaves two questions to
the evidence milestone: whether the evidence may be user-directed (Open Question 2),
and the exact schema (deferred to an Observation/IS milestone).

This RFC ratifies **explicit user designation** — a directly witnessed declaration by
the user that their work continues at a specific subject — as the continuation
evidence class. It defines the smallest canonical evidence contract that can
truthfully establish Work Continuity:

- one new Observation Language concept (`WorkDesignated`);
- one new Observation Schema (`OBS-WORK-DESIGNATED/v1`) with exactly one required
  canonical concept and one required subject, no optional concepts;
- one currency rule (a designation is current until a later designation supersedes it);
- one derivation rule (the current designation's resolved Artifact is the
  continuation target, resolving the IS-0021 §25.2 multi-candidate insufficiency and
  establishing the §25.5 Next Step).

No new architectural primitive, no new field on any frozen object, no new persisted
canonical object, and no Workspace/Snapshot/Artifact semantic change is required.

---

# Motivation

Evo's product promise is that the user remembers only what they want to do, and Evo
reconstructs the environment required to continue (Product, "The Promise"). The
frozen contracts reach the boundary of that promise at IS-0021 §25.5: no canonical
continuation evidence exists, so no Next Step can be derived. RFC-0010 and IS-0021
§25.11 require canonical continuation evidence to enter through the Observation
layer before IS-0021 may be amended.

Every inferred signal — recency, frequency, focus, co-membership, commit,
modification, unresolved conditions — produces identical evidence in the two worlds
the competing-world test distinguishes and is prohibited by RFC-0010. The user's own
declaration is the one evidence class that is both canonical and discriminating: it
is a witnessed fact about the state of the work, supplied by the person whose work
it is, and it never requires Evo to guess.

---

# Scope

This RFC defines:

- the evidence candidate analysis and the competing-world test;
- the surviving evidence class (Work Designation) and its semantic boundary;
- the canonical concept and Observation Schema;
- the producer (witness) contract;
- the resolution rule by which a designation references an existing canonical
  Artifact;
- the derivation rule and insufficient-evidence behavior;
- the compatibility analysis against frozen documents;
- the architectural sequence that authorizes implementation.

This RFC does NOT define:

- implementation;
- capture mechanisms beyond the producer contract;
- execution;
- resource identity or executable-target identity;
- ranking, recommendation, or retrieval;
- Blockers or unresolved-work evidence (deferred; unchanged: zero Blockers);
- Context Chain semantics (unchanged: empty).

---

# Definitions

**Work Designation**

A directly witnessed declaration by the user that their work continues at a specific
subject.

**Designation Subject**

The canonical subject string (e.g., a file path, URL, or window subject) the user
designated — the same subject semantics as the four frozen schemas (IS-0003 §4.3).

**Current Designation**

The most recent WorkDesignated observation by canonical Observation Time. A later
designation supersedes all earlier ones. This supersession rule is the evidence
class's own defined semantic — a declaration is current until replaced — not a
chronological inference over unrelated evidence.

**Continuation Target**

The canonical Artifact resolved from the current designation by the resolution rule,
when that Artifact belongs to the Workspace under evaluation.

---

# Evidence Candidate Analysis

## The Competing-World Test

A proposed rule is sufficient evidence for Work Continuity only when it produces
different evidence in:

- **WORLD A** — the work genuinely continues with B;
- **WORLD B** — A and B are merely associated, or the work was abandoned.

A rule whose evidence is identical in both worlds cannot distinguish continuation
from association and is rejected. This is consistent with RFC-0010's distinction
between Work Continuity (E) and Chronology (A), Association (B), Identity Evolution
(C), and Prediction (D).

## Candidate-by-Candidate Results

| # | Candidate evidence rule | WORLD A evidence | WORLD B evidence | Verdict |
|---|---|---|---|---|
| 1 | Observation order / recency | B after A | B after A | **Reject** — identical; prohibited |
| 2 | Frequency | B most frequent | B most frequent | **Reject** — identical; prohibited |
| 3 | Window focus | B focused | B focused | **Reject** — identical; RFC-0010 prohibits |
| 4 | Window titles / application identity | same titles | same titles | **Reject** — identical; prohibited |
| 5 | File modification | B modified | B modified | **Reject** — identical; prohibited |
| 6 | Commit | commit exists | commit exists | **Reject** — identical; prohibited |
| 7 | Co-membership (repo / commit / Workspace) | A, B co-member | A, B co-member | **Reject** — identical; prohibited |
| 8 | Temporal co-occurrence / session grouping | A, B co-occur | A, B co-occur | **Reject** — identical; inference; IS-0001 §5A.1 |
| 9 | Unresolved conditions | condition present | condition present | **Reject for continuation** — RFC-0010; MAY later support §25.4 Blockers only |
| 10 | Artifact supersession / identity evolution | B supersedes A | B supersedes A | **Reject** — identical; RFC-0010 distinction C |
| 11 | Semantic similarity / LLM output | same | same | **Reject** — prohibited; non-deterministic |
| 12 | Runtime / OS / UI state at derivation | same | same | **Reject** — RFC-0010 Requirement 6 |
| 13 | Attachment confidence / ordering | same | same | **Reject** — W-7; IS-0021 §25.1 |
| 14 | **Explicit user designation** | **user declared continuation at B** | **no declaration** | **SURVIVES** — the evidence differs |

## Why Designation Survives — and Its Honest Limit

The declaration is the only candidate whose evidence records the user's own
expression rather than Evo's inference. Its truth condition is the performance of
the act ("the user declared continuation at subject S"), which is directly
observable through a trusted capture origin and requires no interpretation — the
same epistemic pattern as "window X gained focus."

**Steelman — designation then abandonment.** If the user designates B and later
abandons the work, the evidence "user designated B" is identical in the continuation
world and the abandonment world. This does not reject the class: Work Continuity is
explicitly not a prediction (RFC-0010). The derivation establishes the state of the
work *as of the latest canonical evidence*: at the moment of declaration, the work's
state IS "continues at B." Abandonment is a later state change no observation class
can witness — the same limitation the four frozen classes share. Designation is the
strongest truthfully available evidence because it is the user's word, and its
staleness is made visible by carrying provenance (time) into presentation.

**Steelman — "designate everything."** If the user designates several subjects, the
supersession rule selects the current (latest) declaration; presentation shows the
declaration evidence and its time, so the user sees exactly what Evo derived and why.
Recency is not used as a selector over Artifacts: the selector is the designation
evidence itself; chronology orders only declarations of this one class, under a
semantic meaning the contract defines (RFC-0010 Requirement 3 permits the contract
to define its meaning explicitly — it is not *recovered* from chronology).

---

# The Evidence Class

## 1. Observation Language Concept (IS-0002 amendment)

**Concept name:** `WorkDesignated`

**Semantics:** the directly witnessed fact that the user declared that their work
continues at a specific subject.

**Semantic boundary — SHALL NOT imply:**

- that the user will actually perform further work at the subject;
- any prediction of future behavior;
- that the work is unfinished;
- Workspace membership;
- importance, priority, or relevance;
- any classification or interpretation of the work.

**IS-0002 §9 conformance (new-concept gate):**

1. *Directly observable fact* — YES. The truth condition is the performance of the
   declaration act through a trusted capture origin.
2. *Stable platform-independent semantics* — YES. The declaration is meaningful on
   every platform; the canonical representation carries no platform-specific value.
3. *Cannot be represented by an existing concept* — YES. No frozen concept records
   declarations; Provenance may not carry it (IS-0001 §5A.1); the four schemas have
   no optional concepts (IS-0003 §4.1).
4. *Justified by architecture or demonstrated requirements* — YES. RFC-0010
   §25.11/§25.5 requires canonical continuation evidence; TRACE-0001 phase 5 requires
   a Restore Plan; the Product promise requires continuity.

**IS-0002 §6 conformance (Direct-Witness Constraint):** the concept records the
declaration act, not an inferred mental state. The concept's truth does not require
interpretation. This resolves RFC-0010 Open Question 2 in the affirmative: a
witnessed work-state fact MAY be user-directed, because the user's own declaration
is the witnessed fact.

## 2. Observation Schema (IS-0003 amendment, §4.1.5)

- **Schema Identifier:** `OBS-WORK-DESIGNATED`
- **Schema Version:** 1
- **Observation Type:** WorkDesignated
- **Required Canonical Concept:** `WorkDesignated`
- **Required Subject:** the subject the user designated as the continuation of their
  work.
- **Optional Canonical Concepts:** None.
- **Platform neutrality:** the schema SHALL NOT require application identity, process
  identity, browser identity, or any other platform-specific value (mirrors
  §4.1.1–§4.1.4).
- **Subject identity boundary (§4.3):** the Required Subject SHALL NOT be interpreted
  as canonical Artifact Identity; the Artifact layer resolves it.

The new schema is a new schema identity added to the initial set (the word "initial"
in IS-0003 §4.1 leaves the set open). No existing schema changes; no existing
Observation migrates; previously accepted Observations replay identically.

## 3. Producer / Witness Contract

**Producer:** the Evo desktop application's designation surface — a user action
marking a witnessed subject as "where my work continues."

**Witness:** the user, acting through Evo's own trusted capture origin
(`user_designation` Observation Source).

**Provenance (IS-0001 §5A):** Observation Source = `user_designation`;
Observation Time = when the declaration was witnessed; Observation Context = directly
observed circumstances only. Provenance SHALL NOT contain interpretation of the
declaration.

**Replayability:** the declaration is persisted as a canonical Observation in the
append-only log and participates in replay exactly like the four frozen classes.

## 4. Resolution Rule — Referencing an Existing Canonical Artifact

A WorkDesignated observation does NOT establish a new external entity. It references
the entity the user designated, which other canonical Observations have already
established. Therefore:

- a WorkDesignated observation SHALL be accepted and persisted through the standard
  Observation acceptance pipeline (IS-0001), exactly like any canonical Observation;
- a WorkDesignated observation SHALL NOT by itself establish an Artifact (RFC-0002:
  "Observations MAY support one Artifact"; a designation references, it does not create).
  It SHALL NOT originate a Workspace either, because under RFC-0014 there is no
  origination step to invoke: bodies of work are derived from the whole corpus, and a
  declaration is ground truth *inside* that derivation, which is where the user's
  judgement belongs (RFC-0014 Requirement 11). See the v1.1 Amendment below;
- the designation's subject resolves to the canonical Artifact established by the
  content Observations of that subject (OBS-FILE-SAVED, OBS-URL-NAVIGATED,
  OBS-WINDOW-FOCUS-GAINED, OBS-COMMIT-MADE), through the deterministic identity
  derivation of the Artifact layer;
- if the designated subject resolves to no Artifact, or to more than one distinct
  Artifact, resolution is absent — the designation is rejected by the producer
  before any candidate Observation is created, so no partial canonical designation
  ever exists;
- the producer (desktop) SHALL offer only subjects already witnessed in the canonical
  Observation log.

This keeps the designation honest: it never manufactures an Artifact, never uses
fuzzy or similarity matching, and never invents identity.

## 5. Derivation Rule (IS-0021 §25 amendment per §25.11)

Inputs (amended §25.1): the selected canonical Snapshot; the canonical Workspace; the
canonical Artifacts referenced by the Snapshot's Attachments; the canonical
Attachment records; and the canonical designated Artifact — the Artifact resolved
from the current WorkDesignated evidence by the resolution rule, presented as
canonical Artifact-level understanding (RFC-0010 Requirement 1: evidence represented
in canonical understanding, never raw Observations consumed by derivation).

**Currency rule:** the current designation is the most recent WorkDesignated
observation by canonical Observation Time; where times are equal, the later
append-order record is current.

**§25.2 Resume Point:** when the current designation's resolved Artifact is among the
selected Snapshot's eligible candidates, that Artifact SHALL be the Resume Point,
resolving the multi-candidate insufficiency.

**§25.5 Next Step:** the Next Step is established iff the current designation's
resolved Artifact is represented by the selected Snapshot. That Artifact is the
continuation target. Semantic Meaning: "the user declared that the work continues at
this Artifact."

**§25.3 Context Chain:** unchanged — empty (no canonical supporting relationship).

**§25.4 Blockers:** unchanged — zero (no canonical unresolved-work representation).

**§25.6 RestorationPlan Construction:** a Complete plan is constructible — Resume
Point and Next Step derive from the current designation; Context Chain (empty) and
Blockers (zero) are already derived values.

## 6. Insufficient-Evidence Behavior (IS-0021 §25.8, unchanged in kind)

- No current designation → Next Step insufficient, exactly as before this RFC.
- Current designation resolving outside the Workspace → insufficient for that
  Workspace (§25.7 cross-consistency).
- No confidence, no fallback, no guessing. Silence is preserved.

## 7. Invalid States

- A WorkDesignated observation whose subject is empty or structurally invalid —
  rejected at acceptance (IS-0001).
- A designation used to establish a component in a Workspace whose Artifacts do not
  contain the designated Artifact — invalid (§25.7).
- A designation presented as a prediction of user behavior — invalid (RFC-0010,
  distinction D).

---

# Compatibility Analysis (frozen documents)

| Document | Clause | Assessment |
|---|---|---|
| IS-0002 | §6 Direct-Witness Constraint | Compatible: the declaration is witnessed, not inferred |
| IS-0002 | §9 Vocabulary Evolution | All four conditions satisfied (above) |
| IS-0003 | §4.1 initial set / §5 evolution | Compatible: new schema identity; existing schemas untouched; no migration |
| IS-0001 | §5A Provenance | Compatible: provenance records source/time/context only |
| RFC-0002 / IS-0010 | Identity derivation | Compatible: the resolution rule uses the existing deterministic identity derivation; a designation never creates an Artifact on its own |
| RFC-0003 / IS-0011 / RFC-0014 | Workspace, Snapshot, Engagement Derivation | Unchanged by this RFC: no new component, no field, no semantic change; Work Continuity stays derived (RFC-0010). Declaration outranks every measurement (RFC-0014 Requirement 11). IS-0012 is superseded in full; the row's claim is unaffected. |
| IS-0018 | CommittedUnderstanding | Compatible: the NextStep slot becomes fillable by derivation |
| IS-0019 | Restoration Plan invariants | Compatible: exactly one Resume Point, one Next Step, ordered Context Chain, zero+ Blockers |
| IS-0021 | §25.1/§25.2/§25.5/§25.11 | Amended by this milestone per §25.11 — the only amendment this RFC authorizes |
| RFC-0010 | Open Question 2 | **Resolved in the affirmative** by this RFC |

**What changes:** exactly one new Observation concept; one new schema; one new
producer surface; one amendment to IS-0021 §25 (per §25.11). Nothing else.

**What never changes:** the five primitives; the four frozen schemas; Workspace,
Snapshot, Attachment, Artifact semantics; observation immutability; replay;
local-first operation; determinism.

---

# Architectural Sequence

Implementation is authorized only in this order (RFC-0010 Future Dependency Order):

1. This RFC is accepted; the schema is defined and frozen (IS-0003 amendment;
   IS-0002 concept amendment).
2. The producer (Evo UI designation surface → daemon) is implemented and witnessed.
3. IS-0021 §25.1/§25.2/§25.5 is amended per §25.11 to consume the evidence.
4. Continuation derivation is implemented.
5. The full RestorationPlan is constructed when evidence is sufficient.
6. Resource identity and Future Restoration Execution remain separately defined.

---

# Open Questions (resolved)

1. **Currency semantics:** resolved — "the most recent declaration is current, with
   append order as the tie-break." The alternative (per-Artifact currency with
   explicit ambiguity when several current designations exist) was considered and
   rejected: it introduces multi-designation ambiguity inside one Workspace and is
   not the smallest contract.
2. **Whether the evidence may be user-directed (RFC-0010 Open Question 2):**
   resolved in the affirmative — the user's declaration is the witnessed fact. A
   designation is never an inference of intent.
3. **Whether unresolved-work evidence (future Blockers, §25.4) is a second evidence
   class:** deferred. It must not be collapsed into designation; Blockers remain
   zero under this RFC.

---

# Non-Goals

This RFC does not define:

- ranking, scoring, retrieval, or recommendation;
- LLM inference of any kind;
- a sixth architectural primitive;
- fields on Artifact / Workspace / Snapshot / Attachment;
- graph or edge infrastructure;
- execution or resource identity;
- predicting user behavior;
- Context Chain or Blocker semantics.

---

# Rationale

Evo's product promise is that the user remembers only what they want to do, and Evo
reconstructs the environment required to continue. The frozen contracts reach the
boundary of that promise at IS-0021 §25.5. Every inferred signal fails the
competing-world test and is prohibited by RFC-0010. The user's own declaration is
the one evidence class that is both canonical and discriminating: it is a witnessed
fact about the state of the work, supplied by the person whose work it is, and it
never requires Evo to guess. Where the user declares nothing, Evo remains silent —
exactly as the Constitution and RFC-0010 require.
