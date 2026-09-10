#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use payroll_run_domain::{
    AnomalyFlag, MoneyAmount, PayeeVarianceTotal, PayrollDomainError, PayrollVarianceInput,
    evaluate_payroll_variance,
};

// Sentinel BPS value used for dropped-payee lines (mirrors the crate constant).
const DROPPED_PAYEE_SENTINEL_BPS: i64 = -10_000;

fn krw(amount_minor: i64) -> MoneyAmount {
    MoneyAmount {
        amount_minor,
        currency: "KRW".to_owned(),
    }
}

fn total(payee_id: &str, amount_minor: i64) -> PayeeVarianceTotal {
    PayeeVarianceTotal {
        payee_id: payee_id.to_owned(),
        net_amount: krw(amount_minor),
    }
}

fn base_input() -> PayrollVarianceInput {
    PayrollVarianceInput {
        run_id: "prun_kr_2026_01".to_owned(),
        current_period_totals: vec![
            total("payee_001", 1_010_000), // +1% vs prior
            total("payee_002", 2_020_000), // +1% vs prior
        ],
        prior_period_totals: vec![total("payee_001", 1_000_000), total("payee_002", 2_000_000)],
        variance_tolerance_bps: 500,
        rulepack_ref: "rulepack/kr-payroll-2026".to_owned(),
        rulepack_effective_date: "2026-01-01".to_owned(),
        evidence_refs: vec![
            "audit/payroll/variance/001".to_owned(),
            "audit/payroll/variance/002".to_owned(),
        ],
        evaluated_at: 1_700_000_000,
    }
}

#[test]
fn happy_path_within_tolerance() {
    let verdict = evaluate_payroll_variance(base_input()).expect("variance verdict");

    assert!(verdict.gate_passed.value, "gate_passed should be true");
    assert!(
        verdict.anomaly_flags.value.is_empty(),
        "no anomaly flags expected"
    );
    assert_eq!(verdict.schema_version.value, 1);

    let line0 = &verdict.lines.value[0];
    assert_eq!(line0.variance_bps.value, 100);
    assert!(!line0.anomaly.value);

    let line1 = &verdict.lines.value[1];
    assert_eq!(line1.variance_bps.value, 100);
    assert!(!line1.anomaly.value);

    let digest = &verdict.evidence_digest.value.value;
    assert!(digest.starts_with("sha256:"), "digest prefix");
    assert_eq!(digest.len(), "sha256:".len() + 64);

    assert_eq!(verdict.run_id.value.value, "prun_kr_2026_01");
}

#[test]
fn over_tolerance_swing() {
    let mut input = base_input();
    input.current_period_totals[0] = total("payee_001", 1_200_000);

    let verdict = evaluate_payroll_variance(input).expect("verdict");

    assert!(!verdict.gate_passed.value, "gate_passed should be false");
    let flags = &verdict.anomaly_flags.value;
    assert!(
        flags.iter().any(|f| matches!(
            f,
            AnomalyFlag::OverToleranceSwing { payee_id } if payee_id.value == "payee_001"
        )),
        "OverToleranceSwing flag for payee_001 expected"
    );
}

#[test]
fn sign_flip() {
    let mut input = base_input();
    input.current_period_totals[1] = total("payee_002", -500_000);

    let verdict = evaluate_payroll_variance(input).expect("verdict");

    assert!(!verdict.gate_passed.value, "gate_passed should be false");
    let flags = &verdict.anomaly_flags.value;
    assert!(
        flags.iter().any(|f| matches!(
            f,
            AnomalyFlag::SignFlip { payee_id } if payee_id.value == "payee_002"
        )),
        "SignFlip flag for payee_002 expected"
    );
}

#[test]
fn dropped_payee() {
    let mut input = base_input();
    input
        .current_period_totals
        .retain(|t| t.payee_id != "payee_002");

    let verdict = evaluate_payroll_variance(input).expect("verdict");

    assert!(!verdict.gate_passed.value, "gate_passed should be false");
    let flags = &verdict.anomaly_flags.value;
    assert!(
        flags.iter().any(|f| matches!(
            f,
            AnomalyFlag::DroppedPayee { payee_id } if payee_id.value == "payee_002"
        )),
        "DroppedPayee flag for payee_002 expected"
    );
}

#[test]
fn missing_tolerance() {
    let mut input = base_input();
    input.variance_tolerance_bps = 0;

    assert_eq!(
        evaluate_payroll_variance(input),
        Err(PayrollDomainError::VarianceToleranceRequired)
    );
}

#[test]
fn invalid_evidence_ref() {
    let mut input = base_input();
    input.evidence_refs[0] = "audit/payroll/../secret".to_owned();

    assert_eq!(
        evaluate_payroll_variance(input),
        Err(PayrollDomainError::InvalidEvidenceRef)
    );
}

#[test]
fn new_payee_without_prior_baseline_returns_missing_baseline_error() {
    let mut input = base_input();
    input
        .current_period_totals
        .push(total("payee_003", 500_000));

    assert_eq!(
        evaluate_payroll_variance(input),
        Err(PayrollDomainError::MissingBaselineForPayee),
        "a current payee with no prior baseline must return MissingBaselineForPayee"
    );
}

#[test]
fn run_net_variance_bps_equals_sum_of_per_payee_bps() {
    let verdict = evaluate_payroll_variance(base_input()).expect("variance verdict");

    assert_eq!(
        verdict.run_net_variance_bps.value, 200,
        "run_net_variance_bps must be the sum of per-payee bps"
    );
}

#[test]
fn dropped_payee_synthetic_line_carries_sentinel_bps_and_anomaly_true() {
    let mut input = base_input();
    input
        .current_period_totals
        .retain(|t| t.payee_id != "payee_002");

    let verdict = evaluate_payroll_variance(input).expect("verdict");

    let dropped_line = verdict
        .lines
        .value
        .iter()
        .find(|l| l.payee_id.value.value == "payee_002")
        .expect("synthetic line for dropped payee_002 must be present");

    assert_eq!(
        dropped_line.variance_bps.value, DROPPED_PAYEE_SENTINEL_BPS,
        "dropped payee synthetic line must use the sentinel BPS value"
    );
    assert!(
        dropped_line.anomaly.value,
        "dropped payee synthetic line must have anomaly=true"
    );
    assert_eq!(
        dropped_line.current_amount.value.amount_minor, 0,
        "dropped payee synthetic line must have zero current amount"
    );

    assert_eq!(
        verdict.run_net_variance_bps.value,
        100 + DROPPED_PAYEE_SENTINEL_BPS,
        "run_net_variance_bps must include the dropped-payee sentinel"
    );
}

#[test]
fn sign_flip_and_over_tolerance_both_flagged_on_same_payee() {
    let mut input = base_input();
    input.current_period_totals[0] = total("payee_001", -2_000_000);

    let verdict = evaluate_payroll_variance(input).expect("verdict");

    assert!(!verdict.gate_passed.value, "gate must be closed");
    let flags = &verdict.anomaly_flags.value;

    assert!(
        flags.iter().any(|f| matches!(
            f,
            AnomalyFlag::OverToleranceSwing { payee_id } if payee_id.value == "payee_001"
        )),
        "OverToleranceSwing flag for payee_001 must be present"
    );
    assert!(
        flags.iter().any(|f| matches!(
            f,
            AnomalyFlag::SignFlip { payee_id } if payee_id.value == "payee_001"
        )),
        "SignFlip flag for payee_001 must be present alongside OverToleranceSwing"
    );
}

#[test]
fn evaluated_at_zero_returns_invalid_received_at() {
    let mut input = base_input();
    input.evaluated_at = 0;

    assert_eq!(
        evaluate_payroll_variance(input),
        Err(PayrollDomainError::InvalidReceivedAt),
        "evaluated_at=0 must return InvalidReceivedAt"
    );
}

#[test]
fn invalid_run_id_prefix_returns_invalid_run_id() {
    let mut input = base_input();
    input.run_id = "run_kr_2026_01".to_owned(); // missing "p"

    assert_eq!(
        evaluate_payroll_variance(input),
        Err(PayrollDomainError::InvalidRunId),
        "run_id without prun_ prefix must return InvalidRunId"
    );
}

#[test]
fn invalid_rulepack_effective_date_is_rejected() {
    let mut input = base_input();
    input.rulepack_effective_date = "2026-13-01".to_owned(); // month 13 is invalid

    assert_eq!(
        evaluate_payroll_variance(input),
        Err(PayrollDomainError::InvalidRulepackEffectiveDate),
        "rulepack_effective_date with month 13 must return InvalidRulepackEffectiveDate"
    );
}

#[test]
fn invalid_rulepack_ref_prefix_is_rejected() {
    let mut input = base_input();
    input.rulepack_ref = "kr-payroll-2026".to_owned(); // missing rulepack/ prefix

    assert_eq!(
        evaluate_payroll_variance(input),
        Err(PayrollDomainError::InvalidRulepackRef),
        "rulepack_ref without rulepack/ prefix must return InvalidRulepackRef"
    );
}
