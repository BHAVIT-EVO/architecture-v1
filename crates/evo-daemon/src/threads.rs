//! The bridge from the canonical Observation log to the threads engine.
//!
//! The desktop's work list was served by the Engagement reconstruction —
//! the affinity/clustering pipeline whose singleton fragmentation is the
//! known failure this crate family is being replaced by. This module reads
//! the same append-only `observation.log` that pipeline read, projects each
//! canonical Observation onto the engine's event contract, replays the
//! engine over it, and settles at read time. One source of truth, two
//! readers; the new one does not interpret, it attributes.
//!
//! Projection rules (Observation → `evo_threads::Event`):
//!
//! | Observation | Engine event | Notes |
//! |---|---|---|
//! | OBS-WINDOW-FOCUS-GAINED | `Person/Focused` | The subject the AX source resolved. |
//! | OBS-URL-NAVIGATED | `Person/Visited` | |
//! | OBS-FILE-SAVED | `Person/Mutated` | **Deliberately not `Saved`.** The log's save records are FSEvents writes, and on this corpus ~94% of them are service churn (photo-library plists, IDE telemetry) indistinguishable from real saves at the record level. Routed as `Mutated`, the engine's churn gate applies: a write counts as the person's only where the person was actually at that resource. Real saves on document-grain capture (focus on the `file://` path) pass; churn never does. On today's title-grain log both are excluded — the honest direction, and the exact failure the old pipeline's D2 mega-component came from. |
//! | OBS-INPUT-ACTIVITY | `Person/Typed` (weight = keys) | Content-free counters, once the daemon records them. |
//! | OBS-WORK-GROUPED | `Person/Declared(SameWork)` | The user's word; the only merge path. |
//! | OBS-COMMIT-MADE, OBS-REPOSITORY-MEMBERSHIP, designations, surfaces | *no event* | Identity context the engine does not consume yet; dropping is honest, guessing is not. |
//!
//! Origin: every mapped observation is `Person`. The log's provenance
//! distinguishes sources (`macos_fsevents`, `macos_accessibility`), not
//! person vs. system, and every concept above is defined as a person act.
//! Churn wears the same badge, which is why the FILE-SAVED row routes
//! through the mutation gate rather than trusting the badge.

use crate::errors::DaemonError;
use crate::persistence::load_persisted_observations;
use evo_observation::evidence::FactValue;
use evo_observation::observation::Observation;
use evo_threads::{Declaration, Engine, Event, Interaction, Origin};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A thread the desktop can render: the engine's ThreadView paired with a
/// human-readable name, the resume bundle, and a quality tier that decides
/// whether it belongs on the Home surface at all.
#[derive(Debug, Clone)]
pub struct DisplayThread {
    pub thread_id: u64,
    /// A short, human-readable name derived from the thread's own material.
    pub name: String,
    /// The engine's resume bundle for this thread.
    pub bundle: evo_threads::ResumeBundle,
    /// Whether this thread is real work (vs. download noise or a reflex).
    pub is_work: bool,
    /// Engagement score for ranking: episodes × anchors, so the work the
    /// person returned to most ranks first.
    pub engagement: f64,
    /// The understanding layer's narrative: phase, progress, next step.
    pub understanding: crate::work_understanding::WorkUnderstanding,
    /// When this work was last active, in epoch milliseconds. Drives the
    /// recency decay: fresh work ranks high, old work fades. None means
    /// the engine had no timestamp (treated as fresh).
    pub last_active_ms: Option<u64>,
    /// The raw witnesses of this work's member pages (URLs, document
    /// paths, window titles) — the vocabulary the engine's merge
    /// proposals speak. Presentation matching by membership, never name.
    pub member_subjects: Vec<String>,
}

/// The quality layer: takes the engine's raw thread views and produces
/// what the desktop renders — named, ranked, noise-filtered.
///
/// Three rules, each principled:
///
/// 1. **Download-only threads are not work.** A thread whose every anchor
///    is a `Downloaded:` resource is file management, not a body of work.
///    The person downloaded things; they did not *do* anything. Filtered
///    from the list — they remain in the engine, retrievable by name, but
///    they do not clutter the surface.
///
/// 2. **Names come from the work's own material.** The domain of the most-
///    attended URL, or the file name of the strongest anchor — never a
///    guess, never a category, never an application name.
///
/// 3. **Ranking follows engagement.** Episodes × anchor count: the work the
///    person returned to across the most sittings, with the most produced-
///    on surfaces, comes first. Not recency (which favors a reflex), not
///    downloads (which favor churn).
pub fn display_threads(engine: &Engine) -> Vec<DisplayThread> {
    let threads = engine.threads();
    let mut display = Vec::new();

    for thread in &threads {
        let bundle = engine.resume_bundle(thread.id);

        // Rule 1: a thread whose every anchor is a download is not work,
        // and a thread with nothing openable to restore offers the person
        // nothing. Both are filtered — kept in the engine, off the surface.
        let all_downloads = !thread.anchors.is_empty()
            && thread
                .anchors
                .iter()
                .all(|a| a.resource.starts_with("Downloaded:"));
        let has_openable = bundle
            .restore_set
            .iter()
            .any(|r| !r.starts_with("Downloaded:"));
        if all_downloads || !has_openable {
            continue;
        }

    // Rule 2: derive a readable name from the thread's own material.
    let name = thread_name(thread, &bundle);

    // Rule 3: engagement = episodes × distinct anchors.
    let engagement = (thread.episodes.len() as f64)
        * (thread.anchors.iter().filter(|a| !a.resource.starts_with("Downloaded:")).count().max(1) as f64);

    // The restore set the desktop offers: the engine's set, filtered to
    // things that can actually be reopened. A downloaded file sitting in
    // ~/Downloads is not a surface to restore — it is a product of the
    // work, not a tool for it. URLs and file paths reopen; downloads do
    // not.
    let openable_restore: Vec<String> = bundle
        .restore_set
        .iter()
        .filter(|r| !r.starts_with("Downloaded:"))
        .cloned()
        .collect();

    // The brief note: what the person was doing, in one sentence, from the
    // thread's own material. "You were working on X, with Y and Z open."
    let brief_note = brief_note(thread, &bundle);

    // Rebuild the bundle with the filtered restore set and the note as the
    // reason, so the desktop renders the product-facing version.
    let display_bundle = evo_threads::ResumeBundle {
        resume_point: bundle.resume_point.clone(),
        resume_reason: brief_note,
        restore_set: openable_restore,
        open_deltas: bundle.open_deltas.clone(),
    };

    // The understanding layer: derive phase, progress, next step, narrative.
    let understanding = crate::work_understanding::understand(thread, &bundle);

    // The work's last activity: the most recent episode's end.
    let last_active_ms = thread
        .episodes
        .last()
        .map(|ep| ep.span.1)
        .filter(|ts| *ts > 0);

    display.push(DisplayThread {
        thread_id: thread.id,
        name: understanding.title.clone(),
        bundle: display_bundle,
        is_work: !thread.anchors.is_empty(),
        engagement,
        understanding,
        last_active_ms,
        // The legacy thread path has no merge proposals; the hypothesis
        // engine's threads carry their member witnesses.
        member_subjects: Vec::new(),
    });
    }

    // ─── Work lifecycle ─────────────────────────────────────────────────────
    //
    // Per the research (Recall's fade-out, Apple's expirationDate, every
    // competitor's answer): salience decays, existence doesn't.
    //
    // * A work untouched for >14 days drops below fresh work in ranking
    //   (exponential decay, 2-week half-life on engagement).
    // * A work untouched for >30 days fades from the default surface
    //   entirely (still retrievable by name, never shown unprompted).
    // * Nothing is ever auto-deleted: local-first means deletion is the
    //   person's call.
    //
    // The decay factor: engagement × 0.5^(days_since_last_activity / 14).
    // Fresh work is unaffected; two-week-old work is halved; month-old
    // work is quartered — so today's session outranks last month's
    // marathon without erasing it.
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let display_retained: Vec<DisplayThread> = display
        .into_iter()
        .filter_map(|mut dt| {
            // Decay the engagement score by recency.
            if let Some(last_ms) = last_activity_ms(&dt) {
                let days = (now_ms.saturating_sub(last_ms)) as f64 / 86_400_000.0;
                dt.engagement *= 0.5_f64.powf(days / 14.0);
                // Faded work: untouched for 30+ days. Off the default
                // surface; still in the engine, findable by name.
                if days > 30.0 {
                    return None;
                }
            }
            Some(dt)
        })
        .collect();

    let mut display = display_retained;
    display.sort_by(|a, b| {
        b.engagement
            .partial_cmp(&a.engagement)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.thread_id.cmp(&b.thread_id))
    });
    display
}

/// Finds the work the person means by a natural phrase, over the
/// display threads' names and understanding text.
///
/// "reopen my Evo workspace" → the thread whose name, narrative, or trail
/// contains "Evo". Returns the index into the display list, or None.
pub fn find_work(query: &str, works: &[DisplayThread]) -> Option<usize> {
    let query_lower = query.trim().to_lowercase();
    if query_lower.is_empty() {
        return None;
    }
    let tokens: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|t| !is_stop_word(t))
        .collect();
    if tokens.is_empty() {
        return None;
    }

    let mut best: Option<(usize, f64)> = None;
    for (index, work) in works.iter().enumerate() {
        let mut score = 0.0;
        // Match against the title (strongest signal)
        let title_lower = work.name.to_lowercase();
        for token in &tokens {
            if title_lower.contains(token) {
                score += 10.0;
            }
        }
        // Match against the narrative
        let narrative_lower = work.understanding.narrative.to_lowercase();
        for token in &tokens {
            if narrative_lower.contains(token) {
                score += 3.0;
            }
        }
        // Match against the trail entries
        for entry in &work.understanding.trail {
            let entry_lower = entry.name.to_lowercase();
            for token in &tokens {
                if entry_lower.contains(token) {
                    score += 2.0;
                }
            }
        }
        // Match against the pending items
        for item in &work.understanding.pending {
            let item_lower = item.to_lowercase();
            for token in &tokens {
                if item_lower.contains(token) {
                    score += 1.0;
                }
            }
        }
        if score > 0.0 {
            match best {
                Some((_, best_score)) if best_score >= score => {}
                _ => best = Some((index, score)),
            }
        }
    }
    best.map(|(index, _)| index)
}

fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "the" | "a" | "an" | "my" | "open" | "hey" | "evo" | "can" | "you" | "continue"
            | "resume" | "work" | "please" | "just" | "only" | "that" | "this" | "for" | "me"
            | "of" | "in" | "on" | "at" | "to" | "is" | "it" | "and" | "or" | "with" | "was"
            | "reopen" | "workspace" | "find" | "show" | "restore"
    )
}

/// The last activity timestamp of a display thread, from its most recent
/// episode's span, in epoch milliseconds.
fn last_activity_ms(dt: &DisplayThread) -> Option<u64> {
    dt.last_active_ms
}

/// Derives a short, human-readable name from the thread's own witnessed
/// material. The research is clear: names should read as what the work IS,
/// not where it lives. "Pitch Deck" not "app.pitch.com". "Demo Build" not
/// "canva.com".
///
/// Priority: the most meaningful anchor name (a file's stem, a URL's page
/// title), falling back to a natural combination of surfaces. Never a raw
/// domain, never an app name, never a category — the work's own identity.
fn thread_name(thread: &evo_threads::ThreadView, bundle: &evo_threads::ResumeBundle) -> String {
    // Collect the distinct, non-download surfaces
    let mut surfaces: Vec<String> = Vec::new();
    let mut push = |resource: &str| {
        if resource.starts_with("Downloaded:") || surfaces.len() >= 3 {
            return;
        }
        if let Some(name) = meaningful_name(resource) {
            if !surfaces.contains(&name) {
                surfaces.push(name);
            }
        }
    };
    // Prefer URL anchors (they carry more context), then the resume point
    for anchor in thread.anchors.iter() {
        if anchor.resource.starts_with("http") {
            push(&anchor.resource);
        }
    }
    push(&bundle.resume_point);
    for anchor in thread.anchors.iter() {
        push(&anchor.resource);
    }
    for (resource, _) in thread.companions.iter() {
        push(resource);
    }

    match surfaces.len() {
        0 => format!("Work ({})", thread.episodes.len()),
        1 => surfaces[0].clone(),
        2 => format!("{} + {}", surfaces[0], surfaces[1]),
        _ => format!("{} + {}", surfaces[0], surfaces[1]),
    }
}

/// A meaningful name for a resource: the page slug from a URL (not the
/// domain), the file stem from a path. Strips UUIDs, query parameters,
/// and technical identifiers. This is what a human would call it.
fn meaningful_name(resource: &str) -> Option<String> {
    if resource.starts_with("http") {
        let url = resource.trim_start_matches("https://").trim_start_matches("http://");
        let parts: Vec<&str> = url.split('/').filter(|p| !p.is_empty()).collect();

        let skip = ["app", "www", "c", "chat", "projects", "design", "watch",
                    "edit", "v", "u", "p", "g", "d", "e"];
        let candidates: Vec<String> = parts[1..]
            .iter()
            .filter_map(|p| {
                // Strip query params and fragments
                let clean = p.split('?').next()?.split('#').next()?;
                if clean.is_empty() || skip.contains(&clean) {
                    return None;
                }
                // Strip double-dash suffixes (Framer project IDs):
                // "waitlist-template-copy--mjkw3dlVcNf5JWzPBC2A-1wDmz" → "waitlist-template-copy"
                let name_part = if let Some(pos) = clean.find("--") {
                    &clean[..pos]
                } else {
                    clean
                };
                if name_part.len() < 3 || is_identifier(name_part) {
                    return None;
                }
                Some(name_part)
            })
            .take(2)
            .map(|s: &str| s.to_string())
            .collect::<Vec<String>>();

        if !candidates.is_empty() {
            let cleaned: Vec<String> = candidates
                .iter()
                .map(|s| title_case(s))
                .collect();
            return Some(cleaned.join(" — "));
        }
        let domain = parts.first()?.trim_start_matches("www.");
        return Some(domain.to_string());
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
    if resource.len() > 3 && !resource.starts_with("app:") && !resource.starts_with("term:") {
        return Some(resource.chars().take(30).collect());
    }
    None
}

/// Whether a string looks like a technical identifier (UUID, hex, numeric ID)
/// rather than a human-readable name.
fn is_identifier(s: &str) -> bool {
    // UUIDs contain 4+ consecutive dashes/hyphens
    if s.matches('-').count() >= 4 {
        return true;
    }
    // Pure hex or alphanumeric IDs (no vowels or too short)
    if s.len() >= 8 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return true;
    }
    // Pure numbers
    if s.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    false
}

/// Title-cases a slug: "my-cool-project" → "My Cool Project"
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

/// A readable name for one resource: the domain of a URL, the file name
/// of a path, or a truncated title. Never an application name, never a
/// category — the work's own identity, stated plainly.
fn readable_surface_name(resource: &str) -> Option<String> {
    if resource.starts_with("http") {
        let url = resource.trim_start_matches("https://").trim_start_matches("http://");
        let domain = url.split('/').next()?;
        // Strip "www." and take the base domain.
        let domain = domain.trim_start_matches("www.");
        if domain.is_empty() {
            return None;
        }
        return Some(domain.to_string());
    }
    if resource.starts_with("file://") || resource.starts_with('/') {
        let path = resource.trim_start_matches("file://");
        let file = path.rsplit('/').next()?;
        if file.is_empty() {
            return None;
        }
        // Strip extension for readability.
        let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(file);
        return Some(stem.to_string());
    }
    // A window title or app name: truncate to the first meaningful segment.
    if resource.len() > 3 {
        return Some(resource.chars().take(30).collect());
    }
    None
}

/// The brief note: what the person was doing, in one or two sentences,
/// derived entirely from the thread's own witnessed material.
///
/// The note names the surfaces the person was on — not what they mean, not
/// what category they belong to, just where the attention and production
/// lived. "Working in kie.ai with framer and canva open" is a fact; "building
/// a pitch deck" would be a guess the engine has no evidence for.
fn brief_note(thread: &evo_threads::ThreadView, bundle: &evo_threads::ResumeBundle) -> String {
    // Collect the distinct surfaces: the resume point, then the strongest
    // anchors, then companions — up to three names total.
    let mut surfaces: Vec<String> = Vec::new();
    let mut push_surface = |resource: &str, surfaces: &mut Vec<String>| {
        if surfaces.len() >= 3 {
            return;
        }
        if resource.starts_with("Downloaded:") {
            return; // a download is a product, not a surface
        }
        if let Some(name) = readable_surface_name(resource) {
            if !surfaces.contains(&name) {
                surfaces.push(name);
            }
        }
    };

    push_surface(&bundle.resume_point, &mut surfaces);
    for anchor in thread.anchors.iter().take(3) {
        push_surface(&anchor.resource, &mut surfaces);
    }
    for (resource, _) in thread.companions.iter().take(2) {
        push_surface(resource, &mut surfaces);
    }

    if surfaces.is_empty() {
        return format!("{} sessions of work", thread.episodes.len());
    }

    let session_word = if thread.episodes.len() == 1 { "session" } else { "sessions" };
    match surfaces.len() {
        1 => format!("Working in {} across {} {}.", surfaces[0], thread.episodes.len(), session_word),
        2 => format!(
            "Working in {} with {} open, across {} {}.",
            surfaces[0], surfaces[1], thread.episodes.len(), session_word
        ),
        _ => format!(
            "Working across {}, {}, and {}, over {} {}.",
            surfaces[0], surfaces[1], surfaces[2], thread.episodes.len(), session_word
        ),
    }
}
pub fn engine_from_live_data(root: &Path) -> Result<Engine, DaemonError> {
    // The SQLite DB lives at the parent of the storage root (the daemon's
    // data directory): ~/Library/Application Support/evo/evo.db, while the
    // storage root is ~/Library/Application Support/evo/storage/. Try both
    // the root itself and its parent, because callers may pass either.
    let sqlite_candidates = [
        root.join("evo.db"),
        root.parent().map(|p| p.join("evo.db")).unwrap_or_default(),
    ];

    // Collect events from both sources: the SQLite store (the old daemon's
    // window/URL/download events) AND the observation log (the input
    // counter's InputActivity observations). Both are Person-origin; the
    // merge is by timestamp so the engine sees one interleaved stream.
    let mut all_events: Vec<Event> = Vec::new();

    for sqlite_path in &sqlite_candidates {
        if sqlite_path.exists() {
            match read_sqlite_events(sqlite_path) {
                Ok(events) => {
                    all_events.extend(events);
                    break;
                }
                Err(err) => {
                    eprintln!("EVO-THREADS sqlite adapter failed: {err}");
                }
            }
        }
    }

    // The observation log carries InputActivity from the standalone counter
    // (and any other new-source observations the old daemon doesn't write).
    // These are the production signals the engine was starving for.
    if let Ok(observations) = load_persisted_observations(root) {
        for observation in &observations {
            if let Some(event) = project_observation(observation) {
                if event.interaction == Interaction::Typed {
                    all_events.push(event);
                }
            }
        }
    }

    // Sort by timestamp so the engine sees one interleaved stream.
    all_events.sort_by_key(|e| e.timestamp);

    if all_events.is_empty() {
        return Err(DaemonError::Designation(
            "no events from any source".to_string(),
        ));
    }

    let mut engine = Engine::replay(&all_events);
    let settle_at = all_events.last().map(|e| e.timestamp).unwrap_or(0) + 86_400_000;
    engine.settle(settle_at);
    Ok(engine)
}

/// Reads the SQLite canonical store and replays the engine over it.
///
/// The SQLite store is the legacy daemon's schema: window events (app,
/// title, URL, dwell) and downloads, without origin or interaction-class
/// columns. The projection is honest about what it cannot know: everything
/// is Person-origin (the old store did not distinguish), focus events are
/// `Focused` with the row's dwell_ms as weight (capped at one interval),
/// URL-bearing rows are `Visited`, and downloads are `Downloaded`.
fn engine_from_sqlite(path: &Path) -> Result<Engine, DaemonError> {
    let events = read_sqlite_events(path)?;
    let mut engine = Engine::replay(&events);
    let settle_at = events.last().map(|e| e.timestamp).unwrap_or(0) + 86_400_000;
    engine.settle(settle_at);
    Ok(engine)
}

/// Reads events from the SQLite store, projecting each row onto the engine's
/// contract. No rusqlite dependency: the store is read through the `sqlite3`
/// CLI, which is always present on macOS. Output is JSON, parsed minimally.
fn read_sqlite_events(path: &Path) -> Result<Vec<Event>, DaemonError> {
    use std::process::Command;

    // Query: URL if present, else window title; dwell capped at 10 minutes.
    // Ordered by timestamp. Limited to the last 7 days to keep the replay
    // fast on a store with 68k rows.
    let query = r#"
        SELECT ts_ms,
               COALESCE(normalized_url, window_title),
               event_kind,
               MIN(dwell_ms, 600000),
               download_path
        FROM canonical_events
        WHERE ts_ms > (SELECT MAX(ts_ms) - 604800000 FROM canonical_events)
        ORDER BY ts_ms
    "#;

    let output = Command::new("sqlite3")
        .arg("-json")
        .arg(path)
        .arg(query)
        .output()
        .map_err(|err| DaemonError::Designation(format!("sqlite3 read failed: {err}")))?;

    if !output.status.success() {
        return Err(DaemonError::Designation(format!(
            "sqlite3 query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let json = String::from_utf8_lossy(&output.stdout);
    let mut events = Vec::new();

    // Minimal JSON array-of-objects parsing: each line is one object.
    // The sqlite3 -json output is [{...}, {...}, ...] but we can parse
    // each {...} block directly since the fields are known.
    for line in json.lines() {
        let trimmed = line.trim().trim_start_matches('[').trim_end_matches(']');
        if trimmed.is_empty() || !trimmed.starts_with('{') {
            continue;
        }
        // Parse the known fields from the JSON object.
        let ts = extract_json_u64(trimmed, "ts_ms").unwrap_or(0);
        if ts == 0 {
            continue;
        }
        let resource = extract_json_string(trimmed, "COALESCE(normalized_url, window_title)")
            .or_else(|| extract_json_string(trimmed, "normalized_url"))
            .or_else(|| extract_json_string(trimmed, "window_title"))
            .unwrap_or_default();
        if resource.is_empty() {
            continue;
        }
        let kind = extract_json_string(trimmed, "event_kind").unwrap_or_default();
        let dwell = extract_json_u64(trimmed, "MIN(dwell_ms, 600000)").unwrap_or(0);

        let event = if kind == "download" {
            Event {
                timestamp: ts,
                origin: Origin::Person,
                interaction: Interaction::Downloaded,
                resource: normalize_download_resource(&resource),
                weight: None,
                detail: None,
            }
        } else if resource.starts_with("http") {
            Event {
                timestamp: ts,
                origin: Origin::Person,
                interaction: Interaction::Visited,
                resource,
                weight: Some(dwell.max(1000)),
                detail: None,
            }
        } else {
            Event {
                timestamp: ts,
                origin: Origin::Person,
                interaction: Interaction::Focused,
                resource,
                weight: Some(dwell.max(1000)),
                detail: None,
            }
        };
        events.push(event);
    }

    Ok(events)
}

/// Extracts a u64 value from a flat JSON object string by key.
fn extract_json_u64(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\":");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Extracts a string value from a flat JSON object by key.
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Normalizes a download resource name the way the engine expects:
/// strips macOS copy counters (" (1)", " (2)") before the extension.
fn normalize_download_resource(resource: &str) -> String {
    let dot = resource.rfind('.').unwrap_or(resource.len());
    let (stem, ext) = resource.split_at(dot);
    if let Some(open) = stem.rfind(" (") {
        if stem.ends_with(')') {
            let num = &stem[open + 2..stem.len() - 1];
            if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                return format!("{}{}", &stem[..open], ext);
            }
        }
    }
    resource.to_string()
}

/// Builds a settled engine over every persisted observation under `root`.
///
/// The engine is replayed in append order (the log's order is the witness
/// order) and settled one day past the last observed event, so the final
/// sitting is visible. Pure function of the log; identical logs produce
/// identical thread views.
pub fn engine_from_storage(root: &Path) -> Result<Engine, DaemonError> {
    let observations = load_persisted_observations(root)?;
    let mut events = observations
        .iter()
        .filter_map(project_observation)
        .collect::<Vec<Event>>();
    // The log's records carry no dwell; what they carry is the *next*
    // witnessed act. A focused surface held attention until the next thing
    // that happened, bounded by one attention interval so a window left
    // focused overnight contributes one interval, not the night. This is
    // the same bound the old pipeline measured, applied where it belongs:
    // at the projection boundary, with the engine's own cap doing the
    // discounting.
    let cap = 600_000u64;
    for index in 0..events.len() {
        if !matches!(
            events[index].interaction,
            Interaction::Focused | Interaction::Visited
        ) {
            continue;
        }
        let held_until = events
            .get(index + 1)
            .map(|next| next.timestamp.saturating_sub(events[index].timestamp))
            .unwrap_or(0);
        events[index].weight = Some(held_until.min(cap));
    }
    let mut engine = Engine::replay(&events);
    let settle_at = observations
        .iter()
        .map(|observation| milliseconds(observation.provenance().observed_at()))
        .max()
        .unwrap_or(0)
        + 86_400_000;
    engine.settle(settle_at);
    Ok(engine)
}

/// Projects one canonical Observation onto the engine's contract, or `None`
/// for the concepts the engine does not consume.
fn project_observation(observation: &Observation) -> Option<Event> {
    let timestamp = milliseconds(observation.provenance().observed_at());
    let subject = || {
        observation
            .evidence()
            .fact(observation.schema().canonical_fact_name()?)
            .and_then(|fact| match fact.value() {
                FactValue::Text(subject) => Some(subject.clone()),
                _ => None,
            })
    };

    match observation.schema().name() {
        "OBS-WINDOW-FOCUS-GAINED" => Some(Event {
            timestamp,
            origin: Origin::Person,
            interaction: Interaction::Focused,
            resource: subject()?,
            weight: None,
            detail: None,
        }),
        "OBS-URL-NAVIGATED" => Some(Event {
            timestamp,
            origin: Origin::Person,
            interaction: Interaction::Visited,
            resource: subject()?,
            weight: None,
            detail: None,
        }),
        "OBS-FILE-SAVED" => Some(Event {
            timestamp,
            origin: Origin::Person,
            interaction: Interaction::Mutated,
            resource: subject()?,
            weight: None,
            detail: None,
        }),
        "OBS-INPUT-ACTIVITY" => {
            let keys = match observation.evidence().fact("Keys").map(|f| f.value()) {
                Some(FactValue::Integer(keys)) if *keys > 0 => *keys as u64,
                _ => return None,
            };
            Some(Event {
                timestamp,
                origin: Origin::Person,
                interaction: Interaction::Typed,
                resource: subject()?,
                weight: Some(keys),
                detail: None,
            })
        }
        "OBS-WORK-GROUPED" => {
            let first = subject()?;
            let second = match observation.evidence().fact("CoMember").map(|f| f.value()) {
                Some(FactValue::Text(second)) => second.clone(),
                _ => return None,
            };
            Some(Event {
                timestamp,
                origin: Origin::Person,
                interaction: Interaction::Declared,
                resource: String::new(),
                weight: None,
                detail: Some(Declaration::SameWork { a: first, b: second }),
            })
        }
        _ => None,
    }
}

fn milliseconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_observation::accept::accept;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::observation_schema::ObservationSchema;
    use evo_observation::provenance::{ObservationSource, Provenance};
    use evo_storage::{Storage, StorageObjectKind};
    use std::collections::HashMap;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("evo-daemon-threads-{tag}-{}", std::process::id()))
    }

    fn persist(root: &Path, at_secs: u64, concept: &ObservationConcept) {
        let _guard = Storage::with_thread_root(root.to_path_buf());
        let storage = Storage::new();
        let schema = concept.schema();
        let provenance = Provenance::new(
            ObservationSource::new("macos_adapter").unwrap(),
            UNIX_EPOCH + std::time::Duration::from_secs(at_secs),
            HashMap::new(),
        );
        let candidate =
            evo_observation::candidate::CandidateObservation::new(schema.clone(), provenance, concept.evidence());
        let observation = accept(candidate, &schema).unwrap();
        let record = crate::persistence::encode_observation_record(&observation);
        storage.append(StorageObjectKind::Observation, record.as_bytes()).unwrap();
    }

    #[test]
    fn focused_files_mint_works_and_cold_saves_are_churn() {
        let root = temp_root("saves");
        // Document-grain pattern: the person is AT the document when it is
        // written — the mutation gate passes and the work stands up.
        persist(&root, 100, &ObservationConcept::WindowFocusGained {
            subject: "file:///work/report.md".into(),
        });
        // The autosave pattern: the editor writes the focused document
        // repeatedly while the person works. Each write passes the mutation
        // gate (the person is at the resource); together they cross the
        // birth floor.
        persist(&root, 110, &ObservationConcept::FileSaved {
            subject: "file:///work/report.md".into(),
        });
        persist(&root, 115, &ObservationConcept::WindowFocusGained {
            subject: "file:///work/report.md".into(),
        });
        persist(&root, 120, &ObservationConcept::FileSaved {
            subject: "file:///work/report.md".into(),
        });
        persist(&root, 125, &ObservationConcept::FileSaved {
            subject: "file:///work/report.md".into(),
        });
        persist(&root, 130, &ObservationConcept::FileSaved {
            subject: "file:///work/notes.md".into(),
        });

        let engine = engine_from_storage(&root).unwrap();
        let threads = engine.threads();
        assert!(!threads.is_empty(), "a focused, saved document mints a thread");
        let bundle = engine.resume_bundle(threads[0].id);
        assert!(
            bundle.resume_point.starts_with("file:///work/report.md"),
            "resume point must be the witnessed work, got {}",
            bundle.resume_point
        );
        // notes.md was written with nobody at it: service churn, quarantined.
        let anchors: Vec<_> = threads.iter()
            .flat_map(|t| t.anchors.iter().map(|a| a.resource.clone()))
            .collect();
        assert!(
            !anchors.iter().any(|r| r.ends_with("notes.md")),
            "a save with no person at the resource must never anchor a work"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cold_save_bursts_never_mint_anything() {
        // The shape of the real corpus's churn: a service rewriting plists
        // beside whatever the person did. No focus ever lands on them.
        let root = temp_root("churn");
        persist(&root, 100, &ObservationConcept::WindowFocusGained {
            subject: "editor".into(),
        });
        for index in 0..5 {
            persist(&root, 105 + index, &ObservationConcept::FileSaved {
                subject: format!("file:///Library/Sync/state-{index}.plist"),
            });
        }

        let engine = engine_from_storage(&root).unwrap();
        assert!(
            engine.threads().is_empty(),
            "five cold plist writes must mint nothing"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unmapped_concepts_project_nothing() {
        // Repository membership is identity context, not engine input.
        let observation_none = ObservationConcept::RepositoryMembership {
            member: "/repo/a.rs".into(),
            repository: "/repo/.git".into(),
        };
        let schema = observation_none.schema();
        let provenance = Provenance::new(
            ObservationSource::new("test").unwrap(),
            UNIX_EPOCH,
            HashMap::new(),
        );
        let candidate = evo_observation::candidate::CandidateObservation::new(
            schema.clone(),
            provenance,
            observation_none.evidence(),
        );
        let observation = accept(candidate, &schema).unwrap();
        assert!(project_observation(&observation).is_none());
    }

    #[test]
    fn grouped_subjects_merge_threads() {
        let root = temp_root("grouped");
        persist(&root, 100, &ObservationConcept::WindowFocusGained { subject: "file:///a.md".into() });
        persist(&root, 110, &ObservationConcept::FileSaved { subject: "file:///a.md".into() });
        persist(&root, 200, &ObservationConcept::WindowFocusGained { subject: "file:///b.md".into() });
        persist(&root, 210, &ObservationConcept::FileSaved { subject: "file:///b.md".into() });
        persist(&root, 300, &ObservationConcept::WorkGrouped {
            first: "file:///a.md".into(),
            second: "file:///b.md".into(),
        });

        let engine = engine_from_storage(&root).unwrap();
        let threads = engine.threads();
        // The declaration is the only merge path; two saved-file sittings
        // joined by the user's word become one work.
        assert!(
            threads.len() <= 2,
            "declaration merges: {} threads",
            threads.len()
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
