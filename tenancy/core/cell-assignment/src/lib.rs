//! Tenant-to-Cell binding contracts and legacy assignment compatibility.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

mod authority_high_water;
mod binding_abort_writes;
mod binding_attempt;
mod binding_attempt_writes;
mod binding_audit;
mod binding_audit_effect;
mod binding_authority;
mod binding_checkpoints;
mod binding_control_contribution;
mod binding_control_contribution_ack;
mod binding_control_contribution_store;
mod binding_control_partition;
mod binding_invocation;
mod binding_model;
mod binding_operation;
mod binding_projection;
mod binding_projection_store;
mod binding_proof_domain;
mod binding_proof_verifier;
mod binding_reconciliation;
mod binding_reconciliation_authority;
mod binding_repair_authority;
mod binding_service;
mod binding_service_stub;
mod binding_store;
mod binding_work_snapshot;
mod binding_writes;
mod cell_binding_index;
mod cell_binding_index_projection;
mod migration_claim_identity;
mod migration_coordination_service;
mod migration_coordination_service_stub;
mod migration_fence_writes;
mod migration_release;
mod migration_retarget;
mod migration_seal;
mod migration_work;
mod participant;
mod participant_phase_closure;
mod participant_receipt;
mod participant_store;
mod participant_work;
mod projection_convergence;
mod rollback_window;
mod serving_authority;
mod serving_authority_freeze;
mod serving_authority_handoff;
mod serving_authority_installation;
mod serving_authority_invocation;
mod serving_authority_proof;
mod serving_authority_reconciliation;
mod serving_authority_restore;
mod serving_authority_retirement;
mod serving_authority_service;
mod serving_authority_store;
mod source_fence_directive_store;
mod source_release_issuance;
mod store_commit_attestation;
mod tenant_birth;
mod transfer_authority;
mod transfer_execution;
mod transfer_execution_store;
mod transfer_journal;
mod transfer_repair;
mod transfer_work;
mod write_authority_consumer;
mod write_authority_credential;
mod write_authority_issuer;
mod write_authority_lease_issuance;
mod write_authority_lease_state;
mod write_authority_token_issuer;
mod write_fence;

pub use authority_high_water::*;
pub use binding_abort_writes::*;
pub use binding_attempt::*;
pub use binding_attempt_writes::*;
pub use binding_audit::*;
pub use binding_audit_effect::*;
pub use binding_authority::*;
pub use binding_checkpoints::*;
pub use binding_control_contribution::*;
pub use binding_control_contribution_ack::*;
pub use binding_control_contribution_store::*;
pub use binding_control_partition::*;
pub use binding_invocation::*;
pub use binding_model::*;
pub use binding_operation::*;
pub use binding_projection::*;
pub use binding_projection_store::*;
pub use binding_proof_domain::*;
pub use binding_proof_verifier::*;
pub use binding_reconciliation::*;
pub use binding_reconciliation_authority::*;
pub use binding_repair_authority::*;
pub use binding_service::*;
pub use binding_service_stub::*;
pub use binding_store::*;
pub use binding_work_snapshot::*;
pub use binding_writes::*;
pub use cell_binding_index::*;
pub use cell_binding_index_projection::*;
pub use cell_placement::CellId as CanonicalCellId;
pub use migration_claim_identity::*;
pub use migration_coordination_service::*;
pub use migration_coordination_service_stub::*;
pub use migration_fence_writes::*;
pub use migration_release::*;
pub use migration_retarget::*;
pub use migration_seal::*;
pub use migration_work::*;
pub use participant::*;
pub use participant_phase_closure::*;
pub use participant_receipt::*;
pub use participant_store::*;
pub use participant_work::*;
pub use projection_convergence::*;
pub use rollback_window::*;
pub use serving_authority::*;
pub use serving_authority_freeze::*;
pub use serving_authority_handoff::*;
pub use serving_authority_installation::*;
pub use serving_authority_invocation::*;
pub use serving_authority_proof::*;
pub use serving_authority_reconciliation::*;
pub use serving_authority_restore::*;
pub use serving_authority_retirement::*;
pub use serving_authority_service::*;
pub use serving_authority_store::*;
pub use source_fence_directive_store::*;
pub use source_release_issuance::*;
pub use store_commit_attestation::*;
pub use tenancy_kernel::TenantId;
pub use tenant_birth::*;
pub use transfer_authority::*;
pub use transfer_execution::*;
pub use transfer_execution_store::*;
pub use transfer_journal::*;
pub use transfer_repair::*;
pub use transfer_work::*;
pub use write_authority_consumer::*;
pub use write_authority_credential::*;
pub use write_authority_issuer::*;
pub use write_authority_lease_issuance::*;
pub use write_authority_lease_state::*;
pub use write_authority_token_issuer::*;
pub use write_fence::*;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CellId(pub String); // data_class: INTERNAL_ONLY

pub type LegacyCellId = CellId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellIdentityConversionError {
    NotImplemented,
    InvalidLegacyCellId,
    NonCanonicalCellId,
}

pub fn canonical_cell_id_from_legacy(
    _legacy: &LegacyCellId,
) -> Result<CanonicalCellId, CellIdentityConversionError> {
    Err(CellIdentityConversionError::NotImplemented)
}

pub fn legacy_cell_id_from_canonical(
    _canonical: &CanonicalCellId,
) -> Result<LegacyCellId, CellIdentityConversionError> {
    Err(CellIdentityConversionError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ShardKey(pub u64); // data_class: INTERNAL_ONLY

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceTask {
    pub tenant: String,    // data_class: INTERNAL_ONLY
    pub from_cell: CellId, // data_class: INTERNAL_ONLY
    pub to_cell: CellId,   // data_class: INTERNAL_ONLY
    pub reason: String,    // data_class: INTERNAL_ONLY
}

pub trait CellAssignmentRepository {
    fn assigned_cell(&self, tenant: &str) -> Result<Option<CellId>, CellKernelError>;
    fn record_assignment(&self, tenant: &str, cell: &CellId) -> Result<(), CellKernelError>;
}

pub trait CellHealthProbe {
    fn probe(&self, cell: &CellId) -> Result<CellHealth, CellKernelError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellKernelError {
    NoHealthyCell,
    ProbeFailed,
    PersistenceUnavailable,
    RebalanceConflict,
}
