use std::collections::BTreeMap;

use cell_region::{AzCode, CellId, RegionCode};
use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};
use network_residency::{ResidencyClass, residency_class_allows_home_region_label};

use crate::{
    error::CloudResourceError,
    identity::{
        IamPolicyId, MeteringTag, PrincipalId, ResourceId, ResourceIdParts, TagKey, TagValue,
        typed_policy_ids, typed_tags, validate_tenant_id,
    },
    kind::ResourceKind,
    lifecycle::{ResourceState, state_transition_allowed},
};

pub(crate) const RESOURCE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCreate {
    pub id: String,                          // data_class: INTERNAL_ONLY
    pub tenant_id: String,                   // data_class: INTERNAL_ONLY
    pub region: String,                      // data_class: PUBLIC
    pub az: Option<String>,                  // data_class: PUBLIC
    pub cell_id: String,                     // data_class: PUBLIC
    pub kind: ResourceKind,                  // data_class: PUBLIC
    pub data_class: DataClass,               // data_class: PUBLIC
    pub owner_principal: String,             // data_class: INTERNAL_ONLY
    pub state: ResourceState,                // data_class: PUBLIC
    pub tags: BTreeMap<String, String>,      // data_class: INTERNAL_ONLY
    pub iam_policy_attachments: Vec<String>, // data_class: INTERNAL_ONLY
    pub metering_tag: String,                // data_class: INTERNAL_ONLY
    pub residency: ResidencyClass,           // data_class: INTERNAL_ONLY
    pub created_at_epoch_seconds: u64,       // data_class: INTERNAL_ONLY
    pub updated_at_epoch_seconds: u64,       // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resource {
    pub id: Classified<ResourceId>,
    pub tenant_id: Classified<String>,
    pub region: Classified<RegionCode>,
    pub az: Classified<Option<AzCode>>,
    pub cell_id: Classified<CellId>,
    pub kind: Classified<ResourceKind>,
    pub data_class: Classified<PrivacyDataClass>,
    pub owner_principal: Classified<PrincipalId>,
    pub state: Classified<ResourceState>,
    pub tags: Classified<BTreeMap<TagKey, TagValue>>,
    pub iam_policy_attachments: Classified<Vec<IamPolicyId>>,
    pub metering_tag: Classified<MeteringTag>,
    pub residency: Classified<ResidencyClass>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub updated_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

impl Resource {
    pub fn new(input: ResourceCreate) -> Result<Self, CloudResourceError> {
        validate_tenant_id(&input.tenant_id)?;
        validate_time_order(
            input.created_at_epoch_seconds,
            input.updated_at_epoch_seconds,
        )?;
        if input.state.is_terminal() {
            return Err(CloudResourceError::InvalidInitialState);
        }
        let id = ResourceId::new(input.id)?;
        let id_parts = id.parts()?;
        let region =
            RegionCode::new(input.region).map_err(|_| CloudResourceError::InvalidResourceId)?;
        let az = input
            .az
            .map(AzCode::new)
            .transpose()
            .map_err(|_| CloudResourceError::AzRegionMismatch)?;
        let cell_id =
            CellId::new(input.cell_id).map_err(|_| CloudResourceError::CellLocationMismatch)?;
        validate_resource_id_matches(&id_parts, &region, &input.tenant_id, input.kind)?;
        validate_az_requirement(input.kind, az.as_ref())?;
        validate_az_region(az.as_ref(), &region)?;
        validate_cell_location(&cell_id, &region, az.as_ref())?;
        if !residency_class_allows_home_region_label(&input.residency, &region.value) {
            return Err(CloudResourceError::ResidencyRegionMismatch);
        }
        let data_class = resource_data_class_from_legacy(input.data_class)?;
        let owner_principal = PrincipalId::new(input.owner_principal)?;
        let tags = typed_tags(input.tags)?;
        let iam_policy_attachments = typed_policy_ids(input.iam_policy_attachments)?;
        let metering_tag = MeteringTag::new(input.metering_tag, &input.tenant_id, input.kind)?;
        Ok(Self {
            id: internal(id),
            tenant_id: internal(input.tenant_id),
            region: public(region),
            az: public(az),
            cell_id: public(cell_id),
            kind: public(input.kind),
            data_class: public(data_class),
            owner_principal: internal(owner_principal),
            state: public(input.state),
            tags: internal(tags),
            iam_policy_attachments: internal(iam_policy_attachments),
            metering_tag: internal(metering_tag),
            residency: internal(input.residency),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            updated_at_epoch_seconds: internal(input.updated_at_epoch_seconds),
            schema_version: public(RESOURCE_SCHEMA_VERSION),
        })
    }

    pub fn transition_state(
        &self,
        next_state: ResourceState,
        updated_at_epoch_seconds: u64,
    ) -> Result<Self, CloudResourceError> {
        validate_time_order(
            self.updated_at_epoch_seconds.value,
            updated_at_epoch_seconds,
        )?;
        if !state_transition_allowed(self.state.value, next_state) {
            return Err(CloudResourceError::InvalidStateTransition);
        }
        let mut resource = self.clone();
        resource.state = public(next_state);
        resource.updated_at_epoch_seconds = internal(updated_at_epoch_seconds);
        Ok(resource)
    }
}

pub fn resource_data_class_from_legacy(
    data_class: DataClass,
) -> Result<PrivacyDataClass, CloudResourceError> {
    PrivacyDataClass::new(data_class).map_err(|_| CloudResourceError::InvalidDataClass)
}

fn validate_resource_id_matches(
    id_parts: &ResourceIdParts,
    region: &RegionCode,
    tenant_id: &str,
    kind: ResourceKind,
) -> Result<(), CloudResourceError> {
    if &id_parts.region != region {
        return Err(CloudResourceError::ResourceIdRegionMismatch);
    }
    if id_parts.tenant_id != tenant_id {
        return Err(CloudResourceError::ResourceIdTenantMismatch);
    }
    if id_parts.kind_label != kind.type_label() {
        return Err(CloudResourceError::ResourceIdKindMismatch);
    }
    Ok(())
}

fn validate_az_requirement(
    kind: ResourceKind,
    az: Option<&AzCode>,
) -> Result<(), CloudResourceError> {
    if kind.requires_az() && az.is_none() {
        Err(CloudResourceError::AzRequiredForResourceKind)
    } else {
        Ok(())
    }
}

fn validate_az_region(az: Option<&AzCode>, region: &RegionCode) -> Result<(), CloudResourceError> {
    if let Some(az) = az {
        if az.value == region.value
            || az
                .value
                .strip_prefix(&region.value)
                .is_some_and(|suffix| suffix.starts_with('-') && suffix.len() > 1)
        {
            Ok(())
        } else {
            Err(CloudResourceError::AzRegionMismatch)
        }
    } else {
        Ok(())
    }
}

fn validate_cell_location(
    cell_id: &CellId,
    region: &RegionCode,
    az: Option<&AzCode>,
) -> Result<(), CloudResourceError> {
    let expected_prefix = match az {
        Some(az) => format!("cell-{}-", az.value),
        None => format!("cell-{}-", region.value),
    };
    if cell_id.value.starts_with(&expected_prefix) {
        Ok(())
    } else {
        Err(CloudResourceError::CellLocationMismatch)
    }
}

fn validate_time_order(start: u64, end: u64) -> Result<(), CloudResourceError> {
    if end >= start {
        Ok(())
    } else {
        Err(CloudResourceError::InvalidTimeOrder)
    }
}

fn public<T>(value: T) -> Classified<T> {
    Classified::new(value, DataClass::Public)
}

fn internal<T>(value: T) -> Classified<T> {
    Classified::new(value, DataClass::InternalOnly)
}
