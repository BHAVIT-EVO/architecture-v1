//! Executable-target classification from canonical evidence.
//!
//! An executable target is never invented. It is derived deterministically
//! from the witnessed subject of a canonical Observation, classified by the
//! Observation's frozen schema (IS-0003):
//!
//! - `OBS-WINDOW-FOCUS-GAINED` → the subject is a window title;
//! - `OBS-FILE-SAVED` → the subject is a file path;
//! - `OBS-URL-NAVIGATED` → the subject is a URL;
//! - `OBS-COMMIT-MADE` → a commit hash is not an executable resource; no
//!   target is classified.
//!
//! Classification is deterministic, replayable, and explainable entirely from
//! canonical evidence. It never consults runtime state, recency, focus,
//! titles as meaning, or any prohibited signal.

use evo_artifact::artifact_id::ArtifactId;

/// The frozen canonical Observation schema names (IS-0003).
pub const SCHEMA_WINDOW_FOCUS_GAINED: &str = "OBS-WINDOW-FOCUS-GAINED";
pub const SCHEMA_FILE_SAVED: &str = "OBS-FILE-SAVED";
pub const SCHEMA_URL_NAVIGATED: &str = "OBS-URL-NAVIGATED";
pub const SCHEMA_COMMIT_MADE: &str = "OBS-COMMIT-MADE";

/// The kind of executable target a canonical subject classifies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocatorKind {
    /// The subject is the title of a window (OBS-WINDOW-FOCUS-GAINED).
    WindowTitle(String),
    /// The subject is a file path (OBS-FILE-SAVED).
    FilePath(String),
    /// The subject is a URL (OBS-URL-NAVIGATED).
    Url(String),
}

/// A canonical executable-target reference for one Artifact.
///
/// This is a derived value, never a new primitive, never a field on any
/// frozen model, and never persisted as ground truth. It is produced by
/// [`classify_locator`] from canonical Observation evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locator {
    artifact_id: ArtifactId,
    kind: LocatorKind,
}

impl Locator {
    /// Constructs a locator for one Artifact.
    pub fn new(artifact_id: ArtifactId, kind: LocatorKind) -> Self {
        Self { artifact_id, kind }
    }

    /// The Artifact this locator refers to.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// The classified executable-target kind.
    pub fn kind(&self) -> &LocatorKind {
        &self.kind
    }
}

/// Classifies a canonical Observation subject into an executable target kind.
///
/// Deterministic: the same schema name and subject always produce the same
/// classification. `None` means the frozen schema carries no executable
/// resource (e.g. a commit hash), so no target exists.
pub fn classify_locator(schema_name: &str, subject: &str) -> Option<LocatorKind> {
    match schema_name {
        SCHEMA_WINDOW_FOCUS_GAINED => Some(LocatorKind::WindowTitle(subject.to_string())),
        SCHEMA_FILE_SAVED => Some(LocatorKind::FilePath(subject.to_string())),
        SCHEMA_URL_NAVIGATED => Some(LocatorKind::Url(subject.to_string())),
        _ => None,
    }
}

/// Classifies a bare resource string — no schema, no provenance — by its
/// structure alone. The threads engine holds resources as opaque strings;
/// when the executor must act on one, this is the whole rule:
///
/// * `file://…` and absolute paths name files;
/// * anything with a URL scheme (`https://…`) names a URL;
/// * everything else is a window title.
///
/// No application names, no extensions, no directory knowledge: the
/// string's shape is the only witness, and an unclassifiable shape is a
/// title — the one kind that can still be raised if it is live.
pub fn classify_subject(subject: &str) -> LocatorKind {
    if let Some(path) = subject.strip_prefix("file://") {
        return LocatorKind::FilePath(path.to_string());
    }
    if subject.contains("://") {
        return LocatorKind::Url(subject.to_string());
    }
    if subject.starts_with('/') {
        return LocatorKind::FilePath(subject.to_string());
    }
    LocatorKind::WindowTitle(subject.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_focus_classifies_to_title() {
        let kind = classify_locator(SCHEMA_WINDOW_FOCUS_GAINED, "Design Brief — Canvas");
        assert_eq!(
            kind,
            Some(LocatorKind::WindowTitle("Design Brief — Canvas".into()))
        );
    }

    #[test]
    fn file_saved_classifies_to_path() {
        let kind = classify_locator(SCHEMA_FILE_SAVED, "/Users/me/work/report.md");
        assert_eq!(
            kind,
            Some(LocatorKind::FilePath("/Users/me/work/report.md".into()))
        );
    }

    #[test]
    fn url_navigated_classifies_to_url() {
        let kind = classify_locator(SCHEMA_URL_NAVIGATED, "https://example.com/doc");
        assert_eq!(
            kind,
            Some(LocatorKind::Url("https://example.com/doc".into()))
        );
    }

    #[test]
    fn commit_made_has_no_executable_target() {
        // A commit hash is not an executable resource; no target exists.
        assert_eq!(classify_locator(SCHEMA_COMMIT_MADE, "abc123"), None);
    }

    #[test]
    fn unknown_schema_has_no_executable_target() {
        assert_eq!(classify_locator("OBS-UNKNOWN", "anything"), None);
    }

    #[test]
    fn classification_is_deterministic() {
        for subject in ["Design Brief — Canvas", "Untitled", "Notes – 28 notes"] {
            let a = classify_locator(SCHEMA_WINDOW_FOCUS_GAINED, subject);
            let b = classify_locator(SCHEMA_WINDOW_FOCUS_GAINED, subject);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn locator_carries_artifact_and_kind() {
        let id = ArtifactId::new("artifact-abc").unwrap();
        let locator = Locator::new(id.clone(), LocatorKind::Url("https://x.dev".into()));
        assert_eq!(locator.artifact_id(), &id);
        assert_eq!(locator.kind(), &LocatorKind::Url("https://x.dev".into()));
    }
}
