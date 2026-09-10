#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! End-to-end token validation against real `aws-lc-rs` crypto rather than a
//! stubbed verifier, so a signature check that silently passes everything would
//! fail these tests.

use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use iam_identity_workload_domain::WorkloadState;
use iam_identity_workload_oidc::{
    Jwk, Jwks, OidcValidationError, ValidationConfig, validate_workload_token,
};

fn b64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn mint(claims_json: &str, kid: &str) -> (String, Jwk) {
    let rng = SystemRandom::new();
    let pkcs8 =
        EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).expect("pkcs8");
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref()).expect("key");
    let public = key.public_key().as_ref();
    let (x, y) = (&public[1..33], &public[33..65]);

    let header = format!(r#"{{"alg":"ES256","typ":"JWT","kid":"{kid}"}}"#);
    let signing_input = format!(
        "{}.{}",
        b64url(header.as_bytes()),
        b64url(claims_json.as_bytes())
    );
    let sig = key.sign(&rng, signing_input.as_bytes()).expect("sign");
    (
        format!("{signing_input}.{}", b64url(sig.as_ref())),
        Jwk::ec_p256(kid, b64url(x), b64url(y)),
    )
}

#[test]
fn genuine_workload_token_validates_into_active_principal() {
    let now: i64 = 1_750_000_000;
    let claims = format!(
        r#"{{"iss":"https://idp.oyatie.com","aud":"cloud-storage","exp":{},"iat":{},"tenant_id":"ten_globex","sub":"wl_backup_agent","owning_capability":"cap.cloud.storage","scope":"storage.object.read storage.object.list"}}"#,
        now + 600,
        now
    );
    let (token, jwk) = mint(&claims, "globex-2026");
    let jwks = Jwks::new().add_key(jwk);
    let config = ValidationConfig::new("https://idp.oyatie.com", "cloud-storage");

    let principal = validate_workload_token(&token, &jwks, &config, now).expect("valid token");
    assert_eq!(principal.tenant_id().as_str(), "ten_globex");
    assert_eq!(principal.workload_id().as_str(), "wl_backup_agent");
    assert_eq!(principal.state(), WorkloadState::Active);
    assert!(principal.has_scope("storage.object.read"));
    assert!(principal.has_scope("storage.object.list"));
}

#[test]
fn token_signed_by_untrusted_key_is_rejected() {
    let now: i64 = 1_750_000_000;
    let claims = format!(
        r#"{{"iss":"https://idp.oyatie.com","aud":"cloud-storage","exp":{},"tenant_id":"ten_globex","sub":"wl_x","owning_capability":"cap.cloud.storage"}}"#,
        now + 600
    );
    // Same kid, different key: the kid must not be trusted to select a key
    // whose signature was never checked.
    let (token, _attacker_jwk) = mint(&claims, "globex-2026");
    let (_other_token, honest_jwk) = mint(&claims, "globex-2026");
    let jwks = Jwks::new().add_key(honest_jwk);
    let config = ValidationConfig::new("https://idp.oyatie.com", "cloud-storage");

    let err = validate_workload_token(&token, &jwks, &config, now)
        .expect_err("untrusted signer must be rejected");
    assert!(matches!(err, OidcValidationError::SignatureInvalid));
}
