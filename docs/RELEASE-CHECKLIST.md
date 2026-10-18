# Evo local release checklist

This is an evidence checklist, not an approval to launch, install, sign, change
permissions, download models, or read a person's activity history. Keep every
receipt tied to the exact working tree, model assets, platform and launch chain.
The implementation narrative and earlier receipts are in
[LOCAL-WORK-RESTORATION.md](architecture/LOCAL-WORK-RESTORATION.md).

**Updated 2026-10-06:** doctor syntax and **21 synthetic doctor tests pass**;
**14 synthetic packaging/signing contract tests pass** with fake Mach-O/tools,
not real runtime loading or signing. The
explicitly provisioned MiniLM directory passes all five pinned SHA-256 checks,
and the real-weight offline synthetic smoke passes. `cargo check --workspace
--all-targets --offline --locked -j1` passes; a full `cargo build` remains
environment-dependent on local `ort-sys`/`libonnxruntime` linking. The isolated
native identity/resource adapter passed its exact-document positive and
mismatched-document refusal probes. Full capture→daemon→UI, cold AX **-25204**,
and a real browser profile/tab receipt remain unpassed gates. A new isolated
browser capture→persistence→daemon-target→injected-executor test has passed;
it is not a concrete DevTools or desktop receipt.
“Complete” alone never means shipped or release-ready; checkboxes below require
current receipts.

## 1. Report-only local preflight

```sh
python3 scripts/evo-doctor.py --syntax-check
python3 scripts/evo-doctor.py --self-test
python3 scripts/evo-doctor.py --report-only
```

- [x] Script syntax and synthetic fixtures pass. Fixtures do not invoke Cargo,
  load macOS frameworks, download assets, or inspect history.
- [x] Workspace manifest/lockfile and `cargo`/`rustc` on PATH are present.
  Presence does **not** prove lock freshness, compiler availability or a build.
- [x] Five nonempty readable local model files exist in the explicit pinned
  install: `model.onnx`,
  `tokenizer.json`, `config.json`, `special_tokens_map.json`,
  `tokenizer_config.json`. No download or inference is attempted by the doctor.
- [ ] Accessibility and Input Monitoring diagnoses are recorded, if available.
  The no-prompt results describe **this Python/responsible launch chain**, not
  Evo's installed application or its helpers. An enabled settings toggle is not
  an effective-grant receipt. `--skip-permissions` omits these probes entirely.

Exit 0 means requested **local** checks passed; exit 1 means a local prerequisite
or requested build is blocked. Neither exit code means release-ready. Permission
diagnostics and the named release gates are not folded into that local exit code.
Do not use `crates/evo-daemon/examples/evo_doctor.rs` as a substitute: it examines
canonical activity history and requires a separate, explicit opt-in.

## 2. OFFLINE CARGO/BUILD gate — explicit, not report mode

Only with authorization to compile:

```sh
python3 scripts/evo-doctor.py --build
cargo check --workspace --all-targets --offline --locked -j1
cargo test --workspace --offline --locked -j1 --no-fail-fast --quiet -- \
  --skip real_log \
  --skip ground_truth_leisure_never_anchors_and_reported_works_stay_coherent
```

- [x] Record actual command, exit, warnings and test counts, including every
  skipped/ignored test. Audit new tests for personal-history reads before running
  the suite; these filters reflect the currently known history tests, not a
  permanent privacy sandbox.
- [x] `Cargo.lock` stays unchanged; missing cached dependencies fail rather than
  fetching. The doctor's build uses `--offline --locked` and
  `CARGO_NET_OFFLINE=true`, and never retries an interrupted/uncertain build.
- [ ] Repeat in a network-denied environment for an offline claim. Cargo's flags
  prevent Cargo fetching but do **not** sandbox arbitrary dependency build
  scripts or test code.
- [x] External development packaging/signatures were verified with explicitly
  approved local steps. The verified bundle used `Evo Code Signing` and an
  external ONNX runtime; the doctor still does not install LaunchAgents, launch
  a bundle, or claim portable redistribution.
- [x] A fresh optimized bundle built with the explicit external runtime mode
  and passed deep strict codesign verification when written outside the
  Desktop/FileProvider-managed workspace. The normal `dist/` location on this
  host is not a stable receipt location because Finder metadata returns after
  signing; use `EVO_APP_OUTPUT_DIR` for packaging evidence. The current
  generalization tree's bundle was built at
  `/private/var/tmp/evo-generalization-dist-qspHKb/Evo.app` and passed
   `codesign --verify --deep --strict` without launching it.

### Opt-in local bundled ONNX Runtime packaging — local artifact verified

The external development path above is unchanged. A narrow **arm64 only** local
packaging path now accepts ONNX Runtime **1.29.0** with exactly
`@rpath/libonnxruntime.1.dylib`, compatibility version `1.0.0`, the reviewed
eleven Apple/system dependencies, and the `OrtGetApiBase` export. The current
`ort`/`ort-sys` **2.0.0-rc.13** lock entries are pinned for this contract; a binding
upgrade requires a new review rather than silently accepting an impossible ABI.
This wheel's Mach-O requires **macOS 14.0**. Bundled mode sets the app minimum to
14.0 and refuses x86/universal or newer-minimum executables; external mode still
uses the existing 13.0 bundle minimum.

Only after separately reviewing and supplying exact local notice files:

```sh
export EVO_ORT_PROVISIONING=bundled
export EVO_ORT_DYLIB="$HOME/.cache/uv/archive-v0/sHE2fDE6TGO5MrGO/onnxruntime/capi/libonnxruntime.1.29.0.dylib"
export EVO_ORT_LICENSE=/exact/local/onnxruntime-v1.29.0/LICENSE
export EVO_ORT_THIRD_PARTY_NOTICES=/exact/local/onnxruntime-v1.29.0/ThirdPartyNotices.txt

# Read-only: metadata, exact closure, binding pin and SHA-256s. No dlopen.
python3 scripts/evo-doctor.py --check-ort-inputs

# Only with separate authorization to compile/package/sign a NEW local bundle.
# This refuses an existing Evo.app at the selected output, including dist/.
EVO_APP_OUTPUT_DIR=/exact/fresh/local/output bash scripts/build-app.sh

# Read-only artifact checks, independent of the original wheel/cache paths.
python3 scripts/evo-doctor.py --check-ort-bundle /exact/fresh/local/output/Evo.app
EVO_APP_PATH=/exact/fresh/local/output/Evo.app \
  python3 scripts/evo-doctor.py --report-only --skip-permissions --verify-bundle
```

Bundled mode snapshots exactly the three supplied files under
`target/ort-bundled-link.*` (or `CARGO_TARGET_DIR`), forces dynamic/offline linking,
and byte-compares copies before signing. The app carries
`Contents/Frameworks/libonnxruntime.1.dylib`,
`Contents/Resources/onnxruntime/LICENSE`, and
`Contents/Resources/onnxruntime/ThirdPartyNotices.txt`. It removes development
rpaths from the **copied** executables and adds only
`@executable_path/../Frameworks` before signing. All non-system dependencies must
close inside this one-runtime contract; additional third-party/vendor dylibs refuse.
The runtime and executables are explicitly signed before the entire app is
sealed and verified with `codesign --verify --deep --strict`. Default
`sign-dev.sh` still signs only target binaries, never `dist`; its new
`--app /exact/Evo.app` path is explicit, bundled-only, checks the supplied notice
bytes, and signs/verifies nested code and the app. No signing script is invoked
by doctor checks.

- [x] `bash -n scripts/build-app.sh scripts/sign-dev.sh` passes.
- [x] `python3 scripts/evo-doctor.py --syntax-check` and `--self-test` pass
  (**21 tests**). Report-only bundled presence checks run no inspection tools.
- [x] `PYTHONDONTWRITEBYTECODE=1 python3 scripts/tests/test_ort_packaging.py`
  passes (**14 tests**, synthetic temporary apps and fake tools only). It checks
  missing/empty/symlinked notices, exact source selection, bad architecture/ID/
  versions/exports/dependencies, stale binding pins, development-rpath removal,
  nested signing order, strict-verification failure, existing-output refusal,
  explicit re-sign behavior, and unchanged external development packaging.
  Both synthetic suites were also repeated successfully with macOS
  `sandbox-exec -p '(version 1) (allow default) (deny network*)'`; this denies
  network for fixture/tool checks, not a real bundled build or inference receipt.
- [x] Read-only `lipo`, `otool` and `nm` metadata validation on the reviewed local
  wheel passes the pinned runtime contract.
- [x] Supply the official **v1.29.0** `LICENSE` and
  `ThirdPartyNotices.txt` locally. Upstream source files exist at
  `https://github.com/microsoft/onnxruntime/blob/v1.29.0/LICENSE` and
  `https://github.com/microsoft/onnxruntime/blob/v1.29.0/ThirdPartyNotices.txt`;
  the files were downloaded at immutable commit
  `2e2543fbe9fae542f921d47a72d21d5a4ef0b710` and matched upstream Git
  blob IDs. The wheel METADATA declares MIT. This preserves supplied notices,
  **not legal redistribution approval**.
- [x] A fresh local bundle at
  `/private/var/tmp/evo-bundled-dist-ipijKy/Evo.app` passed exact input preflight,
  static closure, nested signing and `codesign --verify --deep --strict`.
  Without `DYLD_LIBRARY_PATH` or `ORT_LIB_PATH`, its bundled dylib loaded in
  an isolated Python process and negotiated ONNX Runtime **1.29.0 / C API 24**.
  A separate isolated Rust test executable, patched to use only its sibling
  Frameworks dylib, passed real-weight E5 synthetic multilingual inference
  with external runtime paths unset. The Evo app itself was **not launched**.
- [ ] Repeat packaging/inference with network denial and attach exact commands,
  input digests, artifact path, signing identity and limitations. Cargo offline
  flags are not a dependency-build-script network sandbox. Local stable signing
  is not Developer ID distribution, notarization, or a release-ready decision.

## 3. SEMANTIC gate — real weights, synthetic inputs, offline

- [x] Provision the pinned, correctly licensed model separately; no automatic
  download or remote fallback. Asset resolution matches the loader:
  `EVO_EMBEDDING_MODEL_DIR`, otherwise `HF_HOME`, otherwise
  `FASTEMBED_CACHE_DIR`, otherwise `.fastembed_cache` relative to runtime cwd.
  A configured empty/broken location must fail, not try a lower-priority source.
- [x] Run successful local inference with the actual non-quantized 384-dimension
  MiniLM export and **synthetic** strings, with network denied. Capture model
  identity/digests, dimension, finite/nonzero vectors, and exact failure behavior
  for corrupt/missing assets; never log personal history or credentials.
- [ ] Invalid output/model failures stay errors, not `NoMatch`, widening scope
  or authorizing a restore. File presence, mocked embeddings and Cargo success
  do not pass this gate or establish multi-person task-inference accuracy.
- [x] Optional multilingual E5 profile is local-only and uses `passage:` for
  documents and `query:` for requests. The doctor validates its exact asset
  layout. Until held-out E5 calibration exists, a semantic voice match requires
  explicit `CHOOSE`, fuzzy `SCOPE` refuses, and exact resource selection remains
  available. The MIT-licensed immutable E5 export was provisioned outside Git;
  all five pinned SHA-256 checks passed. Real-weight local retrieval preferred
  a relevant bicycle-repair passage over an unrelated recipe for four synthetic
  queries (English, Arabic, Japanese, Hindi). This is not cross-person
  calibration or proof of automatic restoration accuracy.

## 4. LIVE gate — separate native and end-to-end receipts

- [ ] Obtain user authorization for each live fixture/app/restore step. The
  doctor must never request permissions, inspect AX windows, create an event tap,
  read TCC databases, reset grants or launch apps.
- [ ] Verify effective grants for the **actual** capture/execution launch chain;
  denied/unavailable capture must not masquerade as an empty desktop.
- [ ] Isolated native fixture: exact process lifetime + unique AXIdentifier +
  exact resource yields an independently verified focused-window postcondition;
  wrong resource/duplicate identity/stale lifetime refuses without fallback.
- [ ] Resolve the recorded cold `AXFocusedApplication -25204` read failure with
  a fresh controlled receipt. Do not extrapolate from Python trust diagnostics.
  The previously signed synthetic probe was run with an impossible fixture PID
  as a no-AX-read preflight and returned `AXIsProcessTrusted() == false` (exit
  3). No fixture app or unrelated window was inspected. An effective grant for
  the responsible test launch chain remains required; this task did not alter
  system permissions.
- [ ] End to end: controlled synthetic capture → persisted observations → one
  work snapshot → visible single-resource control → daemon → exact native target
  → dispatch and verification receipts. Supplied harness identities are not
  actual capture evidence; an isolated adapter pass is not this gate.
- [ ] Pending, timeout/unknown, partial, failed and unverified results stay
  distinct; no automatic retry or opening extra resources. Other pod/desktop
  restore paths must share identity validation and pre-dispatch provenance.

## 5. BROWSER gate — adapter implemented; live receipt pending

- [ ] Prove on a controlled real browser that the explicitly configured adapter witnesses and preserves
  profile/session/window/tab identity from capture through execution.
- [x] Controlled injected tabs verify duplicate URLs/titles in different
  profiles, closed tabs, stale sessions and denied capabilities. Requested
  profile/tab constraints never degrade to a generic URL open.
- [ ] Verify original session/state postconditions only within capabilities the
  adapter can prove. Embedded-browser or OS-default URL dispatch is not original
  authenticated-session, navigation-history, unsaved-form or scroll restoration.

The implemented adapter remains opt-in and refuses unsupported constraints.
The gate is not release-ready until a current isolated local-browser receipt
proves capture → persistence → exact restore → postcondition verification.
The gstack `/browse` installation was used only for public model/license
research. Its controlled browser does not expose an Evo-verified
profile/session/window/tab identity through the adapter's required explicit
DevTools endpoint, so browsing a synthetic page there would not be a valid
original-browser restore receipt. No private browser profile was inspected.

## 7. Current synthetic-only follow-up (2026-10-06)

- [x] Captured browser identity survives canonical acceptance and durable read-back,
  reaches a daemon RESOURCE target, and passes an injected exact dispatch and
  independent postcondition; eleven tests cover wrong/stale/unreadable identity.
  This does **not** exercise the real browser, AX capture, SystemPlatform, or UI.
- [x] Pod entry explicitly refuses original-browser intent and partial browser
  tokens without opening the URL or changing containment; ordinary URL hosting
  remains separate.
- [x] A later unidentified focus no longer inherits a stale earlier surface
  identity during voice restoration. Synthetic deletion/truncation invalidates
  derived daemon views, and missing/inaccessible storage is not treated as
  successful empty history.
- [ ] After the persistent signed helper was granted Accessibility, a controlled
  synthetic fixture reproduced system-wide `AXFocusedApplication -25204` while
  the exact fixture's app-scoped `AXFocusedWindow` read returned success (`0`)
  and its window PID matched the fixture.
  `evo-execution` now falls back only on `-25204` to the frontmost process's
  app-scoped window, refusing a PID/focus change. Its unit test and compile
  pass; a **fresh production Rust live postcondition receipt is still needed**.
- [x] An offline, network-denied local build linked successfully when supplied
  an existing arm64 ONNX Runtime 1.29 dylib via `ORT_LIB_PATH` and
  `ORT_PREFER_DYNAMIC_LINK=1`; real-weight synthetic smoke passed after setting
  its runtime library path inside the sandbox. This is not a portable signed
  release bundle. Without the local library, the build still fails at link.
- [x] The workspace suite, with three named personal-history tests excluded,
  passed offline with the staged local ONNX dylib in this tree: **1,313 passed,
  0 failed, 5 ignored in 61 suite results**. The new E5 real-weight smoke is
  ignored by default and was run separately. A blanket deny-network sandbox fails seven
  daemon tests because it also denies their local Unix-domain socket binds;
  this is not evidence of an external-network dependency. The isolated model
  smoke and build succeeded under that deny-network sandbox.
- [x] A synthetic 2,000-observation reload benchmark in the deny-network
  sandbox measured 812.7 ms/full reparse, 713.2 ms/initial incremental build,
  and 78.8 µs/idle refresh on this machine. It does not measure cold start,
  battery, sleep/wake, or real-life usefulness.
- [x] Local held-out synthetic evaluator: grouping usefulness **8/8**, grouping
  safety **9/9**, restoration usefulness **5/5**, restoration safety **15/15**,
  deterministic replay **3/3**. These are authored contracts, not human
  accuracy or semantic-understanding evidence.
- [x] Metamorphic evaluation across opaque, application-like, and multilingual/
  profession labels passes **38/38** normalized grouping/standing/role
  invariants. Same-length Observation-log replacement invalidates every
  derived view in a synthetic privacy test; an inconsistent selected resource
  and locator refuses before OS dispatch. These are local regressions, not a
  substitute for exclusions/retention/revocation or cross-person accuracy.
- [ ] The workspace `dist/Evo.app` still fails deep strict verification when
  Finder/FileProvider reattaches `com.apple.FinderInfo`; this is avoided by the
  verified external output path, not silently treated as solved in `dist/`.

## 6. Product and privacy acceptance

- [ ] Prospective opt-in, locally labeled, held-out multi-person workflows meet
  declared safety **and usefulness** criteria. Synthetic regressions alone do
  not establish cross-profession/language accuracy.
- [ ] Exclusions, pause, deletion, retention and permission revocation propagate
  to all indexes/caches; network-denied inference and egress audit pass.
- [ ] Measure wrong-work/resource actions, missed required resources,
  unsupported-state and clarification rates, time-to-resume, cold start,
  growing-history CPU/memory/battery, sleep/wake and capture outages separately.
- [ ] Attach current receipts and limitations to the release; a prior working
  tree's test totals or live fixture success are not a new release receipt.

**Release decision:** no broad release until all applicable gates have current
evidence. The doctor deliberately reports semantic/live/browser gates separately
and never grants release approval.
