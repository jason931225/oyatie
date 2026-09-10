#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use data_boundary_kernel::Purpose;
use iam_identity_domain::{
    CredentialRequest, CredentialRequestKind, CredentialStatus, Principal, RevocationError,
    RevocationLedger, RevocationReason, UnknownRevocationReason, issue_credential, issue_token,
    token_fingerprint,
};

#[test]
fn revocation_reason_wire_round_trip_via_public_re_export() {
    for variant in RevocationReason::ALL {
        let wire = variant.as_str();
        let parsed = RevocationReason::from_wire(wire).expect("all wire strings must round-trip");
        assert_eq!(parsed, variant);
    }
}

#[test]
fn unknown_revocation_reason_rejects_unknown_and_empty() {
    assert!(RevocationReason::from_wire("unknown_value").is_err());
    assert!(RevocationReason::from_wire("").is_err());
    let err = UnknownRevocationReason("bad".to_string());
    let msg = err.to_string();
    assert!(msg.contains("bad"));
    assert!(msg.contains("compromised"));
}

#[test]
fn credential_status_is_valid_only_for_active() {
    assert!(CredentialStatus::Active.is_valid());
    assert!(!CredentialStatus::Expired.is_valid());
    for reason in RevocationReason::ALL {
        assert!(!CredentialStatus::Revoked(reason).is_valid());
    }
}

#[test]
fn token_path_full_deny_precedence_proof() {
    let token = issue_token(
        "ten_alpha".into(),
        "usr_admin".into(),
        Purpose::CapabilityInvocation,
        900,
        1_000,
    )
    .expect("valid token");

    let fp = token_fingerprint(&token);
    assert!(
        fp.starts_with("tok1:"),
        "fingerprint must have tok1: prefix"
    );

    let mut ledger = RevocationLedger::new("ten_alpha").unwrap();

    assert_eq!(
        ledger.evaluate_token(&token, 1_899).unwrap(),
        CredentialStatus::Active
    );

    // Exactly at expiry, not past it: the boundary is exclusive.
    assert_eq!(
        ledger.evaluate_token(&token, 1_900).unwrap(),
        CredentialStatus::Expired
    );

    ledger
        .revoke(fp.clone(), RevocationReason::Compromised)
        .expect("first revoke must succeed");

    // Revocation outranks a still-valid TTL.
    let status = ledger.evaluate_token(&token, 1_500).unwrap();
    assert_eq!(
        status,
        CredentialStatus::Revoked(RevocationReason::Compromised)
    );
    assert!(!status.is_valid(), "revoked credential must not be valid");

    // Revocation outranks expiry too: the status stays Revoked, not Expired.
    let status = ledger.evaluate_token(&token, 2_000).unwrap();
    assert_eq!(
        status,
        CredentialStatus::Revoked(RevocationReason::Compromised)
    );
    assert!(!status.is_valid());
}

#[test]
fn token_fingerprint_is_deterministic_and_tok1_prefixed() {
    let token = issue_token(
        "ten_alpha".into(),
        "usr_admin".into(),
        Purpose::CapabilityInvocation,
        900,
        1_000,
    )
    .unwrap();
    let fp1 = token_fingerprint(&token);
    let fp2 = token_fingerprint(&token);
    assert_eq!(fp1, fp2, "must be deterministic");
    assert!(fp1.starts_with("tok1:"));
    assert!(
        !fp1.starts_with("sts1:"),
        "tok1: must be distinct from sts1:"
    );
}

fn make_sts(tenant: &str, issued: u64, ttl: u64) -> iam_identity_domain::StsCredential {
    let principal = Principal::human(tenant.into(), "usr_admin".into()).unwrap();
    issue_credential(CredentialRequest {
        principal,
        kind: CredentialRequestKind::Sts,
        purpose: Purpose::CapabilityInvocation,
        scopes: vec!["cloud.iam.read".into()],
        ttl_seconds: ttl,
        issued_at_epoch_seconds: issued,
    })
    .expect("valid credential")
}

#[test]
fn sts_path_revoked_but_live_reports_revoked_and_is_valid_false() {
    let cred = make_sts("ten_alpha", 1_000, 900);
    let fp = cred.token_fingerprint.value.clone();
    assert!(
        fp.starts_with("sts1:"),
        "STS fingerprint must have sts1: prefix"
    );

    let mut ledger = RevocationLedger::new("ten_alpha").unwrap();
    ledger
        .revoke(fp, RevocationReason::PrincipalDeprovisioned)
        .expect("revoke must succeed");

    // Revocation outranks a still-valid TTL.
    let status = ledger.evaluate_sts(&cred, 1_500).unwrap();
    assert_eq!(
        status,
        CredentialStatus::Revoked(RevocationReason::PrincipalDeprovisioned)
    );
    assert!(!status.is_valid(), "is_valid must be false for Revoked");
}

#[test]
fn sts_path_not_revoked_active_within_ttl() {
    let cred = make_sts("ten_alpha", 1_000, 900);
    let ledger = RevocationLedger::new("ten_alpha").unwrap();
    assert_eq!(
        ledger.evaluate_sts(&cred, 1_899).unwrap(),
        CredentialStatus::Active
    );
}

#[test]
fn sts_path_expired_at_boundary() {
    let cred = make_sts("ten_alpha", 1_000, 900);
    let ledger = RevocationLedger::new("ten_alpha").unwrap();
    assert_eq!(
        ledger
            .evaluate_sts(&cred, cred.expires_at_epoch_seconds.value)
            .unwrap(),
        CredentialStatus::Expired
    );
}

#[test]
fn cross_tenant_token_is_tenant_mismatch_fail_closed() {
    let token = issue_token(
        "ten_alpha".into(),
        "usr_admin".into(),
        Purpose::CapabilityInvocation,
        900,
        1_000,
    )
    .unwrap();
    let ledger = RevocationLedger::new("ten_beta").unwrap();
    assert_eq!(
        ledger.evaluate_token(&token, 1_500),
        Err(RevocationError::TenantMismatch)
    );
}

#[test]
fn cross_tenant_sts_is_tenant_mismatch_fail_closed() {
    let cred = make_sts("ten_alpha", 1_000, 900);
    let ledger = RevocationLedger::new("ten_beta").unwrap();
    assert_eq!(
        ledger.evaluate_sts(&cred, 1_500),
        Err(RevocationError::TenantMismatch)
    );
}

#[test]
fn ledger_same_reason_revoke_is_idempotent() {
    let mut ledger = RevocationLedger::new("ten_alpha").unwrap();
    ledger
        .revoke("fp_abc", RevocationReason::Superseded)
        .unwrap();
    ledger
        .revoke("fp_abc", RevocationReason::Superseded)
        .expect("same-reason re-revoke must succeed");
    assert_eq!(ledger.len(), 1);
}

#[test]
fn ledger_conflicting_reason_returns_error_and_preserves_original() {
    let mut ledger = RevocationLedger::new("ten_alpha").unwrap();
    ledger
        .revoke("fp_abc", RevocationReason::Compromised)
        .unwrap();
    let err = ledger
        .revoke("fp_abc", RevocationReason::PolicyViolation)
        .unwrap_err();
    assert_eq!(err, RevocationError::ConflictingRevocation);
    assert_eq!(
        ledger.reason_for("fp_abc"),
        Some(RevocationReason::Compromised),
        "original reason must be preserved"
    );
}

#[test]
fn ledger_empty_fingerprint_returns_empty_fingerprint_error() {
    let mut ledger = RevocationLedger::new("ten_alpha").unwrap();
    assert_eq!(
        ledger.revoke("", RevocationReason::Compromised),
        Err(RevocationError::EmptyFingerprint)
    );
    assert_eq!(
        ledger.revoke("   ", RevocationReason::Compromised),
        Err(RevocationError::EmptyFingerprint)
    );
    assert!(ledger.is_empty());
}
