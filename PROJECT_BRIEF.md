# EVO: PROJECT BRIEF
(Re-read this file at the start of every session.)

## 1. Your role
You are a senior product designer, design engineer and Rust engineer in one. Evo's features and backend already work. Your job is to turn it into a complete, launch-ready product that feels handmade, calm and trustworthy. It must NOT feel like a dashboard, an admin tool, or AI-generated slop.

## 2. The product
Evo helps people get back into their work.
- Say "Hey Evo" (or press a hotkey): the notch at the top of the screen expands, listens, and opens what you asked for.
- Open the app to browse recent work and reopen anything in one click.
- Free to use. Built mostly in Rust. Two deliverables: the desktop app and a public website.

Coming next (design room for these, but do NOT fake them):
1. Personal work context: Evo learns what "your work" means, so people never re-explain context to any AI model.
2. Delegated autonomy: with explicit permission, AI can act on the user's work on its own, within limits the user sets.

Because of #2, trust is the product. The user must always be able to see what Evo hears, stores and does, and stop it instantly.

Users: anyone who works on a computer and loses time getting back into their work. Platform: detect in Phase 0. Assume macOS first (the notch); other OSes get a floating top-center pill.

Success means:
- A new user completes a first successful "Hey Evo" within 2 minutes of install, without reading docs.
- Every screen has one obvious next action.
- People show it to their friends.

## 3. Feeling and language
Feel: calm, warm, unhurried. A quiet clearing in a forest, a desk with the lamp on. Evo waits for you. It never nags, rushes or shouts. Working tagline: "Your work, ready when you are."

Language rules:
- Plain, short sentences. Second person. Sentence case.
- Buttons are verbs ("Open", "Try it", "Not now").
- Errors say what happened and what to do next. No blame, no "Oops".
- Empty states teach in one sentence and offer one action.
- Banned words: supercharge, unlock, seamless, leverage, revolutionary, magic, game-changer, "AI-powered".
- No exclamation marks and no emoji in the UI.
- Write ALL copy yourself in one voice. Keep every string in one central copy file so it is easy to edit and translate later.

## 4. Visual references (described in words)
1. Convex K1 (keyboard): pure black page. Headline in a light-grey dot-matrix/LED typeface. Small clean sans body text. Soft mint-green gradient pill button. The hero object (a keyboard overgrown with moss and daisies) bleeds off the right edge. Nature and tech merged.
2. Frobug (bug tracker): rainy, dark-green pixel-art swamp. A frog sits on an old CRT showing green terminal text. White clean-sans headline. Green pill primary button, white pill secondary. A white code card overlaps the scene, so real product UI sits inside the art.
3. Convex (travel): the view through a car's rear window onto a painted seaside pier under huge cumulus clouds. Dark vignette frame. Centered white headline, white pill button, logo strip at the bottom.
4. Finmain (finance): bright Ghibli-style blue sky with soft painted clouds. A traffic light glowing red is the metaphor. Large light-weight numerals and a segmented bar sit directly on the sky, so data is part of the scene. Tiny sparkles and fine diagonal hatching.
5. Relay (agent control): navy background with halftone/risograph grain. A hand holds strings made of tiny letters and puppeteers a voxel cube character on a hill. Editorial serif headline, one orange accent, stats strip at the bottom.
6. Evo's own banner: pixel-art forest clearing at dusk. A stone path leads to a CRT monitor with "evo" on its screen. An open book, paper, folder and notebook are scattered along the path. Chunky cream pixel headline: "Your work, ready when you are."

Principles to extract (do not copy any reference):
- Each brand lives in ONE illustrated world built around ONE metaphor object.
- Oversized, confident headlines in a distinctive display face, paired with a quiet, very readable UI sans.
- Restraint: lots of negative space, ONE accent color, pill-shaped buttons, ONE primary action per view.
- Texture and atmosphere (grain, glow, scanlines, soft light). Never flat and sterile.
- The product is part of the art: real UI (here, the notch and the recents list) appears inside the scene.
- Social proof appears only when it is real.

## 5. Evo's world and design system
The world is "The Clearing": a forest clearing with a stone path, a CRT with "evo" on it waiting at the end, and the user's work (book, paper, folder) resting along the path like things left where you put them down.
- Dusk (dark, default): the forest at dusk from reference 6. Forest green-black base, warm cream text.
- Day (light): the painted-sky meadow from references 3 and 4. Pale sky and cloud-white base, deep forest-green text.
- Auto follows the OS.
- ONE accent color: soft leaf-green (as in references 1 and 2). Pick final values and check WCAG AA contrast.
- Art lives in the margins: headers, onboarding, empty states, the website. Dense working surfaces (lists, settings) stay calm and quiet, with no art behind text.

Typography:
- ONE display face (pixel or dot-matrix) for big headlines and the wordmark ONLY. Never for body text.
- ONE UI sans, highly legible, minimum 14px in the app.
- Optionally ONE mono face for tiny labels.
- Open licence (OFL), self-hosted and bundled. The desktop app must work offline.
- Candidates, choose and justify in an ADR: Pixelify Sans, Silkscreen, DotGothic16, Departure Mono (display); Geist, Instrument Sans, Figtree, DM Sans (UI).

Motion: spring-like, 150 to 350ms, purposeful. Skeletons instead of spinners. Everything respects reduced-motion (swap motion for a simple crossfade).

## 6. Anti-slop rules (hard bans)
- No purple/blue-pink gradients, no gradient blobs, no glassmorphism card grids.
- No emoji or sparkle icons. Use one consistent icon set.
- No default component-library look (unstyled shadcn/Material).
- No stat-card dashboards, and no sidebar with 12 items.
- No generic "Welcome back, User!".
- No fake testimonials, logos, ratings, numbers, lorem ipsum, or placeholder data in production paths. Anything deferred goes in TODO.md with a reason.
- Never more than 2 font families plus 1 mono, and never more than 1 accent color.

## 7. The notch (the main experience)
States:
- Resting: invisible, or blends with the hardware notch.
- Wake: soft glow and a quick expand.
- Listening: expanded pill with live waveform and live transcript. Esc cancels.
- Thinking: a gentle breathing animation, never a spinner.
- Acting: "Opening [name]" with an icon.
- Done: a brief check, then it collapses.
- Needs you: a confirmation or permission request with two buttons.
- Error: calm message plus a retry.
- Paused: mic off.

Requirements:
- It responds visually within 100ms of the wake word.
- A clear mic indicator is visible whenever audio is being captured.
- A hotkey fallback and type-to-Evo, for accessibility and quiet rooms.
- If several matches are found, show up to 3 choices.
- If nothing matches, say so plainly and suggest what to try.
- Handle false wake-ups, noisy rooms, mic permission denied, mic device change, and offline.
- Confirm what Evo heard before any risky action.
- A single obvious "Pause Evo" control, also available from the tray and by hotkey.

## 8. The app
- First run (max 4 steps, skippable and resumable): Welcome, then permissions explained one at a time (why, and what happens if refused), then choose wake word/hotkey and test it live ("Say 'Hey Evo' now"), then a first win (open something recent). End with a small, calm celebration.
- Home: "Pick up where you left off". One large Continue card for the most recent work. Below it, recents grouped by Today / Yesterday / Earlier. Each item shows name, type, location, last touched, and a one-click Open. Search and command palette (Cmd/Ctrl+K). Pinned items. Hover actions: Reveal, Pin, Remove from recents. One hint line, e.g. "Try saying: Hey Evo, open my last project."
- Coming next: "Memory" and "Autopilot" appear as quiet, honest, non-clickable previews labeled "Coming soon". No fake buttons.
- Settings: General, Voice (wake word, sensitivity, mic, hotkey, language), Privacy and Trust, Appearance (Auto/Day/Dusk), Shortcuts, About and Updates.
- Help: searchable in-app guide, replay the tour, shortcut sheet, What's new, and "Report a problem" (shows a preview of exactly what will be sent and needs approval).
- Tray/menu-bar item: status, pause listening, open Evo, quit.
- Design every state: loading, empty, error, offline, permission-denied.

## 9. Trust, security and privacy
Principles: local-first, least privilege, off by default, and the user can see and stop everything.
- Phase 0 must document where audio and text go. If anything leaves the device, say so plainly in onboarding, Settings, and PRIVACY.md.
- The wake-word buffer stays in memory. Audio is never written to disk unless the user opts in.
- Build the permission model (data model and UI) now, even though autonomy is not built yet:
  - Scopes: folders, apps, sites.
  - Actions: open, read, write, run.
  - Modes: Ask every time / Ask for risky actions / Act within scope.
  - An Activity log (what, when, why, undoable where possible).
  - A global kill switch.
- Never build shell commands from unsanitized strings (use argument arrays). Canonicalize paths and block traversal. Validate every IPC/command input. Use parameterized queries. Never load remote content in a webview.
- If Tauri: minimal capabilities and strict CSP.
- Secrets go in the OS keychain. Logs contain no transcripts, file paths or personal data unless the user opts in.
- Code signing and notarization. Signed auto-updates that verify the signature. Validate deep links.
- Rust hygiene: cargo audit, cargo deny, clippy with -D warnings, review every unsafe block, no unwrap/expect in user-facing paths, typed errors.
- Accounts and servers: add NOTHING that isn't needed. If Phase 0 finds accounts, licensing, sync or a server, apply full auth hardening (verified email, single-use expiring reset tokens, rate limiting, session revoke, server-side authorization on every endpoint, audit log, CSRF/CORS/CSP headers). Otherwise stay local-first.
- Storage: document the local data location. Versioned, safe schema migrations. Export my data. "Delete everything" must work.
- Integrations: only ones that exist. Each needs connect/disconnect, plain-language scopes, keychain tokens, backoff, a health status, and graceful degradation.

## 10. Reliability and observability
- Panic hook and error boundaries. No silent crashes.
- Timeouts, retries with backoff, and an offline mode.
- If the voice engine fails, hotkey and typing still work.
- A Diagnostics screen (mic, permissions, voice engine, storage).
- Structured local logs with rotation (the tracing crate).
- Opt-in crash reports and opt-in anonymous analytics with a documented event list (no content, ever).
- Performance budgets: cold start under 2s, idle CPU under 2%, notch at 60fps. Measure the battery cost of always listening.

## 11. Website (separate folder, simple static site)
- Home: primary CTA above the fold ("Download Evo, it's free", OS-aware), a one-line value proposition, the hero scene with the real notch UI inside it, How it works (3 steps), features, Coming next (honest), FAQ, closing CTA.
- Pages: Download, Guide/Docs, Privacy and Trust (plain language), Changelog, About, Contact/feedback, Privacy Policy, Terms.
- System pages: on-brand 404 (a lost path in the forest, with links home, to download, and to the guide), 500/maintenance, and a Thank-you page after download (3 steps to your first "Hey Evo", a link to the guide, and a conversion event only if analytics consent was given).
- Internal links in the header, footer and body copy. Breadcrumbs on docs. sitemap.xml and robots.txt.
- SEO: unique titles and descriptions, canonical URLs, OG/Twitter images, SoftwareApplication structured data.
- Targets: Lighthouse 95+, LCP under 2.5s, CLS under 0.1, WCAG 2.2 AA, keyboard navigable, mobile-first, content works without JS, self-hosted fonts, responsive avif/webp images.
- Cookie banner only if cookies or analytics are actually used.

## 12. Assets
You cannot paint the art, so:
1. Build with original CSS/SVG where possible.
2. Write ASSETS.md with image-generation prompts in ONE locked style (16-bit pixel-art dusk forest for Dusk, painted-sky meadow for Day). One prompt per asset, with exact dimensions and file names: website hero (both worlds), app header art, onboarding scenes (x4), empty states, 404, thank-you, OG image, app icon, favicon set.
3. Wire correctly sized placeholders so I can drop the files in.
Also deliver wordmark/logo directions as SVG (3 options).

## 13. Working rules
- One phase per session. One screen or component at a time. Keep files under about 300 lines. Prefer small edits over rewriting whole files.
- Never change backend behavior or public interfaces without listing it under "Backend changes" in SPEC.md and justifying it.
- After every change run: cargo build, cargo clippy -- -D warnings, cargo test, plus the frontend's build/lint. Report the real output. If you cannot run something, say so. Never say "done" without running things.
- Small, well-named commits on a branch per phase.
- If unsure, write the question in QUESTIONS.md instead of guessing.
- SPEC.md is the source of truth. Each feature has acceptance criteria and a test.