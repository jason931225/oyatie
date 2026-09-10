#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

//! HR employee lifecycle status.
//!
//! Statute: 대한민국.노동.근로기준법 §17, §42.

use std::fmt;

/// Employee lifecycle status. [`EmployeeStatus::as_str`] is the SQL wire form
/// stored in `hr.employees.status`, and it is a frozen contract: renaming one
/// orphans every row already written under the old spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum EmployeeStatus {
    Active,
    Terminated,
    OnLeave,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownEmployeeStatus(pub String);

impl fmt::Display for UnknownEmployeeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown employee status: {:?}; expected one of ", self.0)?;
        for (index, accepted) in EmployeeStatus::ALL.iter().enumerate() {
            if index > 0 {
                f.write_str("|")?;
            }
            f.write_str(accepted.as_str())?;
        }
        Ok(())
    }
}

impl std::error::Error for UnknownEmployeeStatus {}

impl EmployeeStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Terminated => "terminated",
            Self::OnLeave => "on_leave",
        }
    }

    pub fn from_wire(s: &str) -> Result<Self, UnknownEmployeeStatus> {
        match s {
            "active" => Ok(Self::Active),
            "terminated" => Ok(Self::Terminated),
            "on_leave" => Ok(Self::OnLeave),
            other => Err(UnknownEmployeeStatus(other.to_string())),
        }
    }

    pub const fn as_korean(self) -> &'static str {
        match self {
            Self::Active => "재직중",
            Self::Terminated => "퇴직",
            Self::OnLeave => "휴직중",
        }
    }

    /// 근로기준법 §60: leave keeps accruing through an approved absence, so
    /// only termination stops it.
    pub const fn is_leave_accrual_eligible(self) -> bool {
        !matches!(self, Self::Terminated)
    }

    pub const ALL: [Self; 3] = [Self::Active, Self::Terminated, Self::OnLeave];
}

impl fmt::Display for EmployeeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_in_all_has_a_distinct_wire_string() {
        let all = EmployeeStatus::ALL;
        let mut seen = std::collections::HashSet::new();
        for variant in all {
            assert!(
                seen.insert(variant.as_str()),
                "duplicate wire string: {}",
                variant.as_str()
            );
        }
    }

    #[test]
    fn round_trip_from_wire_for_every_variant() {
        for variant in EmployeeStatus::ALL {
            let wire = variant.as_str();
            let parsed = EmployeeStatus::from_wire(wire).expect("all wire strings must round-trip");
            assert_eq!(parsed, variant, "round-trip failed for {wire}");
        }
    }

    #[test]
    fn from_wire_rejects_unknown_value() {
        let err =
            EmployeeStatus::from_wire("suspended").expect_err("unknown string must be rejected");
        assert_eq!(err.0, "suspended");
        let msg = err.to_string();
        assert!(msg.contains("unknown employee status"));
        assert!(msg.contains("suspended"));
    }

    #[test]
    fn from_wire_rejects_empty_string() {
        let err = EmployeeStatus::from_wire("").expect_err("empty string must be rejected");
        assert_eq!(err.0, "");
        assert!(err.to_string().contains("unknown employee status"));
    }

    #[test]
    fn display_renders_wire_string() {
        assert_eq!(EmployeeStatus::Active.to_string(), "active");
        assert_eq!(EmployeeStatus::Terminated.to_string(), "terminated");
        assert_eq!(EmployeeStatus::OnLeave.to_string(), "on_leave");
    }

    #[test]
    fn korean_labels_are_non_empty_and_distinct() {
        let labels: Vec<&str> = EmployeeStatus::ALL.iter().map(|v| v.as_korean()).collect();
        let unique: std::collections::HashSet<_> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 3, "Korean labels must be distinct");
        for label in &labels {
            assert!(!label.is_empty(), "Korean label must not be empty");
        }
    }

    #[test]
    fn leave_accrual_eligibility_contract() {
        assert!(EmployeeStatus::Active.is_leave_accrual_eligible());
        assert!(EmployeeStatus::OnLeave.is_leave_accrual_eligible());
        assert!(!EmployeeStatus::Terminated.is_leave_accrual_eligible());
    }

    #[test]
    fn unknown_employee_status_error_display_covers_expected_values() {
        let msg = UnknownEmployeeStatus("xyz".to_string()).to_string();
        assert!(msg.contains("xyz"));
        assert!(msg.contains("active"));
        assert!(msg.contains("terminated"));
        assert!(msg.contains("on_leave"));
    }

    #[test]
    fn wire_strings_are_frozen_against_renaming() {
        let frozen_by_contract = ["active", "terminated", "on_leave"];
        for wire in frozen_by_contract {
            let parsed = EmployeeStatus::from_wire(wire)
                .unwrap_or_else(|_| panic!("DDL wire value {wire:?} must parse"));
            assert_eq!(parsed.as_str(), wire, "as_str must be identity for {wire}");
        }
    }
}
