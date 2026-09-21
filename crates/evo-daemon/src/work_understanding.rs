//! The understanding layer: what turns attributed resources into a
//! narrative a human can use to resume work.
//!
//! The research is unambiguous (Parnin 2010, Altmann & Trafton 2002,
//! Iqbal & Bailey 2005): resumption speed is a function of cue
//! availability at the moment of return. The best cue format is a
//! chronological trail of recent activity plus the intended next action.
//! NOT a summary — a trail.
//!
//! This module derives, from the engine's witnessed facts:
//! * **Phase** — Researching / Building / Reviewing
//! * **Progress** — what was accomplished vs. what's pending
//! * **Next step** — the single most logical continuation
//! * **Narrative** — a template-based summary where every word is
//!   derived from a witnessed fact, never guessed.

use evo_threads::{EpisodeView, ThreadView, ResumeBundle};

/// What the work IS, stated so a human can use it to resume.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkUnderstanding {
    pub title: String,
    pub phase: WorkPhase,
    pub trail: Vec<TrailEntry>,
    pub completed: Vec<String>,
    pub pending: Vec<String>,
    pub next_step: String,
    pub narrative: String,
}

/// The phase of work, from resource-mix analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkPhase {
    Researching,
    Building,
    /// Typing-dominant engagement without observed file output: composing
    /// in a chat, a draft, a buffer that never hit disk. Distinguished
    /// from Building by evidence (no attached saves), not by app.
    Writing,
    Reviewing,
}

impl WorkPhase {
    pub fn label(&self) -> &'static str {
        match self {
            WorkPhase::Researching => "Researching",
            WorkPhase::Building => "Building",
            WorkPhase::Writing => "Writing",
            WorkPhase::Reviewing => "Reviewing",
        }
    }
    pub fn verb(&self) -> &'static str {
        match self {
            WorkPhase::Researching => "Researching",
            WorkPhase::Building => "Working on",
            WorkPhase::Writing => "Writing",
            WorkPhase::Reviewing => "Reviewing",
        }
    }
}

/// One entry in the chronological trail.
#[derive(Debug, Clone, PartialEq)]
pub struct TrailEntry {
    pub name: String,
    /// The raw resource identifier (full URL, full file path) — searchable.
    pub raw: String,
    pub action: TrailAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrailAction {
    Visited,
    Opened,
    Interacted,
    Saved,
    Downloaded,
}

impl TrailAction {
    pub fn phrase(&self) -> &'static str {
        match self {
            TrailAction::Visited => "visited",
            TrailAction::Opened => "opened",
            TrailAction::Interacted => "worked in",
            TrailAction::Saved => "saved changes to",
            TrailAction::Downloaded => "downloaded",
        }
    }
}

/// Derives the full understanding from the engine's thread data.
pub fn understand(thread: &ThreadView, bundle: &ResumeBundle) -> WorkUnderstanding {
    let title = derive_title(thread, bundle);
    let phase = detect_phase(thread);
    let trail = build_trail(thread);
    let completed = derive_completed(thread);
    let pending = derive_pending(thread, bundle);
    let next_step = derive_next_step(thread, bundle, &pending);
    let narrative = compose_narrative(&title, &phase, &completed, &pending, &next_step, thread);

    WorkUnderstanding { title, phase, trail, completed, pending, next_step, narrative }
}

fn derive_title(thread: &ThreadView, bundle: &ResumeBundle) -> String {
    // Prefer URL anchors — they carry the most human-readable context
    let url_anchor = thread.anchors.iter()
        .filter(|a| a.resource.starts_with("http"))
        .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(anchor) = url_anchor {
        if let Some(name) = meaningful_name(&anchor.resource) { return name; }
    }
    // Then file anchors
    let file_anchor = thread.anchors.iter()
        .filter(|a| !a.resource.starts_with("Downloaded:") && !a.resource.starts_with("http"))
        .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(anchor) = file_anchor {
        if let Some(name) = meaningful_name(&anchor.resource) { return name; }
    }
    // Then companions
    for (resource, weight) in &thread.companions {
        if let Some(name) = meaningful_name(resource) { return name; }
    }
    if let Some(name) = meaningful_name(&bundle.resume_point) { return name; }
    "Work".to_string()
}

fn detect_phase(thread: &ThreadView) -> WorkPhase {
    // Production = Mutation anchors only (saves, downloads, typing).
    // Recurrence anchors (URL returns) are consumption, not production.
    let production_count = thread.anchors.iter()
        .filter(|a| a.kind == evo_threads::AnchorKind::Mutation)
        .count();

    // Recent resource mix: how much is URLs vs. files vs. apps?
    let recent: Vec<&EpisodeView> = thread.episodes.iter().rev().take(3).collect();
    let mut url_count = 0usize;
    let mut total = 0usize;
    let mut distinct = std::collections::BTreeSet::new();

    for ep in &recent {
        for (resource, _) in &ep.participants {
            distinct.insert(resource.clone());
            total += 1;
            if resource.starts_with("http") { url_count += 1; }
        }
    }
    if total == 0 {
        return if production_count > 0 { WorkPhase::Building } else { WorkPhase::Researching };
    }
    let url_ratio = url_count as f64 / total as f64;
    let diversity = distinct.len() as f64 / total.max(1) as f64;

    // Pure URL browsing with no production = Researching (or Reviewing if focused)
    if production_count == 0 {
        if url_ratio > 0.5 {
            return WorkPhase::Researching;
        }
        if diversity < 0.3 && total > 3 {
            return WorkPhase::Reviewing;
        }
        return WorkPhase::Researching;
    }

    // Has production → Building
    WorkPhase::Building
}

fn build_trail(thread: &ThreadView) -> Vec<TrailEntry> {
    let mut entries = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for ep in &thread.episodes {
        for anchor in &ep.anchors {
            if !seen.contains(anchor) {
                seen.insert(anchor.clone());
                let action = if anchor.starts_with("Downloaded:") {
                    TrailAction::Downloaded
                } else {
                    TrailAction::Saved
                };
                entries.push(TrailEntry {
                    name: meaningful_name(anchor).unwrap_or_else(|| anchor.chars().take(30).collect()),
                    raw: anchor.clone(),
                    action,
                });
            }
        }
        for (resource, attention) in &ep.participants {
            if !seen.contains(resource) && *attention > 5000.0 {
                seen.insert(resource.clone());
                let action = if resource.starts_with("http") {
                    TrailAction::Visited
                } else if *attention > 60000.0 {
                    TrailAction::Interacted
                } else {
                    TrailAction::Opened
                };
                entries.push(TrailEntry {
                    name: meaningful_name(resource).unwrap_or_else(|| resource.chars().take(50).collect()),
                    raw: resource.clone(),
                    action,
                });
            }
        }
    }
    entries.truncate(8);
    entries
}

fn derive_completed(thread: &ThreadView) -> Vec<String> {
    let mut completed = Vec::new();
    let mut saved = std::collections::BTreeSet::new();
    let mut downloads = 0usize;
    for ep in &thread.episodes {
        for anchor in &ep.anchors {
            if anchor.starts_with("Downloaded:") {
                downloads += 1;
            } else if !saved.contains(anchor) {
                saved.insert(anchor.clone());
                if let Some(name) = meaningful_name(anchor) {
                    completed.push(name);
                }
            }
        }
    }
    if downloads > 0 {
        completed.push(format!("Downloaded {} file{}", downloads, if downloads == 1 { "" } else { "s" }));
    }
    completed.truncate(5);
    completed
}

fn derive_pending(thread: &ThreadView, bundle: &ResumeBundle) -> Vec<String> {
    let mut pending = Vec::new();
    for delta in &bundle.open_deltas {
        let desc = match delta {
            evo_threads::Delta::UnsavedEdits { resource } =>
                format!("Unsaved changes in {}", meaningful_name(resource)
                    .unwrap_or_else(|| resource.chars().take(50).collect())),
            evo_threads::Delta::UncommittedChanges { root } =>
                format!("Uncommitted changes in {}", meaningful_name(root)
                    .unwrap_or_else(|| root.chars().take(30).collect())),
            evo_threads::Delta::DownloadedNotYetUsed { path } =>
                format!("{} (not yet used)", meaningful_name(path)
                    .unwrap_or_else(|| path.chars().take(50).collect())),
            evo_threads::Delta::UnfinishedDraftSurface { resource } =>
                format!("Draft in {} left unfinished", meaningful_name(resource)
                    .unwrap_or_else(|| resource.chars().take(50).collect())),
        };
        pending.push(desc);
    }
    pending.truncate(4);
    pending
}

fn derive_next_step(thread: &ThreadView, bundle: &ResumeBundle, pending: &[String]) -> String {
    if let Some(first) = pending.first() { return first.clone(); }
    if let Some(last_ep) = thread.episodes.last() {
        if let Some((resource, _)) = last_ep.participants.first() {
            if let Some(name) = meaningful_name(resource) {
                return format!("Continue in {}", name);
            }
        }
        if let Some(anchor) = last_ep.anchors.first() {
            if let Some(name) = meaningful_name(anchor) {
                return format!("Continue working on {}", name);
            }
        }
    }
    if !bundle.resume_point.is_empty() {
        if let Some(name) = meaningful_name(&bundle.resume_point) {
            return format!("Continue in {}", name);
        }
    }
    "Pick up where you left off".to_string()
}

fn compose_narrative(
    title: &str, phase: &WorkPhase, completed: &[String], pending: &[String],
    next_step: &str, thread: &ThreadView,
) -> String {
    let mut parts = Vec::new();
    let sessions = thread.episodes.len();
    let session_word = if sessions == 1 { "session" } else { "sessions" };
    parts.push(format!("You were {} {} across {} {}.", phase.verb(), title, sessions, session_word));
    if !completed.is_empty() {
        let items = completed.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", ");
        parts.push(format!("Completed: {}", items));
    }
    if !pending.is_empty() {
        let items = pending.iter().map(|p| p.as_str()).collect::<Vec<_>>().join("; ");
        parts.push(format!("Still pending: {}", items));
    }
    parts.push(format!("Next: {}", next_step));
    parts.join(" ")
}

pub fn meaningful_name(resource: &str) -> Option<String> {
    // Strip the "Downloaded:" prefix — the filename is the name
    let resource = resource.strip_prefix("Downloaded:").unwrap_or(resource).trim();
    if resource.starts_with("http") {
        let url = resource.trim_start_matches("https://").trim_start_matches("http://");
        let parts: Vec<&str> = url.split('/').filter(|p| !p.is_empty()).collect();
        let skip = ["app", "www", "c", "chat", "projects", "design", "watch", "edit", "v", "u", "p", "g", "d", "e"];
        let candidates: Vec<String> = parts[1..].iter()
            .filter_map(|p| {
                let clean = p.split('?').next()?.split('#').next()?;
                if clean.is_empty() || skip.contains(&clean) { return None; }
                let name = if let Some(pos) = clean.find("--") { &clean[..pos] } else { clean };
                if name.len() < 3 || is_identifier(name) { return None; }
                Some(name.to_string())
            })
            .take(2)
            .collect();
        if !candidates.is_empty() {
            return Some(candidates.iter().map(|s| title_case(s)).collect::<Vec<_>>().join(" — "));
        }
        let domain = parts.first()?.trim_start_matches("www.");
        return Some(domain.to_string());
    }
    if resource.starts_with("file://") || resource.starts_with('/') {
        let path = resource.trim_start_matches("file://");
        let file = path.rsplit('/').next()?;
        if file.is_empty() || is_identifier(file) { return None; }
        let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(file);
        return Some(title_case(stem));
    }
    if resource.len() > 3 && !resource.starts_with("app:") && !resource.starts_with("term:") {
        return Some(resource.chars().take(50).collect());
    }
    None
}

fn is_identifier(s: &str) -> bool {
    if s.matches('-').count() >= 4 { return true; }
    if s.len() >= 8 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-') { return true; }
    if s.chars().all(|c| c.is_ascii_digit()) { return true; }
    false
}

fn title_case(s: &str) -> String {
    s.split(['-', '_'])
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
