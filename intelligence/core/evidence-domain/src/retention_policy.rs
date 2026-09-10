//! How long evidence must be retained under a given regulatory schedule.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// A count of calendar days. Unvalidated on its own; [`RetentionPolicy`] is
/// what rejects zero and below-minimum values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RetentionDays(pub u32);

impl RetentionDays {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// Each named variant is a product-level retention duty, not a jurisdiction.
/// `Custom` exists so a new duty does not have to break the enum; prefer a
/// named variant wherever one exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RegulatorySchedule {
    EvidenceRecordThreeYear,
    ProcessingActivityRecordThreeYear,
    Custom,
}

impl RegulatorySchedule {
    /// `None` for `Custom`, whose duration only the caller knows.
    pub fn default_retention_days(self) -> Option<RetentionDays> {
        match self {
            RegulatorySchedule::EvidenceRecordThreeYear => Some(RetentionDays(1_095)),
            RegulatorySchedule::ProcessingActivityRecordThreeYear => Some(RetentionDays(1_095)),
            RegulatorySchedule::Custom => None,
        }
    }
}

/// Errors produced when constructing a [`RetentionPolicy`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetentionPolicyError {
    ZeroRetentionDays,
    MissingCustomDuration,
    BelowScheduleMinimum {
        supplied: RetentionDays,
        minimum: RetentionDays,
    },
}

/// # Invariants
/// - `retention_days` > 0 always.
/// - `retention_days` ≥ `schedule.default_retention_days()` when the
///   schedule prescribes a minimum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetentionPolicy {
    schedule: RegulatorySchedule,
    retention_days: RetentionDays,
}

impl RetentionPolicy {
    pub fn schedule(&self) -> RegulatorySchedule {
        self.schedule
    }

    pub fn retention_days(&self) -> RetentionDays {
        self.retention_days
    }

    /// The only constructor, which is what makes the invariants above hold
    /// for every value of this type.
    pub fn new(
        schedule: RegulatorySchedule,
        retention_days: RetentionDays,
    ) -> Result<Self, RetentionPolicyError> {
        if retention_days.0 == 0 {
            return Err(RetentionPolicyError::ZeroRetentionDays);
        }
        if let Some(minimum) = schedule.default_retention_days()
            && retention_days < minimum
        {
            return Err(RetentionPolicyError::BelowScheduleMinimum {
                supplied: retention_days,
                minimum,
            });
        }
        Ok(Self {
            schedule,
            retention_days,
        })
    }

    /// Fails for `Custom`, which prescribes no default.
    pub fn from_schedule_default(
        schedule: RegulatorySchedule,
    ) -> Result<Self, RetentionPolicyError> {
        let retention_days = schedule
            .default_retention_days()
            .ok_or(RetentionPolicyError::MissingCustomDuration)?;
        Self::new(schedule, retention_days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_record_default_is_1095_days() {
        let policy =
            RetentionPolicy::from_schedule_default(RegulatorySchedule::EvidenceRecordThreeYear)
                .unwrap();
        assert_eq!(policy.retention_days().as_u32(), 1_095);
        assert_eq!(
            policy.schedule(),
            RegulatorySchedule::EvidenceRecordThreeYear
        );
    }

    #[test]
    fn processing_activity_record_default_is_1095_days() {
        let policy = RetentionPolicy::from_schedule_default(
            RegulatorySchedule::ProcessingActivityRecordThreeYear,
        )
        .unwrap();
        assert_eq!(policy.retention_days().as_u32(), 1_095);
    }

    #[test]
    fn custom_schedule_requires_explicit_duration() {
        let err = RetentionPolicy::from_schedule_default(RegulatorySchedule::Custom).unwrap_err();
        assert_eq!(err, RetentionPolicyError::MissingCustomDuration);
    }

    #[test]
    fn custom_schedule_accepts_explicit_duration() {
        let policy = RetentionPolicy::new(RegulatorySchedule::Custom, RetentionDays(365)).unwrap();
        assert_eq!(policy.retention_days().as_u32(), 365);
    }

    #[test]
    fn zero_retention_days_is_rejected() {
        let err = RetentionPolicy::new(
            RegulatorySchedule::EvidenceRecordThreeYear,
            RetentionDays(0),
        )
        .unwrap_err();
        assert_eq!(err, RetentionPolicyError::ZeroRetentionDays);
    }

    #[test]
    fn below_minimum_is_rejected() {
        let err = RetentionPolicy::new(
            RegulatorySchedule::EvidenceRecordThreeYear,
            RetentionDays(364),
        )
        .unwrap_err();
        assert_eq!(
            err,
            RetentionPolicyError::BelowScheduleMinimum {
                supplied: RetentionDays(364),
                minimum: RetentionDays(1_095),
            }
        );
    }

    #[test]
    fn above_minimum_is_accepted() {
        let policy = RetentionPolicy::new(
            RegulatorySchedule::EvidenceRecordThreeYear,
            RetentionDays(2_000),
        )
        .unwrap();
        assert_eq!(policy.retention_days().as_u32(), 2_000);
    }

    #[test]
    fn retention_days_ordering() {
        assert!(RetentionDays(100) < RetentionDays(200));
        assert!(RetentionDays(200) > RetentionDays(100));
        assert_eq!(RetentionDays(100), RetentionDays(100));
    }
}
