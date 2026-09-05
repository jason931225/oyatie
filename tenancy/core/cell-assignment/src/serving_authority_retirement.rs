use crate::{BindingDigest32, BindingProofVerificationError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityIndependentRetirementPayloadV1 {
    pub schema_version: u32,
    pub claim_identity: crate::MigrationClaimIdentityV1,
    pub prior_instance: crate::ServingAuthorityInstanceV1,
    pub installation_issuance_digest: BindingDigest32,
    pub issuer_path_manifest_digest: BindingDigest32,
    pub issuer_path_count: u64,
    pub permanent_issuer_fence: cell_placement::ImmutableEvidenceRefV1,
    pub permanent_delayed_install_fence: cell_placement::ImmutableEvidenceRefV1,
    pub retirement_policy_digest: BindingDigest32,
    pub retired_at_unix_seconds: u64,
    pub retirement_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedServingAuthorityIndependentRetirementV1 {
    pub payload: ServingAuthorityIndependentRetirementPayloadV1,
    pub envelope: crate::BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityRetirementExpectationV1 {
    pub claim_identity: crate::MigrationClaimIdentityV1,
    pub prior_instance: crate::ServingAuthorityInstanceV1,
    pub installation_issuance_digest: BindingDigest32,
    pub issuer_path_manifest_digest: BindingDigest32,
    pub issuer_path_count: u64,
    pub retirement_policy_digest: BindingDigest32,
    pub expected_producer: crate::BindingProducerId,
    pub expected_audience: crate::BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthorityIndependentRetirement(
    SignedServingAuthorityIndependentRetirementV1,
);

impl VerifiedServingAuthorityIndependentRetirement {
    #[must_use]
    pub fn signed(&self) -> &SignedServingAuthorityIndependentRetirementV1 {
        &self.0
    }
}

pub fn verify_serving_authority_independent_retirement(
    _verifier: &dyn crate::BindingProofVerifier,
    _signed: SignedServingAuthorityIndependentRetirementV1,
    _expected: &ServingAuthorityRetirementExpectationV1,
) -> Result<VerifiedServingAuthorityIndependentRetirement, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServingAuthorityRetirementEvidenceV1 {
    LocalFreeze(Box<crate::ServingAuthorityFreezeResultV1>),
    Independent(Box<SignedServingAuthorityIndependentRetirementV1>),
}

#[derive(Debug, Eq, PartialEq)]
pub enum VerifiedServingAuthorityRetirementV1 {
    LocalFreeze(Box<crate::VerifiedServingAuthorityFreezeResult>),
    Independent(Box<VerifiedServingAuthorityIndependentRetirement>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityLeaseExpiryRecoveryBasisV1 {
    pub freeze_result: crate::ServingAuthorityFreezeResultV1,
    pub qualified_time_evidence: cell_placement::ImmutableEvidenceRefV1,
    pub source_isolation_proof: cell_placement::ImmutableEvidenceRefV1,
    pub clock_authority_digest: BindingDigest32,
    pub clock_uncertainty_bound_millis: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServingAuthorityRecoveryBasisV1 {
    LeaseExpiry(Box<ServingAuthorityLeaseExpiryRecoveryBasisV1>),
    IndependentPermanentFence {
        retirement: Box<SignedServingAuthorityIndependentRetirementV1>,
        effect_isolation_proof: cell_placement::ImmutableEvidenceRefV1,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedRetiredSourceEffectClosureV1 {
    retirement: ServingAuthorityRetirementEvidenceV1,
    completion: crate::VerifiedSourceFencingCompletionV1,
}

impl VerifiedRetiredSourceEffectClosureV1 {
    #[must_use]
    pub fn retirement(&self) -> &ServingAuthorityRetirementEvidenceV1 {
        &self.retirement
    }

    #[must_use]
    pub fn completion(&self) -> &crate::VerifiedSourceFencingCompletionV1 {
        &self.completion
    }
}

pub fn verify_retired_source_effect_closure(
    _retirement: &VerifiedServingAuthorityRetirementV1,
    _completion: crate::VerifiedSourceFencingCompletionV1,
    _expected: &ServingAuthorityRetirementExpectationV1,
) -> Result<VerifiedRetiredSourceEffectClosureV1, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}
