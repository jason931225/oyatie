use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationError {
    pub code: String,    // data_class: INTERNAL_ONLY
    pub message: String, // data_class: INTERNAL_ONLY
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationResult {
    Response(serde_json::Value),
    Error(OperationError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Accepted,
    Validating,
    Queued,
    Running,
    WaitingForReconciler,
    Succeeded,
    Failed,
    CancelRequested,
    Cancelled,
    Compensating,
    RolledBack,
}

impl OperationState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::RolledBack
        )
    }

    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Accepted, Self::Validating)
                | (Self::Accepted, Self::CancelRequested)
                | (Self::Validating, Self::Queued)
                | (Self::Validating, Self::Failed)
                | (Self::Validating, Self::CancelRequested)
                | (Self::Queued, Self::Running)
                | (Self::Queued, Self::CancelRequested)
                | (Self::Running, Self::WaitingForReconciler)
                | (Self::Running, Self::Succeeded)
                | (Self::Running, Self::Failed)
                | (Self::Running, Self::CancelRequested)
                | (Self::Running, Self::Compensating)
                | (Self::WaitingForReconciler, Self::Running)
                | (Self::WaitingForReconciler, Self::CancelRequested)
                | (Self::CancelRequested, Self::Cancelled)
                | (Self::CancelRequested, Self::Failed)
                | (Self::Compensating, Self::RolledBack)
                | (Self::Compensating, Self::Failed)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationPhase {
    ApiGateway,
    ResourceRegistry,
    OperationLedger,
    WorkflowReconciler,
    BackendActuationBoundary,
}
