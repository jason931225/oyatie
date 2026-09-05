use crate::{
    BoxTenancyFuture, ServingAuthorityFreezeResultV1, ServingAuthorityFreezeWriteSetV1,
    ServingAuthorityInstallationResultV1, ServingAuthorityInstallationWriteSetV1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServingAuthorityStoreError {
    NotImplemented,
    Unavailable,
    Conflict,
    ScopeMismatch,
    StaleIncarnation,
    RejectedInstallation,
    FrozenAuthority,
    UncommittedIssuance,
    IdempotencyKeyReuse,
    ProofAlreadyApplied,
    IncompletePriorAuthorityClosure,
    RestoreEvidenceRequired,
    PageLimitExceeded,
    RetainedEvidenceUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityResultQueryV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub business: crate::ServingAuthorityBusinessIdV1,
}

pub trait CellServingAuthorityStore: Send + Sync {
    fn install<'a>(
        &'a self,
        write_set: &'a ServingAuthorityInstallationWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<ServingAuthorityInstallationResultV1, ServingAuthorityStoreError>,
    >;

    fn freeze<'a>(
        &'a self,
        write_set: &'a ServingAuthorityFreezeWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<ServingAuthorityFreezeResultV1, ServingAuthorityStoreError>>;

    fn renew_write_authority_lease<'a>(
        &'a self,
        write_set: &'a crate::WriteAuthorityLeaseRenewalWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::WriteAuthorityLeaseRenewalResultV1, ServingAuthorityStoreError>,
    >;

    fn publish_write_authority_lease<'a>(
        &'a self,
        write_set: &'a crate::WriteAuthorityLeasePublicationWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::WriteAuthorityLeasePublicationResultV1, ServingAuthorityStoreError>,
    >;

    fn load_committed_write_authority_lease_issuance<'a>(
        &'a self,
        authority: &'a crate::VerifiedServingAuthorityInvocation,
        query: &'a crate::CommittedWriteAuthorityLeaseIssuanceQueryV1,
        lease: &'a crate::ServingAuthorityPublicationLeaseV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<
            Option<crate::CommittedWriteAuthorityLeaseIssuanceClaimV1>,
            ServingAuthorityStoreError,
        >,
    >;

    fn get_lease_state<'a>(
        &'a self,
        authority: &'a crate::VerifiedServingAuthorityInvocation,
        instance: &'a crate::ServingAuthorityInstanceV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::WriteAuthorityLeaseStateV1>, ServingAuthorityStoreError>,
    >;

    fn get_latest_published_lease<'a>(
        &'a self,
        authority: &'a crate::VerifiedServingAuthorityInvocation,
        instance: &'a crate::ServingAuthorityInstanceV1,
        minimum_valid_until_unix_seconds: u64,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::PublishedWriteAuthorityLeaseV1>, ServingAuthorityStoreError>,
    >;

    fn get_installation_result<'a>(
        &'a self,
        authority: &'a crate::VerifiedServingAuthorityInvocation,
        query: &'a ServingAuthorityResultQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<ServingAuthorityInstallationResultV1>, ServingAuthorityStoreError>,
    >;

    fn get_freeze_result<'a>(
        &'a self,
        authority: &'a crate::VerifiedServingAuthorityInvocation,
        query: &'a ServingAuthorityResultQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<ServingAuthorityFreezeResultV1>, ServingAuthorityStoreError>,
    >;
}
