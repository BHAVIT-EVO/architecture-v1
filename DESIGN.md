# Design System — Evo "Control Room"

The design contract for evo-desktop's frontend. Every visual decision in
`theme.rs` and `ui.rs` implements this document; a screen that bypasses it is a
bug in that screen, not a matter of taste. The daemon, capture, persistence and
restoration semantics are explicitly out of scope: this contract governs
presentation only.

## Product Context

- **What this is:** Evo — a local, private witness of your work on macOS that
  remembers bodies of work and continues them (restore + pod entry).
- **Who it's for:** a solo builder running many parallel works; the app sits
  quietly in the background and is opened to decide "what do I continue?".
- **Space/industry:** personal work-memory desktop utility, macOS-first.
- **Project type:** native desktop app (egui/eframe 0.35, single window).

## Aesthetic Direction

- **Direction:** Control Room — a dark warm-charcoal studio, lit like hardware
  at night. Workspaces are tracks; Evo is the deck.
- **Decoration level:** intentional. The caramel accent does the expressive
  work; everything else is hairline edges, two shadow depths, and one banded
  vertical wash. No gradients on controls, no glow, no noise textures.
- **Mood:** calm, warm, precise. The user glances, picks a track, continues.
  Evo watches; it does not perform.
- **References:** the user-supplied dark music-studio theme (2 images + video,
  measured: 73–87% dark pixels; teal wash #4C6F75 family; caramel tiers
  #C8A372/#887055/#635240).

## Vocabulary (screens and surfaces)

- **The rack** — Home. Up to three track cards; overflow is one expanding
  stack row ("N more in the stack"). Never more than 3 cards on the deck.
- **The stage** — one opened body of work, zoomed to full screen. Entry is
  a card click; the detail screen's tabs keep their home in the detail
  screen until the stage redesign moves them into the dock.
- **The dock** — the persistent bottom control panel on every screen: the
  single primary action (Continue) with live step progress, Eject while a pod
  is entered, and Library when a stage is in focus. View tabs
  join when the stage ships.
- **Track card** — a workspace card: index, serif initial on a recessed
  tile, serif title, narrative summary, phase eyebrow (teal voice), and the
  data-voice age stamp. A card's identity is its engine thread; the card's
  Continue enters the pod, and the stage opens through the witnessed-
  membership bridge (`workspace_for_thread`) or Evo says honestly that the
  record has not yet linked them.
- **Marks** — round = superseding choice; square = accumulating set. Unchanged
  from the previous system.

## Typography

Three voices, one per job:

- **Display voice — Instrument Serif (static TTF, 400).** Work titles,
  screen headings, first-run headings, the app wordmark. Serif presence is the
  identity risk we took deliberately (approved D3).
- **Data voice — JetBrains Mono (static TTFs, 400/500).** Durations, counts,
  paths, ages, step counters. Tabular by nature.
- **UI voice — system SF (unchanged loading path).** All body, labels,
  buttons, captions. Weight still via variable-font instances (500/600/700).
- **Loading:** display + data faces ship as bundled static TTFs inside the
  binary (`include_bytes!`), registered as named families (`evo-display`,
  `evo-mono`); SFNS keeps the existing load-or-fallback path with the tested
  binding invariant (every named family must resolve to real font data).
- **Scale:** the eight-role ramp (sizes/tracking/leading) is unchanged:
  Display 30 Bold→now the serif face · Headline 22 · Title 16.5 · Lede 15 ·
  Body 13.5 · Caption 12 · Eyebrow 10.5. New: **Data 12.5 Mono** role for the
  data voice. Hierarchy stays size+weight together, never color alone.

## Color

- **Approach:** restrained — one warm accent on a cool substrate.
- **Environment:** `#15181A` → `#0F1214` (16-band top-lit wash; the only
  gradient in the app).
- **Raised (track cards, hero):** `#1B1F21` · **Recessed:** `#0D1012` ·
  **Well (inputs):** `#0B0D0F` · **Dock / action / sheet:** `#1E2224`.
- **Ink:** text `#ECEAE6` · muted `#A8AAA8` · tertiary `#8A8D8D` ·
  receded `#5C6062` (decoration only). Contrast: all information-bearing inks
  hold ≥4.5:1 on both raised and recessed surfaces; receded is never
  information.
- **Accent — caramel (approved D2):** primary `#C89B6B`, hover `#D4AA7A`,
  pressed `#B08757`, tint 14% alpha; on-accent ink `#1A1206` (≥7:1 on the
  accent). Caramel means *continue / declared choice* — never status.
- **Teal voice (approved D2):** `#5B8A8F` for phases, record evidence, and the
  informational ink; tint 16% alpha. Teal never means success (the semantic
  green does that).
- **Semantic:** success `#63B87F` · warning `#E0813F` · error `#E5484D` ·
  neutral `#8A8D8D` — meanings unchanged from the prior system; a rejection can
  still never borrow success's color. Status word tints stay alpha-based over
  their surface.
- **Dark mode:** dark **only** (approved D1). The light warm-paper palette was
  retired with this contract. Do not reintroduce a theme toggle without a new
  decision.

## Spacing

- **Base unit:** 4px; the S1–S8 scale is unchanged (4/8/12/16/24/32/48/64),
  with the M1–M6 i8 mirrors and the semantic gap aliases.
- **Density:** comfortable, identical to before; this re-theme changed color
  and voice, not rhythm.
- **Page margin** S6 (32) · content measure 700 · prose 600 · stack breakpoint
  760 — all unchanged.

## Layout

- **Approach:** zoom-as-navigation (approved D4/D5): rack → stage; the dock
  persists across both.
- **Grid:** rack cards: 3 across ≥1080px, 2 at 720–1080, 1 below. The stack
  row expands in place; work is never hidden behind a modal.
- **Max content width:** 700 (main) / 600 (prose), unchanged.
- **Border radius:** window 12 · surfaces 12 · dock 0 (top hairline only) ·
  controls 8 · chips/notices/marks 8. Card corner tightened from 16 → 12 to
  read denser, like the references.

## Motion

- **Approach:** minimal-functional; the same critically damped `settle()` —
  arrival 0.35s, departure 0.2s, touch 0.12s. No overshoot anywhere.
- **Easing:** `settle()` (cubic ease-out) for every transition; hover/press at
  RESPONSE_TOUCH.
- **Durations:** micro 120ms (hover/press/wash) · short 200ms (departures,
  dock crossfades) · medium 350ms (card→stage zoom, scale 0.97→1.0 with
  opacity; stack fan-out staggered ~40ms).
- **Rule:** leaving is never slower than arriving (tested).

## Control Language

One button exists: `ui::Control::{primary, secondary, quiet}`.

- **Primary (caramel fill):** the one action a screen asks for — Continue,
  Enter Evo. At most one per screen.
- **Secondary (raised fill, hairline):** real alternatives — Open System
  Settings, explicit mode changes.
- **Quiet (text until touched):** Settings, Done, Check again, dismissals.
- **Rules:** presses register on pointer-down; hover is animated; labels are
  measured so text is never clipped; rows reflow instead of truncating. No
  `egui::CollapsingHeader`, `ui.button`, `ui.small_button`, or
  `selectable_label` anywhere in Evo's own chrome — that is the bug class this
  contract exists to prevent. The pod bar inherits the same language; URLs
  leave Evo through the normal macOS opener rather than an embedded browser.
- **Copy:** pods are tracks in user-facing strings — "Continue" (was "Enter
  this pod →"), "Eject" (was "Leave pod"); phases are the engine's own
  witnessed phases (Researching / Building / Writing / Reviewing) in the
  teal voice — the reference theme's studio phases were aspirational and
  are not fabricated over the evidence.

## Reference Visual Grammar (pass 9 study)

Deep study of the user's two design references (player view + expanded-panels
view) and the 9.2s interaction video, measured with the pass-1 analyzer
(which found 73–87% dark pixels, teal `#4C7077` family, caramel tiers) plus
pass-9 motion forensics. This is the grammar the references actually speak;
Evo adopts the grammar, not the pixels.

### Evidence

- **Player view:** one artwork canvas fills the stage. All chrome floats on it
  as dark panels: top nav (back · project title · phase pills · user), a
  right-edge icon rail, three panel-cards docked at the bottom edge
  (Overview / Tracks / Lyrics — icon, label, expand glyph, nothing else),
  and the transport bar (view pills · mono timecode `00:00 / 03:06` ·
  transport buttons · A/B pair) over a hairline progress line.
- **Expanded view:** the same three panels grown into full-height columns
  (a fourth peeks from the right edge — horizontal continuation). Each column
  carries its own tab strip and dense data rows with mono durations
  (`3:08`, `4:05`) and status glyphs. The artwork recedes behind; the
  transport bar stays visible and unchanged.
- **Video (9.2s, 554 frames):** zero hard cuts (max scene-cut score 0.015).
  Sampled mean luma deltas: start→mid Δ47, mid→expanded Δ51, one moderate
  Δ17.7 transition, and end≈start Δ4.3 — an open-then-close round trip
  executed as continuous in-place motion, not page swaps.
- **Panel plane measured** at `#27292B`; artwork teal `#4C7077` dominant;
  substrate >90% black.

### Grammar principles

- **G1 — One hero, floating chrome.** The work is a single large canvas; UI
  surfaces float over it and never replace it. Identity lives in the canvas.
- **G2 — Panels are one object in two states.** A data surface is a collapsed
  strip (icon + label + glyph) that grows into a dense column. Same object,
  same position, one continuous motion between states — never a modal, never
  a navigation.
- **G3 — Nothing is modal; chrome persists.** Expanding dims the canvas, not
  the controls: the transport bar stays put through the expansion. Back is
  always one gesture away (top-left chevron).
- **G4 — Every control round-trips.** Each expand glyph has its collapse;
  the A/B pair is a symmetric toggle; phase pills are segmented with exactly
  one active. State is always visibly escapable.
- **G5 — Two voices: light UI, mono data.** Titles and labels in the light
  UI voice; every measured quantity (timecodes, durations, counts) in mono,
  tabular, right- or center-aligned.
- **G6 — Density scales with state, content does not swap.** Collapsed shows
  the label; expanded shows the table. The surface scales; the information
  architecture inside it is continuous.
- **G7 — Restraint as material.** >90% dark substrate, one accent family on
  canvas + active states, hairlines as the only separators, motion smooth
  with no overshoot.

### What Evo must NOT adopt

- **Backdrop blur / glass:** egui cannot cheaply blur a backdrop; Evo's raised
  fill + two shadow depths carry the same figure-ground job.
- **Waveform timeline:** Evo has no audio data; a fake waveform is dishonest
  furniture.
- **Horizontal 4th-panel peek:** Evo's works number ≤3 on the deck; the stack
  row already handles overflow honestly.
- **A/B mix toggles as literal A/B:** Evo's marks (round = supersede, square =
  accumulate) map to the *placement and symmetry* of that pair, not its
  semantics.

## Adoption Map — Ranked for one build pass

Each item is a concrete change nameable in the code, honest about what egui
can animate (height/position/opacity/inset per frame via `settle()`; no
blur, no per-pixel effects). Ranked by identity payoff per effort.

1. **Stage hero canvas** (`rack.rs` stage pass, `theme.rs` tokens).
   Opening a card zooms into a full-bleed canvas built from the work's own
   material: recessed→raised wash derived from the work's accent, the serif
   initial at display scale, the serif title floating on it (G1). The current
   flat detail screen's tabs move off the top and into the stage's panel
   strips (item 2). Zoom motion is the existing 350ms scale 0.97→1.0 + fade.
2. **Panel-cards on the stage: Overview / Activity / Record** (`rack.rs`,
   reuses `ui::Control` + `ui::caption/eyebrow`). Bottom-docked raised
   `#1B1F21` cards on the canvas: collapsed = eyebrow title + expand glyph
   only (G6); expanded = the same card grown upward to a tall column holding
   the existing tab content. Expansion animates height with `settle()` over
   350ms (egui-honest: allocate per frame), collapse at 200ms (leaving is
   never slower — contract already). Clicking elsewhere or the glyph
   collapses (G4 round-trip; the merge two-step already proved this
   arm/disarm pattern).
3. **Transport dock** (`dock.rs`). The dock adopts the transport grammar:
   phase pills as a segmented row in the teal voice (active = teal tint
   fill — teal is informational, never success), the live step counter set
   in Data 12.5 Mono (`12 / 31 steps`), Continue as the play-position
   primary (caramel fill unchanged). Dock persists across rack and stage
   (already true; this makes it look it).
4. **Card hover/press affordance** (`rack.rs`, `theme.rs`). Track cards get
   the reference's tile hover: tile inset lightens one step, hairline
   brightens, 120ms touch — press already registers on pointer-down. No
   scale (egui re-layout jitter); light only.
5. **Stack row joins the panel family** (`rack.rs`). The "N more in the
   stack" row adopts the panel-card visual (eyebrow + chevron glyph) so
   overflow reads as the same object family as stage panels (G2/G6).
6. **Marks placed like the A/B pair** (stage, `rack.rs` marks render).
   Round/square marks take the reference's symmetric right-edge toggle
   placement and mono labeling; semantics unchanged.

Not in this pass: window-level chrome (macOS traffic lights stay), any fake
data furniture, or a browser surface inside Evo. URLs use the normal macOS
opener instead.

## Decisions Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-09-27 | Initial design system created by /design-consultation from the user's theme references | measured palette/grammar from assets; user approved |
| 2026-09-27 | D1: dark only | references are 87% dark; a watcher app should sit quietly |
| 2026-09-27 | D2: caramel #C89B6B primary accent; teal #5B8A8F secondary voice | warm identity on cool substrate; unique in category |
| 2026-09-27 | D3: Instrument Serif display + JetBrains Mono data + SF body | album-cover titles; tabular data; zero-risk UI voice |
| 2026-09-27 | D4: click card opens the stage; dock owns Continue | one primary action per screen; dock shows live progress |
| 2026-09-27 | D5: 3 cards + expanding stack row | honest overflow, calm composition |
| 2026-09-27 | Amended: cards bind to engine threads; stage opens via witnessed-membership bridge | the engine's DisplayThread carries no WorkspaceId; membership-overlap matching is the same rule merge proposals were corrected to |
| 2026-09-27 | Amended: dock view tabs deferred to the stage redesign; phases are the engine's real phases | ship the rack/dock without inventing state; copy stays honest to the record |
| 2026-09-28 | Pass 11: panel strips read as instruments (painter-drawn 12px icons, no icon font); expanded panel grows over the identity from just below the nav (cap 480, ±12px breathing); Record carries honest Surface/Related/Witnessed tabs (teal underline, per-workspace temp state); strips arrive on a 40ms/index stagger; executing progress fills the dock's top hairline in teal; the stack row joins the strip family | make the stage *behave* like the reference (G2/G3/G6), not just resemble it; every new control round-trips and nothing fabricated was added |
