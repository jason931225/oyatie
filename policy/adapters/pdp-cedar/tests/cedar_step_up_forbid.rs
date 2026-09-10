#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod conformance;

use conformance::*;

#[test]
fn tenant_overlay_permit_cannot_bypass_step_up_forbid() {
    // `forbid-restricted-read-without-step-up` must stay non-bypassable: even a
    // perfectly tenant-confined overlay permit cannot grant bob (NO
    // step_up_class) a read of the RESTRICTED acme-doc-1. Forbid wins.
    let overlay = r#"
@id("ovl-bob-restricted")
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
                "req-stepup-bypass",
                "acme",
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
        "an overlay permit must not bypass the step-up forbid on restricted reads"
    );
    // The SAME overlay must still grant something, or the deny above proves
    // nothing about the gate.
    let allowed = pdp
        .authorize(
            &request(
                "req-stepup-ok",
                "acme",
                entity_ref("OyaPlatform::Principal", "bob"),
                "resource.read",
                non_restricted_acme_doc(),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(
        allowed.response.decision,
        Decision::Allow,
        "the overlay still grants an ordinary (non-restricted) within-tenant read"
    );
}

#[test]
fn step_up_forbid_still_allows_a_stepped_up_restricted_read() {
    let pdp = pdp(vec![]);
    let outcome = pdp
        .authorize(
            &request(
                "req-stepup-allow",
                "acme",
                entity_ref("OyaPlatform::Principal", "alice"),
                "resource.read",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(outcome.response.decision, Decision::Allow);
}

#[test]
fn step_up_forbid_blocks_a_pbac_link_to_a_restricted_read() {
    // The one path no other test in this file covers: an EXPLICIT template link
    // granting bob (NO step_up_class) a read of the RESTRICTED acme-doc-1 still
    // loses to `forbid-restricted-read-without-step-up`.
    let link = TemplateLink {
        template_id: TEMPLATE_ID.to_owned(),
        link_id: "pbac-link-bob-restricted".to_owned(),
        principal: entity_ref("OyaPlatform::Principal", "bob"),
        resource: entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
    };
    let pdp_restricted = pdp(vec![link]);
    let denied = pdp_restricted
        .authorize(
            &request(
                "req-pbac-restricted-deny",
                "acme",
                entity_ref("OyaPlatform::Principal", "bob"),
                "resource.read",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(
        denied.response.decision,
        Decision::Deny,
        "a PBAC template-link must not bypass the step-up forbid on restricted reads"
    );

    // The SAME link must still grant something, or the deny above proves the
    // link never worked rather than that the forbid beat it.
    let link_ok = TemplateLink {
        template_id: TEMPLATE_ID.to_owned(),
        link_id: "pbac-link-bob-restricted".to_owned(),
        principal: entity_ref("OyaPlatform::Principal", "bob"),
        resource: non_restricted_acme_doc(),
    };
    let pdp_ok = pdp(vec![link_ok]);
    let allowed = pdp_ok
        .authorize(
            &request(
                "req-pbac-restricted-ok",
                "acme",
                entity_ref("OyaPlatform::Principal", "bob"),
                "resource.read",
                non_restricted_acme_doc(),
            ),
            &entity_slice(),
        )
        .unwrap();
    assert_eq!(
        allowed.response.decision,
        Decision::Allow,
        "the link still grants an ordinary (non-restricted) within-tenant read"
    );
}
