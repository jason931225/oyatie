#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use compute_domain::{
    ComputeProviderKind, ComputeProviderVmCreateRequest, ComputeProviderVmError,
    ComputeProviderVmPort, ComputeProviderVmReceipt, ImageRefKind, Instance, image_ref_kind_label,
    instance_flavor_label, instance_state_label,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OciComputeAdapterConfigError {
    InvalidEndpoint,
    InvalidCompartmentRef,
    InvalidAvailabilityDomain,
    InvalidRegion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OciComputeAdapter {
    endpoint_origin: String,     // data_class: INTERNAL_ONLY
    compartment_ref: String,     // data_class: INTERNAL_ONLY
    availability_domain: String, // data_class: INTERNAL_ONLY
    region: String,              // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OciComputeCommand {
    pub operation: &'static str,       // data_class: PUBLIC
    pub method: &'static str,          // data_class: PUBLIC
    pub endpoint_origin: String,       // data_class: INTERNAL_ONLY
    pub path: String,                  // data_class: INTERNAL_ONLY
    pub body_canonical: String,        // data_class: INTERNAL_ONLY
    pub provider_evidence_ref: String, // data_class: INTERNAL_ONLY
}

impl OciComputeAdapter {
    pub fn new(
        endpoint_origin: impl Into<String>,
        compartment_ref: impl Into<String>,
        availability_domain: impl Into<String>,
        region: impl Into<String>,
    ) -> Result<Self, OciComputeAdapterConfigError> {
        let endpoint_origin = endpoint_origin.into();
        let compartment_ref = compartment_ref.into();
        let availability_domain = availability_domain.into();
        let region = region.into();
        validate_endpoint(&endpoint_origin)?;
        validate_segment(
            &compartment_ref,
            OciComputeAdapterConfigError::InvalidCompartmentRef,
        )?;
        validate_segment(
            &availability_domain,
            OciComputeAdapterConfigError::InvalidAvailabilityDomain,
        )?;
        validate_region(&region)?;
        Ok(Self {
            endpoint_origin,
            compartment_ref,
            availability_domain,
            region,
        })
    }

    pub fn provider_instance_ref(&self, instance_resource_id: &str) -> String {
        format!(
            "oci-compute://{}/{}/{}",
            self.compartment_ref, self.availability_domain, instance_resource_id
        )
    }

    pub fn launch_instance_command(
        &self,
        request: &ComputeProviderVmCreateRequest,
    ) -> Result<OciComputeCommand, ComputeProviderVmError> {
        request.validate()?;
        self.ensure_provider_instance(&request.provider_instance_ref, &request.instance)?;
        let instance = &request.instance;
        let provider_evidence_ref = format!(
            "oci-compute://{}/{}/{}/{}",
            self.compartment_ref,
            self.availability_domain,
            instance.resource_id.value.value,
            request.request_id
        );
        Ok(OciComputeCommand {
            operation: "LaunchInstance",
            method: "POST",
            endpoint_origin: self.endpoint_origin.clone(),
            path: "/20160918/instances".to_string(),
            body_canonical: canonical_body(&vm_fields(
                &self.compartment_ref,
                &self.availability_domain,
                &self.region,
                instance,
                request,
            )),
            provider_evidence_ref,
        })
    }

    fn ensure_provider_instance(
        &self,
        provider_instance_ref: &str,
        instance: &Instance,
    ) -> Result<(), ComputeProviderVmError> {
        let expected = self.provider_instance_ref(&instance.resource_id.value.value);
        if provider_instance_ref == expected && instance.region.value.value == self.region {
            Ok(())
        } else {
            Err(ComputeProviderVmError::ProviderRejected {
                provider: ComputeProviderKind::OciCompute,
                reason:
                    "provider_instance_ref or region does not match configured OCI Compute target"
                        .to_string(),
            })
        }
    }
}

impl ComputeProviderVmPort for OciComputeAdapter {
    fn provider_kind(&self) -> ComputeProviderKind {
        ComputeProviderKind::OciCompute
    }

    fn create_vm(
        &self,
        input: ComputeProviderVmCreateRequest,
    ) -> Result<ComputeProviderVmReceipt, ComputeProviderVmError> {
        let _command = self.launch_instance_command(&input)?;
        Err(ComputeProviderVmError::ProviderRejected {
            provider: self.provider_kind(),
            reason: "OCI Compute adapter is command-projection preview only; create_vm does not perform production provisioning"
                .to_string(),
        })
    }
}

fn validate_endpoint(value: &str) -> Result<(), OciComputeAdapterConfigError> {
    if value.starts_with("https://") && no_space_or_control(value) {
        Ok(())
    } else {
        Err(OciComputeAdapterConfigError::InvalidEndpoint)
    }
}

fn validate_segment(
    value: &str,
    error: OciComputeAdapterConfigError,
) -> Result<(), OciComputeAdapterConfigError> {
    if value.trim().is_empty() || value.contains('/') || !no_space_or_control(value) {
        Err(error)
    } else {
        Ok(())
    }
}

fn validate_region(value: &str) -> Result<(), OciComputeAdapterConfigError> {
    if value.trim().is_empty()
        || value.contains('/')
        || !no_space_or_control(value)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Err(OciComputeAdapterConfigError::InvalidRegion)
    } else {
        Ok(())
    }
}

fn no_space_or_control(value: &str) -> bool {
    !value
        .bytes()
        .any(|byte| byte.is_ascii_control() || byte == b' ')
}

fn vm_fields(
    compartment_ref: &str,
    availability_domain: &str,
    region: &str,
    instance: &Instance,
    request: &ComputeProviderVmCreateRequest,
) -> Vec<(&'static str, String)> {
    let flavor = instance.flavor.value;
    let image = &instance.image.value;
    let key_pair = instance
        .key_pair
        .value
        .as_ref()
        .map(|key_pair| key_pair.value.as_str())
        .unwrap_or("");
    let iam_role = instance
        .iam_role
        .value
        .as_ref()
        .map(|role| role.value.as_str())
        .unwrap_or("");
    let user_data_uri = instance
        .user_data_uri
        .value
        .as_ref()
        .map(|uri| uri.value.as_str())
        .unwrap_or("");
    let security_groups = instance
        .security_groups
        .value
        .iter()
        .map(|group| group.value.as_str())
        .collect::<Vec<_>>()
        .join(",");
    vec![
        ("compartment_ref", compartment_ref.to_string()),
        ("availability_domain", availability_domain.to_string()),
        ("region", region.to_string()),
        ("resource_id", instance.resource_id.value.value.clone()),
        ("tenant_id", instance.tenant_id.value.clone()),
        ("az", instance.az.value.value.clone()),
        ("cell_id", instance.cell_id.value.value.clone()),
        (
            "flavor_class",
            instance_flavor_label(flavor.class).to_string(),
        ),
        ("vcpu", flavor.vcpu.to_string()),
        ("memory_gb", flavor.memory_gb.to_string()),
        ("gpu_count", flavor.gpu_count.to_string()),
        ("local_ssd_gb", flavor.local_ssd_gb.to_string()),
        ("image_ref", image.value.clone()),
        ("image_kind", image_kind_label(image.kind).to_string()),
        ("key_pair", key_pair.to_string()),
        ("vpc_id", instance.vpc_id.value.value.clone()),
        ("subnet_id", instance.subnet_id.value.value.clone()),
        ("security_groups", security_groups),
        ("iam_role", iam_role.to_string()),
        ("user_data_uri", user_data_uri.to_string()),
        (
            "residency",
            instance
                .residency
                .value
                .label()
                .unwrap_or("per_pack")
                .to_string(),
        ),
        (
            "state",
            instance_state_label(instance.state.value).to_string(),
        ),
        ("data_class", instance.data_class.value.label().to_string()),
        ("actor", request.actor.clone()),
        ("idempotency_key", request.idempotency_key.clone()),
        (
            "requested_at_epoch_seconds",
            request.requested_at_epoch_seconds.to_string(),
        ),
    ]
}

fn image_kind_label(kind: ImageRefKind) -> &'static str {
    image_ref_kind_label(kind)
}

fn canonical_body(fields: &[(&str, String)]) -> String {
    fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests;
