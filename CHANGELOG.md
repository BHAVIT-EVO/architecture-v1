# Changelog

Newest first. The website's copy of this file is `site/changelog.html`; keep
them in step.

## 0.1.0 — first release

### Home

- The scene: a pixel-art clearing at dusk, a stone path, four work objects,
  fireflies, and the CRT. 320×180, seeded, 12 fps, paused when hidden.
- One Continue card for the newest work; Scene and List views over the same
  set; arrow keys and Enter; hover previews with work names.
- Search with ⌘K, rename, merge, hide, and "This isn't work", all reversible.
- The intro (five captions, ~23 s, skippable) and a four-step onboarding that
  resumes where it stopped. Both replayable from Help.

### The notch

- Resting pill, wake, listening with the equalizer, thinking, acting, done,
  needs you, error with retry, paused.
- Up to three choices when several works match; a plain sentence when none do.
- Wake latency recorded; the panel sits above the menu bar without stealing
  focus.

### Trust

- Permissions in three scopes (folders, apps, sites) and three modes (ask every
  time, ask for risky actions, act within scope). Out-of-scope is refused in
  every mode.
- Activity with what, when and why, capped at 100, with one-shot undo.
- One kill switch, shared by the notch, the tray, Settings and the shortcut.
- Settings, Help, Diagnostics, and the tray menu. Every state is designed:
  loading (pale outlines, no spinner), empty, error, offline, permission denied.
- Data export and Delete everything.

### Privacy and security

- No account, no server, no analytics, no crash upload.
- `PRIVACY.md`, `SECURITY.md`, and the in-app Privacy and Trust screen.
- Paths are canonicalized and containment-checked; commands are argument
  arrays; scope and permission input is validated.
- `cargo audit` in CI; one known advisory left in bundled SQLite (see
  `SECURITY.md`).

### Website

- Home with the scene and a live notch demo, How it works, What it can do,
  Coming next, FAQ, and a closing call to action.
- Download, Thank-you, Guide, Privacy and trust, Changelog, About, Contact,
  Privacy Policy, Terms, and a 404 with one lost object on the path.
- `sitemap.xml`, `robots.txt`, structured data, self-hosted fonts, and content
  that reads without JavaScript.

### Known gaps

- The menu-bar menu still lacks status, Pause and Quit (web tray only).
- Idle CPU for the always-on listener is ~35 %, over the 2 % budget.
- Memory and Autopilot are not in this release.
- The download buttons point at the setup page; no installer is attached.
