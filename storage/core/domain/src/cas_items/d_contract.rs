#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CasObjectRecord {
    pub address: TenantScopedBlake3Address,
    pub size_bytes: u64,
    pub kms_boundary: TenantKekBoundary,
    pub worm_policy: CasWormPolicy,
    pub audit_anchor: CasAuditAnchor,
    pub durability: CasDurabilityPolicy,
    pub user_metadata: BTreeMap<String, String>,
    pub stored_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectStoreError {
    InvalidTenantId,
    InvalidBlake3Digest,
    InvalidPayload,
    AddressDigestMismatch {
        expected: String,
        actual: String,
    },
    CrossTenantAccessDenied,
    InvalidKekBoundary,
    InvalidWormPolicy,
    ExpiredWormPolicy {
        retain_until_epoch_seconds: u64,
        requested_at_epoch_seconds: u64,
    },
    InvalidAuditAnchor,
    InvalidTransitionalBoundary,
    InvalidRequestTimestamp,
    InvalidUserMetadata,
    InvalidDeleteRequest,
    NotFound {
        tenant_id: String,
        digest: String,
    },
    WormRetentionActive {
        retain_until_epoch_seconds: u64,
        legal_hold: bool,
    },
    DuplicateCasWriteConflict {
        tenant_id: String,
        digest: String,
    },
    BackendUnavailable {
        detail: String,
    },
}

impl fmt::Display for ObjectStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTenantId => write!(f, "invalid tenant id; expected ten_<id>"),
            Self::InvalidBlake3Digest => {
                write!(f, "invalid BLAKE3 digest; expected 64 lowercase hex chars")
            }
            Self::InvalidPayload => write!(f, "invalid CAS payload"),
            Self::AddressDigestMismatch { expected, actual } => write!(
                f,
                "CAS address digest mismatch: expected {expected}, computed {actual}"
            ),
            Self::CrossTenantAccessDenied => write!(f, "cross-tenant CAS access denied"),
            Self::InvalidKekBoundary => write!(f, "invalid tenant KEK boundary"),
            Self::InvalidWormPolicy => write!(f, "invalid WORM policy"),
            Self::ExpiredWormPolicy {
                retain_until_epoch_seconds,
                requested_at_epoch_seconds,
            } => write!(
                f,
                "expired WORM policy: retain_until={retain_until_epoch_seconds} requested_at={requested_at_epoch_seconds}"
            ),
            Self::InvalidAuditAnchor => write!(f, "invalid audit anchor"),
            Self::InvalidTransitionalBoundary => write!(f, "invalid transitional adapter boundary"),
            Self::InvalidRequestTimestamp => write!(f, "request timestamp must be non-zero"),
            Self::InvalidUserMetadata => write!(f, "invalid CAS user metadata"),
            Self::InvalidDeleteRequest => write!(f, "invalid CAS delete request"),
            Self::NotFound { tenant_id, digest } => {
                write!(
                    f,
                    "CAS object not found: tenant={tenant_id} blake3={digest}"
                )
            }
            Self::WormRetentionActive {
                retain_until_epoch_seconds,
                legal_hold,
            } => write!(
                f,
                "CAS object is WORM-protected until {retain_until_epoch_seconds} (legal_hold={legal_hold})"
            ),
            Self::DuplicateCasWriteConflict { tenant_id, digest } => write!(
                f,
                "duplicate CAS write conflicts with existing metadata: tenant={tenant_id} blake3={digest}"
            ),
            Self::BackendUnavailable { detail } => {
                write!(f, "object-store backend unavailable: {detail}")
            }
        }
    }
}

impl std::error::Error for ObjectStoreError {}

/// A successful `put_cas` MUST make the address immediately visible to
/// `head_cas` and `get_cas`; adapters prove that invariant through
/// `run_object_store_conformance_suite`.
pub trait ObjectStore: Send + Sync {
    fn put_cas(
        &self,
        request: CasPutRequest,
        payload: &mut dyn CasPayloadReader,
    ) -> Result<CasObjectRecord, ObjectStoreError>;

    fn head_cas(&self, request: CasReadRequest) -> Result<CasObjectRecord, ObjectStoreError>;

    fn get_cas(
        &self,
        request: CasReadRequest,
        sink: &mut dyn CasPayloadSink,
    ) -> Result<CasObjectRecord, ObjectStoreError>;

    fn delete_cas(&self, request: CasDeleteRequest) -> Result<(), ObjectStoreError>;
}

/// Application code depends on `ObjectStore`; transitional adapter receipts
/// stay on this separate plane.
pub trait ObjectStoreDiagnostics {
    fn backend_kind(&self) -> ObjectStoreBackendKind;

    fn adapter_boundary(
        &self,
        request: CasReadRequest,
    ) -> Result<Option<TransitionalAdapterBoundary>, ObjectStoreError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectStoreConformanceReport {
    pub checks: Vec<&'static str>,
}

/// # Errors
/// Returns the first failed object-store operation or a `BackendUnavailable`
/// detail when an adapter violates a post-condition.
pub fn run_object_store_conformance_suite(
    store: &dyn ObjectStore,
) -> Result<ObjectStoreConformanceReport, ObjectStoreError> {
    let tenant_id = TenantId::parse("ten_conformance")?;
    let chunks = vec![b"object-store ".to_vec(), b"conformance payload".to_vec()];
    let payload = CasPayload::from_chunks(&chunks)?;
    let request = CasPutRequest::new_with_payload(
        tenant_id.clone(),
        payload.clone(),
        TenantKekBoundary::new(
            tenant_id.clone(),
            "kms/ten_conformance/object-store",
            1,
            "ct/ten_conformance/object-store",
            Some("shred/ten_conformance/object-store".to_string()),
        )?,
        CasWormPolicy::compliance_until(1_800_000_000, false),
        CasAuditAnchor::new(
            "audit_evt_object_store_conformance",
            Blake3Digest::for_payload(b"object-store-conformance-chain"),
            1_700_000_001,
        )?,
        1_700_000_010,
    )?;
    let address = request.address.clone();
    let mut reader = InMemoryPayloadReader::from_chunks(chunks.clone())?;
    let record = store.put_cas(request, &mut reader)?;
    let read_request = CasReadRequest::new(tenant_id.clone(), address.clone());

    let head = store.head_cas(read_request.clone())?;
    if head != record {
        return Err(ObjectStoreError::BackendUnavailable {
            detail: "head_cas did not immediately observe put_cas record".to_string(),
        });
    }

    let mut sink = InMemoryPayloadSink::default();
    let get_record = store.get_cas(read_request.clone(), &mut sink)?;
    if get_record != record || sink.payload()? != payload {
        return Err(ObjectStoreError::BackendUnavailable {
            detail: "get_cas did not immediately observe put_cas payload".to_string(),
        });
    }

    let mut cross_tenant_sink = InMemoryPayloadSink::default();
    match store.get_cas(
        CasReadRequest::new(TenantId::parse("ten_conformance_other")?, address.clone()),
        &mut cross_tenant_sink,
    ) {
        Err(ObjectStoreError::CrossTenantAccessDenied) => {}
        Ok(_) => {
            return Err(ObjectStoreError::BackendUnavailable {
                detail: "cross-tenant read was not denied".to_string(),
            });
        }
        Err(error) => return Err(error),
    }

    match store.delete_cas(CasDeleteRequest::new(
        tenant_id.clone(),
        address.clone(),
        1_799_999_999,
        "audit_evt_object_store_conformance_delete",
    )) {
        Err(ObjectStoreError::WormRetentionActive { .. }) => {}
        Ok(()) => {
            return Err(ObjectStoreError::BackendUnavailable {
                detail: "WORM-protected delete was not refused".to_string(),
            });
        }
        Err(error) => return Err(error),
    }

    let other_tenant = TenantId::parse("ten_conformance_other")?;
    let other_request = CasPutRequest::new_with_payload(
        other_tenant.clone(),
        payload.clone(),
        TenantKekBoundary::new(
            other_tenant.clone(),
            "kms/ten_conformance_other/object-store",
            1,
            "ct/ten_conformance_other/object-store",
            Some("shred/ten_conformance_other/object-store".to_string()),
        )?,
        CasWormPolicy::compliance_until(1_800_000_000, false),
        CasAuditAnchor::new(
            "audit_evt_object_store_conformance_other",
            Blake3Digest::for_payload(b"object-store-conformance-chain-other"),
            1_700_000_002,
        )?,
        1_700_000_011,
    )?;
    let other_address = other_request.address.clone();
    let mut other_reader = InMemoryPayloadReader::from_chunks(chunks)?;
    let other_record = store.put_cas(other_request, &mut other_reader)?;
    if other_record.address == record.address || other_address.tenant_id == address.tenant_id {
        return Err(ObjectStoreError::BackendUnavailable {
            detail: "same payload across tenants was not isolated".to_string(),
        });
    }

    Ok(ObjectStoreConformanceReport {
        checks: vec![
            "put_immediate_head_visibility",
            "put_immediate_get_visibility",
            "tenant_isolation",
            "worm_delete_refusal",
            "same_payload_cross_tenant_isolation",
        ],
    })
}
