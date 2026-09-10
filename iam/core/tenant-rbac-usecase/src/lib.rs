//! Prepares metadata-only Tenant RBAC envelopes for a later runtime adapter to
//! execute. Nothing here performs IO: no persistence, no network, no process
//! execution.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};
use iam_tenant_rbac_domain::{
    AiSuggestionRef, AuditEvidenceRef, CrossServiceWorkflowPlan, DeterministicGate,
    IncidentFirstAction, IncidentId, IncidentPlanStatus, IncidentRemediationRoute,
    IncidentRollbackPlan, IncidentTrigger, ObjectGraphRelationshipRef, TenantId, WorkflowRef,
};

const TENANT_RBAC_OPS_TOPIC: &str = "audit.tenant-rbac.ops.command";
const TENANT_RBAC_WORKFLOW_TOPIC: &str = "workflow.tenant-rbac.cross-service.dispatch";
const TENANT_RBAC_INCIDENT_ROLLBACK_TOPIC: &str = "incident.tenant-rbac.rollback.plan";
const TENANT_ID_PREFIX: &str = "ten_";
const AUDIT_EVIDENCE_PREFIX: &str = "audit/";
const OPENTOFU_PLAN_PREFIX: &str = "opentofu/";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TenantRbacOpsRoute {
    MakefileTarget,
    OyaOps,
    OpsConsole,
    ManualSsh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum OpsCommandKind {
    Bootstrap,
    Install,
    Plan,
    Apply,
    Rollback,
    Day2Change,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantRbacOpsCommandInput {
    pub tenant_id: String,            // data_class: INTERNAL_ONLY
    pub route: TenantRbacOpsRoute,    // data_class: INTERNAL_ONLY
    pub command_kind: OpsCommandKind, // data_class: INTERNAL_ONLY
    pub evidence_ref: String,         // data_class: INTERNAL_ONLY
    pub change_plan_ref: String,      // data_class: INTERNAL_ONLY
    pub idempotency_key: String,      // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantRbacOpsEnvelope {
    pub topic: Classified<String>,             // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<TenantId>,       // data_class: INTERNAL_ONLY
    pub route: Classified<TenantRbacOpsRoute>, // data_class: INTERNAL_ONLY
    pub command_kind: Classified<OpsCommandKind>, // data_class: INTERNAL_ONLY
    pub evidence_ref: Classified<AuditEvidenceRef>, // data_class: INTERNAL_ONLY
    pub change_plan_ref: Classified<String>,   // data_class: INTERNAL_ONLY
    pub idempotency_key: Classified<String>,   // data_class: INTERNAL_ONLY
    pub payload_data_class: Classified<DataClass>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,       // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrossServiceWorkflowEnvelope {
    pub topic: Classified<String>,             // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<TenantId>,       // data_class: INTERNAL_ONLY
    pub workflow_ref: Classified<WorkflowRef>, // data_class: INTERNAL_ONLY
    pub object_graph_relationship_ref: Classified<ObjectGraphRelationshipRef>, // data_class: INTERNAL_ONLY
    pub required_gates: Classified<Vec<DeterministicGate>>, // data_class: INTERNAL_ONLY
    pub gate_evidence_refs: Classified<Vec<AuditEvidenceRef>>, // data_class: INTERNAL_ONLY
    pub ai_suggestion_ref: Classified<Option<AiSuggestionRef>>, // data_class: INTERNAL_ONLY
    pub idempotency_key: Classified<String>,                // data_class: INTERNAL_ONLY
    pub payload_data_class: Classified<DataClass>,          // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,                    // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncidentRollbackEnvelope {
    pub topic: Classified<String>,            // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<TenantId>,      // data_class: INTERNAL_ONLY
    pub incident_id: Classified<IncidentId>,  // data_class: INTERNAL_ONLY
    pub trigger: Classified<IncidentTrigger>, // data_class: INTERNAL_ONLY
    pub first_action: Classified<IncidentFirstAction>, // data_class: INTERNAL_ONLY
    pub remediation_route: Classified<IncidentRemediationRoute>, // data_class: INTERNAL_ONLY
    pub canary_evidence_ref: Classified<AuditEvidenceRef>, // data_class: INTERNAL_ONLY
    pub incident_evidence_ref: Classified<AuditEvidenceRef>, // data_class: INTERNAL_ONLY
    pub rollback_evidence_ref: Classified<AuditEvidenceRef>, // data_class: INTERNAL_ONLY
    pub convergence_ref: Classified<String>,  // data_class: INTERNAL_ONLY
    pub idempotency_key: Classified<String>,  // data_class: INTERNAL_ONLY
    pub plan_status: Classified<IncidentPlanStatus>, // data_class: INTERNAL_ONLY
    pub payload_data_class: Classified<DataClass>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,      // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TenantRbacApplicationError {
    InvalidTenantId,
    InvalidEvidenceRef,
    InvalidChangePlanRef,
    InvalidIdempotencyKey,
    ManualSshRefused,
}

pub fn prepare_tenant_rbac_ops_envelope(
    input: TenantRbacOpsCommandInput,
) -> Result<TenantRbacOpsEnvelope, TenantRbacApplicationError> {
    if input.route == TenantRbacOpsRoute::ManualSsh {
        return Err(TenantRbacApplicationError::ManualSshRefused);
    }
    require_prefixed(
        &input.tenant_id,
        TENANT_ID_PREFIX,
        TenantRbacApplicationError::InvalidTenantId,
    )?;
    require_prefixed(
        &input.evidence_ref,
        AUDIT_EVIDENCE_PREFIX,
        TenantRbacApplicationError::InvalidEvidenceRef,
    )?;
    require_prefixed(
        &input.change_plan_ref,
        OPENTOFU_PLAN_PREFIX,
        TenantRbacApplicationError::InvalidChangePlanRef,
    )?;
    require_non_empty_key(&input.idempotency_key)?;

    Ok(TenantRbacOpsEnvelope {
        topic: internal(TENANT_RBAC_OPS_TOPIC.to_owned()),
        tenant_id: internal(TenantId {
            value: input.tenant_id,
        }),
        route: internal(input.route),
        command_kind: internal(input.command_kind),
        evidence_ref: internal(AuditEvidenceRef {
            value: input.evidence_ref,
        }),
        change_plan_ref: internal(input.change_plan_ref),
        idempotency_key: internal(input.idempotency_key),
        payload_data_class: internal(DataClass::InternalOnly),
        schema_version: public(1),
    })
}

pub fn prepare_cross_service_workflow_envelope(
    plan: &CrossServiceWorkflowPlan,
) -> CrossServiceWorkflowEnvelope {
    CrossServiceWorkflowEnvelope {
        topic: internal(TENANT_RBAC_WORKFLOW_TOPIC.to_owned()),
        tenant_id: internal(plan.tenant_id.value.clone()),
        workflow_ref: internal(plan.workflow_ref.value.clone()),
        object_graph_relationship_ref: internal(plan.object_graph_relationship_ref.value.clone()),
        required_gates: internal(plan.required_gates.value.clone()),
        gate_evidence_refs: internal(
            plan.gate_evidence
                .value
                .iter()
                .map(|entry| entry.evidence_ref.value.clone())
                .collect(),
        ),
        ai_suggestion_ref: internal(plan.ai_suggestion_ref.value.clone()),
        idempotency_key: internal(plan.idempotency_key.value.clone()),
        payload_data_class: internal(DataClass::InternalOnly),
        schema_version: public(1),
    }
}

pub fn prepare_incident_rollback_envelope(plan: &IncidentRollbackPlan) -> IncidentRollbackEnvelope {
    IncidentRollbackEnvelope {
        topic: internal(TENANT_RBAC_INCIDENT_ROLLBACK_TOPIC.to_owned()),
        tenant_id: internal(plan.tenant_id.value.clone()),
        incident_id: internal(plan.incident_id.value.clone()),
        trigger: internal(plan.trigger.value),
        first_action: internal(plan.first_action.value),
        remediation_route: internal(plan.remediation_route.value),
        canary_evidence_ref: internal(plan.canary_evidence_ref.value.clone()),
        incident_evidence_ref: internal(plan.incident_evidence_ref.value.clone()),
        rollback_evidence_ref: internal(plan.rollback_evidence_ref.value.clone()),
        convergence_ref: internal(plan.convergence_ref.value.value.clone()),
        idempotency_key: internal(plan.idempotency_key.value.clone()),
        plan_status: internal(plan.plan_status.value),
        payload_data_class: internal(DataClass::InternalOnly),
        schema_version: public(1),
    }
}

fn require_prefixed(
    value: &str,
    prefix: &str,
    error: TenantRbacApplicationError,
) -> Result<(), TenantRbacApplicationError> {
    if value.len() <= prefix.len()
        || !value.starts_with(prefix)
        || value.contains("..")
        || value.chars().any(char::is_whitespace)
    {
        return Err(error);
    }
    Ok(())
}

fn require_non_empty_key(value: &str) -> Result<(), TenantRbacApplicationError> {
    if value.trim().is_empty() || value.contains("..") || value.chars().any(char::is_whitespace) {
        return Err(TenantRbacApplicationError::InvalidIdempotencyKey);
    }
    Ok(())
}

fn internal<T>(value: T) -> Classified<T> {
    Classified::new(value, PrivacyDataClass::internal_only())
}

fn public<T>(value: T) -> Classified<T> {
    Classified::new(value, DataClass::Public)
}
