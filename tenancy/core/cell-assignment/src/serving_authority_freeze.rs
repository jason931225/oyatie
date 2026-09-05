use crate::{
    BindingDigest32, BindingProofEnvelopeV1, ServingAuthorityBusinessIdV1,
    ServingAuthorityInstanceV1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityFreezeIntentV1 {
    pub control_partition: crate::TenantControlPartitionRefV1,
    pub instance: ServingAuthorityInstanceV1,
    pub installation_issuance_digest: BindingDigest32,
    pub business: ServingAuthorityBusinessIdV1,
    pub migration_claim_digest: BindingDigest32,
    pub successor_generation: crate::BindingGeneration,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedServingAuthorityFreezeClaimV1 {
    pub intent: ServingAuthorityFreezeIntentV1,
    pub attestation: crate::SignedServingAuthorityControlCommitAttestationV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedServingAuthorityFreeze(CommittedServingAuthorityFreezeClaimV1);

impl VerifiedCommittedServingAuthorityFreeze {
    #[must_use]
    pub fn claim(&self) -> &CommittedServingAuthorityFreezeClaimV1 {
        &self.0
    }
}

pub fn verify_committed_serving_authority_freeze(
    _verifier: &dyn crate::BindingProofVerifier,
    _claim: CommittedServingAuthorityFreezeClaimV1,
    _expectation: &crate::ServingAuthorityHandoffExpectationV1,
) -> Result<VerifiedCommittedServingAuthorityFreeze, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedServingAuthorityFreezeGrantV1 {
    pub committed: CommittedServingAuthorityFreezeClaimV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthorityFreezeGrant(SignedServingAuthorityFreezeGrantV1);

impl VerifiedServingAuthorityFreezeGrant {
    #[must_use]
    pub fn signed(&self) -> &SignedServingAuthorityFreezeGrantV1 {
        &self.0
    }
}

pub fn verify_serving_authority_freeze_grant(
    _verifier: &dyn crate::BindingProofVerifier,
    _signed: SignedServingAuthorityFreezeGrantV1,
    _expectation: &crate::ServingAuthorityHandoffExpectationV1,
) -> Result<VerifiedServingAuthorityFreezeGrant, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityRejectionV1 {
    pub instance: ServingAuthorityInstanceV1,
    pub installation_issuance_digest: BindingDigest32,
    pub freeze_intent_digest: BindingDigest32,
    pub revision: u64,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServingAuthorityFrozenStateV1 {
    Installed {
        authority: Box<crate::InstalledServingAuthorityV1>,
        frozen_lease_state: Box<crate::WriteAuthorityLeaseStateV1>,
        rejection: ServingAuthorityRejectionV1,
    },
    RejectedBeforeInstallation(ServingAuthorityRejectionV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServingAuthorityCommittedIssuanceHorizonV1 {
    NeverInstalled {
        rejection_digest: BindingDigest32,
    },
    Issued {
        issuance_root_digest: BindingDigest32,
        issuance_count: u64,
        maximum_expires_at_unix_seconds: u64,
        frozen_lease_state_digest: BindingDigest32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityFreezeResultPayloadV1 {
    pub schema_version: u32,
    pub instance: ServingAuthorityInstanceV1,
    pub business: ServingAuthorityBusinessIdV1,
    pub installation_issuance_digest: BindingDigest32,
    pub freeze_intent_digest: BindingDigest32,
    pub frozen_state: ServingAuthorityFrozenStateV1,
    pub complete_committed_horizon: ServingAuthorityCommittedIssuanceHorizonV1,
    pub rejection_high_water: crate::ServingAuthorityRejectionHighWaterV1,
    pub committed_transaction_digest: BindingDigest32,
    pub result_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityFreezeResultV1 {
    pub payload: ServingAuthorityFreezeResultPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthorityFreezeResult(ServingAuthorityFreezeResultV1);

impl VerifiedServingAuthorityFreezeResult {
    #[must_use]
    pub fn signed(&self) -> &ServingAuthorityFreezeResultV1 {
        &self.0
    }
}

pub fn verify_serving_authority_freeze_result(
    _verifier: &dyn crate::BindingProofVerifier,
    _signed: ServingAuthorityFreezeResultV1,
    _expectation: &crate::ServingAuthorityHandoffExpectationV1,
) -> Result<VerifiedServingAuthorityFreezeResult, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityFreezeWriteSetV1 {
    parts: ServingAuthorityFreezeWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityFreezeWriteSetPartsV1 {
    pub partition: crate::CellServingPartitionRefV1,
    pub instance: ServingAuthorityInstanceV1,
    pub authority: crate::VerifiedServingAuthorityInvocation,
    pub precondition: crate::ServingAuthorityLocalPreconditionV1,
    pub grant: VerifiedServingAuthorityFreezeGrant,
    pub next_state: ServingAuthorityFrozenStateV1,
    pub next_rejection_high_water: crate::ServingAuthorityRejectionHighWaterV1,
    pub business: ServingAuthorityBusinessIdV1,
    pub result: ServingAuthorityFreezeResultPayloadV1,
    pub idempotency: crate::BindingIdempotencyRecordV1,
    pub proof_consumptions: Vec<crate::ServingAuthorityProofConsumptionV1>,
    pub audit_outbox: crate::BindingAuditRecordV1,
}

impl ServingAuthorityFreezeWriteSetV1 {
    pub fn assemble(
        _parts: ServingAuthorityFreezeWriteSetPartsV1,
    ) -> Result<Self, crate::ServingAuthorityStoreError> {
        Err(crate::ServingAuthorityStoreError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &ServingAuthorityFreezeWriteSetPartsV1 {
        &self.parts
    }
}

pub trait ServingAuthorityFreezeGrantIssuer: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        committed: &'a VerifiedCommittedServingAuthorityFreeze,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<SignedServingAuthorityFreezeGrantV1, crate::BindingStoreError>,
    >;
}
