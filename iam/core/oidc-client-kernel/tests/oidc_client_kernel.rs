#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! A stub `JwsVerifier` isolates the header, issuer, audience, expiry and ACR
//! checks from signature verification. Real-crypto coverage lives in the
//! adapter crate, so nothing here proves a signature is ever checked.

use serde_json::json;
use shared_oidc_client_kernel::{
    AcrLevel, Audience, Jwk, Jwks, JwsVerifier, JwtHeader, OidcClaims, OidcClient, OidcError,
    ReferenceOidcVerifier, VerifyConfig, b64url_encode,
};

/// Accepts every signature except under `kid == "broken"`, which is the only
/// way a test here can force a verification failure.
struct StubVerifier;
impl JwsVerifier for StubVerifier {
    fn verify(
        &self,
        jwk: &Jwk,
        _alg: &str,
        _signing_input: &[u8],
        _signature_b64url: &str,
    ) -> Result<(), OidcError> {
        if jwk.kid == "broken" {
            Err(OidcError::SignatureInvalid)
        } else {
            Ok(())
        }
    }
}

fn mk_jwks() -> Jwks {
    Jwks {
        keys: vec![
            Jwk {
                kid: "k1".into(),
                kty: "RSA".into(),
                alg: "RS256".into(),
                r#use: Some("sig".into()),
                n: Some("dummy".into()),
                e: Some("AQAB".into()),
                x: None,
                y: None,
                crv: None,
            },
            Jwk {
                kid: "broken".into(),
                kty: "RSA".into(),
                alg: "RS256".into(),
                r#use: Some("sig".into()),
                n: Some("dummy".into()),
                e: Some("AQAB".into()),
                x: None,
                y: None,
                crv: None,
            },
        ],
    }
}

fn forge_token(kid: &str, payload: serde_json::Value) -> String {
    let header = JwtHeader {
        alg: "RS256".into(),
        kid: kid.into(),
        typ: Some("JWT".into()),
    };
    let h = b64url_encode(
        serde_json::to_vec(&header)
            .expect("header serialize")
            .as_slice(),
    );
    let p = b64url_encode(
        serde_json::to_vec(&payload)
            .expect("payload serialize")
            .as_slice(),
    );
    let s = b64url_encode(b"signature-stub");
    format!("{h}.{p}.{s}")
}

fn default_cfg(now: i64) -> VerifyConfig {
    VerifyConfig {
        expected_issuer: "https://identity-eu.oyatie.com".into(),
        expected_audience: "foundry".into(),
        clock_tolerance: Default::default(),
        now_unix_seconds: now,
    }
}

fn valid_claims_json() -> serde_json::Value {
    json!({
        "iss": "https://identity-eu.oyatie.com",
        "aud": "foundry",
        "sub": "user-123",
        "iat": 1_700_000_000_i64,
        "exp": 1_700_000_900_i64,
        "tenant_id": "tenant-acme",
        "acr": "elevated",
        "acr_event_at": 1_700_000_000_i64,
        "purpose": "feature-billing",
        "data_class": "FINANCIAL"
    })
}

#[test]
fn verifies_well_formed_token() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let token = forge_token("k1", valid_claims_json());
    let claims = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .expect("verify ok");
    assert_eq!(claims.tenant_id, "tenant-acme");
    assert_eq!(claims.acr, AcrLevel::Elevated);
    assert!(matches!(claims.aud, Audience::Single(ref a) if a == "foundry"));
}

#[test]
fn rejects_unknown_kid() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let token = forge_token("k-unknown", valid_claims_json());
    let err = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .unwrap_err();
    assert!(matches!(err, OidcError::UnknownKid(_)));
}

#[test]
fn rejects_invalid_signature() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let token = forge_token("broken", valid_claims_json());
    let err = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .unwrap_err();
    assert_eq!(err, OidcError::SignatureInvalid);
}

#[test]
fn rejects_wrong_issuer() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let mut p = valid_claims_json();
    p["iss"] = json!("https://evil.example");
    let token = forge_token("k1", p);
    let err = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .unwrap_err();
    assert!(matches!(err, OidcError::IssuerMismatch { .. }));
}

#[test]
fn rejects_wrong_audience() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let mut p = valid_claims_json();
    p["aud"] = json!("not-foundry");
    let token = forge_token("k1", p);
    let err = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .unwrap_err();
    assert!(matches!(err, OidcError::AudienceMismatch { .. }));
}

#[test]
fn accepts_audience_array_when_match_present() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let mut p = valid_claims_json();
    p["aud"] = json!(["a", "foundry", "b"]);
    let token = forge_token("k1", p);
    let claims = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .expect("ok");
    assert!(matches!(claims.aud, Audience::Many(ref v) if v.len() == 3));
}

#[test]
fn rejects_expired_token_outside_skew() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let token = forge_token("k1", valid_claims_json());
    // 100s past exp, which is beyond the 60s skew allowance.
    let err = verifier
        .verify(&token, &default_cfg(1_700_001_000))
        .unwrap_err();
    assert!(matches!(err, OidcError::Expired { .. }));
}

#[test]
fn accepts_just_expired_within_skew() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let token = forge_token("k1", valid_claims_json());
    // 50s past exp, which the 60s skew allowance still covers.
    verifier
        .verify(&token, &default_cfg(1_700_000_950))
        .expect("within skew should pass");
}

#[test]
fn rejects_missing_tenant_id_claim() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let mut p = valid_claims_json();
    p["tenant_id"] = json!("");
    let token = forge_token("k1", p);
    let err = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .unwrap_err();
    assert_eq!(err, OidcError::MissingClaim("tenant_id"));
}

#[test]
fn acr_ordering_routine_lt_critical() {
    assert!(AcrLevel::Critical.meets(AcrLevel::Routine));
    assert!(!AcrLevel::Routine.meets(AcrLevel::Critical));
    assert!(AcrLevel::Sensitive.meets(AcrLevel::Elevated));
    assert!(!AcrLevel::Elevated.meets(AcrLevel::Sensitive));
}

#[test]
fn rejects_malformed_three_segments_required() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let err = verifier
        .verify("not.a.valid.jwt", &default_cfg(1_700_000_500))
        .unwrap_err();
    assert!(matches!(err, OidcError::Malformed(_)));
    let err = verifier
        .verify("a.b", &default_cfg(1_700_000_500))
        .unwrap_err();
    assert!(matches!(err, OidcError::Malformed(_)));
}

#[test]
fn meets_acr_helper_through_oidcclient_trait() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let token = forge_token("k1", valid_claims_json());
    let claims = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .expect("ok");
    assert!(verifier.meets_acr(&claims, AcrLevel::Routine));
    assert!(verifier.meets_acr(&claims, AcrLevel::Elevated));
    assert!(!verifier.meets_acr(&claims, AcrLevel::Sensitive));
    assert!(!verifier.meets_acr(&claims, AcrLevel::Critical));
}

#[test]
fn jwks_from_json_parses() {
    let json =
        br#"{"keys":[{"kid":"k1","kty":"RSA","alg":"RS256","use":"sig","n":"n","e":"AQAB"}]}"#;
    let jwks = Jwks::from_json(json).expect("parse");
    assert!(jwks.find("k1").is_some());
    assert!(jwks.find("k2").is_none());
}

#[test]
fn b64url_round_trip() {
    use shared_oidc_client_kernel::b64url_decode;
    let original = b"hello world: oyatie identity test \xff\x00\x7f";
    let encoded = b64url_encode(original);
    assert!(!encoded.contains('='));
    assert!(!encoded.contains('+'));
    assert!(!encoded.contains('/'));
    let decoded = b64url_decode(&encoded).expect("decode");
    assert_eq!(decoded, original);
}

#[test]
fn rejects_disallowed_alg_hs256() {
    // A symmetric alg lets anyone holding the verification key also mint
    // tokens, so an RP must refuse it (RFC 8725 §3.1).
    let mut verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    // Set explicitly rather than relying on the default, so this stays a test
    // of the rejection and not of the default list.
    verifier.allowed_algs = vec!["RS256".to_owned(), "ES256".to_owned()];
    let header = JwtHeader {
        alg: "HS256".into(),
        kid: "k1".into(),
        typ: Some("JWT".into()),
    };
    let h = b64url_encode(&serde_json::to_vec(&header).expect("ser"));
    let p = b64url_encode(&serde_json::to_vec(&valid_claims_json()).expect("ser"));
    let s = b64url_encode(b"sig");
    let token = format!("{h}.{p}.{s}");
    let err = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .unwrap_err();
    assert!(matches!(err, OidcError::AlgMismatch { .. }));
}

#[test]
fn claims_preserve_additional_fields() {
    let verifier = ReferenceOidcVerifier::new(mk_jwks(), StubVerifier);
    let mut p = valid_claims_json();
    p["custom_oyatie_pack"] = json!("eu");
    let token = forge_token("k1", p);
    let claims: OidcClaims = verifier
        .verify(&token, &default_cfg(1_700_000_500))
        .expect("ok");
    assert_eq!(
        claims
            .additional
            .get("custom_oyatie_pack")
            .map(|v| v.as_str().unwrap_or("")),
        Some("eu")
    );
}
