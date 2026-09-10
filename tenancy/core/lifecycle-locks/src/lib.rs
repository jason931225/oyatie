#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct LockId(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockReason {
    PendingDeletionGrace,
    JurisdictionMigration,
    KybReverification,
    DrPromotionWindow,
    PaymentDispute,
    LegalHold,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleLock {
    pub id: LockId,
    pub tenant_id: String,
    pub reason: LockReason,
    pub holder: String,
    pub expires_at_epoch_s: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockDecision {
    pub allow: bool,
    pub blocking_locks: Vec<LockId>,
    pub explanation: String,
}

pub fn evaluate(action: &str, locks: &[LifecycleLock]) -> LockDecision {
    let blockers: Vec<LockId> = locks.iter().map(|l| l.id.clone()).collect();
    let allow = blockers.is_empty();
    LockDecision {
        allow,
        blocking_locks: blockers,
        explanation: format!("action={action} stub-decision"),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LockKernelError {
    PrecedenceConflict,
    ReleaseUnauthorized,
    Expired,
}
