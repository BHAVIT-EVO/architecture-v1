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

use crate::locator::{LocatorKind, classify_subject};
use crate::macos;

/// One resource to bring back, with the application it was witnessed in
/// when the caller knows it (the app association is observed fact, never
/// inferred here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreTarget {
    pub resource: String,
    pub app: Option<String>,
}

impl RestoreTarget {
    /// A target with no witnessed application; the OS default handler owns it.
    pub fn with_default_handler(resource: &str) -> Self {
        Self {
            resource: resource.to_string(),
            app: None,
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
pub fn plan(targets: &[RestoreTarget], live_window_titles: &[String]) -> Vec<RestoreAction> {
    targets
        .iter()
        .map(|target| match classify_subject(&target.resource) {
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
                if live_window_titles.iter().any(|live| live == &title) {
                    RestoreAction::RaiseExisting { title }
                } else {
                    RestoreAction::Unopenable {
                        resource: target.resource.clone(),
                        reason: "no live window carries this title; a title cannot be opened"
                            .to_string(),
                    }
                }
            }
        })
        .collect()
}

/// Executes a plan. Each action is attempted independently — one refusal
/// never cancels the neighborhood — and every result is reported.
pub fn execute(actions: &[RestoreAction]) -> RestoreOutcome {
    let mut outcome = RestoreOutcome::default();
    for action in actions {
        match action {
            RestoreAction::OpenUrl { url, app } => {
                let result = match app.as_deref() {
                    Some(app) => open_with_app(app, url),
                    None => macos::open_target("url", url).map_err(|err| err.to_string()),
                };
                record(&mut outcome, url, result);
            }
            RestoreAction::OpenFile { path, app } => {
                // Raise-first: a window already showing this exact document
                // is the restore — opening it again would duplicate the
                // person's context, the opposite of resuming.
                let raised = macos::windows_showing(path)
                    .ok()
                    .and_then(|windows| windows.first().cloned());
                if let Some(window) = raised {
                    match macos::raise_window(&window) {
                        Ok(()) => outcome.raised.push(path.clone()),
                        Err(err) => outcome.unopened.push((path.clone(), err)),
                    }
                    continue;
                }
                let result = match app.as_deref() {
                    Some(app) => open_with_app(app, path),
                    None => macos::open_target("file", path).map_err(|err| err.to_string()),
                };
                record(&mut outcome, path, result);
            }
            RestoreAction::RaiseExisting { title } => {
                let window = macos::live_windows()
                    .ok()
                    .and_then(|windows| windows.into_iter().find(|window| window.title == *title));
                match window {
                    Some(window) => match macos::raise_window(&window) {
                        Ok(()) => outcome.raised.push(title.clone()),
                        Err(err) => outcome.unopened.push((title.clone(), err)),
                    },
                    None => outcome.unopened.push((
                        title.clone(),
                        "the window closed between planning and executing".to_string(),
                    )),
                }
            }
            RestoreAction::Unopenable { resource, reason } => {
                outcome.unopened.push((resource.clone(), reason.clone()));
            }
        }
    }
    outcome
}

/// Restores a whole neighborhood in one call: plan, then execute.
pub fn restore(targets: &[RestoreTarget]) -> RestoreOutcome {
    let live: Vec<String> = macos::live_windows()
        .map(|windows| windows.iter().map(|window| window.title.clone()).collect())
        .unwrap_or_default();
    execute(&plan(targets, &live))
}

fn record(outcome: &mut RestoreOutcome, resource: &str, result: Result<(), String>) {
    match result {
        Ok(()) => outcome.opened.push(resource.to_string()),
        Err(reason) => outcome.unopened.push((resource.to_string(), reason)),
    }
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

    #[test]
    fn urls_plan_as_opens_keeping_the_witnessed_app() {
        let actions = plan(
            &[
                RestoreTarget {
                    resource: "https://claude.ai/chat/abc".into(),
                    app: Some("Google Chrome".into()),
                },
                RestoreTarget::with_default_handler("https://example.com/doc".into()),
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
                "/Users/p/work/report.md".into(),
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
            &[RestoreTarget::with_default_handler("Evo — ZCode".into())],
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
            &[RestoreTarget::with_default_handler("Terminal".into())],
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
    fn opening_in_a_missing_app_is_an_honest_error_on_any_platform() {
        // `open -a` with a bogus app fails on macOS (the OS refuses);
        // on other platforms `open` itself is missing. Either way the
        // caller gets an Err, never a silent success.
        let result = open_with_app("Definitely Not A Real Application", "/tmp/x.md");
        assert!(result.is_err(), "a missing app must not report success");
    }
}
