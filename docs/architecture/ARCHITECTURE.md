# Evo — Architecture

**Status:** Frozen
**Audience:** Engineers joining Evo, before reading any RFC
**Purpose:** Understanding, not implementation

This document explains what Evo is, why it is shaped the way it is, and which decisions are permanent versus which are expected to change. Read this before you read any RFC. If an RFC contradicts this document, the RFC is wrong.

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

Five primitives. Nothing else is persisted as ground truth. Everything else is a view, computed on demand and cached only for performance.

### Observation
An immutable, timestamped, append-only fact Evo directly witnessed — a window gaining focus, a file being saved, a URL being navigated to, a commit being made. Never edited. Never interpreted at the moment it is written. This is the log, and it is the only sacred table in the system.

### Artifact
A persistent-identity thing that exists independently of any single Observation — a file, a repository, a URL, a document, a person, a meeting. Observations reference Artifacts by stable ID. Identity resolution is lookup and deduplication, not inference.

### Workspace
The single durable, confidence-bearing derived container representing an ongoing body of work. A Workspace consists of an evolving set of artifact Attachments (each with a confidence score), a lifecycle state, and a history of Snapshots. It is the one deliberate exception to "everything derived is disposable": a Workspace is derived from Observations and Artifacts, but it is cached and persisted because restoration must be instant, not recomputed live. Workspace is the primary object a user interacts with.

### Knowledge
Small, slow-changing, high-confidence facts that survive independently of any single Workspace's lifecycle. A fact belongs in Knowledge if and only if it would remain true and useful even if every current Workspace were deleted and rebuilt from scratch. Knowledge references Artifacts and behavioral patterns, never a Workspace, because a Workspace has a lifecycle and Knowledge by definition outlives any single container. Knowledge requires corroboration across multiple, independent occurrences before being written — a single observation never becomes Knowledge.

### Decision
A log of every action Evo takes — a restoration offered, a notification suppressed, a search result ranked. Exists purely for explainability. Every Decision entry can be traced back to the evidence that produced it.

### Derived Views (not primitives)
Moments, Tasks, intent, narrative summaries beyond what is stored verbatim in a Snapshot — none of these are persisted as first-class, migratable entities. They are computed from the five primitives above, cached where performance requires it, and treated as fully disposable. If an interpretation algorithm improves, the correct action is to recompute the view, never to migrate a table that shouldn't exist.

---

## 4. Processing Pipeline

The complete path from a raw operating system event to a successful restoration, in eight stages. Each stage has exactly one responsibility and hands off a single, well-defined object to the next.

```
Raw OS Event
    │
    ▼
[1] Capture
    Witness reality, write it down. No interpretation.
    ▼
Observation (immutable)
    │
    ▼
[2] Artifact Resolution
    "Have I seen this thing before?"
    ▼
Observation bound to a stable Artifact ID
    │
    ▼
[3] Workspace Assignment
    "Which ongoing body of work does this belong to,
    and how confident am I?"
    ▼
Attachment (artifact ↔ workspace, with confidence)
    │
    ▼
[4] Workspace State Update
    Keep the Workspace's cached, restorable
    representation current.
    ▼
Updated Workspace
    │
    ▼
[5] Snapshot
    Freeze a restorable checkpoint, triggered by
    idle time, significant change, or explicit request.
    ▼
Snapshot (immutable once written)
    │
    ▼
    ... time passes ...
    │
    ▼
[6] Resume Request
    User opens Evo, selects a Workspace, clicks Restore.
    ▼
[7] Restoration Planning
    Turn the latest Snapshot into an ordered plan.
    ▼
Restoration Plan
    │
    ▼
[8] Restoration Execution
    Open artifacts in order, report success or
    failure per item.
    ▼
Restored Workspace + Decision log entry
```

No stage performs more than one job. There is no separate "Understanding" stage producing Moments or intent as an independent pipeline step — that question is answered entirely within stage 3, as part of deciding attachment. There is no stage dedicated to narrative reconstruction, conceptual modeling, or reflective analysis — these were explored during design and deliberately excluded; they do not serve restoration and are not part of this pipeline.

---

## 5. Engine Boundaries

Four engines. Each engine corresponds to one or more pipeline stages, owns a narrow responsibility, and communicates with adjacent engines only through the primitives defined above.

### Capture Engine
**Responsibility:** Observe activity and append Observations to the log.
**Invariant:** Every write is append-only; nothing is ever mutated or deleted except explicit user-requested purge. No interpretation happens here.
**Must not:** Decide importance, decide task membership, or perform any semantic processing inline. Capture must never be slowed or made fallible by intelligence — a bug in inference must never cause a dropped Observation.

### Artifact Engine
**Responsibility:** Identity resolution — given a raw reference, return a stable Artifact ID.
**Invariant:** Idempotent; the same real-world thing always resolves to the same ID.
**Must not:** Score importance, assign artifacts to Workspaces, or merge artifacts based on semantic similarity alone. Identity is resolved by strong signals — same path, same URL, same repository — not by embedding proximity.

### Workspace Engine
**Responsibility:** For each new Observation and resolved Artifact, decide whether it attaches to an existing Workspace, forms a new one, or attaches nowhere. Maintain each Workspace's cached state, Attachments, and Snapshots.
**Invariant:** Attachments are additive and superseded, never destructively edited — confidence changes produce a new record, with the old one marked superseded, so history remains explainable without a separate provenance subsystem. Workspaces are never hard-merged; only linked by a relationship with a strength score. Under-attachment is always preferred to over-attachment. Attachment decisions consider only a small, recent, local candidate set — never the full history.
**Must not:** Call a model to decide identity or attachment. Maintain competing scoring subsystems beyond a single confidence value per Attachment. Persist any interpretation beyond what is needed to answer "what belongs here, and how important is it."

### Restoration Engine
**Responsibility:** Turn a Workspace's latest Snapshot into an ordered, executable plan; execute it; record what succeeded and what did not.
**Invariant:** Restoration is layered and progressive. Every attempted artifact reports success or failure explicitly; nothing fails silently. Restoration executes a plan — it never re-runs attachment inference.
**Must not:** Perform new inference at restore time. Skip failed restorations without logging them. Block the user on every artifact opening before showing progress.

### Explicitly Not Engines
Search, Voice, Notifications, and Prediction are Experience-layer features — consumers of Workspace and Decision data through read interfaces, not independent reasoning systems. When built, they call into the Workspace Engine's existing outputs. They do not introduce new inference responsibility or new primitives.

---

## 6. Storage Philosophy

**Immutable, never rewritten:** the Observation log. Append-only, indexed by time and artifact, never touched by a rewriting migration.

**Persisted, technically derivable but cached for restoration speed:** Artifacts, Workspaces, Attachments, Snapshots, Knowledge, Decisions. Each of these could in principle be rebuilt from the Observation log, but is persisted because recomputing live would violate the restoration-speed guarantee or the audit-trail guarantee.

**Cached, explicitly disposable and cheaply rebuilt:** embeddings, ranking scores, any home-screen ordering. If every cache table were dropped tonight, the system should be able to fully rebuild all of it by morning with zero data loss. This is the test: can it be silently dropped and regenerated without the user noticing anything beyond a brief delay?

**Replayed, never independently stored as a mutable object:** Moments, Tasks, intent, any narrative text beyond what is stored verbatim in a Snapshot. Computed on demand, cached only as an optimization, never migrated.

**Never stored:** standalone confidence-state machinery independent of an Attachment. Any inference intermediate that exists only to justify another inference. Raw screen content or recordings — if visual signal is ever needed, it is processed into a lightweight derived signal at capture time and the source discarded.

The governing rule: the Observation log, Artifacts, and Knowledge must be sufficient to rebuild everything else. Anything that cannot be justified against that rule does not get a table.

---

## 7. Replay Philosophy

Replay is how Evo improves without accumulating migration debt. It means re-running the Workspace Engine's attachment function over historical Observations, producing new Attachment records that supersede the old ones.

Two rules govern replay precisely:

1. **Attachments are replayable.** When the attachment function improves, replay regenerates Attachments and, going forward, better Workspaces — without touching the Observation log, which never changes.
2. **Snapshots are not replayable.** A Snapshot is a frozen historical fact — what restoration would have looked like, and what the system believed, at a specific point in time. Replaying an improved function never rewrites a past Snapshot. Only the next Snapshot benefits from a better function. This preserves history as an honest record rather than a rewritten one.

Improvement is replay. It is never migration.

---

## 8. Learning Philosophy

Learning is scoped to exactly one place: the function that maps Observations and Artifacts to Attachments, inside the Workspace Engine. Nothing else in the system is permitted to "learn" in a way that requires new tables or new architectural layers.

What remains fixed forever: the five primitives, the append-only nature of Observations, the rule that confidence is always a number attached to a derived Attachment rather than baked into stored truth, and the rule that models verbalize but never decide identity.

What is free to change constantly, without architectural consequence: the specific heuristics or models used inside the Workspace Engine's attachment function, ranking formulas, and any model used for embeddings. Because the engine boundary is a stable interface — evidence in, confidence-scored attachment out — any implementation behind it, from a hand-written heuristic to a future on-device model, is interchangeable. This is what "AI model independent" means structurally rather than as a stated preference.

A correction from the user is evidence. It updates confidence on an Attachment like any other signal. It does not require its own subsystem.

---

## 9. Restoration Philosophy

Restoration is the heart of the product, and it has a precise meaning.

**What is restored:** a Workspace's most recent Snapshot, executed as a layered plan — not "files," not "windows," a frozen, pre-organized projection of where the user left off.

**Order:** Layer 1, context — the Snapshot's summary, shown instantly. Layer 2, the single primary artifact, opened first. Layer 3, supporting artifacts, capped to a small number. Layer 4, reference artifacts, opened last or on demand. Layer 5, historical context, never opened automatically.

**Success:** measured mechanically and honestly. Evo can know, deterministically, whether each planned artifact opened successfully. Whether restoration truly helped the user resume productive work is a harder question the architecture does not claim to answer with certainty — it is something to learn from real usage, not something to architect false confidence around.

**Partial restoration:** a normal, expected outcome, never an error state. A failed artifact is logged and shown plainly; restoration continues with everything else. A single missing file must never block the rest of the plan.

**The one product metric:** Time To Productive — wall-clock time from the Resume click to the primary artifact being open and in the foreground. Every architectural decision in this document exists, ultimately, to make that number smaller.

**Architectural guarantees this depends on:** the Snapshot restoration reads from must be a cheap, pre-computed read — restoration never triggers live inference. Partial failure is handled gracefully as a first-class outcome, never an exception path.

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

**Attachment** — a scored, superseded-not-edited link between an Artifact and a Workspace, produced by the Workspace Engine.

**Workspace** — a persistent, restorable container for an ongoing body of work. The primary object a user interacts with.

**Snapshot** — an immutable, pre-organized, restoration-ready projection of a Workspace at a point in time. Contains only structurally evidenced content; never an inferred narrative claim.

**Knowledge** — a small set of durable facts that outlive any single Workspace's lifecycle, requiring corroboration across multiple occurrences.

**Decision** — a logged record of any action Evo took, kept for explainability.

**Moment / Task** — derived views computed on demand from the five primitives. Never persisted as independent, migratable entities.

**Time To Productive (TTP)** — the primary product metric: wall-clock time from clicking Resume to the primary artifact being open and in the foreground.

---

## 12. Summary of Architectural Constraints

For quick reference, the constraints that govern every future RFC:

1. Local-first. No architectural component requires a network call to function.
2. The Observation log is immutable and append-only, forever.
3. Everything except the five primitives is a derived view, computed and disposable.
4. Confidence is a number attached to a derived Attachment. It is never baked into stored truth.
5. No hard merges, anywhere. Supersession and relationship links only.
6. Under-attachment is always preferred to over-attachment.
7. Models verbalize. They never decide identity or attachment.
8. Improvement is replay, not migration.
9. Snapshots are historical facts. They are never rewritten by replay.
10. Every feature is evaluated against one question: does it reduce the effort required to continue meaningful work? If not, it is out of scope until restoration is exceptional.
