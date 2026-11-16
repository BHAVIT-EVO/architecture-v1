# EVO — SPEC (S0 audit)

Audit only. **No code changed.** Evidence is marked **[V]** (read in source or seen on this machine) or **[U]** (unverified).

## 0. First gate: is the app a web view or native?

**Native.** Rust + egui/eframe 0.35, edition 2024 (`crates/evo-desktop/Cargo.toml`); a real `NSWindow` with the title bar hidden and a full-size content view, 1200×880 initial, 420pt floor (`src/main.rs`) **[V]**. There is no Tauri, no Electron, and no HTML shell. The single WebKit use is `src/websurface.rs`, an optional child view for a work's own web pages; Home, Settings and the notch draw no HTML **[V]**.

Consequence: the mockup's scene is a 320×180 canvas with a seeded generator, a 12fps loop and cover-crop hit targets. None of that can be embedded; it has to be ported into egui and painted by us.

### Proposed plan (needs your approval before any code)

1. `scene/stage.rs` — port the mockup's canvas code (seed 11, sky/grass/path/sprite functions byte-for-byte) into a 320×180 RGBA buffer, uploaded to one egui texture and repainted at 12fps, cover-cropped with the mockup's math.
2. `scene/sprites.rs`, `scene/fireflies.rs` — the four sprites, the selection caret and the glow, moved without redesign.
3. Header: leaf mark, wordmark, listening chip, "Stays on this Mac" chip, Scene/List segmented pill, search with ⌘K, gear — all in §2 tokens.
4. Home left column at mockup geometry (32px inset, 360px column, 56px serif h1, 24px panel card, 46px pill, dashed coming-soon chips, trust footer).
5. List view as the accessible equivalent, plus real hit targets over scene objects with `aria-label`s and arrow/Enter keys.
6. Only then run the §9 screenshot diff and iterate to the 3% threshold.

Stop here if you disagree with the order.

## 1. Screens that exist today

| Screen | File | Today | Gap to mockup |
|---|---|---|---|
| Home | `src/home/mod.rs`, `src/card/mod.rs`, `src/stage/mod.rs` | Flat wash + grain, serif 48px headline, one Continue card, 56px rows, "…" menus, search, gear | No scene, no chips, no toggle, no trust footer, no coming-soon, wrong geometry and type scale |
| Work view | `src/detail.rs` | Tabs Overview / Activity / Record, Continue, per-target receipts | Not in the mockup; needs design review |
| Settings | `src/shell.rs` | Capture state, Appearance, Privacy, Help, Diagnostics — one sheet | Six sections required; Help and Diagnostics must become screens |
| First run | `src/shell.rs::first_run` | 3 static steps | Brief wants 4 resumable steps with a live wake test |
| Notch | `src/notch.rs`, `src/notch_window.rs` | Listening, outcome, choice carousel, browse, rack band | Needs Needs-you, Paused, Thinking, Acting, Done, Error |
| Menu bar | `src/presence.rs` | Works, Leave, Next, Pod Bar, Open | No status, no pause, no quit |
| Scoped restore sheet | `src/scoped_restore.rs` | Single-resource restore with receipts | Not in the mockup |

No intro film, no onboarding overlay, no Activity/permissions screen, no website. **[V]**

## 2. Where audio goes

`mic → cpal → 16 kHz f32 in memory → openWakeWord ONNX (melspectrogram.onnx → embedding_model.onnx → hey_evoh.onnx, threshold 0.12) → whisper-rs / whisper.cpp on Metal (whisper-base.en.bin) → text` **[V]**

- All four models load **from beside the executable**, not from the network (`evo-voice/src/wake_model.rs`, `transcribe.rs`). **[V]**
- Audio is **never written to disk** in the product path; only dev binaries (`wake_probe`, `transcribe_dump`) take a file or dump. **[V]**
- No network call exists: `evo-desktop/tests/trust_constraints.rs` bans HTTP/TLS crates in every manifest, bans screen/OCR crates, keeps audio crates behind `voice-audio`/`voice-wake`, and scans source for sockets that are not loopback. **[V]**
- The wake buffer is in memory only; the recorded utterance lives in RAM until transcribed. **[V]**
- Voice is **optional at build time** (`voice-audio`, `voice-wake`); a default build has no microphone code, and `using-evo.md` says the shipped bundle does not carry the models. Whether the installed `Evo.app` has them is **[U]** — the agent runs, which implies they exist somewhere beside it.

## 3. Where text goes

`trigger text → local Unix socket (per-user temp dir, named by a hash of the storage root) → daemon → evo-threads engine → restoration` **[V]**

- Everything derived is local. On disk: `~/Library/Application Support/Evo/storage/` — `observation.log`, `artifact.log`, `ledger.db`, `daemon.status`, `pods/`, `preferences/` **[V]**. Plaintext, unencrypted, no retention limit **[V]**.
- **No account, no server, no sync, no analytics, no crash upload.** There is no HTTP client in the workspace to send with. **[V]**
- The optional `fastembed` embedding feature can reach the network for model assets; it is off by default and loads from a local directory when on. **[V]**

## 4. How the notch window is created

A separate **`NSPanel`**, built through the Objective-C runtime — no Swift, no NIB, no Tauri window (`src/notch_window.rs`) **[V]**:

- `objc_allocateClassPair(NSPanel, "EvoNotchPanel")` plus an `NSView` subclass `EvoNotchView` registered the same way, methods added with `class_addMethod` and called through `objc_msgSend`.
- `initWithContentRect:styleMask:backing:defer:` with styleMask `(1<<4)|(1<<7)|(1<<13)` = utility window | nonactivating panel | HUD window **[V]**.
- Level **27** (above the menu bar, not screensaver), `setFloatingPanel`, opaque off, shadow off, `hidesOnDeactivate` off, `movable` off, transparent `NSColor.clear` background **[V]**.
- Collection behaviour `1|16|64|256` = canJoinAllSpaces | stationary | ignoresCycle | fullScreenAuxiliary **[V]**.
- Canvas `CANVAS_SIZE = (380.0, 320.0)`, top-anchored, horizontally centred, `NSWindow` y computed from the screen top **[V]**.
- Content is painted into a bitmap with CoreText; the cutout is measured from `auxiliaryTopLeftArea`/`auxiliaryTopRightArea` and `safeAreaInsets.top`; Own vs Float is decided by an occupancy probe **[V]**.
- Created on the main thread via a blocking hop; the surface keeps its own thread **[V]**.

## 5. "Stays on this Mac" — true, with conditions

The claim is **true for this build**:

- Audio is transcribed on device (whisper.cpp) and the wake model runs in-process **[V]**.
- Text is matched on device and stored only under Application Support **[V]**.
- No HTTP/TLS client is present, and a test enforces that at the manifest and source level **[V]**.
- There is no account or server to send anything to **[V]**.

Conditions a reviewer must know:

1. **The claim is not on screen today.** The only trust line is the first-run "Your record lives on this Mac"; the mockup's "Stays on this Mac" chip does not exist yet **[V]**. Adding it is honest only while the points above hold.
2. **Model files may be missing** from the bundle, so voice can fail silently on a fresh install. The claim is about where processing happens, not about voice working **[U]**.
3. **`fastembed`** (optional) can download model assets; keep it out of default builds or reword the claim **[V]**.
4. **The plaintext log is local but readable** by any process running as the same user. "Stays on this Mac" stays true; "private on this Mac" would overclaim.

## 6. Proof of match (§9) — baseline

Measured, not eyeballed:

- Mockup: Chromium at 1100×806 (preview panel, DPR 2 downscaled), mockup rendered with its own JS animation.
- App: last real capture of Home, 1200×880 window downscaled to 1100×806.
- Comparison: per-pixel RGB, threshold 12/255 (no Playwright/pixelmatch installed; a pure-Python BMP compare was used instead).
- Result: **650,856 / 886,600 pixels differ = 73.41%**. Diff image: `docs/shots/diff-home-v3.png`. Mockup: `docs/shots/mockup-home-v3.png`.

Caveats: the mockup shot carries the browser's "Freebuff" badge in the top-left corner (tool chrome, not the design); two different source resolutions were scaled; reduced-motion emulation was not applied. Treat 73.41% as a floor-level baseline, not a verdict.

### Every difference found

**World**
1. No scene at all — no sky, moon, stars, treeline, grass, stone path, CRT, or the four work objects; only a wash and grain.
2. No object hit targets, no hover tag, no selection caret, no fireflies.
3. No scene modes (home/quiet/intro/empty), no dimmed veil behind panels.

**Header**
4. Missing leaf-mark logo tile; wordmark is bare serif text.
5. Missing listening chip with animated level bars (status is dot + words).
6. Missing "Stays on this Mac" chip.
7. Missing Scene/List segmented control.
8. Search is a bordered field without ⌘K hint; mockup is a chip-styled field with the shortcut and a search glyph.
9. Gear is a drawn outline at the far right without the 36px circular chip; header height 56 vs 66.
10. Traffic lights are native (mockup draws its own dots for preview only) — expected, not a defect.

**Left column**
11. Headline is one line at 48px; mockup is 56px, two lines, letter-spacing −0.015em, typed with a caret.
12. Missing the sub-line "Everything you were working on, in the order you left it."
13. Card is 24px radius but wrong width, wrong padding (32 vs 22), and has no "when" line, no serif 31px title, no source row with icon, no 46px pill, no 40px "…" button.
14. Continue is 34px tall; mockup is 46px.
15. Missing "Show 33 more in the list" as an underlined link.
16. Missing dashed "Memory, coming soon" and "Autopilot, coming soon" chips.
17. Missing trust footer: lock, "Evo only sees what you allow.", "See activity and permissions".

**Body**
18. App renders every work as a full-width row under the hero; mockup renders one card plus objects in the scene, list only in List view.
19. Rows use 24px radius and a shadow; mockup rows are 14px, flat, 38px leading icon.
20. No List view panel (560px, right side).

**Type and colour**
21. App body stack is SF via system face; mockup specifies Instrument Sans 400/500/600. Instrument Sans is not bundled.
22. App uses a 12.5px status line and a mono data role; mockup has no mono at all.
23. Accent, panel, line and text hexes differ from §2 (`#8fe0a8` etc. are not the current theme values).
24. Focus rings: mockup 2px accent at 3px offset; not implemented globally.

**Behaviour missing**
25. ⌘K palette, arrow-key selection, Enter to continue, hover tags on objects, Space to demo the notch.
26. Reduced-motion handling for the scene, typing and card arrival.
27. Notch demo (resting → listening → opening → done) is not wired to the app window.

## 7. Backend changes

None. Nothing in this document changes behaviour, storage or interfaces.

Open questions for you are in `QUESTIONS.md` (new, next session if you want): Instrument Sans bundling, whether the CRT pixel "evo" replaces the serif wordmark, and whether the List view replaces Home's rows entirely.

## Backend changes (S9-2)

Added for the web shell, each with tests:

| Change | Where | Test |
|---|---|---|
| `hide_work` — persisted in `ui-state.json`, undoable | `ui_state::UiState::hide/unhide` | `hiding_is_idempotent_and_undo_is_exact`, `names_hidden_works_and_pause_survive_a_round_trip` |
| Build info — `EVO_BUILD_TIME` stamped by `build.rs`, read by `get_about` | `build.rs`, `bridge::Backend::respond` | `replies_carry_the_request_id_and_a_stable_error_shape` |
| Versioned settings store — onboarding step, appearance, names, merges, pause | `ui_state.rs` (`version: 1`, atomic write) | `a_corrupt_or_future_file_is_refused`, round-trip test |
| Bridge command allowlist and validation | `bridge::parse` | `refuses_unknown_commands`, `refuses_bad_fields_and_extra_keys`, `refuses_oversized_messages_and_names` |
| Kill switch enforced in Rust, not the page | `bridge::Backend::continue_work` | `a_paused_app_refuses_to_continue` |
