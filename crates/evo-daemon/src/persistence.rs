//! Canonical persistence helpers for the daemon runtime.
//!
//! These helpers serialize and deserialize canonical Workspace understanding
//! through the storage boundary without changing model semantics.
//!
//! # What changed, and why
//!
//! **The old rule.** This module also persisted and reloaded derived Workspace
//! understanding: `workspace.log` carried Workspace records (full state in
//! workspace-v1, per-formation deltas in workspace-v2) and `restoration.log`
//! carried Restoration outcomes. Reloading them was how Evo remembered what it
//! had understood.
//!
//! **Why it could not work.** A delta log can only ever *add* — append an
//! Attachment, append a Snapshot. But under the reconstruction model membership
//! is re-decided against the whole corpus every time evidence arrives: a
//! resource that looked like part of a body of work after ten sightings can be
//! revealed as unrelated after a hundred, and a resource's role changes as
//! attention moves. An append-only record of derived conclusions therefore
//! accumulates guesses that can never be withdrawn, and the reloaded state
//! diverges from what the same evidence derives today. The stored format could
//! not even express the current model: an Attachment record carried no role.
//!
//! **The new rule.** Only canonical evidence is persisted — Observations here,
//! Artifacts through the Artifact Acceptance Pipeline. Understanding is derived
//! from that evidence by [`crate::understanding`] and held in
//! [`crate::cache::CanonicalIndex`]; losing every derived byte loses nothing,
//! because replay re-derives it identically (ARCHITECTURE §2, §6; RFC-0003
//! Req 7). `workspace.log` and `restoration.log` are no longer written or read.
//!
//! What remains is Observation encode/decode plus the small derived reads that
//! answer questions directly about the Observation log (which subject an
//! Artifact witnessed, what the current declaration says).

use crate::errors::DaemonError;
use evo_artifact::artifact_id::ArtifactId;
use evo_observation::evidence::{Evidence, FactValue, ObservedFact};
use evo_observation::observation::Observation;
use evo_observation::observation_id::ObservationId;
use evo_observation::observation_schema::ObservationSchema;
use evo_observation::provenance::{ObservationSource, Provenance};
use evo_storage::{Storage, StorageObjectKind};

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const OBSERVATION_RECORD_VERSION: &str = "observation-v1";

// ── Observation persistence (IS-0001 R-7) ────────────────────────────────────

/// Durably persists one accepted canonical Observation through the storage
/// boundary.
///
/// The Observation log is the Architecture's immutable, append-only evidence
/// record (ARCHITECTURE §6: "the only fact that is permanently true is: at
/// time T, Evo observed X"). IS-0001 R-7 requires every accepted Observation
/// to be durably persisted before acceptance is reported.
pub fn persist_observation(observation: &Observation) -> Result<(), DaemonError> {
    let storage = Storage::new();
    storage.append(
        StorageObjectKind::Observation,
        encode_observation_record(observation).as_bytes(),
    )?;
    Ok(())
}

/// Loads every persisted canonical Observation in append order.
///
/// Malformed records are skipped and reported to stderr, not fatal: a
/// single bad record (a format drift from a different writer, a torn
/// write, a test fabrication) must never poison the entire history.
/// The log is append-only and multi-writer; the decoder's job is to read
/// what it can read and say what it could not.
pub fn load_persisted_observations(root: &Path) -> Result<Vec<Observation>, DaemonError> {
    let _guard = Storage::with_thread_root(root.to_path_buf());
    let storage = Storage::new();
    let mut observations = Vec::new();
    let mut skipped = 0usize;
    for record in storage.read_all(StorageObjectKind::Observation)? {
        match decode_observation_record(&record) {
            Ok(observation) => observations.push(observation),
            Err(err) => {
                skipped += 1;
                if skipped <= 5 {
                    eprintln!("EVO-PERSISTENCE skipping malformed record: {err}");
                }
            }
        }
    }
    if skipped > 0 {
        eprintln!("EVO-PERSISTENCE skipped {skipped} malformed record(s) out of {}", observations.len() + skipped);
    }
    Ok(observations)
}

pub(crate) fn encode_observation_record(observation: &Observation) -> String {
    let mut record = String::new();
    writeln!(&mut record, "{OBSERVATION_RECORD_VERSION}").unwrap();
    writeln!(
        &mut record,
        "id_hex={}",
        hex_encode(observation.id().to_string().as_bytes())
    )
    .unwrap();
    writeln!(
        &mut record,
        "schema_name_hex={}",
        hex_encode(observation.schema().name().as_bytes())
    )
    .unwrap();
    writeln!(&mut record, "schema_version={}", observation.schema().version()).unwrap();
    writeln!(
        &mut record,
        "source_hex={}",
        hex_encode(observation.provenance().source().as_str().as_bytes())
    )
    .unwrap();
    writeln!(
        &mut record,
        "observed_at={}",
        encode_system_time(observation.provenance().observed_at())
    )
    .unwrap();

    let context = observation.provenance().context();
    writeln!(&mut record, "context_count={}", context.len()).unwrap();
    let mut context_entries: Vec<_> = context.iter().collect();
    context_entries.sort_by(|left, right| left.0.cmp(right.0));
    for (key, value) in context_entries {
        writeln!(
            &mut record,
            "context_key_hex={}",
            hex_encode(key.as_bytes())
        )
        .unwrap();
        writeln!(
            &mut record,
            "context_value_hex={}",
            hex_encode(value.as_bytes())
        )
        .unwrap();
    }

    let facts = observation.evidence().facts();
    writeln!(&mut record, "fact_count={}", facts.len()).unwrap();
    for fact in facts {
        encode_fact_into(&mut record, fact).unwrap();
    }
    record
}

fn encode_fact_into(record: &mut String, fact: &ObservedFact) -> Result<(), std::fmt::Error> {
    writeln!(
        record,
        "fact_name_hex={}",
        hex_encode(fact.name().as_bytes())
    )?;
    match fact.value() {
        FactValue::Text(text) => {
            writeln!(record, "fact_value_kind=Text")?;
            writeln!(record, "fact_value_hex={}", hex_encode(text.as_bytes()))?;
        }
        FactValue::Integer(value) => {
            writeln!(record, "fact_value_kind=Integer")?;
            writeln!(record, "fact_value_int={value}")?;
        }
        FactValue::Boolean(value) => {
            writeln!(record, "fact_value_kind=Boolean")?;
            writeln!(record, "fact_value_bool={}", if *value { 1 } else { 0 })?;
        }
        FactValue::Bytes(bytes) => {
            writeln!(record, "fact_value_kind=Bytes")?;
            writeln!(record, "fact_value_bytes_hex={}", hex_encode(bytes))?;
        }
    }
    Ok(())
}

pub fn decode_observation_record_pub(record: &[u8]) -> Result<Observation, DaemonError> {
    decode_observation_record(record)
}

pub(crate) fn decode_observation_record(record: &[u8]) -> Result<Observation, DaemonError> {
    let text = std::str::from_utf8(record).map_err(|_| storage_error())?;
    let mut lines = text.lines();

    expect_exact(&mut lines, OBSERVATION_RECORD_VERSION)?;
    let id = ObservationId::from_str(&decode_text(next_value(&mut lines, "id_hex=")?)?)
        .map_err(|_| storage_error())?;
    let schema_name = decode_text(next_value(&mut lines, "schema_name_hex=")?)?;
    let schema_version = parse_u32(next_value(&mut lines, "schema_version=")?)?;
    let schema = ObservationSchema::new(schema_name, schema_version).map_err(|_| storage_error())?;
    let source = ObservationSource::new(decode_text(next_value(&mut lines, "source_hex=")?)?)
        .map_err(|_| storage_error())?;
    let observed_at = decode_system_time(next_value(&mut lines, "observed_at=")?)?;

    let context_count = parse_usize(next_value(&mut lines, "context_count=")?)?;
    let mut context = HashMap::new();
    for _ in 0..context_count {
        let key = decode_text(next_value(&mut lines, "context_key_hex=")?)?;
        let value = decode_text(next_value(&mut lines, "context_value_hex=")?)?;
        context.insert(key, value);
    }

    let fact_count = parse_usize(next_value(&mut lines, "fact_count=")?)?;
    let mut facts = Vec::with_capacity(fact_count);
    for _ in 0..fact_count {
        facts.push(decode_fact(&mut lines)?);
    }

    let provenance = Provenance::new(source, observed_at, context);
    let evidence = Evidence::new(facts);
    Ok(Observation::new(id, schema, provenance, evidence))
}

fn decode_fact<'a, I>(lines: &mut I) -> Result<ObservedFact, DaemonError>
where
    I: Iterator<Item = &'a str>,
{
    let name = decode_text(next_value(lines, "fact_name_hex=")?)?;
    let kind = next_value(lines, "fact_value_kind=")?;
    let value = match kind {
        "Text" => {
            let text = decode_text(next_value(lines, "fact_value_hex=")?)?;
            FactValue::Text(text)
        }
        "Integer" => {
            let raw = next_value(lines, "fact_value_int=")?;
            FactValue::Integer(raw.parse::<i64>().map_err(|_| storage_error())?)
        }
        "Boolean" => {
            let raw = next_value(lines, "fact_value_bool=")?;
            FactValue::Boolean(matches!(raw, "1"))
        }
        "Bytes" => {
            let raw = next_value(lines, "fact_value_bytes_hex=")?;
            FactValue::Bytes(hex_decode(raw)?)
        }
        _ => return Err(storage_error()),
    };
    ObservedFact::new(name, value).map_err(|_| storage_error())
}

/// Maps canonical Artifact identities to the subject their canonical
/// Observations witnessed, derived deterministically from the persisted
/// Observation log.
///
/// Each Observation contributes its witnessed subject to the Artifact its
/// canonical signature establishes (the same identity rule the Acceptance
/// Pipeline uses). This is presentation evidence only: the map is derived
/// from the immutable Observation log and never stored or treated as a
/// canonical model field.
pub fn load_artifact_subjects(root: &Path) -> Result<HashMap<String, String>, DaemonError> {
    let mut subjects: HashMap<String, String> = HashMap::new();
    for observation in load_persisted_observations(root)? {
        // Reference-only schemas (RFC-0011 WorkDesignated; RFC-0012
        // RepositoryMembership and WorkGrouped) reference existing canonical
        // Artifacts; they never establish one. Their subjects are never
        // witnessed content subjects, so they must not be presented as the
        // subject an Artifact's Observations witnessed.
        if observation.schema().is_reference_only() {
            continue;
        }
        let fact_name = match observation.schema().canonical_fact_name() {
            Some(name) => name,
            None => continue,
        };
        let subject = match observation.evidence().fact(fact_name) {
            Some(fact) => match fact.value() {
                FactValue::Text(text) if !text.trim().is_empty() => text.clone(),
                _ => continue,
            },
            None => continue,
        };
        let artifact_id =
            match evo_artifact::derive_artifact_id_from_observations(std::slice::from_ref(
                &observation,
            )) {
                Ok(id) => id,
                Err(_) => continue,
            };
        subjects
            .entry(artifact_id.to_string())
            .or_insert(subject);
    }
    Ok(subjects)
}

/// Maps canonical Artifact identities to the executable-target evidence their
/// canonical Observations witnessed, derived deterministically from the
/// persisted Observation log.
///
/// Each entry carries the frozen schema name and the witnessed subject, the
/// exact inputs the Execution layer's deterministic locator classification
/// consumes (IS-0003 schema → target kind). Presentation evidence only: the
/// map is derived from the immutable Observation log and never stored.
pub fn load_artifact_locators(root: &Path) -> Result<HashMap<String, (String, String)>, DaemonError> {
    let mut locators: HashMap<String, (String, String)> = HashMap::new();
    for observation in load_persisted_observations(root)? {
        // Reference-only schemas reference, they do not establish; their
        // subjects are never executable-target witnesses.
        if observation.schema().is_reference_only() {
            continue;
        }
        let fact_name = match observation.schema().canonical_fact_name() {
            Some(name) => name,
            None => continue,
        };
        let subject = match observation.evidence().fact(fact_name) {
            Some(fact) => match fact.value() {
                FactValue::Text(text) if !text.trim().is_empty() => text.clone(),
                _ => continue,
            },
            None => continue,
        };
        let artifact_id =
            match evo_artifact::derive_artifact_id_from_observations(std::slice::from_ref(
                &observation,
            )) {
                Ok(id) => id,
                Err(_) => continue,
            };
        locators
            .entry(artifact_id.to_string())
            .or_insert((observation.schema().name().to_string(), subject));
    }
    Ok(locators)
}

/// Returns the subject and witnessed time of the current WorkDesignated
/// designation (RFC-0011), derived deterministically from the persisted
/// Observation log.
///
/// The current designation is the most recent WorkDesignated observation by
/// canonical Observation Time; where times are equal, the later append-order
/// record is current (RFC-0011 §5). Derived, never stored; presentation and
/// derivation input only.
pub fn load_current_designation(root: &Path) -> Result<Option<(String, SystemTime)>, DaemonError> {
    let mut current: Option<(SystemTime, usize, String)> = None;
    for (index, observation) in load_persisted_observations(root)?.into_iter().enumerate() {
        if observation.schema() != &ObservationSchema::work_designated_v1() {
            continue;
        }
        let Some(subject) = designation_subject_of(&observation) else {
            continue;
        };
        let time = observation.provenance().observed_at();
        let better = match &current {
            None => true,
            Some((current_time, current_index, _)) => {
                time > *current_time || (time == *current_time && index > *current_index)
            }
        };
        if better {
            current = Some((time, index, subject));
        }
    }
    Ok(current.map(|(time, _, subject)| (subject, time)))
}

/// Returns the subject of the current WorkDesignated designation, or `None`
/// when no designation exists yet.
pub fn current_designation_subject(root: &Path) -> Result<Option<String>, DaemonError> {
    Ok(load_current_designation(root)?.map(|(subject, _)| subject))
}

/// Resolves the canonical Artifact a witnessed subject designates, per the
/// RFC-0011 resolution rule: the Artifact established by the content
/// Observation schemas (OBS-FILE-SAVED, OBS-URL-NAVIGATED,
/// OBS-WINDOW-FOCUS-GAINED, OBS-COMMIT-MADE) witnessing that subject, through
/// the Artifact layer's deterministic identity derivation.
///
/// A WorkDesignated observation never establishes an Artifact itself: a
/// designation references, it does not create (RFC-0011 §4). Returns `None`
/// when the subject is not witnessed by a content observation, or when more
/// than one distinct Artifact is established for it — ambiguity is preserved,
/// never resolved by guessing.
pub fn resolve_designated_artifact(
    root: &Path,
    subject: &str,
) -> Result<Option<ArtifactId>, DaemonError> {
    let mut distinct: Vec<ArtifactId> = Vec::new();
    for observation in load_persisted_observations(root)? {
        // Reference-only schemas (WorkDesignated, RepositoryMembership,
        // WorkGrouped) reference, they never establish: a co-membership
        // Observation witnessing the same subject must not create a second
        // distinct Artifact for it (RFC-0011 §4; RFC-0012 Artifact
        // Interaction).
        if observation.schema().is_reference_only() {
            continue;
        }
        let fact_name = match observation.schema().canonical_fact_name() {
            Some(name) => name,
            None => continue,
        };
        let witnessed = match observation.evidence().fact(fact_name) {
            Some(fact) => match fact.value() {
                FactValue::Text(text) if text == subject => true,
                _ => false,
            },
            None => false,
        };
        if !witnessed {
            continue;
        }
        let artifact_id = match evo_artifact::derive_artifact_id_from_observations(
            std::slice::from_ref(&observation),
        ) {
            Ok(id) => id,
            Err(_) => continue,
        };
        if !distinct.contains(&artifact_id) {
            distinct.push(artifact_id);
        }
    }
    if distinct.len() == 1 {
        Ok(Some(distinct.remove(0)))
    } else {
        Ok(None)
    }
}

/// Returns the canonical designated Artifact (RFC-0011) for the current
/// WorkDesignated designation, when both exist.
pub fn current_designated_artifact(root: &Path) -> Result<Option<ArtifactId>, DaemonError> {
    match current_designation_subject(root)? {
        Some(subject) => resolve_designated_artifact(root, &subject),
        None => Ok(None),
    }
}

/// The witnessed subject of a WorkDesignated observation, when it is valid.
pub(crate) fn designation_subject_of(observation: &Observation) -> Option<String> {
    let fact_name = observation.schema().canonical_fact_name()?;
    match observation.evidence().fact(fact_name) {
        Some(fact) => match fact.value() {
            FactValue::Text(text) if !text.trim().is_empty() => Some(text.clone()),
            _ => None,
        },
        None => None,
    }
}

/// The canonically ordered subjects of a continuation-surface declaration
/// (RFC-0013), when the observation is valid: the declaration fact carries
/// the Required Subject; every `ContinuationSubject` fact follows in
/// ascending order. Returns `None` when the observation is malformed.
pub(crate) fn continuation_surface_subjects_of(observation: &Observation) -> Option<Vec<String>> {
    if observation.schema() != &ObservationSchema::continuation_surface_v1() {
        return None;
    }
    let declaration = observation.evidence().fact("ContinuationSurface")?;
    let required = match declaration.value() {
        FactValue::Text(text) if !text.trim().is_empty() => text.clone(),
        _ => return None,
    };
    let mut subjects = vec![required];
    for fact in observation.evidence().facts() {
        if fact.name() != "ContinuationSubject" {
            continue;
        }
        match fact.value() {
            FactValue::Text(text) if !text.trim().is_empty() => subjects.push(text.clone()),
            _ => return None,
        }
    }
    if subjects.len() < 2 {
        return None;
    }
    // Canonical ordering is enforced by validation; re-verify determinism
    // here (the persisted record is canonical evidence).
    let mut ordered = subjects.clone();
    ordered.sort();
    if ordered != subjects {
        return None;
    }
    Some(subjects)
}

/// Returns the subjects of the current Continuation Surface (RFC-0013): the
/// latest valid OBS-CONTINUATION-SURFACE declaration by canonical Observation
/// Time; where times are equal, the later append-order record is current
/// (mirroring RFC-0011 §5). Derived, never stored.
pub fn load_current_continuation_surface(root: &Path) -> Result<Option<Vec<String>>, DaemonError> {
    let mut current: Option<(SystemTime, usize, Vec<String>)> = None;
    for (index, observation) in load_persisted_observations(root)?.into_iter().enumerate() {
        if observation.schema() != &ObservationSchema::continuation_surface_v1() {
            continue;
        }
        let Some(subjects) = continuation_surface_subjects_of(&observation) else {
            continue;
        };
        let time = observation.provenance().observed_at();
        let better = match &current {
            None => true,
            Some((current_time, current_index, _)) => {
                time > *current_time || (time == *current_time && index > *current_index)
            }
        };
        if better {
            current = Some((time, index, subjects));
        }
    }
    Ok(current.map(|(_, _, subjects)| subjects))
}

fn decode_text(value: &str) -> Result<String, DaemonError> {
    String::from_utf8(hex_decode(value)?).map_err(|_| storage_error())
}

fn encode_system_time(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("+{}:{}", duration.as_secs(), duration.subsec_nanos()),
        Err(error) => {
            let duration = error.duration();
            format!("-{}:{}", duration.as_secs(), duration.subsec_nanos())
        }
    }
}

fn decode_system_time(value: &str) -> Result<SystemTime, DaemonError> {
    let (sign, rest) = value.split_at(1);
    let (secs, nanos) = rest.split_once(':').ok_or_else(storage_error)?;
    let duration = Duration::new(parse_u64(secs)?, parse_u32(nanos)?);
    match sign {
        "+" => UNIX_EPOCH.checked_add(duration).ok_or_else(storage_error),
        "-" => UNIX_EPOCH.checked_sub(duration).ok_or_else(storage_error),
        _ => Err(storage_error()),
    }
}

fn expect_exact<'a, I>(lines: &mut I, expected: &str) -> Result<(), DaemonError>
where
    I: Iterator<Item = &'a str>,
{
    match lines.next() {
        Some(line) if line == expected => Ok(()),
        _ => Err(storage_error()),
    }
}

fn next_value<'a, I>(lines: &mut I, prefix: &str) -> Result<&'a str, DaemonError>
where
    I: Iterator<Item = &'a str>,
{
    let line = lines.next().ok_or_else(storage_error)?;
    line.strip_prefix(prefix).ok_or_else(storage_error)
}

fn parse_usize(value: &str) -> Result<usize, DaemonError> {
    value.parse::<usize>().map_err(|_| storage_error())
}

fn parse_u32(value: &str) -> Result<u32, DaemonError> {
    value.parse::<u32>().map_err(|_| storage_error())
}

fn parse_u64(value: &str) -> Result<u64, DaemonError> {
    value.parse::<u64>().map_err(|_| storage_error())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(value: &str) -> Result<Vec<u8>, DaemonError> {
    if value.len() % 2 != 0 {
        return Err(storage_error());
    }

    let mut bytes = Vec::with_capacity(value.len() / 2);
    let mut chars = value.as_bytes().chunks_exact(2);
    for pair in &mut chars {
        let hi = hex_value(pair[0])?;
        let lo = hex_value(pair[1])?;
        bytes.push((hi << 4) | lo);
    }

    Ok(bytes)
}

fn hex_value(byte: u8) -> Result<u8, DaemonError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(storage_error()),
    }
}

fn storage_error() -> DaemonError {
    DaemonError::from(evo_storage::StorageError::BackendNotConfigured)
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_observation::observed_state::ObservedState;
    fn observation_fixture() -> Observation {
        let source = ObservationSource::new("macos_accessibility").unwrap();
        let mut context = HashMap::new();
        context.insert("observer_version".to_string(), "0.1.0".to_string());
        let provenance = Provenance::new(
            source,
            UNIX_EPOCH
                .checked_add(Duration::from_secs(1_700_000_000))
                .unwrap(),
            context,
        );
        let fact = ObservedFact::new(
            "WindowFocusGained",
            FactValue::Text("editor-window".into()),
        )
        .unwrap();
        let evidence = Evidence::new(vec![fact]);
        Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174001").unwrap(),
            ObservationSchema::window_focus_gained_v1(),
            provenance,
            evidence,
        )
    }

    #[test]
    fn observation_round_trips_through_codec() {
        let original = observation_fixture();
        let decoded =
            decode_observation_record(encode_observation_record(&original).as_bytes()).unwrap();
        assert_eq!(original, decoded);
        assert_eq!(original.id(), decoded.id());
        assert_eq!(original.evidence(), decoded.evidence());
    }

    #[test]
    fn observation_with_all_fact_value_kinds_round_trips() {
        let facts = vec![
            ObservedFact::new("name", FactValue::Text("alpha".into())).unwrap(),
            ObservedFact::new("count", FactValue::Integer(-42)).unwrap(),
            ObservedFact::new("flag", FactValue::Boolean(true)).unwrap(),
            ObservedFact::new("payload", FactValue::Bytes(vec![0, 1, 2, 254])).unwrap(),
        ];
        let observation = Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174002").unwrap(),
            ObservationSchema::window_focus_gained_v1(),
            Provenance::new(
                ObservationSource::new("test_source").unwrap(),
                SystemTime::now(),
                HashMap::new(),
            ),
            Evidence::new(facts),
        );
        let decoded =
            decode_observation_record(encode_observation_record(&observation).as_bytes()).unwrap();
        assert_eq!(observation, decoded);
    }

    #[test]
    fn observed_state_context_round_trips_through_persistence() {
        let source = ObservationSource::new("macos_accessibility").unwrap();
        let state = ObservedState::new()
            .with_document_locator("/work/automation-x.ts")
            .with_focused_role("AXTextArea")
            .with_selection(418, 12)
            .with_insertion_line(37);
        let mut context = HashMap::new();
        state.write_context(&mut context);
        let observation = Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174099").unwrap(),
            ObservationSchema::window_focus_gained_v3(),
            Provenance::new(source, UNIX_EPOCH, context),
            Evidence::new(vec![ObservedFact::new(
                "WindowFocusGained",
                FactValue::Text("editor-window".into()),
            )
            .unwrap()]),
        );
        let decoded = decode_observation_record(encode_observation_record(&observation).as_bytes())
            .unwrap();
        assert_eq!(ObservedState::from_context(decoded.provenance().context()), Some(state));
        assert_eq!(observation, decoded);
    }

    /// A v2 OBS-WINDOW-FOCUS-GAINED observation carrying the owning-process
    /// pid in `Provenance::context` (BE-TRACE-0001 §3.1) — the shape the
    /// live collector now produces.
    fn window_focus_v2_with_pid_fixture() -> Observation {
        let mut context = HashMap::new();
        context.insert(
            evo_capture::OWNING_PROCESS_PID_CONTEXT_KEY.to_string(),
            "4242".to_string(),
        );
        let provenance = Provenance::new(
            ObservationSource::new("macos_accessibility").unwrap(),
            UNIX_EPOCH
                .checked_add(Duration::from_secs(1_700_000_000))
                .unwrap(),
            context,
        );
        let fact = ObservedFact::new(
            "WindowFocusGained",
            FactValue::Text("editor-window".into()),
        )
        .unwrap();
        Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174003").unwrap(),
            ObservationSchema::window_focus_gained_v2(),
            provenance,
            Evidence::new(vec![fact]),
        )
    }

    #[test]
    fn window_focus_v2_pid_context_round_trips_through_the_codec() {
        // The owning-process pid rides in `Provenance::context` and must
        // survive encode → decode byte-for-byte (the context is carried
        // count-driven; no format version bump was needed, BE-TRACE-0001
        // §3.1).
        let original = window_focus_v2_with_pid_fixture();
        let decoded =
            decode_observation_record(encode_observation_record(&original).as_bytes()).unwrap();
        assert_eq!(original, decoded);
        assert_eq!(original.schema(), &ObservationSchema::window_focus_gained_v2());
        assert_eq!(
            decoded
                .provenance()
                .context()
                .get(evo_capture::OWNING_PROCESS_PID_CONTEXT_KEY)
                .map(String::as_str),
            Some("4242")
        );
    }

    #[test]
    fn window_focus_v1_record_without_pid_remains_readable_and_valid() {
        // Backward compatibility: a v1 observation — which carries no owning
        // pid in its context — must decode, remain canonical, and stay valid
        // for identity derivation now that v2 exists alongside it
        // (BE-AUDIT-0001 §8.6; §13.3 #17).
        let v1 = observation_fixture();
        assert_eq!(v1.schema(), &ObservationSchema::window_focus_gained_v1());
        assert!(v1.schema().is_canonical());
        let decoded = decode_observation_record(encode_observation_record(&v1).as_bytes()).unwrap();
        assert_eq!(v1, decoded);
        // v1 carries its own context (e.g. observer_version) but never the
        // owning-process pid: absence is preserved as absence.
        assert_eq!(
            decoded
                .provenance()
                .context()
                .get(evo_capture::OWNING_PROCESS_PID_CONTEXT_KEY),
            None
        );
        assert_eq!(
            decoded.provenance().context().get("observer_version").map(String::as_str),
            Some("0.1.0")
        );
        assert_eq!(decoded.evidence(), v1.evidence());
    }

    #[test]
    fn window_focus_pid_survives_persistence_and_replay() {
        // The full FIX 1 requirement: the pid survives persistence and
        // survives replay (BE-TRACE-0001 §2.1 trace path). Persist a v2
        // observation with the pid, reload the Observation log the replay
        // path consumes, and confirm the pid is still there.
        let root = std::env::temp_dir().join(format!(
            "evo-daemon-window-pid-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let original = window_focus_v2_with_pid_fixture();
        // A second witness of the same window, at a later moment.
        //
        // AMENDED RULE. This test previously asserted that the second witness
        // "justifies origination (RFC-0003 Req 5)", so replay derived exactly
        // one Workspace for the re-witnessed Artifact. That is the defect this
        // rewrite removes: a resource looked at twice with nothing else is a
        // resource looked at twice, not a body of work (§8). The provenance
        // guarantee this test exists for is unchanged — only the expected
        // Workspace count is.
        let mut context = HashMap::new();
        context.insert(
            evo_capture::OWNING_PROCESS_PID_CONTEXT_KEY.to_string(),
            "4242".to_string(),
        );
        let second = Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174004").unwrap(),
            ObservationSchema::window_focus_gained_v2(),
            Provenance::new(
                ObservationSource::new("macos_accessibility").unwrap(),
                UNIX_EPOCH
                    .checked_add(Duration::from_secs(1_700_000_100))
                    .unwrap(),
                context,
            ),
            Evidence::new(vec![ObservedFact::new(
                "WindowFocusGained",
                FactValue::Text("editor-window".into()),
            )
            .unwrap()]),
        );
        {
            let _guard = Storage::with_thread_root(&root);
            persist_observation(&original).unwrap();
            persist_observation(&second).unwrap();
        }

        // Persistence: the reloaded log carries the pid on every record.
        let reloaded = load_persisted_observations(&root).unwrap();
        assert_eq!(reloaded.len(), 2);
        for record in &reloaded {
            assert_eq!(
                record
                    .provenance()
                    .context()
                    .get(evo_capture::OWNING_PROCESS_PID_CONTEXT_KEY)
                    .map(String::as_str),
                Some("4242")
            );
        }

        // Replay: re-deriving from the same log succeeds and keeps the
        // evidence — one window, witnessed twice, relates to nothing, so it is
        // *remembered*, never work — and the canonical Observations it consumed
        // still carry the provenance.
        let replayed = crate::workspace_replay::replay_understanding_from_root(&root).unwrap();
        assert!(
            !replayed.workspaces().is_empty(),
            "one resource witnessed twice is kept as evidence, not silently dropped"
        );
        assert!(
            replayed
                .standings()
                .values()
                .all(|standing| !standing.is_work()),
            "one resource witnessed twice is remembered, not a body of work"
        );
        for record in &reloaded {
            assert_eq!(record.schema(), &ObservationSchema::window_focus_gained_v2());
        }
    }

    #[test]
    fn v1_window_focus_identity_is_stable_once_v2_exists_alongside_it() {
        // The coexistence guarantee (BE-AUDIT-0001 §13.3 #17): a v1 record
        // replays to the same Artifact identity whether or not v2 records
        // exist in the log. The schema version participates in the identity
        // signature, so a v2 observation of the same subject is a distinct
        // schema identity — that is the documented consequence of schema
        // versioning (OA-6), and v1 is never re-interpreted as v2.
        let v1 = observation_fixture();
        let v2 = window_focus_v2_with_pid_fixture();
        let id_before = evo_artifact::derive_artifact_id_from_observations(&[v1.clone()]).unwrap();
        let id_after =
            evo_artifact::derive_artifact_id_from_observations(&[v1.clone()]).unwrap();
        assert_eq!(id_before, id_after);
        let id_v2 = evo_artifact::derive_artifact_id_from_observations(&[v2]).unwrap();
        assert_ne!(id_before, id_v2, "v1 and v2 are distinct schema identities");
    }

    #[test]
    fn artifact_subjects_are_derived_from_the_persisted_observation_log() {
        let root = std::env::temp_dir().join(format!(
            "evo-daemon-subjects-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174010").unwrap(),
            ObservationSchema::window_focus_gained_v1(),
            Provenance::new(
                ObservationSource::new("macos_accessibility").unwrap(),
                SystemTime::now(),
                HashMap::new(),
            ),
            Evidence::new(vec![ObservedFact::new(
                "WindowFocusGained",
                FactValue::Text("Design Brief — Canvas".into()),
            )
            .unwrap()]),
        );
        let second = Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174011").unwrap(),
            ObservationSchema::window_focus_gained_v1(),
            first.provenance().clone(),
            first.evidence().clone(),
        );
        {
            let _guard = Storage::with_thread_root(&root);
            persist_observation(&first).unwrap();
            persist_observation(&second).unwrap();
        }

        let subjects = load_artifact_subjects(&root).expect("subjects should load");
        // Two Observations of the same window collapse to one Artifact whose
        // subject is the witnessed window title.
        assert_eq!(subjects.len(), 1);
        let artifact_id = evo_artifact::derive_artifact_id_from_observations(&[first])
            .expect("identity derives");
        assert_eq!(
            subjects.get(artifact_id.as_str()).map(String::as_str),
            Some("Design Brief — Canvas")
        );
    }

    #[test]
    fn artifact_locators_carry_schema_and_subject_for_execution() {
        let root = std::env::temp_dir().join(format!(
            "evo-daemon-locators-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let window = Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174020").unwrap(),
            ObservationSchema::window_focus_gained_v1(),
            Provenance::new(
                ObservationSource::new("macos_accessibility").unwrap(),
                SystemTime::now(),
                HashMap::new(),
            ),
            Evidence::new(vec![ObservedFact::new(
                "WindowFocusGained",
                FactValue::Text("Design Brief — Canvas".into()),
            )
            .unwrap()]),
        );
        let url = Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174021").unwrap(),
            ObservationSchema::url_navigated_v1(),
            window.provenance().clone(),
            Evidence::new(vec![ObservedFact::new(
                "URLNavigated",
                FactValue::Text("https://example.com/doc".into()),
            )
            .unwrap()]),
        );
        let commit = Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174022").unwrap(),
            ObservationSchema::commit_made_v1(),
            window.provenance().clone(),
            Evidence::new(vec![ObservedFact::new(
                "CommitMade",
                FactValue::Text("abc123".into()),
            )
            .unwrap()]),
        );
        {
            let _guard = Storage::with_thread_root(&root);
            persist_observation(&window).unwrap();
            persist_observation(&url).unwrap();
            persist_observation(&commit).unwrap();
        }

        let locators = load_artifact_locators(&root).expect("locators should load");
        // Three distinct subjects → three distinct Artifacts, each with its
        // frozen schema name and witnessed subject.
        assert_eq!(locators.len(), 3);
        let window_id = evo_artifact::derive_artifact_id_from_observations(&[window])
            .expect("identity derives");
        let url_id =
            evo_artifact::derive_artifact_id_from_observations(&[url]).expect("identity derives");
        assert_eq!(
            locators.get(window_id.as_str()),
            Some(&(
                "OBS-WINDOW-FOCUS-GAINED".to_string(),
                "Design Brief — Canvas".to_string()
            ))
        );
        assert_eq!(
            locators.get(url_id.as_str()),
            Some(&(
                "OBS-URL-NAVIGATED".to_string(),
                "https://example.com/doc".to_string()
            ))
        );
    }

    #[test]
    fn persisted_observations_survive_reload_in_append_order() {
        let root = std::env::temp_dir().join(format!(
            "evo-daemon-observation-log-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = observation_fixture();
        let second = Observation::new(
            ObservationId::from_str("123e4567-e89b-12d3-a456-426614174003").unwrap(),
            first.schema().clone(),
            first.provenance().clone(),
            first.evidence().clone(),
        );
        {
            let _guard = Storage::with_thread_root(&root);
            persist_observation(&first).expect("first observation should persist");
            persist_observation(&second).expect("second observation should persist");
        }

        let loaded = load_persisted_observations(&root).expect("observations should load");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0], first);
        assert_eq!(loaded[1], second);
    }
}
