//! Voice trigger channel.
//!
//! The desktop shell runs the mic/wake/transcription driver. When it hears
//! a trigger, it sends the trigger text over this Unix socket; the daemon —
//! which replays the canonical thread history at request time — resolves
//! the trigger, restores the work, marks the restored subjects so capture
//! does not learn Evo's own restores as user work, and replies with the
//! outcome. The desktop renders the outcome; no daemon→desktop push channel
//! is needed.
//!
//! # Protocol
//!
//! Requests (client → daemon):
//! ```text
//! TRIGGER <byte_len>\n<utf8 trigger text>
//! CHOOSE <work_id>\n
//! CHOOSE_OCCURRENCE <work_id> <selection_key>\n
//! SCOPE <work_id> <byte_len>\n<utf8 surface query>
//! RESOURCE <work_id> <byte_len>\n<exact utf8 locator>
//! ASK\n
//! BRIEF <work_id>\n
//! WORKS\n
//! ```
//!
//! `ASK` and `BRIEF` are the engine-backed read path (Phase 4): they return
//! `OUTCOME BRIEF / COUNT n / NARRATION <one line>` — the rack (or one
//! work), spoken — computed at request time from the engine over the
//! storage log (`Engine::briefs()` narrated by `evo_voice::narrate`); they
//! need no embedder. All commands use the engine's numeric thread ids and
//! the same request-time snapshot. Its restore set is the neighborhood,
//! not the museum; legacy engagement ids are not reinterpreted.
//! `SCOPE` is structured selected-work transport, not natural-language command
//! parsing. Empty/unresolved scope never falls back to `CHOOSE`.
//!
//! Resolution asks before it guesses and hides nothing: a TRIGGER that no
//! work answers clearly above the measured restore floor
//! (`RETRIEVAL_RESTORE_FLOOR`) is answered with `AMBIGUOUS` and the nearest
//! works, best first — never with the raw recents list; a work is restored
//! without asking only when it is clearly above that floor *and* clearly
//! ahead of the runner-up.
//!
//! `WORKS` is the browsable read path for the notch's scrollable list: the
//! same request-time snapshot `ASK`/`BRIEF` narrate (`voice_facts`, one
//! replay per request — never cached), rendered as one line per work in the
//! engine's canonical display order. It is the one verb whose reply is not a
//! [`VoiceOutcome`], so it has its own response kind; `VoiceOutcome` stays
//! untouched for the desktop's existing parsers.
//!
//! Responses (daemon → client), line-encoded; every kind but `WORKS` is a
//! [`VoiceOutcome`] statement:
//! ```text
//! OUTCOME RESTORED\nTITLE <title>\nSUMMARY <summary>\n[OFFERED <resource>\n...]\nEND\n
//! OUTCOME AMBIGUOUS\nCANDIDATE <work_id>\t<title>[\tOCCURRENCE\t<selection_key>]\n...\nEND\n
//! OUTCOME NOTFOUND\nRECENT <work_id>\t<title>\n...\nEND\n
//! OUTCOME FAILED\nREASON <text>\nEND\n
//! OUTCOME NOCOMMAND\nEND\n
//! WORKS <count>\nWORK <id>\t<status>\t<episodes>\t<title>\t<resume_reason>\n...\nEND\n
//! ```
//!
//! # No hardcoding
//!
//! Nothing here names an application, domain, or command phrase. Trigger
//! resolution is semantic (embeddings over the person's own witnessed
//! material); ASK/BRIEF render the engine's own facts through narration;
//! the socket is a dumb transport.

use evo_engagement::Reconstruction;
use evo_execution::locator::{LocatorKind, classify_subject};
use evo_execution::{RestoreOutcome, RestoreTarget};
use evo_observation::observation::Observation;
use evo_observation::observed_state::ObservedState;
use evo_observation::surface_identity::SurfaceIdentity;
use evo_semantic::{Embedder, SemanticIndex, SemanticParams, SemanticResolution};
use evo_threads::retrieval::{ScopeResolution, resolve_scope};
use evo_threads::{Engine, ResumeBundle, WorkBrief};
use evo_voice::{RestoredSubjects, VoiceCandidate, VoiceOutcome, VoiceSelection};

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};

/// Max trigger text length, in bytes. A spoken command is a sentence, not
/// a document; this bounds a misbehaving client.
const MAX_TRIGGER_BYTES: usize = 4096;

/// Max `WORK` lines in one `WORKS` reply. The listing is a browse of the
/// top of the rack, not an export; the engine's canonical order already
/// ranks best-supported first, so the cap only bounds the wire.
const MAX_LISTED_WORKS: usize = 24;

/// Max works in a trigger's below-floor carousel. Three to five reads as
/// "did you mean…" when spoken; the whole rack would read as a listing and
/// bury the related items this carousel exists to surface.
const NEAREST_CANDIDATES: usize = 5;

#[cfg(test)]
#[path = "../tests/browser_identity_fixture/mod.rs"]
mod browser_identity_fixture;

/// Deterministic socket path: FNV-1a hash of the case-folded storage root,
/// same derivation rule as the other daemon channels, distinct prefix.
pub fn voice_socket_path(storage_root: &Path) -> PathBuf {
    let folded = storage_root.to_string_lossy().to_lowercase();
    let hash = fnv1a64(folded.as_bytes());
    std::env::temp_dir().join(format!("evo-voice-{hash:016x}.sock"))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// Legacy publication contract retained for the runtime worker. Voice
/// commands no longer consult it: thread facts and alternatives are read
/// together from canonical storage even before the first publication.
pub struct VoiceKnowledge {
    pub reconstruction: Arc<Reconstruction>,
    pub recent: Vec<VoiceCandidate>,
}

/// One immutable request snapshot for matching, narration and execution.
struct VoiceFacts {
    briefs: Vec<WorkBrief>,
    bundles: BTreeMap<u64, ResumeBundle>,
    names: BTreeMap<String, String>,
    documents: Vec<(u64, String)>,
    /// Work-local occurrences from the selected work's last assigned episode.
    /// A conflicting/invalid witness is a refusal, never an unbound fallback.
    surfaces: BTreeMap<(u64, String), Result<SurfaceIdentity, String>>,
    /// Distinct complete surface witnesses for each work-local resource.
    /// These are retained so a person can explicitly choose one instead of
    /// forcing a work-level trigger to guess.
    occurrences: BTreeMap<(u64, String), Vec<SurfaceOccurrence>>,
}

#[derive(Debug, Clone)]
struct SurfaceOccurrence {
    observed_at: u64,
    surface: SurfaceIdentity,
    label: Option<String>,
}

/// Read once, including names. Unlike the historical best-effort loader,
/// voice must not call a damaged history "no works" or act on a partial one.
fn voice_facts(storage_root: &Path) -> Result<VoiceFacts, VoiceOutcome> {
    use evo_storage::{Storage, StorageObjectKind};
    let _guard = Storage::with_thread_root(storage_root.to_path_buf());
    // read_all repairs torn tails in place. A live request must not truncate
    // a concurrent writer: read_from is read-only. Require at least the
    // complete prefix that existed when the request began reading.
    let length = match std::fs::metadata(storage_root.join("observation.log")) {
        Ok(metadata) => metadata.len(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => 0,
        Err(err) => return Err(history_failed(err.to_string())),
    };
    let (records, complete) = Storage::new()
        .read_from(StorageObjectKind::Observation, 0)
        .map_err(|err| history_failed(err.to_string()))?;
    if complete < length {
        return Err(history_failed(
            "incomplete observation log; retry after the writer finishes".into(),
        ));
    }
    let observations = records
        .iter()
        .map(|record| crate::persistence::decode_observation_record_pub(record))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| history_failed(err.to_string()))?;
    Ok(facts_from_observations(&observations))
}

fn history_failed(reason: String) -> VoiceOutcome {
    VoiceOutcome::Failed {
        reason: format!("could not read work history: {reason}"),
    }
}

fn facts_from_observations(observations: &[Observation]) -> VoiceFacts {
    let engine = crate::threads::engine_from_observations(observations);
    let mut facts = facts_from_engine(
        &engine,
        &crate::name_index::NameIndex::from_observations(observations),
    );
    for work in engine.threads() {
        let Some(episode) = work.episodes.last() else {
            continue;
        };
        for resource in &facts.bundles[&work.id].restore_set {
            let mut witnessed = Vec::new();
            let mut occurrences = Vec::new();
            let mut invalid = false;
            let mut latest_identified = false;
            for observation in observations {
                if observation.provenance().is_execution_origin()
                    || observation.schema().name() != "OBS-WINDOW-FOCUS-GAINED"
                {
                    continue;
                }
                let at = observation
                    .provenance()
                    .observed_at()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
                    .min(u64::MAX as u128) as u64;
                if at < episode.span.0 || at > episode.span.1 {
                    continue;
                }
                if !crate::threads::project_observation(observation)
                    .is_some_and(|event| event.resource == *resource)
                {
                    continue;
                }
                let context = observation.provenance().context();
                if let Some(state) = ObservedState::from_context(context)
                    && let Some(surface) = state.surface_identity().cloned()
                {
                    latest_identified = true;
                    if !witnessed.contains(&surface) {
                        witnessed.push(surface.clone());
                        occurrences.push(SurfaceOccurrence {
                            observed_at: at,
                            surface,
                            label: state.window_title().map(str::to_string),
                        });
                    }
                } else {
                    latest_identified = false;
                    if context
                        .keys()
                        .any(|key| key.starts_with("observed_state.surface."))
                    {
                        invalid = true;
                    }
                }
            }
            let constraint = if invalid {
                Some(Err(
                    "malformed occurrence evidence; refusing an unbound fallback".into(),
                ))
            } else if witnessed.len() > 1 {
                Some(Err(format!(
                    "{} witnessed occurrences in this work's last episode; choose an occurrence before restoring",
                    witnessed.len()
                )))
            } else if !witnessed.is_empty() && !latest_identified {
                // Replay follows append order. An identity-free later focus
                // cannot inherit an earlier window merely by sharing its
                // locator. Old logs with no occurrence evidence stay unbound.
                Some(Err(
                    "latest focus has no occurrence identity; refusing stale occurrence or unbound fallback".into(),
                ))
            } else {
                witnessed.pop().map(Ok)
            };
            facts
                .occurrences
                .insert((work.id, resource.clone()), occurrences);
            if let Some(constraint) = constraint {
                facts
                    .surfaces
                    .insert((work.id, resource.clone()), constraint);
            }
        }
    }
    facts
}

fn facts_from_engine(engine: &Engine, names_index: &crate::name_index::NameIndex) -> VoiceFacts {
    let briefs = engine.briefs();
    let bundles: BTreeMap<u64, ResumeBundle> = briefs
        .iter()
        .map(|b| (b.thread_id, engine.resume_bundle(b.thread_id)))
        .collect();
    let mut names = BTreeMap::new();
    for b in &briefs {
        for r in b
            .anchors
            .iter()
            .map(|a| &a.resource)
            .chain(&b.companions)
            .chain(&b.contested)
            .chain(std::iter::once(&b.resume_point))
            .chain(&b.restore_set)
        {
            if let Some(n) = names_index.present_name_of(r) {
                names.insert(r.clone(), n);
            }
        }
    }
    // No Home age filter: dormant work still answers to its witnessed name.
    let documents = engine
        .threads()
        .iter()
        .map(|thread| {
            // One short natural-language line from the understanding layer
            // (the work's own next step: "Continue in …", "Unsaved changes in
            // …") so a vague spoken phrase can meet a sentence before it
            // meets a locator. Measured on a real log: this kept every strong
            // hit's ranking, improved a near-tie's order, and lifted no vague
            // phrase over the restore floor; the longer narrative was
            // measured to dilute true hits and is deliberately not used.
            let understanding = crate::work_understanding::understand_named(
                thread,
                &bundles[&thread.id],
                names_index,
            );
            let material = evo_threads::retrieval::thread_document_with_surface(
                thread,
                &[understanding.next_step.as_str()],
            );
            // Names first, so an opaque locator cannot consume the model's token
            // budget before the words the person actually saw. Membership stays
            // the engine's; enrichment supplies vocabulary, never more targets.
            let witnessed: Vec<_> = thread
                .anchors
                .iter()
                .map(|a| a.resource.as_str())
                .chain(thread.companions.iter().map(|(r, _)| r.as_str()))
                .filter_map(|r| names_index.name_of(r))
                .collect();
            let document = if witnessed.is_empty() {
                material
            } else {
                format!("{} | {material}", witnessed.join(" | "))
            };
            (thread.id, document)
        })
        .collect();
    VoiceFacts {
        briefs,
        bundles,
        names,
        documents,
        surfaces: BTreeMap::new(),
        occurrences: BTreeMap::new(),
    }
}

/// Spawns the voice listener thread.
///
/// - `knowledge`: retained for source compatibility with the runtime publisher.
/// - `restored`: the execution-witness registry (this listener marks;
///   the capture pipeline checks).
/// - `embedder`: the single loaded model, shared by reference. `None` when
///   the model failed to load — TRIGGER then fails honestly; ids and narration
///   need no model.
pub fn spawn_voice_listener(
    storage_root: PathBuf,
    knowledge: Arc<RwLock<Option<VoiceKnowledge>>>,
    restored: Arc<Mutex<RestoredSubjects>>,
    embedder: Option<Arc<dyn Embedder + Send + Sync>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let socket_path = voice_socket_path(&storage_root);
        let _ = std::fs::remove_file(&socket_path);
        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => l,
            Err(err) => {
                eprintln!("EVO-DAEMON voice listener bind failed: {err}");
                return;
            }
        };
        eprintln!("EVO-DAEMON voice listener on {}", socket_path.display());
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let knowledge = Arc::clone(&knowledge);
                    let restored = Arc::clone(&restored);
                    let embedder = embedder.clone();
                    let root = storage_root.clone();
                    thread::spawn(move || {
                        handle_voice_connection(stream, &root, &knowledge, &restored, embedder);
                    });
                }
                Err(err) => {
                    eprintln!("EVO-DAEMON voice accept failed: {err}");
                }
            }
        }
    })
}

fn handle_voice_connection(
    stream: UnixStream,
    storage_root: &Path,
    _knowledge: &Arc<RwLock<Option<VoiceKnowledge>>>,
    restored: &Arc<Mutex<RestoredSubjects>>,
    embedder: Option<Arc<dyn Embedder + Send + Sync>>,
) {
    handle_voice_stream(
        stream,
        storage_root,
        restored,
        embedder.as_deref().map(|e| e as &dyn Embedder),
        evo_execution::restore,
    );
}

/// The actual wire path, injectable for socket tests without a live daemon.
fn handle_voice_stream(
    stream: UnixStream,
    storage_root: &Path,
    restored: &Arc<Mutex<RestoredSubjects>>,
    embedder: Option<&dyn Embedder>,
    restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;

    let mut header = String::new();
    if reader.read_line(&mut header).is_err() {
        return;
    }
    let header = header.strip_suffix('\n').unwrap_or(&header);
    let header = header.strip_suffix('\r').unwrap_or(header);

    let reply = dispatch_voice_request(
        header,
        &mut reader,
        storage_root,
        restored,
        embedder,
        restore,
    );
    let encoded = match reply {
        VoiceReply::Outcome(outcome) => encode_outcome(&outcome),
        VoiceReply::Works(listing) => listing,
    };
    let _ = writer.write_all(encoded.as_bytes());
}

/// One request, one response kind. Every classic verb answers with a
/// [`VoiceOutcome`]; `WORKS` answers with its own browse listing, already
/// line-encoded. The transport only encodes and writes.
enum VoiceReply {
    Outcome(VoiceOutcome),
    Works(String),
}

/// The single request dispatch: one replay of the canonical log per request,
/// never a cache. `WORKS` (the browse listing) is matched beside `ASK` and
/// `BRIEF`; its failures still flow through the honest [`VoiceOutcome`]
/// path, so an unavailable engine reads the same for every verb.
fn dispatch_voice_request(
    header: &str,
    reader: &mut impl Read,
    storage_root: &Path,
    restored: &Arc<Mutex<RestoredSubjects>>,
    embedder: Option<&dyn Embedder>,
    restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
) -> VoiceReply {
    if let Some(rest) = header.strip_prefix("TRIGGER ") {
        VoiceReply::Outcome(handle_trigger(
            rest,
            reader,
            storage_root,
            restored,
            embedder,
            restore,
        ))
    } else if let Some(rest) = header.strip_prefix("SCOPE ") {
        VoiceReply::Outcome(handle_scope(
            rest,
            reader,
            storage_root,
            restored,
            embedder,
            false,
            restore,
        ))
    } else if let Some(rest) = header.strip_prefix("RESOURCE ") {
        VoiceReply::Outcome(handle_scope(
            rest,
            reader,
            storage_root,
            restored,
            embedder,
            true,
            restore,
        ))
    } else if let Some(work_id) = header.strip_prefix("CHOOSE ") {
        VoiceReply::Outcome(match voice_facts(storage_root) {
            Ok(facts) => restore_from_facts(work_id.trim(), &facts, restored, restore, None),
            Err(outcome) => outcome,
        })
    } else if let Some(rest) = header.strip_prefix("CHOOSE_OCCURRENCE ") {
        VoiceReply::Outcome(handle_occurrence_choice(
            rest,
            storage_root,
            restored,
            restore,
        ))
    } else if header.trim() == "ASK" {
        // Engine-backed read path: narrate the rack. No embedder, no
        // engagement — the engine's facts, computed at request time, spoken.
        VoiceReply::Outcome(ask(storage_root))
    } else if let Some(work_id) = header.strip_prefix("BRIEF ") {
        VoiceReply::Outcome(brief(work_id.trim(), storage_root))
    } else if header.trim() == "WORKS" {
        // The browse read path: the rack as a list, from the same
        // request-time snapshot ASK/BRIEF narrate. No embedder needed.
        match works(storage_root) {
            Ok(listing) => VoiceReply::Works(listing),
            Err(outcome) => VoiceReply::Outcome(outcome),
        }
    } else {
        VoiceReply::Outcome(VoiceOutcome::Failed {
            reason: format!("unknown voice request: {header:?}"),
        })
    }
}

fn handle_occurrence_choice(
    header: &str,
    storage_root: &Path,
    restored: &Arc<Mutex<RestoredSubjects>>,
    restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
) -> VoiceOutcome {
    let mut fields = header.split(' ');
    let Some(work_id) = fields.next() else {
        return VoiceOutcome::Failed {
            reason: "malformed occurrence choice".into(),
        };
    };
    let Some(key) = fields.next() else {
        return VoiceOutcome::Failed {
            reason: "malformed occurrence choice".into(),
        };
    };
    if fields.next().is_some()
        || work_id
            .parse::<u64>()
            .ok()
            .map(|id| id.to_string())
            .as_deref()
            != Some(work_id)
        || key.len() != 16
        || !key.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return VoiceOutcome::Failed {
            reason: "malformed occurrence choice".into(),
        };
    }
    let facts = match voice_facts(storage_root) {
        Ok(facts) => facts,
        Err(outcome) => return outcome,
    };
    let Some(selected) = find_selected_occurrence(work_id, key, &facts) else {
        return VoiceOutcome::Failed {
            reason: "That witnessed occurrence is no longer available; no substitute was opened."
                .into(),
        };
    };
    restore_from_facts(work_id, &facts, restored, restore, Some(&selected))
}

/// The classic `VoiceOutcome`-only view of the dispatch, retained for the
/// socket tests and the browser identity fixture, which supply an executor
/// and never touch desktop windows. Production goes through
/// [`dispatch_voice_request`] so `WORKS` can answer with its own kind.
#[cfg(test)]
fn handle_voice_request(
    header: &str,
    reader: &mut impl Read,
    storage_root: &Path,
    restored: &Arc<Mutex<RestoredSubjects>>,
    embedder: Option<&dyn Embedder>,
    restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
) -> VoiceOutcome {
    match dispatch_voice_request(header, reader, storage_root, restored, embedder, restore) {
        VoiceReply::Outcome(outcome) => outcome,
        VoiceReply::Works(_) => unreachable!("WORKS answers with its own listing kind"),
    }
}

fn handle_scope(
    header: &str,
    reader: &mut impl Read,
    storage_root: &Path,
    restored: &Arc<Mutex<RestoredSubjects>>,
    embedder: Option<&dyn Embedder>,
    exact: bool,
    restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
) -> VoiceOutcome {
    let command = if exact { "RESOURCE" } else { "SCOPE" };
    let malformed = || VoiceOutcome::Failed {
        reason: format!(
            "Expected {command} <canonical-thread-id> <byte_len> followed by nonempty UTF-8 text (1–4096 bytes)."
        ),
    };
    let Some((work_id, length)) = header.split_once(' ') else {
        return malformed();
    };
    let Ok(id) = work_id.parse::<u64>() else {
        return malformed();
    };
    let Ok(length_value) = length.parse::<usize>() else {
        return malformed();
    };
    if id.to_string() != work_id
        || length_value.to_string() != length
        || !(1..=MAX_TRIGGER_BYTES).contains(&length_value)
    {
        return malformed();
    }
    let mut body = vec![0; length_value];
    if reader.read_exact(&mut body).is_err() {
        return malformed();
    }
    let Ok(query) = String::from_utf8(body) else {
        return malformed();
    };
    if query.trim().is_empty() {
        return malformed();
    }
    let facts = match voice_facts(storage_root) {
        Ok(facts) => facts,
        Err(outcome) => return outcome,
    };
    let Some(b) = facts.briefs.iter().find(|b| b.thread_id == id) else {
        return unknown_work(work_id);
    };
    let Some(bundle) = facts.bundles.get(&id) else {
        return unknown_work(work_id);
    };
    let resolution = if exact {
        if !bundle.restore_set.contains(&query) {
            return VoiceOutcome::Failed { reason: "The selected exact resource is no longer eligible in this work. Refresh the list and choose again; no substitute was opened.".into() };
        }
        ScopeResolution::Matched {
            resources: vec![query],
        }
    } else if let Some(embedder) = embedder {
        if !embedder.allows_similarity_restore() {
            if let Some(resource) = bundle.restore_set.iter().find(|r| r.as_str() == query) {
                return restore_resources(
                    b,
                    &facts,
                    std::slice::from_ref(resource),
                    restored,
                    restore,
                    None,
                );
            }
            return VoiceOutcome::Failed {
                reason: "This embedding model is not calibrated for automatic scoped restoration. Choose an exact resource from this work; no substitute was opened.".into(),
            };
        }
        match resolve_scope(bundle, &query, embedder) {
            Ok(resolution) => resolution,
            Err(err) => {
                return VoiceOutcome::Failed {
                    reason: format!(
                        "Scope matching failed: {err}. Retry with an exact resource locator from this work."
                    ),
                };
            }
        }
    } else if let Some(resource) = bundle
        .restore_set
        .iter()
        .find(|r| r.as_str() == query)
        .or_else(|| {
            bundle
                .restore_set
                .iter()
                .find(|r| r.as_str() == query.trim())
        })
    {
        ScopeResolution::Matched {
            resources: vec![resource.clone()],
        }
    } else {
        return VoiceOutcome::Failed {
            reason: concat!(
                "Scope model unavailable. Retry with an exact resource locator ",
                "from the selected work."
            )
            .into(),
        };
    };
    match resolution {
        ScopeResolution::Matched { resources }
            if !resources.is_empty()
                && resources.iter().all(|r| bundle.restore_set.contains(r)) =>
        {
            restore_resources(b, &facts, &resources, restored, restore, None)
        }
        ScopeResolution::Ambiguous { candidates } => VoiceOutcome::Failed {
            reason: format!(
                "Scope matches multiple resources. Retry SCOPE {id} with one exact locator: {}",
                candidates.join("; ")
            ),
        },
        _ => VoiceOutcome::Failed {
            reason: format!(
                "No resource in work {id} matched that scope. Retry SCOPE with an exact resource locator from this work."
            ),
        },
    }
}

/// ASK: the whole rack, narrated from the engine's facts — computed at
/// request time, so the spoken answer is as fresh as the log itself.
fn ask(storage_root: &Path) -> VoiceOutcome {
    let facts = match voice_facts(storage_root) {
        Ok(facts) => facts,
        Err(outcome) => return outcome,
    };
    narrate_rack(&facts.briefs, &facts.names, facts.briefs.len())
}

/// One `VoiceOutcome::Brief` from briefs + names. The pure seam the tests
/// pin: facts in, sentences out.
fn narrate_rack(
    briefs: &[WorkBrief],
    names: &std::collections::BTreeMap<String, String>,
    count: usize,
) -> VoiceOutcome {
    let lookup = name_lookup(names);
    let mut narration = evo_voice::narrate_briefs(briefs, &lookup);
    for b in briefs {
        narration.push_str(&fork_notice(b));
    }
    VoiceOutcome::Brief { count, narration }
}

/// BRIEF <work_id>: one work, narrated from request-time facts. Engine
/// thread ids are numeric.
fn brief(work_id: &str, storage_root: &Path) -> VoiceOutcome {
    let facts = match voice_facts(storage_root) {
        Ok(facts) => facts,
        Err(outcome) => return outcome,
    };
    match facts
        .briefs
        .iter()
        .find(|b| b.thread_id.to_string() == work_id)
    {
        Some(b) => {
            let lookup = name_lookup(&facts.names);
            VoiceOutcome::Brief {
                count: 1,
                narration: evo_voice::narrate_brief(b, &lookup) + &fork_notice(b),
            }
        }
        None => VoiceOutcome::Failed {
            reason: format!("unknown work id: {work_id:?}"),
        },
    }
}

/// WORKS: the rack as a browse listing — the same request-time facts
/// ASK/BRIEF narrate, one replay per request, never cached. Engine
/// unavailability is the same honest failure the other read verbs report.
fn works(storage_root: &Path) -> Result<String, VoiceOutcome> {
    let facts = voice_facts(storage_root)?;
    Ok(encode_works(&facts))
}

/// Encodes the browse listing from the request-time snapshot.
///
/// Ranking is `Engine::briefs()` order — the engine's canonical display
/// order, the same order `ASK` narrates and `NOTFOUND`'s `RECENT` lines
/// list — capped at [`MAX_LISTED_WORKS`]. The count is the number of `WORK`
/// lines that follow, so a capped page stays self-describing. Titles reuse
/// the candidate lines' own logic; tabs and newlines are flattened so a
/// witnessed name can never break the line protocol.
fn encode_works(facts: &VoiceFacts) -> String {
    let listed = facts
        .briefs
        .iter()
        .take(MAX_LISTED_WORKS)
        .collect::<Vec<_>>();
    let mut out = format!("WORKS {}\n", listed.len());
    for b in listed {
        let c = candidate(b, facts);
        out.push_str(&format!(
            "WORK {}\t{}\t{}\t{}\t{}\n",
            c.work_id,
            status_word(b.status),
            b.episodes,
            sanitize_line(&c.title),
            sanitize_line(&b.resume_reason),
        ));
    }
    out.push_str("END\n");
    out
}

/// The view's own word, lowercase, as the listing carries it.
fn status_word(status: evo_threads::Status) -> &'static str {
    match status {
        evo_threads::Status::Active => "active",
        evo_threads::Status::Dormant => "dormant",
    }
}

fn fork_notice(b: &WorkBrief) -> String {
    match &b.fork {
        Some(ids) => format!(
            " Work {} has an unresolved choice between works {}.",
            b.thread_id,
            ids.iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        None => String::new(),
    }
}

fn name_lookup(names: &std::collections::BTreeMap<String, String>) -> evo_voice::NameLookup<'_> {
    names
        .iter()
        .map(|(r, n)| (r.as_str(), n.as_str()))
        .collect()
}

/// Handles a TRIGGER request: resolve the trigger text and restore.
fn handle_trigger(
    len_str: &str,
    reader: &mut impl Read,
    storage_root: &Path,
    restored: &Arc<Mutex<RestoredSubjects>>,
    embedder: Option<&dyn Embedder>,
    restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
) -> VoiceOutcome {
    let byte_len: usize = match len_str.trim().parse() {
        Ok(n) if (1..=MAX_TRIGGER_BYTES).contains(&n) => n,
        _ => {
            return VoiceOutcome::Failed {
                reason: format!("bad TRIGGER length: {len_str:?}"),
            };
        }
    };
    let mut buf = vec![0u8; byte_len];
    if reader.read_exact(&mut buf).is_err() {
        return VoiceOutcome::Failed {
            reason: "TRIGGER body unreadable".to_string(),
        };
    }
    let text = match String::from_utf8(buf) {
        Ok(t) => t,
        Err(_) => {
            return VoiceOutcome::Failed {
                reason: "TRIGGER body is not UTF-8".to_string(),
            };
        }
    };

    // The daemon is a second trust boundary: a client must not turn a wake
    // gesture, placeholder, or noisy model marker into a semantic restore.
    // Strip a repeated wake only so a well-formed explicit command remains
    // compatible with older clients; the wake itself is never resolved.
    let text = evo_voice::sanitize_transcript(&text);
    let text = match evo_voice::detect_wake(&text) {
        evo_voice::WakeResult::Detected { command } => command,
        evo_voice::WakeResult::Absent => text,
    };
    if text.is_empty() {
        return VoiceOutcome::NoCommand;
    }

    let facts = match voice_facts(storage_root) {
        Ok(facts) => facts,
        Err(outcome) => return outcome,
    };
    let Some(embedder) = embedder else {
        return VoiceOutcome::Failed {
            reason: voice_model_unavailable_reason(),
        };
    };
    trigger_from_facts(&text, &facts, embedder, restored, restore)
}

#[cfg(feature = "embeddings")]
fn voice_model_unavailable_reason() -> String {
    "voice model unavailable: no usable local embedding model was loaded".into()
}

#[cfg(not(feature = "embeddings"))]
fn voice_model_unavailable_reason() -> String {
    "voice model unavailable: semantic embeddings are disabled; build with the `embeddings` feature for local semantic triggers".into()
}

fn trigger_from_facts(
    text: &str,
    facts: &VoiceFacts,
    embedder: &dyn Embedder,
    restored: &Arc<Mutex<RestoredSubjects>>,
    restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
) -> VoiceOutcome {
    let params = SemanticParams {
        margin: evo_threads::retrieval::RETRIEVAL_MARGIN,
        // The restore bar, not the matching bar: a trigger opens a work
        // without asking only on confidence evidence (the constant carries
        // its measured calibration).
        floor: evo_threads::retrieval::RETRIEVAL_RESTORE_FLOOR,
    };
    let index = match SemanticIndex::build(facts.documents.iter().cloned(), embedder, params) {
        Ok(index) => index,
        Err(err) => return embedding_failed(err),
    };
    match index.resolve(text, embedder) {
        Ok(SemanticResolution::Matched { id, .. }) => {
            let Some(b) = facts.briefs.iter().find(|b| b.thread_id == id) else {
                return unknown_work(&id.to_string());
            };
            if let Some(candidates) = &b.fork {
                return ambiguous(facts, candidates);
            }
            if !embedder.allows_similarity_restore() {
                return ambiguous(facts, &[id]);
            }
            // Restore from the snapshot we matched, never a second replay.
            restore_from_facts(&id.to_string(), facts, restored, restore, None)
        }
        Ok(SemanticResolution::Ambiguous { candidates }) => ambiguous(facts, &candidates),
        // Nothing cleared the restore floor. Recency is not relatedness: the
        // honest answer is the nearest works — shown in the same carousel an
        // ambiguous trigger uses, never opened.
        Ok(SemanticResolution::NoMatch) => nearest_candidates(text, facts, &index, embedder),
        Err(err) => embedding_failed(err),
    }
}

fn embedding_failed(err: evo_semantic::EmbedError) -> VoiceOutcome {
    VoiceOutcome::Failed {
        reason: format!("embedding failed: {err}"),
    }
}

/// The below-floor carousel: the nearest works to what was said, best first.
///
/// Reuses the ambiguous outcome's machinery and wire shape — the desktop
/// already renders [`VoiceOutcome::Ambiguous`] as a question — so a vague
/// phrase meets related items instead of an arbitrary recents list. Ranking
/// is evidence, not permission: no candidate here is ever restored. Only a
/// rack with nothing in it is `NotFound`.
fn nearest_candidates(
    text: &str,
    facts: &VoiceFacts,
    index: &SemanticIndex<u64>,
    embedder: &dyn Embedder,
) -> VoiceOutcome {
    let ranked = match index.rank(text, embedder) {
        Ok(ranked) => ranked,
        Err(err) => return embedding_failed(err),
    };
    if ranked.is_empty() {
        return VoiceOutcome::NotFound {
            recent: facts.briefs.iter().map(|b| candidate(b, facts)).collect(),
        };
    }
    let nearest: Vec<u64> = ranked
        .iter()
        .take(NEAREST_CANDIDATES)
        .map(|(id, _)| *id)
        .collect();
    ambiguous(facts, &nearest)
}

fn candidate(b: &WorkBrief, facts: &VoiceFacts) -> VoiceCandidate {
    let resource = b
        .anchors
        .first()
        .map(|a| a.resource.as_str())
        .unwrap_or(&b.resume_point);
    VoiceCandidate::work(
        b.thread_id.to_string(),
        facts.names.get(resource).cloned().unwrap_or_else(|| {
            if resource.is_empty() {
                format!("work {}", b.thread_id)
            } else {
                resource.to_string()
            }
        }),
    )
}

fn ambiguous(facts: &VoiceFacts, ids: &[u64]) -> VoiceOutcome {
    let mut candidates = Vec::new();
    let mut pending = ids.to_vec();
    let mut seen = std::collections::BTreeSet::new();
    let mut index = 0;
    while index < pending.len() {
        let id = pending[index];
        index += 1;
        if !seen.insert(id) {
            continue;
        }
        let Some(b) = facts.briefs.iter().find(|b| b.thread_id == id) else {
            return unknown_work(&id.to_string());
        };
        candidates.push(candidate(b, facts));
        if let Some(fork) = &b.fork {
            pending.extend(fork);
        }
    }
    VoiceOutcome::Ambiguous { candidates }
}

fn unknown_work(work_id: &str) -> VoiceOutcome {
    VoiceOutcome::Failed {
        reason: format!("unknown work id: {work_id:?}"),
    }
}

/// Builds an opaque, deterministic capability for one witnessed occurrence.
/// The daemon recomputes and verifies it from the fresh request snapshot; the
/// desktop never needs to know the surface identity fields themselves.
fn occurrence_key(
    work_id: u64,
    resource: &str,
    observed_at: u64,
    surface: &SurfaceIdentity,
) -> String {
    let material = format!("{work_id}\0{resource}\0{observed_at}\0{surface:?}");
    format!("{:016x}", fnv1a64(material.as_bytes()))
}

fn occurrence_candidates(
    b: &WorkBrief,
    facts: &VoiceFacts,
    resources: &[String],
) -> Vec<VoiceCandidate> {
    let work_title = candidate(b, facts).title;
    let mut choices = Vec::new();
    for resource in resources {
        let Some(occurrences) = facts.occurrences.get(&(b.thread_id, resource.clone())) else {
            continue;
        };
        if occurrences.len() < 2 {
            continue;
        }
        let resource_name = facts
            .names
            .get(resource)
            .cloned()
            .unwrap_or_else(|| resource.clone());
        for (index, occurrence) in occurrences.iter().enumerate() {
            let detail = occurrence
                .label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| "witnessed surface".into());
            let title = if resources.len() == 1 {
                format!("{work_title} · occurrence {} — {detail}", index + 1)
            } else {
                format!(
                    "{work_title} · {resource_name} · occurrence {} — {detail}",
                    index + 1
                )
            };
            choices.push(VoiceCandidate::occurrence(
                b.thread_id.to_string(),
                title,
                occurrence_key(
                    b.thread_id,
                    resource,
                    occurrence.observed_at,
                    &occurrence.surface,
                ),
            ));
        }
    }
    choices
}

#[derive(Debug, Clone)]
struct SelectedOccurrence {
    resource: String,
    surface: SurfaceIdentity,
}

fn find_selected_occurrence(
    work_id: &str,
    key: &str,
    facts: &VoiceFacts,
) -> Option<SelectedOccurrence> {
    let id = work_id.parse::<u64>().ok()?;
    let mut selected = None;
    for ((candidate_id, resource), occurrences) in &facts.occurrences {
        if *candidate_id != id {
            continue;
        }
        for occurrence in occurrences {
            if occurrence_key(id, resource, occurrence.observed_at, &occurrence.surface) == key {
                if selected.is_some() {
                    return None;
                }
                selected = Some(SelectedOccurrence {
                    resource: resource.clone(),
                    surface: occurrence.surface.clone(),
                });
            }
        }
    }
    selected
}

/// Only canonical decimal ids from this snapshot are eligible. No parsing
/// or fallback that could turn a legacy hexadecimal id into another work.
/// Unscoped TRIGGER/CHOOSE share the execution boundary with explicit SCOPE.
fn restore_from_facts(
    work_id: &str,
    facts: &VoiceFacts,
    restored: &Arc<Mutex<RestoredSubjects>>,
    restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
    selected: Option<&SelectedOccurrence>,
) -> VoiceOutcome {
    let Some(b) = facts
        .briefs
        .iter()
        .find(|b| b.thread_id.to_string() == work_id)
    else {
        return unknown_work(work_id);
    };
    restore_resources(b, facts, &b.restore_set, restored, restore, selected)
}

fn restore_resources(
    b: &WorkBrief,
    facts: &VoiceFacts,
    resources: &[String],
    restored: &Arc<Mutex<RestoredSubjects>>,
    mut restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
    selected: Option<&SelectedOccurrence>,
) -> VoiceOutcome {
    if resources.is_empty() {
        return VoiceOutcome::Failed {
            reason: format!("work {} has nothing restorable", b.thread_id),
        };
    }
    if selected.is_none() {
        let candidates = occurrence_candidates(b, facts, resources);
        if !candidates.is_empty() {
            return VoiceOutcome::Ambiguous { candidates };
        }
    }
    let mut outcome = RestoreOutcome::default();
    for resource in resources {
        let mut target = RestoreTarget::with_default_handler(resource);
        match facts.surfaces.get(&(b.thread_id, resource.clone())) {
            Some(Ok(surface)) => target.surface = Some(surface.clone()),
            Some(Err(reason)) => {
                if selected.is_some_and(|selected| selected.resource == *resource) {
                    target.surface = selected.map(|selected| selected.surface.clone());
                } else {
                    outcome.unopened.push((resource.clone(), reason.clone()));
                    continue;
                }
            }
            None => {}
        }
        // These are only the two exact strings the executor can report for
        // this dispatch, not general URI/path equivalence or percent decoding.
        let locator = match classify_subject(resource) {
            LocatorKind::FilePath(path)
            | LocatorKind::Url(path)
            | LocatorKind::WindowTitle(path) => path,
        };
        let mut aliases = vec![resource.as_str()];
        if locator != *resource {
            aliases.push(&locator);
        }
        let reservation = restored
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .reserve_aliases(&aliases);
        let Some(reservation) = reservation else {
            outcome.unopened.push((
                resource.clone(),
                "restore witness registry unavailable".into(),
            ));
            continue;
        };
        // Reserve before the OS effect, but never hold the mutex across it:
        // capture must be able to observe the reservation during dispatch.
        let receipt = restore(&[target]);
        // One target requires exactly one receipt in exactly one bucket.
        // Duplicates, conflicting buckets and alien resources all fail closed.
        let opened = receipt.opened.len() == 1 && aliases.contains(&receipt.opened[0].as_str());
        let raised = receipt.raised.len() == 1 && aliases.contains(&receipt.raised[0].as_str());
        let success = receipt.opened.len() + receipt.raised.len() + receipt.unopened.len() == 1
            && (opened || raised);
        // Reconcile only this operation. Other attempts on either identity
        // cannot conceal its handoff or recreate it through a late receipt.
        restored
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .complete(reservation, success);
        // Count only receipts for the dispatched target, once. A missing or
        // contradictory receipt cannot claim restoration or extend suppression.
        if success && opened {
            outcome.opened.push(resource.clone());
        } else if success {
            outcome.raised.push(resource.clone());
        } else {
            outcome.unopened.push((
                resource.clone(),
                receipt
                    .unopened
                    .iter()
                    .find(|(r, _)| aliases.contains(&r.as_str()))
                    .map(|(_, reason)| reason.clone())
                    .unwrap_or_else(|| {
                        "executor returned no unique successful receipt for target".into()
                    }),
            ));
        }
    }
    if !outcome.any_action() {
        return VoiceOutcome::Failed {
            reason: format!(
                "Nothing restored. {} {}",
                outcome.summary(),
                outcome
                    .unopened
                    .iter()
                    .map(|(_, reason)| reason.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        };
    }
    VoiceOutcome::Restored {
        title: candidate(b, facts).title,
        summary: format!(
            "{} OS dispatch receipts do not guarantee restored application state.",
            outcome.summary()
        ),
        // The engine already caps the neighborhood. Every member has an
        // outcome (including reservation refusals); no extra historical offers.
        offered: vec![],
    }
}

/// Encodes a [`VoiceOutcome`] as the line protocol the desktop parses.
fn encode_outcome(outcome: &VoiceOutcome) -> String {
    let mut out = String::new();
    match outcome {
        VoiceOutcome::Restored {
            title,
            summary,
            offered,
        } => {
            out.push_str("OUTCOME RESTORED\n");
            out.push_str(&format!("TITLE {}\n", sanitize_line(title)));
            out.push_str(&format!("SUMMARY {}\n", sanitize_line(summary)));
            for resource in offered {
                out.push_str(&format!("OFFERED {}\n", sanitize_line(resource)));
            }
        }
        VoiceOutcome::Ambiguous { candidates } => {
            out.push_str("OUTCOME AMBIGUOUS\n");
            for c in candidates {
                match &c.selection {
                    VoiceSelection::Work => out.push_str(&format!(
                        "CANDIDATE {}\t{}\n",
                        sanitize_line(&c.work_id),
                        sanitize_line(&c.title)
                    )),
                    VoiceSelection::Occurrence { key } => out.push_str(&format!(
                        "CANDIDATE {}\t{}\tOCCURRENCE\t{}\n",
                        sanitize_line(&c.work_id),
                        sanitize_line(&c.title),
                        sanitize_line(key)
                    )),
                }
            }
        }
        VoiceOutcome::NotFound { recent } => {
            out.push_str("OUTCOME NOTFOUND\n");
            for c in recent {
                out.push_str(&format!(
                    "RECENT {}\t{}\n",
                    sanitize_line(&c.work_id),
                    sanitize_line(&c.title)
                ));
            }
        }
        VoiceOutcome::Failed { reason } => {
            out.push_str("OUTCOME FAILED\n");
            out.push_str(&format!("REASON {}\n", sanitize_line(reason)));
        }
        VoiceOutcome::NoCommand => {
            out.push_str("OUTCOME NOCOMMAND\n");
        }
        VoiceOutcome::Brief { count, narration } => {
            out.push_str(&format!("OUTCOME BRIEF\nCOUNT {count}\n"));
            out.push_str(&format!("NARRATION {}\n", sanitize_line(narration)));
        }
    }
    out.push_str("END\n");
    out
}

/// One line, no tabs or newlines: the protocol is line-oriented.
fn sanitize_line(s: &str) -> String {
    s.replace(['\n', '\r', '\t'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_observation::observation_language::ObservationConcept;
    use evo_semantic::EmbedError;
    use evo_storage::{Storage, StorageObjectKind};

    struct History(PathBuf);

    impl History {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "evo-voice-{}-{}-{}",
                std::process::id(),
                crate::threads::milliseconds(std::time::SystemTime::now()),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn persist(&self, seconds: u64, concept: ObservationConcept) {
            self.persist_with_origin(seconds, concept, false);
        }

        fn persist_with_origin(&self, seconds: u64, concept: ObservationConcept, execution: bool) {
            self.persist_context(seconds, concept, Default::default(), execution);
        }

        fn persist_context(
            &self,
            seconds: u64,
            concept: ObservationConcept,
            context: std::collections::HashMap<String, String>,
            execution: bool,
        ) {
            use evo_observation::accept::accept;
            use evo_observation::candidate::CandidateObservation;
            use evo_observation::provenance::{ObservationSource, Provenance};
            let schema = concept.schema();
            let provenance = Provenance::new(
                ObservationSource::new("voice-test").unwrap(),
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds),
                context,
            );
            let mut observation = accept(
                CandidateObservation::new(schema.clone(), provenance, concept.evidence()),
                &schema,
            )
            .unwrap();
            if execution {
                observation = observation.with_execution_origin();
            }
            self.append(crate::persistence::encode_observation_record(&observation).as_bytes());
        }

        fn append(&self, bytes: &[u8]) {
            let _guard = Storage::with_thread_root(self.0.clone());
            Storage::new()
                .append(StorageObjectKind::Observation, bytes)
                .unwrap();
        }

        fn work(&self, at: u64, resource: &str, name: &str) {
            self.persist(
                at,
                ObservationConcept::WindowFocusGained {
                    subject: resource.into(),
                },
            );
            self.persist(
                at + 10,
                ObservationConcept::InputActivity {
                    subject: resource.into(),
                    keys: 150,
                    clicks: 0,
                    scrolls: 0,
                },
            );
            self.persist(
                at + 20,
                ObservationConcept::SurfaceNamed {
                    subject: resource.into(),
                    name: name.into(),
                },
            );
        }

        fn two_works() -> Self {
            let history = Self::new();
            history.work(100, "https://example.test/p/9fae", "Lantern notebook");
            history.work(4000, "https://example.test/p/17bd", "Prism notebook");
            history
        }
    }

    impl Drop for History {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    struct StubEmbedder(BTreeMap<String, Vec<f32>>);

    impl StubEmbedder {
        fn for_facts(facts: &VoiceFacts) -> Self {
            let mut vectors = BTreeMap::new();
            for (position, (_, text)) in facts.documents.iter().enumerate() {
                let mut vector = vec![0.0; facts.documents.len() + 1];
                vector[position] = 1.0;
                vectors.insert(text.clone(), vector);
            }
            Self(vectors)
        }

        fn query_for(&mut self, facts: &VoiceFacts, query: &str, id: u64) {
            let document = &facts
                .documents
                .iter()
                .find(|(tid, _)| *tid == id)
                .unwrap()
                .1;
            self.0.insert(query.into(), self.0[document].clone());
        }
    }

    impl Embedder for StubEmbedder {
        fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
            self.0
                .get(text)
                .cloned()
                .ok_or_else(|| EmbedError::Refused(text.into()))
        }
    }

    struct UncalibratedEmbedder(StubEmbedder);

    impl Embedder for UncalibratedEmbedder {
        fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
            self.0.embed(text)
        }

        fn allows_similarity_restore(&self) -> bool {
            false
        }
    }

    #[test]
    fn uncalibrated_multilingual_similarity_suggests_but_never_dispatches() {
        let (_history, facts, id) = scope_fixture();
        let id: u64 = id.parse().unwrap();
        let mut stub = StubEmbedder::for_facts(&facts);
        stub.query_for(&facts, "synthetic multilingual query", id);
        let outcome = trigger_from_facts(
            "synthetic multilingual query",
            &facts,
            &UncalibratedEmbedder(stub),
            &registry(),
            never_restore,
        );
        assert!(
            matches!(outcome, VoiceOutcome::Ambiguous { candidates } if candidates.len() == 1 && candidates[0].work_id == id.to_string())
        );
    }

    #[test]
    fn uncalibrated_scope_refuses_fuzzy_query_but_preserves_exact_selection() {
        let (history, facts, id) = scope_fixture();
        let embedder = UncalibratedEmbedder(StubEmbedder::for_facts(&facts));
        let outcome = wire_request(
            &history,
            &scope_request(&id, "just the notes"),
            &registry(),
            Some(&embedder),
            never_restore,
        );
        assert!(outcome.contains("not calibrated"), "{outcome}");

        let locator = "file:///work/café notes.md";
        let mut attempted = Vec::new();
        let outcome = wire_request(
            &history,
            &scope_request(&id, locator),
            &registry(),
            Some(&embedder),
            |targets| {
                attempted.extend(targets.iter().map(|target| target.resource.clone()));
                opened(targets)
            },
        );
        assert!(outcome.starts_with("OUTCOME RESTORED"), "{outcome}");
        assert_eq!(attempted, vec![locator]);
    }

    fn registry() -> Arc<Mutex<RestoredSubjects>> {
        Arc::new(Mutex::new(RestoredSubjects::new()))
    }

    fn never_restore(_: &[RestoreTarget]) -> RestoreOutcome {
        panic!("this request must never reach execution")
    }

    fn opened(targets: &[RestoreTarget]) -> RestoreOutcome {
        RestoreOutcome {
            opened: targets.iter().map(|t| t.resource.clone()).collect(),
            ..Default::default()
        }
    }

    fn wire_request(
        history: &History,
        request: &[u8],
        restored: &Arc<Mutex<RestoredSubjects>>,
        embedder: Option<&dyn Embedder>,
        restore: impl FnMut(&[RestoreTarget]) -> RestoreOutcome,
    ) -> String {
        let (mut client, server) = UnixStream::pair().unwrap();
        client.write_all(request).unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        handle_voice_stream(server, &history.0, restored, embedder, restore);
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        response
    }

    fn scope_fixture() -> (History, VoiceFacts, String) {
        let history = History::new();
        history.work(100, "file:///work/café notes.md", "Notes");
        history.work(200, "https://example.test/slides", "Slides");
        history.persist(
            500,
            ObservationConcept::WorkGrouped {
                first: "file:///work/café notes.md".into(),
                second: "https://example.test/slides".into(),
            },
        );
        history.work(8000, "https://example.test/unrelated", "Unrelated");
        let facts = voice_facts(&history.0).unwrap();
        let b = facts
            .briefs
            .iter()
            .find(|b| b.restore_set.contains(&"file:///work/café notes.md".into()))
            .unwrap();
        assert_eq!(b.restore_set.len(), 2);
        let id = b.thread_id.to_string();
        (history, facts, id)
    }

    fn scope_request(id: &str, query: &str) -> Vec<u8> {
        format!("SCOPE {id} {}\n{query}", query.len()).into_bytes()
    }

    fn fixture_surface(window: u64) -> SurfaceIdentity {
        SurfaceIdentity {
            capture_session: "synthetic-capture-1".into(),
            window_id: format!("window-{window}"),
            process_id: 123,
            process_started_at: format!("{:016x}", 1_791_000_000.5f64.to_bits()),
            window_identifier: Some(format!("fixture-window-{window}")),
            browser_window: None,
            browser_profile: None,
            browser_session: None,
            browser_tab: None,
        }
    }

    fn identity_fixture() -> (History, String) {
        let history = History::new();
        history.work(100, "file:///work/café notes.md", "Notes");
        let id = voice_facts(&history.0).unwrap().briefs[0]
            .thread_id
            .to_string();
        (history, id)
    }

    fn persist_surface(
        history: &History,
        at: u64,
        resource: &str,
        surface: SurfaceIdentity,
        execution: bool,
    ) {
        let mut context = std::collections::HashMap::new();
        ObservedState::new()
            .with_surface_identity(surface)
            .write_context(&mut context);
        history.persist_context(
            at,
            ObservationConcept::WindowFocusGained {
                subject: resource.into(),
            },
            context,
            execution,
        );
    }

    #[test]
    fn resource_wire_refuses_stale_exact_locator_without_using_the_model() {
        let (history, _, id) = scope_fixture();
        let locator = "file:///work/removed.md";
        let request = format!("RESOURCE {id} {}\n{locator}", locator.len());
        let response = wire_request(
            &history,
            request.as_bytes(),
            &registry(),
            Some(&NeverEmbed),
            |_| panic!("stale selection must not execute"),
        );
        assert!(response.contains("no longer eligible"), "{response}");
    }

    struct NeverEmbed;
    impl Embedder for NeverEmbed {
        fn embed(&self, _: &str) -> Result<Vec<f32>, evo_semantic::EmbedError> {
            panic!("exact RESOURCE transport must not use similarity");
        }
    }

    #[test]
    fn persisted_identity_reaches_exact_scope_dispatch_from_one_snapshot() {
        let (history, id) = identity_fixture();
        let locator = "file:///work/café notes.md";
        let surface = fixture_surface(1);
        persist_surface(&history, 130, locator, surface.clone(), false);
        let request = format!("RESOURCE {id} {}\n{locator}", locator.len());
        let response = wire_request(&history, request.as_bytes(), &registry(), None, |targets| {
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].resource, locator);
            assert_eq!(targets[0].surface, Some(surface.clone()));
            // Changes after planning cannot replace the selected occurrence.
            persist_surface(&history, 140, locator, fixture_surface(2), false);
            opened(targets)
        });
        assert!(response.starts_with("OUTCOME RESTORED"), "{response}");
    }

    #[test]
    fn work_choice_exposes_distinct_occurrences_and_selected_choice_restores_exact_surface() {
        let (history, id) = identity_fixture();
        let locator = "file:///work/café notes.md";
        persist_surface(&history, 130, locator, fixture_surface(1), false);
        persist_surface(&history, 140, locator, fixture_surface(2), false);
        let facts = voice_facts(&history.0).unwrap();
        let occurrences = facts
            .occurrences
            .get(&(id.parse::<u64>().unwrap(), locator.to_string()))
            .expect("occurrence evidence");
        assert_eq!(occurrences.len(), 2);
        let key = occurrence_key(
            id.parse().unwrap(),
            locator,
            occurrences[0].observed_at,
            &occurrences[0].surface,
        );

        let refused = wire_request(
            &history,
            format!("CHOOSE {id}\n").as_bytes(),
            &registry(),
            None,
            never_restore,
        );
        assert!(refused.starts_with("OUTCOME AMBIGUOUS\n"), "{refused}");
        assert!(refused.contains("OCCURRENCE"), "{refused}");

        let restored = registry();
        let response = wire_request(
            &history,
            format!("CHOOSE_OCCURRENCE {id} {key}\n").as_bytes(),
            &restored,
            None,
            |targets| {
                assert_eq!(targets.len(), 1);
                assert_eq!(targets[0].resource, locator);
                assert_eq!(targets[0].surface, Some(occurrences[0].surface.clone()));
                opened(targets)
            },
        );
        assert!(response.starts_with("OUTCOME RESTORED\n"), "{response}");
    }

    #[test]
    fn latest_unidentified_focus_cannot_reuse_an_earlier_occurrence() {
        let (history, id) = identity_fixture();
        let locator = "file:///work/café notes.md";
        persist_surface(&history, 130, locator, fixture_surface(1), false);
        // This may be another window on the same document. Missing capture
        // identity is not evidence that the earlier occurrence stayed selected.
        history.persist(
            140,
            ObservationConcept::WindowFocusGained {
                subject: locator.into(),
            },
        );
        let restored = registry();
        for request in [
            format!("RESOURCE {id} {}\n{locator}", locator.len()).into_bytes(),
            scope_request(&id, locator),
            format!("CHOOSE {id}\n").into_bytes(),
        ] {
            let response = wire_request(&history, &request, &restored, None, |_| {
                panic!("latest unidentified occurrence must not select an older window")
            });
            assert!(response.starts_with("OUTCOME FAILED"), "{response}");
            assert!(response.contains("latest focus"), "{response}");
            assert!(!restored.lock().unwrap().is_execution(locator));
            assert!(!restored.lock().unwrap().is_execution("/work/café notes.md"));
        }
        let facts = voice_facts(&history.0).unwrap();
        let mut embedder = StubEmbedder::for_facts(&facts);
        embedder.query_for(&facts, "selected notes", id.parse().unwrap());
        let response = wire_request(
            &history,
            b"TRIGGER 14\nselected notes",
            &restored,
            Some(&embedder),
            never_restore,
        );
        assert!(response.starts_with("OUTCOME FAILED"), "{response}");
        assert!(response.contains("latest focus"), "{response}");
    }

    #[test]
    fn later_identity_can_resolve_missing_focus_without_changing_legacy_selection() {
        let (history, id) = identity_fixture();
        let locator = "file:///work/café notes.md";
        let request = format!("RESOURCE {id} {}\n{locator}", locator.len());
        // Entirely identity-free history retains its original locator-only
        // contract. This fix does not turn legacy inference into authority.
        let response = wire_request(&history, request.as_bytes(), &registry(), None, |targets| {
            assert_eq!(targets[0].surface, None);
            opened(targets)
        });
        assert!(response.starts_with("OUTCOME RESTORED"), "{response}");
        let surface = fixture_surface(1);
        persist_surface(&history, 130, locator, surface.clone(), false);
        history.persist(
            140,
            ObservationConcept::WindowFocusGained {
                subject: locator.into(),
            },
        );
        persist_surface(&history, 150, locator, surface.clone(), false);
        let response = wire_request(&history, request.as_bytes(), &registry(), None, |targets| {
            assert_eq!(targets[0].surface, Some(surface.clone()));
            opened(targets)
        });
        assert!(response.starts_with("OUTCOME RESTORED"), "{response}");
    }

    #[test]
    fn malformed_occurrences_refuse_and_ambiguous_occurrences_offer_safe_choices() {
        for malformed in [false, true] {
            let (history, id) = identity_fixture();
            let locator = "file:///work/café notes.md";
            persist_surface(&history, 130, locator, fixture_surface(1), false);
            if malformed {
                let mut context = std::collections::HashMap::new();
                context.insert("observed_state.surface.window_id".into(), "window-2".into());
                history.persist_context(
                    140,
                    ObservationConcept::WindowFocusGained {
                        subject: locator.into(),
                    },
                    context,
                    false,
                );
            } else {
                persist_surface(&history, 140, locator, fixture_surface(2), false);
            }
            let request = format!("RESOURCE {id} {}\n{locator}", locator.len());
            let response = wire_request(&history, request.as_bytes(), &registry(), None, |_| {
                panic!("uncertain occurrence cannot execute")
            });
            if malformed {
                assert!(response.starts_with("OUTCOME FAILED"), "{response}");
                assert!(response.contains("malformed occurrence"), "{response}");
            } else {
                assert!(response.starts_with("OUTCOME AMBIGUOUS"), "{response}");
                assert!(response.contains("OCCURRENCE"), "{response}");
                assert!(response.contains("occurrence 1"), "{response}");
                assert!(response.contains("occurrence 2"), "{response}");
            }
        }
    }

    #[test]
    fn execution_origin_occurrence_cannot_replace_human_restore_identity() {
        let (history, id) = identity_fixture();
        let locator = "file:///work/café notes.md";
        persist_surface(&history, 130, locator, fixture_surface(1), false);
        persist_surface(&history, 140, locator, fixture_surface(2), true);
        // Execution-open resources are withheld until real person contact.
        history.persist(
            150,
            ObservationConcept::InputActivity {
                subject: locator.into(),
                keys: 1,
                clicks: 0,
                scrolls: 0,
            },
        );
        // The handoff can begin a fresh episode. Its occurrence must be
        // witnessed again, rather than borrowing another episode's identity.
        persist_surface(&history, 151, locator, fixture_surface(1), false);
        let request = format!("RESOURCE {id} {}\n{locator}", locator.len());
        let response = wire_request(&history, request.as_bytes(), &registry(), None, |targets| {
            assert_eq!(targets[0].surface, Some(fixture_surface(1)));
            opened(targets)
        });
        assert!(response.starts_with("OUTCOME RESTORED"), "{response}");
    }

    #[test]
    fn scope_without_model_preserves_exact_trailing_space_locator() {
        let history = History::new();
        let resource = "file:///work/notes.md ";
        history.work(100, resource, "Notes");
        let facts = voice_facts(&history.0).unwrap();
        let id = facts
            .briefs
            .iter()
            .find(|b| b.restore_set.contains(&resource.to_string()))
            .unwrap()
            .thread_id
            .to_string();
        let mut attempted = Vec::new();
        let response = wire_request(
            &history,
            &scope_request(&id, resource),
            &registry(),
            None,
            |targets| {
                attempted.push(targets[0].resource.clone());
                opened(targets)
            },
        );
        assert_eq!(attempted, vec![resource]);
        assert!(response.starts_with("OUTCOME RESTORED\n"), "{response}");
    }

    #[test]
    fn scope_wire_exact_locator_needs_no_model_and_file_receipt_uses_execution_alias() {
        let (history, facts, id) = scope_fixture();
        let restored = registry();
        let resource = "file:///work/café notes.md";
        let locator = "/work/café notes.md";
        let mut attempted = Vec::new();
        let response = wire_request(
            &history,
            &scope_request(&id, resource),
            &restored,
            None,
            |targets| {
                assert_eq!(targets, &[RestoreTarget::with_default_handler(resource)]);
                let mut registry = restored.try_lock().expect("no mutex across effects");
                assert!(registry.is_execution(resource));
                assert!(registry.is_execution(locator));
                assert!(!registry.is_execution("https://example.test/slides"));
                attempted.push(resource.to_string());
                RestoreOutcome {
                    opened: vec![locator.into()],
                    ..Default::default()
                }
            },
        );
        assert_eq!(attempted, vec![resource]);
        assert!(response.starts_with("OUTCOME RESTORED\n"), "{response}");
        assert!(response.contains("SUMMARY Opened 1, raised 0 already open."));
        assert!(!response.contains("OFFERED"));
        assert!(restored.lock().unwrap().is_execution(resource));
        assert!(restored.lock().unwrap().is_execution(locator));
        // Exact scope from another work must not escape the selected bundle.
        let response = wire_request(
            &history,
            &scope_request(&id, "https://example.test/unrelated"),
            &restored,
            None,
            never_restore,
        );
        assert!(response.starts_with("OUTCOME FAILED\n"));
        let mut all = Vec::new();
        let response = wire_request(
            &history,
            format!("CHOOSE {id}\n").as_bytes(),
            &registry(),
            None,
            |targets| {
                all.push(targets[0].resource.clone());
                opened(targets)
            },
        );
        assert!(response.starts_with("OUTCOME RESTORED\n"));
        assert_eq!(all, facts.bundles[&id.parse::<u64>().unwrap()].restore_set);
    }

    #[test]
    fn scope_wire_semantics_never_widen_ambiguous_unmatched_or_failed_queries() {
        let (history, facts, id) = scope_fixture();
        let bundle = &facts.bundles[&id.parse::<u64>().unwrap()];
        // Only selected-bundle resources have vectors. Embedding another work
        // would error, exposing a cross-work retrieval regression.
        let mut vectors = BTreeMap::new();
        for (index, resource) in bundle.restore_set.iter().enumerate() {
            let mut vector = vec![0.0; 3];
            vector[index] = 1.0;
            vectors.insert(resource.clone(), vector);
        }
        vectors.insert(
            "one surface".into(),
            vectors[&bundle.restore_set[0]].clone(),
        );
        vectors.insert("either".into(), vec![1.0, 1.0, 0.0]);
        vectors.insert("xyzzy plugh".into(), vec![0.0, 0.0, 1.0]);
        let embedder = StubEmbedder(vectors);
        let mut attempted = Vec::new();
        let response = wire_request(
            &history,
            &scope_request(&id, "one surface"),
            &registry(),
            Some(&embedder),
            |targets| {
                attempted.push(targets[0].resource.clone());
                opened(targets)
            },
        );
        assert!(response.starts_with("OUTCOME RESTORED\n"), "{response}");
        assert_eq!(attempted, vec![bundle.restore_set[0].clone()]);
        for query in ["either", "xyzzy plugh", "model-error"] {
            let response = wire_request(
                &history,
                &scope_request(&id, query),
                &registry(),
                Some(&embedder),
                never_restore,
            );
            assert!(
                response.starts_with("OUTCOME FAILED\n"),
                "{query}: {response}"
            );
            assert!(
                response.contains("exact locator") || response.contains("exact resource locator")
            );
            assert!(!response.contains("CANDIDATE "));
        }
    }

    #[test]
    fn scope_wire_rejects_malformed_requests_before_history_model_or_effects() {
        let history = History::new();
        history.append(b"broken history must not be read for malformed input");
        let embedder = StubEmbedder(BTreeMap::new());
        for request in [
            "SCOPE 01 1\nx",
            "SCOPE +1 1\nx",
            "SCOPE x 1\nx",
            "SCOPE 18446744073709551616 1\nx",
            "SCOPE 1 0\n",
            "SCOPE 1 -1\n",
            "SCOPE 1 4097\n",
            "SCOPE 1 01\nx",
            "SCOPE 1 1 extra\nx",
            "SCOPE 1  1\nx",
            "SCOPE 1 1 \nx",
            "SCOPE 1 10\nx",
            "SCOPE 1 3\n \t\n",
        ] {
            let response = wire_request(
                &history,
                request.as_bytes(),
                &registry(),
                Some(&embedder),
                never_restore,
            );
            assert!(
                response.contains("Expected SCOPE"),
                "{request:?}: {response}"
            );
        }
        let response = wire_request(
            &history,
            b"SCOPE 1 1\n\xff",
            &registry(),
            Some(&embedder),
            never_restore,
        );
        assert!(response.contains("Expected SCOPE"));
        let history = History::two_works();
        let response = wire_request(
            &history,
            b"SCOPE 999999 1\nx",
            &registry(),
            Some(&embedder),
            never_restore,
        );
        assert!(response.contains("unknown work id"));
    }

    #[test]
    fn scope_execution_uses_the_same_snapshot_that_was_matched() {
        struct ChangingScope<'a>(&'a History);
        impl Embedder for ChangingScope<'_> {
            fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
                if text == "query" {
                    std::fs::write(self.0.0.join("observation.log"), b"").unwrap();
                }
                Ok(if text == "https://example.test/slides" {
                    vec![0.0, 1.0]
                } else {
                    vec![1.0, 0.0]
                })
            }
        }
        let (history, _, id) = scope_fixture();
        let response = wire_request(
            &history,
            &scope_request(&id, "query"),
            &registry(),
            Some(&ChangingScope(&history)),
            |targets| {
                assert_eq!(
                    targets,
                    &[RestoreTarget::with_default_handler(
                        "file:///work/café notes.md"
                    )]
                );
                opened(targets)
            },
        );
        assert!(response.starts_with("OUTCOME RESTORED\n"), "{response}");
        assert!(voice_facts(&history.0).unwrap().briefs.is_empty());
    }

    #[test]
    fn file_receipts_accept_only_one_exact_original_or_execution_locator() {
        let (_history, facts, id) = scope_fixture();
        let b = facts
            .briefs
            .iter()
            .find(|b| b.thread_id.to_string() == id)
            .unwrap();
        for resource in ["file:///work/a b.md", "file:///work/a%20b.md"] {
            let LocatorKind::FilePath(locator) = classify_subject(resource) else {
                unreachable!()
            };
            for reported in [resource, locator.as_str()] {
                for raised in [false, true] {
                    let restored = registry();
                    let outcome = restore_resources(
                        b,
                        &facts,
                        &[resource.into()],
                        &restored,
                        |_| {
                            let mut registry = restored.try_lock().unwrap();
                            assert!(registry.is_execution(resource));
                            assert!(registry.is_execution(&locator));
                            if raised {
                                RestoreOutcome {
                                    raised: vec![reported.into()],
                                    ..Default::default()
                                }
                            } else {
                                RestoreOutcome {
                                    opened: vec![reported.into()],
                                    ..Default::default()
                                }
                            }
                        },
                        None,
                    );
                    assert!(matches!(outcome, VoiceOutcome::Restored { .. }));
                }
            }
            for receipt in [
                RestoreOutcome {
                    opened: vec!["alien".into()],
                    ..Default::default()
                },
                RestoreOutcome {
                    opened: vec![resource.into(), locator.clone()],
                    ..Default::default()
                },
                RestoreOutcome {
                    opened: vec![locator.clone(), locator.clone()],
                    ..Default::default()
                },
                RestoreOutcome {
                    opened: vec![resource.into()],
                    raised: vec![locator.clone()],
                    ..Default::default()
                },
                RestoreOutcome {
                    opened: vec![locator.clone()],
                    unopened: vec![(resource.into(), "failed".into())],
                    ..Default::default()
                },
                RestoreOutcome {
                    opened: vec![locator.clone(), "alien".into()],
                    ..Default::default()
                },
            ] {
                let restored = registry();
                let outcome = restore_resources(
                    b,
                    &facts,
                    &[resource.into()],
                    &restored,
                    |_| receipt.clone(),
                    None,
                );
                assert!(
                    matches!(outcome, VoiceOutcome::Failed { .. }),
                    "{receipt:?}"
                );
                assert!(!restored.lock().unwrap().is_execution(resource));
                assert!(!restored.lock().unwrap().is_execution(&locator));
            }
            if resource.contains("%20") {
                let outcome = restore_resources(
                    b,
                    &facts,
                    &[resource.into()],
                    &registry(),
                    |_| RestoreOutcome {
                        opened: vec!["/work/a b.md".into()],
                        ..Default::default()
                    },
                    None,
                );
                assert!(
                    matches!(outcome, VoiceOutcome::Failed { .. }),
                    "no second URI decoder"
                );
            }
        }
    }

    #[test]
    fn file_alias_reservations_rollback_together_before_effect_or_after_handoff() {
        let (_history, facts, id) = scope_fixture();
        let b = facts
            .briefs
            .iter()
            .find(|b| b.thread_id.to_string() == id)
            .unwrap();
        let resource = "file:///work/a b.md";
        let locator = "/work/a b.md";
        let restored = registry();
        // At capacity neither alias is installed and dispatch is refused.
        let mut held = Vec::new();
        {
            let mut registry = restored.lock().unwrap();
            while let Some(reservation) = registry.reserve("unrelated") {
                held.push(reservation);
            }
        }
        let outcome = restore_resources(
            b,
            &facts,
            &[resource.into()],
            &restored,
            never_restore,
            None,
        );
        assert!(matches!(outcome, VoiceOutcome::Failed { .. }));
        assert!(!restored.lock().unwrap().is_execution(resource));
        assert!(!restored.lock().unwrap().is_execution(locator));
        assert!(restored.lock().unwrap().is_execution("unrelated"));
        // One free operation slot accommodates the entire two-identity target.
        restored
            .lock()
            .unwrap()
            .complete(held.pop().unwrap(), false);
        let outcome = restore_resources(b, &facts, &[resource.into()], &restored, opened, None);
        assert!(matches!(outcome, VoiceOutcome::Restored { .. }));
        // Handoff after successful completion also clears the pair, not just
        // the exact identity on which input happened.
        restored.lock().unwrap().handoff(locator);
        assert!(!restored.lock().unwrap().is_execution(resource));
        assert!(!restored.lock().unwrap().is_execution(locator));
        assert!(restored.lock().unwrap().is_execution("unrelated"));
        for handoff in [resource, locator] {
            let restored = registry();
            let mut fresh = None;
            let outcome = restore_resources(
                b,
                &facts,
                &[resource.into()],
                &restored,
                |_| {
                    let mut registry = restored.try_lock().unwrap();
                    registry.handoff(handoff);
                    fresh = Some(registry.reserve_aliases(&[resource, locator]).unwrap());
                    RestoreOutcome {
                        raised: vec![locator.into()],
                        ..Default::default()
                    }
                },
                None,
            );
            assert!(matches!(outcome, VoiceOutcome::Restored { .. }));
            // Removing the fresh attempt reveals whether the old completion
            // recreated/retained anything after its handoff.
            restored.lock().unwrap().complete(fresh.unwrap(), false);
            assert!(!restored.lock().unwrap().is_execution(resource));
            assert!(!restored.lock().unwrap().is_execution(locator));
        }
    }

    fn assert_restored(outcome: VoiceOutcome, expected_title: &str) {
        match outcome {
            VoiceOutcome::Restored { title, offered, .. } => {
                assert_eq!(title, expected_title);
                assert!(
                    offered.is_empty(),
                    "attempted targets are never offered again"
                );
            }
            other => panic!("expected restore, got {other:?}"),
        }
    }

    #[test]
    fn persisted_execution_is_audit_only_in_home_and_voice_without_history_mutation() {
        let history = History::new();
        for (at, concept) in [
            (
                100,
                ObservationConcept::WindowFocusGained {
                    subject: "https://example.test/restored".into(),
                },
            ),
            (
                110,
                ObservationConcept::InputActivity {
                    subject: "https://example.test/restored".into(),
                    keys: 300,
                    clicks: 0,
                    scrolls: 0,
                },
            ),
            (
                120,
                ObservationConcept::FileSaved {
                    subject: "https://example.test/restored".into(),
                },
            ),
            (
                800,
                ObservationConcept::URLNavigated {
                    subject: "https://example.test/other".into(),
                },
            ),
        ] {
            history.persist_with_origin(at, concept, true);
        }
        let path = history.0.join("observation.log");
        let before = std::fs::read(&path).unwrap();
        let home = crate::threads::engine_from_storage(&history.0).unwrap();
        let voice = voice_facts(&history.0).unwrap();
        assert!(home.threads().is_empty(), "restore cannot mint work");
        assert_eq!(voice.briefs, home.briefs());
        assert_eq!(std::fs::read(path).unwrap(), before);
    }

    #[test]
    fn execution_declaration_cannot_merge_persisted_human_works() {
        let history = History::two_works();
        let before = voice_facts(&history.0).unwrap().briefs;
        assert_eq!(before.len(), 2);
        history.persist_with_origin(
            4100,
            ObservationConcept::WorkGrouped {
                first: "https://example.test/p/9fae".into(),
                second: "https://example.test/p/17bd".into(),
            },
            true,
        );
        let bytes = std::fs::read(history.0.join("observation.log")).unwrap();
        let home = crate::threads::engine_from_storage(&history.0).unwrap();
        let voice = voice_facts(&history.0).unwrap();
        assert_eq!(
            home.briefs(),
            before,
            "execution declarations have no authority"
        );
        assert_eq!(voice.briefs, home.briefs());
        assert_eq!(
            std::fs::read(history.0.join("observation.log")).unwrap(),
            bytes
        );
    }

    #[test]
    fn failed_missing_and_contradictory_receipts_do_not_claim_restored() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        let b = &facts.briefs[0];
        for mode in ["failed", "missing", "wrong-target", "contradictory"] {
            let restored = registry();
            let outcome = restore_from_facts(
                &b.thread_id.to_string(),
                &facts,
                &restored,
                |targets| {
                    let resource = &targets[0].resource;
                    assert!(restored.try_lock().unwrap().is_execution(resource));
                    match mode {
                        "failed" => RestoreOutcome {
                            unopened: vec![(resource.clone(), "refused".into())],
                            ..Default::default()
                        },
                        "wrong-target" => RestoreOutcome {
                            opened: vec!["other".into()],
                            ..Default::default()
                        },
                        "contradictory" => RestoreOutcome {
                            opened: vec![resource.clone()],
                            unopened: vec![(resource.clone(), "refused".into())],
                            ..Default::default()
                        },
                        _ => RestoreOutcome::default(),
                    }
                },
                None,
            );
            assert!(
                matches!(outcome, VoiceOutcome::Failed { .. }),
                "{mode}: {outcome:?}"
            );
            for resource in &b.restore_set {
                assert!(!restored.lock().unwrap().is_execution(resource));
            }
            assert!(!restored.lock().unwrap().is_execution("other"));
        }
    }

    #[test]
    fn overlapping_voice_failure_preserves_success_and_handoff_survives_late_receipt() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        let id = facts.briefs[0].thread_id.to_string();
        let restored = registry();
        let outcome = restore_from_facts(
            &id,
            &facts,
            &restored,
            |targets| {
                // An overlapping operation completes while the first is pending.
                assert_restored(
                    restore_from_facts(&id, &facts, &restored, opened, None),
                    &candidate(&facts.briefs[0], &facts).title,
                );
                RestoreOutcome {
                    unopened: vec![(targets[0].resource.clone(), "failed first operation".into())],
                    ..Default::default()
                }
            },
            None,
        );
        assert!(matches!(outcome, VoiceOutcome::Failed { .. }));
        let resource = &facts.briefs[0].restore_set[0];
        assert!(restored.lock().unwrap().is_execution(resource));
        let outcome = restore_from_facts(
            &id,
            &facts,
            &restored,
            |targets| {
                // Capture gets real input before this dispatch returns successfully.
                restored.try_lock().unwrap().handoff(&targets[0].resource);
                opened(targets)
            },
            None,
        );
        assert_restored(outcome, &candidate(&facts.briefs[0], &facts).title);
        assert!(!restored.lock().unwrap().is_execution(resource));
    }

    #[test]
    fn snapshot_matches_home_replay_and_retains_dormant_work() {
        let history = History::two_works();
        // Advance the engine clock without adding a work: the old thread
        // becomes dormant, and Home's age filter would hide both works.
        history.persist(
            8_000_000,
            ObservationConcept::SurfaceNamed {
                subject: "https://example.test/p/9fae".into(),
                name: "Lantern notebook".into(),
            },
        );
        let home = crate::threads::engine_from_storage(&history.0).unwrap();
        let facts = voice_facts(&history.0).unwrap();
        assert_eq!(facts.briefs.len(), 2);
        assert_eq!(facts.briefs, home.briefs());
        assert!(
            facts
                .briefs
                .iter()
                .any(|b| b.status == evo_threads::Status::Dormant)
        );
        assert!(crate::threads::display_threads(&home).is_empty());
        let id = facts.briefs[0].thread_id;
        let mut embedder = StubEmbedder::for_facts(&facts);
        embedder.query_for(&facts, "old notebook", id);
        let outcome =
            trigger_from_facts("old notebook", &facts, &embedder, &registry(), |targets| {
                assert_eq!(
                    targets
                        .iter()
                        .map(|t| t.resource.clone())
                        .collect::<Vec<_>>(),
                    home.resume_bundle(id).restore_set
                );
                opened(targets)
            });
        assert_restored(outcome, &candidate(&facts.briefs[0], &facts).title);
    }

    #[test]
    fn all_requests_share_canonical_ids_and_names_without_legacy_knowledge() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        assert_eq!(facts.briefs.len(), 2);
        let restored = registry();
        let mut embedder = StubEmbedder::for_facts(&facts);
        // The request seam takes no VoiceKnowledge, and only TRIGGER needs a model.
        let ask = handle_voice_request(
            "ASK",
            &mut &b""[..],
            &history.0,
            &restored,
            None,
            never_restore,
        );
        match ask {
            VoiceOutcome::Brief { count, narration } => {
                assert_eq!(count, 2);
                assert!(narration.contains("Lantern notebook"));
                assert!(narration.contains("Prism notebook"));
            }
            other => panic!("{other:?}"),
        }
        for b in &facts.briefs {
            let c = candidate(b, &facts);
            let expected = &b.restore_set;
            let document = &facts
                .documents
                .iter()
                .find(|(id, _)| *id == b.thread_id)
                .unwrap()
                .1;
            assert!(
                document.starts_with(&c.title),
                "witnessed words precede opaque locators"
            );
            assert!(document.contains(&b.anchors[0].resource));
            let outcome = handle_voice_request(
                &format!("BRIEF {}", c.work_id),
                &mut &b""[..],
                &history.0,
                &restored,
                None,
                never_restore,
            );
            assert!(
                matches!(outcome, VoiceOutcome::Brief { count: 1, narration } if narration.contains(&c.title))
            );
            let outcome = handle_voice_request(
                &format!("CHOOSE {}", c.work_id),
                &mut &b""[..],
                &history.0,
                &restored,
                None,
                |targets| {
                    assert_eq!(
                        targets
                            .iter()
                            .map(|t| t.resource.clone())
                            .collect::<Vec<_>>(),
                        *expected
                    );
                    RestoreOutcome {
                        opened: expected.clone(),
                        ..Default::default()
                    }
                },
            );
            assert_restored(outcome, &c.title);
            embedder.query_for(&facts, &c.title, b.thread_id);
            let outcome = handle_voice_request(
                &format!("TRIGGER {}", c.title.len()),
                &mut c.title.as_bytes(),
                &history.0,
                &restored,
                Some(&embedder),
                |targets| {
                    assert_eq!(
                        targets
                            .iter()
                            .map(|t| t.resource.clone())
                            .collect::<Vec<_>>(),
                        *expected
                    );
                    RestoreOutcome {
                        raised: expected.clone(),
                        ..Default::default()
                    }
                },
            );
            assert_restored(outcome, &c.title);
            for resource in expected {
                assert!(restored.lock().unwrap().is_execution(resource));
            }
        }
    }

    #[test]
    fn trigger_boundary_never_resolves_wake_or_placeholder_text() {
        let history = History::new();
        for text in ["hey evo", "[BLANK_AUDIO]", "hey evo [ Inaudible ]"] {
            let header = format!("TRIGGER {}", text.len());
            let outcome = handle_voice_request(
                &header,
                &mut text.as_bytes(),
                &history.0,
                &registry(),
                None,
                never_restore,
            );
            assert_eq!(
                outcome,
                VoiceOutcome::NoCommand,
                "{text:?} crossed the boundary"
            );
        }
    }

    #[test]
    fn ambiguous_and_below_floor_queries_keep_canonical_candidates() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        let mut embedder = StubEmbedder::for_facts(&facts);
        embedder
            .0
            .insert("either notebook".into(), vec![1.0, 1.0, 0.0]);
        embedder.0.insert("unrelated".into(), vec![0.0, 0.0, 1.0]);
        let ambiguous = trigger_from_facts(
            "either notebook",
            &facts,
            &embedder,
            &registry(),
            never_restore,
        );
        let candidates = match ambiguous {
            VoiceOutcome::Ambiguous { candidates } => candidates,
            other => panic!("must ask, not guess: {other:?}"),
        };
        assert_eq!(candidates.len(), 2);
        for c in &candidates {
            assert!(facts.briefs.iter().any(|b| candidate(b, &facts) == *c));
            assert_restored(
                restore_from_facts(&c.work_id, &facts, &registry(), opened, None),
                &c.title,
            );
        }
        // Both scores are zero: below the restore floor, and still nothing is
        // hidden — the nearest works arrive in canonical (tie) order, never
        // as a raw recents list.
        match trigger_from_facts("unrelated", &facts, &embedder, &registry(), never_restore) {
            VoiceOutcome::Ambiguous { candidates } => {
                assert_eq!(
                    candidates,
                    facts
                        .briefs
                        .iter()
                        .map(|b| candidate(b, &facts))
                        .collect::<Vec<_>>()
                );
            }
            other => panic!("a below-floor trigger must ask with nearest works: {other:?}"),
        }
    }

    /// A vague phrase can sit above the old verbatim-calibrated floor (0.25)
    /// and still be no basis for opening a work unasked: cosine 0.36 to the
    /// first work, 0.19 to the second, so the old pair would have restored
    /// the first. Now it asks, nearest first, and never reaches execution.
    #[test]
    fn above_the_match_floor_but_below_the_restore_floor_still_asks() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        let ids = facts.briefs.iter().map(|b| b.thread_id).collect::<Vec<_>>();
        let mut embedder = StubEmbedder::for_facts(&facts);
        let tail = (1.0f32 - 0.36 * 0.36 - 0.19 * 0.19).sqrt();
        embedder.0.insert(
            "that vague thing from earlier".into(),
            vec![0.36, 0.19, tail],
        );
        let outcome = trigger_from_facts(
            "that vague thing from earlier",
            &facts,
            &embedder,
            &registry(),
            never_restore,
        );
        match outcome {
            VoiceOutcome::Ambiguous { candidates } => {
                assert_eq!(
                    candidates
                        .iter()
                        .map(|c| c.work_id.clone())
                        .collect::<Vec<_>>(),
                    ids.iter().map(u64::to_string).collect::<Vec<_>>(),
                    "nearest first, and never a restore"
                );
            }
            other => panic!("below the restore floor the answer must ask: {other:?}"),
        }
    }

    /// Clearly above the restore floor and clearly ahead of the runner-up:
    /// the trigger still restores without asking.
    #[test]
    fn clearly_confident_triggers_still_restore() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        let b = &facts.briefs[0];
        let expected = b.restore_set.clone();
        let mut embedder = StubEmbedder::for_facts(&facts);
        let tail = (1.0f32 - 0.60 * 0.60 - 0.20 * 0.20).sqrt();
        embedder
            .0
            .insert("the lantern notebook".into(), vec![0.60, 0.20, tail]);
        let outcome = trigger_from_facts(
            "the lantern notebook",
            &facts,
            &embedder,
            &registry(),
            |targets| {
                assert_eq!(
                    targets
                        .iter()
                        .map(|t| t.resource.clone())
                        .collect::<Vec<_>>(),
                    expected
                );
                opened(targets)
            },
        );
        assert_restored(outcome, &candidate(b, &facts).title);
    }

    /// Confident but contested: a runner-up inside the margin keeps the
    /// answer a question even above the restore floor (0.55 ≥ 0.85 × 0.60).
    #[test]
    fn a_close_runner_up_still_asks_even_above_the_restore_floor() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        let ids = facts.briefs.iter().map(|b| b.thread_id).collect::<Vec<_>>();
        let mut embedder = StubEmbedder::for_facts(&facts);
        let tail = (1.0f32 - 0.60 * 0.60 - 0.55 * 0.55).sqrt();
        embedder
            .0
            .insert("both lantern things".into(), vec![0.60, 0.55, tail]);
        let outcome = trigger_from_facts(
            "both lantern things",
            &facts,
            &embedder,
            &registry(),
            never_restore,
        );
        match outcome {
            VoiceOutcome::Ambiguous { candidates } => {
                assert_eq!(
                    candidates
                        .iter()
                        .map(|c| c.work_id.clone())
                        .collect::<Vec<_>>(),
                    ids.iter().map(u64::to_string).collect::<Vec<_>>(),
                );
            }
            other => panic!("a close runner-up must stay a question: {other:?}"),
        }
    }

    /// The below-floor carousel shows the nearest few, best first — not the
    /// whole rack.
    #[test]
    fn the_below_floor_carousel_is_capped_at_the_nearest_few() {
        let history = History::new();
        for i in 0..7u64 {
            history.work(
                100 + i * 4000,
                &format!("https://example.test/p/{i:02}"),
                &format!("Work {i:02}"),
            );
        }
        let facts = voice_facts(&history.0).unwrap();
        assert_eq!(
            facts.documents.len(),
            7,
            "the fixture must mint seven works"
        );
        let ids = facts.briefs.iter().map(|b| b.thread_id).collect::<Vec<_>>();
        let mut embedder = StubEmbedder::for_facts(&facts);
        let components = [0.30f32, 0.25, 0.20, 0.15, 0.10, 0.05, 0.01];
        let tail = (1.0 - components.iter().map(|c| c * c).sum::<f32>()).sqrt();
        let mut query = components.to_vec();
        query.push(tail);
        embedder.0.insert("something vague".into(), query);
        let outcome = trigger_from_facts(
            "something vague",
            &facts,
            &embedder,
            &registry(),
            never_restore,
        );
        match outcome {
            VoiceOutcome::Ambiguous { candidates } => {
                assert_eq!(candidates.len(), NEAREST_CANDIDATES);
                assert_eq!(
                    candidates
                        .iter()
                        .map(|c| c.work_id.clone())
                        .collect::<Vec<_>>(),
                    ids.iter()
                        .take(NEAREST_CANDIDATES)
                        .map(u64::to_string)
                        .collect::<Vec<_>>(),
                    "the five nearest, ranked"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    /// An empty rack stays honestly empty: NotFound with no recents, and the
    /// embedder is never even consulted.
    #[test]
    fn a_trigger_on_an_empty_rack_is_not_found_not_a_carousel() {
        let history = History::new();
        let facts = voice_facts(&history.0).unwrap();
        let embedder = StubEmbedder::for_facts(&facts);
        let outcome = trigger_from_facts("anything", &facts, &embedder, &registry(), never_restore);
        assert_eq!(outcome, VoiceOutcome::NotFound { recent: vec![] });
    }

    #[test]
    fn model_failures_never_become_no_match_or_restore() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        let restored = registry();
        let no_model = handle_voice_request(
            "TRIGGER 5",
            &mut &b"query"[..],
            &history.0,
            &restored,
            None,
            never_restore,
        );
        assert!(
            matches!(no_model, VoiceOutcome::Failed { reason } if reason.contains("model unavailable"))
        );
        // Failure while indexing a document.
        let broken = StubEmbedder(BTreeMap::new());
        assert!(
            matches!(trigger_from_facts("query", &facts, &broken, &restored, never_restore),
            VoiceOutcome::Failed { reason } if reason.contains("embedding failed"))
        );
        // Failure while embedding the query, including inconsistent width.
        let mut broken = StubEmbedder::for_facts(&facts);
        assert!(matches!(
            trigger_from_facts("query", &facts, &broken, &restored, never_restore),
            VoiceOutcome::Failed { .. }
        ));
        broken.0.insert("query".into(), vec![1.0]);
        assert!(
            matches!(trigger_from_facts("query", &facts, &broken, &restored, never_restore),
            VoiceOutcome::Failed { reason } if reason.contains("dimensions"))
        );
    }

    #[test]
    fn corrupt_or_unreadable_history_fails_every_request_without_mutating_log() {
        for kind in ["malformed-record", "mid-log", "torn-tail", "unreadable"] {
            let history = History::new();
            let path = history.0.join("observation.log");
            match kind {
                "malformed-record" => history.append(b"not a canonical observation"),
                "mid-log" => std::fs::write(&path, b"not-a-length\n3\nabc\n").unwrap(),
                "torn-tail" => std::fs::write(&path, b"99\nunfinished").unwrap(),
                "unreadable" => std::fs::create_dir(&path).unwrap(),
                _ => unreachable!(),
            }
            let before = std::fs::read(&path).ok();
            for header in ["ASK", "BRIEF 1", "CHOOSE 1", "TRIGGER 5", "SCOPE 1 5"] {
                let outcome = handle_voice_request(
                    header,
                    &mut &b"query"[..],
                    &history.0,
                    &registry(),
                    Some(&StubEmbedder(BTreeMap::new())),
                    never_restore,
                );
                assert!(
                    matches!(outcome, VoiceOutcome::Failed { reason } if reason.contains("history")),
                    "{kind} {header} must report a history failure"
                );
            }
            assert_eq!(
                std::fs::read(&path).ok(),
                before,
                "voice cannot repair a live writer's log"
            );
        }
    }

    #[test]
    fn unknown_legacy_and_noncanonical_ids_never_select_another_thread() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        let id = facts.briefs[0].thread_id;
        for work_id in [
            "unknown".into(),
            "999999999".into(),
            format!("{id:032x}"),
            format!("0{id}"),
            format!("+{id}"),
            format!("thread:{id}"),
        ] {
            for header in [format!("CHOOSE {work_id}"), format!("BRIEF {work_id}")] {
                assert!(
                    matches!(handle_voice_request(&header, &mut &b""[..], &history.0,
                    &registry(), None, never_restore), VoiceOutcome::Failed { reason } if reason.contains("unknown work id"))
                );
            }
        }
    }

    #[test]
    fn empty_restore_set_fails_instead_of_empty_success() {
        let history = History::two_works();
        let mut facts = voice_facts(&history.0).unwrap();
        let id = facts.briefs[0].thread_id;
        facts.briefs[0].restore_set.clear();
        let mut embedder = StubEmbedder::for_facts(&facts);
        embedder.query_for(&facts, "the empty work", id);
        assert!(
            matches!(restore_from_facts(&id.to_string(), &facts, &registry(), never_restore, None),
            VoiceOutcome::Failed { reason } if reason.contains("nothing restorable"))
        );
        assert!(
            matches!(trigger_from_facts("the empty work", &facts, &embedder, &registry(), never_restore),
            VoiceOutcome::Failed { reason } if reason.contains("nothing restorable"))
        );
    }

    #[test]
    fn forked_work_stays_ambiguous_until_explicit_choice() {
        let history = History::two_works();
        let mut facts = voice_facts(&history.0).unwrap();
        let ids = facts.briefs.iter().map(|b| b.thread_id).collect::<Vec<_>>();
        facts.briefs[0].fork = Some(ids.clone());
        let mut embedder = StubEmbedder::for_facts(&facts);
        embedder.query_for(&facts, "best semantic match", ids[0]);
        let outcome = trigger_from_facts(
            "best semantic match",
            &facts,
            &embedder,
            &registry(),
            never_restore,
        );
        match outcome {
            VoiceOutcome::Ambiguous { candidates } => {
                assert_eq!(
                    candidates
                        .iter()
                        .map(|c| c.work_id.clone())
                        .collect::<Vec<_>>(),
                    ids.iter().map(u64::to_string).collect::<Vec<_>>()
                );
            }
            other => panic!("fork must survive a semantic winner: {other:?}"),
        }
        assert!(matches!(narrate_rack(&facts.briefs, &facts.names, 2),
            VoiceOutcome::Brief { narration, .. } if narration.contains("unresolved choice")));
        let outcome = restore_from_facts(
            &ids[0].to_string(),
            &facts,
            &registry(),
            |targets| {
                assert_eq!(
                    targets
                        .iter()
                        .map(|t| t.resource.clone())
                        .collect::<Vec<_>>(),
                    facts.briefs[0].restore_set
                );
                opened(targets)
            },
            None,
        );
        assert_restored(outcome, &candidate(&facts.briefs[0], &facts).title);
    }

    #[test]
    fn exact_resume_set_is_attempted_once_with_no_duplicate_offers() {
        let history = History::two_works();
        let mut facts = voice_facts(&history.0).unwrap();
        let expected = vec![
            "https://example.test/resume".to_string(),
            "A witnessed title".to_string(),
            "file:///work/unavailable.md".to_string(),
        ];
        let b = &mut facts.briefs[0];
        b.restore_set = expected.clone();
        b.companions
            .push("https://example.test/historical-context".into());
        b.contested
            .push("https://example.test/shared-anchor".into());
        b.anchors.push(evo_threads::WorkAnchor {
            resource: "https://example.test/old-anchor".into(),
            kind: evo_threads::AnchorKind::Mutation,
        });
        let id = b.thread_id;
        let mut embedder = StubEmbedder::for_facts(&facts);
        embedder.query_for(&facts, "restore selected work", id);
        for trigger in [false, true] {
            let restored = registry();
            let mut attempted = Vec::new();
            let execute = |targets: &[RestoreTarget]| {
                assert_eq!(targets.len(), 1, "one reserved target per effect");
                let resource = &targets[0].resource;
                assert_eq!(resource, &expected[attempted.len()]);
                attempted.push(resource.clone());
                // try_lock proves the effect is outside the lock; checking here
                // simulates capture while the OS dispatch is still in flight.
                let mut registry = restored.try_lock().expect("no lock across effect");
                assert!(registry.is_execution(resource));
                assert!(!registry.is_execution("https://example.test/old-anchor"));
                if resource == &expected[0] {
                    assert!(
                        !registry.is_execution(&expected[1]),
                        "not reserved prematurely"
                    );
                    opened(targets)
                } else if resource == &expected[1] {
                    RestoreOutcome {
                        raised: vec![resource.clone()],
                        ..Default::default()
                    }
                } else {
                    RestoreOutcome {
                        unopened: vec![(resource.clone(), "not available".into())],
                        ..Default::default()
                    }
                }
            };
            let outcome = if trigger {
                trigger_from_facts(
                    "restore selected work",
                    &facts,
                    &embedder,
                    &restored,
                    execute,
                )
            } else {
                restore_from_facts(&id.to_string(), &facts, &restored, execute, None)
            };
            match outcome {
                VoiceOutcome::Restored {
                    summary, offered, ..
                } => {
                    assert_eq!(
                        summary,
                        "Opened 1, raised 1 already open; 1 could not be opened. OS dispatch receipts do not guarantee restored application state."
                    );
                    assert!(
                        offered.is_empty(),
                        "even failed attempts must not masquerade as offers"
                    );
                }
                other => panic!("{other:?}"),
            }
            assert_eq!(attempted, expected);
            let mut registry = restored.lock().unwrap();
            assert!(registry.is_execution(&expected[0]));
            assert!(registry.is_execution(&expected[1]));
            assert!(!registry.is_execution(&expected[2]));
            assert!(!registry.is_execution("https://example.test/old-anchor"));
        }
    }

    #[test]
    fn trigger_restores_the_matched_snapshot_even_when_history_changes() {
        struct ChangingEmbedder<'a> {
            inner: StubEmbedder,
            history: &'a History,
        }
        impl Embedder for ChangingEmbedder<'_> {
            fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
                if text == "query" {
                    // Simulate replacement during model inference. A second
                    // replay would have no work to restore (or could reuse ids).
                    std::fs::write(self.history.0.join("observation.log"), b"").unwrap();
                }
                self.inner.embed(text)
            }
        }
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        let b = &facts.briefs[0];
        let mut inner = StubEmbedder::for_facts(&facts);
        inner.query_for(&facts, "query", b.thread_id);
        let embedder = ChangingEmbedder {
            inner,
            history: &history,
        };
        let outcome = handle_voice_request(
            "TRIGGER 5",
            &mut &b"query"[..],
            &history.0,
            &registry(),
            Some(&embedder),
            |targets| {
                assert_eq!(
                    targets
                        .iter()
                        .map(|t| t.resource.clone())
                        .collect::<Vec<_>>(),
                    b.restore_set
                );
                opened(targets)
            },
        );
        assert_restored(outcome, &candidate(b, &facts).title);
        assert!(voice_facts(&history.0).unwrap().briefs.is_empty());
    }

    #[test]
    fn socket_path_is_deterministic() {
        let root = Path::new("/Users/test/Library/Application Support/evo");
        assert_eq!(voice_socket_path(root), voice_socket_path(root));
    }

    #[test]
    fn socket_path_differs_by_root() {
        let a = voice_socket_path(Path::new("/a"));
        let b = voice_socket_path(Path::new("/b"));
        assert_ne!(a, b);
    }

    #[test]
    fn encode_restored_round_trips() {
        let outcome = VoiceOutcome::Restored {
            title: "Evo stuff".to_string(),
            summary: "opened 2".to_string(),
            offered: vec![],
        };
        let encoded = encode_outcome(&outcome);
        assert!(encoded.starts_with("OUTCOME RESTORED\n"));
        assert!(encoded.contains("TITLE Evo stuff\n"));
        assert!(encoded.ends_with("END\n"));
    }

    #[test]
    fn encode_ambiguous_lists_candidates() {
        let outcome = VoiceOutcome::Ambiguous {
            candidates: vec![
                VoiceCandidate::work("1", "Alpha"),
                VoiceCandidate::work("2", "Beta"),
            ],
        };
        let encoded = encode_outcome(&outcome);
        assert!(encoded.contains("CANDIDATE 1\tAlpha\n"));
        assert!(encoded.contains("CANDIDATE 2\tBeta\n"));
    }

    #[test]
    fn sanitize_strips_protocol_breakers() {
        assert_eq!(sanitize_line("a\nb\rc\td"), "a b c d");
    }

    #[test]
    fn brief_outcome_encodes_count_and_narration() {
        let outcome = VoiceOutcome::Brief {
            count: 3,
            narration: "First, this is the work where you made X.".to_string(),
        };
        let encoded = encode_outcome(&outcome);
        assert!(encoded.starts_with("OUTCOME BRIEF\n"));
        assert!(encoded.contains("COUNT 3\n"));
        assert!(encoded.contains("NARRATION First, this is the work where you made X.\n"));
        assert!(encoded.ends_with("END\n"));
    }

    #[test]
    fn ask_over_missing_storage_answers_honestly() {
        // No log at this root: the engine mints nothing, and the spoken
        // answer is the honest empty rack — never an error, never a guess.
        let outcome = ask(Path::new("/nonexistent/evo-ask-test"));
        match outcome {
            VoiceOutcome::Brief { count, narration } => {
                assert_eq!(count, 0);
                assert_eq!(narration, "You have no works yet.");
            }
            other => panic!("expected an honest empty rack, got {other:?}"),
        }
    }

    #[test]
    fn narrate_rack_uses_witnessed_names() {
        use evo_threads::{AnchorKind, Status, WorkAnchor};
        let briefs = vec![WorkBrief {
            thread_id: 7,
            status: Status::Active,
            anchors: vec![WorkAnchor {
                resource: "ZCode".to_string(),
                kind: AnchorKind::Mutation,
            }],
            contested: vec![],
            companions: vec![],
            episodes: 2,
            resume_point: "ZCode".to_string(),
            resume_reason: "You kept returning to it.".to_string(),
            restore_set: vec!["ZCode".to_string()],
            open_deltas: vec![],
            fork: None,
        }];
        let names: std::collections::BTreeMap<String, String> =
            [("ZCode".to_string(), "Evo desktop".to_string())]
                .into_iter()
                .collect();
        let outcome = narrate_rack(&briefs, &names, briefs.len());
        match outcome {
            VoiceOutcome::Brief { count, narration } => {
                assert_eq!(count, 1);
                assert!(
                    narration.contains("you made Evo desktop"),
                    "names are what get spoken: {narration}"
                );
                assert!(
                    narration.contains("You left off at Evo desktop."),
                    "{narration}"
                );
            }
            other => panic!("expected a brief, got {other:?}"),
        }
    }

    /// The protocol seam for the browse read path: one `WORKS` request over
    /// a socket pair, response read back as bytes.
    fn works_response(history: &History) -> String {
        wire_request(history, b"WORKS\n", &registry(), None, never_restore)
    }

    #[test]
    fn works_request_parses_and_lists_the_rack() {
        let history = History::two_works();
        let response = works_response(&history);
        assert!(response.starts_with("WORKS 2\n"), "{response}");
        assert!(response.ends_with("END\n"), "{response}");
        assert!(response.contains("Lantern notebook"), "{response}");
        assert!(response.contains("Prism notebook"), "{response}");
    }

    #[test]
    fn works_round_trips_engine_order_fields_and_candidate_titles() {
        let history = History::two_works();
        let facts = voice_facts(&history.0).unwrap();
        assert_eq!(facts.briefs.len(), 2);
        let response = works_response(&history);
        // Line 1 counts; then one tab-separated line per work in the
        // engine's canonical display order (`briefs()` — the order ASK
        // narrates and NOTFOUND's RECENT lists); then END.
        let mut lines = response.lines();
        assert_eq!(
            lines.next(),
            Some(format!("WORKS {}", facts.briefs.len()).as_str())
        );
        for b in &facts.briefs {
            let c = candidate(b, &facts);
            let status = match b.status {
                evo_threads::Status::Active => "active",
                evo_threads::Status::Dormant => "dormant",
            };
            let expected = format!(
                "WORK {}\t{}\t{}\t{}\t{}",
                c.work_id,
                status,
                b.episodes,
                sanitize_line(&c.title),
                sanitize_line(&b.resume_reason),
            );
            assert_eq!(lines.next(), Some(expected.as_str()), "{response}");
        }
        assert_eq!(lines.next(), Some("END"));
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn works_flattens_tabs_and_newlines_so_names_cannot_break_the_protocol() {
        use evo_threads::{AnchorKind, Status, WorkAnchor};
        let resource = "file:///work/notes.md";
        let facts = VoiceFacts {
            briefs: vec![WorkBrief {
                thread_id: 7,
                status: Status::Active,
                anchors: vec![WorkAnchor {
                    resource: resource.to_string(),
                    kind: AnchorKind::Mutation,
                }],
                contested: vec![],
                companions: vec![],
                episodes: 3,
                resume_point: resource.to_string(),
                resume_reason: "You\tleft\noff\rhere.".to_string(),
                restore_set: vec![resource.to_string()],
                open_deltas: vec![],
                fork: None,
            }],
            bundles: BTreeMap::new(),
            names: [(resource.to_string(), "Ta\tb\nName".to_string())]
                .into_iter()
                .collect(),
            documents: vec![],
            surfaces: BTreeMap::new(),
            occurrences: BTreeMap::new(),
        };
        let encoded = encode_works(&facts);
        assert_eq!(
            encoded,
            "WORKS 1\nWORK 7\tactive\t3\tTa b Name\tYou left off here.\nEND\n"
        );
        assert_eq!(encoded.lines().count(), 3, "exactly header, work, END");
    }

    #[test]
    fn works_caps_the_listing_at_the_documented_bound() {
        let history = History::new();
        for i in 0..30u64 {
            history.work(
                100 + i * 4000,
                &format!("https://example.test/p/{i:02}"),
                &format!("Work {i:02}"),
            );
        }
        let facts = voice_facts(&history.0).unwrap();
        assert!(
            facts.briefs.len() > MAX_LISTED_WORKS,
            "the fixture must outrun the cap"
        );
        let response = works_response(&history);
        assert_eq!(response.lines().next(), Some("WORKS 24"));
        let listed = response
            .lines()
            .skip(1)
            .take(MAX_LISTED_WORKS)
            .collect::<Vec<_>>();
        assert_eq!(listed.len(), MAX_LISTED_WORKS);
        for (line, b) in listed.iter().zip(&facts.briefs) {
            // A capped page keeps the engine's order: the first 24 works.
            assert!(
                line.starts_with(&format!("WORK {}\t", b.thread_id)),
                "{line}"
            );
        }
        assert_eq!(
            response.matches("\nWORK ").count(),
            MAX_LISTED_WORKS,
            "{response}"
        );
        assert!(response.ends_with("END\n"), "{response}");
    }

    #[test]
    fn works_over_an_empty_history_is_zero_works_and_end() {
        let history = History::new();
        assert_eq!(works_response(&history), "WORKS 0\nEND\n");
    }

    #[test]
    fn works_refuses_an_unsupported_query_instead_of_ignoring_a_body() {
        // The optional semantic-filter form is not implemented: a `WORKS`
        // header carrying extra bytes is refused, never answered as a plain
        // listing while its body sits unread.
        let history = History::two_works();
        let response = wire_request(
            &history,
            b"WORKS 5\nhello",
            &registry(),
            None,
            never_restore,
        );
        assert!(response.starts_with("OUTCOME FAILED\n"), "{response}");
    }

    #[test]
    fn works_over_damaged_history_fails_honestly_without_mutating_the_log() {
        let history = History::new();
        let path = history.0.join("observation.log");
        std::fs::write(&path, b"not-a-length\n3\nabc\n").unwrap();
        let before = std::fs::read(&path).unwrap();
        let response = works_response(&history);
        assert!(response.starts_with("OUTCOME FAILED\n"), "{response}");
        assert!(
            response.contains("could not read work history"),
            "{response}"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "voice cannot repair a live writer's log"
        );
    }
}
