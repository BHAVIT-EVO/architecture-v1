# ADR 0005 — A web reference implementation for design and proof

**Status:** accepted (S1–S7)

## Context

§9 of the design law requires a pixel diff against the mockup at 1100×806 with
bundled fonts. The native app cannot be screenshotted by the tooling available
here, and Playwright is not installed.

## Decision

Keep a web implementation in `ui/` that shares the tokens (`ui/tokens.css`),
the scene (`ui/scene/`) and the components with the mockup. Measure Home there
against the mockup image, and build the design-system gallery and the public
website (`site/`) on the same files.

## Consequences

- The measured proof is of the web layer, not of the native window. The native
  window's fidelity is asserted by the shared tokens and geometry, not measured.
- Two implementations of the same screen exist. The web layer is the reference;
  the native shell is the product. Changes to one are checked against the other.
- This is a deliberate duplication with a stated cost; it ends when the native
  window can be captured by the same harness.
