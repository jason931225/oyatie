const BLAKE3_HEX_LEN: usize = 64;
const TENANT_ID_PREFIX: &str = "ten_";
const MAX_TENANT_ID_LEN: usize = 128;
const MAX_REFERENCE_LEN: usize = 512;
const MAX_PAYLOAD_CHUNK_BYTES: usize = 16 * 1024 * 1024;


#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TenantId(String);

impl TenantId {
    /// Parse a canonical tenant id (`ten_...`).
    pub fn parse(value: &str) -> Result<Self, ObjectStoreError> {
        if value.len() <= TENANT_ID_PREFIX.len()
            || value.len() > MAX_TENANT_ID_LEN
            || !value.starts_with(TENANT_ID_PREFIX)
            || !value[TENANT_ID_PREFIX.len()..].bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
            })
        {
            return Err(ObjectStoreError::InvalidTenantId);
        }
        Ok(Self(value.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// BLAKE3 digest encoded as 64 lowercase hex characters.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Blake3Digest(String);

impl Blake3Digest {
    #[must_use]
    pub fn for_payload(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes).to_hex().to_string())
    }

    /// Parse a lower-case BLAKE3 hex digest.
    pub fn parse(value: &str) -> Result<Self, ObjectStoreError> {
        if is_lower_hex(value, BLAKE3_HEX_LEN) {
            Ok(Self(value.to_string()))
        } else {
            Err(ObjectStoreError::InvalidBlake3Digest)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Blake3Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The owned content address: `(tenant_id, blake3(payload))`.
///
/// The tenant component is not decoration. It is the anti-dedup boundary that
/// preserves the storage owner contract's crypto-shred boundary and prevents
/// cross-tenant side channels.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TenantScopedBlake3Address {
    pub tenant_id: TenantId,
    pub digest: Blake3Digest,
}

impl TenantScopedBlake3Address {
    #[must_use]
    pub fn for_payload(tenant_id: TenantId, bytes: &[u8]) -> Self {
        Self::for_digest(tenant_id, Blake3Digest::for_payload(bytes))
    }

    #[must_use]
    pub const fn for_digest(tenant_id: TenantId, digest: Blake3Digest) -> Self {
        Self { tenant_id, digest }
    }

    /// Build an address from already-computed parts.
    pub fn parse(tenant_id: &str, digest: &str) -> Result<Self, ObjectStoreError> {
        Ok(Self {
            tenant_id: TenantId::parse(tenant_id)?,
            digest: Blake3Digest::parse(digest)?,
        })
    }

    #[must_use]
    pub fn canonical(&self) -> String {
        format!("cas://{}/blake3/{}", self.tenant_id, self.digest)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TenantKekBoundary {
    pub tenant_id: TenantId,
    pub kms_key_ref: String,
    pub kms_key_version: u32,
    pub ciphertext_ref: String,
    pub crypto_shred_ref: Option<String>,
}

impl TenantKekBoundary {
    pub fn new(
        tenant_id: TenantId,
        kms_key_ref: impl Into<String>,
        kms_key_version: u32,
        ciphertext_ref: impl Into<String>,
        crypto_shred_ref: Option<String>,
    ) -> Result<Self, ObjectStoreError> {
        let boundary = Self {
            tenant_id,
            kms_key_ref: kms_key_ref.into(),
            kms_key_version,
            ciphertext_ref: ciphertext_ref.into(),
            crypto_shred_ref,
        };
        boundary.validate()?;
        Ok(boundary)
    }

    fn validate(&self) -> Result<(), ObjectStoreError> {
        if self.kms_key_version == 0
            || !is_valid_reference(&self.kms_key_ref)
            || !is_valid_reference(&self.ciphertext_ref)
            || self
                .crypto_shred_ref
                .as_ref()
                .is_some_and(|reference| !is_valid_reference(reference))
        {
            return Err(ObjectStoreError::InvalidKekBoundary);
        }
        Ok(())
    }
}

/// Object-Lock-style WORM mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CasWormMode {
    Governance,
    Compliance,
}

impl CasWormMode {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Governance => "governance",
            Self::Compliance => "compliance",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CasWormPolicy {
    pub mode: CasWormMode,
    pub retain_until_epoch_seconds: u64,
    pub legal_hold: bool,
}

impl CasWormPolicy {
    #[must_use]
    pub const fn governance_until(retain_until_epoch_seconds: u64, legal_hold: bool) -> Self {
        Self {
            mode: CasWormMode::Governance,
            retain_until_epoch_seconds,
            legal_hold,
        }
    }

    #[must_use]
    pub const fn compliance_until(retain_until_epoch_seconds: u64, legal_hold: bool) -> Self {
        Self {
            mode: CasWormMode::Compliance,
            retain_until_epoch_seconds,
            legal_hold,
        }
    }

    fn validate(&self) -> Result<(), ObjectStoreError> {
        if self.retain_until_epoch_seconds == 0 && !self.legal_hold {
            return Err(ObjectStoreError::InvalidWormPolicy);
        }
        Ok(())
    }

    #[must_use]
    pub const fn write_protected_after(&self, epoch_seconds: u64) -> bool {
        self.legal_hold || self.retain_until_epoch_seconds > epoch_seconds
    }

    #[must_use]
    pub const fn deletion_protected_at(&self, epoch_seconds: u64) -> bool {
        self.legal_hold || epoch_seconds < self.retain_until_epoch_seconds
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CasAuditAnchor {
    pub audit_event_id: String,
    pub digest_chain_head: Blake3Digest,
    pub anchored_at_epoch_seconds: u64,
}

impl CasAuditAnchor {
    pub fn new(
        audit_event_id: impl Into<String>,
        digest_chain_head: Blake3Digest,
        anchored_at_epoch_seconds: u64,
    ) -> Result<Self, ObjectStoreError> {
        let anchor = Self {
            audit_event_id: audit_event_id.into(),
            digest_chain_head,
            anchored_at_epoch_seconds,
        };
        anchor.validate()?;
        Ok(anchor)
    }

    fn validate(&self) -> Result<(), ObjectStoreError> {
        if !is_valid_reference(&self.audit_event_id) || self.anchored_at_epoch_seconds == 0 {
            return Err(ObjectStoreError::InvalidAuditAnchor);
        }
        Ok(())
    }
}
