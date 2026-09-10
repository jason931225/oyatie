//! Integration rung: seal and verify a real audit
//! digest chain end-to-end — SHA-256 + Ed25519 via aws-lc-rs, audit
//! events serialized through the canonical CloudEvents envelope. GREEN
//! path plus RED tamper fixtures (forged batch, truncation, cross-key
//! forgery) — static-stability of verification demonstrated, not claimed.

use shared_audit_digest_adapter_awslc::{Ed25519ChainSigner, Ed25519ChainVerifier, Sha256Digester};
use shared_audit_event_kernel::{
    AUDIT_PAYLOAD_SCHEMA_VERSION, AuditCloudEvent, AuditLogPayload, AuditStatus, AuditStream,
    AuthenticationInfo, AuthorizationInfo, DigestChainError, Digester, GENESIS_PREV_LINK_DIGEST,
    RequestMetadata, link_digest_hex, seal_link, verify_chain,
};

fn admin_event(sequence: u64) -> AuditCloudEvent {
    AuditCloudEvent::new(
        format!("01HMZX00000000000000000{sequence:03}"),
        "//oyatie.com/cloud-tenancy/cell/cell-kr-1",
        "2026-06-10T00:00:00Z",
        AuditLogPayload {
            schema_version: AUDIT_PAYLOAD_SCHEMA_VERSION,
            stream: AuditStream::AdminActivity,
            service_name: "cloud-tenancy".into(),
            method_name: "oya.cloud.tenancy.v1.CreateTenant".into(),
            resource_name: format!("tenants/ten_acme{sequence}"),
            tenant_id: Some(format!("ten_acme{sequence}")),
            cell_id: "cell-kr-1".into(),
            authentication_info: AuthenticationInfo {
                principal: "wl_tenancy_cp".into(),
            },
            authorization_info: vec![AuthorizationInfo {
                resource: format!("tenants/ten_acme{sequence}"),
                permission: "cloud.tenancy.create".into(),
                granted: true,
                policy_version: Some("sha256:policybundle".into()),
            }],
            request_metadata: RequestMetadata::default(),
            status: AuditStatus::ok(),
        },
    )
    .expect("well-formed admin event")
}

fn batch_bytes(events: &[AuditCloudEvent]) -> Vec<u8> {
    let mut out = Vec::new();
    for event in events {
        out.extend_from_slice(&event.canonical_json().expect("canonical json"));
        out.push(b'\n');
    }
    out
}

fn seal_real_chain(
    signer: &Ed25519ChainSigner,
    batches: u64,
) -> Vec<shared_audit_event_kernel::DigestChainLink> {
    let digester = Sha256Digester;
    let mut links = Vec::new();
    let mut prev = GENESIS_PREV_LINK_DIGEST.to_owned();
    for sequence in 0..batches {
        let events: Vec<_> = (0..3).map(|i| admin_event(sequence * 3 + i)).collect();
        let link = seal_link(
            &digester,
            signer,
            sequence,
            &prev,
            &batch_bytes(&events),
            1_780_000_000 + sequence as i64,
        )
        .expect("seal");
        prev = link_digest_hex(&digester, &link);
        links.push(link);
    }
    links
}

#[test]
fn green_real_chain_seals_and_verifies() {
    let signer = Ed25519ChainSigner::generate("audit-seal-key-1").expect("keygen");
    let verifier =
        Ed25519ChainVerifier::new().with_key("audit-seal-key-1", signer.public_key_bytes());
    let links = seal_real_chain(&signer, 5);
    verify_chain(
        &Sha256Digester,
        &verifier,
        GENESIS_PREV_LINK_DIGEST,
        0,
        &links,
    )
    .expect("intact real chain must verify");
}

#[test]
fn red_forged_batch_digest_fails_signature() {
    let signer = Ed25519ChainSigner::generate("audit-seal-key-1").expect("keygen");
    let verifier =
        Ed25519ChainVerifier::new().with_key("audit-seal-key-1", signer.public_key_bytes());
    let mut links = seal_real_chain(&signer, 3);
    links[1].events_digest_hex = Sha256Digester.digest_hex(b"forged");
    let err = verify_chain(
        &Sha256Digester,
        &verifier,
        GENESIS_PREV_LINK_DIGEST,
        0,
        &links,
    )
    .expect_err("forged batch digest must fail");
    assert_eq!(err, DigestChainError::SignatureInvalid { sequence: 1 });
}

#[test]
fn red_truncated_chain_fails_with_sequence_gap() {
    let signer = Ed25519ChainSigner::generate("audit-seal-key-1").expect("keygen");
    let verifier =
        Ed25519ChainVerifier::new().with_key("audit-seal-key-1", signer.public_key_bytes());
    let links = seal_real_chain(&signer, 3);
    let truncated = vec![links[0].clone(), links[2].clone()];
    let err = verify_chain(
        &Sha256Digester,
        &verifier,
        GENESIS_PREV_LINK_DIGEST,
        0,
        &truncated,
    )
    .expect_err("truncation must fail");
    assert_eq!(
        err,
        DigestChainError::SequenceGap {
            expected: 1,
            found: 2
        }
    );
}

#[test]
fn red_cross_key_forgery_fails() {
    // Adversary seals a substitute suffix with their OWN key: verification
    // against the legitimate key registry rejects it.
    let legit = Ed25519ChainSigner::generate("audit-seal-key-1").expect("keygen");
    let adversary = Ed25519ChainSigner::generate("audit-seal-key-1").expect("keygen");
    let verifier =
        Ed25519ChainVerifier::new().with_key("audit-seal-key-1", legit.public_key_bytes());

    let mut links = seal_real_chain(&legit, 2);
    let digester = Sha256Digester;
    let prev = link_digest_hex(&digester, &links[1]);
    let forged_tail = seal_link(
        &digester,
        &adversary,
        2,
        &prev,
        b"adversarial-batch",
        1_780_000_010,
    )
    .expect("adversary can seal locally");
    links.push(forged_tail);

    let err = verify_chain(&digester, &verifier, GENESIS_PREV_LINK_DIGEST, 0, &links)
        .expect_err("cross-key forgery must fail");
    assert_eq!(err, DigestChainError::SignatureInvalid { sequence: 2 });
}

#[test]
fn checkpoint_resume_verifies_real_suffix() {
    let signer = Ed25519ChainSigner::generate("audit-seal-key-1").expect("keygen");
    let verifier =
        Ed25519ChainVerifier::new().with_key("audit-seal-key-1", signer.public_key_bytes());
    let links = seal_real_chain(&signer, 4);
    let checkpoint = link_digest_hex(&Sha256Digester, &links[1]);
    verify_chain(&Sha256Digester, &verifier, &checkpoint, 2, &links[2..])
        .expect("suffix from checkpoint must verify");
}
