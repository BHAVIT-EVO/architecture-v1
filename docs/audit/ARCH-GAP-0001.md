# ARCH-GAP-0001 — Architectural Gap Analysis

**Status:** Analysis. Not authority. Nothing here amends a frozen contract.
**Builds on:** BE-AUDIT-0001, BE-TRACE-0001, and the four grounded fixes implemented after them (window-focus pid schema v2; plan/execution fidelity; Home ordering; storage root migration).
**Source changes:** none. No file under `crates/` was created or modified.
**Method:** every claim below is grounded in a clause of the frozen corpus (cited as `doc:line` or `§clause`) and, where behavioral, in source read at `crates/…:line`. The implementation state described in §1 was verified by reading source this pass; the earlier four fixes were verified by `cargo test --workspace` (731 passed) in the prior session and are taken as the baseline.

---

# Part 1 — The 20 questions, answered

## 1. What is the true unit of work Evo is supposed to represent?

The **open engagement** — a coherent period of directed activity that remains meaningfully continuable after attention leaves it (`COGNITIVE_MODEL` Principle I, Principle II: "Openness is a property of an engagement. It is not a separate object."). Its computational representation is the **Workspace** (`ARCHITECTURE.md:58`: "The single durable, confidence-bearing derived container representing an ongoing body of work"; `RFC-0003`: "Evo's current best explanatory hypothesis that a collection of Artifact histories collectively describe the evolution of one coherent body of work").

The unit is therefore already named by the corpus. The defect is that in the data the Workspace has collapsed into the Artifact: whenever neither co-membership signal fires, a Workspace holds exactly one Artifact forever, so the two concepts are "distinct in the model and identical in the record" (BE-AUDIT-0001 §14.5). The unit of work is not missing; the formation channel that would let it span multiple heterogeneous Artifacts is missing.

## 2. The distinctions

| Concept | Definition | Authority |
|---|---|---|
| **Observation** | An immutable, witnessed, append-only fact ("at time T, Evo observed X"). Never interpreted at write time. The only sacred table. | `ARCHITECTURE.md:47-52`, §3 |
| **Artifact** | A persistent-identity thing referenced by Observations (file, URL, repository, document). Identity is lookup and dedup, never inference. Answers only "is this the same external entity?" | `ARCHITECTURE.md:53-55`; IS-0004 R-2 |
| **Workspace** | Evo's best explanatory hypothesis that Artifact histories describe one coherent body of work. Derived, disposable, replayable, persisted only for restoration speed. | RFC-0003; `ARCHITECTURE.md:56-58` |
| **Work body / workstream** | The same concept as Workspace. No separate object exists or is permitted (Law XVI: concepts that relate/describe must not become objects; `ARCHITECTURE.md:58` is singular: "the single … container"). | Law XVI; RFC-0003 |
| **Continuation Surface** | The set of already-witnessed subjects the user *declares* their work currently continues across. Declaration-only; never inferred; never changes membership. | RFC-0013 |
| **Resume Point** | The canonical cognitive entry point into a Workspace ("where should the user continue thinking?"). Exactly one. | IS-0019 RSP-1–RSP-4 |
| **Context Chain** | Ordered supporting context required to understand the Resume Point. Currently **empty by contract** — no canonical supporting relationship exists. | IS-0021 §25.3 |

## 3. Why heterogeneous resources cannot associate

Co-membership has exactly two signals, both binary (`crates/evo-workspace/src/co_membership.rs:46,50`): repository membership (0.8, emitted only for file paths and commit hashes by the fsevents collector) and the user's explicit WorkGrouped declaration (1.0). `strength(url, x) = 0.0` and `strength(window_title, x) = 0.0` always, because no collector emits a membership Observation naming a URL or window title (BE-AUDIT §5.1, §5.2). So:

- **browser URL + file:** join only via a WorkGrouped declaration — which exists in the daemon (`grouping.rs:213`) but has **no desktop surface** (verified: no `submit_grouping` reference anywhere under `evo-desktop/src` or `examples`).
- **terminal + repository:** the terminal is witnessed as a window title; a window title never carries repository identity; no link, unless declared.
- **non-git file + git file:** only the git file has membership; the non-git file scores 0.0; no link, unless declared.
- **multiple browser tabs:** URLs never co-member each other automatically.
- **application/window + file:** window-title artifacts never co-member anything.
- **resources across applications:** same — structurally 0.0.

RFC-0012's Competing-World Test is *why* this is frozen: every automatic signal (recency, frequency, app identity, title similarity, domain similarity, filesystem proximity, temporal co-occurrence, session grouping) produces identical evidence in "genuinely same body" vs "merely associated" worlds and is rejected. `ARCHITECTURE.md:131` explicitly excludes session segmentation and a separate "understanding" stage. This is a *principled* wall, not an implementation omission.

## 4. What evidence the collectors legitimately provide

| Collector | Witnesses | Canonical evidence produced |
|---|---|---|
| Accessibility window source | focused window title + owning pid | `OBS-WINDOW-FOCUS-GAINED` v1/v2 (pid now in `Provenance::context`, FIX 1) |
| FSEvents | file save paths; git reflog appends | `OBS-FILE-SAVED`, `OBS-COMMIT-MADE`, and `OBS-REPOSITORY-MEMBERSHIP` for paths/commits inside a resolved repository (`macos_fsevents.rs:297-331`) |
| URL poller | same-pid URL transitions in web areas | `OBS-URL-NAVIGATED` (baseline-first, refuses unwitnessed transitions) |
| User declaration channels | designation / grouping / surface acts through trusted sockets | `OBS-WORK-DESIGNATED`, `OBS-WORK-GROUPED`, `OBS-CONTINUATION-SURFACE` |

Everything else — which windows the user focused *in sequence*, how long, which app — is either witnessed-and-discarded (fixed for pid; title sequence still not relational evidence) or not witnessed at all.

## 5–8. Which evidence can establish what

- **Co-membership:** repository membership (files/commits in one repo) and WorkGrouped declarations. Nothing else (RFC-0012).
- **Continuation:** WorkDesignated (single subject) and ContinuationSurface (multi-subject) — both declaration-only, never inferred (RFC-0011, RFC-0013).
- **Supporting context:** **none.** No canonical supporting-relationship class exists; IS-0021 §25.3 therefore hard-empties the Context Chain, and the comment in `derivation.rs:529-536` cites that clause. This is the Layer-3 gap (CONFLICT M, BE-TRACE §3.3).
- **Impossible to represent today:** URL↔file, window↔file, terminal↔repo relatedness (automatic); Workspace↔Workspace relationship with strength score (mandated by `ARCHITECTURE.md:151` "only linked by a relationship with a strength score", absent everywhere — CONFLICT K, BE-AUDIT §3.3); supporting/reference/continuation relationships inside the Snapshot (IS-0021 §25.1 names them explicitly absent); the third formation outcome "attaches nowhere" (`ARCHITECTURE.md:150`, unrepresentable — BE-AUDIT §4.2.1).

## 9–10. The correct primitive, and the smallest change

The six options, assessed against the frozen corpus:

- **A. Richer Workspace formation — YES, the core fix.** `ARCHITECTURE.md:150` mandates three outcomes ("attaches to an existing Workspace, forms a new one, **or attaches nowhere**"); the implementation has exactly two (`WorkspaceDecision` has `AttachExisting`/`RecognizeNew`; `form_workspace_from` returns `Workspace`, not `Option<Workspace>`). Implementing the third outcome is **implementing a level-3 mandate, not inventing semantics.** This alone collapses "every witnessed resource" into "every witnessed *engagement*".
- **B. Workspace↔Workspace relationships — mandated but not the smallest step.** `ARCHITECTURE.md:151` requires "a relationship with a strength score"; nothing below level 3 specifies it. It is the reconciliation of CONFLICT N (BE-AUDIT §7.2: the "weak, scored, non-membership link" is the middle register between no-relationship and membership). It is a *large* specification (evidence, strength semantics, storage, replay) and belongs after A.
- **C. A new Work Body layer above Workspaces — REJECT.** Contradicts `ARCHITECTURE.md:58` ("the single … container"), the closed five-primitive model (`ARCHITECTURE.md:47` "Five primitives. Nothing else is persisted as ground truth"), Law XV (smallest set of concepts), and Law XVI (openness "is not a separate object"; `COGNITIVE_MODEL` II).
- **D. Richer evidence classes — only the unsurfaced one.** No automatic class survives the Competing-World Test; RFC-0012 already ran it. The one evidence class that exists, is canonical, and is *not delivered to the user* is **WorkGrouped**. Surfacing it is delivery of accepted semantics, not invention.
- **E. Combination — the answer:** **A + delivering D's existing WorkGrouped surface.** B deferred and named. C forbidden.
- **F. Something else:** nothing else is needed; the "attaches nowhere" outcome *is* the something else.

**Fewest new semantics for the largest product value:** implement the third formation outcome (A), which reuses the entire existing pipeline and changes one decision enum plus one persistence branch; and expose the existing WorkGrouped declaration to the desktop (D), which reuses the accepted RFC-0012 channel and its daemon socket verbatim.

## 11–15. Effects on the downstream surfaces

- **Continuation Surface (RFC-0013):** unchanged in every clause. With A + WorkGrouped, a user's multi-resource declaration stops being sliced into per-Workspace slivers — the resources actually live in one Workspace, so `surface(W) = {declared ∩ W}` is no longer {one resource}.
- **Context Chain:** unchanged — remains empty by IS-0021 §25.3 until a supporting-relationship evidence class is specified (deferred, not invented). The Layer-3 question (`ARCHITECTURE.md:211` cap vs RFC-0013's unbounded surface) is a document decision, flagged in §Decisions.
- **Resume Point:** unchanged (designation or single-artifact under §25.2). With A, "single artifact" now means a genuinely witnessed engagement, not a one-off focus.
- **Home:** with A, the canonical set shrinks to bodies of work; with the existing FIX-3 ordering (continuable first, canonical within groups, no omission), Home stops asserting resumability for one-off witnessed events. No new ranking, no recency.
- **Restoration:** the declared-surface execution path (already wired, plan/execution aligned by FIX 2) restores multi-resource bodies when the body actually contains the resources. Execution semantics unchanged.

## 16–20. Honesty under volume, ambiguity, and uncertainty

- **20 unrelated things in one day:** under A, each one-off witness attaches nowhere (witnessed, indexed, searchable, no Workspace). Workspaces form only for re-witnessed artifacts, repo-coherent artifacts, or declared work. Home shows the engagements, not the events.
- **Avoiding "every resource becomes a Workspace":** A, directly — it is the third outcome of `ARCHITECTURE.md:150`.
- **Avoiding merging unrelated work:** no new automatic signal is added (RFC-0012's rejections stand); WorkGrouped is the user's own judgment (Law IX); repository membership stays 0.8, not 1.0, precisely because a repo may hold several bodies of work (RFC-0012).
- **Ambiguous evidence:** the artifact attaches nowhere; no Workspace, no claim. Re-witnessing or a declaration later brings it into a body deterministically (replay).
- **Uncertainty:** silence. Insufficiency is a valid, explicit outcome (RFC-0010 Requirement 7; IS-0021 §25.8; Law V, Law VI; Constitution Article III). Nothing is guessed, ranked, or scored.

---

# Part 2 — Scenarios

## Scenario A — project/plan.md, project/notes.md, terminal in same repo, browser research

- **Current:** plan.md + notes.md share one Workspace via repository membership (0.8). The terminal (a window title) and the research tab (a URL) each become their own permanent single-member Workspace — no evidence class can ever relate them. A ContinuationSurface spanning all four is intersected per Workspace: project gets {plan, notes}, research gets {url}, terminal gets {title}. The product promise — "reconstructs the complete environment that thought belongs in" (`PRODUCT.md:118-136`) — is structurally unreachable.
- **Should:** one body of work. The user declares once, through the existing WorkGrouped channel, that {plan.md, notes.md, terminal, research-url} are related; all four attach to one Workspace (declarations are 1.0 and conclusive, RFC-0012). Designation marks plan.md; the surface {plan, notes, research} restores all three in the announced order.
- **Required:** the third formation outcome (so the URL and terminal are not locked into their own Workspaces before the declaration can route them), and the WorkGrouped desktop surface. No new semantics.

## Scenario B — 40 PDFs, 20 tabs, 15 files, 5 repositories

- **Current:** 5 repos → 5 Workspaces (the one mechanism that works). 40 PDFs, 20 URLs, 15 non-git files → ~75 additional single-member Workspaces, all asserting resumability on Home.
- **Should:** 5 Workspaces. The 75 one-off events are witnessed, searchable, and can be declared into a body later — but they are not bodies of work.
- **Required:** the third formation outcome alone. No new evidence, no noise list, no scoring. This is the 100 FILES test at scale: git files pass today (BE-AUDIT §5.2); everything else must stop forming Workspaces on first witness.

## Scenario C — 90 minutes of research: Safari → PDF → Notes → Terminal → browser → document

- **Current:** six single-member Workspaces; nothing links them; each restores one resource or nothing.
- **Should:** one body of work — established honestly by the user's declaration (WorkGrouped and/or a ContinuationSurface), never by session/time-based inference (`ARCHITECTURE.md:131` excludes sessions; RFC-0012 rejects temporal co-occurrence).
- **Required:** the declaration surfaces (WorkGrouped on desktop) + the third formation outcome so the pieces are not pre-locked. If the user never declares, Evo stays honest with six witnessed pieces — that is the correct, Law-VI behavior, not a failure.

## Scenario D — two completely different tasks in one repository

- **Current:** one repo → one Workspace (0.8, correctly — a repo is one history of co-evolution). Both tasks' files are members. Designation marks one task's entry point; the surface {task-A files} restores A and leaves B historical (RFC-0013: historical membership survives; `selection.rs` never preflights history). This already works, and it validates the 0.8 strength and the surface design.
- **Should:** identical behavior. This scenario is the argument *against* over-grouping: the repo must stay one Workspace, and the user's declaration — not recency or membership — discriminates the current task.
- **Required:** nothing new.

---

# Part 3 — Decisions required from you

The proposed change touches frozen documents at exactly four points. Nothing else requires amendment. Per the mission rule, I stop at each rather than resolve it silently.

1. **The third formation outcome (A).** `ARCHITECTURE.md:150` mandates it; IS-0012 Stage 3 currently says "Exactly one outcome SHALL occur: Attach to an existing Workspace or Recognize a new Workspace. No other outcome is permitted." By the precedence rule (`ARCHITECTURE.md:7`: "If an RFC contradicts this document, the RFC is wrong") the level-3 text wins and IS-0012 Stage 3 must be amended to admit the third outcome. This is a one-clause IS amendment.
2. **The origination evidence floor.** The RFC-0012 Origination Invariant ("co-membership evidence extends Workspaces; it never originates them") must be refined to state the sufficiency condition for origination: *a Workspace is created when a content Observation establishes a previously unseen Artifact **and** that Artifact carries witnessed continuity evidence — a prior witness of the same Artifact, or canonical repository membership for the Artifact.* This keeps the invariant's letter (the trigger is still a content Observation; co-membership observations alone still create nothing) while defining the floor. RFC-0011's "a designation SHALL NOT establish a Workspace" is preserved: designation alone does not originate (an unattached designated Artifact originates on its next witness or its repo membership; the current designation then applies). If you want designation to originate directly, that requires a separate RFC-0011 amendment — flagged, not assumed.
3. **The Layer-3 question (BE-TRACE §3.3).** Is `ARCHITECTURE.md:211`'s "supporting artifacts, capped to a small number" the unimplemented Context Chain, or is the Continuation Surface Evo's Layer 3? If the latter, `:211`'s cap contradicts RFC-0013 and one document must be amended. I recommend: declare the Continuation Surface the Layer-3 set, remove the cap conflict by amending `:211` (the cap language predates RFC-0013 and was written when no Layer-3 set existed), and keep the Context Chain empty. This is a document decision.
4. **Surfacing WorkGrouped in the desktop (D).** This is delivery of accepted RFC-0012 semantics (the daemon socket and schema already exist), not a contract change and not a frontend redesign — but it is a UI addition, so I list it rather than assume it.

---

# Part 4 — Final: exactly three sections

## 1. CURRENT SYSTEM — what is objectively true today

- The pipeline is built and largely correct: capture → observation → artifact identity → formation → derivation → planning → execution → Home, with replay and honest per-target failure reporting (BE-AUDIT §14.4; 731 tests passing after the four fixes).
- **Eight** canonical Observation kinds, all witnessed facts, none inferred: four content (window focus, file saved, URL navigated, commit made), one structural (repository membership), three declaration classes (work designated, work grouped, continuation surface) — `BE-TRACE-0001 §0.2`.
- **Formation has exactly two outcomes** (`WorkspaceDecision::AttachExisting | RecognizeNew`); the level-3-mandated third outcome "attaches nowhere" (`ARCHITECTURE.md:150`) is unrepresentable. Every witnessed resource that scores 0.0 against every candidate becomes a new single-member Workspace.
- **Co-membership is exactly two binary signals** (repo 0.8, WorkGrouped 1.0). URLs, window titles, and non-git files structurally score 0.0 against every candidate — they can never join an existing body of work automatically, and WorkGrouped has no desktop surface to do it manually.
- **Context Chain is empty by contract** (IS-0021 §25.3); Blockers are zero by contract (§25.4); the Resume Point and Next Step derive only from a designation or a single-artifact set (§25.2, §25.5); the Continuation Surface is declaration-only, intersected per Workspace, never gating completeness (RFC-0013; IS-0019 CS-1–CS-4).
- **Home** lists every formed Workspace, ordered (FIX 3) so continuable work is first, nothing omitted, provenance copy amended to state that grouping truthfully. **Restoration** announces and executes the same set (FIX 2), with a declared surface executed as its restore-worthy members and everything else reported per-target. **Storage** lives in Application Support with a verified copy-forward migration (FIX 4). **Window focus** carries its owning pid in provenance (FIX 1).
- The first point an ordinary observed resource becomes a Workspace is `form_workspace_from` → `RecognizeNew` → `build_new_workspace` (`crates/evo-workspace/src/formation.rs`), with no filter, gate, or third outcome anywhere on the path.

## 2. PRODUCT GAP — what prevents Evo from delivering its intended value

- **Proliferation.** Home shows witnessed events as bodies of work. 100 non-git files → 100 Workspaces; every tab and window → its own Workspace (BE-AUDIT §5.2). The product promise is continuity of *thought*; the system currently asserts resumability for every *touch* (BE-AUDIT §6.2: a FAIL against Constitution Article III and Law VI).
- **Fragmentation.** The resources of one real engagement — browser tab + file + terminal — are structurally incapable of joining one Workspace. Because they cannot join, the Continuation Surface is sliced into one-resource slivers, the Context Chain is empty, and "open a Workspace → one resource → nothing to restore" is the guaranteed steady state for two of the four content classes.
- **The honest mechanisms that do exist are not delivered.** WorkGrouped — the accepted, canonical, user-owned answer to heterogeneous relatedness — has no desktop surface. The user cannot do the one thing RFC-0012 makes authoritative: declare their own grouping.
- **The Layer-3 contradiction is unresolved.** `ARCHITECTURE.md:211` promises layered restoration (supporting artifacts, capped); IS-0021 §25.3 empties the only layer that would carry them; RFC-0013's surface is the only set that exists but predates no cap. Restoration of multi-resource work therefore depends entirely on declarations, and even then only within a single Workspace.
- In one sentence: **Evo implements its evidence contracts faithfully, but the only contracts it has cannot represent a heterogeneous body of work, and the one contract that can — the user's declaration — is half-delivered.**

## 3. PROPOSED ARCHITECTURAL CHANGE — the smallest principled change that closes the gap

**A. Implement the third formation outcome — "attaches nowhere" (`ARCHITECTURE.md:150`).**
`WorkspaceDecision` gains `AttachNowhere`; `form_workspace_from` returns `Option<Workspace>` (or an equivalent discriminated result); the daemon persists nothing for a nowhere outcome (the Artifact remains canonical; the derived index tracks unattached Artifacts; replay re-derives everything). Origination requires witnessed continuity evidence: a prior witness of the same Artifact, or canonical repository membership for it. This is deterministic, replayable, consumes only canonical input, and is the direct fix for proliferation. IS-0012 Stage 3 and the RFC-0012 Origination Invariant are amended per Part 3.

**B. Deliver the existing WorkGrouped declaration to the desktop.**
The desktop gains the declaration surface the daemon socket (`grouping.rs:213`) already implements, offering only witnessed subjects (the RFC-0012 producer rule). The user declares their own relatedness — Law IX's authority — and heterogeneous resources join one Workspace through the accepted 1.0 channel. No new semantics, no new evidence class, no inference.

**C. Resolve the Layer-3 contradiction by document decision.**
Declare the Continuation Surface the Layer-3 restore set; amend `ARCHITECTURE.md:211`'s cap sentence (written before RFC-0013) to reconcile; keep the Context Chain empty until a supporting-relationship evidence class is separately specified. Do not invent a supporting class.

**D. Defer the Workspace↔Workspace relationship link, explicitly.**
`ARCHITECTURE.md:151` mandates it; it is the reconciliation of CONFLICT N; but it is a full specification (evidence classes, strength semantics, storage, replay) and is not needed for the proliferation or fragmentation fixes above. It is the named next layer, not this change.

**E. Keep every downstream contract fixed.**
Continuation Surface, Resume Point, Context Chain, Blockers, Next Step, execution, Home ordering, determinism, replay, and local-first are untouched. The five primitives stay five. No scoring, no recency, no frequency, no LLM, no hardcoded categories, no omission from Home, no new canonical state.

**MUST NOT change:** CONSTITUTION, PRODUCT, ARCHITECTURE (except the flagged `:211` cap reconciliation), the Architectural Laws, RFC-0010/0011/0013 semantics, RFC-0012's Competing-World rejections, the five primitives, Observation immutability, Artifact/Workspace/Snapshot/Attachment semantics, determinism and replay, and local-first operation.

**Tests that prove it:** a red-first formation test asserting a single witnessed non-git file save forms **no** Workspace; a test that a re-witnessed file, a repo-coherent file, and a declared file each originate; a test that a WorkGrouped declaration routes a URL and a terminal into the repo Workspace; a 100-files scenario (4 repos → 4 Workspaces; 100 non-git single saves → 0 Workspaces); replay equivalence between the two-outcome and three-outcome rules over unchanged logs; and an existing-test revision: `scenario_3_same_repository_many_files_stays_honest` (`scenarios.rs:234-242`), whose assertion "3 files → 3 Workspaces" is correct today and becomes false under A — its name finally matches its content.

---

# Part 5 — Implementation status (implementation phase)

This appendix records what the implementation phase executed, per the approved architecture above.

## Decisions executed

- **D1 — AttachNowhere implemented.** `WorkspaceDecision` gained `AttachNowhere`; `form_workspace`/`form_workspace_with_candidates`/`form_workspace_with_evidence` return `Option<Workspace>`, with `OriginationEvidence { prior_witness, repository_membership }` as the sufficiency floor (RFC-0003 Req 5 — Historical Basis; RFC-0012). The daemon's vertical pipeline (`runtime.rs`), the derived index (`cache.rs`), and full replay (`workspace_replay.rs`) compute origination evidence identically and persist nothing for a nowhere outcome. Deterministic replay is preserved. `IS-0012 Stage 3` and the `RFC-0012 Origination Invariant` were amended minimally to reconcile with `ARCHITECTURE.md:150` (the architecture is the higher authority; precedence rule).
- **D2 — WorkGrouped delivered to the desktop.** `state::submit_grouping` forwards the user's explicit declaration through the daemon's existing `grouping.rs` socket; the Workspace detail screen gained a "Related work — your call" section that offers only witnessed subjects outside the current Workspace, reflects the daemon's honest response verbatim, and never infers relatedness.
- **D3 — Layer-3 contradiction resolved by document decision.** `ARCHITECTURE.md:211`'s cap sentence was amended: the Continuation Surface (RFC-0013) is Evo's Layer 3, deliberately uncapped (a cap would silently decline to reopen user-declared resources — Law IX). No new evidence class was invented; the Context Chain stays empty by contract (IS-0021 §25.3).
- **D4 — Workspace↔Workspace relationships deferred, deliberately.** Not implemented. `ARCHITECTURE.md:151` mandates the link; it is the reconciliation of CONFLICT N; but it is a full specification (evidence classes, strength semantics, storage, replay) and is not needed for the proliferation or fragmentation fixes. Recorded here as the named next layer and as the subject of a future RFC.

**BLOCKED (deferred):** Workspace↔Workspace relationship/affinity architecture.

**WHY:** it requires a new evidence class, strength semantics, storage, and replay rules that no existing authority document specifies; inventing them would violate the mission's prohibition on new canonical semantics.

**SOURCE:** `ARCHITECTURE.md:151` (mandate), `BE-TRACE-0001 §3.3` (Layer 3 gap), CONFLICT N (unreconciled).

**MUST NOT change (unchanged in this phase):** CONSTITUTION, PRODUCT, the Architectural Laws, RFC-0001–0013 semantics (except the two minimal D1/D3 reconciliations above), the five primitives, Observation immutability, determinism and replay, and local-first operation.
