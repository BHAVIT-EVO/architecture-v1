# IS-0023 — Pods

**Status:** Proposed (concept frozen; implementation landing)
**Depends on:** IS-0022 (Rooms; superseded by this document — IS-0022 remains
frozen as history), IS-0019, IS-0021, RFC-0014

---

## 1. Purpose

Rooms made a work *a place you enter*. Pods make a work **a thing you switch
between** — a first-class object with a face, a voice, and physical gestures.

IS-0022's remaining problem: the room had no identity of its own. The person
still navigated an invisible list. Pods add:

* **The Pod Bar** — one surface where every live work appears as a card:
  name, color, miniaturized stage, one-line resume reason, honest open-delta
  badges, and direct hotkeys (`⌥1..9`).
* **Stage choreography** — entering a pod arranges its windows into a
  remembered layout (hero window large and central, supporting windows as
  satellites, reference companions in a thin rail). The layout belongs to
  the work, learned across days, not re-performed each morning.
* **Dim** (default) alongside park — non-pod windows stay on screen behind a
  translucent veil with the OS kept honest: nothing is gone, it is just not
  in your way. Park (minimize) and hide remain available modes.
* **Gestures are declarations** — dragging a window onto a pod card, merging
  two cards, or ejecting a window emits the engine's `Declaration`
  (`SameWork`, `SeparateWork`, `NotMine`) so the physical correction *is*
  the engine's only merge path.
* **Instance isolation (P1)** — a pod may run its own app instances (its own
  VS Code data dir, its own Chromium profile) through an explicit
  **InstancePolicy**: declared configuration per bundle id, never string-
  matched app knowledge embedded in code.
* **Wrapper bundles (P1)** — per-pod real macOS app identities (child
  bundles with their own `CFBundleIdentifier`, ad-hoc signed) so Dock and
  Cmd-Tab show *the work*, not the app.

## 2. Invariants

1. **No hardcoded applications.** Behavior is evidence-driven: surfaces from
   the work's own witnesses; launch behavior from the InstancePolicy file a
   user or integrator declares. Code knows *shapes* (document, page, title),
   never products.
2. **Silence over invention.** Badges render only from engine `Delta`s;
   suggestions render only above their evidence floor; a pod bar never shows
   what the engine didn't say.
3. **Under-containment.** Dim/park touches only windows provably not part of
   the active pod. A doubtful window stays fully visible.
4. **Reversibility.** Every activation produces a lease; deactivation replays
   it in exact reverse order. Nothing is touched that the lease can't undo.
5. **The pod bar is a view of the engine,** not its own state machine. The
   engine is the only grouping truth; the bar re-derives from it.

## 3. Data flow

```
evo-threads Engine ──► daemon DisplayThread ──► pods::Pod (surfaces+badges)
                                                  │
window inventory (Accessibility frames) ──────────┤
                                                  ▼
                                    StageRecipe (resource→role, fractions
                                    per DisplaySignature) + DimPlan
                                                  │
                                        PodLease (receipt)
```

## 4. Modes

| Action | On pod's own windows | On on-screen foreign windows |
|---|---|---|
| enter_pod | restore missing targets, then arrange per recipe, hero raised | dim overlay (default) / park / hide |
| leave_pod | untouched (person's arrangement is theirs) | veil lifted; lease reversed |
| mothball | apps quit (after confirm if dishonest deltas exist); recipe persisted | veil lifted |
| rehydrate | targets reopened, recipe applied | — |

## 5. Verification gates (before merge)

* Pure-logic suite: recipe derivation/layout math/lease reversal/gesture→
  declaration/policy parsing/suggestions — all platform-free.
* `cargo test --workspace` green on Linux **and** the macOS toolchain.
* Compile-level: no app-name literals in `crates/evo-pods/src`
  (allowlist: documentation comments).
* Mac-side: Gate-C checklist (choreography feel, veil correctness, hotkeys,
  wrapper identity in Dock/Cmd-Tab) — runs on the maintainer's machine.
