use crate::{
    BindingDigest32, BindingProofEnvelopeV1, ServingAuthorityBusinessIdV1,
    ServingAuthorityInstanceV1, TenantControlPartitionRefV1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityInstallationIssuanceV1 {
    pub control_partition: TenantControlPartitionRefV1,
    pub instance: ServingAuthorityInstanceV1,
    pub business: ServingAuthorityBusinessIdV1,
    pub participant_manifest_digest: BindingDigest32,
    pub previous_authority_closure_digest: Option<BindingDigest32>,
    pub revision: u64,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityControlCommitAttestationPayloadV1 {
    pub schema_version: u32,
    pub control_partition: TenantControlPartitionRefV1,
    pub instance: ServingAuthorityInstanceV1,
    pub business: ServingAuthorityBusinessIdV1,
    pub committed_binding_digest: BindingDigest32,
    pub committed_issuance_revision: u64,
    pub committed_issuance_digest: BindingDigest32,
    pub committed_transaction_digest: BindingDigest32,
    pub committed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedServingAuthorityControlCommitAttestationV1 {
    pub payload: ServingAuthorityControlCommitAttestationPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedServingAuthorityInstallationClaimV1 {
    pub issuance: ServingAuthorityInstallationIssuanceV1,
    pub attestation: SignedServingAuthorityControlCommitAttestationV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedServingAuthorityInstallation(
    CommittedServingAuthorityInstallationClaimV1,
);

impl VerifiedCommittedServingAuthorityInstallation {
    #[must_use]
    pub fn claim(&self) -> &CommittedServingAuthorityInstallationClaimV1 {
        &self.0
    }
}

pub fn verify_committed_serving_authority_installation(
    _verifier: &dyn crate::BindingProofVerifier,
    _claim: CommittedServingAuthorityInstallationClaimV1,
    _expectation: &crate::ServingAuthorityHandoffExpectationV1,
) -> Result<VerifiedCommittedServingAuthorityInstallation, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedServingAuthorityInstallGrantV1 {
    pub committed: CommittedServingAuthorityInstallationClaimV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthorityInstallGrant(SignedServingAuthorityInstallGrantV1);

impl VerifiedServingAuthorityInstallGrant {
    #[must_use]
    pub fn signed(&self) -> &SignedServingAuthorityInstallGrantV1 {
        &self.0
    }
}

pub fn verify_serving_authority_install_grant(
    _verifier: &dyn crate::BindingProofVerifier,
    _signed: SignedServingAuthorityInstallGrantV1,
    _expectation: &crate::ServingAuthorityHandoffExpectationV1,
) -> Result<VerifiedServingAuthorityInstallGrant, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityInstallationWriteSetV1 {
    parts: ServingAuthorityInstallationWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityInstallationWriteSetPartsV1 {
    pub partition: crate::CellServingPartitionRefV1,
    pub instance: ServingAuthorityInstanceV1,
    pub authority: crate::VerifiedServingAuthorityInvocation,
    pub precondition: crate::ServingAuthorityLocalPreconditionV1,
    pub grant: VerifiedServingAuthorityInstallGrant,
    pub restore_basis: crate::ServingAuthorityRestoreBasisV1,
    pub installed: crate::InstalledServingAuthorityV1,
    pub next_rejection_high_water: crate::ServingAuthorityRejectionHighWaterV1,
    pub first_lease_state: crate::WriteAuthorityLeaseStateV1,
    pub first_lease_issuance: crate::WriteAuthorityLeaseIssuanceRecordV1,
    pub business: ServingAuthorityBusinessIdV1,
    pub result: ServingAuthorityInstallationResultPayloadV1,
    pub idempotency: crate::BindingIdempotencyRecordV1,
    pub proof_consumptions: Vec<crate::ServingAuthorityProofConsumptionV1>,
    pub audit_outbox: crate::BindingAuditRecordV1,
}

impl ServingAuthorityInstallationWriteSetV1 {
    pub fn assemble(
        _parts: ServingAuthorityInstallationWriteSetPartsV1,
    ) -> Result<Self, crate::ServingAuthorityStoreError> {
        Err(crate::ServingAuthorityStoreError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &ServingAuthorityInstallationWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityInstallationResultPayloadV1 {
    pub schema_version: u32,
    pub instance: ServingAuthorityInstanceV1,
    pub business: ServingAuthorityBusinessIdV1,
    pub installed: crate::InstalledServingAuthorityV1,
    pub first_lease_issuance_digest: BindingDigest32,
    pub local_state_revision: u64,
    pub committed_transaction_digest: BindingDigest32,
    pub result_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityInstallationResultV1 {
    pub payload: ServingAuthorityInstallationResultPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthorityInstallationResult(ServingAuthorityInstallationResultV1);

impl VerifiedServingAuthorityInstallationResult {
    #[must_use]
    pub fn signed(&self) -> &ServingAuthorityInstallationResultV1 {
        &self.0
    }
}

pub fn verify_serving_authority_installation_result(
    _verifier: &dyn crate::BindingProofVerifier,
    _signed: ServingAuthorityInstallationResultV1,
    _expectation: &crate::ServingAuthorityHandoffExpectationV1,
) -> Result<VerifiedServingAuthorityInstallationResult, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

pub trait ServingAuthorityInstallationGrantIssuer: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        committed: &'a VerifiedCommittedServingAuthorityInstallation,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<SignedServingAuthorityInstallGrantV1, crate::BindingStoreError>,
    >;
}
