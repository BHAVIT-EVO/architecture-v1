//! The understanding layer: what turns attributed resources into a
//! narrative a human can use to resume work.
//!
//! Thread views retain resource participation and anchors, but not the
//! underlying interaction kind or a person's task-completion statement.
//! Present observed activity and open deltas without assigning a profession,
//! a semantic phase, or completed tasks from resource syntax or anchor kind.

use evo_observation::evidence::FactValue;
use evo_observation::observation::Observation;
use evo_threads::{ResumeBundle, ThreadView};

/// A small, content-free presentation witness extracted from canonical
/// observations. It is never used for identity or membership; it only allows
/// the title ladder to say what was visibly happening when a structural name
/// is unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEvidence {
    pub resource: String,
    pub action: ActivityAction,
    pub observed_at: u64,
    pub weight: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum ActivityAction {
    Focused,
    Visited,
    Typed,
    Saved,
}

/// Aggregate quality metrics for the final title strings shown to a person.
/// These are presentation metrics only; they never feed grouping or ranking.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TitleMetrics {
    pub presented_work_count: usize,
    pub untitled_count: usize,
    pub identifier_count: usize,
    pub readable_count: usize,
}

impl TitleMetrics {
    pub fn untitled_rate(self) -> f64 {
        rate(self.untitled_count, self.presented_work_count)
    }

    pub fn identifier_rate(self) -> f64 {
        rate(self.identifier_count, self.presented_work_count)
    }

    pub fn readability_rate(self) -> f64 {
        rate(self.readable_count, self.presented_work_count)
    }

    pub fn identifier_floor_met(self) -> bool {
        self.identifier_count == 0
    }
}

fn rate(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        count as f64 / total as f64
    }
}

/// Scores the titles after rendering, so the metric cannot accidentally grade
/// an intermediate structural name rather than what Home will show.
pub fn title_metrics<'a>(titles: impl IntoIterator<Item = &'a str>) -> TitleMetrics {
    let mut metrics = TitleMetrics::default();
    for title in titles {
        metrics.presented_work_count += 1;
        let clean = title.trim();
        if clean.starts_with("Untitled") {
            metrics.untitled_count += 1;
        }
        if is_identifier_title(clean) {
            metrics.identifier_count += 1;
        } else if !clean.is_empty() {
            metrics.readable_count += 1;
        }
    }
    metrics
}

fn is_identifier_title(title: &str) -> bool {
    if title.is_empty()
        || is_address(title)
        || title.starts_with("app:")
        || title.starts_with("term:")
        || title.starts_with("window:")
        || is_identifier(title)
    {
        return true;
    }
    matches!(
        title.to_ascii_lowercase().as_str(),
        "login" | "search" | "new tab" | "loading" | "error" | "home"
    )
}

/// Extracts only schema-defined, content-free activity from the canonical log.
/// Unknown/reference-only observations are intentionally ignored.
pub fn activity_evidence_from_observations(
    observations: &[Observation],
) -> Vec<ActivityEvidence> {
    observations
        .iter()
        .filter_map(|observation| {
            let action = match observation.schema().name() {
                "OBS-WINDOW-FOCUS-GAINED" => ActivityAction::Focused,
                "OBS-URL-NAVIGATED" => ActivityAction::Visited,
                "OBS-FILE-SAVED" => ActivityAction::Saved,
                "OBS-INPUT-ACTIVITY" => ActivityAction::Typed,
                _ => return None,
            };
            let fact_name = observation.schema().canonical_fact_name()?;
            let resource = match observation.evidence().fact(fact_name)?.value() {
                FactValue::Text(resource) if !resource.trim().is_empty() => resource.clone(),
                _ => return None,
            };
            let weight = match action {
                ActivityAction::Typed => match observation.evidence().fact("Keys")?.value() {
                    FactValue::Integer(keys) if *keys > 0 => *keys as u64,
                    _ => return None,
                },
                _ => 1,
            };
            let observed_at = observation
                .provenance()
                .observed_at()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_millis()
                .min(u64::MAX as u128) as u64;
            Some(ActivityEvidence {
                resource,
                action,
                observed_at,
                weight,
            })
        })
        .collect()
}

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

/// Presentation vocabulary. Existing semantic variants remain available to
/// callers with direct evidence; resource-only views default to neutral activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkPhase {
    ObservedActivity,
    Researching,
    Building,
    Writing,
    Reviewing,
}

impl WorkPhase {
    pub fn label(&self) -> &'static str {
        match self {
            WorkPhase::ObservedActivity => "Observed activity",
            WorkPhase::Researching => "Researching",
            WorkPhase::Building => "Building",
            WorkPhase::Writing => "Writing",
            WorkPhase::Reviewing => "Reviewing",
        }
    }
    pub fn verb(&self) -> &'static str {
        match self {
            WorkPhase::ObservedActivity => "active in",
            WorkPhase::Researching => "Researching",
            WorkPhase::Building => "Working on",
            WorkPhase::Writing => "Writing",
            WorkPhase::Reviewing => "Reviewing",
        }
    }
}

/// One resource cue, grouped by episode. Within an episode the thread view
/// orders resources by attention/evidence, not by individual event timestamps.
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
            TrailAction::Interacted => "observed activity on",
            TrailAction::Saved => "saved changes to",
            TrailAction::Downloaded => "downloaded",
        }
    }
}

/// Derives the full understanding from the engine's thread data.
pub fn understand(thread: &ThreadView, bundle: &ResumeBundle) -> WorkUnderstanding {
    understand_named(thread, bundle, &evo_daemon_names_empty())
}

/// The empty name index, for callers that have no witnessed names. Kept as
/// a function so `work_understanding` does not depend on construction order
/// anywhere else.
fn evo_daemon_names_empty() -> crate::name_index::NameIndex {
    crate::name_index::NameIndex::empty()
}

/// Derives the full understanding with the log's witnessed names joined in:
/// the title and every trail entry prefer the name the person actually saw
/// the surface display (OBS-SURFACE-NAMED), falling back to the structural
/// derivation when the surface never named itself. Identity is untouched:
/// only the words presenting it change.
pub fn understand_named(
    thread: &ThreadView,
    bundle: &ResumeBundle,
    names: &crate::name_index::NameIndex,
) -> WorkUnderstanding {
    understand_classified(
        thread,
        bundle,
        names,
        &crate::classification::ClassifiedSurfaces::empty(),
    )
}

/// [`understand_named`] with the log's classified surfaces joined in.
///
/// A subject the person classified (OBS-SURFACE-CLASSIFIED) may never name the
/// work: it is skipped as a title source even when it rides along as a companion
/// or as the resume point. This is the presentation half of the same record the
/// engine read to refuse it an anchor — a class gates founding *and* titling,
/// and it deletes nothing (the classified surface still appears in the trail and
/// the member set).
pub fn understand_classified(
    thread: &ThreadView,
    bundle: &ResumeBundle,
    names: &crate::name_index::NameIndex,
    classified: &crate::classification::ClassifiedSurfaces,
) -> WorkUnderstanding {
    understand_classified_with_activity(thread, bundle, names, classified, &[])
}

/// [`understand_classified`] with content-free activity witnesses from the
/// canonical log. The witnesses affect presentation only.
pub fn understand_classified_with_activity(
    thread: &ThreadView,
    bundle: &ResumeBundle,
    names: &crate::name_index::NameIndex,
    classified: &crate::classification::ClassifiedSurfaces,
    activity: &[ActivityEvidence],
) -> WorkUnderstanding {
    let title = derive_title_named(thread, bundle, names, classified, activity);
    let phase = detect_phase(thread);
    let trail = build_trail_named(thread, names);
    let completed = derive_completed(thread);
    let pending = derive_pending(thread, bundle);
    let next_step = derive_next_step(thread, bundle, &pending);
    let narrative = compose_narrative(&title, &phase, &completed, &pending, &next_step, thread);

    WorkUnderstanding {
        title,
        phase,
        trail,
        completed,
        pending,
        next_step,
        narrative,
    }
}

/// The presentation name of one surface under the witnessed-name join:
/// what the surface said it was, else what its structure says it is.
fn present_or_structural(
    names: &crate::name_index::NameIndex,
    resource: &str,
    fallback_cap: usize,
) -> Option<String> {
    if let Some(witnessed) = names.enrich_surface(resource) {
        return Some(witnessed);
    }
    meaningful_name(resource).or_else(|| {
        (!resource.trim().is_empty())
            .then(|| resource.chars().take(fallback_cap).collect::<String>())
    })
}

/// The work's title, from the most specific witnessed name available.
///
/// # The ladder, and its one rule
///
/// A title is chosen in this order: the name the surface displayed
/// (`OBS-SURFACE-NAMED`), then the structural reading of the address (a URL's
/// meaningful path, a file's stem), then — when neither yields anything a
/// person would recognise — [`untitled_name`].
///
/// **The rule: a title is never an address.** It used to fall through to a
/// bare host (`youtube.com`) and then to the raw resource truncated to fifty
/// characters (`https://www.youtube.com/watch?v=…`). Both are identifiers, not
/// names. A list in which one work is named after an address makes every
/// neighbouring name look unreliable, so an unusable address now yields the
/// honest fallback rather than a plausible-looking string. See
/// [`title_or_structural`] for where the refusal happens.
fn derive_title_named(
    thread: &ThreadView,
    bundle: &ResumeBundle,
    names: &crate::name_index::NameIndex,
    classified: &crate::classification::ClassifiedSurfaces,
    activity: &[ActivityEvidence],
) -> String {
    // The engine already orders anchors by evidence. Resource syntax supplies
    // no reason to favor a URL over an image, file or opaque surface identity.
    // A classified surface names nothing, so it is skipped wherever the ladder
    // would otherwise reach it.
    for anchor in &thread.anchors {
        if classified.is_classified(&anchor.resource) {
            continue;
        }
        if let Some(name) = title_or_structural(names, &anchor.resource) {
            return name;
        }
    }
    // Then companions
    for (resource, _weight) in &thread.companions {
        if classified.is_classified(resource) {
            continue;
        }
        if let Some(name) = title_or_structural(names, resource) {
            return name;
        }
    }
    if !classified.is_classified(&bundle.resume_point)
        && let Some(name) = title_or_structural(names, &bundle.resume_point)
    {
        return name;
    }
    if let Some(phrase) = activity_phrase(thread, activity, classified) {
        return phrase;
    }
    untitled_name(thread)
}

/// Closed, local grammar for the final title rung. It reports the witnessed
/// interaction class and the number of places, never a guessed app/domain or
/// semantic task. Ties are deterministic: later evidence wins, then action
/// precedence, then resource order.
fn activity_phrase(
    thread: &ThreadView,
    activity: &[ActivityEvidence],
    classified: &crate::classification::ClassifiedSurfaces,
) -> Option<String> {
    let members: std::collections::BTreeSet<&str> = thread
        .anchors
        .iter()
        .map(|anchor| anchor.resource.as_str())
        .chain(thread.companions.iter().map(|(resource, _)| resource.as_str()))
        .chain(
            thread
                .episodes
                .iter()
                .flat_map(|episode| episode.participants.iter().map(|(resource, _)| resource.as_str())),
        )
        .filter(|resource| !classified.is_classified(resource))
        .collect();
    if members.is_empty() {
        return None;
    }
    let mut totals = std::collections::BTreeMap::<ActivityAction, u64>::new();
    let mut latest = std::collections::BTreeMap::<ActivityAction, (u64, String)>::new();
    for witness in activity.iter().filter(|witness| members.contains(witness.resource.as_str())) {
        *totals.entry(witness.action).or_default() += witness.weight;
        let candidate = (witness.observed_at, witness.resource.clone());
        if latest.get(&witness.action).is_none_or(|current| candidate > *current) {
            latest.insert(witness.action, candidate);
        }
    }
    let action = totals
        .into_iter()
        .max_by(|(left_action, left_total), (right_action, right_total)| {
            left_total
                .cmp(right_total)
                .then(left_action.cmp(right_action))
                .then_with(|| {
                    latest
                        .get(left_action)
                        .cmp(&latest.get(right_action))
                })
        })
        .map(|(action, _)| action)?;
    let places = members.len();
    let place_word = if places == 1 { "place" } else { "places" };
    let verb = match action {
        ActivityAction::Focused => "Working across",
        ActivityAction::Visited => "Reviewing across",
        ActivityAction::Typed => "Writing across",
        ActivityAction::Saved => "Updating across",
    };
    Some(format!("{verb} {places} {place_word}"))
}

/// Whether a resource string is an *address* — a locator rather than a name.
///
/// Structural shape only; the same test the ledger's identity chain uses, so
/// no application list, domain list or file-type list enters here.
fn is_address(resource: &str) -> bool {
    resource.starts_with("http")
        || resource.starts_with("file://")
        || resource.starts_with('/')
}

/// One resource's name for *titling* purposes: what the surface said, else what
/// its structure says, else nothing at all.
///
/// Differs from [`present_or_structural`] in exactly one way, and the
/// difference is the point: when a resource is an address and its structure
/// yields nothing readable, this returns `None` instead of the truncated raw
/// string. A trail entry may legitimately show a raw address as context; a
/// **title** may not, because a title is the identity the person will use to
/// find this work again.
fn title_or_structural(
    names: &crate::name_index::NameIndex,
    resource: &str,
) -> Option<String> {
    if let Some(witnessed) = names.enrich_surface(resource) {
        return Some(witnessed);
    }
    if let Some(structural) = meaningful_name(resource) {
        return Some(structural);
    }
    if is_address(resource)
        || resource.starts_with("app:")
        || resource.starts_with("term:")
        || resource.starts_with("window:")
    {
        return None;
    }
    (!resource.trim().is_empty()).then(|| resource.chars().take(50).collect())
}

/// The honest fallback title: only what was actually measured.
///
/// Reached when no witnessed name on the work is usable — a lone address with no
/// readable path, a generic interface page, or an empty name. It states the
/// number of places the work touched and how long it was attended.
///
/// It deliberately carries **no weekday**. Producing a local date would need a
/// timezone source this crate does not have (there is no date or timezone
/// dependency, by design), and epoch days are UTC, so a person five hours east
/// of Greenwich would see the wrong day in the evening. A name that states the
/// wrong day is worse than one that states only what was measured; the card's
/// own age stamp carries the when, and it is honest about it.
fn untitled_name(thread: &ThreadView) -> String {
    let places = thread.anchors.len() + thread.companions.len();
    let place_word = if places == 1 { "place" } else { "places" };
    match tracked_duration_ms(thread) {
        Some(ms) if ms >= 60_000 => {
            format!("Untitled — {places} {place_word}, {}", human_duration(ms))
        }
        _ => format!("Untitled — {places} {place_word}"),
    }
}

/// Total witnessed attention across the work's episodes, in milliseconds.
///
/// Each episode is clamped to twelve hours. An episode is already bounded by
/// the engine's attention rules, but this number is shown to the person as a
/// duration, so it must not be able to print something absurd if a span is ever
/// malformed.
fn tracked_duration_ms(thread: &ThreadView) -> Option<u64> {
    const EPISODE_CLAMP_MS: u64 = 12 * 60 * 60 * 1000;
    let total: u64 = thread
        .episodes
        .iter()
        .map(|ep| ep.span.1.saturating_sub(ep.span.0).min(EPISODE_CLAMP_MS))
        .sum();
    (total > 0).then_some(total)
}

/// "2h 15m", "45m", "3h" — whole minutes only, never seconds the evidence
/// cannot support.
fn human_duration(ms: u64) -> String {
    let minutes = ms / 60_000;
    let (hours, rest) = (minutes / 60, minutes % 60);
    match (hours, rest) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

fn detect_phase(_thread: &ThreadView) -> WorkPhase {
    // Mutation includes typing and human declarations as well as saves.
    // Neither anchor kind nor URL/file ratio witnesses a semantic work phase.
    WorkPhase::ObservedActivity
}

fn build_trail_named(thread: &ThreadView, names: &crate::name_index::NameIndex) -> Vec<TrailEntry> {
    let mut entries = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for ep in &thread.episodes {
        for anchor in &ep.anchors {
            if !seen.contains(anchor) {
                seen.insert(anchor.clone());
                entries.push(TrailEntry {
                    name: present_or_structural(names, anchor, 30)
                        .unwrap_or_else(|| anchor.chars().take(30).collect()),
                    raw: anchor.clone(),
                    action: TrailAction::Interacted,
                });
            }
        }
        for (resource, attention) in &ep.participants {
            if !seen.contains(resource) && *attention > 5000.0 {
                seen.insert(resource.clone());
                entries.push(TrailEntry {
                    name: present_or_structural(names, resource, 50)
                        .unwrap_or_else(|| resource.chars().take(50).collect()),
                    raw: resource.clone(),
                    action: TrailAction::Interacted,
                });
            }
        }
    }
    entries.truncate(8);
    entries
}

fn derive_completed(_thread: &ThreadView) -> Vec<String> {
    // No task-completion statement is present in this API. Even an actual
    // save would witness persistence, not completion of the person's work.
    Vec::new()
}

fn derive_pending(_thread: &ThreadView, bundle: &ResumeBundle) -> Vec<String> {
    let mut pending = Vec::new();
    for delta in &bundle.open_deltas {
        let desc = match delta {
            evo_threads::Delta::UnsavedEdits { resource } => format!(
                "Unsaved changes in {}",
                meaningful_name(resource).unwrap_or_else(|| resource.chars().take(50).collect())
            ),
            evo_threads::Delta::UncommittedChanges { root } => format!(
                "Uncommitted changes in {}",
                meaningful_name(root).unwrap_or_else(|| root.chars().take(30).collect())
            ),
            evo_threads::Delta::DownloadedNotYetUsed { path } => format!(
                "{} (not yet used)",
                meaningful_name(path).unwrap_or_else(|| path.chars().take(50).collect())
            ),
            evo_threads::Delta::UnfinishedDraftSurface { resource } => format!(
                "Input without an observed save in {}",
                meaningful_name(resource).unwrap_or_else(|| resource.chars().take(50).collect())
            ),
        };
        pending.push(desc);
    }
    pending.truncate(4);
    pending
}

fn derive_next_step(thread: &ThreadView, bundle: &ResumeBundle, pending: &[String]) -> String {
    if let Some(first) = pending.first() {
        return first.clone();
    }
    if let Some(last_ep) = thread.episodes.last() {
        if let Some((resource, _)) = last_ep.participants.first()
            && let Some(name) = meaningful_name(resource)
        {
            return format!("Continue in {}", name);
        }
        if let Some(anchor) = last_ep.anchors.first()
            && let Some(name) = meaningful_name(anchor)
        {
            return format!("Continue working on {}", name);
        }
    }
    if !bundle.resume_point.is_empty()
        && let Some(name) = meaningful_name(&bundle.resume_point)
    {
        return format!("Continue in {}", name);
    }
    "Pick up where you left off".to_string()
}

fn compose_narrative(
    title: &str,
    phase: &WorkPhase,
    completed: &[String],
    pending: &[String],
    next_step: &str,
    thread: &ThreadView,
) -> String {
    let mut parts = Vec::new();
    let sessions = thread.episodes.len();
    let session_word = if sessions == 1 { "episode" } else { "episodes" };
    if *phase == WorkPhase::ObservedActivity {
        parts.push(format!(
            "Observed activity for {} across {} recorded {}.",
            title, sessions, session_word
        ));
    } else {
        parts.push(format!(
            "You were {} {} across {} {}.",
            phase.verb(),
            title,
            sessions,
            session_word
        ));
    }
    if !completed.is_empty() {
        let items = completed
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("Completed: {}", items));
    }
    if !pending.is_empty() {
        let items = pending
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        parts.push(format!("Still pending: {}", items));
    }
    parts.push(format!("Next: {}", next_step));
    parts.join(" ")
}

pub fn meaningful_name(resource: &str) -> Option<String> {
    // Strip the "Downloaded:" prefix — the filename is the name
    let resource = resource
        .strip_prefix("Downloaded:")
        .unwrap_or(resource)
        .trim();
    if resource.starts_with("http") {
        let url = resource
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let parts: Vec<&str> = url.split('/').filter(|p| !p.is_empty()).collect();
        let skip = [
            "app", "www", "c", "chat", "projects", "design", "watch", "edit", "v", "u", "p", "g",
            "d", "e",
        ];
        let candidates: Vec<String> = parts
            .get(1..)?
            .iter()
            .filter_map(|p| {
                let clean = p.split('?').next()?.split('#').next()?;
                if clean.is_empty() || skip.contains(&clean) {
                    return None;
                }
                let name = if let Some(pos) = clean.find("--") {
                    &clean[..pos]
                } else {
                    clean
                };
                if name.len() < 3 || is_identifier(name) || is_generic_surface_name(name) {
                    return None;
                }
                Some(name.to_string())
            })
            .take(2)
            .collect();
        if !candidates.is_empty() {
            return Some(
                candidates
                    .iter()
                    .map(|s| title_case(s))
                    .collect::<Vec<_>>()
                    .join(" — "),
            );
        }
        // No readable path segment. This used to return the bare domain, which
        // is how a watched video became a work called "youtube.com" — a host is
        // an address, not a name. Returning `None` here lets the caller fall
        // through to the honest fallback instead.
        return None;
    }
    if resource.starts_with("file://") || resource.starts_with('/') {
        let path = resource.trim_start_matches("file://");
        let file = path.rsplit('/').next()?;
        if file.is_empty() || is_identifier(file) {
            return None;
        }
        let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(file);
        return Some(title_case(stem));
    }
    if resource.len() > 3
        && !resource.starts_with("app:")
        && !resource.starts_with("term:")
        && !resource.starts_with("window:")
    {
        return Some(resource.chars().take(50).collect());
    }
    None
}

fn is_identifier(s: &str) -> bool {
    if s.matches('-').count() >= 4 {
        return true;
    }
    if s.len() >= 8 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return true;
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    false
}

/// Whether a URL path segment names a *place in an interface* rather than a
/// subject. "Login", "Search", "New Tab" identify nothing, so a work titled
/// after one tells the person nothing.
///
/// Deliberately applied to URL segments only, never to file stems: `index.rs`
/// and `home.rs` are real, meaningful files in a codebase, and refusing them
/// would be a regression. This is a closed list of *interface* words, held
/// short on purpose — it is a readability guard, not a classifier, and it never
/// contributes to work identity or membership.
fn is_generic_surface_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "login"
            | "log-in"
            | "signin"
            | "sign-in"
            | "signup"
            | "sign-up"
            | "search"
            | "new-tab"
            | "untitled"
            | "loading"
            | "error"
            | "404"
            | "not-found"
            | "home"
            | "index"
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_empty_url_names_do_not_crash_narration() {
        for resource in ["https://", "http://", "https:////", "http:////"] {
            assert_eq!(meaningful_name(resource), None);
        }
    }

    #[test]
    fn an_address_never_becomes_a_title() {
        // The reported failure, at the naming layer: a watched video was
        // presented *and named* by its own address. Every one of these has no
        // readable subject, so the title path must refuse them rather than
        // return a host or a truncated URL.
        let names = crate::name_index::NameIndex::empty();
        for address in [
            "https://www.youtube.com/watch?v=abc123",
            "https://www.youtube.com/",
            "https://www.google.com/search?q=x",
            "https://claude.ai/chat/0743f7a1-9c2e-4b1f-8a3d-2f6e5c4b1a09",
            "file:///Users/x/Downloads/",
            "/Users/x/",
        ] {
            assert_eq!(
                meaningful_name(address),
                None,
                "{address} produced a structural name"
            );
            assert_eq!(
                title_or_structural(&names, address),
                None,
                "{address} was accepted as a title"
            );
        }
    }

    #[test]
    fn a_readable_url_path_still_titles_a_work() {
        // The refusal above must not cost real titles: a URL whose path says
        // something is still the best name available.
        let names = crate::name_index::NameIndex::empty();
        let titled = title_or_structural(&names, "https://github.com/evo/evo-capture/pull/42")
            .expect("a URL with readable path segments titles the work");
        assert!(titled.contains("Evo"), "got {titled:?}");
    }

    #[test]
    fn a_generic_interface_page_is_refused_but_a_generic_file_stem_is_not() {
        // The generic-name guard is URL-only on purpose. `index.rs` and
        // `home.rs` are real, meaningful files in a codebase and refusing them
        // would be a regression; a login *page* identifies nothing.
        assert_eq!(
            meaningful_name("/Users/x/evo/crates/evo-daemon/src/index.rs"),
            Some("Index".to_string())
        );
        assert_eq!(meaningful_name("https://example.com/login"), None);
        assert_eq!(meaningful_name("https://example.com/search?q=x"), None);
    }

    #[test]
    fn a_watched_video_is_titled_honestly_not_by_its_address() {
        // The whole reported path, end to end. Typing on a video page gives
        // the thread production, so the engine is right to mint it — the fix
        // is that it must be *named* by the honest fallback, never by the
        // address it happens to live at.
        let address = "https://www.youtube.com/watch?v=abc123";
        let events = vec![
            event(1_000, Interaction::Typed, address, Some(500)),
            event(
                1_000 + 5 * 60_000,
                Interaction::Focused,
                address,
                Some(5 * 60_000),
            ),
        ];
        let engine = settled(&events);
        let threads = engine.threads();
        let thread = threads.first().expect("the typed page mints a thread");
        let understanding = understand(thread, &engine.resume_bundle(thread.id));
        assert!(
            understanding.title.starts_with("Untitled"),
            "a video page must not be titled by its address; got {:?}",
            understanding.title
        );
        assert!(
            !understanding.title.contains("youtube") && !understanding.title.contains("://"),
            "the title still carries the address: {:?}",
            understanding.title
        );
        assert!(
            understanding.title.contains("place"),
            "the fallback must state what was measured; got {:?}",
            understanding.title
        );
    }

    #[test]
    fn activity_phrase_is_the_fallback_for_opaque_surfaces() {
        let events = vec![
            event(1_000, Interaction::Typed, "window:opaque/101", Some(500)),
            event(
                1_000 + 5 * 60_000,
                Interaction::Focused,
                "window:opaque/101",
                Some(5 * 60_000),
            ),
        ];
        let engine = settled(&events);
        let threads = engine.threads();
        let thread = threads.first().expect("opaque activity mints work");
        let activity = vec![ActivityEvidence {
            resource: "window:opaque/101".into(),
            action: ActivityAction::Typed,
            observed_at: 1_000,
            weight: 500,
        }];
        let understanding = understand_classified_with_activity(
            thread,
            &engine.resume_bundle(thread.id),
            &crate::name_index::NameIndex::empty(),
            &crate::classification::ClassifiedSurfaces::empty(),
            &activity,
        );
        assert_eq!(understanding.title, "Writing across 1 place");
    }

    #[test]
    fn title_metrics_reject_addresses_and_opaque_identifiers() {
        let metrics = title_metrics([
            "Writing across 1 place",
            "Untitled — 1 place",
            "https://example.com/x",
            "window:opaque/101",
            "Search",
        ]);
        assert_eq!(metrics.presented_work_count, 5);
        assert_eq!(metrics.untitled_count, 1);
        assert_eq!(metrics.identifier_count, 3);
        assert_eq!(metrics.readable_count, 2);
        assert!(!metrics.identifier_floor_met());
    }

    #[test]
    fn title_fixture_has_thirty_presentable_cases_and_zero_identifiers() {
        let titles: Vec<&str> = include_str!("../../../tools/threads-regression/title_fixture.txt")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let metrics = title_metrics(titles.iter().copied());
        assert_eq!(metrics.presented_work_count, 30);
        assert_eq!(metrics.identifier_count, 0);
        assert!(metrics.readability_rate() >= 0.80);
    }

    use evo_threads::{AnchorKind, Declaration, Engine, Event, Interaction, Origin};

    fn event(at: u64, interaction: Interaction, resource: &str, weight: Option<u64>) -> Event {
        Event {
            timestamp: at,
            origin: Origin::Person,
            interaction,
            resource: resource.into(),
            weight,
            detail: None,
        }
    }

    fn settled(events: &[Event]) -> Engine {
        let mut engine = Engine::replay(events);
        engine.settle(events.last().unwrap().timestamp + 86_400_000);
        engine
    }

    fn assert_neutral(engine: &Engine) {
        assert!(
            !engine.threads().is_empty(),
            "fixture has actual identity evidence"
        );
        for thread in engine.threads() {
            let understanding = understand(&thread, &engine.resume_bundle(thread.id));
            assert_eq!(understanding.phase, WorkPhase::ObservedActivity);
            assert_eq!(understanding.phase.label(), "Observed activity");
            assert!(
                understanding.completed.is_empty(),
                "anchors are not completion statements"
            );
            assert!(!understanding.narrative.contains("Completed:"));
            assert!(
                understanding
                    .trail
                    .iter()
                    .all(|entry| entry.action == TrailAction::Interacted)
            );
        }
    }

    #[test]
    fn work_understanding_does_not_assign_semantic_phase_from_host_or_extension() {
        for resource in [
            "https://research.test/étude/方法",
            "https://coding.test/संवाद",
            "file:///fictional/कथा/recette.rs",
            "file:///fictional/文献/сцена.jpg",
            "window:fictional/101/title=Inventario — مخزون",
        ] {
            // Real typing witnesses input, but not coding, research, writing,
            // reviewing or task completion. Changing the locator changes none
            // of those conclusions.
            assert_neutral(&settled(&[event(
                1_000,
                Interaction::Typed,
                resource,
                Some(200),
            )]));
            // Real saves do not imply that the person's task is complete either.
            assert_neutral(&settled(&[
                event(1_000, Interaction::Saved, resource, None),
                event(2_000, Interaction::Saved, resource, None),
            ]));
        }
    }

    #[test]
    fn work_understanding_recurrence_and_declarations_do_not_imply_saves() {
        let resource = "https://archive.test/祖母/история";
        let recurring = settled(&[
            event(1_000, Interaction::Visited, resource, Some(90_000)),
            event(86_401_000, Interaction::Visited, resource, Some(90_000)),
        ]);
        assert!(
            recurring.threads()[0]
                .anchors
                .iter()
                .all(|a| a.kind == AnchorKind::Recurrence)
        );
        assert_neutral(&recurring);

        let declared = settled(&[Event {
            timestamp: 1_000,
            origin: Origin::Person,
            interaction: Interaction::Declared,
            resource: String::new(),
            weight: None,
            detail: Some(Declaration::SameWork {
                a: resource.into(),
                b: "file:///fictional/échantillon.png".into(),
            }),
        }]);
        assert!(
            declared.threads()[0]
                .anchors
                .iter()
                .all(|a| a.kind == AnchorKind::Mutation)
        );
        assert_neutral(&declared);
    }

    #[test]
    fn work_understanding_download_prefix_does_not_witness_a_download_or_completion() {
        let engine = settled(&[event(
            1_000,
            Interaction::Typed,
            "Downloaded:/fictional/चित्र.jpg",
            Some(200),
        )]);
        assert_neutral(&engine);
        let thread = &engine.threads()[0];
        let understanding = understand(thread, &engine.resume_bundle(thread.id));
        assert_eq!(understanding.trail[0].action, TrailAction::Interacted);
    }

    #[test]
    fn work_understanding_preserves_witnessed_multilingual_names_and_exact_identity() {
        let root = "file:///fictional/recueil.md";
        let photo = "file:///fictional/photo.png";
        let url = "https://oral-history.test/record/57";
        let engine = settled(&[
            event(1_000, Interaction::Typed, root, Some(200)),
            event(2_000, Interaction::Visited, photo, Some(60_000)),
            event(3_000, Interaction::Visited, url, Some(90_000)),
            event(310_000, Interaction::Typed, root, Some(80)),
        ]);
        let mut names = crate::name_index::NameIndex::empty();
        names.record(root, "Café — حكاية — 記録", 1_000);
        names.record(photo, "Été / 写真 / бабушка", 2_000);
        names.record(url, "कथा — Memória", 3_000);
        let thread = &engine.threads()[0];
        let result = understand_named(thread, &engine.resume_bundle(thread.id), &names);
        assert_eq!(result.title, "Café — حكاية — 記録");
        for (raw, name) in [
            (root, "Café — حكاية — 記録"),
            (photo, "Été / 写真 / бабушка"),
            (url, "कथा — Memória"),
        ] {
            assert!(
                result
                    .trail
                    .iter()
                    .any(|entry| entry.raw == raw && entry.name == name)
            );
        }
        assert_neutral(&engine);
    }

    #[test]
    fn work_understanding_same_title_does_not_collapse_two_surface_identities() {
        let a = "window:fictional-session/101";
        let b = "window:fictional-session/202";
        let engine = settled(&[
            event(1_000, Interaction::Typed, a, Some(200)),
            event(86_401_000, Interaction::Typed, b, Some(200)),
        ]);
        assert_eq!(engine.threads().len(), 2);
        let mut names = crate::name_index::NameIndex::empty();
        names.record(a, "Café — 計画", 1_000);
        names.record(b, "Café — 計画", 86_401_000);
        let results: Vec<_> = engine
            .threads()
            .iter()
            .map(|thread| understand_named(thread, &engine.resume_bundle(thread.id), &names))
            .collect();
        assert_eq!(results[0].title, results[1].title);
        let raws: std::collections::BTreeSet<_> = results
            .iter()
            .flat_map(|result| result.trail.iter().map(|entry| entry.raw.as_str()))
            .collect();
        assert_eq!(raws, std::collections::BTreeSet::from([a, b]));
    }

    #[test]
    fn work_understanding_a_classified_surface_never_names_a_work() {
        // The engine (built without the classification) anchors the video and
        // the witnessed name would title the work. The presentation layer
        // holds the person's class against it, so the same work may not be
        // named by a surface they classified — and is not deleted either.
        let video = "https://www.youtube.com/watch?v=EsW2BoFAUs";
        let engine = settled(&[
            event(1_000, Interaction::Typed, video, Some(200)),
            event(2_000, Interaction::Saved, video, None),
        ]);
        let mut names = crate::name_index::NameIndex::empty();
        names.record(video, "A Video I Watched - YouTube", 1_000);
        let thread = &engine.threads()[0];
        let bundle = engine.resume_bundle(thread.id);

        let named = understand_named(thread, &bundle, &names);
        assert!(
            named.title.contains("A Video I Watched"),
            "without a class the witnessed name titles the work, got {:?}",
            named.title
        );

        let mut classified = crate::classification::ClassifiedSurfaces::empty();
        classified.record(video, "NotWork");
        let gated = understand_classified(thread, &bundle, &names, &classified);
        assert!(
            !gated.title.contains("A Video I Watched"),
            "a classified surface must not name a work, got {:?}",
            gated.title
        );
        // Non-deletion: the classified surface is still witnessed in the trail.
        assert!(
            gated.trail.iter().any(|entry| entry.raw == video),
            "a class must never remove the subject from the work"
        );
    }

    #[test]
    fn work_understanding_retains_observed_open_deltas_without_completed_claims() {
        let resource = "file:///fictional/plan-é.md";
        let engine = settled(&[
            event(1_000, Interaction::Typed, resource, Some(200)),
            event(2_000, Interaction::Saved, resource, None),
            event(3_000, Interaction::Typed, resource, Some(80)),
        ]);
        let thread = &engine.threads()[0];
        let result = understand(thread, &engine.resume_bundle(thread.id));
        assert_eq!(result.pending, vec!["Unsaved changes in Plan É"]);
        assert_eq!(result.next_step, result.pending[0]);
        assert_neutral(&engine);
    }
}
