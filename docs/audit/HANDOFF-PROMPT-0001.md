# HANDOFF-PROMPT-0001 — feed this whole document to the next model

You are taking over an in-progress engineering task in an existing Rust repository.
Do not treat this as a greenfield build. Read this entire document before touching
anything, then follow the "First hour" checklist at the end.

---

## 1. The project, in one paragraph

**Evo** is a *local-first, privacy-first* macOS app that watches only what a person
does (window focus, file saves, browser navigation, input counts, plus the person's
own explicit declarations) and reconstructs their **bodies of work** so they can
resume any of them later with minimal mental reload. There is no cloud, no screen
capture, no audio, no telemetry. The repository is a Cargo workspace at
`/Users/bhavitsaini/Desktop/Evo-Evolution.nosync` on branch `pods`.

The active workstream is a plan document called **PLAN-0001**:
`docs/audit/PLAN-0001-understand-work-backend.md` — *"What must change for Evo to
understand work."* Your job is to finish that plan. It is organised into Phases 0–6.
Phases 0, 1, 2 are shipped; Phase 3 is shipped except for two sub-items; Phases 4, 5,
6 are not started. Details in §8 and §9 below.

**Critical framing:** PLAN-0001 is a *proposal*, not authority. Authority is
`docs/architecture/ARCHITECTURE.md`, the accepted RFCs in `docs/rfcs/`, and the frozen observation
corpus. Where the plan and those disagree, they win.

---

## 2. Environment + hard rules

- OS: macOS (darwin), shell: bash. Repo root: `/Users/bhavitsaini/Desktop/Evo-Evolution.nosync`.
- Rust workspace. **Always build and test offline and locked:**
  `cargo check --workspace --all-targets --offline --locked`
  `cargo test -p <crate> --offline --locked`
  (`.cargo`/vendored deps exist; the `--offline --locked` flags are mandatory.)
- The working tree is **large and mostly uncommitted** (hundreds of modified and
  untracked files from earlier sessions — `git status` will be huge). Do not assume
  "modified" means "your change". `git diff` per file before editing.
- Two instruction files govern behaviour and you must follow them: `AGENTS.md` and
  `CLAUDE.md` (root). Summary of what they require:
  - **Read `DESIGN.md` before any visual/UI decision.** All fonts, colors, spacing
    are defined there; do not deviate without explicit approval.
  - Use the project's **graphify** knowledge graph for codebase questions:
    `graphify query "<question>"`, `graphify path "<A>" "<B>"`, `graphify explain "<concept>"`.
    `graphify-out/` exists (graph.json, wiki). Run **`graphify update .`** after you
    modify code (AST-only, no API cost).
  - Do not use Chrome/`mcp__claude-in-chrome__` tools for browsing; the repo routes
    web browsing through its own `gstack` skill.
- **Never** introduce a network client into the default build. See constraints in §4.

---

## 3. What "understand work" means here — the actual problem

Four complaints from the person using Evo, which PLAN-0001 §2 re-anchors against the
real code. These are the whole point of the task; keep them in view:

1. **"It can't group apps by work, and can't say which 2–3 of the 6 open things I still
   need."** → the subset/lifecycle problem. PLAN-0001 Phase 6.
2. **"It makes anything a work."** (A leisure YouTube video becomes a "project.") →
   the missing *intent* / "not work" axis. PLAN-0001 Phase 3.
3. **"No sense of urgency; work is displayed without order."** → ordering. PLAN-0001
   Phase 2 (shipped).
4. **"The name is a URL / meaningless."** → titling. PLAN-0001 Phase 1.1 (shipped) +
   Phase 4 (not started).

---

## 4. Non-negotiable constraints (PLAN-0001 §0.5) — these override everything

These are product invariants. If a proposed change violates one, the change is void
and must be redesigned. Cite them by name (C1..C6) when you reason about a design.

- **C1 — Evo never captures the screen.** No screen recording, screenshots, window-image
  capture, or OCR over pixels. Ever. Not deferred — refused. (Several reverse-engineered
  competitor apps *do* this; you may not copy that mechanism.)
- **C2 — Evo never listens.** No microphone, ambient audio, ASR, or diarisation in the
  default product.
- **C3 — Nothing ever leaves the machine.** No network egress of any observation,
  artifact, name, embedding, log, or "anonymous" aggregate. Enforced structurally: the
  default build must contain no HTTP client, no cloud SDK, and no network-capable crate.
  The only sockets are local Unix domain sockets for daemon↔shell IPC, plus a
  **loopback-pinned** local Chrome DevTools client used to read the local browser's tab
  titles/URLs (that one is required, and must refuse any non-loopback address).
- **C4 — Fewest permissions, and never in advance.** Accessibility is the one required
  permission (it powers `OBS-WINDOW-FOCUS-GAINED`). Input Monitoring and Microphone are
  optional and must be presented as optional. Screen Recording / Camera / Full Disk
  Access are never requested. Never ask for a permission at launch or as a bundle.
- **C5 — No new capture source without a consent story and an off switch** that stops it
  being *written*, not merely shown.
- **C6 — Derived-only; the source is discarded.** If a raw form is needed to derive
  something, the raw form is discarded at capture time and never stored as ground truth.

---

## 5. The reverse-engineered material — what it is, where, and WHY you must read it

Before this plan existed, a separate reverse-engineering (RE) pass analysed ~13
competitor macOS time/context-tracking apps to learn their *mechanisms*. PLAN-0001 is
derived from that material. **You cannot do this task well without reading it**, because:

- The plan's design choices are justified by specific mechanisms found in those apps
  (e.g. ManicTime's user-authored classification rules + "absorb" gap-filling; Dayflow's
  checkable title-evidence contract; Session Buddy's saved-vs-previous snapshot split;
  Bunch's per-item optional flag; Rize/Toggl in-flow feedback). You need to know what
  those mechanisms actually are so you implement the *right* part of them.
- Almost every mechanism must be **filtered** through Evo's laws. The RE material
  recommends things Evo is forbidden to do (built-in app/domain classification, OCR,
  ASR, persisted derived snapshots). If you don't read it, you will either copy a
  forbidden mechanism or fail to take the permitted part.
- The RE material is tagged with evidence markers. PLAN-0001 §3.0 defines them (roughly:
  `[M]` measured from the shipped binary, `[D]` documented, `[V]` vendor/docs, `[?]`
  unresolved). **Read §3.0 to get the exact definitions, and carry the markers into your
  reasoning** — an `[M]` fact is safe to build on; a `[?]` fact is a hypothesis.

### 5.1 Where the RE files are

Master evidence report:
- `RE done/REA_EVIDENCE_REPORT.md`

The rebuild blueprint (the big design doc derived from the RE pass — **read this, but
read it skeptically; §11 below lists where it is wrong**):
- `RE done/evo-reverse-engineering/EVO-REBUILD-BLUEPRINT.md`

Raw per-app evidence:
- `RE done/evo-reverse-engineering/evidence/` — for each app (Bunch, Dia, Granola,
  ManicTime, Rambox, Rize, Session Buddy, Timing, Toggl, …) there are files like
  `*_macho.json` (Mach-O), `*_arch.json` (architecture), `*_frameworks.txt`,
  `*_plist.json`, `*_main_strings.txt` (extracted strings), `*_macos_dir.txt`,
  `*_resources.txt`.
- `RE done/evo-reverse-engineering/evidence/MECHANISMS-SUMMARY.txt` — the condensed
  cross-app mechanism list. **Start here.**
- `RE done/evo-reverse-engineering/evidence/REA_*.md` — focused notes
  (`REA_sessionbuddy.md`, `REA_plist_manictime.md`, `REA_plist_timing.md`, …).
- `RE done/evo-reverse-engineering/evidence/js/` — extracted Electron/JS sources
  (`sb_all.js`, `sb_sessions.txt`, `toggl_autotracker.txt`, `granola_renderer.txt`,
  `rize_strings.txt`, `rambox.txt`, plus the `extract_asar.py` / `extract_context.py`
  tools used to pull them).
- `RE done/evo-reverse-engineering/evidence/asar/` — unpacked Electron bundles
  (e.g. `asar/Toggl/…`, `asar/Rambox/…`).
- `RE done/evo-reverse-engineering/evidence/rea-native/` and `rea-js/` — per-app
  REA tool output (dispatch tables, dirs, etc.).
- `RE done/rea_*.json` — top-level per-app summary JSONs (arc, bunch, dia, granola,
  manictime, memtime, openworkspace, qbserve, rambox, rize, timing, toggl; `*_macho.json`
  and `*_plist.json` variants).

### 5.2 The Evo-side analysis docs you must also read

- `docs/audit/PLAN-0001-understand-work-backend.md` — **the plan you are finishing.**
  Read §0 (exec summary), §0.5 (constraints), §1 (how the backend actually runs),
  §2 (the four complaints), §3 (what the RE gives us), §4 (the six phases), §5 (extra
  additions), §6 (conflict log), §7 (deferred/refused), §8 (what would make the plan
  wrong), §9 (order of work), §10 (implementation status — read this to see what is
  already done and what is explicitly *not*).
- `docs/audit/DIAGNOSIS-0001-why-evo-does-not-understand-work.md` — the original diagnosis.
- `docs/audit/BE-AUDIT-0001.md`, `docs/audit/BE-TRACE-0001.md` — backend audit/trace;
  these explain the storage/observation internals and cite exact line numbers.
- `docs/audit/ARCH-GAP-0001.md`, `docs/audit/PHASE0-0001-product-truth-architecture-reset.md`,
  `docs/audit/attack-log-redesign.md` — architecture gaps and a real acceptance-pipeline
  bug (`OBS-WORK-NAMED` missing from a schema allow-list) that was found and fixed.
- `docs/research/ARBITRARY-WORK-GENERALIZATION.md` — the reasoning that Evo must not
  hardcode work categories, and the local-model (naming) policy.
- `docs/rfcs/RFC-0011…0014` — the accepted contracts for user declarations
  (continuation evidence, co-membership, continuation surface, engagement/naming R11).
- `docs/implementation/IS-0003 — Observation Schemas.md` — the schema spec (note: it is
  frozen; newer schemas such as `OBS-SURFACE-NAMED/v1`, `OBS-INPUT-ACTIVITY/v1`, and the
  one you'll work with, `OBS-SURFACE-CLASSIFIED/v1`, are specified in code rather than
  in this file — see §8).
- `docs/BUILD-LOG.md` — historical engineering log, useful for "why is this like this".

---

## 6. How Evo's backend actually runs (so you don't fix the wrong layer)

PLAN-0001 §0/§1 is emphatic: **two interpretation paths existed; only one is live, and
much of the RE blueprint diagnoses the dead one.** Internalise this:

```
CAPTURE (macos_fsevents, macos_accessibility, url poller, input counter, declarations)
   → CANONICAL LOG  observation.log  (append-only; the single source of truth)
   → evo_threads engine  (replays the log into threads/works)
   → evo_daemon::threads::display_threads_*  (maps works to presentable cards)
   → evo-desktop (rack.rs renders Home)
```

Key truths:
- **Home is served by `evo_threads`**, bridged in `crates/evo-daemon/src/threads.rs`.
  It is *not* served by the older `evo-engagement` affinity/clustering pipeline. The RE
  blueprint's §5–§7 target `evo-engagement` and `evo-desktop/src/state.rs` — those are
  **not** the live path. Do not "fix" the engagement ranking; it is not what Home shows.
- The engine already ranks (episodes × anchors, decay) and already filters some junk.
- **A second, decoy engine path once existed** (`engine_from_live_data`,
  `read_sqlite_events` in `threads.rs`) reading a month-stale legacy SQLite store
  (`~/Library/Application Support/evo/evo.db`). It was removed in Phase 0.2. If you see
  any reference to it, it is dead/removed — keep it gone.
- The engine is a **pure replay** of the append-only log: `Engine::replay(events)`. This
  is why "retroactive" changes (like classification) are free — replay from the top.

---

## 7. The persona/verification bar

- You are a careful engineer, not a cheerleader. **Do not claim success you did not
  verify.** Run the command; preserve the exit status; if you pipe output through a
  filter, capture `$?` or use `set -o pipefail` — a filtered success is not a pass.
- Prefer fixing causes over weakening tests/assertions/suppressions.
- When something cannot be verified in this environment (e.g. a GUI gesture), say so
  explicitly under a "Limitations / not verified" heading.

---

## 8. WHAT HAS ALREADY BEEN DONE (do not redo it; build on it)

This is the current implementation state. File paths are exact. Where a thing is
"verified", the command was run and passed on this machine.

### Phase 0 — restore the input (shipped)
- **0.1** Home already surfaces `capture=partial` and offers "Check again"
  (`evo-desktop/src/{daemon,shell,app}.rs`, `evo-daemon/src/daemon_status.rs`). Already
  existed; confirmed, not rebuilt.
- **0.2** Removed the decoy legacy engine path: `engine_from_live_data`,
  `engine_from_sqlite`, `read_sqlite_events`, and the two examples
  (`evo-daemon/examples/sqlite_probe.rs`, `examples/debug_storage.rs`). **Do not
  reintroduce.** (`evo-daemon/src/threads.rs`)
- **0.4** `scripts/build-app.sh` defaults `EVO_VOICE=off`: the base `Info.plist` declares
  **no microphone key**, voice models are not bundled, voice binaries are not built.
  `EVO_VOICE=on` restores everything and adds the mic key with a local-only statement.
- **0.5** Trust constraints are now executable:
  `crates/evo-desktop/tests/trust_constraints.rs` (6 tests): no screen-capture/OCR
  dependency, no audio dep in the default build, no HTTP client, no camera/screen
  permissions in the generated `Info.plist`, and every socket-opening file pins to
  loopback. Passes.

### Phase 1 — make the list honest (shipped)
- **1.1** Titles are never addresses. `meaningful_name` no longer falls back to a bare
  host; generic interface slugs (`login`, `search`, …) are refused for URL segments only;
  `title_or_structural` refuses the truncated-raw-string path for address-shaped
  resources; `untitled_name` supplies `Untitled — N places, <duration>`.
  (`evo-daemon/src/work_understanding.rs`)
- **1.2** The rack presents only work; un-continuable records are held out and disclosed
  in one quiet line. (`evo-desktop/src/rack.rs` — note: this file is **new/untracked**.)
- **1.3** Removed the dead naming path (`thread_name` cluster) in `threads.rs`; canonical
  naming lives in `work_understanding.rs`.

### Phase 2 — urgency ordering (shipped)
- Ordering is a **lexicographic key over witnessed facts**: `has_open_loop` → coarse
  recency bucket → decayed engagement → thread id. `DisplayThread` gained
  `has_open_loop` and `urgency_reason`; the reason renders in the teal voice.
  Recreation reads the work's own last human activity, **never** the newest record in
  the log (the log's newest records are photo-library churn). (`evo-daemon/src/threads.rs`)

### Phase 3 — the "intent" axis (mostly shipped; see §9 for the gap)
New canonical observation **`OBS-SURFACE-CLASSIFIED/v1`** = `{ subject, class }`,
`source = UserDeclared` (nothing else may write it). "This surface is not work / is
media." Every piece of it:

- **Schema** — `crates/evo-observation/src/observation_schema.rs`
  (`surface_classified_v1()`, registered in `is_canonical`, `is_reference_only`,
  `is_user_declaration`, `canonical_fact_name` → `"SurfaceClassified"`);
  concept variant in `observation_language.rs` (`SurfaceClassified { subject, class }`,
  evidence facts `SurfaceClassified` + `Class`); structural validation in
  `validation.rs` (`"OBS-SURFACE-CLASSIFIED" => Some("Class")`).
  **Deliberate deviation:** v1 carries no optional `productivity` field. If a later
  phase needs it, add `v2` (registry is keyed `(name, version)`; v1 stays readable).
- **Engine gating** — `crates/evo-threads/`: new `Declaration::Classified { resource,
  class }` (`types.rs`); `State.classified: BTreeMap<String,String>` (`state.rs`);
  `Decision::SurfaceClassified` (`trace.rs`); the gate itself in `assign.rs`:
  - `classify_surface()` records the class and **demotes** any existing anchor of that
    resource back to context (retroactive over the whole log via replay; no migration),
  - `commit_episode` refuses to mint an anchor from a classified resource's production
    (but still consumes the credit watermark and remembers it as a companion),
  - `recurrence_qualify` / `promote_recurrence_anchors` / `spawn_recurrence_thread`
    refuse to found a work by recurrence on a classified surface,
  - `sweep_parked`'s birth-production sum excludes classified credits,
  - `declare_anchor` refuses to anchor a classified surface,
  - `do_merge` refuses to copy a classified anchor.
  **Non-deletion guarantee holds:** the subject stays a participant and a companion. A
  class is *not* a quarantine (`Declaration::NotMine` is the quarantine; different thing).
- **Daemon read path** — `evo-daemon/src/threads.rs` projects
  `OBS-SURFACE-CLASSIFIED` → `Declaration::Classified`; new index
  `crates/evo-daemon/src/classification.rs` (`ClassifiedSurfaces`, built from
  observations like `NameIndex`); titling gate in `work_understanding.rs`
  (`understand_classified`, and `derive_title_named` skips classified subjects so a
  classified surface never names a work even as a companion/resume point);
  `display_threads_classified` + `display_threads_from_storage` wire it together.
- **Gates** — `crates/evo-ledger/src/thread_gate.rs`: two new gates
  `"a classified surface founds no work"` and `"a classified surface names no work"`
  (`evaluate_classified_gate`). The ledger log parser
  (`crates/evo-ledger/src/observation_log.rs`) now also returns classifications, so the
  shadow gate replays them (signature is a 4-tuple; all callers updated:
  `gate.rs`, `thread_gate.rs`, `bin/evo-ledger-engine.rs`, `examples/scale_harness.rs`).
- **Authoring channel (the writer)** — you cannot classify anything without this, so it
  was built:
  - `evo-capture/src/adapters/macos.rs`: `MacOSSignal::SurfaceClassified { subject,
    class, observed_at }` + the adapter translation to
    `ObservationConcept::SurfaceClassified` (with the standard non-canonical guards).
  - `evo-daemon/src/classification.rs`: the Unix-socket declaration channel
    (`classification_socket_path`, `ClassificationResponse`,
    `spawn_classification_listener`, `submit_classification`), mirroring `naming.rs` /
    `grouping.rs` / `continuation.rs` exactly. The listener validates shape only
    (a declaration is the person's word — witnessing is not a precondition for speaking).
  - `evo-daemon/src/runtime.rs`: `classification_adapter` (source
    `user_classification`), dispatch arm, handoff-subject arm, spawned listener, stored
    handle for cleanup.
  - `evo-desktop/src/state.rs`: `submit_classification` wrapper.
  - `evo-desktop/src/rack.rs`: `RackAction::NotWork(u64)` + a quiet "Not work…" control
    on the card and the stack row; `evo-desktop/src/app.rs` handles it by resolving the
    work's strongest witnessed member and sending the class.
- **Tests added:** `crates/evo-threads/tests/classification.rs` (4);
  `thread_gate.rs` unit tests (2); `evo-daemon/src/threads.rs` e2e
  `a_classified_surface_founds_no_work_and_names_none`;
  `work_understanding.rs` `work_understanding_a_classified_surface_never_names_a_work`;
  `evo-daemon/src/classification.rs` channel tests (3);
  `evo-desktop/src/state.rs` `submit_classification_forwards_to_the_daemon_and_reports_honestly`.

### Also done (the plan's §10.4 recommendation was NOT done)
`fastembed` is still a **non-optional** dependency of `evo-voice`, which is a non-optional
dependency of `evo-daemon` and `evo-desktop`. PLAN-0001 §10.4 recommends making it
optional behind a feature so the default build's dependency graph contains no
network-capable library, which would let the C3 guardrail strengthen from "no HTTP
client" to "no network-capable crate." **This is open work.**

### Verified numbers at handoff (from the last runs on this machine)
- `cargo check --workspace --all-targets --offline --locked` — clean; the only two
  warnings are pre-existing (`evo-daemon/src/name_index.rs:285`,
  `evo-daemon/src/continuation.rs:347`).
- `evo-capture --lib` 63 · `evo-daemon --lib` 195 (1 ignored) · `evo-desktop --lib` 249 ·
  `evo-ledger --lib` 63 · `evo-observation` 138 (+5 doctests) · `evo-threads` all suites
  pass (+4 new) · `evo-pods` 23+15 · `evo-desktop --test trust_constraints` 6.
- **NOT run to completion:** `evo-ledger/tests/regression_gate.rs` (pre-existing heavy
  real-log integration test — it exceeds the tool timeout). Run it yourself when you can
  spare the time.

---

## 9. WHAT REMAINS — the complete task, phase by phase

Do these in this order unless you have a strong, stated reason. Each has acceptance
criteria; honour them, and if you cannot measure one (e.g. it needs a human), say so under
"Limitations" rather than pretending.

### 9.0 FIRST: unblock the input (Phase 0 acceptance — this gates everything)
The machine currently reports `capture=partial`, `window-focus=unavailable`,
`detail=Accessibility permission is required to witness focused windows`
(`~/Library/Application Support/evo/storage/daemon.status`). **The single most
load-bearing signal (`Person/Focused`) is not being written.** This means Home is frozen
at the moment focus was lost, and the newest canonical records are ~23,000 file-churn
events. Consequences:
- Any measurement you make on the real log right now measures a starved engine. Fix
  capture first, or explicitly state that your measurement is invalid.
- Phase 2's acceptance metric (mean reciprocal rank of the work actually resumed next)
  is **specified but unmeasured**. Grant Accessibility, use the machine, then measure.
- Also confirm which binary is running. PLAN-0001 §8 items 2–3 warn the installed app may
  predate the tree (the tree is newer than the last built/installed agent). Confirm by
  checking strings of the running binary for `display_threads_from_storage`, not by
  reading source.

### 9.1 Phase 3 remainder — 3.2 and 3.3 (the "rules" engine)
Rationale (PLAN-0001 §3.3, §4 Phase 3): Evo may not ship built-in classification
(`youtube.com → Media` is forbidden by Architecture §5 / RFC-0014 / the arbitrary-work
research). It *may* hold **user-authored** rules, because those are the person's own
statements and are admissible as declarations (RFC-0014 R11).
- **3.2** `ClassificationRule { id, priority, enabled, matcher: {field, op, value},
  assign: {class, productivity?}, apply: Retroactive }` stored as **declarations**; a
  rule compiles to `OBS-SURFACE-CLASSIFIED` observations on creation and re-evaluates on
  replay. Matcher fields are structural (`App | WindowTitle | Url | DocumentPath | Repo`)
  and the *value* is the person's string — never a table Evo ships.
- **3.3** **Absorb + gap-fill** over classifications (ManicTime `[M]` mechanism):
  `absorb_max_gap` stored per rule; a short interval between two agreeing classifications
  inherits them; disagreement splits. This is the mechanism that lets a 90-second video
  page sit harmlessly inside a long coding session *without Evo knowing what YouTube is*.
- **Why now (and why it was deferred before):** the deferral reason was "no writer
  exists". The **authoring channel now exists** (§8), so the rules engine can be built and
  tested against a real gesture. Build the matcher **with tests that prove it consults no
  application/domain identity of Evo's own** (the guardrail is the point).
- **Acceptance (PLAN-0001 §4 Phase 3):** the existing gate `"no host is a work identity"`
  plus the two new gates pass on the real log; zero works founded on a user-classified
  surface. Note the honest limit: with no rules authored, this phase changes nothing by
  itself.

### 9.2 Phase 4 — naming that describes, locally
The name is the product. Phase 1.1 already killed identifier-titles and added the
`Untitled — N places, <duration>` fallback. What remains is the **local activity-phrase
ladder** and the metrics.
- Take Dayflow's checkable contract (`IntervalEvidence { activities:[{activity, minutes,
  subject, interaction_kind}] }` that **must sum to the interval**; `foreground_only:
  true`; `combine_recurring_visits`; `separate_subjects_sharing_an_app`) and make the
  naming *input* checkable.
- Ladder, which must never degrade to an identifier:
  1. the person's own name (`OBS-WORK-NAMED`, RFC-0014 — exists, outranks derivation),
  2. a local **activity phrase** composed from witnessed facts — interaction class
     (from `Interaction::{Typed, Saved, Downloaded, Mutated, Visited, Focused}` and
     `AnchorKind`) + the most specific *witnessed* object name. Example: `Typed` +
     `evo-capture` → *"Writing evo-capture"*. This is a small closed grammar over
     evidence Evo already has; it is not an LLM and it decides nothing about identity.
  3. `Untitled — N places, <duration>` (exists).
  4. Optional, opt-in, **local only**: the `evo-semantic` profile
     (`multilingual-e5-small`, offline, SHA-verified) may *verbalize* the phrase but must
     never decide identity. No network path, ever.
- **Watch out:** `ThreadView` currently does **not** expose per-resource interaction kind
  (episodes carry participants + attention; anchors carry only `AnchorKind`). You will
  need to derive the dominant interaction kind from the log (the daemon already does
  something similar in `name_index.rs`, which reads observations to build a presentation
  index) or thread it through the engine deliberately. Decide and document.
- **Metrics + acceptance:** title-readability judged blind by two people on a 30-work
  sample ≥80%; `"Untitled"` rate measured and falling; identifier rate 0%. Add both
  metrics to `evo-threads/src/bin/regression.rs`'s report and as a CI floor.
- **Risk:** auto-prefixing every title with a verb could *regress* the surface. Measure
  before/after and gate the change; do not ship a title grammar that makes titles worse.

### 9.3 Phase 5 — the feedback loop (highest leverage per line)
Today there are ~7 design events in ~53,000 observations — people cannot correct Evo, so
Evo guesses. Build the in-flow affordances and record each as a declaration:
| Affordance | Recorded as | Effect |
|---|---|---|
| Dismiss a work | `OBS-WORK-DISMISSED/v1` | off the surface; retained in the log; a repeated dismissal proposes a rule |
| "Not work" on a surface | `OBS-SURFACE-CLASSIFIED` (exists now) | retroactive re-run over all history |
| Rename a work | `OBS-WORK-NAMED` (exists) | title; a recurring rename proposes a rule |
| Mark member complete / superseded | `OBS-MEMBER-STATE/v1` | the answer to "which 2–3 of the 6" — feeds Phase 6 |
| This item is optional | per-member `optional` flag | Bunch's `_ignoreInSingleBunchMode` `[M]` |
| Ask when uncertain | `retrieval_margin` | already implemented; keep |
Reuse the channel pattern from §8 (socket + signal + observation + engine honouring). If
you add new observation schemas, follow the shape of `OBS-SURFACE-CLASSIFIED/v1` and
**document them** (optionally an RFC — see §11 item 6).
- **Acceptance:** design-event rate ≥ 5 per 1,000 observations (today ≈ 0.13). This is a
  *product* metric; if it stays ~0, the affordances are not in the flow.

### 9.4 Phase 6 — per-member lifecycle and the subset question
This is the person's #1 complaint and it is last because it needs Phase 5's affordance.
- Add to the engine's per-member record: `MemberState = Active | Completed | Superseded
  | Optional`, sourced in precedence order: (1) declared (`OBS-MEMBER-STATE`,
  authoritative per RFC-0014 R11), (2) derived from a commit/merge/close in captured
  evidence, (3) derived from non-return while siblings return → **labelled inferred** on
  the card (this promotes today's silent `restore_recency` filter into a named state).
- Restoration runs an **activation plan**: skip `Completed`/`Superseded`, skip `Optional`
  in reduced mode, keep `restore_recency` for the rest. Keep the existing `restore_cap`
  (5) and reduced mode (3) — they are cognitive limits.
- **Acceptance:** membership stability (Jaccard of presented members across two
  consecutive restores with no new evidence) = 1.0; restore precision (items kept open
  after 5 minutes) ≥ 80% over ~20 restores.
- **Why member lifecycle matters:** §5.4 of the plan — `restore_recency` already *is* the
  product's answer to "which 2–3 of the 6", but it is an invisible filter the person can
  neither see nor override. Promoting it to a named, rendered, correctable state is the
  highest product-improvement-to-code ratio in the plan.

### 9.5 Do-not-build list
- **OCR / screen reading** (Rize-style) — refused by C1, permanently. Do not reintroduce
  as "research".
- **Audio / ASR / diarisation** (Granola-style three processes) — refused by C2.
- **Built-in app/domain classification or allow/blocklists** — forbidden by Architecture
  §5. Only user-authored rules.
- **A persisted `SessionSnapshot` object** — refused by Amendment 1 ("nothing derived is
  persisted"). If you need Session Buddy's saved/previous split, extend
  `OBS-CONTINUATION-SURFACE` (RFC-0013) with `{kind: Saved|Previous, windows, links}`
  instead (§5.6).
- **A weighted urgency score** — forbidden; ordering must stay lexicographic over
  witnessed facts (§5.2).

### 9.6 Deferred (do not start without a decision)
- Calendar (EventKit) as naming evidence — opt-in, derived-only, never an input to work
  membership. After Phase 5.
- Idle / screen-lock detection (`OBS-SESSION-STATE/v1`) — a boundary refinement, not a
  correctness fix; slot into Phase 0's permission work if you're already there.

---

## 10. Known traps — mistakes already made that you must not repeat

1. **Fixing the wrong layer.** The RE blueprint diagnoses `evo-engagement` and
   `evo-desktop/src/state.rs` for two of the four complaints. Home is served by
   `evo_threads` via `evo-daemon/src/threads.rs`. **Verify the live path before accepting
   any diagnosis of "the pipeline."** (PLAN-0001 §5.1 says this is the single most
   valuable thing in the plan.)
2. **Copying a mechanism that Evo's law forbids.** Three of the RE recommendations are
   unconstitutional as written (built-in classification; weighted urgency; persisted
   snapshots). See §9.5 and PLAN-0001 §6's conflict log.
3. **Ordering by raw recency.** The log's newest records are machine churn (photo-library
   / cloud-sync writes). Ordering by "most recent event" ranks churn first. Order on open
   loops (shipped), never on the newest log line.
4. **Believing the source instead of the running binary.** The installed agent may
   predate the tree. Check the binary before deciding which pipeline the person sees.
5. **Adding a fact you don't consume.** The plan's own principle: a parameter/fact is
   admissible only when it expresses something the system honestly acts on. That is why
   `OBS-SURFACE-CLASSIFIED/v1` has no `productivity` yet.
6. **Schema process is mixed.** RFC-0011–0014 each introduced a schema, but the newer
   `OBS-SURFACE-NAMED/v1` and `OBS-INPUT-ACTIVITY/v1` shipped without RFCs. If you add a
   schema and want the paper trail complete, writing a short RFC (next number: RFC-0015)
   for `OBS-SURFACE-CLASSIFIED/v1` is the right move and changes no behaviour.
7. **The audit trail claims "verified" precisely.** When you finish something, add a row
   to PLAN-0001 §10.1 and a `verified / not verified` note to §10.3, in the same style.
   The next person's trust depends on this being accurate.

---

## 11. Commands you will use constantly

```bash
# whole-workspace type/borrow check (mandatory flags)
cargo check --workspace --all-targets --offline --locked

# targeted suites (fast)
cargo test -p evo-observation --offline --locked
cargo test -p evo-threads --offline --locked
cargo test -p evo-daemon --lib --offline --locked
cargo test -p evo-desktop --lib --offline --locked
cargo test -p evo-desktop --test trust_constraints --offline --locked
cargo test -p evo-ledger --lib --offline --locked
cargo test -p evo-capture --lib --offline --locked
cargo test -p evo-pods --offline --locked

# capture status on the live machine (Phase 0 acceptance)
cat "$HOME/Library/Application Support/evo/storage/daemon.status"

# keep the knowledge graph current after code edits
graphify update .
```

Preserve exit codes when you filter (`set -o pipefail`, or capture `$?` explicitly) — a
pipe through `grep`/`rg` that prints matching lines is not a passing test.

---

## 12. Your concrete deliverables / definition of done

Finish PLAN-0001. Concretely, in priority order:

1. **Phase 0 acceptance unblocked** or explicitly reported blocked: Accessibility
   granted, `daemon.status` shows `window-focus=active`, a new
   `OBS-WINDOW-FOCUS-GAINED` appears in the log within minutes of use.
2. **Phase 3 complete** — `ClassificationRule` store + deterministic matcher compiled to
   `OBS-SURFACE-CLASSIFIED` observations, plus absorb/gap-fill, with tests proving no
   built-in app/domain knowledge.
3. **Phase 5** — the feedback affordances and their observation schemas, reusing the
   channel pattern.
4. **Phase 6** — per-member lifecycle + the activation plan, with membership-stability
   and restore-precision measured.
5. **Phase 4** — the local activity-phrase ladder + identifier/Untitled metrics, gated on
   a before/after readability check.
6. **§10.4** — make `fastembed` optional so the default dependency graph is
   network-capable-crate-free, then strengthen the C3 guardrail accordingly.
7. Keep `docs/audit/PLAN-0001-understand-work-backend.md` §10 honest and current, and run
   `graphify update .`.

For every item: implement, test, verify with a captured exit status, and record what is
verified vs not. Do not weaken a test to make a check pass; fix the cause. If a check
cannot run here, report it as such — never as a pass.

---

## 13. First hour checklist (do this before writing code)

1. `git status --short | head -50` and `git log --oneline -12` — understand you are on
   `pods` with a large uncommitted tree.
2. Read `docs/audit/PLAN-0001-understand-work-backend.md` **in full** (§0.5, §4, §9, §10
   are the load-bearing sections).
3. Read `RE done/evo-reverse-engineering/evidence/MECHANISMS-SUMMARY.txt`, then skim
   `RE done/REA_EVIDENCE_REPORT.md` and the parts of
   `RE done/evo-reverse-engineering/EVO-REBUILD-BLUEPRINT.md` that PLAN-0001 §3/§6 cite.
   **Read them to take the permitted mechanism, not the forbidden one.**
4. Read `docs/rfcs/RFC-0011…0014`, `docs/architecture/ARCHITECTURE.md` (especially §5 Workspace Engine's
   "must not" list, §8 models, §10 privacy), and `docs/research/ARBITRARY-WORK-GENERALIZATION.md`.
5. Read `DESIGN.md` if you will touch UI.
6. Run `cargo check --workspace --all-targets --offline --locked` and the fast test suites
   in §11 — establish your own green baseline before changing anything.
7. Check `~/Library/Application Support/evo/storage/daemon.status` and accept that until
   `window-focus=active`, no real-log measurement is meaningful.
8. Only then pick up §9.1 (Phase 3 remainder) and proceed.

Good luck. The person this is for cannot resume their own work when Evo guesses wrong —
that is the standard you are held to.
