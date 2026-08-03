//! supporting_evidence.rs

use crate::errors::KnowledgeError;
use evo_artifact::artifact_id::ArtifactId;
use evo_observation::observation_id::ObservationId;

/// Canonical references supporting a Knowledge constraint.
///
/// Follows IS-0015 §6:
/// - MAY reference Observations and Artifacts.
/// - SHALL NOT reference Workspaces, Snapshots, etc.
/// - SHALL NOT duplicate data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportingEvidence {
    observation_ids: Vec<ObservationId>,
    artifact_ids: Vec<ArtifactId>,
}

impl SupportingEvidence {
    /// Constructs SupportingEvidence.
    ///
    /// Follows IS-0015 KI-3: Knowledge never exists without supporting evidence.
    /// At least one observation or artifact reference is required.
    pub fn new(
        observation_ids: Vec<ObservationId>,
        artifact_ids: Vec<ArtifactId>,
    ) -> Result<Self, KnowledgeError> {
        if observation_ids.is_empty() && artifact_ids.is_empty() {
            return Err(KnowledgeError::NoSupportingEvidence);
        }
        for (i, candidate) in observation_ids.iter().enumerate() {
    for existing in observation_ids[..i].iter() {
        if existing == candidate {
            return Err(KnowledgeError::DuplicateObservationEvidence {
                observation_id: candidate.clone(),
            });
        }
    }
}
    for (i, candidate) in artifact_ids.iter().enumerate() {
    for existing in artifact_ids[..i].iter() {
        if existing == candidate {
            return Err(KnowledgeError::DuplicateArtifactEvidence {
                artifact_id: candidate.clone(),
            });
        }
    }
}

        Ok(Self {
            observation_ids,
            artifact_ids,
        })
    }

    pub fn observation_ids(&self) -> &[ObservationId] {
        &self.observation_ids
    }

    pub fn artifact_ids(&self) -> &[ArtifactId] {
        &self.artifact_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_artifact::artifact_id::ArtifactId;
    use evo_observation::observation_id::ObservationId;    

    // Mock IDs for testing since we can't import the actual constructors
    // easily without full crate dependencies set up in this test environment.
    // In a real environment, we'd use ObservationId::new() and ArtifactId::new().
    // Assuming ObservationId and ArtifactId have a way to be mocked or constructed.
    // For the sake of this isolated test, we assume they can be represented.

    // Note: To make this compile as a standalone snippet assuming evo_* dependencies,
    // we would need those crates. Assuming they exist and provide standard UUID/String wrappers.

    #[test]
    fn empty_evidence_is_rejected() {
        let result = SupportingEvidence::new(vec![], vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), KnowledgeError::NoSupportingEvidence);
    }

    fn obs_id() -> ObservationId {
        ObservationId::new()
    }

    fn art_id(name: &str) -> ArtifactId {
        ArtifactId::new(name).unwrap()
    }

    #[test]
    fn construction_and_accessors() {
        let oid = obs_id();
        let aid = art_id("artifact-1");
        
        let evidence = SupportingEvidence::new(vec![oid.clone()], vec![aid.clone()]).unwrap();
        
        assert_eq!(evidence.observation_ids().len(), 1);
        assert_eq!(evidence.observation_ids()[0], oid);
        assert_eq!(evidence.artifact_ids().len(), 1);
        assert_eq!(evidence.artifact_ids()[0], aid);
    }

    #[test]
    fn construction_with_only_observations_is_valid() {
        let evidence = SupportingEvidence::new(vec![obs_id()], vec![]).unwrap();
        assert_eq!(evidence.observation_ids().len(), 1);
        assert!(evidence.artifact_ids().is_empty());
    }

    #[test]
    fn construction_with_only_artifacts_is_valid() {
        let evidence = SupportingEvidence::new(vec![], vec![art_id("artifact-2")]).unwrap();
        assert!(evidence.observation_ids().is_empty());
        assert_eq!(evidence.artifact_ids().len(), 1);
    }

    #[test]
    fn duplicate_observation_evidence_is_rejected() {
        let oid = obs_id();
        let result = SupportingEvidence::new(vec![oid.clone(), oid.clone()], vec![]);
        assert_eq!(
            result.unwrap_err(),
            KnowledgeError::DuplicateObservationEvidence {
                observation_id: oid
            }
        );
    }

    #[test]
    fn duplicate_artifact_evidence_is_rejected() {
        let aid = art_id("artifact-duplicate");
        let result = SupportingEvidence::new(vec![], vec![aid.clone(), aid.clone()]);
        assert_eq!(
            result.unwrap_err(),
            KnowledgeError::DuplicateArtifactEvidence {
                artifact_id: aid
            }
        );
    }

    #[test]
    fn clone_is_equal_to_original() {
        let evidence = SupportingEvidence::new(vec![obs_id()], vec![art_id("artifact-3")]).unwrap();
        let cloned = evidence.clone();
        assert_eq!(evidence, cloned);
    }

    #[test]
    fn supporting_evidence_is_immutable_after_construction() {
        let evidence = SupportingEvidence::new(vec![obs_id()], vec![art_id("artifact-5")]).unwrap();
        
        let _: &[ObservationId] = evidence.observation_ids();
        let _: &[ArtifactId] = evidence.artifact_ids();
    }
}