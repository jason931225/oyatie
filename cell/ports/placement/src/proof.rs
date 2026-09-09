pub const CELL_PROOF_ENVELOPE_SCHEMA_VERSION: u32 = 1;
pub const CELL_PROOF_SIGNING_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Digest32([u8; 32]);

impl Digest32 {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
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

            pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
                Err(ProofConstructionError::NotImplemented)
            }
        }
    };
}

opaque_id!(ProducerId);
opaque_id!(KeyId);
opaque_id!(ProofNonce);
opaque_id!(PlacementIdempotencyKey);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CellProofDomainV1 {
    PlacementInvocation,
    PlacementIntent,
    PlacementDecision,
    PlacementContinuation,
    PlacementExhaustion,
    AssuranceCompilation,
    AssuranceEvidence,
    RecoveryEvidence,
    CellControlInvocation,
    DrainContributorManifest,
    DrainContributorProof,
    DrainCompletion,
    ReleaseCompatibility,
    PromotionEvidence,
    ReservationArmIntent,
    ReservationArmReceipt,
    ReservationCommitPermit,
    BindingOutcome,
    BindingParticipantManifestCommitment,
    MovementPermit,
    RepairAuthority,
    ReconciliationInvocation,
    MovementBudgetSettlementClaim,
    DrainContributorSeal,
    DrainContributorSealCommitObservation,
    CellControlRepairAuthority,
    PlacementActor,
    PlacementPolicyDecision,
    MovementActionClosure,
    MovementActionResult,
    PromotionEconomicsSourceFinalization,
    PromotionEconomicsCheckpointCommitObservation,
    CapabilityEffectGrant,
    LocalEffectCommitReceipt,
    CapabilityAdapterAcceptance,
    CapabilityReceiptRecovery,
    LocalEffectReceiptCommitObservation,
    RebalanceInvocationIssuanceCommit,
    MovementActionClosureCommit,
    MovementActionResultCommit,
    MovementPermitCommitObservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellProofEnvelopeV1 {
    pub schema_version: u32,
    pub signing_format_version: u32,
    pub domain: CellProofDomainV1,
    pub producer: ProducerId,
    pub audience: ProducerId,
    pub key_id: KeyId,
    pub nonce: ProofNonce,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub payload_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellProofReuseScopeV1 {
    Operation(crate::PlacementOperationKey),
    ControlOperation(crate::CellControlOperationKeyV1),
    Reservation(crate::ReservationRefV1),
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellProofConsumptionV1 {
    scope: CellProofReuseScopeV1,
    domain: CellProofDomainV1,
    producer: ProducerId,
    nonce: ProofNonce,
    payload_digest: Digest32,
    envelope_digest: Digest32,
}

impl CellProofConsumptionV1 {
    #[must_use]
    pub fn scope(&self) -> &CellProofReuseScopeV1 {
        &self.scope
    }

    #[must_use]
    pub fn domain(&self) -> CellProofDomainV1 {
        self.domain
    }

    #[must_use]
    pub fn producer(&self) -> &ProducerId {
        &self.producer
    }

    #[must_use]
    pub fn nonce(&self) -> &ProofNonce {
        &self.nonce
    }

    #[must_use]
    pub fn payload_digest(&self) -> Digest32 {
        self.payload_digest
    }

    #[must_use]
    pub fn envelope_digest(&self) -> Digest32 {
        self.envelope_digest
    }
}

/// The verified proofs that can be recorded as CONSUMED by
/// [`bind_cell_proof_consumption`].
///
/// This is not "every verified proof". A variant belongs here only when some
/// write set actually carries the resulting [`CellProofConsumptionV1`], because
/// a consumption with nowhere durable to land records nothing. Read evidence
/// needs no variant at all: nothing is spent by reading.
///
/// THE TEST FOR A VARIANT is a place to land, in the SAME durable transaction:
/// the verified value must reach a write set that carries
/// `proof_consumptions`, whether directly or inside a value minted from it. A
/// `proof_consumptions` field on a write set in a different transaction is not
/// a destination -- `RebalanceJobWriteSetPartsV1` has the field and is the
/// JOB-level write under `CellControlPersistenceAuthorityV1`, so it does not
/// qualify the issuance or closure paths.
///
/// The rebalance-source movement path has no variant because no write set it
/// reaches carries `proof_consumptions` at all. Replay there is refused by
/// state instead: `RebalanceSourcePreconditionV1` and
/// `RebalanceActionPreconditionV1` pin the revisions, digests and states a
/// second issuance would have to restate.
#[derive(Clone, Copy, Debug)]
pub enum VerifiedCellProofRefV1<'a> {
    PlacementInvocation(&'a crate::VerifiedPlacementInvocation),
    PlacementIntent(&'a crate::VerifiedPlacementIntent),
    PlacementDecision(&'a crate::VerifiedCellPlacementDecision),
    AssuranceCompilation(&'a crate::VerifiedAssuranceCompilation),
    AssuranceEvidence(&'a crate::VerifiedAssuranceEvidence),
    RecoveryEvidence(&'a crate::VerifiedRecoveryEvidence),
    CellControlInvocation(&'a crate::VerifiedCellControlInvocation),
    DrainContributorManifest(&'a crate::VerifiedDrainContributorManifest),
    DrainContributorProof(&'a crate::VerifiedDrainContributorProof),
    DrainCompletion(&'a crate::VerifiedCellDrainCompletion),
    ReleaseCompatibility(&'a crate::VerifiedOwnerReleaseCompatibility),
    PromotionEvidence(&'a crate::VerifiedCellPromotionEvidence),
    ReservationArmIntent(&'a crate::VerifiedReservationArmIntent),
    ReservationArmReceipt(&'a crate::VerifiedReservationArmReceipt),
    ReservationCommitPermit(&'a crate::VerifiedReservationCommitPermit),
    BindingOutcome(&'a crate::VerifiedBindingOutcome),
    BindingParticipantManifestCommitment(&'a crate::VerifiedBindingParticipantManifestCommitment),
    MovementPermit(&'a crate::VerifiedCellMovementPermit),
    RepairAuthority(&'a crate::VerifiedPlacementRepairAuthority),
    ReconciliationInvocation(&'a crate::VerifiedReconciliationInvocation),
    MovementBudgetSettlementClaim(&'a crate::VerifiedMovementBudgetSettlementClaim),
    DrainContributorSeal(&'a crate::VerifiedDrainContributorSeal),
    DrainContributorSealCommitObservation(&'a crate::VerifiedCommittedDrainContributorSeal),
    MovementPermitCommitObservation(&'a crate::VerifiedCommittedMovementPermitIssuance),
    CellControlRepairAuthority(&'a crate::VerifiedCellControlRepairAuthority),
}

impl<'a> VerifiedCellProofRefV1<'a> {
    #[must_use]
    pub fn proof_envelope(self) -> &'a CellProofEnvelopeV1 {
        match self {
            Self::PlacementInvocation(proof) => &proof.signed().envelope,
            Self::PlacementIntent(proof) => &proof.signed().envelope,
            Self::PlacementDecision(proof) => &proof.signed().envelope,
            Self::AssuranceCompilation(proof) => &proof.signed().envelope,
            Self::AssuranceEvidence(proof) => &proof.signed().envelope,
            Self::RecoveryEvidence(proof) => &proof.signed().envelope,
            Self::CellControlInvocation(proof) => &proof.signed().envelope,
            Self::DrainContributorManifest(proof) => &proof.signed().envelope,
            Self::DrainContributorProof(proof) => &proof.signed().envelope,
            Self::DrainCompletion(proof) => &proof.signed().envelope,
            Self::ReleaseCompatibility(proof) => &proof.signed().envelope,
            Self::PromotionEvidence(proof) => &proof.signed().envelope,
            Self::ReservationArmIntent(proof) => &proof.signed().envelope,
            Self::ReservationArmReceipt(proof) => &proof.signed().envelope,
            Self::ReservationCommitPermit(proof) => &proof.signed().envelope,
            Self::BindingOutcome(proof) => &proof.signed().envelope,
            Self::BindingParticipantManifestCommitment(proof) => &proof.signed().envelope,
            Self::MovementPermit(proof) => &proof.signed().envelope,
            Self::RepairAuthority(proof) => &proof.signed().envelope,
            Self::ReconciliationInvocation(proof) => &proof.signed().envelope,
            Self::MovementBudgetSettlementClaim(proof) => &proof.signed().envelope,
            Self::DrainContributorSeal(proof) => &proof.signed().envelope,
            Self::DrainContributorSealCommitObservation(proof) => {
                &proof.claim().observation.envelope
            }
            Self::MovementPermitCommitObservation(proof) => &proof.claim().observation.envelope,
            Self::CellControlRepairAuthority(proof) => &proof.signed().envelope,
        }
    }
}

pub fn bind_cell_proof_consumption(
    _proof: VerifiedCellProofRefV1<'_>,
    _scope: CellProofReuseScopeV1,
) -> Result<CellProofConsumptionV1, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellProofSigningPreimageV1 {
    domain_prefix: &'static str,
    envelope: CellProofEnvelopeV1,
    canonical_payload: Vec<u8>,
}

impl CellProofSigningPreimageV1 {
    #[must_use]
    pub fn domain_prefix(&self) -> &'static str {
        self.domain_prefix
    }

    #[must_use]
    pub fn envelope(&self) -> &CellProofEnvelopeV1 {
        &self.envelope
    }

    #[must_use]
    pub fn canonical_payload(&self) -> &[u8] {
        &self.canonical_payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProofConstructionError {
    NotImplemented,
    InvalidIdentifier,
    InvalidCanonicalPayload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProofVerificationError {
    NotImplemented,
    UnsupportedSchemaVersion {
        observed: u32,
        supported: crate::ProtocolVersionRangeV1,
    },
    UnsupportedSigningFormat {
        observed: u32,
        supported: crate::ProtocolVersionRangeV1,
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

pub trait CellProofVerifier: Send + Sync {
    fn verify_signature(
        &self,
        preimage: &CellProofSigningPreimageV1,
        signature: &[u8],
    ) -> Result<(), ProofVerificationError>;
}
