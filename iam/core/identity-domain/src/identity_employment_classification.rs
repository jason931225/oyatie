#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

//! Employment classification, per Bominal ADR-0126.

use std::fmt;

/// Employment classification. [`EmploymentClassification::as_str`] is the
/// Korean wire form stored in `identity.employments.classification`, and it is
/// a frozen contract: renaming one orphans every row already written under the
/// old spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum EmploymentClassification {
    Regular,
    Contract,
    PartTime,
    Dispatched,
    Outsourced,
    Freelancer,
    Intern,
    Executive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownEmploymentClassification(pub String);

impl fmt::Display for UnknownEmploymentClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown employment classification: {:?}; expected one of ",
            self.0
        )?;
        for (index, accepted) in EmploymentClassification::ALL.iter().enumerate() {
            if index > 0 {
                f.write_str("|")?;
            }
            f.write_str(accepted.as_str())?;
        }
        Ok(())
    }
}

impl std::error::Error for UnknownEmploymentClassification {}

impl EmploymentClassification {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "정규직",
            Self::Contract => "계약직",
            Self::PartTime => "단시간근로자",
            Self::Dispatched => "파견",
            Self::Outsourced => "도급",
            Self::Freelancer => "프리랜서",
            Self::Intern => "인턴",
            Self::Executive => "임원",
        }
    }

    pub fn from_wire(s: &str) -> Result<Self, UnknownEmploymentClassification> {
        match s {
            "정규직" => Ok(Self::Regular),
            "계약직" => Ok(Self::Contract),
            "단시간근로자" => Ok(Self::PartTime),
            "파견" => Ok(Self::Dispatched),
            "도급" => Ok(Self::Outsourced),
            "프리랜서" => Ok(Self::Freelancer),
            "인턴" => Ok(Self::Intern),
            "임원" => Ok(Self::Executive),
            other => Err(UnknownEmploymentClassification(other.to_string())),
        }
    }

    pub const ALL: [Self; 8] = [
        Self::Regular,
        Self::Contract,
        Self::PartTime,
        Self::Dispatched,
        Self::Outsourced,
        Self::Freelancer,
        Self::Intern,
        Self::Executive,
    ];
}

impl fmt::Display for EmploymentClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_in_all_has_a_distinct_wire_string() {
        let all = EmploymentClassification::ALL;
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
    fn round_trip_from_str_for_every_variant() {
        for variant in EmploymentClassification::ALL {
            let wire = variant.as_str();
            let parsed = EmploymentClassification::from_wire(wire)
                .expect("all wire strings must round-trip");
            assert_eq!(parsed, variant, "round-trip failed for {wire}");
        }
    }

    #[test]
    fn from_str_rejects_unknown_value() {
        let err = EmploymentClassification::from_wire("unknown")
            .expect_err("unknown string must be rejected");
        assert_eq!(err.0, "unknown");
        let msg = err.to_string();
        assert!(msg.contains("unknown employment classification"));
        assert!(msg.contains("unknown"));
    }

    #[test]
    fn display_renders_korean_wire_string() {
        assert_eq!(EmploymentClassification::Regular.to_string(), "정규직");
        assert_eq!(EmploymentClassification::Executive.to_string(), "임원");
    }

    #[test]
    fn from_str_rejects_empty_string() {
        let err =
            EmploymentClassification::from_wire("").expect_err("empty string must be rejected");
        assert_eq!(err.0, "");
        assert!(
            err.to_string()
                .contains("unknown employment classification")
        );
    }

    #[test]
    fn from_wire_rejects_the_english_variant_name() {
        let err = EmploymentClassification::from_wire("Regular")
            .expect_err("English label must not match");
        assert_eq!(err.0, "Regular");
    }

    #[test]
    fn unknown_employment_classification_error_display_covers_all_variants() {
        let msg = UnknownEmploymentClassification("xyz".to_string()).to_string();
        assert!(msg.contains("xyz"));
        assert!(msg.contains("정규직"));
        assert!(msg.contains("임원"));
    }

    #[test]
    fn wire_strings_are_frozen_against_renaming() {
        let frozen_by_contract = [
            "정규직",
            "계약직",
            "단시간근로자",
            "파견",
            "도급",
            "프리랜서",
            "인턴",
            "임원",
        ];
        for wire in frozen_by_contract {
            let parsed = EmploymentClassification::from_wire(wire)
                .unwrap_or_else(|_| panic!("wire string {wire:?} must parse"));
            assert_eq!(parsed.as_str(), wire, "as_str must be identity for {wire}");
        }
    }
}
