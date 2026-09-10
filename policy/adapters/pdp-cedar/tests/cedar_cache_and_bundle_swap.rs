#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod conformance;

use conformance::*;

#[test]
fn stale_policy_version_pin_is_refused_not_answered() {
    let pdp = pdp(vec![]);
    let mut req = request(
        "req-zookie-1",
        "acme",
        entity_ref("OyaPlatform::Principal", "alice"),
        "resource.read",
        entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
    );
    req.min_policy_version = Some(PolicyVersion::new("psv-000002").unwrap());
    let err = pdp.authorize(&req, &entity_slice()).unwrap_err();
    assert!(
        matches!(err, PdpError::StalePolicyVersion { .. }),
        "expected stale-version refusal, got {err:?}"
    );
    // Pinning the LOADED version succeeds (read-your-writes satisfied).
    req.min_policy_version = Some(pdp.loaded_policy_version());
    assert!(pdp.authorize(&req, &entity_slice()).is_ok());
}

#[test]
fn cache_replays_decision_content_with_fresh_decision_ids() {
    let pdp = pdp(vec![]);
    let req = request(
        "req-cache-1",
        "acme",
        entity_ref("OyaPlatform::Principal", "alice"),
        "resource.read",
        entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
    );
    let first = pdp.authorize(&req, &entity_slice()).unwrap();
    assert!(!first.cache_hit);
    let mut replay = req.clone();
    replay.request_id = "req-cache-2".to_owned();
    let second = pdp.authorize(&replay, &entity_slice()).unwrap();
    assert!(second.cache_hit, "identical decision surface must hit");
    assert_eq!(second.response.decision, first.response.decision);
    assert_eq!(
        second.response.determining_policy_ids,
        first.response.determining_policy_ids
    );
    assert_ne!(
        second.response.decision_id, first.response.decision_id,
        "every decision — cached or not — gets its own audit-chain key"
    );
    assert_eq!(second.response.request_id, "req-cache-2");
}

#[test]
fn bundle_swap_revokes_immediately_and_disarms_prior_cache() {
    let link = TemplateLink {
        template_id: TEMPLATE_ID.to_owned(),
        link_id: "pbac-link-bob-doc2".to_owned(),
        principal: entity_ref("OyaPlatform::Principal", "bob"),
        resource: non_restricted_acme_doc(),
    };
    let pdp = pdp(vec![link]);
    let req = request(
        "req-revoke-1",
        "acme",
        entity_ref("OyaPlatform::Principal", "bob"),
        "resource.read",
        non_restricted_acme_doc(),
    );
    let before = pdp.authorize(&req, &entity_slice()).unwrap();
    assert_eq!(before.response.decision, Decision::Allow);
    // Revocation: the policy store pushes a bundle WITHOUT the grant. The swap
    // is atomic; the version change makes every cached allow unreachable.
    pdp.swap_bundle(&locked_seed_bundle("psv-000002", vec![]))
        .unwrap();
    let mut after_req = req.clone();
    after_req.request_id = "req-revoke-2".to_owned();
    let after = pdp.authorize(&after_req, &entity_slice()).unwrap();
    assert_eq!(after.response.decision, Decision::Deny);
    assert!(!after.cache_hit, "prior cache entries must be unreachable");
    assert_eq!(after.response.policy_version.as_str(), "psv-000002");
}

#[test]
fn rejected_bundle_never_replaces_a_serving_one() {
    let pdp = pdp(vec![]);
    let mut broken = locked_seed_bundle("psv-000099", vec![]);
    broken.policies_src =
        "permit (principal, action, resource) when { principal.nonexistent_attr == \"x\" };"
            .to_owned();
    let err = pdp.swap_bundle(&broken).unwrap_err();
    assert!(matches!(err, PdpError::BundleRejected { .. }));
    assert_eq!(
        pdp.loaded_policy_version().as_str(),
        "psv-000001",
        "the serving bundle must keep serving after a rejected swap"
    );
}

fn reload_request() -> AuthorizationRequest {
    request(
        "reload-decision",
        "acme",
        entity_ref("OyaPlatform::Principal", "alice"),
        "resource.read",
        entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
    )
}

#[test]
fn identical_current_bundle_reload_preserves_the_warm_cache() {
    let bundle = locked_seed_bundle("psv-000001", vec![]);
    let pdp = pdp(vec![]);
    let request = reload_request();
    let before = pdp.authorize(&request, &entity_slice()).unwrap();
    assert!(!before.cache_hit);
    pdp.swap_bundle(&bundle).unwrap();
    let after = pdp.authorize(&request, &entity_slice()).unwrap();
    assert!(after.cache_hit);
    assert_eq!(
        after.response.policy_version,
        before.response.policy_version
    );
    assert_eq!(after.response.decision, before.response.decision);
}

#[test]
fn current_version_cannot_be_redefined_by_a_reload() {
    let pdp = pdp(vec![]);
    let mut redefined = locked_seed_bundle("psv-000001", vec![]);
    redefined.policies_src.push('\n');
    assert!(matches!(
        pdp.swap_bundle(&redefined),
        Err(PdpError::BundleRejected { .. })
    ));
    assert_eq!(pdp.loaded_policy_version().as_str(), "psv-000001");
}

#[test]
fn replacement_and_rollback_each_start_with_a_cold_cache() {
    let original = locked_seed_bundle("psv-000001", vec![]);
    let pdp = pdp(vec![]);
    let request = reload_request();
    assert!(!pdp.authorize(&request, &entity_slice()).unwrap().cache_hit);
    assert!(pdp.authorize(&request, &entity_slice()).unwrap().cache_hit);
    for bundle in [locked_seed_bundle("psv-000002", vec![]), original] {
        pdp.swap_bundle(&bundle).unwrap();
        let cold = pdp.authorize(&request, &entity_slice()).unwrap();
        assert_eq!(cold.response.policy_version, bundle.version);
        assert!(!cold.cache_hit);
        assert!(pdp.authorize(&request, &entity_slice()).unwrap().cache_hit);
    }
}

#[test]
fn invalid_action_reload_preserves_serving_state_and_cache() {
    let pdp = pdp(vec![]);
    let request = reload_request();
    let before = pdp.authorize(&request, &entity_slice()).unwrap();
    let mut broken = locked_seed_bundle("psv-000002", vec![]);
    broken.action_map.insert(
        "resource.read".to_owned(),
        r#"OyaPlatform::Action::"Undeclared""#.to_owned(),
    );
    assert!(matches!(
        pdp.swap_bundle(&broken),
        Err(PdpError::BundleRejected { .. })
    ));
    let after = pdp.authorize(&request, &entity_slice()).unwrap();
    assert!(after.cache_hit);
    assert_eq!(
        after.response.policy_version,
        before.response.policy_version
    );
    assert_eq!(after.response.decision, before.response.decision);
}

#[test]
fn disabled_cache_stays_disabled_after_replacement() {
    let bundle = locked_seed_bundle("psv-000001", vec![]);
    let pdp = CedarPdp::load(&bundle, Arc::new(SeededIdGenerator::default()), 0).unwrap();
    pdp.swap_bundle(&locked_seed_bundle("psv-000002", vec![]))
        .unwrap();
    let request = reload_request();
    assert!(!pdp.authorize(&request, &entity_slice()).unwrap().cache_hit);
    assert!(!pdp.authorize(&request, &entity_slice()).unwrap().cache_hit);
}

#[test]
fn every_decision_yields_an_attributable_audit_record() {
    let pdp = pdp(vec![]);
    let allow = pdp
        .authorize(
            &request(
                "req-audit-1",
                "acme",
                entity_ref("OyaPlatform::Principal", "alice"),
                "resource.read",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(allow.audit.decision_id, allow.response.decision_id);
    assert_eq!(allow.audit.decision, Decision::Allow);
    assert_eq!(allow.audit.tenant_id, "acme");
    assert_eq!(allow.audit.policy_version.as_str(), "psv-000001");
    assert!(!allow.audit.determining_policy_ids.is_empty());

    let deny = pdp
        .authorize(
            &request(
                "req-audit-2",
                "acme",
                entity_ref("OyaPlatform::Principal", "bob"),
                "resource.write",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(deny.audit.decision, Decision::Deny);
    assert_ne!(deny.audit.decision_id, allow.audit.decision_id);
}
