//! Public-GA FinOps surface: the tenant-visible projection of a `CostReport`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use crate::{CostReport, FinopsError, ReportPeriod};

const MICROS_PER_CENT: u128 = 10_000;

/// Schema version for this public summary shape.
pub const FINOPS_PUBLIC_SCHEMA_VERSION: u32 = 1;

/// Tenant-visible cost summary — no sub-cent precision exposed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicCostSummary {
    pub report_id: String,
    pub period: ReportPeriod,
    pub period_start_unix_ms: u64,
    pub period_end_unix_ms: u64,
    pub spend_cents: u128,
    pub schema_version: u32,
}

/// Convert a validated `CostReport` into a `PublicCostSummary`.
///
/// Returns `Err` if the report fails basic validation (empty id, inverted
/// period); callers should run `validate_report` first or use this as the
/// single validation + projection step.
pub fn report_public(r: &CostReport) -> Result<PublicCostSummary, FinopsError> {
    if r.report_id.is_empty() {
        return Err(FinopsError::EmptyReportId);
    }
    if r.period_end_unix_ms <= r.period_start_unix_ms {
        return Err(FinopsError::InvalidPeriod);
    }
    Ok(PublicCostSummary {
        report_id: r.report_id.clone(),
        period: r.period,
        period_start_unix_ms: r.period_start_unix_ms,
        period_end_unix_ms: r.period_end_unix_ms,
        spend_cents: r.total_spend_micros / MICROS_PER_CENT,
        schema_version: FINOPS_PUBLIC_SCHEMA_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReportPeriod;

    fn report(id: &str, start: u64, end: u64, micros: u128) -> CostReport {
        CostReport {
            report_id: id.into(),
            period: ReportPeriod::Monthly,
            period_start_unix_ms: start,
            period_end_unix_ms: end,
            total_spend_micros: micros,
        }
    }

    #[test]
    fn report_public_valid_produces_summary() {
        let r = report("r1", 1_000, 2_000, 1_234_567_890);
        let s = report_public(&r).unwrap();
        assert_eq!(s.report_id, "r1");
        assert_eq!(s.period, ReportPeriod::Monthly);
        assert_eq!(s.period_start_unix_ms, 1_000);
        assert_eq!(s.period_end_unix_ms, 2_000);
        assert_eq!(s.spend_cents, 123_456);
        assert_eq!(s.schema_version, FINOPS_PUBLIC_SCHEMA_VERSION);
    }

    #[test]
    fn report_public_empty_id_rejected() {
        let r = report("", 1_000, 2_000, 500);
        assert!(matches!(report_public(&r), Err(FinopsError::EmptyReportId)));
    }

    #[test]
    fn report_public_inverted_period_rejected() {
        let r = report("r1", 2_000, 1_000, 500);
        assert!(matches!(report_public(&r), Err(FinopsError::InvalidPeriod)));
    }

    #[test]
    fn report_public_equal_period_rejected() {
        let r = report("r1", 1_000, 1_000, 500);
        assert!(matches!(report_public(&r), Err(FinopsError::InvalidPeriod)));
    }

    #[test]
    fn report_public_zero_spend_allowed() {
        let r = report("r1", 1_000, 2_000, 0);
        let s = report_public(&r).unwrap();
        assert_eq!(s.spend_cents, 0);
    }

    #[test]
    fn report_public_sub_cent_truncated() {
        let r = report("r1", 1_000, 2_000, 9_999);
        let s = report_public(&r).unwrap();
        assert_eq!(s.spend_cents, 0);
    }

    #[test]
    fn schema_version_is_one() {
        assert_eq!(FINOPS_PUBLIC_SCHEMA_VERSION, 1);
    }
}
