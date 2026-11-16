# AUDIT-2

## Context block
Same as AUDIT-1 (commit 49fae2a, macOS 26.5.2, Mac14,2, /Applications/Evo.app).

## Verdict
NOT READY for appearance completeness. Day theme is defined for egui surfaces but the web UI (home.html and friends) ships only Dusk tokens; Day is effectively a stub on the main Home surface. Language audit shows banned internal terms still present per GLOSSARY.md (nothing applied yet).

## Top problems
1. Web UI tokens.css defines only the Dusk palette; no Day palette or data-theme switching.
2. GLOSSARY.md explicitly says "Nothing here is applied yet"; internal terms (workspace, pod, daemon, artifact, etc.) still ship.
3. Brand audit: old sprout removed from top bar and favicon; Dock icon uses new .icns; menu-bar PNGs installed.
4. No automated accessibility/contrast verification run (could not test visually).

## Findings

### Appearance
F-APPEAR-01 [P1] Day theme incomplete for web UI
Symptom: Home surface has no Day token set; selecting Day in Settings cannot restyle the web UI.
Evidence: tokens.css has single :root palette; no [data-theme="day"] block; grep data-theme returned none in CSS.
Confidence: CONFIRMED.

F-APPEAR-02 [P2] egui surfaces do define Day palette
Evidence: theme.rs color(dusk, day) with day arrays; ThemeMode::Day handled.
Confidence: CONFIRMED.

### Language
F-LANG-01 [P1] Internal terms still on screen
Symptom: GLOSSARY.md lists on-screen internal terms and says nothing applied yet.
Evidence: GLOSSARY.md "Status: proposed for approval. Nothing here is applied yet."
Confidence: CONFIRMED.

F-LANG-02 [P2] Banned words check not automated
Evidence: no grep run across all UI strings; manual audit not performed.
Confidence: COULD NOT TEST.

### Accessibility
F-A11Y-01 [COULD NOT TEST] Keyboard/VoiceOver/contrast not measured
Reason: no automated harness; native app not scriptable in this environment.
Confidence: N/A.

### Brand
F-BRAND-01 [PASS] Logo replaced in top bar, About, Dock icon, menu bar
Evidence: home.html img evo-mark.svg; settings.js evo-lockup.svg; /Applications/Evo.app/Contents/Resources/AppIcon.icns; EvoTemplate.png installed.
Confidence: CONFIRMED.

F-BRAND-02 [PASS] No sprout glyph remains in repo (crates/site/docs)
Evidence: grep sprout returned no matches.
Confidence: CONFIRMED.

## Could not test
- Visual screenshots of every screen in Day/Dusk/Auto.
- WCAG contrast ratios (computed not measured).
- Reduced-motion, text zoom, VoiceOver.
- Language string dump and typo scan.