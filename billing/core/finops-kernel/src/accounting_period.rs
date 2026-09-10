//! Accounting-period boundaries and the close lifecycle that gates posting.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// K-GAAP accounting period kind (월마감 = Monthly, 연마감 = Annual).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum AccountingPeriodKind {
    Monthly,
    /// The annual close supersedes every monthly close in the same year — a
    /// K-GAAP rule this type records but does not yet enforce.
    Annual,
}

impl AccountingPeriodKind {
    /// Short label used in audit trails and period IDs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Monthly => "monthly",
            Self::Annual => "annual",
        }
    }
}

/// Lifecycle state of an accounting period.
///
/// Enforces the K-GAAP rule that no journal entries may be posted to a period
/// once it is `Closed` — any attempt must return `PeriodCloseError::PeriodAlreadyClosed`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PeriodCloseState {
    Open,
    /// Period is pending final review; soft-locked (no new postings allowed).
    PendingReview,
    /// Closed — no further postings permitted. The close is not enforced as
    /// irreversible: `state` is public and there is no reopen guard.
    Closed,
}

impl PeriodCloseState {
    pub fn allows_posting(self) -> bool {
        matches!(self, Self::Open)
    }
}

/// An accounting period — fiscal boundary + kind + close lifecycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountingPeriod {
    /// Unique stable ID for this period (e.g. `"ap_2025_03"`, `"ap_2025_annual"`).
    pub period_id: String,
    pub kind: AccountingPeriodKind,
    /// Inclusive start of period in Unix milliseconds.
    pub start_unix_ms: u64,
    /// Exclusive end of period in Unix milliseconds.
    pub end_unix_ms: u64,
    pub state: PeriodCloseState,
}

/// Errors from accounting-period operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PeriodCloseError {
    EmptyPeriodId,
    InvalidPeriodBoundary,
    PeriodAlreadyClosed,
    InvalidStateTransition {
        from: PeriodCloseState,
        to: PeriodCloseState,
    },
}

impl PeriodCloseError {
    pub fn message(&self) -> String {
        match self {
            Self::EmptyPeriodId => "period_id must not be empty".to_owned(),
            Self::InvalidPeriodBoundary => {
                "end_unix_ms must be strictly greater than start_unix_ms".to_owned()
            }
            Self::PeriodAlreadyClosed => "cannot post to a closed period".to_owned(),
            Self::InvalidStateTransition { from, to } => {
                format!(
                    "invalid period-close state transition: {:?} -> {:?}",
                    from, to
                )
            }
        }
    }
}

/// Validate an `AccountingPeriod` for structural correctness.
pub fn validate_accounting_period(p: &AccountingPeriod) -> Result<(), PeriodCloseError> {
    if p.period_id.is_empty() {
        return Err(PeriodCloseError::EmptyPeriodId);
    }
    if p.end_unix_ms <= p.start_unix_ms {
        return Err(PeriodCloseError::InvalidPeriodBoundary);
    }
    Ok(())
}

/// Attempt to post to a period — returns `Err` if the period is not `Open`
/// or if the period is structurally invalid (empty id or inverted boundaries).
pub fn assert_posting_allowed(p: &AccountingPeriod) -> Result<(), PeriodCloseError> {
    validate_accounting_period(p)?;
    if !p.state.allows_posting() {
        if matches!(p.state, PeriodCloseState::Closed) {
            return Err(PeriodCloseError::PeriodAlreadyClosed);
        }
        return Err(PeriodCloseError::InvalidStateTransition {
            from: p.state,
            to: PeriodCloseState::Open,
        });
    }
    Ok(())
}

/// Advance a period from `Open` → `PendingReview`.
/// Returns `Err` if the period is structurally invalid or not in `Open` state.
pub fn begin_review(p: &mut AccountingPeriod) -> Result<(), PeriodCloseError> {
    validate_accounting_period(p)?;
    if p.state != PeriodCloseState::Open {
        return Err(PeriodCloseError::InvalidStateTransition {
            from: p.state,
            to: PeriodCloseState::PendingReview,
        });
    }
    p.state = PeriodCloseState::PendingReview;
    Ok(())
}

/// Advance a period from `PendingReview` → `Closed` (월마감/연마감 final close).
/// Returns `Err` if the period is structurally invalid or not in `PendingReview` state.
pub fn close_period(p: &mut AccountingPeriod) -> Result<(), PeriodCloseError> {
    validate_accounting_period(p)?;
    if p.state != PeriodCloseState::PendingReview {
        return Err(PeriodCloseError::InvalidStateTransition {
            from: p.state,
            to: PeriodCloseState::Closed,
        });
    }
    p.state = PeriodCloseState::Closed;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_period(id: &str) -> AccountingPeriod {
        AccountingPeriod {
            period_id: id.into(),
            kind: AccountingPeriodKind::Monthly,
            start_unix_ms: 1_000,
            end_unix_ms: 2_000,
            state: PeriodCloseState::Open,
        }
    }

    #[test]
    fn period_kind_labels_distinct() {
        assert_ne!(
            AccountingPeriodKind::Monthly.label(),
            AccountingPeriodKind::Annual.label()
        );
    }

    #[test]
    fn validate_valid_period() {
        let p = open_period("ap_2025_03");
        assert!(validate_accounting_period(&p).is_ok());
    }

    #[test]
    fn validate_empty_id_rejected() {
        let p = open_period("");
        assert!(matches!(
            validate_accounting_period(&p),
            Err(PeriodCloseError::EmptyPeriodId)
        ));
    }

    #[test]
    fn validate_inverted_boundary_rejected() {
        let p = AccountingPeriod {
            period_id: "ap_2025_03".into(),
            kind: AccountingPeriodKind::Monthly,
            start_unix_ms: 2_000,
            end_unix_ms: 1_000,
            state: PeriodCloseState::Open,
        };
        assert!(matches!(
            validate_accounting_period(&p),
            Err(PeriodCloseError::InvalidPeriodBoundary)
        ));
    }

    #[test]
    fn open_period_allows_posting() {
        let p = open_period("ap_2025_03");
        assert!(assert_posting_allowed(&p).is_ok());
    }

    #[test]
    fn pending_review_blocks_posting() {
        let mut p = open_period("ap_2025_03");
        begin_review(&mut p).unwrap();
        assert!(assert_posting_allowed(&p).is_err());
    }

    #[test]
    fn closed_period_blocks_posting_with_specific_error() {
        let mut p = open_period("ap_2025_03");
        begin_review(&mut p).unwrap();
        close_period(&mut p).unwrap();
        assert!(matches!(
            assert_posting_allowed(&p),
            Err(PeriodCloseError::PeriodAlreadyClosed)
        ));
    }

    #[test]
    fn happy_path_open_to_pending_to_closed() {
        let mut p = open_period("ap_2025_annual");
        assert_eq!(p.state, PeriodCloseState::Open);
        begin_review(&mut p).unwrap();
        assert_eq!(p.state, PeriodCloseState::PendingReview);
        close_period(&mut p).unwrap();
        assert_eq!(p.state, PeriodCloseState::Closed);
    }

    #[test]
    fn begin_review_non_open_rejected() {
        let mut p = open_period("ap_2025_03");
        begin_review(&mut p).unwrap();
        assert!(matches!(
            begin_review(&mut p),
            Err(PeriodCloseError::InvalidStateTransition { .. })
        ));
    }

    #[test]
    fn close_period_without_review_rejected() {
        let mut p = open_period("ap_2025_03");
        assert!(matches!(
            close_period(&mut p),
            Err(PeriodCloseError::InvalidStateTransition { .. })
        ));
    }

    #[test]
    fn annual_period_kind_label() {
        let p = AccountingPeriod {
            period_id: "ap_2025_annual".into(),
            kind: AccountingPeriodKind::Annual,
            start_unix_ms: 0,
            end_unix_ms: 31_536_000_000,
            state: PeriodCloseState::Open,
        };
        assert_eq!(p.kind.label(), "annual");
    }

    #[test]
    fn error_messages_non_empty() {
        let errors = [
            PeriodCloseError::EmptyPeriodId,
            PeriodCloseError::InvalidPeriodBoundary,
            PeriodCloseError::PeriodAlreadyClosed,
            PeriodCloseError::InvalidStateTransition {
                from: PeriodCloseState::Open,
                to: PeriodCloseState::Closed,
            },
        ];
        for e in &errors {
            assert!(!e.message().is_empty(), "empty message for {:?}", e);
        }
    }
}
