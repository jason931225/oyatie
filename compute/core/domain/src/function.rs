use cell_region::{AzCode, CellId, RegionCode};
use compute_resource::{FunctionRuntime, ResourceId, ResourceKind};
use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};
use network_residency::ResidencyClass;

use crate::{
    COMPUTE_SCHEMA_VERSION, CloudComputeError, FunctionName, IdempotencyKey, ImageRef,
    InvocationId, MAX_FUNCTION_COLD_START_BUDGET_MS, internal, map_resource_error, privacy_classes,
    public, public_metadata_class, region_for, resource_id_for, validate_az_region,
    validate_cell_az, validate_tenant_id,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FunctionDeploymentState {
    Deploying,
    Active,
    Disabled,
    Deleting,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionDeploymentCreate {
    pub resource_id: String,                  // data_class: INTERNAL_ONLY
    pub tenant_id: String,                    // data_class: INTERNAL_ONLY
    pub region: String,                       // data_class: PUBLIC
    pub az: String,                           // data_class: PUBLIC
    pub cell_id: String,                      // data_class: PUBLIC
    pub runtime: FunctionRuntime,             // data_class: PUBLIC
    pub name: String,                         // data_class: PUBLIC
    pub bundle: String,                       // data_class: INTERNAL_ONLY
    pub cold_start_budget_ms: u32,            // data_class: PUBLIC
    pub timeout_ms: u32,                      // data_class: PUBLIC
    pub memory_mb: u32,                       // data_class: PUBLIC
    pub max_concurrency: u32,                 // data_class: PUBLIC
    pub allowed_data_classes: Vec<DataClass>, // data_class: INTERNAL_ONLY
    pub residency: ResidencyClass,            // data_class: INTERNAL_ONLY
    pub state: FunctionDeploymentState,       // data_class: PUBLIC
    pub data_class: DataClass,                // data_class: PUBLIC
    pub created_at_epoch_seconds: u64,        // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionDeployment {
    pub resource_id: Classified<ResourceId>,
    pub tenant_id: Classified<String>,
    pub region: Classified<RegionCode>,
    pub az: Classified<AzCode>,
    pub cell_id: Classified<CellId>,
    pub runtime: Classified<FunctionRuntime>,
    pub name: Classified<FunctionName>,
    pub bundle: Classified<ImageRef>,
    pub cold_start_budget_ms: Classified<u32>,
    pub timeout_ms: Classified<u32>,
    pub memory_mb: Classified<u32>,
    pub max_concurrency: Classified<u32>,
    pub allowed_data_classes: Classified<Vec<PrivacyDataClass>>,
    pub residency: Classified<ResidencyClass>,
    pub state: Classified<FunctionDeploymentState>,
    pub data_class: Classified<PrivacyDataClass>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionInvocationRequest {
    pub invocation_id: String,               // data_class: INTERNAL_ONLY
    pub tenant_id: String,                   // data_class: INTERNAL_ONLY
    pub function_id: String,                 // data_class: INTERNAL_ONLY
    pub region: String,                      // data_class: PUBLIC
    pub payload_data_class: DataClass,       // data_class: INTERNAL_ONLY
    pub idempotency_key: String,             // data_class: INTERNAL_ONLY
    pub current_concurrent_invocations: u32, // data_class: INTERNAL_ONLY
    pub requested_at_epoch_seconds: u64,     // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionInvocationReceipt {
    pub invocation_id: Classified<InvocationId>,
    pub tenant_id: Classified<String>,
    pub function_id: Classified<ResourceId>,
    pub region: Classified<RegionCode>,
    pub payload_data_class: Classified<PrivacyDataClass>,
    pub idempotency_key: Classified<IdempotencyKey>,
    pub cold_start_budget_ms: Classified<u32>,
    pub accepted_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}
impl FunctionDeployment {
    pub fn new(input: FunctionDeploymentCreate) -> Result<Self, CloudComputeError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != FunctionDeploymentState::Deploying {
            return Err(CloudComputeError::InvalidFunctionState);
        }
        let region = region_for(&input.region, &input.residency)?;
        let az = AzCode::new(input.az).map_err(|_| CloudComputeError::InvalidAzCode)?;
        validate_az_region(&az, &region)?;
        let cell_id = CellId::new(input.cell_id).map_err(|_| CloudComputeError::InvalidCellId)?;
        validate_cell_az(&cell_id, &az)?;
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::Function(input.runtime),
        )?;
        let bundle = ImageRef::new(input.bundle)?;
        if !bundle.is_function_bundle() {
            return Err(CloudComputeError::InvalidImageRef);
        }
        if input.cold_start_budget_ms == 0
            || input.cold_start_budget_ms > MAX_FUNCTION_COLD_START_BUDGET_MS
            || !(100..=900_000).contains(&input.timeout_ms)
            || !(128..=10_240).contains(&input.memory_mb)
            || input.max_concurrency == 0
            || input.max_concurrency > 10_000
        {
            return Err(CloudComputeError::InvalidFunctionBudget);
        }
        let allowed_data_classes = privacy_classes(input.allowed_data_classes)?;
        if allowed_data_classes.is_empty() {
            return Err(CloudComputeError::PayloadDataClassNotAllowed);
        }
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            region: public(region),
            az: public(az),
            cell_id: public(cell_id),
            runtime: public(input.runtime),
            name: public(FunctionName::new(input.name)?),
            bundle: internal(bundle),
            cold_start_budget_ms: public(input.cold_start_budget_ms),
            timeout_ms: public(input.timeout_ms),
            memory_mb: public(input.memory_mb),
            max_concurrency: public(input.max_concurrency),
            allowed_data_classes: internal(allowed_data_classes),
            residency: internal(input.residency),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(COMPUTE_SCHEMA_VERSION),
        })
    }

    pub fn activate(mut self) -> Result<Self, CloudComputeError> {
        if self.state.value != FunctionDeploymentState::Deploying {
            return Err(CloudComputeError::InvalidFunctionState);
        }
        self.state = public(FunctionDeploymentState::Active);
        Ok(self)
    }

    pub fn invoke(
        &self,
        input: FunctionInvocationRequest,
    ) -> Result<FunctionInvocationReceipt, CloudComputeError> {
        if self.state.value != FunctionDeploymentState::Active {
            return Err(CloudComputeError::FunctionNotActive);
        }
        validate_tenant_id(&input.tenant_id)?;
        if input.tenant_id != self.tenant_id.value {
            return Err(CloudComputeError::ResourceTenantMismatch);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudComputeError::InvalidResourceId)?;
        if region != self.region.value {
            return Err(CloudComputeError::ResourceRegionMismatch);
        }
        let function_id = ResourceId::new(input.function_id).map_err(map_resource_error)?;
        if function_id != self.resource_id.value {
            return Err(CloudComputeError::UnknownFunction);
        }
        let payload_data_class = PrivacyDataClass::new(input.payload_data_class)
            .map_err(|_| CloudComputeError::InvalidDataClass)?;
        if !self
            .allowed_data_classes
            .value
            .contains(&payload_data_class)
        {
            return Err(CloudComputeError::PayloadDataClassNotAllowed);
        }
        if input.current_concurrent_invocations >= self.max_concurrency.value {
            return Err(CloudComputeError::QuotaExceeded);
        }
        Ok(FunctionInvocationReceipt {
            invocation_id: internal(InvocationId::new(input.invocation_id)?),
            tenant_id: internal(input.tenant_id),
            function_id: internal(function_id),
            region: public(region),
            payload_data_class: internal(payload_data_class),
            idempotency_key: internal(IdempotencyKey::new(input.idempotency_key)?),
            cold_start_budget_ms: public(self.cold_start_budget_ms.value),
            accepted_at_epoch_seconds: internal(input.requested_at_epoch_seconds),
            schema_version: public(COMPUTE_SCHEMA_VERSION),
        })
    }
}
