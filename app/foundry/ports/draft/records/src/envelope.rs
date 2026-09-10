const MAX_IDENTITY_FIELD_BYTES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvelopeError {
    Empty { field: &'static str },
    NotTrimmed { field: &'static str },
    TooLong { field: &'static str },
    ZeroSchemaRevision,
}

/// One Action, as appended: the unit of write in Foundry.
///
/// The payload is opaque bytes on purpose — the log stores what the Action
/// declared, and projection logic interprets it under `schema_revision`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionEnvelope {
    pub tenant_id: String,         // data_class: INTERNAL_ONLY
    pub object_ref: String,        // data_class: INTERNAL_ONLY
    pub action_type: String,       // data_class: INTERNAL_ONLY
    pub idempotency_key: String,   // data_class: INTERNAL_ONLY
    pub schema_revision: u32,      // data_class: INTERNAL_ONLY
    pub payload: Vec<u8>,          // data_class: PII_IDENTIFYING
    pub observed_at_epoch_ms: u64, // data_class: INTERNAL_ONLY
}

impl ActionEnvelope {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: impl Into<String>,
        object_ref: impl Into<String>,
        action_type: impl Into<String>,
        idempotency_key: impl Into<String>,
        schema_revision: u32,
        payload: Vec<u8>,
        observed_at_epoch_ms: u64,
    ) -> Result<Self, EnvelopeError> {
        let envelope = Self {
            tenant_id: tenant_id.into(),
            object_ref: object_ref.into(),
            action_type: action_type.into(),
            idempotency_key: idempotency_key.into(),
            schema_revision,
            payload,
            observed_at_epoch_ms,
        };
        for (field, value) in [
            ("tenant_id", &envelope.tenant_id),
            ("object_ref", &envelope.object_ref),
            ("action_type", &envelope.action_type),
            ("idempotency_key", &envelope.idempotency_key),
        ] {
            if value.trim().is_empty() {
                return Err(EnvelopeError::Empty { field });
            }
            if value.trim() != value {
                return Err(EnvelopeError::NotTrimmed { field });
            }
            if value.len() > MAX_IDENTITY_FIELD_BYTES {
                return Err(EnvelopeError::TooLong { field });
            }
        }
        if envelope.schema_revision == 0 {
            return Err(EnvelopeError::ZeroSchemaRevision);
        }
        Ok(envelope)
    }
}

/// Proof of one append.
///
/// `ordinal` is dense and total per tenant; `object_sequence` is dense per
/// `(tenant, object_ref)`. A deduplicated receipt restates the original
/// append's positions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub ordinal: u64,         // data_class: INTERNAL_ONLY
    pub object_sequence: u64, // data_class: INTERNAL_ONLY
    pub deduplicated: bool,   // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedEnvelope {
    pub envelope: ActionEnvelope, // data_class: PII_IDENTIFYING
    pub receipt: Receipt,         // data_class: INTERNAL_ONLY
}
