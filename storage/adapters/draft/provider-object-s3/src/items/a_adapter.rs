#[derive(Clone, Debug, Eq, PartialEq)]
pub enum S3AdapterConfigError {
    InvalidEndpoint,
    InvalidRegion,
    InvalidBucketName,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct S3Adapter {
    endpoint_origin: String,
    region: String,
    bucket_name: String,
    clock_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct S3ObjectCommand {
    pub operation: &'static str,
    pub method: &'static str,
    pub endpoint_origin: String,
    pub path: String,
    pub body_canonical: String,
    pub provider_evidence_ref: String,
}

impl S3Adapter {
    pub fn new(
        endpoint_origin: impl Into<String>,
        region: impl Into<String>,
        bucket_name: impl Into<String>,
    ) -> Result<Self, S3AdapterConfigError> {
        let endpoint_origin = endpoint_origin.into();
        let region = region.into();
        let bucket_name = bucket_name.into();
        validate_endpoint(&endpoint_origin)?;
        validate_region(&region)?;
        validate_bucket_name(&bucket_name)?;
        Ok(Self {
            endpoint_origin,
            region,
            bucket_name,
            clock_epoch_seconds: 0,
        })
    }

    pub fn with_clock(mut self, clock_epoch_seconds: u64) -> Self {
        self.clock_epoch_seconds = clock_epoch_seconds;
        self
    }

    pub fn provider_bucket_ref(&self) -> String {
        format!("s3://{}/{}", self.region, self.bucket_name)
    }

    pub fn put_command(
        &self,
        request: &StorageProviderObjectPutRequest,
    ) -> Result<S3ObjectCommand, StorageProviderObjectError> {
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
    ) -> Result<S3ObjectCommand, StorageProviderObjectError> {
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
    ) -> S3ObjectCommand {
        S3ObjectCommand {
            operation,
            method,
            endpoint_origin: self.endpoint_origin.clone(),
            path: format!("/{}/{}", self.bucket_name, encode_object_path(object_key)),
            body_canonical: canonical_body(fields),
            provider_evidence_ref: format!(
                "s3://{}/{}/{}/{}",
                self.region, self.bucket_name, object_key, request_id
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
                provider: StorageProviderKind::S3ObjectStorage,
                reason: "provider_bucket_ref does not match configured S3 bucket".to_string(),
            })
        }
    }

    fn provider_request_id(&self, request_id: &str) -> String {
        format!("s3-object-{}-{request_id}", self.clock_epoch_seconds)
    }
}

impl StorageProviderObjectPort for S3Adapter {
    fn provider_kind(&self) -> StorageProviderKind {
        StorageProviderKind::S3ObjectStorage
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

fn validate_endpoint(value: &str) -> Result<(), S3AdapterConfigError> {
    if value.starts_with("https://") && no_space_or_control(value) {
        Ok(())
    } else {
        Err(S3AdapterConfigError::InvalidEndpoint)
    }
}

fn validate_region(value: &str) -> Result<(), S3AdapterConfigError> {
    if value.trim().is_empty()
        || value.contains('/')
        || !no_space_or_control(value)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Err(S3AdapterConfigError::InvalidRegion)
    } else {
        Ok(())
    }
}

fn validate_bucket_name(value: &str) -> Result<(), S3AdapterConfigError> {
    if value.len() < 3
        || value.len() > 63
        || !no_space_or_control(value)
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'.'
        })
        || !value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        || value.contains("..")
        || value.contains(".-")
        || value.contains("-.")
    {
        Err(S3AdapterConfigError::InvalidBucketName)
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
