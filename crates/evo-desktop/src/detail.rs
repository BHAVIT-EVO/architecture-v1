//! The Workspace detail screen: one body of work, in focus.
//!
//! The screen answers a single question — *where does this work continue, and
//! can Evo reopen it?* — and everything on it is arranged around that answer.
//!
//! The screen is exactly three levels, in descending order of what the user
//! came for:
//!
//! * **Identity** sits in the environment, unhoused. A body of work is not a
//!   card; it is the pod you are standing in.
//! * **The hero** — one raised surface — carries the answer and the action:
//!   the Resume Point, the Next Step, the plan, and `Continue`. The action
//!   lives beside what it affects instead of in a pinned bar at the bottom of
//!   the window, which is what previously produced a second visually dominant
//!   primary control and two competing animation states for one disclosure.
//! * **Used alongside** — one recessed surface — carries the small, read-only
//!   remainder of the current continuation: the few things that come back
//!   *with* the Resume Point, each with what preflight says about it. It is
//!   darker and lowered rather than a second white card, because it is context
//!   about the Workspace — never application navigation.
//! * **Remembered** — quiet material behind a single "+ N more remembered"
//!   line — carries everything else Evo holds: the way to reconsider what
//!   comes back, the way to relate other work, and the whole witnessed record.
//!   It is present without being offered, and never spread open by default.
//!
//! Membership is not importance and is not auto-open: a resource being in the
//! record does not put it in front of the user, and being surfaced does not
//! mean it is reopened. The three tiers encode exactly that ordering.
//!
//! Two structures divide what the screen may read from what it may write.
//! [`Canonical`] is Evo's own record and is borrowed immutably: the screen can
//! project it and can never author it. [`Transient`] is the user's in-flight
//! state — a draft, a status line, a report they have not dismissed — and is
//! the only thing the screen mutates. Nothing in [`Transient`] is canonical
//! until the daemon accepts a declaration.
//!
//! Designation and Continuation Surface stay distinct, as RFC-0013 Negative
//! Case 10 requires, but the distinction is carried by *form* rather than by
//! vocabulary: a round mark that supersedes (where this work continues) and a
//! square mark that accumulates into a draft (what comes back with it). The
//! user never has to read the words "designation" or "surface" to act
//! correctly.

use crate::state;
use crate::theme::{self, Role, Severity};
use crate::ui::{self, Control};

use evo_daemon::continuation::ContinuationResponse;
use evo_daemon::designation::DesignationResponse;
use evo_daemon::grouping::GroupingResponse;
use evo_engagement::Standing;
use evo_execution::{
    ExecutionReport, Locator, LocatorKind, PreflightOutcome, PreflightStatus, RestorationSelection,
    RestorationStep, StepPlan, TargetAttempt, TargetStatus, WithheldMember,
};
use evo_restoration::DerivationOutcome;
use evo_workspace::workspace::Workspace;
use evo_workspace::workspace_id::WorkspaceId;

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::{Instant, SystemTime};

/// The number of witnessed resources rendered in the full history list. A
/// bounded display window is presentation only: the record is complete, and
/// the count above the list always reports the true total.
pub const MAX_RENDERED_RESOURCES: usize = 50;

/// Where the Continue action is in its lifecycle.
///
/// `Idle` offers the primary control. `Preparing` is entered the instant the
/// user commits: the control is replaced by the restoring transition — which
/// *is* the lock against a second press, since there is no longer a button to
/// press — and it carries the moment it began so the shell can let the
/// transition paint before the restoration begins. `Executing` carries the
/// decided plan and the results that have actually come back so far, which is
/// what lets the screen show honest per-item progress: a row can only be marked
/// once `done` holds a real result for it.
///
/// The restoration itself is run by the shell (`app::EvoApp::logic`), one step
/// per frame, never inside the click handler. Structuring it this way is what
/// lets a future asynchronous or remote execution layer be a change in the
/// shell alone: the screen already treats Continue as "commit now, learn each
/// result as it arrives", which is exactly the shape an async boundary has.
#[derive(Clone, Default)]
pub enum ContinuePhase {
    /// At rest: the primary control is offered.
    #[default]
    Idle,
    /// The user has committed; the transition is showing and the shell will
    /// decide the plan once `since` has dwelled long enough to paint it.
    Preparing { since: Instant },
    /// The plan is decided and being acted on. `done` holds the results the
    /// executor has actually returned, in step order; every step beyond its
    /// length is still pending and is drawn as such.
    Executing {
        steps: Vec<RestorationStep>,
        done: Vec<TargetAttempt>,
    },
}

impl ContinuePhase {
    /// When the user committed, if a restoration is waiting to be decided.
    pub fn prepared_since(&self) -> Option<Instant> {
        match self {
            ContinuePhase::Preparing { since } => Some(*since),
            _ => None,
        }
    }

    /// Whether a decided plan is in flight.
    pub fn is_executing(&self) -> bool {
        matches!(self, ContinuePhase::Executing { .. })
    }

    /// Acts on exactly one planned step, and returns the finished report once
    /// every step has a real result.
    ///
    /// One step per call is what makes the progress the user sees honest: a row
    /// is marked because the executor returned a result for it, and the frame
    /// between calls is what paints it. The frontier is `done.len()`, so the
    /// same step can never be acted on twice — and because the phase is only
    /// left through this method, a second Continue cannot re-run a restoration
    /// that is already in flight. (The screen's own lock is stronger still: no
    /// Continue control exists outside `Idle`.)
    ///
    /// A phase that is not `Executing` performs nothing and reports nothing.
    pub fn advance<E>(&mut self, executor: &E) -> Option<ExecutionReport>
    where
        E: evo_execution::PlatformExecutor + evo_execution::PreflightSource,
    {
        let ContinuePhase::Executing { steps, done } = self else {
            return None;
        };
        if done.len() < steps.len() {
            let attempt = evo_execution::execute_step(&steps[done.len()], executor, executor);
            done.push(attempt);
            return None;
        }
        // Every step has a real result — or the plan was empty, in which case
        // the report says exactly that.
        let report = ExecutionReport::new(std::mem::take(done));
        *self = ContinuePhase::Idle;
        Some(report)
    }
}

/// Everything the detail screen may read. Borrowed immutably, because the
/// screen projects Evo's record and never authors it.
pub struct Canonical<'a> {
    /// The Workspace being viewed, identified canonically by its `WorkspaceId`.
    pub workspace: &'a Workspace,
    /// The replayed derivation for this Workspace, when one exists.
    pub outcome: Option<&'a DerivationOutcome>,
    /// What restoration derived from the current continuation.
    pub selection: Option<&'a RestorationSelection>,
    /// What the executor reports about the derived selection, right now.
    pub preflight: &'a [PreflightOutcome],
    /// Artifact identity → witnessed subject. Presentation only.
    pub subjects: &'a HashMap<String, String>,
    /// Artifact identity → witnessed kind. Presentation only.
    pub kinds: &'a HashMap<String, String>,
    /// Workspace identity → what that body of work is about. Presentation
    /// only; the name the work is called by, not a name for any resource in it.
    pub titles: &'a HashMap<String, String>,
    /// Workspace identity → Engagement-layer standing. The detail screen does
    /// not branch on it — opening a body of work by name is the same act
    /// whatever its standing — but it is threaded here so the canonical
    /// hand-off carries the whole derived record, not a convenient subset.
    pub standings: &'a HashMap<String, Standing>,
    /// Artifact identity → locator, for describing what `Continue` will try.
    pub locators: &'a HashMap<String, Locator>,
    /// The standing designation and when it was recorded, if there is one.
    pub designation: Option<&'a (String, SystemTime)>,
}

/// Everything the detail screen may write: the user's own in-flight state.
///
/// A canonical reload may replace derived state; it may never touch anything
/// in here. A draft the user is still editing, a status line they have not
/// read, and an execution report they have not dismissed all outlive a reload.
pub struct Transient<'a> {
    /// Where declarations are submitted.
    pub storage_root: &'a Path,
    /// Which Workspace the shell is showing, by canonical identity.
    pub selected: &'a mut Option<WorkspaceId>,
    /// The daemon's answer to the last designation, and whether it accepted.
    pub designation_status: &'a mut Option<(String, bool)>,
    /// Marks the user has changed but not yet declared. `None` means the
    /// marks shown are Evo's, not the user's.
    pub continuation_draft: &'a mut Option<BTreeSet<String>>,
    /// The daemon's answer to the last declaration, and whether it accepted.
    pub continuation_status: &'a mut Option<(String, bool)>,
    /// The daemon's answer to the last related-work declaration (RFC-0012
    /// WorkGrouped), and whether it accepted.
    pub grouping_status: &'a mut Option<(String, bool)>,
    /// The last execution's report. It stays until the user dismisses it.
    pub execution_result: &'a mut Option<ExecutionReport>,
    /// Where the Continue action is in its lifecycle. Writing `Preparing` here
    /// is how the click handler commits; the shell runs the restoration and
    /// returns it to `Idle`.
    pub continue_phase: &'a mut ContinuePhase,
}

/// Renders the detail screen for one body of work.
///
/// The screen is exactly three levels, in descending order of what the user
/// came for: the raised **hero** (where you left off, and Continue); the small
/// recessed **used alongside** (what comes back with it); and the quiet,
/// collapsed **remembered** (everything else Evo holds for this work, reachable
/// but never in the way). `siblings` are the other bodies of work, so the user
/// can step from this Place into another without returning Home first.
pub fn screen(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    siblings: &[state::WorkspaceCard],
    tr: &mut Transient<'_>,
) {
    ui::scroll(ui, |ui| {
        place_nav(ui, canon, siblings, tr);
        ui::gap(ui, theme::S5);

        identity(ui, canon);
        ui::gap(ui, theme::S6);

        // The candidates for a continuation origin, paired with their subjects
        // in one pass. Pairing them separately is what previously let the
        // label shown and the subject written drift apart when a witnessed
        // resource had no recorded subject.
        let candidates: Vec<(String, String)> =
            state::distinct_artifacts_in_latest_snapshot(canon.workspace)
                .into_iter()
                .filter_map(|artifact_id| {
                    let subject = state::subject_for(canon.subjects, &artifact_id)?.to_string();
                    Some((artifact_id, subject))
                })
                .collect();

        let total = ui.available_width();
        if total < theme::STACK_BREAKPOINT {
            continuation(ui, canon, tr, &candidates);
            ui::gap(ui, theme::S6);
            alongside(ui, canon);
        } else {
            let (main, gap, side) = theme::detail_split(total);
            // Each column needs its layout stated. `allocate_ui` inherits the
            // parent's, and the parent here is left-to-right, so plain
            // `allocate_ui` laid both columns' contents out sideways: every
            // block landed on one baseline and prose wrapped a character per
            // line.
            let column = egui::Layout::top_down(egui::Align::Min);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(egui::vec2(main, ui.available_height()), column, |ui| {
                    continuation(ui, canon, tr, &candidates);
                });
                ui.add_space(gap);
                ui.allocate_ui_with_layout(egui::vec2(side, ui.available_height()), column, |ui| {
                    alongside(ui, canon);
                });
            });
        }

        ui::gap(ui, theme::S7);
        remembered(ui, canon, tr);
        ui::gap(ui, theme::S6);
    });
}

// ────────────────────────────────────────────────────────────────────────
// Navigation between Places
// ────────────────────────────────────────────────────────────────────────

/// The way out of this Place, and the way across to another.
///
/// "All work" returns Home. The switcher steps sideways into another body of
/// work without going Home first — the Place-to-Place move the mental model
/// wants ("…Automation X → Automation Y"). It reuses the one navigation path
/// there is: setting the selected Workspace identity. Nothing about the target
/// Place is derived here; opening it derives its own state on the next frame,
/// exactly as a Home selection does.
fn place_nav(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    siblings: &[state::WorkspaceCard],
    tr: &mut Transient<'_>,
) {
    if ui::control(ui, Control::quiet("← All work")).clicked() {
        *tr.selected = None;
    }

    let current = canon.workspace.id();
    // Only work is offered as a sideways step. Remembered bodies stay findable
    // by name from Home; they are not thrown into the switcher (Home's own
    // work/remembered split is the authority on that).
    let others: Vec<&state::WorkspaceCard> = siblings
        .iter()
        .filter(|card| card.standing.is_work() && &card.id != current)
        .collect();
    if others.is_empty() {
        return;
    }

    ui::gap(ui, theme::S2);
    let open = ui::disclosure(
        ui,
        ui.id().with("evo-switch-work"),
        &format!("Switch to other work ({})", others.len()),
    );
    if !open {
        return;
    }
    ui::gap(ui, theme::S2);
    // A bounded set of doorways, not a directory. The rest stay one step away
    // through "All work" — the switcher is for stepping between the few things
    // you are actually moving among, never a second Home.
    const SWITCH_CAP: usize = 6;
    for card in others.iter().take(SWITCH_CAP) {
        let id = ui.id().with(("evo-switch", card.id.to_string()));
        let (row, _) = ui::row(ui, id, |ui| {
            ui.horizontal_top(|ui| {
                ui::one_line(ui, Role::Body, &card.title, theme::TEXT);
            });
        });
        if row.clicked() {
            *tr.selected = Some(card.id.clone());
        }
    }
    if others.len() > SWITCH_CAP {
        ui::gap(ui, theme::S2);
        ui::caption(
            ui,
            &format!(
                "{} more — open “All work” to find them.",
                others.len() - SWITCH_CAP
            ),
        );
    }
}

// ────────────────────────────────────────────────────────────────────────
// Identity
// ────────────────────────────────────────────────────────────────────────

/// The body of work's identity, in the environment rather than on a surface.
///
/// Deliberately absent: the Workspace identifier, the snapshot count and the
/// lifecycle word in normal operation. Those describe Evo's internals, not the
/// user's work. A lifecycle *other* than active does change what the record
/// means, so that one is stated.
fn identity(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    let card = state::workspace_card(
        canon.workspace,
        canon.outcome,
        canon.selection,
        canon.subjects,
        canon.kinds,
        canon.titles,
        canon.standings,
    );

    // The doorway: the workspace's derived name IS the Place identity.
    // "Body of work" is Evo's internal classification; the user sees
    // the name of their work as the pod they are standing in.
    ui::eyebrow(ui, &card.title);
    ui::label(ui, Role::Headline, "Continue this work", theme::TEXT);
    ui::gap(ui, theme::S3);

    if state::distinct_artifacts_across_history(canon.workspace).is_empty() {
        ui::unknown(ui, "Evo has not witnessed anything in this work yet.");
    } else {
        ui::lede(ui, &card.description);
    }

    let lifecycle = state::lifecycle_label(canon.workspace.lifecycle());
    if lifecycle != "Active" {
        ui::notice(
            ui,
            Severity::Caution,
            lifecycle,
            "Evo's record marks this grouping as superseded. Nothing has been \
             removed — it remains exactly as it was witnessed.",
        );
    }
}

// ────────────────────────────────────────────────────────────────────────
// The answer, and the action
// ────────────────────────────────────────────────────────────────────────

/// The one raised surface on the screen: where this work continues, and the
/// control that returns to it.
fn continuation(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    candidates: &[(String, String)],
) {
    ui::raised(ui, |ui| {
        ui.set_width(ui.available_width());
        ui::eyebrow(ui, "Continue from");

        designation_elsewhere(ui, canon, candidates);

        match canon.outcome.and_then(|outcome| outcome.resume_point()) {
            Some(resume) => {
                resume_answer(ui, canon, resume.artifact_id().as_str());
                if !candidates.is_empty() {
                    ui::gap(ui, theme::S4);
                    let open = ui::disclosure(
                        ui,
                        ui.id().with("evo-change-origin"),
                        "Change where this work continues",
                    );
                    if open {
                        origin_list(ui, canon, tr, candidates);
                    } else {
                        designation_answer(ui, tr);
                    }
                }
            }
            None => unresolved(ui, canon, tr, candidates),
        }

        next_step(ui, canon);
        blockers(ui, canon);
        action(ui, canon, tr);
        report(ui, canon, tr);
    });
}

/// A designation that points outside this body of work. Stated where the
/// answer would otherwise be, because it explains why the answer is missing.
fn designation_elsewhere(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    candidates: &[(String, String)],
) {
    let Some((subject, recorded)) = canon.designation else {
        return;
    };
    if candidates.iter().any(|(_, known)| known == subject) {
        return;
    }
    ui::notice(
        ui,
        Severity::Caution,
        "Marked elsewhere",
        &format!(
            "You marked “{subject}” (at {}) as where you continue, but Evo's \
             record does not place it in this body of work.",
            state::local_time_label(*recorded)
        ),
    );
    ui::gap(ui, theme::S3);
}

/// The Resume Point: the largest, quietest thing on the screen.
fn resume_answer(ui: &mut egui::Ui, canon: &Canonical<'_>, artifact_id: &str) {
    let subject = state::subject_for(canon.subjects, artifact_id);
    match subject {
        Some(subject) => {
            ui::label(ui, Role::Display, subject, theme::TEXT);
            ui::gap(ui, theme::S2);
        }
        None => ui::unknown(ui, "Evo's record does not name this resource."),
    }

    if let Some(kind) = state::kind_for(canon.kinds, artifact_id) {
        ui::caption(ui, &sentence_case(kind));
    }

    // Why *this* one. Under IS-0021 §25.2 there are exactly two grounds: the
    // user marked it, or the record holds a single resource. Neither is
    // recency, and the sentence says so.
    let marked = canon
        .designation
        .is_some_and(|(designated, _)| Some(designated.as_str()) == subject);
    if let (true, Some((_, recorded))) = (marked, canon.designation) {
        ui::provenance(
            ui,
            &format!(
                "Because you marked it, at {}. Evo does not guess and does not rank.",
                state::local_time_label(*recorded)
            ),
        );
    } else {
        ui::provenance(
            ui,
            "Because Evo's record holds exactly one resource for this work. \
             Nothing was ranked or guessed.",
        );
    }
}

/// What Evo has derived as the step this work continues into.
fn next_step(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    let Some(next) = canon.outcome.and_then(|outcome| outcome.next_step()) else {
        return;
    };
    ui::gap(ui, theme::S5);
    ui::eyebrow(ui, "Next step");
    ui::label(ui, Role::BodyStrong, next.description(), theme::TEXT);
    ui::gap(ui, theme::S1);
    ui::provenance(
        ui,
        "Derived from what Evo witnessed. It is a note to you, not an action Evo will take.",
    );
}

/// Derived blockers.
///
/// These sit above the action rather than at the bottom of the screen: a
/// blocker changes what pressing `Continue` means, so it has to be readable
/// before the user reaches for it. They are set quietly — the heading carries
/// the meaning, so no per-line badge is needed.
fn blockers(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    let blockers = canon
        .outcome
        .map(|outcome| outcome.blockers())
        .unwrap_or(&[]);
    if blockers.is_empty() {
        return;
    }
    ui::gap(ui, theme::S5);
    ui::eyebrow(ui, "In the way");
    for blocker in blockers {
        ui::label(
            ui,
            Role::Body,
            &format!("— {}", blocker.description()),
            theme::TEXT,
        );
        ui::gap(ui, theme::S1);
    }
}

/// Whether the primary Continue act is ready.
///
/// Continue is enabled exactly when the shared ordering function — the same
/// one `state::run_execution` consumes — announces at least one target that
/// will actually be attempted (UI-VALIDATION-0001 B.9: the promise and the
/// act derive from one shared function). A declared Continuation Surface
/// (RFC-0013) is the restore candidate set even when derivation is
/// Insufficient with no Resume Point (IS-0021 §25.2), so the gate follows
/// the plan, never the Resume Point alone.
fn continue_ready(canon: &Canonical<'_>) -> bool {
    plan_line(canon).is_some()
}

/// The climax of the screen: one primary control, and an honest sentence about
/// exactly what it will attempt.
fn action(ui: &mut egui::Ui, canon: &Canonical<'_>, tr: &mut Transient<'_>) {
    // Committed: the transition replaces the control entirely. With no button
    // present, a second Continue is structurally impossible while one is being
    // prepared or acted on — the lock is the absence of the affordance, not a
    // disabled flag that a fast double-click could still race.
    match &*tr.continue_phase {
        ContinuePhase::Preparing { since } => {
            preparing(ui, canon, *since);
            return;
        }
        ContinuePhase::Executing { steps, done } => {
            executing(ui, canon, steps, done);
            return;
        }
        ContinuePhase::Idle => {}
    }

    ui::gap(ui, theme::S5);
    ui::hairline(ui);
    ui::gap(ui, theme::S4);

    let plan = plan_line(canon);
    // The button's enabled state and the tests both read the same named gate,
    // so "is Continue ready" has exactly one definition.
    let ready = continue_ready(canon);
    match &plan {
        Some(plan) => ui::caption(ui, &format!("Evo will attempt: {plan}")),
        None => {
            // Nothing will be attempted. Say why honestly: a declaration or
            // a derived Resume Point exists but none of it resolves to an
            // openable target, versus no continuation evidence at all, where
            // the user's own mark is the next act.
            let declared_or_resumed = canon.outcome.is_some_and(|outcome| {
                !outcome.continuation_surface().is_empty() || outcome.resume_point().is_some()
            });
            if declared_or_resumed {
                ui::caption(
                    ui,
                    "Evo's record holds no resource it can open for this work.",
                );
            } else {
                ui::caption(
                    ui,
                    "Mark where this work continues, and Evo will be able to reopen it.",
                );
            }
        }
    }
    ui::gap(ui, theme::S2);

    let pressed = ui::control(ui, Control::primary("Continue this work").enabled(ready)).clicked();
    if pressed {
        // Commit only. The last report is cleared so the transition begins
        // clean, and the shell performs the (blocking) restoration after a
        // dwell so this frame can paint the transition first. The click never
        // runs execution itself — that seam is exactly what a real async or
        // remote-binding layer would replace, and nothing else would move.
        *tr.execution_result = None;
        *tr.continue_phase = ContinuePhase::Preparing {
            since: Instant::now(),
        };
    }
}

/// The restoring transition: shown from the instant Continue is pressed until
/// the shell has decided the plan.
///
/// It states what Evo is *about to* attempt — the same ordered plan the button
/// announced — and never what has happened. Every target is pending here,
/// because no step has run yet. This screen never draws a mark it has not been
/// handed.
fn preparing(ui: &mut egui::Ui, canon: &Canonical<'_>, since: Instant) {
    let _ = since;
    ui::gap(ui, theme::S5);
    ui::hairline(ui);
    ui::gap(ui, theme::S4);
    ui::eyebrow(ui, "Preparing your workspace");

    for (index, (artifact_id, action)) in plan_targets(canon).iter().enumerate() {
        pending_line(ui, canon, index, artifact_id, action);
    }

    ui::provenance(
        ui,
        "Evo is reopening what you left off in. Native apps stay native — Evo \
         reveals them, it does not contain them.",
    );
}

/// The restoration underway: the decided plan, with each row carrying its real
/// result the moment the executor returns one and nothing before.
///
/// Rows above the frontier are results (`attempt_line`); the rest are still
/// pending (`pending_line`). Nothing here is a prediction: a checkmark appears
/// only because `done` holds an `Opened` status for that step, and a failure
/// appears with the executor's own reason. One failed row never fails the
/// others — the remaining steps still run and still report for themselves.
fn executing(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    steps: &[RestorationStep],
    done: &[TargetAttempt],
) {
    ui::gap(ui, theme::S5);
    ui::hairline(ui);
    ui::gap(ui, theme::S4);
    ui::eyebrow(ui, "Restoring your work");

    for (index, step) in steps.iter().enumerate() {
        match done.get(index) {
            Some(attempt) => attempt_line(ui, canon, index, attempt),
            None => pending_line(
                ui,
                canon,
                index,
                step.artifact_id().as_str(),
                &step_action(step),
            ),
        }
    }

    ui::provenance(
        ui,
        "Evo is reopening what you left off in. Native apps stay native — Evo \
         reveals them, it does not contain them.",
    );
}

/// One planned step that has no result yet: a pending mark, what it concerns,
/// and what Evo will try.
///
/// A gentle pulse on the mark, so the row reads as work underway rather than a
/// frozen list; it is driven by the shell's per-frame repaint request while a
/// restoration is in flight. Each row settles in a little after the last, so
/// the plan reads as a sequence being made ready rather than a block that
/// appeared at once.
fn pending_line(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    index: usize,
    artifact_id: &str,
    action: &str,
) {
    let label = state::subject_for(canon.subjects, artifact_id)
        .map(|_| state::describe_artifact(canon.kinds, canon.subjects, artifact_id))
        .unwrap_or_else(|| sentence_case(action));
    let reveal = ui.ctx().animate_bool_with_time(
        ui.id().with(("evo-prepare", index)),
        true,
        theme::RESPONSE_IN + 0.05 * index as f32,
    );
    let time = ui.input(|i| i.time) as f32;
    let pulse = 0.45 + 0.55 * (0.5 + 0.5 * (time * 3.2 + index as f32 * 0.7).sin());
    ui.scope(|ui| {
        ui.set_opacity(theme::settle(reveal));
        ui.horizontal_top(|ui| {
            ui.scope(|ui| {
                ui.set_opacity(pulse);
                ui::label(ui, Role::Body, "○", theme::MUTED);
            });
            ui.add_space(theme::S3);
            ui.vertical(|ui| {
                ui::one_line(ui, Role::Body, &label, theme::TEXT);
                ui::gap(ui, theme::S1);
                ui::label(ui, Role::Caption, action, theme::MUTED);
            });
        });
    });
    ui::gap(ui, theme::S3);
}

/// The ordered locator plan, in the executor's own order.
///
/// Describes exactly the targets `state::run_execution` will attempt, from
/// the same shared ordering function: a declared Continuation Surface is
/// announced as its restore-worthy members in canonical order, otherwise the
/// derived Resume Point first and the Context Chain after (BE-AUDIT-0001
/// §2.4). A refusal is not a plan, so there is no plan line to state; the
/// caller already says so honestly rather than guessing.
fn plan_line(canon: &Canonical<'_>) -> Option<String> {
    let targets = plan_targets(canon);
    if targets.is_empty() {
        None
    } else {
        Some(
            targets
                .into_iter()
                .map(|(_, action)| action)
                .collect::<Vec<_>>()
                .join(" · "),
        )
    }
}

/// The ordered targets Continue will attempt, each paired with the sentence
/// describing its action, in the executor's own order.
///
/// Empty when nothing resolves to an openable target. This is the single
/// source both the announced plan line and the restoring transition read from,
/// so the promise, the transition, and the act cannot drift apart
/// (UI-VALIDATION-0001 B.9): all three are this one function's output.
fn plan_targets(canon: &Canonical<'_>) -> Vec<(String, String)> {
    let Some(outcome) = canon.outcome else {
        return Vec::new();
    };
    let Ok(targets) = evo_execution::ordered_attempt_targets(Some(outcome), canon.selection) else {
        return Vec::new();
    };
    targets
        .iter()
        .filter_map(|artifact_id| {
            let locator = canon.locators.get(artifact_id.as_str())?;
            Some((
                artifact_id.as_str().to_string(),
                locator_action(locator.kind()),
            ))
        })
        .collect()
}

/// What Evo will try for a canonical executable target, in one place, so the
/// announcement, the transition, and the restoration in flight cannot describe
/// the same act in different words.
fn locator_action(kind: &LocatorKind) -> String {
    match kind {
        LocatorKind::WindowTitle(title) => format!("focus window “{title}”"),
        LocatorKind::FilePath(path) => format!("open file {path}"),
        LocatorKind::Url(url) => format!("open URL {url}"),
    }
}

/// What a decided step will try — the same words the announcement used.
///
/// A refused step will try nothing, and says so. Its executor-given reason
/// arrives as its result, not as a promise here.
fn step_action(step: &RestorationStep) -> String {
    match step.plan() {
        StepPlan::Attempt(kind) => locator_action(kind),
        StepPlan::Refuse(_) => "not attempted".to_string(),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Designation
// ────────────────────────────────────────────────────────────────────────

/// The screen when Evo has no Resume Point: honest about what it lacks, and
/// immediately actionable.
fn unresolved(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    candidates: &[(String, String)],
) {
    if candidates.is_empty() {
        ui::headline(ui, "Not enough witnessed yet");
        ui::prose(
            ui,
            "Evo remembers this body of work, but its record does not yet hold \
             a resource it could reopen. Keep working — Evo will record what it \
             witnesses.",
        );
        return;
    }

    ui::headline(ui, "You haven't marked where this continues");
    ui::prose(
        ui,
        "Evo witnessed several resources here and will not choose between them \
         for you. Mark the one your work continues from.",
    );
    origin_list(ui, canon, tr, candidates);
}

/// The candidates for a continuation origin.
///
/// The mark is round and exclusive: choosing one supersedes the last, which is
/// how RFC-0011 defines a designation. Nothing here is a draft — a designation
/// is recorded the moment it is made, and the reply says so.
fn origin_list(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    candidates: &[(String, String)],
) {
    ui::gap(ui, theme::S4);
    for (artifact_id, subject) in candidates {
        let chosen = canon
            .designation
            .is_some_and(|(designated, _)| designated == subject);
        let label = state::describe_artifact(canon.kinds, canon.subjects, artifact_id);
        let id = ui.id().with(("evo-origin", artifact_id.as_str()));
        let (row, mark) = ui::row(ui, id, |ui| {
            ui.horizontal_top(|ui| {
                let mark = ui::origin_mark(ui, id.with("mark"), chosen);
                ui.add_space(theme::S3);
                ui::one_line(
                    ui,
                    Role::Body,
                    &label,
                    if chosen { theme::TEXT } else { theme::MUTED },
                );
                mark
            })
            .inner
        });
        // `||` short-circuits, so exactly one record is submitted no matter
        // which of the two widgets egui awarded the click to.
        if (row.clicked() || mark.clicked()) && !chosen {
            *tr.designation_status = Some(record_designation(tr.storage_root, subject));
        }
    }
    designation_answer(ui, tr);
}

/// The daemon's reply to the last designation.
fn designation_answer(ui: &mut egui::Ui, tr: &mut Transient<'_>) {
    let Some((message, accepted)) = tr.designation_status.as_ref() else {
        return;
    };
    ui::gap(ui, theme::S3);
    let (severity, word) = if *accepted {
        (Severity::Good, "Recorded")
    } else {
        (Severity::Bad, "Not recorded")
    };
    ui::notice(ui, severity, word, message);
}

/// Submits a designation and reports, verbatim, what the daemon said.
fn record_designation(root: &Path, subject: &str) -> (String, bool) {
    match state::submit_designation(root, subject) {
        Ok(DesignationResponse::Accepted) => {
            ("Evo will continue from this resource.".to_string(), true)
        }
        Ok(DesignationResponse::Rejected(reason)) => {
            (format!("Evo could not record that: {reason}"), false)
        }
        Err(error) => (
            format!("Evo could not reach the capture worker: {error}"),
            false,
        ),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Execution
// ────────────────────────────────────────────────────────────────────────

/// The execution report.
///
/// It is held in [`Transient`] and cleared only by the control below it. A
/// successful restoration produces new observations, which change the derived
/// state; tying this report's visibility to that state is what previously made
/// a success erase its own result.
fn report(ui: &mut egui::Ui, canon: &Canonical<'_>, tr: &mut Transient<'_>) {
    let mut dismissed = false;
    if let Some(report) = tr.execution_result.as_ref() {
        ui::gap(ui, theme::S5);
        ui::hairline(ui);
        ui::gap(ui, theme::S4);
        ui::eyebrow(ui, "What Evo did");

        if report.attempts().is_empty() {
            ui::unknown(
                ui,
                "Nothing was executed: the record referenced no resource.",
            );
        } else {
            ui::label(
                ui,
                Role::BodyStrong,
                &state::execution_result_line(report),
                theme::TEXT,
            );
            ui::gap(ui, theme::S3);
            for (index, attempt) in report.attempts().iter().enumerate() {
                attempt_line(ui, canon, index, attempt);
            }
        }

        ui::gap(ui, theme::S2);
        dismissed = ui::control(ui, Control::quiet("Dismiss")).clicked();
    }
    if dismissed {
        *tr.execution_result = None;
    }
}

/// One attempt, with its status word, what it concerned, and the executor's
/// own reason where it gave one.
///
/// The lines materialise in order, each a little later than the last, so the
/// report reads as a sequence of things that happened rather than a block that
/// appeared. The stagger is carried by the transition's response, so an
/// interrupted or re-run execution never has to wait for it to finish.
fn attempt_line(ui: &mut egui::Ui, canon: &Canonical<'_>, index: usize, attempt: &TargetAttempt) {
    let artifact_id = attempt.artifact_id().as_str();
    let (word, severity) = match attempt.status() {
        TargetStatus::Opened { .. } => ("Opened", Severity::Good),
        TargetStatus::Failed { .. } => ("Failed", Severity::Bad),
        TargetStatus::Unavailable { .. } => ("Unavailable", Severity::Caution),
        TargetStatus::Ambiguous { .. } => ("Ambiguous", Severity::Caution),
        TargetStatus::Unsupported { .. } => ("Unsupported", Severity::Neutral),
    };
    let reason = match attempt.status() {
        TargetStatus::Opened { detail } => detail.as_str(),
        TargetStatus::Failed { reason } => reason.as_str(),
        TargetStatus::Unavailable { reason } => reason.as_str(),
        TargetStatus::Ambiguous { reason } => reason.as_str(),
        TargetStatus::Unsupported { reason } => reason.as_str(),
    };
    let label = state::subject_for(canon.subjects, artifact_id)
        .map(|_| state::describe_artifact(canon.kinds, canon.subjects, artifact_id));

    let reveal = ui.ctx().animate_bool_with_time(
        ui.id().with(("evo-attempt", index)),
        true,
        theme::RESPONSE_IN + 0.05 * index as f32,
    );
    ui.scope(|ui| {
        ui.set_opacity(theme::settle(reveal));
        ui.horizontal_top(|ui| {
            ui::status_word(ui, word, severity);
            ui.add_space(theme::S3);
            ui.vertical(|ui| {
                match label {
                    Some(label) => {
                        ui::one_line(ui, Role::Body, &label, theme::TEXT);
                    }
                    None => {
                        ui::one_line(
                            ui,
                            Role::Body,
                            "A resource Evo's record does not name",
                            theme::MUTED,
                        );
                    }
                }
                if !reason.is_empty() {
                    ui::gap(ui, theme::S1);
                    ui::label(ui, Role::Caption, reason, theme::MUTED);
                }
            });
        });
    });
    ui::gap(ui, theme::S3);
}

// ────────────────────────────────────────────────────────────────────────
// Used alongside — the small, read-only second tier
// ────────────────────────────────────────────────────────────────────────

/// The recessed second tier: what returns *alongside* where you left off.
///
/// This is the current continuation minus the Resume Point the hero already
/// carries — the handful of things that come back with the work — shown as they
/// are, each with its canonical preflight word. It is deliberately read-only
/// and deliberately small: reconsidering membership and the whole witnessed
/// record live one tier down under "remembered", so this stays a glance and
/// never a workbench. Membership is not importance and being here is not being
/// reopened: the status word, not the presence of a row, says what happens.
fn alongside(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    ui::recessed(ui, |ui| {
        ui.set_width(ui.available_width());
        ui::eyebrow(ui, "Used alongside");

        let Some(selection) = canon.selection else {
            ui::caption(
                ui,
                "Evo has not derived a continuation for this body of work yet.",
            );
            return;
        };

        // The Resume Point is skipped here: the hero already carries it, and
        // this tier is about what comes back *with* it.
        let resume = canon
            .outcome
            .and_then(|outcome| outcome.resume_point())
            .map(|resume| resume.artifact_id().as_str().to_string());
        let statuses: HashMap<&str, &PreflightStatus> = canon
            .preflight
            .iter()
            .map(|outcome| (outcome.artifact_id().as_str(), outcome.status()))
            .collect();

        // Reopenable members first, then those held out of reach; the per-row
        // status word draws the distinction, so no subheading repeats it.
        let mut shown = 0usize;
        for selected in selection.restore_worthy() {
            let artifact_id = selected.artifact_id().as_str();
            if resume.as_deref() == Some(artifact_id) {
                continue;
            }
            let status = statuses.get(artifact_id).copied();
            display_row(
                ui,
                canon,
                artifact_id,
                preflight_word(status, false),
                status.and_then(PreflightStatus::reason),
            );
            shown += 1;
        }
        for item in selection.unavailable() {
            let artifact_id = item.artifact_id().as_str();
            if resume.as_deref() == Some(artifact_id) {
                continue;
            }
            let status = statuses.get(artifact_id).copied();
            // Preflight's reason when it ran, otherwise restoration's own: both
            // are canonical and honest, and one always exists for a member
            // restoration placed out of reach.
            let reason = status
                .and_then(PreflightStatus::reason)
                .unwrap_or_else(|| item.reason());
            display_row(
                ui,
                canon,
                artifact_id,
                preflight_word(status, true),
                Some(reason),
            );
            shown += 1;
        }

        if shown == 0 {
            ui::caption(
                ui,
                "Only where you left off comes back — nothing else is in the \
                 current continuation.",
            );
        }
    });
}

/// One member of the current continuation, shown for reading, not editing.
///
/// The disposition word is the canonical [`PreflightStatus::label`], never a
/// phrasing invented here; the reason is whatever canonical explanation exists.
/// There is no mark: changing what comes back is a deliberate act that lives in
/// the remembered tier, not something the calm glance offers.
fn display_row(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    artifact_id: &str,
    disposition: (&'static str, Severity),
    reason: Option<&str>,
) {
    if state::subject_for(canon.subjects, artifact_id).is_none() {
        return;
    }
    let label = state::describe_artifact(canon.kinds, canon.subjects, artifact_id);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui::one_line(ui, Role::Body, &label, theme::TEXT);
            ui::gap(ui, theme::S1);
            let (word, severity) = disposition;
            ui::status_word(ui, word, severity);
            if let Some(reason) = reason {
                if !reason.is_empty() {
                    ui::gap(ui, theme::S1);
                    ui::label(ui, Role::Caption, reason, theme::MUTED);
                }
            }
        });
    });
    ui::gap(ui, theme::S3);
}

// ────────────────────────────────────────────────────────────────────────
// Remembered — the quiet third tier
// ────────────────────────────────────────────────────────────────────────

/// The quiet third tier: everything else Evo holds for this work.
///
/// Collapsed to a single line by default — "+ N more remembered" — because
/// membership is not importance and the record is not a dashboard. Opening it
/// reveals, in one quiet surface: the way to reconsider what comes back
/// (RFC-0013), the way to relate other work (RFC-0012), and the full witnessed
/// record. None of it is offered until asked for, and none of it competes with
/// the hero. The RFC write paths are fully wired here — quiet is not the same
/// as deferred.
fn remembered(ui: &mut egui::Ui, canon: &Canonical<'_>, tr: &mut Transient<'_>) {
    // Nothing witnessed yet: `identity` has already said so, and an empty
    // disclosure would be noise. There is no record to remember.
    if state::distinct_artifacts_across_history(canon.workspace).is_empty() {
        return;
    }

    ui::quiet(ui, |ui| {
        let count = remembered_count(canon);
        let label = if count == 0 {
            // Everything witnessed is already in the two tiers above; the way
            // in is still offered, because reconsidering and the record itself
            // still live here.
            "Everything Evo remembers here".to_string()
        } else {
            format!("+ {count} more remembered")
        };
        let open = ui::disclosure(ui, ui.id().with("evo-remembered"), &label);
        if !open {
            // A single quiet line at rest — never the record spread open.
            return;
        }

        ui::gap(ui, theme::S4);
        reconsider(ui, canon, tr);
        grouping(ui, canon, tr);
        witnessed_body(ui, canon);
    });
}

/// How many resources Evo holds for this work beyond the two visible tiers —
/// the count behind "+ N more remembered".
///
/// Everything witnessed, minus what the hero (the Resume Point) and "used
/// alongside" (the current continuation) already show. It is a count of what is
/// *out of sight*, so the affordance never overstates how much is hidden.
fn remembered_count(canon: &Canonical<'_>) -> usize {
    let surfaced = surfaced_ids(canon);
    state::distinct_artifacts_across_history(canon.workspace)
        .iter()
        .filter(|id| !surfaced.contains(id.as_str()))
        .count()
}

/// The artifact identities already shown in the hero and the "used alongside"
/// tier, so the remembered count does not re-count what is already in front of
/// the user.
fn surfaced_ids(canon: &Canonical<'_>) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    if let Some(resume) = canon.outcome.and_then(|outcome| outcome.resume_point()) {
        ids.insert(resume.artifact_id().as_str().to_string());
    }
    if let Some(selection) = canon.selection {
        for selected in selection.restore_worthy() {
            ids.insert(selected.artifact_id().as_str().to_string());
        }
        for item in selection.unavailable() {
            ids.insert(item.artifact_id().as_str().to_string());
        }
    }
    ids
}

/// Reconsidering what comes back with this work (RFC-0013 Continuation
/// Surface).
///
/// The editable counterpart to the read-only "used alongside" tier: current
/// members can be unmarked, withheld membership can be marked in, and the whole
/// draft declared. It lives inside the quiet remembered tier because changing
/// what returns is a deliberate act, not the default glance — but it is never
/// more than one disclosure away, and the write path is fully wired.
fn reconsider(ui: &mut egui::Ui, canon: &Canonical<'_>, tr: &mut Transient<'_>) {
    let Some(selection) = canon.selection else {
        return;
    };

    ui::eyebrow(ui, "Reconsider what comes back");

    let current: BTreeSet<String> = state::surface_subjects(selection, canon.subjects)
        .into_iter()
        .collect();
    let statuses: HashMap<&str, &PreflightStatus> = canon
        .preflight
        .iter()
        .map(|outcome| (outcome.artifact_id().as_str(), outcome.status()))
        .collect();

    if current.is_empty() {
        ui::caption(
            ui,
            "Evo's record does not yet show a current continuation for this body \
             of work. Mark the resources it continues across — Evo never assumes \
             that where you continue from is everything that comes back with you.",
        );
    } else {
        ui::caption(ui, "Unmark anything that should not return with this work.");
        ui::gap(ui, theme::S3);

        // Reopenable first, then not — the per-row status word carries the
        // distinction, so no subheading is needed to repeat it.
        for selected in selection.restore_worthy() {
            let artifact_id = selected.artifact_id().as_str();
            let status = statuses.get(artifact_id).copied();
            member_row(
                ui,
                canon,
                tr,
                artifact_id,
                &current,
                Some(preflight_word(status, false)),
                status.and_then(PreflightStatus::reason),
            );
        }
        for item in selection.unavailable() {
            let artifact_id = item.artifact_id().as_str();
            let status = statuses.get(artifact_id).copied();
            let reason = status
                .and_then(PreflightStatus::reason)
                .unwrap_or_else(|| item.reason());
            member_row(
                ui,
                canon,
                tr,
                artifact_id,
                &current,
                Some(preflight_word(status, true)),
                Some(reason),
            );
        }
    }

    related(ui, canon, tr, selection, &current);
    declaration(ui, tr);
}

/// The user-declared related-work surface (RFC-0012 WorkGrouped).
///
/// The user may declare that this body of work is related to another
/// witnessed resource — a browser tab, a window, a file in another folder —
/// that Evo's canonical co-membership evidence cannot relate on its own. The
/// declaration is authoritative user evidence, exactly as RFC-0012 defines
/// WorkGrouped: it is never inferred, never scored, never guessed. The shell
/// only forwards the explicit declaration through the daemon's trusted
/// channel and reflects the daemon's honest response.
fn grouping(ui: &mut egui::Ui, canon: &Canonical<'_>, tr: &mut Transient<'_>) {
    // The anchor is the Workspace's own first witnessed resource in canonical
    // order — the same deterministic rule the Home card uses for its title.
    // The user then declares that another witnessed resource belongs with it.
    let Some(anchor) = state::distinct_artifacts_across_history(canon.workspace)
        .first()
        .and_then(|id| state::subject_for(canon.subjects, id))
    else {
        return;
    };
    let anchor = anchor.to_string();

    // Other witnessed resources: every subject Evo has witnessed that is NOT
    // a member of this Workspace. Presentation-only candidates; the daemon
    // re-validates both subjects against the canonical log before accepting.
    let own: BTreeSet<&str> = state::distinct_artifacts_across_history(canon.workspace)
        .iter()
        .filter_map(|id| state::subject_for(canon.subjects, id))
        .collect();
    let mut others: Vec<&str> = canon
        .subjects
        .values()
        .map(String::as_str)
        .filter(|subject| !own.contains(subject) && *subject != anchor)
        .collect();
    others.sort_unstable();
    others.dedup();
    if others.is_empty() {
        return;
    }

    ui::gap(ui, theme::S5);
    ui::hairline(ui);
    ui::gap(ui, theme::S4);
    ui::eyebrow(ui, "Related work — your call");
    ui::caption(
        ui,
        "Evo never guesses that two resources belong to the same body of work. \
         You can say so: declare that another witnessed resource is part of \
         this work.",
    );
    ui::gap(ui, theme::S3);
    // The candidate list is gated behind its own count, so opening "remembered"
    // never spills every other-workspace subject at once. Declaring related
    // work is a deliberate reach, not something the tier presents by default.
    let open = ui::disclosure(
        ui,
        ui.id().with("evo-group-list"),
        &format!("Show {}", plural(others.len(), "other resource")),
    );
    if open {
        ui::gap(ui, theme::S3);
        for other in others {
            let id = ui.id().with(("evo-group", other));
            let (row, button) = ui::row(ui, id, |ui| {
                ui.horizontal_top(|ui| {
                    ui::one_line(ui, Role::Body, other, theme::MUTED);
                    let button = ui::control(ui, Control::quiet("Declare related"));
                    button
                })
                .inner
            });
            if row.clicked() || button.clicked() {
                *tr.grouping_status = Some(record_grouping(tr.storage_root, &anchor, other));
            }
        }
    }
    if let Some((message, accepted)) = tr.grouping_status.as_ref() {
        ui::gap(ui, theme::S3);
        let (severity, word) = if *accepted {
            (Severity::Good, "Recorded")
        } else {
            (Severity::Bad, "Not recorded")
        };
        ui::notice(ui, severity, word, message);
    }
}

/// Submits a WorkGrouped declaration (RFC-0012) and reports, verbatim, what
/// the daemon said.
fn record_grouping(root: &Path, first: &str, second: &str) -> (String, bool) {
    match state::submit_grouping(root, first, second) {
        Ok(GroupingResponse::Accepted) => (
            "Evo recorded that these resources belong to the same body of work.".to_string(),
            true,
        ),
        Ok(GroupingResponse::Rejected(reason)) => {
            (format!("Evo could not record that: {reason}"), false)
        }
        Err(error) => (
            format!("Evo could not reach the capture worker: {error}"),
            false,
        ),
    }
}

/// Membership that is deliberately outside the current continuation.
///
/// Two sections, because the selection layer derived two answers and they are
/// not the same answer. *Available if needed* is work: the person worked in it
/// or consulted it, and Evo is holding it back only because it is not the
/// continuation being returned to. *History only* is everything Evo happened to
/// witness while the work was going on. Showing them as one list is how a
/// restoration turns back into a list of everything that happened.
fn related(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    selection: &RestorationSelection,
    current: &BTreeSet<String>,
) {
    if selection.withheld().is_empty() {
        return;
    }
    ui::gap(ui, theme::S5);
    ui::hairline(ui);
    ui::gap(ui, theme::S4);

    let to_hand: Vec<&WithheldMember> = selection.available_if_needed().collect();
    let record: Vec<&WithheldMember> = selection.history_only().collect();

    if !to_hand.is_empty() {
        ui::eyebrow(ui, "Part of this work — not reopened");
        ui::caption(
            ui,
            "Evo will not reopen these, and has not put them away either. \
             Marking one moves it into the current continuation.",
        );
        withheld_rows(ui, canon, tr, &to_hand, current, "evo-related-to-hand");
    }

    if !record.is_empty() {
        if !to_hand.is_empty() {
            ui::gap(ui, theme::S4);
        }
        ui::eyebrow(ui, "Witnessed at the time — kept in the record");
        ui::caption(
            ui,
            "Nothing beyond having been there ties these to this work. They stay \
             findable, and you can still say otherwise.",
        );
        withheld_rows(ui, canon, tr, &record, current, "evo-related-record");
    }

    ui::gap(ui, theme::S2);
    ui::caption(
        ui,
        "Membership is never removed by time. Marking one moves it into the \
         current continuation; unmarking moves it out.",
    );
}

/// One collapsed section of withheld membership.
///
/// Each row shows the canonical [`evo_execution::Disposition::label`] and the
/// canonical reason the selection layer attached — never a phrasing invented
/// here.
fn withheld_rows(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    members: &[&WithheldMember],
    current: &BTreeSet<String>,
    id_salt: &'static str,
) {
    let open = ui::disclosure(
        ui,
        ui.id().with(id_salt),
        &format!("Show {}", plural(members.len(), "resource")),
    );
    if !open {
        return;
    }
    ui::gap(ui, theme::S3);
    for member in members {
        member_row(
            ui,
            canon,
            tr,
            member.artifact_id().as_str(),
            current,
            Some((member.disposition().label(), Severity::Neutral)),
            Some(member.reason()),
        );
    }
}

/// One member of this body of work.
///
/// `disposition` is the canonical word for what happens to this member — from
/// [`PreflightStatus::label`] for the current continuation, or
/// [`evo_execution::Disposition::label`] for withheld membership. Neither
/// vocabulary is invented here. `reason` is whatever canonical explanation
/// exists — never a phrasing of this module's own.
fn member_row(
    ui: &mut egui::Ui,
    canon: &Canonical<'_>,
    tr: &mut Transient<'_>,
    artifact_id: &str,
    current: &BTreeSet<String>,
    disposition: Option<(&'static str, Severity)>,
    reason: Option<&str>,
) {
    let Some(subject) = state::subject_for(canon.subjects, artifact_id) else {
        return;
    };
    let subject = subject.to_string();
    let label = state::describe_artifact(canon.kinds, canon.subjects, artifact_id);

    let declared = current.contains(&subject);
    let member = match tr.continuation_draft.as_ref() {
        Some(draft) => draft.contains(&subject),
        None => declared,
    };
    let drafted = tr.continuation_draft.is_some() && member != declared;

    let ink = if !member {
        theme::RECEDED
    } else if drafted {
        theme::MUTED
    } else {
        theme::TEXT
    };

    let id = ui.id().with(("evo-member", artifact_id));
    let (row, mark) = ui::row(ui, id, |ui| {
        ui.horizontal_top(|ui| {
            let mark = ui::set_mark(ui, id.with("mark"), member, drafted);
            ui.add_space(theme::S3);
            ui.vertical(|ui| {
                ui::one_line(ui, Role::Body, &label, ink);
                if let Some((word, severity)) = disposition {
                    ui::gap(ui, theme::S1);
                    ui::status_word(ui, word, severity);
                }
                if let Some(reason) = reason {
                    if !reason.is_empty() {
                        ui::gap(ui, theme::S1);
                        ui::label(ui, Role::Caption, reason, theme::MUTED);
                    }
                }
            });
            mark
        })
        .inner
    });
    // `||` short-circuits, so exactly one toggle results no matter which of
    // the two widgets egui awarded the click to.
    if row.clicked() || mark.clicked() {
        toggle(tr, current, &subject);
    }
}

/// The status word for one member of the current continuation.
///
/// The word comes from [`PreflightStatus::label`] so the UI cannot drift from
/// the canonical `READY` / `UNAVAILABLE` / `AMBIGUOUS` / `UNSUPPORTED`
/// vocabulary. A missing entry is reported as unchecked rather than assumed
/// reopenable; `UNAVAILABLE` without an entry is not a guess, because
/// restoration itself placed the resource in the unavailable set.
fn preflight_word(
    status: Option<&PreflightStatus>,
    known_unavailable: bool,
) -> (&'static str, Severity) {
    match status {
        Some(status) => (
            status.label(),
            match status {
                PreflightStatus::Ready => Severity::Good,
                PreflightStatus::Unavailable { .. } | PreflightStatus::Ambiguous { .. } => {
                    Severity::Caution
                }
                PreflightStatus::Unsupported { .. } => Severity::Neutral,
            },
        ),
        None if known_unavailable => ("UNAVAILABLE", Severity::Caution),
        None => ("NOT CHECKED", Severity::Neutral),
    }
}

/// Moves one subject into or out of the draft.
///
/// The draft starts as a copy of what Evo currently holds, so the first mark
/// the user makes is an edit to Evo's understanding rather than a fresh empty
/// set. Nothing here is recorded.
fn toggle(tr: &mut Transient<'_>, current: &BTreeSet<String>, subject: &str) {
    *tr.continuation_status = None;
    let mut next = tr
        .continuation_draft
        .clone()
        .unwrap_or_else(|| current.clone());
    if next.contains(subject) {
        next.remove(subject);
    } else {
        next.insert(subject.to_string());
    }
    *tr.continuation_draft = Some(next);
}

/// The declaration control, offered only once there is something to declare.
fn declaration(ui: &mut egui::Ui, tr: &mut Transient<'_>) {
    ui::gap(ui, theme::S5);

    if tr.continuation_draft.is_some() {
        ui::caption(
            ui,
            "You've changed what's marked. Nothing is recorded until you declare it.",
        );
        if ui::control(ui, Control::secondary("Declare continuation")).clicked() {
            let subjects: Vec<String> = tr
                .continuation_draft
                .as_ref()
                .map(|draft| draft.iter().cloned().collect())
                .unwrap_or_default();
            let outcome = record_surface(tr.storage_root, &subjects);
            let accepted = outcome.1;
            *tr.continuation_status = Some(outcome);
            if accepted {
                *tr.continuation_draft = None;
            }
        }
    } else {
        ui::caption(
            ui,
            "Marking a resource changes what Evo would reopen. You declare when you're ready.",
        );
    }

    if let Some((message, accepted)) = tr.continuation_status.as_ref() {
        ui::gap(ui, theme::S3);
        let (severity, word) = if *accepted {
            (Severity::Good, "Recorded")
        } else {
            (Severity::Bad, "Not recorded")
        };
        ui::notice(ui, severity, word, message);
    }
}

/// Submits a Continuation Surface declaration and reports the daemon's answer.
///
/// The subjects are submitted in canonical order, which `BTreeSet` already
/// guarantees; the daemon canonicalises again regardless.
fn record_surface(root: &Path, subjects: &[String]) -> (String, bool) {
    match state::submit_continuation_surface(root, subjects) {
        Ok(ContinuationResponse::Accepted) => {
            ("Evo updated the current continuation.".to_string(), true)
        }
        Ok(ContinuationResponse::Rejected(reason)) => {
            (format!("Evo could not record that: {reason}"), false)
        }
        Err(error) => (
            format!("Evo could not reach the capture worker: {error}"),
            false,
        ),
    }
}

// ────────────────────────────────────────────────────────────────────────
// What Evo witnessed
// ────────────────────────────────────────────────────────────────────────

/// The evidence, rendered inside the remembered tier's quiet surface.
///
/// This is the deepest thing on the screen by design: it is why Evo believes
/// what it believes, which matters, but it is not what the user came for — so
/// it sits behind the remembered disclosure, never on its own surface (two
/// translucent layers must not stack). The latest witnessed moment is
/// presentation-only information about the record's most recent append — never
/// a reason anything is restored.
fn witnessed_body(ui: &mut egui::Ui, canon: &Canonical<'_>) {
    ui::gap(ui, theme::S5);
    ui::hairline(ui);
    ui::gap(ui, theme::S4);
    ui::eyebrow(ui, "What Evo witnessed");

    match canon.workspace.snapshots().last() {
        Some(latest) => {
            ui::field(ui, "Latest moment", &state::snapshot_time_label(latest));
            ui::gap(ui, theme::S3);
            let recent = state::distinct_artifacts_in_latest_snapshot(canon.workspace);
            if recent.is_empty() {
                ui::unknown(ui, "Nothing was witnessed in that moment.");
            } else {
                for artifact_id in &recent {
                    ui::one_line(
                        ui,
                        Role::Body,
                        &format!(
                            "— {}",
                            state::describe_artifact(canon.kinds, canon.subjects, artifact_id)
                        ),
                        theme::MUTED,
                    );
                    ui::gap(ui, theme::S1);
                }
            }
        }
        None => {
            ui::unknown(
                ui,
                "Evo has not witnessed a moment in this body of work yet.",
            );
            return;
        }
    }

    let all = state::distinct_artifacts_across_history(canon.workspace);
    if all.is_empty() {
        return;
    }
    ui::gap(ui, theme::S5);
    ui::field(
        ui,
        "Across this work",
        &format!(
            "{}, over {}",
            plural(all.len(), "resource"),
            plural(canon.workspace.snapshots().len(), "moment")
        ),
    );
    ui::gap(ui, theme::S2);
    let open = ui::disclosure(ui, ui.id().with("evo-history"), "Show every resource");
    if !open {
        return;
    }
    ui::gap(ui, theme::S3);
    for artifact_id in all.iter().take(MAX_RENDERED_RESOURCES) {
        ui::one_line(
            ui,
            Role::Body,
            &format!(
                "— {}",
                state::describe_artifact(canon.kinds, canon.subjects, artifact_id)
            ),
            theme::MUTED,
        );
        ui::gap(ui, theme::S1);
    }
    if all.len() > MAX_RENDERED_RESOURCES {
        ui::gap(ui, theme::S2);
        ui::caption(
            ui,
            &format!(
                "Showing the first {MAX_RENDERED_RESOURCES} of {}. The record holds them all.",
                all.len()
            ),
        );
    }
}

// ────────────────────────────────────────────────────────────────────────
// Wording
// ────────────────────────────────────────────────────────────────────────

/// Capitalises a witnessed kind label for use at the start of a line.
///
/// Purely typographic: the canonical kind string is untouched.
fn sentence_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// `1 resource` / `2 resources`. Avoids the `(s)` that reads as machine output.
fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_label_is_capitalized_without_being_rewritten() {
        assert_eq!(sentence_case("focused window"), "Focused window");
        assert_eq!(sentence_case("saved file"), "Saved file");
        assert_eq!(sentence_case(""), "");
        // Already capitalized text survives untouched.
        assert_eq!(sentence_case("URL"), "URL");
    }

    // FIX 2 (BE-AUDIT-0001 §2.4): the announced plan line must describe
    // exactly what execution will attempt, in exactly the attempted order, on
    // both branches. This is the regression test whose absence let the
    // announcement and the act diverge (audit §13.2(a)).
    #[test]
    fn plan_line_describes_exactly_what_execution_will_attempt() {
        use evo_artifact::artifact_id::ArtifactId;
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};
        use evo_workspace::attachment::{Attachment, ResourceRole};
        use evo_workspace::confidence::ConfidenceScore;
        use evo_workspace::lifecycle::WorkspaceLifecycle;
        use evo_workspace::snapshot::Snapshot;
        use evo_workspace::workspace_id::WorkspaceId;
        use std::collections::HashMap;
        use std::str::FromStr;

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174950").unwrap();
        let snapshot = Snapshot::new(
            std::time::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![
                Attachment::new(
                    ArtifactId::new("artifact-zz").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    // These fixtures exercise announcement ordering, which
                    // reads the surface rather than the roles; Primary is the
                    // plain "the work happens here" role.
                    ResourceRole::Primary,
                ),
                Attachment::new(
                    ArtifactId::new("artifact-aa").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    ResourceRole::Primary,
                ),
            ],
        );
        let ws = Workspace::new(
            workspace_id.clone(),
            WorkspaceLifecycle::Active,
            snapshot.attachments().to_vec(),
            vec![snapshot],
        );

        // Surface branch: the Resume Point (artifact-zz) is announced first,
        // because "where the work continues" sorts before everything else
        // (IS-0019 RSP-1; plan_selection uses restoration_key).
        let plan = RestorationPlan::new_with_continuation_surface(
            workspace_id.clone(),
            ResumePoint::new(
                workspace_id.clone(),
                ArtifactId::new("artifact-zz").unwrap(),
            ),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
            vec![
                ArtifactId::new("artifact-zz").unwrap(),
                ArtifactId::new("artifact-aa").unwrap(),
            ],
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);
        let mut evidence = HashMap::new();
        evidence.insert(
            "artifact-zz".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/zz.md".to_string()),
        );
        evidence.insert(
            "artifact-aa".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/aa.md".to_string()),
        );
        let members: Vec<(ArtifactId, ResourceRole)> = ws
            .attachments()
            .iter()
            .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
            .collect();
        let selection = evo_execution::select_restoration(&outcome, &members, &evidence);
        let mut locators = HashMap::new();
        locators.insert(
            "artifact-zz".to_string(),
            Locator::new(
                ArtifactId::new("artifact-zz").unwrap(),
                LocatorKind::FilePath("/repo/zz.md".into()),
            ),
        );
        locators.insert(
            "artifact-aa".to_string(),
            Locator::new(
                ArtifactId::new("artifact-aa").unwrap(),
                LocatorKind::FilePath("/repo/aa.md".into()),
            ),
        );
        let canon = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: Some(&selection),
            preflight: &[],
            subjects: &HashMap::new(),
            kinds: &HashMap::new(),
            titles: &HashMap::new(),
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert_eq!(
            plan_line(&canon).expect("a declared surface announces a plan"),
            "open file /repo/zz.md · open file /repo/aa.md"
        );

        // Non-surface branch: Resume Point first, then the Context Chain in
        // its canonical order — which is not ascending ArtifactId here.
        let plan = RestorationPlan::new(
            workspace_id.clone(),
            ResumePoint::new(
                workspace_id.clone(),
                ArtifactId::new("artifact-mid").unwrap(),
            ),
            ContextChain::new(vec![
                ArtifactId::new("artifact-z").unwrap(),
                ArtifactId::new("artifact-a").unwrap(),
            ])
            .unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);
        let mut locators = HashMap::new();
        locators.insert(
            "artifact-mid".to_string(),
            Locator::new(
                ArtifactId::new("artifact-mid").unwrap(),
                LocatorKind::FilePath("/repo/mid.md".into()),
            ),
        );
        locators.insert(
            "artifact-z".to_string(),
            Locator::new(
                ArtifactId::new("artifact-z").unwrap(),
                LocatorKind::FilePath("/repo/z.md".into()),
            ),
        );
        locators.insert(
            "artifact-a".to_string(),
            Locator::new(
                ArtifactId::new("artifact-a").unwrap(),
                LocatorKind::FilePath("/repo/a.md".into()),
            ),
        );
        let canon = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: None,
            preflight: &[],
            subjects: &HashMap::new(),
            kinds: &HashMap::new(),
            titles: &HashMap::new(),
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert_eq!(
            plan_line(&canon).expect("a Resume Point announces a plan"),
            "open file /repo/mid.md · open file /repo/z.md · open file /repo/a.md"
        );
    }

    #[test]
    fn counts_read_as_english() {
        assert_eq!(plural(0, "resource"), "0 resources");
        assert_eq!(plural(1, "resource"), "1 resource");
        assert_eq!(plural(2, "moment"), "2 moments");
    }

    // The primary Continue act is enabled exactly when the shared ordering
    // function (the one run_execution consumes) announces at least one target
    // that will actually be attempted. A declared Continuation Surface
    // (RFC-0013) is the restore candidate set even when the derivation is
    // Insufficient with no Resume Point (IS-0021 §25.2: several eligible
    // candidates, no designation) — the surface path of run_execution
    // executes it. The gate must therefore follow the plan, not the Resume
    // Point (UI-VALIDATION-0001 B.9: the promise and the act derive from one
    // shared function).
    #[test]
    fn a_declared_surface_enables_continue_without_a_resume_point() {
        use evo_artifact::artifact_id::ArtifactId;
        use evo_restoration::{RestorationInput, derive_restoration_plan};
        use evo_workspace::attachment::{Attachment, ResourceRole};
        use evo_workspace::confidence::ConfidenceScore;
        use evo_workspace::lifecycle::WorkspaceLifecycle;
        use evo_workspace::snapshot::Snapshot;
        use evo_workspace::workspace_id::WorkspaceId;
        use std::collections::HashMap;
        use std::str::FromStr;

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174952").unwrap();
        let snapshot = Snapshot::new(
            std::time::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![
                Attachment::new(
                    ArtifactId::new("artifact-aa").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    // Announcement ordering reads the surface, not the roles.
                    ResourceRole::Primary,
                ),
                Attachment::new(
                    ArtifactId::new("artifact-zz").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    ResourceRole::Primary,
                ),
            ],
        );
        let ws = Workspace::new(
            workspace_id.clone(),
            WorkspaceLifecycle::Active,
            snapshot.attachments().to_vec(),
            vec![snapshot],
        );
        // Two candidates, no designation → Insufficient with no Resume Point
        // (IS-0021 §25.2); the declared surface still carries the
        // continuation (RFC-0013).
        let surface = vec![
            ArtifactId::new("artifact-aa").unwrap(),
            ArtifactId::new("artifact-zz").unwrap(),
        ];
        let snapshot = ws.snapshots().last().unwrap();
        let input =
            RestorationInput::new_with_continuation_surface(&ws, snapshot, None, Some(surface))
                .unwrap();
        let outcome = derive_restoration_plan(&input);
        assert!(outcome.resume_point().is_none());
        assert_eq!(outcome.continuation_surface().len(), 2);

        let mut evidence = HashMap::new();
        evidence.insert(
            "artifact-aa".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/aa.md".to_string()),
        );
        evidence.insert(
            "artifact-zz".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/zz.md".to_string()),
        );
        let members: Vec<(ArtifactId, ResourceRole)> = ws
            .attachments()
            .iter()
            .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
            .collect();
        let selection = evo_execution::select_restoration(&outcome, &members, &evidence);
        let mut locators = HashMap::new();
        locators.insert(
            "artifact-aa".to_string(),
            Locator::new(
                ArtifactId::new("artifact-aa").unwrap(),
                LocatorKind::FilePath("/repo/aa.md".into()),
            ),
        );
        locators.insert(
            "artifact-zz".to_string(),
            Locator::new(
                ArtifactId::new("artifact-zz").unwrap(),
                LocatorKind::FilePath("/repo/zz.md".into()),
            ),
        );
        let canon = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: Some(&selection),
            preflight: &[],
            subjects: &HashMap::new(),
            kinds: &HashMap::new(),
            titles: &HashMap::new(),
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        // The Continue gate follows the shared plan: a declared surface with
        // no Resume Point still announces both members, so Continue is ready
        // — the regression the gate previously failed (it required a Resume
        // Point and left the declared continuation dead).
        assert_eq!(
            plan_line(&canon).expect("a declared surface without a Resume Point announces"),
            "open file /repo/aa.md · open file /repo/zz.md"
        );
        assert!(continue_ready(&canon));
    }

    // Continue is ready only when something can actually be opened: no
    // outcome, an undeclared multi-candidate Workspace, or a declared surface
    // whose members are all unavailable produce no plan, so the button stays
    // off and the reason stays visible.
    #[test]
    fn continue_is_ready_only_when_something_can_be_opened() {
        use evo_artifact::artifact_id::ArtifactId;
        use evo_restoration::{RestorationInput, derive_restoration_plan};
        use evo_workspace::attachment::{Attachment, ResourceRole};
        use evo_workspace::confidence::ConfidenceScore;
        use evo_workspace::lifecycle::WorkspaceLifecycle;
        use evo_workspace::snapshot::Snapshot;
        use evo_workspace::workspace_id::WorkspaceId;
        use std::collections::HashMap;
        use std::str::FromStr;

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174953").unwrap();
        let snapshot = Snapshot::new(
            std::time::UNIX_EPOCH,
            WorkspaceLifecycle::Active,
            vec![
                Attachment::new(
                    ArtifactId::new("artifact-aa").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    // Announcement ordering reads the surface, not the roles.
                    ResourceRole::Primary,
                ),
                Attachment::new(
                    ArtifactId::new("artifact-zz").unwrap(),
                    ConfidenceScore::new(1.0).unwrap(),
                    ResourceRole::Primary,
                ),
            ],
        );
        let ws = Workspace::new(
            workspace_id.clone(),
            WorkspaceLifecycle::Active,
            snapshot.attachments().to_vec(),
            vec![snapshot],
        );
        let snapshot = ws.snapshots().last().unwrap();
        let empty: HashMap<String, String> = HashMap::new();
        let no_evidence: HashMap<String, (String, String)> = HashMap::new();
        let members: Vec<(ArtifactId, ResourceRole)> = ws
            .attachments()
            .iter()
            .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
            .collect();
        let locators: HashMap<String, Locator> = HashMap::new();

        // (a) No derived outcome at all → no plan, Continue off.
        let canon_none = Canonical {
            workspace: &ws,
            outcome: None,
            selection: None,
            preflight: &[],
            subjects: &empty,
            kinds: &empty,
            titles: &empty,
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert!(plan_line(&canon_none).is_none());
        assert!(!continue_ready(&canon_none));

        // (b) Two candidates, no surface, no designation → Insufficient, no
        // Resume Point, and nothing to announce (IS-0021 §25.2/§25.8).
        let input = RestorationInput::new(&ws, snapshot).unwrap();
        let outcome = derive_restoration_plan(&input);
        assert!(outcome.resume_point().is_none());
        let selection = evo_execution::select_restoration(&outcome, &members, &no_evidence);
        let canon_undeclared = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: Some(&selection),
            preflight: &[],
            subjects: &empty,
            kinds: &empty,
            titles: &empty,
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert!(plan_line(&canon_undeclared).is_none());
        assert!(!continue_ready(&canon_undeclared));

        // (c) A declared surface whose members are all a commit (no
        // executable target) → restore-worthy is empty, so nothing is
        // announced and Continue stays off.
        let surface = vec![ArtifactId::new("artifact-aa").unwrap()];
        let input =
            RestorationInput::new_with_continuation_surface(&ws, snapshot, None, Some(surface))
                .unwrap();
        let outcome = derive_restoration_plan(&input);
        let mut commit_evidence = HashMap::new();
        commit_evidence.insert(
            "artifact-aa".to_string(),
            (
                "OBS-COMMIT-MADE".to_string(),
                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
            ),
        );
        let selection = evo_execution::select_restoration(&outcome, &members, &commit_evidence);
        assert!(selection.restore_worthy().is_empty());
        let canon_commit = Canonical {
            workspace: &ws,
            outcome: Some(&outcome),
            selection: Some(&selection),
            preflight: &[],
            subjects: &empty,
            kinds: &empty,
            titles: &empty,
            standings: &HashMap::new(),
            locators: &locators,
            designation: None,
        };
        assert!(plan_line(&canon_commit).is_none());
        assert!(!continue_ready(&canon_commit));
    }

    #[test]
    fn the_status_vocabulary_comes_from_the_canonical_source() {
        // Not re-typed here: if the canonical vocabulary ever changes, the UI
        // follows it instead of quietly disagreeing with it.
        assert_eq!(
            preflight_word(Some(&PreflightStatus::Ready), false).0,
            PreflightStatus::Ready.label()
        );
        for status in [
            PreflightStatus::Unavailable {
                reason: "gone".to_string(),
            },
            PreflightStatus::Ambiguous {
                reason: "two matches".to_string(),
            },
            PreflightStatus::Unsupported {
                reason: "no handler".to_string(),
            },
        ] {
            assert_eq!(preflight_word(Some(&status), false).0, status.label());
            assert_ne!(preflight_word(Some(&status), false).0, "READY");
        }
    }

    #[test]
    fn a_missing_preflight_entry_is_never_reported_as_reopenable() {
        // Absence of evidence is reported as absence, not as readiness.
        assert_eq!(preflight_word(None, false).0, "NOT CHECKED");
        // Unless restoration itself already placed it out of reach.
        assert_eq!(preflight_word(None, true).0, "UNAVAILABLE");
        assert_ne!(preflight_word(None, false).0, "READY");
        assert_ne!(preflight_word(None, true).0, "READY");
    }

    #[test]
    fn an_unsupported_resource_borrows_neither_success_nor_failure() {
        let unsupported = PreflightStatus::Unsupported {
            reason: "no handler".to_string(),
        };
        let (_, severity) = preflight_word(Some(&unsupported), false);
        assert_eq!(severity, Severity::Neutral);
        assert_ne!(severity, Severity::Good);
        assert_ne!(severity, Severity::Bad);
    }

    #[test]
    fn a_status_word_exists_for_every_preflight_outcome() {
        // Every branch produces a word: the UI can never fall back to a bare
        // color, which is what IS-0019 and RFC-0006 R-2 forbid.
        for status in [
            None,
            Some(PreflightStatus::Ready),
            Some(PreflightStatus::Unavailable {
                reason: String::new(),
            }),
            Some(PreflightStatus::Ambiguous {
                reason: String::new(),
            }),
            Some(PreflightStatus::Unsupported {
                reason: String::new(),
            }),
        ] {
            let (word, _) = preflight_word(status.as_ref(), false);
            assert!(!word.is_empty());
        }
    }
}
