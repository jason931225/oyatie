use crate::{BindingDigest32, BoxTenancyFuture, ServingAuthorityStoreError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityPublicationLeaseV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub issuance_digest: BindingDigest32,
    pub worker_id: String,
    pub lease_epoch: u64,
    pub expires_at_unix_seconds: u64,
    pub lease_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityPendingIssuanceQueryV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub changed_before_unix_seconds: u64,
    pub maximum_page_size: u32,
    pub continuation: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityPendingIssuancePageV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub issuances: Vec<crate::WriteAuthorityLeaseIssuanceRecordV1>,
    pub next_continuation: Option<Vec<u8>>,
    pub retained_from_revision: u64,
    pub page_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ClaimServingAuthorityPublicationWriteSetV1 {
    parts: ClaimServingAuthorityPublicationWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ClaimServingAuthorityPublicationWriteSetPartsV1 {
    pub authority: crate::VerifiedServingAuthorityInvocation,
    pub installed: crate::InstalledServingAuthorityV1,
    pub issuance: crate::WriteAuthorityLeaseIssuancePreconditionV1,
    pub expected_lease_epoch: Option<u64>,
    pub next_lease: ServingAuthorityPublicationLeaseV1,
    pub audit_outbox: crate::BindingAuditRecordV1,
}

impl ClaimServingAuthorityPublicationWriteSetV1 {
    pub fn assemble(
        _parts: ClaimServingAuthorityPublicationWriteSetPartsV1,
    ) -> Result<Self, ServingAuthorityStoreError> {
        Err(ServingAuthorityStoreError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &ClaimServingAuthorityPublicationWriteSetPartsV1 {
        &self.parts
    }
}

pub trait ServingAuthorityPublicationReconciliationStore: Send + Sync {
    fn list_pending<'a>(
        &'a self,
        authority: &'a crate::VerifiedServingAuthorityInvocation,
        query: &'a ServingAuthorityPendingIssuanceQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<ServingAuthorityPendingIssuancePageV1, ServingAuthorityStoreError>,
    >;

    fn claim<'a>(
        &'a self,
        write_set: &'a ClaimServingAuthorityPublicationWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<ServingAuthorityPublicationLeaseV1, ServingAuthorityStoreError>>;
}
