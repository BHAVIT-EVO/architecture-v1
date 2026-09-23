# Pods (IS-0023) — Mac Verification Checklist

~10 minutes. Paste screenshots + log tails back; every line has its command.
If anything fails, stop and report — do not patch first.

## 0. Setup

```bash
git fetch origin && git checkout pods && git log --oneline -1
# expect: 6f743c7 Pods (IS-0023): replace Rooms end-to-end ...
./scripts/build-app.sh 2>&1 | tail -5
# expect: build succeeds, app bundle produced
open target/release/bundle/osx/Evo.app   # or the script's own launch instruction
```

## 0.5. Permission FIRST (mandatory for the pod legs)

Arrange/contain are AX operations — macOS denies them silently without it.
(Also: every dev rebuild revokes a previous grant — ad-hoc signing changes
the binary's identity.)

- [ ] Yellow **PARTIAL** banner gone? If visible: Open System Settings →
      Accessibility → enable Evo → click **Check again**.
- [ ] No PARTIAL banner before proceeding below.

## 1. Baseline Home (30 s)

- [ ] Evo opens to Home; work cards listed as before.
- [ ] Card buttons say **"Enter this pod"** (not "room"), **"Leave pod"** when active.

## 2. Enter / Leave pod (2 min)

- [ ] Click **Enter this pod** on a work that has 2+ witnessed apps open.
- [ ] **Windows tile onto the stage**: hero window takes the wide-left area,
      satellites stack at right; foreign windows dim/park per containment
      mode. The note says "N arranged" (N > 0) — not zeros.
- [ ] Restore note is clean: no "Unable to find application named '<page
      title>'" lines; at most "[2 real failures … +N more]" style honesty.
- [ ] **No web pages open inside Evo** on entry (round-2 change).
- [ ] With a pod active, Home strip reads `● In: <name…> · Leave pod · Pages
      · ⌘⇧E cycles pods`. Click **Pages** → the in-Evo pane opens on demand
      (logins isolated per work). Leave pod → pane hibernates.
- [ ] Home now scrolls when content overflows (mouse wheel / trackpad).

## 3. Presence (2 min)

- [ ] Menu-bar icon: lists pods by recency, active one marked ●; **Leave pod**, **Open Evo**, **Pod bar ⌥Space** items present.
- [ ] **⌘⇧E** from ANY app: cycles pods (each press enters the next pod; desktop follows).
- [ ] **⌥1 … ⌥9**: enters the pod at that index directly.

## 4. Pod Bar (1 min)

- [ ] **⌥Space**: the bar overlays the current window; cards show color rail, name, reason line, mini-stage geometry, badges (only witnessed ones — unsaved/uncommitted/download ready/unfinished/shared).
- [ ] Click a card → pod enters, bar closes. **Esc** or click-outside → closes with no side effect.

## 5. Evidence to paste back

```bash
tail -50 ~/Library/Application\ Support/Evo/logs/evo.log
```

- [ ] Log tail pasted.
- [ ] Screenshot of the Pod Bar open.
- [ ] Screenshot of menu-bar menu.

## Round-2 regressions to look for

- [ ] Pod bar responds on first click (single-area input fix).
- [ ] No perceivable lag while the bar is open (repaint loop removed).
- [ ] "0 arranged / 0 veiled / 0 parked" notes acceptable ONLY if the pod
      genuinely had no matching windows.

## Known sandbox-unverifiable items (accept/reject on this machine)

- Veil stacking level (dim mode overlays sit at status level; may hover above menus).
- Hotkey registration order (⌥1–⌥9 ids 2..10, ⌥Space id 11) — first real registration test.
- `restore_set` URLs feeding the browser; previously `bundle.urls` did not exist (field fixed to `restore_set` during the build — first live hydration test).

## Round 3 — spaces (2026-09-23)
- [ ] Default containment is Stage: entering a pod makes the desktop show
      ONLY that work; other apps' windows disappear without piling in the
      Dock (their apps hide). A Chrome used by two pods keeps the pod's
      window visible and parks only its alien window.
- [ ] The pod's own minimized window comes back when entering (note says
      "N remembered"), and leaving re-parks it (desktop returns exactly).
- [ ] Pod bar: panel is opaque (no wallpaper bleed-through); with >4 pods
      the strip scrolls sideways; active card shows the stage receipts.

## Round 4 — Add to Pod + exactly-what-needs-Accessibility (2026-09-23)

### Accessibility: what to tick (and what NOT to)
Verified against the code's process graph:
- Evo.app (the bundle you granted: Evo-Evolution.nosync/dist/Evo.app) —
  CORRECT and sufficient for the installed app. The capture daemon
  (evo-daemon) ships INSIDE the same bundle (Contents/MacOS/evo-daemon,
  verified in daemon.rs::daemon_binary_path), so one permission covers
  both. Nothing else is needed for the installed app.
- Do NOT add: Screen Recording (nothing reads pixels — capture is AX
  titles/documents only), Full Disk Access, Input Monitoring.
- ONLY if you run dev builds from a terminal (`cargo run -p evo-desktop
  --release`): a dev binary is Terminal's "responsible process", so
  Terminal.app needs the Accessibility tick instead — or just run the
  installed Evo.app with the tick it already has.
- Clean-up: remove any stale "Evo" entries pointing at old paths you no
  longer run (macOS keys the list per app path; dead entries do nothing
  but confuse). Keep: the dist/Evo.app one.
- After granting: quit and relaunch Evo (the TCC cache applies on
  restart). The PARTIAL banner should flip to full; then arrangement /
  Stage / claiming actually execute.

### Round-4 verification items
- [ ] Bar "+": picker opens, live windows listed, claim adds ("Added …"),
      the claimed window now belongs to the pod's stage on next enter.
- [ ] Teach an app + a file + a URL; Done; relaunch Evo; re-open bar:
      chips still there (pod-addons.tsv persisted); tap chip = opens.
- [ ] Enter the pod: saved resources re-open before staging ("N pod
      resources re-opened" in the note).
