use crate::{BindingDigest32, ServingAuthorityInstanceV1};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthoritySurvivingQuorumEvidenceV1 {
    pub instance: ServingAuthorityInstanceV1,
    pub committed_state_revision: u64,
    pub committed_state_digest: BindingDigest32,
    pub rejection_high_water_digest: BindingDigest32,
    pub quorum_evidence: cell_placement::ImmutableEvidenceRefV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityReplacementEvidenceV1 {
    pub prior_instance: ServingAuthorityInstanceV1,
    pub replacement_instance: ServingAuthorityInstanceV1,
    pub prior_installation_issuance_digest: BindingDigest32,
    pub independent_physical_fence: cell_placement::ImmutableEvidenceRefV1,
    pub prior_effect_path_fencing_digest: BindingDigest32,
    pub prior_installation_rejection_digest: BindingDigest32,
    pub recovery_authority_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthoritySurvivingQuorum(ServingAuthoritySurvivingQuorumEvidenceV1);

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthorityReplacement(ServingAuthorityReplacementEvidenceV1);

impl VerifiedServingAuthoritySurvivingQuorum {
    #[must_use]
    pub fn evidence(&self) -> &ServingAuthoritySurvivingQuorumEvidenceV1 {
        &self.0
    }
}

impl VerifiedServingAuthorityReplacement {
    #[must_use]
    pub fn evidence(&self) -> &ServingAuthorityReplacementEvidenceV1 {
        &self.0
    }
}

pub fn verify_serving_authority_surviving_quorum(
    _verifier: &dyn crate::BindingProofVerifier,
    _evidence: ServingAuthoritySurvivingQuorumEvidenceV1,
    _expected: &ServingAuthorityInstanceV1,
) -> Result<VerifiedServingAuthoritySurvivingQuorum, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

pub fn verify_serving_authority_replacement(
    _verifier: &dyn crate::BindingProofVerifier,
    _evidence: ServingAuthorityReplacementEvidenceV1,
    _expected: &ServingAuthorityInstanceV1,
    _effect_fencing: &crate::VerifiedSourceFencingCompletionV1,
) -> Result<VerifiedServingAuthorityReplacement, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServingAuthorityRestoreBasisV1 {
    FirstInstallation {
        surviving_partition: Box<VerifiedServingAuthoritySurvivingQuorum>,
        uninstalled_precondition_digest: BindingDigest32,
    },
    SurvivingQuorum(Box<VerifiedServingAuthoritySurvivingQuorum>),
    IndependentlyFencedReplacement(Box<VerifiedServingAuthorityReplacement>),
}
