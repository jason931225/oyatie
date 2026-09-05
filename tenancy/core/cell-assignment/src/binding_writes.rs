use crate::{
    BindingAuditRecordV1, BindingDigest32, BindingGeneration, BindingHistoryEntry,
    BindingIdempotencyKey, BindingOperationKey, BindingOperationPreconditionV1, BindingOperationV1,
    BindingPersistenceAuthorityV1, BindingProofConsumptionV1, BindingReservationAttemptRevision,
    BindingReservationAttemptV1, BindingRevision, BindingStoreError,
    MigrationFenceClaimTransitionV1, ServingAuthorityInstallationIssuanceV1,
    SignedBindingProjectionV1, TenantCellBinding, TenantWriteAuthorityBindingMutationV1,
    VerifiedMigrationCommitSeal, VerifiedParticipantPhaseClosure, VerifiedStagedTenantBirthRecord,
};
use cell_placement::{
    BindingOutcomeQueryRefV1, CellId, CellProofConsumptionV1, SignedBindingOutcomeV1,
    VerifiedCellPlacementDecision, VerifiedReservationCommitPermit,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingWritePrecondition {
    Unbound,
    Matches {
        source_cell_id: CellId,
        generation: BindingGeneration,
        revision: BindingRevision,
        record_digest: BindingDigest32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingOutcomePreconditionV1 {
    pub query: BindingOutcomeQueryRefV1,
    pub binding_attempt_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingIdempotencyRecordV1 {
    pub tenant_id: crate::TenantId,
    pub idempotency_key: BindingIdempotencyKey,
    pub request_digest: BindingDigest32,
    pub operation: BindingOperationKey,
    pub immutable_result_digest: Option<BindingDigest32>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingWriteSetV1 {
    authority: BindingPersistenceAuthorityV1,
    precondition: BindingWritePrecondition,
    outcome_precondition: BindingOutcomePreconditionV1,
    expected_attempt_revision: BindingReservationAttemptRevision,
    reservation_attempt: BindingReservationAttemptV1,
    control_partition: crate::BindingControlPartitionMutationV1,
    previous_authority_closure: Option<crate::VerifiedServingAuthorityTerminalClosure>,
    installation_handoff: crate::ServingAuthorityHandoffRecordV1,
    tenant_birth: Option<VerifiedStagedTenantBirthRecord>,
    initial_participant_preparation: Option<VerifiedParticipantPhaseClosure>,
    binding: TenantCellBinding,
    history: BindingHistoryEntry,
    operation_precondition: BindingOperationPreconditionV1,
    operation: BindingOperationV1,
    idempotency: BindingIdempotencyRecordV1,
    projection_outbox: SignedBindingProjectionV1,
    audit_outbox: BindingAuditRecordV1,
    reservation_outcome: SignedBindingOutcomeV1,
    placement_decision: VerifiedCellPlacementDecision,
    reservation_commit_permit: VerifiedReservationCommitPermit,
    migration_fence_claim_transition: Option<MigrationFenceClaimTransitionV1>,
    migration_commit_seal: Option<VerifiedMigrationCommitSeal>,
    cell_proof_consumptions: Vec<CellProofConsumptionV1>,
    binding_proof_consumptions: Vec<BindingProofConsumptionV1>,
    authority_high_water: TenantWriteAuthorityBindingMutationV1,
    installation_issuance: ServingAuthorityInstallationIssuanceV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingWriteSetPartsV1 {
    pub authority: BindingPersistenceAuthorityV1,
    pub precondition: BindingWritePrecondition,
    pub outcome_precondition: BindingOutcomePreconditionV1,
    pub expected_attempt_revision: BindingReservationAttemptRevision,
    pub reservation_attempt: BindingReservationAttemptV1,
    pub control_partition: crate::BindingControlPartitionMutationV1,
    pub previous_authority_closure: Option<crate::VerifiedServingAuthorityTerminalClosure>,
    pub installation_handoff: crate::ServingAuthorityHandoffRecordV1,
    pub tenant_birth: Option<VerifiedStagedTenantBirthRecord>,
    pub initial_participant_preparation: Option<VerifiedParticipantPhaseClosure>,
    pub binding: TenantCellBinding,
    pub history: BindingHistoryEntry,
    pub operation_precondition: BindingOperationPreconditionV1,
    pub operation: BindingOperationV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub projection_outbox: SignedBindingProjectionV1,
    pub audit_outbox: BindingAuditRecordV1,
    pub reservation_outcome: SignedBindingOutcomeV1,
    pub placement_decision: VerifiedCellPlacementDecision,
    pub reservation_commit_permit: VerifiedReservationCommitPermit,
    pub migration_fence_claim_transition: Option<MigrationFenceClaimTransitionV1>,
    pub migration_commit_seal: Option<VerifiedMigrationCommitSeal>,
    pub cell_proof_consumptions: Vec<CellProofConsumptionV1>,
    pub binding_proof_consumptions: Vec<BindingProofConsumptionV1>,
    pub authority_high_water: TenantWriteAuthorityBindingMutationV1,
    pub installation_issuance: ServingAuthorityInstallationIssuanceV1,
}

impl BindingWriteSetV1 {
    pub fn assemble(_parts: BindingWriteSetPartsV1) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn authority(&self) -> &BindingPersistenceAuthorityV1 {
        &self.authority
    }

    #[must_use]
    pub fn precondition(&self) -> &BindingWritePrecondition {
        &self.precondition
    }

    #[must_use]
    pub fn outcome_precondition(&self) -> &BindingOutcomePreconditionV1 {
        &self.outcome_precondition
    }

    #[must_use]
    pub fn expected_attempt_revision(&self) -> BindingReservationAttemptRevision {
        self.expected_attempt_revision
    }

    #[must_use]
    pub fn reservation_attempt(&self) -> &BindingReservationAttemptV1 {
        &self.reservation_attempt
    }

    #[must_use]
    pub fn control_partition(&self) -> &crate::BindingControlPartitionMutationV1 {
        &self.control_partition
    }

    #[must_use]
    pub fn previous_authority_closure(
        &self,
    ) -> Option<&crate::VerifiedServingAuthorityTerminalClosure> {
        self.previous_authority_closure.as_ref()
    }

    #[must_use]
    pub fn installation_handoff(&self) -> &crate::ServingAuthorityHandoffRecordV1 {
        &self.installation_handoff
    }

    #[must_use]
    pub fn binding(&self) -> &TenantCellBinding {
        &self.binding
    }

    #[must_use]
    pub fn operation_precondition(&self) -> BindingOperationPreconditionV1 {
        self.operation_precondition
    }

    #[must_use]
    pub fn operation(&self) -> &BindingOperationV1 {
        &self.operation
    }

    #[must_use]
    pub fn reservation_outcome(&self) -> &SignedBindingOutcomeV1 {
        &self.reservation_outcome
    }

    #[must_use]
    pub fn proof_consumptions(&self) -> (&[CellProofConsumptionV1], &[BindingProofConsumptionV1]) {
        (
            &self.cell_proof_consumptions,
            &self.binding_proof_consumptions,
        )
    }

    #[must_use]
    pub fn authority_high_water(&self) -> &TenantWriteAuthorityBindingMutationV1 {
        &self.authority_high_water
    }

    #[must_use]
    pub fn installation_issuance(&self) -> &ServingAuthorityInstallationIssuanceV1 {
        &self.installation_issuance
    }

    #[must_use]
    pub fn tenant_birth(&self) -> Option<&VerifiedStagedTenantBirthRecord> {
        self.tenant_birth.as_ref()
    }

    #[must_use]
    pub fn initial_participant_preparation(&self) -> Option<&VerifiedParticipantPhaseClosure> {
        self.initial_participant_preparation.as_ref()
    }

    #[must_use]
    pub fn history(&self) -> &BindingHistoryEntry {
        &self.history
    }

    #[must_use]
    pub fn idempotency(&self) -> &BindingIdempotencyRecordV1 {
        &self.idempotency
    }

    #[must_use]
    pub fn projection_outbox(&self) -> &SignedBindingProjectionV1 {
        &self.projection_outbox
    }

    #[must_use]
    pub fn audit_outbox(&self) -> &BindingAuditRecordV1 {
        &self.audit_outbox
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
    pub fn migration_fence_claim_transition(&self) -> Option<&MigrationFenceClaimTransitionV1> {
        self.migration_fence_claim_transition.as_ref()
    }

    #[must_use]
    pub fn migration_commit_seal(&self) -> Option<&VerifiedMigrationCommitSeal> {
        self.migration_commit_seal.as_ref()
    }
}
