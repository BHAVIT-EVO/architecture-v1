# Evo Build Log

Durable working memory of substantive build rounds: what changed, how it was
verified, and what is knowingly open. Newest entry first. Claims here carry
the command and its observed output, never bare assertions.

---

## 2026-09-23 (round 2) — Pods hardware-verification fixes + arrangement planner repair

**Branch:** `pods` (continuing), after Bhavit's first Mac run of the P2 build.

### What the machine showed (4 bugs + 1 design correction)

- **B1** `open -a "<window title>"` restore failures: the upstream identity
  fallback chain (`document_path > url_host > window_title > app_name`)
  records window titles in the app slot when Accessibility is off.
  Presentation-layer fix: any identity that also appears among the work's
  window titles is a window, never an app — `apps` and `resource_apps`
  are cleaned at pod construction AND re-checked in `restore_work`
  (skip-count reported honestly, failures capped at 2 shown + N more).
- **B2** Home didn't scroll: the work section was never wrapped in
  `ui::scroll`; it is now.
- **B3** Pod bar cards were inert: two sibling `egui::Area`s at equal
  order — backdrop could swallow clicks. One Area now (backdrop painted
  and hit-tested FIRST, widgets later; newest wins).
- **B4** Lag: continuous `ctx.request_repaint()` loop while the bar is
  visible (60fps churn). Removed; egui repaints on input natively.
- **D1** In-Evo browser auto-opening on entry ("feels like rooms"):
  `enter_at` no longer sets `wants_browser`; the Pages pane is on demand
  (Pages button on the Home strip; hibernation on leave/switch unchanged).
- **Active-pod strip legibility:** name truncated to 48 chars, `·`
  separators, "Browser" relabeled "Pages".

### The engineering catch

- **`dim::plan_activation` materialized recipe ratios against each
  window's own rect instead of the screen** — entering could never stage
  windows correctly on the actual display. Signature now takes
  `screen: Option<&Frame>` (main screen, surface contract is main-first);
  regression test `recipe_frames_materialize_on_the_screen_not_the_window_rect`
  pins the geometry (28/28 evo-pods green).
- Landscape memory committed at `docs/design/POD-WORKSPACE-LANDSCAPE.md`:
  what rcmd/Bunch/ContextSwitcher/FlashSpace/MacLayoutManager/WindowLayout/
  SpacePigeon/Ikuna et al. actually do, what we take (exact-title-first
  matching, skip fullscreen/minimized, launch-without-fronting), what we
  don't (manual snapshots, Spaces manipulation).
- Design position reaffirmed with evidence: permission velocity is the
  core UX dependency — arrangement machinery (stage recipe moves,
  containment) was already present and its zeros without Accessibility
  were honest; after granting Accessibility + "Check again", entering a
  pod arranges member windows by the pod's stage recipe.

### Verification

- `cargo test --workspace` → **exit 0; 49 suites; 936 passed, 0 failed**
  (preceding round: 935; +1 regression test for the screen materialization).

### Open, knowingly

- `Role::Rail` seasoning and multi-display staging remain unexercised on
  hardware; Stage Manager awareness deferred; the Pages pane's own tabs
  show raw URLs until page titles load (cosmetic).
- Pod names on Home/bar can still be long page titles (truncated at the
  strip; engine naming pass is part of W2 read-path work).

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
