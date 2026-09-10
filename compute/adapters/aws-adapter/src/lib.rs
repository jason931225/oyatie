#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use compute_domain::{
    ComputeProviderKind, ComputeProviderVmCreateRequest, ComputeProviderVmError,
    ComputeProviderVmPort, ComputeProviderVmReceipt, ImageRefKind, Instance, image_ref_kind_label,
    instance_flavor_label, instance_state_label,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AwsComputeAdapterConfigError {
    InvalidEndpoint,
    InvalidAccountRef,
    InvalidRegion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AwsComputeAdapter {
    endpoint_origin: String, // data_class: INTERNAL_ONLY
    account_ref: String,     // data_class: INTERNAL_ONLY
    region: String,          // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AwsComputeCommand {
    pub operation: &'static str,       // data_class: PUBLIC
    pub method: &'static str,          // data_class: PUBLIC
    pub endpoint_origin: String,       // data_class: INTERNAL_ONLY
    pub path: String,                  // data_class: INTERNAL_ONLY
    pub body_canonical: String,        // data_class: INTERNAL_ONLY
    pub provider_evidence_ref: String, // data_class: INTERNAL_ONLY
}

impl AwsComputeAdapter {
    pub fn new(
        endpoint_origin: impl Into<String>,
        account_ref: impl Into<String>,
        region: impl Into<String>,
    ) -> Result<Self, AwsComputeAdapterConfigError> {
        let endpoint_origin = endpoint_origin.into();
        let account_ref = account_ref.into();
        let region = region.into();
        validate_endpoint(&endpoint_origin)?;
        validate_segment(
            &account_ref,
            AwsComputeAdapterConfigError::InvalidAccountRef,
        )?;
        validate_region(&region)?;
        Ok(Self {
            endpoint_origin,
            account_ref,
            region,
        })
    }

    pub fn provider_instance_ref(&self, instance_resource_id: &str) -> String {
        format!(
            "aws-ec2://{}/{}/{}",
            self.account_ref, self.region, instance_resource_id
        )
    }

    pub fn create_instance_command(
        &self,
        request: &ComputeProviderVmCreateRequest,
    ) -> Result<AwsComputeCommand, ComputeProviderVmError> {
        request.validate()?;
        self.ensure_provider_instance(&request.provider_instance_ref, &request.instance)?;
        let instance = &request.instance;
        let provider_evidence_ref = format!(
            "aws-ec2://{}/{}/{}/{}",
            self.account_ref, self.region, instance.resource_id.value.value, request.request_id
        );
        Ok(AwsComputeCommand {
            operation: "RunInstances",
            method: "POST",
            endpoint_origin: self.endpoint_origin.clone(),
            path: "/".to_string(),
            body_canonical: canonical_body(&vm_fields(
                &self.account_ref,
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
                provider: ComputeProviderKind::AwsEc2,
                reason: "provider_instance_ref or region does not match configured AWS EC2 target"
                    .to_string(),
            })
        }
    }
}

impl ComputeProviderVmPort for AwsComputeAdapter {
    fn provider_kind(&self) -> ComputeProviderKind {
        ComputeProviderKind::AwsEc2
    }

    fn create_vm(
        &self,
        input: ComputeProviderVmCreateRequest,
    ) -> Result<ComputeProviderVmReceipt, ComputeProviderVmError> {
        let _command = self.create_instance_command(&input)?;
        Err(ComputeProviderVmError::ProviderRejected {
            provider: self.provider_kind(),
            reason: "AWS EC2 adapter is command-projection preview only; create_vm does not perform production provisioning"
                .to_string(),
        })
    }
}

fn validate_endpoint(value: &str) -> Result<(), AwsComputeAdapterConfigError> {
    if value.starts_with("https://") && no_space_or_control(value) {
        Ok(())
    } else {
        Err(AwsComputeAdapterConfigError::InvalidEndpoint)
    }
}

fn validate_segment(
    value: &str,
    error: AwsComputeAdapterConfigError,
) -> Result<(), AwsComputeAdapterConfigError> {
    if value.trim().is_empty() || value.contains('/') || !no_space_or_control(value) {
        Err(error)
    } else {
        Ok(())
    }
}

fn validate_region(value: &str) -> Result<(), AwsComputeAdapterConfigError> {
    if value.trim().is_empty()
        || value.contains('/')
        || !no_space_or_control(value)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Err(AwsComputeAdapterConfigError::InvalidRegion)
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
    account_ref: &str,
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
        ("account_ref", account_ref.to_string()),
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
