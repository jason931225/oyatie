pub use audit_chain_domain::{
    AuditAppendInput, AuditChain, AuditChainError, Ed25519SigningKey, Ed25519VerificationKeySet,
    Plane, append as audit_append,
};
pub use audit_file_adapter::FileAuditLedger;
pub use data_boundary_kernel::{DataClass, Purpose};
pub use policy_pdp_cedar::{
    AuditChainCedarPdp, CedarPdp, PDP_DECISION_AUDIT_SURFACE, PdpAuditChainError,
    PdpDecisionAuditChainLogger,
};
pub use shared_pdp_kernel::{
    EntityRecord, EntitySlice, PdpError, PolicyBundle, PolicyDecisionPoint, TemplateLink,
    TemplateSrc,
};
pub use shared_platform_contracts_kernel::pdp::{
    AuthorizationRequest, Decision, EntityRef, PolicyVersion,
};
pub use shared_ulid_id_kernel::SeededIdGenerator;

pub use super::seeds::{POLICIES_SRC, SCHEMA_SRC, TEMPLATE_SRC};
pub use std::collections::BTreeMap;
pub use std::path::{Path, PathBuf};
pub use std::sync::Arc;
pub use std::time::{SystemTime, UNIX_EPOCH};

pub const TEMPLATE_ID: &str = "pbac-resource-read-grant";

/// Targeted by every ordinary within-tenant read grant (PBAC links, tenant
/// overlays, workload permits) so they exercise their real intent without
/// colliding with `forbid-restricted-read-without-step-up`, which only fires on
/// `data_class == "restricted"`.
pub fn non_restricted_acme_doc() -> EntityRef {
    entity_ref("OyaPlatform::TenantResource", "acme-doc-2")
}

/// Lets a cross-tenant read (acme principal -> globex resource) be exercised so
/// the structural forbid can be proved as the runtime boundary.
pub fn foreign_tenant_doc() -> EntityRef {
    entity_ref("OyaPlatform::TenantResource", "globex-doc-1")
}

pub fn entity_ref(entity_type: &str, entity_id: &str) -> EntityRef {
    EntityRef {
        entity_type: entity_type.to_owned(),
        entity_id: entity_id.to_owned(),
    }
}

pub fn action_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "resource.read".to_owned(),
            r#"OyaPlatform::Action::"ReadResource""#.to_owned(),
        ),
        (
            "resource.write".to_owned(),
            r#"OyaPlatform::Action::"WriteResource""#.to_owned(),
        ),
        (
            "tenant.administer".to_owned(),
            r#"OyaPlatform::Action::"AdministerTenant""#.to_owned(),
        ),
    ])
}

pub fn locked_seed_bundle(version: &str, template_links: Vec<TemplateLink>) -> PolicyBundle {
    locked_seed_bundle_with_overlays(version, template_links, BTreeMap::new())
}

pub fn locked_seed_bundle_with_overlays(
    version: &str,
    template_links: Vec<TemplateLink>,
    tenant_policies: BTreeMap<String, String>,
) -> PolicyBundle {
    PolicyBundle {
        version: PolicyVersion::new(version).unwrap(),
        schema_src: SCHEMA_SRC.to_owned(),
        policies_src: POLICIES_SRC.to_owned(),
        tenant_policies,
        templates: vec![TemplateSrc {
            template_id: TEMPLATE_ID.to_owned(),
            src: TEMPLATE_SRC.to_owned(),
        }],
        template_links,
        action_map: action_map(),
    }
}

/// Two tenants in two cells; one cross-tenant-polluted admin group — the
/// exact misconfiguration the structural forbid must neutralize (mirrors the
/// locked contract-kernel validation fixture).
pub fn entity_slice() -> EntitySlice {
    let string_attrs = |pairs: &[(&str, &str)]| -> BTreeMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), serde_json::json!(v)))
            .collect()
    };
    EntitySlice {
        entities: vec![
            EntityRecord {
                uid: entity_ref("OyaPlatform::Tenant", "acme"),
                attributes: string_attrs(&[
                    ("tenant_id", "acme"),
                    ("cell_id", "cell-001"),
                    ("lifecycle_state", "active"),
                ]),
                parents: vec![],
            },
            EntityRecord {
                uid: entity_ref("OyaPlatform::Tenant", "globex"),
                attributes: string_attrs(&[
                    ("tenant_id", "globex"),
                    ("cell_id", "cell-002"),
                    ("lifecycle_state", "active"),
                ]),
                parents: vec![],
            },
            EntityRecord {
                uid: entity_ref("OyaPlatform::Group", "tenant-admins"),
                attributes: string_attrs(&[("tenant_id", "acme")]),
                parents: vec![entity_ref("OyaPlatform::Tenant", "acme")],
            },
            EntityRecord {
                uid: entity_ref("OyaPlatform::Principal", "alice"),
                attributes: string_attrs(&[
                    ("tenant_id", "acme"),
                    ("kind", "human"),
                    ("step_up_class", "a"),
                ]),
                parents: vec![entity_ref("OyaPlatform::Group", "tenant-admins")],
            },
            // bob: acme principal WITHOUT step-up class and WITHOUT group
            // membership — only a PBAC template link can let him read.
            EntityRecord {
                uid: entity_ref("OyaPlatform::Principal", "bob"),
                attributes: string_attrs(&[("tenant_id", "acme"), ("kind", "human")]),
                parents: vec![],
            },
            // mallory belongs to ANOTHER tenant but is (mis)joined to the
            // same group — the structural forbid must still deny everything.
            EntityRecord {
                uid: entity_ref("OyaPlatform::Principal", "mallory"),
                attributes: string_attrs(&[
                    ("tenant_id", "globex"),
                    ("kind", "human"),
                    ("step_up_class", "a"),
                ]),
                parents: vec![entity_ref("OyaPlatform::Group", "tenant-admins")],
            },
            EntityRecord {
                uid: entity_ref("OyaPlatform::WorkloadIdentity", "payments"),
                attributes: string_attrs(&[
                    ("tenant_id", "acme"),
                    ("spiffe_id", "spiffe://oyatie/acme/payments"),
                ]),
                parents: vec![entity_ref("OyaPlatform::Tenant", "acme")],
            },
            EntityRecord {
                uid: entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
                attributes: string_attrs(&[
                    ("tenant_id", "acme"),
                    ("resource_kind", "document"),
                    ("data_class", "restricted"),
                    ("cell_id", "cell-001"),
                ]),
                parents: vec![entity_ref("OyaPlatform::Tenant", "acme")],
            },
            EntityRecord {
                uid: non_restricted_acme_doc(),
                attributes: string_attrs(&[
                    ("tenant_id", "acme"),
                    ("resource_kind", "document"),
                    ("data_class", "internal"),
                    ("cell_id", "cell-001"),
                ]),
                parents: vec![entity_ref("OyaPlatform::Tenant", "acme")],
            },
            EntityRecord {
                uid: foreign_tenant_doc(),
                attributes: string_attrs(&[
                    ("tenant_id", "globex"),
                    ("resource_kind", "document"),
                    ("data_class", "internal"),
                    ("cell_id", "cell-002"),
                ]),
                parents: vec![entity_ref("OyaPlatform::Tenant", "globex")],
            },
        ],
    }
}

pub fn request(
    request_id: &str,
    tenant_id: &str,
    principal: EntityRef,
    action: &str,
    resource: EntityRef,
) -> AuthorizationRequest {
    AuthorizationRequest {
        request_id: request_id.to_owned(),
        tenant_id: tenant_id.to_owned(),
        principal,
        action: action.to_owned(),
        resource,
        context: BTreeMap::new(),
        min_policy_version: None,
    }
}

pub fn pdp(links: Vec<TemplateLink>) -> CedarPdp {
    CedarPdp::load(
        &locked_seed_bundle("psv-000001", links),
        Arc::new(SeededIdGenerator::default()),
        64,
    )
    .expect("locked seed bundle must load")
}

pub fn unique_ledger_path(test_name: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test-side: system clock must be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "pdp-audit-chain-{test_name}-{}-{now}.ledger",
        std::process::id()
    ))
}

pub fn audit_input(tenant_id: &str, decision: &str) -> AuditAppendInput {
    AuditAppendInput {
        tenant_id: tenant_id.to_owned(),
        surface: PDP_DECISION_AUDIT_SURFACE.to_owned(),
        plane: Plane::Control,
        purpose: Purpose::CoreService,
        data_classes: vec![DataClass::InternalOnly, DataClass::Audit],
        decision: decision.to_owned(),
    }
}
