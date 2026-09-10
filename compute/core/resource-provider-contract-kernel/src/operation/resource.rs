use serde::{Deserialize, Serialize};

use super::{
    OPERATION_NAME_PREFIX, OperationError, OperationLedgerEntry, OperationResult, OperationState,
};
use crate::error::ContractShapeError;

/// An AIP-151-shaped operation resource for async mutations. The
/// constructors enforce the structural invariant `done == result.is_some()`:
/// a pending operation has no result, a terminal one always has exactly one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub name: String,                    // data_class: INTERNAL_ONLY
    pub done: bool,                      // data_class: INTERNAL_ONLY
    pub metadata: OperationLedgerEntry,  // data_class: INTERNAL_ONLY
    pub result: Option<OperationResult>, // data_class: INTERNAL_ONLY
}

impl Operation {
    fn checked_name(name: impl Into<String>) -> Result<String, ContractShapeError> {
        let name = name.into();
        if name.len() > OPERATION_NAME_PREFIX.len() && name.starts_with(OPERATION_NAME_PREFIX) {
            Ok(name)
        } else {
            Err(ContractShapeError::MalformedOperationName { value: name })
        }
    }

    fn checked_name_and_ledger(
        name: impl Into<String>,
        metadata: &OperationLedgerEntry,
    ) -> Result<String, ContractShapeError> {
        let name = Self::checked_name(name)?;
        metadata.validate()?;
        let expected = format!("{OPERATION_NAME_PREFIX}{}", metadata.operation_id);
        if name == expected {
            Ok(name)
        } else {
            Err(ContractShapeError::MalformedOperationLedger {
                message: format!(
                    "operation name {name:?} must match ledger operation_id {:?}",
                    metadata.operation_id
                ),
            })
        }
    }

    pub fn pending(
        name: impl Into<String>,
        metadata: OperationLedgerEntry,
    ) -> Result<Self, ContractShapeError> {
        let name = Self::checked_name_and_ledger(name, &metadata)?;
        if metadata.state.is_terminal() {
            return Err(ContractShapeError::InvalidOperationState {
                message: format!(
                    "pending operation cannot carry terminal state {:?}",
                    metadata.state
                ),
            });
        }
        Ok(Self {
            name,
            done: false,
            metadata,
            result: None,
        })
    }

    pub fn succeeded(
        name: impl Into<String>,
        metadata: OperationLedgerEntry,
        response: serde_json::Value,
    ) -> Result<Self, ContractShapeError> {
        let name = Self::checked_name_and_ledger(name, &metadata)?;
        if metadata.state != OperationState::Succeeded {
            return Err(ContractShapeError::InvalidOperationState {
                message: format!(
                    "successful operation must carry succeeded ledger state, got {:?}",
                    metadata.state
                ),
            });
        }
        Ok(Self {
            name,
            done: true,
            metadata,
            result: Some(OperationResult::Response(response)),
        })
    }

    pub fn failed(
        name: impl Into<String>,
        metadata: OperationLedgerEntry,
        error: OperationError,
    ) -> Result<Self, ContractShapeError> {
        let name = Self::checked_name_and_ledger(name, &metadata)?;
        if !metadata.state.is_terminal() || metadata.state == OperationState::Succeeded {
            return Err(ContractShapeError::InvalidOperationState {
                message: format!(
                    "failed operation must carry failed/cancelled/rolled_back ledger state, got {:?}",
                    metadata.state
                ),
            });
        }
        Ok(Self {
            name,
            done: true,
            metadata,
            result: Some(OperationResult::Error(error)),
        })
    }

    /// Surface the structural invariant for operations received over the
    /// wire (where constructors were not in control).
    pub fn validate(&self) -> Result<(), ContractShapeError> {
        Self::checked_name_and_ledger(self.name.clone(), &self.metadata)?;
        if self.done != self.result.is_some() {
            return Err(ContractShapeError::InvalidOperationState {
                message: format!("{} done/result mismatch", self.name),
            });
        }
        if self.done != self.metadata.state.is_terminal() {
            return Err(ContractShapeError::InvalidOperationState {
                message: format!(
                    "{} done flag {:?} disagrees with ledger state {:?}",
                    self.name, self.done, self.metadata.state
                ),
            });
        }
        match (&self.result, self.metadata.state) {
            (Some(OperationResult::Response(_)), OperationState::Succeeded) | (None, _) => Ok(()),
            (Some(OperationResult::Error(_)), state)
                if state.is_terminal() && state != OperationState::Succeeded =>
            {
                Ok(())
            }
            (Some(OperationResult::Response(_)), state) => {
                Err(ContractShapeError::InvalidOperationState {
                    message: format!("response result cannot accompany ledger state {state:?}"),
                })
            }
            (Some(OperationResult::Error(_)), state) => {
                Err(ContractShapeError::InvalidOperationState {
                    message: format!("error result cannot accompany ledger state {state:?}"),
                })
            }
        }
    }
}
