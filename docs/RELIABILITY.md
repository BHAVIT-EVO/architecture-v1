# Reliability and performance — measured

All numbers below were taken on this Mac (Apple silicon, macOS 26.4.1) on
13 November 2026. Nothing here is an estimate.

## Startup

| What | Number | How |
|---|---|---|
| Web shell, first paint / DOM ready | **292 ms** | `performance.getEntriesByType("navigation")[0].domContentLoadedEventEnd` in Diagnostics on `app/home.html`. The paint-timing entry was empty in that browser, so the DOM-ready fallback was used. |
| Web shell, JS heap after load | **3 MB** | `performance.memory.usedJSHeapSize` in Diagnostics. |
| Native window startup | **not measured** | The installed app was already running; launching it would take over the screen. The binary was built and linked, but time-to-first-frame was not captured. |

## Idle cost

Sampled over a 30-second window with `ps` CPU-time deltas, `Evo.app` helpers
running with no voice command issued:

| Process | CPU time in 30 s | Share of one core | Resident memory |
|---|---|---|---|
| `evo-agent` (always-on listener) | 10.60 s | **~35 %** | 20,240 KB (19.8 MB) |
| `evo-daemon` (capture) | 0.06 s | **0.2 %** | 6,864 KB (6.7 MB) |

**This misses the budget.** Brief §10 asks for idle CPU under 2 %; the agent
sits at about 35 %. The daemon is fine. The always-on voice pipeline is the
suspect: wake-word inference and the microphone loop run continuously rather
than gating on speech. Treat this as a failure with an owner, not a footnote.

## Notch frame rate

`paint_into` for the 1× LISTENING surface, release build, 300 samples
(`cargo test --release -p evo-desktop --features evo-notch measure_paint_into_p50_baseline -- --nocapture`):

| Percentile | Cost per frame | Ceiling |
|---|---|---|
| p50 | **2.247 ms** | ~445 fps |
| p95 | 3.403 ms | ~294 fps |
| p99 | 4.385 ms | ~228 fps |

Against the budgets that matter: 60 fps allows 16.7 ms, so the median frame
uses **13 %** of it and the p99 uses 26 %. The listening equalizer's 24 fps
cadence allows 41.7 ms, so it uses **5 %** (p99: 11 %). The notch is not
frame-bound.

## Graceful degradation — what was shown

| Failure | Evidence |
|---|---|
| **Voice engine fails or dies** | The engine manager caps consecutive failed spawns at 3 (`engine.rs`), so a broken binary cannot turn the UI thread into a respawn loop; the agent restarts a panicked voice thread (`bin/evo-agent.rs`, "voice thread panicked; restarting"). The hotkey and typed commands do not depend on the voice path. Not exercised against a deliberately broken binary in this session. |
| **No network** | `cargo test -p evo-desktop --test trust_constraints` → **7 passed**, including `nothing_in_the_workspace_can_transmit_what_evo_witnessed` and `no_source_reaches_anywhere_but_this_machine`: no HTTP/TLS client exists, so the offline path is the only path. |
| **Denied microphone** | The web layer renders the permission-denied state as one panel with one action (`site`/`ui/app/states.js`, `?state=denied`, screenshot `docs/shots/s6-denied.png`) and the native daemon reports `unavailable: grant Input Monitoring in System Settings…` in its status line. The Diagnostics microphone test was exercised up to the browser's permission prompt; a denied-grant run was not completed. |
| **Panic** | `install_panic_hook` prints `EVO-PANIC: <message> at <file>:<line>` and re-runs the default hook; with reports on it appends a JSON line to `crashes.jsonl`. Covered by `observability` tests. |
| **Full disk / unwritable log** | `Logger::write` returns `false` instead of panicking; test `a_missing_log_directory_is_not_an_error`. |

## Tests

- `cargo test -p evo-desktop --lib observability` → **6 passed**.
- `cargo test -p evo-desktop --lib` → **288 passed, 0 failed** (282 + the 6 new).
- `cargo test -p evo-desktop --test trust_constraints` → **7 passed**.
- `cargo build -p evo-desktop` → links (debug).

## Open

1. Idle CPU at ~35 % for `evo-agent` — the one number that fails its target.
2. Native window startup and the window process's own memory are unmeasured.
3. Denied-microphone and broken-voice-engine paths are shown by code and by
   the web layer's states, not by a scripted native run.
4. Crash uploads and analytics transport do not exist; the records stay local.
