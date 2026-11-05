//! The restore executor: the verb that makes Evo a product.
//!
//! The engine knows *what* the person's work is and *which neighborhood* it
//! continues in; this module turns that knowledge into open windows. The
//! contract is deliberately small: a list of resource strings in, an honest
//! account of what happened out.
//!
//! # How a resource becomes an action
//!
//! Classification reuses the witness layer's own vocabulary — never
//! application semantics:
//!
//! * a **URL** opens in the browser (the OS default handler, or the app it
//!   was witnessed in when the caller knows it);
//! * a **file path** opens through its owning application, and if a live
//!   window already shows that exact document (by the window's own
//!   `AXDocument` claim), that window is *raised* instead — restoring is
//!   not duplicating;
//! * a **window title** can only ever be raised: a title is not something
//!   the system can open, so if no live window carries it, the restore
//!   says so and moves on.
//!
//! Nothing here knows what any resource *means*. "Open the resource in the
//! app you were using it in" is a witnessed fact, not a semantic; and when
//! no app was witnessed, the operating system's own handler decides. The
//! person's intent is never guessed at the execution layer — it was
//! attributed upstream or it does not reach this code.
//!
//! # Honesty of the outcome
//!
//! Every target lands in exactly one bucket — opened, raised, or
//! unopened-with-reason — and the buckets are reported whole. A restore
//! that silently skipped half the neighborhood is worse than one that says
//! "could not open this": the person trusts the second and corrects it.

use crate::browser_macos::{BrowserIdentity, MacOSBrowserAdapter, restore_browser_identity};
use crate::locator::{LocatorKind, classify_subject};
use crate::macos::{self, WindowInfo, WindowSelectionError, select_unique_window};
use evo_observation::surface_identity::SurfaceIdentity;

/// One resource to bring back, with the application it was witnessed in
/// when the caller knows it (the app association is observed fact, never
/// inferred here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreTarget {
    pub resource: String,
    pub app: Option<String>,
    /// Exact witnessed surface constraints. When present, reopening or title
    /// matching is forbidden, including when native evidence is unavailable.
    pub surface: Option<SurfaceIdentity>,
}

impl RestoreTarget {
    /// A target with no witnessed application; the OS default handler owns it.
    pub fn with_default_handler(resource: &str) -> Self {
        Self {
            resource: resource.to_string(),
            app: None,
            surface: None,
        }
    }
}

/// One planned action per target. Pure, deterministic, testable: the plan
/// is a function of the targets and (for files and titles) the live window
/// state, and execution is the only side effect in this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreAction {
    /// Open the URL, preferring the witnessed application.
    OpenUrl { url: String, app: Option<String> },
    /// Open the file with its owning application.
    OpenFile { path: String, app: Option<String> },
    /// Raise a window that already shows the resource.
    RaiseExisting { title: String },
    /// Nothing could be done, and why. Reported, never hidden.
    Unopenable { resource: String, reason: String },
}

/// The honest account of one restore.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreOutcome {
    pub opened: Vec<String>,
    pub raised: Vec<String>,
    pub unopened: Vec<(String, String)>,
}

/// A native command being accepted is not evidence that the target is focused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// The requested postcondition was directly observed after dispatch.
    Verified,
    /// The postcondition could not be witnessed, including unsupported adapters.
    Unverified { reason: String },
    /// Observed state mismatched, or a dispatched identity action lost its
    /// required lifetime/focus before verification could succeed.
    Failed { reason: String },
}

/// Dispatch evidence, kept separate from the observed postcondition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchResult {
    OpenAccepted,
    RaiseAccepted,
    NotDispatched { reason: String },
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetReceipt {
    pub resource: String,
    pub dispatch: DispatchResult,
    pub verification: VerificationResult,
}

/// The legacy buckets stay intact; `targets` records honest dispatch and
/// postcondition details for every target in input order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreReceipt {
    pub outcome: RestoreOutcome,
    pub targets: Vec<TargetReceipt>,
}

impl RestoreOutcome {
    /// Whether anything at all happened.
    pub fn any_action(&self) -> bool {
        !self.opened.is_empty() || !self.raised.is_empty()
    }

    /// The one-breath summary for the UI: what came back, what did not.
    pub fn summary(&self) -> String {
        match (self.opened.len(), self.raised.len(), self.unopened.len()) {
            (0, 0, 0) => "Nothing to restore.".to_string(),
            (opened, raised, 0) => {
                format!("Opened {opened}, raised {raised} already open.")
            }
            (opened, raised, unopened) => format!(
                "Opened {opened}, raised {raised} already open; {unopened} could not be opened."
            ),
        }
    }
}

/// Classifies one target into an action, given the currently live windows.
///
/// Pure with respect to the inputs: the same targets and window titles
/// always produce the same plan.
/// App-scoped titles require [`restore`]: this legacy plan format carries
/// neither live owners nor an application constraint on `RaiseExisting`.
pub fn plan(targets: &[RestoreTarget], live_window_titles: &[String]) -> Vec<RestoreAction> {
    targets
        .iter()
        .map(|target| {
            // The legacy action schema has nowhere to retain a surface. Do
            // not silently erase it when callers split planning/execution.
            if target.surface.is_some() {
                return RestoreAction::Unopenable {
                    resource: target.resource.clone(),
                    reason: "the legacy resource plan cannot retain exact surface identity; use restore with identity evidence".to_string(),
                };
            }
            match classify_subject(&target.resource) {
            LocatorKind::Url(url) => RestoreAction::OpenUrl {
                url,
                app: target.app.clone(),
            },
            LocatorKind::FilePath(path) => {
                // A live window already showing a document claims nothing
                // about its path through its title, so the raise-first
                // check for files is made against the document claim at
                // execution time (windows_showing_document), not against
                // titles. Titles are checked only for title-identified
                // targets below.
                RestoreAction::OpenFile {
                    path,
                    app: target.app.clone(),
                }
            }
            LocatorKind::WindowTitle(title) => {
                if target.app.is_some() {
                    RestoreAction::Unopenable {
                        resource: target.resource.clone(),
                        reason: "a title-only plan cannot retain the witnessed application; use restore with live owner evidence".to_string(),
                    }
                } else {
                    plan_title(&title, live_window_titles.iter().map(String::as_str))
                }
            }
            }
        })
        .collect()
}

fn plan_title<'a>(title: &str, titles: impl Iterator<Item = &'a str>) -> RestoreAction {
    let count = titles.filter(|live| *live == title).count();
    match count {
        1 if !title.trim().is_empty() => RestoreAction::RaiseExisting {
            title: title.to_string(),
        },
        _ => RestoreAction::Unopenable {
            resource: title.to_string(),
            reason: if count > 1 {
                format!("{count} live windows carry this title; cannot choose a unique window")
            } else {
                "no live window carries this title; a title cannot be opened".to_string()
            },
        },
    }
}

// Keep OS effects behind one seam so all restore decisions and failure paths
// can be exercised without a display or launching an application.
trait RestorePlatform {
    fn windows(&self) -> Result<Vec<WindowInfo>, String>;
    fn documents(&self, path: &str) -> Result<Vec<WindowInfo>, String>;
    fn raise(&self, window: &WindowInfo) -> Result<(), String>;
    fn open(&self, kind: &str, resource: &str, app: Option<&str>) -> Result<(), String>;
    fn raise_identity(&self, surface: &SurfaceIdentity, resource: &str) -> macos::IdentityAttempt {
        let _ = (surface, resource);
        macos::IdentityAttempt::unavailable("native identity adapter unavailable")
    }
    fn verify(
        &self,
        kind: &str,
        resource: &str,
        app: Option<&str>,
        window: Option<&WindowInfo>,
    ) -> VerificationResult {
        let _ = (kind, resource, app, window);
        VerificationResult::Unverified {
            reason: "postcondition inspection is unavailable on this platform".into(),
        }
    }
}

struct SystemPlatform;

impl RestorePlatform for SystemPlatform {
    fn windows(&self) -> Result<Vec<WindowInfo>, String> {
        macos::live_windows()
    }

    fn documents(&self, path: &str) -> Result<Vec<WindowInfo>, String> {
        macos::windows_showing(path)
    }

    fn raise(&self, window: &WindowInfo) -> Result<(), String> {
        macos::raise_window(window)
    }

    fn open(&self, kind: &str, resource: &str, app: Option<&str>) -> Result<(), String> {
        match app {
            Some(app) => open_with_app(app, resource),
            None => macos::open_target(kind, resource),
        }
    }

    fn raise_identity(&self, surface: &SurfaceIdentity, resource: &str) -> macos::IdentityAttempt {
        if surface.browser_profile.is_some()
            || surface.browser_session.is_some()
            || surface.browser_tab.is_some()
            || surface.browser_window.is_some()
        {
            let identity = match BrowserIdentity::from_surface(surface, resource) {
                Ok(identity) => identity,
                Err(reason) => return macos::IdentityAttempt::unavailable(reason),
            };
            let adapter = match MacOSBrowserAdapter::from_environment(surface.process_id) {
                Ok(adapter) => adapter,
                Err(reason) => return macos::IdentityAttempt::unavailable(reason),
            };
            let receipt = restore_browser_identity(&identity, &adapter);
            return macos::IdentityAttempt {
                dispatch: receipt.dispatch,
                verification: receipt.verification,
            };
        }
        macos::raise_surface_resource(surface, resource)
    }

    fn verify(
        &self,
        kind: &str,
        resource: &str,
        app: Option<&str>,
        window: Option<&WindowInfo>,
    ) -> VerificationResult {
        macos::verify_resource(kind, resource, app, window)
    }
}

fn windows_in_app(
    mut windows: Vec<WindowInfo>,
    app: Option<&str>,
) -> Result<Vec<WindowInfo>, String> {
    if let Some(app) = app {
        if app.trim().is_empty()
            || windows
                .iter()
                .any(|window| window.owner_name.trim().is_empty())
        {
            return Err(
                "cannot verify the witnessed application against unreadable window owners"
                    .to_string(),
            );
        }
        windows.retain(|window| window.owner_name == app);
    }
    Ok(windows)
}

/// Executes a plan. Each action is attempted independently — one refusal
/// never cancels the neighborhood — and every result is reported.
pub fn execute(actions: &[RestoreAction]) -> RestoreOutcome {
    execute_with_platform(actions, &SystemPlatform)
}

pub fn execute_with_receipt(actions: &[RestoreAction]) -> RestoreReceipt {
    execute_receipt_with_platform(actions, &SystemPlatform)
}

fn execute_with_platform(
    actions: &[RestoreAction],
    platform: &impl RestorePlatform,
) -> RestoreOutcome {
    execute_receipt_with_platform(actions, platform).outcome
}

fn execute_receipt_with_platform(
    actions: &[RestoreAction],
    platform: &impl RestorePlatform,
) -> RestoreReceipt {
    let mut receipt = RestoreReceipt::default();
    for action in actions {
        append_receipt(&mut receipt, execute_action(action, None, platform), false);
    }
    receipt
}

fn execute_action(
    action: &RestoreAction,
    title_app: Option<&str>,
    platform: &impl RestorePlatform,
) -> TargetReceipt {
    match action {
        RestoreAction::OpenUrl { url, app } => dispatch_receipt(
            url,
            false,
            platform.open("url", url, app.as_deref()),
            || platform.verify("url", url, app.as_deref(), None),
        ),
        RestoreAction::OpenFile { path, app } => {
            // Raise-first: a window already showing this exact document
            // is the restore — opening it again would duplicate the
            // person's context, the opposite of resuming.
            let candidates = match platform
                .documents(path)
                .and_then(|windows| windows_in_app(windows, app.as_deref()))
            {
                Ok(windows) => windows,
                Err(reason) => {
                    return unavailable_receipt(
                        path,
                        format!("cannot inspect document windows: {reason}"),
                    );
                }
            };
            match candidates.as_slice() {
                [] => dispatch_receipt(
                    path,
                    false,
                    platform.open("file", path, app.as_deref()),
                    || platform.verify("file", path, app.as_deref(), None),
                ),
                [window] => dispatch_receipt(path, true, platform.raise(window), || {
                    platform.verify("file", path, app.as_deref(), Some(window))
                }),
                _ => unavailable_receipt(
                    path,
                    format!(
                        "{} live windows show this exact document; cannot choose a unique window",
                        candidates.len()
                    ),
                ),
            }
        }
        RestoreAction::RaiseExisting { title } => {
            let windows = match platform
                .windows()
                .and_then(|windows| windows_in_app(windows, title_app))
            {
                Ok(windows) => windows,
                Err(reason) => {
                    return unavailable_receipt(
                        title,
                        format!("cannot inspect open windows: {reason}"),
                    );
                }
            };
            match select_unique_window(&windows, title) {
                Ok(window) => dispatch_receipt(title, true, platform.raise(window), || {
                    platform.verify("title", title, title_app, Some(window))
                }),
                Err(WindowSelectionError::NoMatch) => unavailable_receipt(
                    title,
                    "no matching window remains between planning and executing".to_string(),
                ),
                Err(WindowSelectionError::MultipleMatches { count }) => unavailable_receipt(
                    title,
                    format!("{count} live windows carry this title; cannot choose a unique window"),
                ),
            }
        }
        RestoreAction::Unopenable { resource, reason } => {
            unavailable_receipt(resource, reason.clone())
        }
    }
}

/// Restores a whole neighborhood in one call: plan, then execute.
pub fn restore(targets: &[RestoreTarget]) -> RestoreOutcome {
    restore_with_platform(targets, &SystemPlatform)
}

/// Restores targets and inspects the postcondition independently of dispatch.
/// Generic URLs have no native tab-verification adapter and remain unverified.
pub fn restore_with_receipt(targets: &[RestoreTarget]) -> RestoreReceipt {
    restore_receipt_with_platform(targets, &SystemPlatform)
}

fn restore_with_platform(
    targets: &[RestoreTarget],
    platform: &impl RestorePlatform,
) -> RestoreOutcome {
    restore_receipt_with_platform(targets, platform).outcome
}

fn restore_receipt_with_platform(
    targets: &[RestoreTarget],
    platform: &impl RestorePlatform,
) -> RestoreReceipt {
    let mut receipt = RestoreReceipt::default();
    for target in targets {
        if let Some(surface) = &target.surface {
            let attempt = platform.raise_identity(surface, &target.resource);
            append_receipt(
                &mut receipt,
                TargetReceipt {
                    resource: target.resource.clone(),
                    dispatch: attempt.dispatch,
                    verification: attempt.verification,
                },
                true,
            );
            continue;
        }
        let action = match classify_subject(&target.resource) {
            LocatorKind::WindowTitle(title) => match platform
                .windows()
                .and_then(|windows| windows_in_app(windows, target.app.as_deref()))
            {
                Ok(windows) => {
                    plan_title(&title, windows.iter().map(|window| window.title.as_str()))
                }
                Err(reason) => RestoreAction::Unopenable {
                    resource: target.resource.clone(),
                    reason: format!("cannot inspect open windows: {reason}"),
                },
            },
            _ => plan(std::slice::from_ref(target), &[]).remove(0),
        };
        append_receipt(
            &mut receipt,
            execute_action(&action, target.app.as_deref(), platform),
            false,
        );
    }
    receipt
}

fn unavailable_receipt(resource: &str, reason: String) -> TargetReceipt {
    TargetReceipt {
        resource: resource.to_string(),
        dispatch: DispatchResult::NotDispatched { reason },
        verification: VerificationResult::Unverified {
            reason: "no action was dispatched".into(),
        },
    }
}

fn dispatch_receipt(
    resource: &str,
    raised: bool,
    result: Result<(), String>,
    verify: impl FnOnce() -> VerificationResult,
) -> TargetReceipt {
    match result {
        Ok(()) => TargetReceipt {
            resource: resource.to_string(),
            dispatch: if raised {
                DispatchResult::RaiseAccepted
            } else {
                DispatchResult::OpenAccepted
            },
            verification: verify(),
        },
        Err(reason) => TargetReceipt {
            resource: resource.to_string(),
            dispatch: DispatchResult::Failed { reason },
            verification: VerificationResult::Unverified {
                reason: "dispatch was refused".into(),
            },
        },
    }
}

fn append_receipt(report: &mut RestoreReceipt, receipt: TargetReceipt, exact_surface: bool) {
    match &receipt.dispatch {
        DispatchResult::OpenAccepted => report.outcome.opened.push(receipt.resource.clone()),
        DispatchResult::RaiseAccepted
            if !exact_surface || receipt.verification == VerificationResult::Verified =>
        {
            report.outcome.raised.push(receipt.resource.clone());
        }
        DispatchResult::RaiseAccepted => {
            let reason = match &receipt.verification {
                VerificationResult::Failed { reason }
                | VerificationResult::Unverified { reason } => reason,
                VerificationResult::Verified => unreachable!(),
            };
            report.outcome.unopened.push((
                receipt.resource.clone(),
                format!("raise dispatch accepted but postcondition verification failed: {reason}"),
            ));
        }
        DispatchResult::NotDispatched { reason } | DispatchResult::Failed { reason } => {
            report
                .outcome
                .unopened
                .push((receipt.resource.clone(), reason.clone()));
        }
    }
    report.targets.push(receipt);
}

/// `open -a <app> <target>`: the operating system's own app-targeted
/// opener. The app name is witnessed fact from the caller; this module
/// never chooses an app on the person's behalf.
/// Asks the OS to open `target` in the witnessed application: plain
/// `open -a`, so an already-running app is simply brought forward —
/// never a second instance, never any per-app knowledge. The exit
/// status is checked like `launch_app`: a missing or refusing app is an
/// honest error, never a silent no-op.
fn open_with_app(app: &str, target: &str) -> Result<(), String> {
    let output = std::process::Command::new("open")
        .args(["-a", app, target])
        .output()
        .map_err(|err| format!("could not ask {app:?} to open {target:?}: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "the OS refused to open {target:?} in {app:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Launches (or activates) an application by its witnessed name: plain
/// `open -a` with no target, so an already-running app is simply
/// brought forward — never a second instance, never any per-app
/// knowledge. The name is witnessed fact from the caller; this module
/// never chooses an app on its own.
pub fn launch_app(app: &str) -> Result<(), String> {
    let output = std::process::Command::new("open")
        .args(["-a", app])
        .output()
        .map_err(|err| format!("could not ask the OS to open {app:?}: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "the OS refused to open {app:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[derive(Debug, PartialEq, Eq)]
    enum Effect {
        Raise(WindowInfo),
        Open(String, String, Option<String>),
        Identity(SurfaceIdentity),
    }

    struct FakePlatform {
        windows: RefCell<VecDeque<Result<Vec<WindowInfo>, String>>>,
        documents: Result<Vec<WindowInfo>, String>,
        raised: Result<(), String>,
        opened: Result<(), String>,
        identity: macos::IdentityAttempt,
        identity_adapter: Option<ResourceIdentityAdapter>,
        verification: VerificationResult,
        effects: RefCell<Vec<Effect>>,
    }

    impl Default for FakePlatform {
        fn default() -> Self {
            Self {
                windows: RefCell::new(VecDeque::from([Ok(vec![])])),
                documents: Ok(vec![]),
                raised: Ok(()),
                opened: Ok(()),
                identity: macos::IdentityAttempt::unavailable("test identity adapter unavailable"),
                identity_adapter: None,
                verification: VerificationResult::Unverified {
                    reason: "test postcondition unsupported".into(),
                },
                effects: RefCell::new(vec![]),
            }
        }
    }

    impl RestorePlatform for FakePlatform {
        fn windows(&self) -> Result<Vec<WindowInfo>, String> {
            let mut snapshots = self.windows.borrow_mut();
            if snapshots.len() > 1 {
                snapshots.pop_front().unwrap()
            } else {
                snapshots.front().expect("window snapshot").clone()
            }
        }

        fn documents(&self, _path: &str) -> Result<Vec<WindowInfo>, String> {
            self.documents.clone()
        }

        fn raise(&self, window: &WindowInfo) -> Result<(), String> {
            self.effects
                .borrow_mut()
                .push(Effect::Raise(window.clone()));
            self.raised.clone()
        }

        fn open(&self, kind: &str, resource: &str, app: Option<&str>) -> Result<(), String> {
            self.effects.borrow_mut().push(Effect::Open(
                kind.to_string(),
                resource.to_string(),
                app.map(str::to_string),
            ));
            self.opened.clone()
        }

        fn raise_identity(
            &self,
            surface: &SurfaceIdentity,
            resource: &str,
        ) -> macos::IdentityAttempt {
            self.effects
                .borrow_mut()
                .push(Effect::Identity(surface.clone()));
            match &self.identity_adapter {
                Some(adapter) => {
                    macos::raise_surface_resource_with_adapter(surface, resource, adapter)
                }
                None => self.identity.clone(),
            }
        }

        fn verify(
            &self,
            _kind: &str,
            _resource: &str,
            _app: Option<&str>,
            _window: Option<&WindowInfo>,
        ) -> VerificationResult {
            self.verification.clone()
        }
    }

    fn window(title: &str, pid: i32, app: &str) -> WindowInfo {
        WindowInfo {
            title: title.to_string(),
            owner_pid: pid,
            owner_name: app.to_string(),
        }
    }

    fn with_windows(windows: Vec<WindowInfo>) -> FakePlatform {
        FakePlatform {
            windows: RefCell::new(VecDeque::from([Ok(windows)])),
            ..FakePlatform::default()
        }
    }

    fn file(app: Option<&str>) -> RestoreAction {
        RestoreAction::OpenFile {
            path: "/work/report.md".to_string(),
            app: app.map(str::to_string),
        }
    }

    #[test]
    fn urls_plan_as_opens_keeping_the_witnessed_app() {
        let actions = plan(
            &[
                RestoreTarget {
                    resource: "https://claude.ai/chat/abc".into(),
                    app: Some("Google Chrome".into()),
                    surface: None,
                },
                RestoreTarget::with_default_handler("https://example.com/doc"),
            ],
            &[],
        );
        assert_eq!(
            actions[0],
            RestoreAction::OpenUrl {
                url: "https://claude.ai/chat/abc".into(),
                app: Some("Google Chrome".into())
            }
        );
        assert_eq!(
            actions[1],
            RestoreAction::OpenUrl {
                url: "https://example.com/doc".into(),
                app: None
            }
        );
    }

    #[test]
    fn file_paths_plan_as_opens() {
        let actions = plan(
            &[RestoreTarget::with_default_handler(
                "/Users/p/work/report.md",
            )],
            &[],
        );
        assert_eq!(
            actions[0],
            RestoreAction::OpenFile {
                path: "/Users/p/work/report.md".into(),
                app: None
            }
        );
    }

    #[test]
    fn a_live_window_title_plans_as_raise_not_reopen() {
        let actions = plan(
            &[RestoreTarget::with_default_handler("Evo — ZCode")],
            &["Evo — ZCode".to_string()],
        );
        assert_eq!(
            actions[0],
            RestoreAction::RaiseExisting {
                title: "Evo — ZCode".into()
            }
        );
    }

    #[test]
    fn a_dead_window_title_is_unopenable_and_says_so() {
        let actions = plan(
            &[RestoreTarget::with_default_handler("Terminal")],
            &["Something Else".to_string()],
        );
        match &actions[0] {
            RestoreAction::Unopenable { reason, .. } => {
                assert!(
                    reason.contains("cannot be opened"),
                    "honest reason: {reason}"
                );
            }
            other => panic!("expected unopenable, got {other:?}"),
        }
    }

    #[test]
    fn the_outcome_summary_names_every_bucket() {
        let outcome = RestoreOutcome {
            opened: vec!["a".into()],
            raised: vec!["b".into()],
            unopened: vec![("c".into(), "gone".into())],
        };
        let summary = outcome.summary();
        assert!(
            summary.contains("Opened 1")
                && summary.contains("raised 1")
                && summary.contains("1 could not")
        );
    }

    #[test]
    fn plan_has_no_artifact_cap_every_target_becomes_an_action() {
        // Seven artifacts (more than the old five-item UI cap): the
        // plan must carry every one — nothing is silently dropped.
        let targets: Vec<RestoreTarget> = (0..7)
            .map(|i| RestoreTarget::with_default_handler(&format!("/tmp/work-{i}.md")))
            .collect();
        let actions = plan(&targets, &[]);
        assert_eq!(actions.len(), 7, "every artifact plans an action");
        for (i, action) in actions.iter().enumerate() {
            assert_eq!(
                *action,
                RestoreAction::OpenFile {
                    path: format!("/tmp/work-{i}.md"),
                    app: None
                }
            );
        }
    }

    #[test]
    fn opening_in_a_missing_app_is_an_honest_error() {
        let platform = FakePlatform {
            opened: Err("the OS refused the witnessed app".into()),
            ..FakePlatform::default()
        };
        let outcome = execute_with_platform(&[file(Some("Missing"))], &platform);
        assert!(!outcome.any_action());
        assert!(outcome.unopened[0].1.contains("OS refused"));
    }

    #[test]
    fn duplicated_titles_are_refused_in_planning_and_execution() {
        let targets = [RestoreTarget::with_default_handler("Untitled")];
        let actions = plan(&targets, &["Untitled".into(), "Untitled".into()]);
        assert!(
            matches!(&actions[0], RestoreAction::Unopenable { reason, .. } if reason.contains("2 live windows"))
        );
        for second_pid in [10, 20] {
            let platform = with_windows(vec![
                window("Untitled", 10, "A"),
                window("Untitled", second_pid, "B"),
            ]);
            let outcome = execute_with_platform(
                &[RestoreAction::RaiseExisting {
                    title: "Untitled".into(),
                }],
                &platform,
            );
            assert!(!outcome.any_action());
            assert!(outcome.unopened[0].1.contains("2 live windows"));
            assert!(platform.effects.borrow().is_empty());
        }
    }

    #[test]
    fn duplicate_documents_never_raise_or_fall_back_to_open() {
        let platform = FakePlatform {
            documents: Ok(vec![window("A", 10, "Editor"), window("B", 10, "Editor")]),
            ..FakePlatform::default()
        };
        for app in [None, Some("Editor")] {
            let outcome = execute_with_platform(&[file(app)], &platform);
            assert!(!outcome.any_action());
            assert!(
                outcome.unopened[0]
                    .1
                    .contains("2 live windows show this exact document")
            );
        }
        assert!(platform.effects.borrow().is_empty());
    }

    #[test]
    fn document_candidates_are_scoped_to_exact_witnessed_app() {
        let expected = window("report", 20, "Editor");
        for documents in [
            vec![window("report", 10, "Other"), expected.clone()],
            vec![expected.clone(), window("report", 10, "Other")],
        ] {
            let platform = FakePlatform {
                documents: Ok(documents),
                ..FakePlatform::default()
            };
            let outcome = execute_with_platform(&[file(Some("Editor"))], &platform);
            assert_eq!(outcome.raised, ["/work/report.md"]);
            assert_eq!(
                *platform.effects.borrow(),
                [Effect::Raise(expected.clone())]
            );
        }
    }

    #[test]
    fn no_document_match_opens_exact_file_in_witnessed_app_or_default() {
        for app in [None, Some("Editor")] {
            let platform = FakePlatform::default();
            let outcome = execute_with_platform(&[file(app)], &platform);
            assert_eq!(outcome.opened, ["/work/report.md"]);
            assert_eq!(
                *platform.effects.borrow(),
                [Effect::Open(
                    "file".into(),
                    "/work/report.md".into(),
                    app.map(str::to_string)
                )]
            );
        }
        let platform = FakePlatform {
            documents: Ok(vec![window("report", 10, "editor")]),
            ..FakePlatform::default()
        };
        let outcome = execute_with_platform(&[file(Some("Editor"))], &platform);
        assert_eq!(outcome.opened, ["/work/report.md"]);
        assert_eq!(
            *platform.effects.borrow(),
            [Effect::Open(
                "file".into(),
                "/work/report.md".into(),
                Some("Editor".into())
            )]
        );
    }

    #[test]
    fn unique_unscoped_title_and_document_raise_without_reopening() {
        let expected = window("Report", 10, "Editor");
        let platform = FakePlatform {
            documents: Ok(vec![expected.clone()]),
            ..with_windows(vec![expected.clone()])
        };
        let outcome = execute_with_platform(
            &[
                file(None),
                RestoreAction::RaiseExisting {
                    title: "Report".into(),
                },
            ],
            &platform,
        );
        assert_eq!(outcome.raised, ["/work/report.md", "Report"]);
        assert_eq!(
            *platform.effects.borrow(),
            [Effect::Raise(expected.clone()), Effect::Raise(expected)]
        );
    }

    #[test]
    fn restore_retains_witnessed_app_for_titles_at_both_checks() {
        let target = RestoreTarget {
            resource: "Report".into(),
            app: Some("Editor".into()),
            surface: None,
        };
        // A caller splitting the legacy plan/execute API must not lose the app.
        assert!(
            matches!(&plan(std::slice::from_ref(&target), &["Report".into()])[0], RestoreAction::Unopenable { reason, .. } if reason.contains("witnessed application"))
        );
        let expected = window("Report", 20, "Editor");
        let platform = with_windows(vec![window("Report", 10, "Other"), expected.clone()]);
        let outcome = restore_with_platform(&[target], &platform);
        assert_eq!(outcome.raised, ["Report"]);
        assert_eq!(*platform.effects.borrow(), [Effect::Raise(expected)]);
    }

    #[test]
    fn scoped_title_does_not_match_wrong_owner_or_choose_between_owners_with_same_name() {
        for windows in [
            vec![window("Report", 10, "editor")],
            vec![
                window("Report", 10, "Editor"),
                window("Report", 20, "Editor"),
            ],
        ] {
            let platform = with_windows(windows);
            let outcome = restore_with_platform(
                &[RestoreTarget {
                    resource: "Report".into(),
                    app: Some("Editor".into()),
                    surface: None,
                }],
                &platform,
            );
            assert!(!outcome.any_action());
            assert_eq!(outcome.unopened.len(), 1);
            assert!(platform.effects.borrow().is_empty());
        }
    }

    #[test]
    fn unknown_owners_cannot_be_treated_as_a_different_witnessed_app() {
        let platform = FakePlatform {
            documents: Ok(vec![window("Report", 10, "")]),
            ..with_windows(vec![window("Report", 10, "")])
        };
        let outcome = restore_with_platform(
            &[
                RestoreTarget {
                    resource: "Report".into(),
                    app: Some("Editor".into()),
                    surface: None,
                },
                RestoreTarget {
                    resource: "/work/report.md".into(),
                    app: Some("Editor".into()),
                    surface: None,
                },
            ],
            &platform,
        );
        assert!(!outcome.any_action());
        assert_eq!(outcome.unopened.len(), 2);
        assert!(
            outcome
                .unopened
                .iter()
                .all(|(_, reason)| reason.contains("unreadable window owners"))
        );
        assert!(platform.effects.borrow().is_empty());
    }

    #[test]
    fn changed_live_state_is_rechecked_after_planning() {
        let original = window("Report", 10, "Editor");
        for (later, reason) in [
            (
                Ok(vec![original.clone(), original.clone()]),
                "2 live windows",
            ),
            (Ok(vec![]), "no matching window remains"),
            (
                Ok(vec![window("Report", 20, "Other")]),
                "no matching window remains",
            ),
            (
                Err("Accessibility permission lost".into()),
                "cannot inspect open windows",
            ),
        ] {
            let platform = FakePlatform {
                windows: RefCell::new(VecDeque::from([Ok(vec![original.clone()]), later])),
                ..FakePlatform::default()
            };
            let outcome = restore_with_platform(
                &[RestoreTarget {
                    resource: "Report".into(),
                    app: Some("Editor".into()),
                    surface: None,
                }],
                &platform,
            );
            assert!(!outcome.any_action());
            assert!(outcome.unopened[0].1.contains(reason), "{outcome:?}");
            assert!(platform.effects.borrow().is_empty());
        }
    }

    #[test]
    fn inspection_failure_is_not_an_empty_result_and_other_targets_continue() {
        let platform = FakePlatform {
            windows: RefCell::new(VecDeque::from([Err(
                "Accessibility permission missing".into()
            )])),
            documents: Err("document inspection failed".into()),
            ..FakePlatform::default()
        };
        let outcome = restore_with_platform(
            &[
                RestoreTarget::with_default_handler("Report"),
                RestoreTarget::with_default_handler("/work/report.md"),
                RestoreTarget::with_default_handler("https://example.com/exact"),
            ],
            &platform,
        );
        assert_eq!(outcome.opened, ["https://example.com/exact"]);
        assert!(outcome.raised.is_empty());
        assert_eq!(outcome.unopened.len(), 2);
        assert!(
            outcome.unopened[0]
                .1
                .contains("Accessibility permission missing")
        );
        assert!(outcome.unopened[1].1.contains("document inspection failed"));
        assert_eq!(
            *platform.effects.borrow(),
            [Effect::Open(
                "url".into(),
                "https://example.com/exact".into(),
                None
            )]
        );
    }

    #[test]
    fn native_late_refusal_does_not_reopen_and_other_targets_continue() {
        let platform = FakePlatform {
            documents: Ok(vec![window("Report", 10, "Editor")]),
            raised: Err("2 windows now carry the title; cannot raise a unique window".into()),
            ..FakePlatform::default()
        };
        let outcome = execute_with_platform(
            &[
                file(None),
                RestoreAction::OpenUrl {
                    url: "https://example.com/exact".into(),
                    app: None,
                },
            ],
            &platform,
        );
        assert_eq!(outcome.opened, ["https://example.com/exact"]);
        assert!(outcome.raised.is_empty());
        assert!(
            outcome.unopened[0]
                .1
                .contains("cannot raise a unique window")
        );
        assert_eq!(platform.effects.borrow().len(), 2);
    }

    fn surface() -> SurfaceIdentity {
        SurfaceIdentity {
            capture_session: "capture-fixture".into(),
            window_id: "window-1".into(),
            process_id: 10,
            process_started_at: format!("{:016x}", 1_791_000_000.5f64.to_bits()),
            window_identifier: Some("fixture-window".into()),
            browser_window: None,
            browser_profile: None,
            browser_session: None,
            browser_tab: None,
        }
    }

    #[test]
    fn identity_targets_never_plan_or_execute_a_wider_resource_fallback() {
        for resource in ["Report", "/work/report.md", "https://example.com/exact"] {
            let target = RestoreTarget {
                resource: resource.into(),
                app: Some("Editor".into()),
                surface: Some(surface()),
            };
            assert!(matches!(
                plan(std::slice::from_ref(&target), &[resource.into()])[0],
                RestoreAction::Unopenable { .. }
            ));
            let expected = window("Report", 10, "Editor");
            let platform = FakePlatform {
                documents: Ok(vec![expected.clone()]),
                ..with_windows(vec![expected])
            };
            let receipt = restore_receipt_with_platform(std::slice::from_ref(&target), &platform);
            assert!(!receipt.outcome.any_action());
            assert_eq!(receipt.outcome.unopened.len(), 1);
            assert_eq!(*platform.effects.borrow(), [Effect::Identity(surface())]);
            assert!(matches!(
                receipt.targets[0].dispatch,
                DispatchResult::NotDispatched { .. }
            ));
        }
    }

    #[test]
    fn identity_dispatch_accepted_is_not_raised_until_postcondition_is_verified() {
        for verification in [
            VerificationResult::Verified,
            VerificationResult::Failed {
                reason: "wrong focused identifier".into(),
            },
            VerificationResult::Unverified {
                reason: "focused attribute unavailable".into(),
            },
        ] {
            let platform = FakePlatform {
                identity: macos::IdentityAttempt {
                    dispatch: DispatchResult::RaiseAccepted,
                    verification: verification.clone(),
                },
                ..FakePlatform::default()
            };
            let target = RestoreTarget {
                resource: "https://example.com/exact".into(),
                app: None,
                surface: Some(surface()),
            };
            let receipt = restore_receipt_with_platform(&[target], &platform);
            assert_eq!(receipt.targets[0].dispatch, DispatchResult::RaiseAccepted);
            assert_eq!(receipt.targets[0].verification, verification);
            assert!(receipt.outcome.opened.is_empty());
            if verification == VerificationResult::Verified {
                assert_eq!(receipt.outcome.raised, ["https://example.com/exact"]);
            } else {
                assert!(receipt.outcome.raised.is_empty());
                assert!(
                    receipt.outcome.unopened[0]
                        .1
                        .contains("dispatch accepted but postcondition verification failed")
                );
            }
            assert_eq!(*platform.effects.borrow(), [Effect::Identity(surface())]);
        }
    }

    #[test]
    fn ordinary_receipts_distinguish_dispatch_from_postcondition_for_every_kind() {
        let expected = window("Report", 10, "Editor");
        let platform = with_windows(vec![expected]);
        let receipt = restore_receipt_with_platform(
            &[
                RestoreTarget::with_default_handler("https://example.com/exact"),
                RestoreTarget::with_default_handler("/work/report.md"),
                RestoreTarget::with_default_handler("Report"),
            ],
            &platform,
        );
        assert_eq!(
            receipt.outcome.opened,
            ["https://example.com/exact", "/work/report.md"]
        );
        assert_eq!(receipt.outcome.raised, ["Report"]);
        assert_eq!(receipt.targets.len(), 3);
        assert!(
            receipt
                .targets
                .iter()
                .all(|target| matches!(target.verification, VerificationResult::Unverified { .. }))
        );
        assert_eq!(receipt.targets[0].dispatch, DispatchResult::OpenAccepted);
        assert_eq!(receipt.targets[2].dispatch, DispatchResult::RaiseAccepted);
        assert!(matches!(
            macos::verify_resource("url", "https://example.com/exact", None, None),
            VerificationResult::Unverified { .. }
        ));
    }

    #[test]
    fn ordinary_verification_failure_is_preserved_in_receipt_without_changing_legacy_buckets() {
        let platform = FakePlatform {
            verification: VerificationResult::Failed {
                reason: "focused document differs".into(),
            },
            ..FakePlatform::default()
        };
        let receipt = restore_receipt_with_platform(
            &[RestoreTarget::with_default_handler("/work/report.md")],
            &platform,
        );
        assert_eq!(receipt.outcome.opened, ["/work/report.md"]);
        assert!(matches!(
            receipt.targets[0].verification,
            VerificationResult::Failed { .. }
        ));
    }

    struct ResourceIdentityAdapter {
        before: Option<String>,
        after: Option<String>,
        effects: RefCell<Vec<&'static str>>,
    }

    impl macos::IdentityAdapter for ResourceIdentityAdapter {
        type Window = ();
        fn process_started_at(&self, _pid: i32) -> Result<String, String> {
            Ok(surface().process_started_at)
        }
        fn windows(&self, _pid: i32) -> Result<Vec<()>, String> {
            Ok(vec![()])
        }
        fn identifier(&self, _window: &()) -> Result<Option<String>, String> {
            Ok(surface().window_identifier)
        }
        fn resource(&self, _window: &(), _kind: &LocatorKind) -> Result<Option<String>, String> {
            Ok(if self.effects.borrow().is_empty() {
                self.before.clone()
            } else {
                self.after.clone()
            })
        }
        fn focused_window(&self, _pid: i32) -> Result<(), String> {
            Ok(())
        }
        fn focused_window_identifier(&self, _pid: i32) -> Result<Option<String>, String> {
            Ok(surface().window_identifier)
        }
        fn raise(&self, _window: &()) -> Result<(), String> {
            self.effects.borrow_mut().push("raise");
            Ok(())
        }
        fn activate(&self, _pid: i32) -> Result<(), String> {
            self.effects.borrow_mut().push("activate");
            Ok(())
        }
    }

    #[test]
    fn constrained_restore_rejects_wrong_or_missing_resource_before_dispatch_without_fallback() {
        for before in [Some("/work/other.md".into()), None] {
            let platform = FakePlatform {
                identity_adapter: Some(ResourceIdentityAdapter {
                    before,
                    after: Some("/work/report.md".into()),
                    effects: RefCell::new(vec![]),
                }),
                ..FakePlatform::default()
            };
            let target = RestoreTarget {
                resource: "/work/report.md".into(),
                app: None,
                surface: Some(surface()),
            };
            let receipt = restore_receipt_with_platform(&[target], &platform);
            assert!(matches!(
                receipt.targets[0].dispatch,
                DispatchResult::NotDispatched { .. }
            ));
            assert!(!receipt.outcome.any_action());
            assert!(
                platform
                    .identity_adapter
                    .as_ref()
                    .unwrap()
                    .effects
                    .borrow()
                    .is_empty()
            );
            assert_eq!(*platform.effects.borrow(), [Effect::Identity(surface())]);
        }
    }

    #[test]
    fn constrained_restore_switched_document_is_dispatch_accepted_not_a_raised_success() {
        let platform = FakePlatform {
            identity_adapter: Some(ResourceIdentityAdapter {
                before: Some("/work/report.md".into()),
                after: Some("/work/other.md".into()),
                effects: RefCell::new(vec![]),
            }),
            ..FakePlatform::default()
        };
        let target = RestoreTarget {
            resource: "/work/report.md".into(),
            app: None,
            surface: Some(surface()),
        };
        let receipt = restore_receipt_with_platform(&[target], &platform);
        assert_eq!(receipt.targets[0].dispatch, DispatchResult::RaiseAccepted);
        assert!(matches!(
            receipt.targets[0].verification,
            VerificationResult::Failed { .. }
        ));
        assert!(receipt.outcome.opened.is_empty());
        assert!(receipt.outcome.raised.is_empty());
        assert!(
            receipt.outcome.unopened[0]
                .1
                .contains("dispatch accepted but postcondition verification failed")
        );
        assert_eq!(
            *platform.identity_adapter.as_ref().unwrap().effects.borrow(),
            ["raise", "activate"]
        );
        assert_eq!(*platform.effects.borrow(), [Effect::Identity(surface())]);
    }
}
