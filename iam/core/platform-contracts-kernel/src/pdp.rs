//! PDP contract family: authorization request/response.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ContractViolation, MAX_ID_LEN, check_opaque_token, check_slug};

/// Opaque policy-store version token; despite the derived `Ord`, compare only for equality.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PolicyVersion(String);

impl PolicyVersion {
    pub fn new(token: impl Into<String>) -> Result<Self, Vec<ContractViolation>> {
        let token = token.into();
        let mut out = Vec::new();
        check_opaque_token("policy_version", &token, &mut out);
        if out.is_empty() {
            Ok(Self(token))
        } else {
            Err(out)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityRef {
    /// Namespaced entity type, e.g. `OyaPlatform::TenantResource`.
    pub entity_type: String, // data_class: INTERNAL_ONLY
    pub entity_id: String, // data_class: TENANT_SCOPED
}

impl EntityRef {
    fn collect_violations(&self, field: &'static str, out: &mut Vec<ContractViolation>) {
        if self.entity_type.is_empty() {
            out.push(ContractViolation::MissingValue { field });
        } else {
            let type_ok = self
                .entity_type
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':');
            if !type_ok {
                out.push(ContractViolation::InvalidCharset {
                    field,
                    value: self.entity_type.clone(),
                });
            }
        }
        if self.entity_id.is_empty() {
            out.push(ContractViolation::MissingValue { field });
        }
    }
}

/// The authorization decision. There is no "not applicable": absence of a permit IS a deny.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny,
}

impl Decision {
    #[must_use]
    pub fn is_allow(self) -> bool {
        matches!(self, Self::Allow)
    }
}

/// A PARC-shaped authorization request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationRequest {
    /// Caller-supplied correlation id, echoed on the response.
    pub request_id: String, // data_class: INTERNAL_ONLY
    pub tenant_id: String,    // data_class: TENANT_SCOPED
    pub principal: EntityRef, // data_class: TENANT_SCOPED
    /// Action id within the platform action namespace (slug form).
    pub action: String, // data_class: INTERNAL_ONLY
    pub resource: EntityRef,  // data_class: TENANT_SCOPED
    /// ABAC context exposed to attribute conditions (deterministic order).
    pub context: BTreeMap<String, serde_json::Value>, // data_class: TENANT_SCOPED
    /// Zookie freshness floor: the PDP MUST evaluate against a version at least this fresh.
    pub min_policy_version: Option<PolicyVersion>, // data_class: INTERNAL_ONLY
}

impl AuthorizationRequest {
    /// Surface-all invariant check.
    pub fn validate(&self) -> Result<(), Vec<ContractViolation>> {
        let mut out = Vec::new();
        check_opaque_token(
            "authorization_request.request_id",
            &self.request_id,
            &mut out,
        );
        check_slug(
            "authorization_request.tenant_id",
            &self.tenant_id,
            MAX_ID_LEN,
            &mut out,
        );
        check_slug(
            "authorization_request.action",
            &self.action,
            MAX_ID_LEN,
            &mut out,
        );
        self.principal
            .collect_violations("authorization_request.principal", &mut out);
        self.resource
            .collect_violations("authorization_request.resource", &mut out);
        if out.is_empty() { Ok(()) } else { Err(out) }
    }
}

/// An obligation attached to an allow. PEPs MUST enforce obligations or fail closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obligation {
    pub obligation_id: String,                // data_class: INTERNAL_ONLY
    pub parameters: BTreeMap<String, String>, // data_class: INTERNAL_ONLY
}

/// The PDP response: decision id, decision, and the policy version evaluated against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationResponse {
    /// Unique decision id; the audit-chain correlation key for this decision.
    pub decision_id: String, // data_class: INTERNAL_ONLY
    /// Echo of the request's correlation id.
    pub request_id: String, // data_class: INTERNAL_ONLY
    pub decision: Decision, // data_class: INTERNAL_ONLY
    /// The policy-store version evaluated against; pass it back as `min_policy_version`.
    pub policy_version: PolicyVersion, // data_class: INTERNAL_ONLY
    /// Ids of the policies that determined the outcome; an `Allow` MUST name a permit policy.
    pub determining_policy_ids: Vec<String>, // data_class: INTERNAL_ONLY
    pub obligations: Vec<Obligation>, // data_class: INTERNAL_ONLY
}

impl AuthorizationResponse {
    /// Surface-all invariant check.
    pub fn validate(&self) -> Result<(), Vec<ContractViolation>> {
        let mut out = Vec::new();
        check_opaque_token(
            "authorization_response.decision_id",
            &self.decision_id,
            &mut out,
        );
        check_opaque_token(
            "authorization_response.request_id",
            &self.request_id,
            &mut out,
        );
        if self.decision.is_allow() && self.determining_policy_ids.is_empty() {
            out.push(ContractViolation::InvalidShape {
                field: "authorization_response.determining_policy_ids",
                detail: "an allow must be attributable to at least one permit policy".to_owned(),
            });
        }
        for policy_id in &self.determining_policy_ids {
            check_opaque_token(
                "authorization_response.determining_policy_ids",
                policy_id,
                &mut out,
            );
        }
        for obligation in &self.obligations {
            check_slug(
                "authorization_response.obligations",
                &obligation.obligation_id,
                MAX_ID_LEN,
                &mut out,
            );
        }
        if out.is_empty() { Ok(()) } else { Err(out) }
    }

    /// Whether this response satisfies a caller's exact zookie freshness floor.
    #[must_use]
    pub fn satisfies_exact_version(&self, required: &PolicyVersion) -> bool {
        &self.policy_version == required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> AuthorizationRequest {
        AuthorizationRequest {
            request_id: "req-7f3a".to_owned(),
            tenant_id: "acme".to_owned(),
            principal: EntityRef {
                entity_type: "OyaPlatform::Principal".to_owned(),
                entity_id: "alice".to_owned(),
            },
            action: "tenancy.read".to_owned(),
            resource: EntityRef {
                entity_type: "OyaPlatform::TenantResource".to_owned(),
                entity_id: "documents/doc-1".to_owned(),
            },
            context: BTreeMap::from([(
                "channel".to_owned(),
                serde_json::Value::String("console".to_owned()),
            )]),
            min_policy_version: Some(PolicyVersion::new("psv-000042").unwrap()),
        }
    }

    fn response(decision: Decision, determining: Vec<String>) -> AuthorizationResponse {
        AuthorizationResponse {
            decision_id: "dec-01jx".to_owned(),
            request_id: "req-7f3a".to_owned(),
            decision,
            policy_version: PolicyVersion::new("psv-000042").unwrap(),
            determining_policy_ids: determining,
            obligations: vec![Obligation {
                obligation_id: "emit-audit-event".to_owned(),
                parameters: BTreeMap::new(),
            }],
        }
    }

    #[test]
    fn valid_request_passes_and_round_trips() {
        let r = request();
        r.validate().unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(
            serde_json::from_str::<AuthorizationRequest>(&json).unwrap(),
            r
        );
    }

    #[test]
    fn request_closed_schema_rejects_unknown_fields() {
        let mut value = serde_json::to_value(request()).unwrap();
        value["extra"] = serde_json::json!(1);
        assert!(serde_json::from_value::<AuthorizationRequest>(value).is_err());
    }

    #[test]
    fn malformed_entity_type_is_a_violation() {
        let mut r = request();
        r.principal.entity_type = "Oya Platform Principal".to_owned();
        let violations = r.validate().unwrap_err();
        assert!(matches!(
            violations.as_slice(),
            [ContractViolation::InvalidCharset { .. }]
        ));
    }

    #[test]
    fn allow_without_determining_policy_is_a_violation() {
        let violations = response(Decision::Allow, Vec::new())
            .validate()
            .unwrap_err();
        assert!(matches!(
            violations.as_slice(),
            [ContractViolation::InvalidShape { .. }]
        ));
    }

    #[test]
    fn default_deny_needs_no_determining_policy() {
        response(Decision::Deny, Vec::new()).validate().unwrap();
    }

    #[test]
    fn attributed_allow_passes() {
        response(Decision::Allow, vec!["rbac-tenant-admin-group".to_owned()])
            .validate()
            .unwrap();
    }

    #[test]
    fn policy_version_is_opaque_and_validated() {
        assert!(PolicyVersion::new("").is_err());
        assert!(PolicyVersion::new("has whitespace").is_err());
        let v = PolicyVersion::new("psv-000042").unwrap();
        assert_eq!(v.as_str(), "psv-000042");
        let echo = response(Decision::Deny, Vec::new());
        assert!(echo.satisfies_exact_version(&v));
        assert!(!echo.satisfies_exact_version(&PolicyVersion::new("psv-000043").unwrap()));
    }

    #[test]
    fn policy_version_serializes_transparently() {
        let v = PolicyVersion::new("psv-000042").unwrap();
        assert_eq!(serde_json::to_string(&v).unwrap(), "\"psv-000042\"");
    }
}
