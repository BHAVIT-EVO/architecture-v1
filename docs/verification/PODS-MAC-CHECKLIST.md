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

## 1. Baseline Home (30 s)

- [ ] Evo opens to Home; work cards listed as before.
- [ ] Card buttons say **"Enter this pod"** (not "room"), **"Leave pod"** when active.

## 2. Enter / Leave pod (2 min)

- [ ] Click **Enter this pod** on a work that has 2+ witnessed apps open.
- [ ] Non-members hide; members stay/raise. The restore note appears (apps opened, documents restored or honestly reported).
- [ ] If the work has witnessed URLs: the in-Evo browser panel opens with its own tabs (logins isolated per work — sign in to a site in one pod's panel, check it's NOT signed in in another pod's panel).
- [ ] Click **Leave pod** (menu bar or Home): the full desktop comes back, browser panel hibernates.

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

## Known sandbox-unverifiable items (accept/reject on this machine)

- Veil stacking level (dim mode overlays sit at status level; may hover above menus).
- Hotkey registration order (⌥1–⌥9 ids 2..10, ⌥Space id 11) — first real registration test.
- `restore_set` URLs feeding the browser; previously `bundle.urls` did not exist (field fixed to `restore_set` during the build — first live hydration test).
