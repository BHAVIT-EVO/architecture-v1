//! Witnessed names: the log's OBS-SURFACE-NAMED records, indexed by subject.
//!
//! A surface naming itself is presentation evidence (OBS-SURFACE-NAMED/v1):
//! the page title a browser displayed, the document title an editor
//! displayed, the window title any surface displayed. The engine's thread
//! identity never consults it — names decorate and never vote — but the
//! *presentation* of a work should use exactly the words the person saw on
//! screen. This index is the read-side join between the two: subject → the
//! names the person actually saw it carry, ranked by how often and how
//! recently the surface said so.
//!
//! The index never re-derives or normalizes what a name means. It does one
//! structural cleanup before presentation: a browser appends its own
//! application name to page titles ("Page — Google Chrome"), and the set of
//! application names a machine witnesses is available in the log itself
//! (focus records carry `owning_process_name` in their provenance context).
//! A trailing title segment that matches a *witnessed* application name is
//! the browser's suffix, not the page's words — stripping it is removing one
//! machine's framing, not editing the person's title. No application list is
//! hardcoded; the set is derived from the same machine's own witnesses.

use evo_observation::evidence::FactValue;
use evo_observation::observation::Observation;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// How many witnessed names are kept per subject: the surface's own recent
/// vocabulary, not its full publishing history. Eviction drops the
/// least-counted, then the oldest — a name the surface stopped presenting
/// fades from presentation exactly as it faded from the screen.
const NAMES_PER_SUBJECT: usize = 4;

/// Presentation cap for a witnessed name. The stored name is never
/// truncated; what surfaces on a card is.
const PRESENT_NAME_MAX: usize = 80;

/// One witnessed name with its witness count and most recent sighting.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WitnessedName {
    name: String,
    count: u32,
    last_seen_ms: u64,
}

/// The witnessed-name index: subject → names the person saw it carry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NameIndex {
    names: HashMap<String, Vec<WitnessedName>>,
    /// Application names witnessed on this machine (from focus records'
    /// `owning_process_name` context) — the candidate browser suffixes.
    witnessed_app_names: BTreeSet<String>,
    /// The person's own names (OBS-WORK-NAMED, RFC-0014): the word the
    /// person chose for a subject's body of work. Presentation evidence of
    /// the strongest kind — what the person *said*, not what a surface
    /// displayed — so it outranks every witnessed surface name.
    user_names: BTreeMap<String, String>,
}

impl NameIndex {
    /// An index with nothing witnessed in it. Presentation falls back to
    /// the structural names it would have used without this module.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builds the index over a slice of canonical Observations. Every
    /// OBS-SURFACE-NAMED record contributes a witnessed name; every focus
    /// record's `owning_process_name` contributes a witnessed application
    /// name (the candidate title suffixes).
    pub fn from_observations(observations: &[Observation]) -> Self {
        let mut index = Self::empty();
        for observation in observations {
            let schema = observation.schema();
            if schema.name() == "OBS-WORK-NAMED" {
                let subject =
                    observation
                        .evidence()
                        .fact("WorkNamed")
                        .and_then(|f| match f.value() {
                            FactValue::Text(subject) => Some(subject.clone()),
                            _ => None,
                        });
                let name = observation
                    .evidence()
                    .fact("Name")
                    .and_then(|f| match f.value() {
                        FactValue::Text(name) => Some(name.clone()),
                        _ => None,
                    });
                if let (Some(subject), Some(name)) = (subject, name) {
                    let subject = subject.trim().to_string();
                    if !subject.is_empty() && !name.trim().is_empty() {
                        index.user_names.insert(subject, name.trim().to_string());
                    }
                }
            } else if schema.name() == "OBS-SURFACE-NAMED" {
                let at_ms = crate::threads::milliseconds(observation.provenance().observed_at());
                let subject =
                    observation
                        .evidence()
                        .fact("SurfaceNamed")
                        .and_then(|f| match f.value() {
                            FactValue::Text(subject) => Some(subject.clone()),
                            _ => None,
                        });
                let name = observation
                    .evidence()
                    .fact("Name")
                    .and_then(|f| match f.value() {
                        FactValue::Text(name) => Some(name.clone()),
                        _ => None,
                    });
                if let (Some(subject), Some(name)) = (subject, name) {
                    index.record(&subject, &name, at_ms);
                }
            } else if schema.name() == "OBS-WINDOW-FOCUS-GAINED" {
                if let Some(app) = observation
                    .provenance()
                    .context()
                    .get(evo_capture::OWNING_PROCESS_NAME_CONTEXT_KEY)
                    .filter(|app| !app.trim().is_empty())
                {
                    index.witnessed_app_names.insert(app.trim().to_string());
                }
            }
        }
        index
    }

    /// Records one witnessed naming: the surface `subject` presented `name`
    /// at `at_ms`. Repeat sightings accumulate; the surface's strongest
    /// recent self-presentation wins the read.
    pub fn record(&mut self, subject: &str, name: &str, at_ms: u64) {
        let subject = subject.trim();
        let name = name.trim();
        if subject.is_empty() || name.is_empty() {
            return;
        }
        let entry = self.names.entry(subject.to_string()).or_default();
        if let Some(existing) = entry.iter_mut().find(|w| w.name == name) {
            existing.count += 1;
            existing.last_seen_ms = existing.last_seen_ms.max(at_ms);
            return;
        }
        entry.push(WitnessedName {
            name: name.to_string(),
            count: 1,
            last_seen_ms: at_ms,
        });
        if entry.len() > NAMES_PER_SUBJECT {
            // Evict the least-counted, then the oldest — one deterministic
            // rule, no guessing about which name "matters".
            let victim = entry
                .iter()
                .enumerate()
                .min_by_key(|(_, w)| (w.count, w.last_seen_ms))
                .map(|(i, _)| i)
                .expect("entry is non-empty");
            entry.remove(victim);
        }
    }

    /// The surface's strongest witnessed name, verbatim — what the person
    /// most often and most recently saw it called. The person's own declared
    /// name (OBS-WORK-NAMED) outranks every witnessed surface name. `None`
    /// when neither was witnessed for this subject.
    pub fn name_of(&self, subject: &str) -> Option<&str> {
        if let Some(user) = self.user_names.get(subject.trim()) {
            return Some(user.as_str());
        }
        let names = self.names.get(subject.trim())?;
        names
            .iter()
            .max_by_key(|w| (w.count, w.last_seen_ms))
            .map(|w| w.name.as_str())
    }

    /// Whether anything at all is witnessed.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.user_names.is_empty()
    }

    /// The presentation name for a surface: the person's own name verbatim
    /// when they declared one (their words are never re-edited), otherwise
    /// the witnessed name with browser suffixes removed when one is present.
    /// This is what a card, a pod, or a trail entry shows the person. `None`
    /// when the surface has no witnessed name — the caller falls back to its
    /// structural name.
    pub fn present_name_of(&self, subject: &str) -> Option<String> {
        if let Some(user) = self.user_names.get(subject.trim()) {
            return Some(present_truncate(user));
        }
        let name = self.name_of(subject)?;
        Some(self.clean_witnessed_name(name))
    }

    /// Enriches one engine-facing surface (a URL, a file locator, or a
    /// window subject) into a human-recognizable presentation name:
    /// witnessed name when the surface named itself, `None` otherwise (the
    /// caller's structural fallback — slug, file stem — still applies).
    pub fn enrich_surface(&self, surface: &str) -> Option<String> {
        self.present_name_of(surface)
    }

    /// Strips browser application suffixes from a witnessed title,
    /// structurally: a trailing segment — after " - ", " – ", or " — " —
    /// that matches an application name this machine itself witnessed is
    /// the browser's framing, not the page's words. Repeated so
    /// "file — evo — Visual Studio Code" sheds both machine segments.
    /// A title with no witnessed-app suffix survives verbatim, separators
    /// and all — the page's own words are never rewritten.
    fn clean_witnessed_name(&self, name: &str) -> String {
        let mut pieces = split_title(name);
        while pieces.len() > 1 {
            let last = pieces.last().map(|(_, text)| text.trim()).unwrap_or("");
            let is_app = self
                .witnessed_app_names
                .iter()
                .any(|app| app.eq_ignore_ascii_case(last));
            if is_app {
                pieces.pop();
            } else {
                break;
            }
        }
        let joined: String = pieces
            .iter()
            .map(|(sep, text)| format!("{sep}{text}"))
            .collect();
        present_truncate(joined.trim())
    }
}

/// Splits a title into (separator-before, text) pieces on the three
/// witnessed title separators, remembering which separator joined each
/// segment so survivors rejoin exactly as the title wrote them.
fn split_title(name: &str) -> Vec<(String, String)> {
    let separators = [" - ", " \u{2014} ", " \u{2013} "];
    let mut pieces: Vec<(String, String)> = Vec::new();
    let mut pending_sep = "";
    let mut rest = name;
    loop {
        let mut earliest: Option<(usize, &'static str)> = None;
        for sep in separators {
            if let Some(pos) = rest.find(sep) {
                let earlier = earliest
                    .map(|(bp, _): (usize, &str)| pos < bp)
                    .unwrap_or(true);
                if earlier {
                    earliest = Some((pos, sep));
                }
            }
        }
        match earliest {
            None => {
                pieces.push((pending_sep.to_string(), rest.to_string()));
                break;
            }
            Some((pos, sep)) => {
                pieces.push((pending_sep.to_string(), rest[..pos].to_string()));
                pending_sep = sep;
                rest = &rest[pos + sep.len()..];
            }
        }
    }
    pieces
}

/// Presentation cap: a card title that never ends. The stored witness keeps
/// its full text; this only bounds what is shown.
fn present_truncate(name: &str) -> String {
    if name.chars().count() <= PRESENT_NAME_MAX {
        return name.to_string();
    }
    let mut cut: String = name.chars().take(PRESENT_NAME_MAX).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::encode_observation_record;
    use evo_observation::accept::accept;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::observation_schema::ObservationSchema;
    use evo_observation::provenance::{ObservationSource, Provenance};
    use std::collections::HashMap;
    use std::time::{Duration, UNIX_EPOCH};

    fn observation(
        concept: &ObservationConcept,
        at_secs: u64,
        context: HashMap<String, String>,
    ) -> Observation {
        let schema = concept.schema();
        let provenance = Provenance::new(
            ObservationSource::new("test").unwrap(),
            UNIX_EPOCH + Duration::from_secs(at_secs),
            context,
        );
        let candidate = evo_observation::candidate::CandidateObservation::new(
            schema.clone(),
            provenance,
            concept.evidence(),
        );
        accept(candidate, &schema).unwrap()
    }

    fn named(subject: &str, name: &str, at_secs: u64) -> Observation {
        observation(
            &ObservationConcept::SurfaceNamed {
                subject: subject.into(),
                name: name.into(),
            },
            at_secs,
            HashMap::new(),
        )
    }

    #[test]
    fn empty_index_witnesses_nothing() {
        let index = NameIndex::empty();
        assert!(index.is_empty());
        assert!(index.name_of("https://example.com").is_none());
        assert!(index.enrich_surface("https://example.com").is_none());
    }

    #[test]
    fn strongest_witnessed_name_wins() {
        let mut index = NameIndex::empty();
        index.record("https://example.com/guide", "A Field Guide", 100);
        index.record("https://example.com/guide", "A Field Guide", 200);
        index.record("https://example.com/guide", "Guide — edited", 300);
        // 2 sightings of the original beat 1 of the edit.
        assert_eq!(
            index.name_of("https://example.com/guide"),
            Some("A Field Guide")
        );
    }

    #[test]
    fn a_newer_name_with_equal_count_wins() {
        let mut index = NameIndex::empty();
        index.record("https://example.com/guide", "First title", 100);
        index.record("https://example.com/guide", "Second title", 200);
        assert_eq!(
            index.name_of("https://example.com/guide"),
            Some("Second title")
        );
    }

    #[test]
    fn per_subject_vocabulary_is_bounded_and_evicts_the_weakest() {
        let mut index = NameIndex::empty();
        for i in 0..6 {
            index.record("s", &format!("name-{i}"), i);
        }
        // One entry over the cap was evicted; the strongest survivor wins.
        assert!(index.name_of("s").is_some());
        let names = index.name_of("s").map(|_| ()).unwrap();
        let _ = names;
        // The earliest, least-witnessed names are gone; the most recent survives.
        assert_ne!(index.name_of("s"), Some("name-0"));
        assert_eq!(index.name_of("s"), Some("name-5"));
    }

    #[test]
    fn degenerate_namings_are_not_recorded() {
        let mut index = NameIndex::empty();
        index.record("", "name", 1);
        index.record("   ", "name", 1);
        index.record("s", "", 1);
        index.record("s", "   ", 1);
        assert!(index.is_empty());
    }

    #[test]
    fn browser_suffix_is_stripped_when_the_app_was_witnessed() {
        // The full read path: focus records witness the app names, surface
        // records witness titles with the app's own suffix. The suffix is
        // recognized because the machine itself witnessed that app.
        let mut focus_ctx = HashMap::new();
        focus_ctx.insert(
            "owning_process_name".to_string(),
            "Google Chrome".to_string(),
        );
        let observations = vec![
            observation(
                &ObservationConcept::WindowFocusGained {
                    subject: "Some Page".into(),
                },
                10,
                focus_ctx,
            ),
            named(
                "https://example.com/field-guide",
                "A Field Guide to Checkbox Land - Google Chrome",
                20,
            ),
        ];
        let index = NameIndex::from_observations(&observations);
        assert_eq!(
            index.enrich_surface("https://example.com/field-guide"),
            Some("A Field Guide to Checkbox Land".to_string())
        );
    }

    #[test]
    fn a_suffix_that_is_not_a_witnessed_app_survives() {
        // Without a witnessed application name, " - something" is part of
        // the page's own words and is never touched.
        let observations = vec![named(
            "https://example.com/x",
            "Chapter One - The Beginning",
            10,
        )];
        let index = NameIndex::from_observations(&observations);
        assert_eq!(
            index.enrich_surface("https://example.com/x"),
            Some("Chapter One - The Beginning".to_string())
        );
    }

    #[test]
    fn repeated_machine_segments_are_stripped() {
        // VS Code titles carry the workspace and the app: "file — evo —
        // Visual Studio Code". Both machine segments shed.
        let mut focus_ctx = HashMap::new();
        focus_ctx.insert(
            "owning_process_name".to_string(),
            "Visual Studio Code".to_string(),
        );
        let observations = vec![
            observation(
                &ObservationConcept::WindowFocusGained {
                    subject: "main.rs".into(),
                },
                10,
                focus_ctx,
            ),
            named(
                "file:///work/main.rs",
                "main.rs — evo — Visual Studio Code",
                20,
            ),
        ];
        let index = NameIndex::from_observations(&observations);
        assert_eq!(
            index.enrich_surface("file:///work/main.rs"),
            Some("main.rs — evo".to_string())
        );
    }

    #[test]
    fn surface_named_records_round_trip_through_the_log() {
        // The persistence contract: encode → decode → rebuild the index,
        // and the witnessed name is exactly what was accepted.
        let named_obs = named("https://example.com", "Example Title - Google Chrome", 42);
        let record = encode_observation_record(&named_obs);
        let decoded = crate::persistence::decode_observation_record_pub(record.as_bytes()).unwrap();
        let index = NameIndex::from_observations(&[decoded]);
        assert_eq!(
            index.name_of("https://example.com"),
            Some("Example Title - Google Chrome")
        );
    }

    #[test]
    fn the_persons_own_name_outranks_every_witnessed_surface_name() {
        // RFC-0014: what the person SAID about their work outranks what any
        // surface displayed. Their name is presented verbatim — never
        // suffix-stripped, never truncated mid-word by the app-name rule.
        let mut user_ctx = HashMap::new();
        user_ctx.insert("source".to_string(), "user_naming".to_string());
        let observations = vec![
            named(
                "file:///work/main.rs",
                "main.rs — evo — Visual Studio Code",
                10,
            ),
            observation(
                &ObservationConcept::WorkNamed {
                    subject: "file:///work/main.rs".into(),
                    name: "Thesis rewrite".into(),
                },
                20,
                user_ctx,
            ),
        ];
        let index = NameIndex::from_observations(&observations);
        assert_eq!(
            index.name_of("file:///work/main.rs"),
            Some("Thesis rewrite")
        );
        assert_eq!(
            index.present_name_of("file:///work/main.rs"),
            Some("Thesis rewrite".to_string())
        );
        assert_eq!(
            index.enrich_surface("file:///work/main.rs"),
            Some("Thesis rewrite".to_string())
        );
    }

    #[test]
    fn a_work_named_record_round_trips_through_the_log() {
        let mut user_ctx = HashMap::new();
        user_ctx.insert("source".to_string(), "user_naming".to_string());
        let obs = observation(
            &ObservationConcept::WorkNamed {
                subject: "https://example.com/thesis".into(),
                name: "Thesis rewrite".into(),
            },
            42,
            user_ctx,
        );
        let record = encode_observation_record(&obs);
        let decoded = crate::persistence::decode_observation_record_pub(record.as_bytes()).unwrap();
        let index = NameIndex::from_observations(&[decoded]);
        assert_eq!(
            index.name_of("https://example.com/thesis"),
            Some("Thesis rewrite")
        );
    }
}
