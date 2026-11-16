//! Reliability and observability (brief §10).
//!
//! Three small things, all local:
//!
//! - a structured log: one JSON object per line, rotated by size, with the
//!   home directory shortened and every value truncated;
//! - a panic hook that prints the panic and, only when crash reports are
//!   opted into, writes the same record to a file;
//! - an analytics counter set that is **off by default**, accepts only the
//!   published event names in [`EVENTS`], and never records content.
//!
//! Nothing here opens a socket. In this build an opted-in analytics record is
//! written next to the log and goes nowhere; `docs/ANALYTICS.md` says so.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Once;

pub const LOG_FILE: &str = "evo.jsonl";
pub const CRASH_FILE: &str = "crashes.jsonl";

/// Rotate when the live file passes this size.
pub const MAX_BYTES: u64 = 1 << 20;
/// Archives kept beside the live file.
pub const KEEP: usize = 2;
/// The longest field value written.
pub const MAX_FIELD: usize = 200;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

/// The whole analytics vocabulary. Adding a name here is a documented act.
pub const EVENTS: [&str; 8] = [
    "app_launch",
    "onboarding_finished",
    "work_opened",
    "search_used",
    "voice_wake",
    "voice_failed",
    "pause_toggled",
    "report_opened",
];

/// The log directory used when the shell has no storage root yet.
pub fn default_log_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join("Library/Application Support/Evo/logs")
}

/// Shortens the home directory and flattens a value.
pub fn field_value(value: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let shortened = if home.is_empty() {
        value.to_string()
    } else {
        value.replace(&home, "~")
    };
    let flat: String = shortened
        .chars()
        .map(|c| {
            if c == '\n' || c == '\r' || c == '\t' {
                ' '
            } else {
                c
            }
        })
        .collect();
    let trimmed = flat.trim();
    if trimmed.chars().count() <= MAX_FIELD {
        return trimmed.to_string();
    }
    let mut cut: String = trimmed.chars().take(MAX_FIELD - 1).collect();
    cut.push('…');
    cut
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// One JSON object, in the field order the reader sees.
pub fn record(level: Level, event: &str, fields: &[(&str, &str)]) -> String {
    let mut out = format!(
        "{{\"level\":\"{}\",\"event\":\"{}\"",
        level.as_str(),
        escape(event)
    );
    for (key, value) in fields {
        out.push_str(&format!(
            ",\"{}\":\"{}\"",
            escape(key),
            escape(&field_value(value))
        ));
    }
    out.push('}');
    out
}

/// A size-rotated JSON-lines log.
#[derive(Clone, Debug)]
pub struct Logger {
    dir: PathBuf,
    max_bytes: u64,
    keep: usize,
}

impl Logger {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            max_bytes: MAX_BYTES,
            keep: KEEP,
        }
    }

    pub fn with_limits(dir: impl Into<PathBuf>, max_bytes: u64, keep: usize) -> Self {
        Self {
            dir: dir.into(),
            max_bytes,
            keep: keep.max(1),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(LOG_FILE)
    }

    /// Writes one line. Failure is reported as `false` and never panics: a
    /// full disk must not take the UI down.
    pub fn write(&self, line: &str) -> bool {
        if fs::create_dir_all(&self.dir).is_err() {
            return false;
        }
        if self.over_limit() {
            self.rotate();
        }
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path())
        {
            Ok(mut file) => writeln!(file, "{line}").is_ok(),
            Err(_) => false,
        }
    }

    pub fn event(&self, level: Level, event: &str, fields: &[(&str, &str)]) -> bool {
        self.write(&record(level, event, fields))
    }

    fn over_limit(&self) -> bool {
        fs::metadata(self.path())
            .map(|meta| meta.len() >= self.max_bytes)
            .unwrap_or(false)
    }

    /// `evo.jsonl` → `evo.1.jsonl`; older archives shift down and the last is
    /// removed. The live file always starts empty afterwards.
    fn rotate(&self) {
        for index in (1..self.keep).rev() {
            let from = self.dir.join(format!("evo.{index}.jsonl"));
            let to = self.dir.join(format!("evo.{}.jsonl", index + 1));
            if from.exists() {
                let _ = fs::rename(from, to);
            }
        }
        let first = self.dir.join("evo.1.jsonl");
        let _ = fs::rename(self.path(), first);
        let oldest = self.dir.join(format!("evo.{}.jsonl", self.keep + 1));
        let _ = fs::remove_file(oldest);
    }
}

const PANIC_ONCE: Once = Once::new();

/// Installs the panic hook. The message always goes to stderr; when
/// `report_crashes` is true it is also appended to `crashes.jsonl`.
pub fn install_panic_hook(dir: PathBuf, report_crashes: bool) {
    PANIC_ONCE.call_once(move || {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let message = panic_message(info);
            let location = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "unknown".to_string());
            eprintln!("EVO-PANIC: {message} at {location}");
            if report_crashes {
                write_crash(&dir, &message, &location);
            }
            default_hook(info);
        }));
    });
}

fn panic_message(info: &std::panic::PanicHookInfo<'_>) -> String {
    if let Some(text) = info.payload().downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = info.payload().downcast_ref::<String>() {
        text.clone()
    } else {
        "panic with a non-text payload".to_string()
    }
}

fn write_crash(dir: &Path, message: &str, location: &str) -> bool {
    let _ = fs::create_dir_all(dir);
    let line = record(
        Level::Error,
        "panic",
        &[("message", message), ("at", location)],
    );
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(CRASH_FILE))
        .map(|mut file| writeln!(file, "{line}").is_ok())
        .unwrap_or(false)
}

/// Opt-in analytics. Off by default; the flag is the user's, not the build's.
#[derive(Clone, Debug, Default)]
pub struct Analytics {
    opted_in: bool,
    counts: BTreeMap<String, u32>,
}

impl Analytics {
    pub fn new(opted_in: bool) -> Self {
        Self {
            opted_in,
            counts: BTreeMap::new(),
        }
    }

    pub fn is_opted_in(&self) -> bool {
        self.opted_in
    }

    /// Tallies one event. Unknown names are refused outright, so a caller
    /// cannot invent a metric; opted out means nothing is stored at all.
    pub fn record(&mut self, event: &str) -> bool {
        if !EVENTS.contains(&event) || !self.opted_in {
            return false;
        }
        *self.counts.entry(event.to_string()).or_insert(0) += 1;
        true
    }

    pub fn counts(&self) -> &BTreeMap<String, u32> {
        &self.counts
    }
}

/// What Diagnostics shows about the local records.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct LogFacts {
    pub files: usize,
    pub bytes: u64,
    pub crashes: usize,
}

pub fn log_facts(dir: &Path) -> LogFacts {
    let mut facts = LogFacts::default();
    let Ok(entries) = fs::read_dir(dir) else {
        return facts;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.starts_with("evo") && name.ends_with(".jsonl") {
            facts.files += 1;
            facts.bytes += meta.len();
        }
        if name == CRASH_FILE {
            facts.crashes = fs::read_to_string(&path)
                .map(|text| text.lines().filter(|l| !l.trim().is_empty()).count())
                .unwrap_or(0);
        }
    }
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-obs-{label}-{nanos}"))
    }

    #[test]
    fn one_line_of_json_escapes_the_hostile_parts() {
        let line = record(
            Level::Warn,
            "report_opened",
            &[("note", "a \"quoted\"\nthing")],
        );
        assert!(line.starts_with("{\"level\":\"warn\",\"event\":\"report_opened\""));
        assert!(line.contains("a \\\"quoted\\\" thing"));
        assert_eq!(line.lines().count(), 1);
        assert!(line.ends_with('}'));
    }

    #[test]
    fn values_are_shortened_and_cut() {
        let long = "x".repeat(500);
        assert_eq!(field_value(&long).chars().count(), MAX_FIELD);
        assert!(field_value(&long).ends_with('…'));
    }

    #[test]
    fn rotation_keeps_the_live_file_and_the_archives() {
        let dir = temp_dir("rotate");
        let logger = Logger::with_limits(&dir, 120, 2);
        for i in 0..40 {
            assert!(logger.event(Level::Info, "app_launch", &[("n", &i.to_string())]));
        }
        let live = fs::metadata(dir.join(LOG_FILE))
            .map(|m| m.len())
            .unwrap_or(0);
        assert!(
            live < 200,
            "the live file was rotated below the limit, got {live}"
        );
        assert!(dir.join("evo.1.jsonl").exists(), "one archive exists");
        assert!(
            dir.join("evo.2.jsonl").exists(),
            "the second archive exists"
        );
        assert!(
            !dir.join("evo.3.jsonl").exists(),
            "older archives are deleted"
        );
        let facts = log_facts(&dir);
        assert_eq!(facts.files, 3);
        assert!(facts.bytes > 0);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_log_directory_is_not_an_error() {
        let dir = temp_dir("missing").join("nested");
        let logger = Logger::new(&dir);
        assert!(logger.event(Level::Info, "app_launch", &[]));
        assert!(dir.join(LOG_FILE).exists());
        fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    #[test]
    fn analytics_is_off_by_default_and_only_knows_published_events() {
        let mut off = Analytics::new(false);
        assert!(!off.record("work_opened"));
        assert!(off.counts().is_empty());

        let mut on = Analytics::new(true);
        assert!(on.record("work_opened"));
        assert!(
            !on.record("work_opened_with_title"),
            "unpublished names are refused"
        );
        assert!(!on.record("transcript"));
        assert_eq!(on.counts().get("work_opened"), Some(&1));
        assert_eq!(on.counts().len(), 1);
    }

    #[test]
    fn a_crash_record_is_written_only_when_asked() {
        let dir = temp_dir("crash");
        assert!(write_crash(&dir, "boom", "src/x.rs:1"));
        let text = fs::read_to_string(dir.join(CRASH_FILE)).unwrap();
        assert!(text.contains("\"event\":\"panic\""));
        assert!(text.contains("boom"));
        assert_eq!(log_facts(&dir).crashes, 1);
        fs::remove_dir_all(&dir).ok();
    }
}
