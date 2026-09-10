#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct QuotaKey {
    pub tenant_id: String,
    pub resource: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuotaDecision {
    pub limit: u64,
    pub effective: u64,
    pub source: QuotaSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuotaSource {
    ClassDefault,
    PackOverride,
    TenantOverride,
    HardCap,
}

pub trait TenantClassReader {
    fn class(&self, tenant_id: &str) -> Result<String, QuotaUsecaseError>;
}

pub trait QuotaOverrideRepository {
    fn lookup(&self, key: &QuotaKey) -> Result<Option<u64>, QuotaUsecaseError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuotaUsecaseError {
    UnknownTenant,
    PersistenceUnavailable,
}

pub fn resolve<C: TenantClassReader, O: QuotaOverrideRepository>(
    classes: &C,
    overrides: &O,
    key: &QuotaKey,
) -> Result<QuotaDecision, QuotaUsecaseError> {
    let _class = classes.class(&key.tenant_id)?;
    let override_value = overrides.lookup(key)?;
    let (limit, source) = override_value
        .map(|v| (v, QuotaSource::TenantOverride))
        .unwrap_or((0, QuotaSource::ClassDefault));
    Ok(QuotaDecision {
        limit,
        effective: limit,
        source,
    })
}
