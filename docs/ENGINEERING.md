# Evo

> **Demo:** Evo's boundary service, running against a synthetic corpus —
> eight works, two claimed, six refused, and a verifier you can try to break.
> **[Read the demo document → DEMO.md](DEMO.md)** · or run it in two commands:
>
> ```bash
> cd exports/ship-happens && python3 evo_server.py serve   # localhost:8765
> ```

### A local-first work memory for macOS

Evo helps people return to meaningful work without reconstructing the context they
lost. It observes the relationships between applications, files, windows, and
activities, models them as bodies of work, and restores the right working
surface when it is time to continue.

Evo is designed around a simple product promise:

> **Continue your work, not the archaeology required to find it again.**

The project is intentionally local-first. The desktop shell, activity ledger,
work reconstruction, native restoration, voice interface, wake word, speech
transcription, and semantic matching are implemented as local components. No
cloud speech or intent API is required for the voice path.

> **Project status:** active development. Evo is currently a macOS-focused
> engineering build, not a finished public release. APIs, storage formats, and
> product surfaces may change.

## What Evo does

- **Remembers work as a unit** instead of treating every application window as
  unrelated activity.
- **Reconstructs context** from local observations, artifacts, threads, and
  workspace evidence.
- **Restores native applications** through macOS rather than trapping work inside
  a custom container.
- **Organizes multi-window work** into a coherent desktop arrangement.
- **Provides an always-on voice surface** with the local “Hey Evo” wake word,
  Whisper transcription, and local semantic intent matching.
- **Shows a native notch-style interaction surface** that can follow the active
  display and remain visible across applications and Spaces.
- **Keeps the core architecture testable** through separate Rust crates and
  deterministic domain contracts.

## How it works

Evo is a Cargo workspace with a layered architecture:

```text
macOS observation
        │
        ▼
artifact identity + activity ledger
        │
        ▼
work/thread formation and retrieval
        │
        ▼
restoration planning + native app launch
        │
        ▼
desktop shell, workspace arrangement, notch, and voice
```

The most important boundaries are:

| Layer | Responsibility |
| --- | --- |
| `evo-observation` | Local observation contracts and event language |
| `evo-artifact` | Artifact identity and lifecycle |
| `evo-ledger` | Deterministic local activity ledger with SQLite/FTS5 |
| `evo-threads` | Formation of bodies of work from activity evidence |
| `evo-retrieval` | Retrieval and meaning-based continuation |
| `evo-restoration` | Platform-neutral restoration planning |
| `evo-execution` | Native restoration execution |
| `evo-workspace` | Workspace state and arrangement models |
| `evo-daemon` | Background coordination and local runtime services |
| `evo-voice` | Microphone, wake word, Whisper, and local intent pipeline |
| `evo-desktop` | Native macOS shell, notch, voice agent, and user surface |

See the [architecture documentation](docs/architecture/ARCHITECTURE.md) and
[implementation playbook](docs/implementation/IMPLEMENTATION_PLAYBOOK.md) for
the deeper system model.

## Requirements

- macOS on Apple Silicon
- Xcode Command Line Tools
- Rust toolchain with Cargo
- A working local ONNX Runtime installation for voice-enabled builds
- macOS permissions appropriate to the features you enable:
  - Microphone for voice capture
  - Accessibility for observing and arranging application windows
  - Input Monitoring or related permissions when requested by macOS

The default build is passive and does not enable microphone capture. Voice is
opt-in at build time.

## Build Evo

Clone the repository and enter the project:

```sh
git clone https://github.com/BHAVIT-EVO/architecture-v1.git
cd architecture-v1
```

Build the standard macOS application:

```sh
bash scripts/build-app.sh
```

This produces and installs the canonical application at
`/Applications/Evo.app` by default. To build the voice-enabled application with
the notch and always-on agent:

```sh
EVO_VOICE=on bash scripts/build-app.sh
```

For a debug build:

```sh
PROFILE=dev EVO_VOICE=on bash scripts/build-app.sh
```

### ONNX Runtime provisioning

Voice builds use ONNX Runtime for the local acoustic wake-word models. The
build intentionally requires explicit provisioning so the runtime source is
clear and reproducible.

Use an external local runtime directory:

```sh
EVO_ORT_PROVISIONING=external \
ORT_LIB_PATH=/path/to/onnx-runtime \
ORT_PREFER_DYNAMIC_LINK=1 \
EVO_VOICE=on \
bash scripts/build-app.sh
```

Or provide a local bundled runtime and its notices:

```sh
EVO_ORT_PROVISIONING=bundled \
EVO_ORT_DYLIB=/path/to/libonnxruntime.dylib \
EVO_ORT_LICENSE=/path/to/LICENSE \
EVO_ORT_THIRD_PARTY_NOTICES=/path/to/ThirdPartyNotices.txt \
EVO_VOICE=on \
bash scripts/build-app.sh
```

Do not commit model files, ONNX Runtime binaries, app bundles, or machine-local
runtime paths. The repository intentionally excludes those artifacts.

## Enable always-on capture

After building a voice-enabled app, install the macOS LaunchAgents:

```sh
bash scripts/install-agents.sh
```

The installer uses `/Applications/Evo.app` by default. To use another bundle:

```sh
BUNDLE=/path/to/Evo.app bash scripts/install-agents.sh
```

To remove the installed agents:

```sh
bash scripts/install-agents.sh --remove
```

The always-on agent owns the voice/notch runtime. Do not start a second copy of
the daemon against the same storage root.

## Local voice pipeline

When enabled, the voice path runs locally:

1. The microphone stream is captured and resampled locally.
2. The acoustic wake-word models detect “Hey Evo”.
3. A native notch surface provides immediate listening feedback.
4. Whisper transcribes the bounded utterance locally.
5. Local embeddings and Evo's semantic layers match the request to an intent.
6. The execution and restoration layers perform the requested local action.

The voice build expects these runtime assets beside the installed agent:

- `whisper-base.en.bin`
- `melspectrogram.onnx`
- `embedding_model.onnx`
- `hey_evoh.onnx`

These assets are not stored in Git. The build script validates and copies them
from the local model/runtime locations used by the installation.

More detail is available in [the voice wake-word guide](docs/voice-wakeword.md)
and [the local model notes](docs/model-local.md).

## Development commands

Check the workspace without enabling voice:

```sh
cargo check --workspace --offline --locked
```

Check the desktop and voice integration:

```sh
cargo check -p evo-desktop \
  --features voice-wake,evo-notch,embeddings \
  --offline --locked
```

Run the workspace tests:

```sh
cargo test --workspace --offline --locked
```

Useful operational scripts:

```sh
bash scripts/evo-consolidation-checklist.sh
python3 scripts/evo-doctor.py
bash scripts/verify-live-capture.sh
```

Some checks require macOS permissions, a running app bundle, or locally
provisioned model/runtime assets. Treat failures from those checks as environment
signals rather than automatically as unit-test failures.

## Repository map

- `crates/` — Rust workspace and domain/runtime implementation
- `docs/foundation/` — product principles, cognitive model, and architectural laws
- `docs/architecture/` — system architecture and restoration model
- `docs/implementation/` — implementation contracts and feature specifications
- `docs/user/` — user-facing operating notes
- `docs/voice-wakeword.md` — local voice and wake-word details
- `scripts/` — build, install, verification, and model tooling
- `DESIGN.md` — visual and interaction design contract
- `THIRD-PARTY-NOTICES.md` — third-party software notices

## Privacy and data handling

Evo is built to keep its working data on the Mac. Local storage, activity
observations, voice models, transcription, and semantic matching are intended
to remain within the local runtime. The application still requires powerful
macOS permissions for features such as Accessibility and microphone capture.
Review and grant only the permissions needed for the capabilities you choose to
use.

This repository may contain research and engineering evidence in addition to
the active implementation. See the applicable file licenses and
`THIRD-PARTY-NOTICES.md` before redistributing derived or reference material.

## Contributing

Before opening a pull request:

1. Keep changes within the relevant crate or documented architectural boundary.
2. Add or update tests for behavior changes.
3. Run the smallest applicable `cargo check` and test command.
4. Do not commit secrets, local databases, model binaries, app bundles, logs,
   ONNX Runtime binaries, or machine-specific paths.
5. Update directly related documentation.

For product and architecture context, start with the
[constitution](docs/constitution/CONSTITUTION.md), [cognitive model](docs/foundation/COGNITIVE_MODEL.md),
and [architectural laws](docs/foundation/ARCHITECTURAL_LAWS.md).
