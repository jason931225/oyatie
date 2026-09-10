use std::fmt;

use crate::{IdempotencyKey, ResourceName, ResourceProvider};

use super::violation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceViolation {
    pub check: &'static str,
    pub detail: String,
}

impl fmt::Display for ConformanceViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.check, self.detail)
    }
}

impl std::error::Error for ConformanceViolation {}

/// What a service supplies to run the harness: a fresh provider per check
/// plus deterministic, ordinal-indexed fixtures. Distinct ordinals MUST
/// yield distinct payloads (`resource_payload(a) != resource_payload(b)` for
/// `a != b`).
pub trait ConformanceFixture {
    type Provider: ResourceProvider;

    fn fresh_provider(&self) -> Self::Provider;

    fn collection(&self) -> &str;

    fn resource_payload(&self, ordinal: u32) -> <Self::Provider as ResourceProvider>::Resource;

    fn resource_name(&self, ordinal: u32) -> Result<ResourceName, ConformanceViolation> {
        ResourceName::new(self.collection(), format!("res-{ordinal:04}"))
            .map_err(|error| violation("fixture", error.to_string()))
    }

    fn idempotency_key(&self, ordinal: u32) -> Result<IdempotencyKey, ConformanceViolation> {
        IdempotencyKey::new(format!("00000000-0000-4000-8000-{ordinal:012x}"))
            .map_err(|error| violation("fixture", error.to_string()))
    }

    fn resource_orn(&self, name: &ResourceName) -> String {
        format!(
            "orn:oya:local-test:account-test:{}:{}/{}",
            name.collection(),
            name.collection(),
            name.resource_id()
        )
    }

    fn tenant_account_project(&self) -> &str {
        "tenant-test/account-test/project-test"
    }

    fn region_cell(&self) -> &str {
        "local-test/cell-0001"
    }

    fn principal(&self) -> &str {
        "principal:test-harness"
    }
}
