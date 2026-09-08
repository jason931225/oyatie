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

/// # External arrival: no local producer is required
///
/// `SignedServingAuthorityIndependentRetirementV1` has a verifier, a proof
/// domain and consumers in this crate, and is returned by nothing here. That is
/// the invocation pattern, not the missing-producer defect, and the payload is
/// what establishes it: its subject is the PRIOR authority's retirement.
/// `ServingAuthorityReplacementEvidenceV1` names it beside `prior_instance`,
/// explicitly qualified against `replacement_instance`, so the state attested
/// is another party's and the own-write rule does not bite.
///
/// "Independent" is also load-bearing: the point of this artifact is that
/// neither the authority being retired nor the replacement attests it. A local
/// producer in this crate would defeat that, since the only local parties are
/// exactly those two.
///
/// CONFIDENCE. This rests on the payload's subject, which is the same test that
/// settled the surviving quorum the other way. It does NOT rest on issuer
/// symmetry with the install and freeze grant issuers; that symmetry is real
/// but those attest this store's own committed control rows, and this one
/// does not, so the asymmetry is the correct outcome rather than an omission.
/// What would overturn this: any evidence that the retiring party is the same
/// control plane that performs the replacement write.
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
    LocalFreeze(Box<crate::SignedServingAuthorityFreezeResultV1>),
    Independent(Box<SignedServingAuthorityIndependentRetirementV1>),
}

#[derive(Debug, Eq, PartialEq)]
pub enum VerifiedServingAuthorityRetirementV1 {
    LocalFreeze(Box<crate::VerifiedServingAuthorityFreezeResult>),
    Independent(Box<VerifiedServingAuthorityIndependentRetirement>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityLeaseExpiryRecoveryBasisV1 {
    pub freeze_result: crate::SignedServingAuthorityFreezeResultV1,
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
