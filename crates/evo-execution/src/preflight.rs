//! Executor Preflight — the deterministic establishment of a selected
//! resource's executable-target disposition BEFORE any OS action is
//! attempted.
//!
//! # The preflight question
//!
//! For each resource the user's declared continuation selected, Evo must be
//! able to state — without attempting anything and without guessing — whether
//! the executable target is:
//!
//! - **READY** — present and uniquely addressable; it will be attempted;
//! - **UNAVAILABLE** — the resource is not present (missing/renamed file, no
//!   matching window, cannot be witnessed);
//! - **AMBIGUOUS** — more than one equally valid target exists; Evo never
//!   chooses arbitrarily;
//! - **UNSUPPORTED** — the resource is inherently non-executable (a commit is
//!   a repository object); its identity is preserved but no action exists.
//!
//! # Contractual position
//!
//! Preflight is **execution-layer state**, exactly as IS-0021 §7 permits:
//! "A future execution layer MAY inspect current operating-system state when
//! executing a RestorationPlan, but that state SHALL NOT become an input to
//! Restoration Derivation." Preflight inspects the file system and the live
//! window list at execution time; that state never enters derivation, never
//! becomes continuation evidence, and is never persisted as canonical state
//! (Architectural Law XVI — derived values, not objects).
//!
//! Selection remains authoritative: this module consumes the already-derived
//! [`RestorationSelection`] and never redisovers what is relevant. Historical
//! members are never preflighted — they are never passed to execution — and
//! recent-but-undeclared resources are not surface members, so they are never
//! preflighted either.
//!
//! # Honesty rules
//!
//! - Files resolve by the exact canonical path. Missing/renamed → UNAVAILABLE.
//!   No fuzzy filename/path recovery, ever.
//! - URLs resolve directly from the canonical URL identity → READY. No
//!   ranking, no substitution of similar URLs.
//! - Window targets resolve by the exact witnessed title: zero matches →
//!   UNAVAILABLE, more than one → AMBIGUOUS. No arbitrary choice.
//! - Commits preserve their identity but are UNSUPPORTED: Evo never guesses a
//!   repository, editor, terminal, or application to open one.

use crate::macos::{WindowInfo, WindowSelectionError, select_unique_window};
use crate::resource::ResourceIdentity;
use crate::selection::RestorationSelection;
use evo_artifact::artifact_id::ArtifactId;

/// The preflight disposition of one selected resource's executable target.
///
/// This is the honest per-target answer to "can Evo attempt this right now,
/// and why (not)?" — established deterministically from the canonical
/// resource identity plus execution-time OS state, before any action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightStatus {
    /// The executable target is present and uniquely addressable; it will be
    /// attempted.
    Ready,
    /// The target cannot be resolved — the resource is not present (a file
    /// was deleted/renamed, no window with the exact title is open, or the
    /// platform cannot witness the required state).
    Unavailable { reason: String },
    /// The target cannot be resolved uniquely — more than one equally valid
    /// candidate exists; Evo never chooses between them arbitrarily.
    Ambiguous { reason: String },
    /// The resource is inherently non-executable (e.g. a commit — a
    /// repository object). Its identity is preserved, but no action exists.
    Unsupported { reason: String },
}

impl PreflightStatus {
    /// The canonical status vocabulary this mission establishes: one of
    /// `READY`, `UNAVAILABLE`, `AMBIGUOUS`, `UNSUPPORTED`. Presentation
    /// constant only; never a canonical model value.
    pub fn label(&self) -> &'static str {
        match self {
            PreflightStatus::Ready => "READY",
            PreflightStatus::Unavailable { .. } => "UNAVAILABLE",
            PreflightStatus::Ambiguous { .. } => "AMBIGUOUS",
            PreflightStatus::Unsupported { .. } => "UNSUPPORTED",
        }
    }

    /// Whether the target is ready to be attempted.
    pub fn is_ready(&self) -> bool {
        matches!(self, PreflightStatus::Ready)
    }

    /// The honest reason behind a non-ready disposition.
    pub fn reason(&self) -> Option<&str> {
        match self {
            PreflightStatus::Ready => None,
            PreflightStatus::Unavailable { reason }
            | PreflightStatus::Ambiguous { reason }
            | PreflightStatus::Unsupported { reason } => Some(reason),
        }
    }
}

/// One preflight outcome for one selected resource, in canonical order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightOutcome {
    artifact_id: ArtifactId,
    status: PreflightStatus,
}

impl PreflightOutcome {
    /// Constructs a preflight outcome for one Artifact.
    pub fn new(artifact_id: ArtifactId, status: PreflightStatus) -> Self {
        Self {
            artifact_id,
            status,
        }
    }

    /// The canonical Artifact this outcome concerns.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// The established disposition of this resource's executable target.
    pub fn status(&self) -> &PreflightStatus {
        &self.status
    }
}

/// The live OS state the execution layer may consult — both to preflight a
/// target and to bind it to a live resource ([`crate::binding`]).
///
/// This is the *only* runtime state the execution layer sees, and it is
/// consumed exclusively at execution time (IS-0021 §7) — it never becomes an
/// input to Restoration Derivation and never becomes continuation evidence.
pub trait PreflightSource {
    /// Whether the file at the exact canonical path currently exists.
    /// Resolution is exact-path only; there is no fuzzy recovery.
    fn file_exists(&self, path: &str) -> bool;

    /// The currently open windows, or an error when the platform cannot
    /// witness them (e.g. missing Accessibility permission).
    fn windows(&self) -> Result<Vec<WindowInfo>, String>;

    /// The live windows the operating system itself reports are displaying the
    /// file at this exact path.
    ///
    /// This is the evidence that lets Evo raise a file the user already has
    /// open instead of opening a second view of it. It is the OS's own answer
    /// (on macOS, the Accessibility `AXDocument` attribute) — never an
    /// inference from window titles, and never application-specific knowledge.
    ///
    /// The default is `Err`, not an empty list, because "this platform cannot
    /// tell" and "no window is showing it" are different facts: a source that
    /// cannot answer must say so, so the binding layer degrades to opening the
    /// file and reports *why* rather than implying the file was closed.
    fn windows_showing_document(&self, _path: &str) -> Result<Vec<WindowInfo>, String> {
        Err("this platform cannot report which windows are displaying a file".to_string())
    }
}

/// Establishes the preflight disposition of one canonical resource identity.
///
/// Deterministic given an identity and a window list: files resolve by exact
/// path existence, URLs resolve directly, windows resolve by exact title
/// uniqueness, and commits are UNSUPPORTED. Never guesses, never substitutes.
pub fn preflight_identity(
    identity: &ResourceIdentity,
    source: &dyn PreflightSource,
) -> PreflightStatus {
    match identity {
        ResourceIdentity::FilePath(path) => {
            if source.file_exists(path) {
                PreflightStatus::Ready
            } else {
                PreflightStatus::Unavailable {
                    reason: format!(
                        "the file “{path}” does not exist right now; Evo does not guess which \
                         resource to open"
                    ),
                }
            }
        }
        // A URL is resolved directly from its canonical identity. Evo never
        // ranks URLs and never substitutes a similar one; opening is delegated
        // to the platform, which may honestly fail at attempt time.
        ResourceIdentity::Url(_) => PreflightStatus::Ready,
        ResourceIdentity::WindowTitle(title) => match source.windows() {
            Err(reason) => PreflightStatus::Unavailable {
                reason: format!("cannot inspect open windows: {reason}"),
            },
            Ok(windows) => match select_unique_window(&windows, title) {
                Ok(_) => PreflightStatus::Ready,
                Err(WindowSelectionError::NoMatch) => PreflightStatus::Unavailable {
                    reason: format!(
                        "no window titled “{title}” is currently open; Evo does not guess which \
                         application owned it"
                    ),
                },
                Err(WindowSelectionError::MultipleMatches { count }) => {
                    PreflightStatus::Ambiguous {
                        reason: format!(
                            "{count} open windows are titled “{title}”; Evo cannot choose between \
                         them without more canonical evidence"
                        ),
                    }
                }
            },
        },
        // A commit is a real resource with identity but no executable target:
        // Evo never guesses a repository, editor, terminal, or application to
        // open it.
        ResourceIdentity::Commit(hash) => PreflightStatus::Unsupported {
            reason: format!(
                "this resource is a repository object (commit “{hash}”) and has no executable \
                 target; Evo does not guess how to open it"
            ),
        },
    }
}

/// Preflights one identity using a pre-fetched window list when available.
///
/// For WindowTitle identities the window list is used directly instead of
/// calling `source.windows()` again — this is the inner loop of
/// [`preflight_selection`] and the caller already fetched the list once.
fn preflight_identity_cached(
    identity: &ResourceIdentity,
    source: &dyn PreflightSource,
    cached_windows: &Option<Result<Vec<WindowInfo>, String>>,
) -> PreflightStatus {
    match identity {
        ResourceIdentity::WindowTitle(title) => {
            let windows_result = match cached_windows {
                Some(result) => result,
                None => &source.windows(),
            };
            match windows_result {
                Err(reason) => PreflightStatus::Unavailable {
                    reason: format!("cannot inspect open windows: {reason}"),
                },
                Ok(windows) => match select_unique_window(windows, title) {
                    Ok(_) => PreflightStatus::Ready,
                    Err(WindowSelectionError::NoMatch) => PreflightStatus::Unavailable {
                        reason: format!(
                            "no window titled \u{201c}{title}\u{201d} is currently open; Evo does not guess which \
                             application owned it"
                        ),
                    },
                    Err(WindowSelectionError::MultipleMatches { count }) => {
                        PreflightStatus::Ambiguous {
                            reason: format!(
                                "{count} open windows are titled \u{201c}{title}\u{201d}; Evo cannot choose between \
                             them without more canonical evidence"
                            ),
                        }
                    }
                },
            }
        }
        _ => preflight_identity(identity, source),
    }
}

/// Preflights every current-continuation member of a selection.
///
/// The preflight set is exactly the selection's surface — restore-worthy ∪
/// unavailable — in canonical order (ascending ArtifactId). Historical
/// members are never preflighted (they are never passed to execution), and
/// recent-but-undeclared resources are not surface members, so they are never
/// preflighted either.
///
/// The window list is fetched at most once per call so that multiple
/// WindowTitle locators share a single Accessibility sweep instead of each
/// triggering a fresh, expensive sweep of every running application.
///
/// Deterministic: identical selection + identical OS snapshot produce
/// identical outcomes.
pub fn preflight_selection(
    selection: &RestorationSelection,
    source: &dyn PreflightSource,
) -> Vec<PreflightOutcome> {
    // Prefetch the window list once. Only WindowTitle locators need it;
    // FilePath and Url locators never call source.windows(), so this is a
    // no-op when no WindowTitle members exist.
    let has_window_titles = selection
        .restore_worthy()
        .iter()
        .any(|s| matches!(s.identity(), ResourceIdentity::WindowTitle(_)));
    let cached_windows = if has_window_titles {
        Some(source.windows())
    } else {
        None
    };

    let mut outcomes: Vec<PreflightOutcome> = Vec::new();
    for selected in selection.restore_worthy() {
        let status = preflight_identity_cached(selected.identity(), source, &cached_windows);
        outcomes.push(PreflightOutcome::new(
            selected.artifact_id().clone(),
            status,
        ));
    }

    for unavailable in selection.unavailable() {
        let status = match unavailable.identity() {
            // A classified but inherently non-executable resource (a commit):
            // preserved identity, no action, honest reason from the selection.
            Some(ResourceIdentity::Commit(_)) => PreflightStatus::Unsupported {
                reason: unavailable.reason().to_string(),
            },
            // A surface member whose identity cannot be resolved: honestly
            // unavailable, never guessed.
            Some(_) => PreflightStatus::Unavailable {
                reason: unavailable.reason().to_string(),
            },
            None => PreflightStatus::Unavailable {
                reason: unavailable.reason().to_string(),
            },
        };
        outcomes.push(PreflightOutcome::new(
            unavailable.artifact_id().clone(),
            status,
        ));
    }
    // One canonical deterministic order for the whole report: ascending
    // ArtifactId across the surface. Execution attempts follow this order.
    outcomes.sort_by(|a, b| a.artifact_id().as_str().cmp(b.artifact_id().as_str()));
    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{PlatformExecutor, TargetStatus};
    use crate::resource::ResourceIdentity;
    use crate::selection::{
        Disposition, RestorationSelection, SelectedResource, UnavailableReason, WithheldMember,
    };
    use evo_workspace::WorkspaceId;

    use std::collections::HashMap;

    fn artifact(id: &str) -> ArtifactId {
        ArtifactId::new(id).unwrap()
    }

    fn window(title: &str, pid: i32) -> WindowInfo {
        WindowInfo {
            title: title.to_string(),
            owner_pid: pid,
            owner_name: "Fixture App".to_string(),
        }
    }

    /// A deterministic, configurable preflight source for tests.
    #[derive(Debug, Default, Clone)]
    struct FakeSource {
        files: Vec<String>,
        windows: Vec<WindowInfo>,
        window_error: Option<String>,
    }

    impl FakeSource {
        fn with_files(paths: &[&str]) -> Self {
            Self {
                files: paths.iter().map(|p| p.to_string()).collect(),
                ..Self::default()
            }
        }
    }

    impl PreflightSource for FakeSource {
        fn file_exists(&self, path: &str) -> bool {
            self.files.iter().any(|p| p == path)
        }
        fn windows(&self) -> Result<Vec<WindowInfo>, String> {
            match &self.window_error {
                Some(reason) => Err(reason.clone()),
                None => Ok(self.windows.clone()),
            }
        }
    }

    fn identity_file(path: &str) -> ResourceIdentity {
        ResourceIdentity::FilePath(path.to_string())
    }

    // ── The four dispositions ────────────────────────────────────────────────

    #[test]
    fn exact_existing_file_is_ready() {
        let source = FakeSource::with_files(&["/repo/plan.md"]);
        let status = preflight_identity(&identity_file("/repo/plan.md"), &source);
        assert_eq!(status, PreflightStatus::Ready);
        assert!(status.is_ready());
        assert_eq!(status.label(), "READY");
    }

    #[test]
    fn missing_file_is_unavailable() {
        let source = FakeSource::with_files(&[]);
        let status = preflight_identity(&identity_file("/repo/plan.md"), &source);
        assert!(matches!(status, PreflightStatus::Unavailable { .. }));
        assert!(!status.is_ready());
        assert_eq!(status.label(), "UNAVAILABLE");
    }

    #[test]
    fn renamed_or_moved_file_is_unavailable_with_no_fuzzy_recovery() {
        // The canonical path was witnessed; the file now lives elsewhere.
        // Preflight resolves the exact canonical path only — it never
        // searches for a similar filename or path.
        let source = FakeSource::with_files(&["/repo/renamed-plan.md"]);
        let status = preflight_identity(&identity_file("/repo/plan.md"), &source);
        assert!(matches!(
            status,
            PreflightStatus::Unavailable { ref reason } if reason.contains("/repo/plan.md")
        ));
        // The moved file is never substituted.
        assert!(
            !matches!(status, PreflightStatus::Ready),
            "a moved file must never be resolved through fuzzy recovery"
        );
    }

    #[test]
    fn exact_url_is_ready_without_ranking() {
        let source = FakeSource::default();
        let status =
            preflight_identity(&ResourceIdentity::Url("https://docs.dev/a".into()), &source);
        assert_eq!(status, PreflightStatus::Ready);
    }

    #[test]
    fn multiple_matching_windows_are_ambiguous() {
        let source = FakeSource {
            windows: vec![window("Untitled", 1), window("Untitled", 2)],
            ..FakeSource::default()
        };
        let status = preflight_identity(&ResourceIdentity::WindowTitle("Untitled".into()), &source);
        assert!(
            matches!(status, PreflightStatus::Ambiguous { ref reason } if reason.contains("2 open windows"))
        );
        assert_eq!(status.label(), "AMBIGUOUS");
    }

    #[test]
    fn no_matching_window_is_unavailable() {
        let source = FakeSource {
            windows: vec![window("Other", 1)],
            ..FakeSource::default()
        };
        let status = preflight_identity(&ResourceIdentity::WindowTitle("Untitled".into()), &source);
        assert!(matches!(status, PreflightStatus::Unavailable { .. }));
    }

    #[test]
    fn unwitnessable_windows_are_unavailable() {
        let source = FakeSource {
            window_error: Some("Accessibility permission is missing".to_string()),
            ..FakeSource::default()
        };
        let status = preflight_identity(&ResourceIdentity::WindowTitle("Untitled".into()), &source);
        assert!(matches!(
            status,
            PreflightStatus::Unavailable { reason } if reason.contains("Accessibility")
        ));
    }

    #[test]
    fn commit_preserves_identity_but_is_unsupported() {
        let hash = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
        let status = preflight_identity(
            &ResourceIdentity::Commit(hash.into()),
            &FakeSource::default(),
        );
        assert!(
            matches!(status, PreflightStatus::Unsupported { ref reason } if reason.contains("commit"))
        );
        assert_eq!(status.label(), "UNSUPPORTED");
        // The identity is preserved in the caller's selection; preflight only
        // classifies the disposition.
        assert_eq!(ResourceIdentity::Commit(hash.into()).subject(), hash);
    }

    // ── preflight_selection over a full selection ───────────────────────────

    fn selection_with(
        restore_worthy: Vec<(&str, ResourceIdentity)>,
        unavailable: Vec<(&str, Option<ResourceIdentity>)>,
        historical: Vec<&str>,
    ) -> RestorationSelection {
        let worthy: Vec<SelectedResource> = restore_worthy
            .into_iter()
            .map(|(id, identity)| {
                SelectedResource::new(
                    artifact(id),
                    identity,
                    "the continuation of this work happens here",
                )
            })
            .collect();
        let unavail: Vec<UnavailableReason> = unavailable
            .into_iter()
            .map(|(id, identity)| {
                UnavailableReason::new(artifact(id), identity, "honest reason from the selection")
            })
            .collect();
        let hist: Vec<WithheldMember> = historical
            .iter()
            .map(|id| {
                WithheldMember::new(
                    artifact(id),
                    Disposition::HistoryOnly,
                    "witnessed while the work was happening, and nothing more",
                )
            })
            .collect();
        RestorationSelection::new(WorkspaceId::new(), worthy, unavail, hist)
    }

    #[test]
    fn preflight_covers_every_surface_member_and_never_history() {
        let source = FakeSource::with_files(&["/repo/a.md"]);
        let selection = selection_with(
            vec![
                ("artifact-a", identity_file("/repo/a.md")),
                (
                    "artifact-url",
                    ResourceIdentity::Url("https://a.dev".into()),
                ),
            ],
            vec![(
                "artifact-commit",
                Some(ResourceIdentity::Commit("hash".into())),
            )],
            vec!["artifact-old"],
        );
        let outcomes = preflight_selection(&selection, &source);
        // Surface = restore-worthy ∪ unavailable = 3 members. Historical is
        // never preflighted (never passed to execution).
        assert_eq!(outcomes.len(), 3);
        let by_id: HashMap<&str, &PreflightStatus> = outcomes
            .iter()
            .map(|outcome| (outcome.artifact_id().as_str(), outcome.status()))
            .collect();
        assert_eq!(by_id["artifact-a"], &PreflightStatus::Ready);
        assert_eq!(by_id["artifact-url"], &PreflightStatus::Ready);
        assert!(matches!(
            by_id["artifact-commit"],
            PreflightStatus::Unsupported { .. }
        ));
        // Deterministic canonical order: ascending ArtifactId.
        let ids: Vec<&str> = outcomes.iter().map(|o| o.artifact_id().as_str()).collect();
        assert_eq!(ids, vec!["artifact-a", "artifact-commit", "artifact-url"]);
    }

    #[test]
    fn preflight_is_deterministic() {
        let source = FakeSource::with_files(&["/repo/a.md"]);
        let selection = selection_with(
            vec![("artifact-a", identity_file("/repo/a.md"))],
            vec![],
            vec![],
        );
        let first = preflight_selection(&selection, &source);
        let second = preflight_selection(&selection, &source);
        assert_eq!(first, second);
    }

    // ── Boundary: preflight output feeds the executor, never the reverse ────

    #[test]
    fn preflight_never_invokes_platform_actions() {
        // Preflight is read-only: it must never open, focus, or launch
        // anything. A recording executor used as the preflight source proves
        // no action is taken during preflight.
        #[derive(Default)]
        struct Recorder {
            calls: std::sync::Mutex<Vec<String>>,
        }
        impl PreflightSource for Recorder {
            fn file_exists(&self, path: &str) -> bool {
                self.calls.lock().unwrap().push(format!("stat:{path}"));
                false
            }
            fn windows(&self) -> Result<Vec<WindowInfo>, String> {
                self.calls.lock().unwrap().push("windows".to_string());
                Ok(vec![])
            }
        }
        impl PlatformExecutor for Recorder {
            fn open_url(&self, url: &str) -> TargetStatus {
                self.calls.lock().unwrap().push(format!("url:{url}"));
                TargetStatus::Opened {
                    detail: String::new(),
                }
            }
            fn open_file(&self, path: &str) -> TargetStatus {
                self.calls.lock().unwrap().push(format!("file:{path}"));
                TargetStatus::Opened {
                    detail: String::new(),
                }
            }
            fn focus_window(&self, title: &str) -> TargetStatus {
                self.calls.lock().unwrap().push(format!("window:{title}"));
                TargetStatus::Opened {
                    detail: String::new(),
                }
            }
        }
        let recorder = Recorder::default();
        let _ = preflight_identity(&identity_file("/repo/a.md"), &recorder);
        let _ = preflight_identity(&ResourceIdentity::WindowTitle("W".into()), &recorder);
        let calls = recorder.calls.lock().unwrap().clone();
        assert!(
            calls
                .iter()
                .all(|call| call.starts_with("stat:") || call == "windows"),
            "preflight only inspects state, never acts: {calls:?}"
        );
    }
}
