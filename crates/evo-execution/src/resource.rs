//! Resource Identity — the derived, app-agnostic identity of the concrete
//! external resource a canonical Artifact refers to.
//!
//! This is the Execution layer's answer to "what concrete external resource
//! does this Artifact refer to, and can Evo identify it again?" It is
//! derived deterministically from canonical evidence — the frozen
//! Observation schema and the witnessed subject (IS-0003) — and nothing
//! else. It is NOT Artifact identity (IS-0010), NOT Workspace identity, NOT
//! the Continuation Surface, and NOT an executable target. RFC-0010
//! (Resource Identity Boundary) keeps these separate concepts distinct.
//!
//! # Stability across tool switches
//!
//! Resource identity is the resource, never the application: a file path, a
//! URL, or a window title carries its own identity regardless of which
//! application was used to witness it. A tool switch (Editor A → Editor B on
//! the same file) never changes the file's resource identity, because the
//! identity derives from the canonical subject, never from application,
//! process, or window-owner identity. No application name, domain, or
//! profession appears anywhere in this module.
//!
//! # Identity vs executable target
//!
//! Every content Artifact has a resource identity. Only some have an
//! executable target — the derived "can it be reopened, and how?" A commit
//! is a real external resource (a repository object) with identity but no
//! executable target: Evo never guesses how to "open" a commit.

use crate::locator::{
    LocatorKind, SCHEMA_COMMIT_MADE, SCHEMA_FILE_SAVED, SCHEMA_URL_NAVIGATED,
    SCHEMA_WINDOW_FOCUS_GAINED,
};

/// The derived identity of the concrete external resource a canonical
/// Artifact refers to.
///
/// Each variant carries the canonical witnessed subject (the frozen-schema
/// subject, IS-0003 §4.3) — never an application or platform identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceIdentity {
    /// A window, identified by its exact witnessed title (OBS-WINDOW-FOCUS-GAINED).
    WindowTitle(String),
    /// A file, identified by its witnessed path (OBS-FILE-SAVED).
    FilePath(String),
    /// A URL, identified by its witnessed address (OBS-URL-NAVIGATED).
    Url(String),
    /// A repository commit, identified by its witnessed hash (OBS-COMMIT-MADE).
    /// A real resource with no executable target.
    Commit(String),
}

impl ResourceIdentity {
    /// The canonical witnessed subject of this resource.
    pub fn subject(&self) -> &str {
        match self {
            ResourceIdentity::WindowTitle(subject)
            | ResourceIdentity::FilePath(subject)
            | ResourceIdentity::Url(subject)
            | ResourceIdentity::Commit(subject) => subject,
        }
    }

    /// The executable target of this resource, when one exists.
    ///
    /// `None` means the resource is not reopenable (a commit is a repository
    /// object, not an OS-launchable resource). The target is derived
    /// deterministically from the identity; it is never guessed and never
    /// substituted with a different resource.
    pub fn executable_target(&self) -> Option<LocatorKind> {
        match self {
            ResourceIdentity::WindowTitle(title) => Some(LocatorKind::WindowTitle(title.clone())),
            ResourceIdentity::FilePath(path) => Some(LocatorKind::FilePath(path.clone())),
            ResourceIdentity::Url(url) => Some(LocatorKind::Url(url.clone())),
            ResourceIdentity::Commit(_) => None,
        }
    }

    /// Whether this resource can be reopened through an executable target.
    pub fn is_restorable(&self) -> bool {
        self.executable_target().is_some()
    }

    /// A stable plain-language label for the resource kind. Presentation
    /// only; derived from the canonical schema classification.
    pub fn kind_label(&self) -> &'static str {
        match self {
            ResourceIdentity::WindowTitle(_) => "focused window",
            ResourceIdentity::FilePath(_) => "saved file",
            ResourceIdentity::Url(_) => "visited URL",
            ResourceIdentity::Commit(_) => "commit",
        }
    }
}

/// Classifies a canonical (schema name, witnessed subject) pair into the
/// resource identity it establishes.
///
/// Deterministic and replayable: the same schema and subject always produce
/// the same identity. `None` means no frozen schema in this build classifies
/// the pair — the resource identity is unknown, and the caller must treat it
/// as unavailable rather than guess (Law V, Law VI).
///
/// # Universality
///
/// This is the single classification the Execution layer uses for every
/// canonical Artifact. A future collector that emits a new frozen schema
/// participates in selective restoration by adding one arm here — the
/// selection engine itself is schema-agnostic and never needs rewriting.
pub fn classify_resource_identity(schema_name: &str, subject: &str) -> Option<ResourceIdentity> {
    match schema_name {
        SCHEMA_WINDOW_FOCUS_GAINED => Some(ResourceIdentity::WindowTitle(subject.to_string())),
        SCHEMA_FILE_SAVED => Some(ResourceIdentity::FilePath(subject.to_string())),
        SCHEMA_URL_NAVIGATED => Some(ResourceIdentity::Url(subject.to_string())),
        SCHEMA_COMMIT_MADE => Some(ResourceIdentity::Commit(subject.to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_focus_classifies_to_window_identity() {
        let identity =
            classify_resource_identity(SCHEMA_WINDOW_FOCUS_GAINED, "Design Brief — Canvas")
                .expect("window focus is a classified resource");
        assert_eq!(
            identity,
            ResourceIdentity::WindowTitle("Design Brief — Canvas".into())
        );
        assert!(identity.is_restorable());
        assert_eq!(
            identity.executable_target(),
            Some(LocatorKind::WindowTitle("Design Brief — Canvas".into()))
        );
        assert_eq!(identity.kind_label(), "focused window");
    }

    #[test]
    fn file_save_classifies_to_file_identity() {
        let identity = classify_resource_identity(SCHEMA_FILE_SAVED, "/Users/me/work/report.md")
            .expect("file save is a classified resource");
        assert_eq!(
            identity,
            ResourceIdentity::FilePath("/Users/me/work/report.md".into())
        );
        assert!(identity.is_restorable());
        assert_eq!(
            identity.executable_target(),
            Some(LocatorKind::FilePath("/Users/me/work/report.md".into()))
        );
    }

    #[test]
    fn url_navigation_classifies_to_url_identity() {
        let identity = classify_resource_identity(SCHEMA_URL_NAVIGATED, "https://example.com/doc")
            .expect("url navigation is a classified resource");
        assert_eq!(
            identity,
            ResourceIdentity::Url("https://example.com/doc".into())
        );
        assert!(identity.is_restorable());
        assert_eq!(
            identity.executable_target(),
            Some(LocatorKind::Url("https://example.com/doc".into()))
        );
    }

    #[test]
    fn commit_has_identity_but_no_executable_target() {
        let hash = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
        let identity =
            classify_resource_identity(SCHEMA_COMMIT_MADE, hash).expect("commit is classified");
        assert_eq!(identity, ResourceIdentity::Commit(hash.into()));
        // A commit is a real resource with identity, but Evo never guesses
        // how to "open" it.
        assert!(!identity.is_restorable());
        assert_eq!(identity.executable_target(), None);
        assert_eq!(identity.subject(), hash);
        assert_eq!(identity.kind_label(), "commit");
    }

    #[test]
    fn unknown_schema_has_no_identity() {
        // An unknown schema is honest unavailability, never a guess.
        assert_eq!(classify_resource_identity("OBS-UNKNOWN", "anything"), None);
    }

    #[test]
    fn classification_is_deterministic_and_app_agnostic() {
        for subject in ["Design Brief — Canvas", "/tmp/report.md", "https://x.dev"] {
            let by_schema = [
                (
                    SCHEMA_WINDOW_FOCUS_GAINED,
                    ResourceIdentity::WindowTitle(subject.into()),
                ),
                (
                    SCHEMA_FILE_SAVED,
                    ResourceIdentity::FilePath(subject.into()),
                ),
                (SCHEMA_URL_NAVIGATED, ResourceIdentity::Url(subject.into())),
            ];
            for (schema, expected) in by_schema {
                let first = classify_resource_identity(schema, subject);
                let second = classify_resource_identity(schema, subject);
                assert_eq!(first, second);
                assert_eq!(first, Some(expected));
            }
        }
    }

    #[test]
    fn identity_carries_only_the_canonical_subject() {
        // The identity is the resource itself: no application, process, or
        // platform field exists. Two artifacts witnessing the same subject
        // through the same schema are the same resource.
        let a = classify_resource_identity(SCHEMA_FILE_SAVED, "/repo/src/lib.rs");
        let b = classify_resource_identity(SCHEMA_FILE_SAVED, "/repo/src/lib.rs");
        assert_eq!(a, b);
        // A tool switch never changes the identity: the same file witnessed
        // again (as it would be by a different editor) is the same resource.
        let again = classify_resource_identity(SCHEMA_FILE_SAVED, "/repo/src/lib.rs");
        assert_eq!(a, again);
    }
}
