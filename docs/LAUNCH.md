# Launch checklist

Every line is marked **verified**, **unverified**, or **needs-human**. Nothing
here is an approval; it is the current evidence, dated 13 November 2026.

## Build and tests

| Item | Status | Evidence |
|---|---|---|
| `cargo fmt --check` clean | **verified** | exit 0 after `cargo fmt` |
| Library and unit tests | **verified** | `cargo test -p evo-desktop --lib` → 288 passed |
| Security unit tests | **verified** | 19 passed (path traversal, symlink escape, invalid input, denied permissions) |
| Journey integration tests | **verified** | `cargo test -p evo-desktop --test journeys` — first run, first wake, open a recent, pause, refused permission |
| Trust constraints | **verified** | `cargo test -p evo-desktop --test trust_constraints` → 7 passed |
| Web-layer tests | **verified** | `node --test "home/*.test.js"` → 21 passed |
| Static site links | **verified** | every internal `href` in `site/*.html` resolves |
| `clippy --workspace -D warnings` | **unverified** | `cargo clippy --fix` cleared the machine-applicable half (unused imports, most `transmute` calls, 11 of 17 collapsible `if`s, duplicated attributes). ~45 remain in `evo-desktop`, almost all edition-2024 `unsafe_op_in_unsafe_fn` (`CFRelease`, `AXUIElementCopyAttributeValue`, `msg_send_id` in `room_macos.rs` and `websurface.rs`), plus 6 collapsible `if`s and 3 simplifiable booleans. CI will be red until each call is wrapped in an `unsafe {}` with an invariant comment. |
| No unused dependencies | **verified** | `cargo machete .` → none found |                                                  |
| `cargo audit` clean | **unverified** | 1 advisory: `libsqlite3-sys 0.30.1` (bundled SQLite CVE). Needs a rusqlite major bump. |
| CI pipeline exists | **verified** | `.github/workflows/ci.yml` — fmt, clippy, tests, audit, web tests, link check |
| CI has run on a clean runner | **unverified** | written locally; never executed on GitHub |

## Behaviour

| Item | Status | Evidence |
|---|---|---|
| Design proof, Home | **verified** | 0.71 % of pixels differ from `mockup-home-v3.html` at 1100×806 (header 3.24 %, left 1.34 %, notch 4.36 %, world 0.01 %) |
| Native window matches the mockup | **unverified** | the diff measures the web reference (ADR 0005), not the egui window |
| First run, first wake | **verified** (web) / **unverified** (native) | onboarding walked end to end in the browser; native first-run is a 3-step prototype |
| Open a recent work | **verified** (web) / **unverified** (native) | scene/list selection and Continue in the preview; native restore not scripted |
| Pause | **verified** | kill-switch model tested; web refuses open/listen while paused and the tray toggles it |
| Permission denied | **verified** (model + web) / **unverified** (native run) | out-of-scope denied in all modes; web permission-denied state exists; no scripted denied-mic native run |
| Notch frame rate | **verified** | p50 2.247 ms, p99 4.385 ms per 1× listening paint (release, 300 samples) |
| Idle CPU under 2 % | **failed** | `evo-agent` ≈ 35 % of one core at idle |
| Idle memory | **verified** | agent 19.8 MB, daemon 6.7 MB RSS |
| Startup | **partly verified** | web shell 292 ms to DOM ready; native time-to-first-frame unmeasured |
| Offline | **verified by construction** | no transport exists; trust-constraints test enforces it |
| Crash reports / analytics | **verified local** / **unverified transport** | hook and local records tested; no uploader exists |

## Website

| Item | Status |
|---|---|
| All pages exist and read without JS | **verified** |
| Hero scene and notch demo run | **verified** (no console errors) |
| Structured data, sitemap, robots | **verified present** / **unverified in Search Console** |
| Lighthouse ≥ 95 | **unverified** — not run (tool not installed here) |
| Mobile layout | **partly verified** — no horizontal overflow at 430 px; not visually reviewed |
| Real favicon / OG image / icons | **unverified** — `site/assets/*.svg` are placeholders drawn for this build |
| Download works | **unverified** — buttons lead to `thank-you.html`; no installer attached |

## Documents

| Item | Status |
|---|---|
| README, ARCHITECTURE, CONTRIBUTING, CHANGELOG | **verified** (this phase) |
| PRIVACY.md, SECURITY.md, docs/ANALYTICS.md, docs/RELIABILITY.md | **verified** |
| ADRs 0001–0005 | **verified** |
| Design review of every screen without a mockup | **unverified** — listed in TODO.md |

## Tasks only you can do

1. **Signing and notarization.** An Apple Developer account, a Developer ID
   Application certificate, `codesign --deep --options runtime` and
   `notarytool submit` with an app-specific password. Nothing in the repo can
   do this.
2. **Domain.** Confirm whether the site ships at `evo.app`; canonical URLs,
   sitemap, robots and OG tags all assume it.
3. **Art.** The OG image, favicon set, app icon and the website's hero stills
   need real assets in the locked style (`ASSETS.md` prompts). The current SVGs
   are placeholders.
4. **Legal review.** The Privacy Policy and Terms need a lawyer's read; so does
   the bundled-model licensing (whisper, openWakeWord, Instrument fonts under
   OFL).
5. **Analytics decision.** Confirm whether analytics and crash uploads ship at
   all. If they do, they need a transport, a retention period, a data-protection
   review and a change to the "Stays on this Mac" wording.
6. **Accessibility review.** Keyboard paths are built and the tokens were chosen
   for contrast, but a screen-reader pass with someone who uses one is owed.
7. **Design review** of the screens with no mockup: intro, onboarding, settings,
   search, activity, help, diagnostics, the four states, the tray menu, and the
   whole website.
8. **Idle CPU decision.** The 35 % figure is a product decision as much as an
   engineering one: gating the wake pipeline changes wake latency, and someone
   has to choose the trade.
