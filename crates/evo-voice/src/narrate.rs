//! Narration: the Work Brief, spoken.
//!
//! Phase 3 of the mission. The engine ships facts (`evo_threads::WorkBrief`);
//! this module ships words. The contract both sides can hold:
//!
//! * **Sentences are rendered from witnessed facts only.** Every clause
//!   traces to a brief field — anchors to identity, companions to context,
//!   the resume bundle to "where you left off", open deltas to "still
//!   unresolved". Nothing is invented; when a brief is empty the narration
//!   says exactly that.
//! * **The engine's evidence ranks are the sentence order.** Production
//!   before recurrence (made > returned), identity before context, resume
//!   before open loops — the same caste system `assign.rs` states.
//! * **Deterministic.** Same brief, same words, every time. No clock, no
//!   randomness — a test can pin the exact sentence.
//! * **Names may be injected, never guessed.** A caller that has witnessed
//!   surface names (the daemon's NameIndex) may pass them; narration uses
//!   the cleaned name for a resource when one exists and the raw resource
//!   string otherwise. The engine stays domain-blind; so does this module.
//!
//! # No hardcoding
//!
//! Nothing here names an application, domain, or phrase template per tool.
//! The templates are grammar, not vocabulary: they bind "made", "returned
//! to", "left off at" — the engine's own verbs — to whatever resources the
//! brief carries.

use evo_threads::{AnchorKind, Status, WorkBrief};
use std::collections::BTreeMap;

/// Resolves a resource to its display form: the person's own witnessed name
/// when available, the resource string otherwise.
pub type NameLookup<'a> = BTreeMap<&'a str, &'a str>;

/// One work's narration: the short spoken answer to "what is this work?".
pub fn narrate_brief(brief: &WorkBrief, names: &NameLookup<'_>) -> String {
    let name_of = |r: &str| -> String {
        names
            .get(r)
            .map(|n| (*n).to_string())
            .unwrap_or_else(|| r.to_string())
    };

    if brief.anchors.is_empty() {
        return "You have no work here yet.".to_string();
    }

    let mut sentences: Vec<String> = Vec::new();

    // Sentence 1 — identity, in evidence order: made first, then returned-to.
    let made: Vec<String> = brief
        .anchors
        .iter()
        .filter(|a| a.kind == AnchorKind::Mutation)
        .map(|a| name_of(&a.resource))
        .collect();
    let returned: Vec<String> = brief
        .anchors
        .iter()
        .filter(|a| a.kind == AnchorKind::Recurrence)
        .map(|a| name_of(&a.resource))
        .collect();
    let mut identity: Vec<String> = Vec::new();
    if !made.is_empty() {
        identity.push(format!("you made {}", list(&made)));
    }
    if !returned.is_empty() {
        identity.push(format!("you kept returning to {}", list(&returned)));
    }
    if !identity.is_empty() {
        sentences.push(format!("This is the work where {}.", join_and(identity)));
    }

    // Sentence 2 — where you left off. The bundle's reason is already the
    // engine's own honest sentence; narrate the point and keep the reason's
    // substance.
    if !brief.resume_point.is_empty() {
        sentences.push(format!("You left off at {}.", name_of(&brief.resume_point)));
    } else if brief.resume_reason == "Last seen: nothing." {
        sentences.push("There is nothing to resume yet.".to_string());
    }

    // Sentence 3 — open loops, the shortest honest form.
    let loops = brief.open_deltas.len();
    match loops {
        0 => {}
        1 => sentences.push("One thing is still open.".to_string()),
        n => sentences.push(format!("{n} things are still open.")),
    }

    sentences.join(" ")
}

/// The whole rack, spoken top-first — the "what am I working on?" answer.
pub fn narrate_briefs(briefs: &[WorkBrief], names: &NameLookup<'_>) -> String {
    if briefs.is_empty() {
        return "You have no works yet.".to_string();
    }
    let mut out = Vec::new();
    for (i, b) in briefs.iter().enumerate() {
        let head = if i == 0 {
            "First".to_string()
        } else if i + 1 == briefs.len() {
            "And last".to_string()
        } else {
            "Then".to_string()
        };
        out.push(format!("{head}, {}.", narrate_brief(b, names)));
    }
    out.join(" ")
}

/// The status line: active vs. resting, in the engine's own vocabulary.
pub fn narrate_status(brief: &WorkBrief) -> String {
    match brief.status {
        Status::Active => "still active".to_string(),
        Status::Dormant => "resting".to_string(),
    }
}

/// A human list: "a", "a and b", "a, b, and c".
fn list(items: &[String]) -> String {
    match items.len() {
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => format!(
            "{}, and {}",
            items[..items.len() - 1].join(", "),
            items[items.len() - 1]
        ),
    }
}

/// Join independent clauses with "and".
fn join_and(items: Vec<String>) -> String {
    list(&items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_threads::{Delta, WorkAnchor};

    fn brief(
        anchors: Vec<(AnchorKind, &str)>,
        resume_point: &str,
        deltas: Vec<Delta>,
    ) -> WorkBrief {
        WorkBrief {
            thread_id: 1,
            status: Status::Active,
            anchors: anchors
                .into_iter()
                .map(|(kind, resource)| WorkAnchor {
                    resource: resource.to_string(),
                    kind,
                })
                .collect(),
            contested: vec![],
            companions: vec![],
            episodes: 1,
            resume_point: resume_point.to_string(),
            resume_reason: "You kept returning to it.".to_string(),
            restore_set: vec![],
            open_deltas: deltas,
            fork: None,
        }
    }

    #[test]
    fn sentences_follow_the_evidence_ranks() {
        let b = brief(
            vec![
                (AnchorKind::Recurrence, "https://reading.example/paper"),
                (AnchorKind::Mutation, "file:///manuscript.md"),
            ],
            "file:///manuscript.md",
            vec![],
        );
        let s = narrate_brief(&b, &NameLookup::new());
        assert!(
            s.starts_with("This is the work where you made file:///manuscript.md and you kept returning to https://reading.example/paper."),
            "made outranks returned, in order: {s}"
        );
        assert!(s.contains("You left off at file:///manuscript.md."));
        assert!(!s.contains("open"), "no invented open loops: {s}");
    }

    #[test]
    fn witnessed_names_replace_raw_resources() {
        let b = brief(vec![(AnchorKind::Mutation, "ZCode")], "ZCode", vec![]);
        let names: NameLookup<'_> = [("ZCode", "Evo desktop app")].into_iter().collect();
        let s = narrate_brief(&b, &names);
        assert!(
            s.contains("you made Evo desktop app"),
            "the person's word for the surface is what gets spoken: {s}"
        );
        assert!(
            !s.contains("ZCode"),
            "the raw id never leaks past a name: {s}"
        );
    }

    #[test]
    fn open_loops_are_counted_not_enumerated() {
        let b = brief(
            vec![(AnchorKind::Mutation, "file:///notes.md")],
            "file:///notes.md",
            vec![
                Delta::UnsavedEdits {
                    resource: "file:///notes.md".into(),
                },
                Delta::DownloadedNotYetUsed {
                    path: "/tmp/d.csv".into(),
                },
            ],
        );
        let s = narrate_brief(&b, &NameLookup::new());
        assert!(s.ends_with("2 things are still open."), "{s}");
    }

    #[test]
    fn an_empty_brief_says_exactly_that() {
        let b = brief(vec![], "", vec![]);
        assert_eq!(
            narrate_brief(&b, &NameLookup::new()),
            "You have no work here yet."
        );
    }

    #[test]
    fn the_rack_narrates_in_order() {
        let first = brief(
            vec![(AnchorKind::Mutation, "file:///a.md")],
            "file:///a.md",
            vec![],
        );
        let second = brief(
            vec![(AnchorKind::Recurrence, "https://b.example/")],
            "https://b.example/",
            vec![],
        );
        let s = narrate_briefs(&[first, second], &NameLookup::new());
        assert!(s.starts_with("First, This is the work"), "{s}");
        assert!(s.contains("And last,"), "{s}");
        assert!(s.ends_with("."), "{s}");
    }

    #[test]
    fn status_uses_the_engine_vocabulary() {
        let mut b = brief(vec![(AnchorKind::Mutation, "x")], "x", vec![]);
        assert_eq!(narrate_status(&b), "still active");
        b.status = Status::Dormant;
        assert_eq!(narrate_status(&b), "resting");
    }
}
