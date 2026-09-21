//! Platform-neutral Restoration Execution engine.
//!
//! The engine consumes a canonical derived [`ExecutionRequest`] (a complete
//! Restoration Plan or a Resume Point produced by canonical Restoration
//! Derivation) and executes it through a [`PlatformExecutor`], reporting
//! per-target outcomes explicitly.
//!
//! # Ordering
//!
//! Restoration is guided and sequential: the Resume Point is executed first,
//! then the Context Chain in its canonical order (IS-0019 §7, ARCHITECTURE.md
//! §5: primary artifact first, supporting artifacts next). No artifact is
//! executed that the plan does not reference.
//!
//! # Honesty invariants
//!
//! - Every target originates from the canonical plan, never from a guess.
//! - The engine resolves each Artifact's executable target exclusively from
//!   canonical evidence ([`crate::locator::classify_locator`]).
//! - Before attempting anything, the engine runs **Executor Preflight**
//!   ([`crate::preflight`]): every selected target's disposition is
//!   established from its canonical resource identity plus execution-time OS
//!   state (IS-0021 §7) — READY / UNAVAILABLE / AMBIGUOUS / UNSUPPORTED —
//!   and only READY targets are attempted. Partial failure is a normal,
//!   first-class outcome: one unavailable/ambiguous/unsupported target never
//!   prevents another independently executable target from being attempted.
//! - A READY target is then **bound** to the live resource it names right now
//!   ([`crate::binding`]) and acted on in capability order: something the user
//!   already has open is raised, and only something they do not have open is
//!   opened. The binding is non-canonical, rebuilt per execution, and never
//!   flows back into understanding.
//! - When a target cannot be resolved or executed, the engine reports
//!   [`TargetStatus::Unavailable`] / [`TargetStatus::Ambiguous`] /
//!   [`TargetStatus::Unsupported`] / [`TargetStatus::Failed`] — it never
//!   substitutes a different resource and never claims success.
//! - The engine is deterministic given identical plan + locator + OS-snapshot
//!   inputs.
//!
//! # Planning and acting are separate
//!
//! Every path resolves to an ordered `Vec<`[`RestorationStep`]`>` *before* the
//! first OS action, and each step is then performed by [`execute_step`]. A
//! caller that wants the whole answer at once uses [`execute`] /
//! [`execute_selection`]; a caller that wants to report each result as it
//! actually arrives drives the steps itself. Both run the identical steps in
//! the identical order, so what is promised, what is shown mid-flight, and
//! what is done cannot drift apart.

use std::collections::HashMap;

use evo_artifact::artifact_id::ArtifactId;
use evo_restoration::execution::ExecutionRequest;
use evo_restoration::{DerivationOutcome, ExecutionError};

use crate::binding::{BindingFailure, BoundResource, bind_resource};
use crate::locator::{Locator, LocatorKind};
use crate::preflight::{PreflightSource, PreflightStatus, preflight_identity, preflight_selection};
use crate::resource::ResourceIdentity;
use crate::selection::RestorationSelection;

/// The per-target outcome of one execution attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetStatus {
    /// The target opened successfully.
    Opened { detail: String },
    /// The target could not be opened (e.g. the OS refused).
    Failed { reason: String },
    /// The target cannot be resolved — the resource is not present.
    Unavailable { reason: String },
    /// The target cannot be resolved uniquely — more than one candidate.
    Ambiguous { reason: String },
    /// The resource is inherently non-executable (e.g. a commit); its
    /// identity is preserved but no action exists. Never guessed into a
    /// repository, editor, terminal, or application.
    Unsupported { reason: String },
}

/// One reported execution attempt for one Artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAttempt {
    artifact_id: ArtifactId,
    status: TargetStatus,
}

impl TargetAttempt {
    /// Constructs an attempt record for one Artifact.
    pub fn new(artifact_id: ArtifactId, status: TargetStatus) -> Self {
        Self {
            artifact_id,
            status,
        }
    }

    /// The Artifact this attempt concerned.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// The reported outcome of this attempt.
    pub fn status(&self) -> &TargetStatus {
        &self.status
    }
}

/// The structured result of executing one canonical request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReport {
    attempts: Vec<TargetAttempt>,
}

impl ExecutionReport {
    /// Constructs a report from ordered per-target attempts.
    pub fn new(attempts: Vec<TargetAttempt>) -> Self {
        Self { attempts }
    }

    /// The ordered per-target attempts of this execution.
    pub fn attempts(&self) -> &[TargetAttempt] {
        &self.attempts
    }

    /// Whether every attempted target opened successfully.
    pub fn all_opened(&self) -> bool {
        !self.attempts.is_empty()
            && self
                .attempts
                .iter()
                .all(|attempt| matches!(attempt.status(), TargetStatus::Opened { .. }))
    }
}

/// The platform-specific execution surface consumed by the engine.
///
/// Implementations perform the actual OS interaction for one target kind.
/// The engine stays platform-neutral; platform behavior is injected here.
pub trait PlatformExecutor {
    /// Opens a URL in its default application.
    fn open_url(&self, url: &str) -> TargetStatus;
    /// Opens a file path in its default application.
    fn open_file(&self, path: &str) -> TargetStatus;
    /// Focuses the window with the given title, if uniquely resolvable.
    fn focus_window(&self, title: &str) -> TargetStatus;

    /// Raises and activates one specific live window the binding layer has
    /// already resolved uniquely ([`crate::binding`]).
    ///
    /// This exists because a bound window is identified by more than its
    /// title: a window matched through the OS's own document evidence may
    /// share a title with an unrelated window in another application, and
    /// re-resolving it by title would turn a certainty into an ambiguity. A
    /// platform that can address a window directly should override this; the
    /// default falls back to the title path, which is correct wherever the
    /// title was how the window was found in the first place.
    fn focus_bound_window(&self, window: &crate::macos::WindowInfo) -> TargetStatus {
        self.focus_window(&window.title)
    }
}

/// The ordered Artifacts a request directs execution to act on.
///
/// The Resume Point is always first (IS-0019 RSP-1: the cognitive entry
/// point), followed by the Context Chain in its canonical order. Duplicate
/// Artifacts are collapsed, preserving first occurrence.
pub fn ordered_targets(request: &ExecutionRequest) -> Vec<ArtifactId> {
    let mut targets: Vec<ArtifactId> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    let push = |id: ArtifactId, seen: &mut Vec<String>, targets: &mut Vec<ArtifactId>| {
        let key = id.to_string();
        if !seen.contains(&key) {
            seen.push(key);
            targets.push(id);
        }
    };

    push(
        request.resume_point().artifact_id().clone(),
        &mut seen,
        &mut targets,
    );
    if let Some(plan) = request.plan() {
        for id in plan.context_chain().artifacts() {
            push(id.clone(), &mut seen, &mut targets);
        }
    }
    targets
}

/// The ordered Artifacts execution will attempt for a derivation outcome — the
/// one shared ordering decision for both the announced plan and the executed
/// act, so the promise and the act can never describe different target sets or
/// orders (BE-AUDIT-0001 §2.4; UI-VALIDATION-0001 B.9).
///
/// - A declared Continuation Surface (RFC-0013) routes through the derived
///   selection: its restore-worthy members in canonical order (ascending
///   ArtifactId) — exactly the order [`execute_selection`] attempts them.
/// - Otherwise the derived Resume Point (RFC-0011) routes through the
///   Restoration Plan / Resume Point request: the Resume Point first, then
///   the Context Chain in canonical order (IS-0019 RSP-1).
///
/// Refusals are returned as [`ExecutionError`] so the announcement and the
/// act share the same honest refusal instead of each inventing one.
pub fn ordered_attempt_targets(
    outcome: Option<&DerivationOutcome>,
    selection: Option<&RestorationSelection>,
) -> Result<Vec<ArtifactId>, ExecutionError> {
    let Some(outcome) = outcome else {
        return Err(ExecutionError::NoDerivedResumePoint {
            reason: "no derived restoration understanding is available for this \
                     Workspace yet."
                .to_string(),
        });
    };
    if !outcome.continuation_surface().is_empty() {
        let selection = selection.ok_or(ExecutionError::NoDerivedResumePoint {
            reason: "a continuation surface is declared but its derived selection is \
                     unavailable; Evo does not guess targets."
                .to_string(),
        })?;
        let mut targets: Vec<ArtifactId> = selection
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().clone())
            .collect();
        // Restoration order: where the work continues first, then the rest in
        // canonical order. One shared key
        // ([`RestorationSelection::restoration_key`]) decides it here and in
        // `plan_selection`, so the announcement and the act cannot diverge.
        targets.sort_by(|a, b| {
            selection
                .restoration_key(a)
                .cmp(&selection.restoration_key(b))
        });
        Ok(targets)
    } else {
        let request = ExecutionRequest::from_derivation(outcome)?;
        Ok(ordered_targets(&request))
    }
}

// ── Restoration steps: decide everything before acting ───────────────────────

/// What one step of a restoration will do, decided **before** any OS action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepPlan {
    /// Preflight said READY and canonical evidence records an executable
    /// target. The step will bind this target to a live resource and act.
    Attempt(LocatorKind),
    /// The outcome is already settled and no OS action will be performed:
    /// unavailable, ambiguous, or inherently unsupported. Reported honestly,
    /// never retried as a guess.
    Refuse(TargetStatus),
}

/// One step of one restoration: the Artifact, and what will be done about it.
///
/// Separating the plan from the act is what lets a caller *name every step
/// before the first one runs* and then report each result as it actually
/// arrives — instead of showing a list of promises and later replacing it with
/// a list of outcomes. The ordering of a `Vec<RestorationStep>` is the
/// canonical restoration order; nothing downstream reorders it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorationStep {
    artifact_id: ArtifactId,
    plan: StepPlan,
}

impl RestorationStep {
    /// Constructs one planned step.
    pub fn new(artifact_id: ArtifactId, plan: StepPlan) -> Self {
        Self { artifact_id, plan }
    }

    /// The canonical Artifact this step concerns.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// What this step will do.
    pub fn plan(&self) -> &StepPlan {
        &self.plan
    }

    /// Whether this step will actually touch the operating system.
    pub fn will_attempt(&self) -> bool {
        matches!(self.plan, StepPlan::Attempt(_))
    }
}

/// The one step-resolution rule shared by every execution path: a READY
/// preflight with a recorded locator becomes an attempt; anything else becomes
/// an honest refusal.
fn step_for(
    artifact_id: ArtifactId,
    status: PreflightStatus,
    locators: &HashMap<String, Locator>,
) -> RestorationStep {
    let plan = match status {
        PreflightStatus::Ready => match locators.get(&artifact_id.to_string()) {
            Some(locator) => StepPlan::Attempt(locator.kind().clone()),
            None => StepPlan::Refuse(TargetStatus::Unavailable {
                reason: format!(
                    "no canonical executable target is recorded for Artifact {}; \
                     no action was performed",
                    artifact_id.as_str()
                ),
            }),
        },
        PreflightStatus::Unavailable { reason } => {
            StepPlan::Refuse(TargetStatus::Unavailable { reason })
        }
        PreflightStatus::Ambiguous { reason } => {
            StepPlan::Refuse(TargetStatus::Ambiguous { reason })
        }
        PreflightStatus::Unsupported { reason } => {
            StepPlan::Refuse(TargetStatus::Unsupported { reason })
        }
    };
    RestorationStep::new(artifact_id, plan)
}

/// Plans the restoration of a declared Continuation Surface: every member of
/// the current continuation, preflighted, in restoration order, with nothing
/// yet performed.
///
/// The order is where the work continues first, then the rest canonically —
/// [`RestorationSelection::restoration_key`], the same key
/// [`ordered_attempt_targets`] announces with. Restoring in Artifact-identity
/// order would be equally deterministic and would still put the person's own
/// place somewhere in the middle of it.
pub fn plan_selection<P: PreflightSource>(
    selection: &RestorationSelection,
    locators: &HashMap<String, Locator>,
    preflight: &P,
) -> Vec<RestorationStep> {
    let mut steps: Vec<RestorationStep> = preflight_selection(selection, preflight)
        .into_iter()
        .map(|outcome| {
            step_for(
                outcome.artifact_id().clone(),
                outcome.status().clone(),
                locators,
            )
        })
        .collect();
    steps.sort_by(|a, b| {
        selection
            .restoration_key(a.artifact_id())
            .cmp(&selection.restoration_key(b.artifact_id()))
    });
    steps
}

/// Plans the restoration of a canonical request: the Resume Point first, then
/// the Context Chain, each preflighted, with nothing yet performed.
pub fn plan_request<P: PreflightSource>(
    request: &ExecutionRequest,
    locators: &HashMap<String, Locator>,
    preflight: &P,
) -> Vec<RestorationStep> {
    ordered_targets(request)
        .into_iter()
        .map(|artifact_id| {
            // The plan path resolves the locator first, because a target with
            // no canonical evidence has nothing to preflight.
            let status = match locators.get(&artifact_id.to_string()) {
                Some(locator) => preflight_locator_kind(locator.kind(), preflight),
                None => PreflightStatus::Unavailable {
                    reason: format!(
                        "no canonical executable target is recorded for Artifact {}; \
                         no action was performed",
                        artifact_id.as_str()
                    ),
                },
            };
            step_for(artifact_id, status, locators)
        })
        .collect()
}

/// Performs exactly one planned step and reports what actually happened.
///
/// This is the single place in Evo where an OS action is taken on behalf of a
/// restoration, and it is deliberately one step wide: a caller can drive a
/// restoration incrementally and report each result the moment it arrives,
/// while the whole-report callers ([`execute`], [`execute_selection`]) simply
/// run every step in order. Both therefore act identically.
///
/// The capability order lives here: a READY target is first **bound** to the
/// live resource it names right now ([`crate::binding::bind_resource`]), and
/// only then acted on — raised if the user already has it, opened if they do
/// not. A target that cannot be bound is reported honestly and nothing is
/// performed for it.
pub fn execute_step<E: PlatformExecutor, P: PreflightSource>(
    step: &RestorationStep,
    binding_source: &P,
    executor: &E,
) -> TargetAttempt {
    let status = match step.plan() {
        StepPlan::Refuse(status) => status.clone(),
        StepPlan::Attempt(kind) => {
            match bind_resource(step.artifact_id(), kind, binding_source) {
                Err(failure) => match failure {
                    // Several equally valid live windows is an ambiguity, not a
                    // failure of the machine: Evo declines to choose.
                    BindingFailure::SeveralLiveWindows { .. } => TargetStatus::Ambiguous {
                        reason: failure.reason(),
                    },
                    _ => TargetStatus::Unavailable {
                        reason: failure.reason(),
                    },
                },
                Ok(binding) => match binding.resource() {
                    // Tier 1 — the user already has it; raise it.
                    BoundResource::FocusWindow { window }
                    | BoundResource::FocusDocumentWindow { window, .. } => {
                        executor.focus_bound_window(window)
                    }
                    // Tier 2 — hand it to the system. Where the binding could
                    // not establish that the file was closed, the caveat rides
                    // along with the result: Evo says what it did *and* what it
                    // could not verify, rather than reporting a bare success.
                    BoundResource::OpenFile { path, witnessed } => {
                        qualify(executor.open_file(path), witnessed.reason())
                    }
                    BoundResource::OpenUrl { url } => executor.open_url(url),
                },
            }
        }
    };
    TargetAttempt::new(step.artifact_id().clone(), status)
}

/// Attaches a binding caveat to a successful open, leaving every other status
/// exactly as the executor reported it.
///
/// Only `Opened` is qualified: a failure already carries the system's own
/// reason, and adding a second one would obscure it.
fn qualify(status: TargetStatus, caveat: Option<String>) -> TargetStatus {
    match (status, caveat) {
        (TargetStatus::Opened { detail }, Some(caveat)) => TargetStatus::Opened {
            detail: if detail.is_empty() {
                caveat
            } else {
                format!("{detail} — {caveat}")
            },
        },
        (status, _) => status,
    }
}

/// Runs a planned restoration to completion, in order.
fn run<E: PlatformExecutor, P: PreflightSource>(
    steps: &[RestorationStep],
    binding_source: &P,
    executor: &E,
) -> ExecutionReport {
    ExecutionReport::new(
        steps
            .iter()
            .map(|step| execute_step(step, binding_source, executor))
            .collect(),
    )
}

/// Executes the derived selective-restoration selection for one Workspace.
///
/// Every member of the selection's current continuation — restore-worthy ∪
/// unavailable, in canonical order (ascending ArtifactId) — is preflighted
/// ([`crate::preflight::preflight_selection`]) before anything is attempted.
/// Exactly the READY targets are attempted, in that canonical order;
/// unavailable, ambiguous, and unsupported targets are never attempted and
/// never guessed into a target. Historical members are never passed to
/// execution at all. Every surface member receives an honest per-target
/// outcome: what was attempted, what succeeded, what could not be attempted,
/// and why.
///
/// This is the Continue path once a Continuation Surface has been declared
/// (RFC-0013): the user's declared continuation, narrowed to what can
/// actually be reopened. Recency, frequency, and application identity play
/// no role. Deterministic given identical selection + locator + preflight
/// inputs.
pub fn execute_selection<E: PlatformExecutor, P: PreflightSource>(
    selection: &RestorationSelection,
    locators: &HashMap<String, Locator>,
    preflight: &P,
    executor: &E,
) -> ExecutionReport {
    run(
        &plan_selection(selection, locators, preflight),
        preflight,
        executor,
    )
}

/// The preflight disposition of one locator kind (used by the plan path).
///
/// Window/file/URL locators resolve through the same canonical identity rule
/// as the selection path; there is no locator for an inherently
/// non-executable resource, so a missing locator is honest unavailability.
fn preflight_locator_kind(kind: &LocatorKind, preflight: &dyn PreflightSource) -> PreflightStatus {
    match kind {
        LocatorKind::WindowTitle(title) => {
            preflight_identity(&ResourceIdentity::WindowTitle(title.clone()), preflight)
        }
        LocatorKind::FilePath(path) => {
            preflight_identity(&ResourceIdentity::FilePath(path.clone()), preflight)
        }
        LocatorKind::Url(url) => preflight_identity(&ResourceIdentity::Url(url.clone()), preflight),
    }
}

/// Executes one canonical request through a platform executor.
///
/// For every ordered target the engine resolves the Artifact's executable
/// target from canonical evidence (`locators`) and preflights it before any
/// OS action: only READY targets are attempted, in canonical order; a target
/// that is unavailable, ambiguous, or unsupported at execution time is
/// reported honestly and never substituted. A target with no canonical
/// locator is reported [`TargetStatus::Unavailable`] — the engine never
/// guesses one.
pub fn execute<E: PlatformExecutor, P: PreflightSource>(
    request: &ExecutionRequest,
    locators: &HashMap<String, Locator>,
    preflight: &P,
    executor: &E,
) -> ExecutionReport {
    run(
        &plan_request(request, locators, preflight),
        preflight,
        executor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locator::LocatorKind;
    use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};
    use evo_workspace::WorkspaceId;

    fn resume_point(workspace_id: &WorkspaceId, artifact: &str) -> ResumePoint {
        ResumePoint::new(workspace_id.clone(), ArtifactId::new(artifact).unwrap())
    }

    fn request_with_chain(resume: &str, chain: &[&str]) -> ExecutionRequest {
        let workspace_id = WorkspaceId::new();
        let artifacts: Vec<ArtifactId> =
            chain.iter().map(|s| ArtifactId::new(*s).unwrap()).collect();
        let plan = RestorationPlan::new(
            workspace_id.clone(),
            resume_point(&workspace_id, resume),
            ContextChain::new(artifacts).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue after the resume point").unwrap(),
        )
        .unwrap();
        ExecutionRequest::Plan(plan)
    }

    #[derive(Debug, Default)]
    struct RecordingExecutor {
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl PlatformExecutor for RecordingExecutor {
        fn open_url(&self, url: &str) -> TargetStatus {
            self.calls.lock().unwrap().push(format!("url:{url}"));
            TargetStatus::Opened {
                detail: format!("opened {url}"),
            }
        }
        fn open_file(&self, path: &str) -> TargetStatus {
            self.calls.lock().unwrap().push(format!("file:{path}"));
            TargetStatus::Opened {
                detail: format!("opened {path}"),
            }
        }
        fn focus_window(&self, title: &str) -> TargetStatus {
            self.calls.lock().unwrap().push(format!("window:{title}"));
            TargetStatus::Opened {
                detail: format!("focused {title}"),
            }
        }
    }

    /// A deterministic preflight source for engine tests: every file exists
    /// and the window list is injected, so execution tests exercise the
    /// engine without depending on real OS state.
    #[derive(Debug, Default)]
    struct ReadyPreflight {
        windows: Vec<crate::macos::WindowInfo>,
    }

    impl ReadyPreflight {
        fn with_windows(titles: &[&str]) -> Self {
            Self {
                windows: titles
                    .iter()
                    .enumerate()
                    .map(|(index, title)| crate::macos::WindowInfo {
                        title: title.to_string(),
                        owner_pid: index as i32 + 1,
                        owner_name: "Fixture App".to_string(),
                    })
                    .collect(),
            }
        }
    }

    impl PreflightSource for ReadyPreflight {
        fn file_exists(&self, _path: &str) -> bool {
            true
        }
        fn windows(&self) -> Result<Vec<crate::macos::WindowInfo>, String> {
            Ok(self.windows.clone())
        }
    }

    fn locators(pairs: &[(&str, LocatorKind)]) -> HashMap<String, Locator> {
        pairs
            .iter()
            .map(|(id, kind)| {
                (
                    id.to_string(),
                    Locator::new(ArtifactId::new(*id).unwrap(), kind.clone()),
                )
            })
            .collect()
    }

    #[test]
    fn executes_resume_point_first_then_context_chain() {
        let request = request_with_chain("rp", &["support-a", "support-b"]);
        let executor = RecordingExecutor::default();
        let locs = locators(&[
            ("rp", LocatorKind::Url("https://rp.dev".into())),
            ("support-a", LocatorKind::FilePath("/a.txt".into())),
            ("support-b", LocatorKind::WindowTitle("B".into())),
        ]);
        let report = execute(
            &request,
            &locs,
            &ReadyPreflight::with_windows(&["B"]),
            &executor,
        );
        let calls = executor.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["url:https://rp.dev", "file:/a.txt", "window:B"]);
        assert!(report.all_opened());
        assert_eq!(report.attempts().len(), 3);
    }

    #[test]
    fn duplicates_are_collapsed_preserving_first_occurrence() {
        // The canonical Context Chain forbids duplicates, but a Resume Point
        // that also appears in the chain is a legitimate shape; the engine
        // must act on it exactly once, in its first (Resume Point) position.
        let workspace_id = WorkspaceId::new();
        let plan = RestorationPlan::new(
            workspace_id.clone(),
            resume_point(&workspace_id, "rp"),
            ContextChain::new(vec![ArtifactId::new("rp").unwrap()]).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
        )
        .unwrap();
        let request = ExecutionRequest::Plan(plan);
        let executor = RecordingExecutor::default();
        let locs = locators(&[("rp", LocatorKind::Url("https://rp.dev".into()))]);
        let report = execute(&request, &locs, &ReadyPreflight::default(), &executor);
        let calls = executor.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["url:https://rp.dev"]);
        assert_eq!(report.attempts().len(), 1);
    }

    #[test]
    fn missing_locator_reports_unavailable_never_guesses() {
        let request = request_with_chain("rp", &["support-a"]);
        let executor = RecordingExecutor::default();
        // No locator for "support-a".
        let locs = locators(&[("rp", LocatorKind::Url("https://rp.dev".into()))]);
        let report = execute(&request, &locs, &ReadyPreflight::default(), &executor);
        let attempts = report.attempts();
        assert_eq!(attempts.len(), 2);
        assert!(matches!(attempts[0].status(), TargetStatus::Opened { .. }));
        assert!(matches!(
            attempts[1].status(),
            TargetStatus::Unavailable { reason } if reason.contains("support-a")
        ));
        // The executor must not have been called for the missing target.
        let calls = executor.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["url:https://rp.dev"]);
    }

    #[test]
    fn execution_is_deterministic() {
        let request = request_with_chain("rp", &["support-a"]);
        let locs = locators(&[
            ("rp", LocatorKind::Url("https://rp.dev".into())),
            ("support-a", LocatorKind::FilePath("/a.txt".into())),
        ]);
        let first = execute(
            &request,
            &locs,
            &ReadyPreflight::default(),
            &RecordingExecutor::default(),
        );
        let second = execute(
            &request,
            &locs,
            &ReadyPreflight::default(),
            &RecordingExecutor::default(),
        );
        // Deterministic on outcome shape; executor side effects are not part
        // of the report equality for separate executors, so compare shapes.
        assert_eq!(first.attempts().len(), second.attempts().len());
        assert_eq!(first.attempts()[0].status(), second.attempts()[0].status());
    }

    #[test]
    fn ordered_targets_puts_resume_point_first() {
        let request = request_with_chain("rp", &["support-a"]);
        let targets = ordered_targets(&request);
        assert_eq!(targets[0].as_str(), "rp");
        assert_eq!(targets[1].as_str(), "support-a");
    }

    #[test]
    fn resume_point_only_request_targets_resume_point() {
        let workspace_id = WorkspaceId::new();
        let request = ExecutionRequest::ResumePoint(resume_point(&workspace_id, "only"));
        let targets = ordered_targets(&request);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].as_str(), "only");
    }

    #[test]
    fn ordered_attempt_targets_surface_branch_is_restore_worthy_in_canonical_order() {
        // A declared surface announces its restore-worthy members in canonical
        // order (ascending ArtifactId) — exactly the order execute_selection
        // attempts them — never the Resume Point first. The Resume Point here
        // is "artifact-url", which sorts AFTER "artifact-file": the announced
        // order must not follow the Resume Point, or it would diverge from
        // what execution actually attempts (BE-AUDIT-0001 §2.4).
        let selection = fixture_selection();
        let workspace_id = WorkspaceId::new();
        let plan = RestorationPlan::new_with_continuation_surface(
            workspace_id.clone(),
            resume_point(&workspace_id, "artifact-url"),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
            vec![
                ArtifactId::new("artifact-url").unwrap(),
                ArtifactId::new("artifact-file").unwrap(),
                ArtifactId::new("artifact-commit").unwrap(),
            ],
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);
        let targets = ordered_attempt_targets(Some(&outcome), Some(&selection)).unwrap();
        let ids: Vec<&str> = targets.iter().map(|id| id.as_str()).collect();
        // Canonical order (ascending ArtifactId), restore-worthy only; the
        // commit is unavailable and is never announced as openable.
        assert_eq!(ids, vec!["artifact-file", "artifact-url"]);
    }

    #[test]
    fn ordered_attempt_targets_without_a_surface_is_resume_point_first() {
        let request = request_with_chain("rp", &["support-a"]);
        let outcome = match &request {
            ExecutionRequest::Plan(plan) => {
                evo_restoration::DerivationOutcome::Complete(plan.clone())
            }
            ExecutionRequest::ResumePoint(_) => unreachable!("request_with_chain builds a Plan"),
        };
        let targets = ordered_attempt_targets(Some(&outcome), None).unwrap();
        // The non-surface branch announces exactly the same set, in the same
        // order, that ordered_targets drives execution with.
        assert_eq!(targets, ordered_targets(&request));
        assert_eq!(targets[0].as_str(), "rp");
        assert_eq!(targets[1].as_str(), "support-a");
    }

    #[test]
    fn ordered_attempt_targets_refuses_without_outcome_or_selection() {
        // No derived outcome at all: an honest refusal, not an empty plan.
        assert!(ordered_attempt_targets(None, None).is_err());
        // A declared surface without a derived selection refuses honestly
        // rather than guessing targets.
        let selection = fixture_selection();
        let workspace_id = WorkspaceId::new();
        let plan = RestorationPlan::new_with_continuation_surface(
            workspace_id.clone(),
            resume_point(&workspace_id, "artifact-file"),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
            vec![ArtifactId::new("artifact-file").unwrap()],
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);
        assert!(ordered_attempt_targets(Some(&outcome), None).is_err());
        assert!(ordered_attempt_targets(Some(&outcome), Some(&selection)).is_ok());
    }

    #[test]
    fn report_all_opened_is_false_when_any_failed() {
        let request = request_with_chain("rp", &["support-a"]);
        let locs = locators(&[("rp", LocatorKind::Url("https://rp.dev".into()))]);
        let report = execute(
            &request,
            &locs,
            &ReadyPreflight::default(),
            &RecordingExecutor::default(),
        );
        assert!(!report.all_opened());
    }

    #[test]
    fn report_all_opened_is_true_when_every_target_opened() {
        let request = request_with_chain("rp", &[]);
        let locs = locators(&[("rp", LocatorKind::Url("https://rp.dev".into()))]);
        let report = execute(
            &request,
            &locs,
            &ReadyPreflight::default(),
            &RecordingExecutor::default(),
        );
        assert!(report.all_opened());
    }

    // ── execute_selection: Continue acts on the derived selection ─────────

    use crate::resource::ResourceIdentity;
    use crate::selection::{
        Disposition, RestorationSelection, SelectedResource, UnavailableReason, WithheldMember,
    };

    /// Builds a selection: two restore-worthy resources (file + URL), one
    /// unavailable commit, and one withheld member kept only as record.
    fn fixture_selection() -> RestorationSelection {
        let restore_worthy = vec![
            SelectedResource::new(
                ArtifactId::new("artifact-file").unwrap(),
                ResourceIdentity::FilePath("/repo/a.md".into()),
                "this is where the work continues",
            ),
            SelectedResource::new(
                ArtifactId::new("artifact-url").unwrap(),
                ResourceIdentity::Url("https://a.dev".into()),
                "the continuation also happens here",
            ),
        ];
        let unavailable = vec![UnavailableReason::new(
            ArtifactId::new("artifact-commit").unwrap(),
            Some(ResourceIdentity::Commit("9f86d081884c7d65".into())),
            "a commit has no executable target",
        )];
        let withheld = vec![WithheldMember::new(
            ArtifactId::new("artifact-old").unwrap(),
            Disposition::HistoryOnly,
            "present while the work happened, and nothing more",
        )];
        RestorationSelection::new(WorkspaceId::new(), restore_worthy, unavailable, withheld)
    }

    #[test]
    fn execute_selection_acts_only_on_restore_worthy_members() {
        let selection = fixture_selection();
        let executor = RecordingExecutor::default();
        let locs = locators(&[
            ("artifact-file", LocatorKind::FilePath("/repo/a.md".into())),
            ("artifact-url", LocatorKind::Url("https://a.dev".into())),
            // The commit has a locator in the map too; it must never be
            // consulted because it is not restore-worthy.
            (
                "artifact-commit",
                LocatorKind::Url("https://git.dev/abc".into()),
            ),
        ]);
        let report = execute_selection(&selection, &locs, &ReadyPreflight::default(), &executor);
        let calls = executor.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["file:/repo/a.md", "url:https://a.dev"]);
        // Every surface member receives an honest per-target outcome: the
        // commit is reported UNSUPPORTED (never attempted, never guessed), the
        // two ready targets were attempted and opened.
        assert_eq!(report.attempts().len(), 3);
        assert!(!report.all_opened());
        let commit = report
            .attempts()
            .iter()
            .find(|attempt| attempt.artifact_id().as_str() == "artifact-commit")
            .expect("the commit is a surface member and is reported");
        assert!(matches!(commit.status(), TargetStatus::Unsupported { .. }));
        let opened: Vec<&str> = report
            .attempts()
            .iter()
            .filter(|attempt| matches!(attempt.status(), TargetStatus::Opened { .. }))
            .map(|attempt| attempt.artifact_id().as_str())
            .collect();
        assert_eq!(opened, vec!["artifact-file", "artifact-url"]);
        // The historical member was never passed to execution.
        for attempt in report.attempts() {
            assert_ne!(attempt.artifact_id().as_str(), "artifact-old");
        }
    }

    #[test]
    fn execute_selection_missing_locator_reports_unavailable() {
        let selection = fixture_selection();
        let executor = RecordingExecutor::default();
        // Only the URL has a recorded locator; the file is restore-worthy but
        // has no canonical locator.
        let locs = locators(&[("artifact-url", LocatorKind::Url("https://a.dev".into()))]);
        let report = execute_selection(&selection, &locs, &ReadyPreflight::default(), &executor);
        let attempts = report.attempts();
        assert_eq!(attempts.len(), 3);
        let file = attempts
            .iter()
            .find(|attempt| attempt.artifact_id().as_str() == "artifact-file")
            .expect("the file is a surface member and is reported");
        assert!(matches!(
            file.status(),
            TargetStatus::Unavailable { reason } if reason.contains("artifact-file")
        ));
        let url = attempts
            .iter()
            .find(|attempt| attempt.artifact_id().as_str() == "artifact-url")
            .expect("the url is a surface member and is reported");
        assert!(matches!(url.status(), TargetStatus::Opened { .. }));
        // The executor was called only for the resolvable target.
        let calls = executor.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["url:https://a.dev"]);
    }

    #[test]
    fn execute_selection_empty_selection_attempts_nothing() {
        let selection = RestorationSelection::new(
            WorkspaceId::new(),
            vec![],
            vec![],
            vec![WithheldMember::new(
                ArtifactId::new("artifact-historical").unwrap(),
                Disposition::HistoryOnly,
                "present while the work happened, and nothing more",
            )],
        );
        let executor = RecordingExecutor::default();
        let report = execute_selection(
            &selection,
            &HashMap::new(),
            &ReadyPreflight::default(),
            &executor,
        );
        assert!(report.attempts().is_empty());
        assert!(!report.all_opened());
        assert!(executor.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn execute_selection_is_deterministic() {
        let selection = fixture_selection();
        let locs = locators(&[
            ("artifact-file", LocatorKind::FilePath("/repo/a.md".into())),
            ("artifact-url", LocatorKind::Url("https://a.dev".into())),
        ]);
        let first = execute_selection(
            &selection,
            &locs,
            &ReadyPreflight::default(),
            &RecordingExecutor::default(),
        );
        let second = execute_selection(
            &selection,
            &locs,
            &ReadyPreflight::default(),
            &RecordingExecutor::default(),
        );
        assert_eq!(first.attempts().len(), second.attempts().len());
        assert_eq!(first.attempts()[0].status(), second.attempts()[0].status());
        assert_eq!(first.attempts()[1].status(), second.attempts()[1].status());
        assert_eq!(first.attempts()[2].status(), second.attempts()[2].status());
    }

    // ── Executor Preflight: partial execution and the per-target outcome ────

    /// A fully configurable preflight source: exact file set + injected
    /// window list (or an error). Deterministic and OS-free.
    struct FixturePreflight {
        files: Vec<String>,
        windows: Vec<crate::macos::WindowInfo>,
        window_error: Option<String>,
    }

    impl FixturePreflight {
        fn new(files: &[&str]) -> Self {
            Self {
                files: files.iter().map(|f| f.to_string()).collect(),
                windows: Vec::new(),
                window_error: None,
            }
        }
        fn with_windows(titles: &[&str]) -> Self {
            let windows = titles
                .iter()
                .enumerate()
                .map(|(index, title)| crate::macos::WindowInfo {
                    title: title.to_string(),
                    owner_pid: index as i32 + 1,
                    owner_name: "Fixture App".to_string(),
                })
                .collect();
            Self {
                files: Vec::new(),
                windows,
                window_error: None,
            }
        }
    }

    impl PreflightSource for FixturePreflight {
        fn file_exists(&self, path: &str) -> bool {
            self.files.iter().any(|file| file == path)
        }
        fn windows(&self) -> Result<Vec<crate::macos::WindowInfo>, String> {
            match &self.window_error {
                Some(reason) => Err(reason.clone()),
                None => Ok(self.windows.clone()),
            }
        }
    }

    fn selected(id: &str, identity: ResourceIdentity) -> SelectedResource {
        SelectedResource::new(
            ArtifactId::new(id).unwrap(),
            identity,
            "the continuation of this work happens here",
        )
    }

    /// Builds a selection with the five dispositions of the partial-execution
    /// contract: two READY (file + URL), one UNAVAILABLE (missing file), one
    /// AMBIGUOUS (two matching windows), one UNSUPPORTED (commit).
    fn mixed_selection() -> RestorationSelection {
        let restore_worthy = vec![
            selected(
                "artifact-a",
                ResourceIdentity::FilePath("/repo/a.md".into()),
            ),
            selected(
                "artifact-url",
                ResourceIdentity::Url("https://a.dev".into()),
            ),
            selected(
                "artifact-missing",
                ResourceIdentity::FilePath("/repo/removed.md".into()),
            ),
            selected(
                "artifact-window",
                ResourceIdentity::WindowTitle("Untitled".into()),
            ),
        ];
        let unavailable = vec![UnavailableReason::new(
            ArtifactId::new("artifact-commit").unwrap(),
            Some(ResourceIdentity::Commit("9f86d081884c7d65".into())),
            "this resource is a repository object (a commit) and has no executable target",
        )];
        RestorationSelection::new(WorkspaceId::new(), restore_worthy, unavailable, vec![])
    }

    #[test]
    fn mixed_selection_partial_execution_succeeds() {
        // A→READY, B→READY, C→UNAVAILABLE, D→AMBIGUOUS, E→UNSUPPORTED:
        // A and B are attempted; C, D, E are skipped; every surface member
        // gets an honest outcome.
        let selection = mixed_selection();
        let preflight = FixturePreflight {
            files: vec!["/repo/a.md".to_string()],
            windows: vec![
                crate::macos::WindowInfo {
                    title: "Untitled".to_string(),
                    owner_pid: 1,
                    owner_name: "Notes".to_string(),
                },
                crate::macos::WindowInfo {
                    title: "Untitled".to_string(),
                    owner_pid: 2,
                    owner_name: "TextEdit".to_string(),
                },
            ],
            window_error: None,
        };
        let locs = locators(&[
            ("artifact-a", LocatorKind::FilePath("/repo/a.md".into())),
            ("artifact-url", LocatorKind::Url("https://a.dev".into())),
            (
                "artifact-missing",
                LocatorKind::FilePath("/repo/removed.md".into()),
            ),
            (
                "artifact-window",
                LocatorKind::WindowTitle("Untitled".into()),
            ),
        ]);
        let executor = RecordingExecutor::default();
        let report = execute_selection(&selection, &locs, &preflight, &executor);

        // Exactly the two READY targets were attempted.
        let calls = executor.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["file:/repo/a.md", "url:https://a.dev"]);
        // Every surface member has an outcome: 2 attempted + 3 skipped.
        assert_eq!(report.attempts().len(), 5);
        let by_id: HashMap<String, &TargetStatus> = report
            .attempts()
            .iter()
            .map(|attempt| (attempt.artifact_id().to_string(), attempt.status()))
            .collect();
        assert!(matches!(by_id["artifact-a"], TargetStatus::Opened { .. }));
        assert!(matches!(by_id["artifact-url"], TargetStatus::Opened { .. }));
        assert!(matches!(
            by_id["artifact-missing"],
            TargetStatus::Unavailable { reason } if reason.contains("removed.md")
        ));
        assert!(matches!(
            by_id["artifact-window"],
            TargetStatus::Ambiguous { .. }
        ));
        assert!(matches!(
            by_id["artifact-commit"],
            TargetStatus::Unsupported { .. }
        ));
        // What succeeded, what could not be attempted, and why: every skipped
        // target carries an honest reason.
        for (id, status) in &by_id {
            if !matches!(status, TargetStatus::Opened { .. }) {
                let reason = match status {
                    TargetStatus::Unavailable { reason }
                    | TargetStatus::Ambiguous { reason }
                    | TargetStatus::Unsupported { reason } => reason,
                    _ => unreachable!(),
                };
                assert!(
                    !reason.is_empty(),
                    "{id} must explain why it was not attempted"
                );
            }
        }
    }

    #[test]
    fn unavailable_target_does_not_prevent_another_target() {
        // A missing file must not block the independently executable file
        // that follows it in canonical order.
        let selection = RestorationSelection::new(
            WorkspaceId::new(),
            vec![
                selected(
                    "artifact-a-missing",
                    ResourceIdentity::FilePath("/repo/gone.md".into()),
                ),
                selected(
                    "artifact-b-present",
                    ResourceIdentity::FilePath("/repo/present.md".into()),
                ),
            ],
            vec![],
            vec![],
        );
        let preflight = FixturePreflight::new(&["/repo/present.md"]);
        let locs = locators(&[
            (
                "artifact-a-missing",
                LocatorKind::FilePath("/repo/gone.md".into()),
            ),
            (
                "artifact-b-present",
                LocatorKind::FilePath("/repo/present.md".into()),
            ),
        ]);
        let executor = RecordingExecutor::default();
        let report = execute_selection(&selection, &locs, &preflight, &executor);
        let calls = executor.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["file:/repo/present.md"]);
        let by_id: HashMap<&str, &TargetStatus> = report
            .attempts()
            .iter()
            .map(|attempt| (attempt.artifact_id().as_str(), attempt.status()))
            .collect();
        assert!(matches!(
            by_id["artifact-a-missing"],
            TargetStatus::Unavailable { .. }
        ));
        assert!(matches!(
            by_id["artifact-b-present"],
            TargetStatus::Opened { .. }
        ));
    }

    #[test]
    fn ambiguous_target_never_causes_arbitrary_selection() {
        // Two equally valid windows: preflight is AMBIGUOUS and the executor
        // is never asked to choose between them.
        let selection = RestorationSelection::new(
            WorkspaceId::new(),
            vec![selected(
                "artifact-window",
                ResourceIdentity::WindowTitle("Untitled".into()),
            )],
            vec![],
            vec![],
        );
        let preflight = FixturePreflight::with_windows(&["Untitled", "Untitled"]);
        let locs = locators(&[(
            "artifact-window",
            LocatorKind::WindowTitle("Untitled".into()),
        )]);
        let executor = RecordingExecutor::default();
        let report = execute_selection(&selection, &locs, &preflight, &executor);
        assert!(matches!(
            report.attempts()[0].status(),
            TargetStatus::Ambiguous { reason } if reason.contains("2 open windows")
        ));
        assert!(
            executor.calls.lock().unwrap().is_empty(),
            "an ambiguous target is never arbitrarily selected"
        );
    }

    #[test]
    fn historical_resources_are_never_passed_to_execution() {
        // A withheld member (outside the current continuation) must never
        // be preflighted, attempted, or reported — even when a locator and
        // evidence for it exist. This holds for the *stronger* withheld
        // disposition: a place this work happens, kept to hand, is still not
        // opened unasked.
        let historical = ArtifactId::new("artifact-historical").unwrap();
        let selection = RestorationSelection::new(
            WorkspaceId::new(),
            vec![selected(
                "artifact-current",
                ResourceIdentity::FilePath("/repo/current.md".into()),
            )],
            vec![],
            vec![WithheldMember::new(
                historical.clone(),
                Disposition::AvailableIfNeeded,
                "one of the places this work happens, but not the continuation",
            )],
        );
        let preflight = FixturePreflight::new(&["/repo/current.md", "/repo/old.md"]);
        let locs = locators(&[
            (
                "artifact-current",
                LocatorKind::FilePath("/repo/current.md".into()),
            ),
            (
                "artifact-historical",
                LocatorKind::FilePath("/repo/old.md".into()),
            ),
        ]);
        let executor = RecordingExecutor::default();
        let report = execute_selection(&selection, &locs, &preflight, &executor);
        assert_eq!(report.attempts().len(), 1);
        let calls = executor.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["file:/repo/current.md"]);
    }

    #[test]
    fn undeclared_recent_resources_are_never_passed_to_execution() {
        // Resources witnessed recently but never declared into the
        // continuation are Workspace members outside the surface: historical
        // for execution purposes. They must never be attempted even though a
        // locator exists.
        let selection = RestorationSelection::new(
            WorkspaceId::new(),
            vec![selected(
                "artifact-declared",
                ResourceIdentity::FilePath("/repo/declared.md".into()),
            )],
            vec![],
            vec![WithheldMember::new(
                ArtifactId::new("artifact-recent-chat").unwrap(),
                Disposition::HistoryOnly,
                "witnessed while the work was happening, and nothing more",
            )],
        );
        let preflight = FixturePreflight::new(&["/repo/declared.md", "/repo/chat.md"]);
        let locs = locators(&[
            (
                "artifact-declared",
                LocatorKind::FilePath("/repo/declared.md".into()),
            ),
            (
                "artifact-recent-chat",
                LocatorKind::FilePath("/repo/chat.md".into()),
            ),
        ]);
        let executor = RecordingExecutor::default();
        let report = execute_selection(&selection, &locs, &preflight, &executor);
        assert_eq!(report.attempts().len(), 1);
        assert_eq!(
            executor.calls.lock().unwrap().clone(),
            vec!["file:/repo/declared.md"]
        );
    }

    #[test]
    fn tool_switching_does_not_alter_resource_identity_or_execution() {
        // The same canonical file path witnessed again (as a different editor
        // would) is the same resource identity: execution resolves the target
        // from the identity alone — there is no application-name input, and a
        // re-witnessed identical identity produces the identical attempt.
        let first = RestorationSelection::new(
            WorkspaceId::new(),
            vec![selected(
                "artifact-file",
                ResourceIdentity::FilePath("/repo/src/lib.rs".into()),
            )],
            vec![],
            vec![],
        );
        let second = RestorationSelection::new(
            WorkspaceId::new(),
            vec![selected(
                "artifact-file",
                ResourceIdentity::FilePath("/repo/src/lib.rs".into()),
            )],
            vec![],
            vec![],
        );
        let preflight = FixturePreflight::new(&["/repo/src/lib.rs"]);
        let locs = locators(&[(
            "artifact-file",
            LocatorKind::FilePath("/repo/src/lib.rs".into()),
        )]);
        let executor = RecordingExecutor::default();
        let report = execute_selection(&first, &locs, &preflight, &executor);
        let again = execute_selection(&second, &locs, &preflight, &executor);
        assert_eq!(report, again);
        assert_eq!(
            executor.calls.lock().unwrap().clone(),
            vec!["file:/repo/src/lib.rs", "file:/repo/src/lib.rs"]
        );
        // The identity itself carries no application name.
        assert_eq!(
            ResourceIdentity::FilePath("/repo/src/lib.rs".into()).subject(),
            "/repo/src/lib.rs"
        );
    }

    #[test]
    fn execution_attempts_are_deterministically_ordered() {
        // The report iterates the surface in canonical order (ascending
        // ArtifactId) and is identical across identical inputs.
        let selection = mixed_selection();
        let preflight = FixturePreflight::new(&["/repo/a.md"]);
        let locs = locators(&[
            ("artifact-a", LocatorKind::FilePath("/repo/a.md".into())),
            ("artifact-url", LocatorKind::Url("https://a.dev".into())),
            (
                "artifact-missing",
                LocatorKind::FilePath("/repo/removed.md".into()),
            ),
            (
                "artifact-window",
                LocatorKind::WindowTitle("Untitled".into()),
            ),
        ]);
        let first = execute_selection(&selection, &locs, &preflight, &RecordingExecutor::default());
        let second =
            execute_selection(&selection, &locs, &preflight, &RecordingExecutor::default());
        let ids: Vec<String> = first
            .attempts()
            .iter()
            .map(|attempt| attempt.artifact_id().to_string())
            .collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "deterministic canonical order");
        assert_eq!(first, second);
    }

    #[test]
    fn restart_replay_does_not_alter_derived_selection_or_execution() {
        // A full replay re-derives the identical selection from identical
        // canonical inputs; executing the re-derived selection produces the
        // identical report. (The selection layer's own determinism tests cover
        // the selection equality; here the execution follows the selection.)
        let selection = mixed_selection();
        let replayed = mixed_selection();
        // A fresh WorkspaceId is presentation-independent of the derived
        // understanding; the canonical content a replay reproduces is the
        // partitions themselves.
        assert_eq!(selection.restore_worthy(), replayed.restore_worthy());
        assert_eq!(selection.unavailable(), replayed.unavailable());
        assert_eq!(selection.withheld(), replayed.withheld());
        let preflight = FixturePreflight::new(&["/repo/a.md"]);
        let locs = locators(&[
            ("artifact-a", LocatorKind::FilePath("/repo/a.md".into())),
            ("artifact-url", LocatorKind::Url("https://a.dev".into())),
            (
                "artifact-missing",
                LocatorKind::FilePath("/repo/removed.md".into()),
            ),
            (
                "artifact-window",
                LocatorKind::WindowTitle("Untitled".into()),
            ),
        ]);
        let original =
            execute_selection(&selection, &locs, &preflight, &RecordingExecutor::default());
        let after_restart =
            execute_selection(&replayed, &locs, &preflight, &RecordingExecutor::default());
        assert_eq!(original, after_restart);
    }
}
