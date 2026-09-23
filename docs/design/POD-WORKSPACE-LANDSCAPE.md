# Pod Workspace Landscape — what exists, what it does, what we take

Compiled 2026-09-23 for the Pods round-2 build. Purpose: a durable memory of
the macOS workspace-switching field, each tool's actual mechanism, and the
specific decisions our Pods round makes because of it. Nothing here is
aspirational copy; mechanisms only.

## The field, by mechanism

| Tool | Mechanism | What it actually does |
|---|---|---|
| **rcmd / ShiftPlus** | hotkey → `NSRunningApplication.activate` | instant per-app switching; no windows knowledge |
| **Bunch** | plaintext "bunches" | launches/quits a fixed app+URL set per preset; no windows, no restore semantics |
| **ContextSwitcher** (minsang-alt) | manual save → named window contexts; title-match show/hide; floating HUD | per-WINDOW chrome (one IntelliJ project window, not every IntelliJ window); restore by exact title then front-to-back order |
| **FlashSpace** (wojciech-kulik) | virtual-desktop replacement | app-to-space assignment + fast switching; consciously replaces macOS Spaces |
| **MacLayoutManager** (choandrew) | save/restore named layouts per display-set | frames as display-relative fractions; exact-title match FIRST, then front-to-back; skips fullscreen/minimized; launches missing apps WITHOUT bringing them forward; per-app AX worker concurrency so one slow app doesn't block others; auto-restore 2s after display reconfiguration; 90KB ObjC resident |
| **WindowLayout** (sylpht) | display-reconnect auto-restore | skips fullscreen/minimized; Stage Manager aware; notes that ad-hoc signing revokes Accessibility on every upgrade (permission churn is real) |
| **SpacePigeon** (louivers) | Hammerspoon presets | apps + windows + macOS Spaces + URLs per preset; one hotkey rebuilds |
| **Ikuna / BetterStage / Spencer** | commercial "context managers" | app sets + browser tabs + Focus mode per context; ~3s switch; manual save |
| **Workspace Pro / Parallel Spaces (MAS)** | commercial | per-window contexts, $; manual |
| **Choosy / Firefox containers / Wavebox** | web identity layer | same site, different login per context — browser-level, no desktop knowledge |
| **agent-workspace dev tools** (cmux, Superset, Orca, Paseo, agent-workspace.ai, Vellum's 2026 list) | git-worktree + terminal grids | developer-only agent orchestration; zero knowledge-worker window/context management |

## What nobody does

- **Derive the workspaces.** Every tool above is *manual snapshot*: you name
  a layout, you save it, it rots the moment work drifts. No one infers the
  work from evidence and keeps the grouping honest across days.
- **Explain why a window belongs.** Title-matching restorers can't say
  *why*; a pod can cite the events.
- **Reverse cleanly.** Layout restorers move windows and leave them there
  (except ContextSwitcher's show-all); none play a lease backwards.
- **Web identity + desktop arrangement in one model.** Choosy/containers do
  only web; layout managers do only windows.

## What we take (concretely)

1. **Match windows by exact title first, stable order second**
   (MacLayoutManager). Our resource-matching for stage roles follows a
   deterministic subtitle-in-title rule — never heuristics that can merge.
2. **Skip fullscreen and minimized windows when arranging**
   (WindowLayout, MacLayoutManager). Moving those produces visual mess.
3. **Launch missing apps without forcing them frontmost** — the stage
   raise-order decides who ends on top, not `open`'s habit.
4. **Stage Manager awareness is P-next**, not now; documented gap.
5. **Ad-hoc signing revokes Accessibility on every rebuild** (WindowLayout's
   warning): our permission UX must expect re-grant after dev builds and ask
   the user with "Check again", which Home already offers.

## Round-2 direction (from Bhavit's hardware verification, 2026-09-23)

The user's complaint — "apps opened by Evo landed randomly on the desktop,
untidy; web pages opened inside Evo felt like the old rooms; the pod bar's
cards didn't click; the home page didn't scroll; everything lagged" —
decomposed to four bugs + one design correction:

| # | Symptom | Root cause | Fix |
|---|---|---|---|
| B1 | `open -a "<window title>"` failures | identity fallback chain (`document_path > url_host > window_title > app_name`) fills `task_key.app_name` with titles when Accessibility is off; `Work.apps` then contains titles and restore tries to launch them | clean `apps`/`resource_apps` at the presentation layer: an identity that also appears as a window title is a window, never an app to launch |
| B2 | Home page doesn't scroll | Home body never wrapped in `ui::scroll` (helper existed unused on this path) | wrap the work section in the scroll helper |
| B3 | Pod bar cards inert | two sibling `egui::Area`s at equal `Order` (backdrop + panel): z/input order unreliable | one Area; backdrop painted+hit-tested first, panel content after (later widgets win) |
| B4 | App lags | `ctx.request_repaint()` every frame while the bar is visible (60fps churn) + per-frame unconditional repaint loop | repaint on input/toggle only; egui already repaints on input |
| D1 | Web pages opening inside Evo on entry ("rooms concept") | `enter_at` set `wants_browser = true` unconditionally | entry never opens the pane; "Pages" button opens it on demand; leave/pods-switch hibernation unchanged |

**Arrangement/containment had already shipped (round 1):** `PodRuntime.enter`
plans and applies stage-recipe moves; the user's note showed honest zeros
because Accessibility was off — that is the conservative path working as
designed. The fix for "windows scattered" is therefore mostly **permission
velocity**: make the grant path impossible to miss (per-pod note says what
was skipped and why), plus B1's hygiene so restoring doesn't open junk.

**The "VS Code files" resolution:** the user's analogy — workspaces that
open their applications like an editor opens files, selectable instead of
scattered — maps exactly onto hero/satellite staging: the pod arranges its
own windows into an explicit, reproducible spatial layout (hero front,
satellites ordered, rail small), and the pod bar is the switcher. We add
*no* macOS-Space manipulation: Spaces reordering without consent is scarier
than tidy arrangement on the current desktop.

Non-goals this round: Stage Manager integration, multi-display placement
(single-screen signature only; cross-screen recipes exist in the recipe model
but aren't exercised), per-app AX worker concurrency (moves are serial —
fine at pod sizes ≤ ~10 windows).
