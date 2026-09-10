mod block {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum OciBlockStorageAdapterConfigError {
        InvalidEndpoint,
        InvalidCompartmentRef,
        InvalidAvailabilityDomain,
    }
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct OciBlockStorageAdapter {
        endpoint_origin: String,
        compartment_ref: String,
        availability_domain: String,
        clock_epoch_seconds: u64,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct OciBlockStorageCommand {
        pub operation: &'static str,
        pub method: &'static str,
        pub endpoint_origin: String,
        pub path: String,
        pub body_canonical: String,
        pub provider_evidence_ref: String,
    }

    impl OciBlockStorageAdapter {
        pub fn new(
            endpoint_origin: impl Into<String>,
            compartment_ref: impl Into<String>,
            availability_domain: impl Into<String>,
        ) -> Result<Self, OciBlockStorageAdapterConfigError> {
            let endpoint_origin = endpoint_origin.into();
            let compartment_ref = compartment_ref.into();
            let availability_domain = availability_domain.into();
            validate_block_endpoint(&endpoint_origin)?;
            validate_block_segment(
                &compartment_ref,
                OciBlockStorageAdapterConfigError::InvalidCompartmentRef,
            )?;
            validate_block_segment(
                &availability_domain,
                OciBlockStorageAdapterConfigError::InvalidAvailabilityDomain,
            )?;
            Ok(Self {
                endpoint_origin,
                compartment_ref,
                availability_domain,
                clock_epoch_seconds: 0,
            })
        }

        pub fn with_clock(mut self, clock_epoch_seconds: u64) -> Self {
            self.clock_epoch_seconds = clock_epoch_seconds;
            self
        }

        pub fn provider_volume_ref(&self, volume_id: &str) -> String {
            format!(
                "oci-block://{}/{}/{}",
                self.compartment_ref, self.availability_domain, volume_id
            )
        }

        pub fn create_volume_command(
            &self,
            request: &StorageProviderBlockCreateVolumeRequest,
        ) -> Result<OciBlockStorageCommand, StorageProviderBlockError> {
            request.validate()?;
            self.ensure_provider_volume(&request.provider_volume_ref, &request.volume_id)?;
            let size_gib = request.size_gib.to_string();
            let iops = request.performance.iops.to_string();
            let throughput_mbps = request.performance.throughput_mbps.to_string();
            let requested_at = request.requested_at_epoch_seconds.to_string();
            let kms_key = request.kms_key.as_deref().unwrap_or("");
            Ok(OciBlockStorageCommand {
                operation: "CreateVolume",
                method: "POST",
                endpoint_origin: self.endpoint_origin.clone(),
                path: "/20160918/volumes".to_string(),
                body_canonical: canonical_body(&[
                    ("compartment_ref", self.compartment_ref.as_str()),
                    ("availability_domain", self.availability_domain.as_str()),
                    ("volume_id", request.volume_id.as_str()),
                    ("tenant_id", request.tenant_id.as_str()),
                    ("name", request.name.as_str()),
                    ("region", request.region.as_str()),
                    ("az", request.az.as_str()),
                    ("cell_id", request.cell_id.as_str()),
                    ("residency", request.residency.label().unwrap_or("per_pack")),
                    ("tier", volume_tier_label(request.tier)),
                    ("size_gib", size_gib.as_str()),
                    ("iops", iops.as_str()),
                    ("throughput_mbps", throughput_mbps.as_str()),
                    ("encryption", encryption_label(request.encryption)),
                    ("kms_key", kms_key),
                    ("data_class", request.data_class.label()),
                    ("actor", request.actor.as_str()),
                    ("idempotency_key", request.idempotency_key.as_str()),
                    ("requested_at_epoch_seconds", requested_at.as_str()),
                ]),
                provider_evidence_ref: format!(
                    "oci-block://{}/{}/{}/{}",
                    self.compartment_ref,
                    self.availability_domain,
                    request.volume_id,
                    request.request_id
                ),
            })
        }

        fn ensure_provider_volume(
            &self,
            provider_volume_ref: &str,
            volume_id: &str,
        ) -> Result<(), StorageProviderBlockError> {
            let expected = self.provider_volume_ref(volume_id);
            if provider_volume_ref == expected {
                Ok(())
            } else {
                Err(StorageProviderBlockError::ProviderRejected {
                    provider: StorageProviderKind::OciBlockStorage,
                    reason: "provider_volume_ref does not match configured OCI Block Volume target"
                        .to_string(),
                })
            }
        }

        fn provider_request_id(&self, request_id: &str) -> String {
            format!("oci-block-{}-{request_id}", self.clock_epoch_seconds)
        }
    }

    impl StorageProviderBlockPort for OciBlockStorageAdapter {
        fn provider_kind(&self) -> StorageProviderKind {
            StorageProviderKind::OciBlockStorage
        }

        fn create_volume(
            &self,
            input: StorageProviderBlockCreateVolumeRequest,
        ) -> Result<StorageProviderBlockReceipt, StorageProviderBlockError> {
            let command = self.create_volume_command(&input)?;
            StorageProviderBlockReceipt::create_volume(
                self.provider_kind(),
                input.clone(),
                self.provider_request_id(&input.request_id),
                command.provider_evidence_ref,
            )
        }
    }
    fn validate_block_endpoint(value: &str) -> Result<(), OciBlockStorageAdapterConfigError> {
        if value.starts_with("https://") && no_space_or_control(value) {
            Ok(())
        } else {
            Err(OciBlockStorageAdapterConfigError::InvalidEndpoint)
        }
    }

    fn validate_block_segment(
        value: &str,
        error: OciBlockStorageAdapterConfigError,
    ) -> Result<(), OciBlockStorageAdapterConfigError> {
        if value.trim().is_empty() || value.contains('/') || !no_space_or_control(value) {
            Err(error)
        } else {
            Ok(())
        }
    }

    fn no_space_or_control(value: &str) -> bool {
        !value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
    }

    fn canonical_body(fields: &[(&str, &str)]) -> String {
        fields
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("|")
    }

    const fn volume_tier_label(tier: VolumeTier) -> &'static str {
        match tier {
            VolumeTier::GeneralPurposeSsd => "general_purpose_ssd",
            VolumeTier::ProvisionedIopsSsd => "provisioned_iops_ssd",
        }
    }

    const fn encryption_label(mode: EncryptionMode) -> &'static str {
        match mode {
            EncryptionMode::Sse => "sse",
            EncryptionMode::SseKms => "sse_kms",
            EncryptionMode::Byok => "byok",
            EncryptionMode::Hyok => "hyok",
        }
    }
}

pub use block::*;
