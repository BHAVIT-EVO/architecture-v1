//! Wake-word probe: measure what the trained model scores on this
//! machine's real microphone, so the threshold is tuned against the
//! person's actual voice instead of synthetic expectations.
//!
//! Modes:
//! - `--mic [-s SECS] [--dump out.f32]` — open the mic, stream live
//!   wake scores (honest per-evaluation + per-window best), and
//!   optionally dump the raw 16 kHz f32 audio for offline analysis.
//! - `--selftest` — deterministic checks without a mic: the models load,
//!   silence scores low, a tone scores low. Exit 0 = healthy rig.
//!
//! Examples:
//! ```text
//! cargo run -p evo-voice --features audio,wake-model --bin wake_probe -- --selftest
//! cargo run -p evo-voice --features audio,wake-model --bin wake_probe -- --mic -s 20 --dump /tmp/evo_mic.f32
//! cargo run -p evo-voice --features audio,wake-model --bin wake_probe -- --mic -s 20 -t 0.15
//! ```
//!
//! Threshold: `EVO_WAKE_THRESHOLD` or `-t` overrides the spotter's.

use std::path::PathBuf;
use std::time::Instant;

const RATE: usize = 16_000;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| -> Option<String> {
        let pos = args.iter().position(|a| a == name)?;
        args.get(pos + 1).cloned()
    };
    let threshold_override = flag("-t").and_then(|v| v.parse::<f32>().ok());

    if args.iter().any(|a| a == "--selftest") {
        std::process::exit(selftest(threshold_override));
    }
    if args.iter().any(|a| a == "--mic") {
        let secs: f32 = flag("-s").and_then(|v| v.parse().ok()).unwrap_or(15.0);
        let dump = flag("--dump").map(PathBuf::from);
        if let Err(err) = mic_run(secs, dump, threshold_override) {
            eprintln!("probe failed: {err}");
            std::process::exit(1);
        }
        return;
    }

    eprintln!("usage: wake_probe --mic [-s SECS] [--dump out.f32] [-t THRESHOLD] | --selftest");
    std::process::exit(2);
}

fn load_spotter(threshold_override: Option<f32>) -> Result<evo_voice::CustomWakeSpotter, String> {
    // Models sit beside the built binary, exactly like the desktop loads
    // them; fall back to the crate's testdata for `cargo run` sessions.
    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.to_path_buf()));
    let candidates: Vec<PathBuf> = beside_exe
        .into_iter()
        .chain(std::iter::once(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/wake"),
        ))
        .collect();
    let mut last_err = String::from("no models dir attempted");
    for dir in candidates {
        match evo_voice::CustomWakeSpotter::load(&dir) {
            Ok(spotter) => {
                println!("models loaded from {}", dir.display());
                return Ok(match threshold_override {
                    Some(t) => {
                        println!("threshold override: {t:.3}");
                        spotter.with_threshold(t)
                    }
                    None => {
                        println!(
                            "threshold: env EVO_WAKE_THRESHOLD or default {:.3}",
                            evo_voice::DEFAULT_WAKE_THRESHOLD
                        );
                        match std::env::var("EVO_WAKE_THRESHOLD")
                            .ok()
                            .and_then(|v| v.parse::<f32>().ok())
                        {
                            Some(t) => {
                                println!("EVO_WAKE_THRESHOLD={t:.3}");
                                spotter.with_threshold(t)
                            }
                            None => spotter,
                        }
                    }
                });
            }
            Err(err) => last_err = err,
        }
    }
    Err(last_err)
}

fn selftest(threshold_override: Option<f32>) -> i32 {
    match load_spotter(threshold_override) {
        Ok(mut spotter) => {
            let threshold = spotter.threshold();
            // Deterministic negatives: 3 s silence, then a 220 Hz tone.
            let silence = vec![0.0f32; RATE * 3];
            let tone: Vec<f32> = (0..RATE * 3)
                .map(|i| 0.3 * (2.0 * std::f32::consts::PI * 220.0 * i as f32 / RATE as f32).sin())
                .collect();
            let s_silence = spotter.ingest(&silence).unwrap_or(None);
            let s_tone = spotter.ingest(&tone).unwrap_or(None);
            println!("silence score: {s_silence:?} (threshold {threshold:.3})");
            println!("tone score:    {s_tone:?} (threshold {threshold:.3})");
            let ok =
                s_silence.map_or(true, |s| s < threshold) && s_tone.map_or(true, |s| s < threshold);
            if ok {
                println!("selftest: PASS (negatives score below threshold)");
                0
            } else {
                println!("selftest: FAIL (a negative scored at/above threshold)");
                1
            }
        }
        Err(err) => {
            eprintln!("selftest: FAIL ({err})");
            1
        }
    }
}

fn mic_run(
    secs: f32,
    dump: Option<PathBuf>,
    threshold_override: Option<f32>,
) -> Result<(), String> {
    let mut spotter = load_spotter(threshold_override)?;
    let mic = match evo_voice::Mic::open() {
        Ok(mic) => mic,
        Err(err) => return Err(format!("mic: {err}")),
    };
    // stderr, not stdout: unbuffered under redirection, so live scores
    // are visible however the probe is run.
    let p = |line: String| eprintln!("{line}");
    p(format!(
        "listening {secs}s — say \"hey evo\" a few times, watch the scores."
    ));
    p("Ctrl-C to stop early.".to_string());
    let start = Instant::now();
    let mut raw: Vec<f32> = Vec::new();
    let mut window_best = 0.0f32;
    let mut window_start = Instant::now();
    let mut breaches = 0usize;
    let mut max_seen = 0.0f32;
    let mut total_samples = 0usize;
    while start.elapsed().as_secs_f32() < secs {
        let chunk = mic.drain();
        if chunk.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(20));
            continue;
        }
        total_samples += chunk.len();
        raw.extend_from_slice(&chunk);
        if let Some(score) = spotter.ingest(&chunk)? {
            max_seen = max_seen.max(score);
            window_best = window_best.max(score);
            if score >= spotter.threshold() {
                breaches += 1;
                p(format!(
                    "[{:.1}s] WAKE score {:.3} ≥ threshold {:.3}",
                    start.elapsed().as_secs_f32(),
                    score,
                    spotter.threshold()
                ));
            }
        }
        if window_start.elapsed().as_millis() >= 1000 {
            p(format!(
                "[{:.1}s] samples {total_samples}, window best {:.3}, max {:.3}, threshold {:.3}, breaches {breaches}",
                start.elapsed().as_secs_f32(),
                window_best,
                max_seen,
                spotter.threshold()
            ));
            window_best = 0.0;
            breaches = 0;
            window_start = Instant::now();
        }
    }
    if let Some(path) = dump {
        let bytes: Vec<u8> = raw.iter().flat_map(|s| s.to_ne_bytes()).collect();
        std::fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
        p(format!(
            "dumped {} samples ({:.1}s) to {}",
            raw.len(),
            raw.len() as f32 / RATE as f32,
            path.display()
        ));
    }
    p(format!(
        "done. max score {max_seen:.3} against threshold {:.3} — tune -t to sit between your max-noise and your \"hey evo\" peaks.",
        spotter.threshold()
    ));
    Ok(())
}
