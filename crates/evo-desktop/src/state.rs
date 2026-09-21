//! Canonical state loading for the Evo desktop shell.
//!
//! The desktop shell is a presentation + user-interaction layer. It never
//! owns Workspace identity, Snapshot semantics, persistence semantics, or
//! restoration derivation. It consumes canonical Workspace understanding
//! from the daemon's persistence boundary (`evo_daemon::persistence`).

use evo_artifact::artifact_id::ArtifactId;
use evo_daemon::cache::CanonicalIndex;
use evo_daemon::continuation::{
    ContinuationResponse, submit_continuation_surface as daemon_submit_continuation_surface,
};
use evo_daemon::designation::{
    DesignationResponse, submit_designation as daemon_submit_designation,
};
use evo_daemon::grouping::{GroupingResponse, submit_grouping as daemon_submit_grouping};
use evo_daemon::ui::select_display_workspace;
use evo_engagement::Standing;
use evo_execution::RestorationSelection;
use evo_execution::engine::{
    ExecutionReport, RestorationStep, StepPlan, execute_step, plan_request, plan_selection,
};
use evo_execution::locator::{
    Locator, SCHEMA_COMMIT_MADE, SCHEMA_FILE_SAVED, SCHEMA_URL_NAVIGATED,
    SCHEMA_WINDOW_FOCUS_GAINED, classify_locator,
};
use evo_restoration::DerivationOutcome;
use evo_restoration::execution::ExecutionRequest;
use evo_workspace::attachment::Attachment;
use evo_workspace::snapshot::Snapshot;
use evo_workspace::workspace::Workspace;
use evo_workspace::workspace_id::WorkspaceId;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// The one canonical storage location shared with the daemon runtime.
///
/// Resolved by evo-storage's single shared resolver: `EVO_STORAGE_ROOT`
/// overrides the location for developer/testing purposes only; the default is
/// the application's persistent Application Support location, never the temp
/// directory (BE-TRACE-0001 §3.4).
pub fn canonical_storage_root() -> PathBuf {
    evo_storage::canonical_storage_root()
}

/// The canonical state the desktop shell presents.
pub struct DisplayState {
    /// Every canonical Workspace currently remembered.
    pub workspaces: Vec<Workspace>,
    /// The single Workspace selected for the primary screen.
    pub display: Option<Workspace>,
    /// The canonical Restoration Derivation outcome for the displayed
    /// Workspace, derived by the daemon's canonical index. The shell never
    /// performs derivation itself.
    pub outcome: Option<DerivationOutcome>,
    /// Every derived canonical derivation outcome, keyed by Workspace
    /// identity. The workspace list uses these derived outcomes to show each
    /// remembered body of work's truthful restoration status.
    pub outcomes: HashMap<String, DerivationOutcome>,
    /// What each remembered body of work is about, keyed by Workspace
    /// identity — the name Home calls it by. Derived by the daemon's canonical
    /// index from the vocabulary the work's own members share; presentation
    /// material, never a canonical model field.
    pub titles: HashMap<String, String>,
    /// How much of each body of work Evo is prepared to claim, keyed by
    /// Workspace identity: whether it is presented on Home as work at all
    /// (`Standing::is_work`), and whether it may be auto-opened. Decided by the
    /// Engagement layer from the witnessed relationship graph, never from an
    /// application, domain, file type, or category. A body Evo holds only as
    /// `Standing::Remembered` is still a full Workspace here — kept, findable by
    /// name — it is simply not presented as work (WORK-MODEL; no silent loss).
    pub standings: HashMap<String, Standing>,
    /// The subject each canonical Artifact's Observations witnessed, derived
    /// from the persisted Observation log. Presentation evidence only.
    pub subjects: HashMap<String, String>,
    /// The human-readable kind each canonical Artifact's frozen Observation
    /// schema establishes (focused window / saved file / visited URL /
    /// commit), derived from the persisted Observation log. Presentation
    /// evidence only; never a canonical model field.
    pub kinds: HashMap<String, String>,
    /// The executable-target locator each canonical Artifact's Observations
    /// classify to, derived from the persisted Observation log. Presentation
    /// evidence only; consumed by the Execution layer, never stored.
    pub locators: HashMap<String, Locator>,
    /// The current WorkDesignated designation — the subject the user marked
    /// as the work to continue and the moment it was witnessed (RFC-0011) —
    /// derived from the persisted Observation log. Presentation evidence
    /// only; never a canonical model field.
    pub designation: Option<(String, std::time::SystemTime)>,
    /// The derived selective-restoration understanding for the displayed
    /// Workspace: what Evo would restore from the current continuation,
    /// what is honestly unavailable, and what remains historical (durable
    /// membership outside the surface). Purely derived presentation
    /// material — rebuildable from canonical state, never canonical itself,
    /// never used to mutate Workspace membership or continuation semantics.
    pub selection: Option<RestorationSelection>,
}

/// Loads canonical Workspace understanding and its derived Restoration
/// outcome from the daemon's canonical index, selecting the Workspace the
/// primary screen presents.
///
/// This is the full-replay path: a fresh derived index is built from the
/// canonical logs. The long-lived desktop shell keeps one index and refreshes
/// it incrementally ([`display_state_from_index`]) so repeated reloads do not
/// re-parse the full history; both paths produce identical state.
pub fn load_display_state(root: &Path) -> Result<DisplayState, String> {
    let index = CanonicalIndex::new(root).map_err(|err| err.to_string())?;
    Ok(display_state_from_index(&index))
}

/// Orders the Home list so actual continuable work has prominence, without
/// ever omitting a remembered Workspace (BE-TRACE-0001 §3.2(b)).
///
/// - Workspaces with a real derived Resume Point (RFC-0011) come first —
///   work the user explicitly marked as continuing, which is the work Evo
///   can actually reopen.
/// - Workspaces without one follow.
/// - Within each group the existing deterministic canonical order (the
///   derived projection's order — most recently active body of work first,
///   which is a pure function of the Observation log and so stable across
///   restarts) is preserved exactly.
///
/// The only distinction is the derived Resume Point Home already computes.
/// No score, no recency, no frequency, no application identity, no guessed
/// ranking, and no omission: a stable partition, deterministic on canonical
/// input.
pub fn order_home_workspaces(
    workspaces: &[Workspace],
    outcomes: &HashMap<String, DerivationOutcome>,
) -> Vec<Workspace> {
    let mut ordered = workspaces.to_vec();
    // False sorts before true, so workspaces WITH a Resume Point (key false)
    // come first. `sort_by_key` is stable: canonical order is preserved
    // within each group.
    ordered.sort_by_key(|workspace| {
        !outcomes
            .get(&workspace.id().to_string())
            .is_some_and(|outcome| outcome.resume_point().is_some())
    });
    ordered
}

/// Builds the presentation state from a derived canonical index.
///
/// The index is refreshed by the caller (the desktop shell refreshes its
/// long-lived index every reload; a fresh index is a full replay). All
/// derived maps here are read from the index, never re-derived from the logs.
pub fn display_state_from_index(index: &CanonicalIndex) -> DisplayState {
    let outcomes: HashMap<String, DerivationOutcome> = index.outcomes().clone();
    // Home's list order: continuable work (a derived Resume Point) is grouped
    // first, nothing is ever omitted, and canonical log order is preserved
    // within each group.
    let workspaces = order_home_workspaces(index.workspaces(), &outcomes);
    // The current designation is derived before the display selection so the
    // Workspace containing the designated Artifact can be surfaced first when
    // the user has not selected one explicitly.
    let designation = index.designation();
    // The designated canonical Artifact (RFC-0011), resolved from the current
    // designation's subject against the canonical Observation log.
    let designated = index.current_designated_artifact();
    // The primary surface shows a body of *work*. A Remembered body stays in
    // the ordered list (findable, never dropped) but is never auto-promoted to
    // the primary detail on its own — only an explicit selection or the user's
    // own designation can open one. So the automatic pick is drawn from the
    // work subset, while a designation still resolves against the whole list.
    let standings = index.standings();
    let work_ordered: Vec<Workspace> = workspaces
        .iter()
        .filter(|workspace| {
            standings
                .get(&workspace.id().to_string())
                .is_some_and(|standing| standing.is_work())
        })
        .cloned()
        .collect();
    let display = designated
        .as_ref()
        .and_then(|artifact| {
            workspaces
                .iter()
                .find(|workspace| {
                    workspace
                        .attachments()
                        .iter()
                        .any(|attachment| attachment.artifact_id() == artifact)
                })
                .cloned()
        })
        .or_else(|| select_display_workspace(&work_ordered).cloned());
    let outcome = display
        .as_ref()
        .and_then(|workspace| outcomes.get(&workspace.id().to_string()).cloned());
    let subjects = index.subjects().clone();
    let titles = index.titles().clone();
    let standings = index.standings().clone();
    let kinds = kinds_from_locators(index.locators());
    let locators = locators_from_locators(index.locators());
    let selection = match (display.as_ref(), outcome.as_ref()) {
        (Some(workspace), Some(outcome)) => {
            Some(selection_for(workspace, outcome, index.locators()))
        }
        _ => None,
    };
    DisplayState {
        workspaces,
        display,
        outcome,
        outcomes,
        titles,
        standings,
        subjects,
        kinds,
        locators,
        designation,
        selection,
    }
}

/// Computes the derived selective-restoration understanding for one
/// Workspace from canonical inputs only: the derived plan outcome (whose
/// per-Workspace Continuation Surface is the current continuation, RFC-0013),
/// the Workspace's durable membership with the part each member played, and the
/// canonical (schema, subject) resource evidence per Artifact derived from the
/// persisted Observation log.
///
/// The selection partitions the surface into restore-worthy and unavailable
/// resources and gives every member outside it an explicit disposition —
/// kept to hand, or kept only as record. Purely derived presentation material —
/// never canonical, never fed back into Workspace Formation, Artifact identity,
/// or continuation derivation.
pub fn selection_for(
    workspace: &Workspace,
    outcome: &DerivationOutcome,
    resource_evidence: &HashMap<String, (String, String)>,
) -> RestorationSelection {
    let members = workspace.members_across_history();
    evo_execution::select_restoration(outcome, &members, resource_evidence)
}

/// Builds the canonical Artifact-kind map from the index's derived locator
/// evidence: each Artifact whose Observation log records a frozen schema gets
/// the plain-language kind that schema establishes.
pub fn kinds_from_locators(
    locators: &HashMap<String, (String, String)>,
) -> HashMap<String, String> {
    let mut kinds = HashMap::new();
    for (artifact_id, (schema_name, _subject)) in locators {
        if let Some(label) = schema_kind_label(schema_name) {
            kinds.insert(artifact_id.clone(), label.to_string());
        }
    }
    kinds
}

/// Builds the canonical executable-target locator map from the index's
/// derived locator evidence. Each Artifact's witnessed (schema, subject) is
/// classified deterministically by the Execution layer's frozen rule.
pub fn locators_from_locators(
    locators: &HashMap<String, (String, String)>,
) -> HashMap<String, Locator> {
    let mut resolved = HashMap::new();
    for (artifact_id, (schema_name, subject)) in locators {
        if let Some(kind) = classify_locator(schema_name, subject) {
            if let Ok(id) = ArtifactId::new(artifact_id.clone()) {
                resolved.insert(artifact_id.clone(), Locator::new(id, kind));
            }
        }
    }
    resolved
}

/// Human-readable kind labels for the frozen canonical Observation schemas.
///
/// These are presentation constants mapping a canonical schema name to the
/// plain-language kind of resource its observations witnessed. Deterministic
/// and derived from canonical evidence only; never a canonical model field.
pub const KIND_WINDOW: &str = "focused window";
pub const KIND_FILE: &str = "saved file";
pub const KIND_URL: &str = "visited URL";
pub const KIND_COMMIT: &str = "commit";

/// The plain-language kind label for one frozen canonical Observation schema.
///
/// Returns `None` for schemas this build does not know, so an unknown schema
/// is never mislabeled.
pub fn schema_kind_label(schema_name: &str) -> Option<&'static str> {
    match schema_name {
        SCHEMA_WINDOW_FOCUS_GAINED => Some(KIND_WINDOW),
        SCHEMA_FILE_SAVED => Some(KIND_FILE),
        SCHEMA_URL_NAVIGATED => Some(KIND_URL),
        SCHEMA_COMMIT_MADE => Some(KIND_COMMIT),
        _ => None,
    }
}

/// The plain-language kind of one canonical Artifact, when its Observation
/// log establishes one.
pub fn kind_for<'a>(kinds: &'a HashMap<String, String>, artifact_id: &str) -> Option<&'a str> {
    kinds.get(artifact_id).map(String::as_str)
}

/// Submits the user's explicit designation through the daemon's canonical
/// channel (RFC-0011) and returns the daemon's honest response.
///
/// The shell never writes canonical state: the daemon — the canonical runtime
/// owner — validates the subject, accepts and persists the WorkDesignated
/// observation, and re-derives. This only forwards the user's explicit action.
pub fn submit_designation(root: &Path, subject: &str) -> Result<DesignationResponse, String> {
    daemon_submit_designation(root, subject).map_err(|err| err.to_string())
}

/// Submits the user's explicit continuation-surface declaration through the
/// daemon's canonical channel (RFC-0013) and returns the daemon's honest
/// response.
///
/// The shell never writes canonical state: the daemon — the canonical runtime
/// owner — validates that every subject is witnessed in the canonical log,
/// canonicalizes the set (deduplicated, ascending order), accepts and
/// persists the OBS-CONTINUATION-SURFACE observation, and re-derives every
/// Workspace. This only forwards the user's explicit declaration of what
/// their work currently continues across.
pub fn submit_continuation_surface(
    root: &Path,
    subjects: &[String],
) -> Result<ContinuationResponse, String> {
    daemon_submit_continuation_surface(root, subjects).map_err(|err| err.to_string())
}

/// Submits the user's explicit related-work declaration through the daemon's
/// canonical channel (RFC-0012 WorkGrouped) and returns the daemon's honest
/// response.
///
/// The shell never writes canonical state: the daemon — the canonical runtime
/// owner — validates that both subjects are witnessed in the canonical log,
/// canonicalizes the pair, accepts and persists the OBS-WORK-GROUPED
/// observation, and re-derives. This only forwards the user's explicit
/// declaration that two witnessed resources belong to the same body of work.
pub fn submit_grouping(root: &Path, first: &str, second: &str) -> Result<GroupingResponse, String> {
    daemon_submit_grouping(root, first, second).map_err(|err| err.to_string())
}

/// The witnessed subjects of the Artifacts in a Workspace's current
/// continuation surface (restore-worthy ∪ unavailable), in canonical
/// ascending order.
///
/// This is exactly the set the user's latest OBS-CONTINUATION-SURFACE
/// declaration established for this Workspace (RFC-0013 per-Workspace
/// intersection). Presentation helper only — the canonical evidence remains
/// the persisted Observation; nothing here is canonical or derived state.
pub fn surface_subjects(
    selection: &RestorationSelection,
    subjects: &HashMap<String, String>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for selected in selection.restore_worthy() {
        if let Some(subject) = subjects.get(selected.artifact_id().as_str()) {
            out.push(subject.clone());
        }
    }
    for unavailable in selection.unavailable() {
        if let Some(subject) = subjects.get(unavailable.artifact_id().as_str()) {
            out.push(subject.clone());
        }
    }
    out.sort();
    out.dedup();
    out
}

/// A short human-readable description of one canonical Artifact: its
/// witnessed kind and subject, e.g. `focused window “Design Brief”` or
/// `saved file /Users/me/work/report.md`. Uses only canonical evidence;
/// never infers a kind the record does not establish.
pub fn describe_artifact(
    kinds: &HashMap<String, String>,
    subjects: &HashMap<String, String>,
    artifact_id: &str,
) -> String {
    let subject = subject_for(subjects, artifact_id)
        .map(str::to_string)
        .unwrap_or_else(|| artifact_id.to_string());
    match kind_for(kinds, artifact_id) {
        Some(KIND_WINDOW) => format!("focused window “{subject}”"),
        Some(KIND_FILE) => format!("saved file {subject}"),
        Some(KIND_URL) => format!("visited URL {subject}"),
        Some(KIND_COMMIT) => format!("commit {subject}"),
        _ => subject,
    }
}

/// Selects the Workspace to present, preferring the user's selection when it
/// is still remembered, otherwise falling back to the canonical display
/// selection (the most recently witnessed remembered Workspace).
pub fn select_workspace<'a>(
    workspaces: &'a [Workspace],
    selected: Option<&WorkspaceId>,
) -> Option<&'a Workspace> {
    match selected {
        Some(id) => workspaces.iter().find(|workspace| workspace.id() == id),
        None => select_display_workspace(workspaces),
    }
}

/// Selects the Workspace to present, preferring (in order) the user's
/// explicit selection, then the Workspace containing the current designation's
/// resolved Artifact (RFC-0011) when the user has not selected one, then the
/// canonical display selection.
///
/// The designation preference surfaces the user's own explicit statement of
/// where their work continues (RFC-0006 R-2/R-3: assistance that enables
/// continuation, minimal reconstruction cost) without ranking artifacts: the
/// Workspace is selected by the canonical designated Artifact's membership,
/// deterministically and from canonical evidence only. Presentation state
/// only — never a canonical model field.
pub fn select_workspace_with_designation<'a>(
    workspaces: &'a [Workspace],
    selected: Option<&WorkspaceId>,
    designated: Option<&ArtifactId>,
) -> Option<&'a Workspace> {
    if let Some(id) = selected {
        if let Some(workspace) = workspaces.iter().find(|workspace| workspace.id() == id) {
            return Some(workspace);
        }
    }
    if let Some(artifact) = designated {
        if let Some(workspace) = workspaces.iter().find(|workspace| {
            workspace
                .attachments()
                .iter()
                .any(|attachment| attachment.artifact_id() == artifact)
        }) {
            return Some(workspace);
        }
    }
    select_display_workspace(workspaces)
}

/// The desktop shell's top-level navigation state.
///
/// The primary Evo surface is the Workspace list (Home); a Workspace detail
/// view is entered by an explicit user selection and keyed by the canonical
/// Workspace identity. Transient UI state only — never canonical, never
/// persisted, never a model object (Law XVI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellView {
    /// The Workspace list — the primary Evo surface ("What bodies of work
    /// can I continue?").
    Home,
    /// One remembered Workspace, keyed by its canonical identity.
    Workspace(WorkspaceId),
}

impl ShellView {
    /// Derives the view from the user's selection.
    ///
    /// An explicit selection opens that Workspace while it is still
    /// remembered (canonical identity, never a title); a `None` selection
    /// shows Home; a selection whose Workspace is no longer remembered falls
    /// back to Home honestly — the shell must not pin itself to a forgotten
    /// body of work.
    pub fn from_selection(selected: Option<&WorkspaceId>, workspaces: &[Workspace]) -> Self {
        match selected {
            Some(id) if workspaces.iter().any(|workspace| workspace.id() == id) => {
                ShellView::Workspace(id.clone())
            }
            _ => ShellView::Home,
        }
    }
}

/// Neutral human-readable label for a Workspace.
///
/// Plain-language label for a canonical Workspace/Snapshot lifecycle.
///
/// Presentation constant derived from the canonical lifecycle value; never a
/// canonical model field. Avoids exposing raw enum debug output in the UX.
pub fn lifecycle_label(lifecycle: &evo_workspace::lifecycle::WorkspaceLifecycle) -> &'static str {
    match lifecycle {
        evo_workspace::lifecycle::WorkspaceLifecycle::Active => "Active",
        evo_workspace::lifecycle::WorkspaceLifecycle::Superseded => "Superseded",
    }
}

/// A short, secondary identity snippet for display.
pub fn short_identity(value: &str) -> String {
    value.chars().take(8).collect()
}

/// Display summary of the canonical Artifacts connected to a Workspace.
///
/// Presents the connected Artifact identities, shortened and deduplicated for
/// display only. The attachment count is reported separately and truthfully.
pub fn artifact_ids(attachments: &[Attachment]) -> String {
    let mut seen = BTreeSet::new();
    let mut ids = Vec::new();
    for attachment in attachments {
        let id = attachment.artifact_id().as_str();
        if seen.insert(id.to_string()) {
            ids.push(short_artifact_id(id));
        }
    }
    if ids.is_empty() {
        "none".to_string()
    } else {
        ids.join(", ")
    }
}

/// Short display form of a canonical Artifact identity.
pub fn short_artifact_id(value: &str) -> String {
    value
        .strip_prefix("artifact-")
        .map(|hex| hex.chars().take(8).collect())
        .unwrap_or_else(|| short_identity(value))
}

/// The human-readable subject a canonical Artifact's Observations witnessed,
/// when the persisted Observation log establishes one.
pub fn subject_for<'a>(
    subjects: &'a HashMap<String, String>,
    artifact_id: &str,
) -> Option<&'a str> {
    subjects.get(artifact_id).map(String::as_str)
}

/// Human-readable label for one Artifact: the witnessed subject when the
/// canonical Observation log establishes one, otherwise the short identity.
pub fn artifact_label<'a>(subjects: &'a HashMap<String, String>, artifact_id: &str) -> String {
    subject_for(subjects, artifact_id)
        .map(str::to_string)
        .unwrap_or_else(|| short_artifact_id(artifact_id))
}

/// The distinct Artifact identities represented across every canonical
/// Snapshot in a Workspace's history, in canonical snapshot order.
pub fn distinct_artifacts_across_history(workspace: &Workspace) -> Vec<String> {
    let mut distinct: Vec<String> = Vec::new();
    for snapshot in workspace.snapshots() {
        for attachment in snapshot.attachments() {
            let id = attachment.artifact_id().to_string();
            if !distinct.contains(&id) {
                distinct.push(id);
            }
        }
    }
    distinct
}

/// The distinct Artifact identities represented by the latest canonical
/// Snapshot, in canonical order. Empty when no Snapshot exists yet.
pub fn distinct_artifacts_in_latest_snapshot(workspace: &Workspace) -> Vec<String> {
    workspace
        .snapshots()
        .last()
        .map(|snapshot| distinct_ids(snapshot.attachments()))
        .unwrap_or_default()
}

/// A compact human-readable summary of the artifacts a Workspace's canonical
/// history witnessed, for list rows: the first two kind-aware descriptions
/// plus a count of any remainder. The label never claims a witness kind the
/// canonical record does not support.
pub fn workspace_surface_label(
    subjects: &HashMap<String, String>,
    kinds: &HashMap<String, String>,
    workspace: &Workspace,
) -> String {
    let ids = distinct_artifacts_across_history(workspace);
    if ids.is_empty() {
        return "Nothing witnessed yet".to_string();
    }
    let labels: Vec<String> = ids
        .iter()
        .map(|id| describe_artifact(kinds, subjects, id))
        .collect();
    if labels.len() <= 2 {
        labels.join(", ")
    } else {
        format!("{}, … +{} more", labels[..2].join(", "), labels.len() - 2)
    }
}

/// The searchable plain-language text for one remembered body of work:
/// its surface summary plus every Artifact it witnessed. Presentation-only
/// retrieval text — matching it never establishes meaning, ranking, or
/// continuation evidence. Order is canonical, never recency-ranked.
pub fn workspace_search_text(
    subjects: &HashMap<String, String>,
    kinds: &HashMap<String, String>,
    workspace: &Workspace,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(workspace_surface_label(subjects, kinds, workspace));
    for id in distinct_artifacts_across_history(workspace) {
        parts.push(describe_artifact(kinds, subjects, &id));
    }
    parts.join(" ")
}

/// Presentation material for one Workspace on the home surface.
///
/// Everything here is derived from canonical Workspace / derived-outcome /
/// derived-selection state at reload. Transient UI state only — never
/// canonical, never persisted, never a model object (Law XVI). The Workspace
/// identity is canonical; the title is presentation only.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct WorkspaceCard {
    /// The canonical Workspace identity this card opens.
    pub id: WorkspaceId,
    /// The presentation title: what the body of work is *about*, as derived by
    /// the Engagement layer from the vocabulary its own members share. Falls
    /// back to the Resume Point's witnessed subject, then to the first
    /// witnessed resource in canonical order, then to a neutral placeholder —
    /// each a witnessed name, never invented wording, and never a recency,
    /// frequency, application, or profession heuristic.
    pub title: String,
    /// A short canonical summary of what this body of work is.
    pub description: String,
    /// The cognitive entry point (Resume Point) in plain language, when
    /// canonical evidence established it.
    pub resume_from: Option<String>,
    /// The immediate continuation (Next Step), when canonical evidence
    /// established it. Not an automation command.
    pub next_step: Option<String>,
    /// How many resources belong to this Workspace's current continuation
    /// (the derived selection's surface members). Zero when no surface is
    /// declared or none intersects this Workspace.
    pub continuation_count: usize,
    /// Whether canonical evidence has established where this work continues.
    pub continuable: bool,
    /// How much Evo is prepared to claim of this body of work: whether Home
    /// presents it as work at all, and whether it may be auto-opened. Decided
    /// by the Engagement layer from the witnessed relationship graph alone.
    /// A `Standing::Remembered` body is kept and findable by name, but is not
    /// presented on Home as work (WORK-MODEL). Never derived from an
    /// application, domain, file type, or category.
    pub standing: Standing,
}

/// Derives the presentation material for one Workspace home card from
/// canonical inputs only: the Workspace's durable membership, its derived
/// Restoration outcome, its derived selective-restoration selection, the
/// witnessed subjects/kinds, and the derived name of the body of work.
///
/// The card is honest about insufficiency: a Workspace with no established
/// Resume Point is presented as such, never guessed. This is presentation
/// derivation only — it never changes Workspace membership, Artifact
/// identity, or continuation semantics.
pub fn workspace_card(
    workspace: &Workspace,
    outcome: Option<&DerivationOutcome>,
    selection: Option<&RestorationSelection>,
    subjects: &HashMap<String, String>,
    kinds: &HashMap<String, String>,
    titles: &HashMap<String, String>,
    standings: &HashMap<String, Standing>,
) -> WorkspaceCard {
    let id = workspace.id().clone();
    let resume_from = outcome
        .and_then(DerivationOutcome::resume_point)
        .map(|resume_point| {
            describe_artifact(kinds, subjects, resume_point.artifact_id().as_str())
        });
    // Home names the body of work, not a window inside it. The derived title
    // comes from the layer that measured what this work's members have in
    // common; the Resume Point's subject and the first witnessed resource remain
    // as honest fallbacks when no title was derived. Deterministic,
    // evidence-derived, never recency-ranked. See
    // `evo_daemon::understanding::Understanding::titles` for the authority
    // change this rule replaced.
    let title = titles
        .get(&id.to_string())
        .map(String::as_str)
        .filter(|title| !title.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            outcome
                .and_then(DerivationOutcome::resume_point)
                .and_then(|resume_point| subject_for(subjects, resume_point.artifact_id().as_str()))
                .map(str::to_string)
        })
        .or_else(|| {
            distinct_artifacts_across_history(workspace)
                .first()
                .and_then(|id| subject_for(subjects, id).map(str::to_string))
        })
        .unwrap_or_else(|| "Untitled work".to_string());
    let description = workspace_surface_label(subjects, kinds, workspace);
    let next_step = outcome
        .and_then(DerivationOutcome::next_step)
        .map(|next| next.description().to_string());
    let continuation_count = match selection {
        Some(selection) => selection.restore_worthy().len() + selection.unavailable().len(),
        None => 0,
    };
    let continuable = outcome.is_some_and(|outcome| outcome.resume_point().is_some());
    // The standing the Engagement layer measured for this body of work. Absent
    // only when the derived index carries no standing for it — then Evo makes
    // the least claim it can, `Remembered`: kept and findable, never presented
    // as work on evidence it does not have. Never fabricate work by default.
    let standing = standings
        .get(&id.to_string())
        .copied()
        .unwrap_or(Standing::Remembered);
    WorkspaceCard {
        id,
        title,
        description,
        resume_from,
        next_step,
        continuation_count,
        continuable,
        standing,
    }
}

/// Presentation-only filter over the kind of resources a body of work
/// witnessed. Filtering is a retrieval view, never continuation evidence and
/// never a ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkKindFilter {
    #[default]
    All,
    Files,
    Windows,
    Urls,
    Commits,
}

impl WorkKindFilter {
    /// The plain-language chip label.
    pub fn label(self) -> &'static str {
        match self {
            WorkKindFilter::All => "All work",
            WorkKindFilter::Files => "Files",
            WorkKindFilter::Windows => "Windows",
            WorkKindFilter::Urls => "URLs",
            WorkKindFilter::Commits => "Commits",
        }
    }

    /// All filters in display order (All first).
    pub fn all() -> [WorkKindFilter; 5] {
        [
            WorkKindFilter::All,
            WorkKindFilter::Files,
            WorkKindFilter::Windows,
            WorkKindFilter::Urls,
            WorkKindFilter::Commits,
        ]
    }
}

/// Presentation-only time filter over when a body of work was last witnessed,
/// using the factual canonical snapshot timestamps. Filtering is a retrieval
/// view, never continuation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkPeriodFilter {
    #[default]
    All,
    Today,
    ThisWeek,
    Older,
}

impl WorkPeriodFilter {
    /// The plain-language chip label.
    pub fn label(self) -> &'static str {
        match self {
            WorkPeriodFilter::All => "Any time",
            WorkPeriodFilter::Today => "Today",
            WorkPeriodFilter::ThisWeek => "This week",
            WorkPeriodFilter::Older => "Older",
        }
    }

    /// All filters in display order (All first).
    pub fn all() -> [WorkPeriodFilter; 4] {
        [
            WorkPeriodFilter::All,
            WorkPeriodFilter::Today,
            WorkPeriodFilter::ThisWeek,
            WorkPeriodFilter::Older,
        ]
    }
}

/// Whether a body of work witnessed at least one resource of the requested
/// kind. A workspace with no witnessed artifacts matches only `All`.
pub fn workspace_matches_kind(
    kinds: &HashMap<String, String>,
    workspace: &Workspace,
    filter: WorkKindFilter,
) -> bool {
    if filter == WorkKindFilter::All {
        return true;
    }
    let wanted: &str = match filter {
        WorkKindFilter::Files => KIND_FILE,
        WorkKindFilter::Windows => KIND_WINDOW,
        WorkKindFilter::Urls => KIND_URL,
        WorkKindFilter::Commits => KIND_COMMIT,
        WorkKindFilter::All => return true,
    };
    distinct_artifacts_across_history(workspace)
        .iter()
        .any(|id| kinds.get(id).map(String::as_str) == Some(wanted))
}

/// Whether a body of work was last witnessed within the requested period,
/// judged against the canonical latest snapshot instant and `now`. A
/// workspace with no snapshot matches only `All` (nothing factual to filter
/// on).
pub fn workspace_matches_period(
    workspace: &Workspace,
    filter: WorkPeriodFilter,
    now: std::time::SystemTime,
) -> bool {
    if filter == WorkPeriodFilter::All {
        return true;
    }
    let Some(latest) = workspace.snapshots().last() else {
        return false;
    };
    let elapsed = match now.duration_since(*latest.captured_at()) {
        Ok(elapsed) => elapsed,
        // A witness moment in the future (clock skew) counts as today.
        Err(_) => return filter == WorkPeriodFilter::Today,
    };
    match filter {
        WorkPeriodFilter::Today => elapsed < std::time::Duration::from_secs(24 * 60 * 60),
        WorkPeriodFilter::ThisWeek => elapsed < std::time::Duration::from_secs(7 * 24 * 60 * 60),
        WorkPeriodFilter::Older => elapsed >= std::time::Duration::from_secs(7 * 24 * 60 * 60),
        WorkPeriodFilter::All => true,
    }
}

fn distinct_ids(attachments: &[Attachment]) -> Vec<String> {
    let mut distinct: Vec<String> = Vec::new();
    for attachment in attachments {
        let id = attachment.artifact_id().to_string();
        if !distinct.contains(&id) {
            distinct.push(id);
        }
    }
    distinct
}

/// Decides the ordered restoration steps for the user's explicit Continue
/// request — what will be attempted, for which Artifact, in which order —
/// without performing any of them.
///
/// The shell never decides restoration semantics: it builds the canonical
/// execution request from the derived outcome, resolves each Artifact's
/// executable target from canonical evidence, and reflects whatever the
/// boundary reports — including an explicit refusal or an honest
/// unavailable/ambiguous outcome.
///
/// Separating the plan from the act is what lets the surface show honest
/// per-item progress: the rows are the steps, and a row only gains a result
/// once [`evo_execution::execute_step`] has actually returned one for it.
///
/// What will be attempted, and in what order, is decided by exactly one
/// shared function — `evo_execution::ordered_attempt_targets` — which the
/// plan announcement (`plan_line`) consumes too. The promise and the act
/// therefore cannot describe different target sets or orders
/// (BE-AUDIT-0001 §2.4; UI-VALIDATION-0001 B.9).
pub fn plan_execution<P: evo_execution::PreflightSource>(
    outcome: Option<&DerivationOutcome>,
    selection: Option<&RestorationSelection>,
    locators: &HashMap<String, Locator>,
    preflight: &P,
) -> Vec<RestorationStep> {
    // One shared decision: the ordered attempt set, or an honest refusal.
    // No derived outcome, no derived Resume Point, and a declared surface
    // without a derived selection all refuse here — with the same reason the
    // announcement sees — instead of each branch inventing its own.
    let announced = match evo_execution::ordered_attempt_targets(outcome, selection) {
        Ok(targets) => targets,
        Err(err) => {
            return vec![RestorationStep::new(
                ArtifactId::new("none").expect("static artifact id"),
                StepPlan::Refuse(evo_execution::TargetStatus::Unavailable {
                    reason: err.to_string(),
                }),
            )];
        }
    };
    let outcome = outcome.expect("ordered_attempt_targets refuses without an outcome");

    // Once the user has declared a continuation surface (RFC-0013), Continue
    // acts on exactly its restore-worthy members — the derived selection —
    // and never on anything else: no recency, no frequency, no application
    // identity, no guessed targets. A declared surface whose members cannot
    // be reopened produces an honest empty plan.
    let steps = if !outcome.continuation_surface().is_empty() {
        let selection = selection
            .expect("ordered_attempt_targets refuses a declared surface without a selection");
        plan_selection(selection, locators, preflight)
    } else {
        // No declared surface: the designation-derived Resume Point (RFC-0011)
        // remains the execution authority — exactly the one target the user
        // marked, never a guess from recent activity. The shared function
        // already validated this derivation, so it cannot fail here.
        let request = ExecutionRequest::from_derivation(outcome)
            .expect("ordered_attempt_targets validated this derivation");
        plan_request(&request, locators, preflight)
    };

    // Fidelity guard: the plan must attempt the announced targets in the
    // announced order. A plan also carries honest per-member steps for
    // unavailable members; those are never announced as openable, only
    // reported.
    let attempted: Vec<ArtifactId> = steps
        .iter()
        .filter(|step| announced.contains(step.artifact_id()))
        .map(|step| step.artifact_id().clone())
        .collect();
    debug_assert_eq!(
        announced, attempted,
        "announced plan must equal planned order"
    );
    steps
}

/// Routes the user's explicit Resume request through the domain Restoration
/// Execution boundary and returns the structured result for presentation.
///
/// This is [`plan_execution`] followed by acting on every step in order. It
/// exists for callers that want the whole report at once; the surface drives
/// the same steps one at a time so it can report each result as it arrives.
pub fn run_execution<
    E: evo_execution::engine::PlatformExecutor + evo_execution::PreflightSource,
>(
    outcome: Option<&DerivationOutcome>,
    selection: Option<&RestorationSelection>,
    locators: &HashMap<String, Locator>,
    executor: &E,
) -> ExecutionReport {
    let steps = plan_execution(outcome, selection, locators, executor);
    ExecutionReport::new(
        steps
            .iter()
            .map(|step| execute_step(step, executor, executor))
            .collect(),
    )
}

/// Honest presentation line for a structured execution report.
///
/// Unavailable/ambiguous outcomes present the executor's own reason verbatim;
/// successful attempts report each opened target. Nothing here ever claims a
/// restoration occurred unless the executor reported it.
pub fn execution_result_line(report: &ExecutionReport) -> String {
    let attempts = report.attempts();
    if attempts.is_empty() {
        return "Nothing was executed: the record referenced no resource.".to_string();
    }
    if report.all_opened() {
        format!(
            "Evo opened {} resource(s); every attempt reported explicitly.",
            attempts.len()
        )
    } else {
        let opened = attempts
            .iter()
            .filter(|attempt| {
                matches!(attempt.status(), evo_execution::TargetStatus::Opened { .. })
            })
            .count();
        if opened == 0 {
            "Evo could not open any resource; every attempt reported explicitly.".to_string()
        } else {
            format!(
                "Evo opened {opened} of {} resource(s); every attempt reported explicitly.",
                attempts.len()
            )
        }
    }
}

/// Label for one Snapshot's canonical creation point, in local wall-clock
/// time. Presentation formatting only; the canonical instant is untouched.
pub fn snapshot_time_label(snapshot: &Snapshot) -> String {
    local_time_label(*snapshot.captured_at())
}

/// Formats a canonical instant as local wall-clock time.
pub fn local_time_label(time: std::time::SystemTime) -> String {
    let epoch_secs = match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(err) => -(err.duration().as_secs() as i64),
    };
    format_local(epoch_secs)
}

/// A compact relative time label for a canonical instant, e.g. `just now`,
/// `5 minutes ago`, `2 hours ago`, `3 days ago`. Presentation formatting
/// only; the canonical instant is untouched. Never used as continuation
/// evidence — it is a factual statement of when a moment was witnessed.
pub fn relative_time_label(time: std::time::SystemTime) -> String {
    let elapsed = match std::time::SystemTime::now().duration_since(time) {
        Ok(duration) => duration,
        // A witness moment in the future (clock skew) is still "just now".
        Err(_) => return "just now".to_string(),
    };
    let seconds = elapsed.as_secs();
    if seconds < 60 {
        "just now".to_string()
    } else if seconds < 3600 {
        plural_ago(seconds / 60, "minute")
    } else if seconds < 86400 {
        plural_ago(seconds / 3600, "hour")
    } else {
        plural_ago(seconds / 86400, "day")
    }
}

fn plural_ago(count: u64, unit: &str) -> String {
    if count == 1 {
        format!("1 {unit} ago")
    } else {
        format!("{count} {unit}s ago")
    }
}

#[cfg(unix)]
fn format_local(epoch_secs: i64) -> String {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Tm {
        tm_sec: i32,
        tm_min: i32,
        tm_hour: i32,
        tm_mday: i32,
        tm_mon: i32,
        tm_year: i32,
        tm_wday: i32,
        tm_yday: i32,
        tm_isdst: i32,
        tm_gmtoff: i64,
        tm_zone: *const std::os::raw::c_char,
    }

    unsafe extern "C" {
        fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
    }

    let mut tm: Tm = unsafe { std::mem::zeroed() };
    let ptr = unsafe { localtime_r(&epoch_secs, &mut tm) };
    if ptr.is_null() {
        return "unknown".to_string();
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min
    )
}

#[cfg(not(unix))]
fn format_local(_epoch_secs: i64) -> String {
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use evo_artifact::artifact_id::ArtifactId;
    use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
    use evo_daemon::persistence::persist_observation;
    use evo_observation::provenance::ObservationSource;
    use evo_storage::Storage;
    use evo_workspace::attachment::{Attachment, ResourceRole};
    use evo_workspace::confidence::ConfidenceScore;
    use evo_workspace::lifecycle::WorkspaceLifecycle;
    use evo_workspace::snapshot::Snapshot;
    use evo_workspace::workspace_id::WorkspaceId;

    use std::str::FromStr;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-desktop-{label}-{nanos}"))
    }

    fn attachment(id: &str) -> Attachment {
        Attachment::new(
            ArtifactId::new(id).unwrap(),
            ConfidenceScore::new(1.0).unwrap(),
            // Search, filter, and ordering fixtures read the Attachment Set
            // without regard to role; Primary is the plain "the work happens
            // here" role.
            ResourceRole::Primary,
        )
    }

    fn t(secs: u64) -> SystemTime {
        UNIX_EPOCH
            .checked_add(Duration::from_secs(secs))
            .expect("representable fixture time")
    }

    fn focus(subject: &str, at: u64) -> MacOSSignal {
        MacOSSignal::WindowFocusGained {
            subject: subject.to_string(),
            observed_at: t(at),
            process_identifier: None,
            owning_process_name: None,
            observed_state: None,
        }
    }

    fn saved(subject: &str, at: u64) -> MacOSSignal {
        MacOSSignal::FileSaved {
            subject: subject.to_string(),
            observed_at: t(at),
        }
    }

    /// The daemon's own evidence path, minus the worker thread: real capture
    /// normalization against the frozen schemas, real Observation acceptance,
    /// real append-only persistence.
    ///
    /// Tests use this instead of seeding derived state. Nothing about the
    /// understanding is handed to the desktop: every Workspace, role, outcome
    /// and selection these tests assert on was derived from the Observations
    /// witnessed here, so what they prove is what the shipped desktop shows for
    /// that evidence.
    struct Witness {
        accessibility: MacOSAdapter,
        filesystem: MacOSAdapter,
        declaration: MacOSAdapter,
        engine: CaptureEngine,
    }

    impl Witness {
        fn new() -> Self {
            Self {
                accessibility: MacOSAdapter::new(
                    ObservationSource::new("macos_accessibility").unwrap(),
                ),
                filesystem: MacOSAdapter::new(ObservationSource::new("macos_fsevents").unwrap()),
                declaration: MacOSAdapter::new(
                    ObservationSource::new("user_continuation").unwrap(),
                ),
                engine: CaptureEngine::new(),
            }
        }

        /// Normalizes one platform signal against its frozen schema, accepts it,
        /// and appends it to the canonical Observation log. Must be called under
        /// a [`Storage::with_thread_root`] guard.
        fn record(&mut self, signal: MacOSSignal) {
            let adapter = match &signal {
                MacOSSignal::FileSaved { .. } => &self.filesystem,
                MacOSSignal::ContinuationSurface { .. } => &self.declaration,
                _ => &self.accessibility,
            };
            let raw = adapter
                .normalize(signal)
                .expect("a supported signal normalizes")
                .expect("a supported signal maps to a canonical concept");
            let schema = raw.schema();
            let observation = self
                .engine
                .ingest(raw, &schema)
                .expect("a normalized event is acceptable against its own schema");
            persist_observation(&observation).expect("the canonical log accepts the Observation");
        }
    }

    /// Witnesses one coherent body of work: a document window, the file it
    /// saves, and a terminal, used together in repeated passes across two
    /// sittings four hours apart.
    ///
    /// Relationship, return and depth all come from this evidence. No
    /// application, domain, or category is named anywhere — the three resources
    /// are one body of work because they were used together and returned to.
    fn witness_one_body_of_work(witness: &mut Witness, window: &str, file: &str, terminal: &str) {
        let mut at = 100u64;
        for _sitting in 0..2 {
            for _pass in 0..4 {
                witness.record(focus(window, at));
                at += 90;
                witness.record(saved(file, at));
                at += 90;
                witness.record(focus(terminal, at));
                at += 90;
            }
            // Hours away, then back: the return that makes this worth resuming.
            at += 4 * 60 * 60;
        }
    }

    /// A handful of unrelated one-off sightings, recorded so that the machine's
    /// witnessed vocabulary is wider than the single task under test.
    ///
    /// Promoting a resource Evo only ever saw *change* requires the change to be
    /// corroborated by shared wording with something attended, and shared wording
    /// is weighted by how much information it carries across everything witnessed.
    /// In a three-resource universe every token is carried by every subject, so no
    /// name can be distinctive, nothing can be corroborated, and a file the person
    /// saved eight times is reported as merely present. That is a real and
    /// deliberate outcome — `evo_engagement`'s
    /// `a_change_with_no_evidence_but_timing_is_reported_as_merely_present` pins
    /// it — but it is not the case these end-to-end tests are about.
    ///
    /// Each subject here is seen exactly once, hours from anything else, so none of
    /// them satisfies Return or Depth and none becomes a body of work. They widen
    /// the corpus and nothing else.
    fn witness_unrelated_sightings(witness: &mut Witness, from: u64) {
        let mut at = from;
        for subject in [
            "Hendricks Deposition — Reader",
            "hendricks deposition markup — Notes",
            "Kiln Schedule — Notes",
            "Album Layout — Photos",
        ] {
            witness.record(focus(subject, at));
            at += 6 * 60 * 60;
        }
        for path in [
            "/Users/alice/cases/hendricks/deposition.pdf",
            "/Users/alice/pottery/kiln-schedule.md",
        ] {
            witness.record(saved(path, at));
            at += 6 * 60 * 60;
        }
    }

    /// The canonical Artifact whose Observations witnessed exactly this
    /// subject. Resolved through the display state the production path built,
    /// so a test can only name an Artifact the log actually established.
    fn artifact_for(state: &DisplayState, subject: &str) -> ArtifactId {
        let matches: Vec<&String> = state
            .subjects
            .iter()
            .filter(|(_, witnessed)| witnessed.as_str() == subject)
            .map(|(id, _)| id)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "exactly one Artifact should carry the subject {subject:?}, found {matches:?}"
        );
        ArtifactId::new(matches[0]).expect("a witnessed Artifact identity is valid")
    }

    fn snapshot(offset_secs: u64, attachments: Vec<Attachment>) -> Snapshot {
        Snapshot::new(
            UNIX_EPOCH
                .checked_add(Duration::from_secs(offset_secs))
                .unwrap(),
            WorkspaceLifecycle::Active,
            attachments,
        )
    }

    fn workspace(id: &str, snapshots: Vec<Snapshot>) -> Workspace {
        let attachments = snapshots
            .last()
            .expect("workspace fixture needs at least one snapshot")
            .attachments()
            .to_vec();
        Workspace::new(
            WorkspaceId::from_str(id).unwrap(),
            WorkspaceLifecycle::Active,
            attachments,
            snapshots,
        )
    }

    #[test]
    fn empty_storage_has_no_display_workspace() {
        let state = load_display_state(&unique_root("empty")).unwrap();
        assert!(state.display.is_none());
        assert!(state.workspaces.is_empty());
        assert!(state.outcome.is_none());
        assert!(state.designation.is_none());
    }

    // The current designation (RFC-0011) is derived from the persisted
    // Observation log and carried for presentation: the subject the user
    // marked and the moment it was witnessed.
    #[test]
    fn display_state_carries_the_current_designation_from_the_log() {
        use evo_daemon::persistence::persist_observation;
        use evo_observation::evidence::{Evidence, FactValue, ObservedFact};
        use evo_observation::observation::Observation;
        use evo_observation::observation_id::ObservationId;
        use evo_observation::observation_schema::ObservationSchema;
        use evo_observation::provenance::{ObservationSource, Provenance};

        let root = unique_root("designation-state");
        let older = UNIX_EPOCH.checked_add(Duration::from_secs(5)).unwrap();
        let newer = UNIX_EPOCH.checked_add(Duration::from_secs(60)).unwrap();
        {
            let _guard = Storage::with_thread_root(root.clone());
            for (observed_at, subject) in [(older, "/tmp/old.md"), (newer, "/tmp/plan.md")] {
                let observation = Observation::new(
                    ObservationId::new(),
                    ObservationSchema::work_designated_v1(),
                    Provenance::new(
                        ObservationSource::new("user_designation").unwrap(),
                        observed_at,
                        HashMap::new(),
                    ),
                    Evidence::new(vec![
                        ObservedFact::new("WorkDesignated", FactValue::Text(subject.into()))
                            .unwrap(),
                    ]),
                );
                persist_observation(&observation).unwrap();
            }
        }

        let state = load_display_state(&root).unwrap();
        let (subject, time) = state.designation.expect("current designation is present");
        // Supersession: the later designation is current.
        assert_eq!(subject, "/tmp/plan.md");
        assert_eq!(time, newer);
    }

    // AMENDED RULE. This test previously seeded a hand-built Workspace through
    // `persist_workspace_state` and asserted the desktop loaded it back. There
    // is no such store any more: understanding is derived from the Observation
    // log, so the test now witnesses evidence and asserts what the desktop
    // derives from it.
    #[test]
    fn a_body_of_work_is_derived_from_the_log_and_selected_for_display() {
        let root = unique_root("derived-display");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let mut witness = Witness::new();
            witness_one_body_of_work(
                &mut witness,
                "Migration Plan — Editor",
                "/Users/alice/ops/migration-plan.md",
                "migration plan rollout — Terminal",
            );
        }

        let state = load_display_state(&root).unwrap();
        let display = state
            .display
            .expect("a coherent body of work is worth returning to");
        // Three resources used together became one body of work, not three.
        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(display.attachments().len(), 3);
        // One Snapshot per sitting: the work was left and returned to once.
        assert_eq!(display.snapshots().len(), 2);
    }

    // The searchable text is plain-language retrieval material built from
    // canonical subjects and kinds — it never establishes meaning, and it
    // includes every witnessed artifact so old work is findable by any of
    // its resources.
    #[test]
    fn workspace_search_text_covers_surface_and_every_artifact() {
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174000",
            vec![snapshot(
                0,
                vec![attachment("artifact-aa"), attachment("artifact-bb")],
            )],
        );
        let mut subjects = HashMap::new();
        subjects.insert("artifact-aa".to_string(), "/tmp/report.md".to_string());
        subjects.insert("artifact-bb".to_string(), "Notes".to_string());
        let mut kinds = HashMap::new();
        kinds.insert("artifact-aa".to_string(), KIND_FILE.to_string());
        kinds.insert("artifact-bb".to_string(), KIND_WINDOW.to_string());

        let text = workspace_search_text(&subjects, &kinds, &ws);
        assert!(text.contains("/tmp/report.md"), "search text: {text}");
        assert!(text.contains("Notes"), "search text: {text}");
        assert!(text.contains(KIND_FILE), "search text: {text}");
        assert!(text.contains(KIND_WINDOW), "search text: {text}");
    }

    // Empty search text on a workspace with no witnessed artifacts is honest
    // and findable as such.
    #[test]
    fn workspace_search_text_is_honest_when_nothing_was_witnessed() {
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174000",
            vec![snapshot(0, vec![])],
        );
        let text = workspace_search_text(&HashMap::new(), &HashMap::new(), &ws);
        assert!(
            text.contains("Nothing witnessed yet"),
            "search text: {text}"
        );
    }

    // The kind filter matches a workspace only when it actually witnessed a
    // resource of that kind; a workspace with no artifacts matches only All.
    #[test]
    fn workspace_matches_kind_is_evidence_based() {
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174000",
            vec![snapshot(
                0,
                vec![attachment("artifact-aa"), attachment("artifact-bb")],
            )],
        );
        let mut kinds = HashMap::new();
        kinds.insert("artifact-aa".to_string(), KIND_FILE.to_string());
        kinds.insert("artifact-bb".to_string(), KIND_WINDOW.to_string());

        assert!(workspace_matches_kind(&kinds, &ws, WorkKindFilter::All));
        assert!(workspace_matches_kind(&kinds, &ws, WorkKindFilter::Files));
        assert!(workspace_matches_kind(&kinds, &ws, WorkKindFilter::Windows));
        assert!(!workspace_matches_kind(&kinds, &ws, WorkKindFilter::Urls));
        assert!(!workspace_matches_kind(
            &kinds,
            &ws,
            WorkKindFilter::Commits
        ));

        // A workspace that witnessed nothing matches only All: filtering by
        // any kind must not invent evidence.
        let empty = workspace(
            "123e4567-e89b-12d3-a456-426614174001",
            vec![snapshot(0, vec![])],
        );
        assert!(workspace_matches_kind(&kinds, &empty, WorkKindFilter::All));
        assert!(!workspace_matches_kind(
            &kinds,
            &empty,
            WorkKindFilter::Files
        ));
    }

    // The period filter judges only the factual latest witness instant; a
    // workspace with no snapshot matches only All because there is nothing
    // factual to filter on.
    #[test]
    fn workspace_matches_period_uses_only_the_latest_witness_instant() {
        // "Now" sits 100 days after the epoch; the fixtures below are placed
        // relative to it so every elapsed-time branch is exercised.
        let day = 24 * 60 * 60;
        let now = UNIX_EPOCH
            .checked_add(Duration::from_secs(100 * day))
            .unwrap();
        let at = |offset_secs: u64, id: &str| {
            workspace(
                id,
                vec![snapshot(offset_secs, vec![attachment("artifact-aa")])],
            )
        };
        let today = at(
            99 * day + 21 * 60 * 60,
            "123e4567-e89b-12d3-a456-426614174010",
        ); // ~3h ago
        let this_week = at(96 * day, "123e4567-e89b-12d3-a456-426614174011"); // ~4 days ago
        let older = at(60 * day, "123e4567-e89b-12d3-a456-426614174012"); // ~40 days ago
        let no_snapshot = workspace(
            "123e4567-e89b-12d3-a456-426614174013",
            vec![snapshot(0, vec![])],
        );

        assert!(workspace_matches_period(&today, WorkPeriodFilter::All, now));
        assert!(workspace_matches_period(
            &today,
            WorkPeriodFilter::Today,
            now
        ));
        assert!(workspace_matches_period(
            &today,
            WorkPeriodFilter::ThisWeek,
            now
        ));
        assert!(!workspace_matches_period(
            &today,
            WorkPeriodFilter::Older,
            now
        ));

        assert!(!workspace_matches_period(
            &this_week,
            WorkPeriodFilter::Today,
            now
        ));
        assert!(workspace_matches_period(
            &this_week,
            WorkPeriodFilter::ThisWeek,
            now
        ));
        assert!(!workspace_matches_period(
            &this_week,
            WorkPeriodFilter::Older,
            now
        ));

        assert!(!workspace_matches_period(
            &older,
            WorkPeriodFilter::Today,
            now
        ));
        assert!(!workspace_matches_period(
            &older,
            WorkPeriodFilter::ThisWeek,
            now
        ));
        assert!(workspace_matches_period(
            &older,
            WorkPeriodFilter::Older,
            now
        ));

        // Nothing factual to judge: only All matches.
        assert!(workspace_matches_period(
            &no_snapshot,
            WorkPeriodFilter::All,
            now
        ));
        assert!(!workspace_matches_period(
            &no_snapshot,
            WorkPeriodFilter::Today,
            now
        ));

        // A witness instant after "now" (clock skew) counts as today.
        let future = workspace(
            "123e4567-e89b-12d3-a456-426614174014",
            vec![snapshot(101 * day, vec![attachment("artifact-aa")])],
        );
        assert!(workspace_matches_period(
            &future,
            WorkPeriodFilter::Today,
            now
        ));
    }

    // End-to-end: canonical Observation → Artifact identity → derived
    // Workspace → desktop state. The display state must carry the
    // human-readable subject each canonical Artifact's Observations witnessed,
    // derived from the persisted Observation log.
    //
    // The window title here is the exact one from the reported failure. It is
    // used deliberately: it must reach the display when, and only when, the
    // Observation log witnessed it. No production path may produce it
    // otherwise (§17).
    #[test]
    fn display_state_carries_human_readable_subjects_from_the_observation_log() {
        let root = unique_root("subjects-e2e");
        let subject = "Design Brief — Canvas";
        {
            let _guard = Storage::with_thread_root(root.clone());
            let mut witness = Witness::new();
            witness_one_body_of_work(
                &mut witness,
                subject,
                "/Users/alice/design/brief.md",
                "design brief export — Terminal",
            );
        }

        let state = load_display_state(&root).unwrap();
        assert!(
            state.display.is_some(),
            "a coherent body of work is displayed"
        );
        // The display state carries the witnessed subject for the Artifact the
        // log established for it.
        let artifact_id = artifact_for(&state, subject);
        assert_eq!(
            subject_for(&state.subjects, artifact_id.as_str()),
            Some(subject)
        );
    }

    // End-to-end: canonical evidence → derived Workspace → Snapshot →
    // Restoration Derivation → desktop state. Verifies the actual derived
    // component values and their relationships, not merely that Some(...) was
    // produced.
    #[test]
    fn desktop_receives_actual_derived_restoration_information() {
        let root = unique_root("restoration-e2e");
        let window = "Quarterly Forecast — Spreadsheet";
        let file = "/Users/alice/finance/quarterly-forecast.numbers";
        {
            let _guard = Storage::with_thread_root(root.clone());
            let mut witness = Witness::new();
            witness_one_body_of_work(
                &mut witness,
                window,
                file,
                "quarterly forecast reconcile — Terminal",
            );
            // The rest of this person's week, so that "quarterly" and "forecast"
            // are words that mean something on this machine.
            witness_unrelated_sightings(&mut witness, 200_000);
        }

        let state = load_display_state(&root).unwrap();
        let display = state
            .display
            .clone()
            .expect("a coherent body of work is displayed");
        let outcome = state
            .outcome
            .clone()
            .expect("the derived restoration understanding reaches the desktop");

        // Component values and relationships.
        assert_eq!(outcome.workspace_id(), display.id());
        let resume_point = outcome
            .resume_point()
            .expect("the Continuation role establishes the Resume Point");
        assert_eq!(resume_point.workspace_id(), outcome.workspace_id());
        // The Resume Point is a member of the Workspace's latest Snapshot — it
        // is where this work stopped, never an Artifact from elsewhere.
        let latest = display.snapshots().last().expect("a sitting was recorded");
        assert!(
            latest
                .attachments()
                .iter()
                .any(|attachment| attachment.artifact_id() == resume_point.artifact_id())
        );
        // The other members that played a part are the context of that point,
        // and they are remembered rather than discarded.
        assert_eq!(outcome.context_chain().artifacts().len(), 2);
        assert!(
            !outcome
                .context_chain()
                .artifacts()
                .contains(resume_point.artifact_id()),
            "the Resume Point is not its own context"
        );
        assert!(outcome.blockers().is_empty());
    }

    // End-to-end: the desktop's Resume action routes the derived understanding
    // through the domain Restoration Execution boundary and reflects the
    // structured result — never a claim that restoration occurred unless the
    // executor reported it.
    #[test]
    fn resume_action_reflects_structured_execution_result() {
        let root = unique_root("execution-e2e");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let mut witness = Witness::new();
            witness_one_body_of_work(
                &mut witness,
                "Design Brief — Canvas",
                "/Users/alice/design/brief.md",
                "design brief export — Terminal",
            );
        }

        let state = load_display_state(&root).unwrap();
        let outcome = state
            .outcome
            .clone()
            .expect("the derived restoration understanding reaches the desktop");
        let selection = state
            .selection
            .clone()
            .expect("the derived selection reaches the desktop");

        // The desktop routes the Resume request through the domain Execution
        // boundary with an injected executor; the executor decides the OS
        // action, the shell only reflects what it reports.
        let recorder = RecordingExecutor::default();
        let result = run_execution(Some(&outcome), Some(&selection), &state.locators, &recorder);
        let attempts = result.attempts();
        // Exactly the restore-worthy members are attempted — no declaration was
        // made, so this is the role-derived continuation.
        assert_eq!(attempts.len(), selection.restore_worthy().len());
        assert!(!attempts.is_empty());
        for attempt in attempts {
            assert!(
                matches!(attempt.status(), evo_execution::TargetStatus::Opened { .. }),
                "every preflighted target reports its own outcome: {attempt:?}"
            );
        }
        assert!(execution_result_line(&result).contains("opened"));

        // Without any derived understanding the Resume action still routes
        // through the boundary and reports explicitly, never mocked.
        let unresolved = run_execution(None, None, &state.locators, &recorder);
        assert_eq!(unresolved.attempts().len(), 1);
        assert!(matches!(
            unresolved.attempts()[0].status(),
            evo_execution::TargetStatus::Unavailable { .. }
        ));
    }

    /// Records executor calls and reports every target as opened, so the
    /// desktop routing can be verified without touching the real OS.
    #[derive(Debug, Default)]
    struct RecordingExecutor {
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl evo_execution::engine::PlatformExecutor for RecordingExecutor {
        fn open_url(&self, url: &str) -> evo_execution::TargetStatus {
            self.calls.lock().unwrap().push(format!("url:{url}"));
            evo_execution::TargetStatus::Opened {
                detail: format!("opened {url}"),
            }
        }
        fn open_file(&self, path: &str) -> evo_execution::TargetStatus {
            self.calls.lock().unwrap().push(format!("file:{path}"));
            evo_execution::TargetStatus::Opened {
                detail: format!("opened {path}"),
            }
        }
        fn focus_window(&self, title: &str) -> evo_execution::TargetStatus {
            self.calls.lock().unwrap().push(format!("window:{title}"));
            evo_execution::TargetStatus::Opened {
                detail: format!("focused {title}"),
            }
        }
    }

    impl evo_execution::PreflightSource for RecordingExecutor {
        // The fixture's file targets are treated as present so the routing
        // test exercises execution, not OS state.
        fn file_exists(&self, _path: &str) -> bool {
            true
        }
        // The fixture's window targets are reported as uniquely open, so they
        // preflight READY and are attempted. The list mirrors the window
        // subjects the tests witness; a title absent here is genuinely not
        // open, which is what the unavailable branches exercise.
        fn windows(&self) -> Result<Vec<evo_execution::macos::WindowInfo>, String> {
            Ok([
                ("Design Brief — Canvas", "Canvas"),
                ("design brief export — Terminal", "Terminal"),
            ]
            .into_iter()
            .map(|(title, owner)| evo_execution::macos::WindowInfo {
                title: title.to_string(),
                owner_pid: 1,
                owner_name: owner.to_string(),
            })
            .collect())
        }
    }

    // FIX 2 (BE-AUDIT-0001 §2.4): the announced plan and the executed attempt
    // order come from one shared ordering function and can never diverge. With
    // a declared surface the Resume Point is NOT announced first when it is
    // not first in canonical order — the historical divergence announced
    // Resume-Point-first while execution ran in ArtifactId order.
    #[test]
    fn announced_plan_equals_executed_attempt_order_with_a_declared_surface() {
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174940").unwrap();
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174940",
            vec![snapshot(
                0,
                vec![attachment("artifact-zz"), attachment("artifact-aa")],
            )],
        );
        // The declared surface lists the Resume Point first, but the Resume
        // Point is NOT the canonical-first member.
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
        assert_eq!(selection.restore_worthy().len(), 2);

        let mut locators = HashMap::new();
        locators.insert(
            "artifact-zz".to_string(),
            Locator::new(
                ArtifactId::new("artifact-zz").unwrap(),
                evo_execution::LocatorKind::FilePath("/repo/zz.md".into()),
            ),
        );
        locators.insert(
            "artifact-aa".to_string(),
            Locator::new(
                ArtifactId::new("artifact-aa").unwrap(),
                evo_execution::LocatorKind::FilePath("/repo/aa.md".into()),
            ),
        );
        let recorder = RecordingExecutor::default();
        let result = run_execution(Some(&outcome), Some(&selection), &locators, &recorder);
        // The announced plan (what plan_line describes) equals the executed
        // attempt order, on the surface branch.
        let announced = evo_execution::ordered_attempt_targets(Some(&outcome), Some(&selection))
            .expect("a declared surface with a selection announces");
        let executed: Vec<ArtifactId> = result
            .attempts()
            .iter()
            .map(|attempt| attempt.artifact_id().clone())
            .collect();
        assert_eq!(announced, executed);
        assert_eq!(
            announced.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            vec!["artifact-zz", "artifact-aa"]
        );
        // And the OS actions happened in exactly the announced order.
        let calls = recorder.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["file:/repo/zz.md", "file:/repo/aa.md"]);
    }

    // FIX 2 (BE-AUDIT-0001 §2.4): without a declared surface, the Resume
    // Point is announced first and executed first — even when the Context
    // Chain order is not ascending ArtifactId.
    #[test]
    fn announced_plan_equals_executed_attempt_order_without_a_surface() {
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174941").unwrap();
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
                evo_execution::LocatorKind::FilePath("/repo/mid.md".into()),
            ),
        );
        locators.insert(
            "artifact-z".to_string(),
            Locator::new(
                ArtifactId::new("artifact-z").unwrap(),
                evo_execution::LocatorKind::FilePath("/repo/z.md".into()),
            ),
        );
        locators.insert(
            "artifact-a".to_string(),
            Locator::new(
                ArtifactId::new("artifact-a").unwrap(),
                evo_execution::LocatorKind::FilePath("/repo/a.md".into()),
            ),
        );
        let recorder = RecordingExecutor::default();
        let result = run_execution(Some(&outcome), None, &locators, &recorder);
        let announced = evo_execution::ordered_attempt_targets(Some(&outcome), None)
            .expect("a Resume Point always announces");
        let executed: Vec<ArtifactId> = result
            .attempts()
            .iter()
            .map(|attempt| attempt.artifact_id().clone())
            .collect();
        // Resume Point first, then the Context Chain in its canonical order —
        // which is not ascending ArtifactId here.
        assert_eq!(announced, executed);
        assert_eq!(
            announced.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            vec!["artifact-mid", "artifact-z", "artifact-a"]
        );
        let calls = recorder.calls.lock().unwrap().clone();
        assert_eq!(
            calls,
            vec!["file:/repo/mid.md", "file:/repo/z.md", "file:/repo/a.md"]
        );
    }

    // A declared Continuation Surface (RFC-0013) is the restore candidate set
    // even when derivation is Insufficient with no Resume Point (IS-0021
    // §25.2: several eligible candidates, no designation). Execution acts on
    // exactly the declared restore-worthy members, in canonical order — the
    // path the Continue gate must enable.
    #[test]
    fn execution_acts_on_a_declared_surface_without_a_resume_point() {
        use evo_restoration::{RestorationInput, derive_restoration_plan};

        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174942",
            vec![snapshot(
                0,
                vec![attachment("artifact-aa"), attachment("artifact-zz")],
            )],
        );
        let snapshot = ws.snapshots().last().unwrap();
        let surface = vec![
            ArtifactId::new("artifact-aa").unwrap(),
            ArtifactId::new("artifact-zz").unwrap(),
        ];
        let input =
            RestorationInput::new_with_continuation_surface(&ws, snapshot, None, Some(surface))
                .unwrap();
        let outcome = derive_restoration_plan(&input);
        // No designation → no Resume Point, but the surface carries the
        // continuation.
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
        assert_eq!(selection.restore_worthy().len(), 2);

        let mut locators = HashMap::new();
        locators.insert(
            "artifact-aa".to_string(),
            Locator::new(
                ArtifactId::new("artifact-aa").unwrap(),
                evo_execution::LocatorKind::FilePath("/repo/aa.md".into()),
            ),
        );
        locators.insert(
            "artifact-zz".to_string(),
            Locator::new(
                ArtifactId::new("artifact-zz").unwrap(),
                evo_execution::LocatorKind::FilePath("/repo/zz.md".into()),
            ),
        );
        let recorder = RecordingExecutor::default();
        let result = run_execution(Some(&outcome), Some(&selection), &locators, &recorder);
        let announced = evo_execution::ordered_attempt_targets(Some(&outcome), Some(&selection))
            .expect("a declared surface announces");
        let executed: Vec<ArtifactId> = result
            .attempts()
            .iter()
            .map(|attempt| attempt.artifact_id().clone())
            .collect();
        assert_eq!(announced, executed);
        assert_eq!(
            announced.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            vec!["artifact-aa", "artifact-zz"]
        );
        let calls = recorder.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["file:/repo/aa.md", "file:/repo/zz.md"]);
    }

    // Workspace selection honors the user's choice while it is remembered,
    // and falls back to the canonical display selection otherwise.
    #[test]
    fn workspace_selection_prefers_the_users_choice_and_falls_back() {
        let older = workspace(
            "123e4567-e89b-12d3-a456-426614174100",
            vec![snapshot(10, vec![])],
        );
        let newer = workspace(
            "123e4567-e89b-12d3-a456-426614174200",
            vec![snapshot(20, vec![])],
        );
        let workspaces = [older.clone(), newer.clone()];

        // User selects the older Workspace: it is honored.
        let selected = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174100").unwrap();
        let picked = select_workspace(&workspaces, Some(&selected)).unwrap();
        assert_eq!(picked.id(), older.id());

        // The selection is gone: canonical display selection applies.
        let fallback = select_workspace(&workspaces, None).unwrap();
        assert_eq!(fallback.id(), newer.id());
    }

    // The user's explicit designation (RFC-0011) surfaces the Workspace that
    // contains the designated Artifact when no Workspace is selected — the
    // user's own continuation statement, never a ranking of Artifacts.
    #[test]
    fn designated_workspace_is_surfaced_without_ranking_artifacts() {
        let older = workspace(
            "123e4567-e89b-12d3-a456-426614174400",
            vec![snapshot(10, vec![attachment("artifact-designated")])],
        );
        let newer = workspace(
            "123e4567-e89b-12d3-a456-426614174500",
            vec![snapshot(20, vec![attachment("artifact-other")])],
        );
        let workspaces = [older.clone(), newer.clone()];
        let designated = ArtifactId::new("artifact-designated").unwrap();

        // No selection: the Workspace containing the designated Artifact wins
        // over the most-recent display selection.
        let picked =
            select_workspace_with_designation(&workspaces, None, Some(&designated)).unwrap();
        assert_eq!(picked.id(), older.id());

        // An explicit selection still wins over the designation.
        let selected = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174500").unwrap();
        let picked =
            select_workspace_with_designation(&workspaces, Some(&selected), Some(&designated))
                .unwrap();
        assert_eq!(picked.id(), newer.id());

        // A designation for an Artifact in no remembered Workspace falls back
        // to the canonical display selection — never a guess.
        let foreign = ArtifactId::new("artifact-foreign").unwrap();
        let picked = select_workspace_with_designation(&workspaces, None, Some(&foreign)).unwrap();
        assert_eq!(picked.id(), newer.id());
    }

    // The attested surface is derived factually from the canonical snapshot
    // history: distinct windows across all snapshots, and the latest moment's
    // windows, without any ordering semantics.
    #[test]
    fn workspace_surface_and_latest_moment_are_factual_derivations() {
        let first = attachment("artifact-alpha");
        let second = attachment("artifact-beta");
        let workspace = workspace(
            "123e4567-e89b-12d3-a456-426614174300",
            vec![
                snapshot(10, vec![first.clone()]),
                snapshot(20, vec![first.clone(), second.clone()]),
            ],
        );

        assert_eq!(
            distinct_artifacts_across_history(&workspace),
            vec!["artifact-alpha".to_string(), "artifact-beta".to_string()]
        );
        assert_eq!(
            distinct_artifacts_in_latest_snapshot(&workspace),
            vec!["artifact-alpha".to_string(), "artifact-beta".to_string()]
        );
    }

    // The workspace list label collapses many attested artifacts into a short
    // human summary without inventing a primary.
    #[test]
    fn workspace_surface_label_is_human_readable_and_collapses_many() {
        let subjects = HashMap::new();
        let kinds = HashMap::new();
        let one = workspace(
            "123e4567-e89b-12d3-a456-426614174400",
            vec![snapshot(30, vec![attachment("artifact-5e144303cd2187b4")])],
        );
        assert!(workspace_surface_label(&subjects, &kinds, &one).contains("5e144303"));

        let many = workspace(
            "123e4567-e89b-12d3-a456-426614174401",
            vec![snapshot(
                30,
                vec![
                    attachment("artifact-5e144303cd2187b4"),
                    attachment("artifact-5e144303cd2187b5"),
                    attachment("artifact-5e144303cd2187b6"),
                ],
            )],
        );
        let label = workspace_surface_label(&subjects, &kinds, &many);
        assert!(label.contains("+1 more"), "label was: {label}");

        let empty = workspace(
            "123e4567-e89b-12d3-a456-426614174402",
            vec![snapshot(30, vec![])],
        );
        assert_eq!(
            workspace_surface_label(&subjects, &kinds, &empty),
            "Nothing witnessed yet"
        );
    }

    // Kind labels derive from the frozen canonical schema names and never
    // mislabel an unknown schema.
    #[test]
    fn schema_kind_label_maps_each_frozen_schema_and_ignores_unknowns() {
        assert_eq!(
            schema_kind_label("OBS-WINDOW-FOCUS-GAINED"),
            Some(KIND_WINDOW)
        );
        assert_eq!(schema_kind_label("OBS-FILE-SAVED"), Some(KIND_FILE));
        assert_eq!(schema_kind_label("OBS-URL-NAVIGATED"), Some(KIND_URL));
        assert_eq!(schema_kind_label("OBS-COMMIT-MADE"), Some(KIND_COMMIT));
        assert_eq!(schema_kind_label("OBS-UNKNOWN"), None);
    }

    // describe_artifact uses canonical evidence only: the witnessed kind and
    // subject. It never claims a kind the record does not establish.
    #[test]
    fn describe_artifact_uses_witnessed_kind_and_subject() {
        let mut kinds = HashMap::new();
        kinds.insert("artifact-file".to_string(), KIND_FILE.to_string());
        kinds.insert("artifact-url".to_string(), KIND_URL.to_string());
        kinds.insert("artifact-commit".to_string(), KIND_COMMIT.to_string());
        let mut subjects = HashMap::new();
        subjects.insert(
            "artifact-file".to_string(),
            "/Users/me/work/report.md".to_string(),
        );
        subjects.insert(
            "artifact-url".to_string(),
            "https://example.com/doc".to_string(),
        );
        subjects.insert(
            "artifact-commit".to_string(),
            "169cc8647f7c4746".to_string(),
        );

        assert_eq!(
            describe_artifact(&kinds, &subjects, "artifact-file"),
            "saved file /Users/me/work/report.md"
        );
        assert_eq!(
            describe_artifact(&kinds, &subjects, "artifact-url"),
            "visited URL https://example.com/doc"
        );
        assert_eq!(
            describe_artifact(&kinds, &subjects, "artifact-commit"),
            "commit 169cc8647f7c4746"
        );
        // No kind established: the subject alone, never a guessed kind.
        assert_eq!(
            describe_artifact(&kinds, &subjects, "artifact-unknown"),
            "artifact-unknown"
        );
    }

    #[test]
    fn label_and_identity_formatting_is_neutral_and_short() {
        assert_eq!(
            short_identity("123e4567-e89b-12d3-a456-426614174000"),
            "123e4567"
        );
        assert_eq!(short_artifact_id("artifact-5e144303cd2187b4"), "5e144303");
    }

    #[test]
    fn relative_time_label_is_factual_and_pluralized() {
        let now = SystemTime::now();
        assert_eq!(relative_time_label(now), "just now");
        assert_eq!(
            relative_time_label(now - Duration::from_secs(60)),
            "1 minute ago"
        );
        assert_eq!(
            relative_time_label(now - Duration::from_secs(120)),
            "2 minutes ago"
        );
        assert_eq!(
            relative_time_label(now - Duration::from_secs(3600)),
            "1 hour ago"
        );
        assert_eq!(
            relative_time_label(now - Duration::from_secs(2 * 86400)),
            "2 days ago"
        );
        // A moment slightly in the future (clock skew) reads as just now.
        assert_eq!(
            relative_time_label(now + Duration::from_secs(30)),
            "just now"
        );
    }

    #[test]
    fn lifecycle_labels_are_plain_language() {
        assert_eq!(lifecycle_label(&WorkspaceLifecycle::Active), "Active");
        assert_eq!(
            lifecycle_label(&WorkspaceLifecycle::Superseded),
            "Superseded"
        );
    }

    #[test]
    fn artifact_display_dedupes_and_shortens() {
        let attachments = vec![
            attachment("artifact-5e144303cd2187b4"),
            attachment("artifact-5e144303cd2187b4"),
        ];
        assert_eq!(artifact_ids(&attachments), "5e144303");
    }

    // End-to-end: canonical evidence → derived Workspace → derived plan with a
    // declared per-Workspace Continuation Surface → desktop state. The declared
    // continuation is what reaches the shell: exactly its members are
    // restore-worthy, the rest remain durable historical membership, and the
    // Workspace's canonical membership and Artifact identities are untouched by
    // the selection.
    #[test]
    fn display_state_carries_derived_selective_restoration() {
        let root = unique_root("selection-e2e");
        let window = "Research Notes — Editor";
        let file = "/Users/alice/research/notes.md";
        let terminal = "research notes corpus — Terminal";

        // First witness the body of work, then read back the Artifact identities
        // the log established so the declaration can only name resources Evo
        // actually saw.
        {
            let _guard = Storage::with_thread_root(root.clone());
            let mut witness = Witness::new();
            witness_one_body_of_work(&mut witness, window, file, terminal);
        }
        let before = load_display_state(&root).unwrap();
        let members: Vec<ArtifactId> = [window, file, terminal]
            .into_iter()
            .map(|subject| artifact_for(&before, subject))
            .collect();

        // The user declares that their work continues at two of the three.
        {
            let _guard = Storage::with_thread_root(root.clone());
            let mut witness = Witness::new();
            witness.record(MacOSSignal::ContinuationSurface {
                subjects: vec![window.to_string(), file.to_string()],
                observed_at: t(100_000),
            });
        }

        let state = load_display_state(&root).unwrap();
        let selection = state.selection.expect("derived selection is present");
        assert_eq!(
            selection.workspace_id(),
            state.display.as_ref().expect("displayed workspace").id()
        );
        // Exactly the two declared members are restore-worthy.
        let mut worthy: Vec<String> = selection
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().to_string())
            .collect();
        worthy.sort();
        let mut expected = vec![members[0].to_string(), members[1].to_string()];
        expected.sort();
        assert_eq!(worthy, expected);
        // Both declared members carry executable targets — nothing unavailable.
        assert!(selection.unavailable().is_empty());
        // The third member remains withheld: durable membership, never
        // opened merely because it belongs to the body of work.
        let withheld: Vec<String> = selection
            .withheld()
            .iter()
            .map(|member| member.artifact_id().to_string())
            .collect();
        assert_eq!(withheld, vec![members[2].to_string()]);
        // And it says which way it is kept, and why.
        assert!(!selection.withheld()[0].reason().is_empty());
        assert!(!selection.withheld()[0].disposition().opens_unasked());
        // The declaration changed what opens, not what is remembered.
        let display = state.display.expect("displayed workspace");
        assert_eq!(display.attachments().len(), 3);
    }

    // surface_subjects returns exactly the declared continuation of one
    // Workspace (restore-worthy ∪ unavailable), in canonical order — the set
    // the user's latest declaration established.
    #[test]
    fn surface_subjects_returns_the_declared_continuation_in_canonical_order() {
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174915").unwrap();
        let surface = vec![
            ArtifactId::new("artifact-url").unwrap(),
            ArtifactId::new("artifact-file").unwrap(),
            ArtifactId::new("artifact-commit").unwrap(),
        ];
        let plan = RestorationPlan::new_with_continuation_surface(
            workspace_id.clone(),
            ResumePoint::new(
                workspace_id.clone(),
                ArtifactId::new("artifact-file").unwrap(),
            ),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(workspace_id, "continue").unwrap(),
            surface,
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);
        let members = vec![
            (
                ArtifactId::new("artifact-file").unwrap(),
                ResourceRole::Continuation,
            ),
            (
                ArtifactId::new("artifact-url").unwrap(),
                ResourceRole::Supporting,
            ),
            (
                ArtifactId::new("artifact-commit").unwrap(),
                ResourceRole::Reference,
            ),
        ];
        let mut evidence = HashMap::new();
        evidence.insert(
            "artifact-file".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/plan.md".to_string()),
        );
        evidence.insert(
            "artifact-url".to_string(),
            (
                "OBS-URL-NAVIGATED".to_string(),
                "https://docs.evo.dev".to_string(),
            ),
        );
        evidence.insert(
            "artifact-commit".to_string(),
            (
                "OBS-COMMIT-MADE".to_string(),
                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
            ),
        );
        let selection = evo_execution::select_restoration(&outcome, &members, &evidence);
        let mut subjects = HashMap::new();
        subjects.insert("artifact-file".to_string(), "/repo/plan.md".to_string());
        subjects.insert(
            "artifact-url".to_string(),
            "https://docs.evo.dev".to_string(),
        );
        subjects.insert(
            "artifact-commit".to_string(),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
        );

        let declared = surface_subjects(&selection, &subjects);
        // Canonical ascending order, exactly the declared three subjects.
        assert_eq!(
            declared,
            vec![
                "/repo/plan.md".to_string(),
                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
                "https://docs.evo.dev".to_string(),
            ]
        );
    }

    // Once a continuation surface is declared, Continue acts on exactly its
    // restore-worthy members — never on recently observed but undeclared
    // resources, never on a commit, never on anything the user did not
    // declare. Recency is not restoration authority.
    #[test]
    fn resume_acts_on_the_declared_selection_not_recent_activity() {
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174916").unwrap();
        // Declared surface: file + URL + commit. The unrelated "recent"
        // resources are NOT declared.
        let surface = vec![
            ArtifactId::new("artifact-file").unwrap(),
            ArtifactId::new("artifact-url").unwrap(),
            ArtifactId::new("artifact-commit").unwrap(),
        ];
        let plan = RestorationPlan::new_with_continuation_surface(
            workspace_id.clone(),
            ResumePoint::new(
                workspace_id.clone(),
                ArtifactId::new("artifact-file").unwrap(),
            ),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
            surface,
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);

        let mut evidence = HashMap::new();
        evidence.insert(
            "artifact-file".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/plan.md".to_string()),
        );
        evidence.insert(
            "artifact-url".to_string(),
            (
                "OBS-URL-NAVIGATED".to_string(),
                "https://docs.evo.dev".to_string(),
            ),
        );
        evidence.insert(
            "artifact-commit".to_string(),
            (
                "OBS-COMMIT-MADE".to_string(),
                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
            ),
        );
        // Unrelated resources witnessed around the same period (a browser
        // tab and a chat conversation) — observed, but never declared into
        // the continuation.
        evidence.insert(
            "artifact-browser".to_string(),
            (
                "OBS-URL-NAVIGATED".to_string(),
                "https://unrelated.example/tab".to_string(),
            ),
        );
        evidence.insert(
            "artifact-chat".to_string(),
            (
                "OBS-WINDOW-FOCUS-GAINED".to_string(),
                "Chat — unrelated".to_string(),
            ),
        );
        let members = vec![
            (
                ArtifactId::new("artifact-file").unwrap(),
                ResourceRole::Continuation,
            ),
            (
                ArtifactId::new("artifact-url").unwrap(),
                ResourceRole::Primary,
            ),
            (
                ArtifactId::new("artifact-commit").unwrap(),
                ResourceRole::Reference,
            ),
            (
                ArtifactId::new("artifact-browser").unwrap(),
                ResourceRole::Context,
            ),
            (
                ArtifactId::new("artifact-chat").unwrap(),
                ResourceRole::Context,
            ),
        ];
        let selection = evo_execution::select_restoration(&outcome, &members, &evidence);
        // Restore-worthy: file + URL. The commit is honestly unavailable.
        assert_eq!(selection.restore_worthy().len(), 2);
        assert_eq!(selection.unavailable().len(), 1);
        // The unrelated resources are withheld membership, never opened — and
        // kept as record rather than offered, because nothing beyond having
        // been there ties them to the work.
        assert_eq!(selection.withheld().len(), 2);
        assert_eq!(selection.history_only().count(), 2);

        let mut locators = HashMap::new();
        locators.insert(
            "artifact-file".to_string(),
            Locator::new(
                ArtifactId::new("artifact-file").unwrap(),
                evo_execution::LocatorKind::FilePath("/repo/plan.md".into()),
            ),
        );
        locators.insert(
            "artifact-url".to_string(),
            Locator::new(
                ArtifactId::new("artifact-url").unwrap(),
                evo_execution::LocatorKind::Url("https://docs.evo.dev".into()),
            ),
        );
        locators.insert(
            "artifact-browser".to_string(),
            Locator::new(
                ArtifactId::new("artifact-browser").unwrap(),
                evo_execution::LocatorKind::Url("https://unrelated.example/tab".into()),
            ),
        );
        let recorder = RecordingExecutor::default();
        let result = run_execution(Some(&outcome), Some(&selection), &locators, &recorder);
        let calls = recorder.calls.lock().unwrap().clone();
        // Exactly the two declared, reopenable resources are attempted — never
        // the commit, never the unrelated tab/chat.
        assert_eq!(
            calls,
            vec!["file:/repo/plan.md", "url:https://docs.evo.dev"]
        );
        // Every surface member receives an honest per-target outcome: the two
        // READY targets were attempted; the commit is UNSUPPORTED (a
        // repository object, never guessed into a target).
        assert_eq!(result.attempts().len(), 3);
        assert!(!result.all_opened());
        let commit = result
            .attempts()
            .iter()
            .find(|attempt| attempt.artifact_id().as_str() == "artifact-commit")
            .expect("the commit is a surface member and is reported");
        assert!(matches!(
            commit.status(),
            evo_execution::TargetStatus::Unsupported { .. }
        ));
    }

    // AMENDED RULE (§9, §12). This test previously read
    // `display_state_selection_is_empty_without_a_surface` and asserted that
    // with no declaration nothing is restore-worthy. That made Continue do
    // nothing until the user first told Evo where they were — the one thing Evo
    // exists to spare them. The selection is now the members whose derived role
    // says the work happens there. What the old test protected is kept: the
    // selection is strictly smaller than membership, and the rest stays
    // historical rather than being opened.
    #[test]
    fn without_a_declaration_the_selection_opens_only_where_the_work_happens() {
        let root = unique_root("selection-undeclared-e2e");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let mut witness = Witness::new();
            witness_one_body_of_work(
                &mut witness,
                "Case Summary — Editor",
                "/Users/alice/matters/case-summary.docx",
                "case summary exhibits — Terminal",
            );
        }

        let state = load_display_state(&root).unwrap();
        let display = state
            .display
            .clone()
            .expect("a coherent body of work is displayed");
        let selection = state.selection.expect("derived selection is present");

        // Something opens, without the user having declared anything.
        assert!(
            !selection.restore_worthy().is_empty(),
            "Continue must take the person back to their work unasked"
        );
        // But not everything: the rest is remembered and stays closed.
        let members = display.attachments().len();
        let opening = selection.restore_worthy().len() + selection.unavailable().len();
        assert!(
            opening < members,
            "{opening} of {members} members opening is not selective restoration"
        );
        assert_eq!(selection.withheld().len(), members - opening);
        // Nothing withheld is lost, and each one says which way it is kept.
        for member in selection.withheld() {
            assert!(!member.disposition().opens_unasked());
            assert!(!member.reason().is_empty());
        }
    }

    // The primary surface is the Workspace list; a detail view is entered by
    // an explicit selection keyed by canonical Workspace identity, and Back
    // returns to the list. A stale selection (a Workspace no longer
    // remembered) falls back to Home honestly.
    #[test]
    fn shell_view_navigation_is_identity_keyed_and_returns_home() {
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174922",
            vec![snapshot(0, vec![attachment("artifact-aa")])],
        );
        let id = ws.id().clone();
        let workspaces = [ws];
        // Opening a Workspace keys the detail view by its canonical identity,
        // never by a title.
        assert_eq!(
            ShellView::from_selection(Some(&id), &workspaces),
            ShellView::Workspace(id.clone())
        );
        // Back returns to the Workspace list — the primary surface.
        assert_eq!(
            ShellView::from_selection(None, &workspaces),
            ShellView::Home
        );
        // A stale selection falls back to Home rather than pinning the shell
        // to a forgotten body of work.
        let stale = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174923").unwrap();
        assert_eq!(
            ShellView::from_selection(Some(&stale), &workspaces),
            ShellView::Home
        );
    }

    // The home card derives title, entry point, Next Step, and continuation
    // count from canonical state only: the title follows the Resume Point's
    // witnessed subject, the insufficiency stays honest, and the count is the
    // derived selection's surface members.
    #[test]
    fn workspace_card_derives_title_entry_point_and_continuation_from_canonical_state() {
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174920").unwrap();
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174920",
            vec![snapshot(
                0,
                vec![
                    attachment("artifact-file"),
                    attachment("artifact-url"),
                    attachment("artifact-commit"),
                ],
            )],
        );
        let plan = RestorationPlan::new_with_continuation_surface(
            workspace_id.clone(),
            ResumePoint::new(
                workspace_id.clone(),
                ArtifactId::new("artifact-file").unwrap(),
            ),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
            vec![
                ArtifactId::new("artifact-file").unwrap(),
                ArtifactId::new("artifact-url").unwrap(),
                ArtifactId::new("artifact-commit").unwrap(),
            ],
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);
        let mut evidence = HashMap::new();
        evidence.insert(
            "artifact-file".to_string(),
            ("OBS-FILE-SAVED".to_string(), "/repo/plan.md".to_string()),
        );
        evidence.insert(
            "artifact-url".to_string(),
            (
                "OBS-URL-NAVIGATED".to_string(),
                "https://docs.evo.dev".to_string(),
            ),
        );
        evidence.insert(
            "artifact-commit".to_string(),
            (
                "OBS-COMMIT-MADE".to_string(),
                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
            ),
        );
        let members: Vec<(ArtifactId, ResourceRole)> = ws
            .attachments()
            .iter()
            .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
            .collect();
        let selection = evo_execution::select_restoration(&outcome, &members, &evidence);
        let mut subjects = HashMap::new();
        subjects.insert("artifact-file".to_string(), "/repo/plan.md".to_string());
        subjects.insert(
            "artifact-url".to_string(),
            "https://docs.evo.dev".to_string(),
        );
        subjects.insert(
            "artifact-commit".to_string(),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
        );
        let mut kinds = HashMap::new();
        kinds.insert("artifact-file".to_string(), KIND_FILE.to_string());
        kinds.insert("artifact-url".to_string(), KIND_URL.to_string());
        kinds.insert("artifact-commit".to_string(), KIND_COMMIT.to_string());

        // No derived title for this body of work, so the card falls back to the
        // Resume Point's witnessed subject — the honest second choice, exercised
        // here on purpose. The derived-title path has its own test below.
        let mut standings = HashMap::new();
        standings.insert(workspace_id.to_string(), Standing::Continuable);
        let card = workspace_card(
            &ws,
            Some(&outcome),
            Some(&selection),
            &subjects,
            &kinds,
            &HashMap::new(),
            &standings,
        );
        assert_eq!(card.id, workspace_id);
        // The title falls back to the Resume Point's witnessed subject.
        assert_eq!(card.title, "/repo/plan.md");
        assert_eq!(
            card.resume_from.as_deref(),
            Some("saved file /repo/plan.md")
        );
        assert_eq!(card.next_step.as_deref(), Some("continue"));
        // The commit is a surface member (honestly unavailable) → the current
        // continuation has 3 resources.
        assert_eq!(card.continuation_count, 3);
        assert!(card.continuable);
        assert!(card.description.contains("/repo/plan.md"));
        // The Engagement layer's standing rides along on the card so Home can
        // decide whether to present this as work; it is carried, not invented.
        assert_eq!(card.standing, Standing::Continuable);
        assert!(card.standing.is_work());
    }

    // A Workspace with no derived understanding is presented honestly: no
    // invented title, no guessed entry point, no continuation count.
    #[test]
    fn workspace_card_is_honest_when_nothing_is_established() {
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174921",
            vec![snapshot(0, vec![attachment("artifact-aa")])],
        );
        let card = workspace_card(
            &ws,
            None,
            None,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
        );
        assert_eq!(card.title, "Untitled work");
        assert!(card.resume_from.is_none());
        assert!(card.next_step.is_none());
        assert_eq!(card.continuation_count, 0);
        assert!(!card.continuable);
        assert!(card.description.contains("artifact-aa"));
        // No standing was carried for this body of work, so Evo makes the
        // least claim it can: Remembered — kept and findable, never work by
        // default. Work is something the evidence has to earn.
        assert_eq!(card.standing, Standing::Remembered);
        assert!(!card.standing.is_work());
    }

    // §15: Home names the body of work. The derived title comes from the layer
    // that measured what the work's members have in common, so it outranks the
    // name of whichever resource the person's attention was last in — which is
    // how an afternoon of coursework came to be titled after a music player.
    #[test]
    fn a_card_is_titled_by_the_work_not_by_the_window_attention_ended_in() {
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};

        let workspace_id = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174922").unwrap();
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174922",
            vec![snapshot(
                0,
                vec![attachment("artifact-player"), attachment("artifact-brief")],
            )],
        );
        // The Resume Point is the resource attention was last in.
        let plan = RestorationPlan::new(
            workspace_id.clone(),
            ResumePoint::new(
                workspace_id.clone(),
                ArtifactId::new("artifact-player").unwrap(),
            ),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(workspace_id.clone(), "continue").unwrap(),
        )
        .unwrap();
        let outcome = evo_restoration::DerivationOutcome::Complete(plan);
        let mut subjects = HashMap::new();
        subjects.insert("artifact-player".to_string(), "Some Player".to_string());
        subjects.insert(
            "artifact-brief".to_string(),
            "/Users/x/coursework/brief.odt".to_string(),
        );
        let mut titles = HashMap::new();
        titles.insert(workspace_id.to_string(), "coursework brief".to_string());

        let card = workspace_card(
            &ws,
            Some(&outcome),
            None,
            &subjects,
            &HashMap::new(),
            &titles,
            &HashMap::new(),
        );
        assert_eq!(
            card.title, "coursework brief",
            "the card is named after the work, not after the window in front of it"
        );
        // The Resume Point is untouched: naming the work is not the same claim as
        // where it continues, and the card still reports the latter honestly.
        assert!(
            card.resume_from
                .as_deref()
                .is_some_and(|from| from.contains("Some Player")),
            "the continuation point stays whatever the evidence established"
        );
    }

    // A derived title that is blank claims nothing, and a claim of nothing must
    // not displace a name the evidence can actually supply.
    #[test]
    fn a_blank_derived_title_falls_back_rather_than_blanking_the_card() {
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174923",
            vec![snapshot(0, vec![attachment("artifact-only")])],
        );
        let mut subjects = HashMap::new();
        subjects.insert("artifact-only".to_string(), "/Users/x/thing.md".to_string());
        let mut titles = HashMap::new();
        titles.insert(ws.id().to_string(), "   ".to_string());

        let card = workspace_card(
            &ws,
            None,
            None,
            &subjects,
            &HashMap::new(),
            &titles,
            &HashMap::new(),
        );
        assert_eq!(card.title, "/Users/x/thing.md");
    }

    // FIX 3 (BE-TRACE-0001 §3.2(b)): Home may reorder Workspaces but never
    // omit them. Continuable work (a derived Resume Point) is grouped first;
    // within each group the canonical log order is preserved exactly. The
    // only distinction is the Resume Point Home already computes — no
    // recency, no frequency, no application identity, no score.
    #[test]
    fn home_workspaces_are_ordered_continuable_first_without_omission() {
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};

        // Canonical log order: A, B, C, D. A and C have a derived Resume
        // Point (marked continuation); B and D have none. D is the most
        // recently witnessed — recency must not promote it above continuable
        // work.
        let id_a = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174960").unwrap();
        let id_b = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174961").unwrap();
        let id_c = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174962").unwrap();
        let id_d = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174963").unwrap();
        let ws_a = workspace(
            "123e4567-e89b-12d3-a456-426614174960",
            vec![snapshot(0, vec![attachment("artifact-a")])],
        );
        let ws_b = workspace(
            "123e4567-e89b-12d3-a456-426614174961",
            vec![snapshot(10, vec![attachment("artifact-b")])],
        );
        let ws_c = workspace(
            "123e4567-e89b-12d3-a456-426614174962",
            vec![snapshot(20, vec![attachment("artifact-c")])],
        );
        let ws_d = workspace(
            "123e4567-e89b-12d3-a456-426614174963",
            vec![snapshot(30, vec![attachment("artifact-d")])],
        );
        let all = vec![ws_a, ws_b, ws_c, ws_d];

        let mut outcomes = HashMap::new();
        for (id, artifact) in [(&id_a, "artifact-a"), (&id_c, "artifact-c")] {
            let plan = RestorationPlan::new(
                id.clone(),
                ResumePoint::new(id.clone(), ArtifactId::new(artifact).unwrap()),
                ContextChain::new(vec![]).unwrap(),
                vec![],
                NextStep::new(id.clone(), "continue").unwrap(),
            )
            .unwrap();
            outcomes.insert(
                id.to_string(),
                evo_restoration::DerivationOutcome::Complete(plan),
            );
        }
        // B and D carry no persisted outcome → no Resume Point.

        let ordered = order_home_workspaces(&all, &outcomes);
        // Continuable first, canonical order preserved within each group,
        // nothing omitted: A, C (in log order), then B, D (in log order).
        let ids: Vec<String> = ordered.iter().map(|ws| ws.id().to_string()).collect();
        assert_eq!(
            ids,
            vec![
                id_a.to_string(),
                id_c.to_string(),
                id_b.to_string(),
                id_d.to_string()
            ]
        );
        // Every remembered Workspace is present exactly once — reordering
        // never omits.
        assert_eq!(ordered.len(), all.len());
        // Deterministic: identical canonical input produces identical order.
        assert_eq!(ordered, order_home_workspaces(&all, &outcomes));
    }

    // FIX 3: the grouping is a pure function of the derived Resume Point. A
    // Workspace whose outcome is an Insufficient derivation WITH a Resume
    // Point is still continuable; a Workspace with a Complete plan but no
    // surface is still continuable; the key is never recency.
    #[test]
    fn home_ordering_keys_on_the_derived_resume_point_alone() {
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};

        let id_x = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174970").unwrap();
        let id_y = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174971").unwrap();
        let ws_x = workspace(
            "123e4567-e89b-12d3-a456-426614174970",
            vec![snapshot(0, vec![attachment("artifact-x")])],
        );
        let ws_y = workspace(
            "123e4567-e89b-12d3-a456-426614174971",
            vec![snapshot(1, vec![attachment("artifact-y")])],
        );

        // Y was witnessed later, but X is the continuable one.
        let plan = RestorationPlan::new(
            id_x.clone(),
            ResumePoint::new(id_x.clone(), ArtifactId::new("artifact-x").unwrap()),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(id_x.clone(), "continue").unwrap(),
        )
        .unwrap();
        let mut outcomes = HashMap::new();
        outcomes.insert(
            id_x.to_string(),
            evo_restoration::DerivationOutcome::Complete(plan),
        );

        let ordered = order_home_workspaces(&[ws_x, ws_y], &outcomes);
        assert_eq!(ordered[0].id(), &id_x);
        assert_eq!(ordered[1].id(), &id_y);
    }

    // Phase 8: multiple Workspaces. Each Workspace's continuation is derived
    // from its own declaration intersection — A's current continuation
    // contains only A's declared members, B's only B's; a member never
    // declared by either stays historical; and recency (B witnessed later)
    // never moves a resource into A's continuation.
    #[test]
    fn multi_workspace_continuation_never_leaks_across_workspaces() {
        use evo_restoration::{ContextChain, NextStep, RestorationPlan, ResumePoint};

        // Workspace A: A1+A2 declared; A3 stays historical. Workspace B: B1
        // declared; B2 stays historical. B's snapshots are witnessed LATER
        // than A's — recency must not change A's continuation.
        let id_a = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174930").unwrap();
        let id_b = WorkspaceId::from_str("123e4567-e89b-12d3-a456-426614174931").unwrap();
        let ws_a = workspace(
            "123e4567-e89b-12d3-a456-426614174930",
            vec![snapshot(
                0,
                vec![
                    attachment("artifact-a1"),
                    attachment("artifact-a2"),
                    attachment("artifact-a3"),
                ],
            )],
        );
        let ws_b = workspace(
            "123e4567-e89b-12d3-a456-426614174931",
            vec![snapshot(
                100,
                vec![attachment("artifact-b1"), attachment("artifact-b2")],
            )],
        );
        let plan_a = RestorationPlan::new_with_continuation_surface(
            id_a.clone(),
            ResumePoint::new(id_a.clone(), ArtifactId::new("artifact-a1").unwrap()),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(id_a.clone(), "continue A").unwrap(),
            vec![
                ArtifactId::new("artifact-a1").unwrap(),
                ArtifactId::new("artifact-a2").unwrap(),
            ],
        )
        .unwrap();
        let plan_b = RestorationPlan::new_with_continuation_surface(
            id_b.clone(),
            ResumePoint::new(id_b.clone(), ArtifactId::new("artifact-b1").unwrap()),
            ContextChain::new(vec![]).unwrap(),
            vec![],
            NextStep::new(id_b.clone(), "continue B").unwrap(),
            vec![ArtifactId::new("artifact-b1").unwrap()],
        )
        .unwrap();
        let mut evidence = HashMap::new();
        for (id, subject) in [
            ("artifact-a1", "/repo-a/a1.md"),
            ("artifact-a2", "/repo-a/a2.md"),
            ("artifact-a3", "/repo-a/a3-old.md"),
            ("artifact-b1", "/repo-b/b1.md"),
            ("artifact-b2", "/repo-b/b2.md"),
        ] {
            evidence.insert(
                id.to_string(),
                ("OBS-FILE-SAVED".to_string(), subject.to_string()),
            );
        }
        let members_a: Vec<(ArtifactId, ResourceRole)> = ws_a
            .attachments()
            .iter()
            .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
            .collect();
        let members_b: Vec<(ArtifactId, ResourceRole)> = ws_b
            .attachments()
            .iter()
            .map(|attachment| (attachment.artifact_id().clone(), attachment.role()))
            .collect();
        let selection_a = evo_execution::select_restoration(
            &DerivationOutcome::Complete(plan_a),
            &members_a,
            &evidence,
        );
        let selection_b = evo_execution::select_restoration(
            &DerivationOutcome::Complete(plan_b),
            &members_b,
            &evidence,
        );

        // A's continuation is exactly its own declared surface — never B's
        // members, despite B being witnessed later (recency ≠ authority).
        let mut a_worthy: Vec<&str> = selection_a
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().as_str())
            .collect();
        a_worthy.sort();
        assert_eq!(a_worthy, vec!["artifact-a1", "artifact-a2"]);
        // A3 stays withheld: durable membership, never a restore target.
        assert_eq!(selection_a.withheld().len(), 1);
        assert_eq!(
            selection_a.withheld()[0].artifact_id(),
            &ArtifactId::new("artifact-a3").unwrap()
        );
        // B's continuation is exactly its own declared surface.
        let b_worthy: Vec<&str> = selection_b
            .restore_worthy()
            .iter()
            .map(|selected| selected.artifact_id().as_str())
            .collect();
        assert_eq!(b_worthy, vec!["artifact-b1"]);
        assert_eq!(selection_b.withheld().len(), 1);
        assert_eq!(
            selection_b.withheld()[0].artifact_id(),
            &ArtifactId::new("artifact-b2").unwrap()
        );
        // No cross-Workspace leakage in either direction.
        assert!(a_worthy.iter().all(|id| id.starts_with("artifact-a")));
        assert!(b_worthy.iter().all(|id| id.starts_with("artifact-b")));
        // Workspace identities are canonical and unchanged by the selection.
        assert_eq!(selection_a.workspace_id(), &id_a);
        assert_eq!(selection_b.workspace_id(), &id_b);
    }

    // D2 — the WorkGrouped desktop channel (RFC-0012). The shell only
    // forwards the user's explicit declaration through the daemon's trusted
    // socket and reflects the daemon's honest response: a declaration for two
    // witnessed subjects is accepted and forwarded with the canonical pair
    // ordering; a declaration naming an unwitnessed subject is rejected
    // verbatim, never invented.
    #[test]
    fn submit_grouping_forwards_to_the_daemon_and_reports_honestly() {
        use evo_daemon::grouping::{grouping_socket_path, spawn_grouping_listener};
        use evo_observation::evidence::{Evidence, FactValue, ObservedFact};
        use evo_observation::observation::Observation;
        use evo_observation::observation_id::ObservationId;
        use evo_observation::observation_schema::ObservationSchema;
        use evo_observation::provenance::{ObservationSource, Provenance};

        use std::collections::HashMap;
        use std::sync::mpsc::channel;

        let root = unique_root("grouping");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_grouping_listener(root.clone(), sender);
        let socket = grouping_socket_path(&root);
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        let witness = |subject: &str| {
            let _guard = Storage::with_thread_root(root.clone());
            let observation = {
                let fact =
                    ObservedFact::new("FileSaved", FactValue::Text(subject.to_string())).unwrap();
                Observation::new(
                    ObservationId::new(),
                    ObservationSchema::file_saved_v1(),
                    Provenance::new(
                        ObservationSource::new("grouping-test").unwrap(),
                        UNIX_EPOCH,
                        HashMap::new(),
                    ),
                    Evidence::new(vec![fact]),
                )
            };
            persist_observation(&observation).expect("observation persists");
        };

        // Unwitnessed subjects are ACCEPTED — a declaration is the
        // person's word (Law IX), linking group identity for the future;
        // witnessing is not a precondition for speaking.
        let accepted = submit_grouping(&root, "/tmp/a.md", "/tmp/b.md").expect("submission works");
        assert_eq!(accepted, GroupingResponse::Accepted);
        let signal = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("accepted declaration forwards to the runtime");
        let MacOSSignal::WorkGrouped { first, second, .. } = signal else {
            panic!("forwarded signal must be WorkGrouped");
        };
        assert_eq!(first, "/tmp/a.md", "canonical pair ordering: min first");
        assert_eq!(second, "/tmp/b.md");

        // A second declaration naming a partially-unwitnessed subject is
        // still accepted and forwarded whole.
        let accepted = submit_grouping(&root, "/tmp/a.md", "/tmp/never-witnessed.md")
            .expect("submission works");
        assert_eq!(accepted, GroupingResponse::Accepted);
        let signal = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("the declaration is forwarded whole");
        let MacOSSignal::WorkGrouped { first, second, .. } = signal else {
            panic!("forwarded signal must be WorkGrouped");
        };
        assert_eq!(first, "/tmp/a.md");
        assert_eq!(second, "/tmp/never-witnessed.md");
    }

    // D2 — the WorkGrouped surface must only ever offer witnessed subjects:
    // a Workspace whose every witnessed subject is already a member has
    // nothing to group, so the derived candidate set is empty (no invented
    // candidates, no app/domain/recency heuristics).
    #[test]
    fn grouping_candidates_are_witnessed_subjects_outside_the_workspace() {
        let root = unique_root("grouping-candidates");
        let _guard = Storage::with_thread_root(root.clone());

        let mut subjects = HashMap::new();
        subjects.insert("artifact-a".to_string(), "/tmp/a.md".to_string());
        subjects.insert("artifact-b".to_string(), "/tmp/b.md".to_string());
        let ws = workspace(
            "123e4567-e89b-12d3-a456-426614174099",
            vec![snapshot(1, vec![attachment("artifact-a")])],
        );
        let others: Vec<&str> = subjects
            .values()
            .map(String::as_str)
            .filter(|subject| {
                !ws.attachments()
                    .iter()
                    .any(|a| subject_for(&subjects, a.artifact_id().as_str()) == Some(*subject))
            })
            .collect();
        assert_eq!(
            others,
            vec!["/tmp/b.md"],
            "only the witnessed outsider is a candidate"
        );
    }
}
