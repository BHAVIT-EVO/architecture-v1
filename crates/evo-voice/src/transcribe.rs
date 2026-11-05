//! Local speech-to-text via whisper-rs (whisper.cpp), on-device.
//!
//! Takes 16kHz mono f32 audio samples and returns transcribed text.
//! No network. The model file (.bin) is loaded from disk — download once
//! from HuggingFace (e.g. `ggml-base.en.bin`) and point [`Transcriber::load`]
//! at it.
//!
//! Accent-robustness comes from whisper's training; the GOAL specifies
//! whisper for this reason. Model size trades accuracy for speed/CPU:
//! tiny (75MB) < base (140MB) < small (460MB) < medium (1.5GB).

use std::borrow::Cow;
use std::sync::Mutex;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

/// Sample rate whisper expects: 16kHz mono.
pub const SAMPLE_RATE: u32 = 16_000;

/// Local transcriber backed by a whisper.cpp model.
pub struct Transcriber {
    ctx: WhisperContext,
    // Reused across calls. Each `create_state()` allocates ~210 MB of
    // KV/compute buffers and emits ~9 log lines; the always-on wake scan
    // runs every few seconds, so per-call states turned one agent run
    // into hundreds of re-inits. A whisper state is explicitly reusable
    // across `full()` runs — whisper.cpp resets it internally per call.
    state: Mutex<Option<WhisperState>>,
}

impl Transcriber {
    /// Loads a whisper model from `model_path` (a `.bin` file).
    ///
    /// Downloads are the caller's responsibility — fetch once from
    /// HuggingFace (e.g. `https://huggingface.co/ggerganov/whisper.cpp`).
    pub fn load(model_path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let mut context_params = WhisperContextParameters::default();
        context_params.use_gpu(true).flash_attn(true);
        let ctx = WhisperContext::new_with_params(
            model_path.as_ref().to_str().ok_or("non-UTF8 model path")?,
            context_params,
        )
        .map_err(|e| format!("failed to load whisper model: {e}"))?;
        Ok(Self {
            ctx,
            state: Mutex::new(None),
        })
    }

    /// Initializes Whisper's reusable CPU state before the first user
    /// command. Without this warm-up, the first post-wake partial pays the
    /// state-allocation and backend-initialization cost while the notch is
    /// already counting down its listening tenure.
    pub fn warm_up(&self) -> Result<(), String> {
        self.transcribe(&vec![0.0; SAMPLE_RATE as usize]).map(|_| ())
    }

    /// Transcribes 16kHz mono f32 audio samples to text.
    ///
    /// Returns the concatenated segment texts, trimmed. Empty audio or
    /// silence yields an empty string — the caller decides what that means.
    ///
    /// The underlying whisper state is created once and reused, so repeated
    /// calls (the always-on scan) pay no re-init cost.
    pub fn transcribe(&self, audio: &[f32]) -> Result<String, String> {
        if audio.is_empty() {
            return Ok(String::new());
        }
        // Lazily create the state once, then reuse it for every call.
        let mut state_guard = self
            .state
            .lock()
            .map_err(|_| "whisper state mutex poisoned".to_string())?;
        if state_guard.is_none() {
            let created = self
                .ctx
                .create_state()
                .map_err(|e| format!("failed to create whisper state: {e}"))?;
            *state_guard = Some(created);
        }
        let state = state_guard.as_mut().expect("just initialized");

        // Greedy decoding is materially faster than beam search on the
        // CPU-only base model and keeps final command latency bounded. The
        // acoustic wake model already handles phrase detection; Whisper's
        // job here is fast command text, not offline-quality beam rescoring.
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(num_cpus());
        // English; whisper auto-detects if None, but pinning avoids confusion
        // on short utterances. Bias the product name so wake/command
        // transcripts do not turn "Evo" into a common homophone.
        params.set_language(Some("en"));
        params.set_initial_prompt("Hey Evo.");
        // Don't translate — transcribe in the spoken language.
        params.set_translate(false);
        // State buffers are reused, but one utterance must never inherit the
        // previous utterance's text prompt and hallucinate it over silence.
        params.set_no_context(true);
        params.set_print_realtime(false);
        params.set_print_progress(false);
        params.set_print_timestamps(false);
        params.set_print_special(false);
        // Suppress non-speech tokens.
        params.set_suppress_nst(true);
        params.set_no_speech_thold(0.6);
        // Disable whisper's temperature-fallback retry loop. whisper-rs
        // defaults `temperature_inc` to 0.2, so a low-confidence utterance
        // is re-decoded at temperatures 0.2 → 1.0 and the highest-probability
        // sample wins — a known hallucination source on poor audio. Zero
        // keeps the single deterministic greedy pass.
        params.set_temperature_inc(0.0);

        state
            .full(params, audio)
            .map_err(|e| format!("transcription failed: {e}"))?;

        let num_segments = state.full_n_segments();
        let mut text = String::new();
        for i in 0..num_segments {
            let segment = state
                .get_segment(i)
                .ok_or_else(|| format!("failed to read segment {i}: out of bounds"))?;
            if segment.no_speech_probability() >= 0.6 {
                continue;
            }
            // Lossy decoding: one invalid byte must never void the
            // utterance. Live evidence: 3 UTF-8 transcription failures,
            // each immediately followed by a `NoCommand` — the strict
            // `to_str()?` dropped every segment. `to_str_lossy` replaces
            // invalid bytes with U+FFFD and only errors on a null pointer;
            // even then only that one segment is skipped, never the call.
            let decoded = segment
                .to_str_lossy()
                .map_err(|err| format!("segment {i}: {err}"));
            if !append_segment(&mut text, decoded) {
                break;
            }
        }
        Ok(crate::wake::sanitize_transcript(&text))
    }
}

/// Appends one segment's decoded text to `text`, bounded by
/// [`crate::wake::MAX_TRANSCRIPT_BYTES`].
///
/// Returns `false` when the accumulator is full and the caller should stop
/// reading segments. An unreadable segment ([`Err`]) is logged and skipped,
/// returning `true`: one bad segment must never void the rest of the
/// utterance. Real `WhisperSegment`s need a loaded model, so the
/// transcription loop's accumulation contract is tested through this
/// helper.
fn append_segment(text: &mut String, decoded: Result<Cow<'_, str>, String>) -> bool {
    let segment = match decoded {
        Ok(segment) => segment,
        Err(err) => {
            eprintln!("EVO voice: unreadable segment skipped: {err}");
            return true;
        }
    };
    // Keep model output bounded before it reaches captions or the
    // command resolver. The wake boundary applies the same limit
    // after normalization, but this also bounds the accumulator.
    if text.len() >= crate::wake::MAX_TRANSCRIPT_BYTES {
        return false;
    }
    let remaining = crate::wake::MAX_TRANSCRIPT_BYTES - text.len();
    let segment = segment.trim();
    let mut end = segment.len().min(remaining);
    while !segment.is_char_boundary(end) {
        end -= 1;
    }
    text.push_str(&segment[..end]);
    if text.len() < crate::wake::MAX_TRANSCRIPT_BYTES {
        text.push(' ');
    }
    true
}

/// Number of threads for whisper inference: physical cores, capped.
fn num_cpus() -> i32 {
    std::thread::available_parallelism()
        .map(|n| (n.get() as i32).clamp(1, 8))
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_rate_is_whisper_native() {
        assert_eq!(SAMPLE_RATE, 16_000);
    }

    #[test]
    fn empty_audio_yields_empty_text_without_model() {
        // Contract: empty in → empty out, no model needed.
        // (Real transcription tests need a model file; run manually.)
    }

    #[test]
    fn lossy_segments_do_not_void_the_transcript() {
        // Constraint: building a real `WhisperSegment` requires a loaded
        // whisper model, so this drives `append_segment`, the unit that
        // receives each segment's decoded text. Live regression: one
        // invalid byte failed the whole transcription (`to_str()?`), which
        // the driver turned into `NoCommand`.
        let mut text = String::new();
        assert!(append_segment(
            &mut text,
            Ok(Cow::Borrowed("open the note"))
        ));
        // What `to_str_lossy` returns for a segment containing an invalid
        // byte: U+FFFD in place of the byte, the rest of the text intact.
        let lossy = String::from_utf8_lossy(b"capture \xE2 plan").into_owned();
        assert!(lossy.contains('\u{FFFD}'));
        assert!(append_segment(&mut text, Ok(Cow::Owned(lossy))));
        // A segment that cannot be decoded at all is skipped — the other
        // segments still make it into the transcript.
        assert!(append_segment(
            &mut text,
            Err(
                "segment 2: Invalid UTF-8 detected in a string from Whisper. Index: 7, Length: 1."
                    .to_string()
            )
        ));
        assert!(append_segment(&mut text, Ok(Cow::Borrowed("final words"))));
        assert_eq!(
            text.trim(),
            "open the note capture \u{FFFD} plan final words"
        );
    }

    #[test]
    fn append_segment_stops_at_the_transcript_bound() {
        let mut text = "x".repeat(crate::wake::MAX_TRANSCRIPT_BYTES);
        assert!(!append_segment(&mut text, Ok(Cow::Borrowed("more"))));
    }
}
