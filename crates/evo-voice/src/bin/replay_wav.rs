//! Test-only replay of a 16 kHz mono WAV through the real transcription
//! pipeline. Requires EVO_TEST_HOME. Prints each stage as JSON.
//!
//! Usage:
//!   EVO_TEST_HOME=/tmp/evo-test cargo run -p evo-voice --features audio --bin replay_wav -- file.wav [model.bin]

use std::env;
use std::fs::File;
use std::io::Read;

fn read_wav(path: &str) -> Vec<f32> {
    let mut f = File::open(path).expect("open wav");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("read wav");
    // Find data chunk
    let mut i = 12;
    while i + 8 <= buf.len() {
        let chunk = &buf[i..i+4];
        let size = u32::from_le_bytes(buf[i+4..i+8].try_into().unwrap()) as usize;
        if chunk == b"data" {
            let start = i + 8;
            let end = start + size;
            let mut out = Vec::new();
            let mut j = start;
            while j + 2 <= end && j + 2 <= buf.len() {
                let s = i16::from_le_bytes(buf[j..j+2].try_into().unwrap());
                out.push(s as f32 / 32768.0);
                j += 2;
            }
            return out;
        }
        i += 8 + size;
    }
    Vec::new()
}

fn main() {
    if env::var_os("EVO_TEST_HOME").is_none() {
        eprintln!("{{\"error\":\"EVO_TEST_HOME required\"}}");
        std::process::exit(1);
    }
    let args: Vec<String> = env::args().collect();
    let wav = args.get(1).expect("wav path");
    let model = args.get(2).cloned().unwrap_or_else(|| "whisper-base.en.bin".to_string());
    let samples = read_wav(wav);
    println!("{{\"stage\":\"loaded\",\"samples\":{},\"seconds\":{:.2}}}", samples.len(), samples.len() as f32 / 16000.0);
    let transcriber = evo_voice::Transcriber::load(&model).expect("load whisper");
    let text = transcriber.transcribe(&samples).expect("transcribe");
    println!("{{\"stage\":\"transcript\",\"text\":{}}}", serde_json_string(&text));
    let normalized = evo_voice::wake::normalize_for_trace(&text);
    println!(
        "{{\"stage\":\"normalized\",\"text\":{}}}",
        serde_json_string(&normalized)
    );
    // replay_wav is deliberately a transcription-boundary probe. It does
    // not invent work candidates: an empty score list is the measured result
    // when no reconstructed works are supplied to this binary.
    println!("{{\"stage\":\"match_scores\",\"scores\":[]}}");
    println!("{{\"stage\":\"matched_works\",\"works\":[]}}");
    println!("{{\"stage\":\"action_chosen\",\"action\":\"none\"}}");
}

fn serde_json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}