use super::fixtures::*;

/// A legitimate acme overlay: grant bob ReadResource, tenant-confined by the
/// canonical same-tenant guard. Without it bob is deny-by-default (proved by
/// `abac_step_up_class_gates_restricted_reads`).
pub const ACME_OVERLAY_BOB_READ: &str = r#"
@id("ovl-bob-read")
permit (
  principal == OyaPlatform::Principal::"bob",
  action == OyaPlatform::Action::"ReadResource",
  resource
)
when { principal.tenant_id == resource.tenant_id };
"#;

pub fn pdp_with_overlays(tenant_policies: BTreeMap<String, String>) -> CedarPdp {
    CedarPdp::load(
        &locked_seed_bundle_with_overlays("psv-000001", vec![], tenant_policies),
        Arc::new(SeededIdGenerator::default()),
        64,
    )
    .expect("bundle with overlays must load")
}

/// Each carries the same-tenant equality as a NON-binding token (behind `||`,
/// in an `unless`, behind `!`), so a substring/EST-presence detector would
/// wrongly accept them all.
pub const EVASION_PERMITS: &[(&str, &str)] = &[
    (
        "evasion-or-true",
        r#"permit (principal, action == OyaPlatform::Action::"ReadResource", resource)
           when { principal.tenant_id == resource.tenant_id || true };"#,
    ),
    (
        "evasion-unless-true",
        r#"permit (principal, action == OyaPlatform::Action::"ReadResource", resource)
           when { principal.tenant_id == resource.tenant_id } unless { true };"#,
    ),
    (
        "evasion-guard-in-unless",
        r#"permit (principal, action == OyaPlatform::Action::"ReadResource", resource)
           unless { principal.tenant_id == resource.tenant_id };"#,
    ),
    (
        "evasion-negated",
        r#"permit (principal, action == OyaPlatform::Action::"ReadResource", resource)
           when { !(principal.tenant_id == resource.tenant_id) };"#,
    ),
    (
        "evasion-or-tautology",
        r#"permit (principal, action == OyaPlatform::Action::"ReadResource", resource)
           when { (principal.tenant_id == resource.tenant_id) || (1 == 1) };"#,
    ),
];

pub const LEGITIMATE_PERMITS: &[(&str, &str)] = &[
    (
        "ok-canonical",
        r#"permit (principal, action == OyaPlatform::Action::"ReadResource", resource)
           when { principal.tenant_id == resource.tenant_id };"#,
    ),
    (
        "ok-operand-swap",
        r#"permit (principal, action == OyaPlatform::Action::"ReadResource", resource)
           when { resource.tenant_id == principal.tenant_id };"#,
    ),
    (
        "ok-parenthesized",
        r#"permit (principal, action == OyaPlatform::Action::"ReadResource", resource)
           when { (principal.tenant_id == resource.tenant_id) };"#,
    ),
    (
        "ok-and-nested",
        r#"permit (principal, action == OyaPlatform::Action::"ReadResource", resource)
           when { resource.resource_kind == "document" && principal.tenant_id == resource.tenant_id };"#,
    ),
];

// option_env!, not env!: CARGO_MANIFEST_DIR is undefined at buck2 compile
// time (hermetic sandbox), and the buck2 lane must still COMPILE this target.
// The cargo lane enforces parity; buck2 skips with a notice.
pub fn manifest_dir() -> Option<&'static Path> {
    option_env!("CARGO_MANIFEST_DIR").map(Path::new)
}

pub fn repo_root() -> Option<PathBuf> {
    let mut dir = manifest_dir()?.to_path_buf();
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("docs/decisions").is_dir() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}
