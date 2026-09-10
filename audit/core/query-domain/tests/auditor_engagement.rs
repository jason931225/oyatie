use audit_query_domain::{AuditorEngagement, QueryDomainError, authorize_auditor_engagement};

// 2026-06-01T00:00:00Z, computed independently of the crate's own date math
// (days from 1970-01-01 to 2026-06-01, verified against a standard calendar).
const EXPIRES_AT: &str = "2026-06-01T00:00:00Z";
const EXPIRES_AT_EPOCH_SECONDS: i64 = 20_605 * 86_400; // 2026-06-01 is day 20605.
const ONE_HOUR: i64 = 3_600;

fn engagement() -> AuditorEngagement {
    AuditorEngagement {
        engagement_id: "engagement-2026-q2-audit".to_string(),
        tenant_id: "tenant-alpha".to_string(),
        expires_at: EXPIRES_AT.to_string(),
    }
}

#[test]
fn accepts_engagement_before_expiry_for_matching_tenant() {
    let result = authorize_auditor_engagement(
        &engagement(),
        "tenant-alpha",
        EXPIRES_AT_EPOCH_SECONDS - ONE_HOUR,
    );
    assert_eq!(result, Ok(()));
}

#[test]
fn rejects_expired_engagement() {
    let result = authorize_auditor_engagement(
        &engagement(),
        "tenant-alpha",
        EXPIRES_AT_EPOCH_SECONDS + ONE_HOUR,
    );
    assert_eq!(
        result,
        Err(QueryDomainError::EngagementExpired {
            expires_at: EXPIRES_AT.to_string(),
        })
    );
}

#[test]
fn rejects_engagement_exactly_at_expiry_instant() {
    // The boundary instant itself is not valid: `now >= expires_at` expires.
    let result =
        authorize_auditor_engagement(&engagement(), "tenant-alpha", EXPIRES_AT_EPOCH_SECONDS);
    assert_eq!(
        result,
        Err(QueryDomainError::EngagementExpired {
            expires_at: EXPIRES_AT.to_string(),
        })
    );
}

#[test]
fn rejects_engagement_for_a_different_tenant_than_the_query() {
    let result = authorize_auditor_engagement(
        &engagement(),
        "tenant-beta",
        EXPIRES_AT_EPOCH_SECONDS - ONE_HOUR,
    );
    assert_eq!(
        result,
        Err(QueryDomainError::EngagementTenantMismatch {
            engagement_tenant_id: "tenant-alpha".to_string(),
            query_tenant_id: "tenant-beta".to_string(),
        })
    );
}

#[test]
fn rejects_blank_engagement_id() {
    let mut bad = engagement();
    bad.engagement_id = "   ".to_string();
    let result = authorize_auditor_engagement(&bad, "tenant-alpha", 0);
    assert_eq!(result, Err(QueryDomainError::EmptyEngagementId));
}

#[test]
fn rejects_invisible_only_engagement_id() {
    let mut bad = engagement();
    bad.engagement_id = "\u{200B}".to_string();
    let result = authorize_auditor_engagement(&bad, "tenant-alpha", 0);
    assert_eq!(result, Err(QueryDomainError::EmptyEngagementId));
}

#[test]
fn rejects_malformed_expires_at_format() {
    let mut bad = engagement();
    bad.expires_at = "2026-06-01".to_string(); // date only, no time/Z
    let result = authorize_auditor_engagement(&bad, "tenant-alpha", 0);
    assert_eq!(
        result,
        Err(QueryDomainError::InvalidExpiresAt {
            expires_at: "2026-06-01".to_string(),
        })
    );
}

#[test]
fn rejects_expires_at_with_non_utc_offset() {
    let mut bad = engagement();
    bad.expires_at = "2026-06-01T00:00:00+09:00".to_string();
    let result = authorize_auditor_engagement(&bad, "tenant-alpha", 0);
    assert!(matches!(
        result,
        Err(QueryDomainError::InvalidExpiresAt { .. })
    ));
}

#[test]
fn rejects_blank_tenant_on_both_sides_rather_than_authorizing_a_blank_match() {
    let mut bad = engagement();
    bad.tenant_id = "".to_string();
    let result = authorize_auditor_engagement(&bad, "", EXPIRES_AT_EPOCH_SECONDS - ONE_HOUR);
    assert_eq!(result, Err(QueryDomainError::EmptyTenantId));
}

#[test]
fn rejects_whitespace_only_tenant_on_both_sides() {
    let mut bad = engagement();
    bad.tenant_id = "   ".to_string();
    let result = authorize_auditor_engagement(&bad, "   ", EXPIRES_AT_EPOCH_SECONDS - ONE_HOUR);
    assert_eq!(result, Err(QueryDomainError::EmptyTenantId));
}

#[test]
fn rejects_invisible_only_tenant_on_both_sides() {
    // ZWSP/BOM survive `.trim()`, so a bare equality check on two
    // unvalidated invisible-only strings would compare equal.
    let mut bad = engagement();
    bad.tenant_id = "\u{200B}\u{FEFF}".to_string();
    let result = authorize_auditor_engagement(
        &bad,
        "\u{200B}\u{FEFF}",
        EXPIRES_AT_EPOCH_SECONDS - ONE_HOUR,
    );
    assert_eq!(result, Err(QueryDomainError::EmptyTenantId));
}

#[test]
fn rejects_blank_query_tenant_even_when_engagement_tenant_is_valid() {
    let result =
        authorize_auditor_engagement(&engagement(), "   ", EXPIRES_AT_EPOCH_SECONDS - ONE_HOUR);
    assert_eq!(result, Err(QueryDomainError::EmptyTenantId));
}

#[test]
fn tenant_mismatch_is_reported_before_expiry_is_checked() {
    // Even with an already-expired engagement, a caller querying under the
    // wrong tenant sees the tenant-scope failure, not a leaked expiry detail.
    let result = authorize_auditor_engagement(
        &engagement(),
        "tenant-beta",
        EXPIRES_AT_EPOCH_SECONDS + ONE_HOUR,
    );
    assert_eq!(
        result,
        Err(QueryDomainError::EngagementTenantMismatch {
            engagement_tenant_id: "tenant-alpha".to_string(),
            query_tenant_id: "tenant-beta".to_string(),
        })
    );
}
