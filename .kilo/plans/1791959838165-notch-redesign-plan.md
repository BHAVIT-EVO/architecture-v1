# Notch Redesign Plan — Evo Desktop

## Goal
Redesign the Evo notch surface to match the beautiful, native macOS notch behavior from the `boring.notch-main` reference. The notch should:
- Expand from the **actual hardware notch** (not center of screen)
- Have beautiful typography matching the DESIGN.md system (SF UI voice, proper hierarchy)
- Smooth, spring-based animations (response 0.5, damping 0.75)
- Proper chip/hover states with visual feedback
- Follow all DESIGN.md color tokens (caramel accent, teal voice, dark substrate)

---

## Current State Analysis

### What Works
- Hardware notch detection via `auxiliaryTopLeftArea` / `auxiliaryTopRightArea` ✅
- Priority-based content system (Listening > Outcome > Choice > Rack) ✅
- Voice event integration (WakeGesture → Listening → HeardCommand → Outcome) ✅
- Choice carousel with swipe/drag support ✅
- SurfaceDecision for coexistence with other notch apps ✅

### What's Broken
| Issue | Current | Reference (boring.notch) |
|-------|---------|-------------------------|
| **Anchor** | Centered on canvas `(CANVAS_SIZE.0 - notch_w) / 2.0` | Anchored to actual hardware notch `notch_x` |
| **Shape** | Custom `NotchShape` with fixed radii | Dynamic radii interpolating from square-top (notch) to rounded |
| **Typography** | CoreText with hardcoded sizes (14pt, 11pt, 10pt) | SF system fonts with proper hierarchy (Display/Data/UI voices) |
| **Colors** | Hardcoded RGBA in `surface_layouts()` | DESIGN.md tokens (caramel #C89B6B, teal #5B8A8F, ink #ECEAE6) |
| **Animation** | Single `expand` factor, cubic ease-out | Interactive spring (0.38/0.8), per-layer interpolation |
| **Chips** | Static dark gray, no hover state | Accent-aware, hover feedback, haptic support |
| **Text layout** | `bounded_transcript()` with rough char estimates | Proper line-breaking, marquee for long text |
| **Idle state** | Hidden (correct) | Hidden + "hello" animation on first launch |

---

## Root Cause: Canvas Centering

In `notch_window.rs:273`:
```rust
notch_x: (CANVAS_SIZE.0 - notch_w) / 2.0,  // WRONG — centers on canvas
```

Should be:
```rust
notch_x: geometry.canvas_notch_x(),  // CORRECT — uses actual hardware position
```

The `canvas_notch_x()` method already exists and converts screen coords to canvas coords. The `from_engine_with_geometry` constructor uses it correctly, but the default `from_engine_with_notch` doesn't.

---

## Implementation Plan

### Phase 1: Fix Anchor & Geometry (Critical)
- [ ] **1.1** Fix `RenderState::from_engine_with_notch_and_decision` to use `geometry.canvas_notch_x()` when geometry is available
- [ ] **1.2** Ensure `surface_main` always passes full `NotchGeometry` to `RenderState::from_engine_with_geometry`
- [ ] **1.3** Remove fallback centering logic in `surface_layouts` — trust the anchor
- [ ] **1.4** Verify: notch appears at hardware notch position on MacBook Air M2

### Phase 2: Typography System (Critical)
- [ ] **2.1** Define font roles matching DESIGN.md in `notch_window.rs`:
  - `UI_EYEBROW: 10.5pt semibold` (teal voice) — "LISTENING", "PICK A WORK"
  - `UI_TRANSCRIPT: 13.5pt regular` (light ink) — live partial
  - `UI_CHOICE_TITLE: 13.5pt regular` (light ink) — candidate title
  - `UI_CHOICE_SUBTITLE: 10.5pt semibold` (muted ink) — "work N of M · swipe or click"
  - `UI_OUTCOME_TITLE: 13.5pt regular` (light ink)
  - `UI_OUTCOME_DETAIL: 10.5pt regular` (muted ink)
  - `UI_CHIP: 11pt semibold` (dark ink on chip bg)
- [ ] **2.2** Replace hardcoded sizes in `draw_layer_glyphs` with font role constants
- [ ] **2.3** Use `CTFontCreateUIFontForLanguage` with proper weights (kCTFontUIFontSystem=0, kCTFontUIFontSystemBold=1, Emphasized=2)
- [ ] **2.4** Implement proper line-breaking in `bounded_transcript` using CoreText framesetter instead of char estimates

### Phase 3: Color System (Critical)
- [ ] **3.1** Extract DESIGN.md colors as constants in `notch_window.rs`:
  ```rust
  const INK_PRIMARY: (f64,f64,f64,f64) = (0.93, 0.92, 0.90, 1.0);  // #ECEAE6
  const INK_MUTED: (f64,f64,f64,f64) = (0.62, 0.60, 0.57, 1.0);
  const INK_RECEDED: (f64,f64,f64,f64) = (0.36, 0.38, 0.39, 1.0);
  const TEAL_VOICE: (f64,f64,f64,f64) = (0.357, 0.541, 0.561, 1.0); // #5B8A8F
  const CARAMEL: (f64,f64,f64,f64) = (0.784, 0.608, 0.420, 1.0);    // #C89B6B
  const CARAMEL_HOVER: (f64,f64,f64,f64) = (0.831, 0.667, 0.478, 1.0); // #D4AA7A
  const CHIP_BG: (f64,f64,f64,f64) = (0.055, 0.063, 0.070, 0.94);
  const CARD_BG: (f64,f64,f64,f64) = (0.0, 0.0, 0.0, 1.0);
  ```
- [ ] **3.2** Replace all hardcoded colors in `surface_layouts` and `draw_layer_glyphs` with these constants
- [ ] **3.3** Add hover/pressed state colors for chips (caramel for Restore, chip_bg for others)

### Phase 4: Shape & Animation (High)
- [ ] **4.1** Update `NotchShape` to match reference: square top (0 radius) → rounded bottom (24pt when expanded)
- [ ] **4.2** Interpolate `top_radius` from 0 → 18, `bottom_radius` from 8 → 24 during expansion
- [ ] **4.3** Use the existing `eased_expand` spring (response 0.5, damping 0.75) for all layer interpolation
- [ ] **4.4** Ensure `compress_to_expand` scales content proportionally so nothing escapes card mid-flight

### Phase 5: Chip & Interaction Polish (High)
- [ ] **5.1** Chip visual states:
  - **Default**: CHIP_BG fill, INK_RECEDED text
  - **Hover**: CHIP_BG lighter, INK_PRIMARY text
  - **Pressed**: CHIP_BG darker, INK_PRIMARY text
  - **Restore chip**: CARAMEL fill, ON_ACCENT_INK (#1A1206) text
  - **Restore hover**: CARAMEL_HOVER fill
- [ ] **5.2** Add hover tracking in `surface_main` → pass to `RenderState` → use in `surface_layouts`
- [ ] **5.3** Implement drag swipe on choice carousel (already exists, verify feel matches 24pt step)
- [ ] **5.4** Add haptic feedback on chip press / swipe step (via `NSSound` or `NSHapticFeedback`)

### Phase 6: Text Layout & Marquee (Medium)
- [ ] **6.1** Replace `bounded_transcript` char-estimation with CoreText `CTFramesetterSuggestFrameSizeWithConstraints` for accurate line-breaking
- [ ] **6.2** Implement marquee scrolling for long transcripts/titles (like `MarqueeText` in boring.notch)
- [ ] **6.3** Max 2 lines for transcript, 1 line for choice_title, auto-ellipsis
- [ ] **6.4** Proper vertical centering of glyphs in layer rects

### Phase 7: Idle & First Launch (Medium)
- [ ] **7.1** Keep idle truly hidden (no glyph, panel ordered out) — already correct
- [ ] **7.2** Add "hello" animation on first launch (boring.notch has `HelloAnimation`)
- [ ] **7.3** Rack live activity: show compact notch band only (already implemented)

### Phase 8: Verification & Testing (Required)
- [ ] **8.1** Run existing tests: `cargo test -p evo-desktop notch`
- [ ] **8.2** Visual verification: build evo-agent with `evo-notch` + `voice-wake`, say "hey evo"
- [ ] **8.3** Verify notch position: should expand from hardware notch, not center
- [ ] **8.4** Verify colors match DESIGN.md exactly
- [ ] **8.5** Verify animations feel springy (not linear/ease-out only)

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/evo-desktop/src/notch_window.rs` | All phases — main implementation |
| `crates/evo-desktop/src/notch.rs` | Minor: verify priority constants align with visual hierarchy |
| `crates/evo-desktop/src/app.rs` | Verify notch action handling still works |

---

## Design Decisions Needed

### Q1: Font Loading
The current code uses `CTFontCreateUIFontForLanguage` which loads system SF. DESIGN.md says "UI voice — system SF (unchanged loading path)". **Recommendation**: Keep current approach, just use correct size/weight constants.

### Q2: Marquee for Long Text
boring.notch uses a `MarqueeText` component for scrolling. **Recommendation**: Implement simple marquee for transcript/choice_title when text exceeds layer width. Start with ellipsis-only (Phase 6.2), add marquee if needed.

### Q3: Hover State in Rust Surface Thread
The surface thread runs off-main. Hover events come via `mouseEntered`/`mouseExited` on the NSView. **Recommendation**: Add `hovered_chip: Option<usize>` to `RenderState`, update via `SurfaceInput::Hover(usize)` from view callbacks.

### Q4: Animation Timing
boring.notch uses `response: 0.38, damping: 0.8` for interactive, `response: 0.42/0.45` for open/close. Current uses `response: 0.5, damping: 0.75`. **Recommendation**: Match reference: open 0.42/1.0, close 0.45/1.0, interactive 0.38/0.8.

---

## Validation Checklist

After implementation, verify:
- [ ] Notch appears at hardware notch position (not screen center)
- [ ] "hey evo" → listening surface expands smoothly from notch
- [ ] Transcript text renders in INK_PRIMARY (#ECEAE6), 13.5pt SF regular
- [ ] "LISTENING" eyebrow in TEAL_VOICE (#5B8A8F), 10.5pt semibold
- [ ] Chips have proper hover/pressed states
- [ ] Restore chip uses CARAMEL (#C89B6B) with ON_ACCENT_INK (#1A1206)
- [ ] Choice carousel: title 13.5pt, subtitle 10.5pt muted, pips centered
- [ ] Outcome card: caramel accent band, proper title/detail hierarchy
- [ ] Collapse animation mirrors expand (spring, not instant)
- [ ] No content escapes card bounds during animation
- [ ] Works when another notch app owns the strip (Float mode)
- [ ] All existing tests pass

---

## Out of Scope
- Main Evo window redesign (separate DESIGN.md work)
- Pod integration in notch
- Calendar/webcam in notch
- Settings UI for notch behavior

---

## Dependencies
- No new dependencies — uses existing `core-text`, `core-graphics`, `objc` crates
- Requires `evo-notch` feature enabled
- Works with or without `voice-audio`/`voice-wake`

---

## Rollout
1. Implement phases 1-4 (anchor, typography, colors, shape) — core visual fix
2. Implement phases 5-6 (chips, text layout) — polish
3. Implement phases 7-8 (idle, verification) — completeness
4. Test on MacBook Air M2 (target hardware)
5. Ship behind `evo-notch` feature flag (already exists)