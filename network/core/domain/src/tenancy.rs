use crate::error::{CloudNetworkError, map_resource_error};
use cell_region::AzCode;
use cell_region::CellId;
use cell_region::RegionCode;
use compute_resource::ResourceId;
use compute_resource::ResourceKind;

const TENANT_ID_PREFIX: &str = "ten_";

pub(crate) fn resource_id_for(
    value: &str,
    tenant_id: &str,
    region: &RegionCode,
    kind: ResourceKind,
) -> Result<ResourceId, CloudNetworkError> {
    let id = ResourceId::new(value.to_string()).map_err(map_resource_error)?;
    if id.tenant_id().map_err(map_resource_error)? != tenant_id {
        return Err(CloudNetworkError::ResourceTenantMismatch);
    }
    if id.region().map_err(map_resource_error)? != *region {
        return Err(CloudNetworkError::ResourceRegionMismatch);
    }
    if id.kind_label().map_err(map_resource_error)? != kind.type_label() {
        return Err(CloudNetworkError::ResourceKindMismatch);
    }
    Ok(id)
}

pub(crate) fn validate_cell_region(
    cell_id: &CellId,
    region: &RegionCode,
) -> Result<(), CloudNetworkError> {
    let expected_prefix = format!("cell-{}-", region.value);
    if cell_id.value.starts_with(&expected_prefix) {
        Ok(())
    } else {
        Err(CloudNetworkError::InvalidCellId)
    }
}

pub(crate) fn validate_tenant_id(value: &str) -> Result<(), CloudNetworkError> {
    let Some(suffix) = value.strip_prefix(TENANT_ID_PREFIX) else {
        return Err(CloudNetworkError::InvalidTenantId);
    };
    if !suffix.is_empty()
        && suffix.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
    {
        Ok(())
    } else {
        Err(CloudNetworkError::InvalidTenantId)
    }
}

pub(crate) fn validate_az_region(
    az: &AzCode,
    region: &RegionCode,
) -> Result<(), CloudNetworkError> {
    if az.value == region.value
        || az
            .value
            .strip_prefix(&region.value)
            .is_some_and(|suffix| suffix.starts_with('-') && suffix.len() > 1)
    {
        Ok(())
    } else {
        Err(CloudNetworkError::AzRegionMismatch)
    }
}
