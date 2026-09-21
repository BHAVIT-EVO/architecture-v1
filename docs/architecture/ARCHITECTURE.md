# Evo — Architecture

**Status:** Frozen, with Amendment 1 (see Amendment Record)
**Audience:** Engineers joining Evo, before reading any RFC
**Purpose:** Understanding, not implementation

This document explains what Evo is, why it is shaped the way it is, and which decisions are permanent versus which are expected to change. Read this before you read any RFC. If an RFC contradicts this document, the RFC is wrong.

---

## Amendment Record

### Amendment 1 — Workspace becomes derived, not persisted

Amended sections: **§3, §4, §5, §6, §7, §9, §12.**
Introduced by: RFC-0014 (Engagement Contract).
Consequential document: RFC-0003 v2.0.

**§2 is not amended, and that is the point.** The computational model — *state is
never stored; state is derived by replaying a confidence-scored interpretation
function over an immutable observation log* — is unchanged and is now, for the
first time, actually obeyed.

Sections 3, 6, and 7 carved a deliberate exception out of §2 for the Workspace,
justified by restoration speed: a Workspace was "the one deliberate exception to
'everything derived is disposable'". That exception is the root permission for
Evo's central product failure.

The mechanism, precisely. A persisted Workspace has to be *created* at some
moment, and the only moment available is the first sighting of a resource —
before any relational evidence exists. Once created, it is a durable row that
nothing downstream can unmake, because §5 forbade hard merges and §7 forbade
replaying Snapshots. Every later, better reading of the evidence therefore had
to be reconciled *against a decision taken when the evidence was one observation
long*. The result was mechanical and unavoidable: one Workspace per resource. A
music player, a chat window, a search, a file the operating system rewrote — each
became a durable body of work because each had, once, been seen. Home mirrored
the observation log and called it work.

No amount of improvement to the attachment function could have fixed this. The
function was being asked the wrong question. It was asked *does this resource
attach to a nearby Workspace* — a question that presupposes the Workspace
already exists — when the question the product needs answered is *which of these
resources were being used together, for something, by someone who was there*.
That question cannot be answered one observation at a time, and it cannot be
answered at all against a small local candidate set.

**What Amendment 1 changes.** The Workspace exception is withdrawn. A Workspace
is a projection of an Engagement, derived from the Observation log on read, with
nothing about it persisted. Formation reads history rather than a local candidate
window. Snapshots are derived per sitting rather than frozen on a trigger.

**What it costs, honestly.** Restoration speed is no longer guaranteed by a
cached row; it is guaranteed by derivation being cheap enough. This is a real
trade the amendment accepts deliberately, and it is measured rather than assumed.
Two costs scale differently and are measured separately:

*Log length* — `evo-daemon`'s `bench_reload`, at 50 000 observations over 1 000
distinct resources: one-time full build 874 ms, then 4.2 ms per reload from the
incremental tail, against 961 ms per reload for a full reparse — a 231× steady-state
reduction. Log growth is therefore not the constraint; the index is refreshed from
the tail, never rebuilt.

*Distinct resources* — `evo-engagement`'s `bench_derivation`, measuring the whole
pipeline from acts through significance:

| distinct resources | derivation |
| --- | --- |
| 25 | 1.1 ms |
| 50 | 2.3 ms |
| 100 | 8.4 ms |
| 200 | 28 ms |
| 400 | 121 ms |
| 800 | 563 ms |
| 1 000 | 984 ms |

The curve is cleanly quadratic — the pairwise affinity stage is O(n²) in distinct
resources — and holds a constant ~2 µs per pair. At the scale of a real store
(490 distinct subjects in the checkout's own store) derivation is a few hundred
milliseconds, and one second at a thousand. This is the known scaling limit: it is
comfortable at present sizes and it is not indefinitely comfortable. The honest
statement is that it bounds how large a single store can grow before the affinity
stage needs blocking or pruning, and that no such bound is reached at the sizes
Evo is being built for.

**What it buys.** A better interpretation improves *all* history immediately,
with no migration, no reconciliation, and no stale rows surviving a rule change.
That is what §2 promised. The five primitives are unchanged; one of them simply
stops being an exception to the rule the document already stated.

Each amended section below carries an **Amended (A1)** note at the point of
change. The superseded wording is quoted so nothing is altered silently.

---

## 1. What Evo Is

Evo is a local-first system that keeps an immutable record of everything a person does on their computer, continuously and cheaply derives a best-effort, confidence-scored, always-revisable interpretation of what work that activity belongs to, and uses that interpretation to do one thing exceptionally well: put the person back into the state they were in when they stopped, in seconds, so that resuming work never again costs more attention than the work itself.

The promise, stated the way a user would say it:

> "I never lose the context of my work."

The primary action is **Continue**. **Resume** is its implementation.. Everything else — search, voice, prediction, knowledge accumulation — exists to eventually support that promise. Today, restoration is the entire product.

### 1.1 Non-Goals

Evo's design history includes several directions that were explored and deliberately abandoned. This section exists so they are not accidentally rebuilt.

- Evo is not an adaptive website engine, a browser agent, or a personalization SDK for third-party sites. An early version of the project explored this; it has no relationship to the current architecture.
- Evo is not a note-taking app, a task manager, a knowledge base, or a productivity dashboard.
- Evo is not a chatbot or conversational memory assistant. There is no "ask Evo anything" product surface in this architecture.
- Evo is not an autonomous agent platform. It does not take actions on the user's behalf beyond restoring their own prior context.
- Evo does not attempt to model human cognition, conceptual progress, emotional state, or "what mattered" in any sense beyond what is directly evidenced by observed artifacts. If a claim about the user's work cannot be justified by the artifact evidence itself, Evo does not make that claim.

If a proposed feature does not serve Work Restoration, it is out of scope until Work Restoration is exceptional and shipped.

---

## 2. Computational Model

Every durable system reduces to one sentence describing how it computes. Git's is that history is a graph of immutable snapshots. Kafka's is that state is derived by replaying a log. Evo's is:

> **State is never stored. State is derived, at any point in time, by replaying a confidence-scored interpretation function over an immutable observation log, scoped to a window of resources called a Workspace.**

The only fact that is permanently true is: *at time T, Evo observed X.* Everything else — which body of work X belongs to, how important X is — is the output of a function applied to that fact, not a fact itself. That function is expected to change many times over the system's life. When it does, the system does not migrate data. It replays.

This is the load-bearing idea behind the whole architecture. Interpretation is cheap, a pure function of its inputs, and disposable. If it is wrong, the function is improved and history is replayed. Individual interpretations are never patched by hand.

---

## 3. Canonical Data Model

Five named model objects. **Two of them are persisted as ground truth** — the Observation log and Artifact identity. Everything else, Workspace included, is a view: computed on demand, cached only for performance, and disposable.

**Amended (A1).** Superseded wording: *"Five primitives. Nothing else is persisted as ground truth."* The count is unchanged; what changed is that Workspace has moved from the persisted side of the line to the derived side, so the sentence no longer reads as though all five were stored. Knowledge and Decision remain persisted for the reasons each states, neither of which is restoration speed.

### Observation
An immutable, timestamped, append-only fact Evo directly witnessed — a window gaining focus, a file being saved, a URL being navigated to, a commit being made. Never edited. Never interpreted at the moment it is written. This is the log, and it is the only sacred table in the system.

### Artifact
A persistent-identity thing that exists independently of any single Observation — a file, a repository, a URL, a document, a person, a meeting. Observations reference Artifacts by stable ID. Identity resolution is lookup and deduplication, not inference.

### Workspace
The confidence-bearing derived representation of an ongoing body of work. A Workspace consists of a set of artifact Attachments (each with a confidence score and a role), a lifecycle state, and a history of Snapshots. Workspace is the primary object a user interacts with.

**Amended (A1).** Superseded wording: *"The single durable, confidence-bearing derived container… It is the one deliberate exception to 'everything derived is disposable': a Workspace is derived from Observations and Artifacts, but it is cached and persisted because restoration must be instant, not recomputed live."*

A Workspace is **not persisted and not an exception to §2.** It is projected from an Engagement (RFC-0014) on read, and nothing about it — not its identity, not its attachments, not its snapshots — is written as ground truth. The `workspace`, `attachment`, `snapshot`, and `restoration` logs are retired and are neither written nor read.

The word "container" is also withdrawn here, as RFC-0003 Requirement 3 always required: a Workspace does not own artifacts, it is an explanation of them.

### Engagement (derived, not a primitive)
The level of semantics that answers *what was going on* — which resources the evidence best explains as having been used together for one thing, and how much each matters to resuming it. Introduced by RFC-0014. Derived from the Observation log; never persisted.

**Amended (A1).** This level did not previously exist. Its absence is what forced the step from a resource to a Workspace to be justified by a per-resource property, which is the failure Amendment 1 corrects.

### Knowledge
Small, slow-changing, high-confidence facts that survive independently of any single Workspace's lifecycle. A fact belongs in Knowledge if and only if it would remain true and useful even if every current Workspace were deleted and rebuilt from scratch. Knowledge references Artifacts and behavioral patterns, never a Workspace, because a Workspace has a lifecycle and Knowledge by definition outlives any single container. Knowledge requires corroboration across multiple, independent occurrences before being written — a single observation never becomes Knowledge.

### Decision
A log of every action Evo takes — a restoration offered, a notification suppressed, a search result ranked. Exists purely for explainability. Every Decision entry can be traced back to the evidence that produced it.

### Derived Views (not primitives)
Moments, Tasks, intent, narrative summaries — none of these are persisted as first-class, migratable entities. They are computed from the canonical Observation log and Artifact identity, cached where performance requires it, and treated as fully disposable. If an interpretation algorithm improves, the correct action is to recompute the view, never to migrate a table that shouldn't exist.

**Amended (A1).** Superseded wording: *"computed from the five primitives above"* and *"beyond what is stored verbatim in a Snapshot"*. Both presupposed that Workspaces and Snapshots were themselves stored and could serve as an input basis. They are derived views on the same footing as the ones this paragraph describes.

---

## 4. Processing Pipeline

The complete path from a raw operating system event to a successful restoration. Each stage has exactly one responsibility and hands off a single, well-defined object to the next.

**Amended (A1).** The write path and the read path are now separated, because they have genuinely different shapes: capture is per-event and incremental, while a claim about a body of work is a claim about a whole history and cannot be made one event at a time. The superseded pipeline is quoted and explained after the new one.

```
── Write path (per event, no interpretation) ───────────────

Raw OS Event
    │
    ▼
[1] Capture
    Witness reality, write it down. No interpretation.
    ▼
Observation (immutable, appended to the log)
    │
    ▼
[2] Artifact Resolution
    "Have I seen this thing before?"
    ▼
Observation bound to a stable Artifact ID

── Read path (whole history, on demand) ────────────────────

Canonical Observation log
    │
    ▼
[3] Interpretation
    Observations → Acts, carrying the character of
    each witnessing.
    ▼
Acts + Declarations
    │
    ▼
[4] Engagement Derivation
    "Which resources were being used together, for
    something, by someone who was there — and how
    much does each matter to resuming?"
    Episodes → attention → affinity → grouping →
    significance → roles → naming.
    ▼
Engagements (with Participants and Roles)
    │
    ▼
[5] Workspace Projection
    Project each Engagement into restorable form:
    Attachments (confidence + role), lifecycle,
    one Snapshot per sitting.
    ▼
Workspace (derived, never written)
    │
    ▼
[6] Resume Request
    User opens Evo, selects a body of work, clicks Continue.
    ▼
[7] Restoration Derivation
    Select what must open from the roles already
    derived; produce an ordered plan.
    ▼
Restoration Plan
    │
    ▼
[8] Restoration Execution
    Open the selected artifacts, report success or
    failure per item, honestly.
    ▼
Restored work + Decision log entry
```

**Superseded stage [3], "Workspace Assignment"** — *"Which ongoing body of work does this belong to, and how confident am I?"*, run per Observation, producing an Attachment.

Why it prevented the product from working: the question is unanswerable at the moment a resource is first seen, because at that moment there is no relational evidence at all — and yet the stage had to answer it, because the next stage needed a Workspace to update. The only answerable question was "have I seen this before", so that became the rule, and a Workspace was created for every resource. Stage [4] now asks the relational question against the whole history instead, where it has the evidence to answer it.

**Superseded stage [5], "Snapshot"** — *"Freeze a restorable checkpoint, triggered by idle time, significant change, or explicit request."*

Why it changed: a trigger-fired Snapshot is a side effect of when the process happened to be running, which makes restoration state depend on process uptime rather than on what was witnessed — and it cannot be reproduced by replay, since the triggers are not in the log. Snapshots are now derived one per sitting, directly from the evidence, so replaying the log reconstructs exactly the same set.

**Unchanged:** no stage performs more than one job, and there is still no stage dedicated to narrative reconstruction, conceptual modeling, or reflective analysis. The claim that the body-of-work question "is answered entirely within stage 3, as part of deciding attachment" is withdrawn — that conflation is precisely the collapsed rung Amendment 1 exists to separate.

---

## 5. Engine Boundaries

Four engines. Each engine corresponds to one or more pipeline stages, owns a narrow responsibility, and communicates with adjacent engines only through the primitives defined above.

### Capture Engine
**Responsibility:** Observe activity and append Observations to the log.
**Invariant:** Every write is append-only; nothing is ever mutated or deleted except explicit user-requested purge. No interpretation happens here.
**Must not:** Decide importance, decide task membership, or perform any semantic processing inline. Capture must never be slowed or made fallible by intelligence — a bug in inference must never cause a dropped Observation.

### Artifact Engine
**Responsibility:** Construct and maintain Artifact Identity by deriving the current best identity hypothesis from canonical Observations.
**Invariant:** Artifact Identity is provisional, replayable, and always accountable to observational evidence. Under identical Observation history and identical derivation rules, the same Artifact Identity shall be reproduced. Improvements to identity derivation shall produce new identity hypotheses through replay rather than rewriting Observation history.
**Must not:** Determine Workspace membership, user intent, task membership, semantic meaning, or importance. Artifact Identity answers only whether multiple Observations most likely refer to the same external entity.

### Workspace Engine
**Responsibility:** Derive, from the canonical Observation history, which resources were being used together for one thing, how much each matters to resuming, and whether there is enough evidence to present the result as work at all. Project each result into restorable form.
**Invariant:** Derivation is a pure function of the Observation history and its parameters — no wall-clock time, no randomness, no arrival-order dependence, no state outside its inputs. Identical history yields identical Workspaces, roles, and names. Under-attachment is always preferred to over-attachment. Every conclusion retains the measurements it was derived from, so it can be explained without re-deriving it. Where a signal is unavailable the claim gets weaker; it is never fabricated.
**Must not:** Consult application identity, domain, file extension, profession, or any allowlist or blocklist of these. Originate a body of work from a property of a single resource considered alone. Introduce a numeric threshold without a stated evidential justification. Require type-specific code for a resource type it has not seen. Contradict or silently undo an explicit user declaration. Call a network service. Persist any interpretation.

**Amended (A1).** Three superseded rules, each quoted.

1. *"For each new Observation and resolved Artifact, decide whether it attaches to an existing Workspace, forms a new one, or attaches nowhere."* — Withdrawn. This is the per-observation framing; see §4's superseded stage [3].

2. *"Attachment decisions consider only a small, recent, local candidate set — never the full history."* — **Withdrawn, and this was the most consequential single sentence in the document.** A body of work is a claim about relationships across a history; restricting the evidence to a small recent window makes that claim unavailable *by construction*. What remains derivable from a local window is only "was this seen near something", which under-determines the answer so severely that the implementation fell back to per-resource recurrence. The rule was written to bound cost, which is a legitimate concern; it bounded meaning instead. Derivation now reads the history, and cost is bounded by measurement (`bench_reload`) rather than by discarding evidence.

3. *"Maintain each Workspace's cached state, Attachments, and Snapshots."* / *"Attachments are additive and superseded, never destructively edited."* — Withdrawn as stated, because there is no longer any stored attachment to supersede. The guarantee the rule protected — that history stays explainable and nothing is destructively rewritten — is now structural rather than procedural: the only thing written is the immutable Observation log, so no derived record can be destructively edited, because none exists.

*"Workspaces are never hard-merged; only linked by a relationship with a strength score."* — **Retained in force, satisfied differently.** Grouping does combine resources into one body of work during derivation, but nothing is destroyed by it: no stored Workspace is consumed, and the next derivation is free to reach a different conclusion from the same evidence. The rule existed to prevent irreversible loss, and deriving rather than storing makes irreversible loss impossible.

### Restoration Engine
**Responsibility:** Select, from a derived body of work, the minimum that re-establishes it; turn that selection into an ordered, executable plan; execute it; record what succeeded and what did not.
**Invariant:** Restoration is layered, progressive, and **selective**. Every attempted artifact reports success or failure explicitly; nothing fails silently. Restoration consumes the roles already derived — it never re-runs grouping or recomputes importance.
**Must not:** Perform new inference at restore time. Open every member of a body of work. Skip failed restorations without logging them. Block the user on every artifact opening before showing progress. Substitute something nearby for a resource that is not present.

**Amended (A1).** Superseded wording: *"Turn a Workspace's latest Snapshot into an ordered, executable plan."* Restoration now selects by role rather than replaying a frozen Snapshot wholesale, because a Snapshot records everything that was present in a sitting and opening all of it hands the person back the reload cost the product exists to remove. Selectivity is normatively required by RFC-0014 Requirement 10. The rule that restoration performs no new inference is retained and is in fact strengthened: roles are derived once, upstream, and restoration only reads them.

### Explicitly Not Engines
Search, Voice, Notifications, and Prediction are Experience-layer features — consumers of Workspace and Decision data through read interfaces, not independent reasoning systems. When built, they call into the Workspace Engine's existing outputs. They do not introduce new inference responsibility or new primitives.

---

## 6. Storage Philosophy

**Immutable, never rewritten:** the Observation log. Append-only, indexed by time and artifact, never touched by a rewriting migration.

**Persisted, technically derivable but cached for auditability:** Artifacts, Knowledge, Decisions. Each could in principle be rebuilt from the Observation log, but is persisted because recomputing live would violate the audit-trail guarantee.

**Amended (A1).** Superseded wording: *"Persisted, technically derivable but cached for restoration speed: Artifacts, Workspaces, Attachments, Snapshots, Knowledge, Decisions… persisted because recomputing live would violate the restoration-speed guarantee or the audit-trail guarantee."*

Workspaces, Attachments, and Snapshots are removed from this list. They were the only entries justified by *restoration speed* rather than auditability, and that justification is what made a Workspace durable from its first sighting — see Amendment 1. The `workspace`, `attachment`, `snapshot`, and `restoration` logs are retired: they are no longer written and no longer read, and `evo-daemon`'s `evo_doctor` reports each as retired so a stale file on an existing machine cannot be mistaken for live state.

Restoration speed is now met by derivation being fast enough, measured rather than assumed (4.2 ms per reload at 50 000 observations; see Amendment 1 for the measurements and the scaling limit). That is a weaker structural guarantee and a stronger correctness one, and the trade is deliberate.

**Derived on read, never stored:** Acts, Episodes, the Attention Ledger, affinity, Engagements, Workspaces, Attachments, Snapshots, roles, names. All pure functions of the Observation log.

**Cached, explicitly disposable and cheaply rebuilt:** embeddings, ranking scores, any home-screen ordering. If every cache table were dropped tonight, the system should be able to fully rebuild all of it by morning with zero data loss. This is the test: can it be silently dropped and regenerated without the user noticing anything beyond a brief delay?

**Replayed, never independently stored as a mutable object:** Moments, Tasks, intent, any narrative text.

**Never stored:** standalone confidence-state machinery independent of an Attachment. Any inference intermediate that exists only to justify another inference. Raw screen content or recordings — if visual signal is ever needed, it is processed into a lightweight derived signal at capture time and the source discarded.

The governing rule: the Observation log, Artifacts, and Knowledge must be sufficient to rebuild everything else. Anything that cannot be justified against that rule does not get a table. **Amendment 1 makes this rule true without exception for the first time.**

---

## 7. Replay Philosophy

Replay is how Evo improves without accumulating migration debt. It means re-deriving interpretation from historical Observations.

**Amended (A1).** Superseded wording, in full:

> "Replay… means re-running the Workspace Engine's attachment function over historical Observations, producing new Attachment records that supersede the old ones.
>
> 1. **Attachments are replayable.** …
> 2. **Snapshots are not replayable.** A Snapshot is a frozen historical fact — what restoration would have looked like, and what the system believed, at a specific point in time. Replaying an improved function never rewrites a past Snapshot. Only the next Snapshot benefits from a better function. This preserves history as an honest record rather than a rewritten one."

Why rule 2 prevented the product from working. It made a *derived* object into a historical fact. The consequence was that a person whose Snapshots were built under the old, wrong interpretation would keep being shown that wrong interpretation forever — the improvement reached only their future work, never the work they actually wanted to resume. It also split the system in two: live processing wrote Snapshots on triggers, replay could not reproduce them, so live and replay results could not be compared, and the divergence would be invisible. A system that cannot check its replay against its live path has no replay guarantee at all, only a claim of one.

The rule was protecting something real: **history must not be rewritten.** That concern was correctly identified and applied to the wrong object. What must never be rewritten is the *Observation log* — what Evo witnessed. What restoration *would have looked like* is not a witnessed fact; it is a conclusion, and conclusions are supposed to improve.

**The rules that replace them:**

1. **All interpretation is replayable.** Acts, Episodes, attention, affinity, Engagements, Workspaces, Attachments, Snapshots, roles, and names are all derived from the Observation log on read. An improved derivation improves *all* history immediately, with no migration and no supersession records.

2. **Live and replay are the same computation.** Not two implementations intended to agree — the same function over the same log. Every read-back of state is a full re-derivation, which makes divergence structurally impossible rather than merely unlikely. `evo-daemon`'s `evo_doctor` performs this check on real history and reports the result.

3. **The Observation log is the only thing that is never rewritten.** This is where the honesty guarantee actually lives, and it is absolute.

4. **Derivation is versioned.** Where a derivation rule or parameter set changes, the change is versioned so that a historical result remains reproducible. Honesty about history is preserved by being able to reproduce what Evo concluded and when, not by freezing a conclusion in place.

Improvement is replay. It is never migration.

---

## 8. Learning Philosophy

Learning is scoped to exactly one place: the function that maps Observations and Artifacts to Attachments, inside the Workspace Engine. Nothing else in the system is permitted to "learn" in a way that requires new tables or new architectural layers.

What remains fixed forever: the five named model objects, the append-only nature of Observations, the rule that confidence is always a number attached to a derived Attachment rather than baked into stored truth, and the rule that models verbalize but never decide identity.

What is free to change constantly, without architectural consequence: the specific heuristics or models used inside the Workspace Engine's derivation, ranking formulas, and any model used for embeddings. Because the engine boundary is a stable interface — evidence in, confidence-scored and role-carrying attachment out — any implementation behind it, from a hand-written heuristic to a future on-device model, is interchangeable. This is what "AI model independent" means structurally rather than as a stated preference.

**Amended (A1).** Two clarifications, neither reversing a rule. First, "the function that maps Observations and Artifacts to Attachments" now means the whole derivation — interpretation, segmentation, attention, affinity, grouping, significance, roles, naming — because those are the stages the mapping actually consists of. Second, freedom to change is now genuinely free: with nothing derived being persisted, changing any stage requires no migration and leaves no stale record behind. Under the superseded persisted-Workspace rules, a change to the attachment function had to be reconciled against rows created by the old one, which is why §8's promise of costless improvement was not previously deliverable.

A correction from the user is evidence, and it outranks inference (RFC-0014 Requirement 11). It does not require its own subsystem.

---

## 9. Restoration Philosophy

Restoration is the heart of the product, and it has a precise meaning.

**What is restored:** the minimum that re-establishes a body of work — not "files," not "windows," and deliberately not everything that was present. The rest stays reachable as context without being opened.

**Amended (A1).** Superseded wording: *"a Workspace's most recent Snapshot, executed as a layered plan… a frozen, pre-organized projection of where the user left off."* A Snapshot records everything present in a sitting, so executing one wholesale opens everything, which is the defect selective restoration exists to prevent (RFC-0014 Requirement 10).

**Order:** Layer 1, context — shown instantly. Layer 2, the continuation point and the primary artifacts, opened first; there may be more than one, bounded by a configured limit. Layer 3, supporting artifacts — in Evo's implementation the declared Continuation Surface (RFC-0013), the user's explicit restore set, which is deliberately not capped: capping it would silently decline to reopen resources the user explicitly declared as their continuation (RFC-0013; Law IX, The User Owns Judgment). Layer 4, reference artifacts, opened last or on demand. Layer 5, historical context, never opened automatically.

**Amended (A1).** Superseded wording: *"Layer 2, the single primary artifact, opened first."* Real work frequently has more than one thing you must have in front of you to continue — a document and the terminal running against it, a spreadsheet and the page it is drawn from. Forcing exactly one meant either opening too little to resume or promoting an arbitrary member. The number is now a configured bound rather than a hardcoded one, stated as a product judgement rather than presented as a measurement, and the layer is still the *smallest* set that re-establishes the work.

**Success:** measured mechanically and honestly. Evo can know, deterministically, whether each planned artifact opened successfully. Whether restoration truly helped the user resume productive work is a harder question the architecture does not claim to answer with certainty — it is something to learn from real usage, not something to architect false confidence around.

**Partial restoration:** a normal, expected outcome, never an error state. A failed artifact is logged and shown plainly; restoration continues with everything else. A single missing file must never block the rest of the plan.

**The one product metric:** Time To Productive — wall-clock time from the Resume click to the primary artifact being open and in the foreground. Every architectural decision in this document exists, ultimately, to make that number smaller.

**Architectural guarantees this depends on:** restoration never triggers new inference — the roles it selects by were derived upstream, and it only reads them. Partial failure is handled gracefully as a first-class outcome, never an exception path.

**Amended (A1).** Superseded wording: *"the Snapshot restoration reads from must be a cheap, pre-computed read — restoration never triggers live inference."* The second half is retained in full; the first half named a stored Snapshot as the mechanism. Derivation happens when Home is loaded, not when Continue is clicked, so restoration itself remains a read of already-derived state.

---

## 10. Privacy and Local-First Constraints

These are non-negotiable and apply to every layer described above:

- All processing described in this document happens on-device. No Observation, Artifact, Workspace, Snapshot, or Knowledge record requires a network call to be produced, interpreted, or restored.
- The Observation log is the most sensitive data in the system by construction — it is a record of everything the user did. It must never be transmitted, synced to a third party, or used to train anything beyond the user's own local Workspace Engine, without explicit, specific consent.
- Nothing in this architecture assumes cloud dependency. Future capabilities involving synchronization across a user's own devices are compatible with this model because the Observation log is append-only and conflict-resistant by construction, but synchronization is a feature to be added deliberately, not an assumption baked into the primitives.
- Raw high-fidelity capture (such as visual content) is never retained as ground truth. If a future signal type requires it, only a derived, lightweight representation is kept, and the source is discarded at capture time.

---

## 11. Glossary

**Observation** — an immutable fact Evo witnessed. The only permanent ground truth.

**Artifact** — a persistent-identity thing referenced by Observations (file, URL, repository, person, document).

**Act** — one witnessed interaction with one resource at one moment, carrying the character of the witnessing. Derived. *(Added by A1.)*

**Episode / sitting** — a maximal run of Acts with no gap longer than the presence horizon; the unit within which Evo may claim the person was continuously present. Derived. *(Added by A1.)*

**Engagement** — what was going on: the resources the evidence best explains as having been used together for one thing, with each one's role. Derived. *(Added by A1.)*

**Role** — a participant's importance *to resuming*, distinct from its strength of membership. One of Continuation, Primary, Supporting, Reference, Context. *(Added by A1.)*

**Attachment** — a scored, role-carrying link between an Artifact and a Workspace. Derived on read. *(Amended by A1: previously "superseded-not-edited", which presupposed a stored record.)*

**Workspace** — a derived, restorable representation of an ongoing body of work. The primary object a user interacts with. *(Amended by A1: previously "a persistent, restorable container".)*

**Snapshot** — a restoration-ready projection of a body of work for one sitting, derived from the evidence of that sitting. Contains only structurally evidenced content; never an inferred narrative claim. *(Amended by A1: previously "an immutable… projection of a Workspace at a point in time", frozen on a trigger.)*

**Knowledge** — a small set of durable facts that outlive any single Workspace's lifecycle, requiring corroboration across multiple occurrences.

**Decision** — a logged record of any action Evo took, kept for explainability.

**Moment / Task** — derived views computed on demand. Never persisted as independent, migratable entities.

**Time To Productive (TTP)** — the primary product metric: wall-clock time from clicking Continue to the primary artifacts being open and in the foreground.

---

## 12. Summary of Architectural Constraints

For quick reference, the constraints that govern every future RFC:

1. Local-first. No architectural component requires a network call to function.
2. The Observation log is immutable and append-only, forever.
3. Everything except the Observation log, Artifact identity, Knowledge, and Decisions is a derived view, computed and disposable. *(Amended (A1); previously "everything except the five primitives", which included Workspace.)*
4. Confidence is a number attached to a derived Attachment. It is never baked into stored truth.
5. No hard merges of stored state, anywhere. Grouping during derivation destroys nothing, because nothing derived is stored. *(Amended (A1); the guarantee is unchanged, the mechanism is structural rather than procedural.)*
6. Under-attachment is always preferred to over-attachment.
7. Models verbalize. They never decide identity or attachment.
8. Improvement is replay, not migration.
9. **All interpretation is replayable, including Snapshots. Live processing and replay are the same computation over the same log.** *(Amended (A1); superseded: "Snapshots are historical facts. They are never rewritten by replay." See §7 for why the old rule was protecting the right thing about the wrong object.)*
10. **A body of work is a claim about relationships among resources. No property of a single resource considered alone may originate one.** *(Added by A1; normative statement in RFC-0014 Requirement 1.)*
11. **No stage may consult application identity, domain, file extension, or profession, or maintain any allowlist or blocklist of these.** *(Added by A1; RFC-0014 Requirement 3.)*
12. **Every numeric parameter must state the evidential limit it expresses, be configurable, and be varied by tests.** *(Added by A1; RFC-0014 Requirement 5.)*
13. **Restoration opens the minimum that re-establishes the work and preserves access to the rest.** *(Added by A1; RFC-0014 Requirement 10.)*
14. **An explicit user declaration outranks every measurement, and inference must never require one to operate.** *(Added by A1; RFC-0014 Requirement 11.)*
15. **Where a signal is unavailable, the claim gets weaker. It is never fabricated.** *(Added by A1; RFC-0014 Requirement 13.)*
16. Every feature is evaluated against one question: does it reduce the effort required to continue meaningful work? If not, it is out of scope until restoration is exceptional.
