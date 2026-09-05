use tenancy_kernel::TenantId;

use cell_placement::{
    AssuranceCompilerVersion, AssuranceGeneration, CellId, ImmutableEvidenceRefV1,
    ReservationRefV1, SignedBindingOutcomeV1,
};

use crate::{
    BindingDigest32, BindingIdempotencyKey, BindingOperationKey, BindingReservationAttemptRevision,
    SignedBindingProjectionV1, VerifiedMigrationCommitSeal, VerifiedParticipantPhaseClosure,
    WriteAuthorityEpoch,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingGeneration(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingRevision(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantCellBinding {
    tenant_id: TenantId,
    home_cell_id: CellId,
    warm_recovery_cell_id: Option<CellId>,
    home_reservation: ReservationRefV1,
    warm_recovery_reservation: Option<ReservationRefV1>,
    generation: BindingGeneration,
    revision: BindingRevision,
    write_authority_epoch: WriteAuthorityEpoch,
    assurance_generation: AssuranceGeneration,
    assurance_compiler_version: AssuranceCompilerVersion,
    assurance_requirements_digest: BindingDigest32,
    assurance_evidence_digest: BindingDigest32,
    recovery_evidence_digest: BindingDigest32,
    binding_attempt_digest: BindingDigest32,
    placement_decision_digest: BindingDigest32,
    reservation_commit_permit_digest: BindingDigest32,
    migration_commit_seal_digest: Option<BindingDigest32>,
    participant_manifest_digest: BindingDigest32,
    participant_preparation_closure_digest: BindingDigest32,
    projection_audience_policy: crate::ProjectionAudiencePolicyV1,
    assurance_evidence_ref: ImmutableEvidenceRefV1,
    recovery_evidence_ref: ImmutableEvidenceRefV1,
    record_digest: BindingDigest32,
}

impl TenantCellBinding {
    pub fn rehydrate(_parts: TenantCellBindingPartsV1) -> Result<Self, BindingConstructionError> {
        Err(BindingConstructionError::NotImplemented)
    }

    #[must_use]
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    #[must_use]
    pub fn home_cell_id(&self) -> &CellId {
        &self.home_cell_id
    }

    #[must_use]
    pub fn warm_recovery_cell_id(&self) -> Option<&CellId> {
        self.warm_recovery_cell_id.as_ref()
    }

    #[must_use]
    pub fn reservations(&self) -> (&ReservationRefV1, Option<&ReservationRefV1>) {
        (
            &self.home_reservation,
            self.warm_recovery_reservation.as_ref(),
        )
    }

    #[must_use]
    pub fn generation(&self) -> BindingGeneration {
        self.generation
    }

    #[must_use]
    pub fn revision(&self) -> BindingRevision {
        self.revision
    }

    #[must_use]
    pub fn write_authority_epoch(&self) -> WriteAuthorityEpoch {
        self.write_authority_epoch
    }

    #[must_use]
    pub fn assurance_generation(&self) -> AssuranceGeneration {
        self.assurance_generation
    }

    #[must_use]
    pub fn assurance_compiler_version(&self) -> &AssuranceCompilerVersion {
        &self.assurance_compiler_version
    }

    #[must_use]
    pub fn assurance_requirements_digest(&self) -> BindingDigest32 {
        self.assurance_requirements_digest
    }

    #[must_use]
    pub fn assurance_evidence_digest(&self) -> BindingDigest32 {
        self.assurance_evidence_digest
    }

    #[must_use]
    pub fn recovery_evidence_digest(&self) -> BindingDigest32 {
        self.recovery_evidence_digest
    }

    #[must_use]
    pub fn binding_attempt_digest(&self) -> BindingDigest32 {
        self.binding_attempt_digest
    }

    #[must_use]
    pub fn placement_decision_digest(&self) -> BindingDigest32 {
        self.placement_decision_digest
    }

    #[must_use]
    pub fn reservation_commit_permit_digest(&self) -> BindingDigest32 {
        self.reservation_commit_permit_digest
    }

    #[must_use]
    pub fn migration_commit_seal_digest(&self) -> Option<BindingDigest32> {
        self.migration_commit_seal_digest
    }

    #[must_use]
    pub fn participant_manifest_digest(&self) -> BindingDigest32 {
        self.participant_manifest_digest
    }

    #[must_use]
    pub fn participant_preparation_closure_digest(&self) -> BindingDigest32 {
        self.participant_preparation_closure_digest
    }

    #[must_use]
    pub fn projection_audience_policy(&self) -> &crate::ProjectionAudiencePolicyV1 {
        &self.projection_audience_policy
    }

    #[must_use]
    pub fn assurance_evidence_ref(&self) -> &ImmutableEvidenceRefV1 {
        &self.assurance_evidence_ref
    }

    #[must_use]
    pub fn recovery_evidence_ref(&self) -> &ImmutableEvidenceRefV1 {
        &self.recovery_evidence_ref
    }

    #[must_use]
    pub fn record_digest(&self) -> BindingDigest32 {
        self.record_digest
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantCellBindingPartsV1 {
    pub tenant_id: TenantId,
    pub home_cell_id: CellId,
    pub warm_recovery_cell_id: Option<CellId>,
    pub home_reservation: ReservationRefV1,
    pub warm_recovery_reservation: Option<ReservationRefV1>,
    pub generation: BindingGeneration,
    pub revision: BindingRevision,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub assurance_generation: AssuranceGeneration,
    pub assurance_compiler_version: AssuranceCompilerVersion,
    pub assurance_requirements_digest: BindingDigest32,
    pub assurance_evidence_digest: BindingDigest32,
    pub recovery_evidence_digest: BindingDigest32,
    pub binding_attempt_digest: BindingDigest32,
    pub placement_decision_digest: BindingDigest32,
    pub reservation_commit_permit_digest: BindingDigest32,
    pub migration_commit_seal_digest: Option<BindingDigest32>,
    pub participant_manifest_digest: BindingDigest32,
    pub participant_preparation_closure_digest: BindingDigest32,
    pub projection_audience_policy: crate::ProjectionAudiencePolicyV1,
    pub assurance_evidence_ref: ImmutableEvidenceRefV1,
    pub recovery_evidence_ref: ImmutableEvidenceRefV1,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingConstructionError {
    NotImplemented,
    InvalidInitialGeneration,
    InvalidSuccessorGeneration,
    InvalidRevision,
    InvalidProofRelation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingHistoryEntry {
    pub binding: TenantCellBinding,
    pub previous_cell_id: Option<CellId>,
    pub operation: BindingOperationKey,
    pub committed_at_unix_seconds: u64,
    pub actor_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct InitialBindingRequestV1 {
    pub tenant_birth: crate::TenantBirthRefV1,
    pub operation: BindingOperationKey,
    pub expected_attempt_revision: BindingReservationAttemptRevision,
    pub expected_operation_revision: crate::BindingOperationRevision,
    pub prepared_participant_closure: VerifiedParticipantPhaseClosure,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MoveBindingRequestV1 {
    pub tenant_id: TenantId,
    pub operation: BindingOperationKey,
    pub expected_source_cell_id: CellId,
    pub expected_generation: BindingGeneration,
    pub expected_revision: BindingRevision,
    pub expected_record_digest: BindingDigest32,
    pub expected_attempt_revision: BindingReservationAttemptRevision,
    pub expected_operation_revision: crate::BindingOperationRevision,
    pub migration_commit_seal: VerifiedMigrationCommitSeal,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingCommitResult {
    pub binding: TenantCellBinding,
    pub history: BindingHistoryEntry,
    pub projection: SignedBindingProjectionV1,
    pub reservation_outcome: SignedBindingOutcomeV1,
    pub installation_issuance: crate::ServingAuthorityInstallationIssuanceV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingCommitTransactionResultV1 {
    pub commit: BindingCommitResult,
    pub operation: crate::BindingOperationV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingContractError {
    NotImplemented,
    InvalidRequest,
    IdempotencyKeyReuse,
    PlacementProofRejected,
    ReservationPermitRejected,
    ReservationOutcomeRejected,
    MigrationSealRejected,
    MigrationClaimRejected,
    RepairAuthorityRejected,
    AuthorizationScopeMismatch,
    AlreadyBound,
    NotFoundOrNotAuthorized,
    StaleGeneration,
    StaleRevision,
    SourceCellMismatch,
    TargetEqualsSource,
    PersistenceUnavailable,
    Conflict,
    OutcomeAlreadyFinal,
    ReservationAttemptRejected,
    StaleReservationAttempt,
    StaleAuthorityHighWater,
    StaleWriteAuthorityLeaseState,
    WriteAuthorityLeaseFrozen,
    WriteAuthorityLeaseExpired,
    TokenValidityExceedsLease,
    StaleCapabilityWriteAuthority,
    CapabilityWriteAuthorityFenced,
    CapabilityAuthorityRollback,
    WriteAttemptOutsideAuthorityWindow,
    ActiveMigrationClaim,
    TerminalOperation,
    ForwardRecoveryRequired,
}
