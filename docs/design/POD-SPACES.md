# Pod Spaces — One Space per Pod
**Design research + proposal · round 3 · 2026-09-23 · STATUS: PROPOSAL — awaiting discussion before build**

---

## 1. The problem, in the field

On hardware today, pods materially change *one* desktop:

- A pod's windows enter hero-tiling on the **desktop the user is already sharing** with everything else.
- Opening pod #2 and pod #3 tiles their windows on **the same** desktop.
- 3 pods × 4 windows = ~12 windows interleaved in one space. Chrome used by two works appears twice side by side with zero visual linkage to either pod. There is no notion of "a pod owns a space — entering it shows me my work, leaving it returns me to everything else."

The user expectation (verbatim round-3 ask): *"each and every pod is opened in a different space, so that whenever I jump to the next space I have my other work opened"* — and the solution must work **universally, for every app and window, no app-specific code**.

The round-2 hero/style fixes made arrangement correct; this round is about **space semantics**, not tiling math.

---

## 2. What the platform actually offers (researched)

### Option A — Real macOS Spaces per pod (private SkyLight bridge)

**Findings, fresh as of this round:**

- yabai supports `space --create/--destroy/--focus` only via its **scripting addition injected into Dock.app** — i.e., partially disabled SIP (filesystem protections). That route is a non-starter for a default Evo install. (yabai issue #1582: "cannot create/destroy/focus space due to an error with the scripting-addition"; yabai config docs confirm injection into dock.app.)
- yabai's `window --space N --focus` **"works with both SIP enabled and disabled"** per its wiki — *moving a window to an existing space* is reachable with SIP on.
- **`native-space-kit`** (github.com/hhh2210 & Fjx-dylanZ forks) demonstrates SIP-**enabled** full control via private SkyLight WindowManager bridge: enumerate spaces, create a real type-0 Desktop, activate, move-window between Desktops, reorder/swap, destroy with optional native window migration. Verified scope caveats:
  - **Fullscreen / sticky / modal application windows cannot be moved** between spaces (outside the verified contract).
  - **Destroy refuses the active space** and the last Desktop; migration is opt-in and deferrable.
  - No atomicity guarantees on compound ops; **private API — "availability and behavior may change with macOS updates."**
- Distribution: fine for Developer-ID/self-signed apps; **not** App-Store-safe. Fragility is real but bounded — worst case a macOS update breaks it and Evo must degrade gracefully.

**Verdict:** buildable without SIP changes; true OS-native spaces in Mission Control; but fullscreen-window exclusion hits hard (Chrome fullscreen is common), plus an update-fragility tax.

### Option B — Stage Manager groups

No public API for creating/managing Stage Manager groups. On-screen control only, per-workspace automation impossible. **Dead end for automation** (fine as an OS parallel the user can enable manually).

### Option C — Evo-owned virtual spaces ("Pod Desktops"), pure AX

Evo keeps owning no extra OS surface: **at any moment exactly ONE pod stage is visible** on your desktop; every other pod's windows are *parked* (minimized to Dock or per-pod chips tray) and restored by lease when that pod is entered. Switching pods = reverse current pod's stage lease + apply target pod's stage lease. "Leave pod" becomes "Back to free desktop" (restore everything non-pod).

- Works **today** with the AX verbs + lease/trust machinery already shipped and hardware-verified in rounds 1–2. Zero private APIs, zero SIP states, works for **every standard window of every app**.
- "Space" is conceptual (Evo's), not visible in Mission Control — mitigated by the stage's identity rail + pod bar making containment explicit.
- Parked windows keep running; unsaved docs stay parked unharmed (open-doc protection untouched).

### Option D — Per-app isolated instances (profiles/VMs)

True data isolation but off-mission (users want Chrome-in-two-pods, not two Chrome installs) and app-by-app. **Rejected** for the base mechanic; mentioned only as a future per-pod "isolation mode."

---

## 3. Recommendation — hybrid, phased

**Phase 1 (build next): Pod Desktops (Option C).**
Change the *default stage recipe* for a pod from "hero-tile my windows among everything" to **full-stage**: on entry, park (minimize) every regular window that is not a member of this pod, then lay out the pod's members across the *whole* screen. On exit/leaving, restore the desktop exactly (existing lease reversal). Pod switching = slide from stage to stage: `park others → restore mine → tile mine`. Effects:

| User story today | After Phase 1 |
|---|---|
| 3 pods × 4 windows fight on one desktop | One 4-window stage visible; 8 parked; ⌘⇧E slides between works |
| "Chrome can't belong to 2 works at once" | It does — whichever pod is open pulls its Chrome window(s) to its stage; other pod parks them |
| "there's no next space to jump to" | Each pod *is* a next space; Mission Control stays clean |
| Actually closing all windows feels brittle | Parking ≠ closing; unsaved docs parked safe; open-doc protection still guards against real closes |

Universal: parking/tiling are AX verbs on standard windows. No app code.

**Phase 2 (build after Phase 1 ships & stabilizes, behind `pods.realSpaces` config): real Spaces (Option A).**
Each pod gets a native macOS Desktop via the SkyLight bridge; switch = OS-native; windows migrate into their pod's space; teardown migrates members back. Evo **detects bridge failure at runtime and falls back to Phase-1 semantics transparently.** Exclusions (fullscreen windows etc.) fall back to parking for that window.

This sequencing matches the evolved product line: inferred grouping (round 1) → organization (round 2) → **virtual-space isolation (Phase 1)** → native-space isolation (Phase 2, opt-in).

---

## 4. Mechanics deltas required in Phase 1 (for discussion)

1. **`stage.rs` new containment mode `FullStage`**: parking pass over all regular non-member windows (AXMinimize) before tiling; lease gets a second reversal bucket (`parked_aliens`) today stored as minimized set; Leave = unminimize + original frames untouched.
2. **Membership pull**: on pod entry, member windows found in *any* space/any state are pulled to the stage (AX raising/untab where possible; Chrome tab claims already land here — window-level claim is AX-only, tab-level claim needs the future browser bridge, noted as known limit).
3. **Auto-join while open**: window created during pod focus (event hook already needed for round-5 inheritance) joins that pod's member list; Alt+Q already the manual escape hatch.
4. **Pod bar v2**: opaque panel, horizontal scroll for >4 pods, tighter rows, per-pod status chips (parked count), ⌘⇧E/⌥1..9 symbology — the switcher to your spaces.
5. **Free desktop**: "Leave Pod" restores the mixed desktop as a first-class zero state (`Esc` from stage, or ⌥0); nothing forces stage-ness.

Out of scope here (later rounds): real-spaces backend, CV-style semantic membership with history scoring, Evo-owned neural ranking, chat pod apps, context-snapshot inheritance (round-5 seeds: new window inherits pod's mode/projects/components).

---

## 5. Visual language (generated concepts, round 3)

- `pod-problem-scattered.png` — the 12-window mingle across 3 works (the field-reported chaos).
- `pod-stage-solution.png` — one full-stage pod, satellites docked, others parked in the chip tray.
- `pod-spaces-switching.png` — ⌘⇧E sliding between per-pod stages ("each pod is its own space").
- Earlier rounds: `pod-bar-switcher.png`, `pod-dimming.png`.

## 6. References
- yabai wiki/commands + issue #1582 (SA-injection requirement; `window --space … --focus` SIP-on reach).
- native-space-kit repos (SIP-**enabled** SkyLight bridge; verified scope & exclusions).
- Stage Manager: no public grouping API.
- Prior landscape: `docs/design/POD-WORKSPACE-LANDSCAPE.md`.

---

## 7. Round-3 decisions (user steered, research-grounded) + what shipped

**D1 — Parking UX: whole-app hide first, per-window minimize only for split
apps (built, ships as `ContainMode::Stage`, the default).** FlashSpace's
design history is the authoritative evidence: macOS grants no public
per-window hide, and of the two per-window fallbacks (minimize /
move-to-corner) neither is a good experience — their resolution was
whole-app native hide/show, per-workspace. Evo is window-granular, so the
hybrid: foreign apps with no witness in the pod hide whole (⌘H-clean,
zero Dock clutter); apps witnessed in BOTH pods keep their pod window while
their foreign windows minimize singly (standard yellow-minus semantics,
always recoverable). The pod's own minimized windows resurrect at entry and
re-park at exit (the space keeps its windows).

**D2 — Auto-join is a separate feature: "Add to Pod".** Not built this
round (user asked for more research first). Pattern taken from the
workspace-launcher family (Workspaces.app: per-project resources =
apps/files/folders/websites/Terminal locations + START; Bunch: text lists
of apps+URLs; ShiftPlus: launches + captured layouts):
- Pod model gains `resources: Vec<PodResource>` where
  `PodResource = App{name} | File{path} | Folder{path} | Url{string}`
  (persisted with the recipe store, per pod).
- Bar card gains a "＋" affordance opening a picker with two sections:
  a) **Open windows now...** — live AX inventory of the desktop grouped by
     app; picking one claims it into the pod (writes into
     `surfaces.documents`/`titles` via the daemon's pod of this thread);
  b) **Open into this pod...** — launch any app/file/folder/URL as a pod
     resource; while the pod is active its new windows are claimed by the
     existing window_match evidence (document/url/title), so the launched
     window joins on its own.
- Per-resource toggle "open when entering" feeds `restore_work`'s existing
  launch path; passive auto-join of ad-hoc windows stays OFF by default —
  explicitness beats surprise, per the user's call.

**D3 — Free desktop:** Leave remains the only zero state (no extra ⌥0
space). Confirmed, untested surface.

**D4 — Phase 2 (native SkyLight spaces): DEFERRED, not killed.** After the
research: native spaces add Mission Control presence and swipe continuity
but ship hard exclusions (no fullscreen-window moves, no active-space
destroy, space auto-rearrangement on display changes) plus private-API
churn — all direct fights with "least work to get back". Evo's virtual
stages switch instantly (the #1 praised property of FlashSpace) and are
universal. Product call: virtual is THE mechanism; revisit native only
after Phase-1 has lived in users' hands.

Shipped this round: Stage containment + lease re-park, pod bar v2 (opaque,
horizontal single strip, one-line chrome, live receipts on the active
card), note vocabulary. Test evidence: full workspace green with the
stage-mode suite added.
