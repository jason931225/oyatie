use crate::identifier::FlowAnomalyId;
use compute_resource::ResourceId;
use data_boundary_kernel::Classified;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FlowAnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowAnomalyEvent {
    pub id: Classified<FlowAnomalyId>,  // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<String>,  // data_class: INTERNAL_ONLY
    pub vpc_id: Classified<ResourceId>, // data_class: INTERNAL_ONLY
    pub severity: Classified<FlowAnomalySeverity>, // data_class: PUBLIC
    pub flow_pattern: Classified<String>, // data_class: INTERNAL_ONLY
    pub detected_at_epoch_seconds: Classified<u64>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>, // data_class: PUBLIC
}
