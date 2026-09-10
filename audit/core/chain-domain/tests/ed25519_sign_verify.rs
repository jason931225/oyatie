#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "assertion failure IS the test signal; ADR-0083 Tier 3 cfg(test) exemption"
)]

use audit_chain_domain::{
    AuditAppendInput, AuditChain, AuditChainError, Ed25519Signature, Ed25519SigningKey,
    Ed25519VerificationKeySet, Plane, verify_chain,
};
use data_boundary_kernel::{DataClass, Purpose};

#[test]
fn ed25519_signature_seals_each_merkle_prefix_and_rejects_drift() {
    let signer = Ed25519SigningKey::from_seed_bytes("audit-ed25519-key", [42_u8; 32])
        .expect("test key is valid");
    let trusted_keys =
        Ed25519VerificationKeySet::single(signer.verification_key()).expect("trusted test key set");
    let mut chain = AuditChain::default();
    let event = chain
        .append_signed(
            AuditAppendInput {
                tenant_id: "ten_alpha".to_string(),
                surface: "foundry.capability.invoke".to_string(),
                plane: Plane::Audit,
                purpose: Purpose::CapabilityInvocation,
                data_classes: vec![DataClass::InternalOnly, DataClass::Audit],
                decision: "ALLOW".to_string(),
            },
            &signer,
        )
        .expect("signed append succeeds")
        .clone();

    let signature = event
        .ed25519_signature
        .as_ref()
        .expect("signed append records signature");
    assert_eq!(signature.key_id, signer.key_id());
    assert_eq!(
        signature.public_key_hex,
        signer.verification_key().public_key_hex
    );
    assert_eq!(signature.signature_hex.len(), 128);
    assert_eq!(
        chain
            .verify_signed_with_keys(&trusted_keys)
            .expect("signed chain verifies"),
        event.merkle_root
    );

    let mut tampered_signature_events = chain.events().to_vec();
    let tampered_signature_hex = if signature.signature_hex.starts_with('0') {
        format!("1{}", &signature.signature_hex[1..])
    } else {
        format!("0{}", &signature.signature_hex[1..])
    };
    tampered_signature_events[0].ed25519_signature = Some(Ed25519Signature {
        key_id: signature.key_id.clone(),
        public_key_hex: signature.public_key_hex.clone(),
        signature_hex: tampered_signature_hex,
    });
    assert_eq!(
        AuditChain::from_signed_events(tampered_signature_events, &trusted_keys),
        Err(AuditChainError::InvalidEd25519Signature)
    );

    let mut tampered_root_events = chain.events().to_vec();
    tampered_root_events[0].merkle_root.value =
        "merkle-sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .to_string();
    assert_eq!(
        AuditChain::from_events(tampered_root_events),
        Err(AuditChainError::InvalidChain)
    );
}

#[test]
fn signed_verification_rejects_embedded_key_substitution() {
    let trusted_signer =
        Ed25519SigningKey::from_seed_bytes("audit-ed25519-key", [42_u8; 32]).expect("trusted key");
    let attacker_signer =
        Ed25519SigningKey::from_seed_bytes("attacker-key", [43_u8; 32]).expect("attacker key");
    let trusted_keys = Ed25519VerificationKeySet::single(trusted_signer.verification_key())
        .expect("trusted key set");
    let mut trusted_chain = AuditChain::default();
    let mut attacker_chain = AuditChain::default();
    let input = AuditAppendInput {
        tenant_id: "ten_alpha".to_string(),
        surface: "foundry.capability.invoke".to_string(),
        plane: Plane::Audit,
        purpose: Purpose::CapabilityInvocation,
        data_classes: vec![DataClass::InternalOnly, DataClass::Audit],
        decision: "ALLOW".to_string(),
    };
    trusted_chain
        .append_signed(input.clone(), &trusted_signer)
        .expect("trusted append");
    attacker_chain
        .append_signed(input, &attacker_signer)
        .expect("attacker append with same payload");

    let mut forged_events = trusted_chain.events().to_vec();
    let attacker_signature = attacker_chain.events()[0]
        .ed25519_signature
        .as_ref()
        .expect("attacker signature");
    forged_events[0].ed25519_signature = Some(Ed25519Signature {
        key_id: trusted_signer.key_id().to_string(),
        public_key_hex: attacker_signature.public_key_hex.clone(),
        signature_hex: attacker_signature.signature_hex.clone(),
    });
    let structurally_valid =
        AuditChain::from_events(forged_events).expect("forged embedded key is structurally valid");

    assert_eq!(
        structurally_valid.verify_signed_with_keys(&trusted_keys),
        Err(AuditChainError::Ed25519SignatureKeyMismatch {
            key_id: trusted_signer.key_id().to_string()
        })
    );
}

#[test]
fn signed_verification_requires_every_event_to_have_ed25519_signature() {
    let mut chain = AuditChain::default();
    chain
        .append_classifications(
            "ten_alpha",
            "tenant.create",
            Plane::Control,
            Purpose::CoreService,
            [DataClass::InternalOnly],
            "ALLOW",
        )
        .expect("test fixture: append_classifications must succeed for valid inputs");

    assert!(verify_chain(&chain).is_ok());
    assert_eq!(
        chain.verify_signed_with_keys(
            &Ed25519VerificationKeySet::single(
                Ed25519SigningKey::from_seed_bytes("audit-ed25519-key", [42_u8; 32])
                    .expect("test key")
                    .verification_key(),
            )
            .expect("trusted key set"),
        ),
        Err(AuditChainError::MissingEd25519Signature { sequence: 0 })
    );
}
