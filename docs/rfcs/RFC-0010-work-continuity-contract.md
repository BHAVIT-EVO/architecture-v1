# RFC-0010 — Work Continuity Contract

**Status:** Accepted

**Version:** 1.0

**Depends On:**
- Constitution
- Cognitive Model
- Product
- Architecture
- Architectural Laws
- RFC-0000
- RFC-0001
- RFC-0002
- RFC-0003
- RFC-0006
- RFC-0008

**Related:**
- IS-0003 — Observation Schemas
- IS-0011 — Workspace Model (as amended, Amendment 1)
- RFC-0014 — Engagement Contract (supersedes IS-0012 in full)
- IS-0018 — Committed Understanding
- IS-0019 — Restoration Model
- IS-0021 — Restoration Derivation
- TRACE-0001 — Continue My Work

---

# Abstract

This RFC defines the architectural contract for Work Continuity: the canonical, derived statement that a body of work's continuation lies at a specific continuation target.

Work Continuity is a property of the state of the work. It is never a prediction of the user's future behavior.

Work Continuity SHALL be represented as a derived relationship/value in accordance with Architectural Law XVI. It SHALL NOT become a new architectural primitive, a persisted canonical object, a field on any existing canonical object, or a graph/edge object.

This RFC supplies the lower-layer architectural contract required by IS-0021 §25.11, so that future canonical continuation evidence and future restoration derivation have a defined semantic boundary. It defines what evidence must be, what evidence must never be, and what the derived relationship means.

This RFC is not an implementation specification. It does not define observation schemas, producers, derivation algorithms, or execution.

---

# Motivation

Evo's product promise is that a person should only have to remember what they want to do, never how to get back to doing it (Product, "The Promise"). The Constitution declares continuity the product (Constitution, Article IV). The Cognitive Model establishes that interruption does not necessarily end an engagement, that continuity exists independently of recall, and that time alone does not determine whether an engagement has ended (Cognitive Model, Principles III–IV).

The current restoration spine reaches the boundary of that promise: IS-0021 derives a Resume Point from the selected Snapshot's Attachment Set, but §25.5 cannot derive a Next Step because no canonical continuation evidence exists. IS-0021 §25.11 requires that additional canonical evidence enter through the appropriate lower-layer architectural contract, after which IS-0021 may be amended to consume it.

This RFC is that lower-layer contract. Its purpose is to fix the semantic boundary before any evidence is introduced, so that evidence, when it arrives, arrives as a witnessed work-state fact — and never as recency, focus, ordering, or prediction disguised as continuity.

The determination that the architecture permits this semantic has already been made. This RFC does not reopen it. It defines the contract.

---

# Definition

## Work Continuity

Work Continuity is the canonical, derived statement that the continuation of one body of work lies at a specific continuation target, established exclusively through a frozen, deterministic derivation over canonical evidence describing the state of that work.

Work Continuity describes where the work is unfinished and continues. It does not describe what the user is expected to do.

Work Continuity exists only when a frozen, deterministic derivation over canonical evidence establishes that the work in a source state proceeds to a specific continuation target. An unresolved condition — a failing test, a compilation failure, a merge conflict, an unresolved dependency, incomplete work, or an interruption — MAY be evidence relevant to Work Continuity, but it SHALL NOT, by itself, establish a continuation target. A continuation target SHALL be established only by a frozen, deterministic derivation over canonical evidence, whose rules are defined by the evidence contract. The canonical evidence is not required to state the continuation conclusion as a witnessed event; the conclusion is derived.

## The Semantic Distinction

The following concepts SHALL remain distinct:

- **A. Chronology** — "B happened after A." A witnessed temporal fact. Chronology is not Work Continuity.
- **B. General association** — "B is related to A." An undefined relationship. Undefined association is not Work Continuity.
- **C. Identity evolution** — "B superseded A." Artifact identity lifecycle semantics (IS-0007). Identity evolution is not Work Continuity.
- **D. Prediction / intent** — "B is what the user is expected to do next." A statement about the user's future behavior. Prediction is not Work Continuity and SHALL NOT become canonical evidence.
- **E. Work Continuity** — "The work continues with B." A statement about the state of the work, derived from canonical evidence. Only E is the subject of this RFC.

The RFC explicitly prohibits collapsing E into D. E is a statement about the work; D is a statement about the user.

## Workspace Continuity vs Work Continuity

RFC-0003 defines Workspace continuity: the continued evolution of one coherent body of work, expressed as Workspace identity. That is the identity of the work across time.

Work Continuity, as defined here, is the state of continuation *within* a body of work. It is evaluated in relation to a Workspace but is not a property of Workspace identity. These two meanings of continuity SHALL NOT be conflated.

---

# Scope

This RFC defines:

- the semantic meaning of Work Continuity;
- the properties canonical evidence MUST possess before it can establish Work Continuity;
- the representation of Work Continuity as a derived relationship/value;
- the boundary between observed evidence and derived interpretation;
- the interaction of Work Continuity with Workspace, Snapshot, Artifact, the Observation layer, and IS-0021;
- the explicit non-goals of this contract.

This RFC does NOT define:

- observation schemas;
- producers or capture mechanisms;
- platform-specific signal sources;
- derivation algorithms;
- implementation;
- resource identity;
- executable target identity;
- restoration execution;
- retrieval;
- ranking;
- recommendation.

---

# Architectural Position

Work Continuity occupies the following boundary:

```
Canonical Observation History
        │
        ▼
   Workspace / Snapshot
        │
        ▼
   ┌──────────────────────────────┐
   │  Work Continuity Relationship │   ← this RFC (lower-layer semantic contract)
   └──────────────┬───────────────┘
                  │
                  ▼
   Restoration Derivation (IS-0021 §25, amended only after evidence exists)
                  │
                  ▼
        RestorationPlan / Next Step
```

The derivation of Work Continuity is a deterministic interpretation over canonical evidence (ARCHITECTURE §2: state is derived by replaying an interpretation function over the immutable observation log). It SHALL operate strictly below Restoration Derivation and SHALL NOT perform Workspace Formation, Artifact Resolution, or restoration execution.

## Representation

In accordance with Architectural Law XVI, Work Continuity SHALL be represented as a derived relationship/value:

- a relationship, not a computational object;
- derived, not stored as ground truth;
- disposable, not a new architectural primitive;
- replayable, not a fixed historical fact.

Work Continuity MUST NOT become:

- a sixth architectural primitive;
- a new persisted canonical object;
- a field on Artifact;
- a field on Workspace;
- a field on Snapshot;
- a field on Attachment;
- a field on Knowledge;
- a field on HistoricalUnderstanding;
- a field on CommittedUnderstanding;
- an identity-evolution relationship;
- a graph/edge object;
- a runtime state object.

---

# Canonical Evidence Requirements

## Requirement 1 — Canonical, Not Fabricated

Evidence SHALL be canonical: it SHALL consist of canonical facts accepted through the canonical Observation acceptance pipeline and represented in canonical understanding (IS-0021 §25.1, §25.5). Raw Observations SHALL NOT be used directly as continuation evidence (IS-0021 §25.5). Fabricated, inferred, guessed, or runtime-derived signals are not evidence.

## Requirement 2 — Canonically Defined

Evidence SHALL be defined by a frozen lower-layer contract. A future evidence source SHALL conform to IS-0003, including its platform-neutrality requirements and schema-evolution rules. A signal that is not canonically defined SHALL NOT be used as evidence.

## Requirement 3 — Semantically Explicit Derivation

The derivation rule defined by the evidence contract SHALL be semantically explicit: the work-state meaning it uses SHALL be defined by the contract, not recovered by reinterpreting chronology, ordering, titles, or frequency. The evidence itself is not required to state the continuation conclusion.

## Requirement 4 — Distinguishable From Chronology and Association

The evidence class SHALL be distinct from:

- observation order;
- recency;
- frequency;
- Artifact co-membership;
- Snapshot ordering;
- Workspace membership;
- Artifact supersession.

A source that cannot be distinguished from these is not continuation evidence.

## Requirement 5 — Sufficiency

Evidence is sufficient to establish Work Continuity only when a frozen, deterministic derivation rule defined by the evidence contract establishes, from that evidence, that the work in a source state proceeds to a specific continuation target. The derivation MAY operate over one or more canonical facts; the conclusion is derived by the rule and is not required to be stated by any single canonical fact. Evidence that merely records an unresolved condition or an unfinished state does not, by itself, establish a continuation target. The mapping from evidence to Work Continuity SHALL be defined by the frozen evidence contract, not by heuristics.

## Requirement 6 — Independence From Runtime Restoration State

Evidence SHALL NOT depend upon:

- current application state;
- current window state;
- current OS state;
- current UI state;
- current filesystem state;
- current network state;
- restoration-time conditions;
- transient runtime caches.

A future execution layer MAY inspect runtime state when executing, but such state SHALL NOT be continuation evidence.

## Requirement 7 — Insufficiency Is A Valid Outcome

When the required evidence is absent, NO Work Continuity relationship SHALL be produced. Insufficiency is a valid, explicit outcome. No confidence score SHALL be used to convert insufficient evidence into continuity. No fallback heuristic SHALL be used.

## Prohibited Evidence Sources

The following SHALL NOT establish Work Continuity:

- observation order;
- recency;
- frequency;
- window focus;
- window titles;
- application identity;
- URL appearance alone;
- file appearance alone;
- Artifact co-membership;
- Snapshot ordering;
- Artifact importance;
- Artifact confidence;
- Artifact supersession;
- Workspace membership;
- timestamps used as semantic evidence;
- runtime state;
- current OS state;
- current UI state;
- LLM output;
- user-intent prediction;
- heuristic guesses.

In particular, "the user focused B after A" SHALL NOT become "the work continues with B."

---

# Derived Relationship Semantics

A Work Continuity relationship SHALL identify, at minimum:

- **Workspace** — the body of work in which the relationship is evaluated;
- **Source** — the canonical state or Artifact from which continuation proceeds;
- **Continuation Target** — the canonical object(s) representing the continuation of the work;
- **Supporting Evidence** — the canonical evidence establishing the relationship;
- **Semantic Meaning** — the meaning defined by the evidence contract;
- **Validity Conditions** — the conditions under which the relationship is valid;
- **Invalid States** — the conditions under which the relationship must not exist.

A Work Continuity relationship SHALL be:

- deterministic;
- replayable;
- explainable from its supporting evidence;
- confined to canonical objects;
- non-mutating of all lower-layer objects.

The Semantic Meaning of the relationship SHALL express the progression of the work from its Source to its Continuation Target. A relationship whose semantics do not include that progression is invalid, even when evidence of an unresolved condition exists.

This is a contract describing a derived relationship. It is not a database model and not a computational object. No element SHALL be introduced without an architectural reason.

---

# Evidence vs Interpretation Boundary

Work Continuity SHALL preserve the architecture's epistemic separation (Architectural Law IV, Law XVII):

```
Observed canonical fact
        │
        ▼
Continuity interpretation (defined by the evidence contract)
        │
        ▼
Derived Work Continuity relationship/value
```

Observation is a witnessed fact. Interpretation is a deterministic derivation defined by the evidence contract. Work Continuity is the derived relationship/value produced by that derivation. An Observation SHALL NOT be redefined as Understanding, and a derived Work Continuity relationship SHALL NOT be presented as an observed fact.

The derived relationship SHALL NEVER:

- overwrite the original observation;
- masquerade as the original observation;
- be presented as witnessed fact;
- modify the observation, Artifact, Attachment, Snapshot, or Workspace.

The evidence contract SHALL define what makes evidence sufficient for the interpretation. Where evidence is insufficient, no interpretation is produced, and the absence is reported as insufficiency.

---

# Determinism / Replayability

Given identical:

- canonical evidence;
- canonical Workspace/Snapshot understanding;
- frozen evidence and derivation rules;

Work Continuity derivation SHALL produce an identical result.

No derivation decision SHALL depend upon:

- current time;
- current application;
- current window;
- current OS state;
- current UI state;
- mutable user profile;
- external service;
- LLM response;
- nondeterministic iteration order.

Replaying Work Continuity derivation SHALL NOT modify any canonical object.

---

# Explainability

Every Work Continuity relationship SHALL be explainable from its supporting canonical evidence (Architectural Law II, Law XI; RFC-0006, Requirement 7).

An explanation SHALL take the form:

> "The work continues with B because the evidence contract's frozen derivation establishes, from canonical evidence, that the work proceeds from its source state to B."

An explanation SHALL NOT take the form:

> "Evo thinks you'll probably work on B next."

A relationship that cannot be explained from canonical evidence MUST NOT exist.

---

# Insufficiency / Invalid States

Under insufficient evidence, the derivation SHALL produce no Work Continuity relationship. It SHALL NOT:

- rank candidates;
- choose the most recent Artifact;
- choose the most frequently used Artifact;
- use confidence as a substitute for semantic evidence;
- manufacture a "best guess";
- report uncertainty as certainty.

This is a direct application of the project principle: prefer silence over wrong suggestions (Constitution, Article III; Architectural Law VI).

The following are invalid states and SHALL NOT occur:

- a relationship without supporting evidence;
- a relationship referencing a non-canonical object;
- a relationship whose source or target is not canonically identified;
- a relationship whose semantic meaning is not defined by the evidence contract;
- a relationship derived from a prohibited source;
- a relationship whose only supporting evidence is an unresolved condition and no frozen derivation rule establishing the source-state-to-target progression;
- a relationship that depends on runtime restoration state.

---

# Interaction with Workspace

Work Continuity SHALL be evaluated in relation to a Workspace, but it SHALL NOT become:

- a Workspace component;
- a Workspace field;
- a Workspace semantic container.

The canonical Workspace component set SHALL remain exactly: Workspace Identity, Workspace Lifecycle, Attachment Set, Snapshot History (IS-0011 §4). Workspace SHALL NOT encode restoration order, relevance, or importance (RFC-0003, Non-Goals). Workspace SHALL NOT become a container of continuation semantics (RFC-0003, Requirement 3).

Workspace continuity (the identity of the body of work across time, RFC-0003) and Work Continuity (the state of continuation within the work) SHALL remain distinct concepts.

---

# Interaction with Snapshot

Work Continuity SHALL NOT alter the canonical Snapshot model. A Snapshot SHALL contain exactly: Workspace Identity, Snapshot Identity, Snapshot Creation Point, and one ordered Attachment Set (IS-0011 §3).

Snapshots SHALL NOT contain:

- restoration instructions (IS-0011 §3);
- execution instructions (IS-0011 W-30);
- RestorationPlan semantics (IS-0011 W-29);
- Work Continuity semantics.

Snapshot Construction SHALL NOT derive a Resume Point, Context Chain, Blockers, or Next Step (WF-14). Work Continuity is derived by the Restoration layer from canonical input, never by Workspace Formation.

**Reconciled with RFC-0014.** The citation *"IS-0012, Stage 5"* is withdrawn: IS-0012 is superseded in full and its stage numbering no longer exists. The rule itself is unchanged and still holds — Workspace projection produces Snapshots, and Restoration Derivation (IS-0021 §25) derives Work Continuity from them. WF-14 remains the normative anchor.

---

# Interaction with Artifact

Work Continuity SHALL NOT alter Artifact semantics. An Artifact SHALL answer only the identity question (IS-0004 R-2; RFC-0002, Requirement 2) and SHALL NOT determine restoration behavior or future user actions (IS-0004, Non-Responsibilities).

Artifact supersession and identity evolution (IS-0007) SHALL NEVER be treated as Work Continuity. A Work Continuity relationship MAY reference canonical Artifacts as source or target, but it SHALL NOT become a property of any Artifact.

---

# Interaction with the Observation Layer

Canonical continuation evidence SHALL enter through the Observation layer in accordance with IS-0003. The concrete evidence schema is not defined by this RFC; it SHALL be defined by a later Observation/IS milestone under IS-0003's platform-neutrality and schema-evolution rules.

A future evidence schema SHALL:

- describe a witnessed work-state fact, not an inference;
- remain platform-independent;
- carry no platform-specific values (paths, inodes, process identifiers, browser identifiers, application identifiers — IS-0003 §4.1);
- declare its optional and required canonical concepts explicitly.

Introducing a new Observation type SHALL follow the schema-evolution rules of IS-0003 §5.

---

# Interaction with IS-0021

This RFC supplies the lower-layer semantic contract required by IS-0021 §25.11. The sequence SHALL be:

1. this RFC is accepted;
2. canonical evidence first exists under this contract (evidence schema defined and frozen);
3. the evidence source/schema is subsequently defined and frozen;
4. only after that MAY IS-0021 be amended to consume the evidence;
5. only then MAY Restoration Derivation consume the evidence;
6. until then, IS-0021 §25.5 continues to produce explicit insufficiency.

This RFC does not amend IS-0021. Today's behavior is unchanged.

## Blockers and Next Step

The existing distinction is preserved:

- a future canonical unresolved-work representation MAY support Blocker semantics under IS-0021 §25.4;
- canonical continuation evidence MAY support the Next Step under IS-0021 §25.5.

The following SHALL NOT be collapsed:

- unresolved condition;
- continuation;
- next action.

An unresolved condition MAY be canonical evidence relevant to Work Continuity and MAY support Blocker semantics under IS-0021 §25.4. It SHALL NOT, by itself, imply a continuation target or a Next Step. A continuation target SHALL be established only by a frozen, deterministic derivation over canonical evidence, as defined by the evidence contract; no single canonical fact is required to state the continuation conclusion.

Whether unresolved-work evidence and continuation evidence require one evidence class or distinct evidence classes SHALL be decided by the future evidence contract, subject to the requirement above: no evidence class MAY map an unresolved condition directly to a continuation target without explicitly establishing that progression. This RFC does not invent semantics to make them equivalent.

---

# Resource Identity Boundary

Work Continuity does NOT solve:

- resource identity;
- file paths;
- URLs as executable targets;
- repository locations;
- OS launch targets;
- browser targets;
- application automation;
- restoration execution.

Artifact identity, resource identity, and executable target identity remain separate concepts. The Future Restoration Execution specification (IS-0019 §11; IS-0021 §20) remains downstream. No executable-target semantics are introduced by this RFC.

---

# Restoration Execution Boundary

Work Continuity is understanding. It is not action. This RFC does not define execution, and no Work Continuity relationship SHALL be executed under this RFC. Execution SHALL remain subject to the separate, future Restoration Execution contract.

---

# Guarantees

Every compliant implementation guarantees:

- Work Continuity is derived, never stored as ground truth;
- Work Continuity is established only from canonical evidence through a frozen, deterministic derivation, and is not required to be stated by any single observation;
- Work Continuity is a derived interpretation, never presented as an observed fact;
- Work Continuity is a statement about the state of the work, never a prediction;
- Work Continuity never modifies lower-layer canonical objects;
- Work Continuity derivation is deterministic and replayable;
- Work Continuity is explainable from canonical evidence;
- insufficient evidence produces no relationship;
- an unresolved condition never implies a continuation target;
- no confidence score substitutes for semantic evidence;
- no prohibited signal is promoted to evidence;
- Workspace, Snapshot, Artifact, and Attachment semantics remain unchanged.

---

# Forbidden Behaviour

Work Continuity MUST NEVER:

- be derived from any prohibited evidence source;
- collapse the distinction between Work Continuity and prediction;
- be produced without supporting evidence;
- treat an unresolved condition as a continuation target without a frozen derivation establishing the source-state-to-target progression from canonical evidence;
- use confidence as semantic evidence;
- become a new primitive, persisted object, field, or graph edge;
- modify Observations, Artifacts, Workspaces, Attachments, or Snapshots;
- masquerade as an observed fact;
- depend on runtime or OS state;
- rank candidates under insufficiency;
- select the most recent or most frequent Artifact;
- manufacture a best guess;
- perform restoration execution.

Violation of any of these behaviours invalidates compliance with this RFC.

---

# Identity Criterion

A Work Continuity relationship is identified by its Workspace, source, continuation target, and supporting evidence class. The relationship is deterministic: identical inputs produce an identical relationship.

In accordance with Architectural Law XVI, a Work Continuity relationship is not a computational object and SHALL NOT possess stable identity. It is a derived value.

---

# Non-Goals

This RFC explicitly does not define or require:

- implementation;
- producer implementation;
- platform-specific capture;
- OS APIs;
- browser automation;
- execution;
- resource resolution;
- executable target resolution;
- retrieval;
- ranking;
- recommendation;
- user-intent prediction;
- next-action prediction;
- LLM inference;
- new architectural primitives;
- new persisted canonical objects;
- fields added to frozen models;
- graph/edge infrastructure;
- changing Workspace semantics;
- changing Artifact semantics;
- changing Snapshot semantics;
- changing Attachment semantics;
- changing IS-0021 in this RFC;
- an observation schema (deferred to the evidence milestone).

---

# Compatibility

Any implementation conforms to this RFC if it preserves every behavioural guarantee defined herein.

No representation of the derived relationship is prescribed. No derivation algorithm is prescribed. Behavioural compatibility is independent of implementation.

Two independent teams implementing only this RFC and the future evidence contract must produce behaviourally indistinguishable Work Continuity results: the relationship exists exactly when the evidence contract's sufficiency rule holds, and never otherwise.

---

# Conformance

An implementation conforms to this RFC only if:

1. it represents Work Continuity as a derived relationship/value, never as an object or field;
2. it establishes Work Continuity only from canonical evidence satisfying Requirements 1–7;
3. it never establishes Work Continuity from a prohibited source;
4. it produces no relationship when evidence is insufficient;
5. it never substitutes confidence or heuristics for evidence;
6. it never predicts user behavior;
7. it never modifies lower-layer canonical objects;
8. it derives deterministically and replayably;
9. every relationship is explainable from canonical evidence;
10. Workspace, Snapshot, Artifact, and Attachment semantics remain unchanged;
11. it performs no restoration execution;
12. it never treats an unresolved condition as a continuation target without a frozen derivation establishing the source-state-to-target progression from canonical evidence.

---

# Future Dependency Order

The accepted sequence after this RFC:

1. define and freeze the canonical evidence schema (Observation/IS milestone, per IS-0003 rules);
2. implement the producer that witnesses the work-state fact;
3. amend IS-0021 §25.4/§25.5 per §25.11 to consume the evidence;
4. implement continuation derivation;
5. construct the full RestorationPlan when evidence is sufficient;
6. separately define resource identity and the Future Restoration Execution specification;
7. implement execution.

Until evidence exists under this contract, IS-0021 §25.5 continues to produce explicit insufficiency, and restoration execution remains subject to its own future contract.

---

# Open Questions

1. Whether unresolved-work evidence (Blockers, IS-0021 §25.4) and continuation evidence (Next Step, §25.5) require one evidence class or two. This RFC does not collapse them; and regardless of the answer, an unresolved condition SHALL NOT by itself establish a continuation target — the derivation over the class used for continuation evidence must establish the source-state-to-target progression under the evidence contract's frozen rules. The evidence milestone decides the schema.
2. Whether a witnessed work-state fact may ever be user-directed (a request to continue) is left open. Nothing in this RFC requires user direction as the evidence source. User requests have no canonical representation in the current architecture; the question is deferred to the evidence milestone.
3. Whether the future evidence contract should express the relationship as a single derived value or as a small set of derived values (e.g., separating the continuation target from the supporting evidence reference). Representation-neutral per Law XVI; decided by the evidence milestone.

---

# Rationale

Evo exists to restore continuity of thought (Constitution, Article IV). The Cognitive Model describes continuation as a property of an open engagement — "Openness is a property of an engagement. It is not a separate object" — which is why Work Continuity is a derived relationship, not a new object.

The architecture's restoration layer requires canonical, explainable evidence of where work continues before it can truthfully offer resumption (RFC-0006; TRACE-0001: "Historical association alone is insufficient to justify restoration"). Without a defined semantic boundary, the risk is that convenience — recency, focus, ordering — silently becomes meaning. This RFC closes that door before evidence arrives.

The representation follows Architectural Law XVI: concepts that relate computational objects are represented as relationships, derived values, or transient computational state — never as objects. The derivation follows Law II and Law XVII: it is grounded in evidence and kept epistemically separate from the evidence. The insufficiency behavior follows Law V and Law VI: uncertainty is legitimate state, and under-interpretation is preferred over over-interpretation.

This RFC deliberately does not define evidence schemas or derivation algorithms. Those belong to the evidence and IS-0021 milestones, in that order, exactly as IS-0021 §25.11 prescribes.

---

# Self-Critique

## Assumptions

This RFC assumes immutable Observation history, the closed set of five architectural primitives, and the derived/disposable interpretation model established by the Architecture and RFC-0004. It assumes the semantic determination already made: the architecture permits Work Continuity as a derived value, and evidence is currently unavailable.

## Deliberate Omissions

This RFC intentionally omits the observation schema, the producer contract, the derivation algorithm, resource identity, and execution. These belong to later milestones. Omitting them here is deliberate: this RFC defines the semantic boundary so that later milestones cannot import prohibited semantics.

## Architectural Boundary

Work Continuity is understanding. It neither witnesses evidence nor executes actions. Witnessing belongs to the capture layer; execution belongs to the future execution contract. This RFC defines only the semantic contract between them.

---

# Architectural Law

> **Work Continuity shall be derived from canonical evidence, represented as a relationship rather than an object, and preferred silent over guessed (Architectural Law II, Law V, Law VI, Law XVI, Law XVII).**
