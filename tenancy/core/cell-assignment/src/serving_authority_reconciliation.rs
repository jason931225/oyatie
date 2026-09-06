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
    pub authority: crate::ServingAuthorityPersistenceAuthorityV1,
    pub installed: crate::InstalledServingAuthorityV1,
    pub issuance: crate::WriteAuthorityLeaseIssuancePreconditionV1,
    /// CAS precondition on the current publication lease: `None` to claim an
    /// unheld issuance, `Some(epoch)` to take over from a lease the caller has
    /// read. Read it with
    /// [`ServingAuthorityPublicationReconciliationStore::get_publication_lease`].
    pub expected_lease_epoch: Option<u64>,
    pub next_lease: ServingAuthorityPublicationLeaseV1,
    /// The claimant's clock, compared against the held lease's
    /// `expires_at_unix_seconds`.
    ///
    /// Without it the contract cannot distinguish taking over an EXPIRED lease,
    /// which is the reconciler's normal recovery, from stealing a LIVE one held
    /// by a working peer, which must be refused as
    /// [`crate::ServingAuthorityStoreError::PublicationLeaseHeldByAnotherWorker`].
    /// A CAS on `expected_lease_epoch` alone cannot tell those apart: both are
    /// epoch matches.
    pub now_unix_seconds: u64,
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
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        query: &'a ServingAuthorityPendingIssuanceQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<ServingAuthorityPendingIssuancePageV1, ServingAuthorityStoreError>,
    >;

    /// Reads the publication lease currently held on an issuance, or `None`
    /// when it is unheld.
    ///
    /// `ClaimServingAuthorityPublicationWriteSetPartsV1.expected_lease_epoch` is
    /// a CAS precondition, and until now nothing returned a publication lease
    /// epoch: `claim` yields one only on its own success channel, and `claim` is
    /// what needs the value. The only other path to it was
    /// `WriteAuthorityLeaseCommitObserver::observe_lease_commit`, which cannot
    /// serve here -- its payload carries publication-lease fields that have no
    /// defined value in the pre-claim window, and using an observer as a getter
    /// makes its own expectation circular. So a contending reconciler could not
    /// assemble the write set at all.
    fn get_publication_lease<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        instance: &'a crate::ServingAuthorityInstanceV1,
        issuance_digest: BindingDigest32,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<ServingAuthorityPublicationLeaseV1>, ServingAuthorityStoreError>,
    >;

    fn claim<'a>(
        &'a self,
        write_set: &'a ClaimServingAuthorityPublicationWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<ServingAuthorityPublicationLeaseV1, ServingAuthorityStoreError>>;
}
