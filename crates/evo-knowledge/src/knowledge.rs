//! knowledge.rs

use crate::constraint::Constraint;
use crate::knowledge_id::KnowledgeId;
use crate::revision_state::RevisionState;
use crate::supporting_evidence::SupportingEvidence;

/// A canonical Knowledge constraint derived from canonical evidence.
///
/// Follows IS-0015 §3 and §8.
/// Owns: KnowledgeId, Constraint, SupportingEvidence, RevisionState.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Knowledge {
    id: KnowledgeId,
    constraint: Constraint,
    supporting_evidence: SupportingEvidence,
    revision_state: RevisionState,
}

impl Knowledge {
    /// Constructs a new Knowledge object.
    pub fn new(
        id: KnowledgeId,
        constraint: Constraint,
        supporting_evidence: SupportingEvidence,
        revision_state: RevisionState,
    ) -> Self {
        Self {
            id,
            constraint,
            supporting_evidence,
            revision_state,
        }
    }

    pub fn id(&self) -> &KnowledgeId {
        &self.id
    }

    pub fn constraint(&self) -> &Constraint {
        &self.constraint
    }

    pub fn supporting_evidence(&self) -> &SupportingEvidence {
        &self.supporting_evidence
    }

    pub fn revision_state(&self) -> &RevisionState {
        &
        self.revision_state
    }

    /// Revises the current Knowledge.
    ///
    /// Follows IS-0015 §9 and KI-8:
    /// - Preserves KnowledgeId.
    /// - Replaces Constraint, SupportingEvidence, and RevisionState.
    pub fn revise(
        &self,
        new_constraint: Constraint,
        new_supporting_evidence: SupportingEvidence,
        new_revision_state: RevisionState,
    ) -> Self {
        Self {
            id: self.id.clone(),
            constraint: new_constraint,
            supporting_evidence: new_supporting_evidence,
            revision_state: new_revision_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_artifact::artifact_id::ArtifactId;
    use evo_observation::observation_id::ObservationId;

    fn constraint() -> Constraint {
        Constraint::new("test constraint content")
    }

    fn supporting_evidence() -> SupportingEvidence {
        SupportingEvidence::new(
            vec![ObservationId::new()],
            vec![ArtifactId::new("test-artifact").unwrap()],
        )
        .unwrap()
    }

    #[test]
    fn construction_and_accessors() {
        let id = KnowledgeId::new();
        let constr = constraint();
        let evidence = supporting_evidence();
        let state = RevisionState::Current;

        let knowledge = Knowledge::new(
            id.clone(),
            constr.clone(),
            evidence.clone(),
            state,
        );

        assert_eq!(knowledge.id(), &id);
        assert_eq!(knowledge.constraint(), &constr);
        assert_eq!(knowledge.supporting_evidence(), &evidence);
        assert_eq!(knowledge.revision_state(), &state);
    }

    #[test]
    fn clone_is_equal_to_original() {
        let knowledge = Knowledge::new(
            KnowledgeId::new(),
            constraint(),
            supporting_evidence(),
            RevisionState::Current,
        );
        let cloned = knowledge.clone();
        assert_eq!(knowledge, cloned);
    }

    #[test]
    fn equality_requires_matching_components() {
        let id = KnowledgeId::new();
        let constr = constraint();
        let evidence = supporting_evidence();
        
        let a = Knowledge::new(id.clone(), constr.clone(), evidence.clone(), RevisionState::Current);
        let b = Knowledge::new(id.clone(), constr.clone(), evidence.clone(), RevisionState::Current);
        let c = Knowledge::new(id, constr, evidence, RevisionState::Weakened);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn revision_preserves_identity_and_replaces_components() {
        let original_id = KnowledgeId::new();
        let knowledge = Knowledge::new(
            original_id.clone(),
            constraint(),
            supporting_evidence(),
            RevisionState::Current,
        );

        let new_constraint = Constraint::new("revised constraint");
        let new_evidence = SupportingEvidence::new(
            vec![ObservationId::new()],
            vec![]
        ).unwrap();
        let new_state = RevisionState::Weakened;

        let revised = knowledge.revise(
            new_constraint.clone(),
            new_evidence.clone(),
            new_state,
        );

        // Invariant KI-8: Revision preserves Knowledge identity
        assert_eq!(revised.id(), &original_id);
        assert_eq!(revised.id(), knowledge.id());

        // New components are correctly replaced
        assert_eq!(revised.constraint(), &new_constraint);
        assert_eq!(revised.supporting_evidence(), &new_evidence);
        assert_eq!(revised.revision_state(), &new_state);

        // Original object is untouched (Immutability check)
        assert_eq!(knowledge.revision_state(), &RevisionState::Current);
    }

    #[test]
    fn knowledge_is_immutable_after_construction() {
        let knowledge = Knowledge::new(
            KnowledgeId::new(),
            constraint(),
            supporting_evidence(),
            RevisionState::Current,
        );

        let _: &KnowledgeId = knowledge.id();
        let _: &Constraint = knowledge.constraint();
        let _: &SupportingEvidence = knowledge.supporting_evidence();
        let _: &RevisionState = knowledge.revision_state();
    }
}