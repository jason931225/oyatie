use crate::{BindingOperationKey, BindingProofDomainV1};

pub const BINDING_INVOCATION_SCHEMA_VERSION: u32 = 1;
pub const BINDING_PROOF_SIGNING_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingDigest32([u8; 32]);

impl BindingDigest32 {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingProofConstructionError {
    NotImplemented,
    InvalidIdentifier,
    InvalidCanonicalPayload,
}

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn parse(_value: impl Into<String>) -> Result<Self, BindingProofConstructionError> {
                Err(BindingProofConstructionError::NotImplemented)
            }
        }
    };
}

opaque_id!(BindingProducerId);
opaque_id!(BindingKeyId);
opaque_id!(BindingProofNonce);
opaque_id!(BindingIdempotencyKey);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BindingActionV1 {
    GetBinding,
    CommitInitialBinding,
    MoveBinding,
    ListBindingHistory,
    GetOperation,
    CancelOperation,
    RepairOperation,
    GetBindingOutcome,
    OpenBindingAttempt,
    CheckpointBindingAttempt,
    GetBindingAttempt,
    AbortBindingOutcome,
    ClaimMigrationFence,
    CommitMigrationWriteFence,
    GetMigrationFenceClaim,
    GetMigrationWriteFence,
    PutParticipantManifest,
    AppendParticipantReceipt,
    CloseParticipantPhase,
    AppendTransferAuthorization,
    SealTransferAuthorizationSet,
    IssueTransferExecutionPermit,
    RecordTransferExecutionOutcome,
    GetWriteAuthorityLease,
    RenewWriteAuthorityLease,
    IssueWriteAuthorityToken,
    IssueSourceFenceDirective,
    FinalizeMigrationRelease,
    Reconcile,
    RetargetMigration,
    ReadWorkSnapshot,
    AppendWorkSnapshotPage,
    SealWorkSnapshot,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingPolicyVersionToken(String);

impl BindingPolicyVersionToken {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, BindingProofConstructionError> {
        Err(BindingProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingProofEnvelopeV1 {
    pub schema_version: u32,
    pub signing_format_version: u32,
    pub domain: BindingProofDomainV1,
    pub producer: BindingProducerId,
    pub audience: BindingProducerId,
    pub key_id: BindingKeyId,
    pub nonce: BindingProofNonce,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub payload_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingProofReuseScopeV1 {
    ServingAuthorityHandoff {
        instance: crate::ServingAuthorityInstanceV1,
        handoff_digest: BindingDigest32,
    },
    Operation(BindingOperationKey),
    Migration(BindingOperationKey),
    Projection {
        tenant_id: crate::TenantId,
        audience: crate::ProjectionAudienceId,
        partition_digest: BindingDigest32,
        generation: crate::BindingGeneration,
    },
    CapabilityWrite {
        tenant_id: crate::TenantId,
        cell_id: cell_placement::CellId,
        generation: crate::BindingGeneration,
        epoch: crate::WriteAuthorityEpoch,
        participant_id: crate::CapabilityParticipantId,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingProofConsumptionV1 {
    scope: BindingProofReuseScopeV1,
    domain: BindingProofDomainV1,
    producer: BindingProducerId,
    nonce: BindingProofNonce,
    payload_digest: BindingDigest32,
    envelope_digest: BindingDigest32,
}

impl BindingProofConsumptionV1 {
    #[must_use]
    pub fn scope(&self) -> &BindingProofReuseScopeV1 {
        &self.scope
    }

    #[must_use]
    pub fn domain(&self) -> BindingProofDomainV1 {
        self.domain
    }

    #[must_use]
    pub fn producer(&self) -> &BindingProducerId {
        &self.producer
    }

    #[must_use]
    pub fn nonce(&self) -> &BindingProofNonce {
        &self.nonce
    }

    #[must_use]
    pub fn payload_digest(&self) -> BindingDigest32 {
        self.payload_digest
    }

    #[must_use]
    pub fn envelope_digest(&self) -> BindingDigest32 {
        self.envelope_digest
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VerifiedBindingProofRefV1<'a> {
    ServingAuthorityInstallGrant(&'a crate::VerifiedServingAuthorityInstallGrant),
    ServingAuthorityInstallationResult(&'a crate::VerifiedServingAuthorityInstallationResult),
    ServingAuthorityFreezeResult(&'a crate::VerifiedServingAuthorityFreezeResult),
    Invocation(&'a crate::VerifiedBindingInvocation),
    ResidencyTransferAuthorization(&'a crate::VerifiedResidencyTransferAuthorization),
    ResidencyTransferAuthorizationSet(&'a crate::VerifiedResidencyTransferAuthorizationSet),
    WriteFence(&'a crate::VerifiedWriteFence),
    WriteAuthorityToken(&'a crate::VerifiedWriteAuthorityToken),
    WriteAuthorityLease(&'a crate::VerifiedWriteAuthorityLease),
    MigrationCommitSeal(&'a crate::VerifiedMigrationCommitSeal),
    RepairAuthority(&'a crate::VerifiedBindingRepairAuthority),
    BindingProjection(&'a crate::VerifiedBindingProjection),
    ParticipantManifest(&'a crate::VerifiedParticipantManifest),
    ParticipantReceipt(&'a crate::VerifiedParticipantReceipt),
    ParticipantPhaseClosure(&'a crate::VerifiedParticipantPhaseClosure),
    TransferEffectManifest(&'a crate::VerifiedTransferEffectManifest),
    TransferExecutionPermit(&'a crate::VerifiedTransferExecutionPermit),
    TransferExecutionOutcome(&'a crate::VerifiedTransferExecutionOutcome),
    SourceFenceDirective(&'a crate::VerifiedSourceFenceDirective),
    RecoverySourceFenceCompletion(&'a crate::VerifiedRecoverySourceFenceCompletion),
    ProjectionConvergence(&'a crate::VerifiedProjectionConvergence),
    TenantBirthRecord(&'a crate::VerifiedStagedTenantBirthRecord),
    WriteAuthorityLeaseCommitAttestation(&'a crate::VerifiedCommittedWriteAuthorityLeaseIssuance),
    ReconciliationInvocation(&'a crate::VerifiedBindingReconciliationInvocation),
    RollbackWindowElapsed(&'a crate::VerifiedRollbackWindowElapsed),
    SourceReleaseCommitAttestation(&'a crate::VerifiedCommittedSourceReservationReleaseIssuance),
}

impl<'a> VerifiedBindingProofRefV1<'a> {
    #[must_use]
    pub fn proof_envelope(self) -> &'a BindingProofEnvelopeV1 {
        match self {
            Self::ServingAuthorityInstallGrant(proof) => &proof.signed().envelope,
            Self::ServingAuthorityInstallationResult(proof) => &proof.signed().envelope,
            Self::ServingAuthorityFreezeResult(proof) => &proof.signed().envelope,
            Self::Invocation(proof) => &proof.signed().envelope,
            Self::ResidencyTransferAuthorization(proof) => &proof.signed().envelope,
            Self::ResidencyTransferAuthorizationSet(proof) => &proof.signed().envelope,
            Self::WriteFence(proof) => &proof.signed().envelope,
            Self::WriteAuthorityToken(proof) => &proof.signed().envelope,
            Self::WriteAuthorityLease(proof) => &proof.signed().envelope,
            Self::MigrationCommitSeal(proof) => &proof.signed().envelope,
            Self::RepairAuthority(proof) => &proof.signed().envelope,
            Self::BindingProjection(proof) => &proof.signed().envelope,
            Self::ParticipantManifest(proof) => &proof.signed().envelope,
            Self::ParticipantReceipt(proof) => &proof.signed().envelope,
            Self::ParticipantPhaseClosure(proof) => &proof.signed().envelope,
            Self::TransferEffectManifest(proof) => &proof.signed().envelope,
            Self::TransferExecutionPermit(proof) => &proof.signed().envelope,
            Self::TransferExecutionOutcome(proof) => &proof.signed().envelope,
            Self::SourceFenceDirective(proof) => &proof.signed().envelope,
            Self::RecoverySourceFenceCompletion(proof) => &proof.signed().envelope,
            Self::ProjectionConvergence(proof) => &proof.signed().envelope,
            Self::TenantBirthRecord(proof) => &proof.signed().envelope,
            Self::WriteAuthorityLeaseCommitAttestation(proof) => &proof.attestation().envelope,
            Self::ReconciliationInvocation(proof) => &proof.signed().envelope,
            Self::RollbackWindowElapsed(proof) => &proof.signed().envelope,
            Self::SourceReleaseCommitAttestation(proof) => &proof.claim().attestation.envelope,
        }
    }
}

pub fn bind_binding_proof_consumption(
    _proof: VerifiedBindingProofRefV1<'_>,
    _scope: BindingProofReuseScopeV1,
) -> Result<BindingProofConsumptionV1, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingProofVerificationError {
    NotImplemented,
    UnsupportedSchemaVersion {
        observed: u32,
        supported: cell_placement::ProtocolVersionRangeV1,
    },
    UnsupportedSigningFormat {
        observed: u32,
        supported: cell_placement::ProtocolVersionRangeV1,
    },
    WrongDomain,
    EmptySignature,
    SignatureRejected,
    PayloadDigestMismatch,
    WrongProducer,
    WrongAudience,
    WrongKey,
    HistoricalKeyEvidenceUnavailable,
    HistoricalKeyEvidenceRejected,
    RevokedAtProofTime,
    ProofNotValidAtEvidenceTime,
    RelationMismatch,
    NotYetValid,
    Expired,
}
