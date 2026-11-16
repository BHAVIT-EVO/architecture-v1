# EVO EXPERIENCE SPEC
(Overrides PROJECT_BRIEF.md wherever they conflict. Re-read every session.)

## 0. What counts as done
A change that only touches fonts or colors is a failed delivery. Done means: the screens below exist, match the layouts described, and you have looked at a screenshot of each one and listed every mismatch. Take screenshots with `screencapture -x docs/shots/<name>.png` (macOS) or the webview's screenshot tool. If you cannot take or view screenshots, say so and ask me for one.

## 1. The stage
Evo lives on one painted backdrop ("the stage"), full-bleed behind every screen.
- Day: hand-painted meadow and sky (assets/stage-day.jpg).
- Dusk: pixel-art forest clearing (assets/stage-dusk.jpg).
- Until the art exists: a CSS placeholder (soft vertical gradient plus grain). Never a flat color.
- UI floats on the stage as cards: 24px radius, soft long shadow. Day = warm white, Dusk = deep green. Text never sits directly on the art.
- Cards are tilted in 3D only during the intro and transitions. In the working app they are flat and calm.

## 2. Motion language (from the reference film)
- Captions: one short sentence at a time, centered above the stage, typed in with a thin accent caret, replaced by the next with a soft fade-up.
- Cards enter from depth: scale 0.92 to 1, blur 8px to 0, perspective 1200px, tilt 4 to 8 degrees, spring settle.
- Scene changes: dissolve through a field of tiny monospace characters (canvas, about 600ms) or a soft wash. Never a hard cut.
- A fake cursor with a ripple ring shows a click.
- The last card flattens, scales to the window bounds and becomes the real Home screen (shared-element transition, no cut).
- Reduced motion: the film becomes 4 still frames with crossfades.

## 3. Words
Internal terms must never appear on screen. Before any UI work, write GLOSSARY.md: every internal term visible on screen today (pod, stage, record, members, inferred, stack, observed activity, episodes, and any you find), the plain word you propose, and what it means in the code. I approve it. Then use only the plain words. All strings live in one copy file.

## 4. Intro film (first launch only, about 25 seconds)
Any key or click skips it; a Skip button appears after 1 second. Replayable from Help.
1. Caption "Every day, you open dozens of things." Small cards with icons of apps and files Evo found on this Mac drift in.
2. Caption "Then you lose your place." The cards scatter and dissolve into the character field.
3. Caption "Just say it." The notch expands at the top, a waveform pulses: "Hey Evo."
4. Caption "Evo remembers where you were." Three of the user's real recent works fan in as tilted cards (name, one plain line, "5 days ago").
5. Caption "Pick up where you left off." The cursor clicks the first card.
6. The card flattens and becomes the Home window.
Real data only, from this machine, never sent anywhere. On a fresh install with no history, show three cards clearly labeled "Example".
Build the film as a standalone page first (intro/index.html) so it can be previewed in a browser and tuned, then embed it.

## 5. Onboarding (after the intro, first launch only, 4 steps, resumable)
Same stage, cards and captions.
1. "Hi. I'm Evo." One button: Get started.
2. Permissions, one per screen: why, what happens if you say no, Allow / Not now. Only ask for what is actually needed.
3. "Say 'Hey Evo'." Live waveform, success state when it hears you.
4. First win: pick a recent work, it opens, a soft light bloom, then Home.

## 6. Home (replace the current screen)
- Top bar: wordmark "evo" left. Right: a quiet status ("Listening for Hey Evo" with a small green dot, or "Paused"), a search field, and a gear icon. No all-caps badge.
- Headline: "Pick up where you left off." Display font, sentence case, max 48px, one line. Under it one muted line: "3 things from this week."
- One hero card for the most recent work. Left: a tile that feels like the work (app icon or thumbnail if available, else a colored plate with the initial). Right: title in the UI sans (28px, semi-bold), one plain sentence, "5 days ago", and ONE primary pill button: Continue.
- Rename, Restore, This isn't work, Hide and Merge with... live in a "..." menu on each card and row. No visible stack of text links.
- Below: the other recent works as calm rows (56px, one title line, one meta line, Continue on hover), grouped This week / Earlier. One "Show 34 more" at the end.
- Remove from the main screen: the paragraph about linking records, the "nothing is ranked" paragraph, and the bottom bar with the disabled Continue. If one matters, it becomes a small "i" tooltip or moves to Privacy and Trust.
- Display font only for the headline and wordmark. Work titles use the UI sans.
- Layout: 32px side margins, content max 960px, centered. Nothing flush to the window edge.

## 7. Everything after
Order: intro, onboarding, Home, notch, Settings, Help, empty and error states. Same stage and cards everywhere.