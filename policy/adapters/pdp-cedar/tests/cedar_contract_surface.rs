#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod conformance;

use conformance::*;

#[test]
fn unknown_action_fails_closed() {
    let pdp = pdp(vec![]);
    let err = pdp
        .authorize(
            &request(
                "req-err-1",
                "acme",
                entity_ref("OyaPlatform::Principal", "alice"),
                "resource.purge",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap_err();
    assert!(matches!(err, PdpError::UnknownAction { .. }));
}

#[test]
fn obligations_ride_out_with_annotated_permits() {
    let mut bundle = locked_seed_bundle("psv-000001", vec![]);
    bundle.policies_src.push_str(
        "\n@id(\"workload-read-grant\")\n@obligation(\"emit-step-up-audit\")\npermit (\n  principal is OyaPlatform::WorkloadIdentity,\n  action == OyaPlatform::Action::\"ReadResource\",\n  resource\n)\nwhen { principal.tenant_id == resource.tenant_id };\n",
    );
    let pdp = CedarPdp::load(&bundle, Arc::new(SeededIdGenerator::default()), 64).unwrap();
    let outcome = pdp
        .authorize(
            &request(
                "req-obl-1",
                "acme",
                entity_ref("OyaPlatform::WorkloadIdentity", "payments"),
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
            .map(|o| o.obligation_id.as_str())
            .collect::<Vec<_>>(),
        vec!["emit-step-up-audit"]
    );
}

#[test]
fn qualification_evaluation_does_not_consume_the_serving_cache() {
    let pdp = CedarPdp::load(
        &locked_seed_bundle("psv-qualification-cache-read", vec![]),
        Arc::new(SeededIdGenerator::default()),
        64,
    )
    .unwrap();
    let request = request(
        "qualification-cache-read",
        "acme",
        entity_ref("OyaPlatform::Principal", "alice"),
        "resource.read",
        entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
    );

    assert!(!pdp.authorize(&request, &entity_slice()).unwrap().cache_hit);
    assert!(pdp.authorize(&request, &entity_slice()).unwrap().cache_hit);
    let qualified = pdp
        .authorize_for_qualification(&request, &entity_slice())
        .unwrap();
    assert!(!qualified.cache_hit);
    assert_eq!(qualified.response.decision, Decision::Allow);
}

#[test]
fn qualification_evaluation_does_not_populate_the_serving_cache() {
    let pdp = CedarPdp::load(
        &locked_seed_bundle("psv-qualification-cache-write", vec![]),
        Arc::new(SeededIdGenerator::default()),
        64,
    )
    .unwrap();
    let request = request(
        "qualification-cache-isolation",
        "acme",
        entity_ref("OyaPlatform::Principal", "alice"),
        "resource.read",
        entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
    );

    let qualified = pdp
        .authorize_for_qualification(&request, &entity_slice())
        .unwrap();
    assert!(!qualified.cache_hit);
    assert!(!pdp.authorize(&request, &entity_slice()).unwrap().cache_hit);
}

#[test]
fn crate_local_cedar_seeds_match_canonical() {
    const PAIRS: &[(&str, &str)] = &[
        (
            SCHEMA_SRC,
            "iam/core/platform-contracts-kernel/cedar/platform.cedarschema",
        ),
        (
            POLICIES_SRC,
            "iam/core/platform-contracts-kernel/cedar/platform-policies.cedar",
        ),
        (
            TEMPLATE_SRC,
            "iam/core/platform-contracts-kernel/cedar/platform-templates.cedar",
        ),
    ];
    let Some(root) = repo_root() else {
        eprintln!(
            "cedar_seed_parity: repo root marker not reachable (hermetic sandbox); \
             skipped {} pairs — cargo CI lane enforces parity",
            PAIRS.len()
        );
        return;
    };
    let mut mismatches = Vec::new();
    for (embedded, canonical) in PAIRS {
        let local_bytes = embedded.as_bytes().to_vec();
        let canonical_path = root.join(canonical);
        let canonical_bytes = std::fs::read(&canonical_path).unwrap_or_else(|e| {
            panic!(
                "canonical cedar seed missing: {}: {e}",
                canonical_path.display()
            )
        });
        if local_bytes != canonical_bytes {
            mismatches.push(format!("embedded seed != {canonical}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "crate-local cedar seed copies drifted from the canonical contract-lock \
         sources (canonical wins; sync the crate copy in the same change):\n  {}",
        mismatches.join("\n  ")
    );
}
