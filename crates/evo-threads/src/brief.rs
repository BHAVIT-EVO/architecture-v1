//! The Work Brief: what a work *is*, assembled only from witnessed facts.
//!
//! Product statements:
//! * A brief carries the engine's own answer to "what is this work?" — its
//!   identity carriers (anchors), its recurring context (companions), where
//!   the person left off (the resume bundle), and what is still unresolved
//!   (open deltas). Nothing is invented; no wall clock is read (`settle`
//!   fixes "now"); every ordering is the deterministic one the views
//!   already use.
//! * The brief is the spoken form's *source of truth*, not its prose. Phase
//!   3's narration renders sentences from these facts; the engine ships
//!   facts, the voice ships words. Both stay testable apart.
//! * One capability per reader: `Engine::brief(thread_id)` answers "what is
//!   this?"; `Engine::briefs()` answers "what are all my works?" for the
//!   current top-of-rack moment. Unknown ids produce the empty brief, the
//!   same honest shape `resume_bundle` already returns for them.

use crate::types::{ResumeBundle, ThreadView};
use crate::{Engine, ThreadId};
use serde::{Deserialize, Serialize};

/// One identity carrier, with the kind the engine witnessed: what the
/// person *made* (Mutation) vs. what they deliberately *returned to*
/// (Recurrence). Narration needs the distinction; views already carry it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkAnchor {
    pub resource: String,
    pub kind: crate::types::AnchorKind,
}

/// A work, as the engine can honestly state it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkBrief {
    pub thread_id: ThreadId,
    /// Active or Dormant — the view's own word, not a fresh decision.
    pub status: crate::types::Status,
    /// Identity: what the person made or deliberately returned to,
    /// strongest first (the anchor order the views use).
    pub anchors: Vec<WorkAnchor>,
    /// Shared identity, displayed never hidden: anchors other live works
    /// also carry. Decides nothing; explains a lot.
    pub contested: Vec<String>,
    /// Recurring context, decayed, strongest first.
    pub companions: Vec<String>,
    /// Sitting count — how many distinct occasions the work spans.
    pub episodes: usize,
    /// Where you left off, and the one honest sentence about it.
    pub resume_point: String,
    pub resume_reason: String,
    /// The neighborhood to reopen, not the museum.
    pub restore_set: Vec<String>,
    /// Witnessed open loops, if any.
    pub open_deltas: Vec<crate::types::Delta>,
    /// Retained ambiguity: the forked episode's candidate works, if this
    /// work was never chosen between.
    pub fork: Option<Vec<ThreadId>>,
}

impl Engine {
    /// The brief of one work. Unknown id → the empty brief (resume point
    /// empty, reason "Last seen: nothing."), the same contract
    /// `resume_bundle` keeps.
    pub fn brief(&self, thread_id: ThreadId) -> WorkBrief {
        let bundle = self.resume_bundle(thread_id);
        let Some(view) = self.threads().into_iter().find(|t| t.id == thread_id) else {
            return WorkBrief {
                thread_id,
                status: crate::types::Status::Dormant,
                anchors: vec![],
                contested: vec![],
                companions: vec![],
                episodes: 0,
                resume_point: bundle.resume_point,
                resume_reason: bundle.resume_reason,
                restore_set: bundle.restore_set,
                open_deltas: bundle.open_deltas,
                fork: None,
            };
        };
        Self::brief_from_view(&view, bundle)
    }

    /// Every live work, best-supported first — the rack in one call.
    pub fn briefs(&self) -> Vec<WorkBrief> {
        self.threads()
            .into_iter()
            .map(|view| {
                let bundle = self.resume_bundle(view.id);
                Self::brief_from_view(&view, bundle)
            })
            .collect()
    }

    fn brief_from_view(view: &ThreadView, bundle: ResumeBundle) -> WorkBrief {
        WorkBrief {
            thread_id: view.id,
            status: view.status.clone(),
            anchors: view
                .anchors
                .iter()
                .map(|a| WorkAnchor {
                    resource: a.resource.clone(),
                    kind: a.kind,
                })
                .collect(),
            contested: view.contested.clone(),
            companions: view.companions.iter().map(|(r, _)| r.clone()).collect(),
            episodes: view.episodes.len(),
            resume_point: bundle.resume_point,
            resume_reason: bundle.resume_reason,
            restore_set: bundle.restore_set,
            open_deltas: bundle.open_deltas,
            fork: view.fork.clone(),
        }
    }
}
