# Evo Build Log

Durable working memory of substantive build rounds: what changed, how it was
verified, and what is knowingly open. Newest entry first. Claims here carry
the command and its observed output, never bare assertions.

---

## 2026-09-23 — IS-0023 Pods (P2), Rooms (IS-0022) fully replaced

**Branch:** `pods` (local), base `main @ 95fc3ad`.

### What changed

- **New crate `evo-pods`** — the inference-driven pod engine: `Pod`, `PodRuntime`,
  `PodBoard`, `PodLease`, policy (`ContainMode`), gestures→declarations,
  suggestions, stage recipes, dim/park containment semantics, persistence
  (`Config` v2: `ContainConfig` + per-app `InstancePolicy` list, serde_text
  backed — no app names in engine code, policies are data).
- **`evo-desktop/src/pod_surface.rs`** — the live macOS `PodSurface` (912
  lines): AX window verbs, screens, veil windows (borderless overlay
  `NSWindow`s at status level), move/quit/run legs; honest "pods require
  macOS" stub off-platform.
- **`evo-desktop/src/pods.rs`** — `PodHost` / `SharedPods`: menu-bar +
  hotkey command surface, per-pod hibernated web panes (logins isolated
  per work), restore on continue.
- **`evo-desktop/src/podbar.rs`** — the ⌥Space Pod Bar overlay: cards with
  identity rail, engine reason line, position-true mini-stage, witnessed
  badges only (unsaved / uncommitted / download ready / unfinished /
  contested).
- **Presence rewired** — `PodPresence` (menu-bar pod menu + hotkeys:
  ⌘⇧E cycle, ⌥1–⌥9 direct dial, ⌥Space bar toggle). `take_bar_toggle`.
- **Rooms deleted, not patched** — `evo-desktop/src/rooms.rs`,
  `evo-execution/src/room.rs`, `room_macos.rs`, `room_probe` /
  `room_roundtrip` / `room_trace` bins removed; manifest entries dropped.
- **Pre-existing Linux hygiene** (main was already red on Linux; fixed in
  passing so the suite can run at all): macOS-only diagnostic examples +
  `evo-input-counter` gated per-item with honest off-macOS stubs;
  `pid_from_storage_root` liveness (`kill -0`) + env-then-pid-file contract
  ported to the non-macOS stub of `macos_event_source`;
  macOS-only input-counter lifecycle test cfg-gated; `evo-desktop/build.rs`
  objc shim macOS-gated.

### Verification (Linux sandbox)

- `cargo test --workspace` → **exit 0; 49 suites; 935 passed, 0 failed**
  (includes `evo-pods` unit + `tests/{pods_core,runtime,fake}`,
  `continue_door`, all ledger/threads integration suites).
- Baseline `main @ 95fc3ad` fails the same command on Linux (E0432 in
  `evo-daemon` + 3 pre-existing `evo-capture` failures) — this branch is
  strictly greener than what it branched from.
- Residual warnings are pre-existing dead code on `main` (engagement /
  workspace hashing helpers, capture unused imports); none introduced by
  this round.

### Open / knowingly deferred

- macOS-only verification (`pod_surface`, veil stacking level, hotkey
  registration) needs the Mac build (`./scripts/build-app.sh`) — see the
  verify checklist in the round notes.
- Pod identity on the Home path uses `pod_id_of` (FNV-1a of the engine
  name) until the engine-lineage ids land with the read-path unification
  (assessment W2).
- `PodHost::enter_at` currently takes `(index, now_ms)`; callers pass
  `pods::epoch_ms()` — fine, but consider making the epoch internal next
  round.

---

## 2026-09-22 — IS-0023 Pods scaffold (engine crate skeleton; superseded by the 09-23 entry)
