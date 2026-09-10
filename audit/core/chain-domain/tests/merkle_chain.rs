#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "assertion failure IS the test signal; ADR-0083 Tier 3 cfg(test) exemption"
)]

use audit_chain_domain::{
    AuditAppendInput, AuditChain, AuditChainError, Ed25519SigningKey, Ed25519VerificationKeySet,
    Plane, append, verify_chain,
};
use data_boundary_kernel::{DataClass, Purpose};

#[test]
fn merkle_root_advances_with_each_append_and_detects_payload_tamper() {
    let signer = Ed25519SigningKey::from_seed_bytes("audit-test-key", [7_u8; 32])
        .expect("test seed builds signing key");
    let trusted_keys =
        Ed25519VerificationKeySet::single(signer.verification_key()).expect("trusted test key set");
    let mut chain = AuditChain::default();

    let first = chain
        .append_signed(
            AuditAppendInput {
                tenant_id: "ten_alpha".to_string(),
                surface: "tenant.create".to_string(),
                plane: Plane::Control,
                purpose: Purpose::CoreService,
                data_classes: vec![DataClass::InternalOnly],
                decision: "ALLOW".to_string(),
            },
            &signer,
        )
        .expect("first signed append succeeds")
        .clone();
    let second = chain
        .append_signed(
            AuditAppendInput {
                tenant_id: "ten_alpha".to_string(),
                surface: "identity.user.upsert".to_string(),
                plane: Plane::Control,
                purpose: Purpose::CoreService,
                data_classes: vec![DataClass::PiiIdentifying],
                decision: "ALLOW".to_string(),
            },
            &signer,
        )
        .expect("second signed append succeeds")
        .clone();

    assert_eq!(first.sequence, 0);
    assert_eq!(first.previous_hash, "GENESIS");
    assert_eq!(first.tenant_shard.as_str(), "tenant:ten_alpha");
    assert!(first.hash.starts_with("sha256:"));
    assert!(first.merkle_root.as_str().starts_with("merkle-sha256:"));
    assert_ne!(first.merkle_root, second.merkle_root);
    assert_eq!(second.previous_hash, first.hash);
    assert_eq!(
        verify_chain(&chain).expect("chain verifies"),
        second.merkle_root
    );
    assert_eq!(
        chain
            .verify_signed_with_keys(&trusted_keys)
            .expect("signed chain verifies"),
        second.merkle_root
    );

    let mut tampered_events = chain.events().to_vec();
    tampered_events[0].surface = "tenant.delete".to_string();
    assert_eq!(
        AuditChain::from_events(tampered_events),
        Err(AuditChainError::InvalidChain)
    );
}

#[test]
fn merkle_chain_rejects_cross_tenant_shard_append() {
    let mut chain = AuditChain::default();
    append(
        &mut chain,
        AuditAppendInput {
            tenant_id: "ten_alpha".to_string(),
            surface: "tenant.create".to_string(),
            plane: Plane::Control,
            purpose: Purpose::CoreService,
            data_classes: vec![DataClass::InternalOnly],
            decision: "ALLOW".to_string(),
        },
        None,
    )
    .expect("first tenant establishes shard");

    let error = append(
        &mut chain,
        AuditAppendInput {
            tenant_id: "ten_beta".to_string(),
            surface: "tenant.create".to_string(),
            plane: Plane::Control,
            purpose: Purpose::CoreService,
            data_classes: vec![DataClass::InternalOnly],
            decision: "ALLOW".to_string(),
        },
        None,
    )
    .expect_err("chain is a single per-tenant shard");

    assert!(matches!(error, AuditChainError::TenantShardMismatch { .. }));
}
