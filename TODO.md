# TODO

## Needs design review (built from §4 components, no mockup)

- **Intro film** (`ui/app/intro.js`) — five captions in the h1 voice over scene mode `intro`, the notch waking, the three-work reveal and the hand-over. No `design/mockup-intro.html` exists; the sequence uses §2 type, tokens and the mockup's notch only.
- **Onboarding** (`ui/app/onboarding.js`) — the four resumable steps in the Home card. No `design/mockup-onboarding.html` exists; the card reuses `.panel-card` in the Home column.
- **Permission pages** — the microphone page can really ask; the window-access page cannot (a web page cannot grant Accessibility), so it says what the installed app will do instead.
- **Search** (`ui/app/search.js`, ⌘K) — a centred 640px dialog over the dimmed scene. No mockup.
- **Settings** (`ui/app/settings.js`) — 720px sheet in scene `quiet`, six tabs as a segmented control, rows with toggles, selects and a hotkey recorder. No mockup.
- **Activity** (`ui/app/activity.js`) — three tabs (Activity, Permissions, Data), Pause everything at the top. No mockup.
- **Theme** — the Appearance tab offers Dusk and Match macOS; §2 has no light palette yet, so both paint dusk.
- **Tray** — Pause everything also needs to sit in the menu-bar tray; the tray is native (S3), so only Settings and Activity reach it in this build.

## S6 known gaps

- **Native tray** — the menu-bar menu in `crates/evo-desktop/src/presence.rs` still lists pods, Leave pod, Next pod, Pod Bar and Open Evo. Status, Pause and Quit are in the web tray menu only; wiring Pause to `notch.rs` and adding Quit needs a build and a test pass.
- **Help, Diagnostics, the tray menu and the four states** — built from §4 components; no mockup, so all of them need design review.
- **Single copy file** — this section's strings live in `ui/app/copy.js`; earlier screens still hold their strings inline and need one pass to move.

## S7 website (needs design review)

- **Hero** — full-bleed scene with the notch demo and one CTA. No `design/mockup-site.html` exists, so the hero, its crop and the notch's placement over it need design review.
- **Pages** — Download, Thank-you, Guide, Privacy and trust, Changelog, About, Contact, Privacy Policy, Terms, 404. All built from §4 components and the site's own type scale; none have a mockup.
- **Missing art** — `assets/og.svg` and `assets/favicon.svg` are drawn as placeholders; §12's OG image and icon set still need the real assets.

## Phase 5 findings

- **Idle CPU fails its budget** — `evo-agent` uses ~35 % of one core at idle against §10's 2 %. The always-on wake pipeline needs gating (run the wake model only when audio is above a floor, or move it to a lower-priority thread).
- **Native startup unmeasured** — the window's time-to-first-frame and its own memory are not captured; the web shell's 292 ms and the notch's paint budget are in `docs/RELIABILITY.md`.
- **Degradation is shown, not scripted** — broken voice engine and denied microphone are proven by code paths and the web layer's states; a scripted native run is still owed.
- **No transport for crash reports or analytics** — records stay on disk; `docs/ANALYTICS.md` says so.

## Phase 7

- Run `cargo fmt` across the workspace (140 formatting diffs, now 0) and add `.github/workflows/ci.yml` (fmt, clippy `-D warnings`, tests, audit, web tests, link check).
- Added `crates/evo-desktop/tests/journeys.rs`: first run, first wake, open a recent, pause, refused permission — 7 passed.
- Wrote README documents section, ARCHITECTURE.md, CONTRIBUTING.md, CHANGELOG.md, docs/LAUNCH.md, ADRs 0001–0005.
- **Clippy is not clean** — 32 warnings in `evo-desktop`, dominated by edition-2024 `unsafe_op_in_unsafe_fn` in `websurface.rs` and `notch_window.rs`. CI will be red until they are wrapped in `unsafe {}` with invariant comments.
- CI has never run on GitHub; the workflow is untested.

## Phase 7, clippy pass

- `cargo clippy --fix` applied the mechanical fixes (unused imports, most `transmute` sites, 11 of 17 collapsible `if`s, duplicated attributes) and the suite still passes 288.
- Remaining ~45 warnings are edition-2024 `unsafe_op_in_unsafe_fn` in `room_macos.rs`/`websurface.rs`; each needs an `unsafe {}` with a stated invariant, not a blanket allowance.
- `cargo machete .` finds no unused dependencies, so nothing was removed from the manifests.
