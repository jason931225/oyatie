use cell_placement::{
    CellId, VerifiedCellMovementPermit, VerifiedCellPlacementDecision,
    VerifiedReservationCommitPermit,
};

use crate::{
    BindingDigest32, BindingGeneration, BindingOperationKey, BindingOperationV1, BindingRevision,
    TenantId, VerifiedParticipantManifest, VerifiedParticipantPhaseClosure,
    VerifiedResidencyTransferAuthorizationSet, VerifiedTransferEffectManifest, WriteAuthorityEpoch,
};

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationFenceClaimV1 {
    identity: crate::MigrationClaimIdentityV1,
    operation: BindingOperationKey,
    tenant_id: TenantId,
    source_cell_id: CellId,
    target_cell_id: CellId,
    source_generation: BindingGeneration,
    source_revision: BindingRevision,
    source_binding_record_digest: BindingDigest32,
    binding_attempt_digest: BindingDigest32,
    authority_high_water_revision: crate::TenantWriteAuthorityHighWaterRevision,
    authority_high_water_record_digest: BindingDigest32,
    source_authority: crate::ServingAuthorityInstanceV1,
    source_installation_issuance_digest: BindingDigest32,
    source_freeze_intent_digest: BindingDigest32,
    successor_generation: BindingGeneration,
    write_authority_epoch: WriteAuthorityEpoch,
    participant_manifest: VerifiedParticipantManifest,
    placement_decision: VerifiedCellPlacementDecision,
    reservation_commit_permit: VerifiedReservationCommitPermit,
    transfer_effect_manifest: VerifiedTransferEffectManifest,
    transfer_authorization_set: VerifiedResidencyTransferAuthorizationSet,
    prepared_participant_closure: VerifiedParticipantPhaseClosure,
    movement_permit: VerifiedCellMovementPermit,
    forward_completion_coverage_digest: BindingDigest32,
    claimed_at_unix_seconds: u64,
    record_digest: BindingDigest32,
    revision: crate::MigrationFenceClaimRevision,
    disposition: crate::MigrationFenceClaimDispositionV1,
    superseded_by_operation: Option<BindingOperationKey>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationFenceClaimPartsV1 {
    pub identity: crate::MigrationClaimIdentityV1,
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub source_generation: BindingGeneration,
    pub source_revision: BindingRevision,
    pub source_binding_record_digest: BindingDigest32,
    pub binding_attempt_digest: BindingDigest32,
    pub authority_high_water_revision: crate::TenantWriteAuthorityHighWaterRevision,
    pub authority_high_water_record_digest: BindingDigest32,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub source_installation_issuance_digest: BindingDigest32,
    pub source_freeze_intent_digest: BindingDigest32,
    pub successor_generation: BindingGeneration,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub participant_manifest: VerifiedParticipantManifest,
    pub placement_decision: VerifiedCellPlacementDecision,
    pub reservation_commit_permit: VerifiedReservationCommitPermit,
    pub transfer_effect_manifest: VerifiedTransferEffectManifest,
    pub transfer_authorization_set: VerifiedResidencyTransferAuthorizationSet,
    pub prepared_participant_closure: VerifiedParticipantPhaseClosure,
    pub movement_permit: VerifiedCellMovementPermit,
    pub forward_completion_coverage_digest: BindingDigest32,
    pub claimed_at_unix_seconds: u64,
    pub record_digest: BindingDigest32,
    pub revision: crate::MigrationFenceClaimRevision,
    pub disposition: crate::MigrationFenceClaimDispositionV1,
    pub superseded_by_operation: Option<BindingOperationKey>,
}

impl MigrationFenceClaimV1 {
    pub fn assemble(
        _identity: &crate::MigrationClaimIdentityV1,
        _intent: &crate::ServingAuthorityFreezeIntentV1,
        _parts: MigrationFenceClaimPartsV1,
    ) -> Result<Self, MigrationFenceClaimError> {
        Err(MigrationFenceClaimError::NotImplemented)
    }

    #[must_use]
    pub fn identity(&self) -> &crate::MigrationClaimIdentityV1 {
        &self.identity
    }

    pub fn rehydrate(_parts: MigrationFenceClaimPartsV1) -> Result<Self, MigrationFenceClaimError> {
        Err(MigrationFenceClaimError::NotImplemented)
    }

    #[must_use]
    pub fn operation(&self) -> &BindingOperationKey {
        &self.operation
    }

    #[must_use]
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    #[must_use]
    pub fn source_cell_id(&self) -> &CellId {
        &self.source_cell_id
    }

    #[must_use]
    pub fn target_cell_id(&self) -> &CellId {
        &self.target_cell_id
    }

    #[must_use]
    pub fn source_generation(&self) -> BindingGeneration {
        self.source_generation
    }

    #[must_use]
    pub fn source_revision(&self) -> BindingRevision {
        self.source_revision
    }

    #[must_use]
    pub fn successor_generation(&self) -> BindingGeneration {
        self.successor_generation
    }

    #[must_use]
    pub fn write_authority_epoch(&self) -> WriteAuthorityEpoch {
        self.write_authority_epoch
    }

    #[must_use]
    pub fn movement_permit(&self) -> &VerifiedCellMovementPermit {
        &self.movement_permit
    }

    #[must_use]
    pub fn forward_completion_coverage_digest(&self) -> BindingDigest32 {
        self.forward_completion_coverage_digest
    }

    #[must_use]
    pub fn record_digest(&self) -> BindingDigest32 {
        self.record_digest
    }

    #[must_use]
    pub fn source_binding_record_digest(&self) -> BindingDigest32 {
        self.source_binding_record_digest
    }

    #[must_use]
    pub fn binding_attempt_digest(&self) -> BindingDigest32 {
        self.binding_attempt_digest
    }

    #[must_use]
    pub fn authority_high_water_revision(&self) -> crate::TenantWriteAuthorityHighWaterRevision {
        self.authority_high_water_revision
    }

    #[must_use]
    pub fn authority_high_water_record_digest(&self) -> BindingDigest32 {
        self.authority_high_water_record_digest
    }

    #[must_use]
    pub fn source_authority(&self) -> &crate::ServingAuthorityInstanceV1 {
        &self.source_authority
    }

    #[must_use]
    pub fn source_installation_issuance_digest(&self) -> BindingDigest32 {
        self.source_installation_issuance_digest
    }

    #[must_use]
    pub fn source_freeze_intent_digest(&self) -> BindingDigest32 {
        self.source_freeze_intent_digest
    }

    #[must_use]
    pub fn participant_manifest(&self) -> &VerifiedParticipantManifest {
        &self.participant_manifest
    }

    #[must_use]
    pub fn placement_decision(&self) -> &VerifiedCellPlacementDecision {
        &self.placement_decision
    }

    #[must_use]
    pub fn reservation_commit_permit(&self) -> &VerifiedReservationCommitPermit {
        &self.reservation_commit_permit
    }

    #[must_use]
    pub fn transfer_authorization_set(&self) -> &VerifiedResidencyTransferAuthorizationSet {
        &self.transfer_authorization_set
    }

    #[must_use]
    pub fn transfer_effect_manifest(&self) -> &VerifiedTransferEffectManifest {
        &self.transfer_effect_manifest
    }

    #[must_use]
    pub fn prepared_participant_closure(&self) -> &VerifiedParticipantPhaseClosure {
        &self.prepared_participant_closure
    }

    #[must_use]
    pub fn claimed_at_unix_seconds(&self) -> u64 {
        self.claimed_at_unix_seconds
    }

    #[must_use]
    pub fn revision(&self) -> crate::MigrationFenceClaimRevision {
        self.revision
    }

    #[must_use]
    pub fn disposition(&self) -> crate::MigrationFenceClaimDispositionV1 {
        self.disposition
    }

    #[must_use]
    pub fn superseded_by_operation(&self) -> Option<&BindingOperationKey> {
        self.superseded_by_operation.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationFenceClaimError {
    NotImplemented,
    StaleBinding,
    ActiveClaimExists,
    EpochNotNextAllocation,
    GenerationNotNextAllocation,
    RelationMismatch,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationFenceClaimMutationResultV1 {
    pub claim: MigrationFenceClaimV1,
    pub operation: BindingOperationV1,
}
