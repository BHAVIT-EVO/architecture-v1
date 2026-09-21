//! Observation-log parsing: raw length-prefixed records into ledger events.
//!
//! Shared by the engine process and the regression gate so both read the
//! same evidence the same way. The public surface is
//! [`read_observation_log`]; everything else is its machinery.

// ─── Observation log reader (PRIMARY data source) ─────────────────────────

/// How long after a focus/URL event an input bucket may land and still be
/// attributed to that surface. Focus events are point-in-time; beyond this
/// bound the record is stale evidence of what is on screen, and the input
/// keeps its own app identity.
const INPUT_SURFACE_WINDOW_MS: u64 = 90_000;

/// Reads attention events and file saves from the observation log.
///
/// Returns `(attention_events, file_saves)`. Focus/URL records become
/// attention surfaces; input records are re-keyed onto the surface they
/// happened on (the input counter knows the app, the focus stream knows the
/// window — neither alone carries both); file-save records are held for
/// attribution (they must never create Works on their own).
pub fn read_observation_log(
    log_path: &std::path::Path,
) -> (
    Vec<crate::ObservedEvent>,
    Vec<crate::ProductionSave>,
    Vec<(String, String)>,
) {
    let data = match std::fs::read(log_path) {
        Ok(d) => d,
        Err(_) => return (Vec::new(), Vec::new(), Vec::new()),
    };

    let mut surfaces: Vec<crate::ObservedEvent> = Vec::new();
    let mut inputs: Vec<crate::ObservedEvent> = Vec::new();
    let mut saves = Vec::new();
    let mut declared: Vec<(String, String)> = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let line_end = match data[offset..].iter().position(|&b| b == b'\n') {
            Some(pos) => offset + pos,
            None => break,
        };
        let length: usize = match std::str::from_utf8(&data[offset..line_end])
            .ok()
            .and_then(|s| s.trim().parse().ok())
        {
            Some(l) => l,
            None => break,
        };
        let record_start = line_end + 1;
        let record_end = record_start + length;
        if record_end > data.len() {
            break;
        }

        let record = String::from_utf8_lossy(&data[record_start..record_end]);
        match parse_observation_record(&record) {
            Some(ParsedRecord::Surface(event)) => surfaces.push(event),
            Some(ParsedRecord::Input(event)) => inputs.push(event),
            Some(ParsedRecord::Production(save)) => saves.push(save),
            Some(ParsedRecord::Declaration(a, b)) => declared.push((a, b)),
            None => {}
        }

        offset = record_end + 1;
    }

    let events = attribute_input_to_surfaces(surfaces, inputs);
    (events, saves, declared)
}

/// Fuses input evidence onto the attention surface it happened on.
///
/// An input bucket landing within [`INPUT_SURFACE_WINDOW_MS`] after a
/// surface event carries that surface's identity (so typing while
/// "app.rs — …" is focused enriches the app.rs work) and marks it as
/// production. Buckets with no recent surface keep their own app identity —
/// presence without a known surface is still signal, just coarser.
///
/// The attributed events are appended to the timeline as they are produced,
/// so continuous typing keeps re-anchoring to the same surface (each hop
/// requires a real input bucket within the window). With inputs sorted by
/// time, the timeline stays sorted and each lookup is a binary search for
/// the latest surface at or before the bucket — the naive reverse scan
/// was a measured superlinear hot spot at scale.
fn attribute_input_to_surfaces(
    mut surfaces: Vec<crate::ObservedEvent>,
    mut inputs: Vec<crate::ObservedEvent>,
) -> Vec<crate::ObservedEvent> {
    surfaces.sort_by_key(|e| e.timestamp_ms);
    inputs.sort_by_key(|e| e.timestamp_ms);

    for input in &inputs {
        // The latest surface at or before this bucket, if any, is the
        // newest timeline entry the binary search lands on.
        let bound = surfaces.partition_point(|s| s.timestamp_ms <= input.timestamp_ms);
        let surface = bound
            .checked_sub(1)
            .map(|idx| &surfaces[idx])
            .filter(|s| input.timestamp_ms - s.timestamp_ms <= INPUT_SURFACE_WINDOW_MS);
        match surface {
            Some(s) => {
                let attributed = crate::ObservedEvent {
                    timestamp_ms: input.timestamp_ms,
                    app_name: s.app_name.clone(),
                    window_title: s.window_title.clone(),
                    url: s.url.clone(),
                    document_path: None,
                    typed: input.typed,
                    dwell_ms: 10_000,
                    keys: input.keys,
                    clicks: input.clicks,
                    scrolls: input.scrolls,
                };
                surfaces.push(attributed);
            }
            None => surfaces.push(input.clone()),
        }
    }

    surfaces
}

/// One parsed observation record.
enum ParsedRecord {
    /// A focus or URL event: an attention surface that creates identity.
    Surface(crate::ObservedEvent),
    /// An input-activity bucket: presence evidence, attributed to the
    /// surface it happened on (or kept as its own app identity).
    Input(crate::ObservedEvent),
    /// A file save: production, attributed only when a surface names it.
    Production(crate::ProductionSave),
    /// The person declared two subjects the same work (ground truth).
    Declaration(String, String),
}

/// Context keys the engine reads from observation records. Written by the
/// capture layer per BE-TRACE-0001 §3.1 (provenance of the witnessing).
const CTX_OWNING_NAME: &str = "owning_process_name";
const CTX_DOCUMENT_LOCATOR: &str = "observed_state.document_locator";
const CTX_WINDOW_TITLE: &str = "observed_state.window_title";

/// Parses one observation-v1 record.
fn parse_observation_record(record: &str) -> Option<ParsedRecord> {
    let mut schema = String::new();
    let mut timestamp_nanos: u64 = 0;
    let mut subject = String::new();
    let mut context: Vec<(String, String)> = Vec::new();
    let mut int_facts: Vec<u64> = Vec::new();
    let mut text_facts: Vec<(String, String)> = Vec::new();
    let mut seen_subject = false;
    let mut pending_context_key: Option<String> = None;
    let mut pending_fact_name: Option<String> = None;

    for line in record.lines() {
        if let Some(v) = line.strip_prefix("schema_name_hex=") {
            schema = unhex(v);
        } else if let Some(v) = line.strip_prefix("observed_at=") {
            let v = v.trim_start_matches('+');
            if let Some((secs, nanos)) = v.split_once(':') {
                let s: u64 = secs.parse().unwrap_or(0);
                let n: u64 = nanos.parse().unwrap_or(0);
                timestamp_nanos = s * 1_000_000_000 + n;
            }
        } else if let Some(v) = line.strip_prefix("fact_value_hex=") {
            let value = unhex(v);
            if !seen_subject {
                subject = value.clone();
                seen_subject = true;
            }
            if let Some(name) = pending_fact_name.take() {
                text_facts.push((name, value));
            }
        } else if let Some(v) = line.strip_prefix("fact_name_hex=") {
            pending_fact_name = Some(unhex(v));
        } else if let Some(v) = line.strip_prefix("fact_value_int=") {
            // Integer facts appear in declaration order; for input records
            // that order is Keys, Clicks, Scrolls (the counter contract).
            int_facts.push(v.parse().unwrap_or(0));
        } else if let Some(v) = line.strip_prefix("context_key_hex=") {
            pending_context_key = Some(unhex(v));
        } else if let Some(v) = line.strip_prefix("context_value_hex=") {
            if let Some(key) = pending_context_key.take() {
                context.push((key, unhex(v)));
            }
        }
    }

    let ts_ms = timestamp_nanos / 1_000_000;
    if ts_ms == 0 || subject.is_empty() {
        return None;
    }
    let ctx = |key: &str| -> Option<&str> {
        context
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };

    match schema.as_str() {
        "OBS-WINDOW-FOCUS-GAINED" => {
            // A focus record's subject may be a document locator (the
            // capture layer resolves title → locator when the window
            // exposes one; browsers expose their URL there, editors their
            // file). Route it by shape; the window title and owning app
            // ride the record's context, so the richest records lose
            // nothing to resolution.
            let (url, document_path) = route_locator(
                &subject,
                ctx(CTX_DOCUMENT_LOCATOR).filter(|l| !l.trim().is_empty()),
            );
            let window_title = ctx(CTX_WINDOW_TITLE)
                .filter(|t| !t.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    // Legacy records (no context): the subject IS the title.
                    if url.is_none() && document_path.is_none() {
                        subject.clone()
                    } else {
                        String::new()
                    }
                });
            let app_name = ctx(CTX_OWNING_NAME)
                .filter(|n| !n.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    // No witnessed app name: coarse but honest.
                    if url.is_some() {
                        "Browser".to_string()
                    } else {
                        window_title.clone()
                    }
                });
            Some(ParsedRecord::Surface(crate::ObservedEvent {
                timestamp_ms: ts_ms,
                app_name,
                window_title,
                url,
                document_path,
                typed: false,
                dwell_ms: 5_000,
                keys: 0,
                clicks: 0,
                scrolls: 0,
            }))
        }
        "OBS-URL-NAVIGATED" => {
            let (url, _) = route_locator(&subject, None);
            Some(ParsedRecord::Surface(crate::ObservedEvent {
                timestamp_ms: ts_ms,
                app_name: "Browser".to_string(),
                // No title is captured for navigations; leaving it empty lets
                // the identity chain resolve to the stable URL host instead
                // of keying the work on a raw URL string.
                window_title: String::new(),
                url,
                document_path: None,
                typed: false,
                dwell_ms: 10_000,
                keys: 0,
                clicks: 0,
                scrolls: 0,
            }))
        }
        "OBS-FILE-SAVED" => {
            // Production, not attention: held for attribution. The path is
            // normalized first: editors that write atomically save
            // "name.tmp.PID.HEX" and rename to "name" — the save is about
            // the document, not the transient temp file.
            Some(ParsedRecord::Production(crate::ProductionSave {
                timestamp_ms: ts_ms,
                path: normalize_save_path(&subject),
            }))
        }
        "OBS-WORK-GROUPED" => {
            // The person declared two subjects one work: ground truth.
            // The record carries two text facts — the grouped subject and
            // its co-member.
            let co_member = text_facts
                .iter()
                .find(|(name, _)| name == "CoMember")
                .map(|(_, value)| value.clone())
                .filter(|v| !v.trim().is_empty())?;
            if subject.trim().is_empty() {
                return None;
            }
            Some(ParsedRecord::Declaration(subject.clone(), co_member))
        }
        "OBS-WORK-DESIGNATED" => {
            // A single designated subject: an anchor declaration without a
            // pair. It carries no grouping fact yet — retained for naming
            // when the desktop sends names.
            None
        }
        "OBS-INPUT-ACTIVITY" => {
            let keys = int_facts.first().copied().unwrap_or(0);
            let clicks = int_facts.get(1).copied().unwrap_or(0);
            let scrolls = int_facts.get(2).copied().unwrap_or(0);
            // Only real input counts: idle buckets carry zeros and would
            // fabricate attention the person never gave.
            if keys == 0 && clicks == 0 && scrolls == 0 {
                return None;
            }
            // The counter attributes its bucket to the resolved focus
            // subject, which may itself be a locator (URL or file path).
            let (url, document_path) = route_locator(&subject, None);
            let window_title = if url.is_none() && document_path.is_none() {
                subject.clone()
            } else {
                String::new()
            };
            Some(ParsedRecord::Input(crate::ObservedEvent {
                timestamp_ms: ts_ms,
                app_name: window_title.clone(),
                window_title,
                url,
                document_path,
                typed: keys > 0,
                dwell_ms: 10_000,
                keys,
                clicks,
                scrolls,
            }))
        }
        _ => None,
    }
}

/// Routes a subject (and its witnessed locator, when present) to URL or
/// document identity by shape alone: `http(s)://` is a URL, `file://` is a
/// document, and a bare absolute path is a document only when the window
/// itself claimed it through `AXDocument` — window titles can contain
/// slashes, but they never carry a locator.
fn route_locator(subject: &str, locator: Option<&str>) -> (Option<String>, Option<String>) {
    let effective = locator.unwrap_or(subject);
    if effective.starts_with("http://") || effective.starts_with("https://") {
        return (Some(effective.to_string()), None);
    }
    if let Some(path) = effective.strip_prefix("file://") {
        if !path.is_empty() {
            return (None, Some(path.to_string()));
        }
    }
    if locator.is_some() && effective.starts_with('/') {
        return (None, Some(effective.to_string()));
    }
    (None, None)
}

/// Strips the atomic-write temp suffix from a saved path.
///
/// Editors that save atomically write `name.tmp.<pid>.<hex>` and rename to
/// `name`; the witness sees both writes. The save is about the document,
/// so the transient suffix is normalized away and both writes attach to
/// the same work. A basename keeps its `.tmp.…` part only when it does not
/// match the convention (digits, then hex — a process id and a random
/// token, never a person's naming).
fn normalize_save_path(path: &str) -> String {
    let Some((dir, file)) = path.rsplit_once('/') else {
        return path.to_string();
    };
    let Some((stem, temp)) = file.split_once(".tmp.") else {
        return path.to_string();
    };
    let mut parts = temp.split('.');
    let (pid, token) = match (parts.next(), parts.next()) {
        (Some(p), Some(t)) if parts.next().is_none() => (p, t),
        _ => return path.to_string(),
    };
    // The convention: digits (a process id), then hex (a random token).
    // Never a person's naming.
    let looks_conventional = !pid.is_empty()
        && !token.is_empty()
        && pid.bytes().all(|b: u8| b.is_ascii_digit())
        && token.bytes().all(|b: u8| b.is_ascii_hexdigit());
    if looks_conventional && !stem.is_empty() {
        format!("{dir}/{stem}")
    } else {
        path.to_string()
    }
}

/// Decodes a hex-encoded UTF-8 string. The observation log hex-encodes fact
/// values byte-wise, so multi-byte characters (em dashes, non-ASCII titles)
/// must be decoded as bytes, not per-character.
pub fn unhex(hex: &str) -> String {
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        if let Ok(b) = u8::from_str_radix(&hex[i..i + 2], 16) {
            bytes.push(b);
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
