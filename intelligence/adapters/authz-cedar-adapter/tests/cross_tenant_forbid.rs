//! Adversarial corpus for cross-tenant isolation: near-miss tenant ids that
//! must not be treated as a match.
//!
//! The rule under attack is `cloud-intelligence-forbid-cross-tenant-inference`
//! in `intelligence/cedar/cloud-intelligence.cedar`.
use intelligence_authz_cedar_adapter::CedarAuthzGate;
use intelligence_kernel::{
    AgentId, AuthzAction, AuthzDecision, AuthzGate, AuthzRequest, Provider, TenantId,
};

fn gate() -> CedarAuthzGate {
    CedarAuthzGate::with_default_policy().expect("bundled policy must parse")
}

fn req<'a>(
    principal_tenant: &'a TenantId,
    principal_agent: &'a AgentId,
    resource_tenant: &'a TenantId,
    provider: Provider,
) -> AuthzRequest<'a> {
    AuthzRequest {
        principal_tenant,
        principal_agent,
        action: AuthzAction::SelectSeat,
        resource_tenant,
        resource_provider: provider,
    }
}

#[test]
fn principal_tenant_a_vs_resource_tenant_b_is_forbidden() {
    let g = gate();
    let pt = TenantId::new("tenant-a").unwrap();
    let rt = TenantId::new("tenant-b").unwrap();
    let pa = AgentId::new("agent-1").unwrap();
    assert_eq!(
        g.decide(&req(&pt, &pa, &rt, Provider::Anthropic)),
        AuthzDecision::Forbid
    );
}

#[test]
fn principal_tenant_b_vs_resource_tenant_a_is_forbidden() {
    let g = gate();
    let pt = TenantId::new("tenant-b").unwrap();
    let rt = TenantId::new("tenant-a").unwrap();
    let pa = AgentId::new("agent-1").unwrap();
    assert_eq!(
        g.decide(&req(&pt, &pa, &rt, Provider::Anthropic)),
        AuthzDecision::Forbid
    );
}

#[test]
fn cross_tenant_remains_forbidden_for_codex_provider() {
    let g = gate();
    let pt = TenantId::new("acme").unwrap();
    let rt = TenantId::new("evil-corp").unwrap();
    let pa = AgentId::new("acme-agent-7").unwrap();
    assert_eq!(
        g.decide(&req(&pt, &pa, &rt, Provider::Codex)),
        AuthzDecision::Forbid
    );
}

#[test]
fn near_match_substring_does_not_grant_access() {
    // A substring match must not read as a tenant match.
    let g = gate();
    let pt = TenantId::new("acme").unwrap();
    let rt = TenantId::new("acme-prod").unwrap();
    let pa = AgentId::new("acme-agent-1").unwrap();
    assert_eq!(
        g.decide(&req(&pt, &pa, &rt, Provider::Anthropic)),
        AuthzDecision::Forbid
    );
}

#[test]
fn case_mismatch_blocks_access() {
    let g = gate();
    let pt = TenantId::new("ACME").unwrap();
    let rt = TenantId::new("acme").unwrap();
    let pa = AgentId::new("agent-1").unwrap();
    assert_eq!(
        g.decide(&req(&pt, &pa, &rt, Provider::Anthropic)),
        AuthzDecision::Forbid
    );
}

#[test]
fn whitespace_padded_principal_blocks_access() {
    let g = gate();
    let pt = TenantId::new("acme ").unwrap();
    let rt = TenantId::new("acme").unwrap();
    let pa = AgentId::new("agent-1").unwrap();
    assert_eq!(
        g.decide(&req(&pt, &pa, &rt, Provider::Anthropic)),
        AuthzDecision::Forbid
    );
}

#[test]
fn many_principal_tenants_versus_target_all_forbidden() {
    let g = gate();
    let rt = TenantId::new("target-tenant").unwrap();
    let pa = AgentId::new("attacker-1").unwrap();
    for foreign in &[
        "tenant-1",
        "tenant-2",
        "tenant-3",
        "victim-prefix-target-tenant",
        "target-tenantsuffix",
        "another-target-tenant",
        "TARGET-TENANT",
        "",
        " ",
        "0",
    ] {
        if let Ok(pt) = TenantId::new(*foreign) {
            assert_eq!(
                g.decide(&req(&pt, &pa, &rt, Provider::Anthropic)),
                AuthzDecision::Forbid,
                "principal tenant {foreign:?} should be forbidden",
            );
        }
    }
}

/// HAZARD, pinned rather than asserted-away: the cross-tenant forbid covers
/// only the two inference actions, so an admin action reaching a foreign
/// tenant is allowed here. A caller that must refuse it has to add that.
#[test]
fn cross_tenant_refresh_token_is_allowed_not_forbidden() {
    let g = gate();
    let pt = TenantId::new("tenant-a").unwrap();
    let rt = TenantId::new("tenant-b").unwrap();
    let pa = AgentId::new("agent-1").unwrap();
    let request = AuthzRequest {
        principal_tenant: &pt,
        principal_agent: &pa,
        action: AuthzAction::RefreshToken,
        resource_tenant: &rt,
        resource_provider: Provider::Anthropic,
    };
    assert_eq!(g.decide(&request), AuthzDecision::Allow);
}

#[test]
fn principal_with_tenant_id_resource_with_blank_tenant_is_forbidden() {
    let g = gate();
    let pt = TenantId::new("tenant-x").unwrap();
    let rt = TenantId::new(" ").unwrap_or_else(|_| TenantId::new("blank").unwrap());
    let pa = AgentId::new("agent-1").unwrap();
    assert_eq!(
        g.decide(&req(&pt, &pa, &rt, Provider::Anthropic)),
        AuthzDecision::Forbid
    );
}

#[test]
fn ten_random_cross_tenant_pairs_all_forbidden() {
    let g = gate();
    let pa = AgentId::new("agent-1").unwrap();
    let pairs: Vec<(&str, &str)> = vec![
        ("a", "b"),
        ("alpha", "beta"),
        ("acme", "gamma"),
        ("foo", "bar"),
        ("tenant-001", "tenant-002"),
        ("dev", "prod"),
        ("oyatie", "external"),
        ("kr-01", "us-01"),
        ("eu-west-1", "us-east-1"),
        ("redacted-a", "redacted-b"),
    ];
    for (a, b) in pairs {
        let pt = TenantId::new(a).unwrap();
        let rt = TenantId::new(b).unwrap();
        assert_eq!(
            g.decide(&req(&pt, &pa, &rt, Provider::Anthropic)),
            AuthzDecision::Forbid,
            "{a} vs {b} must be forbidden",
        );
    }
}
