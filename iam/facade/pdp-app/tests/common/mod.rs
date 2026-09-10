//! One fixture set for the REST suite, the gRPC suite, and the live-socket
//! E2E, so no surface can be tested against a bundle the others never see.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test fixtures; not every consumer uses every helper"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use iam_pdp_app::PdpState;
use iam_pdp_bundle_file::{BundleSignature, SignedPolicyBundleDoc};
use iam_pdp_cedar::CedarPdp;
use iam_pdp_kernel::InMemoryDecisionAuditSink;
use shared_audit_digest_adapter_awslc::Ed25519ChainSigner;
use shared_audit_event_kernel::{ChainSigner, encode_hex};
use shared_pdp_kernel::{EntityRecord, EntitySlice, PolicyBundle, TemplateLink, TemplateSrc};
use shared_platform_contracts_kernel::pdp::{AuthorizationRequest, EntityRef, PolicyVersion};
use shared_ulid_id_kernel::SeededIdGenerator;

pub const SCHEMA_SRC: &str = include_str!("../../cedar/platform.cedarschema");
pub const POLICIES_SRC: &str = include_str!("../../cedar/platform-policies.cedar");
pub const TEMPLATE_SRC: &str = include_str!("../../cedar/platform-templates.cedar");

pub const TEMPLATE_ID: &str = "pbac-resource-read-grant";
pub const SEED_VERSION: &str = "psv-000001";

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

pub fn seed_bundle(version: &str, template_links: Vec<TemplateLink>) -> PolicyBundle {
    PolicyBundle {
        version: PolicyVersion::new(version).unwrap(),
        schema_src: SCHEMA_SRC.to_owned(),
        policies_src: POLICIES_SRC.to_owned(),
        tenant_policies: BTreeMap::new(),
        templates: vec![TemplateSrc {
            template_id: TEMPLATE_ID.to_owned(),
            src: TEMPLATE_SRC.to_owned(),
        }],
        template_links,
        action_map: action_map(),
    }
}

pub fn bob_read_link() -> TemplateLink {
    TemplateLink {
        template_id: TEMPLATE_ID.to_owned(),
        link_id: "pbac-link-bob-acme-doc-2".to_owned(),
        principal: entity_ref("OyaPlatform::Principal", "bob"),
        resource: entity_ref("OyaPlatform::TenantResource", "acme-doc-2"),
    }
}

/// mallory is deliberately mis-joined into acme's admin group: without the
/// structural tenant forbid she would authorize, so she is the fixture that
/// proves the forbid, not a mistake in the data.
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
            EntityRecord {
                uid: entity_ref("OyaPlatform::Principal", "bob"),
                attributes: string_attrs(&[("tenant_id", "acme"), ("kind", "human")]),
                parents: vec![],
            },
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
                uid: entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
                attributes: string_attrs(&[
                    ("tenant_id", "acme"),
                    ("resource_kind", "document"),
                    ("data_class", "restricted"),
                    ("cell_id", "cell-001"),
                ]),
                parents: vec![entity_ref("OyaPlatform::Tenant", "acme")],
            },
            // Non-restricted, so an ordinary read grant is testable here
            // without tripping the step-up forbid that gates acme-doc-1.
            EntityRecord {
                uid: entity_ref("OyaPlatform::TenantResource", "acme-doc-2"),
                attributes: string_attrs(&[
                    ("tenant_id", "acme"),
                    ("resource_kind", "document"),
                    ("data_class", "internal"),
                    ("cell_id", "cell-001"),
                ]),
                parents: vec![entity_ref("OyaPlatform::Tenant", "acme")],
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

/// Ids come from a seeded generator, so no assertion depends on wall-clock
/// entropy.
pub fn seeded_state(links: Vec<TemplateLink>) -> (Arc<PdpState>, Arc<InMemoryDecisionAuditSink>) {
    let pdp = CedarPdp::load(
        &seed_bundle(SEED_VERSION, links),
        Arc::new(SeededIdGenerator::default()),
        64,
    )
    .expect("seed bundle must load");
    let sink = Arc::new(InMemoryDecisionAuditSink::new());
    let state = Arc::new(PdpState::new(pdp, Arc::clone(&sink) as Arc<_>));
    (state, sink)
}

/// Writes `contents` VERBATIM: wrap a bundle with [`signed_bundle_doc`] first,
/// or pass raw bytes to build a malformed-envelope fixture.
pub fn temp_bundle_file(tag: &str, contents: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("iam-pdp-e2e-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("bundle.json");
    std::fs::write(&path, contents).expect("write bundle");
    path
}

pub const TEST_SIGNING_KEY_ID: &str = "test-policy-signing-key";

/// Process-global, so a bundle written by one helper verifies against a trust
/// dir written by another.
fn test_signer() -> &'static Ed25519ChainSigner {
    use std::sync::OnceLock;
    static SIGNER: OnceLock<Ed25519ChainSigner> = OnceLock::new();
    SIGNER.get_or_init(|| {
        Ed25519ChainSigner::generate(TEST_SIGNING_KEY_ID).expect("test signer keygen")
    })
}

/// The signature covers the exact `inner_json` bytes embedded in the envelope,
/// so sign and verify agree by construction.
pub fn signed_bundle_doc(inner_json: &str) -> String {
    let signer = test_signer();
    let signature_hex = signer.sign_hex(inner_json.as_bytes()).expect("sign bundle");
    let doc = SignedPolicyBundleDoc {
        bundle: inner_json.to_owned(),
        signatures: vec![BundleSignature {
            key_id: TEST_SIGNING_KEY_ID.to_owned(),
            public_key_hex: encode_hex(&signer.public_key_bytes()),
            signature_hex,
        }],
    };
    serde_json::to_string(&doc).expect("serialize signed envelope")
}

/// A UNIQUE directory per call: parallel socket tests sharing one trust file
/// can read a mid-write `.pub`, whose truncated key bytes then reject a
/// perfectly good signature.
pub fn trust_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "iam-pdp-trust-{}-{}-{tag}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("trust dir");
    let hex = encode_hex(&test_signer().public_key_bytes());
    std::fs::write(dir.join(format!("{TEST_SIGNING_KEY_ID}.pub")), hex).expect("write trusted key");
    dir.to_string_lossy().into_owned().into()
}
