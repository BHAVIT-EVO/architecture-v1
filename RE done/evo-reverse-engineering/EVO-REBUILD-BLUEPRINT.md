# Evo Backend Rebuild Blueprint

**What Evo captures, how it decides what work is, how it groups, names, orders and reopens work — and the exact mechanisms to replace.**

Generated 2026-10-15 · machine: `bhavitsaini` (macOS/arm64) · produced by independent reverse engineering, not from Evo's own documentation.

---

## How to read the evidence markers

Every claim below is tagged with where it came from. Nothing is asserted from memory.

| Tag | Meaning |
|---|---|
| **[M]** | **Measured by me** on this machine, from a real artifact. The artifact path is given. |
| **[V]** | **Verbatim source code** read on this machine. File + symbol given. |
| **[D]** | **Official vendor documentation**, with URL. |
| **[B]** | From your own brief. Used only where it is a statement of intent or a product description, never as a technical fact. |
| **[?]** | Open question / not verified. Listed so it does not get mistaken for a finding. |

**REA CLI 4.0.1** (`rea`, Homebrew) was the primary extraction tool for shipped binaries. §10.0 lists every operation run, its `evidence_id` and subject `sha256`, plus the three cases where REA **refused or failed** — those are reported as limitations, not hidden. Where REA could not read a target, I read the shipped artifact directly and said so at the point of each claim.

**Artifacts used**

| Artifact | Path |
|---|---|
| Evo source (Rust workspace) | `~/Desktop/Evo-Evolution.nosync` |
| Evo live data (canonical log) | `~/Library/Application Support/evo/storage/observation.log` |
| Dayflow source (MIT, local) | `~/Downloads/Dayflow-main` |
| REA evidence (my extraction) | `~/Desktop/evo-reverse-engineering/evidence/` |
| This document | `~/Desktop/evo-reverse-engineering/EVO-REBUILD-BLUEPRINT.md` |

**Measured at:** `observation.log` = **42,603,369 bytes / 883,062 lines / 52,679 records**, read at 19:40 on 2026-10-15. **The log is live** — Evo's daemon appends to it continuously, so the counts below drifted by a few records between two reads (e.g. `OBS-FILE-SAVED` 22,633 → 22,651 in ~20 minutes). Treat every count as a measurement with a timestamp, not a constant. Also present in the same directory: `artifact.log` (2.4 MB, derived) and `ledger.db` (11.2 MB, derived).

The log is a **custom line-oriented format**, not JSON: each record begins with a byte length, then `observation-v1`, then fields whose textual values are **hex-encoded** — e.g. `schema_name_hex=4f42532d57494e444f572d464f4355532d4741494e4544` = `OBS-WINDOW-FOCUS-GAINED`. A naive `grep 'OBS-'` over this file returns **zero matches**. Decoding is required (§Appendix B), and any claim about Evo's capture inventory must come from a decode pass.

---

## 0. Executive summary

Six findings. Each one is the mechanical cause of a complaint you described.

1. **Evo's capture layer records mostly machine churn, not human attention.** Of **52,679** recorded observations, **62.0% come from file-system events** (`macos_fsevents`: `FileSaved` 22,651 + `RepositoryMembership` 9,988). Accessibility events are 35.0%, raw input 3.0%, and the only *semantic* milestone Evo records — a git commit — appears **27 times** (0.05%). Explicit user intent appears **7 times total** (`user_grouping` 4, `user_designation` 3). **[M]**

2. **Nothing in Evo's evidence model can say "this was not work".** There is no observation type for media playback, audio, calls, calendar, or idle/lock, and no negative signal anywhere in the schema. A leisure YouTube video is recorded with the *same* event types as a work session. **[M]**

3. **Evo is deliberately capability-blind, and that is the root cause of the grouping failure.** Every one of the 14 inference thresholds is a pure time/evidence quantity, and the parameter module states its own rule: *"Nothing here names an application, a domain, a file type, a directory, or a category of work."* **[V]** Grouping is therefore "things that were on screen together in time" — which is not the same question as "apps that serve one piece of work". No amount of tuning fixes this; it needs a different evidence axis.

4. **Evo has no concept of completion, an objective, or a next action.** Nothing in the capture schema or the parameters represents *done*, *outstanding*, *blocked* or *next*. So when you ask "of the 6 apps in this work, which 3 are no longer needed?", Evo does not have the vocabulary to answer — it can only re-run a statistical share test over attention. **[M][V]**

5. **A work is named by the `display_name` of one of its resources, and ordered by a single boolean.** Naming is `choose_title()`, whose last resort is literally `participant.resource().display_name()` **[V]** — for a web surface that string is the page title or URL, which is exactly why a hiking video becomes `youtube.com/watch?v=…`. Ordering is `order_home_workspaces()`, a stable partition on *"does this work have a resume point"* and nothing else — its own doc comment says *"No score, no recency, no frequency, no application identity, no guessed ranking."* **[V]**

6. **The resume point that ordering depends on is user-designated, and it has been designated 3 times in the entire log.** **[M]** So the partition degenerates to "one group", and the Home list falls back to raw canonical order. That is the mechanical reason the app "displays work without order" and has no sense of urgency.

**The shortest statement of the whole problem:** Evo currently answers *"which windows were on screen at the same times?"* and presents the answer as *"what were you working on?"* Those are different questions. Every complaint in your brief is a symptom of that substitution.

### 0.1 What the REA pass added (§10)

Seven findings from the reverse engineering that change or sharpen the diagnosis:

- **ManicTime's macOS client is .NET**, so the autotag engine was analysed directly with REA's `rea-dotnet-static` rather than inferred from docs. The file/meeting you suggested downloading was unnecessary. Confirmed in the binary: `AutoTagTimelineAbsorb` and `AutoTagTimelineAppend` as **two separate application modes**, `AutoFillHelper` with a stored `AutoFillInSeconds`, rules on their **own timeline** with explicit comparers, a per-autotag **whitelist**, an **occupancy** model over intervals, schema-versioned autotag timelines, an **export format** for rule sets, and a bundled grammar parser (`Irony.dll`).
- **Bunch's activation semantics are now attested by REA**, not just strings: **140 ObjC classes / 985 dispatch implementations**, including per-item `_delay`, `_waitingSnippet`, `_openAt` under a dedicated `BTBunchScheduler`, `_forceOpenClose`, `_closeOn`, `_ignoreInSingleBunchMode`, and — most usefully — **`newBunchWithOpenApps` / `copyBunchWithOpenApps`**, i.e. *capture the state you are in as the plan to return to*.
- **Session Buddy's snapshot schema is extracted from its shipped bundle**: `sessions`→`windows`→`folders`/`tabs`/`links`, a versioned `$.format` export envelope, and **separate id-maps for `saved` vs `previous` sessions**. That last split is the exact distinction Evo lacks (§7.2), and §11.6 now specifies `SessionSnapshot` against a real schema instead of a proposed one.
- **Granola's capture breadth is the finding, not its naming.** Its process tree dedicates a process each to **Accessibility**, **EventKit calendar**, **audio + local ASR + diarisation**, Mission Control, TCC permissions and SQLite, and its store carries `eventKitEvents`, `people`, `peopleLastFetchedAt`, `meetingsMetadata`. Evo has none of the calendar, audio or permission evidence (§2.2). Its *title generation is server-side*, which I report as an inference from a bounded negative search, not as a mechanism.
- **Rize's rule object and Toggl's idle actions are concrete**: Rize's `trackingRule` carries `category` + `alwaysBlock` and distinguishes breaks / meetings / focus sessions (and it bundles OCR); Toggl has real IPC channels `idle-detection:dismiss` and `idle-detection:action-taken` plus a user-toggleable autotracker. Those are two off-the-shelf answers to Evo's missing "not work" signal and its 7-events-in-52,679 feedback problem.
- **REA refused three targets, which is itself evidence.** All three Electron `app.asar`s (Granola, Toggl, Rambox) failed closed with `artifact_integrity_mismatch` on `asarUnpack`ed native modules — so **`app.asar` alone is not a verifiable artifact for these apps** — and Dia's dispatch read was refused because its Mach-O exceeds 64 MiB.
- **The field is weaker than it looks, and Evo should be held to that standard.** No reference app claims to infer unaided which of 20 open apps belong to one piece of work. They ask, they snapshot, or they apply your rules. Evo's ambition to infer it is legitimate, but it means Evo carries an accuracy obligation none of them carry — which is why §13's metrics and §11.8's feedback loop are not optional extras.

---

## 1. What Evo is and the shape of its backend

Evo is a Rust workspace (crates under `crates/`, e.g. `evo-daemon`, `evo-desktop`, `evo-engagement`, `evo-macos-accessibility`) with a daemon that observes the machine and a desktop shell that presents derived work. **[M]**

The pipeline, in order:

```
  ┌─────────────┐   ┌──────────────┐   ┌─────────────┐   ┌────────────┐   ┌────────────┐
  │ 1 CAPTURE   │──▶│ 2 CANONICAL  │──▶│ 3 INFERENCE │──▶│ 4 DECIDE   │──▶│ 5 PRESENT  │
  │ OS signals  │   │ append-only  │   │ affinity    │   │ Standing:  │   │ title,     │
  │ → OBS-*     │   │ observation  │   │ graph + 14  │   │ Work /     │   │ order,     │
  │   records   │   │ log (truth)  │   │ thresholds  │   │ Remembered │   │ restore    │
  └─────────────┘   └──────────────┘   └─────────────┘   └────────────┘   └────────────┘
```

Two design decisions in this pipeline are **right** and should be preserved:

- **The observation log is the truth; everything else is derived.** `CanonicalIndex` rebuilds all state from the log, and a long-lived index is refreshed incrementally. **[V]** This means *any* change to inference is testable by replaying the real log — the single most valuable property Evo has, and the foundation of the validation harness in §13.
- **Nothing is ever omitted for being unimportant.** A group that fails the bar for "work" is kept as `Remembered` rather than deleted. **[V]** Keep this. Fixing a bad list by deleting evidence would be the wrong repair.

The failures are all in stages 3–5, and stage 1 does not supply the signals stage 3–5 would need.

---

## 2. Layer 1 — Capture: what actually gets recorded

### 2.1 Measured signal inventory **[M]**

Source: `~/Library/Application Support/evo/storage/observation.log` — 42,603,369 bytes / 883,062 lines / **52,679 observation records** (~16.8 lines per record), read 19:40 2026-10-15. Values are hex-encoded, so this table comes from a decode pass (§Appendix B). Counted per `OBS-*` schema — **this is the complete enumeration: exactly nine schemas exist**.

| Observation type | Count | Share | Emitted by | What it means |
|---|---:|---:|---|---|
| `OBS-FILE-SAVED` | 22,651 | 43.0% | `macos_fsevents` | a file changed (`FileSaved`) |
| `OBS-WINDOW-FOCUS-GAINED` | 12,576 | 23.9% | `macos_accessibility` | a window became frontmost |
| `OBS-REPOSITORY-MEMBERSHIP` | 9,988 | 19.0% | `macos_fsevents` | a file belongs to a repo (carries a `Repository` fact) |
| `OBS-URL-NAVIGATED` | 5,261 | 10.0% | browser integration | a page loaded |
| `OBS-INPUT-ACTIVITY` | 1,583 | 3.0% | `macos_accessibility` | one record bundling `Keys` + `Clicks` + `Scrolls` |
| `OBS-SURFACE-NAMED` | 586 | 1.1% | `macos_accessibility` | a window's `Name` |
| `OBS-COMMIT-MADE` | **27** | 0.05% | (integration) | a human decided work was worth recording |
| `OBS-WORK-GROUPED` | **4** | 0.008% | `user_grouping` | the user grouped something (`CoMember` facts) |
| `OBS-WORK-DESIGNATED` | **3** | 0.006% | `user_designation` | the user said "this is what I'm doing" |

By emitter (`source_hex`), which is the clearer statement of who produced the evidence:

| Emitter | Count | Share |
|---|---:|---:|
| `macos_fsevents` | 32,666 | **62.0%** |
| `macos_accessibility` | 18,423 | 35.0% |
| `macos_input_counter` | 1,087 | 2.1% |
| `macos_input` | 496 | 0.9% |
| `user_grouping` | 4 | 0.008% |
| `user_designation` | 3 | 0.006% |

### 2.2 The negative space — what Evo cannot see **[M]**

These are absent from the entire log — verified by **exhaustive enumeration**: only the nine schemas in §2.1 exist, and none of them carries any of the following. Their absence, not their presence, is the important finding:

| Missing signal | Why it matters |
|---|---|
| **Media playback / audio state** (playing, paused, which media) | You cannot separate a video you watched from a document you worked in. |
| **Idle / screen-lock / sleep** | Evo infers "presence" from gaps and thresholds instead of knowing. |
| **Calendar events / meeting attendance** | Rize uses calendar titles + attendees as primary naming evidence **[D]**; Evo has none. |
| **Task-system state** (Linear/Jira/Todoist/Notes) | The only representation of an *objective* and *done*. |
| **Notification / communication context** | A chat that flashed by looks like a chat you worked in. |
| **Intent / negative feedback** | There is no way for the user or the system to record "that was not work". |

### 2.3 What this layer causes

- **Volume is dominated by things that are not human work.** 62.0% of all observations come from `macos_fsevents` (file-system bookkeeping). Meanwhile the only signal that a human deliberately concluded something — a commit — is 0.05% of the log: **one commit per 839 file saves**.
- **The engagement layer must therefore infer human intent from co-occurrence in time**, because that is the only thing the capture layer reliably supplies. This is the structural reason the inference layer is capability-blind: it was built to a signal set that carries no capability.
- **A leisure video is captured identically to real work**: `WINDOW-FOCUS-GAINED` + `URL-NAVIGATED` (+ a little `INPUT-ACTIVITY`). Nothing distinguishes them. This is not a bug in interpretation — the discriminating information was never captured.

---

## 3. Layer 2 — Interpretation: the affinity graph and its 14 thresholds

### 3.1 The parameters, verbatim **[V]**

Read from `crates/evo-engagement/src/params.rs`. This is the complete set of knobs that decide what work is.

| Parameter | Default | Stated reason (abridged) |
|---|---|---|
| `presence_horizon` | 20 min | Beyond this there is no evidence of presence; crossing it starts a new Episode. |
| `max_interval_attention` | 10 min | A window can hold focus while you walk away; wall-clock ≠ attention. All-or-nothing. |
| `min_co_episodes` | 2 | One shared sitting is a coincidence, two is a pattern. |
| `affinity_floor` | 0.20 | Edges below this are dropped rather than clustered. |
| `cohesion_floor` | 0.28 | **Average** linkage, not single linkage, so one incidental edge cannot fuse two tasks. |
| `reciprocal_rank` | 2 | A tie may found a body of work only if it is among the few strongest for **both** loci. |
| `internal_cohesion` | 0.50 | Share of member-pairs that must be related. Below it the grouping is a "hub with spokes". |
| `sustained_sitting_attention` | 2 min | Least attention *in one sitting* for a group to be presented as work. |
| `engaged_attention` | 60 s | Least attention *in one sitting* for one resource to lead/name/reopen a work. |
| `min_revisits` | 2 | The smallest number of visits that is a *return*. |
| `max_primary` | 3 | Cognitive limit: restoration must not recreate a screenful of windows. |
| `attributable_change_locality` | 0.50 | An unattributable change counts only if >50% of the resource's life is inside this work. |
| `speaks_for_work` | 0.50 | >50% of a resource's total relatedness must fall inside this work for it to name/lead/open it. |
| `retrieval_margin` | 0.80 | Runner-up must reach 80% of the winner, else Evo asks instead of guessing. |
| Affinity weights | interleave 0.45 · co-episode 0.25 · lexical 0.20 · structural 0.10 | Within-sitting interleaving is the strongest witness; lexical overlap and shared path are weaker. |

The module's own stated admissibility rule **[V]**:

> *"a parameter is admissible only when it expresses a limit of what Evo can honestly claim to have witnessed, not a preference about applications, professions, or content. … Nothing here names an application, a domain, a file type, a directory, or a category of work."*

### 3.2 Why this is the ceiling, not the bug

This is a coherent, defensible position — for the question *"which resources were used as one body of work?"*. It gives real protections:

- `reciprocal_rank` stops a resource that sits beside everything (desktop, music player, inbox) from founding a group.
- `internal_cohesion` catches the resulting hub-and-spoke grouping.
- `speaks_for_work` stops a messaging window with locality 0.75 or a music player with locality 0.86 from naming a work. **[V — measured values recorded in Evo's own source comments]**
- `sustained_sitting_attention` is measured per sitting specifically so a *habit* (forty 3-second glances) cannot outrank an afternoon of work.

But the parameters are **pure functions of the witnessed co-occurrence graph**, and they contain no input describing *what the person was trying to do*. The consequences are unavoidable:

| Your complaint | Why the parameter set cannot answer it |
|---|---|
| "which apps belong to which work" | Co-occurrence answers *"on screen together"*. Two unrelated tasks in the same repo at the same hour co-occur. `cohesion_floor` was added precisely to fight this, which is evidence it is a real failure mode. |
| "which 3 of the 6 apps are still needed" | Requires knowing which sub-work is **finished**. No parameter can express completion. |
| "a YouTube video became a work" | Requires a notion of what the person was **there to do**. Co-occurrence cannot distinguish intent. |
| "no sense of urgency" | Would require a *preference* signal (deadline, waiting-on-you, unfinished). The admissibility rule explicitly forbids preferences. |
| "the name is a URL" | Requires generating a **description**. The parameters only rank existing resource names. |

---

## 4. Layer 3 — Work decision: what becomes a Work tonight

### 4.1 The gates **[V]**

To be presented as work rather than merely remembered, a group must clear:

1. `internal_cohesion ≥ 0.50` — more of its member-pairs are related than not.
2. `sustained_sitting_attention ≥ 2 min` — *somewhere* in history, one sitting had ≥2 min of genuine attention.
3. Membership floors for its resources — `min_revisits ≥ 2` and `engaged_attention ≥ 60 s` to lead/name/reopen.
4. A successful `choose_title()`.

Anything failing these becomes `Standing::Remembered`: kept, findable, but **never auto-promoted to the primary surface** — the desktop shell filters the automatic pick to `standing.is_work()`. **[V]** The primary *detail* pane therefore only ever shows something that passed all four gates; a designation by the user is the only other way in. **[V]**

### 4.2 Step-by-step failure trace: "I watched a hiking video and it became a work" **[M]+[V]**

| # | What happens | Evidence |
|---|---|---|
| 1 | Video page loads → `OBS-URL-NAVIGATED` | 5,261 such records exist **[M]** |
| 2 | Browser window becomes frontmost → `OBS-WINDOW-FOCUS-GAINED` | 12,576 exist **[M]** |
| 3 | A few scrolls/keys → `OBS-INPUT-ACTIVITY` (bundled `Keys`/`Clicks`/`Scrolls`) | 1,583 exist **[M]** |
| 4 | Browser cache/media churn → `OBS-FILE-SAVED` | 22,651 exist, by far the largest class **[M]** |
| 5 | Sittings inside `presence_horizon` merge into one Episode; total attention well clears `sustained_sitting_attention` (2 min) and `engaged_attention` (60 s) | **[V]** |
| 6 | It is reached twice across the day (open, come back) → clears `min_revisits = 2` on the trivial reading "a return happened" | **[V]** |
| 7 | It is a *single* artifact, so the degenerate case of `internal_cohesion` is trivially satisfied — there are no unrelated pairs to drag the share down | **[V]** |
| 8 | `choose_title()` finds no `opener` that `speaks_for_work`; falls through to *"participants that belong here"*, then to *every participant*, and returns `participant.resource().display_name()` | **[V]** |
| 9 | For a web surface, `display_name` is the tab title or URL → **the hiking video is presented, named, and stored as a work** | **[V]** |

**Two independent defects produce this outcome — fix either and the symptom disappears:**
- **(a) The evidence model has no "not work" state** (steps 1–7). Nothing at any point can say a leisure video is not work.
- **(b) Naming degrades to a resource identifier instead of failing or generating a description** (steps 8–9). The fallback chain ends at `"Untitled work"`, but the last *useful* rung is a raw resource name, so a URL becomes a name first.

The same trace ran for 20 apps/websites produces your other description: each incidental surface can clear the gates independently and be listed as its own work.

---

## 5. Layer 4 — Grouping: how members attach, and the subset problem

Evo builds an **affinity graph** over *loci* (resources) and clusters it. Combined affinity is a weighted sum **[V]**:

```
affinity(a,b) = 0.45 · interleave(a,b)        // alternating within one sitting
              + 0.25 · npmi_co_episode(a,b)   // co-occurrence across ≥2 episodes
              + 0.20 · shared_vocabulary(a,b) // distinctive shared words in witnessed names
              + 0.10 · shared_structure(a,b)  // same containing location / origin
```

Clustering is gated by `reciprocal_rank = 2` (a tie must be among the few strongest for *both* loci), then a pair of groups may merge only if their **average** linkage ≥ `cohesion_floor = 0.28`. Membership is deliberately generous once a group exists — a resource kept out of clustering is still admitted as a member wherever it was genuinely present, with its measured strength recorded. **[V]**

**Consequences for your first complaint:**

- Apps get grouped by *shared time on screen*, plus a weak lexical term (0.20) that only fires when witnessed names happen to share distinctive words. There is **no shared-objective term**, because no such data is captured. Four different works that share a browser, a chat app and a notes app will fight each other in this graph, and the protections that keep them apart (`cohesion_floor`, `reciprocal_rank`, `internal_cohesion`) are all doing arithmetic on co-occurrence rather than separating by purpose.
- **"Which apps comprise this work" is answered statistically, so it is unstable.** `speaks_for_work` and `max_primary` cull members by attention share, so the set you see for one work can change when you work on an *unrelated* work that happens to share a resource.
- **"Which 2–3 of the 6 are still needed" is structurally unanswerable.** Restoration picks leaders via `engaged_attention` + `speaks_for_work` and caps at `max_primary = 3` **[V]**. That cap is a *cognitive* limit — it does not ask whether the sub-task that needed an app is **done**. Evo has no completion evidence at all (§2.2). So the app you no longer need is dropped for the wrong reason (it lost an attention share), and can reappear for the wrong reason (you glanced at it last week).

Session Buddy and Workona are both installed in your Chrome profile **[M]**, and both model this exact problem explicitly — see §10.3. Neither tries to *infer* a subset; they restore a **session snapshot** and let the user go back. That is the honest primitive Evo lacks.

---

## 6. Layer 5 — Naming: why a URL becomes a work title

### 6.1 The algorithm, verbatim **[V]**

`choose_title()` in `crates/evo-engagement/src/engagement.rs`, in order:

1. **Openers** = participants whose `role.opens_on_restore()` **and** which `speaks_for_work(params)` **and** whose `display_name()` is non-empty.
2. Among openers, pick max `graph.shared_vocabulary(participant.subject(), members)` — lexical overlap with the work's members.
3. If that best shared score is **≤ 0.0** (nothing measurable shared), re-rank openers by `(specificity, locality)` — "the one the evidence places most firmly inside this work".
4. If no opener qualifies, fall back to *members that belong here*, then to **every participant**.
5. Return `(participant.resource().display_name(), shared)`.
6. If even that is empty → `"Untitled work"`.

Its doc comments record the previous failure it replaced **[V]**: the old rule let every opener compete and preferred whole `locality`; a messaging window reached locality 0.75 and a music player 0.86, and *"both went on to name work they had nothing to do with."*

### 6.2 Why this cannot produce a good title

- **It never describes an activity.** It *selects an existing resource name*. There is no stage anywhere in Evo that generates a phrase like "Researching attic insulation". So the ceiling on title quality is the quality of the most specific window/tab title in the group — and the floor is an identifier like `youtube.com/watch?v=…` or `Login`.
- **The degenerate single-artifact case always hits rung 5.** With one member, no opener can beat another, `shared_vocabulary` is 0, and there is nothing to rank — the title *is* the resource name. Your hiking video is not an edge case; it is the guaranteed output of that path.
- **`Untitled work` is unreachable in practice** because rung 4 accepts *any* participant, and every browser tab has a `display_name`. So the honest "I don't know" outcome is bypassed in favour of a misleading name. Your description — *"the right work seems irrelevant"* — follows directly: a list where one work is named after a URL makes every neighbouring name look unreliable.

---

## 7. Layer 6 — Arrangement: why there is no sense of urgency

### 7.1 The ordering function, verbatim **[V]**

`order_home_workspaces()` in `crates/evo-desktop/src/state.rs`:

```rust
let mut ordered = workspaces.to_vec();
// sort_by_key is stable: canonical order is preserved within each group.
ordered.sort_by_key(|workspace| {
    outcomes.get(&workspace.id().to_string())
        .is_none_or(|outcome| outcome.resume_point().is_none())
});
```

- Key is a **single boolean**: *does this work have a derived Resume Point?*
- **Within** each group, order is the canonical projection's order (most recently active first).
- The doc comment states the intent explicitly: *"No score, no recency, no frequency, no application identity, no guessed ranking, and no omission: a stable partition, deterministic on canonical input."*
- The Resume Point is described as *"work the user explicitly marked as continuing"* — i.e. a **user designation**, and the `display_state_from_index()` selection prefers a designated artifact before anything else. **[V]**

### 7.2 Why the list reads as unordered **[M]**

The log contains **3** `OBS-WORK-DESIGNATED` events and **4** `OBS-WORK-GROUPED` events in **52,679** observations — 0.013% of the evidence. So:

1. Practically every workspace has **no** resume point → the sort key is `true` for all of them → the partition becomes a **single group**.
2. A single group means the ordering *is* canonical order, i.e. **most recently active**.
3. Recency ≠ relevance. A background file sync, a helper process, or a chat window touched at 23:50 outranks the work you were interrupted in at 18:00, because the only surviving signal is "which thing had a witnessed event most recently".
4. Grouping three *unrelated* works that all contain a shared resource can push any of them up.

So there are two independent causes: **(a)** the ranking design deliberately refuses to rank (only a binary user-designation partition), and **(b)** the one signal that partition depends on is effectively never present. Your expectation — *order by relevance and by last committed work, because that is what a person most likely resumes* — is a **preference** signal, and §3.1's admissibility rule rules exactly that class of input out.

---

## 8. Layer 7 — Restoration

What exists: `DerivationOutcome::resume_point()`, `selection_for(workspace, outcome, locators)`, participant `role.opens_on_restore()`, `max_primary = 3`, `retrieval_margin = 0.80` (ask rather than guess when two candidates are within 20%). **[V]**

What is absent, and each absence is a concrete complaint:

| Absence | Consequence |
|---|---|
| No per-item **delay** | Everything opens at once; the machine thrashes. |
| No **condition to wait for** before opening the next item | Cannot express "open the editor, wait for the dev server, then open the browser". |
| No **force open/close** flag | Cannot reassert a workspace state over what is already on screen. |
| No **item exclusion** | Cannot say "never open the chat app as part of this". |
| No **session snapshot identity** | Restoration is recomputed each time, so the shape of a work can differ between two restores of the same work. |
| No **completion state per member** | The core of your "which 3 of the 6" question (§5). |

All six of these are first-class in Bunch's file format, which I extracted verbatim from the installed app — see §10.9. Bunch is the best available specification for what a restore/activation step should accept.

---

## 9. Root causes mapped to your complaints

| # | Your complaint | Root cause(s) | Where | Fixes that address the cause |
|---|---|---|---|---|
| 1 | Can't group apps by work | Grouping is a co-occurrence graph with no shared-objective term; capture supplies no objective evidence | §3.2, §5, §2.2 | §11.3 (classification layer), §11.4 (objective + completion), calendar/task evidence |
| 2 | Can't tell which apps comprise a work | Membership decided by attention share (`speaks_for_work`) over a growing total; unstable when works share resources | §5, §3.2 | §11.4 (explicit membership derived from objective + rules), §11.3 |
| 3 | Can't tell which 2–3 apps are still needed | **No completion/next-action model anywhere**; `max_primary` is a cognitive cap, not a completion test | §5, §2.2 | §11.4 (per-member lifecycle state), §11.6 (activation spec + session snapshot) |
| 4 | Anything becomes a work (YouTube video) | No negative/distraction signal in capture; gates trivially passable by a lone artifact; naming degrades to a resource name | §4.2, §2.2, §6.2 | §11.3 (media/distraction axis), §11.2 (sessionization), §11.5 (naming contract) |
| 5 | No order / no urgency | Binary boolean sort on a resume point that is user-designated and has occurred 3× in the whole log; ranking explicitly refused | §7, §7.2 | §11.7 (urgency model) |
| 6 | Names are URLs / meaningless | Name = `display_name()` of a resource; no description generation; `Untitled work` bypassed | §6.2 | §11.5 |

---

## 10. Reverse-engineered mechanisms from the installed apps

### 10.0 Method: what was actually run, and where it failed

This section is **my own reverse engineering**, done with **REA CLI 4.0.1** (`rea`, `/opt/homebrew/bin/rea`) plus direct reading of shipped artifacts. The REA audit trail below is reproducible: each REA run returns an `evidence_id` and a subject `sha256`, so every claim can be traced to one analysis of one exact binary.

**REA operations that succeeded** —

| REA operation | Target | `evidence_id` | Notes |
|---|---|---|---|
| `inspect-plist` | `/Applications/ManicTime.app` | `ev_b64cc2aa3abd5f94c8a2c5a79a7132c915f7be33123a196e35f16143e3a67607` | ManicTime `2026.2.1.2`, sha256 `20c27db9…a89` |
| `inspect-plist` | `/Applications/Bunch.app` | `ev_5399546ecb62d95dc206e7c55fb131022ff810a22d5c55e61bfaaa4e1247c2b2` | sha256 `5b12dadb…14b` |
| `inspect-plist` | `/Applications/Timing.app` | `ev_df53179dfde5e266ac104302d0981aa0b71b5a6036b43b4ece09e01a28d32c42` | sha256 `55fe35aa…5fe` |
| `inspect-native-dispatch-metadata` | `/Applications/Bunch.app` | `ev_966db2f1…` (see `rea-native/bunch_dispatch.json`) | **140 ObjC classes, 985 dispatch implementations** |
| `inspect-native-dispatch-metadata` | `/Applications/ManicTime.app` | `ev_79fe0cee88cc50cf8ffc88edb63a95f4a595d92cce2322b8849ecc78a8198d53` | empty class list — see below |
| `inspect-macho` | `/Applications/Dia.app` | recorded in `rea-native/dia_macho.md` | 3.4 MB of format/metadata analysis |
| `inspect-managed-artifact` | `Finkit.ManicTime.Shared.dll` | `ev_e4a3f5fb79dd112a6711e85ceeb44f30d0161534294ee59d048f6ec587b1e8f7` | PE/CLI, sha256 `8b0a0000…5b48` |
| `inspect-managed-members` | `Finkit.ManicTime.Shared.dll` | `ev_b382f76e21a36c1b2e2ad2018b0fa7ea0bc9c79f901ec79ba9f1bd02dee18020` | 208 MB of type/member metadata |
| `inspect-managed-members` | `ManicTime.Client.Tracker.dll`, `ManicTime.dll` | see `dotnet/*_members.json` | 57 MB / 9.4 MB |
| `analyze-javascript-application` | Workona extension dir | `ev_89c4def27b364984a2d4927e7085b1eb220e1c4498030867f2c8ee6a205cfb44` | **succeeded**, but the evidence envelope is 138 MB |

**Three REA limitations hit, all recorded rather than worked around silently** —

1. **`analyze-javascript-application` fails closed on `app.asar` when the app uses `asarUnpack`.** Granola, Toggl and Rambox all returned `artifact_integrity_mismatch`: the sha256 declared in the asar header for an unpacked native module does not match the file on disk (Granola: `node_modules/@napi-rs/xattr-darwin-arm64/xattr.darwin-arm64.node`; Rambox: `better_sqlite3.node`; Toggl: `koffi.node`). REA refused the analysis rather than analysing a possibly-modified artifact — correct behaviour, and a useful fact in itself: **for these apps `app.asar` alone is not a self-contained verifiable artifact.**
2. **`inspect-native-dispatch-metadata` refuses targets over 64 MiB** — for Dia the diagnostics read: *"Apple metadata target must be a regular file no larger than 64 MiB"*. Dia's Mach-O `inspect-macho` succeeded; dispatch metadata did not. **[?]**
3. **`inspect-managed-members --format md` crashes** with `RangeError: Maximum call stack size exceeded` in incur's markdown formatter on large member tables. `--format json` works and is what all ManicTime findings below come from. **[M]**

**Workaround used, stated plainly:** I wrote a minimal ASAR reader (`evidence/js/extract_asar.py`) to materialise each app's text payloads, verified it (146 sampled JSON files parsed, 0 bad; `@granola/electron` v7.645.2 read from its own `package.json`), then mined those trees with single-pass context extraction (`evidence/js/extract_context.py`). Claims sourced this way are tagged **[M]** with the file named, and I flag where REA itself completed the analysis.

**A methodological surprise worth recording:** `ManicTime.app` on macOS **is a .NET application** — `Contents/MonoBundle/` holds 354 assemblies including `ManicTime.dll`, `ManicTime.Client.dll`, `ManicTime.Client.Tracker.dll`, `Finkit.ManicTime.Shared.dll`, plus a bundled `Irony.dll` (a .NET grammar/parser library). **So the Windows build did not need to be downloaded**: REA's `rea-dotnet-static` provider analysed the same managed logic from the installed macOS client. **[M]**

Evidence files: `~/Desktop/evo-reverse-engineering/evidence/` — `rea-native/`, `dotnet/`, `rea-js/`, `asar/{Toggl,Granola,Rambox}/`, `js/`, `MECHANISMS-SUMMARY.txt`.

### 10.1 Timing — ordered rule list + keyword params + hierarchy **[M]** (REA `inspect-plist`: `ev_df53179d…`)

Extracted symbols/prefs from the installed bundle:

| Evidence | Extracted from |
|---|---|
| `ProjectRuleOrderViewController` | Mach-O class table |
| `info.eurocomp.Timing2.preferences.projectRuleOrder` | preference key |
| `paramDescriptorForKeyword:` | Objective-C selector |
| `hideClickedOrSelectedKeywords:` | Objective-C selector |
| `HierarchicalActivityListController` | class table |
| `PerProjectHierarchicalActivityListController` | class table |
| `TimerSuggestion` | class table |
| `ProductivityScoreController` / `ProductivityScoreValueFormatter` / `ProductivityScoreControllerDelegate` | class table |

**Mechanisms worth taking:**
1. **Rules have an explicit persisted order** (`projectRuleOrder`). Classification is therefore deterministic and *inspectable* — a human can see and reorder why an activity went where it did.
2. **Keywords are parameters of a rule** (`paramDescriptorForKeyword:`), not a separate fuzzy layer — so a rule carries its own match inputs.
3. **Explicit two-level hierarchy** (`PerProjectHierarchicalActivityListController`): time rolls up project → sub-activity. Evo's flat graph has no hierarchy, so it cannot express "these three windows are three parts of one task".
4. **A productivity score is a separate axis** from project assignment. This is the key structural idea: *where time went* and *how good that time was* are different questions with different evidence.

### 10.2 ManicTime — Absorb, gap-fill, ordered rule grammar, separate categorization **[M]** **[D]**

**Now analysed from the shipped .NET assemblies with REA (`rea-dotnet-static`), not just docs.** The autotag subsystem is a real, large component: across the three assemblies analysed, `AutoTag` appears **21,447** times, `Rule` 3,856, `Filter` 5,822, `Productivity` 5,321, `Category` 5,846, `Timeline` 79,243. It lives in its own namespace, **`Finkit.ManicTime.Shared.AutoTags`**, and the type names are the specification:

| Extracted type | What it proves |
|---|---|
| `AutoTag` (11 fields), `AutoTagGroup`, `AutoTagHelper`, `AutoTagSettings`, `AutoTagPrivateSettings`, `ClientAutoTagSettings` | autotags are first-class persisted objects with public **and private** settings |
| **`AutoTagFactory.AutoTagTimelineAbsorb`** and **`AutoTagFactory.AutoTagTimelineAppend`** | **Absorb and Append are two distinct application modes**, exactly as documented — now confirmed in the binary |
| **`AutoTagFactory.AutoFillHelper`** (`AutoFillInSeconds` is a real field) | the gap-filler is a dedicated component, and its threshold is a **stored per-autotag second count**, not a constant |
| `AutoTagRule`, `AutoTagRuleTimeline`, **`AutoTagRuleComparer`**, `AutoTagRuleValueComparer`, `AutoTagRuleEqualityComparer` | rules live on **their own timeline**, and there is an explicit **ordering/comparison** notion — priority is data, not code |
| `ApplyTo` (enum, **4 fields**) + `AutoTagModelApplyTo` + `GetAutoTagsToApplyTo` | the "All vs Selected" target set is an enum with four values |
| `AutoTagType` (enum, **5 fields**), `AutoTagSource`, `CopyTagsToAutoTagsOverwriteMode` | five kinds of autotag; "copy to ordinary tags" has an explicit overwrite mode |
| `AutoTagTimelineWhitelistItem`, `AutoTagWhitelistItemHelper` | a **per-autotag allow-list** of whitelisted items |
| `IntervalOccupancy`, `AutoTagOccupancy`, `AutoTagOccupancyFactory` | an **occupancy** model over intervals — the mechanism behind "split the gap equally" |
| `AutoTagTimelineSchemaUpdater`, `PatchAutoTagTimelineSchemaAsync`, `RepairAutotagTimelineLastUpdate` | autotag timelines are **schema-versioned and repairable**, i.e. rules are re-evaluated against stored history |
| `Dto.AutoTagDto`, `AutoTagRuleDto`, `AutoTagRuleTimelineDto`, `AutoTagSettingsDto`, `AutoTagTimelineWhitelistItemDto` + a parallel `Dto.Export.*` family | the whole rule set has a **serialisable import/export format** |
| bundled **`Irony.dll`** | the filter/rule language is **parsed by a real grammar**, not string-split |

Two REA limitations on this finding, stated as such: `inspect-native-dispatch-metadata` on the macOS binary returned an **empty ObjC class list**, because it is .NET rather than ObjC/Swift — the dispatch reader had nothing to read, and the managed reader is the correct route. And I resolved the **field counts** of the `ApplyTo` (4) and `AutoTagType` (5) enums but **not their individual member names**; the documented names All/Selected remain **[D]** and the remaining values are **[?]**.

The corresponding vendor documentation ([Autotagging](https://docs.manictime.com/win-client/autotagging), [Searching](https://docs.manictime.com/win-client/searching), [Automatic Categorization](https://blog.manictime.com/articles/dec-2024/automatic-categorization/), [DB schema](https://docs.manictime.com/server/on-premise-installation/connect-to-database/getting-data-from-database)):

- **Autotags are computed first; then Absorb** reassigns an interval to a *neighbouring* autotag. Canonical example: Word → 1 min Google → Word becomes all-Word. Absorb can be limited to **All** or **Selected** target tags. ManicTime explicitly documents this as *a guess that may be wrong*. **The exact window/threshold is not documented.**
- **Auto-Fill-Gaps**: setting is *"Auto fill gaps smaller than X seconds"* — **threshold is user-configured, no documented default**. If the tags on both sides are identical, the gap is filled; if they differ, the gap is **split equally**. Also documented as a guess.
- **Rule grammar**: wildcards `?` (one non-whitespace char) and `*` (any run of non-whitespace), plus regex as `#"…"` with capture groups usable as `{{1}}`, `{{2}}`. Matchable data includes **application/group, activity/window title, document path, URL**.
- **Priority**: autotags may overlap; with *"When first autotag is matched skip the rest"* enabled, they are evaluated **in displayed order** and later tags only see time left free by earlier ones. Reordering = reprioritising.
- **Retroactivity: yes.** Rules are re-evaluated over past days, so editing a rule changes historical results. To freeze a result, copy autotags into ordinary tags.
- **Data model**: activities have title, start, end, duration, group; **tags are intervals on their own timeline**, not a field on each activity — the Tag timeline is the one *not* auto-populated.
- **Distraction handling is a separate feature**: Automatic Categorization assigns Productive / Unproductive / Social Media / Gaming / **Media & Entertainment**, and categories can be manually managed.

**Mechanisms worth taking:** (a) an **interval-based classification layer over raw activity**, (b) **ordered rules with a "first match wins" mode**, (c) **retroactive re-evaluation**, (d) **absorb + gap-fill** — these two directly kill the "one incidental interval became a work" artifact, and (e) a **media/entertainment category axis that is independent of work clustering**, which is precisely the missing negative signal from §2.2.

### 10.3 Workona + Session Buddy — session snapshot vs saved resource **[D]** **[M]**

**Verified installed in the `Default` Chrome profile [M]** (read from each extension's `manifest.json`): **`Tab Manager by Workona`** (`ailcmbgekjpnablpdkmaaccecekgdhlh`, v3.1.35) and **`Session Buddy - Tab & Bookmark Manager`** (`edacconmaakjimmfgnblocblbcdcpbko`, v4.1.2), alongside `Cookie-Editor`, `html.to.design`, `HTML to Framer`, `Instant Data Scraper`, `Jobright Autofill`, `Loom`, and `Castlevania`. Evo also maintains separate `EvoWork-*` profiles **[M]**. The two tab managers are the most relevant: both are *session* systems, and Session Buddy in particular is the closest installed analogue to the snapshot mechanism proposed in §11.6.

**Session Buddy's actual data model, extracted from its shipped bundle [M]** (`edacconmaakjimmfgnblocblbcdcpbko/4.1.2_0/main.bundle.js` + `background.bundle.js`, 2.4 MB concatenated). These are literal constants from the code, including its versioned import/export schema:

```js
Xs = ["sessions", "collections"]        // top level
Qs = ["windows",  "folders"]           // inside a session
Js = ["tabs",     "links"]              // inside a window
Zs = ["session",  "collection"]        // singular forms
paths = ["$.format", "$.collections.*", "$.sessions.*", "$.windows.*", "$.tabs.*", "$.history.*"]
Ms = { saved:    "nxs.v3-id-map-saved-sessions",
       previous: "nxs.v3-id-map-previous-sessions" }
```

Four mechanisms are visible here, and each transfers directly to Evo's `SessionSnapshot`:

1. **A session contains windows, and a window contains tabs** — the hierarchy is *window-scoped*, so a snapshot can restore layout, not just a flat list of URLs.
2. **`folders` are a distinct level** from `tabs` — grouping of tabs exists *inside* the snapshot, independent of any inference.
3. **A versioned export format (`$.format`) with an explicit JSON schema path list** — snapshots are portable and validated on import (`IngestFormatParseError` is a named error).
4. **`saved` sessions and `previous` sessions are separate id-maps.** The product distinguishes *what you deliberately kept* from *what was merely last seen* — which is precisely the `pinned`/`durable` vs `live`/`transient` distinction Evo lacks. Counts from the same bundle: `windows` 278, `restore` 96, `incognito` 126, `favIconUrl` 78, `pinned` 19.

Both bundles are minified, so property names are mangled and the internal restore heuristic (if any) is **not** readable statically — recorded as **[?]**, not guessed.

Per Workona's help ([Tab Manager](https://workona.com/help/tab-manager/), [Spaces](https://workona.com/help/spaces/)):

- A **space** is a persistent container for resources, notes, tasks and — with Tab Manager — a **tab session**.
- **Tabs are not resources.** Tabs you open while working autosave into the space's live session; keeping one as a durable link requires an explicit **Save as resource**.
- **Opening a space restores its most recent tab session.** Switching spaces merely hides the previous session; it does not close it.
- **Session history + hourly backup snapshots** (product page: restorable up to 90 days) let a user restore a whole older session or individual tabs — **a manual action**.
- **There is no documented heuristic that decides which saved items to reopen.** The current session *is* the working set; saved resources are separate.

**This is the single most important lesson for your "which 2–3 of the 6" question:** the mature product answer is *do not infer it — record the session the working set actually was, and let the user choose among snapshots.* Evo instead recomputes a subset statistically on every restore, which is both unstable (§5) and unexplainable to the user.

### 10.4 Arc / Dia — spaces, pinned-tab profiles, archive with a reason **[M]** **[D]** **[B]**

Extracted from the installed bundle **[M]**: `PinnedTabNavigationProfile`, `InstagramDirectClientPinnedTabNavigationProfile`, `ArcImportSourceProfileLoadAccess`, `ImporterProfile`, `Archive` / `ArchiveError` / `ArchiveIcon`, profile and sync classes. REA's `inspect-macho` on `/Applications/Dia.app` completed and its output is at `rea-native/dia_macho.md`; `inspect-native-dispatch-metadata` was **refused** by the provider because Dia's Mach-O exceeds the 64 MiB reader limit (§10.0), so the class list above comes from direct Mach-O metadata/strings rather than the dispatch reader.

- A **per-site pinned-tab navigation profile** is a durable, named configuration of *which URLs of a service are the ones that matter to you* — a data model for "the 3 tabs of this app that are the working set", owned by the user and **independent of attention statistics**.
- **Archive** exists as a first-class state distinct from closed and from pinned, and the auto-archive staleness rule (~12 h) plus a reason field distinguishing automatic from manual archive is the vendor's documented behaviour **[D/B]**.
- **Import from another browser's profiles** (`ArcImportSourceProfileLoadAccess`) implies the space/profile model is portable and is a first-class artifact, not derived.

**Mechanisms worth taking:** (a) **pinned/durable vs live/transient** as two explicit states instead of one statistical membership set; (b) **auto-archive with a reason field** as the honest way to keep stale works out of the way without deleting them (this pairs perfectly with Evo's correct `Remembered` standing **[V]**); (c) **user-owned profiles per service** as the source of truth for "which URLs of this service belong to this work".

### 10.5 Dayflow — the naming and distraction contract, **verbatim from local source** **[V]**

Dayflow is MIT, source at `~/Downloads/Dayflow-main`. This is the most directly useful artifact for your naming and "anything becomes work" complaints, because it is the *only* one of the reference apps whose logic is readable.

**Data model** (`Core/AI/LLMTypes.swift`):

```swift
struct ActivityCardData: Codable {
  let startTime: String
  let endTime: String
  let category: String
  let subcategory: String
  let title: String
  let summary: String
  let detailedSummary: String
  let distractions: [Distraction]?   // ← distraction is a FIRST-CLASS TYPED FIELD
  let appSites: AppSites?            // ← explicit app/site attribution
}
```

**The naming prompt** (`Core/AI/ClaudePromptDefaults.swift`) — quoted verbatim, because it is the specification you asked for:

> *Write each title as the natural answer to "What did I spend this time doing?" Use a short phrase in sentence case, usually beginning with an activity verb. Name the main activity and its familiar subject. Add a method, person, comparison, creative treatment, or version only when it meaningfully distinguishes this episode. Specificity is optional: keep a title simple when the activity already identifies it. **Prefer a recognizable approach over a list of implementation terms or the platform where work ran.** The title should be understandable on its own tomorrow, accurate to the observed activity, and consistent with neighboring titles.*

With these worked examples (the fourth is your exact complaint, solved):

| Evidence | Title |
|---|---|
| investigated failed OAuth callbacks and Redis sessions for a product named Cedar | `Fixing Cedar sign-in` |
| tested whether combining radar and satellite readings improved Rainbird forecasts | `Testing radar and satellite fusion for Rainbird forecasts` |
| revised the aims and budget of a grant application through an editor and assistant | `Revising the grant proposal` |
| **watched basketball clips for twenty minutes and briefly checked a parcel** | **`Watching basketball highlights`** |
| read advice on insulating an attic; no installation observed | `Researching attic insulation` |

**The evidence contract** (verbatim), which is the mechanism that makes the above possible:

> *For EVERY card, after its detailedSummary and before its title, output a titleEvidence object with activities (an array of {activity, minutes}), selectedActivity, and familiarSubject. **Account for the entire interval, with approximate minutes summing to the card duration. Combine recurring visits to the same actual task; different subjects remain separate even when they share an assistant, browser, or broad project. Count foreground interaction, not background windows.** Select the activity with the most supported time. If it is strictly larger than every other activity, the title names that activity alone. Put side activities only in the summaries. … Omit incidental tools and brief detours.*

Four mechanisms are stated here, and each maps onto a specific Evo defeat:

| Dayflow mechanism | Evo's equivalent |
|---|---|
| **`activities[]` with minutes, summing to the interval** — an explicit, checkable accounting of the span | Nothing. Evo has no per-interval accounting; it aggregates co-occurrence statistics globally. |
| **"Combine recurring visits to the same actual task"** | Partially: `min_revisits` counts returns, but nothing *decides* whether two visits are the same task. |
| **"different subjects remain separate even when they share an assistant, browser, or broad project"** | **Absent.** This is precisely §5's failure: shared browser/chat fuses unrelated subjects. |
| **"Count foreground interaction, not background windows"** | Absent. Evo records `FILE-SAVED` for background churn (43% of the log) and has no foreground/background distinction in the evidence. |
| **`distractions[]` as a typed field** | **Absent.** Evo's evidence model has no way to represent this at all (§2.2). |

Plus: `onTitleChange` and `onCategoryChange` callbacks in `Views/UI/MainView/ActivityCard.swift` **[V]** — the correction affordance. And `ActivityCardPromptOverrides` with `titleBlock`/`summaryBlock`/`detailedBlock` **[V]**, i.e. **the prompt itself is a user-editable artifact**, which is how the product stays tunable without shipping a new build.

**The one-line conclusion:** Dayflow solves your naming and distraction complaints *without* either a co-occurrence graph or an application allow-list. It (a) captures enough to see intent, (b) forces explicit interval accounting, (c) makes distraction a typed output, and (d) asks a language model for a sentence in place of a resource identifier. Evo has **no language model anywhere in inference** — naming is a string copy **[V]**. That is the largest single architectural gap between them.

### 10.6 Rize — most-specific rule, deferred inference, calendar + task evidence **[D]** **[M]**

Per Rize's docs ([How Categorization Works](https://docs.rize.io/categories-and-tracking-rules/how-categorization-works), [App & Website Rules](https://docs.rize.io/labels-and-app-rules/app-and-website-rules), [AI-Powered Features](https://docs.rize.io/automatic-tracking/ai-features), [Time Entry Suggestions](https://docs.rize.io/automatic-tracking/time-entry-suggestions-settings), [Focus Sessions](https://docs.rize.io/focus/create-focus-sessions), [Meeting Sessions](https://docs.rize.io/meetings/create-meeting-sessions)):

- **Resolution order: the most specific matching app/website rule wins.** Deterministic, and specificity is a property of the rules, not of observed attention.
- **Deferred inference:** unmatched activity is temporarily *Miscellaneous*, and only **after two minutes** does the model assign a category. A cheap, explicit "don't spend inference on a glance" threshold.
- **Suggestion inputs:** app/window/URL context, **rules and keywords**, semantic matching, **calendar meeting titles and attendee information**, work history, and **synced task data (ClickUp/Linear/Asana)**.
- **Focus session thresholds:** ≥**15 minutes** in which ~**75%** of activity is in apps/sites configured for focus — *or* a calendar keyword (`#rize-focus`, "focus", "flow").
- **Meeting sessions:** a confirmed calendar event with ≥2 attendees, or 15+ minutes inside an app/site rule configured as a meeting. Calendar is *evidence*, not decoration.
- **Corrections → rules:** Rize documents reviewing a suggestion, correcting it, then fixing the underlying cause (add keywords, create a tracking rule, connect task data, change instructions) — and recommends rules for recurring patterns. It does **not** document that a correction automatically becomes a rule.

**Mechanisms worth taking:** (a) **most-specific-rule-wins**, giving deterministic auditable classification alongside the statistical layer; (b) the **2-minute deferral** before spending inference; (c) **calendar + task-system evidence**, both entirely absent from Evo (§2.2); (d) the **dual-threshold focus rule** (duration *and* share) — which is a better-formed version of Evo's `sustained_sitting_attention`.

**Confirmed in the shipped app [M].** Rize is the one Electron app here that ships an **unpacked** renderer (`Contents/Resources/app/`), so its UI strings are directly readable and they corroborate the documented model:

- **Session types are an explicit enum**: a `switch` over session type returns `"for breaks"`, `"for meetings"`, and the default `"for focus sessions"` — so *breaks*, *meetings* and *focus sessions* are three distinct session kinds.
- **A tracking rule carries a category and a blocking policy**: the same code path reads `t.alwaysBlock` and `t.categor…` off a `trackingRule` — i.e. a rule object with a **blocking flag** and a **category**, which is the app/website rule the docs describe.
- **Focus sessions are announced and goal-oriented**: `"Your focus session started."` with the prompt *"Writing down a goal for your session helps you retain focus…"*, and `"Your focus session ended."` / *"Nice work completing …"* — plus a summary that counts *"…meetings"* and *"…scheduled focus sessions"* "today".
- Rize also bundles `tesseract.js` (OCR) in its renderer, so it captures screenshots and reads text from them — a signal Evo does not have at all.

So Rize's answer to "was this work?" is **rule object → category + alwaysBlock**, with the human setting a goal at session start and the app reporting afterwards. That is the shape §11.3 and §11.8 should take.

### 10.7 Toggl Track — the explicit record + dismiss as negative evidence **[D]** **[M]**

Per Toggl's support and API docs ([iOS timer](https://support.toggl.com/en-us/article/toggl-track-timer-for-ios-8ew1yp/), [Windows desktop](https://support.toggl.com/en-us/article/toggl-track-desktop-app-for-windows-5w1y5/), [API v8](https://github.com/toggl/toggl_api_docs/blob/master/chapters/time_entries.md), [v9 reference](https://engineering.toggl.com/docs/track/api/time_entries/)):

- **Data model:** a time entry = `description`, `wid`, optional `pid`/`tid`, `billable`, `start`/`stop`, `duration` (negative while running), optional tag names. Projects and tags are separate workspace-scoped resources. **The user-facing unit is a named, described record** — not a derived cluster.
- **Suggestions** come from the next calendar event and commonly-used entries; the Windows List view suggests based on **calendar events the user previously copied**, and *"more calendar-event copying increases the chance"*.
- **Accept / dismiss is explicit:** the user taps to start, or picks **Add** or **Dismiss**; *"Dismiss clears the suggestion from the List view."*
- **Autotracker** is rule-based: a keyword rule matches a foreground app/window; with *"Start a Time Entry without asking"* off the user is prompted, on it starts automatically; and triggering can be **delayed until the app has remained in focus for a chosen time**.

**Mechanisms worth taking:** (a) a **first-class user-facing record** with description/project/tags, alongside (not replacing) the derived view; (b) **Dismiss as recorded negative evidence** — the cheapest possible implementation of the missing "not work" signal, and the direct antidote to §4.2; (c) **"arm only after N seconds of focus"**, which suppresses exactly the glancing-at-a-tab artifacts; (d) **most-recently-accepted suggestion** as a mild ranking input for §11.7.

**Confirmed in the shipped app [M].** Extracted from `app.asar` (7,858 text files, 68 MB) — literal IPC channel constants from an `index.cjs`:

```js
IDLE_DETECTION_DISMISS:      "idle-detection:dismiss"
IDLE_ACTION_TAKEN:           "idle-detection:action-taken"
AUTOTRACKER_SET_BETA_ENABLED: "autotracker:set-beta-enabled"
AUTOTRACKER_GET_ENABLED:      "autotracker:get-enabled"
AUTOTRACKER_SET_ENABLED:      "autotracker:set-enabled"
AUTOTRACKER_ENABLED_CHANGED:  "autotracker:enabled-changed"
```

Two things follow, and both matter for Evo:

1. **Idle detection is its own subsystem with two explicit actions** — `dismiss` and `action-taken`. "Idle" is not inferred from gaps, and the user's response is recorded as a distinct event. This is a concrete, cheap design for the idle signal Evo currently guesses at (§2.2) **and** for the negative-feedback path (7 events in 52,679).
2. **The autotracker is enable/disable state that is broadcast and persisted** (`set-enabled`, `enabled-changed`, plus a separate `beta` flag), rather than a fixed always-on inference. The user can turn the automatic grouping off — a control Evo does not offer.

### 10.8 Granola — the capture model, and where naming actually lives **[M]**

Granola's `app.asar` (70,189,093 bytes) refused REA's JavaScript analyser with `artifact_integrity_mismatch` on `xattr.darwin-arm64.node` (§10.0), so I extracted and read the shipped JS directly. Identification from its own `package.json`: **`@granola/electron` v7.645.2**, main `dist-electron/main/index.js` (6.98 MB, minified).

**The process tree is the finding.** Granola's `dist-electron/` contains one process per evidence source:

| Process | Evidence it implies |
|---|---|
| `accessibility_process` | macOS **Accessibility API** — the same source Evo uses |
| **`eventkit_process`** | **Calendar (EventKit)** — the signal Evo has none of (§2.2) |
| `audio_process`, `local_asr_process`, `local_diarization_process` | audio capture + **local speech recognition** + speaker diarisation |
| `mission_control_process` | window/space observation |
| `tcc_process` | macOS **consent/permission** state tracking |
| `sqlite_process` | local database |

**The calendar integration is explicit and its data model is visible.** From the main bundle: a native module `eventkit.node`, IPC handlers `eventkit:get-authorization-status` and `eventkit:request-access`, and a store whose keys are literally:

```
events · eventKitEvents · people · peopleLastFetchedAt · meetingsMetadata
winWebcamCameraState · camStreamActiveSinceUnixMs
```

So Granola's store carries **calendar events, the people attending them, when attendees were last refreshed, meeting metadata, and webcam/mic stream state**. Note `peopleLastFetchedAt`: attendees are a fetched, cached entity, not a string parsed out of a title.

**On naming, the honest negative result:** I searched the client for `generateTitle`, `autoTitle`, `titleSource` and found **no client-side title generator**; the only matches for `eventkit` were the ones above, and the renderer's `Rename`/`Untitled` strings were not present in the minified chunks I scanned. The consistent reading is that **titles are produced server-side by a model, and the client's job is to store the note, display the title and let the user rename it** — which is exactly the human-in-the-loop pattern, with the generator out of reach of static analysis. Recorded as an inference from a bounded negative search **[M]** → **[?]**, not as a proven mechanism.

The transferable mechanism is not the naming call, it is the **capture breadth**: Granola reads calendar and attendee context, microphone/audio state, accessibility window state and its own permission state, and gives each its own process. Evo reads accessibility, file events and URLs. That gap, not the prompt, is why Granola can say what a meeting was about.

### 10.9 Bunch — the activation/restore semantics, **REA dispatch metadata** **[M]**

**Now analysed with REA's dispatch reader**, which is the right tool for a native ObjC app: `inspect-native-dispatch-metadata /Applications/Bunch.app` returned **140 Objective-C classes and 985 dispatch implementations**. The class and ivar names are the specification:

| REA-extracted symbol | Meaning |
|---|---|
| `BTBunch`, `BTBunchItem`, `BTBunchWindow` | a bunch is a list of **items**; there is an explicit window model |
| `BTBunchCommander`, `BTBunchController` | command + controller split: parsing and execution are separate |
| **`BTBunchScheduler`**, `_openAt` | **scheduled** activation — a bunch can open at a time |
| `_launchPath` | per-item launch target |
| `_delay`, `_delayedSnippets` | **per-item delay**, and snippets that are deferred |
| `_waitingSnippet`, `_snippets` | **wait-for-evidence** before continuing, with a snippet list |
| `_forceOpenClose` | force open/close instead of deferring to current state |
| `_closeOn`, `_closeSnippets` | **close-on-exit**, with a separate snippet list for closing |
| `_ignoreInSingleBunchMode` | **exclude this item** from the reduced/single mode |
| `_launchedBunches`, `_recentBunches`, `_bunches`, `_bunchesDir` | bunch state is persisted in a **user-owned directory** and recent ones are tracked |
| `listBunches`, `listOpenBunches`, `listTaggedBunches`, `openBunch`, `closeBunch`, `refreshBrowsersForBunch` | a full lifecycle API, including **tags** and a browser refresh step |
| **`newBunchWithOpenApps`, `copyBunchWithOpenApps`** | **capture the current machine state as a bunch** — the snapshot primitive, exactly as in §11.6 |

**This is the missing specification for Evo's Restoration layer (§8).** An activation plan should accept, per item: *what to open, with what arguments, after what delay, conditioned on what evidence appearing, whether to force the state, and whether the item is part of the core set or optional.* Evo today has none of these per-item affordances — only a global `max_primary = 3`. **[V]**

Note also that `_ignoreInSingleBunchMode` is what makes the "6 apps become 2–3" case a **declared property of each item** rather than an inference, and `newBunchWithOpenApps` is how a good activation plan gets created in the first place: **you snapshot the state you were actually productive in**, rather than recomputing it later.

### 10.10 Rambox — workspace/partition model **[M]**

Extracted `app.asar` in this pass (61 text files, 8.5 MB); the main bundle is a Vite build (`.vite/build/index.js`, 2.7 MB) which is minified, so thresholds are **[?]**. Two concrete facts did come out of it **[M]**:

- **The partition model is a first-class, server-backed concept.** The bundle contains a generated API client with `meControllerGetPartitions` calling **`GET /me/partitions`** — partitions are named, addressable objects in the product's API, not a local rendering detail. A workspace's apps are partitioned sessions that the user owns and the backend tracks.
- **Rambox bundles `electron-chrome-extensions`** (a full `chrome-extension-api.preload.js` plus an extension runtime). So its workspaces run **real, isolated browser profiles with extensions installed per workspace** — that is what "the same app, separate session" means concretely.

Combined with Arc's per-site profiles (§10.4), this is the same idea three times: **a work is a named container of partitioned, durable app instances — declared, not inferred.**

### 10.11 What the reference apps do NOT support

Worth stating so this document does not oversell the field. Every app above is strong on **declaration and correction** and weak on **inference**: none of them claims to discover, unaided, which of 20 open apps belong to one piece of work. They ask, or they snapshot, or they apply your rules. That is consistent with the media finding in Appendix C (*nobody credible claims to infer intent from playback alone*), and it is the standard Evo should be held to: **be right by construction where the user has told you something, and be honest and correctable where you are guessing.**

### 10.12 Cross-app synthesis

Read the nine together and one split appears consistently, in every app that works:

| Question | How these products answer it | How Evo answers it today |
|---|---|---|
| *What did the person do?* | Explicit accounting of the interval (`activities[]`+minutes) and a generated description (Dayflow) | A copy of a resource name (§6) |
| *Was it work?* | A **separate classification axis** (Productive/Unproductive/Media — ManicTime; focus/meeting sessions — Rize; typed `distractions[]` — Dayflow) | **No axis exists** (§2.2) |
| *What belongs together?* | Declared containers: spaces (Workona), workspaces (Rambox), pinned profiles (Arc), ordered rules (Timing, ManicTime) | Inferred by co-occurrence (§5) |
| *What should reopen?* | The **recorded session snapshot**, chosen by the user (Workona), or an explicit per-item plan (Bunch) | A statistical subset, recomputed each time (§8) |
| *How is it named?* | Generated from evidence, then corrected (Dayflow, Granola/Toggl) | `display_name()` (§6) |
| *What is most urgent?* | Deadlines/calendar (Rize), explicit records (Toggl), rarely attention shares (Timing scores) | A boolean with a 3-event history (§7.2) |

**Evo is the only one of the ten that answers "what belongs together", "was it work", "what should reopen" and "how is it named" with a single anonymous statistical mechanism.** Every reference app separates these into at least three layers, and each layer has its own evidence and its own user-editable rules.

---

## 11. Target architecture

Design rule for everything below: **keep Evo's log-is-truth invariant and its refusal to delete evidence; add the evidence and the layers the reference apps show are required.** Nothing here proposes guessing more from the same signals — the point is to capture what is missing and to separate questions that are currently conflated.

### 11.1 Module 1 — Signal model: two axes, not one

The current pipeline reduces everything to "was this on screen". Split every observation into two independent attributes *at capture time*:

```
Observation {
  actor:    Human | Machine          // did a person cause this?
  intent:   Work | Neutral | Leisure | Communication   // classified, not inferred from co-occurrence
  ...existing fields (time, locus, name, source)
}
```

- `actor: Machine` = the 62.0% that is file-system churn unless it is *attributable to a person* — reuse the existing, correct `attributable_change_locality` logic **[V]** to make this split rather than inventing a new one.
- `intent` is populated by **rules first** (§11.3) and by deferred inference second (Rize's 2-minute pattern) **[D]**, never by co-occurrence.

### 11.2 Module 2 — Sessionization with real boundaries

Keep `presence_horizon` (20 min) and `max_interval_attention` (10 min) **[V]**, but add the boundaries that are currently guessed from gaps:

- **Screen lock / sleep / idle** → a hard Episode boundary, replacing inference from silence.
- **Media playback state** → a media interval carries `intent: Leisure|Work` explicitly; ambient media (music) is marked *background* and excluded from attention totals, and a media interval with no other foreground activity can never found a work.
- **Call/meeting state** → an Episode with a declared subject (calendar title) rather than an inferred one.

### 11.3 Module 3 — Classification layer (the layer Evo is missing entirely)

An **interval layer over raw activity**, exactly as ManicTime models tags **[D]**:

```
ClassificationRule {
  id, priority, enabled,
  match: { field: App|WindowTitle|Url|DocumentPath|Repo,
           op: Wildcard|Regex|Equals, value, capture_groups },
  assign: { kind: Media|Social|Communication|Task|Reference|Tool|Deep,
            productivity: Productive|Neutral|Unproductive,
            project: Option<WorkId>, label: Option<String> },
  apply: Retroactive | Forward
}
```

Non-negotiable properties, each taken from a verified mechanism:

1. **Ordered, with first-match-wins available** — Timing's `projectRuleOrder` **[M]**, ManicTime's "skip the rest" **[D]**.
2. **Retroactive re-evaluation** over the canonical log — ManicTime **[D]**. Evo's log-is-truth design makes this nearly free **[V]**, and it is the highest-value property here.
3. **Absorb + gap-fill** — ManicTime **[D]**: a short anomalous interval (a 90-second YouTube page between two sessions in the editor) inherits its neighbours when neighbors agree, or splits when they don't. **This alone eliminates the hiking-video work.**
4. **A separate productivity/media axis**, never merged into work membership — ManicTime's Automatic Categorization, Dayflow's `distractions[]` **[D][V]**.
5. **A user-editable rule/prompt artifact** — Dayflow's `ActivityCardPromptOverrides` **[V]**.

### 11.4 Module 4 — Work as an objective with explicit membership and lifecycle

Replace "cluster of co-occurring loci" with:

```
Work {
  id, title, summary,
  members: [ Member { locus, source: Declared|Rule|Inferred,
                      state: Active|Completed|Parked|Superseded,
                      pinned: bool, evidence: [...confidence...] } ],
  sessions: [ SessionSnapshot ],     // what was actually open, when
  open_loops: [ Loop { text, origin, created_at, done_at } ],
  last_worked_at, next_action: Option<String>,
  status: Active | Waiting | Blocked | Done | Archived(reason)
}
```

- **Membership is declared or rule-derived first, inferred second.** This is Rambox/Arc's container model plus Rize's most-specific-rule **[D]**; the existing affinity graph survives as the *inference fallback* and as a **suggestion engine** ("these three things are usually open with this work — add them?") rather than the authority.
- **`Member.state` is the answer to "which 2–3 of the 6".** `Completed` and `Superseded` members are not reopened. This state can come from: an explicit user mark, a task-system status (Rize's Linear/ClickUp/Asana signal **[D]**), a merge/close/archive event in the captured evidence, **or** non-return across N sessions while sibling members keep being used. The last is the only inferred one and must be shown as inferred.
- **`open_loops` is what generates urgency** (§11.7) and what makes a title like *"Finish the Cedar auth migration"* possible.

### 11.5 Module 5 — Naming service with an evidence contract

Take Dayflow's contract verbatim in spirit **[V]**, and make the *input* checkable so a bad name is visible rather than plausible:

```
1. Build IntervalEvidence for the work's sittings:
     activities: [ {activity, minutes, app/site} ]   // MUST sum to the interval
   Rules: count FOREGROUND interaction only (never background churn);
          combine recurring visits to the same actual task;
          keep different subjects separate even when they share an app.
2. selected = activity with the most supported time.
3. title = LLM(prompt(evidence, selected, familiarSubject, neighbouring titles),
              constraints: sentence case, activity verb first,
              "answer: what did I spend this time doing?",
              "prefer a recognizable approach over implementation terms or the platform",
              "understandable tomorrow")
4. If confidence < threshold or the interval is Leisure → do NOT create a work.
5. Never fall back to a resource name. Fall back to "Untitled — <date> <duration>".
6. Offer the name for correction; store correction as (a) a rule if it recurs, (b) a
   prompt override, (c) training evidence for the next naming pass.
```

Step 5 is a one-line change to `choose_title()` that removes the worst symptom immediately, and step 4 is the one that stops §4.2 at the gate.

### 11.6 Module 6 — Activation spec (Bunch semantics) + honest restore

Make restoration a **saved, inspectable plan** rather than a recomputation:

```
ActivationPlan {
  work_id, created_from: SessionSnapshotId,
  steps: [ Step {
     item: Locus,
     args: [..],                  // Bunch: initWithLaunchPath:arguments:
     delay_ms: u64,               // Bunch: delay
     wait_for: Option<Evidence>,  // Bunch: waitingSnippet
     open_at: Option<Time>,       // Bunch: scheduleOpenAt:
     force: bool,                 // Bunch: forceOpenClose
     optional: bool,              // Bunch: ignoreInSingleBunchMode
     member_state: Active|Completed
  } ]
}
```

Restore = run the plan for the chosen `SessionSnapshot`, skipping `Completed` members and skipping `optional` in reduced mode. **Offer the user the snapshot picker** (Workona's session history **[D]**) instead of silently inferring a subset. This converts your question *"which apps are needed to get back into this work?"* from an inference problem into a **recorded, correctable fact**.

**The snapshot schema is not hypothetical — Session Buddy's is extracted above (§10.3), and it should be copied rather than reinvented:**

```
SessionSnapshot {
  format:      <version>                  // versioned, validated on import
  windows: [ {
      folders: [ { tabs: [ {url,title,favIconUrl,pinned}, … ] } ],
      tabs:    [ {url,title,favIconUrl,pinned}, … ],     // ungrouped
  } ],
  links:       [ … ],                     // saved resources, not live tabs
  kind:        Saved | Previous,           // the crucial split
}
```

Three details from that model are load-bearing, and all three are missing from Evo today (§8):

1. **`kind: Saved | Previous`.** The product keeps *what you deliberately kept* and *what was merely last seen* in **separate id-maps** (`nxs.v3-id-map-saved-sessions`, `nxs.v3-id-map-previous-sessions` **[M]**). Evo has one derived set and no such distinction — which is why a stale work and a live one look the same on Home (§7.2).
2. **Window scope, not a flat URL list.** Layout is part of the snapshot, so restore does not have to guess how many windows you meant.
3. **A versioned export envelope with an explicit schema path list and a named parse error.** Snapshots can be validated, migrated and shared.

And the snapshot should be **captured, not computed**, using Bunch's own primitive: `newBunchWithOpenApps` / `copyBunchWithOpenApps` **[M]** (§10.9) — take the state you were *actually* productive in, at the moment you leave the work, and reuse it.

### 11.7 Module 7 — Urgency: the ranking Evo refuses to do

Evo's current design forbids a score **[V]** because a score invites fabrication. The reference apps rank without fabricating because they rank on **declared** things. Rank on declared/observed facts only, and show the reason:

| Signal | Source | Kind |
|---|---|---|
| Has an unfinished `open_loop` / `next_action` | §11.4 | declared |
| Deadline / calendar proximity | Rize **[D]** | declared |
| Was the most recent work *committed to* (commit, save, task update) | `OBS-COMMIT-MADE`, task state | observed |
| Interrupted vs completed last session | session last-event | observed |
| Waiting on someone else | user mark | declared |
| Time since last worked, decayed | timestamps | observed |
| User pin | explicit | declared |

**Never** rank by raw attention share or by generic recency of machine events (§7.2). Each row must render its reason in the UI ("unfinished task · touched 40 min ago"), so a wrong ranking is visible rather than mysterious.

### 11.8 Module 8 — The feedback loop (currently 7 events in 52,679)

This is the highest-leverage change per line of code. Add the negative-feedback paths the reference apps all have:

- **Dismiss** a work or a suggestion, recorded as evidence (Toggl **[D]**).
- **"Not work"** on an interval, which commits a classification rule and **re-runs retroactively** (ManicTime **[D]**).
- **Rename** a work, which updates its title and can propose a rule (Dayflow `onTitleChange` **[V]**).
- **Mark member complete / superseded**, which is what makes §11.4's subset correct.
- **Ask when uncertain** — keep `retrieval_margin = 0.80` **[V]**; it is one of the best decisions in the current code.

---

## 12. Implementation plan in the Evo repo

Phases are ordered so each one ships an observable improvement, and **every phase is testable offline against the existing 42 MB log**.

| Phase | Change | Crate(s) | Why first |
|---|---|---|---|
| **0** | **Replay harness**: `evo-replay` that feeds the canonical log through the inference pipeline and prints metrics (§13) + a golden set of ~30 works labelled by you. | new `evo-replay` | Without this, every later claim is unfalsifiable. |
| **1** | **Stop the bleed (small, high-impact).** `choose_title()`: never return a bare resource name; add `Untitled — <date> <duration>`. Gate presentation on a minimum *distinct* sitting count and on the interval not being classified Leisure. | `evo-engagement` | Removes the URL-named-work and single-video-work symptoms without touching the architecture. |
| **2** | **Capture the missing signals**: screen lock/idle, media playback state, calendar, and a `source` for foreground vs background. | `evo-*` capture | Everything in §11 depends on this; it is the true root cause. |
| **3** | **Classification layer** (§11.3): ordered rules, retroactive re-evaluation, absorb/gap-fill, separate media/productivity axis. | new `evo-classify` | Delivers the "not work" state and kills incidental artifacts. |
| **4** | **Ranking** (§11.7): replace the boolean partition with a reasoned, declared-signal score, rendered with its reason. | `evo-desktop` | Immediately fixes the "no urgency" complaint, using Phase 3's classification for exclusion. |
| **5** | **Work entity + lifecycle + open loops** (§11.4), with declared membership; demote the affinity graph to a suggestion engine. | `evo-engagement` | Fixes grouping and the "which apps still needed" question. |
| **6** | **Naming service** (§11.5) with an evidence contract and an editable prompt. | `evo-daemon` (naming path already exists — `crates/evo-daemon/src/naming.rs`) | Big visible win; makes the app answer "what did I do?" |
| **7** | **Activation plans + session snapshots + picker** (§11.6). | `evo-desktop` | Fixes restore fidelity and makes the subset correctable. |
| **8** | **Feedback loop** (§11.8), measured by design-event count (today: 7 in 52,679). | all | Compounding: every later model improves without a retrain. |

**Preserve, do not rewrite:** the append-only canonical log **[V]**, `CanonicalIndex` incremental refresh **[V]**, `Remembered` standing's never-delete guarantee **[V]**, `retrieval_margin` ask-don't-guess **[V]**, and `attributable_change_locality`'s machine-vs-human test **[V]** — Phase 1 depends on that last one.

---

## 13. Offline validation harness — how to prove each phase worked

The log-is-truth invariant means every change is measurable *before* it ships. **Acceptance criteria, all computed by replaying the existing 42 MB log:**

| # | Metric | Definition | Current (baseline) | Target |
|---|---|---|---|---|
| 1 | **Incidental-work rate** | works presented whose interval is classified Leisure/Social **or** whose only evidence is <2 distinct sittings, as a share of works presented | to be measured in Phase 0; expected high (the YouTube/video class) | ≤ 5% |
| 2 | **Title-identifier rate** | presented works whose title matches a URL, a bare domain, or a generic surface name (`Login`, `WhatsApp`, `Search`) | **measured as a known defect in Evo's own model notes**; recompute in Phase 0 | 0% |
| 3 | **Title readability (blind)** | share of works whose title names an activity, judged by 2 humans on a 30-work sample, without seeing the evidence | — | ≥ 80% |
| 4 | **Grouping agreement** | ARI/NMI of inferred groups vs your hand-labelled 30-work set, over the same interval | — | ≥ 0.70 |
| 5 | **Membership stability** | Jaccard overlap of a work's presented members across two consecutive restores of the same work, with no new evidence between | recomputed each restore (unstable by construction, §5) | = 1.00 using a snapshot |
| 6 | **Restore precision** | share of auto-opened items the user keeps open after 5 min, over ~20 restores | `max_primary=3` with no completion model | ≥ 80% |
| 7 | **Ordering quality** | in the ordered Home list, rank of the work the user actually resumes next (mean reciprocal rank) | boolean partition, degenerates to canonical order (§7.2) | MRR ≥ 0.6 |
| 8 | **Design-event rate** | user corrections (dismiss / not-work / rename / complete) per 1,000 observations | **7 events per 52,679 ≈ 0.13** | ≥ 5 per 1,000 |

Metrics 1, 2, 5 and 7 are computable **today** from the existing log; measure them in Phase 0 *before* changing anything, so the improvement is a number and not an impression.

---

## 14. Threshold table: current → proposed

Nothing in Evo's existing parameter set is *wrong* — each has a stated reason **[V]**. They are, however, all answering the co-occurrence question. Proposed additions are new evidence axes, not re-tunings of these.

| Parameter | Current | Proposed | Reason for the change |
|---|---|---|---|
| `presence_horizon` | 20 min | **keep 20 min** | Correct as written. |
| `max_interval_attention` | 10 min | **keep 10 min**, but bound by real idle/lock evidence | Currently inferred from silence because idle is not captured. |
| `min_co_episodes` | 2 | **keep 2** | Correct. |
| `affinity_floor` | 0.20 | **keep**, demote to suggestion-only | Should propose membership, not decide it (§11.4). |
| `cohesion_floor` | 0.28 | **keep**, demote as above | Keep as a *hub-and-spoke detector* for rule suggestions. |
| `reciprocal_rank` | 2 | **keep** | Genuinely good protection (verified rationale **[V]**). |
| `internal_cohesion` | 0.50 | **keep, plus** a `distinct_sitting_count ≥ 3` floor for presentation | A single video clears 0.50 trivially; let it be *remembered* but not *presented*. |
| `sustained_sitting_attention` | 2 min | **keep 2 min**, but measured on **foreground** intervals only | 43% of the log is background file churn (§2.1). |
| `engaged_attention` | 60 s | **keep 60 s** | Correct. |
| `min_revisits` | 2 | **keep 2**, but a revisit must be a *distinct sitting*, not two events in one minute | "Return" should mean returning later (§4.2 step 6). |
| `max_primary` | 3 | **keep 3** as the reduced-mode default; let the **activation plan** carry per-item `optional` | A cognitive cap cannot express completion (§5). |
| `attributable_change_locality` | 0.50 | **promote to a capture-time split** (actor: Machine vs Human) | It is the right test; it should filter evidence before inference, not after. |
| `speaks_for_work` | 0.50 | **keep** for naming authority; no longer used to decide membership | Names must stay trustworthy (§6.1 rationale **[V]**). |
| `retrieval_margin` | 0.80 | **keep 0.80** | Ask rather than guess. One of the best decisions in the code. |
| Affinity weights | 0.45/0.25/0.20/0.10 | **keep** for the suggestion layer | Reasonable. Not the authority any more. |
| **NEW** `media_background_ratio` | — | media with no co-occurring foreground activity can never found a work | Direct fix for the hiking video. |
| **NEW** `leisure_founded_work` | — | forbidden; Leisure intervals are `Remembered`, never `Work` | The missing negative state (§2.2). |
| **NEW** `rule_priority` + `first_match_wins` | — | ordered, retroactive | Timing **[M]**, ManicTime **[D]**. |
| **NEW** `absorb_max_gap` | — | small anomalous interval inherits agreeing neighbours; **stored per-rule and user-configured** | ManicTime's `AutoFillInSeconds` + `AutoTagTimelineAbsorb` **[M]**; kills incidental artifacts. |
| **NEW** `focus_session` | — | ≥15 min with ≥75% activity in focus-configured apps/sites, **or** a calendar keyword | Rize **[D]**; better-formed than a single attention threshold. |
| **NEW** `inference_deferral` | — | 2 min before spending model inference on an unclassified interval | Rize **[D]**; a real cost/quality control. |

---

## Appendix A — Raw evidence index

| File | What it is |
|---|---|
| `evidence/rea-native/` | **REA** `inspect-plist` (ManicTime, Bunch, Timing), `inspect-macho` (Dia), `inspect-native-dispatch-metadata` (Bunch: 140 classes / 985 impls; ManicTime: empty, it is .NET) |
| `evidence/dotnet/` | **REA** `inspect-managed-artifact` + `inspect-managed-members` output for `Finkit.ManicTime.Shared.dll` (208 MB), `ManicTime.Client.Tracker.dll` (57 MB), `ManicTime.dll` (9.4 MB), plus `mt_extract.txt` (846 matched types, 2,143 members) |
| `evidence/rea-js/` | **REA** `analyze-javascript-application` attempts: Workona **succeeded** (138 MB envelope), Granola/Toggl/Rambox refused with `artifact_integrity_mismatch`, Session Buddy timed out |
| `evidence/asar/{Granola,Toggl,Rambox}/` | Text payloads extracted from each `app.asar` (Granola 884 files / 29.5 MB, Toggl 7,858 / 68 MB, Rambox 61 / 8.5 MB) — verified: 146 sampled JSON files parsed, 0 bad |
| `evidence/js/extract_asar.py` | The ASAR reader written for this pass (handles asar v3 string offsets, link/unpacked entries) |
| `evidence/js/extract_context.py` | Single-pass keyword context extractor used to mine the bundles |
| `evidence/js/*.txt` | Per-app extracted string evidence (Granola main, Toggl autotracker, Rize strings, Session Buddy sessions, Rambox) |
| `evidence/*_Info.plist.txt`, `*_macho_symbols.txt`, `*_main_strings.txt` | First-pass bundle metadata / Mach-O symbols / `strings` (the empty `strings` results for Electron apps are what led to the ASAR extraction) |
| `evidence/MECHANISMS-SUMMARY.txt` | Keyword-count matrix per app |
| `~/Library/Application Support/evo/storage/observation.log` | Evo's canonical log: 42,603,369 bytes / 883,062 lines / **52,679 records / 9 schemas**, hex-encoded values (the source of §2.1) |
| `~/Library/Application Support/evo/storage/ledger.db` (11.2 MB) | Derived store; not analysed in this pass **[?]** |
| `~/Library/Application Support/evo/storage/artifact.log` (2.4 MB) | Derived artifact log; not analysed in this pass **[?]** |

## Appendix B — Commands used (reproducible)

```bash
# Evo capture inventory (§2.1). NOTE: values are hex-encoded, so a literal grep returns 0.
python3 - <<'PY'
import re, collections, os
d = open(os.path.expanduser("~/Library/Application Support/evo/storage/observation.log"),
         errors="replace").read()
hx = lambda x: bytes.fromhex(x).decode("utf-8", "replace")
for field in ("schema_name_hex", "source_hex", "fact_name_hex"):
    c = collections.Counter(hx(m) for m in re.findall(field + r"=([0-9a-f]+)", d))
    print(field, "total:", sum(c.values()))
    for k, v in c.most_common(12): print(f"  {v:7d}  {k}")
PY

# Evo inference parameters and naming/ordering (§3, §6, §7)
#   read verbatim:
#   crates/evo-engagement/src/params.rs            (all 14 thresholds + reasons)
#   crates/evo-engagement/src/engagement.rs        (choose_title, participant roles)
#   crates/evo-desktop/src/state.rs                (order_home_workspaces, display_state_from_index)

# ---- REA CLI (4.0.1) — the primary tool for shipped binaries (§10.0) ----

# Native macOS: bundle metadata, Mach-O, and ObjC/Swift dispatch metadata
rea inspect-plist                        /Applications/Bunch.app    --format md
rea inspect-macho                        /Applications/Dia.app      --format md
rea inspect-native-dispatch-metadata     /Applications/Bunch.app    --format json   # 140 classes / 985 impls
#   -> refuses targets whose Mach-O exceeds 64 MiB (Dia's dispatch read failed for this reason)

# Managed .NET: ManicTime's macOS client ships MonoBundle/*.dll, so no Windows build is needed
rea inspect-managed-artifact "/Applications/ManicTime.app/Contents/MonoBundle/Finkit.ManicTime.Shared.dll" --format md
rea inspect-managed-members  "/Applications/ManicTime.app/Contents/MonoBundle/Finkit.ManicTime.Shared.dll" --format json
#   NOTE: --format md CRASHES on large member tables (incur RangeError: Maximum call stack size
#   exceeded). Use --format json and mine it with a single streaming pass, not repeated greps.

# JavaScript/Electron
rea analyze-javascript-application <asar-or-dir> [--artifact-format auto|asar|directory] --format json
#   - Works, but the envelope is huge (Workona: 138 MB) — always redirect to a file.
#   - FAILS CLOSED with artifact_integrity_mismatch when the app uses asarUnpack, because the
#     sha256 declared in the asar header disagrees with the unpacked native module on disk.
#     Workaround used here: extract the text payloads, then point REA at the extracted directory.

# ---- What I ran instead when REA refused ----
python3 evidence/js/extract_asar.py   /Applications/Granola.app/Contents/Resources/app.asar  <dest>
python3 evidence/js/extract_context.py <file-or-dir> "term1,term2" <max-samples>
#   ASAR layout: u32@0=4, u32@4=header-pickle payload, u32@8=aligned JSON len,
#   u32@12=exact JSON len, JSON at 16, payload data at 8+u32@4, offsets are decimal STRINGS.
```

## Appendix C — Sources

**Vendor documentation** (all retrieved 2026-10-15):
- ManicTime — [Autotagging](https://docs.manictime.com/win-client/autotagging) · [Searching](https://docs.manictime.com/win-client/searching) · [Tagging](https://docs.manictime.com/win-client/tagging) · [Tracking](https://docs.manictime.com/win-client/tracking) · [Automatic Categorization](https://blog.manictime.com/articles/dec-2024/automatic-categorization/) · [DB schema](https://docs.manictime.com/server/on-premise-installation/connect-to-database/getting-data-from-database)
- Workona — [Tab Manager](https://workona.com/help/tab-manager/) · [Spaces](https://workona.com/help/spaces/) · [Product](https://workona.com/product/)
- Rize — [How Categorization Works](https://docs.rize.io/categories-and-tracking-rules/how-categorization-works) · [App & Website Rules](https://docs.rize.io/labels-and-app-rules/app-and-website-rules) · [AI-Powered Features](https://docs.rize.io/automatic-tracking/ai-features) · [Time Entry Suggestions](https://docs.rize.io/automatic-tracking/time-entry-suggestions-settings) · [Focus Sessions](https://docs.rize.io/focus/create-focus-sessions) · [Meeting Sessions](https://docs.rize.io/meetings/create-meeting-sessions)
- Toggl Track — [iOS timer](https://support.toggl.com/en-us/article/toggl-track-timer-for-ios-8ew1yp/) · [Windows desktop](https://support.toggl.com/en-us/article/toggl-track-desktop-app-for-windows-5w1y5/) · [API v8 time entries](https://github.com/toggl/toggl_api_docs/blob/master/chapters/time_entries.md) · [API v9 reference](https://engineering.toggl.com/docs/track/api/time_entries/)
- Qbserve / RescueTime (comparison only — see note below) — [Qbserve tracking productivity](https://qotoqot.com/qbserve/articles/tracking-productivity/) · [RescueTime categories](https://help.rescuetime.com/article/74-changing-categories-and-productivity-scores)

**Local source** (authoritative, read directly): Dayflow (`~/Downloads/Dayflow-main`, MIT) — `Core/AI/LLMTypes.swift`, `Core/AI/ClaudePromptDefaults.swift`, `Core/AI/ActivityCardPromptOverrides.swift`, `Views/UI/MainView/ActivityCard.swift`; Evo (`~/Desktop/Evo-Evolution.nosync`).

**Note on Qbserve / RescueTime:** worth one paragraph because it is the cleanest statement of the honest position on media. Qbserve checks the frontmost window via macOS accessibility and matches against a local database of known sites/apps, and it **logs YouTube videos and Reddit subs separately rather than inferring whether they are work** — its own docs offer a URL-path rule for a work-related section and concede that mixed-use cases cannot be resolved by rules. RescueTime's documented alternative is an explicit per-activity productivity level plus an **ignore-forever** action that also deletes historical time. Net lesson for Evo: **classification of media is per site/sub-activity and user-adjustable; nobody credible claims to infer it from playback alone.** So §11.3's rule + correction design is the right shape, and the YouTube fix should be *"classified Leisure by default, correctable per video/URL"* rather than a smarter guess.

---

## Appendix D — Open questions (do not mistake these for findings)

1. **[?] Which model does the shipped app implement?** The installed Evo is v0.1.0 (built Oct 14) while the source working tree is newer. If the binary predates the current `evo-engagement` module, the "previous Formation model gated on *witnessed twice*" that the parameter docs describe as replaced **[V]** may be what you are actually looking at — and that model's failure mode is *exactly* your complaint #4 ("**Anything a computer touches twice** satisfies it"). **Confirm before implementing Phase 1**, because it may change the diagnosis from "the new model still has these gaps" to "the new model was never shipped".
2. **[?]** ManicTime's Absorb window and Auto-Fill-Gaps **default** are still not known. **Resolved:** the window is not a constant at all — it is a stored per-autotag field, `AutoFillInSeconds` **[M]**, which is why no default appears in the docs. Its shipped default value is unread; §14's proposed value is inferred from documented behaviour.
3. **[?]** The **defaults and numeric thresholds** inside the Electron apps remain unread. What *was* extracted: identification, process structure, data-model keys, IPC channel names and UI copy (§10.3, §10.6, §10.7, §10.8). Their internal constants would need the running app (DevTools on the renderer, or a debugger) rather than static extraction.
4. **[?]** Granola's **title generation is server-side** — this is now my inference from a bounded negative search of the client, not a verified fact, and the client-side rename flow was not located in the minified renderer chunks. So Granola's naming mechanism is still the least-evidenced claim in §10, and it is **not** load-bearing for the design: Dayflow's prompt **[V]** is the naming specification this document relies on.
5. **[?]** **Workona's internal restore heuristic is still unread.** REA's JS analysis of the extension *succeeded* (`ev_89c4def2…`), and Session Buddy's model was extracted in full from its shipped bundle (§10.3), but both bundles are minified, so per-property semantics beyond the literals quoted are not recoverable statically. The docs' claim — *opening a space restores its latest session, and any subset choice is manual* **[D]** — remains the operative finding.
8. **[?]** ManicTime's `ApplyTo` (4 values) and `AutoTagType` (5 values) enum **member names** were not resolved — only their field counts. The names All/Selected are from docs **[D]**; the rest are unknown.
9. **[?]** `Session Buddy`'s `history` path appears in its export schema (`$.history.*`) alongside `sessions` and `collections`, so the product likely keeps a third, distinct history record. Not pursued.
6. **[?]** Two derived stores sit beside the canonical log and were **not** analysed: `ledger.db` (11.2 MB) and `artifact.log` (2.4 MB). The ledger may already contain a work/membership model that differs from what `evo-engagement` derives — worth reading before Phase 5, because it may change what needs migrating.
7. **[?]** `OBS-INPUT-ACTIVITY` bundles `Keys`/`Clicks`/`Scrolls` into a *single* record with `fact_count=1`, so per-channel input intensity is not separable. If §11.1 wants input intensity as an attention signal, the capture schema has to change.
