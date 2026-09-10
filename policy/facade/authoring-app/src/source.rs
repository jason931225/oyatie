use policy_pdp_kernel::EntitySlice;
use serde::{Deserialize, Serialize};
use shared_platform_contracts_kernel::pdp::{AuthorizationRequest, Decision, Obligation};

pub use policy_bundle_content::PolicySource;

/// Expected complete enforcement content; random correlation IDs are excluded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionExpectation {
    pub decision: Decision,
    pub determining_policy_ids: Vec<String>,
    pub obligations: Vec<Obligation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyCase {
    pub name: String,
    pub request: AuthorizationRequest,
    pub entities: EntitySlice,
    pub expected: DecisionExpectation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyProject {
    pub source: PolicySource,
    pub cases: Vec<PolicyCase>,
}
