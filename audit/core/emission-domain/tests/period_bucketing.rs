//! Coverage for `validate_period_id` and `period_id_from_rfc3339`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use audit_emission_domain::{EmissionDomainError, period_id_from_rfc3339, validate_period_id};

#[test]
fn well_formed_period_id_is_accepted() {
    validate_period_id("2026-02-20").expect("well-formed period id must validate");
}

#[test]
fn leap_day_is_accepted_in_a_leap_year() {
    validate_period_id("2024-02-29").expect("2024 is a leap year");
}

#[test]
fn leap_day_is_rejected_in_a_non_leap_year() {
    let err = validate_period_id("2026-02-29").unwrap_err();
    assert_eq!(
        err,
        EmissionDomainError::MalformedPeriod {
            period: "2026-02-29".to_string()
        }
    );
}

#[test]
fn century_non_leap_year_rejects_feb_29() {
    let err = validate_period_id("1900-02-29").unwrap_err();
    assert!(matches!(err, EmissionDomainError::MalformedPeriod { .. }));
}

#[test]
fn quad_century_year_accepts_feb_29() {
    validate_period_id("2000-02-29").expect("2000 is a leap year");
}

#[test]
fn empty_period_id_is_rejected() {
    let err = validate_period_id("").unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyPeriod);
}

#[test]
fn whitespace_only_period_id_is_rejected() {
    let err = validate_period_id("   ").unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyPeriod);
}

#[test]
fn wrong_separator_is_rejected() {
    let err = validate_period_id("2026/02/20").unwrap_err();
    assert_eq!(
        err,
        EmissionDomainError::MalformedPeriod {
            period: "2026/02/20".to_string()
        }
    );
}

#[test]
fn wrong_length_is_rejected() {
    let err = validate_period_id("20260220").unwrap_err();
    assert!(matches!(err, EmissionDomainError::MalformedPeriod { .. }));
}

#[test]
fn non_digit_characters_are_rejected() {
    let err = validate_period_id("202X-02-20").unwrap_err();
    assert!(matches!(err, EmissionDomainError::MalformedPeriod { .. }));
}

#[test]
fn month_zero_is_rejected() {
    let err = validate_period_id("2026-00-15").unwrap_err();
    assert!(matches!(err, EmissionDomainError::MalformedPeriod { .. }));
}

#[test]
fn month_thirteen_is_rejected() {
    let err = validate_period_id("2026-13-01").unwrap_err();
    assert!(matches!(err, EmissionDomainError::MalformedPeriod { .. }));
}

#[test]
fn day_zero_is_rejected() {
    let err = validate_period_id("2026-01-00").unwrap_err();
    assert!(matches!(err, EmissionDomainError::MalformedPeriod { .. }));
}

#[test]
fn day_thirty_one_in_a_thirty_day_month_is_rejected() {
    let err = validate_period_id("2026-04-31").unwrap_err();
    assert!(matches!(err, EmissionDomainError::MalformedPeriod { .. }));
}

#[test]
fn day_thirty_in_april_is_accepted() {
    validate_period_id("2026-04-30").expect("April has 30 days");
}

#[test]
fn period_start_instant_derives_the_period() {
    assert_eq!(
        period_id_from_rfc3339("2026-02-20T00:00:00Z").expect("valid timestamp"),
        "2026-02-20"
    );
}

#[test]
fn period_end_instant_derives_the_same_period() {
    assert_eq!(
        period_id_from_rfc3339("2026-02-20T23:59:59Z").expect("valid timestamp"),
        "2026-02-20"
    );
}

#[test]
fn instant_one_second_past_period_end_derives_the_next_period() {
    assert_eq!(
        period_id_from_rfc3339("2026-02-21T00:00:00Z").expect("valid timestamp"),
        "2026-02-21"
    );
}

#[test]
fn fractional_seconds_are_accepted() {
    assert_eq!(
        period_id_from_rfc3339("2026-02-20T12:30:00.123456Z").expect("valid timestamp"),
        "2026-02-20"
    );
}

#[test]
fn positive_offset_within_the_same_utc_day_is_accepted() {
    assert_eq!(
        period_id_from_rfc3339("2026-02-20T23:59:59+09:00").expect("valid timestamp"),
        "2026-02-20"
    );
}

#[test]
fn negative_offset_within_the_same_utc_day_is_accepted() {
    assert_eq!(
        period_id_from_rfc3339("2026-02-20T00:00:00-05:00").expect("valid timestamp"),
        "2026-02-20"
    );
}

#[test]
fn positive_offset_that_crosses_midnight_shifts_to_the_previous_utc_day() {
    assert_eq!(
        period_id_from_rfc3339("2026-02-21T00:30:00+09:00").expect("valid timestamp"),
        "2026-02-20"
    );
}

#[test]
fn negative_offset_that_crosses_midnight_shifts_to_the_next_utc_day() {
    assert_eq!(
        period_id_from_rfc3339("2026-02-20T21:00:00-05:00").expect("valid timestamp"),
        "2026-02-21"
    );
}

#[test]
fn same_instant_different_offset_spellings_yield_the_same_period_id() {
    let utc_spelling = period_id_from_rfc3339("2026-02-19T15:00:00Z").expect("valid timestamp");
    let plus_nine_spelling =
        period_id_from_rfc3339("2026-02-20T00:00:00+09:00").expect("valid timestamp");
    let minus_five_spelling =
        period_id_from_rfc3339("2026-02-19T10:00:00-05:00").expect("valid timestamp");
    assert_eq!(utc_spelling, plus_nine_spelling);
    assert_eq!(utc_spelling, minus_five_spelling);
    assert_eq!(utc_spelling, "2026-02-19");
}

#[test]
fn positive_offset_crossing_a_year_boundary_shifts_the_period_id() {
    assert_eq!(
        period_id_from_rfc3339("2027-01-01T00:30:00+09:00").expect("valid timestamp"),
        "2026-12-31"
    );
}

#[test]
fn negative_offset_crossing_a_year_boundary_shifts_the_period_id() {
    assert_eq!(
        period_id_from_rfc3339("2026-12-31T20:00:00-05:00").expect("valid timestamp"),
        "2027-01-01"
    );
}

#[test]
fn negative_offset_crossing_a_month_boundary_shifts_the_period_id() {
    assert_eq!(
        period_id_from_rfc3339("2026-02-28T23:00:00-05:00").expect("valid timestamp"),
        "2026-03-01"
    );
}

#[test]
fn leap_second_value_sixty_is_tolerated() {
    period_id_from_rfc3339("2026-02-20T23:59:60Z").expect("leap second must be tolerated");
}

#[test]
fn missing_time_separator_is_rejected() {
    let err = period_id_from_rfc3339("2026-02-20 00:00:00Z").unwrap_err();
    assert!(matches!(
        err,
        EmissionDomainError::MalformedTimestamp { .. }
    ));
}

#[test]
fn malformed_date_component_is_rejected() {
    let err = period_id_from_rfc3339("2026-13-01T00:00:00Z").unwrap_err();
    assert!(matches!(
        err,
        EmissionDomainError::MalformedTimestamp { .. }
    ));
}

#[test]
fn missing_offset_is_rejected() {
    let err = period_id_from_rfc3339("2026-02-20T00:00:00").unwrap_err();
    assert!(matches!(
        err,
        EmissionDomainError::MalformedTimestamp { .. }
    ));
}

#[test]
fn out_of_range_hour_is_rejected() {
    let err = period_id_from_rfc3339("2026-02-20T24:00:00Z").unwrap_err();
    assert!(matches!(
        err,
        EmissionDomainError::MalformedTimestamp { .. }
    ));
}

#[test]
fn out_of_range_offset_minute_is_rejected() {
    let err = period_id_from_rfc3339("2026-02-20T00:00:00+05:60").unwrap_err();
    assert!(matches!(
        err,
        EmissionDomainError::MalformedTimestamp { .. }
    ));
}

#[test]
fn truncated_timestamp_is_rejected() {
    let err = period_id_from_rfc3339("2026-02-20T00:00").unwrap_err();
    assert!(matches!(
        err,
        EmissionDomainError::MalformedTimestamp { .. }
    ));
}

#[test]
fn empty_timestamp_is_rejected() {
    let err = period_id_from_rfc3339("").unwrap_err();
    assert_eq!(
        err,
        EmissionDomainError::MalformedTimestamp {
            timestamp: String::new()
        }
    );
}

#[test]
fn multibyte_input_does_not_panic_and_is_rejected() {
    // Regression guard: byte-offset slicing on malformed multi-byte input
    // must not panic on a non-char-boundary; it must return a clean error.
    let err = period_id_from_rfc3339("2026-02-2😀T00:00:00Z").unwrap_err();
    assert!(matches!(
        err,
        EmissionDomainError::MalformedTimestamp { .. }
    ));
}

#[test]
fn multibyte_period_id_does_not_panic_and_is_rejected() {
    let err = validate_period_id("2026-0😀-20").unwrap_err();
    assert!(matches!(err, EmissionDomainError::MalformedPeriod { .. }));
}
