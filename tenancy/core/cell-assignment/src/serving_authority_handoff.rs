use crate::{BindingDigest32, BindingStoreError, BoxTenancyFuture};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServingAuthorityHandoffProgressV1 {
    CommittedPotentiallyInstallable,
    Installed(Box<crate::SignedServingAuthorityInstallationResultV1>),
    FreezeRequested(Box<crate::ServingAuthorityFreezeIntentV1>),
    RetiredAwaitingEffectFencing(Box<crate::ServingAuthorityRetirementEvidenceV1>),
    TerminalFenced { closure_digest: BindingDigest32 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityHandoffRecordPartsV1 {
    pub control_partition: crate::TenantControlPartitionRefV1,
    pub issuance: crate::ServingAuthorityInstallationIssuanceV1,
    pub signed_install_grant: Option<crate::SignedServingAuthorityInstallGrantV1>,
    pub progress: ServingAuthorityHandoffProgressV1,
    pub revision: u64,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityHandoffRecordV1 {
    parts: ServingAuthorityHandoffRecordPartsV1,
}

impl ServingAuthorityHandoffRecordV1 {
    pub fn rehydrate(
        _parts: ServingAuthorityHandoffRecordPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &ServingAuthorityHandoffRecordPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityTerminalClosurePayloadV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub installation_issuance_digest: BindingDigest32,
    pub retirement: crate::ServingAuthorityRetirementEvidenceV1,
    pub complete_effect_path_fencing_digest: BindingDigest32,
    pub closure_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthorityTerminalClosure(ServingAuthorityTerminalClosurePayloadV1);

impl VerifiedServingAuthorityTerminalClosure {
    #[must_use]
    pub fn payload(&self) -> &ServingAuthorityTerminalClosurePayloadV1 {
        &self.0
    }
}

pub fn verify_serving_authority_terminal_closure(
    _retirement: &crate::VerifiedServingAuthorityRetirementV1,
    _effect_fencing: &crate::VerifiedRetiredSourceEffectClosureV1,
    _expected: &crate::ServingAuthorityHandoffExpectationV1,
    _payload: ServingAuthorityTerminalClosurePayloadV1,
) -> Result<VerifiedServingAuthorityTerminalClosure, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServingAuthorityControlHandoffEvidenceV1 {
    PublishedInstallGrant(Box<crate::VerifiedServingAuthorityInstallGrant>),
    Installed(Box<crate::VerifiedServingAuthorityInstallationResult>),
    Retired(Box<crate::VerifiedServingAuthorityRetirementV1>),
    TerminalFenced(Box<VerifiedServingAuthorityTerminalClosure>),
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServingAuthorityControlHandoffAuthorityV1 {
    Request(crate::BindingPersistenceAuthorityV1),
    Reconciler(crate::BindingReconciliationPersistenceAuthorityV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityHandoffPreconditionV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub installation_issuance_digest: BindingDigest32,
    pub revision: u64,
    pub record_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityControlHandoffWriteSetV1 {
    parts: ServingAuthorityControlHandoffWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityControlHandoffWriteSetPartsV1 {
    pub control_partition: crate::TenantControlPartitionRefV1,
    pub authority: ServingAuthorityControlHandoffAuthorityV1,
    pub instance: crate::ServingAuthorityInstanceV1,
    pub expected_handoff_revision: u64,
    pub expected_handoff_digest: BindingDigest32,
    pub evidence: ServingAuthorityControlHandoffEvidenceV1,
    pub next_handoff: ServingAuthorityHandoffRecordV1,
    pub operation_precondition: crate::BindingOperationPreconditionV1,
    pub operation: crate::BindingOperationV1,
    pub idempotency: crate::BindingIdempotencyRecordV1,
    pub proof_consumptions: Vec<crate::BindingProofConsumptionV1>,
    pub audit_outbox: crate::BindingAuditRecordV1,
}

impl ServingAuthorityControlHandoffWriteSetV1 {
    pub fn assemble(
        _parts: ServingAuthorityControlHandoffWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &ServingAuthorityControlHandoffWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityControlHandoffResultV1 {
    pub handoff: ServingAuthorityHandoffRecordV1,
    pub operation: crate::BindingOperationV1,
}

pub trait ServingAuthorityControlHandoffStore: Send + Sync {
    fn get_handoff<'a>(
        &'a self,
        authority: &'a crate::BindingReadAuthorityV1,
        instance: &'a crate::ServingAuthorityInstanceV1,
    ) -> BoxTenancyFuture<'a, Result<Option<ServingAuthorityHandoffRecordV1>, BindingStoreError>>;

    /// Reads back the UNSIGNED durable installation issuance, or `None` when
    /// none was ever committed.
    ///
    /// This returns the issuance rather than the full
    /// [`crate::CommittedServingAuthorityInstallationClaimV1`] because the
    /// claim's other member is a signed control-commit attestation, and no
    /// write path carries one into this store: the handoff write set's
    /// `evidence` and `next_handoff` members carry grants and results, never an
    /// attestation. Returning the claim would have obliged the store to produce
    /// a signature only it could have forged.
    ///
    /// The caller pairs this record with a fresh attestation from
    /// [`ServingAuthorityControlCommitObserver::observe_installation_commit`],
    /// assembles the claim and verifies it.
    ///
    /// Absence stays distinct from "result unknown": this is a reconciler
    /// decision path, and collapsing "never committed" into an error would make
    /// the reconciler unable to tell a missing commit from a failed read.
    fn load_installation_issuance<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationPersistenceAuthorityV1,
        query: &'a crate::ServingAuthorityHandoffExpectationV1,
        lease: &'a crate::BindingReconciliationLeaseV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::ServingAuthorityInstallationIssuanceV1>, BindingStoreError>,
    >;

    /// Reads back the UNSIGNED durable freeze intent, or `None` when none was
    /// ever committed. See [`Self::load_installation_issuance`] for why this is
    /// the intent rather than the claim, and why absence is not an error.
    fn load_freeze_intent<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationPersistenceAuthorityV1,
        query: &'a crate::ServingAuthorityHandoffExpectationV1,
        lease: &'a crate::BindingReconciliationLeaseV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::ServingAuthorityFreezeIntentV1>, BindingStoreError>,
    >;

    fn record_handoff_result<'a>(
        &'a self,
        write_set: &'a ServingAuthorityControlHandoffWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<ServingAuthorityControlHandoffResultV1, BindingStoreError>>;
}

/// Independently observes a committed serving-authority control write and signs
/// what it read.
///
/// `SignedServingAuthorityControlCommitAttestationV1` had a consumer, a proof
/// domain (`BindingProofDomainV1::ServingAuthorityControlCommitAttestation`)
/// and a verifier, but no producer: the claims embedding it
/// (`CommittedServingAuthorityInstallationClaimV1`,
/// `CommittedServingAuthorityFreezeClaimV1`) are loaded by
/// [`ServingAuthorityControlHandoffStore`] and were never built. This port is
/// that missing producer, for both control actions.
///
/// Separate from the store on purpose: the party that performed the control
/// write must not be the party that attests it committed. Both methods take a
/// reconciliation read authority and the same lookup the loaders take —
/// control partition, instance and business id, plus the producer, audience and
/// clock the observer needs to mint an envelope. Neither accepts a caller's
/// claim about the committed row; every committed value in the attestation
/// describes what the observer itself re-read.
///
/// No new proof domain is required for either method: one attestation type
/// covers both control actions, disambiguated by
/// [`crate::ServingAuthorityBusinessIdV1`], which is how the existing domain
/// was already designed.
///
/// WHY THESE RETURN A LONE SIGNATURE WHEN THE THREE COMMIT OBSERVERS BORN
/// BESIDE THEM RETURN THE RECORD WITH IT. Handing back a lone signature
/// normally puts the caller in charge of pairing it with a record, which
/// reopens a narrower version of the hazard the barrier exists to close. It
/// does not here, for two reasons that are properties of this attestation
/// rather than of the caller. The payload pins the row it attests BY DIGEST —
/// `committed_binding_digest`, `committed_issuance_revision` and
/// `committed_issuance_digest` — so a caller pairing it with a different row is
/// refused by the verifier rather than believed. And ONE attestation type
/// serves TWO claim types
/// ([`crate::CommittedServingAuthorityInstallationClaimV1`] and
/// [`crate::CommittedServingAuthorityFreezeClaimV1`]), so a claim-typed
/// observer would have to choose the claim shape for the caller off
/// [`crate::ServingAuthorityBusinessIdV1`] and re-read the row the caller's own
/// loader already returned.
pub trait ServingAuthorityControlCommitObserver: Send + Sync {
    fn observe_installation_commit<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationReadAuthorityV1,
        query: &'a crate::ServingAuthorityHandoffExpectationV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::SignedServingAuthorityControlCommitAttestationV1>, BindingStoreError>,
    >;

    fn observe_freeze_commit<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationReadAuthorityV1,
        query: &'a crate::ServingAuthorityHandoffExpectationV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::SignedServingAuthorityControlCommitAttestationV1>, BindingStoreError>,
    >;
}
