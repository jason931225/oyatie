#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OciObjectStorageAdapterConfigError {
    InvalidEndpoint,
    InvalidNamespace,
    InvalidBucketName,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OciObjectStorageAdapter {
    endpoint_origin: String,
    namespace_name: String,
    bucket_name: String,
    clock_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OciObjectStorageCommand {
    pub operation: &'static str,
    pub method: &'static str,
    pub endpoint_origin: String,
    pub path: String,
    pub body_canonical: String,
    pub provider_evidence_ref: String,
}
impl OciObjectStorageAdapter {
    pub fn new(
        endpoint_origin: impl Into<String>,
        namespace_name: impl Into<String>,
        bucket_name: impl Into<String>,
    ) -> Result<Self, OciObjectStorageAdapterConfigError> {
        let endpoint_origin = endpoint_origin.into();
        let namespace_name = namespace_name.into();
        let bucket_name = bucket_name.into();
        validate_endpoint(&endpoint_origin)?;
        validate_segment(
            &namespace_name,
            OciObjectStorageAdapterConfigError::InvalidNamespace,
        )?;
        validate_segment(
            &bucket_name,
            OciObjectStorageAdapterConfigError::InvalidBucketName,
        )?;
        Ok(Self {
            endpoint_origin,
            namespace_name,
            bucket_name,
            clock_epoch_seconds: 0,
        })
    }

    pub fn with_clock(mut self, clock_epoch_seconds: u64) -> Self {
        self.clock_epoch_seconds = clock_epoch_seconds;
        self
    }

    pub fn provider_bucket_ref(&self) -> String {
        format!("oci-object://{}/{}", self.namespace_name, self.bucket_name)
    }

    pub fn put_command(
        &self,
        request: &StorageProviderObjectPutRequest,
    ) -> Result<OciObjectStorageCommand, StorageProviderObjectError> {
        request.validate()?;
        self.ensure_provider_bucket(&request.provider_bucket_ref)?;
        let size_bytes = request.size_bytes.to_string();
        Ok(self.command(
            "PutObject",
            "PUT",
            &request.object_key,
            &request.request_id,
            &[
                ("bucket_id", request.bucket_id.as_str()),
                ("tenant_id", request.tenant_id.as_str()),
                ("object_key", request.object_key.as_str()),
                ("object_body_ref", request.object_body_ref.as_str()),
                ("size_bytes", size_bytes.as_str()),
                ("etag", request.etag.as_str()),
                ("data_class", request.data_class.label()),
                ("kms_key", request.kms_key.as_str()),
                ("ciphertext_ref", request.ciphertext_ref.as_str()),
                ("actor", request.actor.as_str()),
                ("idempotency_key", request.idempotency_key.as_str()),
            ],
        ))
    }

    pub fn get_command(
        &self,
        request: &StorageProviderObjectGetRequest,
    ) -> Result<OciObjectStorageCommand, StorageProviderObjectError> {
        request.validate()?;
        self.ensure_provider_bucket(&request.provider_bucket_ref)?;
        Ok(self.command(
            "GetObject",
            "GET",
            &request.object_key,
            &request.request_id,
            &[
                ("bucket_id", request.bucket_id.as_str()),
                ("tenant_id", request.tenant_id.as_str()),
                ("object_key", request.object_key.as_str()),
                ("result_body_ref", request.result_body_ref.as_str()),
                ("actor", request.actor.as_str()),
            ],
        ))
    }

    fn command(
        &self,
        operation: &'static str,
        method: &'static str,
        object_key: &str,
        request_id: &str,
        fields: &[(&str, &str)],
    ) -> OciObjectStorageCommand {
        OciObjectStorageCommand {
            operation,
            method,
            endpoint_origin: self.endpoint_origin.clone(),
            path: format!(
                "/n/{}/b/{}/o/{}",
                self.namespace_name,
                self.bucket_name,
                encode_object_path(object_key)
            ),
            body_canonical: canonical_body(fields),
            provider_evidence_ref: format!(
                "oci-object://{}/{}/{}/{}",
                self.namespace_name, self.bucket_name, object_key, request_id
            ),
        }
    }

    fn ensure_provider_bucket(
        &self,
        provider_bucket_ref: &str,
    ) -> Result<(), StorageProviderObjectError> {
        let expected = self.provider_bucket_ref();
        if provider_bucket_ref == expected {
            Ok(())
        } else {
            Err(StorageProviderObjectError::ProviderRejected {
                provider: StorageProviderKind::OciObjectStorage,
                reason: "provider_bucket_ref does not match configured OCI Object Storage bucket"
                    .to_string(),
            })
        }
    }

    fn provider_request_id(&self, request_id: &str) -> String {
        format!("oci-object-{}-{request_id}", self.clock_epoch_seconds)
    }
}

impl StorageProviderObjectPort for OciObjectStorageAdapter {
    fn provider_kind(&self) -> StorageProviderKind {
        StorageProviderKind::OciObjectStorage
    }

    fn put_object(
        &self,
        input: StorageProviderObjectPutRequest,
    ) -> Result<StorageProviderObjectReceipt, StorageProviderObjectError> {
        let command = self.put_command(&input)?;
        StorageProviderObjectReceipt::put_object(
            self.provider_kind(),
            input.clone(),
            self.provider_request_id(&input.request_id),
            command.provider_evidence_ref,
        )
    }

    fn get_object(
        &self,
        input: StorageProviderObjectGetRequest,
    ) -> Result<StorageProviderObjectReceipt, StorageProviderObjectError> {
        let command = self.get_command(&input)?;
        StorageProviderObjectReceipt::get_object(
            self.provider_kind(),
            input.clone(),
            self.provider_request_id(&input.request_id),
            command.provider_evidence_ref,
        )
    }
}

fn validate_endpoint(value: &str) -> Result<(), OciObjectStorageAdapterConfigError> {
    if value.starts_with("https://") && no_space_or_control(value) {
        Ok(())
    } else {
        Err(OciObjectStorageAdapterConfigError::InvalidEndpoint)
    }
}

fn validate_segment(
    value: &str,
    error: OciObjectStorageAdapterConfigError,
) -> Result<(), OciObjectStorageAdapterConfigError> {
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

fn encode_object_path(object_key: &str) -> String {
    object_key
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect::<String>()
}

fn canonical_body(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("|")
}
