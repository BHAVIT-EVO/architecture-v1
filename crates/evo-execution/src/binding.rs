//! Live Resource Binding — the **non-canonical**, rebuildable association
//! between a canonical Artifact and the concrete native resource that is
//! observable *right now*.
//!
//! # What this is, and what it must never become
//!
//! Canonical understanding answers "what is this work, and what does it refer
//! to". [`crate::resource::ResourceIdentity`] answers "which external resource
//! is that, app-agnostically". This module answers a third, strictly
//! execution-time question: **"where is that resource on this machine at this
//! instant, and what can Evo truthfully do to it?"**
//!
//! A binding is therefore:
//!
//! - **not identity** — it never becomes an Artifact identity, never a second
//!   canonical identity, and never the source of truth for what the work IS;
//! - **not persisted** — no OS window handle, process id, or AX element is ever
//!   written to canonical storage (Architectural Law XVI: derived values, not
//!   objects). A binding is rebuilt from live probing every single time;
//! - **not an input to derivation** — exactly as IS-0021 §7 permits, execution
//!   MAY inspect current OS state, but that state SHALL NOT flow back into
//!   Restoration Derivation, continuation evidence, or membership.
//!
//! Nothing here mutates canonical membership. An Artifact shared by two bodies
//! of work binds identically for both: the binding is a property of the
//! *resource on this machine*, never of the work that asked for it.
//!
//! # The capability order
//!
//! Binding is what turns the flat question "open this" into the honest,
//! tiered question "is it already here?":
//!
//! 1. **Focus an existing native window** — the resource is already open, so
//!    Evo raises what the user already has instead of making a second copy of
//!    it. Two kinds of evidence can establish this, both generic:
//!    - the exact witnessed window title ([`BoundResource::FocusWindow`]);
//!    - the Accessibility `AXDocument` attribute, by which the OS *itself*
//!      reports which window is displaying which file
//!      ([`BoundResource::FocusDocumentWindow`]).
//! 2. **Open a known file path or URL** through supported system mechanisms
//!    ([`BoundResource::OpenFile`], [`BoundResource::OpenUrl`]).
//!
//! There is no tier that guesses. A window title is matched exactly and must
//! be unique; a document is matched by the path the OS reports, not by
//! resemblance to a window title; and no application-specific knowledge
//! appears anywhere in this module.
//!
//! # Honest capability boundaries
//!
//! Two limits are recorded in the type system rather than hidden:
//!
//! - **A file may be open in more than one window.** Evo does not choose
//!   between them ([`WitnessedOpen::SeveralWindows`]); it hands the file to the
//!   system, which decides — the same act as opening it from the Finder. Evo
//!   never claims to have focused "the" window.
//! - **No supported generic API reports which window or tab is displaying a
//!   URL.** Accessibility exposes no URL attribute for browser tabs, and the
//!   only mechanisms that do are per-application scripting integrations, which
//!   this layer is forbidden to grow. So a URL always binds to
//!   [`BoundResource::OpenUrl`]: the system opens it and activates the browser,
//!   and Evo reports exactly that — never "focused your existing tab".

use crate::locator::LocatorKind;
use crate::macos::{WindowInfo, WindowSelectionError, select_unique_window};
use crate::preflight::PreflightSource;
use evo_artifact::artifact_id::ArtifactId;

/// The concrete native resource one Artifact is bound to right now, together
/// with the single action that can truthfully be performed on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundResource {
    /// Tier 1 — a live window Evo can see, matched by the exact witnessed
    /// title and uniquely resolved. It is raised, not reopened.
    FocusWindow { window: WindowInfo },
    /// Tier 1 — a live window the operating system itself reports is
    /// displaying this exact file (Accessibility `AXDocument`). It is raised,
    /// not reopened, so the user does not get a second view of their own file.
    FocusDocumentWindow { window: WindowInfo, path: String },
    /// Tier 2 — the file is handed to the system to open, because it is not
    /// (or cannot be witnessed as) already open in a single window.
    OpenFile {
        path: String,
        witnessed: WitnessedOpen,
    },
    /// Tier 2 — the URL is handed to the system to open. See the module note
    /// on why an already-open tab cannot be witnessed generically.
    OpenUrl { url: String },
}

/// Why a file is being opened rather than focused.
///
/// This exists so the reason a file took the second tier is *reported*, not
/// assumed. "Evo could not tell" and "Evo could tell it was closed" are
/// different facts and are kept different.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessedOpen {
    /// The system reports no window displaying the file: it is genuinely
    /// closed, and opening it is the correct act.
    NotOpen,
    /// More than one live window displays the file. Evo does not choose
    /// between them; the system decides which receives the open request.
    SeveralWindows { count: usize },
    /// The platform cannot report which windows display a file, so Evo does
    /// not know whether it is already open — and does not claim to.
    CannotWitness { reason: String },
}

impl WitnessedOpen {
    /// What Evo could not establish about this file, if anything.
    ///
    /// `None` means the answer was clear — the file is closed, and opening it
    /// is simply the right act, with nothing to qualify. The other two carry a
    /// caveat that belongs in the result the user reads, because "Evo opened
    /// this" and "Evo opened this and could not tell whether it was already
    /// open" are different claims.
    pub fn reason(&self) -> Option<String> {
        match self {
            WitnessedOpen::NotOpen => None,
            WitnessedOpen::SeveralWindows { count } => Some(format!(
                "{count} live windows are showing this file, so Evo left the \
                 choice of which one receives it to the system"
            )),
            WitnessedOpen::CannotWitness { reason } => Some(format!(
                "Evo could not tell whether this was already open: {reason}"
            )),
        }
    }
}

/// What Evo can truthfully do to a bound resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingCapability {
    /// Raise something the user already has open.
    Focus,
    /// Ask the system to open something that is not open.
    Open,
}

impl BindingCapability {
    /// A stable plain-language verb for this capability. Presentation only.
    pub fn label(&self) -> &'static str {
        match self {
            BindingCapability::Focus => "focus",
            BindingCapability::Open => "open",
        }
    }
}

/// One Artifact bound to one live native resource, for the duration of one
/// execution and no longer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveResourceBinding {
    artifact_id: ArtifactId,
    resource: BoundResource,
}

impl LiveResourceBinding {
    /// Constructs a binding for one Artifact.
    pub fn new(artifact_id: ArtifactId, resource: BoundResource) -> Self {
        Self {
            artifact_id,
            resource,
        }
    }

    /// The canonical Artifact this binding serves. The binding belongs to the
    /// resource, not to any body of work that referenced it.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// The live resource this Artifact currently resolves to.
    pub fn resource(&self) -> &BoundResource {
        &self.resource
    }

    /// The one action this binding permits.
    pub fn capability(&self) -> BindingCapability {
        match &self.resource {
            BoundResource::FocusWindow { .. } | BoundResource::FocusDocumentWindow { .. } => {
                BindingCapability::Focus
            }
            BoundResource::OpenFile { .. } | BoundResource::OpenUrl { .. } => {
                BindingCapability::Open
            }
        }
    }
}

/// Why an Artifact could not be bound to any live native resource.
///
/// A binding failure is never a fallback into guessing: it is the honest end
/// of the road for that one target, reported per-artifact while every other
/// target proceeds independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingFailure {
    /// The witnessed window is not open, and Evo does not know (and will not
    /// guess) which application owned it.
    NoLiveWindow { title: String },
    /// Several live windows carry the exact witnessed title; Evo does not
    /// choose between them.
    SeveralLiveWindows { title: String, count: usize },
    /// The live window list itself could not be read — typically because
    /// Accessibility permission has not been granted.
    CannotInspectWindows { reason: String },
}

impl BindingFailure {
    /// The honest, user-facing reason this Artifact has no live binding.
    pub fn reason(&self) -> String {
        match self {
            BindingFailure::NoLiveWindow { title } => format!(
                "no window titled “{title}” is currently open; Evo does not guess which \
                 application owned it"
            ),
            BindingFailure::SeveralLiveWindows { title, count } => format!(
                "{count} open windows are titled “{title}”; Evo cannot choose between them \
                 without more canonical evidence"
            ),
            BindingFailure::CannotInspectWindows { reason } => {
                format!("cannot inspect open windows: {reason}")
            }
        }
    }
}

/// Binds one canonical executable target to the live native resource it names
/// right now.
///
/// Deterministic given the same live OS answers: the same locator and the same
/// window/document evidence always produce the same binding. Called
/// immediately before the action it authorises, never cached across
/// executions, because the answer is only true for this instant.
pub fn bind_resource(
    artifact_id: &ArtifactId,
    kind: &LocatorKind,
    source: &dyn PreflightSource,
) -> Result<LiveResourceBinding, BindingFailure> {
    let bound = |resource| Ok(LiveResourceBinding::new(artifact_id.clone(), resource));

    match kind {
        // A witnessed window binds only to a window that is actually open.
        // There is no "open it anyway" second tier here: Evo does not know
        // which application owned a title, and inventing one would be a guess.
        LocatorKind::WindowTitle(title) => match source.windows() {
            Err(reason) => Err(BindingFailure::CannotInspectWindows { reason }),
            Ok(windows) => match select_unique_window(&windows, title) {
                Ok(window) => bound(BoundResource::FocusWindow {
                    window: window.clone(),
                }),
                Err(WindowSelectionError::NoMatch) => Err(BindingFailure::NoLiveWindow {
                    title: title.clone(),
                }),
                Err(WindowSelectionError::MultipleMatches { count }) => {
                    Err(BindingFailure::SeveralLiveWindows {
                        title: title.clone(),
                        count,
                    })
                }
            },
        },

        // A file asks the OS the one question that answers the tier: "is any
        // window already displaying exactly this path?" Exactly one → raise
        // it. None → open it. Several, or no answer → hand it to the system
        // and say why, rather than choosing a window Evo has no basis to pick.
        LocatorKind::FilePath(path) => match source.windows_showing_document(path) {
            Err(reason) => bound(BoundResource::OpenFile {
                path: path.clone(),
                witnessed: WitnessedOpen::CannotWitness { reason },
            }),
            Ok(windows) => match windows.len() {
                1 => bound(BoundResource::FocusDocumentWindow {
                    window: windows[0].clone(),
                    path: path.clone(),
                }),
                0 => bound(BoundResource::OpenFile {
                    path: path.clone(),
                    witnessed: WitnessedOpen::NotOpen,
                }),
                count => bound(BoundResource::OpenFile {
                    path: path.clone(),
                    witnessed: WitnessedOpen::SeveralWindows { count },
                }),
            },
        },

        // A URL has no generic "is it already showing?" probe. See the module
        // note: the system opens it and activates the browser, and that is
        // exactly what Evo will report having done.
        LocatorKind::Url(url) => bound(BoundResource::OpenUrl { url: url.clone() }),
    }
}

/// Converts an Accessibility `AXDocument` value into the file path it names.
///
/// `AXDocument` is documented to carry a URL string; document-based
/// applications report `file:///…` with the usual percent-encoding, and some
/// report a bare path. Both are accepted, anything else is rejected: a value
/// Evo cannot read as a local file path yields `None` rather than a guess.
/// Non-file schemes are deliberately refused — a remote document is not the
/// local file the canonical evidence names.
pub(crate) fn document_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("file://") {
        // `file://localhost/path` and `file:///path` both denote a local path;
        // an authority naming another host does not.
        let path = match rest.strip_prefix("localhost/") {
            Some(after) => format!("/{after}"),
            None => {
                if !rest.starts_with('/') {
                    return None;
                }
                rest.to_string()
            }
        };
        return percent_decode(&path);
    }
    if trimmed.starts_with('/') {
        return Some(trimmed.to_string());
    }
    None
}

/// Percent-decodes a URL path into bytes and then UTF-8.
///
/// Returns `None` for a malformed escape or non-UTF-8 result: an unreadable
/// value is honestly unusable, never partially salvaged into a different path.
fn percent_decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hi = *bytes.get(i + 1)?;
            let lo = *bytes.get(i + 2)?;
            let value = (hex(hi)? << 4) | hex(lo)?;
            out.push(value);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(title: &str, pid: i32, owner: &str) -> WindowInfo {
        WindowInfo {
            title: title.to_string(),
            owner_pid: pid,
            owner_name: owner.to_string(),
        }
    }

    fn artifact(id: &str) -> ArtifactId {
        ArtifactId::new(id).expect("valid artifact id")
    }

    /// A live OS whose answers the test dictates exactly.
    struct Live {
        windows: Result<Vec<WindowInfo>, String>,
        documents: Result<Vec<WindowInfo>, String>,
        files: Vec<String>,
    }

    impl Default for Live {
        fn default() -> Self {
            Self {
                windows: Ok(Vec::new()),
                documents: Ok(Vec::new()),
                files: Vec::new(),
            }
        }
    }

    impl PreflightSource for Live {
        fn file_exists(&self, path: &str) -> bool {
            self.files.iter().any(|known| known == path)
        }
        fn windows(&self) -> Result<Vec<WindowInfo>, String> {
            self.windows.clone()
        }
        fn windows_showing_document(&self, _path: &str) -> Result<Vec<WindowInfo>, String> {
            self.documents.clone()
        }
    }

    #[test]
    fn an_open_window_binds_to_focus_not_to_reopening() {
        let live = Live {
            windows: Ok(vec![window("Design Brief — Canvas", 501, "Canvas")]),
            ..Live::default()
        };
        let binding = bind_resource(
            &artifact("a1"),
            &LocatorKind::WindowTitle("Design Brief — Canvas".into()),
            &live,
        )
        .expect("the window is open, so it binds");
        assert_eq!(binding.capability(), BindingCapability::Focus);
        assert_eq!(
            binding.resource(),
            &BoundResource::FocusWindow {
                window: window("Design Brief — Canvas", 501, "Canvas")
            }
        );
    }

    #[test]
    fn a_closed_window_has_no_binding_and_says_why() {
        let live = Live::default();
        let failure = bind_resource(
            &artifact("a1"),
            &LocatorKind::WindowTitle("Design Brief — Canvas".into()),
            &live,
        )
        .expect_err("a window that is not open cannot be bound");
        assert_eq!(
            failure,
            BindingFailure::NoLiveWindow {
                title: "Design Brief — Canvas".into()
            }
        );
        // The honest reason names the resource and refuses to guess an owner.
        assert!(failure.reason().contains("does not guess"));
    }

    #[test]
    fn several_identical_titles_never_bind_arbitrarily() {
        let live = Live {
            windows: Ok(vec![
                window("Untitled", 501, "Editor"),
                window("Untitled", 777, "Notes"),
            ]),
            ..Live::default()
        };
        let failure = bind_resource(
            &artifact("a1"),
            &LocatorKind::WindowTitle("Untitled".into()),
            &live,
        )
        .expect_err("two equally valid windows must not bind");
        assert_eq!(
            failure,
            BindingFailure::SeveralLiveWindows {
                title: "Untitled".into(),
                count: 2
            }
        );
    }

    #[test]
    fn an_unreadable_window_list_is_reported_not_assumed_empty() {
        let live = Live {
            windows: Err("Accessibility permission is not granted".into()),
            ..Live::default()
        };
        let failure = bind_resource(
            &artifact("a1"),
            &LocatorKind::WindowTitle("Anything".into()),
            &live,
        )
        .expect_err("an unreadable window list cannot produce a binding");
        assert!(matches!(
            failure,
            BindingFailure::CannotInspectWindows { .. }
        ));
        assert!(failure.reason().contains("Accessibility"));
    }

    #[test]
    fn a_file_the_os_reports_open_binds_to_focus_that_window() {
        // The OS itself says this window is displaying this file, so Evo
        // raises the user's own window instead of opening a second view.
        let live = Live {
            documents: Ok(vec![window("report.md — Editor", 501, "Editor")]),
            files: vec!["/w/report.md".into()],
            ..Live::default()
        };
        let binding = bind_resource(
            &artifact("a1"),
            &LocatorKind::FilePath("/w/report.md".into()),
            &live,
        )
        .expect("a file always binds");
        assert_eq!(binding.capability(), BindingCapability::Focus);
        assert_eq!(
            binding.resource(),
            &BoundResource::FocusDocumentWindow {
                window: window("report.md — Editor", 501, "Editor"),
                path: "/w/report.md".into()
            }
        );
    }

    #[test]
    fn a_closed_file_binds_to_opening_it() {
        let live = Live {
            files: vec!["/w/report.md".into()],
            ..Live::default()
        };
        let binding = bind_resource(
            &artifact("a1"),
            &LocatorKind::FilePath("/w/report.md".into()),
            &live,
        )
        .expect("a file always binds");
        assert_eq!(binding.capability(), BindingCapability::Open);
        assert_eq!(
            binding.resource(),
            &BoundResource::OpenFile {
                path: "/w/report.md".into(),
                witnessed: WitnessedOpen::NotOpen
            }
        );
    }

    #[test]
    fn a_file_open_in_several_windows_is_delegated_to_the_system_with_its_reason() {
        let live = Live {
            documents: Ok(vec![
                window("report.md — Editor", 501, "Editor"),
                window("report.md — Preview", 777, "Preview"),
            ]),
            files: vec!["/w/report.md".into()],
            ..Live::default()
        };
        let binding = bind_resource(
            &artifact("a1"),
            &LocatorKind::FilePath("/w/report.md".into()),
            &live,
        )
        .expect("a file always binds");
        // Evo does not pick one of the two windows; it hands the file to the
        // system and records that it could not choose.
        assert_eq!(binding.capability(), BindingCapability::Open);
        assert_eq!(
            binding.resource(),
            &BoundResource::OpenFile {
                path: "/w/report.md".into(),
                witnessed: WitnessedOpen::SeveralWindows { count: 2 }
            }
        );
    }

    #[test]
    fn a_platform_that_cannot_witness_documents_degrades_to_opening_and_says_so() {
        let live = Live {
            documents: Err("this platform cannot report document windows".into()),
            files: vec!["/w/report.md".into()],
            ..Live::default()
        };
        let binding = bind_resource(
            &artifact("a1"),
            &LocatorKind::FilePath("/w/report.md".into()),
            &live,
        )
        .expect("a file always binds");
        assert_eq!(binding.capability(), BindingCapability::Open);
        match binding.resource() {
            BoundResource::OpenFile {
                witnessed: WitnessedOpen::CannotWitness { reason },
                ..
            } => assert!(reason.contains("cannot report")),
            other => panic!("expected an unwitnessed open, got {other:?}"),
        }
    }

    #[test]
    fn a_url_binds_to_the_system_open_and_never_claims_a_focused_tab() {
        // Even with windows whose titles look like the page, a URL never
        // binds to Focus: no supported generic API can witness a tab.
        let live = Live {
            windows: Ok(vec![window("Example — Browser", 501, "Browser")]),
            ..Live::default()
        };
        let binding = bind_resource(
            &artifact("a1"),
            &LocatorKind::Url("https://example.com/doc".into()),
            &live,
        )
        .expect("a url always binds");
        assert_eq!(binding.capability(), BindingCapability::Open);
        assert_eq!(
            binding.resource(),
            &BoundResource::OpenUrl {
                url: "https://example.com/doc".into()
            }
        );
    }

    #[test]
    fn binding_is_a_property_of_the_resource_not_of_the_work_that_asked() {
        // The same resource requested for two different bodies of work binds
        // to the same live window. Ownership is never assumed or consulted.
        let live = Live {
            documents: Ok(vec![window("shared.md — Editor", 501, "Editor")]),
            files: vec!["/w/shared.md".into()],
            ..Live::default()
        };
        let kind = LocatorKind::FilePath("/w/shared.md".into());
        let for_x = bind_resource(&artifact("shared-1"), &kind, &live).expect("binds for X");
        let for_y = bind_resource(&artifact("shared-1"), &kind, &live).expect("binds for Y");
        assert_eq!(for_x.resource(), for_y.resource());
        assert_eq!(for_x, for_y);
    }

    #[test]
    fn binding_is_deterministic_for_identical_live_answers() {
        let live = Live {
            windows: Ok(vec![window("W", 1, "App")]),
            documents: Ok(vec![window("D", 2, "App")]),
            files: vec!["/f".into()],
        };
        for kind in [
            LocatorKind::WindowTitle("W".into()),
            LocatorKind::FilePath("/f".into()),
            LocatorKind::Url("https://x.dev".into()),
        ] {
            let first = bind_resource(&artifact("a1"), &kind, &live);
            let second = bind_resource(&artifact("a1"), &kind, &live);
            assert_eq!(first, second);
        }
    }

    #[test]
    fn document_values_resolve_to_local_paths_or_to_nothing() {
        assert_eq!(
            document_path("file:///Users/me/work/report.md"),
            Some("/Users/me/work/report.md".to_string())
        );
        // Percent-encoded spaces and non-ASCII survive intact.
        assert_eq!(
            document_path("file:///Users/me/My%20Work/caf%C3%A9.md"),
            Some("/Users/me/My Work/café.md".to_string())
        );
        assert_eq!(
            document_path("file://localhost/Users/me/a.md"),
            Some("/Users/me/a.md".to_string())
        );
        // A bare absolute path is accepted as itself.
        assert_eq!(
            document_path("/Users/me/a.md"),
            Some("/Users/me/a.md".to_string())
        );
        // Anything Evo cannot read as a local file is refused, never guessed.
        assert_eq!(document_path(""), None);
        assert_eq!(document_path("   "), None);
        assert_eq!(document_path("https://example.com/a.md"), None);
        assert_eq!(document_path("file://otherhost/Users/me/a.md"), None);
        assert_eq!(document_path("relative/path.md"), None);
        assert_eq!(document_path("file:///bad%2escape%ZZ"), None);
    }
}
