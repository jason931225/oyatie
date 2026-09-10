use std::fmt;

use crate::operation::OPERATION_NAME_PREFIX;
use crate::pagination::MAX_PAGE_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractShapeError {
    MalformedIdempotencyKey { value: String },
    MalformedResourceName { value: String },
    EmptyPageToken,
    PageSizeOutOfRange { requested: u32 },
    MalformedOperationName { value: String },
    MalformedOperationLedger { message: String },
    InvalidOperationState { message: String },
}

impl fmt::Display for ContractShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedIdempotencyKey { value } => {
                write!(
                    f,
                    "idempotency key {value:?} is not a canonical RFC 4122 UUID"
                )
            }
            Self::MalformedResourceName { value } => {
                write!(f, "resource name {value:?} is not `collection/resource-id`")
            }
            Self::EmptyPageToken => write!(f, "page token must be non-empty"),
            Self::PageSizeOutOfRange { requested } => {
                write!(f, "page size {requested} is outside 1..={MAX_PAGE_SIZE}")
            }
            Self::MalformedOperationName { value } => {
                write!(
                    f,
                    "operation name {value:?} lacks the {OPERATION_NAME_PREFIX:?} prefix"
                )
            }
            Self::MalformedOperationLedger { message } => {
                write!(f, "malformed operation ledger entry: {message}")
            }
            Self::InvalidOperationState { message } => {
                write!(f, "invalid operation state: {message}")
            }
        }
    }
}

impl std::error::Error for ContractShapeError {}
