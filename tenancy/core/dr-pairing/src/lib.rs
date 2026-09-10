#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrPair {
    pub tenant_id: String,
    pub home_cell: String,
    pub dr_cell: String,
    pub jurisdiction: String,
    pub pair_version: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromotionDecision {
    Eligible,
    Blocked { reason_code: u16 },
}

pub trait DrPairRepository {
    fn current(&self, tenant_id: &str) -> Result<Option<DrPair>, DrPairingError>;
    fn record(&self, pair: &DrPair) -> Result<(), DrPairingError>;
}

pub trait DrSloProbe {
    fn dr_replica_health(&self, cell: &str) -> Result<bool, DrPairingError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrPairingError {
    PairNotFound,
    JurisdictionMismatch,
    SloProbeFailed,
    PersistenceUnavailable,
}

pub fn evaluate_promotion<R: DrPairRepository, S: DrSloProbe>(
    _repo: &R,
    _probe: &S,
    _tenant_id: &str,
) -> Result<PromotionDecision, DrPairingError> {
    Ok(PromotionDecision::Blocked { reason_code: 0 })
}
