# Using Evo

A local-first cognitive operating system that helps you continue work instead of reconstructing it. This page is for the **desktop shell** — what you open, how you get back to your work, and how the voice module is meant to behave.

## What you are running

The canonical installed app on this machine is:

- `/Applications/Evo.app` — the only app bundle that should be opened or used by launchd.
- Development binaries under `target/` are build inputs only, not runtime copies.

The installed app and development binaries can differ in:

- the exact Cargo.toml they were built from,
- the exact build time,
- and critically, whether the voice model files ship beside the binary.

For the everyday answer: build and open `/Applications/Evo.app`; do not open a development binary directly.

## The latest app, in one sentence

The build script installs the latest code into `/Applications/Evo.app` by default. Voice and notch are opt-in and are not part of the default installed bundle.

## How to open Evo

There are three ways once a build exists:

1. **From the Finder / Spotlight.** Launch `/Applications/Evo.app`, or search “Evo” in Spotlight and open it.
2. **From the terminal.**
   ```sh
   open /Applications/Evo.app
   ```
   Or, for the build you compiled yourself:
   ```sh
   open target/release/evo-desktop
   ```
   Note: `evo-desktop` is a plain binary, not an `.app` bundle, so `open` treats it as a document opener. If that does not behave like a normal app launch, run it directly:
   ```sh
   target/release/evo-desktop
   ```
3. **From the menu bar, after it has already run once.** Once Evo has launched, it installs a menu-bar item and a global hotkey. Even when the window is closed, the menu bar stays.

Evo’s window does not “quit” in the usual sense. Closing the window hides the app; the menu bar and global hotkey remain active. Reopening is what “Open Evo” does.

## How to use the app

On launch, Evo shows one of three things:

- **Your work list.** The rack with up to three cards, and an expanding “N more in the stack” row when there is more to show. Each card is one body of work.
- **A work detail screen.** When you open a card, Evo shows that body of work: what it is, where it continues from, and the one action it is asking for.
- **A first-run sequence.** If this is the first time, Evo walks through three short moments: what it remembers, that it all stays on this Mac, and the permission it needs.

The primary action is always the same shape: **Continue**. It is the one thing a screen asks for. The dock holds it on every screen that has one.

Navigation between works happens in three ways:

- **Click a card** on the home list.
- **The menu bar item.** It lists your recent works by recency, with the current one marked. Click a name to enter it, Leave to exit the current one, Next pod to cycle, or Open Evo to bring the window forward.
- **The global hotkey.** From anywhere on the Mac, press `⌘⇧E` to cycle works by recency, or `⌥1` through `⌥9` to jump directly to one of the first nine works. `⌥Space` opens the Pod Bar, the in-app work switcher.

The menu bar is the “open it even when Evo is not frontmost” path. The window can be hidden and still respond to the menu bar and hotkey, because those actions execute on the desktop directly rather than waiting for a frame to paint.

## The first-run and permission gate

The first launch shows three short steps:

1. Evo remembers where you were working.
2. It all stays on this Mac.
3. Give Evo the permission it needs.

Step 3 is the honest one. Evo watches which files, windows, and pages you use through macOS Accessibility. Without that permission it still records file saves, but it cannot witness which window you focused, and it says so rather than pretending otherwise.

There are exactly two controls on that step:

- **Open System Settings** — takes you to Privacy & Security → Accessibility so you can turn Evo on.
- **Check again** — asks Evo to look again after you have granted the permission.

Below those is **Enter Evo**, the one primary control on the step. It is deliberately below the permission actions: entering without the permission is allowed, but it is not the suggestion.

If you already granted Accessibility but Evo says “Blocked” or “Partial”, use **Check again**. The dialog alone does not grant the permission; the setting does.

## How the voice module works

The voice module is a separate background thread that owns the microphone, the wake detection, and the transcription. It is not a button inside the app. You do not “turn on voice” inside Evo. You build and run the app with the right features, the models are present, and then you speak.

The wake phrase is **“hey evo”**.

When the acoustic wake spotter hears it above the threshold, two things happen at once:

- The **notch surface** — the Dynamic Island–style strip at the top of the display — expands into a listening surface showing the live partial transcript.
- The **main window** shows a listening toast, if it is open.

When the command is captured and resolved, you get one of these outcomes:

- **Restored** — a work was resumed; a brief confirmation appears.
- **Ambiguous** — more than one work matched; a numbered panel appears and you pick by voice ordinal, click, or number key.
- **NotFound** — recent work was suggested but nothing clearly matched.
- **Failed** — an honest reason is shown.
- **NoCommand** — nothing was heard.
- **Brief** — a summary of recent work, spoken back.

The voice module is local. The mic is captured through `cpal`, the acoustic wake runs through an ONNX model in-process, and transcription uses a bundled whisper model. Nothing is sent anywhere to be processed.

### What you need for voice to actually fire

Two things have to be true at runtime:

1. **The app must be built with the voice features on.** That means `voice-wake` (which itself requires `voice-audio`), plus `evo-notch` if you want the notch surface to respond. In the code, those are Cargo features named `voice-audio`, `voice-wake`, and `evo-notch`. The binary you run must have been compiled with them. You can tell by checking for the voice and notch symbols in the binary — `NotchEngine`, `NotchEngine::voice_event`, `NotchEngine::show_live`, `notch_window::spawn`, `EvoNotchHost::render`, `VoiceSocketClient`, `AmbiguityPanel`, `CustomWakeSpotter`, `Transcriber`, `Mic`, `detect_wake`. If those symbols are present, the code is there. If they are absent, voice and notch were not compiled in.
2. **The model files must be where the binary can find them.** The code resolves the models beside the executable at runtime:
   - `whisper-base.en.bin` — transcription
   - `melspectrogram.onnx` — wake-feature frontend
   - `embedding_model.onnx` — wake-feature embedding
   - `hey_evoh.onnx` — the trained wake spotter

   The binary looks for them in the same directory as itself. If they are not there, the voice thread prints what failed and exits — which is exactly why an app that has the voice code compiled in can still be completely silent.

## The notch surface

Behind `evo-notch`, the notch surface is a small Dynamic Island–style strip that sits over the display’s notch area. It is a separate window from the main Evo window.

Its job is to be the visible voice surface:

- When idle, it shows a dark, quiet glyph.
- On a wake gesture, it expands into a listening surface with the live partial transcript.
- When the command resolves, it shows the outcome: an outcome card or the numbered choice carousel.
- After a short tenure, it returns to the idle glyph.

When another notch app owns the strip, the surface does not fight it. It becomes a floating card below the other app's surface instead. That decision is made by an occupancy probe, not by guessing.

The notch surface speaks the same voice events as the rest of the app. Its Restore and Close chips, drags, and carousel steps route through the same voice socket as spoken commands. So the notch is not a second, silent display — it is part of the same voice flow.

The wake phrase for the notch is the same as for the app: **“hey evo.”**

## How to operate it while you are in the middle of work

This is the “always on” path, and it has two layers.

### Layer 1 — Evo is running, window hidden or showing

When Evo is running with the voice features on and the models present, the mic/wake thread is alive. You can be in the middle of anything else. You do not need to switch to Evo. You say “hey evo”, and the notch lights up and/or the main window shows the listening toast. The command is captured and resolved through the daemon. You pick when there is ambiguity, by voice, click, or number key.

The window does not need focus for this. The voice thread is its own thread. The notch surface is its own window. The main window just receives the same events.

### Layer 2 — Evo is not running at all

There is a headless agent binary for exactly this: `evo-agent`. It is the always-on voice + notch layer with no main window. Its only UI is the notch surface. This is the binary meant to run as a login LaunchAgent so the voice module is available before you ever open the main Evo window.

The installer for that agent is `evo-agent-install`. It writes the launch-agent plist and calls `launchctl`. It is feature-independent — it only writes the plist and installs it. The agent it points at must of course have been built with the right features.

For “I am in the middle of work and I want Evo to hear me” this is the intended path: the LaunchAgent keeps the mic and wake surface alive even when you never opened the main app.

On this machine, the intended launch agents are two Evo-related entries:

- `com.evo.agent`
- `dev.evo.ledger`

`com.evo.agent` is the single owner/supervisor for the daemon when voice is
installed. `dev.evo.capture` is only a fallback for machines without the voice
agent and must not be loaded alongside it.

Whether `com.evo.agent` is the voice-capable agent and whether it is currently healthy is the thing to verify next, not something to assume. The plumbing is installed; the actual agent binary it points at and its live behavior are the remaining question.

## Expected behavior when everything is wired right

When the right binary is running with the right features and the models beside it:

- Launching Evo shows the work list or a work detail screen, depending on what exists.
- The menu bar shows up in the Mac menu bar. The hotkey `⌘⇧E` cycles works; `⌥1`–`⌥9` jump to specific works; `⌥Space` opens the Pod Bar.
- Closing the window hides Evo; the menu bar stays.
- Saying “hey evo” lights up the notch surface and the main window listening toast, if the main window is open.
- The notch shows the live partial while you speak, then the outcome or the numbered choice carousel.
- Ambiguity is resolved by voice ordinal, click, or number key.
- Everything stays local.

## What breaks the voice module, in practice

The most common failure is not a code problem. It is a missing-model problem.

If the binary has the voice code compiled in but the four model files are not beside it, the voice thread starts, tries to load the transcriber and the wake spotter, fails, prints what is missing, and exits. The app otherwise works. The notch and voice are simply absent at runtime.

The second failure is building without the right features. If the binary was compiled without `voice-audio` and `voice-wake`, there is no mic thread at all, and “hey evo” never gets a chance to fire. The app may look fine and still be voiceless.

The third failure is permissions. On macOS, the microphone is gated by TCC on the hosting process. If the host app does not have microphone permission, the mic open fails and the voice thread exits. Accessibility is a separate permission, needed to witness windows; microphone is a separate one, needed to hear.

## The honest open items

These are the things this document does not close, and that should be checked rather than assumed:

- Whether `/Applications/Evo.app` was built with the optional voice and notch features. The default canonical bundle is passive; enable those features explicitly at build time when they are wanted.
- Whether the installed app bundle ships the model files. It does not — there are no `.onnx` or `.bin` files inside it. That is the practical reason the installed app’s voice path cannot work as shipped.
- Whether `com.evo.agent` is the voice-capable agent and whether it is healthy right now. Verify its live process, daemon child, voice socket, and log rather than assuming the plist alone is sufficient.
- Whether the notch surface looks right on the actual M2 after a wake-word trigger. The paint path has been corrected and its pixel tests are green. That is strong evidence, but it is not an on-device screenshot from your machine.

## Related docs

- `docs/voice-wakeword.md` — the deeper voice reference, including the training recipe and the threshold calibration story.
- `docs/verification/PODS-MAC-CHECKLIST.md` — the Mac verification checklist.
- `docs/architecture/ARCHITECTURE.md` — the overall architecture.
- `DESIGN.md` — the visual contract for the desktop shell.
