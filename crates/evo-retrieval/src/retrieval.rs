//! Retrieval service boundary.
//!
//! The Retrieval service consumes one trigger and the current committed
//! Workspaces. RFC-0007 does not prescribe the retrieval algorithm, so this
//! crate defines only the public service surface required by the
//! architecture.

use crate::errors::RetrievalError;
use evo_workspace::{Workspace, WorkspaceId};

/// A retrieval service boundary.
///
/// `Retrieval` stores no resolver, no policy, and no derived state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Retrieval;

impl Retrieval {
    /// Resolves a trigger into zero or more candidate `WorkspaceId` values.
    ///
    /// RFC-0007 defines the boundary but not the retrieval algorithm. This
    /// method is intentionally unimplemented until a future architectural
    /// specification defines the behavior.
    pub fn retrieve(
        &self,
        trigger: impl Sized,
        current_committed_workspaces: &[Workspace],
    ) -> Result<Vec<WorkspaceId>, RetrievalError> {
        let _ = trigger;
        let _ = current_committed_workspaces;
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_workspace::{Workspace, WorkspaceLifecycle};

    fn workspace() -> Workspace {
        Workspace::new(
            WorkspaceId::new(),
            WorkspaceLifecycle::Active,
            vec![],
            vec![],
        )
    }

    #[test]
    fn unit_struct_is_constructible() {
        let retrieval = Retrieval;
        let _: Retrieval = retrieval;
    }

    #[test]
    fn retrieval_is_copy_clone_and_default() {
        let a = Retrieval;
        let b = a;
        let c = a.clone();
        let d: Retrieval = Default::default();

        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);
    }

    #[test]
    fn retrieval_can_hold_current_workspaces_without_executing() {
        let retrieval = Retrieval;
        let current_workspaces = vec![workspace()];
        let _ = (retrieval, current_workspaces.len());
    }
}
