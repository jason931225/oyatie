//! D7 — positive cases: the Cedar adapter does NOT default-deny every request.
//!
//! `cloud-intelligence-ingress-chat-same-tenant` must fire when the tenants match
//! and the principal is in the IngressRealm. The tail of the file pins all six
//! policy actions in both directions against the widened cross-tenant forbid.
use std::collections::{BTreeSet, HashMap, HashSet};

use cedar_policy::{
    Authorizer, Context, Decision, Entities, Entity, EntityUid, PolicySet, Request,
    RestrictedExpression,
};
use intelligence_authz_cedar_adapter::{CedarAuthzGate, DEFAULT_POLICY_TEXT};
use intelligence_kernel::{
    AgentId, AuthzAction, AuthzDecision, AuthzGate, AuthzRequest, Provider, TenantId,
};

fn gate() -> CedarAuthzGate {
    CedarAuthzGate::with_default_policy().expect("bundled policy must parse")
}

#[test]
fn same_tenant_select_seat_anthropic_is_allowed() {
    let g = gate();
    let t = TenantId::new("acme").unwrap();
    let a = AgentId::new("acme-agent-1").unwrap();
    let r = AuthzRequest {
        principal_tenant: &t,
        principal_agent: &a,
        action: AuthzAction::SelectSeat,
        resource_tenant: &t,
        resource_provider: Provider::Anthropic,
    };
    assert_eq!(g.decide(&r), AuthzDecision::Allow);
}

#[test]
fn same_tenant_select_seat_codex_is_allowed() {
    let g = gate();
    let t = TenantId::new("acme").unwrap();
    let a = AgentId::new("acme-agent-2").unwrap();
    let r = AuthzRequest {
        principal_tenant: &t,
        principal_agent: &a,
        action: AuthzAction::SelectSeat,
        resource_tenant: &t,
        resource_provider: Provider::Codex,
    };
    assert_eq!(g.decide(&r), AuthzDecision::Allow);
}

#[test]
fn different_tenants_with_identical_string_are_allowed() {
    // Sanity check on the byte-equality used by Cedar's `==`.
    let g = gate();
    let t1 = TenantId::new("oyatie").unwrap();
    let t2 = TenantId::new("oyatie").unwrap();
    let a = AgentId::new("oyatie-agent-1").unwrap();
    let r = AuthzRequest {
        principal_tenant: &t1,
        principal_agent: &a,
        action: AuthzAction::SelectSeat,
        resource_tenant: &t2,
        resource_provider: Provider::Anthropic,
    };
    assert_eq!(g.decide(&r), AuthzDecision::Allow);
}

#[test]
fn many_distinct_same_tenant_principals_all_allowed() {
    let g = gate();
    for slug in &[
        "t1",
        "t2",
        "tenant-customer-001",
        "kr-team",
        "eu-team",
        "internal-dogfood",
    ] {
        let t = TenantId::new(*slug).unwrap();
        let a = AgentId::new(format!("{slug}-agent")).unwrap();
        let r = AuthzRequest {
            principal_tenant: &t,
            principal_agent: &a,
            action: AuthzAction::SelectSeat,
            resource_tenant: &t,
            resource_provider: Provider::Anthropic,
        };
        assert_eq!(
            g.decide(&r),
            AuthzDecision::Allow,
            "tenant {slug} must be allowed"
        );
    }
}

#[test]
fn agent_id_does_not_affect_decision_when_tenants_match() {
    // Cedar policy does not currently constrain by agent id — that's the REST
    // adapter's authentication step. Different agents within the same tenant
    // all pass.
    let g = gate();
    let t = TenantId::new("acme").unwrap();
    for agent in &["a", "b", "c", "d", "e", "f"] {
        let a = AgentId::new(*agent).unwrap();
        let r = AuthzRequest {
            principal_tenant: &t,
            principal_agent: &a,
            action: AuthzAction::SelectSeat,
            resource_tenant: &t,
            resource_provider: Provider::Anthropic,
        };
        assert_eq!(
            g.decide(&r),
            AuthzDecision::Allow,
            "agent {agent} must be allowed"
        );
    }
}

#[test]
fn same_tenant_admin_actions_still_allowed_after_cross_tenant_widening() {
    // The cross-tenant forbid covers every action, not just the two inference
    // ones. A forbid that is too broad is its own outage, so pin the legitimate
    // side: an admin acting on its OWN tenant must still pass.
    let g = gate();
    let t = TenantId::new("acme").unwrap();
    let a = AgentId::new("admin:acme").unwrap();
    let mut denied: Vec<&str> = Vec::new();
    for (label, action) in [
        ("RefreshToken", AuthzAction::RefreshToken),
        ("InvalidateSeat", AuthzAction::InvalidateSeat),
        ("SelectSeat", AuthzAction::SelectSeat),
    ] {
        let r = AuthzRequest {
            principal_tenant: &t,
            principal_agent: &a,
            action,
            resource_tenant: &t,
            resource_provider: Provider::Anthropic,
        };
        if g.decide(&r) != AuthzDecision::Allow {
            denied.push(label);
        }
    }
    assert!(
        denied.is_empty(),
        "same-tenant actions wrongly denied by the widened forbid: {denied:?}",
    );
}

#[test]
fn cross_tenant_is_forbidden_for_every_kernel_action() {
    // The complement of the test above, over the actions `decide()` can reach.
    // The `match` is the fence: a fourth `AuthzAction` variant stops compiling
    // here instead of escaping, which the array literal alone would allow.
    // ponytail: the fence forces an edit, it does not derive membership.
    let g = gate();
    let pt = TenantId::new("acme").unwrap();
    let rt = TenantId::new("evil-corp").unwrap();
    let a = AgentId::new("admin:acme").unwrap();
    let mut leaked: Vec<&str> = Vec::new();
    for action in [
        AuthzAction::RefreshToken,
        AuthzAction::InvalidateSeat,
        AuthzAction::SelectSeat,
    ] {
        let label = match action {
            AuthzAction::RefreshToken => "RefreshToken",
            AuthzAction::InvalidateSeat => "InvalidateSeat",
            AuthzAction::SelectSeat => "SelectSeat",
        };
        let r = AuthzRequest {
            principal_tenant: &pt,
            principal_agent: &a,
            action,
            resource_tenant: &rt,
            resource_provider: Provider::Anthropic,
        };
        if g.decide(&r) != AuthzDecision::Forbid {
            leaked.push(label);
        }
    }
    assert!(
        leaked.is_empty(),
        "these actions crossed the tenant boundary: {leaked:?}",
    );
}

/// Every action the bundled policy names, with the realm that permits it.
/// `decide()` reaches only `InvokeChatCompletion` and `RefreshKeyPool`, so the other four are
/// driven at Cedar here. Re-scoping is an independent property: the widening newly denies
/// ListModels, ReadPoolStatus, RefreshKeyPool and ReadAudit — so RefreshKeyPool is in both.
const REALM_BY_ACTION: [(&str, &str); 6] = [
    ("InvokeChatCompletion", "IngressRealm"),
    ("InvokeEmbeddings", "IngressRealm"),
    ("ListModels", "IngressRealm"),
    ("ReadPoolStatus", "AdminRealm"),
    ("RefreshKeyPool", "AdminRealm"),
    ("ReadAudit", "AuditReader"),
];

/// Action names as they appear in the policy text, independent of the table.
fn actions_named_by_policy() -> BTreeSet<String> {
    DEFAULT_POLICY_TEXT
        .split("Action::\"")
        .skip(1)
        .filter_map(|rest| rest.split_once('"'))
        .map(|(name, _)| name.to_owned())
        .collect()
}

#[test]
fn realm_table_covers_exactly_the_policy_actions() {
    // Two independent sources: the table above and the policy text. A seventh
    // or renamed action fails here rather than slipping past the pin below.
    let tabled: BTreeSet<String> = REALM_BY_ACTION
        .iter()
        .map(|(action, _)| (*action).to_owned())
        .collect();
    assert_eq!(actions_named_by_policy(), tabled);
}

const PROBE_PRINCIPAL: &str = r#"Workload::"probe-agent""#;
const PROBE_RESOURCE: &str = r#"Subscription::"probe-resource""#;

fn uid(literal: &str) -> EntityUid {
    literal.parse().unwrap()
}

fn tenant_entity(literal: &str, tenant: &str, parents: HashSet<EntityUid>) -> Entity {
    let attrs = HashMap::from([(
        "tenant_id".to_owned(),
        RestrictedExpression::new_string(tenant.to_owned()),
    )]);
    Entity::new(uid(literal), attrs, parents).unwrap()
}

fn cedar_allows(action: &str, realm: &str, principal_tenant: &str, resource_tenant: &str) -> bool {
    let policy_set: PolicySet = DEFAULT_POLICY_TEXT.parse().expect("bundled policy parses");
    let role = uid(&format!(r#"Role::"{realm}""#));
    let entities = Entities::empty()
        .add_entities(
            [
                tenant_entity(
                    PROBE_PRINCIPAL,
                    principal_tenant,
                    HashSet::from([role.clone()]),
                ),
                tenant_entity(PROBE_RESOURCE, resource_tenant, HashSet::new()),
                Entity::new(role, HashMap::new(), HashSet::new()).unwrap(),
            ],
            None,
        )
        .unwrap();
    let request = Request::new(
        uid(PROBE_PRINCIPAL),
        uid(&format!(r#"Action::"{action}""#)),
        uid(PROBE_RESOURCE),
        Context::empty(),
        None,
    )
    .unwrap();
    Authorizer::new()
        .is_authorized(&request, &policy_set, &entities)
        .decision()
        == Decision::Allow
}

#[test]
fn every_policy_action_is_denied_across_tenants() {
    // Includes ReadAudit: a SIEM reader holding AuditReader and a foreign tenant
    // is denied. That is the deliberate re-scoping, not an accident of the
    // widening, and this is where it is pinned.
    let mut leaked: Vec<&str> = Vec::new();
    for (action, realm) in REALM_BY_ACTION {
        if cedar_allows(action, realm, "acme", "evil-corp") {
            leaked.push(action);
        }
    }
    assert!(
        leaked.is_empty(),
        "these actions crossed the tenant boundary: {leaked:?}",
    );
}

#[test]
fn every_policy_action_is_allowed_within_one_tenant() {
    // The other direction: without it a forbid broad enough to deny everything
    // satisfies the test above, and an outage reads as a pass.
    let mut denied: Vec<&str> = Vec::new();
    for (action, realm) in REALM_BY_ACTION {
        if !cedar_allows(action, realm, "acme", "acme") {
            denied.push(action);
        }
    }
    assert!(
        denied.is_empty(),
        "same-tenant actions wrongly denied: {denied:?}",
    );
}
