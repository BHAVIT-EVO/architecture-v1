# Third-party notices

Evo ships and adapts work from other projects. This file records exactly what
came from where, so the notices travel with the software as their licences
require.

---

## Setpiece

- **Project:** Setpiece — <https://github.com/ChiFungHillmanChan/setpiece>
- **Licence:** MIT
- **Used by:** `crates/evo-pods/src/layout.rs` (the unified work-space layout
  core) and `crates/evo-pods/src/reveal.rs` (the reveal choreography)

Setpiece's `Sources/SceneCore/Layout` is the source of the model Evo's layout
core adapts: normalized `Slot` rects resolved inside a display's usable frame
(`Slot.absoluteRect(in:)`), the sticky-then-fill planning pass with a 10 pt
approximate match and overflow-to-minimize (`LayoutEngine.plan`), the split
tree with the `kind` / `ratio` / `first` / `second` JSON shape
(`LayoutNode`), the 5 % template floor (`LayoutTemplate`), and the ten
built-in presets (`PresetSeeds`). Evo's version is a re-expression in Rust
with its own region model; the names and structure are kept so a person who
knows Setpiece can read it.

The reveal timing model is adapted from Setpiece's `AnimationRunner` /
`FrameInterpolator`: a per-window duration scaled by `sqrt(distance / 600pt)`
and clamped to 70–140 %, the linear / cubic ease-out / damped
`1 − e^(−6t)·cos(8t)` easing set, and the rule that seven or more windows are
placed instantly. Evo's reachable thresholds and stagger values are its own
tokens; the timing grammar is Setpiece's.

The MIT terms that cover that work:

```
MIT License
Copyright (c) 2026 Hillman Chan
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:
The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## Sapphire

- **Project:** Sapphire — <https://github.com/cshariq/Sapphire>
- **Licence:** GNU AGPL-3.0
- **Used by:** nothing copied.

Sapphire's licence is AGPL-3.0, so no Sapphire code, resource or asset is
copied, ported or adapted anywhere in Evo. Its notch surface is **studied for
its interface grammar only** — measurable behaviour a person can observe: the
asymmetric notch silhouette and its quad-curve corners, the spring response
and damping values its transitions use, the fitted host margin, the expanded
content padding, the plane entrance geometry, and the sequence of states a
listening session moves through. Those observations are re-implemented from
scratch in Evo's own rendering stack and Evo's own design tokens
(`DESIGN.md`); they are ideas and measurements, not copied expression.
