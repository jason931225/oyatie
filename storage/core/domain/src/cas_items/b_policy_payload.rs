#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CasWritePath {
    ChainReplication3x,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CasRepairPath {
    LrcErasureCoding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CasDurabilityPolicy {
    pub write_path: CasWritePath,
    pub repair_path: CasRepairPath,
}

impl Default for CasDurabilityPolicy {
    fn default() -> Self {
        Self {
            write_path: CasWritePath::ChainReplication3x,
            repair_path: CasRepairPath::LrcErasureCoding,
        }
    }
}

/// Storage backend kind exposed to diagnostics. Transitional bridge details
/// collapse to a destination-neutral adapter class so the stable interface does
/// not freeze today’s bridge implementations into the owned CAS contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ObjectStoreBackendKind {
    InMemoryReference,
    OwnedCas,
    TransitionalAdapter,
}

impl ObjectStoreBackendKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::InMemoryReference => "in_memory_reference",
            Self::OwnedCas => "owned_cas",
            Self::TransitionalAdapter => "transitional_adapter",
        }
    }

    #[must_use]
    pub const fn is_transitional(self) -> bool {
        matches!(self, Self::TransitionalAdapter)
    }
}

/// Destination-neutral class for a transitional object-store adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TransitionalAdapterClass {
    ProtocolCompatible,
    ObjectGateway,
    BlobCompatible,
}

impl TransitionalAdapterClass {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ProtocolCompatible => "protocol_compatible",
            Self::ObjectGateway => "object_gateway",
            Self::BlobCompatible => "blob_compatible",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TransitionalAdapterBoundary {
    pub adapter_class: TransitionalAdapterClass,
    pub adapter_id: String,
    pub adapter_namespace: String,
    pub adapter_object_ref: String,
    pub adapter_evidence_ref: String,
}

impl TransitionalAdapterBoundary {
    pub fn new(
        adapter_class: TransitionalAdapterClass,
        adapter_id: impl Into<String>,
        adapter_namespace: impl Into<String>,
        adapter_object_ref: impl Into<String>,
        adapter_evidence_ref: impl Into<String>,
    ) -> Result<Self, ObjectStoreError> {
        let boundary = Self {
            adapter_class,
            adapter_id: adapter_id.into(),
            adapter_namespace: adapter_namespace.into(),
            adapter_object_ref: adapter_object_ref.into(),
            adapter_evidence_ref: adapter_evidence_ref.into(),
        };
        boundary.validate()?;
        Ok(boundary)
    }

    fn validate(&self) -> Result<(), ObjectStoreError> {
        if !is_valid_reference(&self.adapter_id)
            || !is_valid_reference(&self.adapter_namespace)
            || !is_valid_reference(&self.adapter_object_ref)
            || !is_valid_reference(&self.adapter_evidence_ref)
        {
            return Err(ObjectStoreError::InvalidTransitionalBoundary);
        }
        Ok(())
    }
}

/// One payload chunk in the destination CAS write/read contract. The trait is
/// chunk-aware so real adapters do not have to pretend infinite-scale objects
/// are whole-buffer values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CasPayloadChunk {
    pub ordinal: u32,
    pub size_bytes: u64,
    pub digest: Blake3Digest,
}

/// Chunked CAS payload manifest with a root BLAKE3 digest over the ordered
/// bytes. It deliberately carries digests and sizes, not object bytes; bytes
/// flow through `CasPayloadReader` / `CasPayloadSink`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CasPayload {
    pub total_size_bytes: u64,
    pub root_digest: Blake3Digest,
    pub chunks: Vec<CasPayloadChunk>,
}

impl CasPayload {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ObjectStoreError> {
        Self::from_chunks(&[bytes.to_vec()])
    }

    pub fn from_chunks(chunks: &[Vec<u8>]) -> Result<Self, ObjectStoreError> {
        if chunks.is_empty() {
            return Err(ObjectStoreError::InvalidPayload);
        }
        if chunks.len() > 1 && chunks.iter().any(Vec::is_empty) {
            return Err(ObjectStoreError::InvalidPayload);
        }
        if chunks
            .iter()
            .any(|chunk| chunk.len() > MAX_PAYLOAD_CHUNK_BYTES)
        {
            return Err(ObjectStoreError::InvalidPayload);
        }

        let mut hasher = blake3::Hasher::new();
        let mut total_size_bytes = 0_u64;
        let mut payload_chunks = Vec::with_capacity(chunks.len());
        for (ordinal, bytes) in chunks.iter().enumerate() {
            let ordinal = u32::try_from(ordinal).map_err(|_| ObjectStoreError::InvalidPayload)?;
            total_size_bytes = total_size_bytes
                .checked_add(bytes.len() as u64)
                .ok_or(ObjectStoreError::InvalidPayload)?;
            hasher.update(bytes);
            payload_chunks.push(CasPayloadChunk {
                ordinal,
                size_bytes: bytes.len() as u64,
                digest: Blake3Digest::for_payload(bytes),
            });
        }

        Ok(Self {
            total_size_bytes,
            root_digest: Blake3Digest(hasher.finalize().to_hex().to_string()),
            chunks: payload_chunks,
        })
    }

    fn validate(&self) -> Result<(), ObjectStoreError> {
        if self.chunks.is_empty() {
            return Err(ObjectStoreError::InvalidPayload);
        }
        if self.chunks.len() > 1 && self.chunks.iter().any(|chunk| chunk.size_bytes == 0) {
            return Err(ObjectStoreError::InvalidPayload);
        }
        if self
            .chunks
            .iter()
            .any(|chunk| chunk.size_bytes > MAX_PAYLOAD_CHUNK_BYTES as u64)
        {
            return Err(ObjectStoreError::InvalidPayload);
        }

        let mut total_size_bytes = 0_u64;
        for (expected_ordinal, chunk) in self.chunks.iter().enumerate() {
            let expected_ordinal =
                u32::try_from(expected_ordinal).map_err(|_| ObjectStoreError::InvalidPayload)?;
            if chunk.ordinal != expected_ordinal {
                return Err(ObjectStoreError::InvalidPayload);
            }
            total_size_bytes = total_size_bytes
                .checked_add(chunk.size_bytes)
                .ok_or(ObjectStoreError::InvalidPayload)?;
        }
        if total_size_bytes != self.total_size_bytes {
            return Err(ObjectStoreError::InvalidPayload);
        }
        Ok(())
    }
}
