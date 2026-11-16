//! Transcribes a raw 16 kHz f32 dump (see `wake_probe --dump`) with the
//! bundled whisper model — the honest check of what the mic actually
//! heard during a calibration run.
//!
//! ```text
//! cargo run -p evo-voice --features audio --bin transcribe_dump -- /tmp/evo_mic_cal2.f32 [target/release/whisper-base.en.bin]
//! ```

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dump = args
        .get(1)
        .expect("usage: transcribe_dump DUMP.f32 [MODEL.bin]");
    let model = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "target/release/whisper-base.en.bin".to_string());
    let bytes = std::fs::read(dump).expect("read dump");
    let floats: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|c| f32::from_ne_bytes(c.try_into().unwrap()))
        .collect();
    println!(
        "loaded {} samples ({:.1}s) from {dump}",
        floats.len(),
        floats.len() as f32 / 16_000.0
    );
    let transcriber = evo_voice::Transcriber::load(&model).expect("load whisper");
    let text = transcriber.transcribe(&floats).expect("transcribe");
    println!("transcript: {text}");
}
