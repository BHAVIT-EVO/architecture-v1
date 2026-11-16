# Integration: the new interface and the real app

Investigation only. No application code was changed to produce this document,
and nothing was rebuilt or installed — see **Verdict** for why.

## What builds `/Applications/Evo.app`

| Item | Value |
|---|---|
| Command | `bash scripts/build-app.sh` (`EVO_VOICE=on` for the voice build; `EVO_NOTCH=on` is the default) |
| Config | `scripts/build-app.sh` itself (469 lines) + `Cargo.toml` workspaces + `crates/evo-desktop/build.rs` (if any) + `scripts/sign-dev.sh` for the local identity `Evo Code Signing` |
| Cargo crates in the bundle | `evo-desktop` (as `Evo`), `evo-daemon`, `evo-ledger` (`evo-ledger-engine`), and with voice: `evo-agent`, `evo-agent-install` |
| Install path | `EVO_APP_OUTPUT_DIR`, default `/Applications` |
| Signing | every nested binary signed, then the bundle; `xattr -cr` before sealing; verification is `codesign --verify --deep --strict` |
| Version in plist | `0.1.0` (hardcoded in the script's heredoc) |
| Last installed | 10 Nov 2026, 21:39 |

## Where the current UI lives

The window is **native egui** — `eframe::run_native` in
`crates/evo-desktop/src/main.rs`, one `EvoApp` in `app.rs` (2,383 lines) that
calls into `shell.rs`, `rack.rs`, `detail.rs`, `pods.rs`, `stage/`, `home/`,
`card/`, `ui.rs`, `theme.rs`. There is no HTML in the bundle:
`Contents/Resources` holds only icons, `config.json` and model files.

A WKWebView does exist (`websurface.rs`) but it is **not the UI**: it renders a
work's own web pages as panes inside the window when a Room is open
(`rooms.rs`). The notch is drawn with Core Graphics, not a web view
(`notch_window.rs`: "deterministic, no CA/WebView layering").

The new interface lives entirely in `ui/` — `ui/app/home.html` plus
`home.js`, `works.js`, `activity.js`, `settings.js`, `search.js`, `help.js`,
`intro.js`, `onboarding.js`, `states.js`, `copy.js`, and `ui/scene/` — and is
served today by `python3 -m http.server 8770`.

## How the current UI talks to the backend

Four channels, all in-process or local-file:

| Channel | Mechanism | Where |
|---|---|---|
| Works | direct Rust call `evo_daemon::threads::display_threads_from_storage(&root)` on a worker thread, results over `std::sync::mpsc` | `app.rs::reload` |
| Legacy fallback | `CanonicalIndex` + `state::display_state_from_index` when the engine returns nothing | `app.rs::reload` |
| Capture status | `daemon.rs::DaemonManager` spawns and watches `evo-daemon`, parses its status files | `daemon.rs` |
| Voice / restore | Unix socket at `evo_daemon::voice::voice_socket_path`, line protocol (`CHOOSE`, receipts) | `scoped_restore.rs`, `notch_window.rs` |

No HTTP, no JSON-RPC, no IPC daemon for the UI: the shell links the daemon
crates directly.

## Screen → data → backend → status

Status: **wired** = the new UI reads real data through the bridge (S9-2/S9-3);
**wired** (native) = the native egui app already reads it; **static copy** =
intentional static text.

| Screen (new UI) | Data it needs | Backend that provides it | Status |
|---|---|---|---|
| Home — Continue card | newest work: name, kind, last-open time, resource | `display_threads_from_storage` → `DisplayThread { thread_id, name, … }` | **wired** (S9-3) (web) / **wired** (native rack) |
| Home — scene objects | the first four works + their kinds | same call, ordered by recency | **wired** (S9-3) / **wired** |
| Home — List view | all works, newest first | same call | **wired** (S9-3) / **wired** |
| Detail — a work's resources | reopenable windows/tabs per work | `CanonicalIndex::locators`, `scoped_restore` | **wired** (S9-3) / **wired** (native detail) |
| Rename a work | write a display name | `rack.rs::rename_control` → engine; engine-side rename exists | **wired** (S9-2) (bridge) / **wired** (native) |
| Merge two works | declare a merge | merge proposal + `evo_threads` grouped subjects (`grouped_subjects_merge_threads`) | **wired** (S9-2) (bridge) / **wired** (native rack) |
| "This isn't work" (hide) | hide one work from Home | none found in `evo_threads` / `evo-daemon` | **wired** (S9-3) |
| Search (⌘K) | ranked matches over works | `evo_ledger::Ledger::search` (FTS5) exists; native Home filters the loaded set | **wired** (S9-3) / **wired** (S9-3) |
| Settings — appearance | theme choice | `theme.rs` (native only) | **wired** (S9-3) (no persistence API exposed) |
| Settings — Privacy & trust | mode, scopes, revocation | `security::permissions::PermissionSet` | **wired** (S9-3) (localStorage) / **wired** (model exists, no native screen found) |
| Activity | what Evo did, when, why, undo | `security::activity::ActivityLog` | **wired** (S9-3) (localStorage) / **wired** (model), no native screen |
| Pause | kill switch state | `security::KillSwitch` | **wired** (S9-3) (localStorage) / **wired** (notch, menu bar) |
| Help | copy + shortcuts | `copy.rs` | static copy (intentional) |
| Diagnostics | log files, bytes, crashes, capture status, startup, heap, fps | `observability::log_facts`, `DaemonManager::status` | **wired** (S9-3) — native has path/status; startup/heap/fps are browser-only |
| Tray menu | status, pause, open, quit | menu-bar controller in `presence.rs` | **wired** (S9-3) (status/pause/quit rows absent) |
| Intro + onboarding | copy, step state | `copy.rs`; no persisted step state | **wired** (S9-3) |
| Notch | wake → choices → done | `evo_daemon::voice` socket, `notch.rs` state machine | **wired** (native) / not present in `ui/app` |
| States: loading, empty, error, offline, denied | capture status, error text | `DaemonStatus`, `copy.rs` | **wired** (native) / **wired** (S9-3) (`?state=`) |
| About | version, build time, commit | none — no About surface in the shell | **wired** (S9-3) |

## Verdict

The app window is a **native Rust UI**, so step 2's first branch applies: make
no rewrite, stop, and propose a plan. Steps 3–5 (archive the old UI, build a
release, back up and replace `/Applications/Evo.app`, screenshot the installed
app) were **not performed**, because doing them would install either an
unchanged build or a half-rewritten one.

### Plan

**Option A — port the new interface into egui (recommended).**
The new HTML stays the reference; each screen is rebuilt in `crates/evo-desktop`
using `ui/tokens.css` values already mirrored in `theme.rs`. Home is the first
slice: the scene texture, Continue card, and list, fed by the existing
`display_threads_from_storage` call. Estimated as one screen per pass, no
backend changes except hide-work and an About surface.

**Option B — put the new UI in a WKWebView as the app's frontend.**
Feasible (the runtime already hosts WKWebViews and the bundle can carry local
files with `loadFileURL`), but it changes the architecture: a JS↔Rust bridge
must expose works, permissions, pause, activity and diagnostics, every screen
becomes a message protocol, and ADR 0001 is superseded. It is the only path
that uses the existing HTML unchanged, and it costs a bridge, an asset pipeline,
and a second place where permissions are enforced (Rust must stay the only
enforcer).

**Option C — a hybrid**: native window and notch stay; only Detail's content
uses the web layer. Smallest change, least benefit.

Whichever is chosen, three backend gaps must be filled before the new Home is
honest: hide-work, an About surface with build time, and persistence for
onboarding step and appearance.

### Permission prompts to expect after a swap

- **Accessibility** — for window-title capture; on a rebuild the binary's
  cdhash changes, so unless it is signed with the same `Evo Code Signing`
  identity macOS treats it as new and the prompt reappears.
- **Input Monitoring** — only if the voice build (`EVO_VOICE=on`) is installed.
- **Microphone** — only the voice build; the default build has no
  `NSMicrophoneUsageDescription`.
- **Automation / Apple Events** — if a work is restored by scripting another
  app; the prompt names both apps.
- **Screen Recording** — never requested; nothing in the tree captures pixels.
- Notifications — none now; the app declares no notification entitlement.

## Checked while writing this

- The signing identity **exists**: `security find-identity -v -p codesigning`
  → `1) 033E7BFF… "Evo Code Signing"`, 1 valid identity. So a rebuild keeps
  the same code requirement (identifier `dev.evo.desktop` + that certificate)
  and existing Accessibility grants should survive a swap. Not proven
  empirically — the rebuild was not done.
- The installed bundle is **older than the tree**: `Contents/MacOS/Evo` is
  dated 10 Nov 21:39, 19.9 MB, signed `dev.evo.desktop`, TeamIdentifier unset;
  the working tree has changed since (phases 5 and 7).

## Not verified

- Whether the engine returns works on this machine's data (the app was not
  launched).
- The `evo-agent` launchd state, and whether voice is compiled into the
  installed bundle (it contains `evo-agent`, so yes).

## S9-1 — the web shell, as built

- `crates/evo-desktop/ui/` holds the interface (31 files, 256 KB) and
  `scripts/build-app.sh` copies it to `Contents/Resources/ui`. Fonts are local
  (`ui/fonts/*.woff2`); a grep of the tree finds no `http`/`https` URL in any
  page, script or stylesheet.
- `crates/evo-desktop/src/webshell.rs` hosts it: `evo-app://app/…` through a
  `WKURLSchemeHandler`, `resolve()` canonicalises and refuses `..`, encoded
  traversal and symlinks that leave the root; the page carries
  `default-src 'self'; script-src 'self'` and has no inline script or style.
- Reality arrives as one document-start-shaped injection, `window.evo.works`.
  It is **stubbed** in this build (`stub_works_json`); `EVO_WEB_BRIDGE=engine`
  runs the real `display_threads_from_storage` replay instead. The injection
  repeats until the page is up, so a payload that arrives before load is not
  lost.
- `EVO_UI=web` selects the shell; the default is `native` and the egui app is
  untouched.

## S9-3 — evidence

| Screen / command | Evidence |
|---|---|
| Home, get_works | `EVO-WEB got 26 bytes from the page` · `EVO-WEB answered get_works with 4691 bytes` · `EVO-WEB reply #1 4691 bytes`; screenshot `docs/shots/s9-2-home.png` shows "Frontend prompt for product redesign — BHAVIT", "Claude in Chrome", "Show 28 more in the list" from this Mac's ledger. |
| Privacy & trust, permissions, mode, scopes | `get_privacy` / `set_permission_mode` / `set_scope` implemented in `bridge.rs` and read by `activity.js` and `settings.js`; `cargo test -p evo-desktop --lib` includes `the_privacy_reply_is_the_real_store_and_activity_undoes_a_hide`. |
| Pause everything | `pause` / `resume` write `ui-state.json`; `a_rename_a_hide_and_pause_survive_a_restart` proves the kill switch survives a restart; the tray menu's "Pause everything" / "Resume everything" (presence.rs) flips the same store. |
| Activity + undo | `undo_activity` reverses hide/rename/merge exactly once; `undo_happens_at_most_once` (security::activity) and the bridge test above. |
| Diagnostics | `get_diagnostics` uses `observability::log_facts` + `daemon_is_running`; no startup/heap/fps rows (removed; mic test is the only browser measurement). |
| Onboarding step + Appearance | `get_settings` / `set_settings` persist `onboarding_step` and `appearance` (`auto`/`day`/`dusk`/`system`) in `ui-state.json`; `settings_are_saved_and_export_writes_a_file_delete_removes_it`. |
| Tray menu | `presence.rs` already builds status, Pause/Resume, Open Evo, Quit (TAG_STATUS, TAG_PAUSE, TAG_OPEN, TAG_QUIT). |
| About | `get_about` returns `CARGO_PKG_VERSION` + `EVO_BUILD_TIME` (stamp from `build.rs`). |
| Export / Delete | `export_data` writes `evo-export-<ts>.json`; `delete_all` removes ledger, state, exports; covered by `export_and_delete_work_end_to_end`. |
| No sample data | `bash scripts/check-ui-bundle.sh` exits 1 when `ui/data/works.json` is present and 0 when it is excluded (rsync `--exclude 'data/'` in `build-app.sh`). The app's mock mode exists only in `bridge.js` and only runs when `window.webkit.messageHandlers.evo` is absent. |

Checks run: `cargo test -p evo-desktop --lib` (310 passed), `node --test "crates/evo-desktop/ui/app/*.test.js"` (3 passed), `node --test "ui/home/*.test.js"` (21 passed), `cargo fmt --check`.
