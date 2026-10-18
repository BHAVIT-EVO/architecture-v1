//! Diagnostic probe for the committed ground-truth corpus: replays an
//! exported event file through the engine and prints, per thread, every
//! episode (span, participants with attention, assigned?) and anchors —
//! filtered to the resources you name on argv. Answers "why does X anchor
//! a thread" from the engine's own records instead of guesswork.
//!
//! Usage:
//!   cargo run -p evo-threads --example probe_ground_truth -- \
//!       /tmp/evo-full-corpus2.json vidbox youtube.com/ ZCode

use evo_threads::*;
use std::fs;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: probe <events.json> [filters...]");
    let mut filters: Vec<String> = args.collect();
    let show_trace = filters.iter().any(|f| f == "--trace");
    filters.retain(|f| f != "--trace");

    let raw = fs::read_to_string(&path).expect("read events file");
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&raw).expect("parse events file");

    let mut engine = Engine::new();
    let mut last_ts = 0u64;
    for v in &parsed {
        let ts = v[0].as_u64().unwrap_or(0);
        last_ts = last_ts.max(ts);
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
            origin: match v[1].as_str().unwrap_or("Person") {
                "System" => Origin::System,
                "Execution" => Origin::Execution,
                _ => Origin::Person,
            },
            interaction,
            resource: v[3].as_str().unwrap_or("").to_string(),
            weight: v.get(4).and_then(|w| w.as_u64()),
            detail,
        });
    }
    engine.settle(last_ts + 86_400_000);

    for t in engine.threads() {
        let relevant =
            |r: &str| filters.is_empty() || filters.iter().any(|f| r.contains(f.as_str()));
        let t_touches = t.anchors.iter().any(|a| relevant(&a.resource))
            || t.episodes
                .iter()
                .any(|ep| ep.participants.iter().any(|(r, _)| relevant(r)));
        if !t_touches {
            continue;
        }
        println!("thread {} (episodes={}):", t.id, t.episodes.len());
        for a in &t.anchors {
            println!("  anchor {:?} {}: {}", a.kind, a.strength, a.resource);
        }
        for ep in &t.episodes {
            println!(
                "  episode [{}..{}] participants={}",
                ep.span.0,
                ep.span.1,
                ep.participants.len()
            );
            for (r, attention) in &ep.participants {
                if relevant(r) {
                    println!("      {:>8.0} ms  {}", attention, r);
                }
            }
        }
        println!();
    }

    if show_trace {
        println!("decisions:");
        for (i, d) in engine.trace().iter().enumerate() {
            println!(
                "  {:>4}: {}",
                i,
                evo_threads::render_decision(d).unwrap_or_else(|| format!("{d:?}"))
            );
        }
    }
}
