use crate::{
    BindingDigest32, BindingProofVerificationError, BindingProofVerifier,
    SignedWriteAuthorityLeaseV1, WriteAuthorityLeaseStateRevision,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WriteAuthorityLeaseIssuanceRevision(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WriteAuthorityLeaseIssuanceStatusV1 {
    PendingSignature,
    Published,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WriteAuthorityLeaseIssuanceKindV1 {
    LocalInstallation,
    Renewal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseIssuanceRecordV1 {
    installed: crate::InstalledServingAuthorityV1,
    intent: crate::WriteAuthorityLeaseIntentV1,
    kind: WriteAuthorityLeaseIssuanceKindV1,
    lease_state_revision_at_commit: WriteAuthorityLeaseStateRevision,
    lease_state_record_digest_at_commit: BindingDigest32,
    status: WriteAuthorityLeaseIssuanceStatusV1,
    signed_lease: Option<SignedWriteAuthorityLeaseV1>,
    revision: WriteAuthorityLeaseIssuanceRevision,
    record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseIssuanceRecordPartsV1 {
    pub installed: crate::InstalledServingAuthorityV1,
    pub intent: crate::WriteAuthorityLeaseIntentV1,
    pub kind: WriteAuthorityLeaseIssuanceKindV1,
    pub lease_state_revision_at_commit: WriteAuthorityLeaseStateRevision,
    pub lease_state_record_digest_at_commit: BindingDigest32,
    pub status: WriteAuthorityLeaseIssuanceStatusV1,
    pub signed_lease: Option<SignedWriteAuthorityLeaseV1>,
    pub revision: WriteAuthorityLeaseIssuanceRevision,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WriteAuthorityLeaseIssuanceConstructionErrorV1 {
    NotImplemented,
    InvalidRevision,
    InvalidStatus,
    InvalidLeaseRelation,
    InvalidStateRelation,
    RecordDigestMismatch,
}

impl WriteAuthorityLeaseIssuanceRecordV1 {
    pub fn rehydrate(
        _parts: WriteAuthorityLeaseIssuanceRecordPartsV1,
    ) -> Result<Self, WriteAuthorityLeaseIssuanceConstructionErrorV1> {
        Err(WriteAuthorityLeaseIssuanceConstructionErrorV1::NotImplemented)
    }

    #[must_use]
    pub fn installed(&self) -> &crate::InstalledServingAuthorityV1 {
        &self.installed
    }

    #[must_use]
    pub fn intent(&self) -> &crate::WriteAuthorityLeaseIntentV1 {
        &self.intent
    }

    #[must_use]
    pub fn kind(&self) -> WriteAuthorityLeaseIssuanceKindV1 {
        self.kind
    }

    #[must_use]
    pub fn lease_state_revision_at_commit(&self) -> WriteAuthorityLeaseStateRevision {
        self.lease_state_revision_at_commit
    }

    #[must_use]
    pub fn lease_state_record_digest_at_commit(&self) -> BindingDigest32 {
        self.lease_state_record_digest_at_commit
    }

    #[must_use]
    pub fn status(&self) -> WriteAuthorityLeaseIssuanceStatusV1 {
        self.status
    }

    #[must_use]
    pub fn signed_lease(&self) -> Option<&SignedWriteAuthorityLeaseV1> {
        self.signed_lease.as_ref()
    }

    #[must_use]
    pub fn revision(&self) -> WriteAuthorityLeaseIssuanceRevision {
        self.revision
    }

    #[must_use]
    pub fn record_digest(&self) -> BindingDigest32 {
        self.record_digest
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseIssuancePreconditionV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub revision: WriteAuthorityLeaseIssuanceRevision,
    pub status: WriteAuthorityLeaseIssuanceStatusV1,
    pub record_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedWriteAuthorityLeaseIssuance(
    crate::CommittedWriteAuthorityLeaseIssuanceClaimV1,
);

impl VerifiedCommittedWriteAuthorityLeaseIssuance {
    #[must_use]
    pub fn record(&self) -> &WriteAuthorityLeaseIssuanceRecordV1 {
        &self.0.issuance
    }

    #[must_use]
    pub fn claim(&self) -> &crate::CommittedWriteAuthorityLeaseIssuanceClaimV1 {
        &self.0
    }

    #[must_use]
    pub fn attestation(&self) -> &crate::SignedWriteAuthorityLeaseCommitAttestationV1 {
        &self.0.attestation
    }
}

pub fn verify_committed_write_authority_lease_issuance(
    _verifier: &dyn BindingProofVerifier,
    _claim: crate::CommittedWriteAuthorityLeaseIssuanceClaimV1,
    _expectation: &crate::WriteAuthorityLeaseCommitAttestationExpectationV1,
) -> Result<VerifiedCommittedWriteAuthorityLeaseIssuance, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}
