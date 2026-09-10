#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DsrRequestId(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DsrKind {
    Access,
    Erasure,
    Rectification,
    Portability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DsrRequest {
    pub id: DsrRequestId,
    pub tenant_id: String,
    pub subject_id: String,
    pub kind: DsrKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErasureReceipt {
    pub request: DsrRequestId,
    pub microservice: String,
    pub merkle_leaf: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofOfErasure {
    pub request: DsrRequestId,
    pub merkle_root: [u8; 32],
    pub receipts: Vec<ErasureReceipt>,
}

pub trait DsrRequestRepository {
    fn open(&self, request: &DsrRequest) -> Result<(), DsrKernelError>;
    fn append_receipt(&self, receipt: &ErasureReceipt) -> Result<(), DsrKernelError>;
    fn finalize(&self, proof: &ProofOfErasure) -> Result<(), DsrKernelError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DsrKernelError {
    UnknownRequest,
    DuplicateReceipt,
    MerkleAggregationFailed,
    SlaBreached,
}
