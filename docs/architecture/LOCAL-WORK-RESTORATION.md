# Evo: evidence-backed, local work restoration

Date: 2026-10-02. Status: implementation brief and research findings, not a claim of universal correctness.

## Mission

Return a person to a specific ongoing intention, with the resources and recoverable state needed to continue it. A work is not an application, a folder, a time interval, or a connected component of everything used nearby. One resource can participate in several works; one work can span arbitrary applications and days.

The system must distinguish four questions:

1. What did the machine actually observe?
2. Which work might that occurrence belong to, with what evidence and alternatives?
3. Which resources and state should be brought back for this request?
4. What did the operating system actually restore?

Evidence, inference, execution authority, and execution receipts are separate. More fluent narration does not repair missing capture evidence. Similarity is not a probability of correctness or permission to act.

## Findings at the start of this audit

- Home reads the `evo-threads` replay through `evo-daemon/src/threads.rs`; voice TRIGGER still resolves the older Engagement reconstruction in `evo-daemon/src/voice.rs`. ASK and numeric CHOOSE use threads. These are incompatible authorities for the same product action.
- `evo-execution/src/restore.rs` uses the first matching title/document window, despite an existing uniqueness rule in `macos.rs`. The native raise path also stops at the first title in a process. Duplicate titles are therefore a real wrong-window risk.
- `evo-voice/src/embed.rs` constructs fastembed's downloading loader. Local inference after a download is not the same as a strictly offline runtime.
- `evo-daemon/src/work_understanding.rs` derives phase from URL ratios and treats episode anchors as saves. These are interpretations, not witnessed facts.
- `evo-threads/src/assign.rs` promotes three sustained surfaces as intentional workspace setup. Dwell/co-presence alone cannot prove that intent. Conversely, a single reading session can be genuine work without typing or recurrence.
- `scope_restore_set` in `evo-threads/src/retrieval.rs` returns the full set when a named scope matches nothing. Unknown scope must not become permission for a broader restore.
- Captured window identity exists, but the replay's opaque resource string and the executor's title/PID binding do not preserve every occurrence's window identity end to end. App launch, document open, tab restore, and exact-state restore remain different capabilities.
- Desktop pods dispatch restore-set URLs through the normal macOS handler. This is intentionally not a promise to restore an original browser profile, authenticated tab, navigation history, or unsaved web form.

## First implementation boundary

Preserve existing user edits and canonical history. Do not replace capture schemas, invent work labels, change grouping thresholds to fit examples, launch the live daemon, or release the app during this pass.

1. Unify daemon voice reads and restoration on request-time thread facts. Keep uncertainty and read failures explicit. Never offer a resource as unopened if this request already attempted it.
2. Refuse ambiguous window binding at planning and execution, including revalidation inside the native raise operation. Prefer witnessed application evidence without guessing app purpose.
3. Load the existing embedding model only from local files/cache. Missing assets must return a useful local error without triggering a network download.

These corrections close concrete integration and safety defects. They do not solve all work inference, add universal adapters, or establish production readiness.

## Target mechanism

### Observation and identity

Capture permissioned events incrementally: focus, navigation, document changes, explicit user corrections, and adapter-provided state. Maintain a session/boot identity, process lifetime, window identity, browser profile/window/tab identity, resource identity, and occurrence timestamp separately. A window ID is not a durable resource ID; the same URL may appear in two profiles and two tasks. A renamed document needs an observed identity link, not a filename guess.

Keep a local append-only evidence ledger with provenance, schema version, and capture coverage. Mark unavailable fields explicitly. A missing Accessibility permission must not look like an empty desktop. Treat OS filesystem changes as machine activity unless human evidence corroborates them. Do not collect passwords, private browsing, or excluded apps; pausing, deletion, retention, and revocation must affect derived caches too.

Use event-triggered updates with bounded reconciliation, not a large model reading the whole desktop every second. Capture latency, queue drops, CPU, memory and battery need measurements. Continuous awareness does not require continuous expensive inference.

### Work hypotheses

Represent a work as an evolving intention hypothesis linked to individual resource-use occurrences. Membership is many-to-many and time-scoped, with supporting evidence, contradictory evidence, uncertainty, and user corrections. Shared resources do not transitively merge tasks. A confidence score is not calibrated merely because it is between zero and one.

Combine witnessed titles/content (only with capture permission), local semantic representations, repeated selective coactivity, explicit relationships, and interruption/return patterns. None alone proves belonging. File extensions, app names, professions, long dwell, typing, and recency must not define whether something counts as work. Reading, watching, composing, preparing, comparing, and monitoring are valid possibilities, not compulsory categories.

Retain unresolved activity rather than guessing it into a task or silently losing it. Ask a small clarification when two hypotheses cannot be distinguished. A correction must be scoped to a work/occurrence, not globally declare that a shared document belongs exclusively to one task. Preserve split/merge lineage before persisting identities independently of replay.

### Retrieval and execution

Parse request intent, temporal constraints, and requested scope separately. "Yesterday" must filter dated occurrences in the person's timezone; an embedding of the word yesterday does not implement this. "Only the slides" must never fall back to reopening everything when matching fails.

Produce a reviewable restore plan with exact targets, evidence, alternatives, freshness, expected capability, and planned side effects. Keep strong targets, optional supporting material, and unresolved candidates separate. Bind each action against current OS state. Prefer reuse over duplication. Do not close unrelated windows, discard unsaved edits, replay commands, submit forms, or replace newer state to reconstruct a past workspace.

Each adapter declares capabilities: activate app, open resource, reuse exact window/tab, restore position/selection, or unavailable. Capability-specific code is necessary and is not a profession heuristic. Arbitrary applications cannot be promised exact restoration when no observable state or supported control exists. Unsaved buffers and authenticated web sessions require application cooperation; opening a URL in a fresh embedded browser does not recreate them.

Record each attempt and independently verify its postcondition. OS acceptance is a dispatch receipt, not proof that the requested document is visible. Correlate restore-origin observations before dispatch, reconcile success/failure, and never learn Evo's actions as fresh user intent. Retry only after inspecting current state.

## Local intelligence and cost

Keep deterministic bookkeeping, identity, exclusions, permissions, provenance, and action validation outside model control. Use an embedding model to retrieve candidates; optionally use a local language/vision model to propose evidence-linked explanations or reranking. Model output cannot create a resource locator or authorize an action.

The current MiniLM model is an English sentence/short-paragraph encoder, not a universal work-understanding model. Its model card specifies 384 dimensions and default truncation beyond 256 word pieces. Long concatenated work histories therefore need chunked, occurrence-aware retrieval; appending more strings is not unlimited context. Its threshold measured on one person's examples is not evidence of cross-profession reliability.

Free-to-use libraries and appropriately licensed weights can avoid API fees. They still require hardware, storage, electricity, packaging, and evaluation. Training a general-purpose foundation model from scratch is not a credible zero-budget prerequisite. This pass explicitly provisioned the already-approved MiniLM export; no new foundation model was trained. Before selecting new weights, measure target hardware/language needs. Multilingual E5 is a candidate for evaluation, not an untested drop-in replacement for MiniLM's thresholds.

Runtime inference and historical data stay local. Opening a requested website naturally contacts that website; it must not upload unrelated observation history to an AI service. This distinction should be explicit in the privacy promise.

## Research sources and limits

Public pages read on 2026-10-02; no private work history was submitted in research queries. Search returned no results, so these are targeted primary-source findings, not an exhaustive market survey.

- [Littlebird](https://littlebird.ai/): currently advertises cross-app actions and recall; its FAQ says captured data is stored in AWS. Do not describe it categorically as chat-only or local-only. Its actual selective-restoration fidelity was not tested.
- [Hansel.io](https://hansel.io): the supplied domain describes product-onboarding nudges, not the desktop-memory competitor. The intended competitor needs its exact URL.
- [Mylyn User Guide](https://github.com/eclipse-mylyn/org.eclipse.mylyn/wiki/User-Guide): task context, degree-of-interest filtering, manual corrections, and reopening editors on task activation are established ideas. This supports task-scoped relevance and shows restoration itself is not an entirely new category; it does not establish automatic task inference for all applications.
- [Chrome sessions API](https://developer.chrome.com/docs/extensions/reference/api/sessions): explicit session IDs can restore tabs/windows; recently-closed queries are bounded (maximum 25 entries). A universal history cannot rely only on recently-closed sessions. Cross-device methods are not appropriate for Evo's local-only requirement.
- [MiniLM model card](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2): Apache-2.0, English, sentence/paragraph similarity, 384 dimensions and documented truncation. License/export compatibility must accompany packaged weights.
- [Multilingual E5-small model card](https://huggingface.co/intfloat/multilingual-e5-small): MIT-licensed multilingual candidate, requiring independent Evo-specific evaluation and model-specific input preparation.
- [llama.cpp](https://github.com/ggml-org/llama.cpp): MIT-licensed local inference runtime with Apple Silicon support. A runtime is not trained weights or a guarantee of accurate work attribution.
- [Generalization decision record](../research/ARBITRARY-WORK-GENERALIZATION.md): subsequent primary-source research, multilingual profile wiring and metamorphic evaluation, with limits kept explicit.

## Release gates and next milestones

1. **Identity end to end:** carry occurrence/window/profile/tab identities into executable targets; verify duplicates, process restarts, renames, missing permissions, closed resources, and shared documents without ordering-based choices.
2. **One inference authority:** retire remaining older paths only after parity and migration tests. Extract the shared replay helper rather than maintaining the temporary voice projection duplicate indefinitely. Fix unsupported scope and time filtering before promoting natural-language restoration.
3. **Evidence-quality evaluation:** synthetic invariants plus opt-in, locally labelled real workflows across professions and languages. Test three interleaved tasks in one app, one file in ten works, 40 tabs/20 apps, reading-only work, prepared-but-not-started work, long gaps, arbitrary names, and machine churn. Rename all apps/resources in a fixture and require identical attribution; this detects hidden category rules.
4. **Safe execution:** per-action provenance before dispatch, verification receipts, idempotency, stale-plan refusal, and explicit partial outcomes. Uncertainty must never silently widen the requested scope.
5. **Privacy/offline packaging:** package or explicitly provision pinned model assets, audit licenses, test with network denied, confirm no telemetry or historical-content egress, test deletion/exclusions across all indexes. The source change alone is not a complete privacy certification.
6. **Performance and field validation:** benchmark growing histories, incremental replay, cold start, battery, sleep/wake, permission loss, and capture outages. Never tune only to one owner's corpus or exempt failing safety gates as proof of success.

Measure wrong-work restores, wrong-resource opens, missed required resources, unsupported-state rate, clarification rate, time-to-resume, and resource cost separately. A system that restores nothing can have zero wrong opens and still be useless. No broad release until both safety and useful restoration meet declared acceptance criteria on held-out workflows.

## Implementation receipt

The first boundary corrections are implemented in the working tree, not committed or released:

- `crates/evo-daemon/src/voice.rs`: all voice commands use request-time thread facts. Retrieval and execution share the same snapshot, with witnessed names supplying vocabulary. Corrupt/incomplete history fails explicitly without truncating a concurrent writer's log. Ambiguous retrieval/forks remain explicit; unknown IDs have no legacy-engine fallback. Restore candidates come only from the selected thread's resume set, and attempted targets are not misreported as unattempted offers.
- `crates/evo-execution/src/restore.rs`: planning and execution reject duplicate title/document matches, retain witnessed-app constraints in the main restore path, and distinguish inspection errors from empty results. Tests use an injected platform, not real window actions.
- `crates/evo-execution/src/macos.rs`: native raise rechecks every title in the owning process before acting; duplicate/unreadable matches refuse. CoreFoundation ownership is released through guards on success and error paths.
- `crates/evo-voice/src/embed.rs`: initialization reads local ONNX/tokenizer files through fastembed's user-defined model API. No hub client or automatic download is constructed by this loader. Existing cache snapshots and explicitly configured flat model directories are supported; missing assets fail rather than using a remote fallback. `EVO_EMBEDDING_MODEL_DIR` selects the explicit directory. Assets are not included by this change.

Independent verification: `cargo test --workspace --lib --offline --locked -j 1 --no-fail-fast` passed **1,009 tests**, failed **0**, ignored **2**, across **16** library suites. Targeted Rust formatting checks pass with each crate's actual edition. Cargo.lock is unchanged from this session's baseline. Tests of local model file resolution/corrupt assets passed; successful inference with real weights and real desktop restoration remain unverified.

The subsequent full run, `cargo test --workspace --offline --locked -j 1 --no-fail-fast`, also passed: **1,123 passed, 0 failed, 3 ignored**, including integration and documentation tests (57 suite results). Two existing daemon test warnings remain (unused import and variable). Passing the existing suites does not validate every inference assumption: some currently encode the heuristics this report flags for replacement.

`graphify update .` completed locally using AST extraction: 7,288 nodes, 17,169 edges, 355 communities. Its visualization is aggregated because the graph exceeds 5,000 nodes. It warned that root Cargo.toml yielded no AST nodes. No semantic/cloud graph extraction was requested.

Remaining concerns: voice still uses OS-default handlers instead of occurrence-specific app/profile bindings; provenance marking occurs after batch execution; the older separate `binding.rs` path delegates multiple document matches to the OS; title/PID binding does not eliminate PID reuse or post-inspection races. Successful OS dispatch is still not independently verified restoration. Model assets must be installed locally before semantic voice matching works. Existing category assumptions elsewhere were identified, not comprehensively removed.

Durable engineering lesson: passing tests in a strict executor does not protect another live executor that bypasses it. Trace Home, voice, and native actions all the way to their real implementation before claiming an invariant holds product-wide.

## Second implementation pass (2026-10-03)

This section supersedes the first-pass concerns about duplicated voice replay, post-batch provenance marking, and the older binding path. It does not supersede the unresolved product-wide capabilities below.

### Implemented and connected

- **Shared replay:** Home/storage and voice call `evo_daemon::threads::engine_from_observations`. The adapter preserves execution-origin events instead of relabelling them as human input; execution-origin declarations cannot merge threads. Time conversion and settlement use saturating arithmetic. Tests persist observations and verify that replay does not manufacture work from Evo's restores.
- **Pre-dispatch witness lifecycle:** voice reserves each target before its OS call, without holding the registry mutex during execution. Each reservation has an operation ID and at most two exact identities: original resource and the executor's classified locator. Completion affects only that operation; failure releases it; genuine input/declaration hands all aliases back. Late completion cannot revive a handed-off operation or cancel a newer overlapping attempt. The registry is bounded to 1,024 operations with expiry.
- **Receipt reconciliation:** a file URI and the exact path returned by the existing executor can identify the same dispatched attempt. No new URI decoder or broad equivalence rule was introduced. Exactly one matching receipt is required; duplicates, conflicting result buckets, alien resources, and empty receipts fail. An all-failed request reports `Failed`, not `Restored`.
- **Explicit scope:** `resolve_scope` returns `Unscoped`, `Matched`, `Ambiguous`, or `NoMatch`. Nonempty misses and ties do not become an executable full set. Exact locators take precedence over embedding; invalid model results propagate. Unscoped resource order preserves the resume point; deduplication preserves first occurrence. Locator matching preserves meaningful trailing spaces and URL path slashes.
- **Connected scope transport:** daemon and `VoiceSocketClient::scope` support `SCOPE <thread_id> <UTF-8-byte-length>\n<surface-query>`. It resolves against one selected bundle from one request snapshot. An exact locator can work without a loaded model. Empty, malformed, unknown, ambiguous, or unmatched scope requests execute nothing and never fall back to CHOOSE. This is a structured API, **not yet a visible scope UI or a natural-language intent parser**.
- **Semantic input checks:** nonfinite/empty/zero-norm embeddings, invalid thresholds, inconsistent dimensions, and duplicate indexed document IDs fail explicitly. Similarity remains a heuristic, not a calibrated correctness probability.
- **Consistent file binding:** the older `bind_resource` path now refuses multiple document matches and inspection errors rather than handing the ambiguous choice to the OS. Unique matches focus; a successful empty enumeration permits opening the exact file. Desktop test fixtures now explicitly model that empty-enumeration case instead of inheriting an unsupported-inspection error.

### Validation and limitations

Targeted tests cover reservation concurrency, in-flight and completed alias handoff, scope isolation over Unix socket fixtures, persisted replay parity, corrupted model output, and independent restore-target progress. A desktop fixture regression was reproduced before updating its explicit fake platform evidence; the production refusal was not weakened to make tests pass.

Final verification, including the exact-locator-with-trailing-space socket regression: `cargo test --workspace --offline --locked -j 1 --no-fail-fast --quiet` passed **1,165 tests, 0 failed, 3 ignored**, across 58 suite results. Targeted formatting and whitespace checks passed. Earlier compilation exceeded a 120-second command budget, so verification was rerun with a longer timeout rather than treating that interruption as a test failure or success.

The local AST graph was refreshed after implementation (7,379 nodes, 17,570 edges). Root Cargo.toml still yields no AST nodes; changed communities received hub-based labels rather than cloud-generated semantic labels. These graph warnings do not establish source/test defects.

No new model, dependency, cloud service, paid API, or observation-history migration was introduced. Cargo.lock remains unchanged from this pass's baseline. Existing warnings are not treated as evidence of failure or hidden as fixes.

**Still not solved:** task inference accuracy across professions; occurrence-local state and stable browser/profile/tab identity; natural-language dates and scopes; a visible scoped-restore flow; original authenticated browser-session restoration; verified postconditions; and desktop pod restore's bypass of the daemon witness registry. TTL matching remains bounded correlation, not proof that a focus change was caused by Evo. Slow effects, missing capture evidence, and other restore callers need explicit treatment. Local-model successful inference and live desktop restoration require separate validation before release.

Next implementation milestone: carry stable occurrence identities and adapter capabilities from capture through a common restore-plan API, then route both desktop and voice through that API. Do not claim that the new structured scope transport interprets unrestricted spoken requests.

## Third implementation pass (2026-10-05)

### Witnessed identity and executable constraints

`SurfaceIdentity` is optional observation context, not an Artifact identity or an assertion of task membership. It carries a capture-session namespace, registry window ID, positive owning PID, witnessed process launch-date token, optional AX window identifier, and optional browser-native window/profile/session/tab constraints. The process token is the hexadecimal bit representation of `NSRunningApplication.launchDate.timeIntervalSince1970`, not PID alone. All fields roundtrip through the existing local observation codec. Malformed or partial context does not become a weaker valid identity.

Capture samples the retained focused AX window for its title, document locator and AXIdentifier, refreshes process lifetime before focus deduplication, and namespaces registry IDs across capture restarts. A changed title does not become a new native identity; duplicate titles do not supply identity. The existing CG front-to-back focus heuristic remains registry provenance, **not an actionable native handle**. AXIdentifier may be absent, duplicated or reused by an application, so its availability is a capability, not a universal guarantee.

The daemon derives target constraints from human focus evidence in the selected work's last assigned episode and the same request snapshot used for selection. It does not attach a different work's latest sighting. Multiple witnessed identities for one resource or malformed identity context refuse restoration instead of silently choosing one or reopening without constraints. Execution-origin observations cannot supply replacement human identity. A new episode after user handoff needs its own witnessed occurrence; a logical resource is not proof that an earlier window is the current stopping occurrence.

`RestoreTarget.surface` carries these constraints to the executor. A constrained restore:

1. Validates the entire identity and rejects unsupported profile/tab constraints.
2. Checks process launch lifetime, enumerates that process's AX windows, and requires exactly one witnessed AXIdentifier. Unreadable candidates and duplicate IDs refuse selection.
3. Retains the selected AX element; verifies its identity and exact requested AXDocument/AXURL/AXTitle before raising it. A window now showing another document or page is not an eligible substitute.
4. Raises once, rechecks process lifetime, activates the process, and verifies global focus, window identifier and exact resource afterward.

There is no URL/path/title fallback when a surface constraint cannot be honored. A post-dispatch mismatch is a failed verification, not a successful `raised` result. `restore_with_receipt` and `execute_with_receipt` expose per-target dispatch evidence separately from `VerificationResult::{Verified, Unverified, Failed}`. Ordinary OS opening remains an accepted command, not proof of recovered editing, login, selection, or scrolling state. Generic URL postconditions remain unverified without a browser adapter.

**Not solved by these tokens:** a closed/replacement window reusing the same AXIdentifier within one process lifetime; process-start identity across all platform/boot conditions; or external browser session identity. Browser profile/tab fields are supported constraints but generic macOS capture does not witness them and the native executor cannot fulfill them. The code does not fabricate tab IDs from URLs, profiles from app names, or native handles from registry counters. A local, explicitly installed browser adapter is still required for real profile/tab restoration.

The existing pre-dispatch witness registry also remains resource/alias based and time bounded, not an occurrence-specific causal channel. It can correlate another window showing the same resource during its TTL; genuine input releases that suppression. Identity-aware execution does not by itself solve this capture-provenance limitation or the remaining desktop pod bypass.

### Visible scope controls and strict transport

The rack now offers **Restore selected…** for each real work. Its native sheet shows the canonical work ID/name and all distinct eligible current restore-set locators. The user chooses exactly one resource; selection, pending state, results and stale-list checks are keyed by thread ID and exact locator, not rack position. Long locators scroll without becoming different identities.

The worker uses `VoiceSocketClient::scope_exact` and the new strict `RESOURCE <thread_id> <byte_len>\n<exact UTF-8 locator>` command. The daemon checks fresh bundle membership byte-for-byte and never invokes semantic matching for RESOURCE. Removed resources fail with a refresh/choose-again message, even if another resource looks similar. Existing SCOPE remains the explicit text-matching API; CHOOSE remains unscoped.

The sheet does not enter a pod, rearrange other windows, open extra resources, or rewrite work membership. It prevents duplicate dispatch while pending. A timeout is **unknown**, not cancellation or success; closing the sheet does not undo a dispatched effect, and no automatic retry is performed. The visible report describes the daemon's actual result and does not claim independent UI/native verification.

Try the synthetic UI without personal history or restoration effects:

```sh
cargo run --offline --locked -p evo-desktop --example scoped_restore_preview
```

The preview exercises the native sheet with synthetic names/resources and does not connect to the daemon. It was compiled; native visual interaction is not yet independently verified in this pass. The real flow is rack → Restore selected… → choose one listed resource → Restore this resource.

### Broader inference checks, not an accuracy claim

Removed the unsupported birth rule that inferred deliberate workspace setup from three surfaces receiving three seconds of attention. A single attention-only episode stays unassigned without qualifying production, an explicit relationship, or supported recurrence. The existing fragment safeguard also applies when reconsidering parked episodes, so substantial unknown activity is not retroactively attached just because it overlaps another work.

Work narration now uses neutral **Observed activity**, not URL/file-ratio guesses such as research or building. Anchor presence no longer claims a save, download or completed task. Actual input/save deltas remain visible with evidence-specific language. Legacy phase variants remain for compatibility, but this inference path does not assign their semantic meanings. Malformed empty URLs cannot crash display-name extraction.

`crates/evo-threads/tests/fictional_workflows.rs` supplies 11 fictional workflows and 27 labeled resource probes: multilingual files/images/URLs, overlapping shared resources, unrelated attention-only tabs, real production-free recurrence, explicit human co-membership, execution feedback and distinct opaque identities sharing a displayed title. Its measured scripted relation counts are 16 true-positive relations, zero false positives and zero missed labeled relations; 12 negative probes safely abstain. Nine additional attention-duration/resource-count combinations test false-birth refusal.

These are synthetic regressions with supplied evidence, **not held-out real-user accuracy, language-model evaluation, or proof of semantic comprehension**. Native capture still uses locator/subject identity for inference; a fixture with two opaque resource keys does not prove that every real application's same-title windows are separated end to end. Remaining older suites contain profession/leisure stories and underasserted probes; they do not justify universal task labels. The machine-local history gates and committed personal-history corpus are deliberately excluded from this pass's combined test run.

### Isolated live verification receipt

New opt-in probes and a synthetic fixture are included:

- `crates/evo-execution/examples/verify_fixture_document.rs`: inspect only by default; explicit `--open` dispatches one supplied fixture document and polls its focused AXDocument postcondition for up to three seconds. It does not enumerate the desktop or log unrelated titles/locators.
- `crates/evo-execution/examples/fixture_app.swift`: an isolated AppKit app with two same-title windows, distinct AXIdentifiers and synthetic document locators; no personal history or network. It terminates itself after three minutes if launched.
- `crates/evo-execution/examples/verify_fixture_identity.rs`: explicit `--raise <fixture-pid> <launch-bits> <AXIdentifier> <absolute-fixture-file>` exercises exact native identity/resource selection. Its logical registry tokens are fixture-owned, not claimed to come from Evo capture; passing it would verify the native adapter, not the complete capture→daemon path.

The Swift fixture compiled locally. Both inspect-only permission probes returned `NotDispatched` and `Unverified: Accessibility permission is missing; cannot inspect windows`. After the user chose to grant permission, this runner still observed missing Accessibility. **No restoration action or fixture-app launch was performed; native success remains unverified.** Do not count the compilation or mocked adapter tests as live proof. Enable Accessibility for the responsible app/terminal in macOS System Settings, confirm the grant is effective to the probe, then run only the isolated fixture. Do not automatically repeat an uncertain restore or read unrelated personal state to compensate.

Follow-up after the user's “ready”: the previous temporary fixture had expired, so a fresh synthetic fixture was created. The inspect-only document probe again returned `NotDispatched` and missing Accessibility. The user confirmed enabling the main OmniRush.ai entry; the probe's immediate launch chain runs through OmniRush.ai's opencode sidecar. No fixture app or restore was launched. Next setup check is a fresh helper after restarting OmniRush.ai; if it remains denied, inspect the exact verifier's grant rather than assuming the main application's grant covers it.

Subsequent permission diagnosis supersedes the suggestion to grant the verifier alone: narrowly filtered TCC diagnostics confirmed the verifier's current code identity was explicitly allowed, but the probe's effective Accessibility request was evaluated against responsible application `ai.omnirush.desktop` and denied. The installed OmniRush.ai bundle validates its code signature. The cause of the responsible application's denial is not yet established; an enabled UI toggle is not an effective-grant receipt. Refresh only the installed `/Applications/OmniRush.ai.app` Accessibility entry directly in macOS and restart that application, or perform the isolated test manually from a separately authorized terminal. No TCC reset, permission-database edits, re-signing, fixture launch, or restore was performed. Raw diagnostics remain outside the repository.

### Verification and next boundary

The only dependency addition is a path edge from evo-execution to the existing workspace evo-observation crate. Cargo.lock changes by that single existing-crate dependency entry; no external package, model, paid service or cloud API was added. No commits, releases or real daemon startup occurred.

Final combined test command:

```sh
cargo test --workspace --offline --locked -j1 --no-fail-fast --quiet -- \
  --skip real_log \
  --skip ground_truth_leisure_never_anchors_and_reported_works_stay_coherent
```

Result: **1,230 passed, 0 failed, 3 existing ignored, 3 personal-history tests explicitly filtered**. `cargo check --workspace --all-targets --offline --locked -j1` passed. The native fixture examples and synthetic scoped UI preview built offline; the Swift fixture compiled with the installed local SDK. Focused formatting/whitespace checks passed after correcting the edition-specific formatting. Existing daemon test warnings remain.

Intermediate failures were kept visible and investigated: an old exact-summary assertion needed the new dispatch-warning contract; the execution-handoff fixture lacked a fresh human witness and appended historic timestamps after a much later unrelated work. The occurrence tests now use chronological, isolated synthetic histories instead of accidentally exercising invalid temporal ordering. No production identity refusal was loosened to make them pass.

The next boundary remains an authenticated/local browser adapter with actually witnessed profile/session/tab identities, a way to choose an occurrence when one resource has several, shared execution across the remaining pod/desktop paths, effective native permissions and independent live postconditions, and prospective multi-person task labels. This pass materially implements window identity constraints and visible single-resource selection; it does not claim those remaining adapters or universal restoration are finished.

### Live native fixture follow-up (2026-10-05)

After the user refreshed the responsible application's Accessibility entry and asked to check again, the missing-grant failure disappeared. The inspect-only document probe still returned an AXFocusedApplication communication error (`-25204`); that cold read-path issue is not claimed fixed.

The isolated AppKit fixture was then compiled and launched once, with two same-title windows carrying distinct native identifiers and synthetic document locators. The actual native identity verifier received the fixture-owned process lifetime, `evo.fixture.1`, and its exact first document:

- **Positive restore:** `RaiseAccepted`, `VerificationResult::Verified`, exactly the requested document in `raised`, no opened or unopened targets. Native identity, process lifetime, globally focused window and exact resource postconditions passed.
- **Negative selection:** the same window identity paired with the second document returned `NotDispatched: identified window does not show the requested exact resource`, with no opened/raised targets. No substitute or broader restore was attempted. Its nonzero verifier exit is the expected refusal, not a successful restore.
- **Cleanup:** the owned fixture executable was checked before sending it a termination signal; a subsequent process check confirmed it was no longer running. No real daemon, personal document, browser/profile session, or workspace containment operation was launched.

This is live proof for the **isolated native identity/resource adapter**, not for the complete capture→persistent work inference→daemon→visible-control path. The fixture's logical capture tokens were supplied by the controlled harness, not actual capture. Real browser profile/tab adapters, occurrence picking, shared pod execution, the cold focused-window probe, and cross-application state fidelity remain separate release gates.

## Local doctor and release evidence boundary (2026-10-05)

`python3 scripts/evo-doctor.py` is a deliberately report-only local preflight:
it checks the workspace manifest/lockfile, reports whether `cargo` and `rustc`
are on `PATH`, checks for the five expected local MiniLM assets without fetching
them, and uses only non-prompting macOS Accessibility/Input Monitoring probes
when available. It never reads activity history, changes permissions, launches
apps, creates an event tap, or downloads a model. `--build` is an explicit opt-in
to `cargo build --workspace --offline --locked -j1`; an interrupted or uncertain
build is not retried and a changed lockfile is reported as blocked.

The doctor intentionally prints separate **SEMANTIC**, **LIVE**, **BROWSER**, and
cold-AX gates. A successful compile, nonempty model files, a real-weight offline smoke, a Python trust
diagnostic, a mocked adapter test, or an isolated native fixture cannot stand in
for real local inference, an end-to-end capture→daemon→UI receipt, effective
permission evidence for the responsible launch chain, or a witnessed browser
profile/session/tab adapter. See [`docs/RELEASE-CHECKLIST.md`](../RELEASE-CHECKLIST.md)
for the current evidence requirements and commands.

## Release status: shipped versus blocked gates (2026-10-05)

The word **complete** is not a release status. In particular, a passing unit,
fixture, compilation, or isolated adapter receipt is not evidence that the full
capture-to-restoration product path shipped. Use the labels below instead:

### Shipped in this working tree

- **Offline loader contract:** `evo-voice` reads only an explicitly installed
  local model directory or an existing pinned cache snapshot. Missing, empty,
  corrupt, or ambiguous assets fail locally; there is no download fallback.
- **Workspace safety checks:** Cargo's lockfile and offline build can be
  checked without starting Evo, reading canonical history, launching an app,
  or changing permissions. Cargo fetching is disabled; dependency build scripts
  still need separate network denial for an offline release claim. The doctor is
  `scripts/evo-doctor.py` (report-only by default). The checker is shipped;
  this runner's actual build remains blocked by local ONNX Runtime linking.
- **Isolated native identity adapter:** the 2026-10-05 fixture receipt verified
  one exact synthetic document and refused a mismatched document without a
  substitute. This is a shipped test boundary, not universal restoration.
- **Pinned local model provisioning:** `scripts/model/minilm.py` provisions the
  Apache-2.0 Qdrant MiniLM export at immutable revision
  `d13954661f83248295ba75c1ed411eef3b7b936e`, verifies all five reviewed
  SHA-256 pins, installs atomically, and never runs from the runtime loader.
  The user-authorized installation was verified at the local model directory;
  the real-weight synthetic offline smoke passed with 384-dimensional finite
  vectors. No model binary is committed.
- **Pod receipt boundary:** pod restoration now reports per-target dispatch and
  verification separately and does not infer profile/tab/native identity from
  pod names, URLs, app names or CG window IDs. Containment and Add-to-Pod state
  remain separate from restoration receipts.
- **Local browser adapter:** `evo_execution::browser_macos` now provides an
  explicit macOS adapter for a configured loopback DevTools endpoint. It
  witnesses process launch, persistent profile arguments, browser context/session,
  browser-native window, exact target id, and URL; it retains the target handle,
  focuses that target without navigation, and verifies the globally focused tab
  afterward. Capture can install the same adapter through the daemon bridge, and
  the normal identity restore path dispatches through it when all browser fields
  are present. No endpoint discovery, browser startup, external networking,
  generic AX relabelling, or URL fallback is allowed. The provider remains
  unsupported unless `EVO_BROWSER_DEVTOOLS_ADDRESS`,
  `EVO_BROWSER_DEVTOOLS_PORT`, and `EVO_BROWSER_PID` are explicitly set.

### Blocked release gates

- **Current local receipt (2026-10-05):** script syntax and **12 synthetic
  doctor tests** pass. The pinned model doctor verifies all five files and exact
  hashes; the explicit real-weight offline smoke passes. Running the general
  doctor without `EVO_EMBEDDING_MODEL_DIR` still correctly blocks because the
  default `.fastembed_cache` has no model ref; that does not invalidate the
  explicitly installed flat directory. `cargo check --workspace --all-targets
  --offline --locked -j1` passes. A full `cargo build` remains separately
  environment-dependent on local `ort-sys`/`libonnxruntime` linking.
- **Semantic/live:** real local inference is now verified on synthetic strings,
  but the full capture → persistent work inference → daemon → visible-control
  path still needs a single end-to-end receipt. The current native pass is
  isolated.
- **Cold Accessibility:** after the responsible application's grant was
  refreshed, the cold `AXFocusedApplication` read still returned communication
  error **-25204**. Do not call the permission toggle an effective live-pass
  receipt; reproduce and record a fresh probe before claiming this gate.
- **Browser:** the local adapter and capture/restore wiring are implemented and
  covered by injected tests. A real browser receipt still requires an explicit
  local DevTools configuration and an isolated synthetic browser fixture or
  approved live browser run; generic URL opening remains insufficient.
- **End-to-end fidelity:** occurrence selection, shared pod/desktop execution,
  cross-application state, and independent post-dispatch postconditions remain
  open. No release, commit, or personal-history corpus is implied here.

### Final offline verification receipt (2026-10-05)

The combined workspace command completed successfully:

```sh
cargo test --workspace --offline --locked -j1 --no-fail-fast --quiet -- \
  --skip real_log \
  --skip ground_truth_leisure_never_anchors_and_reported_works_stay_coherent
```

Result: **1,272 passed, 0 failed, 4 ignored, across 59 suite results**. This
includes 135 desktop tests, 145 execution tests, 52 voice tests, browser
identity refusal/success cases, pod receipt cases, scoped UI tests, fictional
inference workflows, doctests, and the existing offline regression suites.
`cargo check --workspace --all-targets --offline --locked -j1` also passed.
Targeted formatting and `git diff --check` passed. Two existing daemon test
warnings remain (`name_index` unused import and `continuation` unused variable);
they were not hidden or treated as release blockers.

### Re-audit against the original Evo objective (2026-10-05)

The original product goal remains: a person names an intention and Evo returns
only the recoverable, evidence-backed state needed to continue it. The browser
adapter closes one important capability gap, but it does not make the whole
product finished. Remaining work, in priority order:

1. **Prove the browser path end to end.** Run an isolated local browser fixture
   through capture, persistence, replay, exact target restore, and independent
   postcondition verification. The current tests use injected provider state;
   they do not claim a real browser receipt. Verify duplicate profiles/contexts,
   closed tabs, process restarts, endpoint loss, and unsupported browsers.
2. **Carry exact identity through every surface.** Voice now carries witnessed
   occurrence surfaces into the identity-aware executor, but pod and embedded
   web-surface restoration still cannot claim the original browser profile,
   authenticated state, history, or unsaved form. They must either accept the
   same browser identity contract or report the capability as unsupported.
3. **Close the live native gate.** Resolve or freshly explain the cold
   `AXFocusedApplication` communication error `-25204`, then prove the actual
   capture/execution launch chain and the capture → daemon → visible-control
   flow with synthetic fixtures only unless a separate live authorization is
   granted.
4. **Finish inference authority and request semantics.** Retire remaining
   duplicate/legacy reasoning only after parity, and add explicit tests for
   temporal filtering, unmatched scopes, occurrence selection, and the rule
   that model failure never widens a restore request.
5. **Evaluate usefulness as well as safety.** Build held-out, locally labelled
   multilingual and cross-profession fixtures for interleaved work, shared
   documents, reading-only work, long gaps, arbitrary names, and machine churn.
   Measure wrong-work restores, missed resources, clarification rate, and
   time-to-resume separately from unsupported-state rate.
6. **Finish release engineering.** Repeat checks with network denial, resolve
   local ONNX Runtime packaging/linking, verify app/helper signing and launch
   identity, benchmark cold start/history growth/battery/sleep-wake, and verify
   deletion, exclusions, retention, and permission revocation across every
   index and cache. Do not ship personal-history tests as release evidence.

### Synthetic follow-up and fresh blockers (2026-10-06)

The browser fixture in `crates/evo-daemon/tests/browser_identity_fixture/mod.rs`
now carries directly supplied synthetic browser tokens through the capture
registry, accepted observation, durable storage, daemon work/RESOURCE planner,
exact browser executor and independently checked postcondition. Eleven tests
cover the positive path and fail-closed stale/duplicate/unreadable alternatives.
Its retained handles and bridge are injected: this is **not** a real browser,
actual AX capture, concrete DevTools connection, desktop UI or live receipt.

Pods explicitly refuse original-browser intent and partial identity without an
embedded URL fallback. A later unidentified focus cannot inherit an earlier
occurrence's native identity. Deleting or truncating a synthetic observation
log invalidates derived daemon state; inaccessible storage returns an error,
not invented empty history. Exclusion/retention/revocation plumbing, larger
same-size log rewrites, explicit occurrence picking and dated natural-language
scope remain open and must be tested separately.

An isolated synthetic AppKit fixture was launched for a fresh cold AX probe.
The read-only `scripts/probe-cold-ax-fixture.swift` refused before any AX read:
the probe's responsible process lacked an effective Accessibility grant.
A dedicated stable-signed helper was subsequently enabled by the user and
relaunched through LaunchServices, but `AXIsProcessTrusted()` still returned
false. Both owned fixture processes were terminated after refusal. Evo did not
change or reset permissions, and the previous `AXFocusedApplication -25204`
was not claimed solved at that point. With a persistent signed bundle under
`dist/ax-fixture/EvoAXProbe.app` and a new manual grant, a later isolated
fixture reproduced `AXFocusedApplication -25204` while the same fixture's
app-scoped `AXFocusedWindow` returned `0` with a matching window PID.
`evo-execution/src/macos.rs` now
falls back only on that error, using the frontmost PID and the process-scoped
focused window, checking its owner and checking frontmost PID again. The
PID-race unit test and offline compile passed, but a fresh production Rust
live receipt is still required before marking this gate complete. The helper
read no other app's attributes. `scripts/verify-live-capture.sh` is unsuitable for this
synthetic-only gate because it launches genuine applications, inspects real
work and broadly removes a configurable root/terminates daemon processes;
it was not run.

The first `cargo build --workspace --offline --locked -j1` reproduced the
`ort-sys` undefined-symbol link failure without `ORT_LIB_PATH`. An existing
local arm64 ONNX Runtime 1.29 dylib was linked from an external temporary
directory with `ORT_LIB_PATH`, `ORT_PREFER_DYNAMIC_LINK=1` and a deny-network
sandbox: the full debug workspace build passed. The real-weight MiniLM smoke
then passed in the same network-denied sandbox when `DYLD_LIBRARY_PATH` was
set **inside** its child shell (the protected `sandbox-exec` executable strips
inherited `DYLD_*`). No extra download or private binary was committed. The
new binaries still reference `@rpath/libonnxruntime.1.dylib`, which has not
been bundled. The packaging script now accepts `EVO_APP_OUTPUT_DIR` and stages
conventional `libonnxruntime.dylib`/`libonnxruntime.1.dylib` aliases under
`target/ort-link` without mutating the external runtime directory. An optimized
bundle written outside the Desktop/FileProvider-managed workspace,
`/private/var/tmp/evo-dist-wyAPBl/Evo.app`, passed deep strict codesign
verification. This is a verified development bundle with an external runtime,
not a portable redistribution package. The workspace `dist/Evo.app` remains
an unsafe receipt location because Finder metadata returns after signing.

With the staged local dylib in its search path, the offline workspace test
command passed **1,302 tests, 0 failures, 4 ignored across 60 results**; the
known history-reading cases remained filtered. Running that whole suite in the
blanket deny-network sandbox failed seven daemon tests because local Unix
socket `bind` returned `Operation not permitted`; this was not silently counted
as a pass. A synthetic 2,000-observation benchmark in the deny-network sandbox
measured 812.7 ms/full reparse, 713.2 ms/first index build and 78.8 µs/idle
refresh on this Apple Silicon machine. It is one diagnostic sample, not a
battery, cold-start, sleep/wake, field-accuracy or release performance result.

The local held-out synthetic evaluator reports grouping usefulness **8/8**,
grouping safety **9/9**, restoration usefulness **5/5**, restoration safety
**15/15**, and deterministic replay **3/3**. These authored cases cover
overlapping/shared resources, multilingual labels, reading-only recurrence,
unrelated attention, temporal gaps and app/machine renaming. They are contract
regressions, not proof that Evo understands arbitrary real work or generalizes
across people and professions.

### Exact local commands

From the repository root, these commands are safe defaults:

```sh
# Syntax + synthetic fixtures only; no Cargo, apps, permissions, history, or network.
python3 scripts/evo-doctor.py --syntax-check
python3 scripts/evo-doctor.py --self-test

# Report-only local preflight (default; model presence and nonprompting macOS diagnostics).
python3 scripts/evo-doctor.py --report-only

# Explicit workspace build; may write normal Cargo target artifacts, but stays offline and locked.
python3 scripts/evo-doctor.py --build
```

`--build` sets `CARGO_NET_OFFLINE=true` and runs
`cargo build --workspace --offline --locked -j1`; it does not download model
assets or launch an application. The doctor never repairs Cargo.lock, grants
Accessibility/Input Monitoring, reads personal history, or retries uncertain
live actions. Permission output is diagnostic only and is not itself a release
gate receipt. Model assets are checked for local presence only, not inferred to
be valid weights merely because the files exist.

Default model resolution mirrors the loader: `EVO_EMBEDDING_MODEL_DIR`, then
`HF_HOME`, then `FASTEMBED_CACHE_DIR`, then `.fastembed_cache`. An explicit empty
or invalid location never falls back. Run from the repository root to align
relative paths; `--workspace` explicitly sets the doctor's path base.
`--skip-permissions` omits even the no-prompt probes. Exit **0** means requested
local checks passed, not release approval; exit **1** means a local prerequisite
or requested build is blocked. The recorded browser/cold-AX labels require their
own new receipts and are not newly measured by the doctor.

For the opt-in local Chromium-family adapter, configure the exact browser
process and an already-running loopback DevTools endpoint before starting Evo:

```sh
export EVO_BROWSER_DEVTOOLS_ADDRESS=127.0.0.1
export EVO_BROWSER_DEVTOOLS_PORT=9222
export EVO_BROWSER_PID=<the witnessed browser PID>
```

The adapter never starts the browser or discovers the port/PID. It supports
browser contexts that expose `browserContextId`; browsers or tabs that do not
expose complete profile/session/window/tab evidence refuse rather than falling
back to a URL. The profile token is the directly observed
`--user-data-dir::--profile-directory` pair, so a browser launched without both
arguments is deliberately unsupported.

## Generalization follow-up (2026-10-06)

The architecture remains evidence-first and app/profession agnostic; details,
sources and acceptance boundaries are in
[ARBITRARY-WORK-GENERALIZATION.md](../research/ARBITRARY-WORK-GENERALIZATION.md).
An opt-in strictly local multilingual E5-small profile now uses role-aware
`passage:`/`query:` embeddings. Its scores are **not** calibrated against the
MiniLM thresholds: semantic voice matches only suggest a work for explicit
`CHOOSE`, and fuzzy scoped restores refuse. Exact eligible locators still work.
The user subsequently authorized one explicit public E5 download. Pinned local
assets and a real-weight four-query synthetic cross-language smoke passed;
that does not calibrate automatic restoration.

An application/profession/multilingual renaming fixture passed **38/38**
normalized inference invariants; this is not field accuracy. A same-length
Observation-log replacement now resets derived views by file identity; a
selected resource/locator mismatch refuses before dispatch. Sixteen doctor
synthetic tests pass. The current offline workspace run passed **1,313 tests,
0 failed, 5 ignored across 61 results** with the known personal-history tests
filtered. A fresh signed development bundle passed deep strict verification at
`/private/var/tmp/evo-generalization-dist-qspHKb/Evo.app`. An opt-in bundled
runtime build at `/private/var/tmp/evo-bundled-dist-ipijKy/Evo.app` also passed
strict signature, local dependency closure and isolated C API 24 loading. A
separate Rust test executable proved real-weight E5 inference using only that
bundled runtime, with external runtime paths unset. The Evo app itself has not
been launched. Cross-person, cross-
language real-weight evaluation, live native/browser receipts, privacy policy
coverage, and portable packaging remain release gates.
