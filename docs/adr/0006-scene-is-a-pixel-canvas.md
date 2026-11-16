# ADR 0002 — The scene is a 320×180 pixel canvas

**Status:** accepted

## Context

The design's world is 16-bit pixel art: a dusk clearing, a stone path, four
work objects and a CRT. It has to look identical at any window size and stay
cheap enough to idle at 12 fps.

## Decision

Port the mockup's canvas code unchanged into `scene/{stage,sprites,fireflies}`:
320×180, seeded generator (seed 11), the same palettes, the same path function,
the same sprite shapes, upscaled with nearest-neighbour and centre-cropped
(`object-fit: cover` math). Four modes: `home`, `quiet`, `intro`, `empty`.
Repaint at 12 fps and stop whenever the window is hidden or unfocused.

## Consequences

- The UI never scales the art by a fraction; it covers and crops.
- Hit targets over objects are real buttons placed with the same crop math.
- One texture upload per frame, which is what makes the notch's paint budget
  comfortable (p50 2.2 ms).
