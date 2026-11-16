//! Trust, security and privacy (brief §9).
//!
//! Local-first, least privilege, off by default, and everything the user can
//! see and stop. Nothing in this module talks to the network and nothing here
//! reads audio: it only decides what Evo may do, and records what it did.
//!
//! - [`permissions`] — scopes, actions, modes, and the decision function.
//! - [`paths`] — canonical paths, traversal refusal, and shell-free commands.
//! - [`activity`] — the log of what happened, with undo where possible.

pub mod activity;
pub mod paths;
pub mod permissions;

use std::sync::atomic::{AtomicBool, Ordering};

/// The global kill switch. One switch, shared by every surface: the notch, the
/// menu bar, Settings and the shortcut all flip this. Paused denies every
/// request and stops capture at its source.
#[derive(Debug, Default)]
pub struct KillSwitch {
    paused: AtomicBool,
}

impl KillSwitch {
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Returns the new state, so a button can label itself from the answer.
    pub fn toggle(&self) -> bool {
        let next = !self.is_paused();
        self.paused.store(next, Ordering::SeqCst);
        next
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    /// The context the permission decision needs.
    pub fn context(&self) -> permissions::Context {
        permissions::Context {
            paused: self.is_paused(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use permissions::{Action, Decision, DenyReason, Mode, PermissionSet, Scope};

    #[test]
    fn the_kill_switch_denies_everything() {
        let switch = KillSwitch::new();
        let mut set = PermissionSet::new(Mode::ActWithinScope);
        set.allow(Scope::parse("folder", "/Users/me/Documents").unwrap());
        let target = "/Users/me/Documents/notes.md";

        assert_eq!(
            set.decide(switch.context(), Action::Open, target),
            Decision::Allow
        );
        assert!(switch.toggle(), "toggle returns the new state");
        assert_eq!(
            set.decide(switch.context(), Action::Open, target),
            Decision::Deny(DenyReason::Paused)
        );
        switch.resume();
        assert!(
            set.decide(switch.context(), Action::Open, target)
                .is_allowed()
        );
    }
}
