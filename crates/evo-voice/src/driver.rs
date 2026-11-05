//! The voice loop driver: platform-neutral stage logic.
//!
//! ```text
//! audio chunks → rolling buffer → periodic transcribe → detect_wake
//!     → fresh command capture → TriggerTransport → VoiceOutcome → event sink
//! ```
//!
//! The driver is a state machine, not a thread: the owner (the desktop's
//! voice thread) feeds 16 kHz mono audio via [`VoiceDriver::ingest`] and
//! pumps [`VoiceDriver::poll`]. Audio capture, transcription, and the
//! trigger transport are injected as traits, so the whole loop is testable
//! without a microphone, a model, or a daemon:
//!
//! - [`UtteranceTranscriber`] — [`crate::Transcriber`] implements it under
//!   the `audio` feature; tests use a scripted fake.
//! - [`TriggerTransport`] — the desktop implements it over the daemon's
//!   voice socket; tests use a fake.
//!
//! Outcomes leave through an `mpsc::Sender<VoiceEvent>` the UI consumes.
//! When the sink is gone, `poll` returns `false` and the owner stops the
//! thread — shutdown is the receiver going away, no extra signalling.
//!
//! # No hardcoding
//!
//! The wake phrase is the product name; what follows it is resolved by
//! meaning downstream, never by fixed command grammars. A wake scan only
//! opens a fresh capture. Timing constants are physical audio parameters, not
//! app behavior.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use crate::restore::VoiceOutcome;
use crate::wake::{WakeResult, detect_wake, sanitize_transcript};

/// Looks at [`VoiceEvent`]s without owning them: an accessory listener
/// (the notch surface) subscribes to the same stream the UI consumes so
/// both can react to the same wake/command/outcome sequence.
pub trait VoiceEventObserver {
    /// Called for every event the driver emits; return `false` to detach.
    fn observe(&mut self, event: &VoiceEvent) -> bool;
}

impl<T: VoiceEventObserver + ?Sized> VoiceEventObserver for std::sync::Mutex<T> {
    fn observe(&mut self, event: &VoiceEvent) -> bool {
        self.lock().unwrap().observe(event)
    }
}

impl VoiceEventObserver for Sender<VoiceEvent> {
    fn observe(&mut self, event: &VoiceEvent) -> bool {
        self.send(event.clone()).is_ok()
    }
}

/// Whisper/mic native sample rate, in Hz. The driver only needs it to turn
/// time windows into sample counts; the audio itself is opaque `f32`s.
const NATIVE_SAMPLE_RATE: u32 = 16_000;

/// Audio retained across a wake transition and prepended to the command
/// capture. The acoustic spotter needs two 80 ms chunks to confirm the
/// gesture, so words spoken right after "hey evo" have already arrived by
/// the time the capture opens; a transcript wake lags whisper inference
/// (up to ~1 s) the same way. 1.5 s covers both while staying far short
/// of replaying a previous sentence into the capture.
const WAKE_PREROLL: Duration = Duration::from_millis(2_000);

/// Trailing silence that ends a command capture early. Once speech has
/// started, 1 second of quiet means the person finished; the configured
/// `command_capture` (6 s by default) stays as the hard wall-clock ceiling.
const COMMAND_ENDPOINT_SILENCE: Duration = Duration::from_millis(350);

/// Cumulative speech a window must carry before it is sent to whisper.
/// The old "any 30 ms frame above threshold" gate let a single click (or
/// keyboard clack) reach the model and hallucinate; 300 ms of voiced
/// frames filters those while sitting far below ordinary commands. The
/// floor is capped at the configured window length so a physically tiny
/// window is never asked to contain more speech than it can hold.
const MIN_SPEECH: Duration = Duration::from_millis(300);

/// Live partials: never more often than this...
const PARTIAL_INTERVAL: Duration = Duration::from_millis(700);
/// ...and only when at least this much new audio arrived since the last.
const PARTIAL_MIN_NEW_AUDIO: Duration = Duration::from_millis(500);

/// Keep live captions opt-in until they have a dedicated transcription
/// worker. A partial decode sharing Whisper state with the final decode can
/// otherwise delay the command result by many seconds on CPU.
const LIVE_PARTIALS: bool = false;

/// Do not let the wake phrase itself end a newly opened capture. The acoustic
/// spotter confirms on wake audio, so the endpoint needs a short grace period
/// before it is allowed to inspect post-wake silence.
const COMMAND_ENDPOINT_GRACE: Duration = Duration::from_millis(900);

/// Whisper latency grows with the amount of audio. Final commands are
/// intentionally bounded to a recent window; the wake phrase is removed
/// after decoding and the command resolver only needs the spoken request.
const MAX_FINAL_AUDIO: Duration = Duration::from_secs(4);

/// Prevents a completed/no-command round from immediately re-opening on the
/// same acoustic tail or a late model spike. A fresh deliberate wake after
/// this short quiet interval is still accepted.
const WAKE_REARM_COOLDOWN: Duration = Duration::from_secs(2);

/// Transcribes one audio window into text.
///
/// [`crate::Transcriber`] implements this under the `audio` feature.
pub trait UtteranceTranscriber {
    /// Transcribes 16 kHz mono `f32` audio; empty audio yields empty text.
    /// Errors are transient (model hiccup) — the driver logs and continues.
    fn transcribe_utterance(&self, audio: &[f32]) -> Result<String, String>;
}

/// Shared transcribers work anywhere a transcriber does.
impl<T: UtteranceTranscriber + ?Sized> UtteranceTranscriber for std::sync::Arc<T> {
    fn transcribe_utterance(&self, audio: &[f32]) -> Result<String, String> {
        (**self).transcribe_utterance(audio)
    }
}

/// Spots the wake word acoustically, before any transcription.
///
/// Implemented by [`crate::CustomWakeSpotter`] under the `wake-model`
/// feature; tests inject fakes. The driver asks it every 80 ms chunk of
/// ingested audio; a `Some(score) >= threshold` is a bare wake — the
/// existing command-capture window handles what follows.
pub trait AcousticWake {
    /// Feeds one chunk of 16 kHz mono f32 audio. Returns the newest wake
    /// score when the spotter evaluated this call, else `None`.
    fn score(&mut self, audio: &[f32]) -> Result<Option<f32>, String>;
    /// The score above which a wake is accepted.
    fn threshold(&self) -> f32;
    /// Clears streaming context after a wake transition. Stateful spotters
    /// must not carry the wake phrase into the command audio or the next
    /// idle session.
    fn reset(&mut self) {}
}

impl<T: AcousticWake + ?Sized> AcousticWake for std::sync::Mutex<T> {
    fn score(&mut self, audio: &[f32]) -> Result<Option<f32>, String> {
        self.get_mut().unwrap().score(audio)
    }
    fn threshold(&self) -> f32 {
        self.lock().unwrap().threshold()
    }
    fn reset(&mut self) {
        self.get_mut().unwrap().reset();
    }
}

/// Sends a trigger phrase for resolution + restore and returns the outcome.
///
/// The desktop implements this over the daemon's voice socket; the daemon
/// resolves against its live `Reconstruction` and restores there.
pub trait TriggerTransport {
    /// Resolves `command` and restores the matching work. Returns the
    /// outcome for the UI, or an error when the daemon is unreachable —
    /// which the driver reports honestly as `VoiceOutcome::Failed`.
    fn trigger(&self, command: &str) -> Result<VoiceOutcome, String>;
    /// Answers a question about the work — the engine's rack, spoken.
    /// The daemon computes it from its log at request time; no model, no
    /// semantic resolution.
    fn ask(&self) -> Result<VoiceOutcome, String>;
}

/// Timing for the loop. All fields are physical audio parameters.
#[derive(Debug, Clone)]
pub struct VoiceLoopConfig {
    /// How often the rolling buffer is transcribed and wake-scanned.
    pub scan_interval: Duration,
    /// How much audio each scan covers (the keyword-spot window).
    pub scan_window: Duration,
    /// After a bare "hey evo", how long to capture the follow-up command
    /// before submitting whatever audio was heard.
    pub command_capture: Duration,
    /// Rolling buffer cap; older audio is dropped.
    pub max_buffer: Duration,
    /// RMS amplitude below which a scan window is treated as silence and
    /// skipped without a transcription call. A physical audio parameter:
    /// far-field speech sits well above this; quiet rooms sit below it.
    pub speech_rms_threshold: f32,
}

impl Default for VoiceLoopConfig {
    fn default() -> Self {
        Self {
            // Transcribing every few seconds keeps wake latency low without
            // pinning a core: whisper on a few seconds of audio is ~1 s.
            scan_interval: Duration::from_secs(3),
            // The window must cover a whole "hey evo, <command>" utterance.
            scan_window: Duration::from_secs(12),
            // A bare wake buys the person enough time for a natural command
            // with a pause. This is only paid after wake, not on the idle path.
            command_capture: Duration::from_secs(6),
            max_buffer: Duration::from_secs(30),
            // Conservative: misses whisper nothing but saves hours of
            // transcribing room tone. Speech at normal mic distance has
            // RMS an order of magnitude above this.
            speech_rms_threshold: 0.006,
        }
    }
}

/// What the UI needs to know about the voice loop.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceEvent {
    /// The wake word was heard (acoustically or in a transcript): the
    /// spotter/keyword-spot crossed its threshold or a transcript matched.
    /// Emitted once per accepted utterance so any surface (main window,
    /// notch) can expand to show the person Evo is listening.
    WakeGesture {
        /// The score that crossed the acoustic threshold, when the wake
        /// came from the acoustic spotter (`None` for transcript wake).
        score: Option<f32>,
    },
    /// A bare wake is capturing the follow-up: live feedback that Evo is
    /// actively listening (also carries each partial transcript as it
    /// stabilizes, for the notch's live-caption view).
    Listening { partial: Option<String> },
    /// The wake word was heard and this command extracted — shown/said
    /// immediately so the person knows Evo is acting.
    HeardCommand { command: String },
    /// The trigger round-trip finished (or the command never came).
    Outcome(VoiceOutcome),
}

/// A capture transcription handed to the worker thread. Both kinds carry
/// the capture id so a result from a finished or replaced capture can
/// never leak into a newer one.
enum TranscriptionJob {
    Partial { capture: u64, audio: Vec<f32> },
    Final { capture: u64, audio: Vec<f32> },
}

/// A worker result for the poll loop to drain.
enum TranscriptionResult {
    Partial {
        capture: u64,
        result: Result<String, String>,
    },
    Final {
        capture: u64,
        result: Result<String, String>,
    },
}

/// The transcription worker: serves partial/final jobs in submission
/// order on its own thread, so `poll` never waits on whisper (~1 s for a
/// 6 s clip). The driver submits a partial only when none is in flight
/// and a final only at capture close, keeping the queue depth ≤ 1 — a
/// final never waits behind a partial backlog.
fn transcription_worker<Tr: UtteranceTranscriber>(
    transcriber: Arc<Tr>,
    jobs: Receiver<TranscriptionJob>,
    results: Sender<TranscriptionResult>,
) {
    while let Ok(job) = jobs.recv() {
        let started = std::time::Instant::now();
        let result = match job {
            TranscriptionJob::Partial { capture, audio } => {
                let result = transcriber.transcribe_utterance(&audio);
                eprintln!(
                    "EVO voice: partial transcription capture {capture} ({:.2}s audio) finished in {:.2}s",
                    audio.len() as f64 / f64::from(NATIVE_SAMPLE_RATE),
                    started.elapsed().as_secs_f64()
                );
                TranscriptionResult::Partial { capture, result }
            }
            TranscriptionJob::Final { capture, audio } => {
                let result = transcriber.transcribe_utterance(&audio);
                eprintln!(
                    "EVO voice: final transcription capture {capture} ({:.2}s audio) finished in {:.2}s",
                    audio.len() as f64 / f64::from(NATIVE_SAMPLE_RATE),
                    started.elapsed().as_secs_f64()
                );
                TranscriptionResult::Final { capture, result }
            }
        };
        if results.send(result).is_err() {
            // The driver is gone; stop without touching the model again.
            return;
        }
    }
}

/// The voice loop state machine.
///
/// Generic over the transcriber and transport so tests inject fakes.
/// Not a thread: the owner feeds audio and pumps `poll`; partial and
/// final transcriptions run on a worker thread the driver owns, and
/// `poll` drains their results without blocking.
pub struct VoiceDriver<Tr, Tp> {
    /// Shared with the transcription worker (the worker's clone lives
    /// only inside its thread). The whisper scan on the poll path still
    /// needs it; whisper-rs serializes calls through its state mutex.
    transcriber: Arc<Tr>,
    transport: Tp,
    sink: Sender<VoiceEvent>,
    config: VoiceLoopConfig,
    buffer: Vec<f32>,
    last_scan: Instant,
    command_capture_started: Option<Instant>,
    /// Buffer length when the current capture opened: index where the
    /// post-wake audio begins. Speech before this mark is the retained
    /// pre-roll (the wake phrase itself) and can never satisfy the
    /// command endpointer — see [`Self::command_endpoint_reached`].
    capture_start_len: usize,
    /// Id of the current (or, while its final is pending, last) capture.
    capture_id: u64,
    /// Whether a capture is currently collecting audio.
    capture_open: bool,
    /// Capture id whose final transcription is outstanding.
    awaiting_final: Option<u64>,
    /// Final captures that may complete while a newer capture is active.
    pending_finals: HashSet<u64>,
    /// A partial transcription is on the worker right now.
    partial_inflight: bool,
    /// Acoustic spotter, when one is loaded. Checked on every poll with
    /// the audio not yet fed to it; the whisper scan stays as fallback.
    acoustic: Option<Box<dyn AcousticWake>>,
    /// How many buffered samples the acoustic spotter has already seen.
    acoustic_fed: usize,
    /// Accessory listeners (the notch surface) that receive every event
    /// alongside the primary UI sink.
    observers: Vec<Box<dyn VoiceEventObserver>>,
    /// Last time a partial transcription was submitted inside a capture.
    partial_transcribed_at: Instant,
    /// Buffer length when the last partial was submitted (unchanged
    /// buffers are skipped).
    partial_audio_len: usize,
    /// Newest stabilized partial transcript of the current capture.
    last_partial: Option<String>,
    /// Worker channels: jobs out, results back.
    job_sender: Sender<TranscriptionJob>,
    result_receiver: Receiver<TranscriptionResult>,
    wake_rearm_at: Instant,
}

impl<Tr: UtteranceTranscriber + Send + Sync + 'static, Tp: TriggerTransport> VoiceDriver<Tr, Tp> {
    /// Creates the driver. `sink` receives [`VoiceEvent`]s for the UI.
    ///
    /// Spawns the transcription worker thread; the transcriber is shared
    /// (`Arc`) between the worker and the poll-path wake scan.
    pub fn new(
        transcriber: Tr,
        transport: Tp,
        sink: Sender<VoiceEvent>,
        config: VoiceLoopConfig,
    ) -> Self {
        let (job_sender, job_receiver) = std::sync::mpsc::channel();
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let transcriber = Arc::new(transcriber);
        let worker_transcriber = Arc::clone(&transcriber);
        std::thread::Builder::new()
            .name("evo-voice-transcriber".to_string())
            .spawn(move || transcription_worker(worker_transcriber, job_receiver, result_sender))
            .expect("failed to spawn the voice transcription worker");
        Self {
            transcriber,
            transport,
            sink,
            config,
            buffer: Vec::new(),
            last_scan: Instant::now(),
            command_capture_started: None,
            capture_start_len: 0,
            capture_id: 0,
            capture_open: false,
            awaiting_final: None,
            pending_finals: HashSet::new(),
            partial_inflight: false,
            acoustic: None,
            acoustic_fed: 0,
            observers: Vec::new(),
            partial_transcribed_at: Instant::now(),
            partial_audio_len: 0,
            last_partial: None,
            job_sender,
            result_receiver,
            wake_rearm_at: Instant::now(),
        }
    }

    /// Subscribes an accessory listener to every [`VoiceEvent`]. The
    /// observer sees events before the primary sink is checked for
    /// liveness, so detaching early does not disturb the main loop.
    pub fn with_observer(mut self, observer: Box<dyn VoiceEventObserver>) -> Self {
        self.observers.push(observer);
        self
    }

    /// A bare wake just fired (acoustically or in a transcript) and the
    /// driver is now capturing the follow-up: keep the pre-roll, reset
    /// capture-side partial state, and announce the listening gesture so
    /// surfaces can expand. Callers must call [`Self::retain_preroll`]
    /// first; the retained tail is the capture's opening audio.
    fn begin_command_capture(&mut self, score: Option<f32>) -> bool {
        self.capture_id = self.capture_id.wrapping_add(1);
        self.capture_open = true;
        // Only audio arriving after wake confirmation can end the capture.
        // The retained tail still goes to Whisper, so a command spoken in
        // the same breath is preserved without allowing the wake phrase to
        // satisfy the silence endpoint.
        self.capture_start_len = self.buffer.len();
        self.partial_transcribed_at = Instant::now();
        self.partial_audio_len = self.buffer.len();
        self.last_partial = None;
        self.partial_inflight = false;
        self.command_capture_started = Some(Instant::now());
        if let Some(acoustic) = self.acoustic.as_mut() {
            acoustic.reset();
            // The retained wake pre-roll is intentionally not replayed into
            // the freshly reset spotter. It starts again with command audio.
            self.acoustic_fed = self.buffer.len();
        }
        if !self.emit(VoiceEvent::WakeGesture { score }) {
            return false;
        }
        self.emit(VoiceEvent::Listening { partial: None })
    }

    /// Attaches an acoustic wake spotter. When attached, the spotter sees
    /// every ingested sample first; a score over its threshold acts as a
    /// bare wake (command-capture follows). The periodic whisper scan
    /// remains as fallback for utterances the spotter misses.
    pub fn with_acoustic_wake(mut self, wake: Box<dyn AcousticWake>) -> Self {
        self.acoustic = Some(wake);
        self
    }

    /// Feeds one audio chunk (16 kHz mono `f32`) into the rolling buffer.
    ///
    /// The buffer is capped at `max_buffer`; older audio is dropped. Called
    /// from the owner's mic loop as fast as chunks arrive.
    pub fn ingest(&mut self, chunk: Vec<f32>) {
        self.buffer.extend_from_slice(&chunk);
        let max = self.samples(self.config.max_buffer);
        if self.buffer.len() > max {
            let dropped = self.buffer.len() - max;
            self.buffer.drain(..dropped);
            // The acoustic cursor tracks buffer positions; the drop shifted them.
            self.acoustic_fed = self.acoustic_fed.saturating_sub(dropped);
        }
    }

    /// Runs one loop iteration: scan for the wake word, or finish a
    /// pending command capture.
    ///
    /// Returns `false` when the event sink is gone — the owner should stop
    /// the voice thread. Never panics on audio input.
    pub fn poll(&mut self) -> bool {
        // Worker results first: a completed final must reach the UI even
        // when a wake transition or the next capture step is also due.
        if !self.drain_transcriptions() {
            return false;
        }
        if self.command_capture_started.is_some() {
            // Keep wake detection live during a command. A fresh wake starts
            // a new capture immediately and supersedes stale partial state.
            if self.poll_acoustic_wake(true) {
                return true;
            }
            return self.poll_command_capture();
        }
        if self.awaiting_final.is_some() {
            // Keep the acoustic wake path live while an older final is in
            // flight. The next capture has its own id and final result.
            if self.poll_acoustic_wake(true) {
                return true;
            }
            return true;
        }
        if self.poll_acoustic_wake(true) {
            // The spotter handled this iteration (bare wake → capture).
            return true;
        }
        // Once the trained acoustic spotter is loaded it must remain the
        // always-on path.  The Whisper fallback is a synchronous, ~1 second
        // inference over a 12-second window; running it here stalls the
        // microphone consumer and causes the capture queue to drop the exact
        // audio needed by the wake model.  Whisper is still used for command
        // capture and remains the fallback when no acoustic model is present.
        if self.acoustic.is_some() {
            return true;
        }
        if self.last_scan.elapsed() < self.config.scan_interval {
            return true;
        }
        self.last_scan = Instant::now();
        let window = self.samples(self.config.scan_window);
        if self.buffer.len() < window {
            return true;
        }
        let audio: Vec<f32> = self.buffer[self.buffer.len() - window..].to_vec();
        // Two gates: any speech-level frame (cheap veto for room tone) and
        // the cumulative floor (a click is not an utterance).
        if !self.window_has_speech(&audio) || !self.has_min_speech(&audio, self.config.scan_window)
        {
            return true;
        }
        let transcript = match self.transcriber.transcribe_utterance(&audio) {
            Ok(transcript) => sanitize_transcript(&transcript),
            Err(err) => {
                // Transient: the model hiccuped on one window. Log it like
                // any other operational issue and keep listening — a single
                // bad window is not a product failure.
                eprintln!("voice transcription failed: {err}");
                return true;
            }
        };
        match detect_wake(&transcript) {
            WakeResult::Absent => true,
            WakeResult::Detected { .. } => {
                // The wake phrase is a gesture, never a command. Keep the
                // newest pre-roll instead of clearing: the transcript wake
                // arrives one whisper pass after the utterance, and the
                // command spoken after the phrase lives in this tail. The
                // text-side stripper removes the wake phrase itself.
                eprintln!(
                    "EVO voice: wake heard in transcript; listening… ({} xruns, {} dropped input chunks)",
                    xruns_this_session(),
                    dropped_input_chunks_this_session()
                );
                self.retain_preroll();
                self.begin_command_capture(None)
            }
        }
    }

    /// Feeds any new audio to the acoustic spotter. A score over threshold
    /// is a bare wake: consume the buffer, start command capture, and
    /// report `true` (this iteration is handled). `false` means nothing
    /// acoustic happened and the whisper scan should run.
    fn poll_acoustic_wake(&mut self, accept_wake: bool) -> bool {
        let Some(wake) = self.acoustic.as_mut() else {
            return false;
        };
        if self.buffer.len() <= self.acoustic_fed {
            return false;
        }
        let fresh = self.buffer[self.acoustic_fed..].to_vec();
        self.acoustic_fed = self.buffer.len();
        let score = match wake.score(&fresh) {
            Ok(score) => score,
            Err(err) => {
                // One bad inference must not kill the always-on loop.
                eprintln!("wake model failed: {err}");
                return false;
            }
        };
        let Some(score_value) = score else {
            return false;
        };
        if accept_wake && Instant::now() >= self.wake_rearm_at && score_value >= wake.threshold() {
            eprintln!(
                "EVO voice: wake heard (score {:.3} ≥ {:.3}); listening for the command… ({} xruns, {} dropped input chunks)",
                score_value,
                wake.threshold(),
                xruns_this_session(),
                dropped_input_chunks_this_session()
            );
            // The delivered chunk holds the wake phrase — and often the
            // first words of the command, spoken while the spotter was
            // still deciding. Keep the newest tail (in the buffer, so the
            // `fresh` delivery included) instead of clearing; the driver's
            // wake stripper removes the phrase from the capture text.
            self.retain_preroll();
            return self.begin_command_capture(Some(score_value));
        }
        false
    }

    /// Finishes a pending bare-wake command capture when its window
    /// elapses. While the window is open, each loop iteration emits
    /// `Listening` with the accumulated transcript as the live partial;
    /// the notch and main surface show Evo actively listening.
    fn poll_command_capture(&mut self) -> bool {
        let started = self
            .command_capture_started
            .expect("polled in command-capture mode");
        let elapsed = started.elapsed();
        let at_cap = elapsed >= self.config.command_capture;
        if !at_cap && !self.command_endpoint_reached() {
            self.maybe_submit_partial();
            return true;
        }
        self.command_capture_started = None;
        self.capture_open = false;
        self.wake_rearm_at = Instant::now() + WAKE_REARM_COOLDOWN;
        let audio = trim_final_audio(
            std::mem::take(&mut self.buffer),
            self.samples(MAX_FINAL_AUDIO),
        );
        self.acoustic_fed = 0;
        // A completed command must not remain in the streaming wake model's
        // feature history. Keeping those rows lets the next live audio form
        // a cross-capture window and can reproduce the wake several seconds
        // after the notch closed.
        if let Some(acoustic) = self.acoustic.as_mut() {
            acoustic.reset();
        }
        // Truncation must be visible: samples against wall-clock tells
        // whether the endpoint (or a stall) cut the utterance short.
        eprintln!(
            "EVO voice: command capture {} samples ({:.2}s of audio) in {:.2}s wall-clock ({}); {} xruns, {} dropped input chunks",
            audio.len(),
            audio.len() as f64 / f64::from(NATIVE_SAMPLE_RATE),
            elapsed.as_secs_f64(),
            if at_cap {
                "hard cap"
            } else {
                "silence endpoint"
            },
            xruns_this_session(),
            dropped_input_chunks_this_session()
        );
        self.last_partial = None;
        self.partial_inflight = false;
        // Silence and click-only captures are rejected before whisper so
        // it cannot hallucinate a command, and the final pass is the only
        // one that can dispatch: a partial is display-only feedback and is
        // never promoted to a command when the final pass fails.
        if !self.window_has_speech(&audio)
            || !self.has_min_speech(&audio, self.config.command_capture)
        {
            // Keep the re-arm protection even when the capture contains no
            // usable speech; the residual wake tail must not reopen it.
            return self.emit(VoiceEvent::Outcome(VoiceOutcome::NoCommand));
        }
        // The final runs on the worker: `poll` returns immediately and
        // `drain_transcriptions` emits HeardCommand/NoCommand when the
        // result arrives.
        self.awaiting_final = Some(self.capture_id);
        self.pending_finals.insert(self.capture_id);
        if self
            .job_sender
            .send(TranscriptionJob::Final {
                capture: self.capture_id,
                audio,
            })
            .is_err()
        {
            // The worker is gone (spawn succeeded, so this is a poison
            // case): report honestly instead of hanging the capture.
            self.pending_finals.remove(&self.capture_id);
            self.awaiting_final = self.pending_finals.iter().next().copied();
            return self.emit(VoiceEvent::Outcome(VoiceOutcome::Failed {
                reason: "transcription worker unavailable".to_string(),
            }));
        }
        true
    }

    /// Drains worker results. Returns `false` when the UI sink went away.
    fn drain_transcriptions(&mut self) -> bool {
        loop {
            let result = match self.result_receiver.try_recv() {
                Ok(result) => result,
                Err(std::sync::mpsc::TryRecvError::Empty) => return true,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // The worker thread died (a model panic unwound it) and
                    // no result can ever arrive. An outstanding final must
                    // fail honestly instead of leaving its command silently
                    // unresolved forever.
                    if !self.pending_finals.is_empty() {
                        self.pending_finals.clear();
                        self.awaiting_final = None;
                        return self.emit(VoiceEvent::Outcome(VoiceOutcome::Failed {
                            reason: "transcription worker unavailable".to_string(),
                        }));
                    }
                    return true;
                }
            };
            match result {
                TranscriptionResult::Partial { capture, result } => {
                    if capture == self.capture_id {
                        self.partial_inflight = false;
                    }
                    if !self.capture_open || capture != self.capture_id {
                        // A partial from a closed or replaced capture is
                        // stale display data; drop it.
                        continue;
                    }
                    let partial = match result {
                        Ok(partial) => command_text(&partial),
                        Err(err) => {
                            eprintln!("voice partial transcription failed: {err}");
                            continue;
                        }
                    };
                    if partial.is_empty() || self.last_partial.as_deref() == Some(partial.as_str())
                    {
                        continue;
                    }
                    self.last_partial = Some(partial.clone());
                    if !self.emit(VoiceEvent::Listening {
                        partial: Some(partial),
                    }) {
                        return false;
                    }
                }
                TranscriptionResult::Final { capture, result } => {
                    if !self.pending_finals.remove(&capture) {
                        continue;
                    }
                    if self.awaiting_final == Some(capture) {
                        self.awaiting_final = self.pending_finals.iter().next().copied();
                    }
                    self.wake_rearm_at = Instant::now() + WAKE_REARM_COOLDOWN;
                    let final_text = match result {
                        Ok(text) => text,
                        Err(err) => {
                            eprintln!("voice transcription failed: {err}");
                            String::new()
                        }
                    };
                    let command = command_text(&final_text);
                    let keep_going = if command.is_empty() {
                        self.emit(VoiceEvent::Outcome(VoiceOutcome::NoCommand))
                    } else {
                        self.dispatch_command(&command)
                    };
                    if !keep_going {
                        return false;
                    }
                }
            }
        }
    }

    /// Submits a live partial when the capture has materially new audio
    /// and the throttle elapsed. The transcription happens on the worker,
    /// so the poll loop never waits on whisper.
    fn maybe_submit_partial(&mut self) {
        if !LIVE_PARTIALS {
            return;
        }
        // Partials are an early caption, not a second full decode. Limit
        // them to the first two seconds of a turn so a slow local Whisper
        // pass cannot sit in front of the final command transcription.
        let capture_age = self
            .command_capture_started
            .map(|started| started.elapsed())
            .unwrap_or_default();
        if self.partial_inflight
            || capture_age > Duration::from_secs(2)
            || self.partial_transcribed_at.elapsed() < PARTIAL_INTERVAL
            || self.buffer.len() < self.samples(Duration::from_millis(750))
            || self.buffer.len().saturating_sub(self.partial_audio_len)
                < self.samples(PARTIAL_MIN_NEW_AUDIO)
            || !self.window_has_speech(&self.buffer)
            || !self.has_min_speech(&self.buffer, self.config.command_capture)
        {
            return;
        }
        self.partial_transcribed_at = Instant::now();
        self.partial_audio_len = self.buffer.len();
        self.partial_inflight = true;
        // Partial captions should follow the current words, not repeatedly
        // decode the entire growing command. The final pass still receives
        // the complete capture for accuracy.
        let partial_start = self
            .buffer
            .len()
            .saturating_sub(self.samples(Duration::from_secs(2)));
        if self
            .job_sender
            .send(TranscriptionJob::Partial {
                capture: self.capture_id,
                audio: self.buffer[partial_start..].to_vec(),
            })
            .is_err()
        {
            self.partial_inflight = false;
        }
    }

    /// Keeps the newest [`WAKE_PREROLL`] of the rolling buffer (dropping
    /// older audio) and marks it as already seen by the acoustic spotter.
    ///
    /// Called at every wake transition instead of clearing the buffer: the
    /// wake phrase and any command words already spoken sit in this tail.
    /// Clearing made wake→NoCommand the common case — the capture clock
    /// restarted from the next chunk, and the words around the wake were
    /// never heard. The text-side stripper removes the wake phrase from
    /// the capture transcript afterwards.
    fn retain_preroll(&mut self) {
        let keep = self.samples(WAKE_PREROLL);
        if self.buffer.len() > keep {
            let dropped = self.buffer.len() - keep;
            self.buffer.drain(..dropped);
        }
        // Everything retained sits in the buffer's tail, which the spotter
        // has already consumed; new audio continues from here.
        self.acoustic_fed = self.buffer.len();
    }

    /// `true` when speech was heard *after the wake* and the newest sample
    /// is at least [`COMMAND_ENDPOINT_SILENCE`] past its end. A capture
    /// with no post-wake speech never endpoints early: it waits out the
    /// hard cap, and the wake phrase alone transcribes to nothing (the
    /// final gate sees it, the text-side stripper removes it, `NoCommand`
    /// results).
    ///
    /// Regression fix: only audio past [`Self::capture_start_len`] counts
    /// as speech. The retained pre-roll holds the wake phrase itself, so
    /// the moment the phrase was followed by a pause the pre-roll ended
    /// in silence and the endpointer fired before the person drew breath:
    /// the notch opened and instantly reported no-command (live log:
    /// `command capture 24171 samples (1.51s of audio) in 0.00s
    /// wall-clock (silence endpoint)`), and whisper heard only wake-phrase
    /// fragments ("See you ever", "Hey, Evel."). Requiring post-wake
    /// speech makes the endpointer wait for the command to actually
    /// start.
    fn command_endpoint_reached(&self) -> bool {
        if self.command_capture_started.is_none() {
            return false;
        }
        let start = self.capture_start_len.min(self.buffer.len());
        let post_wake = &self.buffer[start..];
        // Use audio duration rather than wall-clock time. The mic handoff
        // can deliver a queued burst after Whisper work, and tests may feed
        // deterministic audio faster than real time; the endpoint should
        // depend on what was heard, not scheduler timing.
        if post_wake.len() < self.samples(COMMAND_ENDPOINT_GRACE) {
            return false;
        }
        let silence = self.samples(COMMAND_ENDPOINT_SILENCE);
        match self.last_speech_end(post_wake) {
            Some(end) => post_wake.len().saturating_sub(end) >= silence,
            None => false,
        }
    }

    /// End sample index of the last speech-level frame in `audio`.
    fn last_speech_end(&self, audio: &[f32]) -> Option<usize> {
        let frame = self.samples(Duration::from_millis(30));
        if frame == 0 {
            return Some(audio.len());
        }
        let mut last = None;
        let mut end = 0;
        for chunk in audio.chunks(frame) {
            end += chunk.len();
            if frame_rms(chunk) >= self.config.speech_rms_threshold {
                last = Some(end);
            }
        }
        last
    }

    /// Short-time energy gate: `true` when the window carries speech-level
    /// energy anywhere in it. The periodic scan only pays for whisper when
    /// someone is actually speaking; hours of silence stay free.
    fn window_has_speech(&self, audio: &[f32]) -> bool {
        let frame = self.samples(Duration::from_millis(30));
        if frame == 0 {
            return true;
        }
        audio
            .chunks(frame)
            .any(|chunk| frame_rms(chunk) >= self.config.speech_rms_threshold)
    }

    /// Cumulative speech gate: the total duration of frames at/above the
    /// RMS threshold must reach [`MIN_SPEECH`] — not just "any 30 ms
    /// frame". A click plus silence must not reach whisper. The floor is
    /// capped at `window`'s own length: a window physically shorter than
    /// [`MIN_SPEECH`] cannot be asked to contain more speech than that.
    fn has_min_speech(&self, audio: &[f32], window: Duration) -> bool {
        let required = self.samples(MIN_SPEECH.min(window));
        let frame = self.samples(Duration::from_millis(30));
        if frame == 0 || required == 0 {
            return true;
        }
        let mut voiced = 0usize;
        for chunk in audio.chunks(frame) {
            if frame_rms(chunk) >= self.config.speech_rms_threshold {
                voiced += chunk.len();
            }
        }
        voiced >= required
    }

    /// Sends one event; `false` means the UI went away.
    fn emit(&mut self, event: VoiceEvent) -> bool {
        self.observers
            .retain_mut(|observer| observer.observe(&event));
        self.sink.send(event).is_ok()
    }

    /// A transport error becomes an honest `Failed` outcome, never silence.
    /// A question about the work ("what am I working on?") routes to the
    /// ask-path before any semantic resolution — it names no work to
    /// restore, it asks what the works are.
    fn dispatch_command(&mut self, command: &str) -> bool {
        let command = command_text(command);
        if command.is_empty() {
            return self.emit(VoiceEvent::Outcome(VoiceOutcome::NoCommand));
        }
        if !self.emit(VoiceEvent::HeardCommand {
            command: command.clone(),
        }) {
            return false;
        }
        let routed = if crate::trigger::is_ask_intent(&command) {
            self.transport.ask()
        } else {
            self.transport.trigger(&command)
        };
        let outcome = match routed {
            Ok(outcome) => outcome,
            Err(err) => VoiceOutcome::Failed { reason: err },
        };
        self.emit(VoiceEvent::Outcome(outcome))
    }

    fn samples(&self, duration: Duration) -> usize {
        (duration.as_secs_f64() * f64::from(NATIVE_SAMPLE_RATE)) as usize
    }
}

/// Mean-centred RMS of one frame: DC offset is room rumble, not speech.
fn frame_rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let mean = frame.iter().sum::<f32>() / frame.len() as f32;
    (frame.iter().map(|s| (s - mean) * (s - mean)).sum::<f32>() / frame.len() as f32).sqrt()
}

fn trim_final_audio(mut audio: Vec<f32>, max_samples: usize) -> Vec<f32> {
    if max_samples == 0 || audio.len() <= max_samples {
        return audio;
    }
    let start = audio.len() - max_samples;
    audio.drain(..start);
    audio
}

/// Mic underruns counted this process (the mic error callback owns it).
/// Zero without the `audio` feature: there is no stream to underrun.
#[cfg(feature = "audio")]
fn xruns_this_session() -> u64 {
    crate::audio::xruns_this_session()
}

#[cfg(feature = "audio")]
fn dropped_input_chunks_this_session() -> u64 {
    crate::audio::dropped_input_chunks_this_session()
}

#[cfg(not(feature = "audio"))]
fn xruns_this_session() -> u64 {
    0
}

#[cfg(not(feature = "audio"))]
fn dropped_input_chunks_this_session() -> u64 {
    0
}

/// Normalizes a transcript at the driver boundary and removes a wake phrase
/// when it is present in a capture. Acoustic spotting starts capture from a
/// fresh audio tail, but Whisper can still include the tail of "hey evo" in
/// its first partial; that phrase is not part of the user's command.
fn command_text(text: &str) -> String {
    match detect_wake(&sanitize_transcript(text)) {
        WakeResult::Detected { command } => command,
        WakeResult::Absent => sanitize_transcript(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Returns scripted transcripts in order, ignoring the audio.
    struct ScriptTranscriber {
        script: Mutex<Vec<String>>,
    }

    impl ScriptTranscriber {
        fn new(lines: Vec<&str>) -> Self {
            Self {
                script: Mutex::new(lines.into_iter().rev().map(str::to_string).collect()),
            }
        }
    }

    impl UtteranceTranscriber for ScriptTranscriber {
        fn transcribe_utterance(&self, _audio: &[f32]) -> Result<String, String> {
            Ok(self.script.lock().unwrap().pop().unwrap_or_default())
        }
    }

    /// Records trigger phrases; returns a canned outcome.
    struct FakeTransport {
        seen: std::sync::Arc<Mutex<Vec<String>>>,
        outcome: VoiceOutcome,
    }

    impl FakeTransport {
        fn new(outcome: VoiceOutcome) -> Self {
            Self {
                seen: std::sync::Arc::new(Mutex::new(Vec::new())),
                outcome,
            }
        }
    }

    impl TriggerTransport for FakeTransport {
        fn trigger(&self, command: &str) -> Result<VoiceOutcome, String> {
            self.seen.lock().unwrap().push(command.to_string());
            Ok(self.outcome.clone())
        }
        fn ask(&self) -> Result<VoiceOutcome, String> {
            self.seen.lock().unwrap().push("<ASK>".to_string());
            Ok(self.outcome.clone())
        }
    }

    fn test_config() -> VoiceLoopConfig {
        VoiceLoopConfig {
            scan_interval: Duration::ZERO,
            scan_window: Duration::from_millis(50),
            command_capture: Duration::from_millis(1),
            max_buffer: Duration::from_secs(30),
            // Disabled so the scripted tests' zero-filled chunks still
            // reach the transcriber; VAD behavior is tested separately.
            speech_rms_threshold: 0.0,
        }
    }

    /// 0.1 s of silence-shaped audio (contents don't matter to the script).
    fn chunk() -> Vec<f32> {
        vec![0.0; 1600]
    }

    /// `samples` of 440 Hz sine at amplitude 0.1: RMS ≈ 0.07, well above
    /// the default speech gate. Deterministic "someone is talking" audio.
    fn speech(samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|i| 0.1 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16_000.0).sin())
            .collect()
    }

    fn restored_outcome() -> VoiceOutcome {
        VoiceOutcome::Restored {
            title: "Evo stuff".to_string(),
            summary: "Opened 2, raised 0 already open.".to_string(),
            offered: vec![],
        }
    }

    /// Pumps the driver until the next event lands, so tests can await the
    /// async transcription worker without racing it. Bounded: a broken
    /// async path fails the test instead of hanging it.
    fn next_event<Tr, Tp>(
        driver: &mut VoiceDriver<Tr, Tp>,
        events: &Receiver<VoiceEvent>,
    ) -> VoiceEvent
    where
        Tr: UtteranceTranscriber + Send + Sync + 'static,
        Tp: TriggerTransport,
    {
        for _ in 0..200 {
            match events.try_recv() {
                Ok(event) => return event,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    panic!("event sink disconnected while waiting for a voice event")
                }
            }
            assert!(
                driver.poll(),
                "driver stopped while waiting for a voice event"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("timed out waiting for the next voice event");
    }

    /// Scripted transcriber whose call at index `gate_at` blocks on the
    /// worker until the test releases it; `entered` announces when the
    /// worker is holding that call, so a job — partial or final — can be
    /// forced to be genuinely in flight across another driver transition.
    struct GateTranscriber {
        script: Mutex<Vec<String>>,
        calls: AtomicUsize,
        gate_at: usize,
        entered: Sender<()>,
        release: Mutex<Receiver<()>>,
    }

    impl UtteranceTranscriber for GateTranscriber {
        fn transcribe_utterance(&self, _audio: &[f32]) -> Result<String, String> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == self.gate_at {
                let _ = self.entered.send(());
                let _ = self.release.lock().unwrap().recv();
            }
            Ok(self.script.lock().unwrap().pop().unwrap_or_default())
        }
    }

    #[test]
    fn wake_only_opens_capture_and_next_command_emits_outcome() {
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(restored_outcome());
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo", "take me back to the evo stuff"]),
            transport,
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll());
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: None })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );

        driver.ingest(chunk());
        std::thread::sleep(Duration::from_millis(2));
        assert!(driver.poll());

        // The final transcription runs on the worker: pump the driver
        // until its events arrive.
        let heard = next_event(&mut driver, &events);
        assert_eq!(
            heard,
            VoiceEvent::HeardCommand {
                command: "take me back to the evo stuff".to_string()
            }
        );
        let outcome = next_event(&mut driver, &events);
        assert_eq!(outcome, VoiceEvent::Outcome(restored_outcome()));
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn wake_suffix_is_never_dispatched_as_an_accidental_command() {
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(restored_outcome());
        let seen = transport.seen.clone();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo open Claude"]),
            transport,
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll());
        assert!(seen.lock().unwrap().is_empty(), "wake scan dispatched");
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: None })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn a_question_routes_to_the_ask_path_not_semantic_search() {
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(VoiceOutcome::Brief {
            count: 2,
            narration: "First, this is the work where you made X.".to_string(),
        });
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo", "what am i working on"]),
            transport,
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll());
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: None })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );
        driver.ingest(chunk());
        std::thread::sleep(Duration::from_millis(2));
        assert!(driver.poll());

        // The final transcription runs on the worker: pump the driver
        // until its events arrive.
        let heard = next_event(&mut driver, &events);
        assert_eq!(
            heard,
            VoiceEvent::HeardCommand {
                command: "what am i working on".to_string()
            }
        );
        let outcome = next_event(&mut driver, &events);
        match outcome {
            VoiceEvent::Outcome(VoiceOutcome::Brief { count, .. }) => assert_eq!(count, 2),
            other => panic!("expected the ask-path brief, got {other:?}"),
        }
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn silence_produces_no_events() {
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["", "um, background conversation"]),
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll());
        driver.ingest(chunk());
        assert!(driver.poll());
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn bare_wake_captures_the_follow_up_command() {
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(restored_outcome());
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo", "open my writing"]),
            transport,
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll(), "bare wake enters command capture");
        assert_eq!(
            events
                .try_recv()
                .expect("transcript wake announces the gesture"),
            VoiceEvent::WakeGesture { score: None }
        );
        assert_eq!(
            events
                .try_recv()
                .expect("listening starts with the capture"),
            VoiceEvent::Listening { partial: None }
        );
        assert!(events.try_recv().is_err(), "no events until follow-up");

        // The follow-up audio arrives, then the capture window elapses.
        driver.ingest(chunk());
        std::thread::sleep(Duration::from_millis(2));
        assert!(driver.poll());

        // The final transcription runs on the worker: pump the driver
        // until its events arrive.
        let heard = next_event(&mut driver, &events);
        assert_eq!(
            heard,
            VoiceEvent::HeardCommand {
                command: "open my writing".to_string()
            }
        );
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(restored_outcome())
        );
    }

    #[test]
    fn bare_wake_recovers_command_from_the_same_audio_window() {
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(restored_outcome());
        let mut driver = VoiceDriver::new(
            // The first pass hears only the wake; the completed capture hears
            // the full utterance from the retained rolling buffer.
            ScriptTranscriber::new(vec!["hey evo", "hey evo open my writing"]),
            transport,
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll(), "bare wake enters command capture");
        std::thread::sleep(Duration::from_millis(2));
        driver.ingest(chunk());
        assert!(driver.poll(), "same-window follow-up is dispatched");

        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: None })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );
        // The final transcription runs on the worker: pump the driver
        // until its events arrive.
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::HeardCommand {
                command: "open my writing".to_string()
            }
        );
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(restored_outcome())
        );
    }

    #[test]
    fn bare_wake_with_silence_follow_up_reports_no_command() {
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo", ""]),
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll());
        assert_eq!(
            events.try_recv().expect("WakeGesture"),
            VoiceEvent::WakeGesture { score: None }
        );
        assert_eq!(
            events.try_recv().expect("Listening"),
            VoiceEvent::Listening { partial: None }
        );
        driver.ingest(chunk());
        std::thread::sleep(Duration::from_millis(2));
        assert!(driver.poll());

        // The final transcription runs on the worker: pump the driver
        // until its outcome arrives.
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(VoiceOutcome::NoCommand)
        );
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn silence_exit_rearms_wake_after_cooldown_instead_of_retriggering_immediately() {
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo"]),
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        )
        .with_acoustic_wake(Box::new(ScriptWake {
            scores: Mutex::new(vec![Some(0.9), None, Some(0.9)]),
            threshold: 0.5,
        }));

        driver.ingest(chunk());
        assert!(driver.poll(), "the first wake opens the capture");
        driver.ingest(chunk());
        std::thread::sleep(Duration::from_millis(2));
        assert!(driver.poll(), "the silent capture closes");
        assert!(matches!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: Some(0.9) })
        ));
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(VoiceOutcome::NoCommand)
        );
        assert!(
            driver.wake_rearm_at > Instant::now(),
            "a no-command capture must keep wake re-arm protection"
        );

        driver.ingest(chunk());
        assert!(driver.poll(), "the residual tail is ignored during cooldown");
        assert!(
            !matches!(events.try_recv(), Ok(VoiceEvent::WakeGesture { .. })),
            "the same tail must not reopen the notch"
        );
    }

    /// Regression: a wake followed by a pause leaves the retained pre-roll
    /// ending in silence, and the endpointer used to read that as "speech
    /// then silence" and close the capture the instant it opened — the
    /// notch opened and instantly reported no-command (live log:
    /// `command capture 24171 samples (1.51s of audio) in 0.00s
    /// wall-clock (silence endpoint)`). The endpointer must not fire until
    /// speech arrives *after* the wake.
    #[test]
    fn endpointer_never_fires_on_preroll_silence_alone() {
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(restored_outcome());
        let seen = transport.seen.clone();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["open my writing"]),
            transport,
            sink,
            VoiceLoopConfig {
                command_capture: Duration::from_secs(6),
                speech_rms_threshold: 0.006,
                ..test_config()
            },
        )
        .with_acoustic_wake(Box::new(ScriptWake {
            scores: Mutex::new(vec![Some(0.9)]),
            threshold: 0.5,
        }));

        // "hey evo" (0.6 s) followed by a pause (1.2 s): the person waiting
        // before speaking the command. The wake fires on the first poll and
        // retains the newest 2 s — the phrase tail and the pause, so the
        // pre-roll's last speech sits > 1 s before its end.
        let mut opening = speech(9_600);
        opening.extend(vec![0.0f32; 19_200]);
        driver.ingest(opening);
        assert!(driver.poll(), "acoustic wake opens the capture");
        // Keep the display-only partial path out of the scripted transcriber.
        driver.partial_transcribed_at =
            Instant::now().checked_add(Duration::from_secs(60)).unwrap();
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: Some(0.9) })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );

        // 1 s of post-wake silence arms the old bug exactly: pre-roll
        // speech ended more than 1 s before the newest sample. With no
        // post-wake speech the capture must stay open and silent.
        driver.ingest(vec![0.0; 16_000]);
        for _ in 0..20 {
            assert!(driver.poll(), "the capture stays open through the pause");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            events.try_recv().is_err(),
            "pre-roll silence alone must not close the capture"
        );

        // The person finally speaks; speech after the wake plus 1 s of
        // trailing silence ends the capture and dispatches the command.
        let mut follow_up = speech(12_800);
        follow_up.extend(vec![0.0f32; 16_000]);
        driver.ingest(follow_up);
        assert!(driver.poll(), "post-wake speech then silence endpoints");

        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::HeardCommand {
                command: "open my writing".to_string()
            }
        );
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(restored_outcome())
        );
        assert_eq!(seen.lock().unwrap().as_slice(), ["open my writing"]);
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn placeholder_after_acoustic_wake_does_not_dispatch() {
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(restored_outcome());
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["[ Inaudible ]"]),
            transport,
            sink,
            test_config(),
        )
        .with_acoustic_wake(Box::new(ScriptWake {
            scores: Mutex::new(vec![Some(0.9)]),
            threshold: 0.5,
        }));
        driver.ingest(chunk());
        assert!(driver.poll(), "acoustic wake starts capture");
        std::thread::sleep(Duration::from_millis(2));
        driver.ingest(chunk());
        assert!(driver.poll(), "placeholder capture remains healthy");

        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: Some(0.9) })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );
        // The final transcription runs on the worker: pump the driver
        // until its outcome arrives.
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(VoiceOutcome::NoCommand)
        );
        assert!(events.try_recv().is_err(), "placeholder was not dispatched");
    }

    #[test]
    fn acoustic_capture_uses_spoken_text_and_strips_a_repeated_wake() {
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo open my writing"]),
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        )
        .with_acoustic_wake(Box::new(ScriptWake {
            scores: Mutex::new(vec![Some(0.9)]),
            threshold: 0.5,
        }));
        driver.ingest(chunk());
        assert!(driver.poll(), "acoustic wake starts capture");
        std::thread::sleep(Duration::from_millis(2));
        driver.ingest(chunk());
        assert!(driver.poll(), "capture dispatches after its window");

        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: Some(0.9) })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );
        // The final transcription runs on the worker: pump the driver
        // until its event arrives.
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::HeardCommand {
                command: "open my writing".to_string()
            }
        );
    }

    #[test]
    fn transport_failure_becomes_an_honest_failed_outcome() {
        struct BrokenTransport;
        impl TriggerTransport for BrokenTransport {
            fn trigger(&self, _command: &str) -> Result<VoiceOutcome, String> {
                Err("daemon unreachable".to_string())
            }
            fn ask(&self) -> Result<VoiceOutcome, String> {
                Err("daemon unreachable".to_string())
            }
        }
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo", "open it"]),
            BrokenTransport,
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll());
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: None })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );
        driver.ingest(chunk());
        std::thread::sleep(Duration::from_millis(2));
        assert!(driver.poll());
        // The final transcription runs on the worker: pump the driver
        // until its events arrive.
        assert!(matches!(
            next_event(&mut driver, &events),
            VoiceEvent::HeardCommand { .. }
        ));
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(VoiceOutcome::Failed {
                reason: "daemon unreachable".to_string()
            })
        );
    }

    #[test]
    fn explicit_command_preserves_ambiguity_without_a_second_transport_call() {
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(VoiceOutcome::Ambiguous {
            candidates: vec![
                crate::restore::VoiceCandidate::work("1", "Alpha"),
                crate::restore::VoiceCandidate::work("2", "Beta"),
            ],
        });
        let seen = transport.seen.clone();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo", "open the notes"]),
            transport,
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll());
        driver.ingest(chunk());
        std::thread::sleep(Duration::from_millis(2));
        assert!(driver.poll());
        assert!(matches!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { .. })
        ));
        assert!(matches!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { .. })
        ));
        // The final transcription runs on the worker: pump the driver until
        // its events arrive, then the transport side effect is observable.
        assert!(matches!(
            next_event(&mut driver, &events),
            VoiceEvent::HeardCommand { command } if command == "open the notes"
        ));
        assert!(matches!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(VoiceOutcome::Ambiguous { candidates }) if candidates.len() == 2
        ));
        assert_eq!(seen.lock().unwrap().as_slice(), ["open the notes"]);
    }

    #[test]
    fn dropped_sink_stops_the_loop() {
        let (sink, _events) = std::sync::mpsc::channel::<VoiceEvent>();
        drop(_events);
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["hey evo open it"]),
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(!driver.poll(), "sink gone → owner stops the thread");
    }

    #[test]
    fn transcription_failure_does_not_kill_the_loop() {
        struct FlakyTranscriber;
        impl UtteranceTranscriber for FlakyTranscriber {
            fn transcribe_utterance(&self, _audio: &[f32]) -> Result<String, String> {
                Err("model hiccup".to_string())
            }
        }
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            FlakyTranscriber,
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll(), "transient failure keeps the loop alive");
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn silent_window_skips_the_transcriber() {
        let (sink, events) = std::sync::mpsc::channel();
        let transcriber = std::sync::Arc::new(ScriptTranscriber::new(vec!["hey evo open it"]));
        let mut driver = VoiceDriver::new(
            transcriber.clone(),
            FakeTransport::new(restored_outcome()),
            sink,
            VoiceLoopConfig {
                speech_rms_threshold: 0.006,
                ..test_config()
            },
        );
        driver.ingest(chunk());
        assert!(driver.poll());
        assert!(events.try_recv().is_err(), "silence never triggers");
        assert_eq!(
            transcriber.script.lock().unwrap().len(),
            1,
            "transcriber was never called on a silent window"
        );
    }

    #[test]
    fn speech_level_window_still_transcribes() {
        let (sink, _events) = std::sync::mpsc::channel();
        let transcriber = std::sync::Arc::new(ScriptTranscriber::new(vec!["hey evo open it"]));
        let mut driver = VoiceDriver::new(
            transcriber.clone(),
            FakeTransport::new(restored_outcome()),
            sink,
            VoiceLoopConfig {
                speech_rms_threshold: 0.006,
                ..test_config()
            },
        );
        // 440 Hz sine at amplitude 0.1: RMS ≈ 0.07, well above the gate.
        let speech: Vec<f32> = (0..1600)
            .map(|i| 0.1 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16_000.0).sin())
            .collect();
        driver.ingest(speech);
        assert!(driver.poll());
        assert_eq!(
            transcriber.script.lock().unwrap().len(),
            0,
            "speech passed the gate and consumed the script"
        );
    }

    /// Acoustic spotter fake: scores every chunk with a scripted value.
    struct ScriptWake {
        scores: Mutex<Vec<Option<f32>>>,
        threshold: f32,
    }

    impl AcousticWake for ScriptWake {
        fn score(&mut self, _audio: &[f32]) -> Result<Option<f32>, String> {
            Ok(self.scores.lock().unwrap().pop().flatten())
        }
        fn threshold(&self) -> f32 {
            self.threshold
        }
    }

    #[test]
    fn acoustic_wake_over_threshold_starts_command_capture() {
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["open my writing"]),
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        )
        .with_acoustic_wake(Box::new(ScriptWake {
            scores: Mutex::new(vec![Some(0.9)]),
            threshold: 0.5,
        }));
        driver.ingest(chunk());
        assert!(driver.poll());
        assert_eq!(
            events
                .try_recv()
                .expect("WakeGesture when the spotter fires"),
            VoiceEvent::WakeGesture { score: Some(0.9) }
        );
        assert_eq!(
            events
                .try_recv()
                .expect("Listening starts with the capture"),
            VoiceEvent::Listening { partial: None }
        );
        assert!(events.try_recv().is_err(), "no more events until follow-up");

        // The capture window elapses; the follow-up becomes the command.
        std::thread::sleep(Duration::from_millis(2));
        driver.ingest(chunk());
        assert!(driver.poll());
        // The final transcription runs on the worker: pump the driver
        // until its event arrives.
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::HeardCommand {
                command: "open my writing".to_string()
            }
        );
    }

    /// An accessory listener recording the event stream the driver emits.
    struct RecordingObserver {
        seen: Mutex<Vec<String>>,
    }

    impl RecordingObserver {
        fn new() -> Self {
            Self {
                seen: Mutex::new(Vec::new()),
            }
        }
        fn names(&self) -> Vec<String> {
            self.seen.lock().unwrap().clone()
        }
        fn record(&self, event: &VoiceEvent) -> bool {
            let name = match event {
                VoiceEvent::WakeGesture { .. } => "WakeGesture",
                VoiceEvent::Listening { .. } => "Listening",
                VoiceEvent::HeardCommand { .. } => "HeardCommand",
                VoiceEvent::Outcome(_) => "Outcome",
            };
            self.seen.lock().unwrap().push(name.to_string());
            true
        }
    }

    impl VoiceEventObserver for RecordingObserver {
        fn observe(&mut self, event: &VoiceEvent) -> bool {
            let name = match event {
                VoiceEvent::WakeGesture { .. } => "WakeGesture",
                VoiceEvent::Listening { .. } => "Listening",
                VoiceEvent::HeardCommand { .. } => "HeardCommand",
                VoiceEvent::Outcome(_) => "Outcome",
            };
            self.seen.lock().unwrap().push(name.to_string());
            true
        }
    }

    #[test]
    fn observer_sees_the_same_event_stream_as_the_ui() {
        let (sink, events) = std::sync::mpsc::channel();
        let observer = std::sync::Arc::new(RecordingObserver::new());
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec!["open my writing"]),
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        )
        .with_acoustic_wake(Box::new(ScriptWake {
            scores: Mutex::new(vec![Some(0.8)]),
            threshold: 0.5,
        }))
        .with_observer(Box::new(SharedObserver(observer.clone())));
        driver.ingest(chunk());
        assert!(driver.poll());
        // The capture window elapses; the follow-up becomes the command on
        // the worker. Pump until the whole stream has fanned out.
        std::thread::sleep(Duration::from_millis(2));
        driver.ingest(chunk());
        assert!(driver.poll());
        for _ in 0..200 {
            if observer.names().len() >= 4 {
                break;
            }
            assert!(driver.poll());
            std::thread::sleep(Duration::from_millis(1));
        }
        // The notch observer saw the exact stream the UI sink consumed.
        assert_eq!(
            observer.names(),
            vec![
                "WakeGesture".to_string(),
                "Listening".to_string(),
                "HeardCommand".to_string(),
                "Outcome".to_string(),
            ]
        );
        for name in ["WakeGesture", "Listening", "HeardCommand", "Outcome"] {
            events.try_recv().expect("UI sees the same stream");
            assert!(!name.is_empty());
        }
        assert!(events.try_recv().is_err());
    }

    /// `Arc` wrapper so both the driver and the test hold the recorder.
    struct SharedObserver(std::sync::Arc<RecordingObserver>);

    impl VoiceEventObserver for SharedObserver {
        fn observe(&mut self, event: &VoiceEvent) -> bool {
            RecordingObserver::record(&self.0, event)
        }
    }

    #[test]
    fn acoustic_wake_below_threshold_does_nothing() {
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec![]),
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        )
        .with_acoustic_wake(Box::new(ScriptWake {
            scores: Mutex::new(vec![Some(0.2)]),
            threshold: 0.5,
        }));
        driver.ingest(chunk());
        assert!(driver.poll());
        assert!(events.try_recv().is_err(), "sub-threshold score is silent");
    }

    #[test]
    fn acoustic_spotter_error_does_not_kill_the_loop() {
        struct BrokenWake;
        impl AcousticWake for BrokenWake {
            fn score(&mut self, _audio: &[f32]) -> Result<Option<f32>, String> {
                Err("onnx hiccup".to_string())
            }
            fn threshold(&self) -> f32 {
                0.5
            }
        }
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec![]),
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        )
        .with_acoustic_wake(Box::new(BrokenWake));
        driver.ingest(chunk());
        assert!(driver.poll(), "spotter failure keeps the loop alive");
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn buffer_is_capped() {
        let (sink, _events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            ScriptTranscriber::new(vec![]),
            FakeTransport::new(restored_outcome()),
            sink,
            VoiceLoopConfig {
                max_buffer: Duration::from_millis(100),
                ..test_config()
            },
        );
        for _ in 0..10 {
            driver.ingest(chunk());
        }
        assert!(driver.buffer.len() <= 1600, "buffer capped at max_buffer");
    }

    /// A wake arriving while the previous capture's final is still in
    /// flight is consumed by the spotter but cannot open a second session or
    /// cross-wire a late result into the notch.
    #[test]
    fn a_wake_while_a_final_is_in_flight_is_disarmed() {
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(restored_outcome());
        let seen = transport.seen.clone();
        let mut driver = VoiceDriver::new(
            GateTranscriber {
                // The worker's first call (session 1's final) is held until
                // the test releases it; a second wake arrives while it is
                // still in flight and must start a new capture.
                script: Mutex::new(
                    ["open the notes", "close the notes"]
                        .into_iter()
                        .rev()
                        .map(str::to_string)
                        .collect(),
                ),
                calls: AtomicUsize::new(0),
                gate_at: 0,
                entered: entered_tx,
                release: Mutex::new(release_rx),
            },
            transport,
            sink,
            test_config(),
        )
        .with_acoustic_wake(Box::new(ScriptWake {
            scores: Mutex::new(vec![Some(0.9), None, Some(0.9)]),
            threshold: 0.5,
        }));

        // Session 1: an acoustic wake opens capture 1.
        driver.ingest(chunk());
        assert!(driver.poll(), "session 1 wake");
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: Some(0.9) })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );

        // Capture 1 closes: its final reaches the worker and is held there.
        std::thread::sleep(Duration::from_millis(2));
        driver.ingest(chunk());
        assert!(driver.poll(), "capture 1 closes");
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker picked up session 1's final");

        // A second wake arrives while session 1's final is still in flight
        // and starts session 2 immediately after the normal re-arm window.
        std::thread::sleep(Duration::from_millis(2_100));
        driver.ingest(chunk());
        assert!(driver.poll(), "pending final remains active");
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: Some(0.9) })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );

        // Release session 1. Its result is the only valid result and is
        // dispatched once; no stale duplicate remains queued.
        release_tx.send(()).expect("release the held final");
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::HeardCommand {
                command: "open the notes".to_string()
            }
        );
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(restored_outcome())
        );
        assert_eq!(seen.lock().unwrap().as_slice(), ["open the notes"]);
        assert!(events.try_recv().is_err());
    }

    /// Worker death must surface as an honest `Failed` outcome — both for
    /// a final already in flight when the worker died and for the next
    /// capture's final submission — never as a hang or silence.
    #[test]
    fn worker_death_is_reported_honestly_and_never_hangs() {
        struct DyingTranscriber {
            calls: AtomicUsize,
        }
        impl UtteranceTranscriber for DyingTranscriber {
            fn transcribe_utterance(&self, _audio: &[f32]) -> Result<String, String> {
                // Call 0 is the poll-path wake scan; the worker's first
                // call (the final) panics and kills its thread.
                if self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
                    panic!("model exploded");
                }
                Ok("hey evo".to_string())
            }
        }
        let (sink, events) = std::sync::mpsc::channel();
        let mut driver = VoiceDriver::new(
            DyingTranscriber {
                calls: AtomicUsize::new(0),
            },
            FakeTransport::new(restored_outcome()),
            sink,
            test_config(),
        );

        // Phase 1: the worker dies while transcribing the final.
        driver.ingest(chunk());
        assert!(driver.poll(), "wake heard, capture opens");
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: None })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );
        std::thread::sleep(Duration::from_millis(2));
        driver.ingest(chunk());
        assert!(driver.poll(), "final is handed to the doomed worker");
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(VoiceOutcome::Failed {
                reason: "transcription worker unavailable".to_string()
            })
        );
        assert!(events.try_recv().is_err());

        // Phase 2: the worker is gone; the next capture's final submission
        // must also fail honestly instead of stalling.
        driver.ingest(chunk());
        assert!(driver.poll(), "next wake still works");
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: None })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );
        std::thread::sleep(Duration::from_millis(2));
        driver.ingest(chunk());
        assert!(driver.poll(), "capture closes");
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(VoiceOutcome::Failed {
                reason: "transcription worker unavailable".to_string()
            })
        );
        assert!(events.try_recv().is_err());
    }

    /// Partials are display-only: a partial result read after its capture
    /// closed (with the final already submitted) must be ignored — it may
    /// never emit `HeardCommand`, an `Outcome`, or a stray partial.
    #[test]
    fn a_late_partial_result_is_ignored_after_the_final() {
        if !LIVE_PARTIALS {
            // Live partials are disabled in the fast path; there is no
            // partial result to become stale or delay the final.
            return;
        }
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (sink, events) = std::sync::mpsc::channel();
        let transport = FakeTransport::new(restored_outcome());
        let seen = transport.seen.clone();
        let mut driver = VoiceDriver::new(
            GateTranscriber {
                // The worker's first call is the partial; it is held until
                // the capture has closed and queued its final behind it.
                script: Mutex::new(
                    ["hey evo", "partial words", "open my writing"]
                        .into_iter()
                        .rev()
                        .map(str::to_string)
                        .collect(),
                ),
                calls: AtomicUsize::new(0),
                gate_at: 1,
                entered: entered_tx,
                release: Mutex::new(release_rx),
            },
            transport,
            sink,
            test_config(),
        );
        driver.ingest(chunk());
        assert!(driver.poll(), "wake heard, capture opens");
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::WakeGesture { score: None })
        );
        assert_eq!(
            events.try_recv(),
            Ok(VoiceEvent::Listening { partial: None })
        );

        // Meet the partial gates by hand instead of sleeping 900 ms: keep
        // the capture open while enough audio accumulates, then submit the
        // partial on the next poll.
        driver.command_capture_started = Instant::now().checked_add(Duration::from_secs(1));
        for _ in 0..8 {
            driver.ingest(chunk());
        }
        driver.partial_transcribed_at = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        assert!(driver.poll(), "partial submitted while the capture is open");
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker picked up the partial");
        assert!(events.try_recv().is_err(), "the partial is still held");

        // Close the capture: the final queues behind the held partial.
        driver.command_capture_started = Instant::now().checked_sub(Duration::from_millis(1));
        assert!(driver.poll(), "capture closes and queues the final");
        release_tx.send(()).expect("release the held partial");

        // The held partial lands after the capture closed; only the final
        // may speak.
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::HeardCommand {
                command: "open my writing".to_string()
            }
        );
        assert_eq!(
            next_event(&mut driver, &events),
            VoiceEvent::Outcome(restored_outcome())
        );
        assert_eq!(seen.lock().unwrap().as_slice(), ["open my writing"]);
        assert!(events.try_recv().is_err());
    }
}
