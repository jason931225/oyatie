use serde::{Deserialize, Serialize};

use super::{OperationPhase, OperationState};
use crate::error::ContractShapeError;
use crate::identity::{IdempotencyKey, is_slug};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryPolicy {
    pub backoff: String,              // data_class: INTERNAL_ONLY
    pub max_attempts: u32,            // data_class: INTERNAL_ONLY
    pub retry_classification: String, // data_class: INTERNAL_ONLY
}

pub const ALLOWED_RETRY_CLASSIFICATIONS: &[&str] = &[
    "transient",
    "quota",
    "policy",
    "dependency",
    "operator_required",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancellationMetadata {
    pub cancel_safe: bool,    // data_class: INTERNAL_ONLY
    pub audit_required: bool, // data_class: INTERNAL_ONLY
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompensationMetadata {
    pub required: bool,   // data_class: INTERNAL_ONLY
    pub strategy: String, // data_class: INTERNAL_ONLY
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationLedgerEntry {
    pub operation_id: String,               // data_class: INTERNAL_ONLY
    pub idempotency_key: String,            // data_class: INTERNAL_ONLY
    pub request_hash: String,               // data_class: INTERNAL_ONLY
    pub resource_orn: String,               // data_class: TENANT_SCOPED
    pub desired_generation: u64,            // data_class: INTERNAL_ONLY
    pub observed_generation: u64,           // data_class: INTERNAL_ONLY
    pub state: OperationState,              // data_class: INTERNAL_ONLY
    pub phase: OperationPhase,              // data_class: INTERNAL_ONLY
    pub tenant_account_project: String,     // data_class: TENANT_SCOPED
    pub region_cell: String,                // data_class: TENANT_SCOPED
    pub principal: String,                  // data_class: INTERNAL_ONLY
    pub audit_chain_id: String,             // data_class: INTERNAL_ONLY
    pub retry_policy: RetryPolicy,          // data_class: INTERNAL_ONLY
    pub cancellation: CancellationMetadata, // data_class: INTERNAL_ONLY
    pub compensation: CompensationMetadata, // data_class: INTERNAL_ONLY
    pub transition_sequence: u64,           // data_class: INTERNAL_ONLY
}

fn malformed(message: impl Into<String>) -> ContractShapeError {
    ContractShapeError::MalformedOperationLedger {
        message: message.into(),
    }
}

impl OperationLedgerEntry {
    pub fn validate(&self) -> Result<(), ContractShapeError> {
        self.validate_identity()?;
        self.validate_generation_bounds()?;
        self.validate_required_scope_fields()?;
        self.validate_retry_policy()?;
        self.validate_cancellation()?;
        self.validate_transition_sequence()
    }

    fn validate_identity(&self) -> Result<(), ContractShapeError> {
        if !is_slug(&self.operation_id) {
            return Err(malformed(format!(
                "operation_id {:?} is not slug-shaped",
                self.operation_id
            )));
        }
        IdempotencyKey::new(self.idempotency_key.clone())
            .map_err(|error| malformed(error.to_string()))?;
        if self.request_hash.is_empty() {
            return Err(malformed("request_hash must be non-empty"));
        }
        if !self.resource_orn.starts_with("orn:") || !self.resource_orn.contains('/') {
            return Err(malformed(format!(
                "resource_orn {:?} is not ORN-shaped",
                self.resource_orn
            )));
        }
        Ok(())
    }

    fn validate_generation_bounds(&self) -> Result<(), ContractShapeError> {
        if self.desired_generation == 0 || self.observed_generation > self.desired_generation {
            return Err(malformed(format!(
                "generation bounds invalid: desired={}, observed={}",
                self.desired_generation, self.observed_generation
            )));
        }
        Ok(())
    }

    fn validate_required_scope_fields(&self) -> Result<(), ContractShapeError> {
        for (field, value) in [
            (
                "tenant_account_project",
                self.tenant_account_project.as_str(),
            ),
            ("region_cell", self.region_cell.as_str()),
            ("principal", self.principal.as_str()),
            ("audit_chain_id", self.audit_chain_id.as_str()),
            ("retry_policy.backoff", self.retry_policy.backoff.as_str()),
            (
                "retry_policy.retry_classification",
                self.retry_policy.retry_classification.as_str(),
            ),
            ("compensation.strategy", self.compensation.strategy.as_str()),
        ] {
            if value.is_empty() {
                return Err(malformed(format!("{field} must be non-empty")));
            }
        }
        Ok(())
    }

    fn validate_retry_policy(&self) -> Result<(), ContractShapeError> {
        if self.retry_policy.max_attempts == 0 {
            return Err(malformed("retry_policy.max_attempts must be non-zero"));
        }
        if !ALLOWED_RETRY_CLASSIFICATIONS.contains(&self.retry_policy.retry_classification.as_str())
        {
            return Err(malformed(format!(
                "retry_policy.retry_classification {:?} is not one of {:?}",
                self.retry_policy.retry_classification, ALLOWED_RETRY_CLASSIFICATIONS
            )));
        }
        Ok(())
    }

    fn validate_cancellation(&self) -> Result<(), ContractShapeError> {
        if self.cancellation.audit_required {
            Ok(())
        } else {
            Err(malformed("cancellation.audit_required must be true"))
        }
    }

    fn validate_transition_sequence(&self) -> Result<(), ContractShapeError> {
        if self.transition_sequence == 0 {
            return Err(malformed("transition_sequence must be non-zero"));
        }
        Ok(())
    }
}
