//! D7 — positive cases: same-tenant SelectSeat permits.
//!
//! These tests confirm the Cedar adapter does NOT default-deny EVERY request —
//! the same-tenant permit rule (`intelligence-app-ingress-chat-same-tenant`) must
//! fire when principal_tenant == resource_tenant and the principal is in the
//! IngressRealm (which v1 hardcodes for `AuthzAction::SelectSeat`).
use intelligence_authz_cedar_adapter::CedarAuthzGate;
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
fn cross_tenant_is_forbidden_for_every_action() {
    // The complement of the test above: no action escapes the tenant boundary.
    let g = gate();
    let pt = TenantId::new("acme").unwrap();
    let rt = TenantId::new("evil-corp").unwrap();
    let a = AgentId::new("admin:acme").unwrap();
    let mut leaked: Vec<&str> = Vec::new();
    for (label, action) in [
        ("RefreshToken", AuthzAction::RefreshToken),
        ("InvalidateSeat", AuthzAction::InvalidateSeat),
        ("SelectSeat", AuthzAction::SelectSeat),
    ] {
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
