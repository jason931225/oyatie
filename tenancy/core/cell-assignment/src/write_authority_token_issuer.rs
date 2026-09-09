use cell_placement::CellId;

use crate::{
    BindingAuthorizationDecisionReceiptV1, BindingDigest32, BindingGeneration,
    BindingIdempotencyKey, BindingProofConsumptionV1, BindingRevision, BindingStoreError,
    BoxTenancyFuture, CapabilityParticipantId, SignedWriteAuthorityTokenV1, TenantId,
    VerifiedBindingInvocation, VerifiedParticipantManifestMember, VerifiedWriteAuthorityLease,
    VerifiedWriteAuthorityToken, WriteAuthorityEpoch,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellLocalWriteAuthorityTokenLedgerRevision(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellLocalWriteAuthorityTokenLedgerV1 {
    pub tenant_id: TenantId,
    pub cell_id: CellId,
    pub participant_id: CapabilityParticipantId,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub write_authority_lease_digest: BindingDigest32,
    pub write_authority_lease_expires_at_unix_seconds: u64,
    pub participant_manifest_digest: BindingDigest32,
    pub issued_token_root_digest: BindingDigest32,
    pub issued_token_count: u64,
    pub maximum_token_expires_at_unix_seconds: u64,
    pub revision: CellLocalWriteAuthorityTokenLedgerRevision,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellLocalWriteAuthorityTokenLedgerPreconditionV1 {
    Absent,
    Matches {
        revision: CellLocalWriteAuthorityTokenLedgerRevision,
        write_authority_lease_digest: BindingDigest32,
        record_digest: BindingDigest32,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct IssueWriteAuthorityTokenRequestV1 {
    pub operation: crate::BindingOperationKey,
    pub lease: VerifiedWriteAuthorityLease,
    pub participant: VerifiedParticipantManifestMember,
    pub ledger_precondition: CellLocalWriteAuthorityTokenLedgerPreconditionV1,
    pub requested_validity_seconds: u64,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellLocalWriteAuthorityIdempotencyRecordV1 {
    pub tenant_id: TenantId,
    pub cell_id: CellId,
    pub participant_id: CapabilityParticipantId,
    pub idempotency_key: BindingIdempotencyKey,
    pub request_digest: BindingDigest32,
    pub immutable_result_digest: BindingDigest32,
}

/// The audit outbox row for one cell-local write-authority token issue.
///
/// THIS ROW HAS NO WIRE FORM, deliberately, and it is recorded here rather than
/// left for a sweep to discover as an unmapped name. It carries a
/// `record_digest` the store derives, so it is a proposed successor by the same
/// rule `CellReservationEffectRecordV1` is, and there is no `message` for it in
/// `tenancy/binding/v1`. The lane it belongs to is cell-local by construction —
/// the token never leaves the cell that issued it — so no adapter reads this
/// row off a schema. What binds it instead is the Rust contract on
/// [`CellLocalWriteAuthorityTokenIssueWriteSetPartsV1`] that carries it,
/// together with [`crate::BindingStoreError::ProposedSuccessorMismatch`], the
/// refusal the Tenancy rule head names for a disagreeing proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellLocalWriteAuthorityAuditRecordV1 {
    pub audit_event_id: String,
    pub tenant_id: TenantId,
    pub cell_id: CellId,
    pub participant_id: CapabilityParticipantId,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub write_authority_lease_digest: BindingDigest32,
    pub token_digest: BindingDigest32,
    pub assurance_audit_policy: cell_placement::AssuranceAuditPolicyV1,
    pub actor_digest: BindingDigest32,
    pub authorization: BindingAuthorizationDecisionReceiptV1,
    pub idempotency_key: BindingIdempotencyKey,
    pub request_digest: BindingDigest32,
    pub occurred_at_unix_seconds: u64,
    pub record_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellLocalWriteAuthorityTokenIssueWriteSetV1 {
    parts: CellLocalWriteAuthorityTokenIssueWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellLocalWriteAuthorityTokenIssueWriteSetPartsV1 {
    pub invocation: VerifiedBindingInvocation,
    pub lease: VerifiedWriteAuthorityLease,
    pub participant: VerifiedParticipantManifestMember,
    pub token: VerifiedWriteAuthorityToken,
    pub ledger_precondition: CellLocalWriteAuthorityTokenLedgerPreconditionV1,
    pub next_ledger: CellLocalWriteAuthorityTokenLedgerV1,
    pub drain_mutation: cell_placement::DrainContributorStateMutationV1,
    pub idempotency: CellLocalWriteAuthorityIdempotencyRecordV1,
    pub audit_outbox: CellLocalWriteAuthorityAuditRecordV1,
    pub proof_consumptions: Vec<BindingProofConsumptionV1>,
}

impl CellLocalWriteAuthorityTokenIssueWriteSetV1 {
    pub fn assemble(
        _parts: CellLocalWriteAuthorityTokenIssueWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &CellLocalWriteAuthorityTokenIssueWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct WriteAuthorityTokenIssueResultV1 {
    pub token: SignedWriteAuthorityTokenV1,
    pub ledger: CellLocalWriteAuthorityTokenLedgerV1,
}

pub trait CellLocalWriteAuthorityTokenAuthority: Send + Sync {
    fn issue<'a>(
        &'a self,
        invocation: &'a VerifiedBindingInvocation,
        lease: &'a VerifiedWriteAuthorityLease,
        participant: &'a VerifiedParticipantManifestMember,
        requested_validity_seconds: u64,
    ) -> BoxTenancyFuture<'a, Result<VerifiedWriteAuthorityToken, BindingStoreError>>;
}

pub trait CellLocalWriteAuthorityTokenStore: Send + Sync {
    fn issue<'a>(
        &'a self,
        write_set: &'a CellLocalWriteAuthorityTokenIssueWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<WriteAuthorityTokenIssueResultV1, BindingStoreError>>;

    fn get_ledger<'a>(
        &'a self,
        invocation: &'a VerifiedBindingInvocation,
        tenant_id: &'a TenantId,
        cell_id: &'a CellId,
        participant_id: &'a CapabilityParticipantId,
        binding_generation: BindingGeneration,
        write_authority_epoch: WriteAuthorityEpoch,
    ) -> BoxTenancyFuture<'a, Result<Option<CellLocalWriteAuthorityTokenLedgerV1>, BindingStoreError>>;
}

pub trait CellLocalWriteAuthorityService: Send + Sync {
    fn issue_write_authority_token<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: IssueWriteAuthorityTokenRequestV1,
    ) -> BoxTenancyFuture<'a, Result<WriteAuthorityTokenIssueResultV1, crate::BindingContractError>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NotImplementedCellLocalWriteAuthorityService;

impl CellLocalWriteAuthorityService for NotImplementedCellLocalWriteAuthorityService {
    fn issue_write_authority_token<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: IssueWriteAuthorityTokenRequestV1,
    ) -> BoxTenancyFuture<'a, Result<WriteAuthorityTokenIssueResultV1, crate::BindingContractError>>
    {
        Box::pin(async { Err(crate::BindingContractError::NotImplemented) })
    }
}
