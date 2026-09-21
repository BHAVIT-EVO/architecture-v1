# RFC-0012 — Canonical Co-Membership Evidence

**Status:** Superseded by RFC-0014 (Engagement Contract)

**Version:** 1.0 (final)

---

## Supersession Record

This RFC is superseded **in full**. It remains in the repository as the record of
a rule that was tried and found wrong, not as authority. Nothing in it is
normative. Implementations MUST NOT consult it.

**The old rule.** Shared repository membership is a distinguished relational
signal of work continuity, emitted as `OBS-REPOSITORY-MEMBERSHIP` and scored
with privileged weight during Workspace Formation.

**Why it prevented the product from working.** Two independent reasons.

1. *It designed the relational model around Git.* Most work is not in a
   repository. A researcher with a browser, a PDF, and a terminal; a student
   with a document, a spreadsheet, and a chat; a designer with a drawing, a
   folder, and a reference page — none of them produce this evidence at all, so
   the only named relational signal in the contract was one that most bodies of
   work cannot generate. Everything not in a repository was left to be related
   by nothing.
2. *It over-grouped exactly where separation matters most.* Two entirely
   unrelated activities inside one repository share repository membership
   completely and at maximum strength. A privileged signal that is identical for
   related and unrelated work cannot distinguish them, so it fused task
   boundaries that the product must preserve.

**What replaces it.** Shared containment — a Path's parent directory, an
Address's origin — is one structural relationship among several, carrying no
privileged weight, no dependence on Git, and no knowledge of what kind of
container it is looking at. It contributes the smallest share of affinity, and
it cannot originate a body of work by itself. Normative statement: RFC-0014
Requirements 1, 3, 6, and 7.

**Status of the observation schema.** `OBS-REPOSITORY-MEMBERSHIP` remains a
frozen canonical schema (RFC-0001 immutability: a schema that has been written
to the log can never be withdrawn), and historical records of it remain valid
evidence. What is withdrawn is its *privileged interpretation*. Records already
in the log are read as ordinary structural evidence.

**Dependent documents reconciled.** RFC-0003 v2.0 Amendment 2 withdraws the
clause that cited this RFC. `evo-daemon`'s `verify_collectors` example still
labels the schema's facts by their RFC-0012 names, which is correct: those are
the frozen field names, not a live scoring rule.

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
- RFC-0003
- RFC-0004
- RFC-0010
- RFC-0011
- IS-0001 — Observation
- IS-0002 — Observation Language
- IS-0003 — Observation Schemas
- IS-0004 — Artifact Model
- IS-0010 — Identity Derivation Contract
- IS-0011 — Workspace Model
- IS-0012 — Workspace Formation
- IS-0014 — Confidence Score
- IS-0019 — Restoration Model
- IS-0020 — Collector
- IS-0021 — Restoration Derivation

---

# Abstract

Workspace Formation currently connects repeated observations of the *same* Artifact only.
Distinct Artifacts can never share a Workspace, because no canonical Observation relates two
distinct entities. Evo therefore cannot represent a real body of work — a repository, an
editor, a browser, a conversation, and a chat client working on one project produce one
Workspace per Artifact.

This RFC defines the missing evidence class: **canonical co-membership evidence** — canonical
Observations that witness a relationship between two distinct canonical subjects. It defines
two producers of that evidence:

1. **RepositoryMembership** — the directly witnessed fact that a file, directory, or commit
   is a member of a repository.
2. **WorkGrouped** — the directly witnessed fact that the user explicitly declared two
   already-witnessed subjects to be related work.

It defines how Workspace Formation consumes this evidence to attach distinct Artifacts to one
Workspace, and it draws the hard boundary that keeps this evidence out of Work Continuity,
Restoration Derivation, and execution.

The governing separation is:

**Same Artifact ≠ Related Work ≠ Currently Relevant ≠ Restore Together.**

This RFC establishes only the second concept. It deliberately does not change continuation,
restoration, or execution semantics.

---

# Motivation

The Product promises that Evo "quietly learns which resources belong together" and
reconstructs "the complete environment that thought belongs in." RFC-0003 defines a Workspace
as Evo's best explanation that multiple Artifact histories describe *one coherent body of
work*. TRACE-0001 Phase 3 requires Evo to identify the body of work the user intends to
continue.

The current canonical vocabulary cannot deliver any of this: each frozen Observation schema
witnesses exactly one entity, Artifact identity is a one-way hash of one subject, and
Formation's only evidence channel is same-Artifact attachment. A body of work such as
"the Evo project" — files, commits, browser research, a chat client, an editor — is
structurally unrepresentable. Before Evo can restore the *relevant subset* of a body of work,
it must first be able to represent the body of work itself.

Relatedness must be established before relevance can be judged. This RFC establishes
relatedness. Relevance and selective restoration remain future work.

---

# Scope

This RFC defines:

- the semantic meaning of co-membership evidence;
- the canonical concepts and Observation Schemas for `RepositoryMembership` and `WorkGrouped`;
- the producer (witness) contracts for both;
- repository identity and its edge cases (worktrees, submodules, detached HEAD, relocation);
- the deterministic Workspace Formation attachment rule that consumes the evidence;
- the boundary that keeps co-membership out of Work Continuity and Restoration;
- the exact amendments required to frozen documents.

This RFC does NOT define:

- ranking, recency, frequency, similarity, or any heuristic grouping;
- continuation evidence beyond RFC-0010/0011;
- selective restoration, the "current relevant subset", or the workspace surface;
- execution;
- UI;
- new collectors beyond extension of the existing git/filesystem witnessing and the
  existing designation surface;
- a new architectural primitive or a new persisted canonical object beyond new Observation
  Schemas.

---

# Definitions

**Co-Membership Evidence**

Canonical Observations that witness a relationship between two distinct canonical subjects,
accepted through the standard Observation acceptance pipeline (IS-0001) and consumed by
Workspace Formation.

**Repository**

An external entity: a version-controlled body of work with a commit history. Identified
canonically by a deterministic repository identity (below). A repository is not a folder.

**Repository Identity**

The canonical text value identifying one repository: the repository's git directory
(git-common-dir), as resolved by the capture mechanism at witness time. It unifies all
worktrees of one repository and is stable across commits. Relocation changes it (an ordinary
identity-revision event, see §Repository Relocation).

**Member**

A canonical subject (file path, directory path, or commit hash) that a RepositoryMembership
observation declares to be a member of a repository.

**Member Resolution**

The deterministic derivation of the member's Artifact identity from the member subject,
identical to the resolution rule RFC-0011 §4 applies to designation subjects. Membership
observations resolve members; they never create member Artifacts.

**WorkGrouped Declaration**

The directly witnessed act of the user declaring two already-witnessed subjects to be
related work.

**Origination**

The creation of a new Workspace by Workspace Formation. Under this RFC, Workspace
origination remains exclusively content-evidence-driven; co-membership evidence only
extends an existing Workspace (see Formation Interaction).

---

# The Problem

Workspace Formation (`form_workspace` → `candidate_score`) scores a Candidate Workspace by
the maximum confidence of that Workspace's Attachments whose `artifact_id` equals the
incoming Artifact's id. Artifact identities are one-way hashes of
`schema|version|fact|subject`. Consequently:

- every distinct witnessed subject yields a distinct Artifact;
- every Artifact scores zero against every Candidate Workspace that does not already contain
  it;
- a zero score for all candidates produces `RecognizeNew`;
- **no canonical Observation relates two distinct subjects, so no rule — present or future —
  implemented over existing evidence can connect two distinct Artifacts.**

This is not an implementation bug. It is the absence of an evidence class. The five frozen
schemas each witness exactly one entity, and IS-0002 §6 forbids "Workspace membership" as an
Observation concept — the language may witness ground facts, never Evo's interpretation.

---

# Co-Membership Evidence — Meaning

Co-membership evidence is the canonical record of a directly witnessed relationship between
two distinct canonical subjects.

**What it proves:**

- `RepositoryMembership(F, R)`: the witnessed structural fact that entity F is a member of
  repository R.
- `WorkGrouped(S1, S2)`: the witnessed act that the user declared S1 and S2 to be related
  work.

**What it does NOT prove:**

- user intent;
- importance, priority, or relevance;
- that the work is unfinished;
- task identity;
- that R is exactly one body of work (a repository may contain several);
- that anything should be restored;
- Work Continuity (see Separation from Work Continuity).

Co-membership evidence records the underlying witnessed fact, never Evo's interpretation.
It is never a Workspace-membership Observation.

---

# Evidence Class 1 — RepositoryMembership

## Canonical Fact

The directly witnessed fact that entity F is a member of repository R.

The truth condition is a structural property of F at witness time, determinable through the
git capture mechanism. It requires no interpretation of intent and no inference about the
user — the same epistemic pattern as "a file was saved."

## Proposed Observation Schema (IS-0003 §4.1.6)

- **Schema Identifier:** `OBS-REPOSITORY-MEMBERSHIP`
- **Schema Version:** 1
- **Observation Type:** RepositoryMembership
- **Required Subject:** the member — the file, directory, or commit to which the witnessed
  membership fact refers.
- **Required Canonical Concepts:** `RepositoryMembership` (the witnessed fact);
  `Repository` (the repository identity, a subject-valued canonical value).
- **Optional Canonical Concepts:** None.
- **Platform neutrality:** the schema SHALL NOT require application identity, process
  identity, or editor identity. The member subject is platform-typical text (a path or
  commit hash), exactly as in the frozen schemas. The repository identity is canonical
  opaque text; Formation compares it for equality only and never interprets it.
- **References, does not create:** a RepositoryMembership observation SHALL NOT by itself
  establish an Artifact (member or repository). The member subject resolves to the Artifact
  established by content Observations (OBS-FILE-SAVED, OBS-COMMIT-MADE) through the
  deterministic identity derivation, per the RFC-0011 §4 resolution pattern. The
  acceptance/runtime pipeline SHALL apply the same handling this RFC's predecessor applies
  to `OBS-WORK-DESIGNATED`.

## Repository Identity

The repository identity is the deterministic canonical text value derived from the
repository's git directory (git-common-dir) at witness time.

- **Main working tree:** `<repo>/.git` — the witnessed reflog path `<repo>/.git/logs/HEAD`
  strips `/logs/HEAD` to yield it.
- **Linked worktree:** the witnessed reflog path
  `<repo>/.git/worktrees/<name>/logs/HEAD` resolves to the same common dir `<repo>/.git`,
  so all worktrees of one repository share one identity.
- **Submodule:** a submodule is its own repository with its own git directory
  (e.g. `<superproject>/.git/modules/<name>`), producing a distinct identity. Submodules are
  therefore separate repositories and separate membership domains. The producer SHALL also
  recognize submodule reflog paths so submodule commits are witnessed as CommitMade.
- **Bare repository:** resolves to its own directory.

The identity is a canonical value, not a canonical Artifact, in this RFC. A repository
Artifact (should the product later need "restore the repository") is a future schema whose
subject is the repository identity; it is out of scope here.

## Witnessing Mechanism

The producer is the existing git/filesystem capture path (the FSEvents-based collector and
the daemon pipeline), extended:

- On a witnessed file-save event: resolve whether the saved path is inside a repository, and
  if so emit `RepositoryMembership(member = path, repository = identity)`. The producer MAY
  invoke git at capture time to resolve membership and identity; Formation SHALL NOT.
- On a witnessed commit event (reflog append): emit
  `RepositoryMembership(member = commit hash, repository = identity)` alongside the
  existing CommitMade observation. This unifies commit Artifacts into the same Workspace as
  the repository's files with the same evidence class.
- Emission is per witnessed event; no suppression. Repeated identical membership
  observations are harmless canonical facts (the identity set deduplicates naturally),
  exactly as repeated saves are.
- Provenance (IS-0001 §5A): Observation Source identifies the git/filesystem capture
  channel; Observation Time is the witness moment; Context contains only directly observed
  circumstances. Provenance SHALL NOT contain interpretation.

## When an Observation MUST NOT Be Emitted

- When the repository identity cannot be resolved truthfully at witness time (IS-0020 §19):
  no observation is emitted. Failure to represent is silence, never fabrication.
- When the member subject is not a file, directory, or commit the collector actually
  witnessed.

## Replay Behavior

RepositoryMembership observations are canonical, immutable, append-only Observations. Given
identical Observation history and identical Formation rules, replay reproduces identical
Workspace understanding. The membership facts never depend on live git state at replay time.

## Repository Relocation

The repository identity is derived from the git directory path. If the user moves the
repository, subsequent membership observations carry the new identity; earlier observations
remain canonical history. The discontinuity is honest and mirrors what already happens to
file subjects when files move (IS-0007 identity revision). No historical Observation is
modified.

---

# Evidence Class 2 — WorkGrouped

## Canonical Fact

The directly witnessed fact that the user explicitly declared two already-witnessed subjects
to be related work.

This is the RFC-0011 pattern applied to relatedness: the truth condition is the performance
of the declaration act through a trusted capture origin (`user_grouping`), exactly as
WorkDesignated's truth condition is the performance of the designation act. The user's own
grouping judgment is the strongest honest authority available (Architectural Law IX — the
user owns judgment).

## Proposed Observation Schema (IS-0003 §4.1.7)

- **Schema Identifier:** `OBS-WORK-GROUPED`
- **Schema Version:** 1
- **Observation Type:** WorkGrouped
- **Required Subject:** the first subject of the declared pair under the canonical pairing
  rule (lexicographically smaller canonical subject string).
- **Required Canonical Concepts:** `WorkGrouped` (the witnessed declaration);
  `CoMember` (the second subject of the pair, subject-valued).
- **Optional Canonical Concepts:** None.
- **Platform neutrality:** the schema SHALL NOT require application, browser, or process
  identity.
- **References, does not create:** a WorkGrouped observation SHALL NOT by itself establish an
  Artifact. Both subjects resolve to Artifacts already established by content Observations.

## Declaration Semantics

- A declaration act naming a set of n subjects emits one observation per unordered pair
  (n choose 2).
- Each pair observation is canonicalized so `subject = min(S1, S2)` and
  `CoMember = max(S1, S2)` under canonical string order. This makes the declaration
  deterministic and order-independent.
- The producer (the desktop declaration surface) SHALL offer only subjects already witnessed
  in the canonical Observation log (the RFC-0011 producer rule). A declaration referencing an
  unwitnessed subject is rejected before any canonical Observation is created; no partial
  declaration ever exists.
- Formation consumes the pairwise facts; transitive grouping (A~B, B~C ⇒ A,B,C in one body)
  is an emergent property of Formation, never a separate declaration.

## When It Must NOT Be Emitted

- For subjects not already present in the canonical Observation log.
- For a declaration Evo cannot witness (the act must be performed through the trusted
  capture origin).

---

# Artifact Interaction

- Co-membership evidence **references** existing Artifact identities; it never creates,
  merges, splits, or modifies them (RFC-0002 Req 2, Req 7; IS-0004 I-3).
- Artifact identity derivation (IS-0010) is **unchanged**. Membership and declaration
  observations are excluded from Artifact establishment exactly as WorkDesignated
  observations are today (RFC-0011 §4), so no second Artifact is created for a member
  subject — no identity fragmentation.
- An Artifact SHALL NOT encode co-membership. Co-membership lives in the Observation layer;
  the Artifact remains answerable only to the identity question.
- No Artifact can exist solely because of co-membership evidence: every Artifact still
  originates from content Observations.

---

# Formation Interaction

## 5A Amendment (IS-0014 — the only Formation-input amendment)

**Proposed amendment to IS-0014 §5A (Derivation Contract):** after the list "Confidence
Score derivation SHALL consume only: the canonical Artifact under evaluation; one Candidate
Workspace; canonical computational objects owned by that Candidate Workspace", add:

> canonical co-membership evidence — accepted canonical Observations of the Co-Membership
> Evidence class (RFC-0012) — establishing a relationship between the Artifact under
> evaluation and canonical computational objects owned by that Candidate Workspace;

The "SHALL NOT consume" list (user state, runtime state, retrieval results, restoration
state, language model output, learned behavior, non-canonical representations) is unchanged.

No other Formation document requires amendment: the new evidence is canonical Observation
input (IS-0012 Inputs already admits "canonical Observation"); Attachment Evaluation still
produces exactly one Confidence Score per candidate (IS-0012 Stage 2); and the comparison
algorithm is explicitly an implementation detail (IS-0012 Stage 3).

## Attachment Evaluation

For an incoming Artifact A and Candidate Workspace W:

- **Same-artifact support** (unchanged): the maximum confidence of W's Attachments whose
  artifact_id equals A.
- **Co-membership support** (new): the maximum over W's attached Artifacts B of the
  co-membership strength between A and B, where strength is:
  - **1.0** when a canonical `WorkGrouped` observation declares A and B;
  - **0.8** when canonical `RepositoryMembership` observations place A and B in the same
    repository. The value is below 1.0 because a repository may contain more than one body
    of work; the declaration, being the user's own word, is conclusive.
  - 0.0 otherwise.
- **Candidate score** = max(same-artifact support, co-membership support).

## Workspace Decision

Exactly one outcome, per IS-0012 Stage 3, over the extended scores:

- If exactly one Candidate Workspace has the unique maximal score and that score is greater
  than zero, the Artifact attaches to that Workspace.
- If multiple Candidate Workspaces tie for the maximal score, the candidate with positive
  same-artifact support wins when exactly one such candidate exists (same-artifact
  continuity is authoritative over co-membership at equal strength; RFC-0012 v1.0).
- Otherwise a new Workspace is recognized.

This preserves today's behavior exactly when no co-membership evidence exists (the
same-artifact channel is unchanged), and it is deterministic and replayable.

## The Origination Invariant

**Co-membership evidence extends Workspaces; it never originates them.** A Workspace is
created only when a content Observation establishes a previously unseen Artifact. Because
co-membership observations never create Artifacts and only attach Artifacts to Workspaces
that already contain a co-membered Artifact, membership alone cannot form a Workspace. This
keeps RFC-0003 Requirement 1 (Workspaces derived exclusively from Artifact histories) and
Requirement 3 (explanatory, not container) intact.

**Sufficiency floor.** The trigger is a content Observation or an explicit user declaration
naming the Artifact (the invariant above holds for co-membership observations), but not every
trigger originates: RFC-0003 Requirement 5 grounds origination in the evolution of Artifact
histories, never instantaneous state. A first-ever witness of a standalone Artifact therefore
attaches nowhere (ARCHITECTURE §5 third outcome) — the Observation and Artifact remain
canonical, but no Workspace is created. Origination is justified exactly when one of the
following holds:

- the content Observation carries witnessed continuity: an earlier content Observation
  established the same Artifact, or the Artifact carries canonical repository-membership
  evidence; or
- an explicit user declaration names the Artifact (WorkDesignated, RFC-0011; WorkGrouped,
  RFC-0012; ContinuationSurface, RFC-0013) and the Artifact belongs to no remembered
  Workspace. The user's declaration is the strongest relational fact available — it is
  the user's own word, never recency, frequency, application identity, or any inference
  rejected by the Competing-World Test. A declaration never re-originates an Artifact
  that already belongs to a Workspace.

## Confidence Semantics

Confidence remains evidential strength only (IS-0014 §5; W-7). Co-membership confidence is
never importance, priority, relevance, or restoration order.

---

# Competing-World Test

A rule may serve as co-membership evidence only when it produces different evidence in:

- **WORLD A** — F1 and F2 genuinely belong to the same body of work;
- **WORLD B** — F1 and F2 are merely on the same machine / merely associated, or F2 belongs
  to a different body of work.

| Signal | WORLD A evidence | WORLD B evidence | Verdict |
|---|---|---|---|
| Temporal co-occurrence ("worked on together") | F1, F2 observed near each other | F1, F2 observed near each other (the user interleaves projects) | **Reject** — identical |
| Recency / observation order | F2 after F1 | F2 after F1 | **Reject** — identical |
| Frequency | F2 frequent | F2 frequent | **Reject** — identical |
| Application identity | same app | same app (VS Code edits both projects) | **Reject** — identical |
| Title similarity | similar titles | similar titles | **Reject** — identical |
| Shared URL domain | same domain | same domain (personal and work research share domains) | **Reject** — identical |
| Filesystem proximity (shared folder) | same folder | same folder (unrelated files share directories) | **Reject** — identical |
| **Repository membership** | F1, F2 both carry membership(R) | F1 carries membership(R); F2 carries membership(Q) or none | **SURVIVES** — the evidence differs |
| **Explicit user declaration** | user declared F1, F2 related | no declaration | **SURVIVES** — the evidence differs |

Repository membership is the one automatic signal whose evidence discriminates the two
worlds, because a repository is a version-controlled history of co-evolution — a relational
continuity fact — not a resemblance or co-occurrence fact. This is why it may contribute
evidence of Workspace continuity while co-occurrence, recency, frequency, app identity,
title/domain similarity, and filesystem proximity may not.

---

# Separation from Work Continuity

Co-membership evidence SHALL NOT feed, directly or indirectly:

- the Resume Point (IS-0021 §25.2);
- the Next Step (IS-0021 §25.5);
- the Context Chain (IS-0021 §25.3);
- Blockers (IS-0021 §25.4);
- any restoration or execution selection.

RFC-0010 already lists "Artifact co-membership" and "Workspace membership" among its
prohibited Work Continuity sources. This RFC extends the identical prohibition to the new
evidence class explicitly, so that the co-membership evidence defined here can never be
reinterpreted as continuation evidence later. Work Continuity remains exclusively
designation-driven (RFC-0011); insufficient designation remains explicit insufficiency
(IS-0021 §25.8). A Workspace may contain many related Artifacts while Restoration still
derives exactly one Resume Point and one Next Step from the current designation and an empty
Context Chain.

**Relatedness is not a reason to restore. This RFC establishes only relatedness.**

---

# Replay

Given identical:

- canonical Observation history (including the new evidence);
- identical Formation rules as frozen by this RFC;

Workspace Formation SHALL produce identical Workspace understanding. No derivation decision
SHALL depend on wall-clock time, live git/filesystem state, OS state, iteration order, or
any non-canonical input. Replay regenerates Workspaces; it never rewrites historical
Observations or Snapshots (RFC-0003 Req 7; Architecture §7).

---

# Backward Compatibility

- Existing Observations of the five frozen schemas replay identically; none migrate.
- The new schemas are additive schema identities (the RFC-0011 precedent: the "initial" set
  in IS-0003 §4.1 is open).
- Existing persisted Workspaces (v1/v2 records) remain readable and valid.
- Historical Snapshots are never rewritten. The new attachment rule affects only new
  Formation operations and replay under the new rule (RFC-0004 interpretation evolution).
- Existing per-Artifact Workspaces remain valid historical understanding.
- Existing designations, retrieval, and Continue behavior are unchanged.

---

# Failure / Insufficiency Behavior

- Unresolvable repository identity → no membership observation (silence).
- Member subject that resolves to no existing Artifact → the membership observation remains
  canonical evidence but contributes no Attachment until content Observations establish the
  member (IS-0014 §5A inputs are canonical evidence; Formation never fabricates).
- Absence of co-membership evidence → existing behavior (separate or insufficient
  Workspaces). No confidence, no fallback, no guessing (Laws V and VI).
- Tied maximal candidate scores → the candidate with same-artifact continuity wins; a
  tie with no same-artifact support produces a new Workspace (the conservative,
  deterministic outcome). Same-artifact continuity is authoritative over co-membership
  at equal evidential strength.

---

# Privacy / Local-First

- All co-membership evidence is produced and consumed locally. No network call, external
  service, or remote state is involved (Law XIII; Architecture §10).
- Repository identity is local canonical text; it is never transmitted.
- Provenance records only directly observed capture circumstances (IS-0001 §5A); no
  high-fidelity capture is retained.

---

# Document Impact (Phase 2)

## Documents requiring amendment (proposed; applied only upon acceptance)

| Document | Clause | Amendment |
|---|---|---|
| IS-0002 | §4 Vocabulary | Add concepts `RepositoryMembership` (§4.6) and `WorkGrouped` (§4.7) with semantics and SHALL-NOT-imply lists, each satisfying the §9 gate (directly observable fact; stable platform-independent semantics; not representable by an existing concept; justified by demonstrated requirements). |
| IS-0003 | §4.1 | Add schemas `OBS-REPOSITORY-MEMBERSHIP/v1` (§4.1.6) and `OBS-WORK-GROUPED/v1` (§4.1.7) as specified above, with the "references, does not create" clause and platform-neutrality notes. |
| IS-0014 | §5A | Expand the permitted confidence-derivation inputs with canonical co-membership evidence, as specified in Formation Interaction above. Nothing else in §5A changes. |
| RFC-0003 | Requirement 2 | Explicit authorization clause (see RFC-0003 §2 resolution below). |

## Documents deliberately untouched (verified necessary to leave frozen)

| Document | Reason |
|---|---|
| Constitution, Product, Architecture, Architectural Laws | No contradiction; the Architecture already frees the attachment function to change (§8). |
| RFC-0001, RFC-0002 | Observation semantics and Artifact identity unchanged. |
| RFC-0010, RFC-0011 | Continuation evidence unchanged; this RFC reinforces their prohibition of co-membership as continuation evidence. |
| IS-0001 | Acceptance pipeline unchanged; the new schemas flow through the existing pipeline with the RFC-0011 schema handling. |
| IS-0010 | Identity derivation unchanged — the new evidence references, never creates. |
| IS-0011 | Workspace model unchanged (components remain Identity, Lifecycle, Attachment Set, Snapshot History). |
| IS-0012 | Unchanged — inputs already admit canonical Observations; Stage 3 already leaves the comparison algorithm to implementation; this RFC freezes the concrete rule. |
| IS-0019, IS-0021 | Restoration semantics unchanged by this RFC. |
| IS-0020 | Collector contract unchanged; the producer extends an existing capture path under the existing contract. |

## RFC-0003 §2 Resolution

RFC-0003 Requirement 2 forbids explaining continuity by "topical similarity, application
similarity, temporal proximity, file organization, or any other isolated observational
feature," while stating "Similarity may contribute evidence. Similarity MUST NEVER define
Workspace identity."

**Resolution adopted by this RFC (explicit, not silent):** repository co-membership is a
relational continuity signal, not a resemblance or file-organization signal, and this RFC
makes it contributing evidence only — it never defines Workspace identity, and the
Origination Invariant guarantees membership alone cannot create a Workspace. To remove all
interpretive ambiguity rather than rely on re-reading a frozen clause, this RFC proposes the
smallest explicit amendment to RFC-0003 Requirement 2, adding one sentence:

> Repository co-membership evidence, as defined by RFC-0012, is a relational continuity
> signal and MAY contribute evidence of Workspace continuity; it SHALL NEVER define
> Workspace identity alone.

If the community instead determines that Requirement 2 already permits this contributing
evidence, the amendment is unnecessary; the behavioral contract of this RFC is unchanged
either way.

---

# Non-Goals

This RFC does NOT define or authorize:

- ranking, recency-based or frequency-based continuation, "apps used together", title or
  domain similarity, filesystem proximity, or session-based grouping;
- inference of user intent;
- Formation inspecting live filesystem or git state (all evidence is canonical in the
  Observation log);
- changes to Artifact identity derivation;
- a Workspace-membership Observation (the Observation records the underlying witnessed fact,
  never Evo's interpretation);
- a sixth architectural primitive, a new persisted canonical object, or fields on frozen
  objects;
- selective restoration, the "current relevant subset", or the workspace surface;
- UI work;
- new collectors beyond extension of the existing git/filesystem witnessing and the
  existing designation surface.

---

# Open Questions

1. **Repository identity as canonical value vs future repository Artifact.** This RFC treats
   the repository as a canonical value. If the product later needs "restore the repository,"
   a schema whose subject is the repository identity can establish a repository Artifact.
   Deferred.
2. **Whether submodule commits should unify into the superproject Workspace.** This RFC keeps
   submodules as separate repositories (separate membership domains). A future RFC could
   authorize superproject/submodule co-membership evidence; the competing-world test for
   that relationship is not established here.
3. **Whether a declared group should supersede or be additive to repository-derived
   grouping.** This RFC treats declarations as additive evidence (they never remove
   repository-derived membership). Supersession semantics are deferred.
4. **Retroactive merging.** Because IS-0012 Stage 3 permits exactly one outcome per
   processed Artifact (attach or recognize new), a declaration between two Artifacts
   already living in two distinct Workspaces does not retroactively merge those
   Workspaces: same-artifact continuity keeps each Artifact in its current home, and
   the declaration routes newly witnessed Artifacts into the declared Workspace.
   Retroactive merge would require detachment semantics outside IS-0012 Stage 3 and is
   explicitly out of scope for this RFC.

---

# Rationale

Evo's Workspace is defined as the explanation that multiple Artifact histories describe one
coherent body of work (RFC-0003). The evidence needed to form that explanation — a canonical
relationship between distinct entities — was absent. This RFC adds the smallest truthful
such evidence: repository membership, witnessed from the version-control history that is the
strongest automatic record of co-evolution, and the user's own declaration, the strongest
honest authority available (Law IX). Both are canonical, immutable, replayable facts; both
flow into a deterministic Formation rule; both are barred from continuation. This preserves
the architecture's epistemic separation (Law IV, Law XVII): the Observation records the
witnessed fact, and only Formation interprets it as relatedness.

The separation of relatedness from restoration follows TRACE-0001 Phase 4 ("Historical
association alone is insufficient to justify restoration") and IS-0019's principle that
Restoration minimizes cognitive reload rather than maximizing historical completeness.
Relatedness is the precondition; relevance and selective restoration are the next milestone.

---

# Architectural Law

> **Co-membership shall be witnessed as canonical fact, interpreted by Workspace Formation
> as contributing evidence only, never as continuation, and kept silent where evidence is
> absent (Law I, Law II, Law IV, Law V, Law VI, Law IX, Law XVII).**
