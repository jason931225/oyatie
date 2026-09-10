//! Cloud Region API boundary for region and availability-zone listing.

mod error;
mod model;
mod operations;
mod projection;

pub use model::*;
pub use operations::{
    list_cloud_azs_from_api, list_cloud_regions_from_api, validate_cloud_az_list_request,
    validate_cloud_region_list_request,
};
