//! Black-box tests for `validate_query`: tenant, pack, period window, and limit.

use audit_query_domain::{
    AuditQuery, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, MAX_QUERY_WINDOW_DAYS, QueryDomainError,
    validate_query,
};

fn base_query() -> AuditQuery {
    AuditQuery {
        tenant_id: "tenant-alpha".to_string(),
        ..Default::default()
    }
}

#[test]
fn rejects_empty_tenant_id() {
    let query = AuditQuery {
        tenant_id: String::new(),
        ..Default::default()
    };
    assert_eq!(validate_query(&query), Err(QueryDomainError::EmptyTenantId));
}

#[test]
fn rejects_whitespace_only_tenant_id() {
    let query = AuditQuery {
        tenant_id: "   ".to_string(),
        ..Default::default()
    };
    assert_eq!(validate_query(&query), Err(QueryDomainError::EmptyTenantId));
}

#[test]
fn rejects_invisible_only_tenant_id() {
    let query = AuditQuery {
        tenant_id: "\u{200B}\u{FEFF}".to_string(),
        ..Default::default()
    };
    assert_eq!(validate_query(&query), Err(QueryDomainError::EmptyTenantId));
}

#[test]
fn accepts_valid_tenant_id() {
    let query = base_query();
    let validated = validate_query(&query).expect("valid tenant_id must validate");
    assert_eq!(validated.tenant_id(), "tenant-alpha");
    assert_eq!(validated.effective_limit(), DEFAULT_PAGE_SIZE);
    assert_eq!(validated.offset(), 0);
}

#[test]
fn accepts_absent_pack() {
    let query = base_query();
    let validated = validate_query(&query).expect("no pack filter is valid");
    assert_eq!(validated.pack(), None);
}

#[test]
fn rejects_blank_pack() {
    let query = AuditQuery {
        pack: Some("   ".to_string()),
        ..base_query()
    };
    assert_eq!(validate_query(&query), Err(QueryDomainError::InvalidPack));
}

#[test]
fn accepts_well_formed_pack() {
    let query = AuditQuery {
        pack: Some("pack-eu".to_string()),
        ..base_query()
    };
    let validated = validate_query(&query).expect("well-formed pack must validate");
    assert_eq!(validated.pack(), Some("pack-eu"));
}

#[test]
fn accepts_bare_day_period_as_a_single_day_window() {
    let query = AuditQuery {
        period: Some("2026-01-01".to_string()),
        ..base_query()
    };
    let validated = validate_query(&query).expect("a bare YYYY-MM-DD day must validate");
    let window = validated
        .period_window()
        .expect("period_window must be Some");
    assert_eq!(window.span_days(), 1);
}

#[test]
fn accepts_bare_month_period_as_that_whole_calendar_month() {
    let query = AuditQuery {
        period: Some("2026-02".to_string()), // 2026 is not a leap year: 28 days
        ..base_query()
    };
    let validated = validate_query(&query).expect("a bare YYYY-MM month must validate");
    let window = validated
        .period_window()
        .expect("period_window must be Some");
    assert_eq!(window.span_days(), 28);
}

#[test]
fn accepts_bare_leap_month_period_with_the_correct_span() {
    let query = AuditQuery {
        period: Some("2024-02".to_string()), // 2024 is a leap year: 29 days
        ..base_query()
    };
    let validated = validate_query(&query).expect("a bare leap-year YYYY-MM month must validate");
    let window = validated
        .period_window()
        .expect("period_window must be Some");
    assert_eq!(window.span_days(), 29);
}

#[test]
fn rejects_malformed_period_garbage_string() {
    let query = AuditQuery {
        period: Some("not-a-period".to_string()),
        ..base_query()
    };
    assert!(matches!(
        validate_query(&query),
        Err(QueryDomainError::InvalidPeriodWindow { .. })
    ));
}

#[test]
fn rejects_period_with_wrong_delimiter() {
    let query = AuditQuery {
        period: Some("2026-01-01,2026-01-02".to_string()),
        ..base_query()
    };
    assert!(matches!(
        validate_query(&query),
        Err(QueryDomainError::InvalidPeriodWindow { .. })
    ));
}

#[test]
fn rejects_bare_period_with_invalid_calendar_date() {
    let query = AuditQuery {
        period: Some("2026-02-30".to_string()),
        ..base_query()
    };
    assert!(matches!(
        validate_query(&query),
        Err(QueryDomainError::InvalidPeriodWindow { .. })
    ));
}

#[test]
fn rejects_bare_period_with_invalid_month() {
    let query = AuditQuery {
        period: Some("2026-13".to_string()),
        ..base_query()
    };
    assert!(matches!(
        validate_query(&query),
        Err(QueryDomainError::InvalidPeriodWindow { .. })
    ));
}

#[test]
fn rejects_period_with_invalid_calendar_date() {
    let query = AuditQuery {
        period: Some("2026-02-30/2026-03-01".to_string()),
        ..base_query()
    };
    assert!(matches!(
        validate_query(&query),
        Err(QueryDomainError::InvalidPeriodWindow { .. })
    ));
}

#[test]
fn rejects_period_with_end_before_start() {
    let query = AuditQuery {
        period: Some("2026-03-01/2026-01-01".to_string()),
        ..base_query()
    };
    assert!(matches!(
        validate_query(&query),
        Err(QueryDomainError::InvalidPeriodWindow { .. })
    ));
}

#[test]
fn accepts_single_day_period() {
    let query = AuditQuery {
        period: Some("2026-01-01/2026-01-01".to_string()),
        ..base_query()
    };
    let validated = validate_query(&query).expect("single-day window must validate");
    let window = validated
        .period_window()
        .expect("period_window must be Some");
    assert_eq!(window.span_days(), 1);
}

#[test]
fn accepts_period_exactly_at_max_window() {
    let query = AuditQuery {
        period: Some("2026-01-01/2027-01-01".to_string()),
        ..base_query()
    };
    let validated = validate_query(&query).expect("exactly-max window must be accepted");
    assert_eq!(
        validated.period_window().unwrap().span_days(),
        MAX_QUERY_WINDOW_DAYS
    );
}

#[test]
fn rejects_period_one_day_over_max_window() {
    let query = AuditQuery {
        period: Some("2026-01-01/2027-01-02".to_string()),
        ..base_query()
    };
    assert_eq!(
        validate_query(&query),
        Err(QueryDomainError::WindowTooLarge)
    );
}

#[test]
fn none_limit_falls_back_to_default_page_size() {
    let query = base_query();
    let validated = validate_query(&query).unwrap();
    assert_eq!(validated.effective_limit(), DEFAULT_PAGE_SIZE);
}

#[test]
fn rejects_zero_limit() {
    let query = AuditQuery {
        limit: Some(0),
        ..base_query()
    };
    assert_eq!(validate_query(&query), Err(QueryDomainError::ZeroLimit));
}

#[test]
fn accepts_limit_at_max_page_size() {
    let query = AuditQuery {
        limit: Some(MAX_PAGE_SIZE),
        ..base_query()
    };
    let validated = validate_query(&query).expect("limit at the cap must be accepted");
    assert_eq!(validated.effective_limit(), MAX_PAGE_SIZE);
}

#[test]
fn rejects_limit_over_max_page_size() {
    let query = AuditQuery {
        limit: Some(MAX_PAGE_SIZE + 1),
        ..base_query()
    };
    assert_eq!(
        validate_query(&query),
        Err(QueryDomainError::LimitExceedsMaximum {
            limit: MAX_PAGE_SIZE + 1,
            max: MAX_PAGE_SIZE,
        })
    );
}

#[test]
fn max_page_size_matches_the_published_contract_maximum() {
    assert_eq!(MAX_PAGE_SIZE, 1000);
}

#[test]
fn default_page_size_matches_the_published_contract_default() {
    assert_eq!(DEFAULT_PAGE_SIZE, 100);
}

#[test]
fn a_contract_legal_limit_of_1000_is_accepted() {
    let query = AuditQuery {
        limit: Some(1000),
        ..base_query()
    };
    let validated = validate_query(&query).expect("contract-legal limit=1000 must be accepted");
    assert_eq!(validated.effective_limit(), 1000);
}

#[test]
fn an_omitted_limit_resolves_to_the_contract_default_of_100() {
    let query = base_query();
    let validated = validate_query(&query).unwrap();
    assert_eq!(validated.effective_limit(), 100);
}
