use shared_audit_digest_adapter_awslc::Ed25519ChainSigner;
use shared_audit_event_kernel::{ChainSigner, encode_hex};

use super::reader_test_support::*;
use super::{
    BundleSignature, BundleStoreError, FilePolicyBundleStore, PolicyBundleStore,
    SignedPolicyBundleDoc,
};

#[test]
fn well_formed_signed_bundle_round_trips() {
    let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
    let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
    let bundle_path = bundle_file("green", &signed_doc_json(&inner, "psk-1", &signer));
    let trust = trust_dir_for("green", &[("psk-1", &signer)]);

    let store = FilePolicyBundleStore::new(&bundle_path, &trust);
    let loaded = store.load().expect("signed bundle loads");
    assert_eq!(loaded, seed_bundle_with_nonempty_overlay());
    assert!(
        store
            .describe()
            .contains(&bundle_path.display().to_string())
    );
    assert!(store.describe().contains(&trust.display().to_string()));
}

#[test]
fn unsigned_bundle_fails_closed() {
    let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
    let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
    let doc = SignedPolicyBundleDoc {
        bundle: inner,
        signatures: vec![],
    };
    let bundle_path = bundle_file("unsigned", &serde_json::to_string(&doc).unwrap());
    let trust = trust_dir_for("unsigned", &[("psk-1", &signer)]);
    let err = FilePolicyBundleStore::new(&bundle_path, &trust)
        .load()
        .unwrap_err();
    assert!(
        matches!(err, BundleStoreError::SignatureRejected { .. }),
        "{err}"
    );
}

#[test]
fn tampered_inner_bytes_are_rejected() {
    let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
    let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
    let doc_json = signed_doc_json(&inner, "psk-1", &signer);
    // Flip one byte of the embedded inner bundle AFTER signing: the stored
    // bytes no longer match the signed bytes.
    let mut doc: SignedPolicyBundleDoc = serde_json::from_str(&doc_json).unwrap();
    let mut tampered = doc.bundle.into_bytes();
    // Mutate a byte deep inside the JSON (the policies_src value region) to a
    // different valid JSON character so the envelope still deserializes.
    let idx = tampered.len() / 2;
    tampered[idx] = if tampered[idx] == b'x' { b'y' } else { b'x' };
    doc.bundle = String::from_utf8(tampered).unwrap();
    let bundle_path = bundle_file("tampered", &serde_json::to_string(&doc).unwrap());
    let trust = trust_dir_for("tampered", &[("psk-1", &signer)]);
    let err = FilePolicyBundleStore::new(&bundle_path, &trust)
        .load()
        .unwrap_err();
    assert!(
        matches!(err, BundleStoreError::SignatureRejected { .. }),
        "{err}"
    );
}

#[test]
fn wrong_key_is_rejected() {
    let real = Ed25519ChainSigner::generate("psk-1").unwrap();
    let attacker = Ed25519ChainSigner::generate("psk-1").unwrap();
    let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
    // Signed by the attacker, but the trust set holds the REAL key under the
    // same key_id: the trusted key cannot validate the attacker's signature.
    let bundle_path = bundle_file("wrong-key", &signed_doc_json(&inner, "psk-1", &attacker));
    let trust = trust_dir_for("wrong-key", &[("psk-1", &real)]);
    let err = FilePolicyBundleStore::new(&bundle_path, &trust)
        .load()
        .unwrap_err();
    assert!(
        matches!(err, BundleStoreError::SignatureRejected { .. }),
        "{err}"
    );
}

#[test]
fn untrusted_key_id_is_rejected() {
    let signer = Ed25519ChainSigner::generate("rogue").unwrap();
    let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
    let bundle_path = bundle_file("untrusted", &signed_doc_json(&inner, "rogue", &signer));
    let trusted = Ed25519ChainSigner::generate("psk-1").unwrap();
    let trust = trust_dir_for("untrusted", &[("psk-1", &trusted)]);
    let err = FilePolicyBundleStore::new(&bundle_path, &trust)
        .load()
        .unwrap_err();
    assert!(
        matches!(err, BundleStoreError::SignatureRejected { .. }),
        "{err}"
    );
}

#[test]
fn version_token_enforced_inside_verified_region() {
    let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
    // A bundle whose version token is malformed, but VALIDLY SIGNED: the
    // signature passes, then the inner version-token re-validation rejects.
    let mut value = serde_json::to_value(seed_bundle_with_nonempty_overlay()).unwrap();
    value["version"] = serde_json::json!("has whitespace");
    let inner = value.to_string();
    let bundle_path = bundle_file("bad-version", &signed_doc_json(&inner, "psk-1", &signer));
    let trust = trust_dir_for("bad-version", &[("psk-1", &signer)]);
    let err = FilePolicyBundleStore::new(&bundle_path, &trust)
        .load()
        .unwrap_err();
    assert!(matches!(err, BundleStoreError::Malformed { .. }), "{err}");
    assert!(err.to_string().contains("version token rejected"), "{err}");
}

#[test]
fn unknown_inner_fields_rejected_inside_verified_region() {
    let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
    let mut value = serde_json::to_value(seed_bundle_with_nonempty_overlay()).unwrap();
    value["extra_field"] = serde_json::json!("smuggled");
    let inner = value.to_string();
    let bundle_path = bundle_file("unknown-inner", &signed_doc_json(&inner, "psk-1", &signer));
    let trust = trust_dir_for("unknown-inner", &[("psk-1", &signer)]);
    let err = FilePolicyBundleStore::new(&bundle_path, &trust)
        .load()
        .unwrap_err();
    assert!(matches!(err, BundleStoreError::Malformed { .. }), "{err}");
}

#[test]
fn unknown_envelope_field_is_rejected() {
    let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
    let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_str(&signed_doc_json(&inner, "psk-1", &signer)).unwrap();
    value["skip_verification"] = serde_json::json!(true);
    let bundle_path = bundle_file("unknown-env", &value.to_string());
    let trust = trust_dir_for("unknown-env", &[("psk-1", &signer)]);
    let err = FilePolicyBundleStore::new(&bundle_path, &trust)
        .load()
        .unwrap_err();
    assert!(matches!(err, BundleStoreError::Malformed { .. }), "{err}");
}

#[test]
fn key_rotation_any_trusted_key_validates() {
    let key_a = Ed25519ChainSigner::generate("psk-a").unwrap();
    let key_b = Ed25519ChainSigner::generate("psk-b").unwrap();
    let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
    let bundle_path = bundle_file("rotation", &signed_doc_json(&inner, "psk-b", &key_b));
    let trust = trust_dir_for("rotation", &[("psk-a", &key_a), ("psk-b", &key_b)]);
    let loaded = FilePolicyBundleStore::new(&bundle_path, &trust)
        .load()
        .expect("bundle signed by any trusted key loads");
    assert_eq!(loaded, seed_bundle_with_nonempty_overlay());
}

#[test]
fn first_untrusted_signature_then_trusted_signature_loads() {
    let trusted = Ed25519ChainSigner::generate("psk-1").unwrap();
    let rogue = Ed25519ChainSigner::generate("rogue").unwrap();
    let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
    // Envelope carries a rogue sig FIRST, then a trusted sig: scanning must
    // not stop at the rogue one — any trusted+valid sig admits the bundle.
    let rogue_sig = rogue.sign_hex(inner.as_bytes()).unwrap();
    let trusted_sig = trusted.sign_hex(inner.as_bytes()).unwrap();
    let doc = SignedPolicyBundleDoc {
        bundle: inner,
        signatures: vec![
            BundleSignature {
                key_id: "rogue".to_owned(),
                public_key_hex: encode_hex(&rogue.public_key_bytes()),
                signature_hex: rogue_sig,
            },
            BundleSignature {
                key_id: "psk-1".to_owned(),
                public_key_hex: encode_hex(&trusted.public_key_bytes()),
                signature_hex: trusted_sig,
            },
        ],
    };
    let bundle_path = bundle_file("multi-sig", &serde_json::to_string(&doc).unwrap());
    let trust = trust_dir_for("multi-sig", &[("psk-1", &trusted)]);
    let loaded = FilePolicyBundleStore::new(&bundle_path, &trust)
        .load()
        .expect("a trusted+valid signature anywhere admits the bundle");
    assert_eq!(loaded, seed_bundle_with_nonempty_overlay());
}

mod availability {
    use super::*;

    #[test]
    fn missing_bundle_file_is_unavailable_not_a_default() {
        let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
        let trust = trust_dir_for("missing-bundle", &[("psk-1", &signer)]);
        let store = FilePolicyBundleStore::new("/nonexistent/pdp/bundle.json", &trust);
        let err = store.load().unwrap_err();
        assert!(matches!(err, BundleStoreError::Unavailable { .. }), "{err}");
    }

    #[test]
    fn malformed_envelope_json_fails_closed() {
        let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
        let bundle_path = bundle_file("garbage", "{ not json");
        let trust = trust_dir_for("garbage", &[("psk-1", &signer)]);
        let err = FilePolicyBundleStore::new(&bundle_path, &trust)
            .load()
            .unwrap_err();
        assert!(matches!(err, BundleStoreError::Malformed { .. }), "{err}");
    }

    #[test]
    fn absent_trust_anchor_dir_refuses() {
        let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
        let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
        let bundle_path = bundle_file("absent-trust", &signed_doc_json(&inner, "psk-1", &signer));
        let absent = std::env::temp_dir().join(format!(
            "iam-pdp-bundle-file-{}-absent-trust-does-not-exist",
            unique("absent")
        ));
        let _ = std::fs::remove_dir_all(&absent);
        let err = FilePolicyBundleStore::new(&bundle_path, &absent)
            .load()
            .unwrap_err();
        assert!(matches!(err, BundleStoreError::Unavailable { .. }), "{err}");
    }

    #[test]
    fn empty_trust_anchor_dir_refuses() {
        let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
        let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
        let bundle_path = bundle_file("empty-trust", &signed_doc_json(&inner, "psk-1", &signer));
        let empty = test_dir(&format!("{}-empty-keys", unique("empty")));
        let err = FilePolicyBundleStore::new(&bundle_path, &empty)
            .load()
            .unwrap_err();
        assert!(
            matches!(err, BundleStoreError::SignatureRejected { .. }),
            "{err}"
        );
    }

    #[test]
    fn non_hex_trusted_key_is_malformed() {
        let inner = serde_json::to_string(&seed_bundle_with_nonempty_overlay()).unwrap();
        let signer = Ed25519ChainSigner::generate("psk-1").unwrap();
        let bundle_path = bundle_file("bad-keyhex", &signed_doc_json(&inner, "psk-1", &signer));
        let trust = test_dir(&format!("{}-bad-keyhex", unique("bad-keyhex")));
        std::fs::write(trust.join("psk-1.pub"), "zznothex").unwrap();
        let err = FilePolicyBundleStore::new(&bundle_path, &trust)
            .load()
            .unwrap_err();
        assert!(matches!(err, BundleStoreError::Malformed { .. }), "{err}");
    }
}
