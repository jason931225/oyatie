#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use hr_employment_domain::{
    AuditEvidenceRef, HrDomainError, OnboardingChecklistItem, OnboardingChecklistItemKind,
    OnboardingDecision, OnboardingReadinessInput, evaluate_onboarding_readiness,
};

include!(concat!(env!("OUT_DIR"), "/onboarding.generated.rs"));
