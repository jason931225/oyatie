
#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredCasObject {
    chunks: Vec<Vec<u8>>,
    record: CasObjectRecord,
}

#[derive(Debug, Default)]
struct InMemoryStorage {
    objects: BTreeMap<TenantScopedBlake3Address, StoredCasObject>,
}

#[derive(Debug)]
pub struct InMemoryObjectStore {
    inner: Mutex<InMemoryStorage>,
    backend_kind: ObjectStoreBackendKind,
    transitional_adapter: Option<(TransitionalAdapterClass, String)>,
}

impl Default for InMemoryObjectStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryObjectStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryStorage::default()),
            backend_kind: ObjectStoreBackendKind::InMemoryReference,
            transitional_adapter: None,
        }
    }

    /// Build an in-memory store that emits transitional adapter boundaries for
    /// tests. This does not change the owned CAS trait surface.
    ///
    /// # Errors
    /// Returns `ObjectStoreError::InvalidTransitionalBoundary` when the adapter
    /// identity is malformed.
    pub fn with_transitional_adapter(
        adapter_class: TransitionalAdapterClass,
        adapter_id: impl Into<String>,
    ) -> Result<Self, ObjectStoreError> {
        let adapter_id = adapter_id.into();
        if !is_valid_reference(&adapter_id) {
            return Err(ObjectStoreError::InvalidTransitionalBoundary);
        }
        Ok(Self {
            inner: Mutex::new(InMemoryStorage::default()),
            backend_kind: ObjectStoreBackendKind::TransitionalAdapter,
            transitional_adapter: Some((adapter_class, adapter_id)),
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lock().objects.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> MutexGuard<'_, InMemoryStorage> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn not_found(address: &TenantScopedBlake3Address) -> ObjectStoreError {
        ObjectStoreError::NotFound {
            tenant_id: address.tenant_id.as_str().to_string(),
            digest: address.digest.as_str().to_string(),
        }
    }

    fn transitional_boundary_for(
        &self,
        address: &TenantScopedBlake3Address,
    ) -> Result<Option<TransitionalAdapterBoundary>, ObjectStoreError> {
        let Some((adapter_class, adapter_id)) = &self.transitional_adapter else {
            return Ok(None);
        };
        TransitionalAdapterBoundary::new(
            *adapter_class,
            adapter_id.clone(),
            format!(
                "adapter/{}/{}/{}",
                adapter_class.label(),
                adapter_id,
                address.tenant_id
            ),
            format!("cas/{}/{}", address.tenant_id, address.digest),
            format!(
                "evidence/object-store/{}/{}",
                address.tenant_id, address.digest
            ),
        )
        .map(Some)
    }
}

impl ObjectStore for InMemoryObjectStore {
    fn put_cas(
        &self,
        request: CasPutRequest,
        payload_reader: &mut dyn CasPayloadReader,
    ) -> Result<CasObjectRecord, ObjectStoreError> {
        request.validate()?;
        let chunks = read_payload_chunks(payload_reader)?;
        let observed_payload = CasPayload::from_chunks(&chunks)?;
        if observed_payload != request.payload {
            return Err(ObjectStoreError::AddressDigestMismatch {
                expected: request.payload.root_digest.as_str().to_string(),
                actual: observed_payload.root_digest.as_str().to_string(),
            });
        }

        let mut store = self.lock();
        if let Some(existing) = store.objects.get(&request.address) {
            if CasPayload::from_chunks(&existing.chunks)? == request.payload {
                if stored_record_matches_put(&existing.record, &request) {
                    return Ok(existing.record.clone());
                }
                return Err(ObjectStoreError::DuplicateCasWriteConflict {
                    tenant_id: request.address.tenant_id.as_str().to_string(),
                    digest: request.address.digest.as_str().to_string(),
                });
            }
            return Err(ObjectStoreError::DuplicateCasWriteConflict {
                tenant_id: request.address.tenant_id.as_str().to_string(),
                digest: request.address.digest.as_str().to_string(),
            });
        }

        let record = CasObjectRecord {
            address: request.address.clone(),
            size_bytes: request.payload.total_size_bytes,
            kms_boundary: request.kms_boundary,
            worm_policy: request.worm_policy,
            audit_anchor: request.audit_anchor,
            durability: request.durability,
            user_metadata: request.user_metadata,
            stored_at_epoch_seconds: request.requested_at_epoch_seconds,
        };

        store.objects.insert(
            record.address.clone(),
            StoredCasObject {
                chunks,
                record: record.clone(),
            },
        );
        Ok(record)
    }

    fn head_cas(&self, request: CasReadRequest) -> Result<CasObjectRecord, ObjectStoreError> {
        request.validate()?;
        let store = self.lock();
        store
            .objects
            .get(&request.address)
            .map(|stored| stored.record.clone())
            .ok_or_else(|| Self::not_found(&request.address))
    }

    fn get_cas(
        &self,
        request: CasReadRequest,
        sink: &mut dyn CasPayloadSink,
    ) -> Result<CasObjectRecord, ObjectStoreError> {
        request.validate()?;
        let store = self.lock();
        let stored = store
            .objects
            .get(&request.address)
            .ok_or_else(|| Self::not_found(&request.address))?
            .clone();
        drop(store);
        for chunk in &stored.chunks {
            sink.write_chunk(chunk)?;
        }
        Ok(stored.record)
    }

    fn delete_cas(&self, request: CasDeleteRequest) -> Result<(), ObjectStoreError> {
        request.validate()?;
        let mut store = self.lock();
        let stored = store
            .objects
            .get(&request.address)
            .ok_or_else(|| Self::not_found(&request.address))?;
        if stored
            .record
            .worm_policy
            .deletion_protected_at(request.requested_at_epoch_seconds)
        {
            return Err(ObjectStoreError::WormRetentionActive {
                retain_until_epoch_seconds: stored.record.worm_policy.retain_until_epoch_seconds,
                legal_hold: stored.record.worm_policy.legal_hold,
            });
        }
        store.objects.remove(&request.address);
        Ok(())
    }
}

impl ObjectStoreDiagnostics for InMemoryObjectStore {
    fn backend_kind(&self) -> ObjectStoreBackendKind {
        self.backend_kind
    }

    fn adapter_boundary(
        &self,
        request: CasReadRequest,
    ) -> Result<Option<TransitionalAdapterBoundary>, ObjectStoreError> {
        request.validate()?;
        let store = self.lock();
        let stored = store
            .objects
            .get(&request.address)
            .ok_or_else(|| Self::not_found(&request.address))?;
        self.transitional_boundary_for(&stored.record.address)
    }
}

