//! Payroll period, pay-cycle and payslip-lifecycle types.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// Granularity of a payroll computation period.
///
/// Encodes the KR-standard payroll cadences: most salaried employees are
/// `Monthly`; hourly/daily workers may run `Weekly` or `Daily` sub-cycles.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PayrollPeriod {
    Daily,
    Weekly,
    BiWeekly,
    Monthly,
    Quarterly,
}

impl PayrollPeriod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::BiWeekly => "bi_weekly",
            Self::Monthly => "monthly",
            Self::Quarterly => "quarterly",
        }
    }

    /// Parse from wire string; returns `Err(UnknownPayrollPeriod)` on mismatch.
    pub fn from_wire(s: &str) -> Result<Self, UnknownPayrollPeriod> {
        match s {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "bi_weekly" => Ok(Self::BiWeekly),
            "monthly" => Ok(Self::Monthly),
            "quarterly" => Ok(Self::Quarterly),
            other => Err(UnknownPayrollPeriod(other.to_owned())),
        }
    }

    /// Returns `true` if NTS 원천징수 requires monthly filing for this period.
    ///
    /// Under 소득세법 §128, withholding must be remitted by the 10th of the
    /// following month; this helper encodes which periods aggregate to monthly.
    pub fn requires_monthly_withholding_filing(self) -> bool {
        matches!(
            self,
            Self::Daily | Self::Weekly | Self::BiWeekly | Self::Monthly
        )
    }
}

/// Pay cycle frequency — how often the payroll *run* is scheduled.
///
/// Distinct from `PayrollPeriod` (which is about the *computation* window):
/// a `Monthly` cycle may still compute `Daily` sub-periods for hourly workers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PayCycleKind {
    Weekly,
    BiWeekly,
    SemiMonthly,
    Monthly,
}

impl PayCycleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Weekly => "weekly",
            Self::BiWeekly => "bi_weekly",
            Self::SemiMonthly => "semi_monthly",
            Self::Monthly => "monthly",
        }
    }

    /// Approximate number of runs per calendar year (used for annualisation).
    pub fn runs_per_year(self) -> u32 {
        match self {
            Self::Weekly => 52,
            Self::BiWeekly => 26,
            Self::SemiMonthly => 24,
            Self::Monthly => 12,
        }
    }
}

/// Payslip lifecycle state.
///
/// FSM: `Draft` → `Approved` → `Dispatched`; `Draft` → `Voided`.
/// Once `Dispatched` or `Voided` the payslip is immutable (근로기준법 §48).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PayslipStatus {
    Draft,
    Approved,
    Dispatched,
    Voided,
}

impl PayslipStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Approved => "approved",
            Self::Dispatched => "dispatched",
            Self::Voided => "voided",
        }
    }

    /// Returns `true` if the payslip may still be modified.
    ///
    /// Immutability begins only at the terminal states `Dispatched` and
    /// `Voided` (근로기준법 §48); `Approved` records may still receive
    /// pre-dispatch corrections.
    pub fn is_mutable(self) -> bool {
        matches!(self, Self::Draft | Self::Approved)
    }

    /// Returns `true` if the payslip has been delivered to the employee.
    ///
    /// Once dispatched the employer must retain the record for 3 years
    /// (근로기준법 §42 서류 보존 의무).
    pub fn is_dispatched(self) -> bool {
        matches!(self, Self::Dispatched)
    }
}

/// Error returned when a wire string cannot be mapped to [`PayrollPeriod`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownPayrollPeriod(pub String);

impl UnknownPayrollPeriod {
    pub fn message(&self) -> String {
        format!("unknown payroll period: {:?}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PayrollPeriod ────────────────────────────────────────────────────────

    #[test]
    fn payroll_period_wire_strings_are_pinned() {
        assert_eq!(PayrollPeriod::Daily.as_str(), "daily");
        assert_eq!(PayrollPeriod::Weekly.as_str(), "weekly");
        assert_eq!(PayrollPeriod::BiWeekly.as_str(), "bi_weekly");
        assert_eq!(PayrollPeriod::Monthly.as_str(), "monthly");
        assert_eq!(PayrollPeriod::Quarterly.as_str(), "quarterly");
    }

    #[test]
    fn payroll_period_round_trips_all_variants() {
        for variant in [
            PayrollPeriod::Daily,
            PayrollPeriod::Weekly,
            PayrollPeriod::BiWeekly,
            PayrollPeriod::Monthly,
            PayrollPeriod::Quarterly,
        ] {
            assert_eq!(PayrollPeriod::from_wire(variant.as_str()).unwrap(), variant);
        }
    }

    #[test]
    fn payroll_period_unknown_wire_string_returns_err() {
        let err = PayrollPeriod::from_wire("fortnightly").unwrap_err();
        assert_eq!(err.0, "fortnightly");
        assert!(err.message().contains("fortnightly"));
    }

    #[test]
    fn monthly_withholding_filing_required_for_sub_monthly_periods() {
        assert!(PayrollPeriod::Daily.requires_monthly_withholding_filing());
        assert!(PayrollPeriod::Weekly.requires_monthly_withholding_filing());
        assert!(PayrollPeriod::BiWeekly.requires_monthly_withholding_filing());
        assert!(PayrollPeriod::Monthly.requires_monthly_withholding_filing());
        assert!(!PayrollPeriod::Quarterly.requires_monthly_withholding_filing());
    }

    // ── PayCycleKind ─────────────────────────────────────────────────────────

    #[test]
    fn pay_cycle_kind_wire_strings_are_pinned() {
        assert_eq!(PayCycleKind::Weekly.as_str(), "weekly");
        assert_eq!(PayCycleKind::BiWeekly.as_str(), "bi_weekly");
        assert_eq!(PayCycleKind::SemiMonthly.as_str(), "semi_monthly");
        assert_eq!(PayCycleKind::Monthly.as_str(), "monthly");
    }

    #[test]
    fn runs_per_year_matches_calendar_cadence() {
        assert_eq!(PayCycleKind::Monthly.runs_per_year(), 12);
        assert_eq!(PayCycleKind::Weekly.runs_per_year(), 52);
        assert_eq!(PayCycleKind::BiWeekly.runs_per_year(), 26);
        assert_eq!(PayCycleKind::SemiMonthly.runs_per_year(), 24);
    }

    // ── PayslipStatus ────────────────────────────────────────────────────────

    #[test]
    fn payslip_status_wire_strings_are_pinned() {
        assert_eq!(PayslipStatus::Draft.as_str(), "draft");
        assert_eq!(PayslipStatus::Approved.as_str(), "approved");
        assert_eq!(PayslipStatus::Dispatched.as_str(), "dispatched");
        assert_eq!(PayslipStatus::Voided.as_str(), "voided");
    }

    #[test]
    fn draft_and_approved_are_mutable_dispatched_and_voided_are_not() {
        assert!(PayslipStatus::Draft.is_mutable());
        assert!(PayslipStatus::Approved.is_mutable());
        assert!(!PayslipStatus::Dispatched.is_mutable());
        assert!(!PayslipStatus::Voided.is_mutable());
    }

    #[test]
    fn approved_payslip_is_mutable_pre_dispatch_regression() {
        assert!(
            PayslipStatus::Approved.is_mutable(),
            "Approved payslips must remain mutable until Dispatched/Voided (P1 regression guard)"
        );
    }

    #[test]
    fn only_dispatched_counts_as_dispatched() {
        assert!(!PayslipStatus::Draft.is_dispatched());
        assert!(!PayslipStatus::Approved.is_dispatched());
        assert!(PayslipStatus::Dispatched.is_dispatched());
        assert!(!PayslipStatus::Voided.is_dispatched());
    }

    #[test]
    fn payslip_fsm_terminal_states_are_not_mutable() {
        for terminal in [PayslipStatus::Dispatched, PayslipStatus::Voided] {
            assert!(!terminal.is_mutable(), "{terminal:?} must be immutable");
        }
    }
}
