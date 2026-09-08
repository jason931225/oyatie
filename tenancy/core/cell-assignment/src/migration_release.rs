use cell_placement::CellProofConsumptionV1;

use crate::{
    BindingAuditRecordV1, BindingDigest32, BindingGeneration, BindingIdempotencyKey,
    BindingIdempotencyRecordV1, BindingOperationKey, BindingOperationRevision, BindingOperationV1,
    BindingPersistenceAuthorityV1, BindingProofConsumptionV1, BindingRevision, BindingStoreError,
    SourceReservationReleaseIssuanceRecordV1, TenantCellBinding, VerifiedParticipantPhaseClosure,
    VerifiedProjectionConvergence, VerifiedRollbackWindowElapsed,
};

#[derive(Debug, Eq, PartialEq)]
pub struct FinalizeMigrationReleaseRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub expected_binding_generation: BindingGeneration,
    pub expected_binding_revision: BindingRevision,
    pub expected_binding_record_digest: BindingDigest32,
    pub expected_projection_audience_policy_digest: BindingDigest32,
    pub target_activation_closure: VerifiedParticipantPhaseClosure,
    pub source_release_closure: VerifiedParticipantPhaseClosure,
    pub projection_convergence: VerifiedProjectionConvergence,
    pub rollback_window: VerifiedRollbackWindowElapsed,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationReleaseWriteSetV1 {
    parts: MigrationReleaseWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationReleaseWriteSetPartsV1 {
    pub authority: BindingPersistenceAuthorityV1,
    pub expected_operation_revision: BindingOperationRevision,
    pub successor_binding: TenantCellBinding,
    /// Compare-and-set on the projection audience policy,
    /// [`crate::ProjectionAudiencePolicyV1`], the binding carries. The policy is
    /// owned outside these write sets -- none of them proposes one -- so this
    /// pins what the release was computed against rather than opening a row.
    pub expected_projection_audience_policy_digest: BindingDigest32,
    pub target_activation_closure: VerifiedParticipantPhaseClosure,
    pub source_release_closure: VerifiedParticipantPhaseClosure,
    pub projection_convergence: VerifiedProjectionConvergence,
    pub rollback_window: VerifiedRollbackWindowElapsed,
    pub release_issuance: SourceReservationReleaseIssuanceRecordV1,
    pub operation: BindingOperationV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub audit_outbox: BindingAuditRecordV1,
    pub cell_proof_consumptions: Vec<CellProofConsumptionV1>,
    pub binding_proof_consumptions: Vec<BindingProofConsumptionV1>,
}

impl MigrationReleaseWriteSetV1 {
    pub fn assemble(_parts: MigrationReleaseWriteSetPartsV1) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &MigrationReleaseWriteSetPartsV1 {
        &self.parts
    }
}
