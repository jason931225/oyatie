#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// Variants are ordered from earliest to latest stage; `Ord` reflects that
/// ordering so callers can assert forward-only progression.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DesignPartnerStatus {
    Provisioned,
    WorkflowAuthored,
    AgentsActive,
    Live,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesignPartnerStatusError {
    IllegalRegression {
        from: DesignPartnerStatus,
        to: DesignPartnerStatus,
    },
    /// e.g. `Provisioned -> Live`, skipping `WorkflowAuthored`/`AgentsActive`.
    SkippedStage {
        from: DesignPartnerStatus,
        to: DesignPartnerStatus,
        expected_next: DesignPartnerStatus,
    },
    AlreadyTerminal {
        from: DesignPartnerStatus,
    },
}

impl DesignPartnerStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Provisioned => "provisioned",
            Self::WorkflowAuthored => "workflow_authored",
            Self::AgentsActive => "agents_active",
            Self::Live => "live",
        }
    }

    pub fn expected_next(self) -> Option<Self> {
        match self {
            Self::Provisioned => Some(Self::WorkflowAuthored),
            Self::WorkflowAuthored => Some(Self::AgentsActive),
            Self::AgentsActive => Some(Self::Live),
            Self::Live => None,
        }
    }

    pub fn advance(self, next: Self) -> Result<Self, DesignPartnerStatusError> {
        if next <= self {
            return Err(DesignPartnerStatusError::IllegalRegression {
                from: self,
                to: next,
            });
        }
        match self.expected_next() {
            None => Err(DesignPartnerStatusError::AlreadyTerminal { from: self }),
            Some(expected) if next == expected => Ok(next),
            Some(expected) => Err(DesignPartnerStatusError::SkippedStage {
                from: self,
                to: next,
                expected_next: expected,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_snake_case() {
        assert_eq!(DesignPartnerStatus::Provisioned.label(), "provisioned");
        assert_eq!(
            DesignPartnerStatus::WorkflowAuthored.label(),
            "workflow_authored"
        );
        assert_eq!(DesignPartnerStatus::AgentsActive.label(), "agents_active");
        assert_eq!(DesignPartnerStatus::Live.label(), "live");
    }

    #[test]
    fn forward_advance_succeeds() {
        let s = DesignPartnerStatus::Provisioned
            .advance(DesignPartnerStatus::WorkflowAuthored)
            .expect("Provisioned -> WorkflowAuthored is valid");
        assert_eq!(s, DesignPartnerStatus::WorkflowAuthored);
    }

    #[test]
    fn full_happy_path() {
        let live = DesignPartnerStatus::Provisioned
            .advance(DesignPartnerStatus::WorkflowAuthored)
            .unwrap()
            .advance(DesignPartnerStatus::AgentsActive)
            .unwrap()
            .advance(DesignPartnerStatus::Live)
            .unwrap();
        assert_eq!(live.label(), "live");
    }

    #[test]
    fn regression_rejected() {
        let err = DesignPartnerStatus::AgentsActive
            .advance(DesignPartnerStatus::Provisioned)
            .expect_err("regression must be rejected");
        assert_eq!(
            err,
            DesignPartnerStatusError::IllegalRegression {
                from: DesignPartnerStatus::AgentsActive,
                to: DesignPartnerStatus::Provisioned,
            }
        );
    }

    #[test]
    fn same_stage_is_also_regression() {
        let err = DesignPartnerStatus::Live
            .advance(DesignPartnerStatus::Live)
            .expect_err("idempotent advance must be rejected");
        assert_eq!(
            err,
            DesignPartnerStatusError::IllegalRegression {
                from: DesignPartnerStatus::Live,
                to: DesignPartnerStatus::Live,
            }
        );
    }

    #[test]
    fn skipped_stage_provisioned_to_live_rejected() {
        let err = DesignPartnerStatus::Provisioned
            .advance(DesignPartnerStatus::Live)
            .expect_err("non-adjacent jump must be rejected");
        assert_eq!(
            err,
            DesignPartnerStatusError::SkippedStage {
                from: DesignPartnerStatus::Provisioned,
                to: DesignPartnerStatus::Live,
                expected_next: DesignPartnerStatus::WorkflowAuthored,
            }
        );
    }

    #[test]
    fn skipped_stage_provisioned_to_agents_active_rejected() {
        let err = DesignPartnerStatus::Provisioned
            .advance(DesignPartnerStatus::AgentsActive)
            .expect_err("non-adjacent jump must be rejected");
        assert!(matches!(err, DesignPartnerStatusError::SkippedStage { .. }));
    }

    #[test]
    fn advance_past_terminal_rejected() {
        let err = DesignPartnerStatus::Live
            .advance(DesignPartnerStatus::Provisioned)
            .expect_err("advance past terminal must be rejected");
        assert!(
            matches!(err, DesignPartnerStatusError::IllegalRegression { .. }),
            "the regression guard must fire before the terminal guard: \
Provisioned < Live, so this is IllegalRegression, not AlreadyTerminal"
        );
    }
}
