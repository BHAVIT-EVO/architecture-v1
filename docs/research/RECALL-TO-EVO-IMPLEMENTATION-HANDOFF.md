# Recall → Evo: evidence and implementation handoff

**For the next engineering agent. Research date: 4 Nov 2026.**

## Mission and non-negotiables

Make Evo reliably help a person return to a piece of work with the relevant resources, a truthful cue for where they left off, an editable preview, and explicit execution. Learn from the observable product behavior of **joinrecall.com**, but use original implementation and preserve Evo's architectural boundaries. **Never capture screenshots, screen recordings, video, pixels, OCR, or hidden screen contents, even temporarily.** Don't copy Recall's assets, branding or code. Work on the current branch; do not reset, clean, or delete the user's exports. Do not expose the user's actual work corpus publicly.

**Read this before coding:** distinguish Recall's published claims, Evo's code, and our proposed design. Nothing in this document establishes Recall's internal algorithms or that Evo currently works end-to-end on a particular Mac. Do not weaken existing trust or architecture tests to get a green result. If a proposed action conflicts with frozen observation schemas, architecture law, permissions, or the user's privacy choice, stop and propose an explicitly scoped change; do not smuggle in new canonical facts or default collection.

## 1. What was actually investigated

- Read Recall's public [product overview](https://www.joinrecall.com/), [docs index](https://www.joinrecall.com/docs), [Smart Return](https://www.joinrecall.com/docs/smart-return), [restore checklist](https://www.joinrecall.com/docs/going-back-to-work), [tray](https://www.joinrecall.com/docs/menu-bar-tray), [notes](https://www.joinrecall.com/docs/notes), [projects](https://www.joinrecall.com/docs/projects-and-threads), [loops](https://www.joinrecall.com/docs/loops), [agent sessions](https://www.joinrecall.com/docs/coding-agents), [asks](https://www.joinrecall.com/docs/asks-and-promises), [nudges](https://www.joinrecall.com/docs/nudges-and-reminders), [privacy](https://www.joinrecall.com/docs/privacy), [roadmap](https://www.joinrecall.com/roadmap), and [0.45 release](https://github.com/grinay/recall-public/releases/tag/v0.45.0). These are **vendor-reported**, not independently verified runtime behavior; pages can lag release notes and docs screenshots are explicitly fictional.
- Inspected the repository sources named in §4. **No macOS runtime, Rust toolchain, or REA MCP server was available here.** The official GitHub v0.45 DMG asset is listed (~21.6 MB), but the attempted download redirected to `release-assets.githubusercontent.com`, which this environment cannot reach. Consequently **no binary, model, screen, executable or runtime was inspected**; no REA evidence record exists. Do not say this is a full binary reverse engineering or claim any derived accuracy results. The REA skill is installed, not its server.
- The project sample has **10,392 events across about 48.08 hours**; these are inputs, not successful groups, users, restorations or an accuracy measure. The fixture may be private: keep it local. Read `crates/evo-threads/src/bin/regression.rs` before using the regression gate; never print its sensitive paths in published reports.

## 2. What Recall publicly says it does (not how its internals work)

| Area | Publicly documented behavior | Evo equivalent / limitation |
|---|---|---|
| Sensing | About every 15 seconds it samples the screen, runs a local model and discards the frame; it keeps descriptive metadata. Screen Recording + Accessibility are required. Some open asks can retain chat thumbnails. [privacy](https://www.joinrecall.com/docs/privacy) | **Different design is mandatory**: no visual frames, OCR or screenshots. Evo has AX focus metadata, URL/file events, and optional content-free input counts. Neither event counts nor titles can reveal the words of an unseen decision. |
| Work grouping | Projects from repo, document, site and people; threads are stretches of work, tasks group threads. User can rename, move, merge, archive, with some undo. [projects](https://www.joinrecall.com/docs/projects-and-threads) | Evo has observations → event replay → `ThreadView`/`ResumeBundle`, anchors/companions/forks, user grouping; exact parity in grouping is unproven. |
| Return | A compact card on request / after a break / drift; note first, explicit checked list, opt-out per item, last active item frontmost. [Smart Return](https://www.joinrecall.com/docs/smart-return) | Evo already has plans, native binding and a scoped single-resource UI. Need prove a single end-to-end, multi-item preview and execution path without bypassing gates. |
| Actions | One checklist from tray, thread, reminder, etc.; cancel opens nothing; disabled when no selected item; result must reflect actual effect. [restore](https://www.joinrecall.com/docs/going-back-to-work) | Build a single reusable pipeline; do not duplicate opening logic in UI paths. Report failures; arbitrary titles are not stable executable targets. |
| Notes | User explicitly records one line on a thread/project; note takes priority over inference. [notes](https://www.joinrecall.com/docs/notes) | Proposed durable, explicit-user note with provenance, if architecture accepts it. No invented next action. |
| Waiting | People/agents/notes; open-work inference has restrictive thresholds; user can mark done, snooze, correct. [loops](https://www.joinrecall.com/docs/loops) | Only witnessed deltas and user notes without separately authorized sources. Do not infer promises or semantic completion from activity timestamps. |
| Agent sessions | Session files polled ~30s; optional hooks detect explicit blocked approvals; a blocked session gets one nudge after 5 min; waiting alone doesn't nag. [agent guide](https://www.joinrecall.com/docs/coding-agents) | Proposed explicit opt-in, read-minimized integrations, separately reviewed. Never scrape terminal contents, install hooks silently, or say a session is blocked without trustworthy event evidence. |
| MCP | [Agent page](https://www.joinrecall.com/ai-agents-mcp) calls its MCP tools **exploring**, not a generally shipped integration; proposal is local, read-only by default, opt-in. | Evo MCP and enforceable work-scoped permissions are **not proven shipped** in this checkout. Design for a later phase, not a prerequisite for returning to work. |
| Multiple machines | [Roadmap](https://www.joinrecall.com/roadmap) marks moving history to another Mac and encrypted cloud sync **exploring**. | Cross-device sync is not verified in this checkout; it conflicts with an absolute no-network guarantee unless the user authorizes a new boundary. No sync in MVP. |

A crucial asymmetry: Recall's visual model can read content on the screen. Evo's metadata alone cannot infer a chat request, the actual unfinished question, whether a task is done, or a person's intentions. If content is essential, explicitly ask for a note or propose a **separate, off-by-default, narrow source with informed consent**, retention limits and a trust review. Do not read arbitrary AX text trees as a silent workaround for the no-pixel rule. It is still sensitive content.

## 3. Desired experience: first vertical slice

1. Person works normally in an editor and browser; interruption occurs; returns. No manual project creation required.
2. An explicit global action/menu command opens a compact, **non-focus-stealing** return view. Only add automatic presentation after reliable idle/lock/sleep evidence and false-positive validation.
3. Header shows *candidate work* and a provenance-backed cue. Explicit note if available; otherwise exact witnessed resource/title and last-seen time, never “you need to finish X” inferred from a filename. If uncertain, show candidates or “not enough evidence”, never silently choose one.
4. Preview reports exactly which known resources are eligible, unavailable, ambiguous or already open. Allow each selectable item to be unchecked. Nothing launches on opening/cancel/close or by merely displaying suggestions. Empty selection disables execute.
5. On explicit Return: revalidate identity, current plan revision, permissions and target. Execute selected eligible resources once; use existing executor rather than an OS-level side channel. Bring the supported current destination forward last, only when binding is proven. A URL opened in browser is not necessarily the original existing tab. Report actual per-item attempted/success/refusal/failure; do not assert native focus unless verified.
6. Keep canonical derivation and execution separate. An execution result must not rewrite prior observations or become evidence the human did this work. Closed, missing, inaccessible and contested resources have honest outcomes.

**Concrete scenario:** work A = a real local file + one browser URL; work B = another file. Capture and switch A→B→A; preview A, untick URL, Return; only A's file is attempted, B and URL are untouched. Interrupt, restart the app, revisit A. The exact observed outcome and any unsupported item must be visible. This scenario is a proposed acceptance test, not a claim that the app already passes.

## 4. Evo code inventory: inspect these exact boundaries first

- `crates/evo-capture/src/adapters/macos.rs`: `MacOSSignal` maps focus, saves, navigation, repo membership, user declarations, aggregated input activity. `macos_event_source.rs`, `macos_url_poller.rs`, `macos_fsevents.rs`, `repo_identity.rs` supply platform evidence. Verify which sources are actually wired at runtime and with which permissions; do not confuse a declared event type with working capture. `macos_input_counter.rs` requests **Input Monitoring**, records aggregate buckets not keystrokes, and has coarse attribution; it is not proof of active work when unavailable. Treat counts as optional and disclose the permission, not a requirement silently added to onboarding.
- `crates/evo-daemon/src/threads.rs`: `engine_from_storage`, observation-to-thread projection and `display_threads*`; FSEvents saves map to `Mutated` and use a churn gate; title-grain data can fail the person-corroboration gate. Trace the actual log and producer before changing thresholds. The old engagement/formation and threads pipelines coexist; establish which powers the active UI before editing. Never assume two similarly named IDs are interchangeable.
- `crates/evo-threads/src/{episodes,assign,continuation,config,brief,types,trace}.rs`: episodes, anchor/companion attribution, unresolved forks, `Engine::resume_bundle`, `activation_plan`, `explain`, and evidence-backed `WorkBrief`. `restore_cap` defaults to five, and continuation deliberately uses the latest worthy episode—not all historic resources. The `Delta` variants are observed state shapes, **not a semantic guarantee** of completion.
- `crates/evo-execution/src/{selection,binding,engine,preflight,locator,restore,browser_identity,browser_macos}.rs`: separate derived selection and live execution. `binding.rs` documents refusal on ambiguity and generic URL open limitations. Review actual current callers and Apple/macOS availability before promising browser tab, editor workspace or chat restoration.
- `crates/evo-desktop/src/{app,state,scoped_restore,rooms,pods,pod_surface}.rs` and `crates/evo-daemon/src/{ui,runtime,voice}.rs`: trace requests from visible UI through daemon to native execution. `scoped_restore` currently deals with **one** eligible locator; a multi-select UI is work to implement and audit, not something to claim exists. Preserve its timeout/pending/revalidation safety properties. Verify normal, room/pod and voice paths separately; don't bypass canonical selection via `rooms.rs` to speed up delivery.
- `crates/evo-desktop/tests/trust_constraints.rs`: existing coarse dependency/source and entitlement restrictions. They are useful guards, **not** a proof of comprehensive security. Check the actual optional voice and embedding features: mic is opt-in; no absolute “never listens” claim for voice-enabled builds. `docs/user/using-evo.md` warns the default installed bundle lacks voice models, so source availability ≠ shipped voice.
- `tools/threads-regression/{sample_events.json,ground_truth_labels.json}` and `crates/evo-threads/src/bin/{regression,evaluation}.rs`: use only locally, with consent, and measure both wrong merges and missed recall. Current regression reports pair colocation for observed pairs and checks fused labeled work / leisure anchors; inspect denominators and unassigned resources, not just a passing exit code.

## 5. Diagnose before tuning (first deliverable from agent)

Instrument a local, redacted, time-bounded trace of the actual pipeline. For one voluntarily chosen work, show counts/reason codes per stage, never dump real paths in reports:

```
permission/source health → raw events → canonical accept/reject →
replay projection → episodes → candidate work / fork →
work brief + restore set → selection and live binding →
user preview → approved execution → per-item report
```

For each stage record exact source, granularity (file / URL / title / app), time range, identities and reasons; redact/hide paths, titles and URLs by default. Separate unsupported, permission-denied, unobserved, ambiguous, filtered and error. Never treat missing observations as negative proof. Establish the starting baseline *before* adjusting heuristics. Verify capture on device with a disposable test folder and synthetic browser pages. Confirm watcher liveness and restart behavior; an event enum or passing fixture is not sufficient. A saved file that the producer identifies only by title cannot be safely joined to a filesystem path without evidence; don't fix it by guessing.

Likely bottlenecks to test, not diagnoses: (1) frontmost title vs document URL mismatch, (2) background saves failing person corroboration, (3) URLs and editor project roots lacking witnessed relationships, (4) shared chat/browser anchors yielding ambiguity, (5) old pipeline UI served while new threads are computed, (6) continuation cap/recency excludes desirable supporting surfaces, (7) restore preflight lacks enough locator identity. Trace first; change only the measured cause.

## 6. Proposed engineering plan, in order

### P0 — Baseline, privacy, and reproducibility

- Map actual entry points and output as in §5; record working-tree revision, permissions, macOS version and features enabled. Baseline `cargo test --workspace` and existing targeted tests on the user's Mac; use the existing ground-truth replay separately. Save aggregate results locally. Run on synthetic fixture first; do not upload corpus.
- Verify actual pause/exclusion/lock/idle handling across **all** capture producers, not just the menu-bar label. Add centrally enforced policy and tests if absent: while paused, locked or in excluded app, suppress content-bearing capture and aggregation. For background filesystem events, define source-safe filtering by owner/context and don't pretend an app-only rule can attribute arbitrary writes. Unknown owner must not bypass exclusion. On resume, never backfill excluded activity.
- Enumerate default permissions and defaults. Protect titles/URLs/paths as sensitive metadata. No screen permission, pixel APIs, screenshots, OCR, raw keys, clipboard capture, content scraping, hidden provider network, telemetry, or agent hooks added under the guise of parity. Existing opt-in voice is a distinct permission; no default activation.

### P1 — Honest return for one work (ship before agent features)

- Define a **single read-only `ReturnPreview`** adapted from existing canonical selection / engine bundle, after tracing which pipeline is authoritative. Include a stable identity and revision/version, source provenance, `resume_reason`, entries with exact locator, capability/status/reason, proposed primary item, and allowed selection; distinct raw identity vs display name. Never persist derived presentation state as canonical history.
- Build a single preparation and execution flow. On confirm send thread identity, plan revision, selected opaque entry IDs and a unique request ID. On daemon side rederive and compare: stale/missing/changed/ambiguous plan ⇒ reject and request fresh confirmation; selected items must be a subset of the freshly authorized candidate set. Never accept user-submitted arbitrary file paths as proof. Set a bounded in-flight latch, avoid duplicate execution after timeout; persist or retain idempotency for a documented TTL; timeout is **unknown**, not permission to retry immediately.
- Preview lists already-open vs exact openable vs unavailable; default selection only when safely bound. Because the existing canonical `selection` and threads `restore_set` have different semantics, explicitly document and test the mapping; do not conflate IDs or let UI-generated order change canonical history. If current policy forbids user override, propose and test the new explicit authority path rather than inventing it.
- Execute non-primary chosen items first; primary last **only** if this is authorized by the executor's contract and can be proven. Deduplicate resources by verified identity. Reprobe before each action. A race/change can produce a refusal; surface per-item results and never promise rollback for already-opened items. Prevent double-open when multiple windows match; ask rather than select the first.
- Preserve user privacy in the preview: no file-body reads, URL query-string or secret leakage in logs, no launching unapproved schemes; establish an allowlist and canonical path validation with tests. Do not open unknown links or arbitrary executables.

### P2 — Explicit notes and corrections

- Offer a short, optional thread note (“Where did you leave off?”) and explicit Done/Not done, with created/updated timestamps and provenance `user`. Store notes in a local user-declaration channel designed with the existing append-only law and deletion/redaction requirements; do not treat them as system observations. If schema or retention architecture doesn't support it, make that an explicit design review and migration, not an unversioned field hack. Resolve conflicts, restart and merge/split lifecycle before claiming durability. Never synthesize a note from merely focused titles.
- Expose existing grouping/rename declarations with confirmation. Only learn a correction at the appropriate entity boundary; no auto-merging across ambiguous shared resources. Support undo where canonical schema permits, or clearly disclose when unavailable. Keep uncertain hypotheses separate from user-confirmed relationships.

### P3 — Quiet suggestions and time

- Add one optional return-after-idle suggestion, gated on *actual witnessed* idle/lock/unlock and a prior strong candidate; no suggestion on first launch, missing permission, unresolved ambiguity, own/excluded app, or while composing/in a call when determinable. Dismiss = no effect on work history; cooldown and an off switch. Set thresholds from controlled labeled tests, not Recall's proprietary behavior. A fallback without reliable idle must say time is approximate or omit it; never count overnight focus as active time.
- A Today history is an optional view derived from evidence, not a dashboard or a source of membership truth. Account for time zones, midnight, overlapping activities, sleep, lock, DST, crash and restarts; unknown is not active. Establish deterministic tests before total-time claims.

### P4 — Agent hooks and portable context, behind separate approval

- Start with an opt-in documented hook for one supported agent, preferably a user-approved direct integration. User previews exact config patch/path, backups and uninstall; preserve pre-existing hooks and unrelated settings. Store only minimal event/session identifiers, time and state needed, with provenance. Treat unauthenticated local socket events as untrusted; restrictive permissions and origin/session validation. Never infer blocked merely from silence. Hook absence/timeout = unknown. No agent content collection by default.
- Design a work-context endpoint later: read-only, local, authenticated/allowlisted per client, source/selection-visible and revocable; narrow data minimization and retention. State clearly that output is evidence/candidates, not a guarantee of full intent or correctness. Mutation/restore require explicit approval. A structured context envelope is **not sandbox enforcement**. Any work-scoped filesystem jurisdiction requires a separate explicit authorization model, actual OS-enforced sandbox, symlink/path/race defenses, threat model and adversarial tests. The existing no-network trust rules may require a distinct reviewed boundary even for local IPC; do not weaken them silently.
- Cross-device later only after a user decision: no remote copy by default; threat model, key ownership, end-to-end encryption, device authentication, delete propagation, conflict resolution and offline recovery. Do not claim this exists.

## 7. Acceptance and adversarial tests

Add deterministic synthetic fixtures (no customer material) before platform automation. Tests must assert outputs AND absence of unauthorized actions:

1. **Grouping:** A→brief unrelated B→A retains A; sustained B births B; shared browser/chat does not fuse unrelated A/B; repeated resource alone cannot prove same intent; ambiguous tie produces fork, no guessed merge; explicit grouping wins only within canonical authority and survives replay.
2. **Noise:** background indexer writes, recurring plist churn, downloads, passive music and overnight open window never become anchors just because they are frequent. Check cross-repo common resources, generic window titles, person-corroboration timing and focus-swap race. Metrics include false merges, false splits, unassigned fraction and coverage, with denominator explicitly defined.
3. **Preview and execution:** open/close/cancel = zero side effects; unticked entries = zero attempts; selection rejects stale/replaced/missing thread; duplicate click, timeout, delayed reply and restart cannot launch twice; existing window focused only if uniquely identified; unavailable/ambiguous report reason; exact order and per-item outcomes; partial failure is not full success.
4. **Privacy:** no pixel capture, screenshot dependencies, screen-recording entitlement, OCR or upload; exercise static trust tests plus runtime spy/fake source. Pause, exclude, lock, inaccessible AX, and no-permission cases across all sources; no backfill. Optional input counts are not content and must not activate without their own permissions and disclosure. Secrets in paths/URLs/titles do not land in diagnostics. Tests must not depend on real credentials or personal data.
5. **Notes:** explicit note wins over machine cue, edit/delete/done persists appropriately, no note appears when none exists, merge/split behavior and undo are defined, deleted or excluded material does not resurface in derived caches or exports.
6. **Time and nudges:** lock, sleep, long idle, app restart, midnight and DST; no prompt without reliable state; respects off switch/cooldown; waiting ≠ blocked; no intrusive focus stealing.
7. **Agent bridge (only if approved):** hooks are opt-in/removable, unrelated config preserved; spoofed, stale, repeated and out-of-order events do not grant access or create fake blocked states. MCP read-only and authorization tested separately from operating-system containment; malicious path, symlink, unexpected network destination and tool injection denied by actual enforcement.
8. **Mac manual end-to-end:** freshly installed normal build, real TCC denial/grant/revocation, synthetic editor+browser, interruption, note, restart, selective return, already open window, missing file, no screen permission, logs inspected. Record steps, elapsed results and exact build version; test after reboot and with locked screen. If browser support cannot restore original tab, report “opened URL” rather than “restored tab”.

Suggested local baseline commands (do not execute unreviewed scripts as root):

```sh
cd /path/to/architecture-v1
git status --short
cargo test --workspace
cargo test -p evo-desktop --test trust_constraints
cargo run -p evo-threads --bin regression -- \
  --events tools/threads-regression/sample_events.json \
  --labels tools/threads-regression/ground_truth_labels.json
```

Run commands only if Rust/macOS prerequisites exist. These were **not run in this sandbox**. The regression command can print private paths/labels; redirect to a protected local file or use a redacted fixture and never upload. A passing regression is not end-to-end proof. Evaluate changes against a held-out set from multiple opted-in users and measure explicit baselines before asserting improvement.

## 8. External parity matrix and trade-offs

| Capability | Without pixels, achievable source | Status/constraint |
|---|---|---|
| Resume files and URLs | AX focused document identity, witnessed URLs and supported OS open/focus | Existing architectural components; live coverage and success need device test. |
| Resume exact browser tab | Browser-specific trusted integration or identity API | Generic layer says it cannot identify existing tabs; current generic fallback opens URL, possibly duplicate. Ask before any extension/automation permission. |
| Resume IDE workspace or agent chat | Verified project/file identity, supported editor deep links or opt-in hook | App-specific; verify capability and prevent duplicates. No guessing from title. |
| Reason/next step | Explicit user note; narrowly witnessed state | Without semantic text there is no automatic explanation of a decision or next task. |
| Unanswered asks/promises | Explicit manual entry or separately consented integrations | Not possible to reconstruct reliably from timestamps/window titles. Never silently read AX chat bodies. |
| Model-independent context for agents | Read-only local interface over approved evidence | Proposed, not shipped. Separate scope for enforcement. |
| Time per work | Focus + reliable idle/lock/sleep signals | Derived estimates; show uncertainty, not employment surveillance. |
| Cross-device | Explicitly designed and approved secure transfer | Not in inspected code; may contradict current local-only invariant. |

## 9. What to send back to Bhavit

Give a short baseline report: exact build/permissions, the first failing stage in §5, which fixes shipped, synthetic test results and on-Mac steps/results, privacy review, gaps and next task. Do not claim tested macOS execution from Linux, data-driven group accuracy without denominator, exact Recall parity, or agent security without enforcement. Do not capture screenshots or screen recordings while testing, and use synthetic rather than private work. Ask for one reproducible failure and a synthetic substitute if the real example contains sensitive data.

### Optional further investigation on an authorized Mac

Obtain the current Recall binary from its official download, verify its origin/signature and license, and inspect only what is permitted. Run it with synthetic data, no private accounts, and document UI behavior vs release notes. Use the installed REA skill only after configuring a supported REA server / CLI and prerequisites; review its setup plan first. Do not assume the current skill installation supplied the MCP server. Observe permissions and network behavior without bypassing authentication or security controls. Mark each finding as observed vs vendor-stated vs hypothesis. Private internals remain unknown until actually measured.
