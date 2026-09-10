#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use observability_domain::slo::{
    AlertDecision, InvalidSLOObjective, PAGE_BUDGET_CONSUMED_MIN, PAGE_BURN_RATE_THRESHOLD,
    SLOObjective, TICKET_BUDGET_CONSUMED_MIN, TICKET_BURN_RATE_THRESHOLD, classify_burn_rate,
    slo_fields,
};

#[test]
fn slo_objective_new_accepts_three_nines_ratio_and_thirty_day_window() {
    let obj = SLOObjective::new(0.999, 2_592_000).unwrap();
    assert_eq!(obj.target_ratio(), 0.999);
    assert_eq!(obj.window_secs(), 2_592_000);
}

#[test]
fn slo_objective_new_accepts_boundary_ratio_exactly_one() {
    let obj = SLOObjective::new(1.0, 86_400).unwrap();
    assert_eq!(obj.target_ratio(), 1.0);
    assert_eq!(obj.window_secs(), 86_400);
}

#[test]
fn slo_objective_new_accepts_very_small_positive_ratio() {
    let obj = SLOObjective::new(0.001, 86_400).unwrap();
    assert_eq!(obj.target_ratio(), 0.001);
}

#[test]
fn slo_objective_new_rejects_zero_ratio_returns_err() {
    let result = SLOObjective::new(0.0, 86_400);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.reason, "target_ratio must be in (0.0, 1.0]");
}

#[test]
fn slo_objective_new_rejects_ratio_above_one_returns_err() {
    let result = SLOObjective::new(1.001, 86_400);
    assert!(result.is_err());
}

#[test]
fn slo_objective_new_rejects_negative_ratio_returns_err() {
    let result = SLOObjective::new(-0.1, 86_400);
    assert!(result.is_err());
}

#[test]
fn slo_objective_accessors_do_not_mutate_stored_values() {
    let obj = SLOObjective::new(0.95, 604_800).unwrap();
    assert_eq!(obj.target_ratio(), 0.95);
    assert_eq!(obj.window_secs(), 604_800);
    // Repeated calls must return identical values.
    assert_eq!(obj.target_ratio(), 0.95);
    assert_eq!(obj.window_secs(), 604_800);
}

#[test]
fn slo_objective_is_cloneable_and_copy() {
    let obj = SLOObjective::new(0.999, 86_400).unwrap();
    let cloned = obj;
    assert_eq!(cloned.target_ratio(), obj.target_ratio());
    assert_eq!(cloned.window_secs(), obj.window_secs());
}

#[test]
fn invalid_slo_objective_is_publicly_accessible_from_crate_root() {
    // Compilation proves the re-export; value check proves correctness.
    let _err: InvalidSLOObjective = SLOObjective::new(0.0, 0).unwrap_err();
}

#[test]
fn slo_fields_slo_name_wire_value_is_stable() {
    assert_eq!(slo_fields::SLO_NAME, "oyatie.slo.name");
}

#[test]
fn slo_fields_objective_ratio_wire_value_is_stable() {
    assert_eq!(
        slo_fields::SLO_OBJECTIVE_RATIO,
        "oyatie.slo.objective_ratio"
    );
}

#[test]
fn slo_fields_error_budget_remaining_wire_value_is_stable() {
    assert_eq!(
        slo_fields::ERROR_BUDGET_REMAINING,
        "oyatie.slo.error_budget_remaining"
    );
}

#[test]
fn slo_fields_burn_rate_wire_value_is_stable() {
    assert_eq!(slo_fields::BURN_RATE, "oyatie.slo.burn_rate");
}

#[test]
fn page_burn_rate_threshold_is_14_point_4() {
    assert_eq!(PAGE_BURN_RATE_THRESHOLD, 14.4);
}

#[test]
fn page_budget_consumed_min_is_0_point_02() {
    assert_eq!(PAGE_BUDGET_CONSUMED_MIN, 0.02);
}

#[test]
fn ticket_burn_rate_threshold_is_6_point_0() {
    assert_eq!(TICKET_BURN_RATE_THRESHOLD, 6.0);
}

#[test]
fn ticket_budget_consumed_min_is_0_point_05() {
    assert_eq!(TICKET_BUDGET_CONSUMED_MIN, 0.05);
}

#[test]
fn classify_both_windows_above_page_threshold_returns_page() {
    assert_eq!(classify_burn_rate(0.03, 15.0, 15.0), AlertDecision::Page);
}

#[test]
fn classify_fast_below_page_slow_above_does_not_page() {
    assert_ne!(classify_burn_rate(0.03, 10.0, 15.0), AlertDecision::Page);
}

#[test]
fn classify_fast_above_page_slow_below_does_not_page() {
    assert_ne!(classify_burn_rate(0.03, 15.0, 10.0), AlertDecision::Page);
}

#[test]
fn classify_page_budget_not_consumed_enough_returns_none() {
    // 0.01 < PAGE_BUDGET_CONSUMED_MIN (0.02); also below ticket budget min
    assert_eq!(classify_burn_rate(0.01, 15.0, 15.0), AlertDecision::None);
}

#[test]
fn classify_exact_page_boundary_returns_page() {
    assert_eq!(classify_burn_rate(0.02, 14.4, 14.4), AlertDecision::Page);
}

#[test]
fn classify_just_below_page_boundary_fast_falls_through_to_ticket() {
    // fast=14.39 < PAGE threshold → no page
    // fast=14.39 >= TICKET threshold (6.0), slow=14.4 >= 6.0, consumed=0.06 >= 0.05
    assert_eq!(classify_burn_rate(0.06, 14.39, 14.4), AlertDecision::Ticket);
}

#[test]
fn classify_both_windows_above_ticket_threshold_returns_ticket() {
    assert_eq!(classify_burn_rate(0.06, 7.0, 7.0), AlertDecision::Ticket);
}

#[test]
fn classify_ticket_budget_not_consumed_enough_returns_none() {
    assert_eq!(classify_burn_rate(0.04, 7.0, 7.0), AlertDecision::None);
}

#[test]
fn classify_below_all_thresholds_returns_none() {
    assert_eq!(classify_burn_rate(0.50, 1.0, 1.0), AlertDecision::None);
}

#[test]
fn classify_page_wins_over_ticket_when_both_thresholds_exceeded() {
    // fast=15 ≥ 14.4, slow=15 ≥ 14.4, consumed=0.10 ≥ 0.02 (page)
    //              and also ≥ 6.0 / ≥ 0.05 (ticket) — page takes priority
    assert_eq!(classify_burn_rate(0.10, 15.0, 15.0), AlertDecision::Page);
}

#[test]
fn classify_zero_burn_rates_zero_consumption_returns_none() {
    assert_eq!(classify_burn_rate(0.0, 0.0, 0.0), AlertDecision::None);
}

#[test]
fn classify_nominal_burn_rate_returns_none() {
    assert_eq!(classify_burn_rate(0.99, 1.0, 1.0), AlertDecision::None);
}

#[test]
fn alert_decision_variants_are_distinct() {
    assert_ne!(AlertDecision::Page, AlertDecision::Ticket);
    assert_ne!(AlertDecision::Page, AlertDecision::None);
    assert_ne!(AlertDecision::Ticket, AlertDecision::None);
}

#[test]
fn alert_decision_is_copy_and_clone() {
    let d = AlertDecision::Page;
    let copy = d;
    assert_eq!(copy, AlertDecision::Page);
    let cloned = d;
    assert_eq!(cloned, AlertDecision::Page);
}

#[test]
fn alert_decision_debug_format_is_non_empty() {
    assert!(!format!("{:?}", AlertDecision::Page).is_empty());
    assert!(!format!("{:?}", AlertDecision::Ticket).is_empty());
    assert!(!format!("{:?}", AlertDecision::None).is_empty());
}
