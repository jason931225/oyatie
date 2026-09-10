use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};

const EMPLOYEE_SCHEMA_VERSION: u32 = 1;
const HR_EVENT_SCHEMA_VERSION: u32 = 1;
const LABOR_OBLIGATION_SCHEMA_VERSION: u32 = 1;
const EMPLOYEE_ID_PREFIX: &str = "emp_";
const TENANT_ID_PREFIX: &str = "ten_";
const LEGAL_ENTITY_ID_PREFIX: &str = "le_";
const PERSON_REF_PREFIX: &str = "person/";
const AUDIT_EVIDENCE_PREFIX: &str = "audit/";
const RULEPACK_REF_PREFIX: &str = "rulepack/";
const HR_EVENT_ID_PREFIX: &str = "hrev_";
const WORKFLOW_REF_PREFIX: &str = "workflow/";
const LABOR_OBLIGATION_ID_PREFIX: &str = "hrobl_";
const LEAVE_REQUEST_ID_PREFIX: &str = "leave_";
const HR_RULEPACK_SOURCE_REF_PREFIX: &str = "hr-rulepack-source/";
const SOURCE_DIGEST_PREFIX: &str = "sha256:";
const LEAVE_PAYROLL_IMPACT_SCHEMA_VERSION: u32 = 1;
const HR_POLICY_REF_PREFIX: &str = "policy/hr/sensitive-read/";
const SENSITIVE_HR_READ_SCHEMA_VERSION: u32 = 1;
const HR_STATUTORY_RULEPACK_SCHEMA_VERSION: u32 = 1;
const LEAVE_BALANCE_LEDGER_SCHEMA_VERSION: u32 = 1;
const LEAVE_CARRYOVER_FORFEITURE_SCHEMA_VERSION: u32 = 1;
const ONBOARDING_READINESS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EmployeeId {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TenantId {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LegalEntityId {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PersonRef {
    pub value: String, // data_class: PII_IDENTIFYING
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct AuditEvidenceRef {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct HrEventId {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RulepackRef {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct WorkflowRef {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PolicyRef {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LaborComplianceObligationId {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LeaveRequestId {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RulepackEffectiveDate {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RulepackSourceDigest {
    pub value: String,
}
