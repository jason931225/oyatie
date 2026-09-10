#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod conformance;

use conformance::*;
use policy_pdp_cedar::validate_bundle;

fn load(bundle: &PolicyBundle) -> Result<CedarPdp, PdpError> {
    CedarPdp::load(bundle, Arc::new(SeededIdGenerator::default()), 4)
}

#[test]
fn action_mappings_are_admitted_against_the_schema() {
    for (slug, uid) in [
        ("", r#"OyaPlatform::Action::"ReadResource""#),
        ("  ", r#"OyaPlatform::Action::"ReadResource""#),
        ("resource.read", "not an entity uid"),
        ("resource.read", r#"OyaPlatform::Action::"Undeclared""#),
        ("resource.read", r#"OyaPlatform::Principal::"alice""#),
    ] {
        let mut bundle = locked_seed_bundle("legacy-v1", vec![]);
        bundle.action_map = BTreeMap::from([(slug.to_owned(), uid.to_owned())]);
        assert!(matches!(
            validate_bundle(&bundle),
            Err(PdpError::BundleRejected { .. })
        ));
        assert!(
            matches!(load(&bundle), Err(PdpError::BundleRejected { .. })),
            "unusable action mapping must be rejected at admission: {slug:?} -> {uid:?}"
        );
    }
}

#[test]
fn action_slugs_must_be_representable_by_authorization_requests() {
    for slug in [
        "Read".to_owned(),
        "_read".to_owned(),
        "read action".to_owned(),
        "read/child".to_owned(),
        "a".repeat(129),
    ] {
        let mut bundle = locked_seed_bundle("legacy-v1", vec![]);
        bundle.action_map = BTreeMap::from([(
            slug.clone(),
            r#"OyaPlatform::Action::"ReadResource""#.to_owned(),
        )]);
        assert!(
            matches!(
                validate_bundle(&bundle),
                Err(PdpError::BundleRejected { .. })
            ),
            "unrequestable action slug must be refused: {slug:?}"
        );
        assert!(matches!(
            load(&bundle),
            Err(PdpError::BundleRejected { .. })
        ));
    }
}

#[test]
fn maximum_length_action_slug_is_admitted_and_servable() {
    let slug = "a".repeat(128);
    let mut bundle = locked_seed_bundle("legacy-v1", vec![]);
    bundle.action_map = BTreeMap::from([(
        slug.clone(),
        r#"OyaPlatform::Action::"ReadResource""#.to_owned(),
    )]);
    validate_bundle(&bundle).unwrap();
    let pdp = load(&bundle).unwrap();
    let req = request(
        "maximum-action-length",
        "acme",
        entity_ref("OyaPlatform::Principal", "alice"),
        &slug,
        entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
    );
    req.validate().unwrap();
    assert_eq!(
        pdp.authorize(&req, &entity_slice())
            .unwrap()
            .response
            .decision,
        Decision::Allow
    );
}

#[test]
fn malformed_deserialized_version_is_rejected_at_admission() {
    let mut encoded = serde_json::to_value(locked_seed_bundle("legacy-v1", vec![])).unwrap();
    encoded["version"] = serde_json::json!("not a valid token");
    let bundle: PolicyBundle = serde_json::from_value(encoded).unwrap();
    assert!(matches!(
        validate_bundle(&bundle),
        Err(PdpError::BundleRejected { .. })
    ));
    assert!(matches!(
        load(&bundle),
        Err(PdpError::BundleRejected { .. })
    ));
}

#[test]
fn valid_opaque_version_and_declared_actions_remain_servable() {
    let bundle = locked_seed_bundle("legacy-v1", vec![]);
    validate_bundle(&bundle).unwrap();
    let pdp = load(&bundle).unwrap();
    let req = request(
        "admitted-decision",
        "acme",
        entity_ref("OyaPlatform::Principal", "alice"),
        "resource.read",
        entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
    );
    let outcome = pdp.authorize(&req, &entity_slice()).unwrap();
    assert_eq!(outcome.response.policy_version.as_str(), "legacy-v1");
    assert_eq!(outcome.response.decision, Decision::Allow);
}

#[test]
fn validation_and_load_share_schema_policy_and_link_refusals() {
    let mut bad_schema = locked_seed_bundle("legacy-v1", vec![]);
    bad_schema.schema_src = "invalid schema".to_owned();
    let mut bad_policy = locked_seed_bundle("legacy-v1", vec![]);
    bad_policy.policies_src = "invalid policy".to_owned();
    let mut bad_template = locked_seed_bundle("legacy-v1", vec![]);
    bad_template.templates[0].src = "invalid template".to_owned();
    let bad_link = locked_seed_bundle(
        "legacy-v1",
        vec![TemplateLink {
            template_id: "missing-template".to_owned(),
            link_id: "missing-link".to_owned(),
            principal: entity_ref("OyaPlatform::Principal", "alice"),
            resource: entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
        }],
    );
    for bundle in [bad_schema, bad_policy, bad_template, bad_link] {
        assert!(matches!(
            validate_bundle(&bundle),
            Err(PdpError::BundleRejected { .. })
        ));
        assert!(matches!(
            load(&bundle),
            Err(PdpError::BundleRejected { .. })
        ));
    }
}

#[test]
fn duplicate_authored_policy_id_is_rejected_at_admission() {
    let mut bundle = locked_seed_bundle("psv-000001", vec![]);
    bundle.policies_src.push_str(
        "\n@id(\"rbac-tenant-admin-group\")\npermit (\n  principal is OyaPlatform::Principal,\n  action == OyaPlatform::Action::\"ReadResource\",\n  resource\n)\nwhen { principal.tenant_id == resource.tenant_id };\n",
    );
    let Err(PdpError::BundleRejected { detail }) = validate_bundle(&bundle) else {
        panic!("a second policy re-using an authored @id must fail admission");
    };
    assert!(
        detail.contains("rbac-tenant-admin-group"),
        "the refusal must name the duplicated id, not some later schema \
         complaint that would make this test pass for the wrong reason: {detail}"
    );
    assert!(matches!(
        load(&bundle),
        Err(PdpError::BundleRejected { .. })
    ));
}
