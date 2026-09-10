use pipeline_admission::{
    ALLOWED_ROOT_DIRS, DATA_ROOTS, META_ROOTS, cap_root_file_ok, layout_violations,
};

#[test]
fn unknown_root_dir_is_red() {
    let violations = layout_violations(&["unlisted-root/leaf.rs".into()]);
    assert_eq!(
        violations,
        vec!["unlisted-root/leaf.rs: unknown root `unlisted-root`".to_owned()]
    );
}

#[test]
fn semantic_root_sets_are_subsets_of_allowed_roots() {
    for (kind, roots) in [("meta", META_ROOTS), ("data", DATA_ROOTS)] {
        for root in roots {
            assert!(
                ALLOWED_ROOT_DIRS.contains(root),
                "{kind} root `{root}` is missing from ALLOWED_ROOT_DIRS"
            );
        }
    }
}

#[test]
fn layout_engine_rejects_dump_and_frozen_markdown_but_accepts_native_faces() {
    let violations = layout_violations(&[
        "plan/foo.md".into(),
        "libs/x.rs".into(),
        "storage/src/lib.rs".into(),
        "storage/core/journal/src/lib.rs".into(),
        "app/foundry/ports/blob/src/lib.rs".into(),
        "docs/decisions/ADR-0720-example.md".into(),
        "docs/decisions/INDEX.md".into(),
    ]);
    assert!(violations.iter().any(|item| item.contains("plan")));
    assert!(violations.iter().any(|item| item.contains("libs")));
    assert!(violations.iter().any(|item| item.contains("storage/src")));
    assert!(!violations.iter().any(|item| item.contains("storage/core")));
    assert!(!violations.iter().any(|item| item.contains("foundry/ports")));
    assert!(
        !violations
            .iter()
            .any(|item| item.contains("ADR-0720-example"))
    );
    assert!(violations.iter().any(|item| {
        item.contains("docs/decisions/INDEX.md") && item.contains("frozen non-root Markdown")
    }));
}

#[test]
fn owner_law_files_are_frozen_non_authority() {
    for name in ["ADR.md", "PRD.md", "SPEC.md", "PLAN.md"] {
        assert!(!cap_root_file_ok(name), "{name}");
    }
    assert!(!cap_root_file_ok("ADR-2.md"));
    let violations = layout_violations(&[
        "network/ADR.md".into(),
        "app/foundry/PLAN.md".into(),
        "network/ADR-2.md".into(),
    ]);
    assert!(
        violations
            .iter()
            .any(|item| item.contains("network/ADR.md"))
    );
    assert!(
        violations
            .iter()
            .any(|item| item.contains("foundry/PLAN.md"))
    );
    assert!(violations.iter().any(|item| item.contains("ADR-2.md")));
}

#[test]
fn do_have_not_capability_roots_are_admitted_when_built() {
    let violations = layout_violations(&[
        "base/core/bytes/src/lib.rs".into(),
        "policy/core/evaluate/src/lib.rs".into(),
        "workflow/core/saga/src/lib.rs".into(),
        "notify/core/send/src/lib.rs".into(),
    ]);
    assert!(violations.is_empty(), "{violations:#?}");
}

#[test]
fn directory_names_cannot_be_added_as_files() {
    let violations = layout_violations(&[
        "policy".into(),
        "policy/core".into(),
        "app/new-product/core".into(),
    ]);
    assert_eq!(violations.len(), 3, "{violations:#?}");
}

#[test]
fn tests_live_in_the_crate_not_an_owner_tests_root() {
    let violations = layout_violations(&[
        "tests/foo.rs".into(),
        "e2e/foo.rs".into(),
        "network/tests/proxy.rs".into(),
        "app/foundry/tests/e2e.rs".into(),
        "network/facade/edge-app/tests/proxy.rs".into(),
        "network/facade/edge-app/tests/e2e/main.rs".into(),
        "iam/adapters/identity-scim-store-postgres/tests/live_rls.rs".into(),
    ]);
    assert!(violations.iter().any(|item| item.contains("tests/foo.rs")));
    assert!(violations.iter().any(|item| item.contains("e2e/foo.rs")));
    assert!(violations.iter().any(|item| item.contains("network/tests")));
    assert!(violations.iter().any(|item| item.contains("foundry/tests")));
    assert!(
        !violations
            .iter()
            .any(|item| item.contains("facade/edge-app/tests/proxy"))
    );
    assert!(
        !violations
            .iter()
            .any(|item| item.contains("tests/e2e/main.rs"))
    );
    assert!(!violations.iter().any(|item| item.contains("live_rls.rs")));
}

#[test]
fn iac_and_observability_are_capabilities_not_meta_roots() {
    let violations = layout_violations(&[
        "iac/src/lib.rs".into(),
        "iac/core/domain/src/lib.rs".into(),
        "observability/adapters/telemetry-tracing/src/lib.rs".into(),
        "docs/foo.md".into(),
    ]);
    assert!(violations.iter().any(|item| item.contains("iac/src")));
    assert!(!violations.iter().any(|item| item.contains("iac/core")));
    assert!(
        !violations
            .iter()
            .any(|item| item.contains("observability/adapters"))
    );
    assert!(violations.iter().any(|item| item.contains("docs/foo")));
}

#[test]
fn packs_are_closed_root_data_not_a_capability() {
    assert!(!pipeline_admission::is_capability_root("packs"));
    let violations = layout_violations(&[
        "packs/eu/policy/gdpr.cedar".into(),
        "packs/kr/placement/data_residency.textproto".into(),
        "packs/kr-eu/policy/combined.cedar".into(),
        "packs/eu/core/evaluator/src/lib.rs".into(),
    ]);
    assert!(!violations.iter().any(|item| item.contains("gdpr.cedar")));
    assert!(
        !violations
            .iter()
            .any(|item| item.contains("data_residency.textproto"))
    );
    assert!(violations.iter().any(|item| item.contains("kr-eu")));
    assert!(violations.iter().any(|item| item.contains("packs require")));
}
