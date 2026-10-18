//! Ground-truth label canonicalization for the committed regression corpus.
//!
//! `tools/threads-regression/ground_truth_labels.json` carries the founder's
//! hand labels for a real recorded day. Several labels name the same body of
//! work in different words ("Evo", "ZCode", "notes", "Claude code hacks"…);
//! the canonical map below folds them so the scoring asks exactly one
//! question: did resources the person called the same work land in the same
//! thread, and did resources from different works stay apart?
//!
//! Shared by the `regression` binary (human-readable report + exit-code CI
//! gate) and the `ground_truth` integration test (the same gate inside
//! `cargo test`), so the two can never drift apart.

/// Canonicalizes one hand label to its work identity.
pub fn canonical_label(label: &str) -> String {
    let lower = label.to_lowercase();
    match lower.as_str() {
        "pitch deck" | "pitchdeck" => "pitch".into(),
        "evo" | "omnirouter/evo" | "claude code hacks" | "github task" | "zcode" | "notes"
        | "research" => "evo".into(),
        "waitlist" | "job related" => "waitlist".into(),
        "demo"
        | "demo video"
        | "probably, demo vide which a html file"
        | "checking something g related to demo video" => "demo".into(),
        "free claude/gpt" | "free claude/gpt search" | "free claude/gpt research" => {
            "freeai".into()
        }
        "leisure" | "random" => "leisure".into(),
        "emails checking" => "comms".into(),
        "signing in" => "noise".into(),
        _ => lower,
    }
}

/// Whether a canonical label names real work (vs. leisure, comms, noise).
///
/// A leisure resource must never anchor a thread; a comms thread must never
/// hijack a resume. Those are the safety properties, so the gate needs to
/// know which labels carry the property.
pub fn is_work_label(label: &str) -> bool {
    !label.is_empty() && label != "leisure" && label != "comms" && label != "noise"
}
