//! Microphone capture via cpal, on-device, always delivered at 16 kHz.
//!
//! Captures f32 audio from the default input device at whatever rate the
//! hardware runs (typically 48 kHz) and **resamples to 16 kHz** —
//! whisper's native format and the wake spotter's feed. Streaming
//! resampling here is what makes the voice loop actually work on real
//! Macs: the old hard-fail ("mic runs at 48000Hz, need 16000Hz") killed
//! the whole voice thread at startup on every modern laptop, so "Hey
//! Evo" could never trigger.
//!
//! The resampler is a small linear interpolator over a rolling tail of
//! source samples (see [`LinearResampler`]): polyphase/filterbank
//! quality is unnecessary at 3:1 for speech rates. The cpal callback only
//! copies raw frames into a bounded queue; resampling happens on a worker so
//! model/queue work cannot starve the real-time audio thread.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Whisper and wake-model native sample rate.
const TARGET_RATE: u32 = 16_000;

/// Underruns (xruns) reported by the input stream this session. The error
/// callback runs on the IO thread, so it only bumps this counter and
/// rate-limits its logging; the voice driver reads the total when a wake
/// fires or a command capture completes.
static XRUNS_THIS_SESSION: AtomicU64 = AtomicU64::new(0);

/// Raw input chunks rejected because the bounded hand-off queue was full.
/// Unlike an unbounded channel, this makes overload visible and prevents stale
/// audio from accumulating until a later command is transcribed too late.
static DROPPED_INPUT_CHUNKS_THIS_SESSION: AtomicU64 = AtomicU64::new(0);

/// A normal macOS input callback is a few to a few dozen milliseconds. This
/// gives the resampler roughly half a second of burst tolerance without
/// retaining minutes of audio or blocking the cpal callback.
const RAW_CHUNK_QUEUE_CAPACITY: usize = 64;

/// Total mic xruns counted since process start.
pub(crate) fn xruns_this_session() -> u64 {
    XRUNS_THIS_SESSION.load(Ordering::Relaxed)
}

/// Raw chunks dropped at the real-time boundary this process lifetime.
pub(crate) fn dropped_input_chunks_this_session() -> u64 {
    DROPPED_INPUT_CHUNKS_THIS_SESSION.load(Ordering::Relaxed)
}

/// A live microphone capture session delivering 16 kHz mono f32.
pub struct Mic {
    _stream: cpal::Stream,
    receiver: mpsc::Receiver<Vec<f32>>,
    // Dropping the handle detaches the worker after the stream/queues close;
    // no shutdown join is allowed on the audio owner path.
    _resampler_thread: std::thread::JoinHandle<()>,
}

impl Mic {
    /// Opens the default input device and starts capture, resampling
    /// whatever the hardware runs at down to 16 kHz mono f32.
    ///
    /// Returns the [`Mic`] and starts capture immediately. Drop the
    /// [`Mic`] to stop.
    pub fn open() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("no default input device found")?;
        let device_name = device
            .description()
            .map(|description| description.name().to_owned())
            .unwrap_or_else(|_| "<unknown>".to_string());

        let config = find_config(&device)?;
        // One startup line naming the exact stream shape, so a mic that
        // silently negotiated something unexpected (or fell onto the
        // resample path) is visible in the log instead of inferred.
        eprintln!(
            "EVO voice: mic capture on {device_name}: {} Hz, {} channel(s), buffer {:?}; {}",
            config.sample_rate,
            config.channels,
            config.buffer_size,
            if config.sample_rate == TARGET_RATE {
                "native 16 kHz (no resample)"
            } else {
                "linear resample → 16 kHz"
            }
        );

        // The real-time callback must not resample, allocate a large output,
        // or block behind the voice loop. It only copies one hardware chunk
        // into this bounded queue. The worker below owns the linear resampler
        // and forwards 16 kHz chunks to the existing consumer API.
        let source_rate = config.sample_rate;
        let (raw_tx, raw_rx) = mpsc::sync_channel::<Vec<f32>>(RAW_CHUNK_QUEUE_CAPACITY);
        let (tx, rx) = mpsc::channel();
        let resampler_thread = std::thread::Builder::new()
            .name("evo-audio-resampler".to_string())
            .spawn(move || {
                let mut resampler = LinearResampler::new(source_rate, TARGET_RATE);
                while let Ok(input) = raw_rx.recv() {
                    let output = resampler.push(&input);
                    if !output.is_empty() && tx.send(output).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| format!("failed to start audio resampler: {e}"))?;
        let err_fn = move |err| {
            let count = XRUNS_THIS_SESSION.fetch_add(1, Ordering::Relaxed) + 1;
            // The IO thread is already starved when this fires; log the
            // first few and every hundredth so the callback stays cheap
            // while a full count is still reported at wake/command time.
            if count <= 3 || count % 100 == 0 {
                eprintln!("EVO voice: mic stream error (xrun #{count}): {err}");
            }
        };

        let stream = device
            .build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Copy only the hardware-sized raw chunk. `try_send` is
                    // essential: a full queue must drop explicitly rather
                    // than block the cpal callback and create an xrun.
                    if raw_tx.try_send(data.to_vec()).is_err() {
                        let count =
                            DROPPED_INPUT_CHUNKS_THIS_SESSION.fetch_add(1, Ordering::Relaxed) + 1;
                        if count <= 3 || count % 100 == 0 {
                            eprintln!(
                                "EVO voice: raw audio queue full; dropped input chunk #{count}"
                            );
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("failed to build input stream on {device_name}: {e}"))?;
        stream
            .play()
            .map_err(|e| format!("failed to start mic stream: {e}"))?;

        Ok(Self {
            _stream: stream,
            receiver: rx,
            _resampler_thread: resampler_thread,
        })
    }

    /// Non-blocking drain: collects all pending 16 kHz audio chunks into
    /// one buffer.
    pub fn drain(&self) -> Vec<f32> {
        let mut out = Vec::new();
        while let Ok(chunk) = self.receiver.try_recv() {
            out.extend_from_slice(&chunk);
        }
        out
    }

    /// Blocking receive: waits for the next audio chunk (up to `timeout`).
    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Vec<f32>, mpsc::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

/// Finds an f32 mono input config at any rate; 16 kHz natively is the
/// only case that skips the resampler.
fn find_config(device: &cpal::Device) -> Result<cpal::StreamConfig, String> {
    // Prefer native 16 kHz mono f32 — the exact pipeline, no resampling.
    let target = cpal::StreamConfig {
        channels: 1,
        sample_rate: TARGET_RATE,
        buffer_size: cpal::BufferSize::Default,
    };
    // Some CoreAudio devices report `Unknown property` while enumerating
    // supported formats even though `default_input_config()` works.  Treat
    // enumeration as an optimisation, not a startup requirement: the old
    // code returned here and caused launchd to restart evo-agent forever.
    let supports_native_target = device
        .supported_input_configs()
        .map(|mut configs| {
            configs.any(|c| {
                c.channels() == 1
                    && c.sample_format() == cpal::SampleFormat::F32
                    && c.min_sample_rate() <= TARGET_RATE
                    && c.max_sample_rate() >= TARGET_RATE
            })
        })
        .unwrap_or(false);
    if supports_native_target {
        return Ok(target);
    }

    // Any other rate is fine: we resample downstream.
    let default = device
        .default_input_config()
        .map_err(|e| format!("mic has no default input config: {e}"))?;
    if default.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!(
            "mic default format is {:?}, need f32 (resampling handles rate conversion only)",
            default.sample_format()
        ));
    }
    if default.channels() != 1 {
        // Mono-configured capture from a multi-channel device is cpal's
        // job: it downmixes when the config asks for 1 channel.
        return Ok(cpal::StreamConfig {
            channels: 1,
            sample_rate: default.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        });
    }
    Ok(default.into())
}

/// Streaming linear resampler from `source_rate` to `target_rate`.
///
/// Holds a one-sample tail between calls so output is continuous
/// across chunk boundaries. Emits whole output frames; the sub-sample
/// phase carries forward so average throughput is exactly the rate
/// ratio (no drift over hours of capture).
pub struct LinearResampler {
    source_rate: f64,
    target_rate: f64,
    /// Reused source scratch: the carried tail of the previous push sits
    /// at the front and each push appends the new input in place. Keeps
    /// the audio callback allocation-free apart from the output buffer it
    /// hands to the channel.
    stream: Vec<f32>,
    /// How many leading samples of `stream` are the carried tail.
    tail_len: usize,
    /// Fractional position into the source stream for the next output.
    phase: f64,
}

impl LinearResampler {
    pub fn new(source_rate: u32, target_rate: u32) -> Self {
        Self {
            source_rate: f64::from(source_rate.max(1)),
            target_rate: f64::from(target_rate.max(1)),
            stream: Vec::new(),
            tail_len: 0,
            phase: 0.0,
        }
    }

    /// Pushes source samples; returns the 16 kHz output produced.
    ///
    /// Only the returned buffer is allocated per call (it is moved into
    /// the mic channel); the interpolation scratch is reused across calls.
    pub fn push(&mut self, input: &[f32]) -> Vec<f32> {
        if (self.source_rate - self.target_rate).abs() < f64::EPSILON {
            return input.to_vec();
        }
        let step = self.source_rate / self.target_rate;
        self.stream.truncate(self.tail_len);
        self.stream.extend_from_slice(input);
        let mut out = Vec::with_capacity((self.stream.len() as f64 / step) as usize + 1);

        // Interpolate at successive phase positions; advance phase by the
        // rate ratio and consume the stream as it's passed.
        while self.phase + 1.0 < self.stream.len() as f64 {
            let pos = self.phase;
            let i = pos.floor() as usize;
            let frac = pos - i as f64;
            let a = self.stream[i];
            let b = self.stream[i + 1];
            out.push(a + (b - a) * frac as f32);
            self.phase += step;
        }
        // Keep enough of the stream for the next call: everything from
        // the phase's integer position onwards, moved to the front.
        let keep_from = (self.phase.floor() as usize).min(self.stream.len().saturating_sub(1));
        self.stream.copy_within(keep_from.., 0);
        self.tail_len = self.stream.len() - keep_from;
        self.stream.truncate(self.tail_len);
        self.phase -= keep_from as f64;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_rates_match() {
        let mut r = LinearResampler::new(16_000, 16_000);
        let input = vec![0.1, 0.2, 0.3];
        assert_eq!(r.push(&input), input);
    }

    #[test]
    fn resampler_rate_ratio_holds_over_many_chunks() {
        // 48 kHz → 16 kHz must produce ~1/3 the samples, chunk-size
        // invariant, no drift: feed 10 × 480 samples (0.1 s each) and
        // expect ≈ 1600 output samples total (10 × 0.1 s × 16 kHz).
        let mut r = LinearResampler::new(48_000, 16_000);
        let mut total = 0usize;
        for _ in 0..10 {
            let chunk = vec![0.5f32; 480];
            total += r.push(&chunk).len();
        }
        assert!(
            (total as i64 - 1600).abs() <= 4,
            "expected ~1600 samples, got {total}"
        );
    }

    #[test]
    fn resampler_affirms_continuity_across_chunks() {
        // A rising ramp resampled per-chunk must remain monotonic: the
        // tail must carry over, not restart.
        let mut r = LinearResampler::new(48_000, 16_000);
        let mut last = 0.0f32;
        for chunk_index in 0..5 {
            let chunk: Vec<f32> = (0..480)
                .map(|i| (chunk_index * 480 + i) as f32 / 48_000.0)
                .collect();
            for s in r.push(&chunk) {
                assert!(s >= last - 1e-6, "resampler output not monotonic");
                last = s;
            }
        }
    }

    // Mic tests need real hardware; run manually.
    // `Mic::open()` on a headless CI box returns "no default input device".
}
