# Wake word: how "Hey Evo" works and how we train it

## What exists today (no training required)

Voice is fully wired and local: mic (cpal) → `VoiceDriver` (evo-voice) →
voice socket → daemon resolve/restore. To run it:

```sh
cargo build --release -p evo-desktop --features voice-audio
# whisper-base.en.bin (141 MB) must sit next to the binary:
#   target/release/whisper-base.en.bin
```

Wake detection in this mode is the **keyword-spot fallback** in
[crates/evo-voice/src/wake.rs](../crates/evo-voice/src/wake.rs): every
`scan_interval` (3 s), whisper transcribes the last `scan_window` (12 s)
and `detect_wake` string-matches "hey evo" on the transcript. There is
nothing to train — detection quality is entirely whisper's.

**Cost:** whisper-base spends ~1 s of CPU per 12 s window, every 3 s.
The energy gate added to `VoiceDriver` (`speech_rms_threshold`,
default 0.006 RMS over 30 ms frames) skips transcription on silence, so
an idle room costs near zero; speech still pays full price.

## The production path: the trained model is integrated

The acoustic spotter ships behind `--features voice-wake`
(desktop) / `wake-model` (evo-voice), in
[crates/evo-voice/src/wake_model.rs](../crates/evo-voice/src/wake_model.rs).
It runs openWakeWord's exact streaming feed in-process via `ort`:

```text
16 kHz f32 → 1280-sample chunks → melspectrogram.onnx → spec/10 + 2
    → sliding 76-frame windows, stride 8 → embedding_model.onnx
    → last 16 rows [1, 16, 96] → hey_evoh.onnx → P(wake)
```

Validated against the real graphs: piper-synthesized "hey evo" scores
0.209 at the phrase position vs ~0.0009 on noise/silence/tone (×230);
the Rust spotter reproduces the reference score bit-for-bit on the same
audio class. Driver integration: `VoiceDriver::with_acoustic_wake` —
a score over threshold acts as a bare wake and the existing
command-capture window handles what follows; the whisper keyword-spot
scan stays as fallback for utterances the spotter misses. The driver
also emits `WakeGesture`/`Listening` events that surface in the UI
(and, under `--features evo-notch`, in the notch surface), and while a
bare-wake capture is open it transcribes a **live partial** (~1 s
cadence) used verbatim at window close — no second full pass.

To activate in development: place `hey_evoh.onnx`, `embedding_model.onnx`,
and `melspectrogram.onnx` next to the desktop binary (same directory as
`whisper-base.en.bin`; copies live in `crates/evo-voice/testdata/wake/`
for tests and `target/release/`). `scripts/build-app.sh` enables
`voice-wake,evo-notch,embeddings` and puts the four voice assets plus the five
embedding assets in `Contents/Resources`.
The runtime checks both the development/agent directory and that bundle
resource directory. Build with
  `cargo build --release -p evo-desktop --features voice-wake,evo-notch,embeddings` or
use the app build script.
Missing acoustic wake models are a normal configuration — the whisper
fallback runs and says exactly which files and directories were searched on
stderr. Missing Whisper is fatal to the voice thread and is reported with the
same searched paths.

For the headless login path, install `evo-agent` with its sibling
`evo-daemon` using `evo-agent-install`; the installer copies the four voice
models, the five embedding assets, and any dynamically linked ONNX Runtime
library into the stable `~/Library/Application Support/Evo` install. It sets
`EVO_EMBEDDING_MODEL_DIR` for the daemon and installs one `com.evo.agent`
LaunchAgent with `RunAtLoad` + `KeepAlive`. The agent supervises both its
daemon child and voice thread, retrying transient deaths with a backoff. If a
required worker, model, or runtime library is absent, installation fails with
an explicit diagnostic instead of leaving a microphone loop connected to a
dead socket.

## Threshold calibration — done, against this machine's voice

The original openWakeWord-typical 0.5 **could never fire**: the model
was trained on synthetic piper speech whose positives peak at 0.209,
and this person's live "Hey Evo" scored lower still. Live evidence
(wake_probe, 8 utterances): peaks 0.151 / 0.229 / 0.328, noise floor
0.013–0.052. Shipped default is now **0.12**; tune without a rebuild
via `EVO_WAKE_THRESHOLD`.

**Measure your own distribution first** (mic permission must be
granted to the host app — macOS TCC gates the mic on the *hosting
process*):

```sh
cargo run --release -p evo-voice --features "audio wake-model" \
  --bin wake_probe -- --mic -s 60 --dump /tmp/evo_mic.f32
# say "hey evo" several times; watch WAKE lines and window-best scores.
# Then check what whisper actually heard:
cargo run --release -p evo-voice --features audio \
  --bin transcribe_dump -- /tmp/evo_mic.f32 target/release/whisper-base.en.bin
```

Pick a threshold ~2× your noise floor and below the weakest
"hey evo" peak. `wake_probe --selftest` verifies the model rig
headless (models load, silence + a tone score below threshold).

## Mic capture note (48 kHz Macs)

Modern Mac inputs run at 48 kHz. The cpal callback copies raw hardware chunks
into a bounded queue, and a worker resamples them to 16 kHz with
`LinearResampler` in [crates/evo-voice/src/audio.rs](../crates/evo-voice/src/audio.rs).
This keeps resampling and allocation off the real-time audio callback while
making overload visible as a dropped-chunk metric. The old hard-fail ("mic runs
at 48000Hz") killed the whole voice thread at startup on every modern laptop.

## Safe restore choices

Semantic matching selects a body of work, not an arbitrary window. If the
selected work contains more than one distinct, directly witnessed surface
occurrence for a restorable resource, Evo refuses to guess. It returns those
occurrences as numbered choices in the desktop/notch UI; clicking, pressing a
number, or saying “the second one” sends an opaque local selection key back to
the daemon. The daemon replays its current snapshot and verifies that key
before binding the restore target to the exact witnessed surface. Malformed,
stale, or identity-free evidence still fails closed and never falls back to a
URL, path, title, or “most recent” guess.

## The notch surface (Dynamic-Island style)

Behind `--features evo-notch` + `EVO_NOTCH_MODE`
(hidden|auto|float|own, default auto): on a wake gesture the notch
expands to a listening surface with the live partial transcript, an
outcome card follows, then the rack/idle glyph returns. When another
notch app owns the strip the surface renders as a floating card below
it instead (CGWindowList occupancy probe; decision logic in
[crates/evo-desktop/src/notch.rs](../crates/evo-desktop/src/notch.rs),
fully tested; window layer in `notch_window.rs`). Notch actions route
through the same voice socket as spoken commands.

The older plan text below remains for the training recipe and context.

### Training recipe

Free training options (all export ONNX, all no-cost):

- **Official Colab notebook** (recommended): dscripka's "Training your
  own openWakeWord models" —
  <https://colab.research.google.com/drive/1q1oe2zOyZp7UsB3jJiQ1IFn8z5YfjwEb>
  Free T4 GPU, ~15–20 min end to end; the notebook downloads all
  datasets itself. Known issue: the notebook's dependency pins can lag
  (openWakeWord#331) — if pip resolution fails, pin the versions it
  asks for. Save the output to Drive so a session reset doesn't lose
  the model.
- **openwakeword.com/train** — a free web trainer (20+ languages, no
  ML setup), exports ONNX. Easiest; less control over training data.
- **Kaggle Notebooks** (kaggle.com/code) — free weekly GPU quota; run
  the repo's `notebooks/training_models.ipynb` there if Colab's free
  tier is throttled.

Then, on the GPU runtime:

1. **Positives** — generate thousands of synthetic "hey evo" utterances
   with openWakeWord's `piper-sample-generator` (TTS + augmentation:
   noise, pitch, room impulse response — the notebook does this).
   Optional: record 30+ real samples of your own voice and mix them in
   for speaker-specific sensitivity.
2. **Negatives** — hours of generic speech/silence *without* the phrase
   (FSD50K, Mozilla Common Voice). This is what stops the model firing
   on podcasts, calls, and background conversation.
3. **Train** — ~15 min on the free T4, exports a ~1 MB `.onnx`.
4. **Verify** — the notebook reports false-positive rate on held-out
   negatives. Target: zero FPs over several hours of negative audio at
   the chosen threshold (~0.5 typical).

### Integration sketch (Rust side)

- Add `ort` (ONNX Runtime) behind a `wake-model` feature of evo-voice.
- New module `wake_model.rs`: load the `.onnx`, run on 16 kHz audio in
  sliding ~1.32 s frames (the model's native window, 16 kHz × 1.32 s),
  emit `Detected { command: "" }` when score ≥ threshold — feeding the
  driver's existing bare-wake capture unchanged.
- Keep whisper loaded lazily: it is only invoked after the wake fires,
  to transcribe the captured command. Idle cost drops from "whisper on
  every speech window" to "a 1 MB MLP per frame".

### Why not Picovoice Porcupine

Porcupine trains a custom keyword in their console with a Rust SDK, but
training happens on Picovoice's cloud and the runtime is proprietary —
against the local-first law. openWakeWord is fully open, trains
anywhere, and runs in-process.

## Honest limitations

- The current text fallback's false-negative rate on accents/mumble is
  whisper's, and there is no metric on it yet (would need recorded
  sessions).
- The VAD gate is energy-based: loud non-speech (music, fans) still
  triggers transcriptions. It bounds the idle cost, not the worst case.
- No trained model exists in-tree yet; this doc is the recipe, not a
  shipped artifact.
