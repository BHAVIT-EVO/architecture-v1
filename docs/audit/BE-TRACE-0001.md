# BE-TRACE-0001 — Pipeline Trace and Divergence Report

**Status:** Delivered 2026-08-17. Companion to `docs/audit/BE-AUDIT-0001.md`.
**Scope:** the complete pipeline, raw observation → artifact identity → workspace formation → continuation derivation → restoration planning → execution → Home presentation, traced at source; each divergence from intended semantics named with its governing clause.
**Source changes:** none. No file under `crates/` was modified.
**Headline:** two of the four approved fixes cannot be implemented as specified. One is a no-op or a contract violation depending on where it is applied; the other cannot be built without falsifying a sentence currently rendered on screen. A third is clean, doubly grounded, and ready. The fourth is ready but blocked on a toolchain.

---

## §0 — Method, and the provenance of each claim

### §0.1 What was read, and by whom

The trace was executed in four read-only segments covering capture→observation, identity→formation, derivation→planning→execution, and Home→storage. Segment reports are not evidence on their own. Every claim this report *concludes* from was re-read at source directly before being written down, and the re-reading changed things twice:

The first correction is small but instructive. A segment report gave `ordered_targets` (`crates/evo-execution/src/engine.rs:133`) as a body using `ordered.contains(artifact_id)` inside a loop. The actual body uses a `push` closure over a separate `seen: Vec<String>`. The *semantics* are identical — deduplicate, Resume Point first, then Context Chain — but the text is not, and a report that presented the paraphrase as verbatim would have been quoting something that does not exist. The second correction is §0.2 below.

Claims are therefore in one of two classes, and this report marks the difference wherever it matters. **Read at source for this report:** the observation-kind enum and its three coupled tables; the derivation of Context Chain and Blockers; `ordered_targets`; the execution dispatch in `crates/evo-desktop/src/state.rs`; the preflight sort and the test that pins it; Home's draw loop, its provenance string, and `work_row`; `WorkspaceCard` and every reference to `continuable`; `plan_line`; the storage-root resolver chain; the pid's scope at the focus-emit site; the two co-membership constants. **Reported by trace and not independently re-read:** the interior of `crates/evo-workspace/src/formation.rs` beyond its three public entry points, the artifact-identity FNV derivation, and the test inventories. Nothing in §2 or §3 rests on the second class.

A post-draft pass re-read every citation in this report at source. It found two inexactnesses in my own draft, both now fixed in place and recorded here rather than silently corrected. First, §3.1's parenthetical about `Evidence` said `validate_structure` "rejects any fact count other than one (or two for the co-membership schemas)" without giving the function's crate path — a live hazard, since `validation.rs` is one of the fifteen basenames that occur in more than one crate — and it omitted a third case: `OBS-CONTINUATION-SURFACE` short-circuits to `validate_continuation_surface` before the count check (`crates/evo-observation/src/validation.rs:153-155`). The conclusion held, because `OBS-WINDOW-FOCUS-GAINED` is in the `expected_len = 1` branch, but the sentence as written was not true of all schemas. Second, §5.3 cited `related` at `crates/evo-desktop/src/detail.rs:745`, which is its doc comment; the function is at `:746` and `selection.historical()` at `:752`. The same pass upgraded one claim from asserted to cited: that `Provenance::context` round-trips additively is now grounded at `crates/evo-daemon/src/persistence.rs:105-118` and `:171-176`, because a recommendation resting on an unverified premise is the defect this report is meant to avoid.

### §0.2 A defect in BE-AUDIT-0001, found by this trace

`BE-AUDIT-0001.md:468` reads, in full:

> The corpus defines exactly ten canonical Observation kinds (`grep -o 'OBS-[A-Z-]*'`): `OBS-WINDOW-FOCUS-GAINED`, `OBS-FILE-SAVED`, `OBS-URL-NAVIGATED`, `OBS-COMMIT-MADE`, `OBS-REPOSITORY-MEMBERSHIP`, `OBS-WORK-DESIGNATED`, `OBS-WORK-GROUPED`, `OBS-CONTINUATION-SURFACE`, plus `OBS-UNKNOWN` and `OBS-FUTURE-COLLECTOR` as reserved sentinels.

**There are eight, not ten, and there are no reserved sentinels.** `ObservationConcept` (`crates/evo-observation/src/observation_language.rs:11-31`) has exactly eight variants and no catch-all. Three coupled tables agree and must be edited together to add a ninth: `is_canonical()` at `crates/evo-observation/src/observation_schema.rs:149-161`, `is_reference_only()` at `:168-176`, and `canonical_fact_name()` at `:179-191`. `OBS-UNKNOWN` and `OBS-FUTURE-COLLECTOR` are defined nowhere. They appear only as string literals inside four `#[cfg(test)]` negative assertions — `crates/evo-execution/src/resource.rs:188`, `crates/evo-execution/src/locator.rs:108`, `crates/evo-execution/src/selection.rs:572`, `crates/evo-desktop/src/state.rs:1564` — whose entire purpose is to assert that an unrecognized schema name maps to `None`.

The published method is what manufactured the error. `grep -o 'OBS-[A-Z-]*'` over the tree cannot distinguish a definition from a test fixture, and I printed the command next to the claim, which gave a reproducible-looking provenance to a count that was wrong. This is the same class as the citation defects recorded in `BE-AUDIT-0001.md §0.5`: a mechanical check whose output reads like a verification but whose premise is unsound. No verdict in the audit depended on the number being ten.

### §0.3 Environment

`cargo`, `rustc` and `rustup` remain absent; no Rust toolchain is installed. **Nothing in this report is compiled, executed, or test-verified. Every claim is static reading of source.** Where this report says a change is "ready", that means the edit site and the governing clause are established — not that the result builds. Any statement about runtime behavior after a change would be **NOT VERIFIED — ENVIRONMENT**, and none is made.

No file under `crates/` was modified. Verified by mtime, not by `git status` (the tree is permanently dirty, ~68 entries): the newest mtime under `crates/` is `1786962712`, older than this report's companion audit at `1786979294`.

---

## §1 — The pipeline as built

Seven stages, each with its real entry point.

| # | Stage | Entry point | Verdict |
|---|---|---|---|
| 1 | Capture → Observation | `crates/evo-capture/src/adapters/macos.rs:92` `normalize` → `crates/evo-observation/src/accept.rs:30` `accept` | PASS, with a lossy boundary (§2.1) |
| 2 | Artifact identity | `crates/evo-artifact/src/identity_assignment.rs:33` `derive_artifact_id_from_observations` | PASS |
| 3 | Workspace formation | `crates/evo-workspace/src/formation.rs:84` `form_workspace_with_evidence`, decided by `crates/evo-workspace/src/workspace_decision.rs:25` `decide` | PASS, bounded by evidence classes (§2.2) |
| 4 | Continuation derivation | `crates/evo-restoration/src/derivation.rs:517` `derive_restoration_plan` | PASS — and honestly empty where evidence is absent (§2.3) |
| 5 | Restoration planning | `crates/evo-execution/src/selection.rs:215` `select_restoration` | PASS |
| 6 | Execution | `crates/evo-execution/src/engine.rs:171` `execute_selection` | PASS in behavior, FAIL in announcement (§2.4) |
| 7 | Home presentation | `crates/evo-desktop/src/app.rs:249-270` → `crates/evo-desktop/src/home.rs:117` `screen` | PRODUCT DECISION REQUIRED (§2.5) |

The shape worth internalizing is that **stages 1–5 are stronger than the audit implied, and the failures cluster at the seams** — at the boundary where capture discards what it witnessed, and at the two places where the interface describes something other than what the system does.

---

## §2 — Divergences

### §2.1 Capture discards witnessed application identity — FAIL, fix ready

`emit_current_subject` (`crates/evo-capture/src/macos_event_source.rs:462-478`) emits `MacOSSignal::WindowFocusGained { subject, observed_at }`, where `subject` is the raw `AXTitle` string and nothing else. The owning process is witnessed and thrown away.

The trace settles the question that mattered for cost. **The pid is already in scope at the emit site; no plumbing is required.** `emit_current_subject` is called at `crates/evo-capture/src/macos_event_source.rs:458`, inside `unsafe fn attach_to_pid(&mut self, pid: i32)` (`:390`), where `pid` is a live parameter — and eleven lines earlier the same value is stored into `FocusAttachment { observer, run_loop_source, element, pid }` (`:447-452`, field declared `:269`). The second call site, the AX focus callback, reaches it through `self.focus_attachment`. So the blocker is not scope. It is that `MacOSSignal::WindowFocusGained` (`crates/evo-capture/src/adapters/macos.rs:17`) has no field to carry it, and `translate` (`:156`) hardcodes an empty context map.

Consequence: a closed window becomes permanently `Unavailable` rather than a reopenable application, because `ResourceIdentity::WindowTitle` can only be resolved by matching a live window title (`crates/evo-execution/src/preflight.rs`, `select_unique_window`). Evo knew which application it was and chose not to write it down.

Note the irony available at zero cost: `MacOSSignal::ApplicationActivated` (`crates/evo-capture/src/adapters/macos.rs:70-75`) already carries `bundle_identifier`, `application_name` and `process_identifier` — and `normalize` discards the entire variant with `Ok(None)` at `:152`. The witnessing exists; only the retention does not.

**Fix status: READY.** Additive schema v2, no contract amendment, no data migration, replay handles old records. Blocked only on the toolchain.

### §2.2 URL and window-title artifacts can never share a Workspace — ARCHITECTURAL, not a defect

Co-membership has exactly two signals, both binary, at `crates/evo-workspace/src/co_membership.rs:46` (`REPOSITORY_MEMBERSHIP_STRENGTH: f64 = 0.8`) and `:50` (`WORK_GROUPED_STRENGTH: f64 = 1.0`). Both doc comments cite RFC-0012 and both are reasoned: a repository "may contain more than one body of work, so automatic repository membership is strong but not conclusive"; the user's explicit declaration "is conclusive."

Repository membership is emitted only for file paths and commit hashes, by `crates/evo-capture/src/macos_fsevents.rs`. No producer ever emits a membership Observation naming a URL or a window title. So those two kinds score 0.0 against every candidate and form single-member Workspaces, permanently.

This is the mechanism `ARCHITECTURE.md:151`/`:265` describes as "Workspaces are never hard-merged; only linked by a relationship with a strength score" — and the Workspace↔Workspace link with a strength score **does not exist as a type anywhere in the tree**. All relatedness is expressed as co-membership of Artifacts *inside* one Workspace; there is no edge *between* Workspaces. This is audit finding §14.2(1), confirmed and localized. It is an architectural gap, not an implementation bug, and closing it means adding a primitive relationship — outside the standing instruction not to invent architecture.

### §2.3 Context Chain and Blockers are empty by derivation — PASS, and this invalidates an approved fix

`crates/evo-restoration/src/derivation.rs:529-536`, verbatim:

```rust
    // §25.3 — No canonical supporting relationship exists in the current
    // model, so the Context Chain is empty. Membership alone never establishes
    // relevance.
    let context_chain = ContextChain::new(Vec::new())
        .expect("an empty Context Chain is always valid (IS-0021 CC-1)");

    // §25.4 — No canonical unresolved-work representation exists in the
    // current model, so zero Blockers are derived. No Blocker is invented.
    let blockers: Vec<Blocker> = Vec::new();
```

This is not an unimplemented stub. It is a principled refusal, citing IS-0021 §25.3 and §25.4, declining to manufacture a supporting-artifact set from membership. It is the correct behavior under Law VI (Under-Interpretation Is Better Than Over-Interpretation) and `ARCHITECTURE.md:151` ("Under-attachment is **always** preferred to over-attachment").

Its downstream consequence is decisive: because the Context Chain is always empty, `ordered_targets` (`crates/evo-execution/src/engine.rs:133-152`) — which pushes the Resume Point and then iterates `plan.context_chain().artifacts()` — returns **exactly one artifact in production**. See §3.3 for why this kills the Layer 3 cap as specified.

### §2.4 The interface announces one target; execution opens N — FAIL, doubly grounded, fix ready

Two code paths, and they do not agree.

**What the screen says.** `plan_line` (`crates/evo-desktop/src/detail.rs:400-423`) builds the "Evo will attempt" line from `evo_execution::ordered_targets(&request)`. Per §2.3 that is one artifact — the Resume Point.

**What actually runs.** `run_execution` (`crates/evo-desktop/src/state.rs:762-777`) branches on `!outcome.continuation_surface().is_empty()` and, when a surface is declared, returns `execute_selection(selection, ...)` — which never consults `ordered_targets` at all. It opens every restore-worthy surface member, ordered by `crates/evo-execution/src/preflight.rs:236-239`:

```rust
    // One canonical deterministic order for the whole report: ascending
    // ArtifactId across the surface. Execution attempts follow this order.
    outcomes.sort_by(|a, b| a.artifact_id().as_str().cmp(b.artifact_id().as_str()));
```

So with a declared surface of six members, the screen names one thing and Evo opens six. The two agree only in the degenerate case of a single restore-worthy member that is also the Resume Point.

There is a second, independent divergence in the same area: the context panel lists restore-worthy items and then unavailable items as two groups (`crates/evo-desktop/src/detail.rs:681-698`), while execution interleaves them by ArtifactId. The codebase proves the interleave in its own test, `preflight_covers_every_surface_member_and_never_history` (`crates/evo-execution/src/preflight.rs:425-450`):

```rust
        assert_eq!(ids, vec!["artifact-a", "artifact-commit", "artifact-url"]);
```

`artifact-commit` is the unsupported member, sitting between two ready ones. A third order exists at `crates/evo-desktop/src/state.rs:260-278`, where `surface_subjects` sorts by subject string.

**Governing clauses, two independent grounds.** `PRODUCT.md:248` — "Evo always explains its actions honestly." `CONSTITUTION.md:193-197` — the explainability mandate, that when Evo acts it must always be possible to answer "Why?" honestly. Neither is a UI preference; both are contract.

**Fix status: READY, and it belongs in the announcement.** The honest change is to make `plan_line` describe what `run_execution` will actually do — mirroring its branch — rather than to change what executes. Changing execution would alter restoration semantics and needs a clause that authorizes a different target set; changing the announcement makes an existing sentence true. This also keeps the change inside the standing prohibition on redesigning the frontend: no screen composition changes, no visual design changes, one string derived from the correct source.

### §2.5 Home ordering — PRODUCT DECISION REQUIRED

Home is confirmed as a total 1:1 map with no filter, sort, rank or take (`crates/evo-desktop/src/app.rs:249-270`), drawn in vector order (`crates/evo-desktop/src/home.rs:150-168`), where the vector order is `workspace.log` first-append order per Workspace identity and is stable across restarts.

But the trace found something the audit did not, and it changes the approved fix. Home **states its ordering contract to the user, on screen** (`crates/evo-desktop/src/home.rs:171-176`):

```rust
        ui::provenance(
            ui,
            "In the order Evo recorded them. Nothing here is ranked, scored, or \
             promoted for being recent.",
        );
```

The module doc says the same at `:22-24`: "Nothing here ranks. Ordering is canonical… No row is promoted for being recent, frequent, or belonging to an active application."

This is a live commitment, not a comment. See §3.2.

---

## §3 — Status of the four approved fixes

The four were approved as: (a) `window_focus` pid schema v2 (§8.6); (b) Resume candidacy + Home ordering (§6.3), scoped to "ordering only, no omission"; (c) Restoration cap + ordering divergence (§13.2a, Layer 3); (d) Storage root off `/tmp` (§9.2).

### §3.1 (a) window_focus pid schema v2 — READY

Grounded, additive, no amendment, pid already in scope (§2.1). Edit sites: the `MacOSSignal::WindowFocusGained` variant (`crates/evo-capture/src/adapters/macos.rs:17`); its `normalize` arm (`:94-100`); `translate`'s hardcoded `HashMap::new()` (`:157`); the emit site (`crates/evo-capture/src/macos_event_source.rs:471-476`); and a v2 constructor plus arms in all three coupled schema tables (`crates/evo-observation/src/observation_schema.rs:149-161`, `:168-176`, `:179-191`). One design question to settle before writing: whether the pid rides in `Provenance::context` or in `Evidence`.

Context is strictly additive. The record codec writes `context_count={}` and then hex key/value pairs (`crates/evo-daemon/src/persistence.rs:105-118`) and reads them back count-driven (`:171-176`), so an added entry needs no format version bump and existing records — which carry `context_count=0` — still parse unchanged.

Evidence is not additive. `validate_structure` (`crates/evo-observation/src/validation.rs:133-162`) computes `expected_len` as 2 for the two co-membership schemas and 1 for everything else (`:148-156`), then rejects any other count outright (`:157-161`); `OBS-CONTINUATION-SURFACE` escapes to its own `validate_continuation_surface` at `:153-155`. `OBS-WINDOW-FOCUS-GAINED` falls in the `expected_len = 1` case, so a second fact would be rejected until a new arm is added — and that arm is a change to what the schema *asserts as witnessed*, not to what it carries as context.

**Context is the smaller change and the semantically honest one — the pid is provenance of the witnessing, not an additional witnessed fact. I would take it absent an objection.**

### §3.2 (b) Home ordering — BLOCKED, decision needed

The authorization was "ordering only, no omission," and ordering is genuinely authorized by `ARCHITECTURE.md:170`, which classifies ranking scores and "any home-screen ordering" as cached and explicitly disposable. That clause permits the mechanism. It does not resolve the collision found in §2.5.

Implementing any Home ordering means one of two things, and both are yours, not mine:

Either the on-screen sentence at `crates/evo-desktop/src/home.rs:171-176` changes — which is a frontend copy change, against the standing "DO NOT change visual design" constraint, and is in any case a product statement about what Evo promises rather than a technical detail. Or the sentence stays and becomes false the moment a ranking exists — which violates `PRODUCT.md:248` and `CONSTITUTION.md:193-197`, the same two clauses that make §2.4 a FAIL. I will not choose between falsifying a product promise and overriding an explicit constraint.

Two things narrow the decision usefully. First, `ARCHITECTURE.md:170` *permits* home-screen ordering; nothing in the corpus *requires* it. Second — and this corrects the audit — the resume-candidacy distinction is **already drawn on screen**. `work_row` (`crates/evo-desktop/src/home.rs:237-252`) renders "Continue from {resume}" when a Resume Point exists and "You haven't marked where this continues" when it does not. So the concepts are not wholly collapsed; what is missing is ordering and prominence, not the distinction itself. That is a materially smaller problem than §6.2 of the audit described, and it may not be worth spending a product promise on. See §5.1 for the correction.

If you do want ordering, the cheapest honest version is to group continuable work above non-continuable work using the Resume Point that Home already computes, and to amend the provenance line to say exactly that. No new state, no score, no recency, no omission.

### §3.3 (c) Restoration cap, Layer 3 — NOT IMPLEMENTABLE AS SPECIFIED

`ARCHITECTURE.md:211` mandates that Layer 3 supporting artifacts be "capped to a small number", and the audit correctly found that `grep` for `take(|MAX_|cap\b|limit` across `evo-execution` and `evo-restoration` returns nothing in production. Both facts hold. The conclusion I drew from them does not.

There are only two places a cap could go, and neither works.

Cap the **Context Chain** — the actual Layer 3 set — and you cap a vector that is empty by derivation, always, for a principled reason that cites IS-0021 §25.3 (§2.3). The change is a no-op with a comment. It would make the grep look right and change nothing, which is precisely the outcome the brief forbids: optimizing for a clean-looking repository rather than for a true one.

Cap the **Continuation Surface** — the only set that is actually unbounded — and Evo silently declines to reopen something the user explicitly declared as their continuation. That contradicts RFC-0013, which makes the declared surface the restore candidate set, and Law IX (The User Owns Judgment). A cap there is not a refinement; it is Evo overruling an explicit instruction.

The real gap is that **the canonical evidence class for a supporting relationship does not exist**. `RestorationComponent` (`crates/evo-restoration/src/derivation.rs:228-233`) has only `ResumePoint` and `NextStep` variants, so insufficiency cannot even be *named* for a Context Chain. Closing this means adding an evidence class and a derivation rule — new architecture and a new semantic layer, both explicitly forbidden to me by your directive.

**What I recommend instead:** take the ordering half of §13.2a, which is real, grounded and fixable (§2.4), and treat the cap half as a contract question for you — specifically, whether `ARCHITECTURE.md:211`'s Layer 3 is unimplemented or whether the Continuation Surface *is* Evo's Layer 3, in which case `:211`'s cap conflicts with RFC-0013 and one of the two documents needs amending. That is a document-level contradiction, and per the precedence rule I am to identify it rather than silently resolve it.

### §3.4 (d) Storage root off /tmp — READY, with one hazard to fix at the same time

Four production resolvers, confirmed. Three honor `EVO_STORAGE_ROOT`: `crates/evo-desktop/src/state.rs:34-38`, `crates/evo-daemon/src/runtime.rs:701-708`, `crates/evo-daemon/src/main.rs:57-63`. The fourth does not:

```rust
fn storage_root() -> PathBuf {
    THREAD_ROOT_OVERRIDE
        .with(|slot| slot.borrow().clone())
        .unwrap_or_else(default_root)
}

fn default_root() -> PathBuf {
    std::env::temp_dir().join("evo-storage")
}
```

(`crates/evo-storage/src/storage.rs:348-356`.) `THREAD_ROOT_OVERRIDE` is `thread_local!` (`:43-45`). The four agree today only because all four hardcode the same literal, and because `crates/evo-desktop/src/daemon.rs:124` passes the env var to the child process. The hazard is that **any storage call on a thread without a `with_thread_root` guard writes to the temp fallback and returns `Ok(())`** — a silent misroute, not an error. `persist_observation` (`crates/evo-daemon/src/persistence.rs:56`) takes no root parameter and is exposed to exactly this, while its own reader `load_persisted_observations(root: &Path)` (`:66`) takes one explicitly.

Also relevant to the destination question: there is no bootstrap, no version marker, and no root-provenance file. `crates/evo-storage/src/lib.rs:31` explicitly disclaims migration policy. Root creation is incidental, in `Storage::append` (`crates/evo-storage/src/storage.rs:89-91`), `DaemonLock::acquire` (`crates/evo-daemon/src/daemon_lock.rs:45`) and `write_capture_report` (`crates/evo-daemon/src/daemon_status.rs:92`). And a first-run marker already lives at `~/Library/Application Support/Evo/first_run_done` (`crates/evo-desktop/src/shell.rs:278-288`) — so the application-support directory is already in use for non-canonical state, which makes it the obvious destination and means the two roots stop being unrelated.

The destination was never in question; Law VIII makes the *transition* the question. Because there is no version marker and no migration path, moving the root silently orphans any existing log. **This is ready to implement and the migration behavior is the one thing I need decided:** adopt-in-place, copy-forward, or start-clean-and-leave-the-old-log.

---

## §4 — Verdict summary

| Item | Verdict |
|---|---|
| §2.1 window_focus discards witnessed pid | FAIL — fix ready, blocked on toolchain |
| §2.2 URL / window-title Workspaces are permanently single-member | ARCHITECTURAL CONFLICT — needs a Workspace↔Workspace relationship primitive |
| §2.3 Context Chain and Blockers empty | PASS — principled, correctly cited |
| §2.4 Announced plan ≠ executed plan | FAIL — fix ready, doubly grounded, blocked on toolchain |
| §2.5 Home ordering vs on-screen ordering promise | PRODUCT DECISION REQUIRED |
| §3.3 Layer 3 cap | ARCHITECTURAL CONFLICT — `ARCHITECTURE.md:211` vs RFC-0013 |
| §3.4 Storage root | FAIL — fix ready, one decision needed (migration behavior) |
| Anything about post-change runtime behavior | NOT VERIFIED — ENVIRONMENT (no Rust toolchain) |

---

## §5 — Corrections to BE-AUDIT-0001

### §5.1 §6.2's central FAIL was overstated

The audit says Home makes "'Evo witnessed this' and 'this is work you may want to resume' … the same claim made by the same list." The structural half is right: `crates/evo-desktop/src/app.rs:249-270` is a bare 1:1 map with no filter, sort or take. The claim half is too strong. `work_row` (`crates/evo-desktop/src/home.rs:237-252`) already renders different body text for the two cases — "Continue from {resume}" versus "You haven't marked where this continues". The collapse is in ordering and prominence, not in the assertion Home makes.

Related, and worth recording: `WorkspaceCard.continuable` (`crates/evo-desktop/src/state.rs:543`, computed at `:588`) is referenced nowhere outside two tests (`:2138`, `:2155`). It is exactly redundant with `resume_from.is_some()`, since both derive from `outcome.resume_point()`. It is dead, but its deadness is harmless rather than a lost signal.

### §5.2 §468's kind count

Eight canonical Observation kinds, not ten; no reserved sentinels. See §0.2 for the full correction and for why the published `grep` method produced it.

### §5.3 §14.2(1) confirmed and localized

The missing Workspace↔Workspace relationship link is confirmed: no `WorkspaceLink`, `WorkspaceRelation`, `Affinity` or equivalent type, field, enum or storage kind exists. `crates/evo-desktop/src/detail.rs:746` has a presentation-only `related` section driven by `selection.historical()` (`:752`), which is a listing, not a relationship. §2.2 above localizes the user-visible consequence.

---

## §6 — What I need from you

Three decisions, in dependency order. Nothing else is blocked.

**1. Home ordering (§3.2).** Ordering as authorized collides with a promise Evo currently makes on screen. Either the promise changes, or the ordering does not happen. Given that the candidacy distinction is already drawn in the row text (§5.1), declining the ordering is a defensible answer and costs less than it appears to.

**2. Layer 3 (§3.3).** Is `ARCHITECTURE.md:211`'s Layer 3 unimplemented, or is the Continuation Surface Evo's Layer 3? If the latter, `:211`'s cap contradicts RFC-0013 and one document needs amending. Either way the approved cap should not be written as specified.

**3. Storage-root migration (§3.4).** Adopt-in-place, copy-forward, or start-clean. There is no version marker, so this cannot be inferred.

Two fixes — §3.1 (pid schema v2) and the announcement half of §2.4 — need no decision and are ready to write the moment a Rust toolchain exists. I will not write them before then, because a change I cannot compile or test is a claim I cannot make honestly.
