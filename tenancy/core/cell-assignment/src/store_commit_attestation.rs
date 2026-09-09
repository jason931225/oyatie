use cell_placement::CellId;

use crate::{
    BindingDigest32, BindingGeneration, BindingOperationKey, BindingProducerId,
    BindingProofEnvelopeV1, BindingRevision, TenantId, WriteAuthorityEpoch,
    WriteAuthorityLeaseIssuancePreconditionV1, WriteAuthorityLeaseIssuanceRecordV1,
    WriteAuthorityLeaseStatePreconditionV1, WriteAuthorityLeaseStateV1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseCommitAttestationPayloadV1 {
    pub schema_version: u32,
    pub tenant_id: TenantId,
    pub cell_id: CellId,
    pub operation: BindingOperationKey,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub binding_record_digest: BindingDigest32,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub lease_intent_digest: BindingDigest32,
    pub issuance_revision: crate::WriteAuthorityLeaseIssuanceRevision,
    pub issuance_record_digest: BindingDigest32,
    pub lease_state_revision: crate::WriteAuthorityLeaseStateRevision,
    pub lease_state_record_digest: BindingDigest32,
    pub instance: crate::ServingAuthorityInstanceV1,
    pub publication_lease_epoch: u64,
    pub publication_lease_digest: BindingDigest32,
    pub publication_lease_expires_at_unix_seconds: u64,
    pub committed_transaction_digest: BindingDigest32,
    pub committed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedWriteAuthorityLeaseCommitAttestationV1 {
    pub payload: WriteAuthorityLeaseCommitAttestationPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommittedWriteAuthorityLeaseIssuanceClaimV1 {
    pub installed: crate::InstalledServingAuthorityV1,
    pub lease_state: WriteAuthorityLeaseStateV1,
    pub issuance: WriteAuthorityLeaseIssuanceRecordV1,
    pub publication_lease: crate::ServingAuthorityPublicationLeaseV1,
    pub attestation: SignedWriteAuthorityLeaseCommitAttestationV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseCommitAttestationExpectationV1 {
    pub tenant_id: TenantId,
    pub cell_id: CellId,
    pub operation: BindingOperationKey,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub binding_record_digest: BindingDigest32,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub lease_intent_digest: BindingDigest32,
    pub issuance_precondition: WriteAuthorityLeaseIssuancePreconditionV1,
    pub lease_state_precondition: WriteAuthorityLeaseStatePreconditionV1,
    pub instance: crate::ServingAuthorityInstanceV1,
    pub publication_lease_epoch: u64,
    pub publication_lease_digest: BindingDigest32,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

/// Independently observes a committed write-authority lease issuance and signs
/// what it read.
///
/// `SignedWriteAuthorityLeaseCommitAttestationV1` had a consumer
/// (`VerifiedServingAuthorityProofRefV1::LocalLeaseCommit`), a proof
/// domain (`BindingProofDomainV1::WriteAuthorityLeaseCommitAttestation`), a
/// verifier and an expectation, but no producer anywhere: the claim it sits
/// inside was loaded by
/// `CellServingAuthorityStore::load_committed_write_authority_lease_issuance`
/// and never built. This port is that missing producer.
///
/// Separate from the store on purpose: the party that performed the lease write
/// must not be the party that attests it committed. Accepts a read authority
/// and a lookup key only — never a caller's claim.
///
/// The caller assembles [`CommittedWriteAuthorityLeaseIssuanceClaimV1`] from
/// the durable issuance plus this attestation, then passes it through
/// `verify_committed_write_authority_lease_issuance`. No new proof domain is
/// required; the existing one was always intended for this.
///
/// WHY THIS ONE RETURNS A LONE SIGNATURE WHEN ITS SIBLINGS RETURN THE RECORD
/// WITH IT. `DrainContributorSealCommitObserver`,
/// `MovementPermitIssuanceCommitObserver` and `SourceReleaseCommitObserver`
/// each hand back a `Committed*ClaimV1`, because handing back a lone signature
/// puts the caller in charge of pairing it with a record. Here the pairing is
/// not the caller's to get wrong: every record the claim carries is pinned BY
/// DIGEST inside the payload this attestation signs over —
/// `issuance_revision`, `issuance_record_digest`, `lease_state_revision`,
/// `lease_state_record_digest`, `binding_record_digest` and
/// `publication_lease_digest` — so a caller pairing it with a different record
/// is refused by `verify_committed_write_authority_lease_issuance` rather than
/// believed. The siblings' claims have no such internal binding, which is why
/// the shapes differ.
///
/// AND THE CLAIM COULD NOT BE BUILT HERE ANYWAY.
/// [`CommittedWriteAuthorityLeaseIssuanceClaimV1`] carries five members drawn
/// from four different rows, and this observer is used in the PRE-PUBLICATION
/// window, where the reasoning at
/// `CellServingAuthorityStore::load_committed_write_authority_lease_issuance`
/// applies: a claim-typed observer would have to re-read all four and would
/// duplicate that loader.
pub trait WriteAuthorityLeaseCommitObserver: Send + Sync {
    fn observe_lease_commit<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        query: &'a crate::CommittedWriteAuthorityLeaseIssuanceQueryV1,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<
            Option<SignedWriteAuthorityLeaseCommitAttestationV1>,
            crate::ServingAuthorityStoreError,
        >,
    >;
}
