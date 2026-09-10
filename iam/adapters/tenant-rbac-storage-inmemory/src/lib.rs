//! NOT FOR PRODUCTION: this adapter is process-local and volatile, losing every
//! record on restart. It exists to pin the repository and idempotency contract
//! that a durable adapter must then satisfy.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use iam_tenant_rbac_domain::{GroupCloseRollup, TenantRbacPolicyDecision};
use iam_tenant_rbac_usecase::{
    CrossServiceWorkflowEnvelope, IncidentRollbackEnvelope, TenantRbacOpsEnvelope,
};

const POLICY_ADMISSION_TOPIC: &str = "policy.tenant-rbac.service-write.admission";
const GROUP_CLOSE_ROLLUP_TOPIC: &str = "projection.tenant-rbac.group-close.rollup";
const IN_MEMORY_BACKEND_LABEL: &str = "in-memory-reference";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TenantRbacStoredRecordKind {
    PolicyAdmission,
    GroupCloseRollup,
    CrossServiceWorkflowPlan,
    IncidentRollbackPlan,
    OpsCommand,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantRbacStoredRecord {
    pub kind: TenantRbacStoredRecordKind, // data_class: INTERNAL_ONLY
    pub topic: String,                    // data_class: INTERNAL_ONLY
    pub tenant_id: String,                // data_class: INTERNAL_ONLY
    pub primary_ref: String,              // data_class: INTERNAL_ONLY
    pub idempotency_key: String,          // data_class: INTERNAL_ONLY
    pub payload_data_class: String,       // data_class: INTERNAL_ONLY
    pub schema_version: u32,              // data_class: PUBLIC
    pub storage_backend: String,          // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantRbacStorageCapabilities {
    pub adapter: String,                   // data_class: PUBLIC
    pub durable_backend_attached: bool,    // data_class: PUBLIC
    pub postgres_rls_attached: bool,       // data_class: PUBLIC
    pub cloud_object_store_attached: bool, // data_class: PUBLIC
    pub runtime_write_path_attached: bool, // data_class: PUBLIC
    pub workflow_execution_attached: bool, // data_class: PUBLIC
    pub schema_version: u32,               // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TenantRbacStorageError {
    DuplicateIdempotencyKey(String),
    InvalidIdempotencyKey(String),
    MissingRecord(String),
}

pub trait TenantRbacStoragePort {
    fn reserve_idempotency_key(&mut self, key: &str) -> Result<(), TenantRbacStorageError>;
    fn put_record(&mut self, record: TenantRbacStoredRecord) -> Result<(), TenantRbacStorageError>;
    fn get_record(&self, idempotency_key: &str) -> Option<&TenantRbacStoredRecord>;
    fn require_record(
        &self,
        idempotency_key: &str,
    ) -> Result<&TenantRbacStoredRecord, TenantRbacStorageError>;
    fn list_records(&self) -> Vec<&TenantRbacStoredRecord>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryTenantRbacStore {
    records_by_idempotency_key: BTreeMap<String, TenantRbacStoredRecord>,
    reserved_idempotency_keys: BTreeSet<String>,
}

impl InMemoryTenantRbacStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn persist_policy_decision(
        &mut self,
        decision: &TenantRbacPolicyDecision,
    ) -> Result<TenantRbacStoredRecord, TenantRbacStorageError> {
        let record = TenantRbacStoredRecord {
            kind: TenantRbacStoredRecordKind::PolicyAdmission,
            topic: POLICY_ADMISSION_TOPIC.to_owned(),
            tenant_id: decision.tenant_id.value.value.clone(),
            primary_ref: decision.legal_entity_id.value.value.clone(),
            idempotency_key: decision.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", decision.payload_data_class.value),
            schema_version: decision.schema_version.value,
            storage_backend: IN_MEMORY_BACKEND_LABEL.to_owned(),
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_group_close_rollup(
        &mut self,
        rollup: &GroupCloseRollup,
    ) -> Result<TenantRbacStoredRecord, TenantRbacStorageError> {
        let record = TenantRbacStoredRecord {
            kind: TenantRbacStoredRecordKind::GroupCloseRollup,
            topic: GROUP_CLOSE_ROLLUP_TOPIC.to_owned(),
            tenant_id: rollup.tenant_id.value.value.clone(),
            primary_ref: rollup.group_id.value.value.clone(),
            idempotency_key: group_close_rollup_key(rollup),
            payload_data_class: "InternalOnly".to_owned(),
            schema_version: rollup.schema_version.value,
            storage_backend: IN_MEMORY_BACKEND_LABEL.to_owned(),
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_cross_service_workflow(
        &mut self,
        envelope: &CrossServiceWorkflowEnvelope,
    ) -> Result<TenantRbacStoredRecord, TenantRbacStorageError> {
        let record = TenantRbacStoredRecord {
            kind: TenantRbacStoredRecordKind::CrossServiceWorkflowPlan,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            primary_ref: envelope.workflow_ref.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            schema_version: envelope.schema_version.value,
            storage_backend: IN_MEMORY_BACKEND_LABEL.to_owned(),
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_incident_rollback(
        &mut self,
        envelope: &IncidentRollbackEnvelope,
    ) -> Result<TenantRbacStoredRecord, TenantRbacStorageError> {
        let record = TenantRbacStoredRecord {
            kind: TenantRbacStoredRecordKind::IncidentRollbackPlan,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            primary_ref: envelope.incident_id.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            schema_version: envelope.schema_version.value,
            storage_backend: IN_MEMORY_BACKEND_LABEL.to_owned(),
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_ops_command(
        &mut self,
        envelope: &TenantRbacOpsEnvelope,
    ) -> Result<TenantRbacStoredRecord, TenantRbacStorageError> {
        let record = TenantRbacStoredRecord {
            kind: TenantRbacStoredRecordKind::OpsCommand,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            primary_ref: envelope.change_plan_ref.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            schema_version: envelope.schema_version.value,
            storage_backend: IN_MEMORY_BACKEND_LABEL.to_owned(),
        };
        self.put_record(record.clone())?;
        Ok(record)
    }
}

impl TenantRbacStoragePort for InMemoryTenantRbacStore {
    fn reserve_idempotency_key(&mut self, key: &str) -> Result<(), TenantRbacStorageError> {
        validate_idempotency_key(key)?;
        if self.records_by_idempotency_key.contains_key(key)
            || !self.reserved_idempotency_keys.insert(key.to_owned())
        {
            return Err(TenantRbacStorageError::DuplicateIdempotencyKey(
                key.to_owned(),
            ));
        }
        Ok(())
    }

    fn put_record(&mut self, record: TenantRbacStoredRecord) -> Result<(), TenantRbacStorageError> {
        validate_idempotency_key(&record.idempotency_key)?;
        if self
            .records_by_idempotency_key
            .contains_key(&record.idempotency_key)
        {
            return Err(TenantRbacStorageError::DuplicateIdempotencyKey(
                record.idempotency_key,
            ));
        }
        self.reserved_idempotency_keys
            .remove(&record.idempotency_key);
        self.records_by_idempotency_key
            .insert(record.idempotency_key.clone(), record);
        Ok(())
    }

    fn get_record(&self, idempotency_key: &str) -> Option<&TenantRbacStoredRecord> {
        self.records_by_idempotency_key.get(idempotency_key)
    }

    fn require_record(
        &self,
        idempotency_key: &str,
    ) -> Result<&TenantRbacStoredRecord, TenantRbacStorageError> {
        self.get_record(idempotency_key)
            .ok_or_else(|| TenantRbacStorageError::MissingRecord(idempotency_key.to_owned()))
    }

    fn list_records(&self) -> Vec<&TenantRbacStoredRecord> {
        self.records_by_idempotency_key.values().collect()
    }

    fn len(&self) -> usize {
        self.records_by_idempotency_key.len()
    }

    fn is_empty(&self) -> bool {
        self.records_by_idempotency_key.is_empty()
    }
}

pub fn tenant_rbac_storage_capabilities() -> TenantRbacStorageCapabilities {
    TenantRbacStorageCapabilities {
        adapter: IN_MEMORY_BACKEND_LABEL.to_owned(),
        durable_backend_attached: false,
        postgres_rls_attached: false,
        cloud_object_store_attached: false,
        runtime_write_path_attached: false,
        workflow_execution_attached: false,
        schema_version: 1,
    }
}

pub fn group_close_rollup_key(rollup: &GroupCloseRollup) -> String {
    format!(
        "{}:{}:group-close-rollup",
        rollup.tenant_id.value.value, rollup.group_id.value.value
    )
}

fn validate_idempotency_key(key: &str) -> Result<(), TenantRbacStorageError> {
    if key.trim().is_empty()
        || key.trim() != key
        || key.contains("..")
        || key.chars().any(char::is_whitespace)
    {
        return Err(TenantRbacStorageError::InvalidIdempotencyKey(
            key.to_owned(),
        ));
    }
    Ok(())
}
