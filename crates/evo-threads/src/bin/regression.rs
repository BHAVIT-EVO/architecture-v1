//! Ground-truth regression harness: replays a real recorded event stream
//! through the evo-threads engine and scores every labeled resource's
//! placement against the founder's hand labels.
//!
//! Usage:
//!   cargo run -p evo-threads --bin regression -- \
//!       --events tools/threads-regression/sample_events.json \
//!       --labels tools/threads-regression/ground_truth_labels.json
//!
//! The events file is a JSON array of arrays:
//!   [timestamp_ms, "Person"|"System"|"Execution",
//!    "Focused"|"Visited"|"Typed"|"Saved"|"Mutated"|"Downloaded",
//!    resource, optional_weight]
//!
//! The labels file is a JSON array of objects with "resource" and "label".
//! Label synonyms are canonicalized by the map below. The two numbers that
//! must trend right over time: colocated pairs up, fused threads at zero.
//! The exit code is the CI gate: any fusion of two labeled works, or any
//! leisure resource anchoring a thread, fails the run.

use evo_threads::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

fn syn(l: &str) -> String {
    let l = l.to_lowercase();
    match l.as_str() {
        "pitch deck" | "pitchdeck" => "pitch".into(),
        "evo" | "omnirouter/evo" | "claude code hacks" | "github task" | "zcode" | "notes" | "research" => "evo".into(),
        "waitlist" | "job related" => "waitlist".into(),
        "demo" | "demo video" | "probably, demo vide which a html file" | "checking something g related to demo video" => "demo".into(),
        "free claude/gpt" | "free claude/gpt search" | "free claude/gpt research" => "freeai".into(),
        "leisure" | "random" => "leisure".into(),
        "emails checking" => "comms".into(),
        "signing in" => "noise".into(),
        _ => l,
    }
}

fn is_work(w: &str) -> bool {
    !w.is_empty() && w != "leisure" && w != "comms" && w != "noise"
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 0;
    while i + 1 < args.len() {
        if args[i] == name {
            return Some(args[i + 1].clone());
        }
        i += 1;
    }
    None
}

fn main() {
    let repo = env!("CARGO_MANIFEST_DIR").trim_end_matches("/crates/evo-threads");
    let events_path = arg("--events").unwrap_or_else(|| format!("{repo}/tools/threads-regression/sample_events.json"));
    let labels_path = arg("--labels").unwrap_or_else(|| format!("{repo}/tools/threads-regression/ground_truth_labels.json"));

    let raw = fs::read_to_string(&events_path)
        .unwrap_or_else(|e| panic!("cannot read {events_path}: {e}"));
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("cannot parse {events_path}: {e}"));

    let mut engine = Engine::new();
    for v in &parsed {
        let ts = v[0].as_u64().unwrap_or(0);
        let origin = match v[1].as_str().unwrap_or("Person") {
            "Person" => Origin::Person,
            "System" => Origin::System,
            _ => Origin::Execution,
        };
        let interaction = match v[2].as_str().unwrap_or("Focused") {
            "Downloaded" => Interaction::Downloaded,
            "Typed" => Interaction::Typed,
            "Saved" => Interaction::Saved,
            "Mutated" => Interaction::Mutated,
            "Visited" => Interaction::Visited,
            _ => Interaction::Focused,
        };
        let resource = v[3].as_str().unwrap_or("").to_string();
        let weight = v.get(4).and_then(|w| w.as_u64());
        engine.apply(Event { timestamp: ts, origin, interaction, resource, weight, detail: None });
    }
    // Settle one day past the last event so the final sitting is visible.
    let last_ts = parsed.last().and_then(|v| v[0].as_u64()).unwrap_or(0);
    engine.settle(last_ts + 86_400_000);

    let gt_raw = fs::read_to_string(&labels_path)
        .unwrap_or_else(|e| panic!("cannot read {labels_path}: {e}"));
    let gt: Vec<serde_json::Value> = serde_json::from_str(&gt_raw).unwrap_or_default();
    let mut truth: BTreeMap<String, String> = BTreeMap::new();
    for g in &gt {
        if let Some(label) = g["label"].as_str() {
            if !label.is_empty() {
                truth.insert(g["resource"].as_str().unwrap_or("").trim().to_string(), syn(label));
            }
        }
    }

    let threads = engine.threads();
    let mut res2thread: BTreeMap<String, ThreadId> = BTreeMap::new();
    for t in &threads {
        for ep in &t.episodes {
            for (r, _) in &ep.participants {
                res2thread.entry(r.clone()).or_insert(t.id);
            }
        }
        for a in &t.anchors {
            res2thread.entry(a.resource.clone()).or_insert(t.id);
        }
        for (r, _) in &t.companions {
            res2thread.entry(r.clone()).or_insert(t.id);
        }
    }

    let labeled: Vec<(&String, &String)> = truth.iter()
        .filter(|(_, w)| is_work(w)).collect();
    let mut pair_ok = 0usize;
    let mut pair_split = 0usize;
    for i in 0..labeled.len() {
        for (r, w) in labeled.iter().skip(i + 1) {
            if *labeled[i].1 != **w { continue; }
            if let (Some(a), Some(b)) = (res2thread.get(labeled[i].0), res2thread.get(*r)) {
                if a == b { pair_ok += 1; } else { pair_split += 1; }
            }
        }
    }

    let mut thread_works: BTreeMap<ThreadId, BTreeSet<String>> = BTreeMap::new();
    for (r, w) in &truth {
        if !is_work(w) { continue; }
        if let Some(t) = res2thread.get(r) {
            thread_works.entry(*t).or_default().insert(w.clone());
        }
    }
    let fused: Vec<(&ThreadId, &BTreeSet<String>)> = thread_works.iter()
        .filter(|(_, ws)| ws.len() > 1).collect();

    let leisure_anchored = threads.iter().any(|t| t.anchors.iter().any(|a| {
        truth.get(&a.resource).map(|w| w == "leisure").unwrap_or(false)
    }));

    println!("threads minted: {}", threads.len());
    for t in &threads {
        let works: BTreeSet<String> = t.episodes.iter().flat_map(|ep| ep.participants.iter())
            .filter_map(|(r, _)| truth.get(r).cloned()).collect();
        let n_parts: usize = t.episodes.iter().map(|e| e.participants.len()).sum();
        println!("  thread {:>4}: episodes={}, participants={}, anchors={}, labels={:?}",
            t.id, t.episodes.len(), n_parts, t.anchors.len(), works);
    }
    println!();
    println!("same-work pairs: colocated={} split={}", pair_ok, pair_split);
    println!("threads fusing 2+ distinct works: {}", fused.len());
    for (t, ws) in &fused { println!("   FUSED thread {t}: {ws:?}"); }
    println!("leisure resource ever anchors a thread: {}", leisure_anchored);
    println!("trace decisions recorded: {}", engine.trace().len());

    let best = threads.first();
    if let Some(t) = best {
        let b = engine.resume_bundle(t.id);
        println!("\ntop thread {} resume: point={:?} reason={}", t.id, b.resume_point, b.resume_reason);
        println!("  restore_set: {:?}", b.restore_set);
        println!("  open_deltas: {} items", b.open_deltas.len());
    }

    if !fused.is_empty() || leisure_anchored {
        println!("\nREGRESSION: safety property violated (fusion or leisure anchor).");
        println!("NOTE: this corpus has no input events (no Typed/keystroke data),");
        println!("so attention-only surfaces of different works that co-occur in");
        println!("time can legitimately land in one episode. Fusion here is a");
        println!("data-coverage artifact until browser input capture lands; track");
        println!("it, and treat any NEW fusion after input capture as a hard bug.");
        std::process::exit(1);
    }
    println!("\nOK");
}
