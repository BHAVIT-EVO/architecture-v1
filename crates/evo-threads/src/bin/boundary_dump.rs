//! Dumps the engine's derived work boundary as JSON, so a local service can
//! serve it to other programs.
//!
//! Usage:
//!   cargo run -p evo-threads --bin boundary_dump -- \
//!       --events tools/threads-regression/sample_events.json \
//!       --out exports/ship-happens/threads.json
//!
//! This binary adds no inference of its own. It replays a recorded event log
//! through `Engine` exactly as the regression harness does, then prints what
//! the engine already believes: one brief per work, with its anchors, its
//! resume point, and the neighborhood to reopen.
//!
//! The engine does not name works. A work is named after its strongest
//! anchor — the thing the person actually made — until the person renames it.

use evo_threads::*;
use std::fs;

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
    let events_path =
        arg("--events").unwrap_or_else(|| format!("{repo}/tools/threads-regression/sample_events.json"));
    let out_path = arg("--out");

    let raw = fs::read_to_string(&events_path)
        .unwrap_or_else(|e| panic!("cannot read {events_path}: {e}"));
    let parsed: Vec<serde_json::Value> =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("cannot parse {events_path}: {e}"));

    let mut engine = Engine::new();
    for v in &parsed {
        let ts = v[0].as_u64().unwrap_or(0);
        let origin = match v[1].as_str().unwrap_or("Person") {
            "Person" => Origin::Person,
            "System" => Origin::System,
            _ => Origin::Execution,
        };
        let kind = v[2].as_str().unwrap_or("Focused");
        let interaction = match kind {
            "Downloaded" => Interaction::Downloaded,
            "Typed" => Interaction::Typed,
            "Saved" => Interaction::Saved,
            "Mutated" => Interaction::Mutated,
            "Visited" => Interaction::Visited,
            "CommandRan" => Interaction::CommandRan,
            "Declared" => Interaction::Declared,
            _ => Interaction::Focused,
        };
        let detail = match kind {
            "Declared" => Some(Declaration::SameWork {
                a: v[5].as_str().unwrap_or("").to_string(),
                b: v[6].as_str().unwrap_or("").to_string(),
            }),
            _ => None,
        };
        engine.apply(Event {
            timestamp: ts,
            origin,
            interaction,
            resource: v[3].as_str().unwrap_or("").to_string(),
            weight: v.get(4).and_then(|w| w.as_u64()),
            detail,
        });
    }

    // Settle one day past the last event so the final sitting is visible.
    let last_ts = parsed.last().and_then(|v| v[0].as_u64()).unwrap_or(0);
    engine.settle(last_ts + 86_400_000);

    let briefs = engine.briefs();

    let works: Vec<serde_json::Value> = briefs
        .iter()
        .map(|b| {
            let name = b
                .anchors
                .first()
                .map(|a| a.resource.clone())
                .unwrap_or_else(|| format!("work {}", b.thread_id));
            serde_json::json!({
                "id": b.thread_id,
                "name": name,
                "status": b.status,
                "episodes": b.episodes,
                "anchors": b.anchors.iter().map(|a| a.resource.clone()).collect::<Vec<_>>(),
                "contested": b.contested,
                "companions": b.companions,
                "resume_point": b.resume_point,
                "resume_reason": b.resume_reason,
                "restore_set": b.restore_set,
                "fork": b.fork,
            })
        })
        .collect();

    let doc = serde_json::json!({
        "source": "engine",
        "engine": "evo-threads",
        "events": parsed.len(),
        "works": works,
    });

    let text = serde_json::to_string_pretty(&doc).expect("serialize");
    match out_path {
        Some(p) => {
            fs::write(&p, text).unwrap_or_else(|e| panic!("cannot write {p}: {e}"));
            eprintln!("wrote {} works to {p}", briefs.len());
        }
        None => println!("{text}"),
    }
}
