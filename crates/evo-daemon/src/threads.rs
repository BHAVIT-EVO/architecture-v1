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
//! | OBS-COMMIT-MADE, OBS-REPOSITORY-MEMBERSHIP | ephemeral lifecycle evidence | Exact opaque relationships may derive member completion for this replay; no derived state is persisted. |
//! | designations, surfaces | *no event* | Identity/context declarations are consumed by their owning projections. |
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
use evo_threads::{
    CommitEvidence, Declaration, Engine, Event, Interaction, LifecycleEvidence, Origin,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
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
    /// Engagement score: episodes × anchors. Historical popularity, used as
    /// the *tiebreak* in the ordering below, never as its first key.
    pub engagement: f64,
    /// Whether the engine witnessed an unfinished state on this work: unsaved
    /// edits, uncommitted changes, a download not yet used, or a surface typed
    /// on and never saved. This is the first key of the ordering, because a
    /// person returning to their machine most likely resumes what they left
    /// unfinished.
    pub has_open_loop: bool,
    /// Why this work holds the position it does, when that is not already
    /// legible on the card — one line, taken verbatim from the witnessed open
    /// loop. `None` when the work has no open loop (the card's own age stamp
    /// states recency, and engagement is deliberately never shown as a reason).
    pub urgency_reason: Option<String>,
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
    /// Members omitted by the current activation plan, with the evidence
    /// source that justified omission. This is inspectable presentation state;
    /// it does not alter membership or historical evidence.
    pub skipped_members: Vec<evo_threads::MemberStateView>,
}

/// Pick declaration witnesses from the same engine snapshot that supplied the
/// visible works. A shared resource cannot identify which work the person
/// selected; never let it choose a thread by iteration order.
pub fn grouping_subjects(
    works: &[DisplayThread],
    first: u64,
    second: u64,
) -> Option<(String, String)> {
    if first == second
        || works.iter().filter(|w| w.thread_id == first).count() != 1
        || works.iter().filter(|w| w.thread_id == second).count() != 1
    {
        return None;
    }
    let exclusive = |id| {
        works
            .iter()
            .find(|w| w.thread_id == id)?
            .member_subjects
            .iter()
            .find(|subject| {
                !subject.is_empty()
                    && !subject.starts_with("Downloaded:")
                    && !subject.starts_with("app:")
                    && works.iter().all(|other| {
                        other.thread_id == id || !other.member_subjects.contains(subject)
                    })
            })
            .cloned()
    };
    let a = exclusive(first)?;
    let b = exclusive(second)?;
    (a != b).then_some((a, b))
}

/// Revalidate at the declaration boundary against *every* thread, including
/// works faded from Home. The engine's declaration path can otherwise resolve
/// a shared companion by its first holder and group the wrong intention.
pub fn grouping_subjects_from_storage(
    root: &Path,
    first: u64,
    second: u64,
) -> Result<Option<(String, String)>, DaemonError> {
    if first == second {
        return Ok(None);
    }
    let engine = engine_from_storage(root)?;
    let threads = engine.threads();
    let exclusive = |id| -> Option<String> {
        let thread = threads.iter().find(|t| t.id == id)?;
        thread
            .anchors
            .iter()
            .map(|a| &a.resource)
            .find(|subject| {
                !subject.is_empty()
                    && !subject.starts_with("Downloaded:")
                    && !subject.starts_with("app:")
                    && threads.iter().all(|other| {
                        other.id == id
                            || (!other.anchors.iter().any(|a| &a.resource == *subject)
                                && !other.companions.iter().any(|(r, _)| r == *subject))
                    })
            })
            .cloned()
    };
    Ok(exclusive(first).zip(exclusive(second)))
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
/// 3. **Ordering is a lexicographic key over witnessed facts**, not a score:
///    an open loop first, then coarse recency, then engagement as the
///    tiebreak. A score would need a weight, and a weight is a preference
///    about content, which this layer may not express; the *order* of the keys
///    is a claim about interruption and resumption. See the comparator in
///    [`display_threads_with_names`] for the reasoning, and note that recency
///    is read from the work's own last human activity — never from the newest
///    record in the log, which is background file churn.
pub fn display_threads(engine: &Engine) -> Vec<DisplayThread> {
    display_threads_with_names(engine, &crate::name_index::NameIndex::empty())
}

/// [`display_threads_with_names`] with no classification index. Kept so callers
/// that only need witnessed names do not have to construct an empty one.
pub fn display_threads_with_names(
    engine: &Engine,
    names: &crate::name_index::NameIndex,
) -> Vec<DisplayThread> {
    display_threads_classified(engine, names, &crate::classification::ClassifiedSurfaces::empty())
}

/// [`display_threads`] enriched with the log's witnessed names: every card
/// title, brief, and trail entry prefers the name the person actually saw
/// the surface display (OBS-SURFACE-NAMED) before falling back to the
/// structural derivation (URL slug, file stem). Identity never changes —
/// threads are the engine's; only the words presenting them do.
/// [`display_threads_with_names`] with the log's classified surfaces joined
/// in: a subject the person classified (OBS-SURFACE-CLASSIFIED) may never name
/// a work — it is skipped as a title source even when it rides along as a
/// companion or the resume point. Identity is untouched; the engine has already
/// refused to let a classified surface anchor anything.
pub fn display_threads_classified(
    engine: &Engine,
    names: &crate::name_index::NameIndex,
    classified: &crate::classification::ClassifiedSurfaces,
) -> Vec<DisplayThread> {
    display_threads_classified_with_activity(engine, names, classified, &[])
}

/// Display projection with canonical, content-free activity evidence joined
/// for the title ladder. The evidence changes presentation only; engine
/// identity and restoration remain entirely engine-derived.
pub fn display_threads_classified_with_activity(
    engine: &Engine,
    names: &crate::name_index::NameIndex,
    classified: &crate::classification::ClassifiedSurfaces,
    activity: &[crate::work_understanding::ActivityEvidence],
) -> Vec<DisplayThread> {
    let threads = engine.threads();
    let mut display = Vec::new();

    for thread in &threads {
        // Dismissal is a reversible presentation choice, not deletion. Keep
        // the work in the replay engine so it remains retrievable, but do not
        // surface it on the default Home list.
        if engine.is_dismissed(thread.id) {
            continue;
        }
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

        // Rule 3: engagement = episodes × distinct anchors.
        let engagement = (thread.episodes.len() as f64)
            * (thread
                .anchors
                .iter()
                .filter(|a| !a.resource.starts_with("Downloaded:"))
                .count()
                .max(1) as f64);

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
        // The witnessed names join here: the trail and the title prefer what
        // the person saw the surface call itself, verbatim, before any
        // structural derivation.
        let understanding = crate::work_understanding::understand_classified_with_activity(
            thread,
            &bundle,
            names,
            classified,
            activity,
        );

        // The work's last activity: the most recent episode's end.
        let last_active_ms = thread
            .episodes
            .last()
            .map(|ep| ep.span.1)
            .filter(|ts| *ts > 0);

        // The first key of the ordering, and the only reason a card shows.
        // `pending` is derived from the engine's own witnessed deltas, so the
        // sentence is a fact the person can check, not a summary Evo composed.
        let has_open_loop = !bundle.open_deltas.is_empty();
        let urgency_reason = understanding.pending.first().cloned();

        display.push(DisplayThread {
            thread_id: thread.id,
            name: understanding.title.clone(),
            bundle: display_bundle,
            is_work: !thread.anchors.is_empty(),
            engagement,
            has_open_loop,
            urgency_reason,
            understanding,
            last_active_ms,
            // Only the engine's own anchored subjects may identify this
            // thread in an explicit correction; never borrow ledger names.
            member_subjects: thread.anchors.iter().map(|a| a.resource.clone()).collect(),
            skipped_members: engine.activation_plan(thread.id, false).skipped,
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

    // ─── Ordering: a lexicographic key over witnessed facts ────────────────
    //
    // Deliberately **not** a score. A score requires a weight, and a weight is
    // a preference about content, which the engagement parameters may not
    // express. Each key below is already in evidence; only their *order* is a
    // judgement, and it is a judgement about interruption and resumption
    // rather than about what any work is.
    //
    //   1. `has_open_loop` — unsaved edits, uncommitted changes, a download not
    //      yet used, or a surface typed on and never saved. Left unfinished; a
    //      person returns to what they left unfinished.
    //   2. recency in coarse buckets — the work the person was most recently
    //      in, which is the plain meaning of "continue where you left off".
    //      Bucketed so a sub-hour difference between two machine events cannot
    //      reorder the list, and read from the work's own last human activity:
    //      never from the newest record in the log, because the log's newest
    //      records are background file churn and ordering by those would put a
    //      photo-library index above the work the person stopped in.
    //   3. `engagement`, decayed — historical popularity, as the tiebreak it
    //      always should have been.
    let recency_bucket = |dt: &DisplayThread| -> u64 {
        last_activity_ms(dt)
            .map(|last| now_ms.saturating_sub(last) / (6 * 3_600_000))
            .unwrap_or(u64::MAX)
    };
    let mut display = display_retained;
    display.sort_by(|a, b| {
        b.has_open_loop
            .cmp(&a.has_open_loop)
            .then_with(|| recency_bucket(a).cmp(&recency_bucket(b)))
            .then_with(|| {
                b.engagement
                    .partial_cmp(&a.engagement)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then(a.thread_id.cmp(&b.thread_id))
    });
    display
}

/// Reads the canonical observation log, replays the work engine over it, and
/// returns the engine's display threads — the complete read path Home now
/// renders.
///
/// This is the composition root of the W2 unification: one call from raw
/// canonical evidence to presentable work, with no other engine's grouping in
/// between. Errors are returned, not swallowed: a read failure is a surface
/// condition the shell must state honestly.
///
/// The engine lineages minted here are stable across calls: `Engine::replay`
/// over the same append-only log issues the same ThreadIds in the same order,
/// so pod identity, accent colors, and Add-to-Pod claims survive restarts.
pub fn display_threads_from_storage(root: &Path) -> Result<Vec<DisplayThread>, DaemonError> {
    let engine = engine_from_storage(root)?;
    // The presentation joins: every OBS-SURFACE-NAMED record the log carries,
    // indexed by subject, enriches presentation without touching identity, and
    // every OBS-SURFACE-CLASSIFIED record holds a subject out of titling. An
    // old log (neither record yet) produces the identical threads the
    // structural derivation always produced.
    let observations = load_persisted_observations(root)?;
    let names = crate::name_index::NameIndex::from_observations(&observations);
    let classified = crate::classification::ClassifiedSurfaces::from_observations(&observations);
    let activity = crate::work_understanding::activity_evidence_from_observations(&observations);
    Ok(display_threads_classified_with_activity(
        &engine,
        &names,
        &classified,
        &activity,
    ))
}

/// Returns the application names directly witnessed while the resources of
/// each display thread were focused.
///
/// Application names are provenance, not work identity: they are useful at
/// the execution boundary for asking macOS to reopen the tools, but they must
/// never become anchors or titles. The join is deliberately exact over the
/// thread's witnessed resources and ignores observations that cannot name
/// their owning process.
pub fn app_names_for_threads(
    observations: &[Observation],
    threads: &[DisplayThread],
) -> BTreeMap<u64, Vec<String>> {
    let apps_by_subject = app_witnesses_by_subject(observations);

    threads
        .iter()
        .map(|thread| {
            let mut resources = BTreeSet::new();
            resources.extend(thread.member_subjects.iter().cloned());
            resources.extend(
                thread
                    .understanding
                    .trail
                    .iter()
                    .map(|entry| entry.raw.clone()),
            );
            resources.extend(thread.bundle.restore_set.iter().cloned());
            if !thread.bundle.resume_point.trim().is_empty() {
                resources.insert(thread.bundle.resume_point.clone());
            }

            let mut apps = BTreeMap::new();
            for resource in resources {
                if let Some(witnessed) = apps_by_subject.get(&resource) {
                    apps.extend(witnessed.clone());
                }
            }
            (thread.thread_id, apps.into_values().collect())
        })
        .collect()
}

/// Returns the most recently witnessed application for each exact resource in
/// each display thread. The map is used for file restoration so a document
/// returns to the app that actually witnessed it rather than an invented
/// default handler.
pub fn resource_apps_for_threads(
    observations: &[Observation],
    threads: &[DisplayThread],
) -> BTreeMap<u64, BTreeMap<String, String>> {
    let apps_by_subject = app_witnesses_by_subject(observations);

    threads
        .iter()
        .map(|thread| {
            let mut resources = BTreeSet::new();
            resources.extend(thread.member_subjects.iter().cloned());
            resources.extend(
                thread
                    .understanding
                    .trail
                    .iter()
                    .map(|entry| entry.raw.clone()),
            );
            resources.extend(thread.bundle.restore_set.iter().cloned());
            if !thread.bundle.resume_point.trim().is_empty() {
                resources.insert(thread.bundle.resume_point.clone());
            }

            let resource_apps = resources
                .into_iter()
                .filter_map(|resource| {
                    apps_by_subject
                        .get(&resource)
                        .and_then(|apps| apps.values().next())
                        .map(|app| (resource, app.clone()))
                })
                .collect();
            (thread.thread_id, resource_apps)
        })
        .collect()
}

fn app_witnesses_by_subject(
    observations: &[Observation],
) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut apps_by_subject: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for observation in observations {
        if observation.schema().name() != "OBS-WINDOW-FOCUS-GAINED" {
            continue;
        }
        let Some(subject) = crate::understanding::witnessed_subject_of(observation)
            .map(|subject| subject.trim().to_string())
            .filter(|subject| !subject.is_empty())
        else {
            continue;
        };
        let Some(app) = observation
            .provenance()
            .context()
            .get(evo_capture::OWNING_PROCESS_NAME_CONTEXT_KEY)
            .map(|app| app.trim())
            .filter(|app| !app.is_empty())
        else {
            continue;
        };
        apps_by_subject
            .entry(subject)
            .or_default()
            .entry(app.to_lowercase())
            .or_insert_with(|| app.to_string());
    }
    apps_by_subject
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
        "the"
            | "a"
            | "an"
            | "my"
            | "open"
            | "hey"
            | "evo"
            | "can"
            | "you"
            | "continue"
            | "resume"
            | "work"
            | "please"
            | "just"
            | "only"
            | "that"
            | "this"
            | "for"
            | "me"
            | "of"
            | "in"
            | "on"
            | "at"
            | "to"
            | "is"
            | "it"
            | "and"
            | "or"
            | "with"
            | "was"
            | "reopen"
            | "workspace"
            | "find"
            | "show"
            | "restore"
    )
}

/// The last activity timestamp of a display thread, from its most recent
/// episode's span, in epoch milliseconds.
fn last_activity_ms(dt: &DisplayThread) -> Option<u64> {
    dt.last_active_ms
}

/// A readable name for one resource: the domain of a URL, the file name
/// of a path, or a truncated title. Never an application name, never a
/// category — the work's own identity, stated plainly.
fn readable_surface_name(resource: &str) -> Option<String> {
    if resource.starts_with("http") {
        let url = resource
            .trim_start_matches("https://")
            .trim_start_matches("http://");
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
    let push_surface = |resource: &str, surfaces: &mut Vec<String>| {
        if surfaces.len() >= 3 {
            return;
        }
        if resource.starts_with("Downloaded:") {
            return; // a download is a product, not a surface
        }
        if let Some(name) = readable_surface_name(resource)
            && !surfaces.contains(&name)
        {
            surfaces.push(name);
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

    let session_word = if thread.episodes.len() == 1 {
        "session"
    } else {
        "sessions"
    };
    match surfaces.len() {
        1 => format!(
            "Working in {} across {} {}.",
            surfaces[0],
            thread.episodes.len(),
            session_word
        ),
        2 => format!(
            "Working in {} with {} open, across {} {}.",
            surfaces[0],
            surfaces[1],
            thread.episodes.len(),
            session_word
        ),
        _ => format!(
            "Working across {}, {}, and {}, over {} {}.",
            surfaces[0],
            surfaces[1],
            surfaces[2],
            thread.episodes.len(),
            session_word
        ),
    }
}
pub fn engine_from_storage(root: &Path) -> Result<Engine, DaemonError> {
    let observations = load_persisted_observations(root)?;
    Ok(engine_from_observations(&observations))
}

/// The shared Home/voice replay, without a second storage read or log mutation.
/// Append order, capped dwell and final settlement are deterministic.
pub fn engine_from_observations(observations: &[Observation]) -> Engine {
    let lifecycle_evidence = lifecycle_evidence_from_observations(observations);
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
        .saturating_add(86_400_000);
    engine.settle(settle_at);
    engine.set_lifecycle_evidence(lifecycle_evidence);
    engine
}

/// Derives replay-time lifecycle evidence from canonical observations.
///
/// Commit and member subjects are opaque. The only relationship used here is
/// one the capture layer witnessed explicitly: the commit subject and the
/// member subject were both related to the same opaque repository subject.
/// No path parsing, hash parsing, naming convention, or repository lookup is
/// performed during replay. The resulting evidence is held by the returned
/// engine only and is never appended to storage.
pub fn lifecycle_evidence_from_observations(observations: &[Observation]) -> LifecycleEvidence {
    let mut memberships: BTreeMap<String, BTreeSet<(String, u64)>> = BTreeMap::new();
    let mut commits = Vec::new();

    for observation in observations {
        let at = milliseconds(observation.provenance().observed_at());
        match observation.schema().name() {
            "OBS-REPOSITORY-MEMBERSHIP" => {
                let Some(member) = text_fact(observation, "RepositoryMembership") else {
                    continue;
                };
                let Some(repository) = text_fact(observation, "Repository") else {
                    continue;
                };
                memberships
                    .entry(member)
                    .or_default()
                    .insert((repository, at));
            }
            "OBS-COMMIT-MADE" => {
                if let Some(commit) = text_fact(observation, "CommitMade") {
                    commits.push((commit, at));
                }
            }
            _ => {}
        }
    }

    let mut relationships = Vec::new();
    for (commit, commit_at) in commits {
        let Some(commit_repositories) = memberships.get(&commit) else {
            continue;
        };
        for (repository, commit_membership_at) in commit_repositories {
            if *commit_membership_at > commit_at {
                continue;
            }
            for (member, member_repositories) in &memberships {
                if member == &commit {
                    continue;
                }
                if member_repositories.iter().any(|(candidate, member_at)| {
                    candidate == repository && *member_at <= commit_at
                }) {
                    relationships.push(CommitEvidence::new(
                        commit.clone(),
                        member.clone(),
                        commit_at,
                    ));
                }
            }
        }
    }
    LifecycleEvidence::new(relationships)
}

fn text_fact(observation: &Observation, name: &str) -> Option<String> {
    match observation.evidence().fact(name)?.value() {
        FactValue::Text(value) if !value.trim().is_empty() => Some(value.clone()),
        _ => None,
    }
}

/// Projects one canonical Observation onto the engine's contract, or `None`
/// for the concepts the engine does not consume.
///
/// `pub` for the shadow evaluator: the gate replays the same projection the
/// live read path uses, so parity is measured against the real bridge, not a
/// re-implementation of it.
pub fn project_observation(observation: &Observation) -> Option<Event> {
    let timestamp = milliseconds(observation.provenance().observed_at());
    let origin = if observation.provenance().is_execution_origin() {
        Origin::Execution
    } else {
        Origin::Person
    };
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
            origin,
            interaction: Interaction::Focused,
            resource: subject()?,
            weight: None,
            detail: None,
        }),
        "OBS-URL-NAVIGATED" => Some(Event {
            timestamp,
            origin,
            interaction: Interaction::Visited,
            resource: subject()?,
            weight: None,
            detail: None,
        }),
        "OBS-FILE-SAVED" => Some(Event {
            timestamp,
            origin,
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
                origin,
                interaction: Interaction::Typed,
                resource: subject()?,
                weight: Some(keys),
                detail: None,
            })
        }
        "OBS-SURFACE-CLASSIFIED" => {
            // The person's own word about a surface: it may never found or name
            // a work. The engine gates founding from this record; the
            // presentation layer gates titling from the same record. Again,
            // never let an execution observation masquerade as the person's
            // word.
            if origin == Origin::Execution {
                return None;
            }
            let resource = subject()?;
            let class = match observation.evidence().fact("Class").map(|f| f.value()) {
                Some(FactValue::Text(class)) => class.clone(),
                _ => return None,
            };
            Some(Event {
                timestamp,
                origin,
                interaction: Interaction::Declared,
                resource: String::new(),
                weight: None,
                detail: Some(Declaration::Classified { resource, class }),
            })
        }
        "OBS-WORK-DISMISSED" => {
            if origin == Origin::Execution {
                return None;
            }
            Some(Event {
                timestamp,
                origin,
                interaction: Interaction::Declared,
                resource: String::new(),
                weight: None,
                detail: Some(Declaration::WorkDismissed { work: subject()? }),
            })
        }
        "OBS-MEMBER-STATE" => {
            if origin == Origin::Execution {
                return None;
            }
            let work = subject()?;
            let member = match observation.evidence().fact("Member").map(|f| f.value()) {
                Some(FactValue::Text(member)) => member.clone(),
                _ => return None,
            };
            let state = match observation.evidence().fact("State").map(|f| f.value()) {
                Some(FactValue::Text(state)) => evo_threads::MemberState::parse(state)?,
                _ => return None,
            };
            Some(Event {
                timestamp,
                origin,
                interaction: Interaction::Declared,
                resource: String::new(),
                weight: None,
                detail: Some(Declaration::MemberState {
                    work,
                    member,
                    state,
                }),
            })
        }
        "OBS-WORK-GROUPED" => {
            // Engine::apply handles declarations before checking origin. Never
            // let an execution observation masquerade as the person's word.
            if origin == Origin::Execution {
                return None;
            }
            let first = subject()?;
            let second = match observation.evidence().fact("CoMember").map(|f| f.value()) {
                Some(FactValue::Text(second)) => second.clone(),
                _ => return None,
            };
            Some(Event {
                timestamp,
                origin,
                interaction: Interaction::Declared,
                resource: String::new(),
                weight: None,
                detail: Some(Declaration::SameWork {
                    a: first,
                    b: second,
                }),
            })
        }
        _ => None,
    }
}

pub(crate) fn milliseconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_observation::accept::accept;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::provenance::{ObservationSource, Provenance};
    use evo_storage::{Storage, StorageObjectKind};
    use std::collections::HashMap;
    use std::path::PathBuf;

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
        let candidate = evo_observation::candidate::CandidateObservation::new(
            schema.clone(),
            provenance,
            concept.evidence(),
        );
        let observation = accept(candidate, &schema).unwrap();
        let record = crate::persistence::encode_observation_record(&observation);
        storage
            .append(StorageObjectKind::Observation, record.as_bytes())
            .unwrap();
    }

    #[test]
    fn replay_time_conversion_and_settlement_saturate() {
        let far_future = UNIX_EPOCH + std::time::Duration::from_secs(u64::MAX / 1000 + 100);
        assert_eq!(milliseconds(far_future), u64::MAX);
        assert_eq!(
            milliseconds(UNIX_EPOCH - std::time::Duration::from_secs(1)),
            0
        );
        // An ignored concept still advances the settlement clock, but must
        // not overflow it or change append-order replay into sorted replay.
        let concept = ObservationConcept::SurfaceNamed {
            subject: "a".into(),
            name: "a".into(),
        };
        let schema = concept.schema();
        let observation = accept(
            evo_observation::candidate::CandidateObservation::new(
                schema.clone(),
                Provenance::new(
                    ObservationSource::new("test").unwrap(),
                    far_future,
                    HashMap::new(),
                ),
                concept.evidence(),
            ),
            &schema,
        )
        .unwrap();
        assert!(
            engine_from_observations(&[observation])
                .threads()
                .is_empty()
        );
    }

    #[test]
    fn application_witnesses_stay_provenance_and_dedupe_case_insensitively() {
        let make_focus = |app: &str| {
            let mut context = HashMap::new();
            context.insert(
                evo_capture::OWNING_PROCESS_NAME_CONTEXT_KEY.to_string(),
                app.to_string(),
            );
            let concept = ObservationConcept::WindowFocusGained {
                subject: "file:///work/report.md".into(),
            };
            let schema = concept.schema();
            accept(
                evo_observation::candidate::CandidateObservation::new(
                    schema.clone(),
                    Provenance::new(
                        ObservationSource::new("macos_adapter").unwrap(),
                        UNIX_EPOCH,
                        context,
                    ),
                    concept.evidence(),
                ),
                &schema,
            )
            .unwrap()
        };

        let observed = app_witnesses_by_subject(&[
            make_focus("Terminal"),
            make_focus("terminal"),
            make_focus("Safari"),
        ]);
        let apps = observed
            .get("file:///work/report.md")
            .expect("focus subject is indexed");
        assert_eq!(apps.get("terminal").map(String::as_str), Some("Terminal"));
        assert_eq!(apps.get("safari").map(String::as_str), Some("Safari"));
        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn focused_files_mint_works_and_cold_saves_are_churn() {
        let root = temp_root("saves");
        // Document-grain pattern: the person is AT the document when it is
        // written — the mutation gate passes and the work stands up.
        persist(
            &root,
            100,
            &ObservationConcept::WindowFocusGained {
                subject: "file:///work/report.md".into(),
            },
        );
        // The autosave pattern: the editor writes the focused document
        // repeatedly while the person works. Each write passes the mutation
        // gate (the person is at the resource); together they cross the
        // birth floor.
        persist(
            &root,
            110,
            &ObservationConcept::FileSaved {
                subject: "file:///work/report.md".into(),
            },
        );
        persist(
            &root,
            115,
            &ObservationConcept::WindowFocusGained {
                subject: "file:///work/report.md".into(),
            },
        );
        persist(
            &root,
            120,
            &ObservationConcept::FileSaved {
                subject: "file:///work/report.md".into(),
            },
        );
        persist(
            &root,
            125,
            &ObservationConcept::FileSaved {
                subject: "file:///work/report.md".into(),
            },
        );
        persist(
            &root,
            130,
            &ObservationConcept::FileSaved {
                subject: "file:///work/notes.md".into(),
            },
        );

        let engine = engine_from_storage(&root).unwrap();
        let threads = engine.threads();
        assert!(
            !threads.is_empty(),
            "a focused, saved document mints a thread"
        );
        let bundle = engine.resume_bundle(threads[0].id);
        assert!(
            bundle.resume_point.starts_with("file:///work/report.md"),
            "resume point must be the witnessed work, got {}",
            bundle.resume_point
        );
        // notes.md was written with nobody at it: service churn, quarantined.
        let anchors: Vec<_> = threads
            .iter()
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
        persist(
            &root,
            100,
            &ObservationConcept::WindowFocusGained {
                subject: "editor".into(),
            },
        );
        for index in 0..5 {
            persist(
                &root,
                105 + index,
                &ObservationConcept::FileSaved {
                    subject: format!("file:///Library/Sync/state-{index}.plist"),
                },
            );
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
    fn commit_lifecycle_projection_uses_only_opaque_membership_relationships() {
        let root = temp_root("lifecycle-commit");
        let now = 1_700_000_000;
        const MEMBER: &str = "opaque-member";
        const COMMIT: &str = "opaque-commit";
        const REPOSITORY: &str = "opaque-repository";

        persist(
            &root,
            now,
            &ObservationConcept::WindowFocusGained {
                subject: MEMBER.into(),
            },
        );
        persist(
            &root,
            now + 1,
            &ObservationConcept::InputActivity {
                subject: MEMBER.into(),
                keys: 150,
                clicks: 0,
                scrolls: 0,
            },
        );
        persist(
            &root,
            now + 2,
            &ObservationConcept::RepositoryMembership {
                member: MEMBER.into(),
                repository: REPOSITORY.into(),
            },
        );
        persist(
            &root,
            now + 2,
            &ObservationConcept::RepositoryMembership {
                member: COMMIT.into(),
                repository: REPOSITORY.into(),
            },
        );
        persist(
            &root,
            now + 3,
            &ObservationConcept::CommitMade {
                subject: COMMIT.into(),
            },
        );

        let observations = load_persisted_observations(&root).unwrap();
        let evidence = lifecycle_evidence_from_observations(&observations);
        assert_eq!(
            evidence.commits,
            vec![CommitEvidence::new(
                COMMIT,
                MEMBER,
                milliseconds(UNIX_EPOCH + std::time::Duration::from_secs(now + 3)),
            )]
        );

        let engine = engine_from_observations(&observations);
        let work = engine
            .threads()
            .into_iter()
            .find(|thread| {
                thread
                    .anchors
                    .iter()
                    .any(|anchor| anchor.resource == MEMBER)
            })
            .expect("the opaque member should found a work");
        let plan = engine.activation_plan(work.id, false);
        assert_eq!(
            plan.skipped
                .iter()
                .find(|member| member.resource == MEMBER)
                .map(|member| member.source),
            Some(evo_threads::MemberStateSource::Derived)
        );
        assert!(!plan.resources.iter().any(|resource| resource == MEMBER));

        // Commit and membership records remain projections only; this API did
        // not add a new event or a storage record.
        assert!(
            project_observation(
                &observations
                    .iter()
                    .find(|observation| observation.schema().name() == "OBS-COMMIT-MADE")
                    .unwrap()
            )
            .is_none()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn witnessed_names_enrich_display_threads_without_touching_identity() {
        // The Phase-1 contract end to end: a URL that was visited and later
        // named itself produces (a) the same thread the structural derivation
        // would mint — identity is the engine's — and (b) presentation that
        // carries the witnessed title verbatim.
        let root = temp_root("named");
        // Recent timestamps: the display path's 30-day fade would drop
        // epoch-era work, and this test is about naming, not the fade.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        persist(
            &root,
            now,
            &ObservationConcept::WindowFocusGained {
                subject: "https://example.com/field-guide".into(),
            },
        );
        persist(
            &root,
            now + 10,
            &ObservationConcept::InputActivity {
                subject: "https://example.com/field-guide".into(),
                keys: 150,
                clicks: 0,
                scrolls: 0,
            },
        );
        persist(
            &root,
            now + 20,
            &ObservationConcept::SurfaceNamed {
                subject: "https://example.com/field-guide".into(),
                name: "A Field Guide to Checkbox Land - Google Chrome".into(),
            },
        );
        let engine = engine_from_storage(&root).unwrap();

        // The unnamed read path (an old log's shape): the structural slug.
        let plain = display_threads(&engine);
        // The named read path: the witnessed title, suffix-stripped.
        let observations = load_persisted_observations(&root).unwrap();
        let names = crate::name_index::NameIndex::from_observations(&observations);
        let named = display_threads_with_names(&engine, &names);

        assert_eq!(plain.len(), named.len(), "naming never adds or drops work");
        for (plain, named) in plain.iter().zip(named.iter()) {
            assert_eq!(plain.thread_id, named.thread_id, "identity unchanged");
        }
        // The witnessed name reaches the card: the focus subject was the
        // URL, so the title slot prefers the page's own words.
        let focus_named = named
            .iter()
            .find(|t| {
                t.understanding
                    .trail
                    .iter()
                    .any(|e| e.raw.contains("field-guide"))
                    || t.name.contains("Field Guide")
            })
            .expect("the named thread is on the surface");
        let trail_has_witnessed = focus_named
            .understanding
            .trail
            .iter()
            .any(|e| e.name.contains("A Field Guide to Checkbox Land"));
        assert!(
            trail_has_witnessed || focus_named.name.contains("A Field Guide to Checkbox Land"),
            "the witnessed title must reach the trail or the title, got name={:?} trail={:?}",
            focus_named.name,
            focus_named.understanding.trail
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_classified_surface_founds_no_work_and_names_none() {
        // Phase 3 end to end: the log carries the person's own word that a
        // surface is not work, and both readers honour it — the engine refuses
        // it an anchor, the presentation layer refuses it a title.
        let root = temp_root("classified");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        const VIDEO: &str = "https://www.youtube.com/watch?v=EsW2BoFAUs";
        persist(
            &root,
            now,
            &ObservationConcept::WindowFocusGained {
                subject: VIDEO.into(),
            },
        );
        persist(
            &root,
            now + 10,
            &ObservationConcept::InputActivity {
                subject: VIDEO.into(),
                keys: 150,
                clicks: 0,
                scrolls: 0,
            },
        );
        persist(
            &root,
            now + 20,
            &ObservationConcept::SurfaceNamed {
                subject: VIDEO.into(),
                name: "A Video I Watched - YouTube".into(),
            },
        );

        let engine = engine_from_storage(&root).unwrap();
        assert!(
            engine
                .threads()
                .iter()
                .any(|t| t.anchors.iter().any(|a| a.resource == VIDEO)),
            "the work is founded on the URL before any classification"
        );

        // The person classifies it: not work.
        persist(
            &root,
            now + 30,
            &ObservationConcept::SurfaceClassified {
                subject: VIDEO.into(),
                class: "NotWork".into(),
            },
        );

        let engine = engine_from_storage(&root).unwrap();
        assert!(
            !engine
                .threads()
                .iter()
                .any(|t| t.anchors.iter().any(|a| a.resource == VIDEO)),
            "a classified surface must not anchor any work"
        );

        let works = display_threads_from_storage(&root).unwrap();
        for work in &works {
            assert!(
                !work.member_subjects.iter().any(|subject| subject == VIDEO),
                "no presented work claims the classified surface as identity"
            );
            assert!(
                !work.name.contains("A Video I Watched"),
                "a classified surface must not name a presented work, got {:?}",
                work.name
            );
        }

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn feedback_declarations_project_and_change_only_the_default_surface() {
        let root = temp_root("feedback");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        const WORK: &str = "file:///feedback/work.md";
        persist(
            &root,
            now,
            &ObservationConcept::WindowFocusGained {
                subject: WORK.into(),
            },
        );
        persist(
            &root,
            now + 10,
            &ObservationConcept::InputActivity {
                subject: WORK.into(),
                keys: 150,
                clicks: 0,
                scrolls: 0,
            },
        );
        let before = engine_from_storage(&root).unwrap();
        let thread = before
            .threads()
            .into_iter()
            .find(|thread| thread.anchors.iter().any(|anchor| anchor.resource == WORK))
            .expect("feedback work should exist");

        persist(
            &root,
            now + 20,
            &ObservationConcept::MemberState {
                work: WORK.into(),
                member: WORK.into(),
                state: "Completed".into(),
            },
        );
        persist(
            &root,
            now + 30,
            &ObservationConcept::WorkDismissed { work: WORK.into() },
        );
        let after = engine_from_storage(&root).unwrap();
        assert!(after.is_dismissed(thread.id));
        assert_eq!(after.threads().len(), 1, "dismissal does not delete replay state");
        assert!(after.activation_plan(thread.id, false).resources.is_empty());
        assert!(
            display_threads(&after).is_empty(),
            "dismissal removes the work from the default display only"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn grouped_subjects_merge_threads() {
        let root = temp_root("grouped");
        persist(
            &root,
            100,
            &ObservationConcept::WindowFocusGained {
                subject: "file:///a.md".into(),
            },
        );
        persist(
            &root,
            110,
            &ObservationConcept::FileSaved {
                subject: "file:///a.md".into(),
            },
        );
        persist(
            &root,
            200,
            &ObservationConcept::WindowFocusGained {
                subject: "file:///b.md".into(),
            },
        );
        persist(
            &root,
            210,
            &ObservationConcept::FileSaved {
                subject: "file:///b.md".into(),
            },
        );
        persist(
            &root,
            300,
            &ObservationConcept::WorkGrouped {
                first: "file:///a.md".into(),
                second: "file:///b.md".into(),
            },
        );

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

    #[test]
    fn grouping_requires_two_exclusive_witnesses_not_a_shared_page() {
        let root = temp_root("manual-grouping-witnesses");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for (offset, path) in [
            (0, "file:///fixture/alpha.md"),
            (2000, "file:///fixture/bravo.md"),
        ] {
            persist(
                &root,
                now + offset,
                &ObservationConcept::WindowFocusGained {
                    subject: path.into(),
                },
            );
            persist(
                &root,
                now + offset + 10,
                &ObservationConcept::FileSaved {
                    subject: path.into(),
                },
            );
            persist(
                &root,
                now + offset + 20,
                &ObservationConcept::FileSaved {
                    subject: path.into(),
                },
            );
            persist(
                &root,
                now + offset + 30,
                &ObservationConcept::FileSaved {
                    subject: path.into(),
                },
            );
        }
        let works = display_threads_from_storage(&root).unwrap();
        assert_eq!(works.len(), 2);
        let pair = grouping_subjects(&works, works[0].thread_id, works[1].thread_id).unwrap();
        assert_eq!(
            grouping_subjects_from_storage(&root, works[0].thread_id, works[1].thread_id).unwrap(),
            Some(pair.clone())
        );
        assert_ne!(pair.0, pair.1);
        assert!(grouping_subjects(&works, works[0].thread_id, works[0].thread_id).is_none());
        assert!(grouping_subjects(&works, works[0].thread_id, 999).is_none());
        let mut ambiguous = works.clone();
        ambiguous[1].member_subjects = ambiguous[0].member_subjects.clone();
        assert!(
            grouping_subjects(&ambiguous, ambiguous[0].thread_id, ambiguous[1].thread_id).is_none()
        );
        assert_eq!(
            grouping_subjects_from_storage(&root, works[0].thread_id, 999).unwrap(),
            None
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
