# ADR-0001: Evo's Clearing visual system

- **Status:** Accepted
- **Date:** 2026-06-02

## Context

The desktop's previous Control Room system was a single, cool charcoal palette with caramel accent and separate teal phase voice. Product direction now calls for a quieter Evo-specific Clearing identity, Dusk by default, optional Day/Auto appearance, and a single leaf-green accent. The renderer is egui and the project already bundles licensed Instrument Serif and JetBrains Mono assets.

## Decision

- Rename the desktop direction to **Evo's Clearing**.
- Default to Dusk; support Day and Auto. Persist the chosen setting under the app's local storage preferences directory. Auto follows egui's system-theme input, falling back to Dusk if absent.
- Use one leaf-green accent for primary actions, selected states, phases and success. Keep warning/error colors semantically distinct.
- Keep Instrument Serif for display and JetBrains Mono for data. Both are bundled and licensed under SIL OFL 1.1; system UI type remains preferred with existing fallback behavior.
- Retain shared spacing, control, and motion primitives. No decorative blur or simulated privacy behavior is introduced.

## Consequences

- Palette tokens are resolved at draw time because custom egui painting does not automatically follow theme styles.
- The desktop shell persists Dusk/Day/Auto. The separate notch agent currently shares the leaf-green accent but does not read this preference; do not imply its Dusk/Day background follows the shell selection.
- Dusk and Day need contrast and component tests; Auto depends on the platform/theme information supplied to egui.
- Existing domain behavior, stored evidence, and public daemon/voice interfaces are unchanged.
