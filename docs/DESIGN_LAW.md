# EVO DESIGN LAW
The source of truth for how Evo looks, moves and speaks. It overrides any visual instruction in PROJECT_BRIEF.md and EXPERIENCE.md. The security, privacy and reliability rules in PROJECT_BRIEF.md still apply in full.

## 0. The reference
design/mockup-home-v3.html is the exact target. Read it fully every session. Do not redesign it and do not "improve" it. If it conflicts with this file, the mockup wins on looks and this file wins on behavior. If you think the mockup is wrong, write the question in QUESTIONS.md and keep the mockup's version.
First gate: tell me whether the app window is a web view (Tauri or similar) or a native Rust UI. If native, stop and propose a plan. Do not rewrite the app without my approval.

## 1. The idea (do not dilute)
- Home is a place, not a list: a pixel-art clearing at dusk. A stone path leads to a CRT that says "evo". The user's recent work are objects on the path (book, paper, folder, notebook). The newest is largest and nearest. Older work recedes toward the CRT.
- Pixel art is the WORLD only. All interface (headlines, cards, chips, buttons, menus, lists, settings) is calm, grown-up serif plus sans. No pixel fonts in the UI. The only pixel text anywhere is "evo" on the CRT screen.
- The CRT is Evo. Click it or say "Hey Evo" and it listens.
- Every screen lives in the same world: the full scene, or the scene dimmed behind a panel. Never a flat background.
- List view always exists, as the fast and accessible equivalent of the scene.

## 2. Tokens (exact)
colors: ink #06120e; panel rgba(9,21,17,.9); line rgba(240,234,216,.13); text #f0ead8; muted #a9b8ab; faint #7f9283; accent #8fe0a8; on-accent #06180f. ONE accent. Amber and moon colors exist only inside the scene art.
type: Instrument Serif for h1 (56px/1.02, -0.015em) and card titles (31px/1.08). Instrument Sans 15px/1.45, weights 400/500/600, for everything else. Bundle both locally (OFL). No other fonts.
radius: card 24, menu 16, row 14, pill 99. Controls: primary button 46px tall, chips 34px, icon buttons 36px, segmented control as a pill.
motion: easing cubic-bezier(.2,.8,.2,1). Headline types in at 42ms per character. The main card arrives after 800ms (blur 8px to 0, rise 30px, 900ms). The notch expands in 450ms. The scene redraws at about 12fps. One orchestrated entrance per screen, not scattered effects.
reduced motion: no animation, the scene draws one still frame, the headline appears at once.

## 3. The scene (port it, do not reinvent it)
- Move the mockup's canvas code into modules: scene/stage, scene/sprites, scene/fireflies. Keep the 320x180 resolution, image-rendering: pixelated, object-fit: cover, the seeded random generator (seed 11), the palettes, the path function and the sprite shapes exactly.
- Scene modes: "home" (all objects, selection marker), "quiet" (no objects, dimmed by a veil, behind Settings, Help, Activity), "intro" (objects arrive and leave), "empty" (empty path, CRT glowing).
- Sprite by work type: web page or chat = book, document or file = paper, folder or project = folder, notes or anything else = notebook.
- Click targets over objects are real buttons (aria-label = work title) placed with the same cover-crop math as the mockup. Arrow keys move the selection and Enter continues.
- Performance: stop the loop whenever the window is hidden, minimized or unfocused. Idle CPU under 2%.

## 4. Components (build once, reuse everywhere)
Panel card, primary button, ghost button, icon button, chip, live status chip, segmented control, text field, toggle, select, menu, row, tag, toast, dialog, notch, empty state. Each needs default, hover, focus-visible (2px accent ring, 3px offset), active, disabled, loading and error states. They copy the mockup: dark panel, 1px line, soft long shadow.

## 5. The notch
Look and timing come from the mockup's .notch: resting pill, then expands to 460x68 with a waveform and live transcript, then "Opening [name]", then "Done", then collapses. The real notch is its own always-on-top window that reuses the same tokens and timings. States: resting, wake, listening, thinking (breathing, no spinner), acting, done, needs you (two buttons), error with retry, paused.

## 6. Trust cues (always visible)
The top bar always shows the live listening chip, the "Stays on this Mac" chip, the Scene/List toggle, search and settings. The footer link "See activity and permissions" opens the Activity screen. Only show a trust claim that is true. Phase 0 must verify "Stays on this Mac" and change the words if audio or text leaves the device.

## 7. Words
Plain sentences, sentence case, second person, buttons are verbs. No exclamation marks, no emoji, no internal terms (see GLOSSARY.md), no banned words from PROJECT_BRIEF.md. Errors say what happened and what to do. Empty states teach in one sentence with one action. All strings live in one copy file.

## 8. Hard bans
No pixel fonts in the UI. No second accent color. No decorative gradients, glass card grids, stat-card dashboards or sidebars. No fake data, testimonials or numbers. No fonts, colors, radii or shadows that are not in section 2. If you need one, ask in QUESTIONS.md.

## 9. Proof of match (you cannot see, so measure)
Use Playwright with reducedMotion "reduce", viewport 1100x806, deviceScaleFactor 1, and the same bundled fonts. Screenshot design/mockup-home-v3.html and the app's Home, diff them with pixelmatch, save the diff image in docs/shots/, and report the percentage of differing pixels. Start with a 3 percent threshold and explain every region that differs. Also report the font families actually used (exactly two), every color used and every radius used, and flag anything outside section 2.
For a screen with no design/mockup-<screen>.html, build it from section 4 components and the layout in the section prompt, and mark it "needs design review" in TODO.md. Do not invent new visual ideas.