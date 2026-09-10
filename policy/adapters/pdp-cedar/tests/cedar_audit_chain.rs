#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod conformance;

use conformance::*;

#[test]
fn audited_pdp_persists_signed_audit_chain_event_with_decision_lineage() {
    let ledger_path = unique_ledger_path("decision-lineage");
    let signer = Ed25519SigningKey::from_seed_bytes("pdp-audit-test-key", [7_u8; 32]).unwrap();
    let trusted_keys = Ed25519VerificationKeySet::single(signer.verification_key()).unwrap();
    let logger = PdpDecisionAuditChainLogger::new(
        FileAuditLedger::new(ledger_path.clone()),
        signer,
        trusted_keys.clone(),
    )
    .unwrap();
    let pdp = AuditChainCedarPdp::load(
        &locked_seed_bundle("psv-000001", vec![]),
        Arc::new(SeededIdGenerator::default()),
        64,
        logger,
    )
    .unwrap();

    let outcome = pdp
        .authorize(
            &request(
                "req-audit-chain-1",
                "acme",
                entity_ref("OyaPlatform::Principal", "alice"),
                "resource.read",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap();

    let persisted = FileAuditLedger::new(ledger_path.clone())
        .load_with_trusted_keys(&trusted_keys)
        .unwrap();
    let events = persisted.events();
    assert_eq!(
        events.len(),
        1,
        "exactly one durable audit-chain event is appended"
    );
    let event = &events[0];
    assert_eq!(event.tenant_id, "acme");
    assert_eq!(event.surface, PDP_DECISION_AUDIT_SURFACE);
    assert_eq!(event.plane, Plane::Control);
    assert_eq!(event.purpose, Purpose::CoreService);
    assert_eq!(
        event.data_classes,
        vec![DataClass::InternalOnly, DataClass::Audit]
    );
    assert!(
        event.ed25519_signature.is_some(),
        "audit-chain event must be signed"
    );

    let lineage: serde_json::Value = serde_json::from_str(&event.decision).unwrap();
    assert_eq!(lineage["decision_id"], outcome.response.decision_id);
    assert_eq!(lineage["request_id"], "req-audit-chain-1");
    assert_eq!(lineage["tenant_id"], "acme");
    assert_eq!(lineage["resource"]["entity_id"], "acme-doc-1");
    assert_eq!(lineage["action"], "resource.read");
    assert_eq!(lineage["decision"], "allow");
    assert_eq!(lineage["policy_version"], "psv-000001");

    std::fs::remove_file(ledger_path).ok();
}

#[test]
fn audited_pdp_refuses_unsigned_prior_multi_tenant_ledger_replay() {
    let ledger_path = unique_ledger_path("unsigned-prior-ledger");
    let ledger = FileAuditLedger::new(ledger_path.clone());
    let mut chain = AuditChain::multi_tenant_shards();
    audit_append(&mut chain, audit_input("acme", "preexisting-acme"), None).unwrap();
    audit_append(
        &mut chain,
        audit_input("globex", "preexisting-globex"),
        None,
    )
    .unwrap();
    ledger.append_chain(&chain).unwrap();

    let signer = Ed25519SigningKey::from_seed_bytes("pdp-audit-test-key", [7_u8; 32]).unwrap();
    let trusted_keys = Ed25519VerificationKeySet::single(signer.verification_key()).unwrap();
    let err = PdpDecisionAuditChainLogger::new(ledger, signer, trusted_keys).unwrap_err();

    assert!(
        matches!(
            err,
            PdpAuditChainError::TrustedSignatureReplay(
                AuditChainError::MissingEd25519Signature { .. }
            )
        ),
        "unsigned prior ledger must fail closed at startup, got {err:?}"
    );

    std::fs::remove_file(ledger_path).ok();
}

#[test]
fn audited_pdp_refuses_attacker_signed_prior_multi_tenant_ledger_replay() {
    let ledger_path = unique_ledger_path("attacker-signed-prior-ledger");
    let ledger = FileAuditLedger::new(ledger_path.clone());
    let trusted_signer =
        Ed25519SigningKey::from_seed_bytes("pdp-audit-test-key", [7_u8; 32]).unwrap();
    let attacker_signer =
        Ed25519SigningKey::from_seed_bytes("attacker-pdp-audit-key", [9_u8; 32]).unwrap();
    let mut chain = AuditChain::multi_tenant_shards();
    audit_append(
        &mut chain,
        audit_input("acme", "attacker-signed-acme"),
        Some(&attacker_signer),
    )
    .unwrap();
    audit_append(
        &mut chain,
        audit_input("globex", "attacker-signed-globex"),
        Some(&attacker_signer),
    )
    .unwrap();
    ledger.append_chain(&chain).unwrap();

    let trusted_keys =
        Ed25519VerificationKeySet::single(trusted_signer.verification_key()).unwrap();
    let err = PdpDecisionAuditChainLogger::new(ledger, trusted_signer, trusted_keys).unwrap_err();

    assert!(
        matches!(
            err,
            PdpAuditChainError::TrustedSignatureReplay(
                AuditChainError::MissingTrustedEd25519Key { .. }
            )
        ),
        "attacker-signed prior ledger must fail closed at startup, got {err:?}"
    );

    std::fs::remove_file(ledger_path).ok();
}

#[test]
fn audited_pdp_refuses_serving_signer_when_trusted_key_material_differs() {
    let ledger_path = unique_ledger_path("untrusted-serving-signer");
    let signer = Ed25519SigningKey::from_seed_bytes("pdp-audit-test-key", [7_u8; 32]).unwrap();
    let key_id_collision =
        Ed25519SigningKey::from_seed_bytes("pdp-audit-test-key", [9_u8; 32]).unwrap();
    let trusted_keys =
        Ed25519VerificationKeySet::single(key_id_collision.verification_key()).unwrap();
    let err = PdpDecisionAuditChainLogger::new(
        FileAuditLedger::new(ledger_path.clone()),
        signer,
        trusted_keys,
    )
    .unwrap_err();

    assert!(
        matches!(
            err,
            PdpAuditChainError::UntrustedSigner(
                AuditChainError::Ed25519SignatureKeyMismatch { .. }
            )
        ),
        "serving signer must match trusted key material, got {err:?}"
    );

    std::fs::remove_file(ledger_path).ok();
}

#[test]
fn audited_pdp_maps_untrusted_persisted_ledger_to_audit_chain_emission() {
    let ledger_path = unique_ledger_path("audit-emission-fail-closed");
    let signer = Ed25519SigningKey::from_seed_bytes("pdp-audit-test-key", [7_u8; 32]).unwrap();
    let trusted_keys = Ed25519VerificationKeySet::single(signer.verification_key()).unwrap();
    let logger = PdpDecisionAuditChainLogger::new(
        FileAuditLedger::new(ledger_path.clone()),
        signer,
        trusted_keys,
    )
    .unwrap();
    let pdp = AuditChainCedarPdp::load(
        &locked_seed_bundle("psv-000001", vec![]),
        Arc::new(SeededIdGenerator::default()),
        64,
        logger,
    )
    .unwrap();

    let mut untrusted_chain = AuditChain::multi_tenant_shards();
    audit_append(
        &mut untrusted_chain,
        audit_input("acme", "post-startup-unsigned"),
        None,
    )
    .unwrap();
    FileAuditLedger::new(ledger_path.clone())
        .append_chain(&untrusted_chain)
        .unwrap();

    let err = pdp
        .authorize(
            &request(
                "req-audit-chain-fail-closed",
                "acme",
                entity_ref("OyaPlatform::Principal", "alice"),
                "resource.read",
                entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            ),
            &entity_slice(),
        )
        .unwrap_err();

    match err {
        PdpError::AuditChainEmission { detail } => {
            assert!(
                detail.contains("trusted signature replay failed"),
                "audit-chain emission detail must name trusted replay failure, got {detail:?}"
            );
            assert!(
                detail.contains("MissingEd25519Signature"),
                "audit-chain emission detail must preserve signature cause, got {detail:?}"
            );
        }
        other => {
            panic!("untrusted persisted ledger must fail closed as audit emission, got {other:?}")
        }
    }

    std::fs::remove_file(ledger_path).ok();
}
