# UI-REVIEW-0001 — Evo Desktop Shell: Current-State Review

**Status:** Review only. No code changed, no document amended, no feature proposed.
**Scope:** `crates/evo-desktop` — UI information architecture, visual design, egui implementation.
**Out of scope (treated as frozen):** product model, architecture, backend contracts, RFCs, IS documents.
**Companion document:** `UI-IA-0002.md` (proposed information architecture and screen-by-screen design).
**Date:** 2026-08-16

---

## 0. How to read this document

**Deliverable map.** The brief asked for sixteen things. They are split across two documents:

| # | Deliverable | Where |
|---|---|---|
| 1 | Understanding of Evo | This doc, §1 |
| 2 | Current UI architecture | This doc, §2 |
| 3 | Complete screen inventory | This doc, §3 |
| 4 | Current UI problems | This doc, §5 + Appendix A |
| 5 | Why it feels ugly / generic / SaaS-like | This doc, §6 |
| 6 | Proposed information architecture | `UI-IA-0002.md` §2–3 |
| 7 | Proposed screen-by-screen design | `UI-IA-0002.md` §7 |
| 8 | Navigation model | `UI-IA-0002.md` §4 |
| 9 | Interaction model | `UI-IA-0002.md` §5 |
| 10 | Visual system | `UI-IA-0002.md` §6 |
| 11 | What should remain from V4 | `UI-IA-0002.md` §8 |
| 12 | What should change from V4 | `UI-IA-0002.md` §9 |
| 13 | Which problems are egui problems | This doc, §7 |
| 14 | Which files would need modification | This doc, §8 |
| 15 | Phased implementation plan | `UI-IA-0002.md` §10 |
| 16 | What I will explicitly NOT change | This doc, §9 |

Review questions **A–L** are answered in §4–§7, each answer indexing into the defect register in **Appendix A**.

**Epistemic conventions.** This review holds itself to the product's own standard (Law V, Law VI: silence is preferable to fabrication).

- **[verified]** — read directly in source this session; line references are as-of-read.
- **[inferred]** — follows necessarily from verified code, but the consequence was reasoned, not observed.
- **[needs visual confirmation]** — a claim about rendered pixels that cannot be settled from source alone. I have not run the binary. Anything about actual clipping, actual contrast, or actual perceived crowding is marked this way and must be confirmed on a real screen before it is treated as fact.

**Defect IDs.** Appendix A numbers every finding. Tags: **[B]** correctness bug, **[L]** information-architecture cause, **[K]** egui/implementation cause, **[V]** visual-system cause, **[H]** honesty/semantics cause, **[D]** dead code or drift. Tags are the *cause*, which is the distinction the brief asked for in questions K and L.

**What I read.** `CONSTITUTION.md`; `PRODUCT.md`; `ARCHITECTURE.md`; `ARCHITECTURAL_LAWS.md`; `COGNITIVE_MODEL.md`; RFC-0010 and RFC-0013 in full, and RFC-0001/0002/0003/0006/0011/0012 via delegated read; IS-0001/0010/0011/0012/0014/0019/0020/0021 via delegated read; `evo-desktop/src/{app.rs, theme.rs, main.rs}` in full; `state.rs` and `daemon.rs` substantially; the restoration, execution, daemon, capture and workspace crates' public types; `EVO_SPATIAL_UI_V4.md` and `EVO_VISUAL_DESIGN_DIRECTION_V3.md` in full; `EVO_SPATIAL_PROTOTYPE_V4.html` structurally.

**Authority gap: closed.** The first draft of this review was written without RFC-0010, and said so. I have since read it in full (635 lines). No conclusion changed; three were sharpened, and they are recorded here rather than buried, because two of them affect the proposal in `UI-IA-0002.md`.

*First,* RFC-0010's Prohibited Evidence Sources list is longer and stricter than RFC-0013's restatement of it: observation order, recency, frequency, window focus, window titles, application identity, URL appearance alone, file appearance alone, artifact co-membership, snapshot ordering, artifact importance, artifact confidence, artifact supersession, workspace membership, timestamps used as semantic evidence, runtime state, current OS state, current UI state, LLM output, user-intent prediction, heuristic guesses. Every conclusion in this review was already more conservative than that list requires. Note in particular that *window titles* and *artifact confidence* are named prohibited sources, which independently confirms rule 2 (titles are presentation) and the refusal to surface confidence scores in the UI.

*Second, and more consequential:* RFC-0010's Open Question 2 asks "whether a witnessed work-state fact may ever be user-directed (a request to continue)", observing that "user requests have no canonical representation in the current architecture" and deferring the question to a later evidence milestone. RFC-0011 and RFC-0013 are that milestone, and they answered it *yes*. That single fact is the cleanest statement of what Evo actually became: continuation evidence in the shipped product is **declared by the user**, not inferred by Evo. It is why the proposed Home hero (`UI-IA-0002.md` §2.3) does not violate the no-ranking prohibition — the hero surfaces the user's own declaration, and RFC-0010's prohibitions govern what *Evo* may treat as evidence, not what the user may state. Had I not read RFC-0010, I would have argued that point from intuition instead of from the contract.

*Third:* the emptiness of `context_chain` and `blockers` (invariant 22, §1.6) is **contractual, not defective.** RFC-0010's §"Interaction with IS-0021" fixes the order — evidence schema frozen, then producer, then IS-0021 amended, then derivation — and states that "until then, IS-0021 §25.5 continues to produce explicit insufficiency." So those fields are empty *by design and by sequence*. This strengthens rather than weakens the recommendation to delete the UI branch that renders them: the branch is not waiting on a bug fix, it is waiting on three unstarted milestones, and a UI built speculatively around it is a UI built on a guess. I have adjusted invariant 22's wording accordingly.

---

## 1. Understanding of Evo (Deliverable 1)

### 1.1 The product, stated plainly

Evo answers one question: **"what was I doing, and can I have it back?"**

It answers it without asking the person to have prepared anything. No project setup, no tagging, no manual save-state. It watches, locally, what a person actually touches — which window had focus, which file was saved, which URL was visited, which commit was made — and appends those observations to an immutable log. When the person comes back, Evo replays that log and reopens the things they were working in.

The primary action is **Continue**. `ARCHITECTURE.md` §1 is unambiguous that Resume is merely its implementation and that "today, restoration is the entire product." Everything else in the repository is scaffolding for that one press.

There is exactly one product metric: **Time To Productive** — wall-clock time from the Continue press to the primary artifact being open and in the foreground. This matters for design because it makes almost every conventional "engagement" instinct actively wrong. A screen the user reads is a screen that increased TTP. The best possible Evo session is one where the person looks at the window for under a second and presses one button.

### 1.2 The one structural idea a designer must internalise

State is never stored. State is *derived*, by replaying an interpretation function over the observation log, scoped to a window of resources called a Workspace (`ARCHITECTURE.md` §2).

The design consequence is severe and non-obvious: **there is no place to put UI state.** No pinned workspaces, no sort preference, no favourites, no dismissed banners, no "seen" flags, no renamed titles, no collapsed-section memory — unless it is either recomputed every frame or written as a canonical Observation, and there are only three canonical write paths in the entire product. Every conventional affordance a designer reaches for by reflex is either free (derivable) or architecturally prohibited (would require new canonical state). There is no middle ground.

### 1.3 The three-way partition that the UI exists to express

This is the single most important thing in the review. Almost every structural defect in §5 is a failure to express this cleanly.

There are three different sets of resources, and users conflate them constantly:

```
1. WORKSPACE MEMBERSHIP        durable · historical · grows forever
   "everything Evo ever witnessed in this body of work"
   → derived from the observation log
   → NEVER a restore target on its own (Rule 15)

2. CONTINUATION SURFACE        declared · current · user-owned
   "what I am currently continuing across"
   → canonical, written by the user, ≥2 subjects (RFC-0013)
   → latest valid declaration supersedes
   → this is the ONLY authority for what "current" means

3. RESTORATION SELECTION       derived · execution-facing · now
   "of what I declared, what can actually be reopened on this machine"
   → derived from (2), partitioned into restore-worthy / unavailable / historical
   → recomputed at execution time, never trusted from the past
```

Read downward, each layer narrows the one above it. Read upward, no layer may leak authority into the one above it. Membership never becomes a target. Relatedness (`OBS-WORK-GROUPED`) never becomes continuation — RFC-0013 states the separation as *"Same Artifact ≠ Related Work ≠ Currently Relevant ≠ Restore Together."* And nothing derived at layer 3 ever writes back into layer 2.

A UI that renders these three sets with the same visual weight, in the same list, with the same affordance, has destroyed the product's central distinction. §5.2 shows that this is close to what the current Workspace Detail screen does.

### 1.4 The epistemic contract

Evo's differentiator is not that it restores work. It is that **it never lies about what it knows.** This is codified: Law V preserves uncertainty, Law VI prefers under-interpretation ("silence is preferable to fabrication"), Law IX gives the user ownership of judgment, Law XI requires every decision to be explainable.

In practice:

- **Partial restoration is a normal outcome, not an error** (`ARCHITECTURE.md` §9). Two of five things opening is a success that reports three honest failures. The UI must have no concept of a failed Continue.
- **Preflight is four-way** — READY / UNAVAILABLE / AMBIGUOUS / UNSUPPORTED — and ambiguity must stay ambiguous. There is no "best guess."
- **Recency is not authority.** Repeated across the contracts. This bans, by construction, the most common shape in this product category: a most-recently-used list. Evo may not sort by recency, may not label anything "recent," and may not let position imply priority.
- **Application identity is not work identity.** `MacOSSignal::ApplicationActivated` carries a `bundle_identifier: Option<String>` (`evo-capture/src/adapters/macos.rs:70-71`) and `normalize` returns `Ok(None)` for it (`:152`) — the signal is witnessed and then deliberately dropped, so application identity never becomes an Observation at all. There are no app icons, no favicons, no thumbnails, and there never will be under the current contracts. The UI is typographic because the data is textual.
- **If the backend does not know, the UI must not appear to know.** No confidence percentages, no match scores, no "probably."

### 1.5 What the UI is therefore for

Four questions, one per level, and nothing else:

| Level | The user's question | Evo's job |
|---|---|---|
| **Home** | What work can I continue? | List bodies of work. Rank nothing. |
| **Workspace** | What is this body of work? | Name it, and say where it continues from. |
| **Continuation** | What does Evo think I'm continuing? | Show it, and let the user correct it. |
| **Continue** | What can actually be reopened? | Attempt, then report honestly. |

And a hard constraint on vocabulary: the user must never encounter `WorkspaceId`, `ArtifactId`, `RestorationSelection`, `PreflightSource`, "designation," "continuation surface," or "canonical evidence." The UI exposes the *consequences* of those concepts. Every one of those words currently appears, or is directly transliterated, on screen.

### 1.6 Invariants I found that are not in the brief's list of twenty

Offered as additions, not corrections.

**21. There is no revoke.** The only canonical writes are `submit_designation`, `submit_continuation_surface`, `submit_grouping`. Nothing un-declares. A continuation surface cannot be cleared, and cannot be reduced below two subjects, because a valid declaration requires ≥2. **[verified]** Design consequence: the UI must never offer "clear," and must treat declaration as consequential rather than casual.

**22. `context_chain` and `blockers` are always empty — by contract, not by defect.** `evo-restoration/src/derivation.rs:531` and `:536` construct them unconditionally empty. **[verified]** RFC-0010 §"Interaction with IS-0021" explains why: the accepted sequence is *freeze the evidence schema → implement the producer → amend IS-0021 §25.4/§25.5 → implement derivation*, and "until then, IS-0021 §25.5 continues to produce explicit insufficiency." Three of those four steps are unstarted. Design consequence: no panel, section, or empty state may be designed around either field, and the current UI's "Blocked by" branch can never render (D-19). It should be deleted rather than kept warm — reviving it is a design conversation that begins when the evidence milestone lands, not before.

**23. `Insufficient` does not mean unusable.** `DerivationOutcome::Insufficient` carrying `resume_point: Some(_)` is the common default state and is fully executable. **[verified]** Design consequence: "insufficient" must never be presented as a failure or a dead end. It is the ordinary condition of a young workspace.

**24. Preflight is a statement about the recent past.** Preflight is recomputed on a 2-second poll, discarded, and then independently recomputed by the execution engine at press time. The green dot the user sees was true up to two seconds ago and carries no promise. **[verified]** Design consequence: preflight may describe, but must never promise. Current copy says resources "will be attempted" (D-24).

**25. Only Accessibility permission is load-bearing.** No other macOS permission gates anything. **[verified]** Design consequence: exactly one permission story, not a permissions system.

---

## 2. Current UI architecture (Deliverable 2)

### 2.1 Files and responsibilities

```
crates/evo-desktop/src/
  main.rs      16 lines    eframe entry. 900×660 default, 680×500 minimum.
                           No with_transparent(true).
  app.rs     2726 lines    EvoApp: all state, all polling, all screens,
                           all layout, all copy. Every screen is a private fn here.
  theme.rs    699 lines    V4 visual system as tokens: colours, spacing, radii,
                           shadows, SF variable-font weights, frame + control helpers.
  state.rs   2266 lines    Derivation-facing helpers: display selection, artifact
                           description, WorkspaceCard, filters, search memo,
                           execution routing. Heavily unit-tested.
  daemon.rs   411 lines    Daemon lifecycle + DaemonStatus + Accessibility deep-link.
  lib.rs                   module wiring.
```

`app.rs` is the whole UI. It is a single 2726-line file holding thirty-odd free functions, one god-struct with ~30 fields, and every string of user-facing copy in the product. This is the root cause of a large share of Appendix A: there is no unit of composition smaller than "a function that draws a card," so nothing can be reasoned about, tested, or restyled in isolation.

### 2.2 The data path

```
Observation log (canonical, on disk)
        │  synchronous full replay
        ▼
CanonicalIndex ──► workspaces[]  subjects{}  kinds{}  outcomes{}  locators{}
        │
        │  per-frame / per-poll derivation in EvoApp
        ▼
outcome: DerivationOutcome        ← for the displayed workspace
selection: RestorationSelection   ← derived from the declared surface
preflight: Vec<PreflightOutcome>  ← real OS probing
home_cards: Vec<WorkspaceCard>    ← presentation projection, one per workspace
        │
        ▼
egui widgets
```

Three things about this path matter for design:

1. **It is synchronous and on the UI thread.** `EvoApp::new` replays the entire log before the first frame; there is consequently no loading state anywhere in the product, because there is no moment at which the UI exists but the data does not. This is fine at small log sizes and becomes a cold-start freeze later (D-31).
2. **Preflight performs real OS work.** `preflight_selection` enumerates windows through the Accessibility API. It runs on the UI thread, on the poll, every two seconds, for the displayed workspace (D-32).
3. **`WorkspaceCard` is honest about itself.** Its doc comment reads *"Transient UI state only — never canonical, never persisted, never a model object (Law XVI)."* This is exactly right and should be preserved and imitated.

### 2.3 The frame loop and cadence

`RELOAD_PERIOD = 2s`. There is no change notification from the daemon, so the UI polls. On each reload the app recomputes derived state and compares a **signature** — `(workspace_id, snapshot_count)` for a workspace, or `("home", workspace_count)` for Home. When the signature changes, it resets `execution_result`, `continuation_draft`, and `continuation_status`.

That reset is the most damaging single behaviour in the shell, and it is worth stating precisely because it looks harmless:

- A user tick-marking checkboxes to correct their continuation loses the draft the moment any observation lands in that workspace. Their edit is destroyed mid-thought by background capture (**B-01**).
- Pressing **Continue** focuses windows. Focusing windows produces `OBS-WINDOW-FOCUS-GAINED` observations. Those observations change `snapshot_count`. Therefore **the execution report deletes itself within about two seconds of appearing** (**B-02**).

B-02 is not a cosmetic bug. The execution report is the product's honesty artifact — the one surface where Evo tells the truth about partial restoration, per `ARCHITECTURE.md` §9. The current implementation guarantees the user cannot finish reading it. Success accelerates its own erasure, because the more windows Evo opens, the faster the report vanishes.

### 2.4 Navigation topology

Two destinations, one modal:

```
First Run (1 → 2 → 3)  ──►  HOME  ◄──────────────►  WORKSPACE DETAIL
                              │   selected=Some       │
                              │   ← All workspaces    │
                              └──────► SETTINGS ◄─────┘  (sheet, either screen)
```

`ShellView::from_selection` derives the view from `Option<WorkspaceId>`. This is clean and should survive the redesign essentially unchanged. Two implementation faults sit on top of it: the back control lives *inside* the scrolling content, so it scrolls away (D-14), and Settings is drawn as an in-flow scope rather than a modal layer, so the content behind it stays interactive (D-11).

Note also that this answer already exists in the codebase and is *computed on every reload before being thrown away.* `state.rs:107` calls `select_workspace_with_designation` to populate `DisplayState::display` — the workspace containing the user's current designation, justified in its own doc comment by RFC-0011 and RFC-0006. Then `app.rs::derive_display_state` recomputes `self.display` from `self.selected` alone (`app.rs:324`) and **no code path in `app.rs` ever reads `DisplayState::display`** (D-38, **[verified]** by grep: the only occurrences in `app.rs` are the field declaration at `:47`, the `None` initialiser at `:150`, and the overwrite at `:324`). So a canonical, non-ranking, architecture-sanctioned answer to "which body of work did you say you were continuing?" is derived, carried across the state boundary, and discarded at the last step. `UI-IA-0002.md` §7.2 builds Home's primary answer on it.

### 2.5 Write paths

Exactly three canonical writes exist in the product, and the UI drives two:

| UI control | Writes | Location |
|---|---|---|
| "Mark to continue" button | `OBS-WORK-DESIGNATED` | `designation_card`, main column |
| "Declare current continuation" | `OBS-CONTINUATION-SURFACE` | `restoration_selection_card`, sidebar |
| *(no UI)* | `OBS-WORK-GROUPED` | — |

Two controls, two write paths, two status lines, two vocabularies, two screen regions — for what the user experiences as **one question**: *"what am I continuing?"* This is the central structural failure and §5.2 treats it at length.

---

## 3. Complete screen inventory (Deliverable 3)

**Thirteen real states exist.** V4 §7 designs twenty-five. The gap is not laziness — several V4 "screens" are the same code path with different data (7.4/7.5 are identical; 7.6/7.7 differ only in list length). But four V4 screens have **no implementation at all**: 7.17 Continue–Executing, 7.19 Continue–Failure as a distinct treatment, 7.25 Correction/Invalid Declaration, and the preflight disclosure interaction in V3 §6.8. And one implemented state — the stale-data error screen — is mis-titled such that it reads as a different screen entirely (D-17).

Fields follow the brief's template. `C·D·T` = Canonical / Derived / Transient.

---

### S-01 · First Run — What Evo Is

- **PURPOSE** — Establish that Evo is a local record of work, before it has any data to show.
- **PRIMARY USER QUESTION** — What is this thing I just opened?
- **PRIMARY ACTION** — Continue to step 2.
- **SECONDARY ACTIONS** — None.
- **DATA SOURCE** — None. Static copy. Gated on absence of `~/Library/Application Support/Evo/first_run_done`.
- **C·D·T** — Transient (step index in `first_run: Option<u8>`); the marker file is local UI state, not canonical.
- **NAVIGATION IN** — App launch with no marker file.
- **NAVIGATION OUT** — Step 2.
- **POTENTIAL UX PROBLEMS** — Step dots are hand-painted at `ui.cursor().left_top() + vec2(4,4)`, radius 3, 8px pitch, with only the current dot filled (D-27): they are unallocated paint, so they do not participate in layout and cannot be reflowed or keyboard-described. No skip; a returning user who deletes the marker file must pass three screens to reach their work.

---

### S-02 · First Run — Privacy

- **PURPOSE** — State that capture is local and network-independent.
- **PRIMARY USER QUESTION** — Where does this data go?
- **PRIMARY ACTION** — Continue to step 3.
- **SECONDARY ACTIONS** — None.
- **DATA SOURCE** — Static copy.
- **C·D·T** — Transient.
- **NAVIGATION IN** — S-01.
- **NAVIGATION OUT** — S-03.
- **POTENTIAL UX PROBLEMS** — No back. The strongest trust claim in the product ("nothing is sent anywhere, and nothing requires the internet to work") is shown once, at the moment of least user investment, and then never again except buried in Settings → About.

---

### S-03 · First Run — Permissions

- **PURPOSE** — Request Accessibility access, the one permission that gates window-focus capture.
- **PRIMARY USER QUESTION** — Why does it need this, and what happens if I say no?
- **PRIMARY ACTION** — Open System Settings.
- **SECONDARY ACTIONS** — Check again; complete first run.
- **DATA SOURCE** — `DaemonStatus` via `daemon.poll()`; `open_accessibility_settings()`.
- **C·D·T** — Derived (permission state).
- **NAVIGATION IN** — S-02.
- **NAVIGATION OUT** — Home.
- **POTENTIAL UX PROBLEMS** — Permission state is classified by `stderr.contains("Accessibility permission is required")` (`daemon.rs` `classify_exit`) — a brittle string match against a subprocess's stderr (D-33). The consequence of declining is stated but the *degraded* mode is not: Evo still captures files, URLs and commits without Accessibility, which is a materially useful product, and the copy does not say so.

---

### S-04 · Home — Empty

- **PURPOSE** — Say honestly that nothing is remembered yet, without implying failure.
- **PRIMARY USER QUESTION** — Is it broken, or is it just new?
- **PRIMARY ACTION** — None. Waiting is the correct behaviour.
- **SECONDARY ACTIONS** — Settings; permission actions if permission is missing.
- **DATA SOURCE** — `workspaces.is_empty()`; `DaemonStatus`.
- **C·D·T** — Derived.
- **NAVIGATION IN** — First-run completion, or launch with an empty log.
- **NAVIGATION OUT** — Settings; implicitly to S-06 when the first workspace appears.
- **POTENTIAL UX PROBLEMS** — `empty_home` has an empty match arm for `DaemonStatus::PartialCapture { .. } | DaemonStatus::PermissionRequired => {}` (D-05). In precisely the state where emptiness might be *caused* by missing permission, the empty screen says nothing about permission. It relies on the banner above it, which is separately positioned inside the scroll area. A user who scrolls, or whose window is short, sees "Evo remembers nothing yet" with no explanation.

---

### S-05 · Home — Storage error / stale data

- **PURPOSE** — Admit that the canonical read failed and that anything visible may be old.
- **PRIMARY USER QUESTION** — Can I trust what I'm looking at?
- **PRIMARY ACTION** — None available.
- **SECONDARY ACTIONS** — Settings.
- **DATA SOURCE** — index load error; previously loaded `workspaces` retained.
- **C·D·T** — Derived + stale transient.
- **NAVIGATION IN** — A failed reload.
- **NAVIGATION OUT** — Recovers silently on a successful reload.
- **POTENTIAL UX PROBLEMS** — `error_screen` is headed `theme::title("Current work", 16.0, …)` above the sentence "Evo could not read its stored state" (D-17). The heading is wrong, at the wrong size, and describes a different screen. Separately, stale cards are dimmed by `let opacity = if stale { 0.6 - 0.2 * index.min(1) as f32 } else { 1.0 };` — the first card renders at 0.6 and *every* other card at 0.4, which encodes no meaning (D-25). All stale data is equally stale.

---

### S-06 · Home — Workspace list

- **PURPOSE** — The product's front door: which bodies of work exist and which can be continued.
- **PRIMARY USER QUESTION** — What work can I continue?
- **PRIMARY ACTION** — Open a workspace.
- **SECONDARY ACTIONS** — Search; filter by kind; filter by period; Settings.
- **DATA SOURCE** — `home_cards: Vec<WorkspaceCard>` built by `state::workspace_card` from `workspaces` + `outcomes` + `subjects` + `kinds`.
- **C·D·T** — Derived (cards) over Canonical (workspaces, outcomes); Transient (query, filters).
- **NAVIGATION IN** — Launch; `← All workspaces` from S-09.
- **NAVIGATION OUT** — S-09; Settings.
- **POTENTIAL UX PROBLEMS** — Three, and they compound. (1) Every card is the same size and weight, so **nothing on the product's front door is more important than anything else** — the user must read all of them (D-06). (2) The card is a click target *and* contains an absolutely-positioned "Open workspace →" button computed from `rect.max` and a laid-out galley, both of which do the identical `*selected = Some(id)` (D-07/D-22): a duplicate primary action, one of which is invisible to layout. (3) The content column is clamped to `CONTENT_WIDTH = 640` while S-09 is full-bleed, so the two halves of the product disagree about how wide Evo is (D-13).

---

### S-07 · Home — Search and filters active

- **PURPOSE** — Let a user with many workspaces narrow the list themselves.
- **PRIMARY USER QUESTION** — Where is the one I'm thinking of?
- **PRIMARY ACTION** — Open a matching workspace.
- **SECONDARY ACTIONS** — Clear query (✕ or Escape); change kind chip; change period chip.
- **DATA SOURCE** — `RetrievalMemo` — lowercased `haystacks: Vec<String>`, `kind_flags: Vec<[bool;4]>`, invalidated by a `Vec<(id, snapshot_count)>` signature. Matching is `haystack.contains(&query)`.
- **C·D·T** — Transient (query, chips) over Derived (memo).
- **NAVIGATION IN** — Typing in S-06.
- **NAVIGATION OUT** — S-06 on clear; S-08 on no match; S-09 on open.
- **POTENTIAL UX PROBLEMS** — Search is a case-insensitive substring test. `evo-retrieval` exists as a documented stub and implements nothing, so this is the entire search product. Correct and honest, but the UI presents it with a search-engine affordance and no statement of scope, so a user who searches for a word inside a document and gets nothing will conclude Evo is broken rather than that Evo indexes resource names (D-18). The period filter judges only `latest.captured_at()`, so "Today" means *the workspace was last witnessed today*, not *it contains work from today* — defensible, undocumented on screen.

---

### S-08 · Home — No matches

- **PURPOSE** — Say honestly that nothing matches, without suggesting alternatives Evo cannot rank.
- **PRIMARY USER QUESTION** — Did I mistype, or is it really not here?
- **PRIMARY ACTION** — Clear or amend the query.
- **SECONDARY ACTIONS** — Reset chips.
- **DATA SOURCE** — Empty filter result.
- **C·D·T** — Transient.
- **NAVIGATION IN** — S-07.
- **NAVIGATION OUT** — S-07.
- **POTENTIAL UX PROBLEMS** — Copy branches on query-vs-chips (`No body of work matches "{}"` / `No body of work matches {} / {}`) but not on both together, so a user filtered to *Commits · Today* who also typed a query cannot tell which constraint eliminated everything. No one-tap escape back to the unfiltered list.

---

### S-09 · Workspace Detail

The largest screen, and the one carrying most of §5. Four regions: header identity, main column (three cards), sidebar (one card), pinned bar.

- **PURPOSE** — Say what this body of work is, where Evo will continue from, and let the user correct that.
- **PRIMARY USER QUESTION** — Where was I, and is Evo right about it?
- **PRIMARY ACTION** — Continue (in the pinned bar, S-10).
- **SECONDARY ACTIONS** — Mark to continue (designation); toggle checkboxes; Declare current continuation; disclose related work; back; Settings.
- **DATA SOURCE** — `display: Workspace`; `outcome: DerivationOutcome`; `selection: RestorationSelection`; `preflight: Vec<PreflightOutcome>`; `designation`; `continuation_draft`; `locators`.
- **C·D·T** — Canonical: workspace, outcome, designation, declared surface. Derived: selection, preflight, card title, `continue_plan_line`. Transient: draft checkbox set, disclosure state (`ui.data` / `insert_temp`), status strings.
- **NAVIGATION IN** — Opening a card on S-06/S-07.
- **NAVIGATION OUT** — `← All workspaces`; Settings; Continue → S-11.
- **POTENTIAL UX PROBLEMS** — The concentration of defects: two competing correction mechanisms (§5.2); identity metadata printed twice on one screen, once in the header line and again in the witnessed grid (D-15); the back control inside the scroll area (D-14); four-to-five visually identical white cards (D-20); up to fifty snapshot rows of audit data rendered as product UI (D-16); the two-column split threshold at 700px against a 680px minimum window (D-28); and a "Blocked by" section that can never appear (D-19).

---

### S-10 · Continue bar — resting

- **PURPOSE** — Keep the primary action permanently reachable and state honestly what it will attempt.
- **PRIMARY USER QUESTION** — What will happen if I press this?
- **PRIMARY ACTION** — Continue this work.
- **SECONDARY ACTIONS** — None.
- **DATA SOURCE** — `continue_plan_line(outcome, selection, locators)`: joins restore-worthy locator actions when a surface is declared, else `ExecutionRequest::from_derivation` + `evo_execution::ordered_targets`.
- **C·D·T** — Derived.
- **NAVIGATION IN** — Present on all of S-09.
- **NAVIGATION OUT** — S-11.
- **POTENTIAL UX PROBLEMS** — The plan label is added with `.truncate()` in the same `ui.horizontal` *before* the right-to-left layout claims space for the button, so the label can consume the row and squeeze the product's only primary action (D-03, **[needs visual confirmation]** — the failure mode is structurally present in the layout order, but whether it clips at 680px must be seen). The bar is hand-painted `rect_filled(rect, 0.0, rgba(255,255,255,242))` with a 1px top stroke and no shadow, while `theme::action_bar_frame()` — the frame a theme unit test asserts about — is never used anywhere (D-21/D-35).

---

### S-11 · Continue bar — expanded execution report

- **PURPOSE** — The honesty surface. Report per-target outcomes, including partial restoration.
- **PRIMARY USER QUESTION** — What actually opened, and why didn't the rest?
- **PRIMARY ACTION** — None; reading is the action.
- **SECONDARY ACTIONS** — Press Continue again.
- **DATA SOURCE** — `ExecutionReport` from `state::run_execution`.
- **C·D·T** — Transient. Deliberately not canonical.
- **NAVIGATION IN** — Pressing Continue.
- **NAVIGATION OUT** — Erases itself (B-02).
- **POTENTIAL UX PROBLEMS** — The worst of the review. (1) **It deletes itself within ~2s** because the windows it opened generate observations that change the signature (B-02). (2) Height is allocated as `64.0 + lines * 24.0` while `execution_line` renders a status line plus a verbatim reason at roughly 40px per attempt, so **the reason text is clipped first** — exactly the partial-restoration explanations `ARCHITECTURE.md` §9 requires (B-03). (3) The expand animation is registered under two different ids from two different `Ui` scopes — `ui.id().with("evo-continue-expand")` in `ui()` and again in `pinned_continue_bar` — so two independent animation clocks drive one visual transition (B-04). (4) Preflight says "UNAVAILABLE" in caps; the report says "unavailable" in lower case, for the same underlying condition (D-23).

---

### S-12 · Settings sheet

- **PURPOSE** — Capture status, storage location, and the local-only claim.
- **PRIMARY USER QUESTION** — Is it running, and where is my data?
- **PRIMARY ACTION** — Done.
- **SECONDARY ACTIONS** — Open System Settings; Check again.
- **DATA SOURCE** — `DaemonStatus`; `storage_root`; static version copy.
- **C·D·T** — Derived + static.
- **NAVIGATION IN** — Header gear, from either screen.
- **NAVIGATION OUT** — Done.
- **POTENTIAL UX PROBLEMS** — Drawn via `ui.scope_builder` in normal flow rather than an `Area`/`Modal`, so the dimmed content behind remains clickable — a user can open a workspace *through* the scrim (D-11). Height is hardcoded `428.0` / `380.0` against variable-length daemon detail strings, so content can overflow a fixed box (D-12, **[needs visual confirmation]**).

---

### S-13 · Capture banners (overlay on S-04/S-06/S-09)

- **PURPOSE** — Disclose that capture is degraded or permission is missing.
- **PRIMARY USER QUESTION** — Is Evo actually watching?
- **PRIMARY ACTION** — Open System Settings.
- **SECONDARY ACTIONS** — Check again.
- **DATA SOURCE** — `DaemonStatus::{PartialCapture, PermissionRequired, Unavailable, Failed}`.
- **C·D·T** — Derived.
- **NAVIGATION IN** — Automatic on status.
- **NAVIGATION OUT** — Disappears when status recovers.
- **POTENTIAL UX PROBLEMS** — Rendered *inside* the scroll area, so the disclosure that Evo is not capturing scrolls out of view (D-14). Not dismissible and not persistent-in-view: the worst combination, because it is both unavoidable and easy to miss. `DisplayState` has no field for capture status, error, or loading (D-04), so the banner is bolted onto the render path rather than being part of the screen's derived state — which is why S-04 forgets about it entirely.

---

### 3.1 What the inventory reveals

- **Four V4 screens are unimplemented:** Continue–Executing (7.17), a distinct failure treatment (7.19), Correction/Invalid Declaration (7.25), and preflight reason disclosure (V3 §6.8). The last matters most: reasons are currently always-visible indented text rather than disclosed on demand, which is a large part of why the screen feels dense (§5.3).
- **There is no loading state and no executing state.** Both are structurally impossible in the current architecture — derivation is synchronous in `new()`, and `run_execution` blocks the frame — so the window is simply unresponsive during execution rather than showing progress (D-31, D-34).
- **There is no rejection state.** RFC-0013 rejects a declaration of fewer than two subjects. The UI has a status line that will print the rejection, but no design for it, and no prior signal that the constraint exists (§5.2, D-10).

---

## 4. What is actually good (Question A; Deliverable 11 in part)

I want to be precise here, because the brief warned me not to assume a design is good because it exists — but the inverse trap is worse. A reviewer who finds nothing good will redesign away the things that took the most judgment to get right. The following are not merely acceptable; they are the product's real assets and **must survive** any redesign. Question **I** — "what should remain exactly as-is" — is answered by this section plus §9.

**A1. The copy is the best part of the product, by a wide margin.** Sentences like *"Evo remembers this body of work, but its record does not show which resource to continue from"* and *"The rest are reported honestly — nothing is attempted, nothing is guessed"* do something very hard: they express epistemic humility without sounding apologetic or technical. `theme::title("Evo remembers nothing yet.")` is a genuinely excellent empty state — it makes newness feel like patience rather than failure. This voice is a moat. I would not change a word of it except where a sentence makes a promise the backend cannot keep (D-24).

**A2. Kind-aware sentence construction instead of a metadata table.** `describe_artifact` produces *"the focused window "X""*, *"the saved file X"*, *"the visited URL X"*. Building a sentence per resource kind, rather than a row with a type column, is the single most product-appropriate decision in the UI. It is why the screen reads as a statement rather than a database view. Keep this mechanism and extend it.

**A3. Two destinations and no more.** Home and Workspace Detail, derived from `Option<WorkspaceId>`. No tabs, no breadcrumbs, no command palette, no nested routes. For a product whose metric is Time To Productive, this restraint is exactly right, and it is the correct skeleton to build on.

**A4. `WorkspaceCard`'s self-awareness.** The doc comment declaring it transient, never canonical, never a model object (Law XVI) is the kind of discipline that keeps a UI layer from quietly becoming a second source of truth. This pattern should be the template for every new presentation type.

**A5. Correct handling of two real traps.** `report.attempts().is_empty()` is checked before rendering, and `execution_result_line` guards emptiness before calling `all_opened()` — because an empty report would otherwise vacuously report total success. Someone thought carefully about vacuous truth. `RestorationSelection::is_empty()` is never relied on for the same reason. These are subtle and correct.

**A6. Real typographic weight.** `theme.rs` registers the SF variable font at `wght` 500/600/700 as separate families, precisely because "egui's `.strong()` only changes color, so genuine weight hierarchy is implemented with dedicated variable-font faces." That is a serious piece of engineering that most egui apps never do, and it is the foundation the visual redesign needs. (The tragedy is that `app.rs` then calls `.strong()` in six places anyway — D-26.)

**A7. Preflight and execution vocabularies pair colour with words.** Never colour alone. V4 §9 requires this and the implementation honours it. The only defect is that the two vocabularies disagree on case (D-23).

**A8. The pinned bar as a concept.** The primary action never scrolls away. Right for the product; keep the concept and rebuild the implementation.

**A9. `state.rs` is genuinely well tested.** Canonical ordering, surface subjects, execution routing, refusal synthesis. The tests encode real invariants. This is why I am confident a UI-layer rebuild can be done safely — the derivation helpers underneath are trustworthy.

---

## 5. What is structurally wrong (Questions C and L; Deliverable 4)

Everything in this section has an **information-architecture** cause. Fixing it requires deciding what the screen *is*, not adjusting how it is drawn. §7 covers the separable class of problems that are purely egui.

### 5.1 The screen does not express the three-way partition

§1.3 described three sets: membership, declared continuation, and what can be reopened now. The current Workspace Detail renders them as:

| Set | Where it appears | Affordance | Visual weight |
|---|---|---|---|
| Declared continuation | Sidebar card, "Can be reopened" / "Cannot currently be reopened" | Checkbox + preflight dot | Same white card |
| Membership (historical) | Same sidebar card, behind "Show related work (N) ▾" | **Checkbox, no preflight dot** | Same white card |
| Latest snapshot members | Main column, `designation_card` | **"Mark to continue" button** | Same white card |
| Full witnessed history | Main column, `witnessed_card` at 0.72 opacity | None | Same white card |

The same conceptual object — a resource — appears in three places with three different affordances, and the *sets themselves* are distinguished only by which disclosure they sit behind. A user cannot tell from looking that ticking a checkbox under "Show related work" does something categorically different from ticking one above it: the first promotes a historical member into a declaration; the second edits an existing declaration.

Worse, **historical rows carry a checkbox but no preflight dot.** So the user learns whether a resource can actually be reopened only *after* declaring it. The product's entire honesty posture is inverted at exactly the point where the user makes a commitment.

### 5.2 Two mechanisms answer one question (the central defect)

The user has one question: *what am I continuing?* The UI gives them two unrelated controls in two different regions:

```
MAIN COLUMN                          SIDEBAR
designation_card                     restoration_selection_card
"Mark what you're continuing"        "Current continuation"
• row  [Mark to continue]            ☑ row ●
• row  [Marked to continue ✓]        ☐ row ●
                                     [Declare current continuation]
writes OBS-WORK-DESIGNATED           writes OBS-CONTINUATION-SURFACE
own status line                      own status line
single-subject                       ≥2 subjects
button semantics                     draft-then-commit semantics
```

Both are legitimate backend concepts — designation is single-subject (RFC-0011), a surface is multi-subject (RFC-0013). But **that distinction is a backend distinction, and the brief explicitly forbids exposing it.** The user is not supposed to know the word "designation." What they experience is: two lists of the same resources, two ways to mark them, two confirmation styles, two status messages, and no explanation of why marking a thing one way is different from marking it another.

Three consequences make this worse than untidy:

**(a) One control silently destroys the other's work.** `handle_designation` builds its persisted outcome via `new_with_designated`, which passes `continuation_surface: None`. So pressing "Mark to continue" **erases a previously declared continuation surface** from the persisted outcome. This is a backend defect (§8.3, flagged not fixed), but it is a hard design constraint right now: the current layout places the control that destroys a declaration directly above the control that creates one, with nothing warning the user. Any redesign must not invite that sequence until the backend is fixed.

**(b) The ≥2-subject rule is invisible until it fails.** RFC-0013 requires at least two subjects. Nothing on screen says so. A user who unticks down to one resource and presses Declare gets a rejection status line for a rule they had no way to know. And because there is no revoke (invariant 21), they cannot get back to a smaller surface at all — the constraint is not just unexplained, it is a one-way door.

**(c) Designation is global; the screen is per-workspace.** A designation is a global statement at evidence level, intersected per workspace. The UI renders it inside every workspace, so *N−1* workspaces display the cross-workspace warning *"You marked "X" (at …) as the work to continue, but it is not part of this body of work."* This is technically true and, as a default state across a user's workspace list, actively confusing — it reads as an error in a workspace that has nothing wrong with it.

### 5.3 The screen answers questions nobody asked, at the same volume as the one they did

`witnessed_card` renders a 2-column grid of Status / Resources witnessed / Snapshots, a "latest witnessed moment," a full witnessed list, and then **up to fifty snapshot history rows** (`MAX_RENDERED_SNAPSHOTS = 50`), each a `ui.horizontal` of four labels including `witnessed.join(", ")`.

This is an audit log. It is presented as product UI, on the same screen as the primary action, in a card of the same material as the primary content, at 0.72 opacity — which is the only thing distinguishing it, and 0.72 is not enough to read as "this is not for you." `PRODUCT.md` lists **dashboard** among the things Evo refuses to become; fifty rows of timestamped capture history is the most dashboard-like artifact in the product.

The user's question at this level is *"is Evo actually watching this work?"* That is answered by one sentence, not by the log. The log's real audience is a developer debugging derivation.

Compounding it: Status / Resources witnessed / Snapshots appear **twice on the same screen** — once in the header identity line (`ws-{short} · N resources witnessed · M snapshots · {lifecycle}`) and again in the witnessed grid. And `ws-{short_identity}` is a truncated `WorkspaceId` printed directly at the user, which the brief forbids by name.

### 5.4 Home has no dominant fact, and no defensible ordering story

Every card is the same size, same material, same shadow, same treatment. The front door of a product whose metric is time-to-productive requires the user to **read every card and decide.**

The instinctive fix — sort by recency — is architecturally prohibited. Recency is not restoration authority; V3 §13 and V4 §10 both list "no relevance ranking" as an anti-pattern. So the design question is real: *what may Home emphasise?*

There is a correct answer, and it is already in the codebase: **the workspace containing the user's current designation.** `state::select_workspace_with_designation` computes exactly this, justifies it in its own doc comment against RFC-0011 and RFC-0006, derives it deterministically from canonical evidence, and ranks nothing — it is the user's own statement about where their work continues, not Evo's guess. `state.rs:107` already calls it on every reload; `app.rs` then overwrites the result with a selection derived only from `self.selected` and never reads the original (D-38). Home's one dominant fact is computed and discarded.

Present ordering is the workspace log's append order (`cache.rs` `tail_workspaces` → `workspaces()`). That is a stable, non-ranking order and is fine, provided the UI never labels it "recent" and never lets position imply priority. It currently does neither, which is correct — but by accident rather than by decision, and the accident is easy to lose.

### 5.5 The two halves of the product disagree about their own geometry

Home constrains content to `CONTENT_WIDTH = 640` centred. Workspace Detail is full-bleed, splitting into main + sidebar above 700px. At the 900px default window, Home shows a 640px column with 130px of empty background either side, and Detail then expands to fill all 900px. Navigating between them, the product appears to change width. Neither measure is wrong; having both is.

Related: `theme::MIN_WINDOW_WIDTH = 420` is never applied — `main.rs` sets a 680px minimum — so V4 §6.10's entire responsive story below 480px is unreachable, and the 700px column-split threshold (`app.rs:689`) sits 20px above the actual minimum window width, making the stacked layout a 20px-wide accident rather than a designed state (D-28, D-36).

### 5.6 Draft state is not owned by anything

`continuation_draft: Option<BTreeSet<String>>` lives on `EvoApp` and is destroyed by a signature change (B-01). Disclosure state lives in `ui.data` via `insert_temp`. Status strings live in two separate `Option<(String, bool)>` fields. There is no single owner of "the user is in the middle of correcting something," which is why background capture can annihilate an in-progress edit without any code path intending it. `DisplayState` — the type that should hold this — has no field for error, loading, or capture status either (D-04).

### 5.7 Sidebar/main allocation inverts the brief

The brief is explicit: a contextual sidebar may exist inside a Workspace but "must never overpower the actual Workspace," and "the sidebar should NOT be the primary product surface." Currently the sidebar (`detail_split`: 38%, clamped 240–340px) holds `restoration_selection_card` — **the only place the declared continuation, preflight status, and the Declare action appear.** The three cards in the main column hold: a restatement of the resume point, a competing correction control, and an audit log.

The most decision-dense, most consequential content in the product is in the narrow column. The wide column holds context and history. This is backwards, and it is why the checkbox rows wrap awkwardly and the reason strings need a two-space string indent to look nested (D-29).

---

## 6. Why it looks generic, SaaS-like, dense, and un-Apple (Questions B, D, E, F, G, H; Deliverable 5)

### 6.1 The root cause, stated once

**The implementation shipped V4's card geometry without V4's material system, and card geometry without material hierarchy is the definition of a SaaS dashboard.**

This is demonstrable rather than impressionistic. Open `EVO_SPATIAL_PROTOTYPE_V4.html`: its `body` is a vivid dark gradient — blues, greens, pinks over `#1a1a2e` — and the window material uses `backdrop-filter: blur(30px) saturate(180%)`. Every bit of the prototype's spatial quality comes from real translucency over a real environment. The M2 white surfaces read as *floating* because you can see the desktop between and behind them.

Now `main.rs`: no `with_transparent(true)`. And `theme.rs`: `BG = from_rgb(0xE8, 0xE5, 0xE0)` — the Tier-2 approximation, an opaque warm beige.

So the shipped product is **white rounded cards on a flat beige page.** Which is, word for word, the thing V3 §1 was written to condemn: *"Every prior prototype used a hardcoded warm beige/gray gradient as the window background. This made Evo feel like a well-crafted SaaS application — clean, readable, even premium — but fundamentally opaque."*

The V4 lineage diagnosed this failure precisely, chose Direction A to escape it, then documented the escape as Tier 2/Tier 3 work — and the implementation built Tier 1 only. The result is not a partial Direction A. It is **Direction B executed without Direction B's compensating design.** Direction B was evaluated in both specs as the option that "could be mistaken for a well-designed card app," and it was rejected — but it was rejected on the assumption that its shadow-and-luminance depth system would be built if chosen. It wasn't chosen, so that system was never designed, and the shipped UI has neither A's translucency nor B's shadow discipline.

**This is the answer to questions D and F.** The UI reads as SaaS and as generic for one structural reason: it has exactly one surface treatment (white card, hairline border, soft shadow, 14px radius) applied to every kind of content, on a flat background. That is the default aesthetic of every dashboard framework in existence.

### 6.2 The material hierarchy is collapsed in code

`theme.rs` provides three card helpers. Here is what they actually differ by:

| Helper | Fill | Shadow | Stroke | Margin |
|---|---|---|---|---|
| `card` | `panel()` = white | `foreground_shadow()` | `border()` α16 | 24 |
| `card_with(emphasized)` | `panel()` = white | `foreground_shadow()` | α16 → α38 | 24 |
| `elevated_card` | `panel()` = white | `foreground_shadow()` | `border()` α16 | 22 |

Same fill, same shadow, ±22 alpha of border, ±2px of margin. **These are the same surface.** V4 §6.1 specifies five materials M0–M4 with distinct opacity, blur and shadow rules; the implementation has one material and calls it three things. `elevated_fill()` (α250) is defined and never used. So is `page_frame()`, `action_bar_frame()`, and `bold_font()` (D-35).

On Workspace Detail this produces **four to five visually identical white rectangles stacked vertically**, holding content of wildly different importance: the resume point, a correction control, an audit log, the declared continuation. The user gets no pre-verbal cue about what matters. V3 §12 states the requirement exactly — *"Before reading a word, the user should understand: white elevated thing = my work; translucent material with quieter text = context"* — and the implementation puts context in a white elevated thing too.

The one place a genuine material distinction exists (`witnessed_card` at `set_opacity(0.72)`) is undermined by the fact that `set_opacity` fades the card's shadow along with its content, so the "recessed" card gets a *weaker* shadow while keeping the same white fill — reading as a faded duplicate of the important cards rather than as a different layer (D-30).

### 6.3 Why it feels dense (Question E)

Density here is not a spacing problem. The tokens are correct: `GAP_SECTION 24`, `GAP_ENTRY 20`, `GAP_INNER 10`, all on a 4px base, with a unit test asserting the base. The problem is **information volume and simultaneity.**

On one Workspace Detail screen, at rest, with nothing expanded:

1. Eyebrow "WORKSPACE" + title + a four-part identity line
2. Resume point sentence at 30pt in accent teal, spanning multiple lines
3. An explanation sentence
4. A "Next:" line
5. "Mark what you're continuing" + one row per latest-snapshot resource, each with a button
6. "Current continuation" + a 2-line subtitle + rows with checkbox, name, dot, ALL-CAPS status, and an always-visible indented reason
7. A preflight summary sentence with two counts
8. A "Show related work (N) ▾" disclosure
9. An always-present instructional sentence
10. The Declare button + a status line
11. Status / Resources / Snapshots grid — *repeating* the header line
12. Latest witnessed moment
13. Witnessed across this body of work
14. Up to fifty snapshot history rows
15. The pinned bar with a plan sentence and the Continue button

Fifteen distinct informational blocks, of which the user needs **two** to act. Every preflight reason is rendered inline and always, rather than disclosed on demand as V3 §6.8 specified ("Click: 'Can't reopen this — ' + reason"). Every instructional sentence is present whether or not it is relevant. The screen never gets simpler, in either direction: it does not simplify when there is little to say, and it does not defer detail when there is a lot.

**What should disappear and does not:** the instructional line when no correction is in progress; reason text when a resource is READY; the designation card once a surface is declared; the entire witnessed card unless asked for; the "Blocked by" branch always (it can never populate, D-19).

### 6.4 What makes it less Apple-like (Question G)

Apple's desktop idiom, reduced to what actually matters here:

**One dominant thing per screen.** Present: the resume point *is* set at 30pt. But it competes with four cards of equal material weight, so the typographic hierarchy is contradicted by the surface hierarchy.

**Restraint with accent colour.** Violated. A multi-line 30pt sentence in `#0A5E58` is a large field of saturated teal. V4 §6.2 annotates the accent "used sparingly." Apple would set that sentence in near-black at display size and reserve teal for the interactive element. Teal-as-emphasis-on-text plus teal-as-primary-button plus teal-as-committed-checkbox means the accent no longer signals "you can act on this" (D-37).

**Type as system, not as decision-per-site.** Violated. V4 §6.3 defines seven roles. `app.rs` calls **exactly fourteen** distinct sizes — 11, 11.5, 12, 12.5, 13, 13.5, 14, 14.5, 15, 16, 18, 24, 26, 30 (**[verified]** by extracting every size argument passed to a `theme::` text helper) — with several off-scale (11.5, 12.5, 14.5) chosen at the call site. This is the clearest single tell of a hand-tuned rather than systematic interface (D-08).

**Weight, not colour, for emphasis.** Half-violated, and painfully: `theme.rs` builds real 500/600/700 variable-font families *specifically because* `.strong()` only changes colour — and then `app.rs` calls `.strong()` in six places, where it produces a colour shift the user reads as a different text tier rather than emphasis (D-26).

**Controls that look allocated.** Violated. Status and preflight dots are painted at `ui.cursor().left_top() + Vec2::new(4.0, 4.0)` without allocating space, so they overlap or collide with neighbours depending on what precedes them, and cannot participate in alignment. `preflight_dot` uses `radius = 6.0`, i.e. a **12px diameter** circle where V4 §6.8 specifies a 6px circle — the status indicators are double-size (D-09).

**Keyboard parity.** Effectively absent. **[verified]** the entire product handles exactly two key events — `Escape` to clear search (`app.rs:1890`) and `Escape` to close Settings (`:2691`) — and `Sense::focusable` appears nowhere. Checkbox rows, cards, and the overlaid Open button are `ui.interact(rect, id, Sense::click())` with no focus handling and no tab order, so nothing outside a text field can be reached or activated from the keyboard. V4 §9 requires "full keyboard reachability." On macOS this is not a nice-to-have, it is table stakes (D-02).

**Real modality.** Absent. Settings is an in-flow scope, not a modal layer (D-11).

### 6.5 Specific visual defects (Question H)

Enumerated in Appendix A; the ones bearing on the specific list the brief named:

- **Bad spacing** — tokens are right; *application* is wrong in three places: `ui.add_space(theme::PAGE_PAD - 12.0)` (an off-system 16px derived by subtraction), `ui.add_space(theme::GAP_ENTRY + 16.0)` (36px reserved to make room for an absolutely-positioned button), and `Margin::same(24)` vs `Margin::same(22)` between otherwise identical cards.
- **Poor indentation** — reason strings are indented with **two literal spaces inside the string**: `format!("  {reason}")`. This is not indentation, it is text content that happens to start with whitespace; it does not survive wrapping, does not align, and cannot be adjusted (D-29).
- **Awkward alignment** — `surface_member_row` nests a `ui.horizontal` inside a `ui.vertical` and paints an unallocated dot inside it, so checkbox, label, dot and status word have no shared baseline.
- **Oversized containers** — `Margin::same(24)` on every card plus 640px content width means a card containing one sentence occupies a large empty rectangle; the Settings sheet is a fixed 428px tall regardless of content.
- **Clipped buttons** — two real risks: the Continue button squeezed by a truncating label in the same row (D-03) and reason text clipped by the height formula (B-03). Both **[needs visual confirmation]** for exact thresholds; both structurally present in source.
- **Excessive borders** — every card carries a 1px hairline *and* a shadow. V4 §6.6 assigns shadow the job of communicating elevation; adding a border to every shadowed surface doubles the signal and flattens it.
- **Excessive cards** — four to five per screen, all identical (§6.2).
- **Weak hierarchy** — §6.2 and §6.4.
- **Confusing navigation** — the back control scrolls away; the primary action exists twice on Home; Settings is reachable but does not trap focus.

### 6.6 One thing that is visually wrong for an honesty reason (Question B)

`preflight_dot_row` renders **READY / UNAVAILABLE / AMBIGUOUS / UNSUPPORTED** in capitals. `execution_line` renders **opened / failed / unavailable / ambiguous / unsupported** in lower case. Same conditions, two typographic registers, on adjacent surfaces. The capitals additionally read as enum values — which is what they are — and the brief prohibits exposing backend vocabulary. A user cannot be expected to know that "UNAVAILABLE" before pressing and "unavailable" after pressing are the same claim (D-23).

And the copy in `restoration_selection_card` says READY resources *"will be attempted."* Per invariant 24, preflight is a statement about the recent past, recomputed independently at press time. The sentence makes a promise the architecture explicitly declines to make (D-24). This is the only place I found where the product's copy is less honest than the product.

---

## 7. Which problems are egui/implementation, not design (Questions K and L; Deliverable 13)

This partition is the most decision-relevant thing in the review, because the two classes have completely different costs. **[K]** defects are fixable inside existing screens without agreeing on anything. **[L]** defects require deciding what a screen *is* first.

### 7.1 Purely implementation — fixable now, no design decision needed (K)

These are wrong under *any* design. If you want value before approving an IA, this is the list.

| ID | Defect | Fix shape |
|---|---|---|
| B-01 | Signature reset destroys `continuation_draft` mid-edit | Scope transient edit state so canonical reloads don't clear it |
| B-02 | Execution report deletes itself in ~2s | Same; report lifetime must be user-controlled, not reload-controlled |
| B-03 | Report height `64 + 24n` under-allocates vs ~40px/attempt with reason | Measure content, or allocate from laid-out galleys |
| B-04 | Expand animation registered under two ids from two `Ui` scopes | Single stable id, computed once |
| B-05 | `designation_card` `zip` mislabels rows (see §7.3) | Carry `(id, Option<subject>)` pairs; never zip across a `filter_map` |
| D-01 | `daemon.poll()` called twice per frame | Poll once per reload |
| D-09 | `preflight_dot` radius 6.0 = 12px vs V4's 6px circle; dots unallocated | Allocate, then paint at radius 3.0 |
| D-11 | Settings sheet is in-flow, not modal | `egui::Modal` or `Area` + `Order::Foreground` + input capture |
| D-12 | Settings sheet height hardcoded 428/380 | Content-driven height |
| D-14 | Back control and capture banner inside the scroll area | Move both to fixed chrome |
| D-21 | Continue bar hand-painted, no shadow; `action_bar_frame()` unused | Use the theme frame the test already asserts |
| D-22 | Home card has two overlapping click targets for one action | One target |
| D-25 | Stale opacity `0.6 - 0.2*index.min(1)` | Uniform staleness treatment |
| D-26 | `.strong()` used where real weight families exist | Use `evo-semibold` |
| D-29 | Two-space string indentation for reason lines | Real indent via layout |
| D-30 | `set_opacity` fades shadow, inverting recession | Distinct material, not opacity |
| D-31 | Full synchronous replay in `new()`; no loading state | Background derivation + real loading state |
| D-32 | AX window enumeration on the UI thread every 2s | Move probing off-thread; cache with explicit staleness |
| D-34 | `run_execution` blocks the frame; no executing state | Async execution + V4 §7.17 |
| D-35 | Dead-in-the-shell theme/state API (`action_bar_frame`, `page_frame`, `elevated_fill`, `bold_font`, `tiny`, the three status tints, `MIN_WINDOW_WIDTH`, `relative_time_label`, `artifact_ids`, `select_workspace`) | Use or delete; a theme test currently asserts a frame nothing renders. Note `select_workspace_with_designation` and `select_display_workspace` *are* called — from `state.rs`, whose result `app.rs` discards (D-38) |

### 7.2 Design/IA cause — must be decided before coding (L)

| ID | Defect | Requires deciding |
|---|---|---|
| D-04 | `DisplayState` has no error/loading/capture field | What a screen's derived state *is* |
| D-05 | `empty_home` ignores permission/partial-capture status | Whether empty and degraded are one state or two |
| D-06 | Home has no dominant fact | What Home may legitimately emphasise (§5.4) |
| D-07 | Duplicate primary action on Home cards | What a card is: a target, or a container of targets |
| D-08 | 14 type sizes against 7 roles | A closed type scale, enforced |
| D-10 | ≥2-subject rule invisible until rejection | How constraint is taught before it bites |
| D-13 | Home 640px vs Detail full-bleed | One measure for the product |
| D-15 | Identity metadata printed twice per screen | Where identity belongs, once |
| D-16 | 50 snapshot rows as product UI | Whether audit data belongs in the product at all |
| D-17 | Error screen mis-headed "Current work" | What the error screen is |
| D-18 | Search presented as search, implemented as substring | How to state scope honestly |
| D-19 | "Blocked by" branch that can never render | Deleting UI for empty backend fields |
| D-20 | 4–5 identical white cards | The material system (§6.2) |
| D-23 | ALL-CAPS preflight vs lower-case execution | One status vocabulary |
| D-24 | "will be attempted" over-promises | Honest preflight copy |
| D-28 | 700px split threshold vs 680px min window | Real responsive breakpoints |
| D-36 | `MIN_WINDOW_WIDTH` unapplied | Actual minimum geometry |
| D-37 | Accent over-used as text colour | Accent's semantic job |
| D-38 | `DisplayState::display` computed then discarded | Home's dominant fact (§5.4) |
| §5.1 | Three sets rendered with one affordance | The partition's visual language |
| §5.2 | Two mechanisms, one question | Merging correction into one control |
| §5.7 | Sidebar holds the primary decision | Column allocation |

**The honest summary:** roughly two-thirds of the *visible ugliness* is [K] and could be fixed without any design conversation. Roughly all of the *confusion* is [L]. Neither set is sufficient alone — fixing [K] yields a tidy screen that still asks the user to understand designation-versus-surface; fixing [L] yields a well-structured screen with 12px dots and a self-deleting report.

### 7.3 One newly found correctness bug worth stating separately

`designation_card` builds two parallel vectors and zips them:

```rust
let candidates = state::distinct_artifacts_in_latest_snapshot(workspace);
let candidates_subjects: Vec<String> = candidates.iter()
    .filter_map(|id| state::subject_for(subjects, id).map(str::to_string))
    .collect();
for (id, subject) in candidates.iter().zip(&candidates_subjects) {
    let label = state::describe_artifact(kinds, subjects, id);
    // ... button writes `subject`, label displays `label`
}
```

`filter_map` **drops** artifacts with no known subject, so `candidates_subjects` can be shorter than `candidates`. The `zip` then pairs artifact *i* with subject *i* of a shortened vector. From the first dropped artifact onward, **every row displays one resource's name on a button that designates a different resource** — and the zip silently truncates the tail, so the last rows vanish.

This is a correctness bug with a canonical write on the other side of it: the user marks the wrong work as the work to continue, and Evo records their statement faithfully. It cannot be seen in the common case (subjects usually resolve) and cannot be recovered from once triggered (invariant 21: there is no revoke).

Reported, not fixed, per the brief. It sits in the frontend, so it is in scope for the implementation phase — I would fix it in Phase 0 (`UI-IA-0002.md` §10) regardless of the IA outcome.

---

## 8. Which files would need modification (Deliverable 14)

### 8.1 Frontend files — in scope, after approval

| File | Change | Scale |
|---|---|---|
| `crates/evo-desktop/src/app.rs` | Decomposed into screen modules; every screen rewritten | Large. This file should not remain 2726 lines. |
| `crates/evo-desktop/src/theme.rs` | Real M0–M4 separation; closed type scale; allocated dots; delete or use dead helpers | Medium — extend and prune, do not replace |
| `crates/evo-desktop/src/state.rs` | Add derived presentation types (screen state incl. error/loading/capture); keep all derivation helpers | Small–medium, additive |
| `crates/evo-desktop/src/main.rs` | Viewport geometry; `with_transparent(true)` if M0 translucency is pursued | Small |
| `crates/evo-desktop/src/daemon.rs` | Structured exit classification instead of stderr matching | Small |
| *(new)* `crates/evo-desktop/src/screens/*.rs` | One module per screen | New |
| *(new)* `crates/evo-desktop/src/copy.rs` | All user-facing strings in one reviewable place | New |

The `copy.rs` proposal is deliberate. The product's voice is its strongest asset (§4 A1) and it is currently scattered across 2726 lines of layout code, where it cannot be reviewed as writing. Centralising it also makes the five verbatim canonical reason strings auditable — including `SURFACE_DECLARATION_REASON`, which is defined in the backend and never rendered (§8.3).

### 8.2 Files I would read but not modify

`evo-restoration`, `evo-execution`, `evo-daemon`, `evo-workspace`, `evo-artifact`, `evo-observation`, `evo-capture`, `evo-storage`. The UI consumes their types; the redesign changes no signature and needs no new query.

**One qualification.** Moving derivation and preflight off the UI thread (D-31, D-32, D-34) means *calling* existing APIs from a worker rather than the frame. That changes no semantics, no schema, and no contract. If a call site turns out to need `Send`/`Sync` it does not currently have, I will flag it rather than change it.

### 8.3 Backend defects found — reported, not fixed

Per the brief, these are stated and left alone. Two of the three constrain the UI directly.

**BE-01 · Designation erases the continuation surface, durably.** **[verified]** in full this session, and it is an inconsistency between three sibling handlers in one file rather than a subtle contract question:

- `runtime.rs:557` (content observation) reads `self.index.current_continuation_surface_artifacts()` and passes it through.
- `runtime.rs:594` (`handle_continuation_surface`) reads it and passes it through.
- `runtime.rs:637` (`handle_designation`) does not. It calls `RestorationInput::new_with_designated(&workspace, &snapshot, designated)`, which delegates to `new_with_continuation_surface(..., None)` (`derivation.rs:161-167`).

So the surface is sitting on the same `self.index` one line away and simply is not read. `handle_designation` then calls `persist_restoration_outcome`, and every later reader — including a desktop full replay, because `CanonicalIndex::refresh` builds `outcomes` from `tail_restorations()`, i.e. from the persisted restoration log rather than by re-derivation — sees the surface-less outcome. **The erasure is durable**, and because no revoke exists (invariant 21) the only route back is declaring a fresh surface of ≥2 subjects.

**UI consequence:** the two correction controls are not merely redundant (§5.2) — one silently destroys the other's work. Until this is fixed, no design may place them adjacently or imply they are alternatives, and the merged control of `UI-IA-0002.md` §3 must not ship. **The fix appears to be one line** — pass `Some(self.index.current_continuation_surface_artifacts())`, exactly as the sibling handler at `:594` already does — but it is a backend change and I am not making it.

**BE-02 · Daemon status whitelist drops two sources it writes.** `runtime.rs:283-294` writes sources `grouping` and `continuation-surface`; `daemon_status.rs:120-123` omits both from its whitelist and drops them. **UI consequence:** the shell cannot reliably observe that a declaration was accepted, which is why declaration feedback is a local `Option<(String, bool)>` rather than derived state.

**BE-03 · `SURFACE_DECLARATION_REASON` is defined and never referenced.** *"The user declared these resources as the resources their work currently continues across."* This is spec-sanctioned, correctly-voiced copy for the exact moment the UI currently explains in its own words. **UI consequence:** free, authoritative copy going unused. I would render it verbatim.

**BE-04 · No revoke path exists** (invariant 21). Not a defect, but the most consequential architectural gap for the UI. Discussed as a flagged question in `UI-IA-0002.md` §11 — I am not proposing a change, only identifying that the design must be honest about a one-way door.

---

## 9. What I will explicitly NOT change (Deliverable 16)

A commitment, not a summary.

**Product model and architecture.** The observation log as sole ground truth; derivation-not-storage; the five primitives; Continue as the single primary action; Time To Productive as the only metric.

**Backend contracts.** All eight Observation schemas at v1. The three canonical write paths. `DerivationOutcome`'s shape, including `Insufficient` carrying an executable resume point. `RestorationSelection`'s three-way partition. Four-way preflight and five-way execution status. `PlatformExecutor`'s three methods. `PreflightSource`'s two variants. Nothing gains a field.

**The semantic separations.** Membership ≠ continuation ≠ executable target. Designation (single-subject) and continuation surface (multi-subject, ≥2) remain distinct *in the model* even where I propose merging their *presentation*. Relatedness never feeds continuation. Recency never becomes authority. Latest-valid-supersedes stands.

**The epistemic posture.** No invented resources. No silent substitution. Missing shown honestly. Ambiguous left ambiguous. Unsupported left unsupported. Partial restoration as a normal outcome. No confidence scores, no percentages, no ranking, no "probably." Where the backend does not know, the UI says nothing.

**The five canonical reason strings, verbatim.** Including the currently unused `SURFACE_DECLARATION_REASON`.

**The product's voice.** §4 A1. Copy changes only where a sentence over-promises (D-24), exposes internal vocabulary, or describes a state that cannot occur.

**Things that do not exist and will not be invented.** No thumbnails, previews, app icons or favicons (bundle identity is deliberately discarded at capture). No human descriptions, renaming, tags, collaborators, progress, deadlines, priority. No favourites, pins, archive, or manual reorder. No window geometry, no per-artifact timestamps, no observation counts. No persisted UI state. No execution progress streaming. No MRU list. Only two timestamps exist — `Provenance::observed_at` and `Snapshot::captured_at` — and the UI will use only those.

**Things Evo refuses to become.** Not a dashboard, chatbot, note-taking app, task manager, project manager, browser, search engine, knowledge base, or autonomous agent surface. No command palette. No notification badges. No relevance ranking. No glassmorphism for decoration.

**The two destinations.** Home and Workspace Detail. No third top-level surface, no tabs, no breadcrumbs.

**The good implementation decisions.** Empty-report guards before rendering and before `all_opened()`. `WorkspaceCard`'s transience contract. Kind-aware sentence construction. Real variable-font weights. Colour never used alone for status.

**And procedurally:** no code will be written, and no frontend file touched, until the design in `UI-IA-0002.md` is approved.

---

## Appendix A — Defect register

Correctness bugs first, then defects by cause. **[K]** implementation · **[L]** information architecture · **[V]** visual system · **[H]** honesty · **[D]** dead code/drift. Severity: **S1** breaks a product promise · **S2** materially damages comprehension · **S3** polish.

### A.1 Correctness bugs

| ID | S | Tags | Location | Defect |
|---|---|---|---|---|
| B-01 | S1 | K | `derive_display_state` (~300–320) | Signature change resets `continuation_draft`; background capture destroys an in-progress correction |
| B-02 | S1 | K H | same | Signature change resets `execution_result`; because Continue focuses windows and focus is captured, the execution report deletes itself within ~2s |
| B-03 | S1 | K H | `continue_bar_expanded_height` (`app.rs:2283`) vs `execution_line` (`app.rs:2346-2364`) | `64 + 24n` allocated. Rendered: a 13.5pt label row, then for every non-Ready status (and any Opened carrying a detail) `add_space(2)` + an 11.5pt reason line + `add_space(2)` — roughly 38–40px per attempt with a reason. Four such attempts need ≈224px and are given 160px, so ≈64px is clipped, and the reason lines clip first because they render last. These are exactly the partial-restoration explanations `ARCHITECTURE.md` §9 requires |
| B-04 | S2 | K | `ui()` (~542) and `pinned_continue_bar` (~2162) | Expand animation registered under two ids from two `Ui` scopes |
| B-05 | S1 | K | `designation_card` (~1276–1378) | `zip` over a `filter_map`-shortened vector mislabels rows; button designates a resource other than the one named, and tail rows vanish |

### A.2 Information architecture

| ID | S | Tags | Location | Defect |
|---|---|---|---|---|
| D-04 | S2 | L | `state.rs` `DisplayState` (~41) | No error, loading, or capture-status field; screens improvise |
| D-05 | S2 | L | `empty_home` | Empty arm for `PartialCapture`/`PermissionRequired` — empty state stays silent about the likely cause |
| D-06 | S1 | L V | `home_screen` | No dominant fact; all cards identical; user must read everything |
| D-07 | S2 | L K | `home_screen` (~2000+) | Card click target plus absolutely-positioned "Open workspace →" both perform the same action |
| D-10 | S2 | L H | `restoration_selection_card` | RFC-0013's ≥2-subject rule is invisible until rejection; no revoke exists |
| D-13 | S2 | L V | `ui()` Home vs `workspace_detail` | 640px column vs full-bleed; product appears to change width |
| D-15 | S3 | L | `workspace_detail` + `witnessed_card` | Status/resources/snapshots printed twice on one screen |
| D-16 | S2 | L | `witnessed_card` (`MAX_RENDERED_SNAPSHOTS = 50`) | Audit log presented as product UI; the most dashboard-like artifact in a product that refuses dashboards |
| D-17 | S2 | L | `error_screen` | Headed `title("Current work", 16.0)` above "Evo could not read its stored state" |
| D-18 | S3 | L H | `home_screen` search | Search-engine affordance over a `String::contains`; scope never stated |
| D-19 | S3 | L D | `continuation_card` "Blocked by" | Branch cannot render: `blockers` always empty (`derivation.rs:536`) |
| D-38 | S2 | L | `app.rs:324` | `DisplayState::display` (designation-aware, `state.rs:107`) is overwritten from `self.selected` and never read; Home's one legitimate dominant fact is computed then discarded |
| — | S1 | L | §5.1, §5.2, §5.7 | Three sets one affordance; two mechanisms one question; sidebar holds the primary decision |

### A.3 Visual system

| ID | S | Tags | Location | Defect |
|---|---|---|---|---|
| D-08 | S2 | V | throughout `app.rs` | 14 distinct type sizes (11 → 30, incl. off-scale 11.5/12.5/14.5) against V4's 7 roles — **[verified]** count |
| D-09 | S2 | V K | `theme::preflight_dot`, `status_dot` | `radius = 6.0` → 12px circle vs V4's 6px; painted unallocated at a cursor offset |
| D-20 | S1 | V | `theme::card`/`card_with`/`elevated_card` | Same fill, same shadow, ±22α border, ±2px margin — one material called three things |
| D-23 | S2 | V H | `preflight_dot_row` vs `execution_line` | ALL-CAPS enum words vs lower-case words for the same conditions |
| D-25 | S3 | V | `home_screen` | `0.6 - 0.2*index.min(1)`: first stale card 0.6, all others 0.4, encoding nothing |
| D-26 | S2 | V | 6 sites incl. `designation_card`, `continuation_card`, `detail_row` | `.strong()` (colour-only) used where real 600-weight families exist |
| D-27 | S3 | V K | `first_run_screen` | Step dots painted unallocated at a cursor offset |
| D-28 | S2 | V L | `app.rs:689` (`if total < 700.0`), `theme::detail_split`, `main.rs` | 700px split threshold vs 680px minimum window: stacked layout is a 20px accident |
| D-29 | S3 | V K | `surface_member_row` | Two literal spaces inside the string as indentation |
| D-30 | S2 | V | `witnessed_card` | `set_opacity(0.72)` fades the shadow too; recession reads as a faded duplicate |
| D-36 | S3 | V D | `theme::MIN_WINDOW_WIDTH` | Defined 420, never applied; V4 §6.10's sub-480px story unreachable |
| D-37 | S2 | V | `continuation_card` | 30pt multi-line accent-teal sentence; accent no longer signals actionability |
| D-21 | S2 | V K D | `pinned_continue_bar` | Bar hand-painted with no shadow while `action_bar_frame()` — asserted by a theme test — is unused |

### A.4 Honesty

| ID | S | Tags | Location | Defect |
|---|---|---|---|---|
| D-24 | S1 | H | `restoration_selection_card` | READY resources "will be attempted" — preflight is a statement about the recent past, recomputed at press time |
| D-02 | S1 | K | checkbox rows, cards, overlay button | Only two key handlers exist in the whole crate (both `Escape`, `app.rs:1890` and `:2691`); no `Sense::focusable` anywhere, so no element is keyboard-reachable. V4 §9 requires full parity |
| D-33 | S2 | K | `daemon.rs` `classify_exit` | Permission state inferred from `stderr.contains(...)` |

### A.5 Performance and lifecycle

| ID | S | Tags | Location | Defect |
|---|---|---|---|---|
| D-01 | S3 | K | `logic()` `app.rs:380`, `ui()` `app.rs:442` | `daemon.poll()` called twice per frame |
| D-31 | S2 | K L | `EvoApp::new` | Full synchronous replay before first frame; no loading state can exist |
| D-32 | S2 | K | `preflight_selection` | Accessibility window enumeration on the UI thread every 2s; `selection_for` recomputed per workspace |
| D-34 | S2 | K L | `run_execution` in the click handler | Blocks the frame; V4 §7.17 "Continue — Executing" unimplementable as written |
| D-35 | S3 | D | `theme.rs`, `state.rs` | Unreferenced from `app.rs`: `action_bar_frame`, `page_frame`, `elevated_fill`, `bold_font`, `tiny`, `success_tint`, `warning_tint`, `error_tint`, `MIN_WINDOW_WIDTH`, `relative_time_label`, `artifact_ids`, `select_workspace` — **[verified]** by grep. `select_workspace_with_designation` / `select_display_workspace` are reached via `state.rs:107`/`:310`, but their result is discarded — see D-38, which is the more accurate finding |

### A.6 Claims requiring visual confirmation

Not asserted as fact. Each is structurally present in source; the rendered consequence is unverified because I have not run the binary.

1. Continue button clipping at 680px width (D-03).
2. Exact clipping threshold of execution reason text (B-03) — the height shortfall is arithmetic, the visible result is not.
3. Settings sheet overflow with a long `PartialCapture` detail string (D-12).
4. Whether 0.72 opacity reads as recession or as staleness (D-30).
5. Perceived crowding of the checkbox / label / dot / status row (§6.5).
6. Whether the 30pt accent sentence reads as premium or as loud at real size (D-37).
7. Actual contrast of `RECEDED #AEAEB2` on `BG #E8E5E0` — arithmetically near 2:1, below V3 §6.2's own ≥3:1 floor, but needs measurement against the real render.

Item 7 is the one I would check first; if confirmed it is an accessibility defect in the token set rather than in its use.

---

## Appendix B — Verification pass

The first draft of this review was written from a full read of the sources. It was then re-verified claim-by-claim against the repository, because a review that asserts the product should not over-claim has an obligation not to over-claim itself. Every file path, line number and symbol cited in both documents was re-checked. This appendix records what the pass found, including where it found me wrong.

**Confirmed exactly as written, by direct re-read:** `derivation.rs:531`/`:536` (`context_chain` and `blockers` constructed unconditionally empty); `runtime.rs:283-294` writing seven capture sources against `daemon_status.rs:120-123` whitelisting five (BE-02); `SURFACE_DECLARATION_REASON` at `derivation.rs:85` with **zero** other references anywhere in `crates/` (BE-03); `MAX_RENDERED_SNAPSHOTS = 50` at `app.rs:34`; `continue_bar_expanded_height` returning `64.0 + lines * 24.0` at `app.rs:2283`; `evo-continue-expand` registered at both `app.rs:544` and `app.rs:2164` from two different `Ui` scopes (B-04); `0.6 - 0.2 * index.min(1)` at `app.rs:1949` (D-25); `format!("  {reason}")` at `app.rs:1187` (D-29); `state_signature` = `(workspace id, snapshots.len())` at `app.rs:357-359`, which is the mechanism of B-02; `daemon.poll()` at both `app.rs:380` and `app.rs:442` (D-01); the hardcoded `428.0 / 380.0` sheet heights at `app.rs:2624` (D-12); `theme::title("Current work", 16.0, …)` heading the storage-error screen at `app.rs:2506` (D-17); `empty_home`'s literally empty arm for `PartialCapture | PermissionRequired` (D-05); `preflight_dot` painting at radius `6.0` on an unallocated cursor offset at `theme.rs:620-626` (D-09); `CONTENT_WIDTH = 640` at `theme.rs:124` against `if total < 700.0` at `app.rs:689` and a 680px minimum window in `main.rs` (D-13, D-28, D-36); and `MIN_WINDOW_WIDTH = 420` defined at `theme.rs:126` and applied nowhere.

**B-05 confirmed in detail.** `designation_card` builds `candidates_subjects` with `.filter_map(|id| state::subject_for(...))` and then iterates `candidates.iter().zip(&candidates_subjects)`. When any candidate has no recorded subject, the vectors have different lengths and `zip` silently pairs each id with a *later* candidate's subject. The label the user reads and the `ArtifactId` the button writes then disagree — with a canonical, unrevokable write on the other side.

**Three claims were sharpened rather than confirmed, and one was wrong.**

*Wrong: D-38.* I wrote that `select_workspace_with_designation` is never called. It is — `state.rs:107` calls it on every reload to populate `DisplayState::display`. What actually happens is worse and more specific: `app.rs::derive_display_state` recomputes `self.display` from `self.selected` alone (`app.rs:324`), and no code path in `app.rs` ever reads `DisplayState::display` (its only occurrences are the declaration at `:47`, the `None` initialiser at `:150`, and that overwrite). So Home's one legitimate dominant fact is derived, carried across the state boundary, and discarded at the last step. The finding stands; the mechanism is different, and the corrected version is a stronger argument for the proposal, since building the hero means *stopping a deletion* rather than adding a feature. D-35's list has been corrected accordingly.

*Overstated: D-02.* I wrote that keyboard handling is "absent entirely." Two handlers exist, both `Escape` (`app.rs:1890`, `:2691`). No `Sense::focusable` appears anywhere, so the conclusion — nothing outside a text field is keyboard-reachable — holds, but "absent entirely" was not literally true and is now stated precisely.

*Understated: BE-01.* I described it as a contract subtlety. It is a straightforward inconsistency between three sibling handlers in one file: `runtime.rs:557` and `:594` both read `self.index.current_continuation_surface_artifacts()` and pass it into derivation; `handle_designation` at `:637` does not, and the surface-less outcome is then persisted and read by everyone afterwards, including a full desktop replay (`CanonicalIndex::refresh` builds `outcomes` from `tail_restorations()`, i.e. from the restoration log, not by re-derivation). The erasure is durable, and the fix appears to be one line. This matters because BE-01 gates the central proposal.

*Imprecise: D-08 and D-26.* Both counts are now exact rather than approximate: **fourteen** distinct type sizes (11, 11.5, 12, 12.5, 13, 13.5, 14, 14.5, 15, 16, 18, 24, 26, 30) and **six** `.strong()` call sites.

**Authority.** RFC-0010 has now been read in full and the gap declared in §0 is closed there, including the two places where it strengthened the argument rather than merely confirming it.

**Repository state.** No file outside `docs/design/` has been created, modified or deleted. `git status` shows the working tree as it was found; the only addition attributable to this review is the untracked `docs/design/` directory containing these two documents. That will remain true until the proposed design is approved.

**What this pass did not do.** It did not run the binary, so the seven items in A.6 remain `[needs visual confirmation]`. It did not run `cargo test`, since no code changed. Those are the honest limits of a source-only review.
