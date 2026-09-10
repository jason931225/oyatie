//! This crate does not serialize protobufs or publish events; runtime
//! adapters use these validated values later when they bind transports.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolParityError {
    MissingField {
        field: &'static str,
    },
    InvalidAsyncApiEventKind {
        value: String,
    },
    InvalidSchemaVersion {
        value: String,
    },
    ReceiptEventTypeMismatch {
        expected: &'static str,
        actual: &'static str,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolParityBinding {
    pub rest_operation_id: &'static str,
    pub asyncapi_operation_id: &'static str,
    pub asyncapi_channel_address: &'static str,
    pub asyncapi_message_name: &'static str,
    pub asyncapi_event_kind: &'static str,
    pub receipt_event_type: &'static str,
    pub proto_package: &'static str,
    pub proto_service: &'static str,
    pub proto_rpc: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolParityBindingSpec {
    pub rest_operation_id: &'static str,
    pub asyncapi_operation_id: &'static str,
    pub asyncapi_channel_address: &'static str,
    pub asyncapi_message_name: &'static str,
    pub asyncapi_event_kind: &'static str,
    pub receipt_event_type: &'static str,
    pub proto_package: &'static str,
    pub proto_service: &'static str,
    pub proto_rpc: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolEventEnvelope {
    pub binding: ProtocolParityBinding,
    pub schema_version: String,
    pub tenant_scope_ref: String,
    pub aggregate_id: String,
    pub audit_correlation_id: String,
    pub idempotency_key: Option<String>,
    pub policy_decision_ref: String,
}

impl ProtocolParityBinding {
    pub fn new(spec: ProtocolParityBindingSpec) -> Result<Self, ProtocolParityError> {
        require_static(spec.rest_operation_id, "rest_operation_id")?;
        require_static(spec.asyncapi_operation_id, "asyncapi_operation_id")?;
        require_static(spec.asyncapi_channel_address, "asyncapi_channel_address")?;
        require_static(spec.asyncapi_message_name, "asyncapi_message_name")?;
        require_static(spec.receipt_event_type, "receipt_event_type")?;
        require_static(spec.proto_package, "proto_package")?;
        require_static(spec.proto_service, "proto_service")?;
        require_static(spec.proto_rpc, "proto_rpc")?;
        if !spec.asyncapi_event_kind.starts_with("oya.")
            || !spec.asyncapi_event_kind.ends_with(".v1")
        {
            return Err(ProtocolParityError::InvalidAsyncApiEventKind {
                value: spec.asyncapi_event_kind.to_string(),
            });
        }
        Ok(Self {
            rest_operation_id: spec.rest_operation_id,
            asyncapi_operation_id: spec.asyncapi_operation_id,
            asyncapi_channel_address: spec.asyncapi_channel_address,
            asyncapi_message_name: spec.asyncapi_message_name,
            asyncapi_event_kind: spec.asyncapi_event_kind,
            receipt_event_type: spec.receipt_event_type,
            proto_package: spec.proto_package,
            proto_service: spec.proto_service,
            proto_rpc: spec.proto_rpc,
        })
    }
}

impl ProtocolEventEnvelope {
    pub fn new(
        binding: ProtocolParityBinding,
        schema_version: impl Into<String>,
        tenant_scope_ref: impl Into<String>,
        aggregate_id: impl Into<String>,
        audit_correlation_id: impl Into<String>,
        idempotency_key: Option<String>,
        policy_decision_ref: impl Into<String>,
    ) -> Result<Self, ProtocolParityError> {
        let envelope = Self {
            binding,
            schema_version: schema_version.into(),
            tenant_scope_ref: tenant_scope_ref.into(),
            aggregate_id: aggregate_id.into(),
            audit_correlation_id: audit_correlation_id.into(),
            idempotency_key,
            policy_decision_ref: policy_decision_ref.into(),
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), ProtocolParityError> {
        require_owned(&self.schema_version, "schema_version")?;
        if !is_semver_core(&self.schema_version) {
            return Err(ProtocolParityError::InvalidSchemaVersion {
                value: self.schema_version.clone(),
            });
        }
        require_owned(&self.tenant_scope_ref, "tenant_scope_ref")?;
        require_owned(&self.aggregate_id, "aggregate_id")?;
        require_owned(&self.audit_correlation_id, "audit_correlation_id")?;
        require_owned(&self.policy_decision_ref, "policy_decision_ref")?;
        if let Some(idempotency_key) = &self.idempotency_key {
            require_owned(idempotency_key, "idempotency_key")?;
        }
        Ok(())
    }
}

pub fn require_receipt_event_type(
    binding: &ProtocolParityBinding,
    actual: &'static str,
) -> Result<(), ProtocolParityError> {
    if binding.receipt_event_type == actual {
        Ok(())
    } else {
        Err(ProtocolParityError::ReceiptEventTypeMismatch {
            expected: binding.receipt_event_type,
            actual,
        })
    }
}

fn require_static(value: &'static str, field: &'static str) -> Result<(), ProtocolParityError> {
    if value.trim().is_empty() {
        Err(ProtocolParityError::MissingField { field })
    } else {
        Ok(())
    }
}

fn require_owned(value: &str, field: &'static str) -> Result<(), ProtocolParityError> {
    if value.trim().is_empty() {
        Err(ProtocolParityError::MissingField { field })
    } else {
        Ok(())
    }
}

fn is_semver_core(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(major) = parts.next() else {
        return false;
    };
    let Some(minor) = parts.next() else {
        return false;
    };
    let Some(patch) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    [major, minor, patch].iter().all(|part| {
        !part.is_empty()
            && part.chars().all(|character| character.is_ascii_digit())
            && (*part == "0" || !part.starts_with('0'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> ProtocolParityBinding {
        ProtocolParityBinding::new(ProtocolParityBindingSpec {
            rest_operation_id: "postMessage",
            asyncapi_operation_id: "emitMessagePosted",
            asyncapi_channel_address: "workflow-events/messenger.message.posted",
            asyncapi_message_name: "MessagePosted",
            asyncapi_event_kind: "oya.messenger.message.posted.v1",
            receipt_event_type: "messenger.message.sent",
            proto_package: "oya.messenger.v1",
            proto_service: "MessageStream",
            proto_rpc: "PostMessage",
        })
        .unwrap()
    }

    #[test]
    fn binding_requires_asyncapi_event_kind_shape() {
        assert_eq!(
            binding().asyncapi_event_kind,
            "oya.messenger.message.posted.v1"
        );
        assert_eq!(
            ProtocolParityBinding::new(ProtocolParityBindingSpec {
                rest_operation_id: "postMessage",
                asyncapi_operation_id: "emitMessagePosted",
                asyncapi_channel_address: "workflow-events/messenger.message.posted",
                asyncapi_message_name: "MessagePosted",
                asyncapi_event_kind: "messenger.message.posted",
                receipt_event_type: "messenger.message.sent",
                proto_package: "oya.messenger.v1",
                proto_service: "MessageStream",
                proto_rpc: "PostMessage",
            }),
            Err(ProtocolParityError::InvalidAsyncApiEventKind {
                value: "messenger.message.posted".into()
            })
        );
    }

    #[test]
    fn envelope_rejects_missing_audit_and_invalid_schema_version() {
        assert_eq!(
            ProtocolEventEnvelope::new(
                binding(),
                "1.0.0",
                "tenant:t",
                "message:m",
                "",
                Some("idem".into()),
                "policy",
            ),
            Err(ProtocolParityError::MissingField {
                field: "audit_correlation_id"
            })
        );
        assert_eq!(
            ProtocolEventEnvelope::new(
                binding(),
                "v1",
                "tenant:t",
                "message:m",
                "audit",
                Some("idem".into()),
                "policy",
            ),
            Err(ProtocolParityError::InvalidSchemaVersion { value: "v1".into() })
        );
    }

    #[test]
    fn schema_version_rejects_leading_zeroes() {
        assert!(is_semver_core("0.0.0"));
        for invalid in ["01.0.0", "1.00.0", "1.0.00"] {
            assert!(!is_semver_core(invalid));
        }
    }

    #[test]
    fn envelope_allows_missing_idempotency_for_non_idempotent_receipts() {
        let envelope = ProtocolEventEnvelope::new(
            binding(),
            "1.0.0",
            "tenant:t",
            "message:m",
            "audit",
            None,
            "policy",
        )
        .unwrap();
        assert_eq!(envelope.idempotency_key, None);
    }

    #[test]
    fn receipt_event_type_must_match_binding_when_checked() {
        assert_eq!(
            require_receipt_event_type(&binding(), "messenger.message.sent"),
            Ok(())
        );
        assert_eq!(
            require_receipt_event_type(&binding(), "messenger.message.posted"),
            Err(ProtocolParityError::ReceiptEventTypeMismatch {
                expected: "messenger.message.sent",
                actual: "messenger.message.posted"
            })
        );
    }
}
