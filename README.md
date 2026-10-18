# Evo

A local-first cognitive operating system that helps people continue work instead of reconstructing it.

## What it is

Evo watches your work on this Mac — the files, windows, and pages you use — and remembers bodies of work so you can pick up where you left off. It is local and private: everything stays on this machine.

The desktop shell is a native macOS window built with egui/eframe. The voice module is local too: microphone, wake word, and transcription all run in-process.

## What you probably want first

- **[Using Evo](docs/user/using-evo.md)** — the latest app, how to open it, how to use it, how the Hey Evo voice module works, and how to use it while you are in the middle of something else.
- **[Wake word: how “Hey Evo” works](docs/voice-wakeword.md)** — the deeper voice reference: the keyword-spot fallback, the trained acoustic spotter, the threshold calibration story, and the training recipe.

## Current status

Architecture frozen. The architecture consists of:

- Constitution
- Cognitive Model
- Architectural Laws
- Behavioral Trace
- RFC-0000 → RFC-0009

Implementation of Evo v2 begins from this architecture.

## Repo map

- `docs/foundation/` — the core ideas: architecture, cognitive model, laws, glossary.
- `docs/rfcs/` — the evidence and interaction contracts.
- `docs/architecture/` — the implementation architecture.
- `docs/implementation/` — the implementation playbook and the feature specs.
- `docs/design/` — product and UI design work.
- `docs/product/` — the product document.

## Building

Evo is a Cargo workspace. The desktop shell is `crates/evo-desktop`.

```sh
bash scripts/build-app.sh
```

The build script installs the canonical `/Applications/Evo.app` by default. The default bundle is passive and does not compile voice or notch; enable both explicitly with `EVO_VOICE=on bash scripts/build-app.sh`. Voice builds also compile the daemon's local semantic embeddings, and the five embedding assets plus four voice models are copied into the bundle/runtime install.

The voice module does not fire from code alone. If the binary has the voice code compiled in but the models are not in one of those runtime locations, the voice thread starts, fails to load the models, and exits — so the app is otherwise normal and completely silent.
