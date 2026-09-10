//! Multi-window aggregation: which window's verdict wins, and what
//! provenance the decision carries.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use intelligence_usage_window_kernel::{
    EnforcedDecision, EnforcementError, EnforcementVerdict, UsageEnforcement, UsageWindow,
    UsageWindowKind, VerdictProvenance,
};

/// Every window built here ends at 18_000, so a `now` past that is expired.
fn window(
    kind: UsageWindowKind,
    used_in: u64,
    used_out: u64,
    limit: u8,
    reserve: u8,
) -> UsageWindow {
    let mut w = UsageWindow::new(kind, 0, 18_000, limit, reserve).unwrap();
    w.tokens_in = used_in;
    w.tokens_out = used_out;
    w
}

#[test]
fn check_limits_empty_set_returns_no_windows() {
    let result = UsageEnforcement::check_limits(&[], 100);
    assert_eq!(result, Err(EnforcementError::NoWindows));
}

#[test]
fn check_limits_single_window_within_limit_returns_decision_with_provenance() {
    let w = window(UsageWindowKind::FiveHour, 100, 0, 80, 5);
    let dec = UsageEnforcement::check_limits(&[(w, 1_000)], 100).unwrap();
    assert!(matches!(
        dec.verdict,
        EnforcementVerdict::WithinLimit { .. }
    ));
    assert_eq!(dec.provenance.window_index, 0);
    assert_eq!(dec.provenance.window_kind, UsageWindowKind::FiveHour);
}

#[test]
fn check_limits_picks_reserve_breached_over_within_limit() {
    let w0 = window(UsageWindowKind::FiveHour, 100, 0, 80, 5);
    let w1 = window(UsageWindowKind::OneWeek, 950, 0, 100, 10);
    let dec = UsageEnforcement::check_limits(&[(w0, 1_000), (w1, 1_000)], 100).unwrap();
    assert!(matches!(
        dec.verdict,
        EnforcementVerdict::ReserveBreached { .. }
    ));
    assert_eq!(dec.provenance.window_index, 1);
    assert_eq!(dec.provenance.window_kind, UsageWindowKind::OneWeek);
}

#[test]
fn check_limits_picks_over_usage_limit_over_within_limit() {
    let w0 = window(UsageWindowKind::FiveHour, 50, 0, 80, 5);
    let w1 = window(UsageWindowKind::OneWeek, 850, 0, 80, 5);
    let dec = UsageEnforcement::check_limits(&[(w0, 1_000), (w1, 1_000)], 100).unwrap();
    assert!(matches!(
        dec.verdict,
        EnforcementVerdict::OverUsageLimit { .. }
    ));
    assert_eq!(dec.provenance.window_index, 1);
    assert_eq!(dec.provenance.window_kind, UsageWindowKind::OneWeek);
}

#[test]
fn check_limits_expired_outranks_reserve_breached() {
    let mut w0 = window(UsageWindowKind::OneWeek, 950, 0, 100, 10);
    w0.ends_at_epoch_secs = 999_999;
    let w1 = window(UsageWindowKind::FiveHour, 100, 0, 80, 5);
    let dec = UsageEnforcement::check_limits(&[(w0, 1_000), (w1, 1_000)], 20_000).unwrap();
    assert_eq!(dec.verdict, EnforcementVerdict::WindowExpired);
    assert_eq!(dec.provenance.window_index, 1);
    assert_eq!(dec.provenance.window_kind, UsageWindowKind::FiveHour);
}

#[test]
fn check_limits_all_within_tie_breaks_to_earliest_index() {
    let w0 = window(UsageWindowKind::FiveHour, 10, 0, 80, 5);
    let w1 = window(UsageWindowKind::OneWeek, 20, 0, 80, 5);
    let dec = UsageEnforcement::check_limits(&[(w0, 1_000), (w1, 1_000)], 100).unwrap();
    assert!(matches!(
        dec.verdict,
        EnforcementVerdict::WithinLimit { .. }
    ));
    assert_eq!(dec.provenance.window_index, 0);
    assert_eq!(dec.provenance.window_kind, UsageWindowKind::FiveHour);
}

#[test]
fn check_limits_equal_rank_tie_breaks_to_earliest_index() {
    let w0 = window(UsageWindowKind::FiveHour, 850, 0, 80, 5);
    let w1 = window(UsageWindowKind::OneWeek, 850, 0, 80, 5);
    let dec = UsageEnforcement::check_limits(&[(w0, 1_000), (w1, 1_000)], 100).unwrap();
    assert!(matches!(
        dec.verdict,
        EnforcementVerdict::OverUsageLimit { .. }
    ));
    assert_eq!(dec.provenance.window_index, 0);
    assert_eq!(dec.provenance.window_kind, UsageWindowKind::FiveHour);
}

#[test]
fn check_limits_provenance_records_correct_kind_and_index_for_three_windows() {
    let mut w0 = window(UsageWindowKind::FiveHour, 10, 0, 80, 5);
    w0.ends_at_epoch_secs = 999_999;
    let mut w1 = window(UsageWindowKind::OneWeek, 20, 0, 80, 5);
    w1.ends_at_epoch_secs = 999_999;
    let w2 = window(UsageWindowKind::Project, 10, 0, 80, 5);
    let dec =
        UsageEnforcement::check_limits(&[(w0, 1_000), (w1, 1_000), (w2, 1_000)], 20_000).unwrap();
    assert_eq!(dec.verdict, EnforcementVerdict::WindowExpired);
    assert_eq!(dec.provenance.window_kind, UsageWindowKind::Project);
    assert_eq!(dec.provenance.window_index, 2);
}

#[test]
fn check_limits_short_circuits_on_first_window_error() {
    let w0 = window(UsageWindowKind::FiveHour, 0, 0, 80, 5);
    let w1 = window(UsageWindowKind::OneWeek, 10, 0, 80, 5);
    let result = UsageEnforcement::check_limits(&[(w0, 0), (w1, 1_000)], 100);
    match result {
        Err(EnforcementError::WindowFailed {
            window_kind,
            window_index,
            source,
        }) => {
            assert_eq!(window_kind, UsageWindowKind::FiveHour);
            assert_eq!(window_index, 0);
            assert!(matches!(*source, EnforcementError::InvalidWindow(_)));
        }
        other => panic!("expected WindowFailed, got {other:?}"),
    }
}

#[test]
fn enforcement_verdict_is_copy() {
    let w = window(UsageWindowKind::FiveHour, 100, 50, 80, 10);
    let verdict: EnforcementVerdict = UsageEnforcement::check_limit(&w, 100, 1_000).unwrap();
    let _first_use = verdict;
    let _second_use = verdict; // requires Copy
    let _ = (_first_use, _second_use);
}

#[test]
fn enforced_decision_is_copy_and_eq() {
    let w = window(UsageWindowKind::FiveHour, 100, 0, 80, 5);
    let dec: EnforcedDecision = UsageEnforcement::check_limits(&[(w, 1_000)], 100).unwrap();
    let copy = dec; // requires Copy
    assert_eq!(dec, copy); // requires Eq
}

#[test]
fn verdict_provenance_is_copy_and_eq() {
    let prov = VerdictProvenance {
        window_kind: UsageWindowKind::FiveHour,
        window_index: 0,
    };
    let copy = prov; // requires Copy
    assert_eq!(prov, copy); // requires Eq
}
