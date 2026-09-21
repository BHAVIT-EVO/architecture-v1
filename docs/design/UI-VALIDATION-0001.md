# UI-VALIDATION-0001 — Phase 0 Correctness + Architecture Validation of UI-IA-0002

**Status:** Review Draft — awaiting review. No code has been written.

**Authority:** The frozen documents and accepted contracts only. Neither the current UI nor UI-IA-0002 is treated as authority anywhere in this report.

**Verdict in one line:** five UI defects and one backend defect are real and should be fixed now; most of UI-IA-0002's information architecture is safe; its single load-bearing mechanism — §3 arity-dispatched commit — cannot be proved safe and is rejected, and one of its premises (that the Continuation Surface determines the executable target) is ahead of contract and needs an amendment before Phase 2.

---

## 0. Method and scope

**Read in full as authority:** ARCHITECTURAL_LAWS (Frozen v1.0), IS-0011 Workspace Model (Frozen), IS-0019 Restoration Model (Frozen), IS-0021 Restoration Derivation §25 (Frozen, as amended), RFC-0006 Restoration Contract (Accepted v1.0), RFC-0011 Continuation Evidence Contract / Work Designation (Accepted v1.0), RFC-0013 Continuation Surface Declaration (Accepted v1.0).

**Read as proposals under review, not as authority:** UI-IA-0002, UI-REVIEW-0001.

**Code:** read-only inspection of `crates/evo-desktop/src/{app.rs, state.rs, theme.rs, daemon.rs}`, `crates/evo-daemon/src/{runtime.rs, cache.rs, persistence.rs, validation.rs, continuation.rs, ui.rs, daemon_status.rs}`, `crates/evo-restoration/src/derivation.rs`, `crates/evo-workspace/src/formation.rs`.

**No file in the repository was created, modified, or deleted while producing this report**, other than this report itself.

**The interface's latitude, and its boundary.** RFC-0006 states that it does not define "UI design… ordering algorithms… ranking algorithms… interaction design", and RFC-0013's Non-Goals list ends with a bare "UI". The interface therefore has real design freedom. That freedom is bounded on one side by RFC-0006 Requirements 1–7, which are behavioural and binding on any surface the user sees, and on the other by the prohibition on presenting a relation the contracts do not establish. Sections B, D and E work inside those bounds; section C marks where UI-IA-0002 crosses them.

### 0.1 What "the six correctness bugs" resolves to

UI-REVIEW-0001 Appendix A registers **five** UI bugs (B-01 … B-05) and **four** backend findings (BE-01 … BE-04). The brief names BE-01 (item 4), the vanishing execution report (item 5, which is B-02) and the `designation_card` mismatch (item 6, which is B-05) as separate items. The six is therefore read as **B-01, B-02, B-03, B-04, B-05, BE-01**. BE-02 is a sixth genuine defect and is included as a candidate; BE-03 is not a defect (it is unused sanctioned copy); BE-04 is not a defect (it is a real architectural gap that the design must be honest about).

### 0.2 Corrections to the record

Five statements in the two review documents are inaccurate. They are corrected here so that the design is not built on them.

| Claim in review docs | Actual |
|---|---|
| `continue_plan_line` lives in `state.rs` (UI-IA-0002 §7.2/§7.4 imply this) | `app.rs:2067–2120`. It is presentation code, and it does **not** carry `run_execution`'s honest-refusal branch. |
| Home is width-constrained to 640 and Detail is full-bleed (UI-REVIEW-0001 D-13) | Directionally right, mechanism different: `CONTENT_WIDTH` is applied at exactly one site, `app.rs:454` (Home). Detail never applies it — it uses `theme::detail_split` over `ui.available_width()` (`app.rs:687–712`), leaving `gap`/`side` unused in the narrow arm. |
| `runtime.rs` handlers at `:557 / :594 / :637` | Those are body lines. Signatures: `handle_observation` `:478`, `handle_continuation_surface` `:593`, `handle_designation` `:630`. Dispatch `:500–507`. |
| B-04: "two ids" from one salt (in-code comment at `app.rs:2159–2161` claims the two sites share one animation) | The salt string is identical but the parents differ: `ui.id()` at `app.rs:542` is the top-level `Ui`; at `app.rs:2162` it is the `scope_builder` child created at `app.rs:2154`. Two distinct `egui::Id`s, two independent animation states, driving `bar_h` and `report_h` out of phase. **The in-code comment is false.** |
| D-38: `select_workspace_with_designation` is unused | Stronger. It **is** called, at `state.rs:107`, and called *with `None` for the `selected` argument* — i.e. invoked specifically so that the designation decides the workspace — and `EvoApp::reload` then discards the field it fills (`app.rs:203–212`, `display` and `selection` fall under `..`). The one call site exists only to exercise the designation preference, and its result is thrown away. |

### 0.3 One finding in neither review document

`CanonicalIndex::current_continuation_surface_artifacts()` (`cache.rs:224–245`) resolves each declared subject through `resolve_designated_artifact` (`cache.rs:202–207`) and **silently skips** any subject that resolves to zero or to more than one Artifact. A user can therefore declare a five-resource surface and be shown three, with no statement anywhere that two were dropped or why. The UI compounds this: no render site displays the declared subject list at all — `restoration_selection_card` shows only the per-workspace derived set (`app.rs:845–846`), and nothing shows the difference between what was declared and what survived. This violates RFC-0006 Requirement 7 (*"If questioned, Evo SHALL explain why the restored context was selected"*). It is a **presentation** defect, not a resolution defect — the skipping itself is correct per RFC-0011 §4, which forbids ambiguous resolution. The fix belongs in section D, not section A: surface the drop, do not change the resolution.

---

## A. Bugs that are unquestionably real and should be fixed now

All six fixes below are local, introduce **no new canonical object, no new canonical field, and no new persisted state**, and leave the three canonical write paths (`submit_designation`, `submit_continuation_surface`, `submit_grouping`) untouched in number and in semantics.

| # | Defect | Evidence | Contract / law | Minimal fix |
|---|---|---|---|---|
| A.1 | **B-01 + B-02** — a canonical reload destroys user-authored transient state | `app.rs:300–320` | RFC-0006 R-4, R-7; IS-0011 W-28/W-29 | Remove three assignments; scope each field's lifetime to an explicit user act |
| A.2 | **B-05** — displayed resource and designated resource can diverge | `app.rs:1290–1294`, `1350–1365` | RFC-0011 §4; RFC-0006 R-7 | Build one paired vector in a single `filter_map` |
| A.3 | **B-03** — report box under-allocates its own height | `app.rs:2283–2293` | RFC-0006 R-7 (an unreadable explanation is not an explanation) | Allocate from measured content |
| A.4 | **B-04** — two independent animations drive one bar | `app.rs:542–550`, `2162–2170` | none (pure correctness) | One id computed once, passed down |
| A.5 | **BE-01** — a designation write durably erases the Continuation Surface | `runtime.rs:630`/`:637` vs `:557` and `:594` | IS-0021 §25 input list + §25.9 + §25.10; RFC-0013 Supersession + Negative Case 10 + Replay; IS-0019 RP-4, RM-6; Law VII | Pass the current surface, exactly as the two sibling handlers do |
| A.6 | **BE-02** — daemon status drops two sources it writes | `runtime.rs:283–294` vs `daemon_status.rs:120–123` | RFC-0006 R-7 | Add the two source strings to the whitelist |

### A.1 B-01 and B-02 are one bug, and it is the most important one to fix first

```rust
// app.rs:310-320
if signature != self.last_signature {
    self.last_signature = signature;
    self.execution_result = None;
    self.continuation_draft = None;
    self.continuation_status = None;
    log_state_change(&self.storage_root, display.as_ref());
}
```

`state_signature` is `(workspace_id, snapshots().len())` (`app.rs:357–359`). The signature therefore changes whenever a new Snapshot lands — which is exactly what happens when Evo opens or focuses a window, because focus is witnessed. This is polled every `RELOAD_PERIOD` (2s). The consequences:

- **B-02:** pressing Continue opens windows → focus is observed → a Snapshot is formed → the signature changes → `execution_result = None`. Success accelerates its own erasure. The more Evo opens, the faster the honest report of what it opened disappears. Typical lifetime: one poll interval.
- **B-01:** the same tick destroys `continuation_draft` mid-edit. A user assembling a set of resources loses the set because unrelated background activity was witnessed.

**Why this is a contract matter and not a polish matter.** RFC-0006 R-4 requires Restoration to *preserve the user's authority over the work* — a draft the user is authoring is the user's work, and background observation is not authorised to discard it. R-7 requires every restoration presented to the user to remain explainable; an explanation that deletes itself in two seconds has not been presented. And the reason there is no legitimate alternative fix — persisting the report — is IS-0011 W-28/W-29: a Snapshot SHALL NOT contain user interface state and SHALL NOT contain RestorationPlan semantics. There is nowhere canonical to put it, and correctly so.

**Minimal fix.** Delete the three resets. Give each field an explicit, user-owned lifetime:

| Field | Created by | Ends on | Never ends on |
|---|---|---|---|
| `execution_result` | pressing Continue | dismiss (✕), the next Continue, or leaving the workspace | any reload, any new Snapshot |
| `continuation_draft` | first mark in the declaration control | commit, explicit cancel, or leaving the workspace | any reload, any new Snapshot |
| `continuation_status` | a commit attempt returning | the next commit attempt, dismiss, or leaving the workspace | any reload, any new Snapshot |

`ExecutionReport` already owns its attempts, so it remains a truthful record of a past act with zero canonical persistence. Keep `log_state_change` on the signature edge — logging *is* reload-driven.

**The rule this encodes, which section D generalises:** a canonical reload may replace **derived** state; it may never touch **user-authored transient** state. That single sentence resolves both bugs and prevents their whole family.

**Staleness, handled honestly rather than by deletion.** The brief requires the result to remain "an honest presentation of the action that just occurred even if the newly opened resources generate new observations." Deleting it was the current answer to staleness. The honest answer is to say so: the report carries the time of the act, and when the signature has moved since, it says that new activity has been witnessed. That is a statement about observation, made in presentation, persisted nowhere.

### A.2 B-05 — `designation_card` can designate a resource other than the one it names

```rust
// app.rs:1290-1294
let candidates = state::distinct_artifacts_in_latest_snapshot(workspace);
let candidates_subjects: Vec<String> = candidates
    .iter()
    .filter_map(|id| state::subject_for(subjects, id).map(str::to_string))
    .collect();
// app.rs:1350
for (id, subject) in candidates.iter().zip(&candidates_subjects) {
```

The label comes from `id` (`app.rs:1351`, `describe_artifact`); the canonical write takes `subject` (`app.rs:1362–1364`, `record_designation`). `filter_map` shortens the second vector; `zip` then pairs positionally. If any candidate has no witnessed subject, every row after it is mislabelled — the button designates a *later* resource than the one named — and the trailing candidates silently vanish, because `zip` stops at the shorter vector. The same misaligned vector drives the `belongs` check at `app.rs:1297` and the `is_current` check at `app.rs:1352–1354`.

This is the worst defect in the set, because the write on the other side is canonical, append-only, and **unrevokable** (BE-04). A mislabelled click is a permanent false declaration in the Observation log about where the user's work continues.

Contract: RFC-0011 §4 requires the producer to offer only subjects already witnessed and to resolve before writing — the whole point being that what the user picks is what gets designated. RFC-0006 R-7 is violated because the presented restoration is not explainable — it is wrong. This is also the most literal available breach of the brief's *identity ≠ application* principle: an `ArtifactId` names one thing while a subject string designates another. (That principle is not itself one of the seventeen Architectural Laws — Law XVI, the Identity Law, governs whether a concept may become a computational object at all, which is a different question. The RFC-0011 and R-7 citations carry this defect on their own.)

**Minimal fix.** One vector, one pass, no positional pairing:

```rust
let candidates: Vec<(ArtifactId, String)> = state::distinct_artifacts_in_latest_snapshot(workspace)
    .into_iter()
    .filter_map(|id| state::subject_for(subjects, &id).map(|s| (id, s.to_string())))
    .collect();
```

Label and submitted subject then come from the same tuple and divergence becomes structurally impossible. Dropping rows with no witnessed subject is correct — RFC-0011 §4 forbids offering them — but per §0.3 the drop should be *stated*, not silent.

### A.3 B-03 — the report box is smaller than the report

`continue_bar_expanded_height` returns `64.0 + attempts * 24.0` (`app.rs:2283–2293`), used at `app.rs:549` and `app.rs:2168`. An attempt line that carries a reason wraps past 24px. The honesty artifact is clipped precisely when it has the most to say — an unavailable resource with an explanation is exactly the case that needs the extra line. Fix: allocate from the laid-out galleys, or measure the content, rather than assuming one line per attempt.

### A.4 B-04 — one bar, two animations

Both sites use the salt `"evo-continue-expand"` but different parents (`app.rs:542` top-level `Ui`; `app.rs:2162` the child from `app.rs:2154`), so `bar_h` and `report_h` are driven by two independent `animate_bool_with_time` states and can disagree during the transition. Fix: compute the id and the factor once in the outer scope and pass both down. Also delete the comment at `app.rs:2159–2161`, which asserts the opposite of the truth.

### A.5 BE-01 — the contract test the brief asked for

**The behaviour.** Three sibling handlers in `runtime.rs` build the restoration input. Two read both canonical evidence classes; one does not.

| Handler | Reads designation | Reads surface |
|---|---|---|
| `handle_observation` (`:478`) | yes | **yes** — `current_continuation_surface_artifacts()` at `:557` |
| `handle_continuation_surface` (`:593`) | yes | **yes** — same call at `:594` |
| `handle_designation` (`:630`) | yes | **no** — `RestorationInput::new_with_designated(...)` at `:637` |

`new_with_designated` (`derivation.rs:161–167`) forwards `None` as the surface; `derive_workspace_surface` returns `Vec::new()` for `None` (`derivation.rs:640–643`). The empty-surface outcome is then persisted (`runtime.rs:648`) and upserted into the cache (`:652`), and `cache.rs:529–540` (`tail_restorations`) reads outcomes back **from the persisted restoration log rather than re-deriving them**. The erasure is therefore durable, not a transient display glitch: submitting a designation deletes the user's declared Continuation Surface from every subsequent read until some other event happens to re-derive it.**Do the frozen contracts clearly require the fix?** Yes — four independent requirements, of which the first is decisive.

1. **IS-0021 §25, input list (line 669).** Derivation "SHALL consume only: … *the Current Continuation Surface established by the latest valid ContinuationSurface declaration (RFC-0013), when it exists*, presented as canonical Artifact-level understanding." When a valid declaration exists, the Current Continuation Surface exists, and derivation is required to consume it. `handle_designation` runs derivation against an input in which it does not exist. This is not a determinism side-effect; it is a direct violation of the specified input boundary. §25.1A then defines `surface(W)` as the intersection over that input — an intersection taken against `None` is not the specified function.

2. **RFC-0013 Supersession + Negative Case 10.** Supersession is defined so that "a later declaration replaces the Current Continuation Surface" — a *ContinuationSurface* declaration. A `WorkDesignated` observation is not one. Negative Case 10 states it explicitly: *"A WorkDesignated does not imply a surface; a surface does not imply a designation."* A designation write that empties the surface makes a designation imply a surface — the empty one.

3. **Replay determinism.** RFC-0013's Replay clause requires that "the Current Continuation Surface resolves identically; per-Workspace surfaces derive identically; Restoration outcomes are identical." IS-0021 §25.9 forbids any derivation decision depending on process or runtime state; §25.10 requires replay to produce the same plan. IS-0019 RP-4 requires determinism; RM-6 requires replayability. Today the derived surface depends on *which handler last fired* — a property of the live event sequence, not of the canonical log. Replaying the same log through `handle_observation` yields a non-empty surface; the live path through `handle_designation` yields an empty one. Two different outcomes from one log.

4. **Law VII.** "Because evidence is immutable and interpretation is disposable, the complete interpretation layer must be reproducible." A persisted interpretation that cannot be reproduced from the evidence is exactly what Law VII forbids.

**Minimal fix.** In `handle_designation`, pass `Some(self.index.current_continuation_surface_artifacts())` instead of relying on the `None`-defaulting constructor — byte-for-byte the pattern already used at `runtime.rs:552–563` and `:593–621`. No new state, no new field, no schema change, no change to any validation rule. The three handlers become consistent, which is the correct shape: **the restoration input is a function of the canonical log, not of which write happened to arrive last.**

**Recommended alongside it (not required by contract):** `tail_restorations` reading persisted outcomes rather than re-deriving is what made this durable. It is a legitimate persistence optimisation under IS-0011 §5 ("Persistence exists solely as an implementation optimization… SHALL NEVER become the source of truth"), but it means any future input-boundary bug also persists. A replay-equivalence test — derive from the log, compare to the persisted outcome — would catch the whole class. That is a test, not a contract change.

### A.6 BE-02 — the shell cannot observe that a declaration was accepted

`runtime.rs:283–294` writes status sources `grouping` and `continuation-surface`; the whitelist at `daemon_status.rs:120–123` omits both and drops them. This is why declaration feedback in the UI is a local `Option<(String, bool)>` instead of derived state. Two strings. RFC-0006 R-7 argues for fixing it, and section D depends on it: the commit confirmation should be derived from the daemon's acceptance, not from the UI's optimism about its own write.

### A.7 BE-03 is not a bug — it is unused sanctioned copy

`SURFACE_DECLARATION_REASON` (`derivation.rs:81–86`) has exactly one occurrence repository-wide: its own definition. *"The user declared these resources as the resources their work currently continues across."* That is the correctly-voiced sentence for the exact moment the UI currently improvises. Render it, second-personed. Free authority.

### A.8 Not fixed, deliberately

- **BE-04, no revoke path.** Confirmed by exhaustive grep: no `revoke`, `unmark`, `clear_designation`, `clear_surface`, `Revoked`, `undesignate`, `remove_designation`, `retract`, `withdraw`, `cancel` anywhere. This is not a defect to patch — adding one would be a new canonical evidence class, which requires an RFC. It is a constraint the interface must be honest about, and section C.5 and section E treat it as one.
- **D-38, the dead designation-preferring selector.** `select_workspace_with_designation` should not be deleted and should not be silently wired up. Section C.2 explains why wiring it up as written would introduce a guess.
- **§0.3, silent surface shrinkage.** Presentation fix, section D.

---

## B. Proposed IA changes that are safe

"Safe" here means: supported by, or at minimum not in tension with, a specific clause of a frozen or accepted document; requiring no new canonical object, field, or write path; and preserving replay.

| Proposal | Contract support | Note |
|---|---|---|
| **B.1** Four levels of hierarchy (bodies of work → one body of work → declaration → execution) | RFC-0006 R-3 (minimise reconstruction cost), R-5 (progressive), IS-0019 RM-7 | RM-7 names cognitive-reload minimisation the *primary optimization objective*. Progressive disclosure is the direct interface expression of it. |
| **B.2** Home presents bodies of work, not architecture | **RFC-0006 R-2: "Restoration MUST NOT exist solely to expose architectural state"** | This is the strongest single clause in the review's favour and neither review document cites it. It converts "the UI reads like a SaaS dashboard" from an aesthetic complaint into a compliance finding. Workspace ids, snapshot counts, attachment counts and confidence values on a primary surface are architectural state exposure. |
| **B.3** Name resources by human locator, never by `ArtifactId` | IS-0011: Workspace Identity "SHALL NOT encode meaning"; Snapshot Identity "SHALL NOT encode semantic meaning"; RFC-0006 R-2 | An id shown to a user is read as a name. It has no meaning to encode, and showing it is architectural-state exposure. |
| **B.4** Remove Attachment Confidence from all user-facing surfaces | **IS-0011 W-7: "Confidence SHALL represent evidential strength only. Confidence SHALL NEVER represent importance, priority, or value"**; IS-0021 §25.1 repeats the prohibition | A number next to a resource is read as importance no matter what the label says. Not showing it is the only compliant option. |
| **B.5** Demote Snapshot history to an explanation disclosure | **RFC-0006 R-1: "Historical Understanding MAY be consulted only for explanation. Historical Understanding MUST NOT determine the current restoration"**; RFC-0005 | A snapshot timeline is legitimate *as explanation*. As a primary surface it invites the user to treat history as the thing being restored. `MAX_RENDERED_SNAPSHOTS = 50` (`app.rs:34`) is a symptom of it having become primary. |
| **B.6** Draft-then-commit for the surface, with explicit commit | RFC-0006 R-4 (user agency); RFC-0011 §4 (producer rejects before writing) | Canonical, unrevokable writes must never be a side effect of a click that also does something else. |
| **B.7** One primary action, named in the user's words | Product ("Hey Evo, continue my work"); RFC-0006 R-4 ("Evo assists. The user acts.") | Continue stays the single primary act and stays explicit. Never automatic. |
| **B.8** Show designation provenance (which resource, when) | RFC-0011 "its staleness is made visible by carrying provenance (time) into presentation"; RFC-0006 R-7 | Already implemented correctly at `app.rs:1299–1306` and `1316–1322` via `state::local_time_label`. Keep it; extend the pattern. |
| **B.9** Honest refusal as a first-class state | RFC-0006 R-5; IS-0021 §25.8 ("Insufficient canonical evidence SHALL be treated as a normal and explicit… outcome"); Law V, Law VI | `run_execution` has this branch; `continue_plan_line` (`app.rs:2067–2120`) does not. Both must call one shared function so the plan line and the execution can never disagree about whether anything can be opened. |
| **B.10** Never show the words "designation", "continuation surface", "artifact", "attachment", "snapshot" | RFC-0006 R-2 | R-2 carries this alone. The concepts must be *expressible* by the user in their own language, not taught as vocabulary. (Law XVII is about witnessed-vs-inferred separation, not vocabulary, and is not cited here.) |
| **B.11** Render `SURFACE_DECLARATION_REASON` verbatim (second-personed) | RFC-0013; BE-03 | Sanctioned copy for the exact moment. |
| **B.12** Keyboard access to the primary path | none required — no contract addressed | Neutral. Safe. |
| **B.13** The material/elevation system (M0–M4) | none required — no contract addressed | Neutral, and explicitly deferred by the brief ("do not modify the visual design yet"). Safe to adopt later. |
| **B.14** Explicit ordering rule for the Home list, stated to the user | RFC-0006 R-7; RFC-0006 explicitly does **not** define ordering or ranking algorithms | The UI may choose an order. It must be able to say what the order is. See C.2 for the order it must **not** choose silently. |

**One correction inside a safe proposal.** UI-IA-0002 §11.6 states that `context_chain` and `blockers` are "empty by contract." That is right, and the citation is stronger than given: IS-0021 §25.3 ("If no canonical supporting relationship exists, the Context Chain SHALL be empty"), §25.4 ("the current canonical derivation SHALL produce zero Blockers"), and IS-0019 BL-3 ("Restoration SHALL NOT invent Blockers"). They are hard-empty in code at `derivation.rs:528–536`. The interface must therefore not contain a region shaped like a blocker list or a context list — an empty region shaped like a feature reads as a broken feature, and worse, invites someone to fill it with inference later.

---

## C. Proposed IA changes that conflict with, or risk changing, Evo's frozen semantics

### C.1 §3 arity-dispatched commit — rejected, with proof of ambiguity

The brief's instruction: *"Do not merge Designation and Continuation Surface semantics unless you can prove the interaction preserves both canonical operations without introducing ambiguity."* The proof fails. Here is the failure, four ways.

**The proposal.** One list of resources, one set of marks, one commit button. `marks == 1` → `submit_designation`. `marks >= 2` → `submit_continuation_surface`. `marks == 0` → inactive.

#### C.1.1 The mapping is neither injective nor surjective

Let *M* be the mark set in the unified control, *d* the Current Designation, *S* the Current Continuation Surface. Both *d* and *S* may be non-empty **simultaneously and independently**. RFC-0013 states this twice, in two different sections: Negative Case 10 — *"A WorkDesignated does not imply a surface; a surface does not imply a designation"* — and Declaration Semantics — *"A declaration never requires a simultaneous WorkDesignated, and WorkDesignated never implies a surface."* Canonical state is the pair `(d, S)`. The control's state is the single set *M*. Then:

- **|M| = 1.** Writes `d := s`. *S is unaffected* — RFC-0013 Supersession replaces the Current Continuation Surface only on a later *surface* declaration. Post-commit canonical state is `(s, S)` with `|S| ≥ 2`. The control now displays one mark while canonical state names up to *n* resources, possibly none of them *s*. **The control's display is false.**
- **|M| ≥ 2.** Writes `S := M`. *d is unaffected* — same Negative Case, in the other direction. Post-commit state is `(d_prev, M)`. If `d_prev ∉ M`, then the resource that establishes the **Resume Point** (IS-0021 §25.2) and the **Next Step** (§25.5) is neither displayed nor reachable through the only control that exists. **The user's resume point becomes invisible and unmanageable.**
- **|M| = 0.** No write exists. There is no revoke path (BE-04, verified). So the control can *display* a state — nothing marked — that it can never *commit*, while canonical state may be `(d, S)` with both non-empty. **A displayable state with no canonical counterpart.**

Three distinct divergence modes. The map `M → (d, S)` is not injective (many *M* produce the same pair) and not surjective (most reachable pairs are unrepresentable as one *M*). Ambiguity is not merely introduced; it is unavoidable. The proof obligation cannot be met.

#### C.1.2 With BE-01 fixed, arity dispatch stops doing what the proposal claims

`state::run_execution` (`state.rs:740–796`) gives a non-empty surface **strict precedence** over the designation:

```rust
if !outcome.continuation_surface().is_empty() { execute_selection(...) }
else { ExecutionRequest::from_derivation(outcome) }
```

Today, marking one resource appears to change what Continue opens — but only because BE-01 wipes the surface on the way through. Once BE-01 is fixed (A.5, which the contracts require), a 1-mark commit leaves the surface intact, the surface keeps precedence, and Continue reopens **the whole surface**. The single control would show exactly one mark and then reopen five things. The proposal's central mechanism depends on the bug it also asks to fix. This interaction is not anticipated anywhere in UI-IA-0002.

**Scope of the claim, stated precisely.** The precedence branch tests `outcome.continuation_surface()`, which is the **per-workspace** derived surface, not the global declaration. RFC-0013 Insufficiency: *"A declaration referencing artifacts outside W → W's surface is empty; the declaration is not an error."* So when the declared surface does not intersect the workspace in view, `surface(W)` is empty, the `else` branch runs, and the designation does govern execution. C.1.2 therefore holds **whenever the declared surface intersects the workspace being viewed** — which is the ordinary case, because the user assembles a surface from within a body of work. The consequence for the design is unchanged and arguably worse: with one merged control, whether a 1-mark commit changes what Continue opens depends on an intersection the user cannot see. That is a second, independent ambiguity.

#### C.1.3 The merged control can never truthfully say "just this one"

There is no revoke, and `submit_continuation_surface` rejects any set smaller than two — enforced twice, at `validation.rs:207–215` and `continuation.rs:144–151`, the latter with the message *"a continuation surface must name at least two distinct subjects; a single-subject continuation declaration belongs to the designation contract (RFC-0011)."* Therefore once a surface exists, it can be **replaced** but never **reduced to one** and never **cleared**. A control whose entire premise is "the number of marks expresses what I want" cannot express the most common intent a user will have — "actually, just this one thing" — because that intent has no canonical write.

#### C.1.4 It builds the interaction that RFC-0013 placed out of scope

RFC-0013 Open Question 2: *"Surface + designation coherence. The RFC treats WorkDesignated and the surface as independent inputs… A future amendment could define a combined declaration (primary + supporting resources) — that would change RFC-0011 and is explicitly out of scope here."*

Arity dispatch is a combined declaration. Keeping two write calls underneath does not change that: the user makes one gesture, sees one control, and forms one mental model — "I am declaring what I'm working on." That model is the combined declaration. Law IV (observation ≠ interpretation) and Law XVII (epistemic separation) both hold that presentation may not assert a relation the contract does not establish. RFC-0006's Forbidden Behaviour list adds "replace user decision-making": inferring *which of two canonical operations the user meant* from a count is the interface deciding, not the user.

#### C.1.5 The strongest case for the merge, and why it does not rescue the design

In fairness, the merge has real support. RFC-0013's own cardinality rationale says *"a single-subject continuation declaration is already the RFC-0011 WorkDesignated contract, and a second channel expressing the same single-subject fact would create two competing authorities for one claim."* The daemon's rejection string says the same. Read plainly, the two contracts look like two arities of one user-level concept — *what my work continues at, or across* — which is precisely the intuition UI-IA-0002 §3 is built on. That intuition is not wrong, and it is the reason the proposal is attractive rather than careless.

It does not rescue the design, because arity-continuity of the *concept* does not give arity-continuity of the *operation*. The two writes differ in supersession scope (each supersedes only its own class), in derivation role (the designation establishes the Resume Point and Next Step; the surface establishes neither — IS-0019 CS-3), and in mutual effect (none, in either direction). A control may borrow the user's single question. It may not borrow a single commit.

#### C.1.6 The thesis error underneath

UI-IA-0002 rests on the claim that the user has "one question — what am I continuing?" Restoration has **two** outputs, and the contracts make them exactly two: **exactly one** Resume Point (IS-0019 RSP-1; §4 "contain exactly one Resume Point") and **a set** to bring back (`surface(W)`, IS-0021 §25.1A, feeding the RestorationSelection). The user has two questions:

1. *Where do I pick up?* — one answer, single-choice → designation.
2. *What do I want back around me?* — many answers, at least two → surface.

A design that presents one question will always be short one answer. Section D presents two.

### C.2 Home hero chosen by the designation — conditionally unsafe as written

UI-IA-0002 §2.3 proposes deriving the Home hero from `select_workspace_with_designation`. As written (`state.rs:325–346`) that function scans workspaces linearly and returns the **first** whose `attachments()` contain the designated Artifact. Three problems:

1. **It is a guess when the artifact co-belongs.** Nothing in IS-0011 prevents one Artifact being attached to more than one Workspace — Attachments are per-Workspace evidence of membership, and the function's own first-match scan presupposes that several may match. On a tie it silently takes the first element of `CanonicalIndex::workspaces()`, i.e. canonical-log append order (`cache.rs:171`, `persistence.rs:695–706`) — earliest-appended, not latest, not id-ordered. Log append order carries no semantics (IS-0011: Workspace Identity "SHALL NOT encode meaning"). Presenting that choice as "where your work continues" is an unwitnessed claim. IS-0021 §25.1 forbids "hidden ranking criteria"; §25.2 forbids resolving multi-candidate cases by ordering alone.
2. **It tests membership against the wrong set.** It checks `workspace.attachments()` — the cumulative, never-pruned, all-history Attachment Set — while `designation_card` tests belonging against `distinct_artifacts_in_latest_snapshot` (`app.rs:1290`, `state.rs:464–470`), a strictly narrower set. Two surfaces would answer "does this designation belong here?" differently. IS-0021 §25.7 requires cross-component consistency; RFC-0006 R-1 requires operating on the *current* committed understanding, with history for explanation only.
3. **The fallback is a ranking.** `select_display_workspace` → `max_by(compare_workspaces)` on `(lifecycle_rank, latest_snapshot_time, id_string)` (`ui.rs:231–245`). Ordering by latest witnessed time is defensible as a *fact about observations*; the `id` tie-break is not defensible as *meaning*, and IS-0011 §7/§10 place Workspace ranking outside the Workspace layer entirely.

**Safe version.** Home may lead with the designated body of work **only when the designated Artifact resolves into exactly one remembered body of work**, and only when the row says why in the user's words with the time of the declaration. When it resolves into more than one, Home must show them as several and ask — that is the "no guessing" law applied at the interface. When it resolves into none, Home falls back to a stated order (most recently witnessed) and says that is the order. Membership must be tested against one set, chosen once, everywhere.

### C.3 Treating the Continuation Surface as the executable target — the code is ahead of contract

This is the most consequential finding in the report, because UI-IA-0002 §3 depends on it and because it is already shipped behaviour.

Four separate clauses say the surface does not select what gets opened:

- **IS-0019 CS-4:** *"The Continuation Surface SHALL NOT change execution ordering or selection; it never authorizes opening, focusing, or launching any resource."*
- **IS-0021 §25.1A:** *"It SHALL NOT change execution ordering or selection; it never authorizes opening, focusing, or launching any resource."*
- **RFC-0013 §Gating Invariants:** *"The surface does not change execution ordering or selection. Execution is downstream and remains out of scope (RFC-0010: Work Continuity does not solve resource identity or executable target identity)."*
- **RFC-0013 §Non-Goals** — the strongest of the four, because it is a statement about authority rather than about behaviour. The RFC *"does NOT define or authorize: … selective restoration execution — opening, focusing, or launching any resource; resource identity or executable target identity."*

And Open Question 3 explicitly **defers** the mapping: *"how execution maps surface members to executable targets (resource identity, launch ordering, failure handling) is the next milestone and requires its own resource/executable identity contract."*

`state.rs:740–796` implements exactly that deferred mapping: a non-empty per-workspace surface takes strict precedence and becomes the execution selection. So the implementation has adopted a semantic that no accepted document defines, that four clauses currently disclaim, and that one Open Question reserves for a contract that does not yet exist.

**Do not "fix" this in Phase 0.** Removing it would break the one behaviour that makes the surface useful, and the brief forbids inventing canonical semantics to suit the UI. The correct handling:

1. **Flag it as an open contract question, not a bug.** It is a deviation of the same class RFC-0013 anticipated and deferred.
2. **Do not build UI that depends on it** until it is ratified. Every part of UI-IA-0002 §3 does depend on it.
3. **Author a short amendment RFC** — the smallest possible: define the surface → RestorationSelection mapping, and amend CS-4 and §25.1A to say the surface does not *by itself* authorise opening but *does* constrain the executable target set when one is requested. That is a one-clause change and it makes the shipped behaviour honest. Until it exists, Phase 2 has no authority.
4. **Meanwhile the interface must not claim the surface opens things.** It may truthfully say the user declared these resources as the ones their work continues across (BE-03 copy), and separately show what Evo will actually attempt.

### C.4 A single undifferentiated list of "marks" — unsafe even without arity dispatch

Even if the merged commit is dropped, a single list with one kind of mark collapses three sets the architecture keeps apart:

| Set | Nature | Owner | Authority |
|---|---|---|---|
| Workspace membership | durable, historical, evidential | derived by Formation | IS-0011: Attachment is "evidential membership… SHALL NOT imply ownership" |
| Continuation Surface | declared, current, ≥2 | the user | RFC-0013 |
| Restoration Selection | derived, execution-facing, now | derivation | IS-0021 §25.1A + RestorationSelection |

RFC-0013 states the distinction as a slogan: *"Same Artifact ≠ Related Work ≠ Currently Relevant ≠ Restore Together."* Law XVI forbids concepts that merely describe or rank from becoming computational objects; the interface analogue is that a checkbox which means three things at once teaches the user a fourth concept that does not exist. The three sets need three visual languages — which UI-IA-0002 §2.2 itself proposes, correctly, and then §3 undoes.

### C.5 Any affordance implying an inverse that does not exist

There is no revoke. An unmarkable checkbox, a greyed "clear", or a mark that appears to toggle off teaches the user that declarations are reversible. They are not: the log is append-only, and the only way to change a surface is to declare a different one with **at least two** members. The interface must be honest about a one-way door — not by refusing the action, but by saying what the action does: *this replaces your current set*. UI-IA-0002 §11.1 raises this and leaves it open; it cannot stay open, because it determines the shape of the control.

### C.6 Silent surface shrinkage must be surfaced, not repaired

Per §0.3: declared subjects that fail resolution are silently dropped (`cache.rs:224–245`), and no UI surface shows the declared set at all. Declared subjects that belong to another body of work are also dropped from this one — and RFC-0013 is explicit that this is normal: *"A declaration referencing artifacts outside W → W's surface is empty; the declaration is not an error."* Normal is not the same as invisible. RFC-0006 R-7 requires that Evo be able to explain why the restored context was selected. The resolution behaviour is correct and must not change (RFC-0011 §4 forbids ambiguous resolution). What must change is that the interface says so: *you named five, Evo can account for three here, and here is where the other two went*. No new state — the difference is derivable at render time from the declared subject list (`cache.rs:218–222`) and the resolved set (`:224–245`), both already available in `CanonicalIndex` and neither currently referenced by the desktop.

### C.7 Summary of C

| Proposal | Verdict |
|---|---|
| §3 arity-dispatched commit (one list, one mark set, one button) | **Reject.** Proof obligation fails (C.1.1); depends on BE-01 remaining unfixed (C.1.2); cannot express "just this one" (C.1.3); constructs the combined declaration RFC-0013 deferred (C.1.4). |
| Home hero from the designation | **Conditional.** Safe only with single-workspace resolution, one membership test, and a stated order (C.2). |
| Surface as executable target (an unstated premise of §3) | **Blocked pending amendment.** Three frozen clauses disclaim it; the code already does it (C.3). |
| One undifferentiated mark type | **Reject.** Collapses three architecturally separate sets (C.4). |
| Reversible-looking declaration controls | **Reject.** No canonical inverse exists (C.5). |
| §11.1 leaving revoke "open" | **Must close now** — it determines the control's shape. |

---

## D. The minimum UI architecture we should implement

Five rules, one state table, one module boundary. Nothing here adds a canonical object, a canonical field, or a fourth write path.

### D.1 Four state classes, and one law about the boundary between them

| Class | Examples | Lives where | May be replaced by a reload? |
|---|---|---|---|
| **Canonical** | Observations, Artifacts, Workspaces, Attachments, Snapshots, the two declarations | the append-only log | n/a — never written by the UI except through the three paths |
| **Derived** | `RestorationOutcome`, `RestorationSelection`, `surface(W)`, the designation tuple, preflight readiness | recomputed from canonical input | **Yes.** This is what reload is for. |
| **Transient, user-authored** | `continuation_draft`, `execution_result`, `continuation_status`, `designation_status`, scroll position, expanded disclosures | UI memory only | **Never.** |
| **Ephemeral** | animation factors, hover, focus ring | egui frame state | irrelevant |

> **The law:** a canonical reload may replace derived state; it may never touch user-authored transient state.

This one sentence is the whole fix for B-01 and B-02 and it forecloses their entire family. It is also the reason no UI state needs to become canonical, and two frozen clauses say so directly: IS-0011 W-28/W-29 forbid a Snapshot from holding interface state or RestorationPlan semantics, and Law XVI's remedy clause states that concepts which merely describe or relate *"SHALL instead be represented as relationships, derived values, or transient computational state."* The fourth class in the table above is the architecture's own prescribed home for this state. RFC-0013's Non-Goals add the same point for the surface itself: no *"new persisted canonical object beyond the Observation Schema (the surface in the plan is derivation output, not new canonical state)."*

### D.2 Two declaration affordances, never one commit

One list of resources. One row per resource. **Two visually and verbally distinct affordances per row**, and two separate commits:

| | "Pick up here" | "Bring these back together" |
|---|---|---|
| Cardinality | single-choice (radio semantics) | multi-select, **≥ 2** |
| Commit | immediate on selection | explicit button over a draft |
| Canonical write | `submit_designation` | `submit_continuation_surface` |
| Establishes | Resume Point (§25.2), Next Step (§25.5) | `surface(W)` (§25.1A) |
| Effect on the other | **none** (RFC-0013 Negative Case 10) | **none** |
| Shown as current | the marked resource + when it was marked | the declared set, plus what could not be accounted for |

They coexist on the same row because they concern the same resource; they are never the same mark, never the same colour-and-shape, and never the same commit. The words "designation" and "continuation surface" never appear. The user reads *pick up here* and *bring these back together*, which is what the two contracts actually mean.

The ≥2 constraint is taught **before** it bites: with one resource selected the commit is inactive and says why — *pick at least two; one on its own is "pick up here", above*. That sentence is a plain-language restatement of RFC-0013's cardinality rationale, so the interface teaches the architecture without naming it.

### D.3 One primary action, explicit, never automatic

**Continue** is the single primary act (Product; RFC-0006 R-4: "Evo assists. The user acts."). Its label states what it will do, in resource counts, never in architecture. When nothing can be opened it is inactive with the reason visible — and `continue_plan_line` and `run_execution` must derive that from **one shared function**, so the promise and the act can never disagree (B.9).

### D.4 Transient state lifetimes, specified

| Field | Born | Dies | Immune to |
|---|---|---|---|
| `execution_result` | Continue pressed | ✕ dismiss · next Continue · leaving the workspace | reload, new Snapshot |
| `continuation_draft` | first multi-select mark | commit · cancel · leaving the workspace | reload, new Snapshot |
| `continuation_status` | commit returns | next commit · dismiss · leaving the workspace | reload, new Snapshot |
| `designation_status` | designation returns | next designation · dismiss · leaving the workspace | reload, new Snapshot |

Staleness is stated, not resolved by deletion: the execution report carries the time of the act and, when the state signature has since moved, one line saying new activity has been witnessed since. Presented, not persisted.

Once BE-02 is fixed, `continuation_status` and `designation_status` should be **derived from the daemon's acceptance** rather than from the UI's assumption that its own write succeeded. Until then they stay transient and must not claim more than "sent".

### D.5 What the interface must always show, and must never show

**Always:**
- provenance on every declaration (which resource, when) — RFC-0011 staleness clause, RFC-0006 R-7;
- the difference between what was declared and what Evo can account for, with reasons — R-7, §0.3;
- the honest refusal, as a normal outcome — IS-0021 §25.8, Laws V and VI;
- what Continue will *attempt*, distinct from what it *achieved*.

**Never:**
- `WorkspaceId`, `ArtifactId`, `SnapshotId`, attachment counts, snapshot counts, Attachment Confidence — RFC-0006 R-2, IS-0011 W-7;
- the words designation, continuation surface, artifact, attachment, snapshot, restoration plan — RFC-0006 R-2;
- an empty region shaped like a feature for Context Chain or Blockers — both are hard-empty by contract (IS-0021 §25.3, §25.4; IS-0019 BL-3);
- colour without words for any status — the implementation already gets this right; keep it;
- an affordance whose canonical inverse does not exist — C.5.

### D.6 Module boundary

Presentation reads **one** derived object per workspace (the restoration outcome plus its selection) and writes through the declaration functions the desktop already owns.

**Correction to a claim in circulation.** The desktop shell exposes **two** canonical write paths, not three: `submit_designation` (`state.rs:228`) and `submit_continuation_surface` (`state.rs:245`). A third canonical declaration channel exists at the contract level — WorkGrouped co-membership, which RFC-0013 refers to as already implemented in the "designation and grouping channels" — but repository-wide grep finds **no** occurrence of `grouping` or `Grouped` anywhere in `crates/evo-desktop`. The desktop has never written it. Phase 1 should not add it: grouping answers "are these related", which is a third question, and RFC-0013 Negative Case 9 is explicit that *"WorkGrouped relatedness does not imply continuation."* Two write paths, two questions, two affordances.

`app.rs` should also stop deriving display selection independently of `state.rs` — today `EvoApp::reload` discards `DisplayState::display` and `DisplayState::selection` (`app.rs:203–212`) and re-derives its own from `self.selected` only (`app.rs:292–296`, `324`, `330–340`), which is how the designation-preferring selector became dead code with a live call site. One derivation, one consumer.

### D.7 Revised phase order

| Phase | Content | Gate |
|---|---|---|
| **0** | A.1–A.6. Fixes only. No IA change, no visual change. | This report reviewed |
| **1** | Section B items + D.1–D.6. Two affordances, four state classes, architectural vocabulary removed, history demoted to explanation. | Phase 0 merged |
| **1.5** | The amendment RFC for surface → executable target (C.3). Smallest possible. | Phase 1 shipped, behaviour observed |
| **2** | Anything that depends on the surface determining what Continue opens. | Amendment accepted |
| **3+** | Visual system (materials, type scale, the fourteen-sizes cleanup). | Phase 1 stable |

UI-IA-0002 §11.3 says BE-01 gates Phase 2. It gates **Phase 0**: until it is fixed, nothing about surface behaviour can be observed truthfully, because every designation write erases the surface.

---

## E. Screen-by-screen layout

No visual design is proposed here — no colour, type, spacing or material decisions. This is content, hierarchy, and the source and write-path of every element.

### E.1 Home — "These are the bodies of work I can return to."

**Content.** A single vertical list of bodies of work. Each row carries three things and nothing else:

1. a human name, derived from the resources of the body of work (locators, never ids);
2. when it was last witnessed, in the user's words;
3. one honest status line — either *you marked where this continues: «resource» (marked at T)*, or *nothing marked yet*.

**Order.** Most recently witnessed, and the interface says that is the order. If the designated Artifact resolves into exactly one body of work, that row may lead, labelled with why. If it resolves into several, none leads and the interface says the mark applies to more than one (C.2).

**Continue on Home?** Only when the designation resolves into exactly one remembered body of work. Otherwise Home has no Continue — a global Continue would have to pick a workspace, and picking without evidence is a guess.

**Not present.** Workspace ids, snapshot counts, attachment counts, confidence, sparklines, "N workspaces" summaries, any metric other than Time To Productive if a metric is shown at all.

**Empty state.** *Evo hasn't witnessed any work yet.* No demo data, no placeholder cards.

**Write paths reachable.** None. Home is read-only.

**Sources.** Workspace list from `CanonicalIndex::workspaces()`; names from locators/subjects; status from the designation tuple; order stated, not inferred from confidence or attachment count.

### E.2 Workspace Detail — "This is what I was doing."

Three stacked regions, in this order, because observation precedes interpretation:

**Region 1 — What Evo witnessed.** The resources of this body of work, from the current committed understanding (the latest Snapshot — RFC-0006 R-1 restricts the current restoration to current understanding). Each row: human name, when last witnessed, and the two affordances from D.2. This region *is* the evidence; both declarations are made from it, which is why the evidence appears first.

**Region 2 — Where it continues.** Either *you're picking up at «resource» — marked at T*, or *you haven't marked where this continues*. Never fabricated, never inferred (IS-0021 §25.8: absence of a designation means Next Step insufficiency, "never an inferred designation"). If the designation names a resource outside this body of work, say exactly that — the implementation already does, at `app.rs:1316–1322`.

**Region 3 — What comes back together.** The declared set, per-workspace. Plus, when they differ, one line accounting for the difference: named elsewhere, or could not be identified (C.6). Plus the sanctioned sentence (BE-03).

**History.** One disclosure, collapsed, labelled as explanation — *how Evo came to understand this*. Never a primary column. Never the thing being restored (RFC-0006 R-1).

**Not present.** Blocker region. Context-chain region. Confidence. Ids. Snapshot counts as a headline number.

**Write paths reachable.** `submit_designation` from Region 1 single-choice; `submit_continuation_surface` from Region 1 multi-select via the Continuation step.

### E.3 Continuation — "These are the things I want back together."

A mode over Region 1's list, not a separate place — the user should never lose sight of the resources while choosing among them.

**Content.** The current declared set shown as the starting point. Multi-select marks. A running count. The commit, labelled *bring these back together*.

**Constraint, taught early.** At one selection the commit is inactive and reads *pick at least two — one on its own is "pick up here"*. At zero it is inactive and reads *nothing selected*.

**The one-way door, stated plainly.** *This replaces what you have now. Evo can't unmark things — you can only declare a new set.* (BE-04, C.5. This is the honest form; hiding it would teach a reversibility that does not exist.)

**On commit.** The draft survives until the daemon answers. On acceptance, Region 3 updates from derived state and the draft clears. On rejection, the reason is shown verbatim from the daemon and **the draft is preserved** — the user's work is theirs (RFC-0006 R-4).

**Write path.** `submit_continuation_surface`, once, explicitly, over a draft the user assembled.

### E.4 Preflight — "These are the things Evo can actually reopen."

The honesty surface. Its job is to be believed.

**Content.** Every resource that would be attempted, each with READY or NOT AVAILABLE **and a reason in words** — colour never carries the meaning alone. A count of what will be attempted. An explicit statement of what will **not** be opened, and why. Nothing hidden, nothing rounded up.

**Language.** Attempts are attempts. "Reopening 5" not "restored 5". The distinction between what Evo will try and what Evo achieved is the product's core honesty commitment (RFC-0006 R-3 forbids maximising restored state; R-7 requires explainability).

**Refusal.** When nothing can be opened, Preflight says so and Continue is inactive **with that reason on screen** — from the same shared function `run_execution` uses, so `continue_plan_line` can never promise what execution will refuse (B.9).

**Contract note carried on this screen.** Until the C.3 amendment exists, Preflight must not describe the declared set as the thing that authorises opening. It describes what Evo will attempt. The two happen to coincide in the current implementation; the interface should not assert the coincidence as a rule.

### E.5 Execution — "This is what just happened."

**Content.** The report: one line per attempt, outcome plus reason, opened and not-opened both visible. The time of the act. Dismissible.

**Lifetime.** Until the user dismisses it, presses Continue again, or leaves the workspace. **Never** cleared by a reload or a new Snapshot (A.1, D.4). This is the requirement the brief made non-negotiable, and it is satisfied with zero canonical persistence — `ExecutionReport` already owns its attempts.

**Staleness.** When the state signature has moved since the act, one added line: *recorded at T; Evo has witnessed new activity since*. The report stays true about the past instead of pretending to be about the present.

**Height.** Measured from laid-out content, so a reason never gets clipped (A.3).

**Write paths reachable.** None. Execution reports; it does not declare.

---

## F. Decisions needed from you before Phase 1

1. **Is C.1 accepted** — arity dispatch dropped in favour of two distinct affordances (D.2)?
2. **The C.3 amendment.** Should I draft the minimal amendment RFC for surface → executable target, or should Phase 2 stay blocked and unbuilt?
3. **§11.1 revoke.** Confirm the honest-disclosure answer in E.3 (say what replacement does) rather than a future revoke evidence class.
4. **Home Continue.** Confirm the conditional rule in E.1 — Continue on Home only when the designation resolves into exactly one body of work.
5. **BE-02.** In or out of Phase 0? It is two strings, and D.4's derived-confirmation depends on it.
6. **Membership test.** One set must be chosen for "does this resource belong here" — latest-Snapshot artifacts (current behaviour of `designation_card`) or the cumulative Attachment Set (current behaviour of `select_workspace_with_designation`). RFC-0006 R-1 favours the former.

---

## G. Verification pass

Every claim in this report was re-checked against the source after the report was drafted, rather than trusted from the drafting pass. That pass found **seven defects in this report's own first draft**, all corrected above and recorded here so the review can see what was wrong and judge whether the corrections are right.

### G.1 Defects found in this report and corrected

| # | Defect | Severity | Correction |
|---|---|---|---|
| 1 | Claimed the desktop has **three** canonical write paths | material | It has **two**. `grouping`/`Grouped` does not appear anywhere in `crates/evo-desktop`. D.6 rewritten. |
| 2 | Attributed *"the surface does not change execution ordering or selection"* to RFC-0013 **Non-Goals** | material | It is in **§Gating Invariants**. Non-Goals contains a *different and stronger* clause — the RFC does not *authorize* "selective restoration execution — opening, focusing, or launching any resource". C.3 now cites four clauses instead of three, and is stronger for the correction. |
| 3 | C.1.2 stated without scope | material | The precedence branch tests the **per-workspace** surface. RFC-0013 Insufficiency: a declaration outside W leaves W's surface empty, and the designation then governs. Qualifier added — and it yields a second independent ambiguity rather than weakening the argument. |
| 4 | Cited **Law XVI** for B-05 (mislabelling) and for B.3 (naming by locator) | material | Law XVI governs whether a concept may become a computational object; it says nothing about mislabelling or UI naming. Removed from both. RFC-0011 §4 + RFC-0006 R-7 carry B-05; IS-0011 identity clauses + R-2 carry B.3. Law XVI's *remedy* clause does legitimately support D.1 and is now cited there instead. |
| 5 | Cited **Law XVII** for the vocabulary prohibition (B.10, D.5) | material | Law XVII is witnessed-vs-inferred separation. RFC-0006 R-2 carries the vocabulary point alone. Removed. |
| 6 | Cited **RFC-0012** for artifact co-membership in C.2 | minor | RFC-0012 was not read this session. Re-grounded on IS-0011 and on the function's own first-match scan, which presupposes multiple matches. |
| 7 | `runtime.rs` surface reads cited as ranges `:552–563` / `:593–621` | nitpick | The actual calls are at `:557` and `:594`. Tightened. |

### G.2 Claims verified verbatim against the source

Quoted text confirmed word-for-word: Law VII (`ARCHITECTURAL_LAWS.md:118`); Law XVI remedy clause (`:244`); RFC-0013 Negative Case 10 (`:539`), Negative Case 9 (`:538`), Negative Case 12 (`:542`), cardinality rationale (`:299–302`), Supersession (`:329–331`), the second mutual-non-implication statement (`:334–335`), Gating Invariants (`:414–416`), Non-Goals (`:598–610`), Open Question 2 (`:582–585`), Open Question 3 (`:586–589`), Replay (`:449–451`), Insufficiency (`:424`); IS-0021 §25 input list (`:669`), §25.1A (`:673–683`), §25.2 designation clause (`:795`), §25.5 (`:920`), §25.8 (`:1013`); RFC-0006 R-1 through R-7 and Forbidden Behaviour; IS-0019 §4, CS-1–CS-4, RP-4, RSP-1, BL-3, RM-6, RM-7; IS-0011 §4, §5, §7, §10, W-7, W-28, W-29.

Law numbering independently confirmed against the document's own headings: IV *Observation And Interpretation Must Never Be Confused*, V *Preserve Uncertainty*, VI *Under-Interpretation Is Better Than Over-Interpretation*, VII *Replay Must Always Be Possible*, XVI *Identity Law*, XVII *Epistemic Separation*. All uses that survive correction match the law's actual text.

Code claims confirmed by reading the file: `run_execution` surface precedence (`state.rs:762–777`) with its three honest-refusal branches (`:746–756`, `:765–776`, `:783–793`) — which is what establishes that `continue_plan_line` is the one place missing them; `handle_designation` at `runtime.rs:630` calling `new_with_designated` at `:637` with no surface read, against siblings at `:557` and `:594`; the reset block at `app.rs:310–320`; `state_signature` at `:357–359`; the `filter_map`/`zip` misalignment at `app.rs:1290–1294` and `:1350`; two distinct animation ids at `app.rs:542` and `:2162`; `continue_bar_expanded_height` at `:2283–2293`; `select_workspace_with_designation` at `state.rs:325–346` testing `attachments()` and returning first match; `DisplayState::display`/`selection` discarded at `app.rs:203–212`.

Absence claims established by exhaustive grep: no revoke path of any name; `SURFACE_DECLARATION_REASON` referenced only at its own definition; no `grouping`/`Grouped` in the desktop crate; `CanonicalIndex::current_continuation_surface()` and `current_continuation_surface_artifacts()` never referenced from the desktop crate.

### G.3 The two conclusions most worth attacking, and why they hold

**BE-01 must be fixed.** Primary ground: IS-0021 §25 (`:669`) defines the derivation input set to include the Current Continuation Surface "when it exists". If that reading were disputed, the conclusion still follows from determinism alone — §25.9, §25.10, IS-0019 RP-4 and RM-6, RFC-0013's Replay clause, and Law VII — because today the derived surface depends on which handler last fired rather than on the canonical log. Two independent grounds, either sufficient.

**Arity dispatch cannot be proved safe.** Primary ground: mutual non-implication, which RFC-0013 states in two separate sections (`:539`, `:334–335`), so the argument does not rest on a single clause. C.1.2 (execution precedence) and C.1.3 (no revoke, no reduction below two) are independent of it and independently fatal. The strongest counter-argument is stated in C.1.5 and is real, but concerns the continuity of the *concept*, not of the *operation*.

### G.4 What was not verified

RFC-0010 (Work Continuity), RFC-0012 (Co-membership), the Constitution, PRODUCT, ARCHITECTURE and TRACE-0001 were not read this session. No claim in this report depends on them; where an earlier draft leaned on RFC-0012 it has been re-grounded (G.1 #6). If any of the six decisions in section F turns on those documents, they should be read first.

The `frontend-design` question is untouched by design — the brief deferred it, and nothing in sections D or E specifies colour, type, spacing, or material.
