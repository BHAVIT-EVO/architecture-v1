//! Restoration Execution boundary.
//!
//! This module implements the Architecture's Restoration Execution layer
//! (ARCHITECTURE.md §4 pipeline stage 8, §5 Restoration Engine): consume a
//! valid derived Restoration Plan / Resume Point and perform the restoration,
//! reporting per-item success or failure explicitly.
//!
//! # Contractual status
//!
//! IS-0019 §11 and IS-0021 §20 define Execution as a separate architectural
//! layer that consumes Restoration Plans and MAY launch applications, restore
//! windows, focus documents, and interact with operating systems. Both
//! specifications defer the concrete execution contract to a future
//! specification ("Future Restoration Execution specification"). RFC-0006
//! likewise explicitly does not define application launching, browser
//! automation, or operating-system APIs.
//!
//! The frozen lower-layer model (IS-0004) records Artifact identity only:
//! an [`Artifact`] is an opaque identity hypothesis with no canonical mapping
//! to an OS-resumable resource, and IS-0011 forbids Snapshots from containing
//! execution instructions.
//!
//! # Behavior under the current frozen model
//!
//! This boundary therefore:
//!
//! - accepts only a valid derived [`ExecutionRequest`] — a complete
//!   Restoration Plan or a Resume Point produced by canonical Restoration
//!   Derivation ([`ExecutionRequest::from_derivation`]);
//! - checks the canonical evidence required to act;
//! - refuses execution explicitly when that evidence is absent, naming the
//!   missing evidence;
//! - returns a structured [`ExecutionResult`];
//! - never fabricates actions and never pretends that execution occurred.
//!
//! # Invariants
//!
//! - Deterministic: identical requests produce identical results.
//! - Does not mutate the request or any canonical object.
//! - Does not consult raw runtime state, current OS state, or user interface
//!   state.
//! - Does not perform Workspace or Artifact inference.
//! - Performs no OS-level action of any kind under the current model.

use evo_artifact::artifact_id::ArtifactId;

use crate::derivation::DerivationOutcome;
use crate::resume_point::ResumePoint;
use crate::restoration_plan::RestorationPlan;

// ── ExecutionRequest ──────────────────────────────────────────────────────────

/// A canonical request to the Restoration Execution boundary.
///
/// Carries exactly one valid derived restoration input: a complete
/// [`RestorationPlan`] or a derived [`ResumePoint`] (IS-0019 §11: Execution
/// consumes Restoration Plans).
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionRequest {
    /// Execute a complete derived Restoration Plan.
    Plan(RestorationPlan),
    /// Execute a derived Resume Point.
    ResumePoint(ResumePoint),
}

impl ExecutionRequest {
    /// Builds the execution request from a canonical derivation outcome.
    ///
    /// Accepts only valid derived inputs:
    ///
    /// - a complete [`DerivationOutcome::Complete`] Restoration Plan; or
    /// - an insufficient outcome that nevertheless established a Resume Point.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::NoDerivedResumePoint`] when the outcome
    /// establishes no Resume Point, so there is nothing valid to execute
    /// (IS-0021 §25.2).
    pub fn from_derivation(outcome: &DerivationOutcome) -> Result<Self, ExecutionError> {
        match outcome {
            DerivationOutcome::Complete(plan) => Ok(ExecutionRequest::Plan(plan.clone())),
            DerivationOutcome::Insufficient(insufficient) => match insufficient.resume_point() {
                Some(resume_point) => Ok(ExecutionRequest::ResumePoint(resume_point.clone())),
                None => Err(ExecutionError::NoDerivedResumePoint {
                    reason: insufficient
                        .resume_point_missing()
                        .map(|missing| missing.reason().to_string())
                        .unwrap_or_else(|| {
                            "the derivation outcome established no Resume Point (IS-0021 §25.2)"
                                .to_string()
                        }),
                }),
            },
        }
    }

    /// Returns the Resume Point this request executes.
    pub fn resume_point(&self) -> &ResumePoint {
        match self {
            ExecutionRequest::Plan(plan) => plan.resume_point(),
            ExecutionRequest::ResumePoint(resume_point) => resume_point,
        }
    }

    /// Returns the complete Restoration Plan, when this request carries one.
    pub fn plan(&self) -> Option<&RestorationPlan> {
        match self {
            ExecutionRequest::Plan(plan) => Some(plan),
            ExecutionRequest::ResumePoint(_) => None,
        }
    }
}

// ── ExecutionResult ───────────────────────────────────────────────────────────

/// The structured result of an execution request.
///
/// Exactly one result is produced per request. Under the current frozen model
/// the boundary can only refuse; [`ExecutionResult::Executed`] documents the
/// per-item reporting shape the Architecture requires (ARCHITECTURE.md §5
/// Restoration Engine: "every attempted artifact reports success or failure
/// explicitly; nothing fails silently") for when a frozen execution contract
/// exists.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// Restoration actions were actually performed, reported per item.
    Executed { attempts: Vec<ExecutionAttempt> },
    /// Execution was refused because required canonical evidence is absent.
    Refused { reason: String },
}

/// One reported execution attempt for one Artifact.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionAttempt {
    artifact_id: ArtifactId,
    status: ExecutionStatus,
}

impl ExecutionAttempt {
    /// Constructs an execution attempt record for one Artifact.
    pub fn new(artifact_id: ArtifactId, status: ExecutionStatus) -> Self {
        Self {
            artifact_id,
            status,
        }
    }

    /// Returns the Artifact this attempt concerned.
    pub fn artifact_id(&self) -> &ArtifactId {
        &self.artifact_id
    }

    /// Returns the reported status of this attempt.
    pub fn status(&self) -> &ExecutionStatus {
        &self.status
    }
}

/// The reported status of one execution attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStatus {
    /// The target opened successfully.
    Opened,
    /// The target could not be opened.
    Failed { reason: String },
}

// ── ExecutionError ────────────────────────────────────────────────────────────

/// Errors from the Restoration Execution boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    /// The canonical derivation outcome establishes no Resume Point, so no
    /// valid execution request exists (IS-0021 §25.2).
    NoDerivedResumePoint { reason: String },
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::NoDerivedResumePoint { reason } => {
                write!(f, "no valid derived execution request: {reason}")
            }
        }
    }
}

impl std::error::Error for ExecutionError {}

// ── execute ──────────────────────────────────────────────────────────────────

/// Runs the Restoration Execution boundary for one valid derived request.
///
/// Checks the canonical evidence required to perform the restoration:
/// an executable target for the referenced Artifact. The frozen model records
/// Artifact identity only (IS-0004), no canonical Artifact-to-resource mapping
/// exists, and no frozen Restoration Execution specification defines how a
/// Plan is performed (IS-0019 §11; IS-0021 §20). Execution therefore cannot
/// truthfully act and is refused with the missing evidence named explicitly.
///
/// No restoration action is ever fabricated, and the result never claims that
/// execution occurred when it did not.
pub fn execute(request: &ExecutionRequest) -> ExecutionResult {
    let target = request.resume_point().artifact_id().clone();

    ExecutionResult::Refused {
        reason: format!(
            "Required canonical evidence is absent: no executable target is recorded for \
             Artifact {target}. The current canonical model stores Artifact identity only \
             (IS-0004), no canonical Artifact-to-resource mapping exists, and no frozen \
             Restoration Execution specification defines how a Restoration Plan is performed \
             (IS-0019 §11; IS-0021 §20). No restoration action was performed."
        ),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use evo_workspace::attachment::{Attachment, ResourceRole};
    use evo_workspace::confidence::ConfidenceScore;
    use evo_workspace::lifecycle::WorkspaceLifecycle;
    use evo_workspace::snapshot::Snapshot;
    use evo_workspace::workspace::Workspace;
    use evo_workspace::WorkspaceId;

    use crate::context_chain::ContextChain;
    use crate::derivation::derive_restoration_plan;
    use crate::next_step::NextStep;
    use crate::RestorationInput;

    use std::time::UNIX_EPOCH;

    fn attachment(id: &str) -> Attachment {
        Attachment::new(
            ArtifactId::new(id).unwrap(),
            ConfidenceScore::new(1.0).unwrap(),
            ResourceRole::Supporting,
        )
    }

    fn snapshot(attachments: Vec<Attachment>) -> Snapshot {
        Snapshot::new(UNIX_EPOCH, WorkspaceLifecycle::Active, attachments)
    }

    fn workspace(snapshots: Vec<Snapshot>) -> Workspace {
        let attachments = snapshots
            .last()
            .map(|snapshot| snapshot.attachments().to_vec())
            .unwrap_or_default();
        Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            attachments,
            snapshots,
        )
    }

    fn derive(workspace: &Workspace, snapshot: &Snapshot) -> DerivationOutcome {
        let input = RestorationInput::new(workspace, snapshot)
            .expect("fixture snapshot must belong to the fixture workspace");
        derive_restoration_plan(&input)
    }

    fn single_artifact_outcome() -> DerivationOutcome {
        let snapshot = snapshot(vec![attachment("artifact-single")]);
        let workspace = workspace(vec![snapshot.clone()]);
        derive(&workspace, &snapshot)
    }

    fn multi_artifact_outcome() -> DerivationOutcome {
        let snapshot = snapshot(vec![
            attachment("artifact-alpha"),
            attachment("artifact-beta"),
        ]);
        let workspace = workspace(vec![snapshot.clone()]);
        derive(&workspace, &snapshot)
    }

    fn valid_plan() -> RestorationPlan {
        let wid = WorkspaceId::new();
        let resume_point = ResumePoint::new(wid.clone(), ArtifactId::new("artifact-plan").unwrap());
        let context_chain = ContextChain::new(vec![]).unwrap();
        let blockers = vec![];
        let next_step = NextStep::new(wid.clone(), "continue after the resume point").unwrap();
        RestorationPlan::new(wid, resume_point, context_chain, blockers, next_step).unwrap()
    }

    #[test]
    fn from_derivation_accepts_single_artifact_outcome() {
        let outcome = single_artifact_outcome();
        let request = ExecutionRequest::from_derivation(&outcome)
            .expect("a derived Resume Point is a valid execution request");
        assert_eq!(
            request.resume_point().artifact_id().as_str(),
            "artifact-single"
        );
        assert!(request.plan().is_none());
    }

    #[test]
    fn from_derivation_accepts_complete_plan() {
        let plan = valid_plan();
        let outcome = DerivationOutcome::Complete(plan.clone());
        let request = ExecutionRequest::from_derivation(&outcome)
            .expect("a complete Restoration Plan is a valid execution request");
        assert_eq!(request.plan(), Some(&plan));
        assert_eq!(
            request.resume_point().artifact_id(),
            plan.resume_point().artifact_id()
        );
    }

    #[test]
    fn from_derivation_refuses_without_derived_resume_point() {
        let outcome = multi_artifact_outcome();
        let error = ExecutionRequest::from_derivation(&outcome)
            .expect_err("no Resume Point means no valid execution request");
        assert!(matches!(
            error,
            ExecutionError::NoDerivedResumePoint { .. }
        ));
        assert!(error.to_string().contains("25.2"));
    }

    #[test]
    fn execute_refuses_and_names_the_missing_canonical_evidence() {
        let outcome = single_artifact_outcome();
        let request = ExecutionRequest::from_derivation(&outcome).unwrap();
        let result = execute(&request);

        let ExecutionResult::Refused { reason } = &result else {
            panic!("execution must refuse under the current canonical model");
        };
        assert!(reason.contains("artifact-single"));
        assert!(reason.contains("no executable target"));
        assert!(reason.contains("IS-0004"));
        assert!(reason.contains("IS-0019 §11"));
        assert!(reason.contains("IS-0021 §20"));
        assert!(reason.contains("No restoration action was performed"));
    }

    #[test]
    fn execute_never_returns_executed_under_the_current_model() {
        // Both valid request shapes — a derived Resume Point and a complete
        // Plan — are refused. Evo never pretends that execution occurred.
        let resume_request =
            ExecutionRequest::from_derivation(&single_artifact_outcome()).unwrap();
        assert!(matches!(execute(&resume_request), ExecutionResult::Refused { .. }));

        let plan_request = ExecutionRequest::from_derivation(&DerivationOutcome::Complete(
            valid_plan(),
        ))
        .unwrap();
        assert!(matches!(execute(&plan_request), ExecutionResult::Refused { .. }));
    }

    #[test]
    fn execute_is_deterministic() {
        let request =
            ExecutionRequest::from_derivation(&single_artifact_outcome()).unwrap();
        let first = execute(&request);
        let second = execute(&request);
        assert_eq!(first, second);
    }

    #[test]
    fn execute_does_not_mutate_the_request() {
        let outcome = single_artifact_outcome();
        let request = ExecutionRequest::from_derivation(&outcome).unwrap();
        let original = request.clone();

        let _ = execute(&request);

        assert_eq!(request, original);
        assert_eq!(
            request.resume_point().artifact_id(),
            original.resume_point().artifact_id()
        );
    }

    #[test]
    fn execution_attempt_shape_reports_status_per_item() {
        // The Architecture's per-item reporting shape (ARCHITECTURE.md §5):
        // every attempted artifact reports success or failure explicitly.
        let opened = ExecutionAttempt::new(
            ArtifactId::new("artifact-opened").unwrap(),
            ExecutionStatus::Opened,
        );
        assert_eq!(opened.artifact_id().as_str(), "artifact-opened");
        assert_eq!(opened.status(), &ExecutionStatus::Opened);

        let failed = ExecutionAttempt::new(
            ArtifactId::new("artifact-failed").unwrap(),
            ExecutionStatus::Failed {
                reason: "target unavailable".to_string(),
            },
        );
        assert!(matches!(
            failed.status(),
            ExecutionStatus::Failed { reason } if reason == "target unavailable"
        ));
    }

    #[test]
    fn execution_error_displays_clearly() {
        let error = ExecutionError::NoDerivedResumePoint {
            reason: "no Resume Point was derived".to_string(),
        };
        assert!(error.to_string().contains("no valid derived execution request"));
        assert!(error.to_string().contains("no Resume Point was derived"));
    }
}
