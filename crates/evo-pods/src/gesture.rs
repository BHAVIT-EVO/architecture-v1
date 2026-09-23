//! Gestures are declarations.
//!
//! The engine forbids inference-path merges; corrections need the user's
//! explicit word. Pods give that word a body: drag a window onto a pod,
//! drop one pod on another, eject a window. Three gestures, three
//! declarations — the physical act IS the correction, and because the
//! declaration flows into the engine's ordinary path, the UI learns
//! nothing on its own.

use crate::pod::Pod;
use evo_threads::Declaration;

/// A physical act on the bar (already resolved to pods/subjects by the UI).
#[derive(Debug, Clone, PartialEq)]
pub enum PodGesture {
    /// Drag a loose window's subject onto a pod card.
    WindowIntoPod {
        /// The window's strongest identity (document > page > title).
        subject: String,
        target_pod: u64,
        /// The pod's cited anchor (its identity carrier).
        target_anchor: Option<String>,
    },
    /// Drop one pod card onto another: "these are one work".
    MergePods {
        source_pod: u64,
        source_anchor: Option<String>,
        target_pod: u64,
        target_anchor: Option<String>,
    },
    /// Drag one window's subject off its pod: "this is not this work".
    EjectWindow {
        subject: String,
        /// The pod it was ejected from (cited as the other side);
        /// absent anchors turn ejection into quarantine instead.
        source_pod: u64,
        source_anchor: Option<String>,
    },
}

/// Honest outcomes: either a declaration the engine can ingest, or an
/// explicit reason the gesture has no binding — never a silent guess.
#[derive(Debug, Clone, PartialEq)]
pub enum GestureOutcome {
    Declare(Declaration),
    /// No anchor to bind against: the pod has no identity carrier, so a
    /// merge would be a guess. The bar should say precisely this.
    Unbound(String),
}

pub fn gesture_to_declaration(gesture: &PodGesture) -> GestureOutcome {
    match gesture {
        PodGesture::WindowIntoPod {
            subject,
            target_anchor,
            ..
        } => match target_anchor {
            Some(anchor) => GestureOutcome::Declare(Declaration::SameWork {
                a: subject.clone(),
                b: anchor.clone(),
            }),
            None => GestureOutcome::Unbound(
                "the pod has no anchor yet; gain one (produce something in it) before binding"
                    .into(),
            ),
        },
        PodGesture::MergePods {
            source_anchor,
            target_anchor,
            ..
        } => match (source_anchor, target_anchor) {
            (Some(a), Some(b)) if a == b => GestureOutcome::Unbound(
                "both pods carry the same anchor; the engine already reads them as one".into(),
            ),
            (Some(a), Some(b)) => GestureOutcome::Declare(Declaration::SameWork {
                a: a.clone(),
                b: b.clone(),
            }),
            _ => GestureOutcome::Unbound(
                "a pod without an anchor can't be merged by declaration; produce in it first"
                    .into(),
            ),
        },
        PodGesture::EjectWindow {
            subject,
            source_anchor,
            ..
        } => match source_anchor {
            Some(anchor) if anchor != subject => GestureOutcome::Declare(Declaration::SeparateWork {
                a: subject.clone(),
                b: anchor.clone(),
            }),
            // No counter-side to separate from: quarantine is the honest
            // fallback — the subject leaves every view.
            _ => GestureOutcome::Declare(Declaration::NotMine {
                resource: subject.clone(),
            }),
        },
    }
}

/// The UI's strongest identity for a loose window, for gesture purposes.
/// Shapes only, never apps: document, then page, then title.
pub fn strongest_identity(
    ax_document: Option<&str>,
    title: &str,
) -> Option<String> {
    if let Some(doc) = ax_document {
        if !doc.is_empty() {
            return Some(doc.to_string());
        }
    }
    if !title.is_empty() {
        return Some(title.to_string());
    }
    None
}

/// Card-level merge availability test for UI affordance (drag hover).
pub fn merge_ready(source: &Pod, target: &Pod) -> bool {
    source.strongest_anchor.is_some()
        && target.strongest_anchor.is_some()
        && source.id != target.id
}
