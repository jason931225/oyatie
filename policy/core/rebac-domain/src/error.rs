use std::fmt;

use policy_cedar_domain::rebac::RebacTupleStoreError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpansionError {
    /// No rewrite is configured for this object type and relation.
    UndefinedRelation {
        object_type: String,
        relation: String,
    },
    /// The rewrite tree nested deeper than the configured bound.
    DepthExceeded { limit: u32 },
    /// The walk read more tuples than the configured bound allowed.
    TupleBudgetExceeded { limit: usize },
    /// The decision enumerated more membership candidates than allowed.
    CandidateBudgetExceeded { limit: usize },
    /// One tupleset paged further than the configured bound allowed. Distinct
    /// from the tuple bound so an operator can tell a wide relation from a
    /// store that is not terminating its pagination.
    PageBudgetExceeded { limit: usize },
    /// A relation reaches itself through the subtracted side of a
    /// `Difference`. Refused when the model is built, never at decision time.
    NonStratified {
        object_type: String,
        relation: String,
    },
    /// A relation re-entered itself while under a subtraction, closed by a
    /// TUPLE rather than by the model. Model-time stratification cannot see
    /// this edge — it exists only in data — so the walk refuses rather than
    /// reading the re-entry as "not excluded" and granting.
    NegatedCycleInData {
        object_type: String,
        relation: String,
    },
    /// The store refused or failed. Never treated as an absent grant.
    Store(RebacTupleStoreError),
}

impl fmt::Display for ExpansionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndefinedRelation {
                object_type,
                relation,
            } => write!(f, "no rewrite configured for {object_type}#{relation}"),
            Self::DepthExceeded { limit } => {
                write!(f, "expansion exceeded the depth bound of {limit}")
            }
            Self::TupleBudgetExceeded { limit } => {
                write!(f, "expansion exceeded the tuple-read bound of {limit}")
            }
            Self::CandidateBudgetExceeded { limit } => {
                write!(f, "expansion exceeded the candidate bound of {limit}")
            }
            Self::PageBudgetExceeded { limit } => {
                write!(f, "one tupleset exceeded the page bound of {limit}")
            }
            Self::NonStratified {
                object_type,
                relation,
            } => write!(
                f,
                "{object_type}#{relation} reaches itself through a Difference \
                 subtraction; such a model grants what it excludes"
            ),
            Self::NegatedCycleInData {
                object_type,
                relation,
            } => write!(
                f,
                "{object_type}#{relation} re-entered itself under a subtraction \
                 via a relationship tuple; the exclusion cannot be decided"
            ),
            Self::Store(error) => write!(f, "tuple store: {error}"),
        }
    }
}

impl std::error::Error for ExpansionError {}

impl From<RebacTupleStoreError> for ExpansionError {
    fn from(error: RebacTupleStoreError) -> Self {
        Self::Store(error)
    }
}
