# QUESTIONS

Blocking questions only. Each stops a phase from being built correctly if it is
answered differently than assumed. Everything else is decided by the documents
and recorded in `SPEC.md` or `GLOSSARY.md`.

At most eight. Answer in any order; each states the assumption I will proceed
with if it is not answered.

---

## Q1. The working tree does not compile. How do you want it repaired?

**Blocks:** everything. No session can build, screenshot or verify until this is
settled.

Real output from `cargo check -p evo-desktop --all-targets` on rustc 1.94.0:

```
error: could not compile `evo-execution` (lib) due to 28 previous errors
EXIT=101
```

All 28 are `error[E0133]: call to unsafe function ... requires unsafe block`, all
in `crates/evo-execution/src/room_macos.rs` (Objective-C/CoreFoundation calls).
The cause is not the toolchain: that file has an **uncommitted edit** that
removed the `unsafe fn` markers (25 `unsafe` occurrences at `HEAD` → 11 in the
working copy) without wrapping the raw FFI calls in `unsafe { }` blocks. Two
other files in the same crate are also modified (`lib.rs`, `room.rs`). The app
running on this machine is a previously built bundle, not this checkout.

- **A.** Restore those three files to `HEAD` (`git checkout -- crates/evo-execution/src/{lib,room,room_macos}.rs`). Fastest, discards the unfinished cleanup. I will confirm the build before touching anything else.
- **B.** Finish the cleanup properly: keep the functions safe and add `unsafe { }` blocks around the FFI calls (28 sites). Better code, a real change to a crate I have not otherwise audited.
- **C.** Leave it; you fix it and tell me when it builds.

**Assumption if unanswered:** A, and I will say so in the session report.

---

## Q2. Which Evo is the deliverable — with voice, or without?

**Blocks:** the intro film, onboarding, and every voice claim on screen.

The bundle is voice-optional: `EVO_VOICE=off` ships a passive app with no mic,
no notch, no `evo-agent`. `using-evo.md` says the installed bundle has voice off
and no model files, yet this machine is running `evo-agent` and `evo-daemon` with
`voice=active`. The brief promises "Hey Evo" within two minutes of install, and
EXPERIENCE §5 requires a live "Say Hey Evo" test.

- **A.** The deliverable is the voice + notch build (`EVO_VOICE=on`, models shipped); onboarding may ask for the microphone and test the wake word.
- **B.** Voice-capable, but models are provisioned separately; onboarding detects absence and says so honestly.
- **C.** The passive build is the deliverable for now; every voice surface is labelled unavailable.

**Assumption if unanswered:** A.

---

## Q3. Which answer to "what is my work" is the product?

**Blocks:** Home, voice resolution, and search at once.

Two grouping implementations run on the same log: `evo-threads` (what Home
renders and what the voice socket resolves against) and `evo-ledger-engine`'s
intervals in `ledger.db` (rebuilt every 10 s, FTS5, not read by the desktop).
They can disagree, and a person meets the disagreement as Evo contradicting
itself.

- **A.** `evo-threads` is canonical; the ledger engine leaves the shipped app and stays a development instrument.
- **B.** Both stay, but the ledger becomes a search index built from thread output instead of its own grouping.
- **C.** Both stay as they are; I document the difference and hide it.

**Assumption if unanswered:** A — one product, one answer.

---

## Q4. Display face: pixel, or keep the serif?

**Blocks:** Phase 1 (type setup), and every later screenshot through it.

`EXPERIENCE.md` §6 confines the display face to the headline and wordmark;
`PROJECT_BRIEF.md` §5 requires a pixel or dot-matrix display face and lists
candidates. ADR-0001 chose Instrument Serif, and `assets/fonts` ships it.

- **A.** Pixelify Sans (OFL) for display + wordmark; system SF for UI; JetBrains Mono for small labels. Retires Instrument Serif.
- **B.** Keep Instrument Serif — the headline will read editorial and the brief's typography requirement stays unmet.
- **C.** A different pixel face from the brief's list (Silkscreen, DotGothic16, Departure Mono).

**Assumption if unanswered:** A.

---

## Q5. May real work titles and paths appear in screenshots?

**Blocks:** the evidence for Phases 1–5.

You asked for real data. Real data on this Mac means real window titles, URLs
and file paths — the most sensitive thing Evo stores — appearing in screenshots.
The app cannot redact them.

- **A.** Yes — screenshots stay in this chat, and I review each before saving.
- **B.** Use a curated copy of the store (`EVO_STORAGE_ROOT`) with a subset you are comfortable showing.
- **C.** Use the live store but label anything not real **Example**, and omit titles in anything that would leave the machine.

**Assumption if unanswered:** A for local screenshots, C for anything published.

---

## Q6. How far does the trust surface go in this pass?

**Blocks:** Phase 5 scope, and what Home may promise.

The brief requires the permission model (scopes/actions/modes), the activity log
and a global kill switch to exist now, even though autonomy is not built. None
exist, and faking them is banned.

- **A.** Build all three for real (data model + UI + tests) in Phase 5.
- **B.** Build Pause Evo for real now; show the permission model and activity log as honest "Coming soon" previews.
- **C.** Build the activity log now (derivable from existing data) and defer the rest.

**Assumption if unanswered:** A.

---

## Q7. The website: where does it live, and what content can I use?

**Blocks:** Phase 6 (in the brief, absent from the repository).

There is no website in this repository. The brief describes a static site with
eight pages, three system pages, SEO and performance targets.

- **A.** A new `site/` folder here, plain static HTML/CSS with no build step, deployable anywhere.
- **B.** The same, in a separate repository.
- **C.** A framework-based site (needs a stack and a host chosen).

I also need: the real download URL (`Evo.app` is not distributed anywhere yet),
whether a demo video or GIF exists, and whether the FAQ answers are mine to write.

**Assumption if unanswered:** A, with copy in Evo's voice and no invented proof.

---

## Q8. Release: is a signed, notarized build in scope, and do you have the credentials?

**Blocks:** Phase 6's final criterion and anything claiming "launch ready".

The app is signed with a self-signed login-keychain certificate so TCC grants
survive rebuilds. There is no Developer ID, no notarization, no updater and no
version display.

- **A.** Yes — Developer ID Application certificate plus an Apple ID and app-specific password are available; build signing, notarization and a signed updater.
- **B.** Not yet — keep the self-signed identity and document the release steps in `docs/RELEASE-CHECKLIST.md` without executing them.
- **C.** Signing only; no updater in this pass.

**Assumption if unanswered:** B, so nothing claims to be distribution-ready.

---

## S1. Four colours the mockup uses that §2 does not name

**Blocks:** nothing; the foundation is built and the values are isolated.

These literals come straight from the mockup and have no token of their own:
menu fill `#10241c`, notch fill `#020504`, chip/field scrim `rgba(9,21,17,.74)`,
and the error state, which has no colour in §2 at all (it currently strengthens
the line to `rgba(240,234,216,.4)` rather than inventing a red).

- **A.** Keep them as they are — the mockup wins on looks, and the error state stays colourless.
- **B.** Promote each to a named token in `ui/tokens.css` with the same value.
- **C.** Drop to the eight §2 colours and accept a flatter menu, notch and chip.

**Assumption if unanswered:** A, with the four values written down in this note.

## S7 website

1. **Translucent chrome.** The sticky site header uses `rgba(6, 18, 14, .72)` and the hero's shade uses ink at four alphas. Neither is a §2 token. The app's `.veil` already does the same thing; should these become a documented "ink at alpha" ramp in §2, or be replaced by a solid panel?
2. **Small radii.** The site's logo tile (9px) and keycap chips (8px) sit below §2's smallest radius (row 14). May a 8/9px "small" radius join the scale?
3. **Error tint.** `#e8b4a0` marks a failed microphone test in Diagnostics. It is not in §2 and not the amber used inside the scene art. Keep it, or use accent plus words only?
4. **Domain.** Canonical URLs, sitemap and OG use `https://evo.app`. Confirm the real domain before launch.
