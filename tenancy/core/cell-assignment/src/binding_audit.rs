use cell_placement::AssuranceAuditPolicyV1;

use crate::{
    BindingAuditEffectV1, BindingAuthorizationDecisionReceiptV1, BindingDigest32,
    BindingIdempotencyKey, BindingOperationKey, BindingPersistenceAuthorityV1, BindingStoreError,
    TenantId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingAuditRecordV1 {
    parts: BindingAuditRecordPartsV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingAuditRecordPartsV1 {
    pub audit_event_id: String,
    pub tenant_id: TenantId,
    pub operation: BindingOperationKey,
    pub actor_digest: BindingDigest32,
    pub authorization: BindingAuthorizationDecisionReceiptV1,
    pub idempotency_key: BindingIdempotencyKey,
    pub request_digest: BindingDigest32,
    pub effect: BindingAuditEffectV1,
    pub occurred_at_unix_seconds: u64,
    pub assurance_audit_policy: AssuranceAuditPolicyV1,
    pub record_digest: BindingDigest32,
}

impl BindingAuditRecordV1 {
    pub fn assemble(
        _authority: &BindingPersistenceAuthorityV1,
        _parts: BindingAuditRecordPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    pub fn assemble_for_reconciliation(
        _authority: &crate::BindingReconciliationPersistenceAuthorityV1,
        _expected_lease: &crate::BindingReconciliationLeaseV1,
        _parts: BindingAuditRecordPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    pub fn assemble_for_serving_authority(
        _authority: &crate::VerifiedServingAuthorityInvocation,
        _parts: BindingAuditRecordPartsV1,
    ) -> Result<Self, crate::ServingAuthorityStoreError> {
        Err(crate::ServingAuthorityStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &BindingAuditRecordPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingAuditPageTokenV1(Vec<u8>);

impl BindingAuditPageTokenV1 {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn parse(_value: Vec<u8>) -> Result<Self, crate::BindingContractError> {
        Err(crate::BindingContractError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingAuditPageRequestV1 {
    pub tenant_id: TenantId,
    pub page_size: u32,
    pub page_token: Option<BindingAuditPageTokenV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingAuditPageV1 {
    pub records: Vec<BindingAuditRecordV1>,
    pub next_page_token: Option<BindingAuditPageTokenV1>,
}

pub trait BindingAuditReader: Send + Sync {
    fn read_page<'a>(
        &'a self,
        authority: &'a crate::BindingReadAuthorityV1,
        request: &'a BindingAuditPageRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingAuditPageV1, BindingStoreError>>;
}
