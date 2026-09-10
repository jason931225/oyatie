use cell_region::{AzCode, CellId, RegionCode};
use compute_resource::{ResourceId, ResourceKind};
use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};
use iam_domain::IamRoleId;
use network_domain::SecurityGroupId;
use network_residency::ResidencyClass;

use crate::{
    COMPUTE_SCHEMA_VERSION, CloudComputeError, ComputeFlavorSpec, ComputeQuotaEnvelope, ImageRef,
    KeyPairId, UserDataUri, internal, public, public_metadata_class, region_for, resource_id_for,
    resource_ref_for, security_groups, validate_az_region, validate_cell_az, validate_tenant_id,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum InstanceState {
    Pending,
    Running,
    Stopping,
    Stopped,
    Terminated,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstanceCreate {
    pub resource_id: String,           // data_class: INTERNAL_ONLY
    pub tenant_id: String,             // data_class: INTERNAL_ONLY
    pub region: String,                // data_class: PUBLIC
    pub az: String,                    // data_class: PUBLIC
    pub cell_id: String,               // data_class: PUBLIC
    pub flavor: ComputeFlavorSpec,     // data_class: PUBLIC
    pub image: String,                 // data_class: INTERNAL_ONLY
    pub key_pair: Option<String>,      // data_class: INTERNAL_ONLY
    pub vpc_id: String,                // data_class: INTERNAL_ONLY
    pub subnet_id: String,             // data_class: INTERNAL_ONLY
    pub security_groups: Vec<String>,  // data_class: INTERNAL_ONLY
    pub iam_role: Option<String>,      // data_class: INTERNAL_ONLY
    pub user_data_uri: Option<String>, // data_class: INTERNAL_ONLY
    pub quota: ComputeQuotaEnvelope,   // data_class: INTERNAL_ONLY
    pub residency: ResidencyClass,     // data_class: INTERNAL_ONLY
    pub state: InstanceState,          // data_class: PUBLIC
    pub data_class: DataClass,         // data_class: PUBLIC
    pub created_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instance {
    pub resource_id: Classified<ResourceId>,
    pub tenant_id: Classified<String>,
    pub region: Classified<RegionCode>,
    pub az: Classified<AzCode>,
    pub cell_id: Classified<CellId>,
    pub flavor: Classified<ComputeFlavorSpec>,
    pub image: Classified<ImageRef>,
    pub key_pair: Classified<Option<KeyPairId>>,
    pub vpc_id: Classified<ResourceId>,
    pub subnet_id: Classified<ResourceId>,
    pub security_groups: Classified<Vec<SecurityGroupId>>,
    pub iam_role: Classified<Option<IamRoleId>>,
    pub user_data_uri: Classified<Option<UserDataUri>>,
    pub residency: Classified<ResidencyClass>,
    pub state: Classified<InstanceState>,
    pub data_class: Classified<PrivacyDataClass>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}
impl Instance {
    pub fn new(input: InstanceCreate) -> Result<Self, CloudComputeError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != InstanceState::Pending {
            return Err(CloudComputeError::InvalidInstanceState);
        }
        let region = region_for(&input.region, &input.residency)?;
        let az = AzCode::new(input.az).map_err(|_| CloudComputeError::InvalidAzCode)?;
        validate_az_region(&az, &region)?;
        let cell_id = CellId::new(input.cell_id).map_err(|_| CloudComputeError::InvalidCellId)?;
        validate_cell_az(&cell_id, &az)?;
        let flavor = input.flavor.validate()?;
        input.quota.admit(flavor.units())?;
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::ComputeInstance(flavor.class),
        )?;
        let image = ImageRef::new(input.image)?;
        if image.is_function_bundle() {
            return Err(CloudComputeError::InvalidImageRef);
        }
        let vpc_id = resource_ref_for(&input.vpc_id, &input.tenant_id, &region, "vpc")?;
        let subnet_id = resource_ref_for(&input.subnet_id, &input.tenant_id, &region, "subnet")?;
        let security_groups = security_groups(input.security_groups)?;
        let key_pair = input.key_pair.map(KeyPairId::new).transpose()?;
        let iam_role = input
            .iam_role
            .map(IamRoleId::new)
            .transpose()
            .map_err(|_| CloudComputeError::ResourceKindMismatch)?;
        let user_data_uri = input.user_data_uri.map(UserDataUri::new).transpose()?;
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            region: public(region),
            az: public(az),
            cell_id: public(cell_id),
            flavor: public(flavor),
            image: internal(image),
            key_pair: internal(key_pair),
            vpc_id: internal(vpc_id),
            subnet_id: internal(subnet_id),
            security_groups: internal(security_groups),
            iam_role: internal(iam_role),
            user_data_uri: internal(user_data_uri),
            residency: internal(input.residency),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(COMPUTE_SCHEMA_VERSION),
        })
    }
}
pub const fn instance_state_label(state: InstanceState) -> &'static str {
    match state {
        InstanceState::Pending => "pending",
        InstanceState::Running => "running",
        InstanceState::Stopping => "stopping",
        InstanceState::Stopped => "stopped",
        InstanceState::Terminated => "terminated",
    }
}
