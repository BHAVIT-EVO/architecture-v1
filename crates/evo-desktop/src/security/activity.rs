//! The Activity log: what Evo did, when, why, and whether it can be undone.
//!
//! Two rules from brief §9 shape this file:
//!
//! - the log is for the user, so it says what and why in plain sentences;
//! - the log is not a telemetry stream, so a home directory is shortened to
//!   `~` and anything longer than [`MAX_TEXT`] is cut. No transcripts are ever
//!   written here.

use std::collections::VecDeque;

/// The most recent entries kept. Older ones fall off the back.
pub const CAPACITY: usize = 100;

/// The longest one field may be, in characters.
pub const MAX_TEXT: usize = 160;

/// An action that can be reversed from Activity.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UndoKind {
    Hide,
    Merge,
    Rename,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Undo {
    pub kind: UndoKind,
    pub id: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Entry {
    /// Seconds since the Unix epoch; the UI turns this into "Just now".
    pub at: u64,
    pub action: String,
    pub reason: String,
    pub undo: Option<Undo>,
}

#[derive(Clone, Debug)]
pub struct ActivityLog {
    entries: VecDeque<Entry>,
    capacity: usize,
}

impl Default for ActivityLog {
    fn default() -> Self {
        Self::new(CAPACITY)
    }
}

impl ActivityLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    /// Records one action. Text is flattened to a single line and truncated,
    /// so a work title can never forge a second row.
    pub fn record(&mut self, at: u64, action: &str, reason: &str, undo: Option<Undo>) {
        self.entries.push_front(Entry {
            at,
            action: one_line(action),
            reason: one_line(reason),
            undo,
        });
        while self.entries.len() > self.capacity {
            self.entries.pop_back();
        }
    }

    /// Newest first.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Removes the entry and hands back what to undo. `None` when the entry
    /// has gone or was never reversible — undo is offered only once.
    pub fn undo(&mut self, index: usize) -> Option<Undo> {
        let entry = self.entries.get(index)?;
        let undo = entry.undo.clone()?;
        self.entries.remove(index);
        Some(undo)
    }
}

/// Shortens a home directory to `~`, then flattens and truncates.
pub fn redact(text: &str, home: &str) -> String {
    let shortened = if !home.is_empty() {
        text.replace(home, "~")
    } else {
        text.to_string()
    };
    one_line(&shortened)
}

fn one_line(text: &str) -> String {
    let flat: String = text
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
    if trimmed.chars().count() <= MAX_TEXT {
        return trimmed.to_string();
    }
    let mut cut: String = trimmed.chars().take(MAX_TEXT - 1).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    fn undo(id: &str) -> Option<Undo> {
        Some(Undo {
            kind: UndoKind::Hide,
            id: id.to_string(),
        })
    }

    #[test]
    fn newest_first_and_capped() {
        let mut log = ActivityLog::new(3);
        for i in 0..5 {
            log.record(
                i,
                &format!("Hid work {i}"),
                "You asked Home to hide it",
                undo(&format!("w{i}")),
            );
        }
        assert_eq!(log.len(), 3);
        let actions: Vec<&str> = log.entries().map(|e| e.action.as_str()).collect();
        assert_eq!(actions, ["Hid work 4", "Hid work 3", "Hid work 2"]);
    }

    #[test]
    fn undo_happens_at_most_once() {
        let mut log = ActivityLog::new(4);
        log.record(
            1,
            "Hid “Q3 roadmap”",
            "You asked Home to hide it",
            undo("q3"),
        );
        assert!(log.undo(0).is_some());
        assert!(
            log.undo(0).is_none(),
            "the entry is gone, so nothing is left to undo"
        );
        assert!(log.is_empty());
    }

    #[test]
    fn an_entry_without_undo_stays_put() {
        let mut log = ActivityLog::new(4);
        log.record(1, "Opened “Q3 roadmap”", "You searched for it", None);
        assert!(log.undo(0).is_none());
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn text_cannot_forge_a_second_entry() {
        let mut log = ActivityLog::new(2);
        log.record(1, "Hid\nEvil row", "because\twhy", None);
        let entry = log.entries().next().unwrap();
        assert_eq!(entry.action, "Hid Evil row");
        assert_eq!(entry.reason, "because why");
    }

    #[test]
    fn a_home_path_is_shortened_and_long_text_cut() {
        let redacted = redact("/Users/me/Documents/secret.md was read", "/Users/me");
        assert_eq!(redacted, "~/Documents/secret.md was read");

        let long = "a".repeat(400);
        let cut = redact(&long, "");
        assert_eq!(cut.chars().count(), MAX_TEXT);
        assert!(cut.ends_with('…'));
    }
}
