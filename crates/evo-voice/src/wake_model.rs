//! Acoustic wake-word spotting with the person's trained openWakeWord model.
//!
//! The feed is exactly what openWakeWord's own streaming path does
//! (validated against the real graphs: 0.209 on a piper-synthesized
//! "hey evo" vs 0.0009 on noise/silence/tone):
//!
//! ```text
//! 16 kHz f32 → 1280-sample chunks → melspectrogram.onnx [1, T, 32]
//!     → spec/10 + 2 → sliding 76-frame windows, stride 8 →
//!     speech_embedding.onnx → (K, 96) feature rows →
//!     last 16 rows as [1, 16, 96] → hey_evoh.onnx → P(wake) ∈ [0, 1]
//! ```
//!
//! Streaming state mirrors `openwakeword.utils.AudioFeatures`: a
//! remainder buffer aligned to 1280-sample boundaries, a mel-frame ring,
//! and capped embedding-row history. The wake model only runs when ≥1
//! new embedding row arrived — one tiny matrix op per chunk.

use std::path::Path;

use crate::driver::AcousticWake;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::TensorRef;

/// Samples per streaming chunk: 80 ms at 16 kHz, openWakeWord's native step.
pub const MODEL_FRAME_CHUNK: usize = 1280;

/// Mel-frame rows the custom model consumes per inference: 16.
pub const MODEL_INPUT_FRAMES: usize = 16;

/// Floats per speech_embedding row (the `speech_embedding` model's output).
const EMBEDDING_ROW: usize = 96;

/// Mel bins per frame (`melspectrogram.onnx` output width).
const MEL_BINS: usize = 32;

/// Above this score the wake word is treated as heard.
///
/// Calibrated on this machine's evidence, in order of strength:
/// - **Synthetic reference**: piper "hey evo" peaks at 0.209; negatives
///   (silence, noise, tones) score ~0.0009.
/// - **Live mic (8 utterances, this voice)**: peaks 0.151, 0.229, 0.328
///   with a noise floor of 0.013–0.052.
/// - **0.12** sits ≥9× above noise, catches all three live peaks, and
///   stays under the weakest synthetic positive. The original
///   openWakeWord-typical 0.5 sat *above every observed positive* and
///   could never fire — it was the reason "Hey Evo" went silent.
/// Machines whose distribution differs should override via
/// `EVO_WAKE_THRESHOLD`; `wake_probe --mic` prints the numbers.
const DEFAULT_THRESHOLD: f32 = 0.12;

/// Streak debounce for the wake decision, with a single-hit escape for
/// unambiguous scores.
///
/// A single 80 ms chunk at/above threshold used to fire immediately. With
/// a documented noise floor up to 0.052 against a 0.12 threshold, an
/// isolated threshold-grazing spike (chair scrape, keyboard clack) can
/// cross on its own, so scores below [`WakeDebouncer::HIGH_CONFIDENCE`]
/// require [`WakeDebouncer::REQUIRED`] consecutive chunks at/above the
/// threshold.
///
/// The streak must never gate a hit this strong, though: the model's
/// output is a single spike at the end of the phrase, and a phrase that
/// straddles a chunk boundary can put that spike in exactly one chunk —
/// no second consecutive hit ever comes and the wake silently dies (the
/// "say it five times" symptom). Live evidence
/// (`~/Library/Logs/evo-agent.log`): real "hey evo" hits score
/// 0.166–0.895 while the noise floor stays ≈0.052, so a single chunk
/// ≥[`WakeDebouncer::HIGH_CONFIDENCE`] (0.5 ≈ 10× noise) is a wake on its
/// own and fires without waiting 80 ms for confirmation. Firing either
/// way resets the run so one utterance yields one gesture.
struct WakeDebouncer {
    consecutive_hits: u32,
}

impl WakeDebouncer {
    /// Consecutive chunks at/above threshold that confirm a wake.
    const REQUIRED: u32 = 2;

    /// A single chunk at/above this score (and the configured threshold)
    /// fires immediately: no second chunk is needed.
    const HIGH_CONFIDENCE: f32 = 0.5;

    fn new() -> Self {
        Self {
            consecutive_hits: 0,
        }
    }

    /// Feeds one evaluated chunk score. Returns `true` on the chunk that
    /// completes a run of [`Self::REQUIRED`] hits, or immediately on a
    /// single hit at/above [`Self::HIGH_CONFIDENCE`] that also crosses
    /// the configured threshold.
    fn observe(&mut self, score: f32, threshold: f32) -> bool {
        if score >= threshold && score >= Self::HIGH_CONFIDENCE {
            self.consecutive_hits = 0;
            return true;
        }
        if score >= threshold {
            self.consecutive_hits += 1;
        } else {
            self.consecutive_hits = 0;
        }
        if self.consecutive_hits >= Self::REQUIRED {
            self.consecutive_hits = 0;
            true
        } else {
            false
        }
    }
}

/// The person's trained wake-word spotter: three ONNX sessions plus the
/// streaming buffers between them.
pub struct CustomWakeSpotter {
    mel: Session,
    embedding: Session,
    wake: Session,
    threshold: f32,
    /// Raw samples not yet folded into a full 1280-sample chunk.
    remainder: Vec<f32>,
    /// Newest-last ring of transformed mel values (spec/10 + 2), row-major.
    mel_ring: Vec<f32>,
    /// How many full mel frames `mel_ring` holds.
    mel_rows: usize,
    /// Most recent embedding rows, capped at 128.
    feature_rows: Vec<[f32; EMBEDDING_ROW]>,
    /// Two-consecutive-chunk confirmation for the wake gesture.
    debouncer: WakeDebouncer,
}

impl CustomWakeSpotter {
    /// Loads `melspectrogram.onnx`, `embedding_model.onnx`, and the
    /// person's custom wake model from `models_dir` (the same directory
    /// the whisper `.bin` lives in is natural).
    pub fn load(models_dir: &Path) -> Result<Self, String> {
        let load = |name: &str| -> Result<Session, String> {
            let path = models_dir.join(name);
            let mut builder =
                Session::builder().map_err(|e| format!("load {}: builder: {e}", path.display()))?;
            builder = builder
                .with_optimization_level(GraphOptimizationLevel::Level3)
                .map_err(|e| format!("load {}: optimization: {e}", path.display()))?;
            builder
                .commit_from_file(&path)
                .map_err(|e| format!("load {}: {e}", path.display()))
        };
        let mel = load("melspectrogram.onnx")?;
        let embedding = load("embedding_model.onnx")?;
        let wake = load("hey_evoh.onnx")?;
        if wake.inputs().first().is_none() {
            return Err("wake model has no inputs".to_string());
        }
        Ok(Self {
            mel,
            embedding,
            wake,
            threshold: DEFAULT_THRESHOLD,
            remainder: Vec::new(),
            mel_ring: Vec::new(),
            mel_rows: 0,
            feature_rows: Vec::new(),
            debouncer: WakeDebouncer::new(),
        })
    }

    /// Overrides the decision threshold. The reference run scored 0.21 on
    /// a synthetic positive vs 0.001 on negatives; 0.1 catches synthetic
    /// speech, 0.5 is the openWakeWord-typical default for real utterances.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// The configured decision threshold.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }
}

/// The [`CustomWakeSpotter`]'s calibrated default decision threshold.
pub const DEFAULT_WAKE_THRESHOLD: f32 = DEFAULT_THRESHOLD;

impl AcousticWake for CustomWakeSpotter {
    fn score(&mut self, audio: &[f32]) -> Result<Option<f32>, String> {
        // The driver consumes this path, so the two-chunk debounce is
        // applied here — where the 80 ms model chunks are visible — rather
        // than in the driver, where a burst of audio drained at once would
        // collapse several chunks into one score and defeat the streak.
        self.ingest_debounced(audio)
    }
    fn threshold(&self) -> f32 {
        self.threshold
    }

    fn reset(&mut self) {
        self.remainder.clear();
        self.mel_ring.clear();
        self.mel_rows = 0;
        self.feature_rows.clear();
        self.debouncer = WakeDebouncer::new();
    }
}

impl CustomWakeSpotter {
    /// Feeds 16 kHz mono f32 audio of any length; internal state
    /// re-aligns to 1280-sample boundaries. Returns the **maximum** score
    /// across every chunk evaluated in this call, or `None` when no
    /// wake-model evaluation happened. Returning the last chunk's score
    /// meant a wake buried in a backlog (drained after a blocked poll
    /// loop) was silently discarded whenever later chunks scored lower.
    /// Errors are reported, not swallowed: the caller decides whether the
    /// always-on loop can afford them.
    pub fn ingest(&mut self, audio: &[f32]) -> Result<Option<f32>, String> {
        self.ingest_inner(audio, false)
    }

    /// [`Self::ingest`] plus the two-consecutive-chunk debounce: `Some`
    /// only when at least two consecutive chunks crossed the threshold.
    /// The driver's [`AcousticWake`] path uses this; the raw probe and
    /// calibration tooling use [`Self::ingest`].
    fn ingest_debounced(&mut self, audio: &[f32]) -> Result<Option<f32>, String> {
        self.ingest_inner(audio, true)
    }

    fn ingest_inner(&mut self, audio: &[f32], debounce: bool) -> Result<Option<f32>, String> {
        self.remainder.extend_from_slice(audio);
        let mut max_score: Option<f32> = None;
        let mut confirmed: Option<f32> = None;
        while self.remainder.len() >= MODEL_FRAME_CHUNK {
            let chunk: Vec<f32> = self.remainder.drain(..MODEL_FRAME_CHUNK).collect();
            if let Some(score) = self.score_chunk(&chunk)? {
                max_score = Some(max_score.map_or(score, |best: f32| best.max(score)));
                if debounce && self.debouncer.observe(score, self.threshold) {
                    confirmed = max_score;
                }
            }
        }
        Ok(if debounce { confirmed } else { max_score })
    }

    /// Runs one 1280-sample chunk through the full feed. Returns `None`
    /// when the chunk produced no wake evaluation (no complete window).
    fn score_chunk(&mut self, chunk: &[f32]) -> Result<Option<f32>, String> {
        // The mel model consumes float-encoded int16 PCM, exactly as
        // openWakeWord feeds it (f32 values carrying int16 magnitudes).
        let encoded: Vec<f32> = chunk
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16 as f32)
            .collect();
        let samples = encoded.len() as i64;
        let input = TensorRef::from_array_view(([1i64, samples], encoded.as_slice()))
            .map_err(|e| format!("mel input: {e}"))?;
        // SessionOutputs borrows the session, so extract owned data
        // inside each block before touching the streaming buffers.
        let (shape, spec): (Vec<i64>, Vec<f32>) = {
            let session = &mut self.mel;
            let outputs = session
                .run(ort::inputs![input])
                .map_err(|e| format!("mel run: {e}"))?;
            let (shape, spec) = outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("mel extract: {e}"))?;
            (shape.to_vec(), spec.to_vec())
        };
        let n_frames = shape.iter().skip(1).product::<i64>() as usize / MEL_BINS;
        self.push_mel(&spec, n_frames);

        // Embedding: sliding 76-row windows at stride 8, oldest-first —
        // the same windows `AudioFeatures._streaming_features` builds.
        let windows = self.pending_windows();
        if windows.is_empty() {
            return Ok(None);
        }
        let flat: Vec<f32> = windows.concat();
        let batch = TensorRef::from_array_view((
            [windows.len() as i64, 76, MEL_BINS as i64, 1],
            flat.as_slice(),
        ))
        .map_err(|e| format!("embedding input: {e}"))?;
        let rows: Vec<f32> = {
            let session = &mut self.embedding;
            let emb_out = session
                .run(ort::inputs![batch])
                .map_err(|e| format!("embedding run: {e}"))?;
            let (_, rows) = emb_out[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("embedding extract: {e}"))?;
            rows.to_vec()
        };
        if rows.len() % EMBEDDING_ROW != 0 {
            return Err(format!(
                "embedding returned {} floats, not a multiple of {EMBEDDING_ROW}",
                rows.len()
            ));
        }
        for row in rows.chunks_exact(EMBEDDING_ROW) {
            self.feature_rows.push(row.try_into().unwrap());
        }
        let cap = 128;
        if self.feature_rows.len() > cap {
            let excess = self.feature_rows.len() - cap;
            self.feature_rows.drain(..excess);
        }

        // The custom model: last 16 embedding rows, [1, 16, 96].
        if self.feature_rows.len() < MODEL_INPUT_FRAMES {
            return Ok(None);
        }
        let last16: Vec<f32> = self.feature_rows[self.feature_rows.len() - MODEL_INPUT_FRAMES..]
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect();
        let input = TensorRef::from_array_view((
            [1i64, MODEL_INPUT_FRAMES as i64, EMBEDDING_ROW as i64],
            last16.as_slice(),
        ))
        .map_err(|e| format!("wake input: {e}"))?;
        let score = {
            let session = &mut self.wake;
            let ww_out = session
                .run(ort::inputs![input])
                .map_err(|e| format!("wake run: {e}"))?;
            let (_, out) = ww_out[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("wake extract: {e}"))?;
            out.first().copied().unwrap_or(0.0)
        };
        Ok(Some(score))
    }

    /// Appends transformed mel frames to the ring (newest-last).
    fn push_mel(&mut self, spec: &[f32], n_frames: usize) {
        let mut frames: Vec<f32> = spec
            .iter()
            .take(n_frames * MEL_BINS)
            .map(|v| v / 10.0 + 2.0)
            .collect();
        self.mel_ring.append(&mut frames);
        self.mel_rows += n_frames;
        let max_rows = 384; // 3 × the 76-frame window at stride 8
        if self.mel_rows > max_rows {
            let drop = (self.mel_rows - max_rows) * MEL_BINS;
            self.mel_ring.drain(..drop);
            self.mel_rows = max_rows;
        }
    }

    /// Builds the embedding window for this chunk: the newest 76 mel
    /// frames. Mirrors openWakeWord's `_streaming_features`, which emits
    /// exactly one window per 1280-sample chunk (its per-call loop over
    /// `accumulated//1280 - 1` runs once), producing one embedding row.
    fn pending_windows(&self) -> Vec<Vec<f32>> {
        if self.mel_rows < 76 {
            return Vec::new();
        }
        let end = self.mel_rows;
        let start = end - 76;
        if start * MEL_BINS + 76 * MEL_BINS > self.mel_ring.len() {
            return Vec::new();
        }
        vec![self.mel_ring[start * MEL_BINS..end * MEL_BINS].to_vec()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn models_dir() -> Option<std::path::PathBuf> {
        // The three graphs are large binaries; they live in testdata/ and
        // the test skips cleanly when absent (CI without model assets).
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/wake");
        dir.exists().then_some(dir)
    }

    /// A 16 kHz f32 sine burst — speech-shaped energy, no wake content.
    fn tone(freq: f32, secs: f32) -> Vec<f32> {
        (0..(16000.0 * secs) as usize)
            .map(|i| 0.3 * (2.0 * std::f32::consts::PI * freq * i as f32 / 16000.0).sin())
            .collect()
    }

    #[test]
    fn missing_models_fall_to_an_honest_error() {
        let err = match CustomWakeSpotter::load(Path::new("/nonexistent-dir")) {
            Ok(_) => panic!("missing models must not load"),
            Err(err) => err,
        };
        assert!(err.contains("missing") || err.contains("load"), "{err}");
    }

    #[test]
    fn tone_audio_scores_low_against_the_trained_model() {
        let Some(dir) = models_dir() else {
            eprintln!("skipping: testdata/wake models not present");
            return;
        };
        let mut spotter = CustomWakeSpotter::load(&dir).unwrap();
        let audio = tone(220.0, 4.0);
        let score = spotter.ingest(&audio).unwrap();
        match score {
            Some(s) => assert!(s < 0.3, "a tone must not wake the model, scored {s}"),
            None => panic!("4 s of audio should produce at least one evaluation"),
        }
    }

    #[test]
    fn streaming_in_small_chunks_matches_whole_buffer_feed() {
        let Some(dir) = models_dir() else {
            eprintln!("skipping: testdata/wake models not present");
            return;
        };
        let audio = tone(440.0, 3.0);
        let whole = CustomWakeSpotter::load(&dir)
            .unwrap()
            .ingest(&audio)
            .unwrap();
        let mut chunked = CustomWakeSpotter::load(&dir).unwrap();
        let mut max = None;
        for chunk in audio.chunks(799) {
            // ingest returns None when no 1280-sample boundary completed
            // in this call — that is "no new evaluation", not a zero.
            if let Some(score) = chunked.ingest(chunk).unwrap() {
                max = Some(max.map_or(score, |best: f32| best.max(score)));
            }
        }
        assert_eq!(whole, max, "chunk size must not change the maximum score");
    }

    #[test]
    fn debouncer_requires_two_consecutive_hits_and_fires_once() {
        let threshold = 0.12;
        let mut debouncer = WakeDebouncer::new();
        assert!(
            !debouncer.observe(0.30, threshold),
            "a single spike must not fire"
        );
        assert!(
            !debouncer.observe(0.01, threshold),
            "a sub-threshold chunk resets the run"
        );
        assert!(
            !debouncer.observe(0.30, threshold),
            "a spike after a reset is still a single hit"
        );
        assert!(
            debouncer.observe(0.15, threshold),
            "the second consecutive hit fires"
        );
        assert!(
            !debouncer.observe(0.20, threshold),
            "firing resets the run: one more hit is not a second wake"
        );
    }

    /// Regression: the model's wake spike can be a single chunk wide when
    /// the phrase straddles a chunk boundary. Requiring two consecutive
    /// threshold hits then made "hey evo" need many attempts; a single hit
    /// at/above HIGH_CONFIDENCE must fire on its own.
    #[test]
    fn high_confidence_single_hit_fires_immediately() {
        let threshold = 0.12;
        let mut debouncer = WakeDebouncer::new();
        assert!(
            debouncer.observe(0.55, threshold),
            "a single hit ≥0.5 must fire without a second consecutive chunk"
        );
        assert!(
            !debouncer.observe(0.18, threshold),
            "firing resets the run: a following grazing hit is again a single"
        );
        assert!(
            debouncer.observe(0.20, threshold),
            "two consecutive grazing hits still fire"
        );
    }

    /// The escape hatch never weakens a configured threshold: a high
    /// score below an operator-raised threshold is not a wake.
    #[test]
    fn high_confidence_single_hit_still_respects_the_threshold() {
        let mut debouncer = WakeDebouncer::new();
        assert!(
            !debouncer.observe(0.60, 0.80),
            "0.60 is high-confidence but below the configured 0.80"
        );
        assert!(!debouncer.observe(0.79, 0.80), "still below the threshold");
        assert!(
            debouncer.observe(0.85, 0.80),
            "at/above the configured threshold a single hit fires"
        );
    }

    /// Regression: `ingest` must aggregate one call's chunks by maximum.
    /// A backlog drained at once evaluates several chunks; returning the
    /// last chunk's score discarded an earlier wake buried in the call.
    #[test]
    fn ingest_returns_the_maximum_score_across_the_call() {
        let Some(dir) = models_dir() else {
            eprintln!("skipping: testdata/wake models not present");
            return;
        };
        // Contrasting segments with a silent tail: per-chunk scores vary,
        // so "max of the call" and "last evaluated chunk" cannot coincide.
        let mut audio = tone(220.0, 2.0);
        audio.extend(tone(880.0, 1.0));
        audio.extend(vec![0.0f32; 16_000]);

        // Feed one 1280-sample chunk per call to capture the chunk scores.
        let mut chunked = CustomWakeSpotter::load(&dir).unwrap();
        let mut per_chunk: Vec<f32> = Vec::new();
        for chunk in audio.chunks(MODEL_FRAME_CHUNK) {
            if let Some(score) = chunked.ingest(chunk).unwrap() {
                per_chunk.push(score);
            }
        }
        assert!(
            per_chunk.len() >= 2,
            "expected several chunk evaluations, got {}",
            per_chunk.len()
        );
        let max = per_chunk.iter().copied().fold(f32::MIN, f32::max);
        let last = *per_chunk.last().unwrap();

        // The same audio in a single call evaluates every chunk at once.
        let mut whole = CustomWakeSpotter::load(&dir).unwrap();
        let returned = whole.ingest(&audio).unwrap().expect("audio was evaluated");
        assert!(
            (returned - max).abs() <= 1e-6,
            "ingest returned {returned}, but the per-chunk maximum is {max} \
             (last chunk scored {last}); a single call must aggregate by max"
        );
        assert!(
            max > last + 1e-6,
            "test audio is degenerate: the loudest chunk ({max}) equals the last ({last})"
        );
    }

    #[test]
    fn threshold_override_takes() {
        let Some(dir) = models_dir() else {
            eprintln!("skipping: testdata/wake models not present");
            return;
        };
        let spotter = CustomWakeSpotter::load(&dir).unwrap().with_threshold(0.13);
        assert!((spotter.threshold() - 0.13).abs() < f32::EPSILON);
    }
}
