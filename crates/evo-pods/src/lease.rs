//! The lease: a receipt for every change an activation made, replayed in
//! exact reverse order when the pod is left. "Nothing else is touched"
//! is testable: the reversal plan contains exactly the inverse verbs of
//! the activation plan, in reverse.

use crate::dim::Arrangement;
use crate::surface::Frame;

/// One undoable unit of a deactivation.
#[derive(Debug, Clone, PartialEq)]
pub enum LeaseStep {
    MoveWindowBack { pid: i32, window_id: u32, frame: Frame },
    UnparkWindow { pid: i32, window_id: u32 },
    /// A pod member we resurrected at entry goes back to minimized so the
    /// desktop returns EXACTLY as it was (Stage mode's space keeps its
    /// own windows parked inside it).
    ReparkWindow { pid: i32, window_id: u32 },
    CloseVeil { handle: u64 },
    RehideApp { pid: i32 },
    UnhideApp { pid: i32 },
}

/// The receipt from one activation.
#[derive(Debug, Clone, PartialEq)]
pub struct PodLease {
    /// The pod name (for messages).
    pub pod: String,
    /// Windows moved from their original frames (recorded at apply time).
    pub moved_windows: Vec<(i32, u32, Frame)>,
    /// Foreign windows we minimized.
    pub parked_windows: Vec<(i32, u32)>,
    /// Veil handles we opened.
    pub veils: Vec<u64>,
    /// Foreign apps we hid.
    pub hidden_apps: Vec<i32>,
    /// Pod apps we un-hid; leaving re-hides them so the desktop returns.
    pub unhidden_apps: Vec<i32>,
    /// Pod members we pulled out of the Dock onto the stage; leaving
    /// re-minimizes them so the space appears to keep its windows.
    pub unparked_members: Vec<(i32, u32)>,
    /// Epoch milliseconds at entry.
    pub entered_at_ms: u64,
}

impl PodLease {
    pub fn new(pod: impl Into<String>, entered_at_ms: u64) -> Self {
        Self {
            pod: pod.into(),
            moved_windows: Vec::new(),
            parked_windows: Vec::new(),
            veils: Vec::new(),
            hidden_apps: Vec::new(),
            unhidden_apps: Vec::new(),
            unparked_members: Vec::new(),
            entered_at_ms,
        }
    }

    /// The exact reverse replay. Order: unveil → unhide/re-hide balance →
    /// unpark foreign → restore moved frames → re-park members — moving
    /// is done while everything is visible (a minimized window cannot be
    /// repositioned), then the space's windows return into the space.
    pub fn reversal(&self) -> Vec<LeaseStep> {
        let mut steps = Vec::new();
        for handle in self.veils.iter().rev() {
            steps.push(LeaseStep::CloseVeil { handle: *handle });
        }
        for pid in self.hidden_apps.iter().rev() {
            steps.push(LeaseStep::UnhideApp { pid: *pid });
        }
        for pid in self.unhidden_apps.iter().rev() {
            steps.push(LeaseStep::RehideApp { pid: *pid });
        }
        for (pid, window_id) in self.parked_windows.iter().rev() {
            steps.push(LeaseStep::UnparkWindow {
                pid: *pid,
                window_id: *window_id,
            });
        }
        for (pid, window_id, frame) in self.moved_windows.iter().rev() {
            steps.push(LeaseStep::MoveWindowBack {
                pid: *pid,
                window_id: *window_id,
                frame: *frame,
            });
        }
        for (pid, window_id) in self.unparked_members.iter().rev() {
            steps.push(LeaseStep::ReparkWindow {
                pid: *pid,
                window_id: *window_id,
            });
        }
        steps
    }

    /// Bookkeeps one applied arrangement: remember the pre-move frame if
    /// the window was actually repositioned.
    pub fn record_arrangement(&mut self, a: &Arrangement, prior: Option<Frame>) {
        if let Some(prior) = prior {
            if a.frame.is_some() {
                self.moved_windows.push((a.pid, a.window_id, prior));
            }
        }
    }
}
