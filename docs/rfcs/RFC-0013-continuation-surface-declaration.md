# RFC-0013 — Continuation Surface Declaration

**Status:** Accepted

**Version:** 1.1

---

## Reconciliation with RFC-0014 (Engagement Contract)

**Every behavioral requirement of this RFC is unchanged.** A declared
Continuation Surface remains ground truth, remains uncapped at Layer 3, and
continues to outrank every inference (RFC-0014 Requirement 11 restates this at
contract level). No requirement, schema, or guarantee here is amended.

Two citation-level notes, so nothing in this document reads as resting on
withdrawn authority:

1. **RFC-0012 is superseded in full.** This RFC cites it extensively, because
   RFC-0012 drew the boundary between "what belongs to the same body of work" and
   "where the work continues right now", and this RFC was written to sit above
   that boundary. **The boundary survives; only the RFC that drew it is
   replaced.** The question "what belongs to the same body of work" is now
   answered by Engagement Derivation (RFC-0014 Requirements 1–7) rather than by
   co-membership scoring, and it still never answers "where the work continues" —
   that is what a declaration, or a derived Continuation role, is for. Every
   argument in this document that turns on the separation of those two questions
   holds verbatim with RFC-0014 substituted for RFC-0012.

2. **IS-0012 is superseded in full.** Where this RFC states that Workspace
   Formation "consumes only canonical Observation input under IS-0012/IS-0014 as
   amended by RFC-0012", read: consumes only canonical Observation input under
   RFC-0014 and IS-0014 as amended. `OBS-CONTINUATION-SURFACE/v1` remains
   reference-only evidence that produces no Attachment and no confidence, exactly
   as specified.

Frozen schemas are unaffected: `OBS-WORK-GROUPED/v1` and
`OBS-REPOSITORY-MEMBERSHIP/v1` remain canonical and replay identically (RFC-0001
immutability). What changed is their interpretation, not their validity.

---

**Depends On:**
- Constitution
- Cognitive Model
- Product
- Architecture
- Architectural Laws
- RFC-0000
- RFC-0001
- RFC-0002
- RFC-0003 (v2.0)
- RFC-0010
- RFC-0011
- RFC-0014
- IS-0001 — Observation
- IS-0002 — Observation Language
- IS-0003 — Observation Schemas
- IS-0004 — Artifact Model
- IS-0010 — Identity Derivation Contract
- IS-0011 — Workspace Model
- IS-0014 — Confidence Score
- IS-0019 — Restoration Model
- IS-0020 — Collector
- IS-0021 — Restoration Derivation

---

# Amendment Record

## v1.1 — What happens when the user has *not* declared

v1.0 answered "where does the work continue" with a declaration and with nothing
else. That was correct about the declaration and wrong about the silence: it
treated the absence of an instruction as an instruction to do nothing. Two
requirements are amended below; a third item is a conformance correction, where
the implementation had deviated from this document rather than this document being
wrong.

Every guarantee about a *declared* surface is unchanged. A declaration remains
ground truth, remains uncapped, and continues to outrank every inference
(RFC-0014 Requirement 11) — including the inference that the user wanted more
opened than they asked for.

### Amendment 1 — Absent a declaration, the surface is derived from role

**Old rule (v1.0, Insufficiency).**

> "No valid declaration → no surface. The plan carries an empty surface; no
> fallback, no guess, no recency-derived substitute (Law V, Law VI)."

**Why it prevented the product from working.** The surface is what Restoration
Execution opens. With no declaration the surface was empty, so pressing Continue
on a body of work Evo had understood perfectly well opened *nothing at all*. The
user had to first tell Evo where they were in order to be taken back there —
which is the one thing Evo exists to spare them (§9: "Evo's value is that it
observes and reconstructs"). Honesty was never the problem here; the emptiness was
truthful and useless.

The v1.0 rule was written when the only alternative on offer *was* a guess —
recency, frequency, or focus (Candidate D, correctly rejected). That is no longer
the alternative. By the time this derivation runs, the Engagement layer has
already decided which members are the places the work happens, as roles derived
from witnessed evidence and carried as canonical Attachment state (RFC-0014
Requirement 8).

**New rule.** Absent a declaration that speaks about this Workspace, the surface
is the Workspace's members whose role says the work happens there — those for
which `ResourceRole::opens_on_restore` holds. Law V and Law VI are satisfied
rather than bypassed: roles are canonical state read as given, so this introduces
no new evidence class, no new threshold, and no second judgement at this layer.
Candidate D remains rejected — recency, frequency, and raw focus are still not
consulted.

A body of work with no such member yields an empty surface, explicit and honest,
exactly as before.

### Amendment 2 — A declaration says nothing about work it does not mention

**Old rule (v1.0, Insufficiency).**

> "A declaration referencing artifacts outside W → W's surface is empty; the
> declaration is not an error."

**Why it prevented the product from working.** A declared surface is also a
declared *grouping* of its members: everything a person names as "where I
continue" belongs to one body of work by construction (RFC-0014 Requirement 11).
Every **other** body of work therefore intersects the declaration emptily. Under
the old rule that emptiness silenced them — so declaring where you continue in
*one* piece of work switched Continue off for *all the rest of your work*,
permanently, and the only repair was to declare a surface in each of them. That
is the explicit-declaration regime §9 forbids, arrived at as a side effect of
using the feature once.

**New rule.** A declaration is authoritative about the body of work whose members
it names, and says nothing about any other. An empty intersection is therefore
*silence, not an instruction*, and this Workspace's surface is derived from role
exactly as it would be with no declaration in the log at all (Amendment 1).
Nothing is guessed and no threshold is introduced: the test is whether the user's
own statement mentions this work.

### Conformance correction — the intersection is the Attachment Set, not the Snapshot

Per-Workspace Consumption specifies `surface(W) = { artifact ∈ artifacts(W) : … }`
— every Artifact in the body of work. The implementation intersected the declared
surface with the *selected Snapshot's* candidates instead, because the Snapshot is
what the rest of the derivation is scoped to.

This document was right and the code was wrong, so nothing here is amended. The
deviation is recorded because its effect was serious: a Snapshot is one sitting
and a body of work is all of them, so a person who declared "continue from the
brief, the notes and that commit" had a commit made on Tuesday silently dropped
from Thursday's surface. Evo took an unambiguous instruction and discarded part of
it without saying so — which §10 forbids outright, and which no user could
diagnose, since the declaration was accepted and the Resource remained a visible
member of the work. The implementation now intersects the canonical Attachment
Set, as specified.

The *undeclared* fallback of Amendment 1 is deliberately Snapshot-scoped, and for
the opposite reason: with no instruction to honour, the sitting being returned to
is what "where I left off" means, and opening the union of every sitting would
open more than the minimum (§12, §16). Declared breadth is the user's choice;
inferred breadth is Evo's, and Evo's is the narrow one.

### Reconciliation and tests

IS-0021 §25.1A and IS-0019 CS-1–CS-4 describe the surface consistently with the
above. The mechanism is `evo_restoration::derivation::derive_workspace_surface`,
which carries the same record at the point of definition. Covered by the
selective-restoration and continuation-surface harnesses
(`verify_selective_restoration`, `verify_continuation_surface`) and by the
derivation unit tests for the undeclared, declared-elsewhere, and
declared-across-sittings cases.

---

# Abstract

RFC-0012 gave Evo the canonical answer to "which distinct artifacts belong to the
same body of work" (co-membership evidence). The remaining product question is:

> "Given everything that belongs to this Workspace, what should Evo bring back to me NOW?"

This RFC defines the missing canonical evidence for that question: **ContinuationSurface** —
a directly witnessed user declaration that a set of already-witnessed subjects currently
constitutes the continuation of the user's work.

The governing separation, established by RFC-0012 and preserved here, is:

**Same Artifact ≠ Related Work ≠ Currently Relevant ≠ Restore Together.**

RFC-0012 established the second concept. This RFC establishes the third: an explicit,
user-declared, canonical record of *what the work continues across right now*. It does not
establish that anything should be restored, does not alter Workspace membership, does not
alter Artifact identity, and does not change a single clause of RFC-0010 or RFC-0011.

---

# Motivation

Evo's Workspace now represents a coherent body of work — a repository, an editor, browser
research, a chat client, and a conversation can all belong to one Workspace through
canonical co-membership evidence (RFC-0012). But a Workspace is a *durable historical
relationship*: it may contain dozens of artifacts accumulated over months. Restoration
must not blindly restore every member. IS-0019's principle is that Restoration minimizes
cognitive reload, not that it maximizes historical completeness; TRACE-0001 Phase 4 holds
that "historical association alone is insufficient to justify restoration."

The product therefore requires a separate, canonical answer to:

> "These resources are the ones my work continues across right now."

RFC-0011's WorkDesignated answers this for exactly **one** subject. Real work spans several
resources at once — an editor at a file, a research tab, a conversation, a terminal. No
accepted evidence class witnesses a *multi-resource continuation declaration*. RFC-0010
explicitly prohibits deriving such a declaration from recency, frequency, focus,
co-membership, temporal grouping, or any other inferential signal. The declaration must
therefore be **witnessed, not derived**: performed by the user through a trusted capture
origin, exactly as WorkDesignated is today.

---

# Scope

This RFC defines:

- the semantic meaning of a continuation surface declaration;
- the canonical concept `ContinuationSurface` and the Observation Schema
  `OBS-CONTINUATION-SURFACE/v1`;
- the producer (witness) contract for the declaration;
- the deterministic rules for consuming the declaration in Restoration Derivation;
- the boundary that keeps the surface out of Workspace Formation, Artifact identity,
  Work Continuity (as defined by RFC-0011), and execution;
- the exact amendments required to frozen documents.

This RFC does NOT define:

- ranking, recency, frequency, focus, or any heuristic relevance;
- a fixed resource count ("recent four") or any application category;
- how the surface is executed (resource identity and executable target identity remain
  downstream concerns);
- a new Workspace-membership rule (the surface never changes Workspace membership);
- changes to Artifact identity derivation (IS-0010) or Workspace identity semantics;
- changes to the meaning of WorkDesignated (RFC-0011);
- a new architectural primitive beyond a new Observation Schema and a RestorationPlan
  component;
- UI.

---

# Definitions

**Continuation Surface**

The set of already-witnessed subjects that the user declares their work currently continues
across. A declared fact, never an inferred one.

**ContinuationSurface (the concept)**

The canonical vocabulary term for the witnessed declaration: "the user declared that their
work currently continues across this set of already-witnessed subjects." A
declaration-class concept under IS-0002, following the `WorkDesignated` precedent.

**ContinuationSubject**

A subject-valued canonical concept: one member of the declared surface, resolved to the
Artifact established by content Observations (the RFC-0011 §4 resolution pattern).

**Current Continuation Surface (derived state)**

The single latest valid `OBS-CONTINUATION-SURFACE/v1` declaration, resolved to the set of
Artifacts it references. Derived, rebuildable, non-canonical — the canonical fact is the
Observation; the resolved set is derivation input.

**WorkDesignated (RFC-0011)**

The existing single-subject continuation declaration. Its semantics are unchanged by this
RFC. This RFC adds a *distinct* declaration class for multi-resource continuation; it does
not modify, extend, or reinterpret WorkDesignated.

**Origination**

A retired concept. Under RFC-0014 there is no origination step: bodies of work are
derived from the whole corpus rather than created at a moment and persisted, so
there is nothing for a declaration to trigger. This RFC never extends, merges, or
splits bodies of work.

What survives of the v1.0 wording is the guarantee, and it is now stronger: a
surface declaration is ground truth *inside* the derivation, so the members it
names are guaranteed to be recognised as one body of work regardless of what
measurement would have concluded (RFC-0014 Requirement 11). See RFC-0011 v1.1 for
the retirement record; the superseded citation was "RFC-0012 Sufficiency floor".

---

# The Problem — The Exact Semantic Gap

The current canonical model answers two of the three questions:

| Question | Answered by | State |
|---|---|---|
| "What is the same artifact?" | Artifact Identity (IS-0010) | Complete |
| "What belongs to the same body of work?" | Workspace Formation + RFC-0012 co-membership | Complete |
| "What should I continue right now?" | — | **Gap** |

Verification of the gap against the accepted contracts and the actual implementation:

1. **RFC-0011 defines exactly one continuation target.** The current designation is the
   single-subject WorkDesignated declaration, latest-valid-supersedes. RFC-0011 explicitly
   rejects "per-Artifact currency" — an artifact does not become current because it is in
   the same Workspace as the designation.
2. **RFC-0010 prohibits every inferential candidate.** It distinguishes Work Continuity
   from chronology, general association, identity evolution, and prediction, and it lists
   recency, frequency, focus, window/application identity, file modification, commit,
   co-membership, temporal grouping, semantic similarity, runtime state, OS state, Git
   state, and attachment ordering/confidence as prohibited continuation sources.
3. **RFC-0012 explicitly bars co-membership from continuation.** Its "Separation from Work
   Continuity" section forbids co-membership evidence from feeding the Resume Point, the
   Next Step, the Context Chain, Blockers, or any restoration/execution selection.
4. **The implemented derivation confirms the boundary.** `derive_restoration_plan`
   consumes exactly {Workspace, Snapshot, designated artifact}. It derives one Resume Point,
   one Next Step (designation-driven), an always-empty Context Chain, and zero Blockers.
   There is no input channel for a multi-resource continuation surface.

**Conclusion:** the gap is not an implementation defect. The canonical Observation language
contains no evidence class that witnesses a multi-resource continuation declaration, and
the derivation contract admits no such input. A new evidence class is required.

---

# Candidate Evidence Classes — Competing-World Analysis

For every candidate, we construct:

- **WORLD A** — the user genuinely continues their work at/with resource set S.
- **WORLD B** — the user has merely interacted with S, or the work continues elsewhere.

A signal may establish the current continuation only when the canonical evidence differs
between the worlds.

## Candidate A — Multi-subject continuation declaration (ContinuationSurface)

The user performs an explicit declaration act (through a trusted capture origin): "these
already-witnessed subjects are what my work continues across."

- **WORLD A evidence:** a canonical `OBS-CONTINUATION-SURFACE/v1` observation exists,
  witnessed at declaration time.
- **WORLD B evidence:** no such observation exists (the user never declared).
- **Verdict:** **SURVIVES.** The evidence differs between the worlds because the truth
  condition is the performance of the declaration act itself — the same epistemic pattern
  RFC-0011 already accepts for WorkDesignated and RFC-0012 accepts for WorkGrouped. The
  user is the strongest honest authority over where their work continues (Law IX).

## Candidate B — Reinterpret WorkDesignated as multi-subject

Extend the existing single-subject designation so that "everything around the designated
artifact" becomes current.

- **WORLD A evidence:** designation(D).
- **WORLD B evidence:** designation(D) — D is merely the artifact last designated, while
  the work continues across a different set.
- **Verdict:** **REJECT.** The evidence is identical in both worlds. This is precisely the
  "do not reinterpret WorkDesignated as 'everything around this Artifact is current'"
  prohibition, and it would silently change RFC-0011 semantics (per-Artifact currency is
  explicitly rejected there).

## Candidate C — Derive the surface from Workspace membership

Treat the Workspace's artifact set (or a subset) as the continuation surface.

- **WORLD A evidence:** artifacts {A, B, C} are members of Workspace W.
- **WORLD B evidence:** the same membership — the user continues only across {B}, while A
  and C are historical.
- **Verdict:** **REJECT.** Membership is identical in both worlds. "A Workspace MUST NOT
  automatically mean restore every artifact." This candidate also violates RFC-0010's
  prohibition of Workspace membership / co-membership as continuation evidence.

## Candidate D — Derive the surface from recency / frequency / focus / temporal grouping

- **WORLD A evidence:** S was used recently / frequently / is focused.
- **WORLD B evidence:** identical — the user merely touched S last, or S is habitually
  open while the work continues elsewhere.
- **Verdict:** **REJECT.** Identical in both worlds; each is an explicit RFC-0010
  prohibition. Also rejected by the mission's rule that these signals never become
  relevance "under another name."

## Candidate E — Reuse WorkGrouped (RFC-0012) as continuation evidence

- **WORLD A evidence:** user declared A and B related work (possibly months ago).
- **WORLD B evidence:** the same declaration — the user related A and B historically, but
  their current continuation is a different subset.
- **Verdict:** **REJECT.** Relatedness is a durable historical fact; it does not change when
  the current continuation changes. RFC-0012 already forbids co-membership from feeding
  continuation.

## Candidate F — LLM / semantic similarity / embeddings

- **Verdict:** **REJECT.** Not canonical evidence; violates Law IV (observation records
  witnessed fact) and RFC-0010's prohibition of semantic similarity as continuation
  evidence. May be useful later as non-canonical presentation only.

## Surviving evidence

**Candidate A alone survives.** The current continuation of a multi-artifact Workspace is
established exclusively by an explicit, witnessed user declaration. Everything else is
either identical across the two worlds or already prohibited.

---

# ContinuationSurface — Meaning

**Canonical fact:** the user explicitly declared that their work currently continues across
a set of already-witnessed subjects.

**What it proves:**

- the performance of the declaration act at the witnessed moment;
- that the declared subjects currently constitute the continuation surface of the user's
  work.

**What it does NOT prove:**

- that the work is unfinished;
- that the work is important or urgent;
- that any declared subject should be opened, focused, or executed now;
- that the declared subjects belong to the same body of work (that is Workspace Formation's
  question, answered by RFC-0012 evidence — a surface declaration does not establish
  membership);
- that the subjects are the *same* artifact (Artifact identity is untouched);
- any relationship between the subjects beyond their joint declaration;
- anything about subjects not declared.

The declaration records the witnessed act, never Evo's interpretation of it (Law IV,
Law XVII).

---

# Observation Schema (proposed IS-0003 §4.1.8)

- **Schema Identifier:** `OBS-CONTINUATION-SURFACE`
- **Schema Version:** 1
- **Observation Type:** ContinuationSurface
- **Required Subject:** the first subject of the declared set under the canonical ordering
  rule (lexicographically smallest canonical subject string) — identical in spirit to the
  RFC-0012 WorkGrouped canonical pair rule.
- **Required Canonical Concepts:**
  - `ContinuationSurface` — the witnessed declaration fact;
  - `ContinuationSubject` — subject-valued; one fact per additional subject beyond the
    Required Subject.
- **Cardinality:** a valid declaration names at least two distinct subjects (the surface is
  the multi-resource declaration; a single-subject continuation declaration is already the
  RFC-0011 WorkDesignated contract, and a second channel expressing the same single-subject
  fact would create two competing authorities for one claim). No upper bound.
- **Optional Canonical Concepts:** None.
- **Platform neutrality:** the schema SHALL NOT require application, browser, process, or
  editor identity. Subjects are platform-typical canonical text (paths, commit hashes, URL
  subjects), exactly as in the frozen schemas.
- **References, does not create:** the observation SHALL NOT by itself establish an
  Artifact. Every subject resolves to the Artifact established by content Observations
  (OBS-FILE-SAVED, OBS-COMMIT-MADE, OBS-URL-NAVIGATED, OBS-WINDOW-FOCUS-GAINED) through the
  deterministic identity derivation, per the RFC-0011 §4 resolution pattern. The
  acceptance/runtime pipeline SHALL apply the same reference-only handling the pipeline
  already applies to `OBS-WORK-DESIGNATED`, `OBS-REPOSITORY-MEMBERSHIP`, and
  `OBS-WORK-GROUPED`.
- **Deterministic encoding:** the declared set is encoded in canonical order — the
  lexicographically smallest subject is the Required Subject, the remainder are
  `ContinuationSubject` facts in ascending order. A declaration expressed in any order
  produces the identical canonical Observation. Replay is therefore order-independent.

## Declaration Semantics

- The truth condition is the performance of the declaration act through a trusted capture
  origin reserved for user continuation declarations (the designation-pattern precedent:
  the desktop shell never writes canonical state; the daemon — the canonical runtime owner
  — validates, accepts, persists, and re-derives).
- The producer SHALL offer only subjects already witnessed in the canonical Observation
  log (the RFC-0011 producer rule, already implemented for WorkDesignated and WorkGrouped).
- A declaration referencing any unwitnessed subject is rejected before any canonical
  Observation is created; no partial declaration ever exists.
- **Supersession:** the latest valid declaration establishes the Current Continuation
  Surface. An earlier declaration remains canonical history but no longer contributes
  derivation input — the identical rule RFC-0011 applies to designations.
- The surface is global (not Workspace-scoped) at the evidence level: it references
  subjects, not Workspace identities. Derivation intersects it per Workspace (below). A
  declaration never requires a simultaneous WorkDesignated, and WorkDesignated never
  implies a surface.
- No canonical inference, ranking, or completion logic is involved at emission: the
  declaration is exactly what the user declared.

## When an Observation MUST NOT Be Emitted

- For any subject not already present in the canonical Observation log.
- For a declaration of fewer than two distinct subjects.
- For a declaration Evo cannot witness (the act must be performed through the trusted
  capture origin; a claim relayed without witness is not evidence).

---

# Artifact Interaction

- The surface **references** existing Artifact identities; it never creates, merges,
  splits, or modifies them (RFC-0002 Req 2, Req 7; IS-0004 I-3).
- Artifact identity derivation (IS-0010) is **unchanged**. The observation is excluded
  from Artifact establishment exactly as WorkDesignated observations are today
  (RFC-0011 §4) — no second Artifact is created for a declared subject.
- An Artifact SHALL NOT encode continuation. Continuation lives in the Observation and
  derivation layers; the Artifact remains answerable only to the identity question.
- No Artifact can exist solely because of a surface declaration.

---

# Workspace Interaction

- The surface **never** changes Workspace membership. It does not attach artifacts, does
  not detach artifacts, and does not merge or split Workspaces.
- **No origination by declaration.** There is no origination step under RFC-0014, so a
  surface declaration triggers nothing. Its members are instead guaranteed to be
  recognised as one body of work by the derivation itself, because a declaration is
  ground truth within it (RFC-0014 Requirement 11) — a stronger guarantee than the v1.0
  clause, and one that does not depend on what had already been persisted. Retirement
  record: RFC-0011 v1.1.
- Engagement Derivation consumes only canonical Observation input (RFC-0014 Requirements
  1–7, superseding IS-0012/RFC-0012). `OBS-CONTINUATION-SURFACE/v1` remains
  reference-only evidence: it produces no Attachment and contributes no confidence.
  It is consumed at derivation time as a declaration, not as a membership signal.
- Historical membership is durable: the "three months later" test is satisfied because
  Workspace membership is a historical relationship that the surface never rewrites. An
  artifact that stops being current remains a member; an artifact that was never declared
  may still belong to the body of work.

---

# Restoration Derivation Interaction

## Input Boundary (proposed IS-0021 §25.1 extension)

Derivation consumes, in addition to the existing {Workspace, Snapshot, designated
Artifact}:

- the **Current Continuation Surface**, resolved by the Artifact layer from the latest
  valid `OBS-CONTINUATION-SURFACE/v1` observation (RFC-0011 §4 resolution). Derivation
  never consults raw Observations and never performs Artifact Resolution (IS-0021 §4,
  §25.1).

## Per-Workspace Consumption

For the Workspace under derivation, where a Current Continuation Surface names at
least one of W's members, the derived surface is:

> surface(W) = { artifact ∈ artifacts(W) : artifact is referenced by the Current
> Continuation Surface }

in canonical deterministic order (ascending ArtifactId). `artifacts(W)` is W's
canonical **Attachment Set** — every Artifact in the body of work, across every
sitting — not the selected Snapshot's candidates. A declaration is a statement
about the work, not about whichever sitting happens to be most recent.

Where the Current Continuation Surface names **none** of W's members, it is not
speaking about W, and:

> surface(W) = { artifact ∈ candidates(Snapshot(W)) : role(artifact) opens on
> restore }

in the same canonical order. The same rule applies when there is no declaration at
all. See v1.1 Amendments 1 and 2 for why each scope is what it is. Surface members
outside W are ignored for W; each Workspace sees only its own declared members.
This is derivation output, not canonical state.

## RestorationPlan Component (proposed IS-0019 extension)

The canonical `RestorationPlan` gains one component: **ContinuationSurface** — the derived
surface(W) set. It carries the resources the user declared their work continues across
*within this Workspace*, available for downstream Restoration execution.

## Gating Invariants (unchanged behavior, explicit)

- The surface does **not** gate completeness. A plan is Complete exactly when the Resume
  Point and Next Step are established (IS-0021 §25.6), exactly as today. A Complete plan
  may carry an empty or non-empty surface; an Insufficient outcome carries the derived
  surface when one exists.
- The surface does **not** change the Resume Point, the Next Step, the Context Chain, or
  Blockers. All four derive exactly as IS-0021 §25.2–§25.5 specifies today.
- The surface does **not** change execution ordering or selection. Execution is downstream
  and remains out of scope (RFC-0010: Work Continuity does not solve resource identity or
  executable target identity).

## Insufficiency

- No valid declaration → the surface is derived from Attachment role (v1.1
  Amendment 1): the members for which `ResourceRole::opens_on_restore` holds, in
  canonical order. No recency-, frequency-, or focus-derived substitute is
  consulted (Law V, Law VI; Candidate D remains rejected). A body of work with no
  such member carries an empty surface, explicit and honest.
- A declaration whose subjects are unwitnessed → rejected at emission; derivation
  never sees it.
- A declaration referencing artifacts outside W → the declaration does not speak
  about W, so W's surface is derived from role as if no declaration existed (v1.1
  Amendment 2). The declaration is not an error, and it does not silence W.

---

# Producer Contract

- **Who witnesses:** the daemon, the canonical runtime owner. The desktop shell proposes;
  the daemon validates against the canonical Observation log, accepts, persists the
  canonical Observation, and re-derives. (The identical division of authority already
  implemented for designation and grouping channels.)
- **Capture origin:** a trusted capture origin reserved for user continuation declarations.
- **What may be declared:** any already-witnessed canonical subject. No application,
  domain, profession, or category restrictions — the mechanism is universal (an editor, a
  browser tab, a document, a terminal, a conversation all declare identically).
- **When:** at any time, on the user's explicit act. Evo never emits this Observation on
  its own initiative.

---

# Replay

Given identical canonical Observation history (including surface declarations) and
identical derivation rules:

- the Current Continuation Surface resolves identically;
- per-Workspace surfaces derive identically;
- Restoration outcomes are identical.

No derivation decision SHALL depend on wall-clock time, live git/filesystem/OS state,
iteration order, or any non-canonical input. Replay regenerates derived state; it never
rewrites historical Observations, Snapshots, or Workspaces (RFC-0003 Req 7; Architecture
§7).

---

# Failure / Insufficiency Behavior

- No declaration → empty surface, explicit and honest.
- Unwitnessed subject in a declaration → the entire declaration is rejected; no partial
  declaration exists.
- Fewer than two distinct subjects → rejected (the single-subject continuation claim
  belongs to the RFC-0011 designation contract).
- Malformed or unparseable declaration → rejected with an honest reason; no canonical
  Observation is created.
- Surface members not present in the Workspace → ignored for that Workspace.
- Derivation input unavailable (storage read failure) → the existing IS-0021 §25.8
  insufficiency path, naming the missing component.

---

# Privacy / Local-First

- The declaration is produced and consumed locally (Law XIII; Architecture §10). No
  network call, external service, or remote state.
- Subjects are the same canonical text the content Observations already record; the
  declaration adds no new high-fidelity capture.
- Provenance records only the witnessed declaration circumstances (IS-0001 §5A).

---

# Document Impact (Phase 2 — proposals only; applied only upon acceptance)

## Documents requiring amendment

| Document | Clause | Amendment |
|---|---|---|
| IS-0002 | §4 Vocabulary | Add concept `ContinuationSurface` (§4.8) — declaration-class concept: "the user declared that their work currently continues across a set of already-witnessed subjects" — with a SHALL-NOT-imply list (does not imply unfinished/important/restorable/membership/same-artifact), satisfying the §9 gate (directly observable act; stable platform-independent semantics; not representable by an existing concept — WorkDesignated is single-subject and WorkGrouped is relatedness, not continuation; justified by the demonstrated selective-continuation requirement). |
| IS-0003 | §4.1 | Add schema `OBS-CONTINUATION-SURFACE/v1` (§4.1.8) as specified above, with the cardinality rule, deterministic encoding, platform-neutrality note, and "references, does not create" clause. |
| IS-0019 | RestorationPlan components | Add the `ContinuationSurface` component (the derived surface(W) set), with invariants: members are Artifacts of the Workspace; the surface never gates completeness; the surface never affects Resume Point / Next Step / Context Chain / Blockers. |
| IS-0021 | §25.1 input boundary + new clause | Extend the derivation input boundary with the Current Continuation Surface (resolved by the Artifact layer, per RFC-0013); add the per-Workspace intersection rule and the explicit non-gating invariant. §25.2–§25.5 text is otherwise unchanged. |

## Documents deliberately untouched

| Document | Reason |
|---|---|
| Constitution, Product, Architecture, Architectural Laws | No contradiction; the surface is an additional evidence class and a derivation input, both within the frozen layering (Observation → … → Restoration). |
| RFC-0001, RFC-0002 | Observation and Artifact semantics unchanged; the surface references, never creates. |
| RFC-0003 | Workspace semantics unchanged; Formation is untouched by this RFC. |
| RFC-0010 | Work Continuity prohibitions reinforced, not changed. |
| RFC-0011 | WorkDesignated semantics unchanged; this RFC defines a *distinct* declaration class and references RFC-0011 (RFC-0000 Principle VIII — an RFC MAY reference another RFC and MUST NOT redefine it). No RFC-0011 clause requires amendment. |
| RFC-0012 | Co-membership semantics and its continuation boundary are unchanged; this RFC sits above the boundary it drew. |
| IS-0001 | Acceptance pipeline unchanged; the schema flows through the existing reference-only handling. |
| IS-0004, IS-0010 | Artifact model and identity derivation unchanged. |
| IS-0011, IS-0012, IS-0014 | Workspace model, Formation, and Confidence unchanged — the surface produces no Attachment and no confidence. |
| IS-0020 | Collector contract unchanged; the producer is a trusted declaration channel, not a collector. |

---

# Migration / Backward Compatibility

- Existing Observations of all frozen schemas (including RFC-0012's) replay identically;
  none migrate.
- The new schema is an additive schema identity (the established precedent: the "initial"
  set in IS-0003 §4.1 is open).
- Existing persisted Workspaces and Snapshots remain valid and are never rewritten.
- Existing plans without a surface remain valid; the surface is an additive component with
  an empty default (complete plans without declarations behave exactly as today).
- Existing designations, retrieval, grouping, and Continue behavior are unchanged.
- Historical declarations remain canonical history; only the latest valid one contributes
  to the Current Continuation Surface.

---

# Negative Cases (testable requirements)

1. No declaration → empty surface; explicit insufficiency; no guessed surface.
2. Declaration referencing an unwitnessed subject → rejected; no partial declaration.
3. Declaration of one subject → rejected.
4. Co-membership evidence (RFC-0012) never creates a surface.
5. Recency, frequency, focus, temporal grouping never create a surface.
6. A surface never changes Workspace membership or confidence.
7. A surface never creates, merges, or splits Artifacts.
8. A surface never changes the Resume Point, Next Step, Context Chain, or Blockers.
9. WorkGrouped relatedness does not imply continuation.
10. A WorkDesignated does not imply a surface; a surface does not imply a designation.
11. Historical membership survives: an old member of the Workspace remains a member after
    the surface changes.
12. Supersession: a later declaration replaces the Current Continuation Surface; the
    earlier declaration remains canonical history.
13. Per-Workspace intersection: a declaration spanning two Workspaces yields each
    Workspace's own subset.
14. Replay determinism: identical logs → identical surfaces, identical derivation outcomes.
15. Incremental and full derivation agree.
16. Artifact identity of every declared subject is unchanged by the declaration.

---

# Tests Required (upon acceptance and implementation)

- **Validation:** unwitnessed subject rejection; one-subject rejection; malformed
  declaration rejection; no-partial-declaration invariant.
- **Encoding:** order-independence of the canonical record; canonical ordering rule.
- **Supersession:** latest-valid-wins; earlier declaration stops contributing.
- **Derivation:** per-Workspace intersection; empty surface; surface on Complete and
  Insufficient outcomes; surface never gates completeness.
- **Boundary:** surface never feeds Formation, confidence, Resume Point, Next Step,
  Context Chain, Blockers, or execution selection.
- **Identity:** artifact identities unchanged; reference-only handling identical to
  WorkDesignated.
- **Replay:** full canonical replay ≡ incremental derivation; restart preserves the
  Current Continuation Surface.
- **Backward compatibility:** pre-RFC-0013 observations and Workspaces replay unchanged.
- **Scenario suite:** the 24 scenarios in the mission (one target; many historical
  artifacts; tool switching A→B→C; old tool historical-but-not-current; two interleaved
  Workspaces; same application unrelated work; same repository multiple tasks; explicit
  insufficiency; replay equivalence; identity and membership unchanged).
- **Real machine:** declaration through the trusted channel on macOS; restart; surface
  survives; replay consistency via evo-doctor.

---

# Open Questions

1. **One active surface vs. many.** This RFC defines a single Current Continuation
   Surface (latest-valid-supersedes), mirroring the single current designation. A future
   "one surface per body of work" model would require the evidence to reference Workspace
   identity at emission time, which is circular today (Workspaces are derived). Deferred.
2. **Surface + designation coherence.** The RFC treats WorkDesignated and the surface as
   independent inputs; the derivation consumes each under its own contract. A future
   amendment could define a combined declaration (primary + supporting resources) — that
   would change RFC-0011 and is explicitly out of scope here.
3. **Execution semantics.** The plan carries the surface; how execution maps surface
   members to executable targets (resource identity, launch ordering, failure handling) is
   the next milestone and requires its own resource/executable identity contract (RFC-0010
   already states Work Continuity does not solve this).
4. **Surface lifetime on inactivity.** This RFC makes no use of time: a declaration stays
   current until superseded. A future "expire stale surfaces" rule would introduce time as
   continuation evidence and is rejected unless a later RFC justifies it against RFC-0010.

---

# Non-Goals

This RFC does NOT define or authorize:

- ranking, recency-based or frequency-based relevance, focus-based relevance, "recent N
  resources", application categories, or any heuristic continuation;
- LLM/semantic/embedding-based continuation;
- selective restoration execution — opening, focusing, or launching any resource;
- resource identity or executable target identity;
- changes to Workspace Formation, Artifact identity, WorkDesignated, or co-membership
  semantics;
- a new persisted canonical object beyond the Observation Schema (the surface in the plan
  is derivation output, not new canonical state);
- UI;
- a per-Workspace surface (see Open Questions).

---

# Rationale

The product question — "what should Evo bring back to me now" — cannot be answered by
derivation over existing evidence: RFC-0010 rejects every inferential signal, RFC-0012
rejects co-membership, and RFC-0011 rejects per-Artifact currency. The only remaining
honest authority for where work continues is the user's own declaration (Law IX), and the
accepted precedent for such declarations is the WorkDesignated pattern: a witnessed act
through a trusted origin, referencing already-witnessed subjects, latest-valid-supersedes,
persisted canonically, replayable. This RFC generalizes that pattern from one subject to a
set of subjects — nothing more.

The separation is preserved at every layer: Workspace answers "what belongs together"
(unchanged, RFC-0012); ContinuationSurface answers "where the work continues right now"
(this RFC); Restoration Plan carries the surface without gating completeness (IS-0021
unchanged in §25.2–§25.5); execution is downstream. Historical membership is never
rewritten, so the "three months later" and "tool switching" scenarios hold: old tools
remain historical members; only the user's declaration makes resources current.

---

# Architectural Law

> **The current continuation of a multi-resource Workspace shall be witnessed as an
> explicit user declaration through a trusted capture origin, resolved to already-witnessed
> subjects, superseded only by a later declaration, and never derived from recency,
> frequency, focus, co-membership, membership, or any other inferential signal (Law I,
> Law II, Law IV, Law V, Law VI, Law IX, Law XIII, Law XVII).**

---

# Implementation Record (accepted scope, implemented)

Implementation of this RFC is complete and verified:

- **Schema:** `OBS-CONTINUATION-SURFACE/v1` (IS-0003 §4.1.8), reference-only, canonical
  ordering (Required Subject = lexicographically smallest; `ContinuationSubject` facts in
  ascending order), validated (≥2 distinct subjects, no duplicates, canonical order).
- **Producer:** the daemon-owned declaration channel (`user_continuation` capture origin)
  validates every subject against the canonical Observation log, canonicalizes the set,
  forwards a ContinuationSurface signal; the vertical runtime accepts, persists, and
  re-derives. Unwitnessed subjects, <2 subjects, and duplicates are rejected before any
  canonical Observation exists.
- **Derivation:** `RestorationInput` carries the surface resolved to Artifacts by the
  Artifact layer; `derive_restoration_plan` computes the per-Workspace intersection in
  ascending ArtifactId order and carries it on both Complete plans and Insufficient
  outcomes (IS-0021 §25.1A; IS-0019 CS-1–CS-4). The surface never gates completeness and
  never changes Resume Point / Next Step / Context Chain / Blockers.
- **Replay:** `replay_current_continuation_surface` re-derives the current surface from the
  canonical Observation log; the derived index and full replay agree (differential test).
- **Verification:** unit tests (validation, encoding order-independence, derivation
  intersection, supersession, backward compatibility) and daemon end-to-end tests
  (per-Workspace intersection, restart preservation, replay equivalence, membership and
  Artifact identity unchanged). Full workspace suite: 611 passed, 0 failed, 1 ignored.
