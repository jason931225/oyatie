#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct VerificationCaseId(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationKind {
    Kyb,
    Kyc,
    Ubo,
    Sanctions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationDecision {
    Pending,
    Approved,
    Rejected,
    EscalatedToHuman,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentRequirement {
    pub name: String,
    pub mandatory: bool,
    pub jurisdiction: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScreeningResult {
    pub provider: String,
    pub hit: bool,
    pub details: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationCase {
    pub id: VerificationCaseId,
    pub kind: VerificationKind,
    pub decision: VerificationDecision,
    pub requirements: Vec<DocumentRequirement>,
    pub screenings: Vec<ScreeningResult>,
}

pub fn decide(case: &VerificationCase) -> VerificationDecision {
    case.decision
}
