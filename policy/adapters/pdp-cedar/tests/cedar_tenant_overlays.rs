#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod conformance;

use conformance::*;

#[test]
fn tenant_overlay_permit_applies_only_within_owning_tenant() {
    let pdp = pdp_with_overlays(BTreeMap::from([(
        "acme".to_owned(),
        ACME_OVERLAY_BOB_READ.to_owned(),
    )]));
    let outcome = pdp
        .authorize(
            &request(
                "req-ovl-1",
                "acme",
                entity_ref("OyaPlatform::Principal", "bob"),
                "resource.read",
                non_restricted_acme_doc(),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(outcome.response.decision, Decision::Allow);
    assert_eq!(
        outcome.response.determining_policy_ids,
        vec!["acme/ovl-bob-read".to_owned()],
        "the allow is attributable to the tenant-namespaced overlay policy"
    );
}

#[test]
fn tenant_overlay_does_not_leak_to_other_tenant() {
    let pdp = pdp_with_overlays(BTreeMap::from([(
        "acme".to_owned(),
        ACME_OVERLAY_BOB_READ.to_owned(),
    )]));
    let outcome = pdp
        .authorize(
            &request(
                "req-ovl-2",
                "globex",
                entity_ref("OyaPlatform::Principal", "bob"),
                "resource.read",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(
        outcome.response.decision,
        Decision::Deny,
        "globex must never be evaluated against acme's overlay"
    );
}

#[test]
fn fail_closed_default_deny_with_empty_overlays() {
    let pdp = pdp_with_overlays(BTreeMap::new());
    let outcome = pdp
        .authorize(
            &request(
                "req-ovl-3",
                "acme",
                entity_ref("OyaPlatform::Principal", "bob"),
                "resource.read",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(outcome.response.decision, Decision::Deny);
}

#[test]
fn malformed_tenant_overlay_rejects_whole_bundle_fail_closed() {
    let result = CedarPdp::load(
        &locked_seed_bundle_with_overlays(
            "psv-000001",
            vec![],
            BTreeMap::from([("acme".to_owned(), "permit (this is not cedar".to_owned())]),
        ),
        Arc::new(SeededIdGenerator::default()),
        64,
    );
    assert!(
        matches!(result, Err(PdpError::BundleRejected { .. })),
        "a malformed overlay must fail closed, got {:?}",
        result.map(|_| "loaded")
    );
}

#[test]
fn tenant_overlay_authoring_cross_tenant_permit_is_rejected_at_load() {
    // An acme overlay permit WITHOUT the same-tenant guard could grant across a
    // tenant boundary — reject at LOAD so isolation is structural, not emergent.
    let cross_tenant_permit = r#"
@id("ovl-cross")
permit (
  principal is OyaPlatform::Principal,
  action == OyaPlatform::Action::"ReadResource",
  resource
);
"#;
    let result = CedarPdp::load(
        &locked_seed_bundle_with_overlays(
            "psv-000001",
            vec![],
            BTreeMap::from([("acme".to_owned(), cross_tenant_permit.to_owned())]),
        ),
        Arc::new(SeededIdGenerator::default()),
        64,
    );
    assert!(
        matches!(result, Err(PdpError::BundleRejected { .. })),
        "an overlay permit that is not tenant-confined must fail closed at load, got {:?}",
        result.map(|_| "loaded")
    );

    let foreign_literal_permit = r#"
@id("ovl-foreign")
permit (
  principal is OyaPlatform::Principal,
  action == OyaPlatform::Action::"ReadResource",
  resource
)
when { resource.tenant_id == "globex" };
"#;
    let result = CedarPdp::load(
        &locked_seed_bundle_with_overlays(
            "psv-000001",
            vec![],
            BTreeMap::from([
                ("acme".to_owned(), foreign_literal_permit.to_owned()),
                ("globex".to_owned(), String::new()),
            ]),
        ),
        Arc::new(SeededIdGenerator::default()),
        64,
    );
    assert!(
        matches!(result, Err(PdpError::BundleRejected { .. })),
        "an overlay naming another known tenant must fail closed at load, got {:?}",
        result.map(|_| "loaded")
    );
}

#[test]
fn tenant_overlay_cannot_escape_structural_forbid() {
    // Even a same-tenant-guarded overlay grant cannot defeat the structural
    // forbid: mallory is a globex principal mis-joined to the acme group; an
    // acme overlay permitting mallory must still be denied cross-tenant.
    let overlay = r#"
@id("ovl-mallory")
permit (
  principal == OyaPlatform::Principal::"mallory",
  action == OyaPlatform::Action::"ReadResource",
  resource
)
when { principal.tenant_id == resource.tenant_id };
"#;
    let pdp = pdp_with_overlays(BTreeMap::from([("globex".to_owned(), overlay.to_owned())]));
    let outcome = pdp
        .authorize(
            &request(
                "req-ovl-forbid",
                "globex",
                entity_ref("OyaPlatform::Principal", "mallory"),
                "resource.read",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(
        outcome.response.decision,
        Decision::Deny,
        "the structural forbid overrides every overlay permit across a tenant boundary"
    );
}

#[test]
fn overlay_permit_obligation_rides_out_from_the_merged_set() {
    let overlay = r#"
@id("ovl-bob-read-with-obligation")
@obligation("redact-pii")
permit (
  principal == OyaPlatform::Principal::"bob",
  action == OyaPlatform::Action::"ReadResource",
  resource
)
when { principal.tenant_id == resource.tenant_id };
"#;
    let pdp = pdp_with_overlays(BTreeMap::from([("acme".to_owned(), overlay.to_owned())]));
    let outcome = pdp
        .authorize(
            &request(
                "req-ovl-obligation",
                "acme",
                entity_ref("OyaPlatform::Principal", "bob"),
                "resource.read",
                non_restricted_acme_doc(),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(outcome.response.decision, Decision::Allow);
    assert_eq!(
        outcome
            .response
            .obligations
            .iter()
            .map(|obligation| obligation.obligation_id.as_str())
            .collect::<Vec<_>>(),
        vec!["redact-pii"],
        "an overlay permit's obligation must be looked up against the merged \
         set that decided the request, not the global set"
    );
}
