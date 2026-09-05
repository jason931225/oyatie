use crate::{
    BindingDigest32, BindingIdempotencyKey, BindingIdempotencyRecordV1, BindingOperationKey,
    BoxTenancyFuture, ServingAuthorityStoreError, VerifiedServingAuthorityInvocation,
    WriteAuthorityLeaseIssuanceRecordV1, WriteAuthorityLeaseStateV1,
};

#[derive(Debug, Eq, PartialEq)]
pub struct RenewWriteAuthorityLeaseRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_authority: crate::InstalledServingAuthorityV1,
    pub expected_lease_state: crate::WriteAuthorityLeaseStatePreconditionV1,
    pub requested_validity_seconds: u64,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseRenewalWriteSetV1 {
    parts: WriteAuthorityLeaseRenewalWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseRenewalWriteSetPartsV1 {
    pub authority: VerifiedServingAuthorityInvocation,
    pub partition: crate::CellServingPartitionRefV1,
    pub installed_precondition: crate::InstalledServingAuthorityV1,
    pub lease_state_precondition: crate::WriteAuthorityLeaseStatePreconditionV1,
    pub next_lease_state: WriteAuthorityLeaseStateV1,
    pub lease_issuance: WriteAuthorityLeaseIssuanceRecordV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub audit_outbox: crate::BindingAuditRecordV1,
    pub proof_consumptions: Vec<crate::ServingAuthorityProofConsumptionV1>,
}

impl WriteAuthorityLeaseRenewalWriteSetV1 {
    pub fn assemble(
        _parts: WriteAuthorityLeaseRenewalWriteSetPartsV1,
    ) -> Result<Self, ServingAuthorityStoreError> {
        Err(ServingAuthorityStoreError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &WriteAuthorityLeaseRenewalWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseRenewalResultV1 {
    pub lease_issuance: WriteAuthorityLeaseIssuanceRecordV1,
    pub state: WriteAuthorityLeaseStateV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PublishWriteAuthorityLeaseRequestV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub operation: BindingOperationKey,
    pub lease_digest: BindingDigest32,
    pub expected_issuance: crate::WriteAuthorityLeaseIssuancePreconditionV1,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedWriteAuthorityLeaseIssuanceQueryV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub lease_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeasePublicationWriteSetV1 {
    parts: WriteAuthorityLeasePublicationWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeasePublicationWriteSetPartsV1 {
    pub authority: VerifiedServingAuthorityInvocation,
    pub partition: crate::CellServingPartitionRefV1,
    pub installed_precondition: crate::InstalledServingAuthorityV1,
    pub publication_lease: crate::ServingAuthorityPublicationLeaseV1,
    pub issuance_precondition: crate::WriteAuthorityLeaseIssuancePreconditionV1,
    pub committed_issuance: crate::VerifiedCommittedWriteAuthorityLeaseIssuance,
    pub lease: crate::VerifiedWriteAuthorityLease,
    pub published_issuance: WriteAuthorityLeaseIssuanceRecordV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub audit_outbox: crate::BindingAuditRecordV1,
    pub proof_consumptions: Vec<crate::ServingAuthorityProofConsumptionV1>,
}

impl WriteAuthorityLeasePublicationWriteSetV1 {
    pub fn assemble(
        _parts: WriteAuthorityLeasePublicationWriteSetPartsV1,
    ) -> Result<Self, ServingAuthorityStoreError> {
        Err(ServingAuthorityStoreError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &WriteAuthorityLeasePublicationWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PublishedWriteAuthorityLeaseV1 {
    pub lease: crate::SignedWriteAuthorityLeaseV1,
    pub issuance: WriteAuthorityLeaseIssuanceRecordV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeasePublicationResultV1 {
    pub published: PublishedWriteAuthorityLeaseV1,
    pub completed_publication_lease: crate::ServingAuthorityPublicationLeaseV1,
}

pub trait WriteAuthorityLeaseAuthority: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        issuance: &'a crate::VerifiedCommittedWriteAuthorityLeaseIssuance,
    ) -> BoxTenancyFuture<'a, Result<crate::VerifiedWriteAuthorityLease, ServingAuthorityStoreError>>;
}

pub trait TenancyWriteAuthorityLeaseService: Send + Sync {
    fn get_latest_published_write_authority_lease<'a>(
        &'a self,
        invocation: VerifiedServingAuthorityInvocation,
        instance: &'a crate::ServingAuthorityInstanceV1,
        minimum_valid_until_unix_seconds: u64,
    ) -> BoxTenancyFuture<'a, Result<PublishedWriteAuthorityLeaseV1, ServingAuthorityStoreError>>;

    fn renew_write_authority_lease<'a>(
        &'a self,
        invocation: VerifiedServingAuthorityInvocation,
        request: RenewWriteAuthorityLeaseRequestV1,
    ) -> BoxTenancyFuture<'a, Result<WriteAuthorityLeaseRenewalResultV1, ServingAuthorityStoreError>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NotImplementedTenancyWriteAuthorityLeaseService;

impl TenancyWriteAuthorityLeaseService for NotImplementedTenancyWriteAuthorityLeaseService {
    fn get_latest_published_write_authority_lease<'a>(
        &'a self,
        _: VerifiedServingAuthorityInvocation,
        _: &'a crate::ServingAuthorityInstanceV1,
        _: u64,
    ) -> BoxTenancyFuture<'a, Result<PublishedWriteAuthorityLeaseV1, ServingAuthorityStoreError>>
    {
        Box::pin(async { Err(ServingAuthorityStoreError::NotImplemented) })
    }

    fn renew_write_authority_lease<'a>(
        &'a self,
        _: VerifiedServingAuthorityInvocation,
        _: RenewWriteAuthorityLeaseRequestV1,
    ) -> BoxTenancyFuture<'a, Result<WriteAuthorityLeaseRenewalResultV1, ServingAuthorityStoreError>>
    {
        Box::pin(async { Err(ServingAuthorityStoreError::NotImplemented) })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NotImplementedWriteAuthorityLeaseAuthority;

impl WriteAuthorityLeaseAuthority for NotImplementedWriteAuthorityLeaseAuthority {
    fn sign_committed<'a>(
        &'a self,
        _: &'a crate::VerifiedCommittedWriteAuthorityLeaseIssuance,
    ) -> BoxTenancyFuture<'a, Result<crate::VerifiedWriteAuthorityLease, ServingAuthorityStoreError>>
    {
        Box::pin(async { Err(ServingAuthorityStoreError::NotImplemented) })
    }
}
