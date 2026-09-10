#![allow(dead_code)]

use audit_sealing_kernel::{SealRecord, SealStatus};

#[derive(Clone, Debug)]
pub struct SealCycleCommand {
    pub pack: String,
    pub tenant_partition: String,
    pub period_id: String,
}

#[derive(Clone, Debug)]
pub enum SealCycleResult {
    Minted(SealRecord),
    AlreadyMinted(SealRecord),
    Degraded { reason: String, status: SealStatus },
}
