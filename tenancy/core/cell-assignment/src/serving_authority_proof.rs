use crate::{BindingDigest32, BindingProofEnvelopeV1, ServingAuthorityInstanceV1};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServingAuthorityProofKindV1 {
    Invocation,
    InstallGrant,
    FreezeGrant,
    LocalLease,
    LocalLeaseCommit,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityProofConsumptionV1 {
    instance: ServingAuthorityInstanceV1,
    kind: ServingAuthorityProofKindV1,
    envelope: BindingProofEnvelopeV1,
    business_digest: BindingDigest32,
}

impl ServingAuthorityProofConsumptionV1 {
    #[must_use]
    pub fn instance(&self) -> &ServingAuthorityInstanceV1 {
        &self.instance
    }
    #[must_use]
    pub fn kind(&self) -> ServingAuthorityProofKindV1 {
        self.kind
    }
    #[must_use]
    pub fn envelope(&self) -> &BindingProofEnvelopeV1 {
        &self.envelope
    }
    #[must_use]
    pub fn business_digest(&self) -> BindingDigest32 {
        self.business_digest
    }
}

pub enum VerifiedServingAuthorityProofRefV1<'a> {
    Invocation(&'a crate::VerifiedServingAuthorityInvocation),
    InstallGrant(&'a crate::VerifiedServingAuthorityInstallGrant),
    FreezeGrant(&'a crate::VerifiedServingAuthorityFreezeGrant),
    LocalLease(&'a crate::VerifiedWriteAuthorityLease),
    LocalLeaseCommit(&'a crate::VerifiedCommittedWriteAuthorityLeaseIssuance),
}

pub fn bind_serving_authority_proof_consumption(
    _proof: VerifiedServingAuthorityProofRefV1<'_>,
    _instance: &ServingAuthorityInstanceV1,
    _business_digest: BindingDigest32,
) -> Result<ServingAuthorityProofConsumptionV1, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}
