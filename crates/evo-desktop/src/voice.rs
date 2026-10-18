//! Desktop voice module: mic → wake → transcribe → daemon → outcome.
//!
//! The desktop owns the microphone and the voice UI. The daemon owns
//! resolution and restoration. This module is the bridge:
//!
//! ```text
//! Mic (cpal) → VoiceDriver (evo-voice) → VoiceSocketClient → daemon socket
//!     → VoiceOutcome → VoiceEvent → egui panel
//! ```
//!
//! # Ambiguity UX
//!
//! When the daemon returns `Ambiguous` (two similar-confidence candidates,
//! low confidence, or unparseable), the desktop shows a floating numbered
//! panel — bottom-center, non-focus-stealing — with 2–3 candidates. Each
//! row shows the ordinal number, work title, and a distinguishing subtitle.
//! The user picks by saying "the second one" / "number two", clicking,
//! or pressing 1/2/3. Esc cancels. After ~8s with no selection the panel
//! dismisses and nothing is restored — never guess on ambiguous input.
//!
//! This follows Apple's converging conventions: numbered overlays (Voice
//! Control), ordinal voice references (Siri), and audible+visual responses
//! (HIG). The visual list eliminates the listen-remember-compare burden
//! that makes voice-only disambiguation fail.
//!
//! # No hardcoding
//!
//! Nothing here names an application, domain, or command phrase. Candidate
//! subtitles are derived from the work's own metadata (recency, title).
//! Ordinal words are the language's own counting words, not app behavior.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
#[cfg(feature = "voice-audio")]
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

#[cfg(feature = "voice-audio")]
const VOICE_MODEL_FILES: [&str; 4] = [
    "whisper-base.en.bin",
    "melspectrogram.onnx",
    "embedding_model.onnx",
    "hey_evoh.onnx",
];

/// Model locations for development binaries, installed headless agents, and
/// app bundles. A bundle keeps large model assets in Contents/Resources.
#[cfg(feature = "voice-audio")]
fn model_search_dirs(executable: &std::path::Path) -> Vec<PathBuf> {
    let Some(binary_dir) = executable.parent() else {
        return Vec::new();
    };
    let mut dirs = vec![binary_dir.to_path_buf()];
    if let Some(contents_dir) = binary_dir.parent() {
        dirs.push(contents_dir.join("Resources"));
    }
    dirs
}

#[cfg(feature = "voice-audio")]
fn model_path(name: &str) -> Result<PathBuf, String> {
    let executable =
        std::env::current_exe().map_err(|err| format!("cannot locate voice executable: {err}"))?;
    let dirs = model_search_dirs(&executable);
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "missing voice model {name:?}; searched {}",
                dirs.iter()
                    .map(|dir| dir.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

#[cfg(feature = "voice-audio")]
fn wake_model_dir() -> Result<PathBuf, String> {
    let executable =
        std::env::current_exe().map_err(|err| format!("cannot locate voice executable: {err}"))?;
    let required = &VOICE_MODEL_FILES[1..];
    let dirs = model_search_dirs(&executable);
    for dir in &dirs {
        if required.iter().all(|name| dir.join(name).is_file()) {
            return Ok(dir.clone());
        }
    }
    Err(format!(
        "missing acoustic wake model(s) {}; searched {}",
        required.join(", "),
        dirs.iter()
            .map(|dir| dir.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

#[cfg(feature = "voice-audio")]
use evo_voice::VoiceDriver;
use evo_voice::{TriggerTransport, VoiceCandidate, VoiceEvent, VoiceOutcome, VoiceSelection};

/// How long the ambiguity panel stays up waiting for a pick before
/// dismissing with no restore. Long enough to read 2–3 rows and decide;
/// short enough that a stale panel never lingers.
const AMBIGUITY_TIMEOUT: Duration = Duration::from_secs(8);

/// Max candidates the panel shows. The daemon sends at most a handful;
/// the panel never shows more than three — option paralysis hits fast
/// in voice contexts.
const MAX_PANEL_CANDIDATES: usize = 3;

/// Max candidate lines one outcome response may carry. The daemon's
/// `NOTFOUND` lists its whole rack (bounded like the browse listing), so
/// five was a lie about honest misses; the panel still shows at most
/// [`MAX_PANEL_CANDIDATES`]. A longer list is a protocol error.
const MAX_READ_CANDIDATES: usize = 24;

/// Max `WORK` lines in one `WORKS` reply, mirroring the daemon's own
/// `MAX_LISTED_WORKS` bound. A count beyond it is a protocol error, so a
/// misbehaving daemon can never stretch the wire.
const MAX_LISTED_WORKS: usize = 24;

/// One browsable work from the daemon's `WORKS` listing: the browse
/// carousel's data, owned and socket-friendly like
/// [`evo_voice::VoiceCandidate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkEntry {
    /// Stable numeric work identity (the daemon's CHOOSE key).
    pub id: String,
    /// The work's state word ("active" / "dormant").
    pub status: String,
    /// Sittings the engine has assigned to the work.
    pub episodes: u32,
    /// What the work is about, from its own witnessed material.
    pub title: String,
    /// The engine's one-breath reason to resume this work.
    pub resume_reason: String,
}

/// The daemon-side voice socket client.
///
/// Implements [`TriggerTransport`] over the daemon's Unix socket:
/// sends `TRIGGER` / `CHOOSE` / structured `SCOPE` / exact `RESOURCE` /
/// browse `WORKS` requests, parses the line-protocol response into
/// [`VoiceOutcome`] (or the `WORKS` listing into [`WorkEntry`]s).
pub struct VoiceSocketClient {
    storage_root: PathBuf,
}

impl VoiceSocketClient {
    pub fn new(storage_root: PathBuf) -> Self {
        Self { storage_root }
    }

    /// Opens the daemon's voice socket.
    ///
    /// Every response read carries a two-minute timeout: a wedged daemon
    /// must fail one request as an honest `Failed` outcome, never freeze
    /// the always-on wake loop (the agent thread would otherwise hang
    /// forever on a `read_line` to a daemon that stopped replying — the
    /// first cold-index build predates the same fix on the daemon side).
    fn connect(&self) -> Result<UnixStream, String> {
        let path = evo_daemon::voice::voice_socket_path(&self.storage_root);
        let stream =
            UnixStream::connect(&path).map_err(|e| format!("voice socket unreachable: {e}"))?;
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(120)))
            .map_err(|e| format!("cannot set voice read timeout: {e}"))?;
        Ok(stream)
    }

    /// Sends a `CHOOSE <work_id>` request after the user picks a candidate.
    pub fn choose(&self, work_id: &str) -> Result<VoiceOutcome, String> {
        let mut stream = self.connect()?;
        let request = format!("CHOOSE {work_id}\n");
        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("voice request failed: {e}"))?;
        parse_voice_response(&stream)
    }

    /// Sends the opaque capability for one witnessed occurrence selected by
    /// the person. The daemon replays the current snapshot and verifies the
    /// key before binding a restore target to that exact surface.
    pub fn choose_occurrence(&self, work_id: &str, key: &str) -> Result<VoiceOutcome, String> {
        if work_id
            .parse::<u64>()
            .ok()
            .map(|id| id.to_string())
            .as_deref()
            != Some(work_id)
        {
            return Err("occurrence choice requires a canonical numeric work id".into());
        }
        if key.len() != 16 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("occurrence choice key is malformed".into());
        }
        let mut stream = self.connect()?;
        let request = format!("CHOOSE_OCCURRENCE {work_id} {key}\n");
        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("voice request failed: {e}"))?;
        parse_voice_response(&stream)
    }

    /// Dispatches the exact action represented by a choice row.
    pub fn choose_candidate(&self, candidate: &VoiceCandidate) -> Result<VoiceOutcome, String> {
        match &candidate.selection {
            VoiceSelection::Work => self.choose(&candidate.work_id),
            VoiceSelection::Occurrence { key } => self.choose_occurrence(&candidate.work_id, key),
        }
    }

    /// Restores an explicitly named surface within one selected work. This is
    /// structured transport; it does not parse scope from spoken commands.
    /// This text API retains daemon scope resolution for compatibility. The
    /// rack's visible exact-locator picker uses [`Self::scope_exact`] instead.
    pub fn scope(&self, work_id: &str, query: &str) -> Result<VoiceOutcome, String> {
        self.scope_request("SCOPE", work_id, query)
    }

    /// Sends one exact UTF-8 locator from the visible scoped restoration sheet.
    /// The daemon requires byte-for-byte membership in the fresh work bundle:
    /// stale resources fail, without semantic matching or broader restoration.
    pub fn scope_exact(&self, work_id: &str, locator: &str) -> Result<VoiceOutcome, String> {
        self.scope_request("RESOURCE", work_id, locator)
    }

    fn scope_request(
        &self,
        verb: &str,
        work_id: &str,
        query: &str,
    ) -> Result<VoiceOutcome, String> {
        if work_id
            .parse::<u64>()
            .ok()
            .map(|id| id.to_string())
            .as_deref()
            != Some(work_id)
        {
            return Err("scope requires a canonical numeric work id".into());
        }
        if query.trim().is_empty() || query.len() > 4096 {
            return Err(
                "scope requires a nonempty UTF-8 surface query of at most 4096 bytes".into(),
            );
        }
        let mut stream = self.connect()?;
        let header = format!("{verb} {work_id} {}\n", query.len());
        stream
            .write_all(header.as_bytes())
            .and_then(|_| stream.write_all(query.as_bytes()))
            .map_err(|e| format!("voice request failed: {e}"))?;
        parse_voice_response(&stream)
    }

    /// Sends the engine-backed `ASK` request: the whole rack, narrated.
    pub fn ask(&self) -> Result<VoiceOutcome, String> {
        let mut stream = self.connect()?;
        stream
            .write_all(b"ASK\n")
            .map_err(|e| format!("voice request failed: {e}"))?;
        parse_voice_response(&stream)
    }

    /// Sends `BRIEF <work_id>`: one work, narrated from engine facts.
    pub fn brief(&self, work_id: &str) -> Result<VoiceOutcome, String> {
        let mut stream = self.connect()?;
        let request = format!("BRIEF {work_id}\n");
        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("voice request failed: {e}"))?;
        parse_voice_response(&stream)
    }

    /// Sends `WORKS`: the rack as a browse listing, one entry per work in
    /// the engine's canonical display order (best-supported first). This
    /// is the one verb whose reply is not a [`VoiceOutcome`]; daemon-side
    /// unavailability still arrives as `OUTCOME FAILED` and maps to
    /// `Err(reason)`, so the caller can report it honestly.
    pub fn works(&self) -> Result<Vec<WorkEntry>, String> {
        let mut stream = self.connect()?;
        stream
            .write_all(b"WORKS\n")
            .map_err(|e| format!("voice request failed: {e}"))?;
        parse_works_response(&stream)
    }
}

impl TriggerTransport for VoiceSocketClient {
    fn trigger(&self, command: &str) -> Result<VoiceOutcome, String> {
        // Keep the transport boundary defensive as well as the driver
        // boundary: callers other than VoiceDriver must not be able to send
        // Whisper's bracketed silence metadata to the daemon.
        let command = evo_voice::sanitize_transcript(command);
        let command = match evo_voice::detect_wake(&command) {
            evo_voice::WakeResult::Detected { command } => command,
            evo_voice::WakeResult::Absent => command,
        };
        if command.is_empty() {
            return Ok(VoiceOutcome::NoCommand);
        }
        let mut stream = self.connect()?;
        let bytes = command.as_bytes();
        let header = format!("TRIGGER {}\n", bytes.len());
        stream
            .write_all(header.as_bytes())
            .and_then(|_| stream.write_all(bytes))
            .map_err(|e| format!("voice request failed: {e}"))?;
        parse_voice_response(&stream)
    }
    fn ask(&self) -> Result<VoiceOutcome, String> {
        // Delegates to the ASK verb: the daemon narrates its engine rack
        // at request time.
        self.ask()
    }
}

/// Parses the daemon's line-protocol response into a [`VoiceOutcome`].
///
/// ```text
/// OUTCOME RESTORED\nTITLE <t>\nSUMMARY <s>\n[OFFERED <resource>\n...]\nEND\n
/// OUTCOME AMBIGUOUS\nCANDIDATE <id>\t<title>[\tOCCURRENCE\t<key>]\n...\nEND\n
/// OUTCOME NOTFOUND\nRECENT <id>\t<title>\n...\nEND\n
/// OUTCOME FAILED\nREASON <text>\nEND\n
/// OUTCOME NOCOMMAND\nEND\n
/// ```
fn parse_voice_response(stream: &UnixStream) -> Result<VoiceOutcome, String> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("voice response unreadable: {e}"))?;
    let header = line.trim_end();

    match header {
        "OUTCOME RESTORED" => {
            let title = read_field(&mut reader, "TITLE")?;
            let summary = read_field(&mut reader, "SUMMARY")?;
            let offered = read_offered(&mut reader)?;
            Ok(VoiceOutcome::Restored {
                title,
                summary,
                offered,
            })
        }
        "OUTCOME AMBIGUOUS" => {
            // `read_candidates` consumes the list's terminating END; the
            // daemon encodes exactly one, so expecting a second here read
            // EOF and turned every ambiguous answer into a failure.
            let candidates = read_candidates(&mut reader, "CANDIDATE")?;
            Ok(VoiceOutcome::Ambiguous { candidates })
        }
        "OUTCOME NOTFOUND" => {
            // Same: the RECENT list's END is the response's END.
            let recent = read_candidates(&mut reader, "RECENT")?;
            Ok(VoiceOutcome::NotFound { recent })
        }
        "OUTCOME FAILED" => {
            let reason = read_field(&mut reader, "REASON")?;
            expect_end(&mut reader)?;
            Ok(VoiceOutcome::Failed { reason })
        }
        "OUTCOME NOCOMMAND" => {
            expect_end(&mut reader)?;
            Ok(VoiceOutcome::NoCommand)
        }
        "OUTCOME BRIEF" => {
            let count = read_field(&mut reader, "COUNT")?
                .parse::<usize>()
                .map_err(|e| format!("bad BRIEF count: {e}"))?;
            let narration = read_field(&mut reader, "NARRATION")?;
            expect_end(&mut reader)?;
            Ok(VoiceOutcome::Brief { count, narration })
        }
        other => Err(format!("unknown voice outcome: {other:?}")),
    }
}

/// Parses the daemon's `WORKS` browse-listing protocol.
///
/// ```text
/// WORKS <count>\n
/// WORK <id>\t<status>\t<episodes>\t<title>\t<resume_reason>\n   ×count
/// END\n
/// ```
///
/// Engine unavailability arrives as the ordinary failure statement
/// (`OUTCOME FAILED\nREASON <text>\nEND\n`) and maps to `Err(reason)`.
/// Malformed structure — a wrong count, a tab-count mismatch, a missing
/// END — is refused honestly; it is never guessed at or partially kept.
fn parse_works_response(stream: &UnixStream) -> Result<Vec<WorkEntry>, String> {
    let mut reader = BufReader::new(stream);
    let header = read_line(&mut reader)?;
    if header == "OUTCOME FAILED" {
        let reason = read_field(&mut reader, "REASON")?;
        expect_end(&mut reader)?;
        return Err(reason);
    }
    let count = header
        .strip_prefix("WORKS ")
        .ok_or_else(|| format!("unknown works response: {header:?}"))?
        .parse::<usize>()
        .map_err(|e| format!("bad WORKS count: {e}"))?;
    if count > MAX_LISTED_WORKS {
        return Err(format!("WORKS count {count} exceeds {MAX_LISTED_WORKS}"));
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let line = read_line(&mut reader)?;
        entries.push(parse_work_line(&line)?);
    }
    expect_end(&mut reader)?;
    Ok(entries)
}

/// Parses one tab-separated `WORK` line. Fields are tab-delimited, so a
/// title may contain spaces and unicode punctuation; the id must be the
/// daemon's canonical numeric thread id (digits, no sign or leading
/// zero) and episodes a decimal count. Anything else is malformed.
fn parse_work_line(line: &str) -> Result<WorkEntry, String> {
    let rest = line
        .strip_prefix("WORK ")
        .ok_or_else(|| format!("expected WORK, got {line:?}"))?;
    let fields: Vec<&str> = rest.split('\t').collect();
    if fields.len() != 5 {
        return Err(format!(
            "malformed work line (expected 5 tab-separated fields): {line:?}"
        ));
    }
    let (id, status, episodes, title, resume_reason) =
        (fields[0], fields[1], fields[2], fields[3], fields[4]);
    if id.parse::<u64>().ok().map(|n| n.to_string()).as_deref() != Some(id) {
        return Err(format!("malformed work id: {line:?}"));
    }
    let episodes = episodes
        .parse::<u32>()
        .map_err(|e| format!("malformed work episodes: {line:?}: {e}"))?;
    Ok(WorkEntry {
        id: id.to_string(),
        status: status.to_string(),
        episodes,
        title: title.to_string(),
        resume_reason: resume_reason.to_string(),
    })
}

fn read_line(reader: &mut BufReader<&UnixStream>) -> Result<String, String> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("voice response truncated: {e}"))?;
    Ok(line.trim_end().to_string())
}

fn read_field(reader: &mut BufReader<&UnixStream>, prefix: &str) -> Result<String, String> {
    let line = read_line(reader)?;
    line.strip_prefix(prefix)
        .map(|s| s.strip_prefix(' ').unwrap_or(s).to_string())
        .ok_or_else(|| format!("expected {prefix}, got {line:?}"))
}

/// Reads `<prefix> <work_id>\t<title>` lines until the terminating END,
/// which is consumed here: the list's END is the response's END, so
/// callers must not expect a second one.
fn read_candidates(
    reader: &mut BufReader<&UnixStream>,
    prefix: &str,
) -> Result<Vec<VoiceCandidate>, String> {
    let mut out = Vec::new();
    loop {
        let line = read_line(reader)?;
        if line == "END" {
            return Ok(out);
        }
        let rest = line
            .strip_prefix(prefix)
            .ok_or_else(|| format!("expected {prefix} or END, got {line:?}"))?;
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        let fields: Vec<&str> = rest.split('\t').collect();
        if fields.len() != 2 && fields.len() != 4 {
            return Err(format!("malformed candidate: {line:?}"));
        }
        let work_id = fields[0];
        if work_id
            .parse::<u64>()
            .ok()
            .map(|id| id.to_string())
            .as_deref()
            != Some(work_id)
        {
            return Err(format!("malformed candidate work id: {line:?}"));
        }
        let candidate = if fields.len() == 2 {
            VoiceCandidate::work(work_id, fields[1])
        } else {
            if prefix != "CANDIDATE"
                || fields[2] != "OCCURRENCE"
                || fields[3].len() != 16
                || !fields[3].bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(format!("malformed occurrence candidate: {line:?}"));
            }
            VoiceCandidate::occurrence(work_id, fields[1], fields[3])
        };
        out.push(candidate);
        if out.len() > MAX_READ_CANDIDATES {
            return Err("too many candidates".to_string());
        }
    }
}

fn expect_end(reader: &mut BufReader<&UnixStream>) -> Result<(), String> {
    let line = read_line(reader)?;
    if line == "END" {
        Ok(())
    } else {
        Err(format!("expected END, got {line:?}"))
    }
}

/// Reads OFFERED lines until END. Used for staged restore: resources not
/// restored but available as one-tap.
fn read_offered(reader: &mut BufReader<&UnixStream>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    loop {
        let line = read_line(reader)?;
        if line == "END" {
            return Ok(out);
        }
        let resource = line
            .strip_prefix("OFFERED")
            .ok_or_else(|| format!("expected OFFERED or END, got {line:?}"))?;
        let resource = resource.strip_prefix(' ').unwrap_or(resource);
        out.push(resource.to_string());
    }
}

/// Parses an ordinal reference into a 0-based candidate index.
///
/// Accepts digits ("2", "number 2"), ordinal words ("second",
/// "the second one"), and bare ordinals ("first"). Returns `None`
/// when the text is not an ordinal pick.
pub fn parse_ordinal_pick(text: &str) -> Option<usize> {
    let lower = text.to_lowercase();
    let lower = lower.trim();
    // Strip common wrappers: "the second one" → "second".
    let core = lower
        .strip_prefix("the ")
        .unwrap_or(lower)
        .strip_suffix(" one")
        .unwrap_or(lower.strip_prefix("the ").unwrap_or(lower));
    let core = core.strip_prefix("number ").unwrap_or(core);

    // Digits first.
    if let Ok(n) = core.parse::<usize>() {
        return n.checked_sub(1);
    }
    // Ordinal words.
    let idx = match core {
        "first" | "1st" => 0,
        "second" | "2nd" => 1,
        "third" | "3rd" => 2,
        _ => return None,
    };
    Some(idx)
}

/// State for the floating ambiguity panel.
///
/// The panel is shown when the daemon returns `Ambiguous`. It holds the
/// candidates, the time it appeared (for timeout), and the user's pick
/// (if any). The egui render code reads this; the voice thread's
/// ordinal listener can also resolve a pick.
#[derive(Debug, Default)]
pub struct AmbiguityPanel {
    candidates: Vec<VoiceCandidate>,
    shown_at: Option<Instant>,
    /// Set when the user picks (click, key, or voice ordinal).
    pub picked: Option<usize>,
    /// Set on Esc or timeout — dismiss without restoring.
    pub dismissed: bool,
}

impl AmbiguityPanel {
    /// Shows the panel with these candidates (truncated to the max).
    pub fn show(&mut self, candidates: Vec<VoiceCandidate>) {
        self.candidates = candidates.into_iter().take(MAX_PANEL_CANDIDATES).collect();
        self.shown_at = Some(Instant::now());
        self.picked = None;
        self.dismissed = false;
    }

    /// Whether the panel is currently visible.
    pub fn is_visible(&self) -> bool {
        self.shown_at.is_some() && !self.dismissed && self.picked.is_none()
    }

    /// The visible candidates, numbered from 1.
    pub fn candidates(&self) -> &[VoiceCandidate] {
        &self.candidates
    }

    /// Checks the timeout; dismisses when expired. Returns `true` when
    /// the panel just timed out (caller should stop listening).
    pub fn poll_timeout(&mut self) -> bool {
        if let Some(shown_at) = self.shown_at
            && self.picked.is_none()
            && !self.dismissed
            && shown_at.elapsed() >= AMBIGUITY_TIMEOUT
        {
            self.dismissed = true;
            return true;
        }
        false
    }

    /// Records a user pick by 0-based index. Ignored when out of range
    /// or the panel is not visible.
    pub fn pick(&mut self, index: usize) {
        if self.is_visible() && index < self.candidates.len() {
            self.picked = Some(index);
        }
    }

    /// Dismisses without picking (Esc, timeout, click-away).
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    /// Clears all state after the pick is handled.
    pub fn clear(&mut self) {
        self.candidates.clear();
        self.shown_at = None;
        self.picked = None;
        self.dismissed = false;
    }

    /// Takes the picked candidate, if any, and clears the panel.
    pub fn take_pick(&mut self) -> Option<VoiceCandidate> {
        let idx = self.picked?;
        let candidate = self.candidates.get(idx)?.clone();
        self.clear();
        Some(candidate)
    }
}

/// Spawns the voice loop thread.
///
/// Owns the mic, transcriber, and [`VoiceDriver`]: feeds 16 kHz audio
/// chunks, pumps the driver, and forwards [`VoiceEvent`]s to the UI
/// over `event_sender`. `notch_input`, when `Some`, receives a copy of
/// every driver event for the notch surface (via the driver's observer
/// fan-out). When the receiver is dropped, the thread exits.
///
/// Returns the thread handle. No extra shutdown signalling: the
/// receiver going away is the stop condition.
///
/// Requires the `audio` feature of `evo-voice` (mic + transcription).
///
/// The notch parameter is only meaningful under the `evo-notch` feature;
/// without it callers pass `None` and the argument is ignored.
#[cfg(feature = "voice-audio")]
pub fn spawn_voice_thread(
    storage_root: PathBuf,
    event_sender: Sender<VoiceEvent>,
    notch_input: Option<NotchEventSink>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        voice_thread_main(storage_root, event_sender, notch_input);
    })
}

/// A notch event sink: only assembled when both features are on.
/// Elsewhere this is a unit type that studio code passes as `None`
/// via [`no_notch_sink`].
#[cfg(feature = "evo-notch")]
pub type NotchEventSink = std::sync::mpsc::Sender<crate::notch_window::SurfaceInput>;
#[cfg(not(feature = "evo-notch"))]
pub type NotchEventSink = ();

/// The `None` value callers pass for the notch sink when the feature
/// is off (or when they have no surface).
#[cfg(feature = "evo-notch")]
pub fn no_notch_sink() -> Option<NotchEventSink> {
    None
}
#[cfg(not(feature = "evo-notch"))]
pub fn no_notch_sink() -> Option<NotchEventSink> {
    None
}

#[cfg(feature = "voice-audio")]
fn voice_thread_main(
    storage_root: PathBuf,
    event_sender: Sender<VoiceEvent>,
    notch_input: Option<NotchEventSink>,
) {
    use evo_voice::{Mic, Transcriber, UtteranceTranscriber, VoiceLoopConfig};

    #[cfg(feature = "voice-wake")]
    let acoustic_wake = load_acoustic_wake();

    // Whisper model is bundled near the binary (per product decision).
    // The desktop binary and daemon binary sit side by side; the model
    // sits beside them.
    let transcriber = match model_path(VOICE_MODEL_FILES[0]).and_then(|path| {
        Transcriber::load(&path).map_err(|err| format!("{}: {err}", path.display()))
    }) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("EVO voice transcriber unavailable: {err}");
            return;
        }
    };
    // Adapt Transcriber to the driver's UtteranceTranscriber trait.
    struct TranscriberAdapter(Transcriber);
    impl UtteranceTranscriber for TranscriberAdapter {
        fn transcribe_utterance(&self, audio: &[f32]) -> Result<String, String> {
            self.0.transcribe(audio)
        }
    }
    let transcriber = TranscriberAdapter(transcriber);
    let transport = VoiceSocketClient::new(storage_root);
    let mic = match Mic::open() {
        Ok(m) => m,
        Err(err) => {
            eprintln!("EVO voice mic unavailable: {err}");
            return;
        }
    };
    // Keep the real-time microphone/resampler path independent from whisper
    // and the acoustic wake model. Both can take longer than one hardware
    // callback on a CPU-only Mac. Previously `driver.poll()` ran on this same
    // thread, so a slow inference stopped draining Mic and filled the raw
    // queue until every new chunk was dropped. The capture worker now drains
    // Mic continuously and keeps only a bounded, recent handoff to the
    // inference loop.
    const AUDIO_HANDOFF_CAPACITY: usize = 64;
    let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(AUDIO_HANDOFF_CAPACITY);
    let (audio_stop_tx, audio_stop_rx) = std::sync::mpsc::channel::<()>();
    let audio_drain = std::thread::Builder::new()
        .name("evo-voice-audio-drain".to_string())
        .spawn(move || {
            loop {
                if audio_stop_rx.try_recv().is_ok() {
                    break;
                }
                let samples = match mic.recv_timeout(Duration::from_millis(100)) {
                    Ok(samples) => samples,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                };
                if audio_tx.try_send(samples).is_err() {
                    // Inference is behind. Drop this slice; the inference loop
                    // keeps only the newest queued slice before every poll, so
                    // wake detection never replays an old backlog.
                }
            }
        })
        .ok();
    let mut driver = VoiceDriver::new(
        transcriber,
        transport,
        event_sender.clone(),
        VoiceLoopConfig::default(),
    );
    #[cfg(feature = "voice-wake")]
    if let Some(wake) = acoustic_wake {
        driver = driver.with_acoustic_wake(wake);
    }
    // The notch surface subscribes to the same event stream: the driver
    // fans every event out to observers before the primary sink.
    #[cfg(feature = "evo-notch")]
    if let Some(sink) = notch_input {
        struct NotchForwarder(NotchEventSink);
        impl evo_voice::VoiceEventObserver for NotchForwarder {
            fn observe(&mut self, event: &VoiceEvent) -> bool {
                self.0
                    .send(crate::notch_window::SurfaceInput::Voice(event.clone()))
                    .is_ok()
            }
        }
        driver = driver.with_observer(Box::new(NotchForwarder(sink)));
    }

    // Feed audio until the UI goes away. The dedicated capture worker keeps
    // Mic drained while this loop performs model inference.
    loop {
        if let Ok(mut newest) = audio_rx.recv_timeout(Duration::from_millis(20)) {
            // Discard queued history and retain the newest slice. A slow model
            // must recover to live audio, not work through stale microphone
            // data after the person has stopped speaking.
            while let Ok(samples) = audio_rx.try_recv() {
                newest = samples;
            }
            if !newest.is_empty() {
                driver.ingest(newest);
            }
        }
        if !driver.poll() {
            break;
        }
    }

    // Stop and join the capture worker before this thread is restarted.  A
    // detached worker keeps owning cpal's stream after a voice failure, which
    // leaves competing microphone streams behind and makes the next launch
    // appear deaf.
    let _ = audio_stop_tx.send(());
    if let Some(audio_drain) = audio_drain {
        let _ = audio_drain.join();
    }
}

/// The effective wake threshold: `EVO_WAKE_THRESHOLD` when set, else the
/// spotter's calibrated default. Live-probe evidence beats any constant,
/// so the person can tune without a rebuild.
#[cfg(feature = "voice-wake")]
fn effective_wake_threshold(spotter: &evo_voice::CustomWakeSpotter) -> f32 {
    match std::env::var("EVO_WAKE_THRESHOLD")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        Some(t) => {
            eprintln!("EVO voice: wake threshold from EVO_WAKE_THRESHOLD = {t:.3}");
            t
        }
        None => spotter.threshold(),
    }
}

/// Loads the acoustic wake spotter from the models directory (beside the
/// exe, next to the whisper model). Absent models are a normal runtime
/// configuration, not an error: the whisper keyword-spot fallback runs.
#[cfg(feature = "voice-wake")]
fn load_acoustic_wake() -> Option<Box<dyn evo_voice::AcousticWake>> {
    let dir = match wake_model_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("EVO voice: acoustic wake unavailable ({err}); using whisper fallback");
            return None;
        }
    };
    match evo_voice::CustomWakeSpotter::load(&dir) {
        Ok(spotter) => {
            let threshold = effective_wake_threshold(&spotter);
            eprintln!("EVO voice: acoustic wake model active (threshold {threshold:.3})");
            Some(Box::new(spotter.with_threshold(threshold)))
        }
        Err(err) => {
            eprintln!(
                "EVO voice: acoustic wake unavailable in {} ({err}); using whisper fallback",
                dir.display()
            );
            None
        }
    }
}

/// Receives [`VoiceEvent`]s on the UI thread and routes them.
///
/// Call once per egui frame with the event receiver:
/// - `WakeGesture` → the notch/gesture surface expands (listening mode).
/// - `Listening` → live listening feedback with the newest partial.
/// - `HeardCommand` → show a transient "heard" indicator.
/// - `Outcome(Restored)` → show a brief confirmation toast.
/// - `Outcome(Ambiguous)` → show the ambiguity panel.
/// - `Outcome(NotFound)` → show "couldn't find it" with recent items.
/// - `Outcome(Failed)` → show the honest error.
///
/// Returns the outcome when one arrived this frame (for toast display).
pub fn drain_voice_events(
    receiver: &Receiver<VoiceEvent>,
    panel: &mut AmbiguityPanel,
) -> Vec<VoiceEvent> {
    let mut events = Vec::new();
    while let Ok(event) = receiver.try_recv() {
        if let VoiceEvent::Outcome(VoiceOutcome::Ambiguous { ref candidates }) = event {
            panel.show(candidates.clone());
        }
        events.push(event);
    }
    // Timeout check lives here so it runs every frame.
    panel.poll_timeout();
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notch::NotchEngine;
    use crate::notch_window::{RenderState, SurfaceInput, surface_layouts};
    use evo_voice::{VoiceDriver, VoiceEvent, VoiceEventObserver, VoiceLoopConfig, VoiceOutcome};

    #[cfg(feature = "voice-audio")]
    #[test]
    fn model_search_includes_app_bundle_resources_after_binary_directory() {
        let dirs = model_search_dirs(std::path::Path::new("/tmp/Evo.app/Contents/MacOS/Evo"));
        assert_eq!(
            dirs,
            vec![
                PathBuf::from("/tmp/Evo.app/Contents/MacOS"),
                PathBuf::from("/tmp/Evo.app/Contents/Resources"),
            ]
        );
    }

    #[test]
    fn scope_socket_encodes_utf8_bytes_and_parses_success_or_clarification() {
        use std::io::Read;
        use std::os::unix::net::UnixListener;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        for success in [true, false] {
            let root = std::env::temp_dir().join(format!(
                "evo-scope-client-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            let path = evo_daemon::voice::voice_socket_path(&root);
            let listener = UnixListener::bind(&path).unwrap();
            let query = "file:///work/café notes\npart two.md";
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                assert_eq!(header, format!("SCOPE 42 {}\n", query.len()));
                let mut body = vec![0; query.len()];
                reader.read_exact(&mut body).unwrap();
                assert_eq!(body, query.as_bytes());
                let response = if success {
                    "OUTCOME RESTORED\nTITLE Notes\nSUMMARY Opened 1, raised 0 already open.\nEND\n"
                } else {
                    "OUTCOME FAILED\nREASON Scope matches multiple resources. Retry with an exact locator.\nEND\n"
                };
                stream.write_all(response.as_bytes()).unwrap();
            });
            let result = VoiceSocketClient::new(root).scope("42", query).unwrap();
            server.join().unwrap();
            std::fs::remove_file(path).unwrap();
            if success {
                assert!(
                    matches!(result, VoiceOutcome::Restored { title, summary, offered }
                    if title == "Notes" && summary.contains("Opened 1") && offered.is_empty())
                );
            } else {
                assert!(
                    matches!(result, VoiceOutcome::Failed { reason } if reason.contains("exact locator"))
                );
            }
        }
    }

    #[test]
    fn occurrence_choice_round_trips_opaque_selection_key() {
        use std::os::unix::net::UnixListener;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "evo-occurrence-client-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let path = evo_daemon::voice::voice_socket_path(&root);
        let listener = UnixListener::bind(&path).unwrap();
        let key = "0123456789abcdef";
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            std::io::BufRead::read_line(&mut std::io::BufReader::new(&mut stream), &mut request)
                .unwrap();
            assert_eq!(request, format!("CHOOSE_OCCURRENCE 42 {key}\n"));
            let response = "OUTCOME AMBIGUOUS\nCANDIDATE 42\tPitch deck · occurrence 1\tOCCURRENCE\t0123456789abcdef\nEND\n";
            std::io::Write::write_all(&mut stream, response.as_bytes()).unwrap();
        });
        let outcome = VoiceSocketClient::new(root)
            .choose_occurrence("42", key)
            .unwrap();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        let VoiceOutcome::Ambiguous { candidates } = outcome else {
            panic!("expected occurrence choice candidates");
        };
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].selection,
            VoiceSelection::Occurrence {
                key: key.to_string()
            }
        );
    }

    #[test]
    fn malformed_scope_is_rejected_before_connecting() {
        let client = VoiceSocketClient::new(PathBuf::from("/no-scope-socket"));
        for id in [
            "",
            "01",
            "+1",
            "1\nCHOOSE 2",
            " 1",
            "1 ",
            "18446744073709551616",
        ] {
            assert!(client.scope(id, "notes").unwrap_err().contains("canonical"));
            assert!(
                client
                    .scope_exact(id, "notes")
                    .unwrap_err()
                    .contains("canonical")
            );
        }
        for query in ["", " \n\t", &"é".repeat(2049)] {
            assert!(
                client
                    .scope("42", query)
                    .unwrap_err()
                    .contains("nonempty UTF-8")
            );
            assert!(
                client
                    .scope_exact("42", query)
                    .unwrap_err()
                    .contains("nonempty UTF-8")
            );
        }
    }

    #[test]
    fn trigger_transport_rejects_wake_only_and_placeholder_text_before_connecting() {
        let client =
            VoiceSocketClient::new(PathBuf::from("/no-voice-socket-for-wake-only-regression"));
        for command in ["hey evo", "[BLANK_AUDIO]", "hey evo [ Inaudible ]"] {
            assert_eq!(
                TriggerTransport::trigger(&client, command).unwrap(),
                VoiceOutcome::NoCommand,
                "{command:?} must never reach the daemon transport"
            );
        }
    }

    #[test]
    fn exact_resource_socket_preserves_locator_bytes_and_stale_failure_without_fallback() {
        use std::io::Read;
        use std::os::unix::net::UnixListener;
        let root = std::env::temp_dir().join(format!("evo-resource-client-{}", std::process::id()));
        let path = evo_daemon::voice::voice_socket_path(&root);
        let listener = UnixListener::bind(&path).unwrap();
        let locator = " file:///work/café notes\npart two.md ";
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            assert_eq!(header, format!("RESOURCE 42 {}\n", locator.len()));
            let mut body = vec![0; locator.len()];
            reader.read_exact(&mut body).unwrap();
            assert_eq!(body, locator.as_bytes());
            stream
                .write_all(
                    b"OUTCOME FAILED\nREASON Exact resource no longer belongs to this work.\nEND\n",
                )
                .unwrap();
            listener.set_nonblocking(true).unwrap();
            assert!(
                matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
            );
        });
        let outcome = VoiceSocketClient::new(root)
            .scope_exact("42", locator)
            .unwrap();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(
            matches!(outcome, VoiceOutcome::Failed { reason } if reason == "Exact resource no longer belongs to this work.")
        );
    }

    /// Serves one canned response on a fresh voice socket; asserts the
    /// request verb before answering. Returns the client, the server
    /// handle, and the socket path to clean up.
    fn voice_socket(
        expected_request: &'static str,
        response: &'static str,
    ) -> (VoiceSocketClient, std::thread::JoinHandle<()>, PathBuf) {
        use std::os::unix::net::UnixListener;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "evo-voice-client-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let path = evo_daemon::voice::voice_socket_path(&root);
        let listener = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            assert_eq!(request, expected_request);
            stream.write_all(response.as_bytes()).unwrap();
        });
        (VoiceSocketClient::new(root), server, path)
    }

    fn works_socket(
        response: &'static str,
    ) -> (VoiceSocketClient, std::thread::JoinHandle<()>, PathBuf) {
        voice_socket("WORKS\n", response)
    }

    #[test]
    fn works_socket_parses_the_full_listing_in_engine_order() {
        let (client, server, path) = works_socket(concat!(
            "WORKS 2\n",
            "WORK 7\tactive\t3\tYesterday's notes\tYou left off in Editor.\n",
            "WORK 9\tdormant\t12\tMorning writing\tThree sittings this week.\n",
            "END\n",
        ));
        let entries = client.works().unwrap();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            entries,
            vec![
                WorkEntry {
                    id: "7".into(),
                    status: "active".into(),
                    episodes: 3,
                    title: "Yesterday's notes".into(),
                    resume_reason: "You left off in Editor.".into(),
                },
                WorkEntry {
                    id: "9".into(),
                    status: "dormant".into(),
                    episodes: 12,
                    title: "Morning writing".into(),
                    resume_reason: "Three sittings this week.".into(),
                },
            ]
        );
    }

    #[test]
    fn works_socket_parses_an_empty_listing() {
        let (client, server, path) = works_socket("WORKS 0\nEND\n");
        assert_eq!(client.works().unwrap(), Vec::new());
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn works_socket_maps_a_failed_response_to_the_reason() {
        let (client, server, path) = works_socket(
            "OUTCOME FAILED\nREASON engine unavailable: observation.log is unreadable\nEND\n",
        );
        let error = client.works().unwrap_err();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(error, "engine unavailable: observation.log is unreadable");
    }

    #[test]
    fn works_socket_keeps_spaces_and_unicode_in_titles() {
        let (client, server, path) = works_socket(concat!(
            "WORKS 1\n",
            "WORK 7\tactive\t3\tDraft  two — café notes\tLeft mid-sentence.\n",
            "END\n",
        ));
        let entries = client.works().unwrap();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(entries[0].title, "Draft  two — café notes");
        assert_eq!(entries[0].resume_reason, "Left mid-sentence.");
    }

    #[test]
    fn works_socket_refuses_a_tab_count_mismatch() {
        for response in [
            "WORKS 1\nWORK 7\tactive\t3\tOnly four fields\nEND\n",
            "WORKS 1\nWORK 7\tactive\t3\tTitle\tReason\tone too many\nEND\n",
        ] {
            let (client, server, path) = works_socket(response);
            let error = client.works().unwrap_err();
            server.join().unwrap();
            std::fs::remove_file(path).unwrap();
            assert!(error.contains("expected 5 tab-separated fields"), "{error}");
        }
    }

    #[test]
    fn works_socket_refuses_a_missing_end() {
        let (client, server, path) = works_socket(concat!(
            "WORKS 1\n",
            "WORK 7\tactive\t3\tYesterday's notes\tYou left off in Editor.\n",
        ));
        let error = client.works().unwrap_err();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(error.contains("expected END"), "{error}");
    }

    #[test]
    fn works_socket_refuses_a_count_mismatch() {
        // Declared 2, sent 1: the second WORK read meets END.
        let (client, server, path) = works_socket(concat!(
            "WORKS 2\n",
            "WORK 7\tactive\t3\tYesterday's notes\tYou left off in Editor.\n",
            "END\n",
        ));
        let error = client.works().unwrap_err();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(error.contains("expected WORK"), "{error}");

        // Declared 0, sent 1: the END read meets a WORK line.
        let (client, server, path) = works_socket(concat!(
            "WORKS 0\n",
            "WORK 7\tactive\t3\tYesterday's notes\tYou left off in Editor.\n",
            "END\n",
        ));
        let error = client.works().unwrap_err();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(error.contains("expected END"), "{error}");
    }

    #[test]
    fn works_socket_refuses_non_canonical_ids_and_episodes() {
        for response in [
            "WORKS 1\nWORK 07\tactive\t3\tYesterday's notes\tReason\nEND\n",
            "WORKS 1\nWORK abc\tactive\t3\tYesterday's notes\tReason\nEND\n",
            "WORKS 1\nWORK 7\tactive\tthree\tYesterday's notes\tReason\nEND\n",
        ] {
            let (client, server, path) = works_socket(response);
            let error = client.works().unwrap_err();
            server.join().unwrap();
            std::fs::remove_file(path).unwrap();
            assert!(
                error.contains("malformed work id") || error.contains("malformed work episodes"),
                "{error}"
            );
        }
    }

    /// Regression: `read_candidates` consumes the list's terminating END,
    /// and the daemon encodes exactly one. The parser used to expect a
    /// second END and read EOF instead — every ambiguous or not-found
    /// answer over the socket became a failure, so the choice carousel
    /// could never receive its candidates.
    #[test]
    fn ambiguous_socket_response_carries_its_candidates() {
        let (client, server, path) = voice_socket(
            "ASK\n",
            concat!(
                "OUTCOME AMBIGUOUS\n",
                "CANDIDATE 7\tYesterday's notes\n",
                "CANDIDATE 9\tMorning writing\n",
                "END\n",
            ),
        );
        let outcome = client.ask().unwrap();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        match outcome {
            VoiceOutcome::Ambiguous { candidates } => {
                assert_eq!(candidates.len(), 2);
                assert_eq!(candidates[0].work_id, "7");
                assert_eq!(candidates[1].title, "Morning writing");
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn candidate_parser_reads_a_full_recent_list_up_to_the_wire_bound() {
        // A large NOTFOUND list is an honest miss, not a protocol error:
        // the parser reads up to the daemon's listing bound.
        let full: &'static str = Box::leak(
            {
                let mut response = String::from("OUTCOME NOTFOUND\n");
                for id in 1..=20u32 {
                    response.push_str(&format!("RECENT {id}\tWork {id}\n"));
                }
                response.push_str("END\n");
                response
            }
            .into_boxed_str(),
        );
        let (client, server, path) = voice_socket("ASK\n", full);
        let outcome = client.ask().unwrap();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        match outcome {
            VoiceOutcome::NotFound { recent } => {
                assert_eq!(recent.len(), 20, "a large recent list parses whole")
            }
            other => panic!("expected NotFound, got {other:?}"),
        }

        // Past the bound the wire is refused: never a silently truncated
        // list pretending to be the rack.
        let over: &'static str = Box::leak(
            {
                let mut response = String::from("OUTCOME NOTFOUND\n");
                for id in 1..=25u32 {
                    response.push_str(&format!("RECENT {id}\tWork {id}\n"));
                }
                response.push_str("END\n");
                response
            }
            .into_boxed_str(),
        );
        let (client, server, path) = voice_socket("ASK\n", over);
        let error = client.ask().unwrap_err();
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(error.contains("too many candidates"), "{error}");
    }

    #[test]
    fn ordinal_digits() {
        assert_eq!(parse_ordinal_pick("1"), Some(0));
        assert_eq!(parse_ordinal_pick("2"), Some(1));
        assert_eq!(parse_ordinal_pick("number 2"), Some(1));
        assert_eq!(parse_ordinal_pick("Number 3"), Some(2));
    }

    #[test]
    fn ordinal_words() {
        assert_eq!(parse_ordinal_pick("first"), Some(0));
        assert_eq!(parse_ordinal_pick("second"), Some(1));
        assert_eq!(parse_ordinal_pick("the second one"), Some(1));
        assert_eq!(parse_ordinal_pick("third"), Some(2));
        assert_eq!(parse_ordinal_pick("1st"), Some(0));
    }

    #[test]
    fn ordinal_rejects_non_picks() {
        assert_eq!(parse_ordinal_pick("hello"), None);
        assert_eq!(parse_ordinal_pick("resume my work"), None);
        assert_eq!(parse_ordinal_pick(""), None);
        assert_eq!(parse_ordinal_pick("0"), None);
    }

    #[test]
    fn panel_show_pick_take() {
        let mut panel = AmbiguityPanel::default();
        assert!(!panel.is_visible());
        panel.show(vec![
            VoiceCandidate::work("a", "Alpha"),
            VoiceCandidate::work("b", "Beta"),
        ]);
        assert!(panel.is_visible());
        assert_eq!(panel.candidates().len(), 2);
        panel.pick(1);
        assert!(!panel.is_visible());
        let picked = panel.take_pick().unwrap();
        assert_eq!(picked.work_id, "b");
        assert!(!panel.is_visible());
    }

    #[test]
    fn panel_dismiss() {
        let mut panel = AmbiguityPanel::default();
        panel.show(vec![VoiceCandidate::work("a", "Alpha")]);
        panel.dismiss();
        assert!(!panel.is_visible());
        assert!(panel.take_pick().is_none());
    }

    #[test]
    fn panel_truncates_to_max() {
        let mut panel = AmbiguityPanel::default();
        panel.show(
            (0..10)
                .map(|i| VoiceCandidate::work(i.to_string(), format!("Work {i}")))
                .collect(),
        );
        assert_eq!(panel.candidates().len(), MAX_PANEL_CANDIDATES);
    }

    #[test]
    fn panel_pick_out_of_range_ignored() {
        let mut panel = AmbiguityPanel::default();
        panel.show(vec![VoiceCandidate::work("a", "Alpha")]);
        panel.pick(5);
        assert!(panel.picked.is_none());
        assert!(panel.is_visible());
    }

    /// Proves the fan-out the notch surface depends on: a bare wake emitted
    /// by the driver reaches an observer as the `SurfaceInput::Voice` the
    /// surface thread consumes. The observer here is the same conversion
    /// `NotchForwarder` (in `voice_thread_main`) performs at runtime.
    #[test]
    fn driver_wake_fans_out_to_surface_input() {
        let (capture, received) = std::sync::mpsc::channel();
        let mut driver = make_test_driver(
            vec!["hey evo", "open my writing"],
            VoiceOutcome::Restored {
                title: "x".into(),
                summary: "y".into(),
                offered: vec![],
            },
            SurfaceInputRecorder(capture.clone()),
        );
        driver.ingest(test_chunk());
        assert!(driver.poll(), "bare wake enters command capture");

        let first = received.recv().expect("WakeGesture as SurfaceInput");
        match first {
            SurfaceInput::Voice(VoiceEvent::WakeGesture { score: None }) => {}
            other => panic!("expected WakeGesture, got {other:?}"),
        }
        let second = received.recv().expect("Listening as SurfaceInput");
        match second {
            SurfaceInput::Voice(VoiceEvent::Listening { partial: None }) => {}
            other => panic!("expected Listening(partial=None), got {other:?}"),
        }
        assert!(
            received.try_recv().is_err(),
            "no more events until follow-up"
        );
    }

    /// A `SurfaceInput::Voice(WakeGesture)` fed through the real engine
    /// (the same call `surface_main` makes) produces a `RenderState` whose
    /// layout is the listening surface — not the idle glyph. This is the
    /// surface thread's core input→render path, exercised without the
    /// macOS host (which is covered separately by the pixel tests).
    #[test]
    fn surface_input_wake_yields_the_listening_layout() {
        let (input_tx, input_rx) = std::sync::mpsc::channel();
        // The surface thread pulls SurfaceInput from its channel and feeds
        // each one into the engine. Replay one WakeGesture through that path.
        let _ = input_tx.send(SurfaceInput::Voice(VoiceEvent::WakeGesture { score: None }));
        drop(input_tx);
        let mut engine = NotchEngine::new();
        while let Ok(input) = input_rx.recv() {
            match input {
                SurfaceInput::Voice(event) => engine.voice_event(&event, Instant::now()),
                SurfaceInput::Quit => break,
                _ => {}
            }
            engine.tick(Instant::now());
        }
        let state = RenderState::from_engine(&engine, None);
        assert!(
            state.listening,
            "wake gesture expands the surface (via SurfaceInput)"
        );
        assert!(!state.idle_glyph, "listening preempts the idle glyph");
        let layers = surface_layouts(&state);
        assert!(
            layers.iter().any(|l| l.id == "transcript"),
            "listening surface has a transcript layer"
        );
        assert!(
            layers.iter().any(|l| l.id == "mic_ring"),
            "listening surface has a mic ring"
        );
    }

    /// Local test fakes mirroring the pattern from evo-voice's own driver
    /// tests, so this integration test is self-contained in evo-desktop.
    struct SurfaceInputRecorder(std::sync::mpsc::Sender<SurfaceInput>);

    impl VoiceEventObserver for SurfaceInputRecorder {
        fn observe(&mut self, event: &VoiceEvent) -> bool {
            self.0.send(SurfaceInput::Voice(event.clone())).is_ok()
        }
    }

    struct TestTranscriber(std::sync::Mutex<Vec<String>>);

    impl evo_voice::UtteranceTranscriber for TestTranscriber {
        fn transcribe_utterance(&self, _audio: &[f32]) -> Result<String, String> {
            Ok(self.0.lock().unwrap().pop().unwrap_or_default())
        }
    }

    struct TestTransport(VoiceOutcome);

    impl evo_voice::TriggerTransport for TestTransport {
        fn trigger(&self, _command: &str) -> Result<VoiceOutcome, String> {
            Ok(self.0.clone())
        }
        fn ask(&self) -> Result<VoiceOutcome, String> {
            Ok(self.0.clone())
        }
    }

    fn make_test_driver(
        script: Vec<&str>,
        outcome: VoiceOutcome,
        observer: SurfaceInputRecorder,
    ) -> VoiceDriver<TestTranscriber, TestTransport> {
        // The desktop observer test is interested in the fan-out, but the
        // driver still requires a live primary UI sink: a disconnected owner
        // is the driver's deliberate shutdown signal.
        let (sink, events) = std::sync::mpsc::channel();
        std::thread::spawn(move || while events.recv().is_ok() {});
        VoiceDriver::new(
            TestTranscriber(std::sync::Mutex::new(
                script.into_iter().rev().map(String::from).collect(),
            )),
            TestTransport(outcome),
            sink,
            VoiceLoopConfig {
                scan_interval: std::time::Duration::ZERO,
                scan_window: std::time::Duration::from_millis(50),
                command_capture: std::time::Duration::from_millis(1),
                max_buffer: std::time::Duration::from_secs(30),
                speech_rms_threshold: 0.0,
            },
        )
        .with_observer(Box::new(observer))
    }

    fn test_chunk() -> Vec<f32> {
        vec![0.0; 1600]
    }
}
