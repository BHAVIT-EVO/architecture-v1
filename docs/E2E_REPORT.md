# E2E Report — Evo (installed /Applications/Evo.app)

Date: 2026-11-15. Build: Nov 14 2026 17:22, signed with "Evo Code Signing"
(identity hash 033e7bffb4213a34b0252d6ea9003184cf4998bd). All evidence below is
from the installed bundle.

| # | Step | Result | Evidence |
|---|------|--------|----------|
| 1 | Cold start under 2s, no crash; fresh data dir shows first-run onboarding | PASS (launch) / CANNOT VERIFY (onboarding) | `/Applications/Evo.app/Contents/MacOS/Evo` printed `EVO-PRESENCE: menu bar item + hotkey installed` and `EVO-WEB serve evo-app://app/app/home.html?shell=1&entry=off` within ~0.5s of launch; no crash/panic in `/tmp/evo.log`. First-run onboarding path could not be exercised live (no accessibility to drive the UI and `EVO_STORAGE_ROOT` does not propagate through `open`); onboarding step persistence is covered by `onboarding.js` + `get_settings`/`set_settings` and bridge tests. |
| 2 | Home shows real recent work, cleaned titles, no sample data; newest selected | CANNOT VERIFY (live) / PASS (unit + earlier) | Earlier S9-2 evidence: `EVO-WEB got 26 bytes from the page`, `EVO-WEB answered get_works with 4691 bytes`, `EVO-WEB reply #1 4691 bytes`; screenshot `docs/shots/s9-2-home.png` shows "Frontend prompt for product redesign — BHAVIT", "Claude in Chrome", "Show 28 more in the list" from this Mac's ledger. Installed run produced `EVO-WEB got 26 bytes from the page`; `get_works` takes ~80s (engine replay) so the full reply was not captured in the window. `scripts/check-ui-bundle.sh /Applications/Evo.app/Contents/Resources/ui` → exit 0 (no sample data). |
| 3 | Continue opens the work | CANNOT VERIFY (live) | No UI drive available. Backend `continue_work` covered by lib tests and `home.js` calls `api.continueWork(w.id)`. |
| 4 | Scene/List toggle, search, Cmd+K, "..." actions change real state and persist | CANNOT VERIFY (live) | Persistence of rename/hide/pause covered by `a_rename_a_hide_and_pause_survive_a_restart` (PASS). Search uses `evo_ledger::Ledger::search` (FTS5). |
| 5 | "Hey Evo"/hotkey expands notch, live transcript, opens right work, collapses; type-to-Evo with mic denied | CANNOT VERIFY (voice) / PARTIAL | `evo-agent` runs and prints `EVO-NOTCH geometry: notch_x=643.5 notch_w=183.0 notch_h=32.0 screen=1470x956 (safe-area probe)` and `EVO notch: display surface 183x32pt acquired`; decision `Some(Own)`. No duplicate notch window observed. Voice/transcript/open-work flow not drivable headlessly; type-to-Evo-with-mic-denied not tested. |
| 6 | Pause everything stops listening and live chip changes; resume works | CANNOT VERIFY (live) | Backend `pause`/`resume` write `ui-state.json`; kill switch survives restart (test PASS). Tray menu Pause/Resume present (`presence.rs`). |
| 7 | Activity shows action just taken; undo works where offered | CANNOT VERIFY (live) | `the_privacy_reply_is_the_real_store_and_activity_undoes_a_hide` (PASS). Undo reverses hide/rename/merge exactly once. |
| 8 | Permissions screen reflects real scopes; denied permission shows right message | CANNOT VERIFY (live) | `get_privacy`/`set_permission_mode`/`set_scope` implemented and read by `activity.js`/`settings.js`; `security::permissions::PermissionSet` is the only enforcer. |
| 9 | Settings persist; Delete everything works and returns to first run | CANNOT VERIFY (live) | `settings_are_saved_and_export_writes_a_file_delete_removes_it` and `export_and_delete_work_end_to_end` (PASS). Appearance/onboarding step persist in `ui-state.json`. |
| 10 | Network off: everything local still works | CANNOT VERIFY | Not tested with network disabled. App makes no outbound network calls in the verified paths (storage, bridge, notch are local). |
| 11 | Idle CPU under 2% for 60s, with numbers | PASS (sampled) | `top` sampled 3× over ~10s on the running app: 0.0%, 0.0%, 0.0% CPU. Full 60s window not measured. |
| 12 | Quit and relaunch: state intact, notch correct, no duplicate notch window | CANNOT VERIFY (live) | Geometry re-probes on launch (`notch_x=643.5 notch_w=183.0 notch_h=32.0`). Single `evo-agent` instance observed; no duplicate notch process. State persistence covered by restart tests. |

## Checks actually run
- `cargo test -p evo-desktop --lib`: 311 passed (bridge, ui-state, notch 93).
- `node --test "crates/evo-desktop/ui/app/*.test.js"`: 3 passed.
- `node --test "ui/home/*.test.js"`: 21 passed.
- `scripts/check-ui-bundle.sh /Applications/Evo.app/Contents/Resources/ui`: exit 0.
- `codesign -dv` / `codesign -d --requirements -`: designated requirement unchanged before/after.
- Idle CPU samples: 0.0%, 0.0%, 0.0%.

## Limitations
Most interactive UI steps (onboarding, Continue, search, pause, activity undo, permissions, settings delete, voice/hotkey transcript, type-to-Evo) could not be driven because this environment has no Accessibility control over the running window; they are covered by the bridge/unit tests listed above, not by live clicks. `get_works` takes ~80s (engine replay), so the full Home data reply was not captured in a short window. Network-off and full 60s CPU runs were not performed.

## Crash fix (post-report)
Root cause: `std::io::stdio::_eprint` panics when writing to a closed stderr (apps launched via LaunchServices have no console). The faulting frame was `evo_desktop::webshell::macos::start_url_scheme_task` at its `eprintln!("EVO-WEB serve …")`. Fix in `crates/evo-desktop/src/main.rs`: at startup, `dup2(/dev/null, 2)` so fd 2 is always writable. Rebuilt and reinstalled; app now stays running when launched via `open` (verified, no new crash report). All unit tests still pass (311 lib, 3 app, 21 home).
