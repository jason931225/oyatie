use cell_placement::CellId;

use crate::{
    BindingDigest32, BindingGeneration, BindingProofVerificationError, BindingRevision,
    BindingStoreError, BoxTenancyFuture, CapabilityParticipantId, TenantId,
    VerifiedSourceFenceDirective, VerifiedWriteAuthorityLease, VerifiedWriteAuthorityToken,
    WriteAuthorityEpoch,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CapabilityWriteAuthorityRevision(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CapabilityWriteAuthorityDispositionV1 {
    Writable,
    Fenced,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityWriteAuthorityStateV1 {
    tenant_id: TenantId,
    participant_id: CapabilityParticipantId,
    cell_id: CellId,
    binding_generation: BindingGeneration,
    binding_revision: BindingRevision,
    write_authority_epoch: WriteAuthorityEpoch,
    disposition: CapabilityWriteAuthorityDispositionV1,
    participant_manifest_digest: BindingDigest32,
    write_authority_lease_digest: BindingDigest32,
    authority_expires_at_unix_seconds: u64,
    revision: CapabilityWriteAuthorityRevision,
    last_authority_proof_digest: BindingDigest32,
    record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityWriteAuthorityStatePartsV1 {
    pub tenant_id: TenantId,
    pub participant_id: CapabilityParticipantId,
    pub cell_id: CellId,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub disposition: CapabilityWriteAuthorityDispositionV1,
    pub participant_manifest_digest: BindingDigest32,
    pub write_authority_lease_digest: BindingDigest32,
    pub authority_expires_at_unix_seconds: u64,
    pub revision: CapabilityWriteAuthorityRevision,
    pub last_authority_proof_digest: BindingDigest32,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CapabilityWriteAuthorityStateConstructionErrorV1 {
    NotImplemented,
    InvalidRevision,
    InvalidAuthorityRelation,
    InvalidDisposition,
    RecordDigestMismatch,
}

impl CapabilityWriteAuthorityStateV1 {
    pub fn rehydrate(
        _parts: CapabilityWriteAuthorityStatePartsV1,
    ) -> Result<Self, CapabilityWriteAuthorityStateConstructionErrorV1> {
        Err(CapabilityWriteAuthorityStateConstructionErrorV1::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> CapabilityWriteAuthorityStatePartsRefV1<'_> {
        CapabilityWriteAuthorityStatePartsRefV1 {
            tenant_id: &self.tenant_id,
            participant_id: &self.participant_id,
            cell_id: &self.cell_id,
            binding_generation: self.binding_generation,
            binding_revision: self.binding_revision,
            write_authority_epoch: self.write_authority_epoch,
            disposition: self.disposition,
            participant_manifest_digest: self.participant_manifest_digest,
            write_authority_lease_digest: self.write_authority_lease_digest,
            authority_expires_at_unix_seconds: self.authority_expires_at_unix_seconds,
            revision: self.revision,
            last_authority_proof_digest: self.last_authority_proof_digest,
            record_digest: self.record_digest,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityWriteAuthorityStatePartsRefV1<'a> {
    pub tenant_id: &'a TenantId,
    pub participant_id: &'a CapabilityParticipantId,
    pub cell_id: &'a CellId,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub disposition: CapabilityWriteAuthorityDispositionV1,
    pub participant_manifest_digest: BindingDigest32,
    pub write_authority_lease_digest: BindingDigest32,
    pub authority_expires_at_unix_seconds: u64,
    pub revision: CapabilityWriteAuthorityRevision,
    pub last_authority_proof_digest: BindingDigest32,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityWriteAuthorityPreconditionV1 {
    Absent,
    Matches {
        revision: CapabilityWriteAuthorityRevision,
        binding_generation: BindingGeneration,
        write_authority_epoch: WriteAuthorityEpoch,
        record_digest: BindingDigest32,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum CapabilityWriteAuthorityTransitionEvidenceV1 {
    Activate(Box<CapabilityWriteAuthorityActivationEvidenceV1>),
    FenceSource(Box<VerifiedSourceFenceDirective>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct CapabilityWriteAuthorityActivationEvidenceV1 {
    pub lease: VerifiedWriteAuthorityLease,
    pub token: VerifiedWriteAuthorityToken,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CapabilityWriteAuthorityTransitionWriteSetV1 {
    parts: CapabilityWriteAuthorityTransitionWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CapabilityWriteAuthorityTransitionWriteSetPartsV1 {
    pub precondition: CapabilityWriteAuthorityPreconditionV1,
    pub evidence: CapabilityWriteAuthorityTransitionEvidenceV1,
    pub next: CapabilityWriteAuthorityStateV1,
    pub drain_mutation: cell_placement::DrainContributorStateMutationV1,
    pub local_idempotency_digest: BindingDigest32,
    pub local_proof_consumption_digest: BindingDigest32,
    pub local_audit_record_digest: BindingDigest32,
}

impl CapabilityWriteAuthorityTransitionWriteSetV1 {
    pub fn assemble(
        _parts: CapabilityWriteAuthorityTransitionWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &CapabilityWriteAuthorityTransitionWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CapabilityAuthorizedWriteSetV1 {
    parts: CapabilityAuthorizedWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CapabilityAuthorizedWriteSetPartsV1 {
    pub authority: VerifiedWriteAuthorityToken,
    /// Compare-and-set on [`CapabilityWriteAuthorityStateV1`], required by
    /// value.
    ///
    /// RULED, NOT CLOSED, AND THE REASON IS THE AXIS. Every other by-value
    /// precondition in these crates discharges either through a read taken
    /// under an authority the write's own arm holds — the ordering stated on
    /// [`crate::BindingPersistenceAuthorityV1::read_authority`] — or through the
    /// reconciliation subject, an earlier write, or a lease-gated read. NONE of
    /// those is available here, and not because a surface is missing: the
    /// authority this write takes is a [`VerifiedWriteAuthorityToken`], a
    /// capability-local write token that has NO read twin at all, so the
    /// ordering rule has nothing to range over. The row's only producer is
    /// [`CapabilityWriteAuthorityStore::apply_transition`], this store's own
    /// write, and the store declares no read.
    ///
    /// Adding one would mean deciding what authority a capability-local READ
    /// takes, which is a boundary decision this crate has not made: the same
    /// disposition is durably held on the Cell side as
    /// `LocalAuthorityStateV1.disposition` under an authority fence, and
    /// nothing in either crate says which of the two records is authoritative
    /// or that this store is superseded. That question is recorded here and
    /// left open rather than answered by inventing a read.
    ///
    /// This write set and this store predate the wave.
    pub expected_authority_state: CapabilityWriteAuthorityStateV1,
    pub write_attempt_at_unix_seconds: u64,
    pub local_effect_digest: BindingDigest32,
    pub drain_mutation: cell_placement::DrainContributorStateMutationV1,
    pub local_idempotency_digest: BindingDigest32,
    pub local_proof_consumption_digest: BindingDigest32,
    pub local_audit_record_digest: BindingDigest32,
}

impl CapabilityAuthorizedWriteSetV1 {
    pub fn assemble(
        _parts: CapabilityAuthorizedWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &CapabilityAuthorizedWriteSetPartsV1 {
        &self.parts
    }
}

pub fn verify_capability_authority_transition(
    _precondition: &CapabilityWriteAuthorityPreconditionV1,
    _evidence: &CapabilityWriteAuthorityTransitionEvidenceV1,
    _next: &CapabilityWriteAuthorityStateV1,
    _now_unix_seconds: u64,
) -> Result<(), BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

pub trait CapabilityWriteAuthorityStore: Send + Sync {
    fn apply_transition<'a>(
        &'a self,
        write_set: &'a CapabilityWriteAuthorityTransitionWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<CapabilityWriteAuthorityStateV1, BindingStoreError>>;

    fn consume_before_write<'a>(
        &'a self,
        write_set: &'a CapabilityAuthorizedWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<BindingDigest32, BindingStoreError>>;
}
