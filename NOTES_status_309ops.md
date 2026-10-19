# Notes: DynamicNotch reference vs Evo rebuild plan

## Status: writing report

Gathered enough evidence to finish the two-part report. Key caveated items:

- DynamicNotch shipped palette/typography are semantic-first: pure black background, white primary text, gray secondary, accent = system/user accent (`.effectiveAccent` / `controlAccentColor`), SF Symbols for icons, SwiftUI text styles (`.headline`/`.body`/`.caption`) rather than a fixed custom font or fixed point-scale, and album-art-derived accent in some media UI. No fixed teal/gray hex documented in README/design notes. So if the user expects a specific hardcoded teal from the reference, I cannot defend that from this repo's public docs; the reference appears to lean on semantic colors + system accent.
- The user's screenshot of the reference README gallery suggests the reference *visually* uses a darker card with lighter text plus colored accents per feature, which is consistent with black card + white/gray text + semantic accent. I can describe that as the observed visual language without claiming a specific hex that I did not confirm from source.
- Animation values I can cite from the Swift sources I read: balanced preset = spring(response 0.50, dampingFraction 0.75) with hideShowDelay 0.32 and queuePacingDelay 0.10; the codebase notes mention slightly different springs in another file (open 0.42/0.8, close 0.45/1.0) so the repo is not perfectly uniform. I will describe the reference as “spring-based, response ~0.4-0.5, damping ~0.75-0.8” and note the exact balanced preset values from `NotchAnimations.swift`.
- Window/stroke lift mechanics I have directly:
  - NotchBackgroundSurface: strokeColor overlay with strokeWidth 2.0 default (configurable via settings)
  - NotchSurfaceContainerView: dynamicIslandMaskShape + NotchShape mask, stroke driven by settings.notchStrokeOpacity and displayedContent?.strokeColor ?? cachedStrokeColor, and a black fill backstop for Dynamic Island base
  - NotchShape path: top corner radius + bottom corner radius, quad-curve construction at all four corners
  - DynamicIslandShape: single corner radius, addRoundedRect, clamped to half min dimension

## Remaining uncertainty

- I have not found an explicit “idle glyph” behavior in the reference app’s public sources I read — the reference seems to mostly show black + content or nothing. That affects how strongly I can claim Evo’s idle mark is “matching the reference.” I will frame it as Evo’s own choice that fits the reference’s visual language, not as a byte-for-byte port of a reference idle glyph behavior.

## Next step

Write the report, keep PART B changes concrete and scoped to notch.rs / notch_window.rs visual+lift changes, no new voice plumbing.
