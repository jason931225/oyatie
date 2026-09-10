#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationFailureReason {
    KeyEpochMismatch,
    SignatureInvalid,
    ProofInvalid,
    PriorRootMissing,
    PackMismatch,
    RedactedEvent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationVerdict {
    Verified,
    Failed(VerificationFailureReason),
}
