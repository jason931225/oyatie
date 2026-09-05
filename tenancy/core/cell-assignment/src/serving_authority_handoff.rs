use crate::{BindingDigest32, BindingStoreError, BoxTenancyFuture};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServingAuthorityHandoffProgressV1 {
    CommittedPotentiallyInstallable,
    Installed(Box<crate::ServingAuthorityInstallationResultV1>),
    FreezeRequested(Box<crate::ServingAuthorityFreezeIntentV1>),
    FrozenAwaitingEffectFencing(Box<crate::ServingAuthorityFreezeResultV1>),
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
    pub freeze_result_digest: BindingDigest32,
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
    _freeze: &crate::VerifiedServingAuthorityFreezeResult,
    _effect_fencing: &crate::VerifiedSourceFencingCompletionV1,
    _expected: &crate::ServingAuthorityHandoffExpectationV1,
    _payload: ServingAuthorityTerminalClosurePayloadV1,
) -> Result<VerifiedServingAuthorityTerminalClosure, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServingAuthorityControlHandoffEvidenceV1 {
    PublishedInstallGrant(Box<crate::VerifiedServingAuthorityInstallGrant>),
    Installed(Box<crate::VerifiedServingAuthorityInstallationResult>),
    Frozen(Box<crate::VerifiedServingAuthorityFreezeResult>),
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

    fn load_installation_claim<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationPersistenceAuthorityV1,
        query: &'a crate::ServingAuthorityHandoffExpectationV1,
        lease: &'a crate::BindingReconciliationLeaseV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::CommittedServingAuthorityInstallationClaimV1, BindingStoreError>,
    >;

    fn load_freeze_claim<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationPersistenceAuthorityV1,
        query: &'a crate::ServingAuthorityHandoffExpectationV1,
        lease: &'a crate::BindingReconciliationLeaseV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::CommittedServingAuthorityFreezeClaimV1, BindingStoreError>,
    >;

    fn record_handoff_result<'a>(
        &'a self,
        write_set: &'a ServingAuthorityControlHandoffWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<ServingAuthorityControlHandoffResultV1, BindingStoreError>>;
}
